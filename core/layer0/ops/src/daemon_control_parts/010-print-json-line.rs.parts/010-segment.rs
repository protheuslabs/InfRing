// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/ops (authoritative)

use crate::deterministic_receipt_hash;
use crate::gateway_supervisor::{self, GatewaySupervisorConfig, GatewaySupervisorResult};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::Path;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, UNIX_EPOCH};

const DASHBOARD_CONNECT_TIMEOUT_MS: u64 = 1_500;
const DASHBOARD_IO_TIMEOUT_MS: u64 = 30_000;
const DASHBOARD_WATCHDOG_HEALTH_IO_TIMEOUT_MS: u64 = 2_000;
const DASHBOARD_HEALTH_MAX_BYTES: usize = 4096;
const DASHBOARD_HEALTH_RETRY_ATTEMPTS: usize = 5;
const DASHBOARD_HEALTH_RETRY_BACKOFF_MS: u64 = 1_000;
const DASHBOARD_WATCHDOG_INTERVAL_DEFAULT_MS: u64 = 2_000;
const DASHBOARD_WATCHDOG_INTERVAL_MIN_MS: u64 = 500;
const DASHBOARD_WATCHDOG_INTERVAL_MAX_MS: u64 = 60_000;
const DASHBOARD_WATCHDOG_STABLE_RETRIES: usize = 2;
const DASHBOARD_WATCHDOG_FAIL_STREAK_THRESHOLD: usize = 6;
const DASHBOARD_WATCHDOG_WEDGE_FAIL_STREAK_THRESHOLD: usize = 2;
const DASHBOARD_PLATFORM_COMMAND_TIMEOUT_MS: u64 = 2_500;
const VERITY_DRIFT_CONFIG_SCHEMA_ID: &str = "infring_verity_drift_policy";
const VERITY_DRIFT_CONFIG_SCHEMA_VERSION: u32 = 1;
const VERITY_DRIFT_CONFIG_POLICY_VERSION: u32 = 1;
const VERITY_DRIFT_MODE_PRODUCTION: &str = "production";
const VERITY_DRIFT_MODE_SIMULATION: &str = "simulation";
const VERITY_DRIFT_PRODUCTION_DEFAULT_MS: i64 = 500;
const VERITY_DRIFT_SIMULATION_DEFAULT_MS: i64 = 30_000;

fn print_json_line(value: &Value) {
    crate::contract_lane_utils::print_json_line(value);
}

#[derive(Debug, Clone)]
struct DashboardHealthSnapshot {
    healthy: bool,
    reason: String,
    listener_reachable: bool,
    timed_out: bool,
    response_complete: bool,
    response_bytes: usize,
    status_code: Option<u16>,
}

impl DashboardHealthSnapshot {
    fn ready(status_code: Option<u16>, response_bytes: usize) -> Self {
        Self {
            healthy: true,
            reason: "dashboard_healthz_ready".to_string(),
            listener_reachable: true,
            timed_out: false,
            response_complete: true,
            response_bytes,
            status_code,
        }
    }

    fn not_ready(
        reason: &str,
        listener_reachable: bool,
        timed_out: bool,
        response_complete: bool,
        response_bytes: usize,
        status_code: Option<u16>,
    ) -> Self {
        Self {
            healthy: false,
            reason: reason.to_string(),
            listener_reachable,
            timed_out,
            response_complete,
            response_bytes,
            status_code,
        }
    }

    fn is_wedged(&self) -> bool {
        self.listener_reachable
            && !self.healthy
            && (self.timed_out
                || !self.response_complete
                || matches!(
                    self.reason.as_str(),
                    "dashboard_healthz_no_response" | "dashboard_healthz_incomplete"
                ))
    }

    fn to_json(&self) -> Value {
        json!({
            "healthy": self.healthy,
            "reason": self.reason.as_str(),
            "listener_reachable": self.listener_reachable,
            "timed_out": self.timed_out,
            "response_complete": self.response_complete,
            "response_bytes": self.response_bytes,
            "status_code": self.status_code,
            "wedged": self.is_wedged(),
        })
    }
}

fn parse_mode(argv: &[String]) -> Option<String> {
    for token in argv {
        if let Some(value) = token.strip_prefix("--mode=") {
            let out = value.trim().to_string();
            if !out.is_empty() {
                return Some(out);
            }
        }
    }
    None
}

fn parse_flag(argv: &[String], key: &str) -> Option<String> {
    let pref = format!("--{key}=");
    let key_token = format!("--{key}");
    let mut idx = 0usize;
    while idx < argv.len() {
        let token = argv[idx].trim();
        if let Some(value) = token.strip_prefix(&pref) {
            let out = value.trim().to_string();
            if !out.is_empty() {
                return Some(out);
            }
        }
        if token == key_token {
            if let Some(next) = argv.get(idx + 1) {
                let out = next.trim().to_string();
                if !out.is_empty() {
                    return Some(out);
                }
            }
        }
        idx += 1;
    }
    None
}

fn parse_bool(raw: Option<&str>, fallback: bool) -> bool {
    match raw.map(|v| v.trim().to_ascii_lowercase()) {
        Some(v) if matches!(v.as_str(), "1" | "true" | "yes" | "on") => true,
        Some(v) if matches!(v.as_str(), "0" | "false" | "no" | "off") => false,
        _ => fallback,
    }
}

fn parse_u16(raw: Option<&str>, fallback: u16) -> u16 {
    raw.and_then(|v| v.trim().parse::<u16>().ok())
        .unwrap_or(fallback)
}

fn parse_u64(raw: Option<&str>, fallback: u64, min: u64, max: u64) -> u64 {
    raw.and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(fallback)
        .clamp(min, max)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VerityDriftSignedConfig {
    schema_id: String,
    schema_version: u32,
    policy_version: u32,
    mode: String,
    production_tolerance_ms: i64,
    simulation_tolerance_ms: i64,
    signature: String,
}

fn resolve_verity_path(root: &Path, env_key: &str, fallback_rel: &str) -> PathBuf {
    let explicit = std::env::var(env_key)
        .ok()
        .map(|raw| raw.trim().to_string())
        .filter(|raw| !raw.is_empty());
    if let Some(raw) = explicit {
        let candidate = PathBuf::from(raw);
        if candidate.is_absolute() {
            return candidate;
        }
        return root.join(candidate);
    }
    root.join(fallback_rel)
}

fn resolve_verity_drift_config_path(root: &Path) -> PathBuf {
    resolve_verity_path(
        root,
        "INFRING_VERITY_DRIFT_CONFIG_PATH",
        "local/state/ops/verity/drift_policy.signed.json",
    )
}

fn resolve_verity_drift_events_path(root: &Path) -> PathBuf {
    resolve_verity_path(
        root,
        "INFRING_VERITY_DRIFT_EVENTS_PATH",
        "local/state/ops/verity/drift_events.jsonl",
    )
}

fn resolve_verity_judicial_lock_path(root: &Path) -> PathBuf {
    resolve_verity_path(
        root,
        "INFRING_VERITY_JUDICIAL_LOCK_PATH",
        "local/state/ops/verity/judicial_lock.json",
    )
}

fn verity_drift_signing_key() -> String {
    std::env::var("INFRING_VERITY_DRIFT_SIGNING_KEY")
        .ok()
        .map(|raw| raw.trim().to_string())
        .filter(|raw| !raw.is_empty())
        .unwrap_or_else(|| "infring-verity-drift-local-key".to_string())
}

fn normalize_verity_mode(raw: &str) -> String {
    let lowered = raw.trim().to_ascii_lowercase();
    if lowered == VERITY_DRIFT_MODE_SIMULATION || lowered == "sim" {
        VERITY_DRIFT_MODE_SIMULATION.to_string()
    } else {
        VERITY_DRIFT_MODE_PRODUCTION.to_string()
    }
}

fn clamp_verity_tolerance_ms(raw: i64, floor: i64, ceil: i64) -> i64 {
    raw.clamp(floor, ceil)
}

fn verity_signature_payload(config: &VerityDriftSignedConfig) -> Value {
    json!({
        "schema_id": config.schema_id,
        "schema_version": config.schema_version,
        "policy_version": config.policy_version,
        "mode": config.mode,
        "production_tolerance_ms": config.production_tolerance_ms,
        "simulation_tolerance_ms": config.simulation_tolerance_ms
    })
}

fn sign_verity_config_payload(payload: &Value) -> String {
    let key = verity_drift_signing_key();
    let digest = crate::deterministic_receipt_hash(&json!({
        "payload": payload,
        "signing_key": key
    }));
    format!("sig:{digest}")
}

fn signed_default_verity_config() -> VerityDriftSignedConfig {
    let mut config = VerityDriftSignedConfig {
        schema_id: VERITY_DRIFT_CONFIG_SCHEMA_ID.to_string(),
        schema_version: VERITY_DRIFT_CONFIG_SCHEMA_VERSION,
        policy_version: VERITY_DRIFT_CONFIG_POLICY_VERSION,
        mode: VERITY_DRIFT_MODE_PRODUCTION.to_string(),
        production_tolerance_ms: VERITY_DRIFT_PRODUCTION_DEFAULT_MS,
        simulation_tolerance_ms: VERITY_DRIFT_SIMULATION_DEFAULT_MS,
        signature: String::new(),
    };
    config.signature = sign_verity_config_payload(&verity_signature_payload(&config));
    config
}

fn write_verity_signed_config(path: &Path, config: &VerityDriftSignedConfig) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let body = serde_json::to_string_pretty(config).unwrap_or_else(|_| "{}".to_string());
    let _ = fs::write(path, body);
}

fn load_verity_signed_config(root: &Path) -> (VerityDriftSignedConfig, bool) {
    let config_path = resolve_verity_drift_config_path(root);
    let default_signed = signed_default_verity_config();
    let raw = match fs::read_to_string(&config_path) {
        Ok(value) => value,
        Err(_) => {
            write_verity_signed_config(&config_path, &default_signed);
            return (default_signed, true);
        }
    };
    let parsed = serde_json::from_str::<VerityDriftSignedConfig>(&raw);
    let mut signed = match parsed {
        Ok(value) => value,
        Err(_) => {
            write_verity_signed_config(&config_path, &default_signed);
            return (default_signed, false);
        }
    };
    signed.mode = normalize_verity_mode(&signed.mode);
    signed.production_tolerance_ms =
        clamp_verity_tolerance_ms(signed.production_tolerance_ms, 1, 60_000);
    signed.simulation_tolerance_ms = clamp_verity_tolerance_ms(
        signed.simulation_tolerance_ms,
        signed.production_tolerance_ms,
        300_000,
    );
    signed.policy_version = signed.policy_version.max(1);
    if signed.schema_id != VERITY_DRIFT_CONFIG_SCHEMA_ID
        || signed.schema_version != VERITY_DRIFT_CONFIG_SCHEMA_VERSION
    {
        signed.schema_id = VERITY_DRIFT_CONFIG_SCHEMA_ID.to_string();
        signed.schema_version = VERITY_DRIFT_CONFIG_SCHEMA_VERSION;
    }
    let expected_signature = sign_verity_config_payload(&verity_signature_payload(&signed));
    let signature_valid = signed.signature.trim() == expected_signature;
    if !signature_valid {
        write_verity_signed_config(&config_path, &default_signed);
        return (default_signed, false);
    }
    if signed.signature.trim() != expected_signature {
        signed.signature = expected_signature;
        write_verity_signed_config(&config_path, &signed);
    }
    (signed, true)
}

fn load_recent_verity_drift_events(root: &Path, limit: usize) -> Vec<Value> {
    let path = resolve_verity_drift_events_path(root);
    let raw = match fs::read_to_string(path) {
        Ok(value) => value,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::<Value>::new();
    for line in raw.lines().rev() {
        if out.len() >= limit {
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(parsed) = serde_json::from_str::<Value>(trimmed) {
            out.push(parsed);
        }
    }
    out.reverse();
    out
}

fn resolve_node_binary() -> String {
    crate::contract_lane_utils::resolve_preferred_node_binary()
}

#[derive(Debug, Clone)]
struct DashboardLaunchConfig {
    enabled: bool,
    open_browser: bool,
    persistent_supervisor: bool,
    host: String,
    port: u16,
    team: String,
    refresh_ms: u64,
    ready_timeout_ms: u64,
    watchdog_interval_ms: u64,
    node_binary: String,
}

impl DashboardLaunchConfig {
    fn url(&self) -> String {
        format!("http://{}:{}/dashboard", self.host, self.port)
    }
}
