fn visible_response_source_for_turn(
    response_text: &str,
    workflow_used: bool,
    visible_response_repaired: bool,
    finalization_outcome: &str,
) -> &'static str {
    if clean_text(response_text, 1_000).is_empty() {
        return "none";
    }
    let outcome = clean_text(finalization_outcome, 1_000).to_ascii_lowercase();
    if visible_response_repaired
        || outcome.contains("repaired_with_initial_draft")
        || outcome.contains("workflow_no_runtime_fallback")
        || outcome.contains("runtime_pending_tool_confirmation_fallback")
    {
        return "llm_draft";
    }
    if outcome.contains("tool_evidence_fallback_used")
        || outcome.contains("tool_evidence_fallback_suppressed")
    {
        return "runtime_tool_evidence_fallback";
    }
    if outcome.contains("empty_visible_response_preserved_without_system_chat")
    {
        return if clean_text(response_text, 1_000).is_empty() {
            "none"
        } else {
            "llm_draft"
        };
    }
    if workflow_used {
        "llm_final"
    } else {
        "llm_draft"
    }
}

fn visible_response_source_is_llm_authored(source: &str) -> bool {
    matches!(source, "llm_final" | "llm_draft")
}

fn visible_response_provenance_contract(response_text: &str, source: &str) -> Value {
    let visible_response_present = !clean_text(response_text, 1_000).is_empty();
    let llm_authored = visible_response_source_is_llm_authored(source);
    json!({
        "contract": "visible_response_provenance_v1",
        "visible_response_present": visible_response_present,
        "visible_response_source": clean_text(source, 80),
        "llm_authored": llm_authored,
        "system_substitution_allowed": false,
        "system_substitution_violation": visible_response_present && !llm_authored
    })
}

fn apply_visible_response_provenance(
    response_workflow: &mut Value,
    response_finalization: &mut Value,
    visible_response_source: &str,
) {
    let source = clean_text(visible_response_source, 80);
    response_workflow["visible_response_source"] = json!(source.clone());
    response_workflow["system_chat_injection_used"] = json!(false);
    response_finalization["visible_response_source"] = json!(source);
    response_finalization["system_chat_injection_used"] = json!(false);
}

fn apply_visible_response_provenance_for_turn(
    response_workflow: &mut Value,
    response_finalization: &mut Value,
    response_text: &str,
    workflow_used: bool,
    visible_response_repaired: bool,
    finalization_outcome: &str,
) -> &'static str {
    let source = visible_response_source_for_turn(
        response_text,
        workflow_used,
        visible_response_repaired,
        finalization_outcome,
    );
    apply_visible_response_provenance(response_workflow, response_finalization, source);
    let provenance_contract = visible_response_provenance_contract(response_text, source);
    response_workflow["visible_response_provenance"] = provenance_contract.clone();
    response_finalization["visible_response_provenance"] = provenance_contract;
    source
}

fn agent_control_plane_health_snapshot_path(root: &Path, agent_id: &str) -> PathBuf {
    root.join("local/state/ops/agent_health_snapshots")
        .join(format!("{}.json", clean_agent_id(agent_id)))
}

fn build_agent_control_plane_health_snapshot(
    agent_id: &str,
    message: &str,
    response_text: &str,
    response_workflow: &Value,
    response_finalization: &Value,
    process_summary: &Value,
    live_eval_monitor: &Value,
) -> Value {
    let current_turn = response_finalization
        .get("current_turn_dominance")
        .cloned()
        .unwrap_or_else(|| current_turn_dominance_payload(message, response_text, &[]));
    let contamination = response_finalization
        .get("contamination_guard")
        .cloned()
        .unwrap_or_else(|| json!({ "detected": false }));
    let provenance_violation = response_finalization
        .pointer("/visible_response_provenance/system_substitution_violation")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let loop_issue = live_eval_monitor
        .get("issues")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter().any(|row| {
                row.get("issue_type")
                    .or_else(|| row.get("reason"))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .contains("repeated")
            })
        })
        .unwrap_or(false)
        || response_workflow_quality_count(response_workflow, "repeated_fallback_loop_detected") > 0;
    let degraded = !current_turn
        .get("dominant")
        .and_then(Value::as_bool)
        .unwrap_or(true)
        || contamination
            .get("detected")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        || provenance_violation
        || loop_issue
        || response_finalization
            .get("system_chat_injection_used")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        || live_eval_monitor
            .get("issue_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            > 0;
    let dashboard_health_indicator = agent_dashboard_health_indicator(
        response_workflow,
        response_finalization,
        process_summary,
        live_eval_monitor,
    );
    json!({
        "contract": "agent_control_plane_health_snapshot_v1",
        "agent_id": clean_agent_id(agent_id),
        "generated_at": crate::now_iso(),
        "status": if degraded { "degraded" } else { "healthy" },
        "request_excerpt": clean_text(message, 240),
        "finalization": {
            "outcome": clean_text(response_finalization.get("outcome").and_then(Value::as_str).unwrap_or(""), 220),
            "visible_response_source": clean_text(response_finalization.get("visible_response_source").and_then(Value::as_str).unwrap_or(""), 80),
            "visible_response_provenance": response_finalization.get("visible_response_provenance").cloned().unwrap_or_else(|| json!({})),
            "system_chat_injection_used": response_finalization.get("system_chat_injection_used").and_then(Value::as_bool).unwrap_or(false),
            "final_ack_only": response_finalization.get("final_ack_only").and_then(Value::as_bool).unwrap_or(false)
        },
        "telemetry": {
            "workflow_visibility_contract": process_summary.pointer("/workflow_visibility/contract").cloned().unwrap_or(Value::Null),
            "live_eval_enabled": live_eval_monitor.get("enabled").and_then(Value::as_bool).unwrap_or(false),
            "live_eval_issue_count": live_eval_monitor.get("issue_count").and_then(Value::as_u64).unwrap_or(0),
            "chat_injection_allowed": live_eval_monitor.get("chat_injection_allowed").and_then(Value::as_bool).unwrap_or(false)
        },
        "loop": {
            "repeated_response_loop_detected": loop_issue
        },
        "tool_gate": process_summary.get("tool_gate").cloned().unwrap_or_else(|| json!({})),
        "contamination": contamination,
        "current_turn_dominance": current_turn,
        "dashboard_health_indicator": dashboard_health_indicator
    })
}

fn persist_agent_control_plane_health_snapshot(root: &Path, agent_id: &str, snapshot: &Value) {
    write_json_pretty(&agent_control_plane_health_snapshot_path(root, agent_id), snapshot);
}

fn persist_agent_control_plane_health_snapshot_for_turn(
    root: &Path,
    agent_id: &str,
    message: &str,
    response_text: &str,
    response_workflow: &Value,
    response_finalization: &Value,
    process_summary: &Value,
    turn_receipt: &Value,
) -> Value {
    let snapshot = build_agent_control_plane_health_snapshot(
        agent_id,
        message,
        response_text,
        response_workflow,
        response_finalization,
        process_summary,
        turn_receipt.get("live_eval_monitor").unwrap_or(&Value::Null),
    );
    persist_agent_control_plane_health_snapshot(root, agent_id, &snapshot);
    snapshot
}

#[cfg(test)]
mod visible_response_provenance_source_tests {
    use super::*;

    #[test]
    fn provenance_marks_no_system_fallback_visible_text_as_llm_draft() {
        let source = visible_response_source_for_turn(
            "Retry later.",
            false,
            false,
            "workflow_llm_unavailable|workflow_no_system_fallback",
        );
        assert_eq!(source, "llm_draft");
        let contract = visible_response_provenance_contract("Retry later.", source);
        assert_eq!(
            contract
                .pointer("/system_substitution_violation")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            contract.pointer("/llm_authored").and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn provenance_keeps_repaired_initial_draft_as_llm_draft_not_final() {
        let source = visible_response_source_for_turn(
            "Draft answer.",
            true,
            true,
            "workflow_authored|repaired_with_initial_draft",
        );
        assert_eq!(source, "llm_draft");
        let contract = visible_response_provenance_contract("Draft answer.", source);
        assert_eq!(
            contract
                .pointer("/system_substitution_violation")
                .and_then(Value::as_bool),
            Some(false)
        );
    }

    #[test]
    fn provenance_marks_pending_tool_confirmation_fallback_as_llm_draft() {
        let source = visible_response_source_for_turn(
            "I can research that on the web. Say `confirm` and I'll run the search.",
            true,
            false,
            "workflow:synthesized|runtime_pending_tool_confirmation_fallback",
        );
        assert_eq!(source, "llm_draft");
    }

    #[test]
    fn provenance_marks_tool_evidence_fallback_as_runtime_substitution() {
        let source = visible_response_source_for_turn(
            "Runtime-composed evidence text.",
            true,
            false,
            "workflow_authored|workflow:tool_evidence_fallback_used",
        );
        assert_eq!(source, "runtime_tool_evidence_fallback");
        let contract = visible_response_provenance_contract("Runtime-composed evidence text.", source);
        assert_eq!(contract.pointer("/llm_authored").and_then(Value::as_bool), Some(false));
        assert_eq!(
            contract
                .pointer("/system_substitution_violation")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn provenance_marks_suppressed_tool_evidence_status_as_runtime_if_visible_text_leaks() {
        let source = visible_response_source_for_turn(
            "Leaked suppressed text.",
            true,
            false,
            "workflow_authored|workflow:tool_evidence_fallback_suppressed",
        );
        assert_eq!(source, "runtime_tool_evidence_fallback");
    }
}
