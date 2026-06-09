// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Value};
use std::env;
#[allow(unused_imports)] use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const DEFAULT_LAUNCHD_LABEL: &str = "ai.infring.gateway";
const SUPERVISOR_COMMAND_TIMEOUT_MS: u64 = 2_500;
#[cfg(target_os = "linux")]
const DEFAULT_SYSTEMD_UNIT: &str = "infring-gateway.service";

#[derive(Debug, Clone)]
pub struct GatewaySupervisorConfig {
    pub host: String,
    pub port: u16,
    pub team: String,
    pub refresh_ms: u64,
    pub ready_timeout_ms: u64,
    pub watchdog_interval_ms: u64,
    pub node_binary: String,
}

#[derive(Debug, Clone)]
pub struct GatewaySupervisorResult {
    pub active: bool,
    pub payload: Value,
}

fn trim_text(text: String, max_len: usize) -> String {
    let mut out = text.trim().to_string();
    if out.len() <= max_len {
        return out;
    }
    out.truncate(max_len);
    out
}

fn run_command(program: &str, args: &[String], cwd: Option<&Path>) -> Value {
    run_command_with_timeout(program, args, cwd, SUPERVISOR_COMMAND_TIMEOUT_MS)
}

fn run_command_with_timeout(
    program: &str,
    args: &[String],
    cwd: Option<&Path>,
    timeout_ms: u64,
) -> Value {
    let mut cmd = Command::new(program);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(err) => {
            return json!({
                "ok": false,
                "status": -1,
                "program": program,
                "args": args,
                "error": format!("spawn_failed:{err}"),
            });
        }
    };
    let started = Instant::now();
    let timeout = Duration::from_millis(timeout_ms.clamp(250, 30_000));
    loop {
        match child.try_wait() {
            Ok(Some(_status)) => break,
            Ok(None) => {
                if started.elapsed() >= timeout {
                    let _ = child.kill();
                    let output = child.wait_with_output();
                    return match output {
                        Ok(out) => json!({
                            "ok": false,
                            "status": out.status.code().unwrap_or(-1),
                            "program": program,
                            "args": args,
                            "timed_out": true,
                            "timeout_ms": timeout.as_millis() as u64,
                            "error": "supervisor_command_timeout",
                            "stdout": trim_text(String::from_utf8_lossy(&out.stdout).to_string(), 12000),
                            "stderr": trim_text(String::from_utf8_lossy(&out.stderr).to_string(), 12000),
                        }),
                        Err(err) => json!({
                            "ok": false,
                            "status": -1,
                            "program": program,
                            "args": args,
                            "timed_out": true,
                            "timeout_ms": timeout.as_millis() as u64,
                            "error": format!("supervisor_command_timeout_wait_failed:{err}"),
                        }),
                    };
                }
                thread::sleep(Duration::from_millis(25));
            }
            Err(err) => {
                let _ = child.kill();
                return json!({
                    "ok": false,
                    "status": -1,
                    "program": program,
                    "args": args,
                    "error": format!("wait_failed:{err}"),
                });
            }
        }
    }
    match child.wait_with_output() {
        Ok(out) => json!({
            "ok": out.status.success(),
            "status": out.status.code().unwrap_or(-1),
            "program": program,
            "args": args,
            "stdout": trim_text(String::from_utf8_lossy(&out.stdout).to_string(), 12000),
            "stderr": trim_text(String::from_utf8_lossy(&out.stderr).to_string(), 12000),
        }),
        Err(err) => json!({
            "ok": false,
            "status": -1,
            "program": program,
            "args": args,
            "error": format!("output_failed:{err}"),
        }),
    }
}

fn command_ok(payload: &Value) -> bool {
    payload.get("ok").and_then(Value::as_bool).unwrap_or(false)
}

#[cfg(target_os = "macos")]
fn launchctl(args: &[String]) -> Value {
    run_command("launchctl", args, None)
}

#[cfg(target_os = "linux")]
fn systemctl_user(args: &[String]) -> Value {
    run_command("systemctl", args, None)
}

fn home_dir() -> Option<PathBuf> {
    env::var("HOME")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

#[allow(dead_code)]
fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }
    let mut out = String::from("'");
    for ch in value.chars() {
        if ch == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

fn launchd_label() -> String {
    env::var("INFRING_GATEWAY_LAUNCHD_LABEL")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_LAUNCHD_LABEL.to_string())
}

#[cfg(target_os = "linux")]
fn systemd_unit_name() -> String {
    env::var("INFRING_GATEWAY_SYSTEMD_UNIT")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_SYSTEMD_UNIT.to_string())
}

fn watchdog_args(executable: &Path, cfg: &GatewaySupervisorConfig) -> Vec<String> {
    vec![
        executable.to_string_lossy().to_string(),
        "daemon-control".to_string(),
        "watchdog".to_string(),
        "--dashboard-autoboot=1".to_string(),
        "--gateway-persist=1".to_string(),
        format!("--dashboard-host={}", cfg.host),
        format!("--dashboard-port={}", cfg.port),
        format!("--dashboard-team={}", cfg.team),
        format!("--dashboard-refresh-ms={}", cfg.refresh_ms),
        format!("--dashboard-ready-timeout-ms={}", cfg.ready_timeout_ms),
        format!(
            "--dashboard-watchdog-interval-ms={}",
            cfg.watchdog_interval_ms
        ),
        format!("--node-binary={}", cfg.node_binary),
    ]
}

#[cfg(target_os = "macos")]
fn canonical_gateway_executable(proposed: &Path) -> PathBuf {
    for env_key in ["INFRING_DAEMON_EXPECTED_BINARY", "INFRING_NPM_BINARY", "INFRING_OPS_BINARY"] {
        let explicit = env::var(env_key)
            .ok()
            .map(|raw| raw.trim().to_string())
            .filter(|raw| !raw.is_empty());
        if let Some(raw) = explicit {
            let candidate = PathBuf::from(raw);
            if candidate.is_file() {
                return candidate;
            }
        }
    }
    if let Some(home) = home_dir() {
        let binary_name = if cfg!(windows) {
            "infring-ops.exe"
        } else {
            "infring-ops"
        };
        let candidate = home.join(".local").join("bin").join(binary_name);
        if candidate.is_file() {
            return candidate;
        }
    }
    proposed.to_path_buf()
}

fn unsupported_payload(action: &str, reason: &str) -> GatewaySupervisorResult {
    GatewaySupervisorResult {
        active: false,
        payload: json!({
            "ok": false,
            "platform": std::env::consts::OS,
            "action": action,
            "error": reason,
        }),
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn current_uid() -> Option<String> {
    if let Ok(raw) = env::var("UID") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() && trimmed.chars().all(|ch| ch.is_ascii_digit()) {
            return Some(trimmed.to_string());
        }
    }
    let out = run_command("id", &[String::from("-u")], None);
    out.get("stdout")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty() && value.chars().all(|ch| ch.is_ascii_digit()))
        .map(ToString::to_string)
}

#[cfg(target_os = "macos")]
fn launchd_paths() -> Option<(String, String, PathBuf)> {
    let uid = current_uid()?;
    let label = launchd_label();
    let plist = home_dir()?
        .join("Library")
        .join("LaunchAgents")
        .join(format!("{label}.plist"));
    Some((uid, label, plist))
}

#[cfg(target_os = "macos")]
fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(target_os = "macos")]
fn launchd_env_path() -> String {
    let mut entries = vec![
        "/usr/local/bin".to_string(),
        "/opt/homebrew/bin".to_string(),
        "/usr/bin".to_string(),
        "/bin".to_string(),
        "/usr/sbin".to_string(),
        "/sbin".to_string(),
    ];
    if let Some(home) = home_dir() {
        let home_text = home.to_string_lossy().to_string();
        entries.insert(0, format!("{home_text}/.local/bin"));
        entries.insert(1, format!("{home_text}/.cargo/bin"));
    }
    let mut dedup = Vec::<String>::new();
    for entry in entries {
        let clean = entry.trim().to_string();
        if clean.is_empty() || dedup.iter().any(|row| row == &clean) {
            continue;
        }
        dedup.push(clean);
    }
    dedup.join(":")
}

#[cfg(target_os = "macos")]
fn refresh_codesign_signature(executable: &Path) -> Value {
    let executable_str = executable.to_string_lossy().to_string();
    let verify_args = vec![
        "--verify".to_string(),
        "--verbose=2".to_string(),
        executable_str.clone(),
    ];
    let verify_before = run_command("codesign", &verify_args, None);
    let resign = run_command(
        "codesign",
        &[
            "--force".to_string(),
            "--sign".to_string(),
            "-".to_string(),
            executable_str.clone(),
        ],
        None,
    );
    let verify_after = run_command("codesign", &verify_args, None);
    json!({
        "ok": command_ok(&verify_after),
        "executable": executable_str,
        "verify_before": verify_before,
        "resign": resign,
        "verify_after": verify_after,
    })
}
