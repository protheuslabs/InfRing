fn run_turn_workflow_final_response(
    root: &Path,
    provider: &str,
    model: &str,
    active_messages: &[Value],
    message: &str,
    workflow_mode: &str,
    response_tools: &[Value],
    workflow_events: &[Value],
    draft_response: &str,
    latest_assistant_text: &str,
) -> Value {
    let enriched_workflow_events = augment_turn_workflow_events_for_final_response(
        message,
        response_tools,
        workflow_events,
        draft_response,
        latest_assistant_text,
    );
    let mut workflow = turn_workflow_metadata(
        workflow_mode,
        response_tools,
        &enriched_workflow_events,
        draft_response,
        message,
    );
    persist_workflow_compact_source_refs(&mut workflow, response_tools);
    let missing_turn_tool_context_prompt =
        workflow_missing_turn_tool_context_prompt(message, response_tools);
    let missing_turn_tool_context_recovery = !missing_turn_tool_context_prompt.is_empty();
    if response_tools.is_empty()
        && (response_is_manual_toolbox_gate_choice(draft_response)
            || response_is_visible_workflow_gate_choice(draft_response)
            || response_has_gate_choice_prefix_leakage(draft_response))
    {
        record_manual_toolbox_pending_request(&mut workflow, draft_response, message);
        if workflow
            .get("manual_toolbox_pending_tool_request")
            .filter(|value| value.is_object())
            .is_some()
        {
            mark_workflow_pending_gate_without_final_synthesis(
                &mut workflow,
                "skipped_pending_tool_confirmation",
                "manual_toolbox_gate_submission",
                0,
            );
            return finalize_workflow_gate_stability(root, workflow, message);
        }
    }
    let initial_visible_gate_choice_submission = initial_visible_gate_choice_submission_allowed(
        response_tools,
        response_tools.is_empty() && response_is_exact_no_tool_gate_submission(draft_response),
        &workflow,
    );
    let required = workflow
        .pointer("/final_llm_response/required")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || missing_turn_tool_context_recovery
        || initial_visible_gate_choice_submission;
    if !required {
        preserve_direct_llm_response_without_fallback(&mut workflow, draft_response);
        workflow["final_llm_response"]["attempted"] = Value::Bool(false);
        if !workflow_final_response_used(&workflow) {
            workflow["final_llm_response"]["status"] =
                Value::String("skipped_not_required".to_string());
        }
        set_turn_workflow_final_stage_status(&mut workflow, "skipped_not_required");
        return finalize_workflow_gate_stability(root, workflow, message);
    }
    if cfg!(test) && !workflow_test_llm_enabled(root) {
        let _ = (message, latest_assistant_text, response_tools);
        workflow["final_llm_response"]["used"] = Value::Bool(false);
        workflow["final_llm_response"]["runtime_interference_disabled"] = Value::Bool(true);
        workflow["final_llm_response"]["attempted"] = Value::Bool(false);
        workflow["final_llm_response"]["status"] = Value::String("skipped_test".to_string());
        set_turn_workflow_final_stage_status(&mut workflow, "skipped_test");
        return finalize_workflow_gate_stability(root, workflow, message);
    }
    let cleaned_provider = clean_text(provider, 80);
    let cleaned_model = clean_text(model, 240);
    if cleaned_provider.is_empty() || cleaned_model.is_empty() {
        let _ = (message, latest_assistant_text, response_tools);
        workflow["final_llm_response"]["used"] = Value::Bool(false);
        workflow["final_llm_response"]["runtime_interference_disabled"] = Value::Bool(true);
        workflow["final_llm_response"]["attempted"] = Value::Bool(false);
        workflow["final_llm_response"]["status"] =
            Value::String("skipped_missing_model".to_string());
        set_turn_workflow_final_stage_status(&mut workflow, "skipped_missing_model");
        return finalize_workflow_gate_stability(root, workflow, message);
    }
    let tool_rows_json = serde_json::to_string(&tool_rows_for_llm_recovery(response_tools, 6))
        .unwrap_or_else(|_| "[]".to_string());
    let synthesis_input_json =
        serde_json::to_string(workflow.get("synthesis_input").unwrap_or(&Value::Null))
            .unwrap_or_else(|_| "{}".to_string());
    let tool_state_summary = workflow_tool_state_prompt_context(response_tools);
    let answer_unit_synthesis_brief =
        workflow_answer_unit_synthesis_prompt_context(message, response_tools);
    let answer_unit_synthesis_block = if answer_unit_synthesis_brief.is_empty() {
        String::new()
    } else {
        format!("\n\n{answer_unit_synthesis_brief}")
    };
    let missing_turn_tool_context_block = if missing_turn_tool_context_prompt.is_empty() {
        String::new()
    } else {
        format!("\n\n{missing_turn_tool_context_prompt}")
    };
    let template_label = workflow_response_template_label(message);
    let detail_style = "workflow_cd_default";
    let final_answer_instruction = workflow_final_answer_prompt_context_for_tools(response_tools);
    let _workflow_mode_clean = clean_text(workflow_mode, 80);
    let initial_no_tool_category_submission =
        response_tools.is_empty() && response_is_exact_no_tool_gate_submission(draft_response);
    if initial_no_tool_category_submission {
        workflow["workflow_control"]["direct_response_path"] =
            Value::String("first_gate_no_tool_category".to_string());
    }
    let manual_toolbox_gate_turn = response_tools.is_empty()
        && !initial_no_tool_category_submission
        && !initial_visible_gate_choice_submission
        && enriched_workflow_events.iter().any(|event| {
            matches!(
                event.get("kind").and_then(Value::as_str).unwrap_or(""),
                "manual_toolbox_candidate_menu"
            )
        });
    let direct_gate_recovery_turn = response_tools.is_empty()
        && !manual_toolbox_gate_turn
        && (initial_no_tool_category_submission
            || initial_visible_gate_choice_submission
            || enriched_workflow_events.iter().any(|event| {
                event.get("kind").and_then(Value::as_str).unwrap_or("") == "draft_response_invalid"
            }));
    let (system_prompt, user_prompt) = if manual_toolbox_gate_turn {
        (
            clean_text(&workflow_library_prompt_context(message, &[]), 2_000),
            manual_toolbox_gate_context_user_prompt(message),
        )
    } else if direct_gate_recovery_turn {
        let temporal_context = agent_runtime_temporal_context_prompt();
        let direct_gate_system_prompt = final_answer_instruction.clone();
        let project_boundary_prompt = current_turn_project_boundary_prompt(message);
        let direct_gate_system_prompt = if project_boundary_prompt.is_empty() {
            direct_gate_system_prompt.to_string()
        } else {
            format!("{direct_gate_system_prompt} {project_boundary_prompt}")
        };
        let direct_gate_system_prompt = format!("{temporal_context} {direct_gate_system_prompt}");
        let direct_gate_user_prompt = format!(
            "User message:\n{message}\n\n{tool_state_summary}{answer_unit_synthesis_block}{missing_turn_tool_context_block}"
        );
        (
            clean_text(&direct_gate_system_prompt, 2_000),
            clean_text(&direct_gate_user_prompt, 6_000),
        )
    } else {
        (
            clean_text(
                &format!(
                    "{}\n\n{}\n\n{}",
                    AGENT_RUNTIME_SYSTEM_PROMPT,
                    agent_runtime_temporal_context_prompt(),
                    final_answer_instruction
                ),
                12_000,
            ),
            if response_tools.is_empty() {
                clean_text(
                    &format!(
                        "User message:\n{message}\n\n{tool_state_summary}{answer_unit_synthesis_block}{missing_turn_tool_context_block}"
                    ),
                    20_000,
                )
            } else {
                clean_text(
                    &format!(
                        "User message:\n{message}\n\n{tool_state_summary}{answer_unit_synthesis_block}{missing_turn_tool_context_block}\n\nSynthesis input envelope:\n{synthesis_input_json}\n\nRecorded tool outcomes:\n{tool_rows_json}"
                    ),
                    20_000,
                )
            },
        )
    };
    let coherence_window_messages = 2usize;
    let recent_context = active_messages
        .iter()
        .rev()
        .take(coherence_window_messages)
        .filter_map(|row| {
            let text = clean_text(
                row.get("text")
                    .or_else(|| row.get("content"))
                    .or_else(|| row.get("message"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                320,
            );
            if text.is_empty() {
                None
            } else {
                Some(text)
            }
        })
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n");
    let max_attempts: u64 = if manual_toolbox_gate_turn {
        manual_toolbox_private_gate_max_attempts()
    } else if missing_turn_tool_context_recovery {
        2
    } else if !response_tools.is_empty() {
        workflow_final_synthesis_attempt_limit(&workflow, response_tools)
    } else {
        1
    };
    let mut gate_state = WorkflowFinalResponseGateState::default();
    let mut diagnostics = WorkflowFinalResponseDiagnostics::default();
    let has_structured_block_evidence = response_tools.iter().any(|row| {
        let status = clean_text(row.get("status").and_then(Value::as_str).unwrap_or(""), 80)
            .to_ascii_lowercase();
        let error = clean_text(row.get("error").and_then(Value::as_str).unwrap_or(""), 160)
            .to_ascii_lowercase();
        let tool_type = clean_text(row.get("type").and_then(Value::as_str).unwrap_or(""), 120)
            .to_ascii_lowercase();
        let blocked = row.get("blocked").and_then(Value::as_bool).unwrap_or(false);
        blocked
            || matches!(status.as_str(), "blocked" | "policy_denied")
            || tool_type == "tool_pre_gate_blocked"
            || error.contains("nexus_delivery_denied")
            || error.contains("tool_permission_denied")
            || row
                .get("status_code")
                .and_then(Value::as_i64)
                .or_else(|| row.get("http_status").and_then(Value::as_i64))
                .map(|code| matches!(code, 401 | 403 | 404 | 422 | 429))
                .unwrap_or(false)
    });
    workflow["quality_telemetry"] = json!({
        "off_topic_reject": 0,
        "deferred_reply_reject": 0,
        "alignment_reject": 0,
        "prompt_echo_reject": 0,
        "unsourced_claim_reject": 0,
        "direct_answer_reject": 0,
        "unexpected_state_loop_reject": 0,
        "contamination_reject": 0,
        "legacy_retry_template_detected": 0,
        "repeated_fallback_loop_detected": 0,
        "meta_control_tool_block": workflow
            .pointer("/tool_gate/meta_control_message")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            && response_tools.is_empty(),
        "final_fallback_used": false
    });
    if response_contains_unexpected_state_retry_boilerplate(draft_response)
        || response_contains_unexpected_state_retry_boilerplate(latest_assistant_text)
    {
        bump_workflow_quality_counter(&mut workflow, "legacy_retry_template_detected");
    }
    let recent_retry_loop_detected = recent_assistant_retry_loop_detected(active_messages);
    if recent_retry_loop_detected {
        bump_workflow_quality_counter(&mut workflow, "repeated_fallback_loop_detected");
    }
    let gate_only_context_messages = Vec::<Value>::new();
    let final_context_messages = if manual_toolbox_gate_turn || direct_gate_recovery_turn {
        gate_only_context_messages.as_slice()
    } else {
        active_messages
    };
    workflow["final_llm_response"]["attempted"] = Value::Bool(true);
    workflow["final_llm_response"]["max_attempts"] = json!(max_attempts);
    workflow["final_llm_response"]["attempt_budget_source"] = if !response_tools.is_empty()
        && workflow
            .pointer(
                "/selected_workflow/tool_menu_interface_contract/final_synthesis_attempt_limit",
            )
            .is_some()
    {
        Value::String("workflow_cd_final_synthesis_attempt_limit".to_string())
    } else {
        Value::String("runtime_default".to_string())
    };
    workflow["final_llm_response"]["coherence_window_messages"] = json!(coherence_window_messages);
    workflow["final_llm_response"]["synthesis_input_schema_version"] = workflow
        .pointer("/synthesis_input/schema_version")
        .cloned()
        .unwrap_or(Value::Null);
    workflow["final_llm_response"]["synthesis_input_ready"] = Value::Bool(
        workflow
            .get("synthesis_input")
            .and_then(Value::as_object)
            .map(|object| !object.is_empty())
            .unwrap_or(false),
    );
    workflow["gate_trace"] = json!({
        "active": manual_toolbox_gate_turn,
        "attempt_count": 0,
        "max_gate_steps": if manual_toolbox_gate_turn { max_attempts } else { 0 },
        "final_synthesis_attempt_count": 0,
        "authority": "llm_private_gate_submission"
    });
    let attempt_context = WorkflowFinalResponseAttemptContext {
        message,
        response_tools,
        enriched_workflow_events: &enriched_workflow_events,
        final_context_messages,
        cleaned_provider: &cleaned_provider,
        cleaned_model: &cleaned_model,
        system_prompt: &system_prompt,
        user_prompt: &user_prompt,
        final_answer_instruction: &final_answer_instruction,
        tool_state_summary: &tool_state_summary,
        answer_unit_synthesis_block: &answer_unit_synthesis_block,
        missing_turn_tool_context_block: &missing_turn_tool_context_block,
        synthesis_input_json: &synthesis_input_json,
        tool_rows_json: &tool_rows_json,
        recent_context: &recent_context,
        template_label,
        detail_style,
        max_attempts,
        manual_toolbox_gate_turn,
        direct_gate_recovery_turn,
        missing_turn_tool_context_recovery,
        has_structured_block_evidence,
    };
    if run_turn_workflow_final_response_attempts(
        root,
        &mut workflow,
        &attempt_context,
        &mut gate_state,
        &mut diagnostics,
    )
    .should_finalize()
    {
        return finalize_workflow_gate_stability(root, workflow, message);
    }
    finish_turn_workflow_final_response_after_attempts(
        root,
        workflow,
        message,
        latest_assistant_text,
        response_tools,
        manual_toolbox_gate_turn,
        max_attempts,
        recent_retry_loop_detected,
        &gate_state,
        &diagnostics,
    )
}
