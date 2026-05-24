// Layer ownership: Core Layer 2 (Scheduling + Execution) - agent runtime native tool execution.
use crate::native_tools::paths::required_abs_path;
use serde_json::{json, Value};
use std::fs::{self, File};
use std::io::Read;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub fn command_run(args: &Value) -> Result<Value, String> {
    let cwd = required_abs_path(&json!({
        "path": args
            .get("cwd")
            .or_else(|| args.get("path"))
            .or_else(|| args.get("working_directory"))
            .or_else(|| args.get("working_dir"))
            .or_else(|| args.get("workdir"))
            .or_else(|| args.get("directory"))
            .or_else(|| args.get("dir"))
            .or_else(|| args.get("project_root"))
            .or_else(|| args.get("root"))
            .and_then(Value::as_str)
            .unwrap_or("")
    }))?;
    if !cwd.is_dir() {
        return Err("command_run_cwd_must_be_directory".to_string());
    }
    let (leading_env, cmd) = split_leading_env_assignments(command_argv(args)?)?;
    let timeout_seconds = args
        .get("timeout_seconds")
        .and_then(Value::as_u64)
        .unwrap_or(120)
        .clamp(1, 300);
    let max_output_bytes = args
        .get("max_output_bytes")
        .and_then(Value::as_u64)
        .unwrap_or(12_000)
        .clamp(1_024, 40_000) as usize;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_nanos())
        .unwrap_or(0);
    let stdout_path = std::env::temp_dir().join(format!(
        "infring-command-run-{}-{nonce}.stdout",
        std::process::id()
    ));
    let stderr_path = std::env::temp_dir().join(format!(
        "infring-command-run-{}-{nonce}.stderr",
        std::process::id()
    ));
    let stdout_file = File::create(&stdout_path)
        .map_err(|error| format!("command_run_stdout_create_failed:{error}"))?;
    let stderr_file = File::create(&stderr_path)
        .map_err(|error| format!("command_run_stderr_create_failed:{error}"))?;
    let mut command = Command::new(&cmd[0]);
    command
        .args(&cmd[1..])
        .current_dir(&cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file));
    if let Some(env) = args.get("env").and_then(Value::as_object) {
        for (key, value) in env.iter().take(32) {
            if let Some(value) = value.as_str() {
                if !key.is_empty() && !key.contains('\0') && !value.contains('\0') {
                    command.env(key, value);
                }
            }
        }
    }
    for (key, value) in leading_env {
        command.env(key, value);
    }
    let started = Instant::now();
    let mut child = command
        .spawn()
        .map_err(|error| format!("command_run_spawn_failed:{error}"))?;
    let timeout = Duration::from_secs(timeout_seconds);
    let mut timed_out = false;
    let status = loop {
        match child
            .try_wait()
            .map_err(|error| format!("command_run_wait_failed:{error}"))?
        {
            Some(status) => break status,
            None if started.elapsed() >= timeout => {
                timed_out = true;
                let _ = child.kill();
                break child
                    .wait()
                    .map_err(|error| format!("command_run_kill_wait_failed:{error}"))?;
            }
            None => thread::sleep(Duration::from_millis(25)),
        }
    };
    let (stdout, stdout_truncated) = read_capped_text(&stdout_path, max_output_bytes);
    let (stderr, stderr_truncated) = read_capped_text(&stderr_path, max_output_bytes);
    let _ = fs::remove_file(&stdout_path);
    let _ = fs::remove_file(&stderr_path);
    Ok(json!({
        "cwd": cwd.display().to_string(),
        "cmd": cmd,
        "exit_code": status.code(),
        "success": status.success() && !timed_out,
        "timed_out": timed_out,
        "duration_ms": started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
        "stdout": stdout,
        "stderr": stderr,
        "stdout_truncated": stdout_truncated,
        "stderr_truncated": stderr_truncated,
    }))
}

fn split_leading_env_assignments(
    cmd: Vec<String>,
) -> Result<(Vec<(String, String)>, Vec<String>), String> {
    let mut env = Vec::new();
    let mut command = Vec::new();
    let mut still_reading_env = true;
    for arg in cmd {
        if still_reading_env {
            if let Some((key, value)) = parse_env_assignment(&arg) {
                env.push((key, value));
                continue;
            }
            still_reading_env = false;
        }
        command.push(arg);
    }
    if command.is_empty() {
        return Err("command_run_cmd_array_required".to_string());
    }
    Ok((env, command))
}

fn parse_env_assignment(arg: &str) -> Option<(String, String)> {
    let (key, value) = arg.split_once('=')?;
    if !valid_env_key(key) || value.contains('\0') {
        return None;
    }
    Some((key.to_string(), value.to_string()))
}

fn valid_env_key(key: &str) -> bool {
    let mut bytes = key.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    if first.is_ascii_digit() {
        return false;
    }
    (first == b'_' || first.is_ascii_alphabetic())
        && bytes.all(|byte| byte == b'_' || byte.is_ascii_alphanumeric())
}

fn command_argv(args: &Value) -> Result<Vec<String>, String> {
    if let Some(raw) = args
        .get("cmd")
        .or_else(|| args.get("command"))
        .and_then(Value::as_str)
    {
        if raw.is_empty() || raw.contains('\0') {
            return Err("command_run_invalid_cmd_arg".to_string());
        }
        return Ok(vec!["sh".to_string(), "-lc".to_string(), raw.to_string()]);
    }
    let values = args
        .get("cmd")
        .or_else(|| args.get("command"))
        .and_then(Value::as_array)
        .ok_or_else(|| "command_run_cmd_array_required".to_string())?;
    let mut out = Vec::new();
    for value in values.iter().take(32) {
        let arg = value
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| "command_run_cmd_args_must_be_strings".to_string())?;
        if arg.is_empty() || arg.contains('\0') {
            return Err("command_run_invalid_cmd_arg".to_string());
        }
        out.push(arg);
    }
    if out.is_empty() {
        return Err("command_run_cmd_array_required".to_string());
    }
    Ok(out)
}

fn read_capped_text(path: &std::path::Path, max_bytes: usize) -> (String, bool) {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(_) => return (String::new(), false),
    };
    let mut bytes = Vec::new();
    let truncated = file
        .by_ref()
        .take(max_bytes as u64 + 1)
        .read_to_end(&mut bytes)
        .map(|read| read > max_bytes)
        .unwrap_or(false);
    if bytes.len() > max_bytes {
        bytes.truncate(max_bytes);
    }
    (String::from_utf8_lossy(&bytes).to_string(), truncated)
}
