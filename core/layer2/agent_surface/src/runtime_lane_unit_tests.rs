// Layer ownership: Core Layer 2 (Scheduling + Execution) - agent runtime surface coordination.
use crate::runtime_lane::{run_runtime_lane, RuntimeLaneRequest};
use serde_json::{json, Value};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn runtime_lane_executes_with_capability_pack_defaults() {
    let response = run_runtime_lane(RuntimeLaneRequest {
        name: "lane-agent".to_string(),
        preamble: Some("You are concise.".to_string()),
        initial_prompt: "Summarize system status in one line.".to_string(),
        provider: Some("local-echo".to_string()),
        model: None,
        tools: vec!["web.search".to_string()],
        capability_packs: vec!["research".to_string()],
        lifespan_seconds: Some(120),
        metadata: json!({"lane":"runtime"}),
        permissions_manifest: Some(json!({
            "grants": {
                "voice.realtime": 1,
                "memory.read": 1,
                "web.search": 1
            }
        })),
        wasm_sandbox: Some(json!({
            "enabled": true,
            "allow_network": true,
            "allowed_modules": ["planner.module"]
        })),
        voice_session: Some(json!({
            "transport": "webrtc",
            "provider": "realtime",
            "model": "gpt-realtime"
        })),
        receipt_merkle: Some(json!({
            "enabled": true,
            "seed": "wave4"
        })),
        previous_receipt_root: Some("root0".to_string()),
        schedule_interval_seconds: Some(120),
        schedule_max_runs: Some(12),
    })
    .expect("runtime lane");
    assert!(response.ok);
    assert_eq!(
        response
            .receipt
            .get("type")
            .and_then(serde_json::Value::as_str),
        Some("agent_run_receipt")
    );
    assert!(response
        .contract
        .get("receipt_merkle")
        .and_then(|value| value.get("root"))
        .and_then(Value::as_str)
        .is_some());
}

#[test]
fn runtime_lane_fail_closes_when_memory_write_permission_is_not_allowed() {
    let response = run_runtime_lane(RuntimeLaneRequest {
        name: "lane-agent".to_string(),
        preamble: Some("You are concise.".to_string()),
        initial_prompt: "Try mutation".to_string(),
        provider: Some("local-echo".to_string()),
        model: None,
        tools: vec!["memory.write".to_string()],
        capability_packs: vec![],
        lifespan_seconds: Some(120),
        metadata: json!({"lane":"runtime"}),
        permissions_manifest: Some(json!({
            "grants": {
                "memory.write": 0
            }
        })),
        wasm_sandbox: None,
        voice_session: None,
        receipt_merkle: None,
        previous_receipt_root: None,
        schedule_interval_seconds: None,
        schedule_max_runs: None,
    })
    .expect("runtime lane");
    assert!(!response.ok);
    assert_eq!(
        response.error.as_deref(),
        Some("runtime_lane_memory_write_denied")
    );
    let contract = response.contract.to_string();
    assert!(contract.contains("\"enforcement_mode\":\"strict_fail_closed\""));
    assert!(contract.contains("\"blocked_permission_key_lineage\""));
    assert!(contract.contains("\"parent_permissions_patch_clamped\""));
}

#[test]
fn runtime_lane_direct_mutation_skips_provider_for_explicit_file_content() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_millis();
    let root = std::env::temp_dir().join(format!("infring-tier0-direct-mutation-{stamp}"));
    fs::create_dir_all(&root).expect("create temp root");
    let target = root.join("src/example.py");
    let prompt = format!(
        "Project root: {}\nCreate `src/example.py` with this content:\n```python\nprint(\"tier0\")\n```",
        root.display()
    );

    let response = run_runtime_lane(RuntimeLaneRequest {
        name: "tier0-agent".to_string(),
        preamble: None,
        initial_prompt: prompt,
        provider: Some("provider-that-should-not-be-called".to_string()),
        model: None,
        tools: vec![],
        capability_packs: vec!["local-coding-files".to_string()],
        lifespan_seconds: Some(120),
        metadata: json!({"lane":"runtime"}),
        permissions_manifest: Some(json!({
            "grants": {
                "file.read": 1,
                "file.write": 1,
                "file.patch": 1,
                "command.run": 1
            }
        })),
        wasm_sandbox: None,
        voice_session: None,
        receipt_merkle: None,
        previous_receipt_root: None,
        schedule_interval_seconds: None,
        schedule_max_runs: None,
    })
    .expect("runtime lane");

    assert!(response.ok);
    assert_eq!(
        fs::read_to_string(target).expect("read target"),
        "print(\"tier0\")\n"
    );
    assert_eq!(
        response
            .contract
            .get("execution_shape")
            .and_then(|shape| shape.get("lane"))
            .and_then(Value::as_str),
        Some("direct_mutation")
    );
    assert_eq!(
        response
            .trace_summary
            .get("phase_latency_ms")
            .and_then(|value| value.get("model_call"))
            .and_then(Value::as_u64),
        Some(0)
    );
    assert!(response
        .output
        .contains("Receipt: single_mutation_execution_1"));
}

#[test]
fn runtime_lane_deterministic_local_loop_executes_manifest_without_provider() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_millis();
    let root = std::env::temp_dir().join(format!("infring-tier1-local-loop-{stamp}"));
    fs::create_dir_all(&root).expect("create temp root");
    let prompt = format!(
        "Run this deterministic local coding action manifest without provider startup:\n```json\n{{\n  \"deterministic_local_loop\": {{\n    \"workspace_root\": \"{}\",\n    \"actions\": [\n      {{\"type\": \"write_file\", \"path\": \"src/app.py\", \"content\": \"def app():\\n    return 'ok'\\n\"}},\n      {{\"type\": \"write_file\", \"path\": \"tests/test_app.py\", \"content\": \"from src.app import app\\n\\ndef test_app():\\n    assert app() == 'ok'\\n\"}}\n    ]\n  }}\n}}\n```",
        root.display()
    );

    let response = run_runtime_lane(RuntimeLaneRequest {
        name: "tier1-agent".to_string(),
        preamble: None,
        initial_prompt: prompt,
        provider: Some("provider-that-should-not-be-called".to_string()),
        model: None,
        tools: vec![],
        capability_packs: vec!["local-coding-files".to_string()],
        lifespan_seconds: Some(120),
        metadata: json!({"lane":"runtime"}),
        permissions_manifest: Some(json!({
            "grants": {
                "file.read": 1,
                "file.write": 1,
                "file.patch": 1,
                "command.run": 1
            }
        })),
        wasm_sandbox: None,
        voice_session: None,
        receipt_merkle: None,
        previous_receipt_root: None,
        schedule_interval_seconds: None,
        schedule_max_runs: None,
    })
    .expect("runtime lane");

    assert!(response.ok);
    assert_eq!(
        fs::read_to_string(root.join("src/app.py")).expect("read app"),
        "def app():\n    return 'ok'\n"
    );
    assert_eq!(
        fs::read_to_string(root.join("tests/test_app.py")).expect("read test"),
        "from src.app import app\n\ndef test_app():\n    assert app() == 'ok'\n"
    );
    assert_eq!(
        response
            .contract
            .get("execution_shape")
            .and_then(|shape| shape.get("lane"))
            .and_then(Value::as_str),
        Some("deterministic_local_loop")
    );
    assert_eq!(
        response
            .trace_summary
            .get("phase_latency_ms")
            .and_then(|value| value.get("model_call"))
            .and_then(Value::as_u64),
        Some(0)
    );
    assert_eq!(
        response
            .receipt
            .get("native_tool_call_count")
            .and_then(Value::as_u64),
        Some(2)
    );
}
