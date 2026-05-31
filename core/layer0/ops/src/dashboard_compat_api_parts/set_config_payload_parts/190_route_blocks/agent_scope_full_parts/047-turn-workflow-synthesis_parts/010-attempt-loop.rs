fn run_turn_workflow_final_response_attempts(
    root: &Path,
    workflow: &mut Value,
    ctx: &WorkflowFinalResponseAttemptContext<'_>,
    gate_state: &mut WorkflowFinalResponseGateState,
    diagnostics: &mut WorkflowFinalResponseDiagnostics,
) -> WorkflowFinalResponseAttemptResult {
    let message = ctx.message;
    let response_tools = ctx.response_tools;
    let enriched_workflow_events = ctx.enriched_workflow_events;
    let final_context_messages = ctx.final_context_messages;
    let cleaned_provider = ctx.cleaned_provider;
    let cleaned_model = ctx.cleaned_model;
    let system_prompt = ctx.system_prompt;
    let user_prompt = ctx.user_prompt;
    let final_answer_instruction = ctx.final_answer_instruction;
    let tool_state_summary = ctx.tool_state_summary;
    let answer_unit_synthesis_block = ctx.answer_unit_synthesis_block;
    let missing_turn_tool_context_block = ctx.missing_turn_tool_context_block;
    let synthesis_input_json = ctx.synthesis_input_json;
    let tool_rows_json = ctx.tool_rows_json;
    let recent_context = ctx.recent_context;
    let template_label = ctx.template_label;
    let detail_style = ctx.detail_style;
    let max_attempts = ctx.max_attempts;
    let manual_toolbox_gate_turn = ctx.manual_toolbox_gate_turn;
    let direct_gate_recovery_turn = ctx.direct_gate_recovery_turn;
    let missing_turn_tool_context_recovery = ctx.missing_turn_tool_context_recovery;
    let has_structured_block_evidence = ctx.has_structured_block_evidence;
    for attempt in 1..=max_attempts {
        let current_manual_toolbox_gate_id = manual_toolbox_active_gate_id(
            &gate_state.manual_toolbox_selected_category_key,
            &gate_state.manual_toolbox_selected_family_key,
            &gate_state.manual_toolbox_selected_tool_key,
        );
        if manual_toolbox_gate_turn {
            workflow["gate_trace"]["attempt_count"] = json!(attempt);
            workflow["gate_trace"]["current_step"] =
                Value::String(current_manual_toolbox_gate_id.to_string());
        } else {
            workflow["gate_trace"]["final_synthesis_attempt_count"] = json!(attempt);
        }
        let active_manual_toolbox_category_turn = manual_toolbox_gate_turn
            && !gate_state.manual_toolbox_no_selected
            && gate_state.manual_toolbox_selected_category_key.is_empty();
        let active_manual_toolbox_family_turn = manual_toolbox_gate_turn
            && !gate_state.manual_toolbox_no_selected
            && !gate_state.manual_toolbox_selected_category_key.is_empty()
            && gate_state.manual_toolbox_selected_family_key.is_empty();
        let active_manual_toolbox_tool_turn = manual_toolbox_gate_turn
            && !gate_state.manual_toolbox_no_selected
            && !gate_state.manual_toolbox_selected_category_key.is_empty()
            && !gate_state.manual_toolbox_selected_family_key.is_empty()
            && gate_state.manual_toolbox_selected_tool_key.is_empty();
        let active_manual_toolbox_payload_turn = manual_toolbox_gate_turn
            && !gate_state.manual_toolbox_no_selected
            && !gate_state.manual_toolbox_selected_category_key.is_empty()
            && !gate_state.manual_toolbox_selected_family_key.is_empty()
            && !gate_state.manual_toolbox_selected_tool_key.is_empty();
        let active_manual_toolbox_private_gate_turn = active_manual_toolbox_category_turn
            || active_manual_toolbox_family_turn
            || active_manual_toolbox_tool_turn
            || active_manual_toolbox_payload_turn;
        if !active_manual_toolbox_private_gate_turn {
            diagnostics.synthesis_attempt_count += 1;
        }
        workflow["final_llm_response"]["attempt_count"] = json!(diagnostics.synthesis_attempt_count.max(1));
        let compact_tool_retry = attempt > 1 && !response_tools.is_empty();
        let attempt_system_prompt = if active_manual_toolbox_category_turn {
            system_prompt.to_string()
        } else if active_manual_toolbox_family_turn {
            workflow_tool_family_prompt_context(
                &gate_state.manual_toolbox_selected_category_key,
                &gate_state.manual_toolbox_selected_category_label,
            )
        } else if active_manual_toolbox_tool_turn {
            workflow_tool_selection_prompt_context(
                &gate_state.manual_toolbox_selected_family_key,
                &gate_state.manual_toolbox_selected_family_label,
            )
        } else if active_manual_toolbox_payload_turn {
            workflow_tool_payload_prompt_context(
                &gate_state.manual_toolbox_selected_family_key,
                &gate_state.manual_toolbox_selected_tool_key,
                &gate_state.manual_toolbox_selected_tool_label,
            )
        } else if gate_state.manual_toolbox_no_selected || compact_tool_retry {
            clean_text(&final_answer_instruction, 2_000)
        } else {
            system_prompt.to_string()
        };
        let gate_context_user_prompt =
            clean_text(&manual_toolbox_gate_context_user_prompt(message), 8_000);
        let gate_retry_guidance = if active_manual_toolbox_private_gate_turn
            && attempt > 1
            && (!diagnostics.last_invalid_excerpt.is_empty() || !diagnostics.last_reject_reason.is_empty())
        {
            workflow_private_gate_retry_prompt_context(
                current_manual_toolbox_gate_id,
                message,
                &diagnostics.last_reject_reason,
                &diagnostics.last_invalid_excerpt,
            )
        } else {
            String::new()
        };
        let final_synthesis_retry_guidance = if !active_manual_toolbox_private_gate_turn
            && attempt > 1
            && !diagnostics.last_reject_reason.is_empty()
        {
            workflow_final_synthesis_retry_prompt_context(
                &diagnostics.last_reject_reason,
                &diagnostics.last_invalid_excerpt,
            )
        } else {
            String::new()
        };
        let attempt_user_prompt = if active_manual_toolbox_category_turn {
            gate_context_user_prompt.to_string()
        } else if !gate_retry_guidance.is_empty() {
            gate_retry_guidance
        } else if active_manual_toolbox_family_turn
            || active_manual_toolbox_tool_turn
            || active_manual_toolbox_payload_turn
        {
            gate_context_user_prompt
        } else if gate_state.manual_toolbox_no_selected {
            clean_text(
                &format!(
                    "User message:\n{message}\n\n{tool_state_summary}{answer_unit_synthesis_block}{missing_turn_tool_context_block}"
                ),
                8_000,
            )
        } else if compact_tool_retry {
            let retry_guidance_block = if final_synthesis_retry_guidance.is_empty() {
                String::new()
            } else {
                format!("{final_synthesis_retry_guidance}\n\n")
            };
            clean_text(
                &format!(
                    "User message:\n{message}\n\n{retry_guidance_block}{tool_state_summary}{answer_unit_synthesis_block}{missing_turn_tool_context_block}\n\nSynthesis input envelope:\n{synthesis_input_json}\n\nRecorded tool outcomes:\n{tool_rows_json}"
                ),
                8_000,
            )
        } else if attempt > 1 {
            let retry_guidance_block = if final_synthesis_retry_guidance.is_empty() {
                String::new()
            } else {
                format!("{final_synthesis_retry_guidance}\n\n")
            };
            clean_text(
                &format!("{user_prompt}\n\n{retry_guidance_block}{final_answer_instruction}"),
                20_000,
            )
        } else {
            user_prompt.to_string()
        };
        let attempt_provider = cleaned_provider.to_string();
        let attempt_model = cleaned_model.to_string();
        workflow["final_llm_response"]["current_attempt"] = json!({
            "attempt": attempt,
            "provider": attempt_provider,
            "model": attempt_model,
            "recovery_attempt": false,
            "tool_state_summary": tool_state_summary.clone()
        });
        match crate::dashboard_provider_runtime::invoke_chat(
            root,
            &attempt_provider,
            &attempt_model,
            &attempt_system_prompt,
            final_context_messages,
            &attempt_user_prompt,
        ) {
            Ok(retried) => {
                let mut retried_text = clean_chat_text(
                    retried
                        .get("response")
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                    32_000,
                );
                // Private gate turns (gate_1 through gate_4) are never user-visible; skip
                // sanitization so structured gate JSON is preserved between internal stages.
                if !active_manual_toolbox_private_gate_turn {
                    retried_text = workflow_final_visible_response_text(&retried_text);
                    if !user_requested_internal_runtime_details(message) {
                        retried_text = abstract_runtime_mechanics_terms(&retried_text);
                    }
                }
                let repaired_missing_turn_tool_context = missing_turn_tool_context_recovery
                    && !workflow_missing_turn_tool_context_response_contract_satisfied(
                        &retried_text,
                    );
                if repaired_missing_turn_tool_context {
                    retried_text = workflow_missing_turn_tool_context_repaired_response(
                        message,
                        response_tools,
                        &retried_text,
                    );
                }
                if let Some(gate_outcome) = handle_manual_toolbox_private_gate_turn(
                    workflow,
                    message,
                    response_tools,
                    attempt,
                    &attempt_provider,
                    &attempt_model,
                    &retried,
                    &retried_text,
                    active_manual_toolbox_category_turn,
                    active_manual_toolbox_family_turn,
                    active_manual_toolbox_tool_turn,
                    active_manual_toolbox_payload_turn,
                    &mut gate_state.manual_toolbox_no_selected,
                    &mut gate_state.manual_toolbox_selected_category_key,
                    &mut gate_state.manual_toolbox_selected_category_label,
                    &mut gate_state.manual_toolbox_selected_family_key,
                    &mut gate_state.manual_toolbox_selected_family_label,
                    &mut gate_state.manual_toolbox_selected_tool_key,
                    &mut gate_state.manual_toolbox_selected_tool_label,
                    &mut diagnostics.last_invalid_excerpt,
                    &mut diagnostics.last_reject_reason,
                ) {
                    match gate_outcome {
                        ManualToolboxPrivateGateOutcome::Continue => continue,
                        ManualToolboxPrivateGateOutcome::Finalize => {
                            return WorkflowFinalResponseAttemptResult::Finalize;
                        }
                    }
                }
                let visible_gate_choice_reply =
                    response_is_visible_workflow_gate_choice(&retried_text)
                        || response_has_gate_choice_prefix_leakage(&retried_text);
                let recorded_tool_result_answer =
                    response_answers_tool_confirmation_with_recorded_result(
                        &retried_text,
                        response_tools,
                    ) || response_answers_successful_tool_result(
                        message,
                        &retried_text,
                        response_tools,
                    );
                let deferred_reply = !recorded_tool_result_answer
                    && (response_is_deferred_execution_preamble(&retried_text)
                        || response_is_deferred_retry_prompt(&retried_text)
                        || workflow_response_requests_more_tooling(&retried_text));
                let off_topic_reply = response_is_unrelated_context_dump(message, &retried_text);
                let stale_code_context_reply =
                    response_contains_stale_code_context_dump(message, &retried_text);
                let low_alignment_reply = !recorded_tool_result_answer
                    && response_low_alignment_with_turn_context(
                        message,
                        &recent_context,
                        &retried_text,
                    );
                let prompt_scaffold_reply = response_contains_prompt_scaffold(&retried_text);
                let prompt_echo_reply = prompt_scaffold_reply
                    || if direct_gate_recovery_turn
                        && !clean_text(message, 240)
                            .eq_ignore_ascii_case(&clean_text(&retried_text, 240))
                    {
                        false
                    } else {
                        response_prompt_echo_detected(message, &retried_text)
                    };
                let receipt_mapped_sources = response_tools
                    .iter()
                    .any(|row| !response_tool_receipt_id(row).is_empty());
                let missing_evidence_tags = !response_tools.is_empty()
                    && !receipt_mapped_sources
                    && !response_has_evidence_tags(&retried_text);
                let missing_direct_answer = !recorded_tool_result_answer
                    && !direct_gate_recovery_response_answers_user(
                        message,
                        &retried_text,
                        direct_gate_recovery_turn,
                    );
                let direct_answer_in_first_two_sentences = !missing_direct_answer;
                let rejects_base_contract = !recorded_tool_result_answer
                    && response_fails_base_final_answer_contract(&retried_text);
                let rejects_speculative_blocker =
                    response_contains_speculative_web_blocker_language(&retried_text)
                        && !has_structured_block_evidence;
                let unsupported_tool_success_claim =
                    response_claims_tool_success_without_current_turn_evidence(
                        message,
                        &retried_text,
                        response_tools,
                    );
                let final_verifier_contract_violation_reason =
                    tool_backed_final_verifier_violation_reason(&retried_text, response_tools)
                        .unwrap_or_default();
                let final_verifier_contract_violation =
                    !final_verifier_contract_violation_reason.is_empty();
                let missing_turn_tool_context_reply = missing_turn_tool_context_recovery
                    && !workflow_missing_turn_tool_context_response_contract_satisfied(
                        &retried_text,
                    );
                let raw_tool_payload_dump =
                    response_looks_like_raw_tool_payload_dump(&retried_text);
                let prompt_analysis_leak =
                    response_contains_workflow_prompt_analysis_leak_for_message(
                        message,
                        &retried_text,
                    );
                let reject_checks = [
                    (
                        visible_gate_choice_reply,
                        "visible_gate_choice_reply",
                        "alignment_reject",
                    ),
                    (
                        prompt_analysis_leak,
                        "workflow_prompt_analysis_leak",
                        "contamination_reject",
                    ),
                    (deferred_reply, "deferred_reply", "deferred_reply_reject"),
                    (off_topic_reply, "off_topic_reply", "off_topic_reject"),
                    (
                        stale_code_context_reply,
                        "stale_code_context_dump",
                        "contamination_reject",
                    ),
                    (
                        low_alignment_reply,
                        "low_alignment_reply",
                        "alignment_reject",
                    ),
                    (prompt_echo_reply, "prompt_echo_reply", "prompt_echo_reject"),
                    (
                        missing_direct_answer,
                        "missing_direct_answer_reply",
                        "direct_answer_reject",
                    ),
                    (retried_text.is_empty(), "empty_reply", ""),
                    (
                        response_is_no_findings_placeholder(&retried_text)
                            && !recorded_tool_result_answer,
                        "placeholder_reply",
                        "",
                    ),
                    (
                        response_contains_unexpected_state_retry_boilerplate(&retried_text),
                        "unexpected_state_retry_boilerplate",
                        "unexpected_state_loop_reject",
                    ),
                    (
                        unsupported_tool_success_claim,
                        "unsupported_tool_success_claim",
                        "unsupported_tool_success_claim_reject",
                    ),
                    (
                        final_verifier_contract_violation,
                        "final_response_verifier_contract",
                        "alignment_reject",
                    ),
                    (
                        missing_turn_tool_context_reply,
                        "missing_turn_tool_context_reply",
                        "direct_answer_reject",
                    ),
                    (
                        raw_tool_payload_dump,
                        "raw_tool_payload_dump",
                        "contamination_reject",
                    ),
                    (
                        response_looks_like_tool_ack_without_findings(&retried_text)
                            && !recorded_tool_result_answer,
                        "ack_only_reply",
                        "",
                    ),
                    (
                        rejects_speculative_blocker || rejects_base_contract,
                        "invalid_reply",
                        "",
                    ),
                ];
                let (reject_reason, reject_counter) = reject_checks
                    .into_iter()
                    .find(|(should_reject, _, _)| *should_reject)
                    .map(|(_, reason, counter)| (reason, counter))
                    .unwrap_or(("", ""));
                if !reject_reason.is_empty() {
                    if !reject_counter.is_empty() {
                        bump_workflow_quality_counter(workflow, reject_counter);
                    }
                    diagnostics.last_reject_reason = if reject_reason == "final_response_verifier_contract"
                        && !final_verifier_contract_violation_reason.is_empty()
                    {
                        final_verifier_contract_violation_reason.clone()
                    } else {
                        reject_reason.to_string()
                    };
                    diagnostics.last_invalid_response_text = retried_text.clone();
                    diagnostics.last_invalid_excerpt = first_sentence(&retried_text, 240);
                    workflow["final_llm_response"]["runtime_interference_disabled"] =
                        Value::Bool(true);
                    workflow["final_llm_response"]["diagnostic_reject_reason"] =
                        Value::String(diagnostics.last_reject_reason.clone());
                    workflow["final_llm_response"]["diagnostic_invalid_excerpt"] =
                        Value::String(diagnostics.last_invalid_excerpt.clone());
                    if attempt < max_attempts {
                        continue;
                    }
                    break;
                }
                let response_provider = clean_text(
                    retried
                        .get("provider")
                        .and_then(Value::as_str)
                        .unwrap_or(&attempt_provider),
                    80,
                );
                let response_model = clean_text(
                    retried
                        .get("runtime_model")
                        .or_else(|| retried.get("model"))
                        .and_then(Value::as_str)
                        .unwrap_or(&attempt_model),
                    240,
                );
                workflow["final_llm_response"]["used"] = Value::Bool(true);
                workflow["final_llm_response"]["status"] = Value::String("synthesized".to_string());
                if repaired_missing_turn_tool_context {
                    workflow["final_llm_response"]["runtime_visible_fallback_source"] =
                        json!("missing_turn_tool_context_repair");
                    workflow["final_llm_response"]["repaired_missing_turn_tool_context"] =
                        Value::Bool(true);
                }
                workflow["final_llm_response"]["provider"] =
                    Value::String(response_provider.clone());
                workflow["final_llm_response"]["model"] = Value::String(response_model.clone());
                workflow["final_llm_response"]["runtime_model"] =
                    Value::String(response_model.clone());
                annotate_final_evidence_outcome_posture(workflow, response_tools);
                workflow["provider"] = Value::String(response_provider);
                workflow["model"] = Value::String(response_model.clone());
                workflow["runtime_model"] = Value::String(response_model);
                if response_tools.is_empty()
                    && enriched_workflow_events.is_empty()
                    && !manual_toolbox_gate_turn
                {
                    mark_workflow_direct_llm_no_tool_answer(workflow);
                }
                set_turn_workflow_final_stage_status(workflow, "synthesized");
                workflow["final_llm_response"]["helpfulness"] = json!({
                    "direct_answer_in_first_two_sentences": direct_answer_in_first_two_sentences,
                    "prompt_echo_detected": prompt_echo_reply,
                    "has_evidence_tags": response_has_evidence_tags(&retried_text)
                        || receipt_mapped_sources
                        || response_tools.is_empty(),
                    "missing_evidence_mapping": missing_evidence_tags,
                    "template_label": template_label,
                    "detail_style": detail_style
                });
                let attempt_count = attempt as f64;
                let off_topic_reject =
                    response_workflow_quality_rate(workflow, "off_topic_reject");
                let direct_answer_rate = if direct_answer_in_first_two_sentences {
                    1.0
                } else {
                    0.0
                };
                let retry_rate = if max_attempts > 1 {
                    ((attempt.saturating_sub(1)) as f64 / (max_attempts.saturating_sub(1)) as f64)
                        .clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let off_topic_reject_rate = if attempt_count > 0.0 {
                    (off_topic_reject / attempt_count).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                workflow["quality_telemetry"]["direct_answer_rate"] = json!(direct_answer_rate);
                workflow["quality_telemetry"]["retry_rate"] = json!(retry_rate);
                workflow["quality_telemetry"]["off_topic_reject_rate"] =
                    json!(off_topic_reject_rate);
                persist_workflow_visible_response(workflow, &retried_text);
                return WorkflowFinalResponseAttemptResult::Finalize;
            }
            Err(err) => {
                diagnostics.last_error = clean_text(&err, 240);
            }
        }
    }
    WorkflowFinalResponseAttemptResult::Exhausted
}
