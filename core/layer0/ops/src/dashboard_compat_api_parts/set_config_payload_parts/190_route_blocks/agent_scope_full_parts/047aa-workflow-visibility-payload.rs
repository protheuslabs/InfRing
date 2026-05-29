fn workflow_trace_string(
    value: &Value,
    paths: &[&[&str]],
    default: &str,
    max_len: usize,
) -> String {
    for path in paths {
        let mut cursor = value;
        let mut found = true;
        for segment in *path {
            if let Some(next) = cursor.get(*segment) {
                cursor = next;
            } else {
                found = false;
                break;
            }
        }
        if found {
            let raw = match cursor {
                Value::Bool(true) => "Yes",
                Value::Bool(false) => "No",
                _ => cursor.as_str().unwrap_or(""),
            };
            let cleaned = clean_text(raw, max_len);
            if !cleaned.is_empty() {
                return cleaned;
            }
        }
    }
    clean_text(default, max_len)
}

fn workflow_visibility_trace_payload(
    response_workflow: &Value,
    response_finalization: &Value,
) -> Value {
    let pending = response_finalization
        .get("pending_tool_request")
        .or_else(|| response_workflow.get("pending_tool_request"))
        .or_else(|| response_workflow.get("manual_toolbox_pending_tool_request"));
    let confirmation_state = pending
        .and_then(|row| row.get("status").and_then(Value::as_str))
        .unwrap_or_else(|| {
            if pending.is_some() {
                "pending_confirmation"
            } else {
                "none"
            }
        });
    let mut selected_option = workflow_trace_string(
        response_workflow,
        &[
            &["tool_gate", "gate_submission", "llm_submission"],
            &["tool_gate", "needs_tool_access"],
        ],
        "",
        80,
    );
    if selected_option.is_empty()
        && (response_workflow
            .pointer("/final_llm_response/status")
            .and_then(Value::as_str)
            == Some("skipped_not_required")
            || response_workflow
                .pointer("/workflow_control/direct_response_path")
                .and_then(Value::as_str)
                == Some("first_gate_no_tool_category"))
    {
        selected_option = "Respond directly".to_string();
    }
    json!({
        "gate_id": workflow_trace_string(response_workflow, &[
            &["tool_gate", "gate_submission", "gate_id"],
            &["current_stage"],
        ], "gate_1_work_category_menu", 120),
        "input_kind": workflow_trace_string(response_workflow, &[
            &["tool_gate", "gate_submission", "input_shape", "type"],
            &["tool_gate", "gate_1_question_type"],
        ], "multiple_choice", 80),
        "selected_option": selected_option,
        "tool_name": clean_text(
            pending
                .and_then(|row| row.get("tool_name").or_else(|| row.get("tool")).and_then(Value::as_str))
                .unwrap_or(""),
            120,
        ),
        "confirmation_state": clean_text(confirmation_state, 80),
        "final_authority": "llm_only"
    })
}

fn workflow_finalization_diagnostic_class(
    response_workflow: &Value,
    response_finalization: &Value,
) -> &'static str {
    let outcome = response_finalization
        .get("outcome")
        .and_then(Value::as_str)
        .unwrap_or("");
    let source = response_finalization
        .get("visible_response_source")
        .and_then(Value::as_str)
        .unwrap_or("");
    let final_status = response_workflow
        .pointer("/final_llm_response/status")
        .and_then(Value::as_str)
        .unwrap_or("");
    let pending_tool = response_finalization.get("pending_tool_request").is_some();
    let guard_diagnostic = outcome.contains("final_response_guard_diagnostic_only")
        || outcome.contains("visible_response_contamination_flagged")
        || outcome.contains("unsupported_tool_success_claim_flagged")
        || outcome.contains("current_turn_dominance_flagged")
        || response_finalization
            .pointer("/final_guard_diagnostic_only")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    let guard_withheld = response_finalization
        .pointer("/current_turn_dominance/withheld")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || response_finalization
            .pointer("/contamination_guard/withheld")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    let empty_visible = source == "none" || outcome.contains("empty_");

    if pending_tool && empty_visible {
        "pending_tool_waiting_for_llm_input"
    } else if !empty_visible {
        "visible_llm_response_preserved"
    } else if guard_withheld {
        "guard_withheld_visible_response"
    } else if guard_diagnostic {
        "guard_diagnostic_empty_llm_response"
    } else if final_status.is_empty() || final_status != "synthesized" {
        "llm_finalization_unavailable_no_system_fallback"
    } else {
        "empty_llm_visible_response_no_system_fallback"
    }
}

fn workflow_finalization_diagnostics_payload(
    response_workflow: &Value,
    response_finalization: &Value,
) -> Value {
    let outcome = clean_text(
        response_finalization
            .get("outcome")
            .and_then(Value::as_str)
            .unwrap_or(""),
        220,
    );
    let visible_response_source = clean_text(
        response_finalization
            .get("visible_response_source")
            .and_then(Value::as_str)
            .unwrap_or("none"),
        80,
    );
    let final_llm_status = clean_text(
        response_workflow
            .pointer("/final_llm_response/status")
            .and_then(Value::as_str)
            .unwrap_or("unknown"),
        80,
    );
    let pending_tool_request = response_finalization.get("pending_tool_request").is_some();
    let empty_visible_response =
        visible_response_source == "none" || outcome.contains("empty_final_response");
    let guard_diagnostic_response = outcome.contains("final_response_guard_diagnostic_only")
        || outcome.contains("visible_response_contamination_flagged")
        || outcome.contains("unsupported_tool_success_claim_flagged")
        || outcome.contains("current_turn_dominance_flagged")
        || response_finalization
            .pointer("/final_guard_diagnostic_only")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    let guard_withheld_response = response_finalization
        .pointer("/current_turn_dominance/withheld")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || response_finalization
            .pointer("/contamination_guard/withheld")
            .and_then(Value::as_bool)
            .unwrap_or(false);

    json!({
        "contract": "workflow_finalization_diagnostics_v1",
        "diagnostic_class": workflow_finalization_diagnostic_class(response_workflow, response_finalization),
        "outcome": outcome,
        "final_llm_status": final_llm_status,
        "visible_response_source": visible_response_source,
        "empty_visible_response": empty_visible_response,
        "pending_tool_request": pending_tool_request,
        "guard_withheld_response": guard_withheld_response,
        "guard_diagnostic_response": guard_diagnostic_response,
        "system_chat_injection_used": response_finalization
            .get("system_chat_injection_used")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        "chat_injection_allowed": false,
        "trace_sufficient_for_diagnosis": true
    })
}

fn workflow_visibility_payload(response_workflow: &Value, response_finalization: &Value) -> Value {
    let visibility = response_workflow
        .get("visibility")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let read_visibility = |key: &str, max_len: usize| -> String {
        let value = visibility
            .get(key)
            .or_else(|| response_workflow.get(key))
            .and_then(Value::as_str)
            .unwrap_or("");
        clean_text(value, max_len)
    };
    let current_stage = read_visibility("current_stage", 80);
    let current_stage_status = read_visibility("current_stage_status", 80);
    let ui_status = read_visibility("ui_status", 180);
    let agent_process_status = read_visibility("agent_process_status", 220);
    let debug_status = read_visibility("debug_status", 320);
    let formats = visibility
        .get("formats")
        .filter(|value| value.is_object())
        .cloned()
        .unwrap_or_else(|| {
            json!({
                "ui": ui_status,
                "agent_process": agent_process_status,
                "debug": debug_status
            })
        });
    let visible_response_source = clean_text(
        response_finalization
            .get("visible_response_source")
            .or_else(|| response_workflow.get("visible_response_source"))
            .and_then(Value::as_str)
            .unwrap_or("none"),
        80,
    );
    let system_chat_injection_used = response_finalization
        .get("system_chat_injection_used")
        .or_else(|| response_workflow.get("system_chat_injection_used"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let active_child_workflow_id = clean_text(
        response_workflow
            .pointer("/workflow_control/active_child_workflow_id")
            .or_else(|| {
                response_workflow.pointer("/selected_workflow/active_child_workflow/name")
            })
            .and_then(Value::as_str)
            .unwrap_or(""),
        120,
    );
    let active_child_phase_reason = clean_text(
        response_workflow
            .pointer("/workflow_control/active_child_phase_reason")
            .or_else(|| {
                response_workflow.pointer(
                    "/selected_workflow/active_child_workflow/phase_reason",
                )
            })
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );

    json!({
        "contract": "workflow_visibility_payload_v1",
        "current_stage": current_stage,
        "current_stage_status": current_stage_status,
        "ui_status": ui_status,
        "agent_process_status": agent_process_status,
        "debug_status": debug_status,
        "formats": formats,
        "workflow_trace": workflow_visibility_trace_payload(response_workflow, response_finalization),
        "selected_workflow_id": clean_text(response_workflow.pointer("/selected_workflow/name").and_then(Value::as_str).unwrap_or(""), 80),
        "active_child_workflow_id": active_child_workflow_id,
        "active_child_phase_reason": active_child_phase_reason,
        "stage_count": response_workflow.get("stage_statuses").and_then(Value::as_array).map(|rows| rows.len()).unwrap_or(0),
        "finalization_status": clean_text(response_finalization.get("outcome").and_then(Value::as_str).unwrap_or(""), 180),
        "diagnostics_only": true,
        "final_chat_authority": "llm_only",
        "visible_chat_text_authority": "llm_only",
        "chat_injection_allowed": false,
        "system_injected_chat_text_allowed": false,
        "system_chat_injection_used": system_chat_injection_used,
        "visible_response_source": visible_response_source,
        "finalization_diagnostics": workflow_finalization_diagnostics_payload(response_workflow, response_finalization)
    })
}
