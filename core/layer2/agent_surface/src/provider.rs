// Layer ownership: Core Layer 2 (Scheduling + Execution) - agent runtime surface coordination.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProviderRequest {
    pub prompt: String,
    pub system: Option<String>,
    pub tools: Vec<String>,
    pub model: Option<String>,
    pub metadata: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProviderResponse {
    pub provider: String,
    pub model: String,
    pub output: String,
    pub usage_tokens: u64,
    pub raw: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProviderStreamEvent {
    pub event_type: String,
    pub text: Option<String>,
    pub raw: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProviderStreamResponse {
    pub response: ProviderResponse,
    pub events: Vec<ProviderStreamEvent>,
    pub stopped_early: bool,
    pub stop_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum ProviderErrorCode {
    Unavailable,
    AuthMissing,
    InvalidRequest,
    NotRegistered,
    Timeout,
}

impl ProviderErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unavailable => "provider_unavailable",
            Self::AuthMissing => "provider_auth_missing",
            Self::InvalidRequest => "provider_invalid_request",
            Self::NotRegistered => "provider_not_registered",
            Self::Timeout => "provider_timeout",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProviderError {
    pub code: ProviderErrorCode,
    pub message: String,
}

impl ProviderError {
    pub fn new(code: ProviderErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

pub trait ProviderClient: Send + Sync {
    fn provider_id(&self) -> &'static str;
    fn complete(&self, request: &ProviderRequest) -> Result<ProviderResponse, ProviderError>;

    fn stream_complete(
        &self,
        request: &ProviderRequest,
    ) -> Result<ProviderStreamResponse, ProviderError> {
        let response = self.complete(request)?;
        Ok(ProviderStreamResponse {
            events: vec![ProviderStreamEvent {
                event_type: "completion".to_string(),
                text: Some(response.output.clone()),
                raw: json!({"fallback": "complete"}),
            }],
            response,
            stopped_early: false,
            stop_reason: None,
        })
    }
}

#[derive(Default)]
pub struct LocalEchoProvider;

impl ProviderClient for LocalEchoProvider {
    fn provider_id(&self) -> &'static str {
        "local-echo"
    }

    fn complete(&self, request: &ProviderRequest) -> Result<ProviderResponse, ProviderError> {
        if request.prompt.trim().is_empty() {
            return Err(ProviderError::new(
                ProviderErrorCode::InvalidRequest,
                "prompt_required",
            ));
        }
        let model = request
            .model
            .clone()
            .unwrap_or_else(|| "local-echo-v1".to_string());
        let output = format!(
            "[{}] {}",
            request
                .system
                .clone()
                .unwrap_or_else(|| "no-system".to_string()),
            request.prompt.trim()
        );
        Ok(ProviderResponse {
            provider: self.provider_id().to_string(),
            model,
            output: output.clone(),
            usage_tokens: output.split_whitespace().count() as u64,
            raw: json!({
                "ok": true,
                "provider": self.provider_id(),
                "echo": output,
                "tools": request.tools,
            }),
        })
    }
}

#[derive(Default)]
pub struct OllamaCliProvider;

impl ProviderClient for OllamaCliProvider {
    fn provider_id(&self) -> &'static str {
        "ollama"
    }

    fn complete(&self, request: &ProviderRequest) -> Result<ProviderResponse, ProviderError> {
        if request.prompt.trim().is_empty() {
            return Err(ProviderError::new(
                ProviderErrorCode::InvalidRequest,
                "prompt_required",
            ));
        }
        let model = request
            .model
            .clone()
            .or_else(|| std::env::var("INFRING_OLLAMA_MODEL").ok())
            .unwrap_or_else(|| "kimi-k2.6:cloud".to_string());
        let binary = std::env::var("INFRING_OLLAMA_BIN")
            .or_else(|_| std::env::var("OLLAMA_BIN"))
            .unwrap_or_else(|_| "ollama".to_string());
        let system = request.system.clone().unwrap_or_default();
        let full_prompt = if system.trim().is_empty() {
            request.prompt.clone()
        } else {
            format!("{system}\n\n{}", request.prompt)
        };
        let omit_thinking_flags = request
            .metadata
            .get("omit_ollama_thinking_flags")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let timeout = provider_timeout_from_request(request);
        let mut output =
            run_ollama_cli_completion(&binary, &model, &full_prompt, omit_thinking_flags, timeout)?;
        let mut stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if !omit_thinking_flags
            && !output.status.success()
            && ollama_thinking_flags_retryable_stderr(&stderr)
        {
            output = run_ollama_cli_completion(&binary, &model, &full_prompt, true, timeout)?;
            stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        }
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !output.status.success() {
            return Err(ProviderError::new(
                ProviderErrorCode::Unavailable,
                format!(
                    "ollama_run_failed:status={}:stderr={}",
                    output.status.code().unwrap_or(-1),
                    stderr
                ),
            ));
        }
        Ok(ProviderResponse {
            provider: self.provider_id().to_string(),
            model,
            output: stdout.clone(),
            usage_tokens: stdout.split_whitespace().count() as u64,
            raw: json!({
                "ok": true,
                "provider": self.provider_id(),
                "stderr": stderr,
                "tools": request.tools,
            }),
        })
    }

    fn stream_complete(
        &self,
        request: &ProviderRequest,
    ) -> Result<ProviderStreamResponse, ProviderError> {
        if request.prompt.trim().is_empty() {
            return Err(ProviderError::new(
                ProviderErrorCode::InvalidRequest,
                "prompt_required",
            ));
        }
        let model = request
            .model
            .clone()
            .or_else(|| std::env::var("INFRING_OLLAMA_MODEL").ok())
            .unwrap_or_else(|| "kimi-k2.6:cloud".to_string());
        let binary = std::env::var("INFRING_OLLAMA_BIN")
            .or_else(|_| std::env::var("OLLAMA_BIN"))
            .unwrap_or_else(|_| "ollama".to_string());
        let system = request.system.clone().unwrap_or_default();
        let full_prompt = if system.trim().is_empty() {
            request.prompt.clone()
        } else {
            format!("{system}\n\n{}", request.prompt)
        };
        let omit_thinking_flags = request
            .metadata
            .get("omit_ollama_thinking_flags")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let timeout = provider_timeout_from_request(request);
        let stop_on_tool_calls = request
            .metadata
            .get("provider_stream_until_tool_calls")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        match run_ollama_cli_streaming_completion(
            &binary,
            &model,
            &full_prompt,
            omit_thinking_flags,
            timeout,
            stop_on_tool_calls,
        ) {
            Ok(stream) => Ok(stream_response_from_output(
                self.provider_id(),
                &model,
                stream,
                request,
            )),
            Err(error)
                if !omit_thinking_flags
                    && ollama_thinking_flags_retryable_stderr(&error.message) =>
            {
                let stream = run_ollama_cli_streaming_completion(
                    &binary,
                    &model,
                    &full_prompt,
                    true,
                    timeout,
                    stop_on_tool_calls,
                )?;
                Ok(stream_response_from_output(
                    self.provider_id(),
                    &model,
                    stream,
                    request,
                ))
            }
            Err(error) => Err(error),
        }
    }
}

#[derive(Default)]
pub struct ClaudeCodeGatewayProvider;

impl ProviderClient for ClaudeCodeGatewayProvider {
    fn provider_id(&self) -> &'static str {
        "claude-code"
    }

    fn complete(&self, request: &ProviderRequest) -> Result<ProviderResponse, ProviderError> {
        if request.prompt.trim().is_empty() {
            return Err(ProviderError::new(
                ProviderErrorCode::InvalidRequest,
                "prompt_required",
            ));
        }
        let base_url = provider_gateway_base_url().ok_or_else(|| {
            ProviderError::new(
                ProviderErrorCode::Unavailable,
                "provider_gateway_url_missing",
            )
        })?;
        let model = request
            .model
            .clone()
            .unwrap_or_else(|| "claude-code/sonnet".to_string());
        let timeout_seconds = provider_timeout_from_request(request)
            .map(|duration| duration.as_secs())
            .unwrap_or(120)
            .clamp(1, 3600);
        let url = format!("{base_url}/api/shell-socket/providers/claude-code/complete");
        let body = json!({
            "prompt": request.prompt.clone(),
            "system": request.system.clone(),
            "model": model.clone(),
            "tools": request.tools.clone(),
            "metadata": request.metadata.clone(),
            "timeout_seconds": timeout_seconds,
        });
        let output = post_provider_gateway_json(&url, &body, timeout_seconds)?;
        let parsed = serde_json::from_slice::<Value>(&output.stdout).map_err(|error| {
            ProviderError::new(
                ProviderErrorCode::Unavailable,
                format!("provider_gateway_invalid_json:{error}"),
            )
        })?;
        if !output.status.success() || !parsed.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            return Err(ProviderError::new(
                ProviderErrorCode::Unavailable,
                parsed
                    .get("error")
                    .and_then(Value::as_str)
                    .unwrap_or("provider_gateway_completion_failed"),
            ));
        }
        let provider = parsed
            .get("provider")
            .and_then(Value::as_str)
            .unwrap_or(self.provider_id())
            .to_string();
        let model = parsed
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or(model.as_str())
            .to_string();
        let output_text = parsed
            .get("output")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        Ok(ProviderResponse {
            provider,
            model,
            output: output_text.clone(),
            usage_tokens: parsed
                .get("usage_tokens")
                .and_then(Value::as_u64)
                .unwrap_or_else(|| output_text.split_whitespace().count() as u64),
            raw: json!({
                "ok": true,
                "gateway_route": "/api/shell-socket/providers/claude-code/complete",
                "receipt_ref": parsed.get("receipt_ref").cloned().unwrap_or(Value::Null),
                "correlation_id": parsed.get("correlation_id").cloned().unwrap_or(Value::Null),
                "latency_ms": parsed.get("latency_ms").cloned().unwrap_or(Value::Null),
            }),
        })
    }
}

fn provider_gateway_base_url() -> Option<String> {
    for key in [
        "INFRING_PROVIDER_GATEWAY_URL",
        "INFRING_SHELL_SOCKET_GATEWAY_URL",
        "INFRING_GATEWAY_URL",
    ] {
        let Ok(value) = std::env::var(key) else {
            continue;
        };
        let cleaned = value.trim().trim_end_matches('/').to_string();
        if !cleaned.is_empty() {
            return Some(cleaned);
        }
    }
    Some("http://127.0.0.1:4173".to_string())
}

fn post_provider_gateway_json(
    url: &str,
    body: &Value,
    timeout_seconds: u64,
) -> Result<std::process::Output, ProviderError> {
    let mut child = Command::new("curl")
        .arg("-sS")
        .arg("--max-time")
        .arg(timeout_seconds.to_string())
        .arg("-X")
        .arg("POST")
        .arg("-H")
        .arg("Content-Type: application/json")
        .arg("--data-binary")
        .arg("@-")
        .arg(url)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            let code = if error.kind() == std::io::ErrorKind::NotFound {
                ProviderErrorCode::Unavailable
            } else {
                ProviderErrorCode::InvalidRequest
            };
            ProviderError::new(code, format!("provider_gateway_curl_spawn_failed:{error}"))
        })?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(body.to_string().as_bytes())
            .map_err(|error| {
                ProviderError::new(
                    ProviderErrorCode::Unavailable,
                    format!("provider_gateway_stdin_write_failed:{error}"),
                )
            })?;
    }
    child.wait_with_output().map_err(|error| {
        ProviderError::new(
            ProviderErrorCode::Unavailable,
            format!("provider_gateway_wait_failed:{error}"),
        )
    })
}

#[derive(Debug)]
struct StreamingProcessOutput {
    stdout: String,
    stderr: String,
    status_code: Option<i32>,
    stopped_early: bool,
    stop_reason: Option<String>,
    first_byte_ms: Option<u64>,
    first_open_brace_ms: Option<u64>,
    first_tool_calls_marker_ms: Option<u64>,
    balanced_tool_calls_json_ms: Option<u64>,
}

fn stream_response_from_output(
    provider_id: &str,
    model: &str,
    output: StreamingProcessOutput,
    request: &ProviderRequest,
) -> ProviderStreamResponse {
    let response = ProviderResponse {
        provider: provider_id.to_string(),
        model: model.to_string(),
        output: output.stdout.trim().to_string(),
        usage_tokens: output.stdout.split_whitespace().count() as u64,
        raw: json!({
            "ok": true,
            "provider": provider_id,
            "stderr": output.stderr,
            "tools": request.tools,
            "streaming": true,
            "stopped_early": output.stopped_early,
            "stop_reason": output.stop_reason,
            "status_code": output.status_code,
            "stream_diagnostics": {
                "first_byte_ms": output.first_byte_ms,
                "first_open_brace_ms": output.first_open_brace_ms,
                "first_tool_calls_marker_ms": output.first_tool_calls_marker_ms,
                "balanced_tool_calls_json_ms": output.balanced_tool_calls_json_ms,
            },
        }),
    };
    let text = response.output.clone();
    ProviderStreamResponse {
        response,
        events: vec![ProviderStreamEvent {
            event_type: if output.stopped_early {
                "tool_calls_json_detected".to_string()
            } else {
                "completion".to_string()
            },
            text: Some(text),
            raw: json!({
                "stopped_early": output.stopped_early,
                "stop_reason": output.stop_reason,
            }),
        }],
        stopped_early: output.stopped_early,
        stop_reason: output.stop_reason,
    }
}

fn ollama_thinking_flags_retryable_stderr(stderr: &str) -> bool {
    stderr.contains("does not support thinking")
        || stderr.contains("400 Bad Request")
        || stderr.contains("Bad Request")
}

fn run_ollama_cli_completion(
    binary: &str,
    model: &str,
    full_prompt: &str,
    omit_thinking_flags: bool,
    timeout: Option<Duration>,
) -> Result<std::process::Output, ProviderError> {
    let mut command = Command::new(binary);
    command.arg("run").arg(model).arg("--nowordwrap");
    if !omit_thinking_flags {
        command.arg("--hidethinking").arg("--think").arg("false");
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            let code = if error.kind() == std::io::ErrorKind::NotFound {
                ProviderErrorCode::Unavailable
            } else {
                ProviderErrorCode::InvalidRequest
            };
            ProviderError::new(code, format!("ollama_spawn_failed:{error}"))
        })?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(full_prompt.as_bytes()).map_err(|error| {
            ProviderError::new(
                ProviderErrorCode::Unavailable,
                format!("ollama_stdin_write_failed:{error}"),
            )
        })?;
    }
    wait_for_ollama_output(child, timeout)
}

fn run_ollama_cli_streaming_completion(
    binary: &str,
    model: &str,
    full_prompt: &str,
    omit_thinking_flags: bool,
    timeout: Option<Duration>,
    stop_on_tool_calls: bool,
) -> Result<StreamingProcessOutput, ProviderError> {
    let mut command = Command::new(binary);
    command.arg("run").arg(model).arg("--nowordwrap");
    if !omit_thinking_flags {
        command.arg("--hidethinking").arg("--think").arg("false");
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            let code = if error.kind() == std::io::ErrorKind::NotFound {
                ProviderErrorCode::Unavailable
            } else {
                ProviderErrorCode::InvalidRequest
            };
            ProviderError::new(code, format!("ollama_spawn_failed:{error}"))
        })?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(full_prompt.as_bytes()).map_err(|error| {
            ProviderError::new(
                ProviderErrorCode::Unavailable,
                format!("ollama_stdin_write_failed:{error}"),
            )
        })?;
    }
    drop(child.stdin.take());

    let pid = child.id();
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| ProviderError::new(ProviderErrorCode::Unavailable, "ollama_stdout_missing"))?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| ProviderError::new(ProviderErrorCode::Unavailable, "ollama_stderr_missing"))?;
    let (stderr_tx, stderr_rx) = mpsc::channel();
    thread::spawn(move || {
        let mut buffer = Vec::new();
        let _ = stderr.read_to_end(&mut buffer);
        let _ = stderr_tx.send(String::from_utf8_lossy(&buffer).to_string());
    });
    let (stdout_tx, stdout_rx) = mpsc::channel();
    thread::spawn(move || {
        let mut buffer = [0u8; 1024];
        loop {
            match stdout.read(&mut buffer) {
                Ok(0) => {
                    let _ = stdout_tx.send(Ok(Vec::new()));
                    break;
                }
                Ok(count) => {
                    if stdout_tx.send(Ok(buffer[..count].to_vec())).is_err() {
                        break;
                    }
                }
                Err(error) => {
                    let _ = stdout_tx.send(Err(error.to_string()));
                    break;
                }
            }
        }
    });

    let started = std::time::Instant::now();
    let mut stdout_text = String::new();
    let mut stopped_early = false;
    let mut stop_reason = None;
    let mut first_byte_ms = None;
    let mut first_open_brace_ms = None;
    let mut first_tool_calls_marker_ms = None;
    let mut balanced_tool_calls_json_ms = None;
    loop {
        let chunk = if let Some(limit) = timeout {
            if started.elapsed() >= limit {
                terminate_process(pid, false);
                return Err(ProviderError::new(
                    ProviderErrorCode::Timeout,
                    format!("ollama_run_timeout:timeout_seconds={}", limit.as_secs()),
                ));
            }
            let remaining = limit.saturating_sub(started.elapsed());
            match stdout_rx.recv_timeout(remaining.min(Duration::from_millis(200))) {
                Ok(chunk) => chunk,
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        } else {
            match stdout_rx.recv() {
                Ok(chunk) => chunk,
                Err(_) => break,
            }
        };
        match chunk {
            Ok(bytes) if bytes.is_empty() => break,
            Ok(bytes) => {
                let count = bytes.len();
                let elapsed_ms = started.elapsed().as_millis() as u64;
                if first_byte_ms.is_none() {
                    first_byte_ms = Some(elapsed_ms);
                }
                stdout_text.push_str(&String::from_utf8_lossy(&bytes));
                if first_open_brace_ms.is_none() && stdout_text.contains('{') {
                    first_open_brace_ms = Some(elapsed_ms);
                }
                if first_tool_calls_marker_ms.is_none() && stdout_text.contains("\"tool_calls\"") {
                    first_tool_calls_marker_ms = Some(elapsed_ms);
                }
                if provider_first_tool_calls_json(&stdout_text).is_some() {
                    if balanced_tool_calls_json_ms.is_none() {
                        balanced_tool_calls_json_ms = Some(elapsed_ms);
                    }
                }
                if stop_on_tool_calls && balanced_tool_calls_json_ms.is_some() {
                    stopped_early = true;
                    stop_reason = Some("tool_calls_json_detected".to_string());
                    terminate_process(pid, false);
                    break;
                }
                if count == 0 {
                    break;
                }
            }
            Err(error) => {
                terminate_process(pid, true);
                return Err(ProviderError::new(
                    ProviderErrorCode::Unavailable,
                    format!("ollama_stdout_read_failed:{error}"),
                ));
            }
        }
    }

    let status = match child.wait() {
        Ok(status) => status,
        Err(error) if stopped_early => {
            let stderr = stderr_rx.recv_timeout(Duration::from_secs(2)).unwrap_or_default();
            return Ok(StreamingProcessOutput {
                stdout: provider_first_tool_calls_json(&stdout_text).unwrap_or(stdout_text),
                stderr,
                status_code: None,
                stopped_early,
                stop_reason,
                first_byte_ms,
                first_open_brace_ms,
                first_tool_calls_marker_ms,
                balanced_tool_calls_json_ms,
            });
        }
        Err(error) => {
            return Err(ProviderError::new(
                ProviderErrorCode::Unavailable,
                format!("ollama_wait_failed:{error}"),
            ))
        }
    };
    let stderr = stderr_rx.recv_timeout(Duration::from_secs(2)).unwrap_or_default();
    if !stopped_early && !status.success() {
        return Err(ProviderError::new(
            ProviderErrorCode::Unavailable,
            format!(
                "ollama_run_failed:status={}:stderr={}",
                status.code().unwrap_or(-1),
                stderr.trim()
            ),
        ));
    }
    Ok(StreamingProcessOutput {
        stdout: if stopped_early {
            provider_first_tool_calls_json(&stdout_text).unwrap_or(stdout_text)
        } else {
            stdout_text
        },
        stderr,
        status_code: status.code(),
        stopped_early,
        stop_reason,
        first_byte_ms,
        first_open_brace_ms,
        first_tool_calls_marker_ms,
        balanced_tool_calls_json_ms,
    })
}

fn provider_first_tool_calls_json(text: &str) -> Option<String> {
    for (start, _) in text.char_indices().filter(|(_, ch)| *ch == '{') {
        let Some(candidate) = provider_balanced_json_from(text, start) else {
            continue;
        };
        let Ok(parsed) = serde_json::from_str::<Value>(&candidate) else {
            continue;
        };
        if parsed
            .get("tool_calls")
            .and_then(Value::as_array)
            .map(|calls| !calls.is_empty())
            .unwrap_or(false)
        {
            return Some(candidate);
        }
    }
    None
}

fn provider_balanced_json_from(text: &str, start: usize) -> Option<String> {
    let mut depth = 0i64;
    let mut in_string = false;
    let mut escaped = false;
    for (offset, ch) in text[start..].char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    let end = start + offset + ch.len_utf8();
                    return Some(text[start..end].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

fn provider_timeout_from_request(request: &ProviderRequest) -> Option<Duration> {
    let seconds = request
        .metadata
        .get("provider_timeout_seconds")
        .and_then(Value::as_u64)
        .or_else(|| {
            request
                .metadata
                .pointer("/native_success_criteria/provider_timeout_seconds")
                .and_then(Value::as_u64)
        })
        .or_else(|| {
            request
                .metadata
                .pointer("/workflow/native_success_criteria/provider_timeout_seconds")
                .and_then(Value::as_u64)
        })
        .or_else(|| {
            std::env::var("INFRING_PROVIDER_TIMEOUT_SECONDS")
                .ok()
                .and_then(|value| value.parse::<u64>().ok())
        })?;
    if seconds == 0 {
        None
    } else {
        Some(Duration::from_secs(seconds.clamp(1, 3600)))
    }
}

fn wait_for_ollama_output(
    child: std::process::Child,
    timeout: Option<Duration>,
) -> Result<std::process::Output, ProviderError> {
    let Some(timeout) = timeout else {
        return child.wait_with_output().map_err(|error| {
            ProviderError::new(
                ProviderErrorCode::Unavailable,
                format!("ollama_wait_failed:{error}"),
            )
        });
    };
    let pid = child.id();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = tx.send(child.wait_with_output());
    });
    match rx.recv_timeout(timeout) {
        Ok(result) => result.map_err(|error| {
            ProviderError::new(
                ProviderErrorCode::Unavailable,
                format!("ollama_wait_failed:{error}"),
            )
        }),
        Err(mpsc::RecvTimeoutError::Timeout) => {
            terminate_process(pid, false);
            if rx.recv_timeout(Duration::from_secs(5)).is_err() {
                terminate_process(pid, true);
                let _ = rx.recv_timeout(Duration::from_secs(2));
            }
            Err(ProviderError::new(
                ProviderErrorCode::Timeout,
                format!("ollama_run_timeout:timeout_seconds={}", timeout.as_secs()),
            ))
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => Err(ProviderError::new(
            ProviderErrorCode::Unavailable,
            "ollama_wait_channel_disconnected",
        )),
    }
}

fn terminate_process(pid: u32, force: bool) {
    let signal = if force { "-KILL" } else { "-TERM" };
    let _ = Command::new("kill")
        .arg(signal)
        .arg(pid.to_string())
        .status();
}

#[derive(Default)]
pub struct ProviderClientRegistry {
    default_provider: String,
    clients: BTreeMap<String, Arc<dyn ProviderClient>>,
}

impl ProviderClientRegistry {
    pub fn new(default_provider: impl Into<String>) -> Self {
        Self {
            default_provider: default_provider.into(),
            clients: BTreeMap::new(),
        }
    }

    pub fn with_builtin() -> Self {
        let mut registry = Self::new("local-echo");
        registry.register(LocalEchoProvider);
        registry.register(OllamaCliProvider);
        registry.register(ClaudeCodeGatewayProvider);
        registry
    }

    pub fn register_arc(&mut self, provider: Arc<dyn ProviderClient>) {
        self.clients
            .insert(provider.provider_id().to_string(), provider.clone());
    }

    pub fn register<C>(&mut self, provider: C)
    where
        C: ProviderClient + 'static,
    {
        self.register_arc(Arc::new(provider));
    }

    pub fn set_default_provider(&mut self, provider_id: impl Into<String>) {
        self.default_provider = provider_id.into();
    }

    pub fn default_provider_id(&self) -> &str {
        self.default_provider.as_str()
    }

    pub fn available_providers(&self) -> Vec<String> {
        self.clients.keys().cloned().collect()
    }

    pub fn from_provider_id(
        &self,
        provider_id: &str,
    ) -> Result<Arc<dyn ProviderClient>, ProviderError> {
        if let Some(provider) = self.clients.get(provider_id) {
            return Ok(provider.clone());
        }
        Err(ProviderError::new(
            ProviderErrorCode::NotRegistered,
            format!("provider_not_registered:{provider_id}"),
        ))
    }

    pub fn from_env(
        &self,
        env: &HashMap<String, String>,
        key: &str,
    ) -> Result<Arc<dyn ProviderClient>, ProviderError> {
        let selected = env
            .get(key)
            .or_else(|| env.get("INFRING_PROVIDER"))
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| self.default_provider.clone());
        self.from_provider_id(&selected)
    }

    pub fn from_process_env(&self) -> Result<Arc<dyn ProviderClient>, ProviderError> {
        let env_map = std::env::vars().collect::<HashMap<String, String>>();
        self.from_env(&env_map, "INFRING_PROVIDER")
    }
}
