// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/ops (authoritative daemon control contracts).

use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};

pub mod adaptive_contract_version_governance;
pub mod autophagy_auto_approval;
pub mod autoresearch_loop;
pub mod biological_computing_adapter;
pub mod bookmark_knowledge_pipeline;
pub mod collector_cache;
pub mod command_center_session;
pub mod company_layer_orchestration;
pub mod context_doctor;
pub mod contract_lane_utils;
pub mod contribution_oracle;
pub mod decentralized_data_marketplace;
pub mod discord_swarm_orchestration;
pub mod gui_drift_manager;
pub mod hot_path_allocators;
pub mod intel_sweep_router;
pub mod maintainability_maps;
pub mod nexus_internal_comms;
pub mod observability_automation_engine;
pub mod observability_slo_runbook_closure;
pub mod opendev_dual_agent;
pub mod operator_critical_paths;
pub mod ops_lane_runtime;
pub mod p2p_gossip_seed;
pub mod persistent_background_runtime;
pub mod public_api_catalog;
pub mod release_gate_canary_rollback_enforcer;
pub mod retrieval_policy;
pub mod srs_contract_runtime;
pub mod startup_agency_builder;
pub mod system_health;
pub mod timeseries_receipt_engine;
pub mod webgpu_inference_adapter;
pub mod wifi_csi_engine;
pub mod workspace_gateway_runtime;

pub fn deterministic_receipt_hash(payload: &Value) -> String {
    hot_path_allocators::deterministic_hash(payload)
}

pub fn print_json_line(value: &Value) {
    println!(
        "{}",
        serde_json::to_string(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

pub fn parse_cli_flag(argv: &[String], key: &str) -> Option<String> {
    let key_pref = format!("--{key}=");
    let key_exact = format!("--{key}");
    let mut idx = 0usize;
    while idx < argv.len() {
        let token = argv[idx].trim();
        if let Some(value) = token.strip_prefix(&key_pref) {
            return Some(value.to_string());
        }
        if token == key_exact && idx + 1 < argv.len() {
            return Some(argv[idx + 1].clone());
        }
        idx += 1;
    }
    None
}

#[derive(Debug, Clone)]
pub struct ParsedArgs {
    pub positional: Vec<String>,
    pub flags: std::collections::HashMap<String, String>,
}

pub fn parse_args(raw: &[String]) -> ParsedArgs {
    let mut positional = Vec::new();
    let mut flags = std::collections::HashMap::new();
    for token in raw {
        if !token.starts_with("--") {
            positional.push(token.clone());
            continue;
        }
        match token.split_once('=') {
            Some((k, v)) => {
                flags.insert(k.trim_start_matches("--").to_string(), v.to_string());
            }
            None => {
                flags.insert(
                    token.trim_start_matches("--").to_string(),
                    "true".to_string(),
                );
            }
        }
    }
    ParsedArgs { positional, flags }
}

pub fn now_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

pub fn daemon_control_receipt(command: &str, mode: Option<&str>) -> Value {
    let mut out = json!({
        "ok": true,
        "type": "daemon_control_receipt",
        "authority": "core/layer2/ops",
        "command": command,
        "mode": mode
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daemon_receipt_has_hash() {
        let payload = daemon_control_receipt("status", Some("persistent"));
        assert!(payload
            .get("receipt_hash")
            .and_then(Value::as_str)
            .is_some());
    }
}
