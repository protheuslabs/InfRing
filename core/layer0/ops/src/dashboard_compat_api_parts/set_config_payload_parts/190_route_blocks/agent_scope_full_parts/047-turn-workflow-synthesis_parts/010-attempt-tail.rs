fn finish_turn_workflow_final_response_after_attempts(
    root: &Path,
    mut workflow: Value,
    message: &str,
    latest_assistant_text: &str,
    response_tools: &[Value],
    manual_toolbox_gate_turn: bool,
    max_attempts: u64,
    recent_retry_loop_detected: bool,
    gate_state: &WorkflowFinalResponseGateState,
    diagnostics: &WorkflowFinalResponseDiagnostics,
) -> Value {
    if manual_toolbox_gate_turn && response_tools.is_empty() && !diagnostics.last_reject_reason.is_empty() {
        workflow["workflow_control"]["direct_response_path"] = Value::String(
            manual_toolbox_pending_direct_response_path(
                &gate_state.manual_toolbox_selected_category_key,
                &gate_state.manual_toolbox_selected_family_key,
                &gate_state.manual_toolbox_selected_tool_key,
            )
            .to_string(),
        );
        workflow["final_llm_response"]["last_reject_reason"] =
            Value::String(diagnostics.last_reject_reason.clone());
        workflow["final_llm_response"]["error"] = Value::String(diagnostics.last_invalid_excerpt.clone());
        mark_workflow_pending_gate_without_final_synthesis(
            &mut workflow,
            manual_toolbox_pending_stage_status(
                &gate_state.manual_toolbox_selected_category_key,
                &gate_state.manual_toolbox_selected_family_key,
                &gate_state.manual_toolbox_selected_tool_key,
            ),
            "invalid_gate_draft_diagnostic_only",
            max_attempts,
        );
        return finalize_workflow_gate_stability(root, workflow, message);
    }
    if maybe_apply_rejected_tool_evidence_fallback(
        &mut workflow,
        message,
        response_tools,
        &diagnostics.last_invalid_response_text,
        &diagnostics.last_invalid_excerpt,
        &diagnostics.last_reject_reason,
    ) {
        return finalize_workflow_gate_stability(root, workflow, message);
    }
    workflow["final_llm_response"]["used"] = Value::Bool(false);
    if !diagnostics.last_invalid_excerpt.is_empty() {
        workflow["final_llm_response"]["status"] = Value::String("synthesis_failed".to_string());
        set_turn_workflow_final_stage_status(&mut workflow, "synthesis_failed");
        workflow["final_llm_response"]["error"] = Value::String(diagnostics.last_invalid_excerpt.clone());
        if !diagnostics.last_reject_reason.is_empty() {
            workflow["final_llm_response"]["last_reject_reason"] =
                Value::String(diagnostics.last_reject_reason.clone());
        }
    } else {
        workflow["final_llm_response"]["status"] = Value::String("invoke_failed".to_string());
        set_turn_workflow_final_stage_status(&mut workflow, "invoke_failed");
        workflow["final_llm_response"]["error"] = Value::String(diagnostics.last_error.clone());
    }
    if should_record_workflow_failure_diagnostic(
        &diagnostics.last_reject_reason,
        &diagnostics.last_invalid_excerpt,
        latest_assistant_text,
        response_tools,
        recent_retry_loop_detected,
    ) {
        let _ = (message, latest_assistant_text, response_tools);
        workflow["quality_telemetry"]["final_fallback_used"] = Value::Bool(false);
        workflow["final_llm_response"]["used"] = Value::Bool(false);
        workflow["final_llm_response"]["status"] =
            Value::String("diagnostic_failure_pass_through".to_string());
        workflow["final_llm_response"]["runtime_interference_disabled"] = Value::Bool(true);
        workflow["final_llm_response"]["last_reject_reason"] =
            Value::String("synthesis_failure_diagnostic_only".to_string());
        record_workflow_diagnostic_event(
            &mut workflow,
            "synthesis_failure_runtime_fallback_suppressed",
            "synthesis_failure_diagnostic",
        );
        set_turn_workflow_final_stage_status(&mut workflow, "diagnostic_failure_pass_through");
    }
    apply_final_retry_boilerplate_diagnostic(
        &mut workflow,
        message,
        latest_assistant_text,
        response_tools,
    );
    apply_final_empty_response_diagnostic(
        &mut workflow,
        message,
        latest_assistant_text,
        response_tools,
    );
    finalize_workflow_gate_stability(root, workflow, message)
}
