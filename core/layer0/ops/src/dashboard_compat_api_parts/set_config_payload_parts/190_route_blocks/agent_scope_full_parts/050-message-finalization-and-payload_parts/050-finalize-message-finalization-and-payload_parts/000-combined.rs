fn workflow_json_final_chat_is_llm_only(response_workflow: &Value) -> bool {
    response_workflow
        .pointer("/selected_workflow/final_output_contract/visible_chat_source")
        .and_then(Value::as_str)
        == Some("llm_final_answer_only")
        || response_workflow
            .pointer("/selected_workflow/tool_menu_interface_contract/visible_chat_policy")
            .and_then(Value::as_str)
            == Some("llm_final_only_no_system_injection")
}

fn workflow_json_private_gate_chat_is_disallowed(response_workflow: &Value) -> bool {
    workflow_json_final_chat_is_llm_only(response_workflow)
        && response_workflow
            .pointer("/selected_workflow/tool_menu_interface_contract/system_injected_chat_text_allowed")
            .and_then(Value::as_bool)
            == Some(false)
}

fn workflow_json_pending_tool_chat_is_disallowed(
    response_workflow: &Value,
    pending_request: &Value,
) -> bool {
    pending_request
        .get("chat_injection_allowed")
        .and_then(Value::as_bool)
        == Some(false)
        || response_workflow
            .pointer(
                "/selected_workflow/tool_menu_interface_contract/tool_request_submission_contract/chat_injection_allowed",
            )
            .and_then(Value::as_bool)
            == Some(false)
}

fn workflow_json_auto_executes_tools_if_permitted(response_workflow: &Value) -> bool {
    response_workflow
        .pointer(
            "/selected_workflow/tool_menu_interface_contract/gates/gate_4b_tool_confirmation_menu/execution_mode",
        )
        .and_then(Value::as_str)
        == Some("auto_if_permitted")
        || response_workflow
            .pointer("/selected_workflow/tool_menu_interface_contract/tool_execution_policy/default_mode")
            .and_then(Value::as_str)
            == Some("auto_if_permitted")
}

fn workflow_json_latent_candidate_recovery_enabled(response_workflow: &Value) -> bool {
    response_workflow
        .pointer("/selected_workflow/tool_menu_interface_contract/latent_candidate_recovery_contract/enabled")
        .and_then(Value::as_bool)
        == Some(true)
}

fn workflow_private_gate_recovery_signal(response_workflow: &Value) -> bool {
    let direct_response_path = response_workflow
        .pointer("/workflow_control/direct_response_path")
        .and_then(Value::as_str)
        .unwrap_or("");
    let private_gate_pending_path = matches!(
        direct_response_path,
        "first_gate_pending_llm_tool_choice"
            | "first_gate_pending_tool_confirmation"
            | "gate_2_pending_llm_tool_request"
    ) || direct_response_path.starts_with("gate_") && direct_response_path.contains("pending");
    let private_gate_reject = matches!(
        response_workflow
            .pointer("/final_llm_response/last_reject_reason")
            .and_then(Value::as_str)
            .unwrap_or(""),
        "invalid_manual_toolbox_gate_submission"
            | "tool_category_without_tool_payload"
            | "visible_gate_choice_reply"
    );
    response_workflow
        .pointer("/final_llm_response/invalid_gate_draft_diagnostic_only")
        .and_then(Value::as_bool)
        == Some(true)
        || private_gate_pending_path
        || private_gate_reject
}

fn workflow_recovery_contract_marker(
    response_workflow: &Value,
    pointer: &str,
    text: &str,
) -> bool {
    let selected_contract = workflow_tool_menu_contract_from_response_workflow(response_workflow);
    workflow_message_matches_contract_markers(&selected_contract, pointer, text)
        || workflow_message_matches_contract_markers(
            &default_workflow_tool_menu_contract(),
            pointer,
            text,
        )
}

fn workflow_latent_candidate_recovery_needed(
    response_workflow: &Value,
    initial_draft_response: &str,
) -> bool {
    let response_text = clean_text(
        response_workflow
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or(""),
        2_000,
    );
    let combined = clean_text(
        &format!(
            "{}\n{}",
            response_text,
            clean_text(initial_draft_response, 2_000)
        ),
        4_000,
    )
    .to_ascii_lowercase();
    let claims_missing_tool_backed_evidence = workflow_recovery_contract_marker(
        response_workflow,
        "/diagnostic_markers/final_response_verifier/missing_evidence_claim_phrases",
        &combined,
    );
    let outside_evidence_used_for_decision = workflow_recovery_contract_marker(
        response_workflow,
        "/diagnostic_markers/final_response_verifier/outside_evidence_source_boundary_phrases",
        &combined,
    ) && workflow_recovery_contract_marker(
        response_workflow,
        "/diagnostic_markers/final_response_verifier/bounded_answer_signals",
        &combined,
    ) && !workflow_response_refuses_unsupported_recommendation(&combined);
    let requests_more_tooling = workflow_recovery_contract_marker(
        response_workflow,
        "/diagnostic_markers/deferred_tool_request_phrases",
        &combined,
    ) || workflow_recovery_contract_marker(
        response_workflow,
        "/diagnostic_markers/unresolved_tool_need_phrases",
        &combined,
    );
    let private_gate_diagnostic = workflow_private_gate_recovery_signal(response_workflow);
    claims_missing_tool_backed_evidence
        || outside_evidence_used_for_decision
        || requests_more_tooling
        || private_gate_diagnostic
}

fn workflow_response_refuses_unsupported_recommendation(normalized: &str) -> bool {
    [
        "not enough to recommend",
        "cannot recommend",
        "can't recommend",
        "no source backed basis to choose",
        "no source-backed basis to choose",
        "no source backed basis to recommend",
        "no source-backed basis to recommend",
        "do not use this as a recommendation",
        "should not be used as a recommendation",
    ]
    .iter()
    .any(|needle| normalized.contains(*needle))
}

fn workflow_terminal_invariant_promotes_required_latent_candidates(
    response_workflow: &Value,
) -> bool {
    response_workflow
        .pointer(
            "/selected_workflow/tool_menu_interface_contract/terminal_invariant_contract/valid_latent_candidate_without_tool_attempt_policy",
        )
        .and_then(Value::as_str)
        == Some("promote_single_required_candidate_or_structured_failure_before_final_answer")
}

fn workflow_terminal_invariant_required_latent_candidate_flag(
    response_workflow: &Value,
) -> String {
    clean_text(
        response_workflow
            .pointer(
                "/selected_workflow/tool_menu_interface_contract/terminal_invariant_contract/required_latent_candidate_flag",
            )
            .and_then(Value::as_str)
            .unwrap_or("requires_tool_attempt_before_final_answer"),
        120,
    )
}

fn latent_candidate_requires_tool_attempt(candidate: &Value, required_flag: &str) -> bool {
    let required_flag = clean_text(required_flag, 120);
    if required_flag.is_empty() {
        return false;
    }
    candidate
        .get(&required_flag)
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn workflow_latent_candidate_recovery_required_by_terminal_invariant(
    response_workflow: &Value,
    latent_tool_candidates: &Value,
) -> bool {
    if !workflow_terminal_invariant_promotes_required_latent_candidates(response_workflow) {
        return false;
    }
    let required_flag = workflow_terminal_invariant_required_latent_candidate_flag(response_workflow);
    let Some(candidates) = latent_tool_candidates.as_array() else {
        return false;
    };
    candidates
        .iter()
        .filter(|candidate| {
            candidate
                .get("workflow_only")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                && latent_candidate_requires_tool_attempt(candidate, &required_flag)
        })
        .count()
        == 1
}

fn final_guard_retry_boilerplate_rewrite(message: &str, response_text: &str) -> Option<String> {
    let lowered = clean_text(response_text, 2_000).to_ascii_lowercase();
    let retry_boilerplate_family = response_contains_unexpected_state_retry_boilerplate(&lowered)
        || lowered.contains("recorded evidence so far")
        || lowered.contains("here's what i found")
        || lowered.contains("here s what i found")
        || lowered.contains("heres what i found")
        || lowered.contains("web retrieval ran, but only")
        || lowered.contains("retrieval attempt did not produce enough");
    if !retry_boilerplate_family {
        return None;
    }
    let _ = message;
    Some(clean_text(
        "The retrieved evidence in this turn was not strong enough to support a clean source-backed conclusion across all requested lanes. The current results are too noisy, incomplete, or mismatched to justify a reliable ranking, comparison, or recommendation. The safest bounded answer is to avoid a firm conclusion from this turn alone.",
        1_200,
    ))
}

fn finalize_message_finalization_and_payload(
    root: &Path,
    agent_id: &str,
    snapshot: &Value,
    existing_agent_row: &Value,
    message: &str,
    result: &Value,
    response_text: String,
    response_tools: Vec<Value>,
    workflow_mode: String,
    mut workflow_system_events: Vec<Value>,
    runtime_summary: Value,
    _state: Value,
    _messages: Vec<Value>,
    active_messages: Vec<Value>,
    provider: String,
    model: String,
    _requested_provider: String,
    _requested_model: String,
    auto_route: Option<Value>,
    virtual_key_id: String,
    virtual_key_gate: Value,
    fallback_window: i64,
    context_active_tokens: i64,
    context_ratio: f64,
    context_pressure: String,
    context_pool_limit_tokens: i64,
    context_pool_tokens: i64,
    pooled_messages_len: usize,
    sessions_total: usize,
    memory_kv_entries: usize,
    active_context_target_tokens: i64,
    active_context_min_recent: usize,
    include_all_sessions_context: bool,
    pre_generation_pruned: bool,
    recent_floor_enforced: bool,
    recent_floor_injected: usize,
    history_trim_confirmed: bool,
    emergency_compact: Value,
    workspace_hints: Value,
    latent_tool_candidates: Value,
    _inline_tools_allowed: bool,
) -> CompatApiResponse {
    let mut response_tools = response_tools;
    let initial_draft_response = clean_chat_text(&response_text, 32_000);
    let initial_ack_only = response_looks_like_tool_ack_without_findings(&initial_draft_response)
        || response_is_no_findings_placeholder(&initial_draft_response);
    let web_intent_route = String::new();
    let web_intent_detected = false;
    let web_intent_source = "workflow_llm_manual_only";
    let web_intent_confidence = 0.0;
    let web_forced_fallback_attempted = false;
    let latest_assistant_text = latest_assistant_message_text(&active_messages);
    let workflow_provider = provider.clone();
    let workflow_model = model.clone();
    if response_tools.is_empty()
        && !message_explicitly_disallows_tool_calls(message)
        && !message_is_affirmative_confirmation(message)
        && !message_is_negative_confirmation(message)
    {
        workflow_system_events.push(turn_workflow_event(
            "manual_toolbox_candidate_menu",
            json!({
                "selection_authority": "llm_only",
                "automatic_execution_allowed": false,
                "candidate_source": "json_cd_default_gate"
            }),
        ));
    }
    let mut response_workflow = run_turn_workflow_final_response(
        root,
        &workflow_provider,
        &workflow_model,
        &active_messages,
        message,
        &workflow_mode,
        &response_tools,
        &workflow_system_events,
        &response_text,
        &latest_assistant_text,
    );
    let mut manual_toolbox_pending_tool_request = response_workflow
        .get("manual_toolbox_pending_tool_request")
        .filter(|value| value.is_object())
        .cloned();
    let latent_candidate_recovery_required =
        workflow_latent_candidate_recovery_required_by_terminal_invariant(
            &response_workflow,
            &latent_tool_candidates,
        );
    if response_tools.is_empty()
        && manual_toolbox_pending_tool_request.is_none()
        && workflow_json_auto_executes_tools_if_permitted(&response_workflow)
        && workflow_json_latent_candidate_recovery_enabled(&response_workflow)
        && (latent_candidate_recovery_required
            || workflow_latent_candidate_recovery_needed(
                &response_workflow,
                &initial_draft_response,
            ))
    {
        let recovered_pending_request =
            workflow_pending_request_from_selected_tool_contract(&response_workflow, message)
                .or_else(|| {
                    manual_toolbox_pending_request_from_latent_candidates(
                        &latent_tool_candidates,
                        message,
                    )
                });
        if let Some(pending_request) = recovered_pending_request {
            let recovery_source = pending_request
                .get("source")
                .and_then(Value::as_str)
                .unwrap_or("latent_candidate_recovery");
            workflow_system_events.push(turn_workflow_event(
                "workflow_pending_tool_request_recovered",
                json!({
                    "source": recovery_source,
                    "tool_name": pending_request
                        .get("tool_name")
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                    "ambiguity_policy": "single_valid_workflow_only_candidate_or_selected_tool_contract"
                }),
            ));
            response_workflow["manual_toolbox_pending_tool_request"] = pending_request.clone();
            response_workflow["pending_tool_request"] = pending_request.clone();
            response_workflow["workflow_control"]["direct_response_path"] =
                Value::String("gate_4_pending_llm_tool_request".to_string());
            response_workflow["final_llm_response"]["used"] = Value::Bool(false);
            response_workflow["final_llm_response"]["status"] =
                Value::String("skipped_pending_tool_confirmation".to_string());
            manual_toolbox_pending_tool_request = Some(pending_request);
        }
    }
    if response_tools.is_empty() && workflow_json_auto_executes_tools_if_permitted(&response_workflow)
    {
        if let Some(pending_request) = manual_toolbox_pending_tool_request.clone() {
            if pending_request.get("status").and_then(Value::as_str) == Some("pending_confirmation")
            {
                let pending_tool = normalize_tool_name(
                    pending_request
                        .get("tool_name")
                        .or_else(|| pending_request.get("tool"))
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                );
                let pending_input = pending_request
                    .get("input")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                let pending_source = clean_text(
                    pending_request
                        .get("source")
                        .and_then(Value::as_str)
                        .unwrap_or("manual_toolbox_gate"),
                    80,
                );
                if !pending_tool.is_empty() {
                    let tool_payload = execute_tool_call_with_recovery(
                        root,
                        snapshot,
                        agent_id,
                        Some(existing_agent_row),
                        &pending_tool,
                        &pending_input,
                    );
                    if tool_error_requires_confirmation(&tool_payload) {
                        store_pending_tool_confirmation(
                            root,
                            agent_id,
                            &pending_tool,
                            &pending_input,
                            &pending_source,
                        );
                    } else {
                        clear_pending_tool_confirmation(root, agent_id);
                        let tool_card_status = tool_card_status_from_payload(&tool_payload);
                        response_tools.push(response_tool_card(
                            format!("tool-auto-{}", normalize_tool_name(&pending_tool)),
                            &pending_tool,
                            &pending_input,
                            &tool_payload,
                            !tool_payload.get("ok").and_then(Value::as_bool).unwrap_or(false),
                            &tool_card_status,
                        ));
                        workflow_system_events.push(turn_workflow_event(
                            "manual_toolbox_pending_tool_request_auto_executed",
                            json!({
                                "tool_name": pending_tool,
                                "source": pending_source,
                                "execution_authority": "cd_auto_if_permitted"
                            }),
                        ));
                        response_workflow = run_turn_workflow_final_response(
                            root,
                            &workflow_provider,
                            &workflow_model,
                            &active_messages,
                            message,
                            "workflow_auto_tool_execution",
                            &response_tools,
                            &workflow_system_events,
                            &response_text,
                            &latest_assistant_text,
                        );
                        let mut executed_pending_request = pending_request.clone();
                        executed_pending_request["status"] = json!("executed");
                        manual_toolbox_pending_tool_request = Some(executed_pending_request.clone());
                        response_workflow["pending_tool_request"] = executed_pending_request.clone();
                        response_workflow["manual_toolbox_pending_tool_request"] =
                            executed_pending_request;
                    }
                }
            }
        }
    }
    apply_final_empty_response_diagnostic(
        &mut response_workflow,
        message,
        &latest_assistant_text,
        &response_tools,
    );
    let pending_tool_confirmation_fallback = manual_toolbox_pending_tool_request
        .as_ref()
        .map(|pending_request| {
            workflow_pending_tool_confirmation_fallback(pending_request, &response_tools)
        })
        .unwrap_or_default();
    let mut response_text = response_workflow
        .get("response")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .unwrap_or_default();
    let mut finalized_response = clean_chat_text(&response_text, 32_000);
    let mut tool_completion = json!({});
    let workflow_status = workflow_final_response_status(&response_workflow);
    let workflow_used = workflow_final_response_used(&response_workflow);
    let workflow_waiting_for_private_json_gate =
        workflow_private_gate_recovery_signal(&response_workflow);
    let mut finalization_outcome = if workflow_used {
        "workflow_authored".to_string()
    } else {
        "workflow_llm_unavailable".to_string()
    };
    if !workflow_status.is_empty() {
        finalization_outcome = merge_response_outcomes(
            &finalization_outcome,
            &format!("workflow:{workflow_status}"),
            200,
        );
    }
    let tooling_fallback_used = false;
    let comparative_fallback_used = false;
    let workflow_system_fallback_used = false;
    let visible_response_repaired = false;
    let final_fallback_used = false;
    if workflow_used {
        tool_completion = tool_completion_report_for_response(
            &finalized_response,
            &response_tools,
            "workflow_authored",
        );
    } else if workflow_status_blocks_runtime_visible_fallback(&workflow_status) {
        finalized_response.clear();
        response_workflow["response"] = json!("");
        response_workflow["final_llm_response"]["runtime_interference_disabled"] = json!(true);
        tool_completion = tool_completion_report_for_response("", &response_tools, "workflow_blocked");
        finalization_outcome =
            merge_response_outcomes(&finalization_outcome, "runtime_visible_fallback_suppressed", 220);
    } else {
        // Keep chat output LLM-authored only; runtime fallback substitution is disabled.
        let missing_turn_tool_context_fallback =
            workflow_missing_turn_tool_context_fallback(message, &response_tools);
        let llm_only_candidate = if !missing_turn_tool_context_fallback.is_empty() {
            response_workflow["final_llm_response"]["runtime_visible_fallback_source"] =
                json!("missing_turn_tool_context");
            missing_turn_tool_context_fallback
        } else {
            initial_draft_response.clone()
        };
        annotate_final_evidence_outcome_posture(&mut response_workflow, &response_tools);
        let (contract_finalized, contract_report, contract_outcome) =
            enforce_user_facing_finalization_contract(message, llm_only_candidate, &response_tools);
        finalized_response = contract_finalized;
        tool_completion = contract_report;
        finalization_outcome =
            merge_response_outcomes(&finalization_outcome, "workflow_no_runtime_fallback", 200);
        finalization_outcome =
            merge_response_outcomes(&finalization_outcome, &contract_outcome, 200);
    }
    if workflow_waiting_for_private_json_gate
        && workflow_json_private_gate_chat_is_disallowed(&response_workflow)
        && response_tools.is_empty()
        && manual_toolbox_pending_tool_request.is_none()
    {
        response_workflow["final_llm_response"]["private_gate_visible_chat_blocked_by_json"] =
            json!(true);
        response_workflow["final_llm_response"]["invalid_gate_draft_diagnostic_only"] =
            json!(true);
        response_workflow["final_llm_response"]["runtime_interference_disabled"] = json!(true);
        response_workflow["final_llm_response"]["diagnostic_source"] =
            json!("json_private_gate_nonfinal");
        finalized_response.clear();
        response_workflow["response"] = json!("");
        response_workflow["visible_response_source"] = json!("json_private_gate_nonfinal");
        finalization_outcome = merge_response_outcomes(
            &finalization_outcome,
            "json_private_gate_nonfinal_visible_chat_blocked",
            220,
        );
    }
    let repair_candidate_contamination = response_contains_stale_code_context_dump(message, &finalized_response)
        || response_contains_unrequested_content_without_tool_evidence(
            message,
            &finalized_response,
            &response_tools,
        );
    tool_completion = enrich_tool_completion_receipt(tool_completion, &response_tools);
    response_text = workflow_final_visible_response_text(&finalized_response);
    if response_text.trim().is_empty() {
        finalization_outcome = merge_response_outcomes(
            &finalization_outcome,
            "empty_final_response_no_system_retry",
            220,
        );
    }
    let manual_toolbox_executed_pending_tool_request = if let Some(pending_request) =
        manual_toolbox_pending_tool_request.as_ref()
    {
        let pending_status = pending_request
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("");
        let pending_tool = clean_text(
            pending_request
                .get("tool_name")
                .or_else(|| pending_request.get("tool"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        );
        let tool_names_match = response_tools.iter().any(|tool| {
            let tool_name = normalize_tool_name(
                tool.get("name")
                    .or_else(|| tool.get("tool"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
            );
            !pending_tool.is_empty() && tool_name == pending_tool
        });
        (pending_status == "pending_confirmation" || pending_status == "executed")
            && !response_tools.is_empty()
            && tool_names_match
    } else {
        false
    };
    if let Some(pending_request) = manual_toolbox_pending_tool_request.clone() {
        let pending_tool = clean_text(
            pending_request
                .get("tool_name")
                .or_else(|| pending_request.get("tool"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        );
        let pending_tool = normalize_tool_name(&pending_tool);
        if !pending_tool.is_empty() {
            let mut pending_request = pending_request;
            if manual_toolbox_executed_pending_tool_request {
                clear_pending_tool_confirmation(root, agent_id);
            } else {
                let pending_input = pending_request
                    .get("input")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                let pending_source = pending_request
                    .get("source")
                    .and_then(Value::as_str)
                    .unwrap_or("manual_toolbox_gate");
                store_pending_tool_confirmation(
                    root,
                    agent_id,
                    &pending_tool,
                    &pending_input,
                    pending_source,
                );
            }
            if manual_toolbox_executed_pending_tool_request {
                pending_request["status"] = json!("executed");
            }
            manual_toolbox_pending_tool_request = Some(pending_request.clone());
            response_workflow["pending_tool_request"] = pending_request.clone();
            response_workflow["manual_toolbox_pending_tool_request"] = pending_request.clone();
            if manual_toolbox_executed_pending_tool_request {
                finalization_outcome = merge_response_outcomes(
                    &finalization_outcome,
                    "manual_toolbox_pending_tool_request_executed",
                    200,
                );
            } else {
                finalization_outcome = merge_response_outcomes(
                    &finalization_outcome,
                    "manual_toolbox_pending_tool_request_awaiting_llm_input",
                    200,
                );
            }
            if !manual_toolbox_executed_pending_tool_request
                && workflow_json_pending_tool_chat_is_disallowed(&response_workflow, &pending_request)
            {
                finalization_outcome = merge_response_outcomes(
                    &finalization_outcome,
                    "json_pending_tool_visible_chat_blocked",
                    240,
                );
                if !manual_toolbox_executed_pending_tool_request
                    && !pending_tool_confirmation_fallback.is_empty()
                {
                    response_text = pending_tool_confirmation_fallback.clone();
                    response_workflow["response"] = json!(response_text.clone());
                    response_workflow["visible_response_source"] =
                        json!("runtime_pending_tool_confirmation");
                    response_workflow["final_llm_response"]["runtime_visible_fallback_source"] =
                        json!("pending_tool_confirmation");
                    finalization_outcome = merge_response_outcomes(
                        &finalization_outcome,
                        "runtime_pending_tool_confirmation_fallback",
                        220,
                    );
                } else {
                    response_text.clear();
                    response_workflow["response"] = json!("");
                    response_workflow["visible_response_source"] =
                        json!("json_private_tool_request");
                }
            }
        }
    }
    let web_tool_attempted = response_tools_include_web_attempt(&response_tools);
    let web_tool_blocked = response_tools_web_blocked(&response_tools);
    let web_tool_low_signal = response_tools_web_low_signal(&response_tools);
    let web_turn_classification = classify_web_turn_state(
        web_intent_detected,
        web_tool_attempted,
        web_tool_blocked,
        web_tool_low_signal,
    );
    let mut web_failure_code = web_failure_code_from_response_tools(&response_tools);
    if web_intent_detected && !web_tool_attempted {
        web_failure_code = "web_route_parse_failed".to_string();
    } else if web_failure_code.is_empty() && web_tool_low_signal {
        web_failure_code = "web_tool_low_signal".to_string();
    }
    let tooling_attempted = !response_tools.is_empty();
    let tooling_blocked = response_tools_any_blocked(&response_tools);
    let tooling_low_signal = response_tools_any_low_signal(&response_tools);
    let tooling_failure_code = tool_failure_code_from_response_tools(&response_tools);
    let tooling_turn_classification = if !tooling_attempted {
        "not_attempted".to_string()
    } else if tooling_blocked {
        "policy_blocked".to_string()
    } else if tooling_low_signal {
        "low_signal".to_string()
    } else if !tooling_failure_code.is_empty() {
        "failed".to_string()
    } else {
        "healthy".to_string()
    };
    let mut tooling_invariant_repair_used = false;
    let mut web_invariant_repair_used = false;
    if web_intent_detected && !web_tool_attempted {
        web_invariant_repair_used = true;
        finalization_outcome = merge_response_outcomes(
            &finalization_outcome,
            "web_invariant_missing_tool_attempt",
            200,
        );
    } else if web_tool_attempted
        && (web_tool_blocked || web_tool_low_signal || !web_failure_code.is_empty())
    {
        web_invariant_repair_used = true;
        finalization_outcome =
            merge_response_outcomes(&finalization_outcome, "web_failure_code_appended", 200);
    }
    if tooling_attempted && (tooling_blocked || tooling_low_signal || !tooling_failure_code.is_empty()) {
        tooling_invariant_repair_used = true;
        finalization_outcome = merge_response_outcomes(&finalization_outcome, "tooling_failure_code_appended", 200);
    }
    let response_guard =
        final_response_guard_report(message, &response_text, &response_tools, repair_candidate_contamination);
    if response_guard_bool(&response_guard, "final_contract_violation") {
        let rewritten_visible_response =
            final_guard_retry_boilerplate_rewrite(message, &response_text);
        if let Some(rewritten) = rewritten_visible_response.clone() {
            response_text = rewritten;
            finalized_response = clean_chat_text(&response_text, 32_000);
            response_workflow["response"] = json!(response_text.clone());
            response_workflow["text"] = json!(response_text.clone());
            response_workflow["message"] = json!(response_text.clone());
            response_workflow["final_llm_response"]["guard_rewrite_applied"] = json!(true);
            response_workflow["final_llm_response"]["guard_rewrite_reason"] =
                json!("retry_boilerplate_visible_response");
            response_workflow["final_llm_response"]["guard_rewrite_excerpt"] =
                json!(first_sentence(&response_text, 240));
            finalization_outcome = merge_response_outcomes(
                &finalization_outcome,
                "final_response_guard_rewritten",
                220,
            );
        }
        response_workflow["final_llm_response"]["runtime_interference_disabled"] = json!(true);
        response_workflow["final_llm_response"]["final_guard_diagnostic_only"] =
            json!(rewritten_visible_response.is_none());
        if response_guard_bool(&response_guard, "final_contamination_violation") {
            bump_workflow_quality_counter(&mut response_workflow, "contamination_reject");
        }
        if response_guard_bool(&response_guard, "current_turn_dominance_violation") {
            bump_workflow_quality_counter(&mut response_workflow, "current_turn_dominance_reject");
        }
        if response_guard_bool(&response_guard, "unsupported_tool_success_claim") {
            bump_workflow_quality_counter(
                &mut response_workflow,
                "unsupported_tool_success_claim_reject",
            );
        }
        finalization_outcome = merge_response_outcomes(
            &finalization_outcome,
            final_response_guard_outcome(&response_guard),
            200,
        );
        if rewritten_visible_response.is_none() {
            finalization_outcome = merge_response_outcomes(
                &finalization_outcome,
                "final_response_guard_diagnostic_only",
                220,
            );
        }
    }
    if manual_toolbox_executed_pending_tool_request {
        finalization_outcome = merge_response_outcomes(
            "manual_toolbox_pending_tool_request_executed",
            &finalization_outcome,
            220,
        );
        response_workflow["workflow_control"]["direct_response_path"] =
            Value::String("manual_toolbox_executed_tool_route".to_string());
        response_workflow["tool_gate"]["needs_tool_access"] = Value::Bool(true);
        response_workflow["tool_gate"]["should_call_tools"] = Value::Bool(true);
    }
    let direct_answer_rate = response_workflow_quality_rate(&response_workflow, "direct_answer_rate");
    let retry_rate = response_workflow_quality_rate(&response_workflow, "retry_rate");
    let off_topic_reject_rate =
        response_workflow_quality_rate(&response_workflow, "off_topic_reject_rate");
    let tool_overcall_rate = 0.0;
    response_workflow["quality_telemetry"]["final_fallback_used"] =
        Value::Bool(final_fallback_used);
    let final_ack_only = response_looks_like_tool_ack_without_findings(&response_text);
    let response_quality_telemetry = build_response_quality_telemetry_payload(
        &response_workflow,
        final_fallback_used,
        tooling_invariant_repair_used,
        &tooling_failure_code,
        direct_answer_rate,
        retry_rate,
        tool_overcall_rate,
        off_topic_reject_rate,
    );
    let tooling_invariant = json!({
        "tool_attempted": tooling_attempted,
        "tool_blocked": tooling_blocked,
        "low_signal": tooling_low_signal,
        "classification": tooling_turn_classification,
        "failure_code": tooling_failure_code,
        "invariant_repair_used": tooling_invariant_repair_used
    });
    let web_invariant = json!({
        "requires_live_web": web_intent_detected,
        "intent_source": web_intent_source,
        "intent_confidence": web_intent_confidence,
        "selected_route": web_intent_route.clone(),
        "tool_attempted": web_tool_attempted,
        "tool_blocked": web_tool_blocked,
        "low_signal": web_tool_low_signal,
        "classification": web_turn_classification,
        "failure_code": web_failure_code,
        "forced_fallback_attempted": web_forced_fallback_attempted,
        "invariant_repair_used": web_invariant_repair_used
    });
    let mut response_finalization = build_response_finalization_payload(
        &finalization_outcome,
        initial_ack_only,
        final_ack_only,
        &tool_completion,
        tooling_fallback_used,
        comparative_fallback_used,
        workflow_system_fallback_used,
        visible_response_repaired,
        &response_quality_telemetry,
        &tooling_invariant,
        &web_invariant,
    );
    let workflow_direct_response_path = response_workflow
        .pointer("/workflow_control/direct_response_path")
        .and_then(Value::as_str)
        .unwrap_or(if manual_toolbox_pending_tool_request.is_some() {
            "first_gate_pending_tool_confirmation"
        } else {
            "first_gate_no_tool_category"
        });
    response_finalization["workflow_control"] = json!({
        "mode": "tool_menu_interface_v1",
        "direct_response_path": workflow_direct_response_path
    });
    apply_response_guard_payloads(&mut response_finalization, &response_guard);
    if let Some(pending_request) = manual_toolbox_pending_tool_request.as_ref() {
        response_finalization["pending_tool_request"] = pending_request.clone();
    }
    let visible_response_source = apply_visible_response_provenance_for_turn(
        &mut response_workflow,
        &mut response_finalization,
        &response_text,
        workflow_used,
        visible_response_repaired,
        &finalization_outcome,
    );
    let canonical_finalized_response = workflow_final_visible_response_text(&response_text);
    if !canonical_finalized_response.trim().is_empty() {
        response_finalization["finalized_output"] =
            Value::String(canonical_finalized_response.clone());
        response_finalization["final_output"] =
            Value::String(canonical_finalized_response.clone());
        response_finalization["final_response"]["text"] =
            Value::String(canonical_finalized_response.clone());
        response_finalization["final_llm_response"]["text"] =
            Value::String(canonical_finalized_response.clone());
        response_workflow["final_llm_response"]["text"] =
            Value::String(canonical_finalized_response.clone());
        response_workflow["final_llm_response"]["visible_response_source"] =
            Value::String(visible_response_source.to_string());
    }
    let final_package_citations = response_finalization
        .get("citations")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let final_package_source_refs = response_finalization
        .get("source_refs")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if !final_package_citations.is_empty() {
        response_workflow["citations"] = Value::Array(final_package_citations.clone());
        response_workflow["final_llm_response"]["citations"] =
            Value::Array(final_package_citations.clone());
    }
    if !final_package_source_refs.is_empty() {
        response_workflow["source_refs"] = Value::Array(final_package_source_refs.clone());
        response_workflow["final_llm_response"]["source_refs"] =
            Value::Array(final_package_source_refs.clone());
    }
    if let Some(posture) = response_workflow
        .pointer("/final_llm_response/evidence_outcome_posture")
        .and_then(Value::as_str)
    {
        response_finalization["final_llm_response"]["evidence_outcome_posture"] =
            Value::String(posture.to_string());
    }
    let turn_transaction = crate::dashboard_tool_turn_loop::turn_transaction_payload(
        "complete",
        if response_tools.is_empty() {
            "none"
        } else {
            "complete"
        },
        "complete",
        "complete",
    );
    let terminal_transcript = tool_terminal_transcript(&response_tools);
    let empty_response_has_guard_diagnostics =
        response_guard_bool(&response_guard, "final_contamination_violation")
            || response_guard_bool(&response_guard, "current_turn_dominance_violation")
            || response_guard_bool(&response_guard, "unsupported_tool_success_claim")
            || response_guard_bool(&response_guard, "visible_gate_choice_leakage");
    if response_text.trim().is_empty() && !empty_response_has_guard_diagnostics {
        finalization_outcome = merge_response_outcomes(
            &finalization_outcome,
            "empty_visible_response_preserved_without_system_chat",
            220,
        );
        response_finalization["outcome"] = json!(finalization_outcome.clone());
    }
    let process_summary = build_turn_process_summary(
        message,
        &response_tools,
        &response_workflow,
        &response_finalization,
    );
    let workflow_visibility = workflow_visibility_payload(&response_workflow, &response_finalization);
    let turn_receipt = append_turn_receipt_with_metadata(
        root,
        agent_id,
        message,
        &response_text,
        Value::Array(response_tools.clone()),
        &response_workflow,
        &response_finalization,
        &process_summary,
        &turn_transaction,
        &terminal_transcript,
    );
    let agent_health_snapshot = persist_agent_control_plane_health_snapshot_for_turn(root, agent_id, message, &response_text, &response_workflow, &response_finalization, &process_summary, &turn_receipt);
    let runtime_provider = clean_text(
        response_workflow
            .pointer("/final_llm_response/provider")
            .or_else(|| response_workflow.get("provider"))
            .and_then(Value::as_str)
            .unwrap_or(&provider),
        80,
    );
    let runtime_model = clean_text(
        response_workflow
            .pointer("/final_llm_response/runtime_model")
            .or_else(|| response_workflow.get("runtime_model"))
            .or_else(|| result.get("runtime_model"))
            .or_else(|| result.get("model"))
            .and_then(Value::as_str)
            .unwrap_or(&model),
        240,
    );
    let runtime_patch = json!({
        "runtime_model": runtime_model,
        "context_window": result.get("context_window").cloned().unwrap_or_else(|| json!(0)),
        "context_window_tokens": result.get("context_window").cloned().unwrap_or_else(|| json!(0)),
        "updated_at": crate::now_iso()
    });
    let _ = update_profile_patch(root, agent_id, &runtime_patch);
    let mut payload = result.clone();
    payload["ok"] = json!(true);
    payload["agent_id"] = json!(agent_id);
    payload["provider"] = json!(runtime_provider);
    payload["model"] = json!(runtime_model.clone());
    payload["runtime_model"] = json!(runtime_model);
    payload["iterations"] = json!(1);
    payload["response"] = json!(response_text);
    payload["runtime_sync"] = runtime_summary;
    payload["tools"] = Value::Array(response_tools);
    payload["response_workflow"] = response_workflow;
    payload["terminal_transcript"] = Value::Array(terminal_transcript);
    payload["response_finalization"] = response_finalization;
    if !final_package_citations.is_empty() {
        payload["citations"] = Value::Array(final_package_citations);
    }
    if !final_package_source_refs.is_empty() {
        payload["source_refs"] = Value::Array(final_package_source_refs);
    }
    if let Some(pending_request) = manual_toolbox_pending_tool_request {
        payload["pending_tool_request"] = pending_request;
    }
    payload["process_summary"] = process_summary;
    payload["workflow_visibility"] = workflow_visibility;
    payload["response_quality_telemetry"] = response_quality_telemetry;
    payload["visible_response_source"] = json!(visible_response_source);
    payload["system_chat_injection_used"] = json!(false);
    payload["web_intent"] = json!({
        "detected": web_intent_detected,
        "source": web_intent_source,
        "confidence": web_intent_confidence,
        "selected_route": web_intent_route
    });
    payload["turn_transaction"] = turn_transaction;
    payload["context_window"] = json!(fallback_window.max(0));
    payload["context_tokens"] = json!(context_active_tokens.max(0));
    payload["context_used_tokens"] = json!(context_active_tokens.max(0));
    payload["context_ratio"] = json!(context_ratio);
    payload["context_pressure"] = json!(context_pressure.clone());
    payload["attention_queue"] = turn_receipt.get("attention_queue").cloned().unwrap_or_else(|| json!({}));
    payload["live_eval_monitor"] = turn_receipt.get("live_eval_monitor").cloned().unwrap_or_else(|| json!({}));
    payload["dashboard_health_indicator"] = agent_health_snapshot.get("dashboard_health_indicator").cloned().unwrap_or_else(|| json!({}));
    payload["agent_health_snapshot"] = agent_health_snapshot;
    payload["memory_capture"] = turn_receipt
        .get("memory_capture")
        .cloned()
        .unwrap_or_else(|| json!({}));
    payload["context_pool"] = json!({
        "pool_limit_tokens": context_pool_limit_tokens,
        "pool_tokens": context_pool_tokens,
        "pool_messages": pooled_messages_len,
        "session_count": sessions_total,
        "system_context_enabled": true,
        "system_context_limit_tokens": context_pool_limit_tokens,
        "llm_context_window_tokens": fallback_window.max(0),
        "cross_session_memory_enabled": true,
        "memory_kv_entries": memory_kv_entries,
        "active_target_tokens": active_context_target_tokens,
        "active_tokens": context_active_tokens,
        "active_messages": active_messages.len(),
        "min_recent_messages": active_context_min_recent,
        "include_all_sessions_context": include_all_sessions_context,
        "context_window": fallback_window.max(0),
        "context_ratio": context_ratio,
        "context_pressure": context_pressure,
        "pre_generation_pruning_enabled": true,
        "pre_generation_pruned": pre_generation_pruned,
        "recent_floor_enforced": recent_floor_enforced,
        "recent_floor_injected": recent_floor_injected,
        "history_trim_confirmed": history_trim_confirmed,
        "emergency_compact_enabled": true,
        "emergency_compact": emergency_compact
    });
    payload["workspace_hints"] = workspace_hints;
    payload["latent_tool_candidates"] = latent_tool_candidates;
    if let Some(route) = auto_route {
        payload["auto_route"] = route.get("route").cloned().unwrap_or_else(|| route.clone());
    }
    if !virtual_key_id.is_empty() {
        let spend_receipt = crate::dashboard_provider_runtime::record_virtual_key_usage(
            root,
            &virtual_key_id,
            payload
                .get("cost_usd")
                .and_then(Value::as_f64)
                .unwrap_or(0.0),
        );
        payload["virtual_key"] = json!({
            "id": virtual_key_id,
            "reservation": virtual_key_gate,
            "spend": spend_receipt
        });
    }
    CompatApiResponse {
        status: 200,
        payload,
    }
}

#[cfg(test)]
mod latent_candidate_recovery_tests {
    use super::*;

    #[test]
    fn latent_candidate_recovery_triggers_on_generic_tool_request_language() {
        let workflow = json!({
            "selected_workflow": {
                "tool_menu_interface_contract": {
                    "diagnostic_markers": {
                        "deferred_tool_request_phrases": [
                            "i need to do web research",
                            "requires real-time external information"
                        ],
                        "unresolved_tool_need_phrases": [
                            "need to verify externally"
                        ],
                        "final_response_verifier": {
                            "missing_evidence_claim_phrases": []
                        }
                    }
                }
            }
        });
        let response = "I can't fulfill this directly because it requires real-time external information. I need to do web research to give you an accurate comparison.";

        assert!(workflow_latent_candidate_recovery_needed(
            &workflow,
            response
        ));
    }
}
