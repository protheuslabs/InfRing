use infring_memory_core_v1::{
    CapabilityAction, CapabilityToken, ContextAppendInput, ContextAtomSourceKind,
    DefaultVerityMemoryPolicy, MemoryScope, NexusRouteContext, UnifiedMemoryHeap,
};
use serde_json::{json, Value};
use std::io::{self, Read};

const DEFAULT_BUDGET_TOKENS: u32 = 6000;

fn clean(value: Option<&Value>, max: usize) -> String {
    value
        .and_then(Value::as_str)
        .unwrap_or_default()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max)
        .collect()
}

fn clean_text(value: Option<&Value>, max: usize) -> String {
    value
        .and_then(Value::as_str)
        .unwrap_or_default()
        .replace("\r\n", "\n")
        .trim()
        .chars()
        .take(max)
        .collect()
}

fn estimate_tokens(text: &str) -> u32 {
    ((text.chars().count() as u32).saturating_add(3) / 4).max(1)
}

fn source_kind(value: Option<&Value>, role: &str) -> ContextAtomSourceKind {
    match clean(value, 80).as_str() {
        "tool_result_bundle" | "tool" => ContextAtomSourceKind::ToolResultBundle,
        "status_summary" | "system" => ContextAtomSourceKind::StatusSummary,
        "workflow_boundary" => ContextAtomSourceKind::WorkflowBoundary,
        "external_reference" => ContextAtomSourceKind::ExternalReference,
        "interaction_unit" => ContextAtomSourceKind::InteractionUnit,
        _ if role == "tool" => ContextAtomSourceKind::ToolResultBundle,
        _ if role == "system" => ContextAtomSourceKind::StatusSummary,
        _ => ContextAtomSourceKind::InteractionUnit,
    }
}

fn string_array(value: Option<&Value>, max_items: usize) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| row.split_whitespace().collect::<Vec<_>>().join(" "))
                .filter(|row| !row.is_empty())
                .take(max_items)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn route() -> NexusRouteContext {
    NexusRouteContext {
        issuer: "agent_runtime_context_materializer".to_string(),
        source: "gateway".to_string(),
        target: "memory_heap".to_string(),
        schema_id: "memory.context.topology.agent_runtime".to_string(),
        lease_id: "agent_runtime_context_materializer_lease".to_string(),
        template_version_id: Some("v1".to_string()),
        ttl_ms: Some(30_000),
    }
}

fn token(agent_id: &str, principal_id: &str) -> CapabilityToken {
    CapabilityToken {
        token_id: format!("cap_agent_runtime_context_{}", agent_id),
        principal_id: principal_id.to_string(),
        scopes: vec![MemoryScope::Core, MemoryScope::Agent(agent_id.to_string())],
        allowed_actions: vec![
            CapabilityAction::Read,
            CapabilityAction::Write,
            CapabilityAction::MaterializeContext,
        ],
        expires_at_ms: u64::MAX,
        verity_class: "standard".to_string(),
        receipt_id: "agent_runtime_context_materializer_capability".to_string(),
    }
}

fn role_for(row: &Value) -> String {
    let role = clean(row.get("role").or_else(|| row.get("origin_kind")), 40).to_lowercase();
    match role.as_str() {
        "human" => "user".to_string(),
        "agent" | "ai" => "assistant".to_string(),
        "function" => "tool".to_string(),
        "user" | "assistant" | "tool" | "system" => role,
        _ => "message".to_string(),
    }
}

fn main() {
    let mut raw = String::new();
    if let Err(err) = io::stdin().read_to_string(&mut raw) {
        println!("{}", json!({"ok": false, "error": format!("stdin_read_failed:{err}")}));
        std::process::exit(1);
    }
    let input: Value = match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(err) => {
            println!("{}", json!({"ok": false, "error": format!("json_parse_failed:{err}")}));
            std::process::exit(1);
        }
    };
    let session_id = clean(input.get("session_id"), 200);
    let agent_id = clean(input.get("agent_id"), 160);
    let session_id = if session_id.is_empty() { "session".to_string() } else { session_id };
    let agent_id = if agent_id.is_empty() { "default".to_string() } else { agent_id };
    let principal_id = format!("agent:{agent_id}");
    let budget_tokens = input
        .get("budget_tokens")
        .and_then(Value::as_u64)
        .map(|row| row.min(64_000) as u32)
        .unwrap_or(DEFAULT_BUDGET_TOKENS);
    let rows = input
        .get("atoms")
        .or_else(|| input.pointer("/context_projection/rows"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut heap = UnifiedMemoryHeap::new(DefaultVerityMemoryPolicy);
    let route = route();
    let capability = token(&agent_id, &principal_id);
    for (idx, row) in rows.iter().enumerate() {
        let role = role_for(row);
        let text = clean_text(
            row.get("text_preview")
                .or_else(|| row.get("text"))
                .or_else(|| row.get("message"))
                .or_else(|| row.get("content")),
            1200,
        );
        if text.is_empty() {
            continue;
        }
        let source_ref = clean(
            row.get("source_ref")
                .or_else(|| row.get("detail_ref"))
                .or_else(|| row.get("id")),
            240,
        );
        let source_ref = if source_ref.is_empty() {
            format!("agent-runtime-row-{idx}")
        } else {
            source_ref
        };
        let token_count = row
            .get("token_count")
            .and_then(Value::as_u64)
            .map(|value| value.min(4000) as u32)
            .unwrap_or_else(|| estimate_tokens(&text));
        let _ = heap.append_context_atom(
            &route,
            principal_id.as_str(),
            &capability,
            ContextAppendInput {
                session_id: session_id.clone(),
                source_kind: source_kind(row.get("source_kind"), role.as_str()),
                source_ref,
                text_preview: text,
                token_count,
                task_refs: string_array(row.get("task_refs"), 12),
                memory_version_refs: string_array(row.get("memory_version_refs"), 12),
                semantic_boundary: row
                    .get("semantic_boundary")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                workflow_boundary: row
                    .get("workflow_boundary")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                lineage_refs: string_array(row.get("lineage_refs"), 12),
            },
            vec!["agent_runtime_context_materializer".to_string()],
        );
    }

    match heap.materialize_context_topology(
        &route,
        principal_id.as_str(),
        &capability,
        session_id.as_str(),
        vec![MemoryScope::Core, MemoryScope::Agent(agent_id.clone())],
        budget_tokens,
        string_array(input.get("pinned_anchor_refs"), 16),
        vec!["agent_runtime_context_materializer".to_string()],
    ) {
        Ok(materialized) => println!(
            "{}",
            json!({
                "ok": true,
                "type": "agent_runtime_context_pack",
                "schema_version": 1,
                "source_basis": "core.layer2.memory.context_topology",
                "source_authority": "kernel_materialize_context_topology_cli",
                "session_id": session_id,
                "agent_id": agent_id,
                "fanout_target": 7,
                "hot_tail_count": 4,
                "row_count": rows.len(),
                "frontier": materialized.frontier,
                "fragments": materialized.fragments,
                "budget_report": materialized.budget_report,
                "receipt_refs": heap.receipts().into_iter().map(|row| row.receipt_id.clone()).collect::<Vec<_>>(),
            })
        ),
        Err(err) => {
            println!("{}", json!({"ok": false, "error": err}));
            std::process::exit(1);
        }
    }
}
