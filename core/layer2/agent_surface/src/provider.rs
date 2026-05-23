// Layer ownership: Core Layer 2 (Scheduling + Execution) - agent runtime surface coordination.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
use std::io::Write;
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
        let mut output = run_ollama_cli_completion(
            &binary,
            &model,
            &full_prompt,
            omit_thinking_flags,
            timeout,
        )?;
        let mut stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if !omit_thinking_flags
            && !output.status.success()
            && stderr.contains("does not support thinking")
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
    let _ = Command::new("kill").arg(signal).arg(pid.to_string()).status();
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
