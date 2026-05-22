// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::{STANDARD as BASE64_STANDARD, URL_SAFE_NO_PAD};
use base64::Engine;
use chrono::{DateTime, SecondsFormat, TimeZone, Utc};
use hmac::{Hmac, Mac};
use rand::Rng;
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::contract_lane_utils as lane_utils;
use crate::now_iso;

type HmacSha256 = Hmac<Sha256>;

const SECRET_BROKER_POLICY_REL: &str = "core/layer0/ops/config/secret_broker_policy.json";
const LEGACY_SECRET_BROKER_POLICY_REL: &str = "client/runtime/config/secret_broker_policy.json";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SecretBrokerState {
    version: String,
    #[serde(default)]
    issued: BTreeMap<String, SecretHandleStateRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SecretHandleStateRow {
    handle_id: String,
    secret_id: String,
    scope: String,
    caller: String,
    reason: Option<String>,
    issued_at: String,
    expires_at: String,
    value_hash: String,
    backend_provider_type: Option<String>,
    backend_provider_ref: Option<String>,
    rotation_status: Option<String>,
    resolve_count: u64,
    last_resolved_at: Option<String>,
    last_backend_provider_type: Option<String>,
    last_rotation_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct RotationConfig {
    warn_after_days: f64,
    max_after_days: f64,
    require_rotated_at: bool,
    enforce_on_issue: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ProviderConfig {
    Env {
        enabled: bool,
        env: String,
        rotated_at_env: String,
    },
    JsonFile {
        enabled: bool,
        paths: Vec<String>,
        field: String,
        rotated_at_field: String,
    },
    EncryptedFile {
        enabled: bool,
        paths: Vec<String>,
        field: String,
        rotated_at_field: String,
    },
    Command {
        enabled: bool,
        command: CommandSpec,
        parse_json: bool,
        value_path: String,
        rotated_at_path: String,
        timeout_ms: i64,
        env: BTreeMap<String, String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum CommandSpec {
    Shell(String),
    Argv(Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SecretSpec {
    secret_id: String,
    providers: Vec<ProviderConfig>,
    rotation: RotationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SecretBrokerPolicy {
    version: String,
    path: String,
    include_backend_details: bool,
    command_timeout_ms: i64,
    secrets: BTreeMap<String, SecretSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ResolvedBackend {
    provider_type: String,
    provider_ref: Option<String>,
    external: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct RotationHealth {
    status: String,
    reason: String,
    rotated_at: Option<String>,
    age_days: Option<f64>,
    warn_after_days: f64,
    max_after_days: f64,
    require_rotated_at: bool,
    enforce_on_issue: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct LoadedSecret {
    ok: bool,
    secret_id: String,
    value: String,
    value_hash: String,
    backend: Option<ResolvedBackend>,
    rotation: Option<RotationHealth>,
    error: Option<String>,
    provider_errors: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct RotationCheckRow {
    secret_id: String,
    status: String,
    reason: Option<String>,
    available: bool,
    provider_type: Option<String>,
    provider_ref: Option<String>,
    external_backend: Option<bool>,
    rotated_at: Option<String>,
    age_days: Option<f64>,
    warn_after_days: Option<f64>,
    max_after_days: Option<f64>,
    enforce_on_issue: Option<bool>,
    provider_errors: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct RotationHealthReport {
    ok: bool,
    #[serde(rename = "type")]
    report_type: String,
    ts: String,
    policy_path: String,
    policy_version: String,
    total: usize,
    level: String,
    counts: Value,
    checks: Vec<RotationCheckRow>,
}

fn usage() {
    println!("secret-broker-kernel commands:");
    println!("  infring-ops secret-broker-kernel load-policy [--payload-base64=<json>]");
    println!("  infring-ops secret-broker-kernel load-secret --payload-base64=<json>");
    println!("  infring-ops secret-broker-kernel put-secret --payload-base64=<json>");
    println!("  infring-ops secret-broker-kernel rotation-health [--payload-base64=<json>]");
    println!("  infring-ops secret-broker-kernel status [--payload-base64=<json>]");
    println!("  infring-ops secret-broker-kernel issue-handle --payload-base64=<json>");
    println!("  infring-ops secret-broker-kernel resolve-handle --payload-base64=<json>");
}

fn cli_receipt(kind: &str, payload: Value) -> Value {
    crate::contract_lane_utils::cli_receipt(kind, payload)
}

fn cli_error(kind: &str, error: &str) -> Value {
    crate::contract_lane_utils::cli_error(kind, error)
}

fn print_json_line(value: &Value) {
    crate::contract_lane_utils::print_json_line(value);
}

fn payload_json(argv: &[String]) -> Result<Value, String> {
    if let Some(raw) = lane_utils::parse_flag(argv, "payload", false) {
        return serde_json::from_str::<Value>(&raw)
            .map_err(|err| format!("secret_broker_kernel_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD
            .decode(raw_b64.as_bytes())
            .map_err(|err| format!("secret_broker_kernel_payload_base64_decode_failed:{err}"))?;
        let text = String::from_utf8(bytes)
            .map_err(|err| format!("secret_broker_kernel_payload_utf8_decode_failed:{err}"))?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("secret_broker_kernel_payload_decode_failed:{err}"));
    }
    Ok(json!({}))
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    value.as_object().unwrap_or_else(|| {
        static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Map::new)
    })
}

fn text(value: Option<&Value>, max_len: usize) -> String {
    value
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .chars()
        .take(max_len)
        .collect::<String>()
}

fn int_value(value: Option<&Value>) -> Option<i64> {
    value.and_then(|v| {
        if let Some(n) = v.as_i64() {
            return Some(n);
        }
        if let Some(n) = v.as_u64() {
            return i64::try_from(n).ok();
        }
        v.as_str()?.trim().parse::<i64>().ok()
    })
}

fn bool_value(value: Option<&Value>, fallback: bool) -> bool {
    match value {
        Some(Value::Bool(v)) => *v,
        Some(Value::String(v)) => lane_utils::parse_bool(Some(v.as_str()), fallback),
        Some(Value::Number(v)) => v.as_i64().map(|n| n != 0).unwrap_or(fallback),
        _ => fallback,
    }
}

fn number_clamped(value: Option<&Value>, lo: f64, hi: f64, fallback: f64) -> f64 {
    let raw = value.and_then(|v| {
        if let Some(n) = v.as_f64() {
            return Some(n);
        }
        v.as_str()?.trim().parse::<f64>().ok()
    });
    raw.unwrap_or(fallback).clamp(lo, hi)
}

fn runtime_root(root: &Path) -> PathBuf {
    root.join("client").join("runtime")
}

fn default_policy_path(root: &Path) -> PathBuf {
    let canonical = root.join(SECRET_BROKER_POLICY_REL);
    if canonical.exists() {
        return canonical;
    }
    let legacy = root.join(LEGACY_SECRET_BROKER_POLICY_REL);
    if legacy.exists() {
        legacy
    } else {
        canonical
    }
}

fn default_state_path(root: &Path) -> PathBuf {
    runtime_root(root)
        .join("local")
        .join("state")
        .join("security")
        .join("secret_broker_state.json")
}

fn default_audit_path(root: &Path) -> PathBuf {
    runtime_root(root)
        .join("local")
        .join("state")
        .join("security")
        .join("secret_broker_audit.jsonl")
}

fn default_secrets_dir() -> PathBuf {
    if let Ok(raw) = std::env::var("SECRET_BROKER_SECRETS_DIR") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".config")
        .join("infring")
        .join("secrets")
}

fn legacy_local_key_path(root: &Path) -> PathBuf {
    runtime_root(root)
        .join("state")
        .join("security")
        .join("secret_broker_key.txt")
}

fn local_key_path(_root: &Path) -> PathBuf {
    if let Ok(raw) = std::env::var("SECRET_BROKER_LOCAL_KEY_PATH") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    default_secrets_dir().join("secret_broker_key.txt")
}

fn resolve_path(
    root: &Path,
    payload: &Map<String, Value>,
    payload_key: &str,
    env_key: &str,
    default_path: PathBuf,
) -> PathBuf {
    if let Some(raw) = payload.get(payload_key).and_then(Value::as_str) {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            let candidate = PathBuf::from(trimmed);
            return if candidate.is_absolute() {
                candidate
            } else {
                root.join(candidate)
            };
        }
    }
    if let Ok(raw) = std::env::var(env_key) {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            let candidate = PathBuf::from(trimmed);
            return if candidate.is_absolute() {
                candidate
            } else {
                root.join(candidate)
            };
        }
    }
    default_path
}

fn read_text(path: &Path) -> String {
    fs::read_to_string(path)
        .map(|v| v.trim().to_string())
        .unwrap_or_default()
}

fn write_secret(path: &Path, value: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("secret_broker_kernel_create_secret_dir_failed:{err}"))?;
    }
    fs::write(path, format!("{value}\n"))
        .map_err(|err| format!("secret_broker_kernel_write_secret_failed:{err}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

fn generated_key() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

fn secret_broker_key(root: &Path) -> Result<String, String> {
    for env_name in [
        "SECRET_BROKER_KEY",
        "REQUEST_GATE_SECRET",
        "CAPABILITY_LEASE_KEY",
    ] {
        if let Ok(raw) = std::env::var(env_name) {
            let trimmed = raw.trim();
            if !trimmed.is_empty() {
                return Ok(trimmed.to_string());
            }
        }
    }
    let local = local_key_path(root);
    let existing = read_text(&local);
    if !existing.is_empty() {
        return Ok(existing);
    }
    let legacy = legacy_local_key_path(root);
    let legacy_existing = read_text(&legacy);
    if !legacy_existing.is_empty() {
        return Ok(legacy_existing);
    }
    let generated = generated_key();
    write_secret(&local, &generated)?;
    Ok(generated)
}

fn sha16(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))[..16].to_string()
}

fn sign_handle(body: &str, key: &str) -> Result<String, String> {
    let mut mac = <HmacSha256 as Mac>::new_from_slice(key.as_bytes())
        .map_err(|err| format!("hmac_init_failed:{err}"))?;
    mac.update(body.as_bytes());
    Ok(hex::encode(mac.finalize().into_bytes()))
}

fn verify_handle_sig(body: &str, sig_hex: &str, key: &str) -> bool {
    let Ok(sig) = hex::decode(sig_hex) else {
        return false;
    };
    let Ok(mut mac) = <HmacSha256 as Mac>::new_from_slice(key.as_bytes()) else {
        return false;
    };
    mac.update(body.as_bytes());
    mac.verify_slice(&sig).is_ok()
}
