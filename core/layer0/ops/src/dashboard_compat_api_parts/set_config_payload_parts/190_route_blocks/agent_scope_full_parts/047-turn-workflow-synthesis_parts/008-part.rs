fn augment_turn_workflow_events_for_final_response(
    message: &str,
    response_tools: &[Value],
    workflow_events: &[Value],
    draft_response: &str,
    latest_assistant_text: &str,
) -> Vec<Value> {
    let mut events = workflow_events.to_vec();
    let cleaned_draft = clean_text(draft_response, 4_000);
    let _ = message;
    if response_is_no_findings_placeholder(&cleaned_draft) {
        events.push(turn_workflow_event(
            "draft_response_invalid",
            json!({
                "reason": "no_findings_placeholder",
                "draft_excerpt": first_sentence(&cleaned_draft, 220)
            }),
        ));
    } else if response_contains_unexpected_state_retry_boilerplate(&cleaned_draft) {
        events.push(turn_workflow_event(
            "draft_response_invalid",
            json!({
                "reason": "unexpected_state_retry_boilerplate",
                "draft_excerpt": first_sentence(&cleaned_draft, 220)
            }),
        ));
    } else if response_looks_like_tool_ack_without_findings(&cleaned_draft) {
        events.push(turn_workflow_event(
            "draft_response_invalid",
            json!({
                "reason": "ack_only",
                "draft_excerpt": first_sentence(&cleaned_draft, 220)
            }),
        ));
    } else if response_is_deferred_execution_preamble(&cleaned_draft)
        || response_is_deferred_retry_prompt(&cleaned_draft)
        || workflow_response_requests_more_tooling(&cleaned_draft)
    {
        events.push(turn_workflow_event(
            "draft_response_invalid",
            json!({
                "reason": "deferred_retry_prompt",
                "draft_excerpt": first_sentence(&cleaned_draft, 220)
            }),
        ));
    }
    let findings = clean_text(&response_tools_summary_for_user(response_tools, 4), 2_000);
    if !findings.is_empty() {
        events.push(turn_workflow_event(
            "tool_findings_summary",
            json!({
                "summary": findings
            }),
        ));
    }
    let failure_summary = clean_text(
        &response_tools_failure_reason_for_user(response_tools, 4),
        2_000,
    );
    if !failure_summary.is_empty() {
        events.push(turn_workflow_event(
            "tool_failure_summary",
            json!({
                "summary": failure_summary
            }),
        ));
    }
    if !response_tools.is_empty()
        && !clean_text(
            &ensure_tool_turn_response_text(draft_response, response_tools),
            2_000,
        )
        .is_empty()
    {
        events.push(turn_workflow_event(
            "tool_response_readability_diagnostic",
            json!({
                "status": "tool_result_needs_llm_finalization"
            }),
        ));
    }
    if tooling_failure_diagnostic_detected(message, draft_response, latest_assistant_text) {
        events.push(turn_workflow_event(
            "tooling_failure_diagnostic",
            json!({
                "status": "tooling_failure_detected"
            }),
        ));
        // Tooling failures are carried as diagnostics only. Visible wording belongs to
        // the final LLM stage, not to workflow-authored fallback text.
    }
    let _ = message;
    events
}

#[cfg(test)]
fn workflow_test_llm_enabled(root: &Path) -> bool {
    root.join("client/runtime/local/state/ui/infring_dashboard/test_chat_script.json")
        .exists()
        || matches!(
            std::env::var("INFRING_LIVE_WEB_TOOLING_SMOKE")
                .ok()
                .as_deref()
                .map(|value| value.trim().to_ascii_lowercase()),
            Some(ref value) if value == "1" || value == "true" || value == "yes"
        )
}

#[cfg(not(test))]
fn workflow_test_llm_enabled(_root: &Path) -> bool {
    false
}

fn workflow_response_template_label(message: &str) -> &'static str {
    let _ = message;
    "workflow_final_response"
}

fn manual_toolbox_gate_context_user_prompt(message: &str) -> String {
    clean_text(
        &format!(
            "Context-only user message. Do not answer it directly. Use it only to produce the artifact required for the current workflow gate:\n{message}"
        ),
        8_000,
    )
}

fn response_tools_prompt_only_gate_required(
    _message: &str,
    _latent_tool_candidates: &Value,
) -> bool {
    false
}

fn direct_gate_recovery_response_answers_user(
    message: &str,
    response_text: &str,
    direct_gate_recovery_turn: bool,
) -> bool {
    let _ = direct_gate_recovery_turn;
    if recovery_complaint_about_response_loop(message)
        && !response_handles_recovery_complaint(message, response_text)
    {
        return false;
    }
    if response_answers_user_early(message, response_text) {
        return true;
    }
    false
}

fn recovery_complaint_about_response_loop(message: &str) -> bool {
    let lowered = clean_text(message, 1_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let complaint = lowered.contains("why")
        || lowered.contains("what")
        || lowered.contains("again")
        || lowered.contains("same")
        || lowered.contains("repeat")
        || lowered.contains("repeating");
    let response_loop = lowered.contains("fallback")
        || lowered.contains("same text")
        || lowered.contains("same response")
        || lowered.contains("repeat")
        || lowered.contains("repeating")
        || lowered.contains("loop");
    complaint && response_loop
}

fn response_handles_recovery_complaint(message: &str, response_text: &str) -> bool {
    if !recovery_complaint_about_response_loop(message) {
        return true;
    }
    let cleaned = clean_text(response_text, 2_000);
    let lowered = cleaned.to_ascii_lowercase();
    if lowered.is_empty()
        || cleaned.trim_end().ends_with('?')
        || lowered.contains("what do you need help")
        || lowered.contains("how can i help")
    {
        return false;
    }
    let names_failure = lowered.contains("fallback")
        || lowered.contains("loop")
        || lowered.contains("repeat")
        || lowered.contains("same text")
        || lowered.contains("same response");
    let gives_cause_or_boundary = lowered.contains("because")
        || lowered.contains("came from")
        || lowered.contains("happened")
        || lowered.contains("workflow")
        || lowered.contains("finalization")
        || lowered.contains("telemetry")
        || lowered.contains("diagnostic");
    let names_correction = lowered.contains("answer directly")
        || lowered.contains("keep")
        || lowered.contains("diagnostic")
        || lowered.contains("visible reply")
        || lowered.contains("chat text")
        || lowered.contains("not repeat");
    names_failure && gives_cause_or_boundary && names_correction
}

fn response_answers_tool_confirmation_with_recorded_result(
    response_text: &str,
    response_tools: &[Value],
) -> bool {
    if response_tools.is_empty() {
        return false;
    }
    let lowered = clean_text(response_text, 1_200).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let has_recorded_failure = !response_tools_failure_reason_for_user(response_tools, 4)
        .trim()
        .is_empty()
        || response_tools_any_low_signal(response_tools);
    if !has_recorded_failure {
        return false;
    }
    if response_is_no_findings_placeholder(&lowered) {
        return true;
    }
    let contract = default_workflow_tool_menu_contract();
    let mentions_tool_result = workflow_message_matches_contract_markers(
        &contract,
        "/diagnostic_markers/recorded_tool_result_answer/tool_result_terms",
        &lowered,
    );
    let explains_no_result = workflow_message_matches_contract_markers(
        &contract,
        "/diagnostic_markers/recorded_tool_result_answer/no_result_explanation_phrases",
        &lowered,
    );
    mentions_tool_result && explains_no_result
}

fn response_answers_successful_tool_result(
    message: &str,
    response_text: &str,
    response_tools: &[Value],
) -> bool {
    if response_tools.is_empty() {
        return false;
    }
    if !response_tools_failure_reason_for_user(response_tools, 4)
        .trim()
        .is_empty()
        || response_tools_any_low_signal(response_tools)
    {
        return false;
    }
    let cleaned = clean_text(response_text, 2_000);
    if cleaned.is_empty()
        || response_is_no_findings_placeholder(&cleaned)
        || response_looks_like_tool_ack_without_findings(&cleaned)
        || response_is_deferred_execution_preamble(&cleaned)
        || response_is_deferred_retry_prompt(&cleaned)
    {
        return false;
    }
    if !response_answers_user_early(message, &cleaned) {
        return false;
    }
    let lowered = cleaned.to_ascii_lowercase();
    let summary_text = clean_text(&response_tools_summary_for_user(response_tools, 4), 2_000);
    response_tools.iter().any(|row| {
        let mut evidence_texts = ["result", "summary", "display_text", "input", "name"]
            .iter()
            .map(|field| clean_text(row.get(*field).and_then(Value::as_str).unwrap_or(""), 2_000))
            .collect::<Vec<_>>();
        evidence_texts.push(clean_text(
            row.pointer("/tool_pipeline/raw_payload/summary")
                .and_then(Value::as_str)
                .unwrap_or(""),
            2_000,
        ));
        evidence_texts.push(summary_text.clone());
        evidence_texts.into_iter().any(|text| {
            text.split(|ch: char| !ch.is_ascii_alphanumeric())
                .map(|token| token.to_ascii_lowercase())
                .filter(|token| token.len() >= 5)
                .filter(|token| {
                    !matches!(
                        token.as_str(),
                        "result"
                            | "results"
                            | "query"
                            | "search"
                            | "source"
                            | "sources"
                            | "about"
                            | "https"
                            | "http"
                            | "tool"
                            | "tools"
                            | "using"
                            | "recorded"
                            | "evidence"
                    )
                })
                .any(|token| lowered.contains(&token))
        })
    })
}

fn response_tools_have_recorded_evidence_refs(response_tools: &[Value]) -> bool {
    response_tools.iter().any(|row| {
        row.get("evidence_refs")
            .or_else(|| row.pointer("/tool_result_quality/evidence_refs"))
            .and_then(Value::as_array)
            .map(|rows| rows.iter().any(recorded_evidence_ref_is_substantive))
            .unwrap_or(false)
            || tool_hidden_array(row, "evidence_pack")
                .iter()
                .any(recorded_evidence_ref_is_substantive)
            || tool_hidden_array(row, "evidence_pack_candidates")
                .iter()
                .any(recorded_evidence_ref_is_substantive)
            || row
                .pointer("/tool_result_quality/evidence_count")
                .and_then(Value::as_u64)
                .map(|count| count > 0)
                .unwrap_or(false)
    })
}

fn response_tools_can_project_compact_source_signal(response_tools: &[Value]) -> bool {
    response_tools_have_recorded_evidence_refs(response_tools)
}

fn recorded_evidence_ref_is_substantive(value: &Value) -> bool {
    if value.get("error").is_some() || value.get("status").and_then(Value::as_str) == Some("error")
    {
        return false;
    }
    let locator = clean_text(
        value
            .get("locator")
            .or_else(|| value.get("url"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        240,
    )
    .to_ascii_lowercase();
    if locator.starts_with("tool:no-results") || locator.starts_with("tool:low-signal") {
        return false;
    }
    let title = clean_text(
        value.get("title").and_then(Value::as_str).unwrap_or(""),
        200,
    )
    .to_ascii_lowercase();
    if title.contains("no usable result") || title.contains("no results") {
        return false;
    }
    value
        .get("score")
        .and_then(Value::as_f64)
        .map(|score| score > 0.0)
        .unwrap_or_else(|| !locator.is_empty() || !title.is_empty())
}

fn final_response_verifier_contract_marker(pointer: &str, text: &str) -> bool {
    workflow_message_matches_contract_markers(&default_workflow_tool_menu_contract(), pointer, text)
}

fn final_response_verifier_contract_marker_for_tools(
    response_tools: &[Value],
    pointer: &str,
    text: &str,
) -> bool {
    response_tools.iter().any(|tool| {
        let tool_key = tool
            .get("name")
            .or_else(|| tool.get("tool_name"))
            .or_else(|| tool.get("tool"))
            .and_then(Value::as_str)
            .map(|row| clean_text(row, 120))
            .unwrap_or_default();
        !tool_key.is_empty()
            && workflow_message_matches_contract_markers(
                &workflow_tool_menu_contract_for_tool_key(&tool_key),
                pointer,
                text,
            )
    }) || final_response_verifier_contract_marker(pointer, text)
}

fn response_violates_tool_backed_final_verifier(
    response_text: &str,
    response_tools: &[Value],
) -> bool {
    tool_backed_final_verifier_violation_reason(response_text, response_tools).is_some()
}

fn tool_backed_final_verifier_violation_reason(
    response_text: &str,
    response_tools: &[Value],
) -> Option<String> {
    if response_tools.is_empty() {
        return None;
    }
    let cleaned = clean_chat_text(response_text, 32_000);
    if cleaned.is_empty() {
        return None;
    }
    if response_contains_workflow_prompt_analysis_leak(&cleaned) {
        return Some("final_response_verifier_contract:internal_scaffold_leaked".to_string());
    }
    if response_looks_truncated_or_incomplete_for_verifier(&cleaned) {
        return Some("final_response_verifier_contract:incomplete_visible_answer".to_string());
    }
    if response_tools_have_answer_ready_evidence_packets(response_tools)
        && response_underuses_available_tool_evidence(&cleaned, response_tools)
    {
        return Some(
            "final_response_verifier_contract:answer_underdeveloped_for_available_evidence"
                .to_string(),
        );
    }
    if response_looks_like_materialization_error_as_answer(&cleaned) {
        return Some(
            "final_response_verifier_contract:materialization_error_substituted_for_answer"
                .to_string(),
        );
    }
    let first = first_sentence(&cleaned, 420).to_ascii_lowercase();
    let full = cleaned.to_ascii_lowercase();
    if response_tools_have_answer_ready_evidence_packets(response_tools)
        && response_looks_like_retrieval_recap_substituted_for_answer(&cleaned)
    {
        return Some(
            "final_response_verifier_contract:retrieval_recap_substituted_for_answer".to_string(),
        );
    }
    let status_first = final_response_verifier_contract_marker_for_tools(
        response_tools,
        "/diagnostic_markers/final_response_verifier/opening_status_phrases",
        &first,
    );
    let bounded_answer_first = final_response_verifier_contract_marker_for_tools(
        response_tools,
        "/diagnostic_markers/final_response_verifier/bounded_answer_signals",
        &first,
    );
    let claims_missing_evidence = response_tools_have_recorded_evidence_refs(response_tools)
        && final_response_verifier_contract_marker_for_tools(
            response_tools,
            "/diagnostic_markers/final_response_verifier/missing_evidence_claim_phrases",
            &full,
        );
    if claims_missing_evidence {
        return Some(
            "final_response_verifier_contract:claims_missing_recorded_evidence".to_string(),
        );
    }
    let outside_evidence_marker = final_response_verifier_contract_marker_for_tools(
        response_tools,
        "/diagnostic_markers/final_response_verifier/outside_evidence_source_boundary_phrases",
        &full,
    ) || ((full.contains("general knowledge") || full.contains("outside retrieved evidence"))
        && (full.contains("not source-backed") || full.contains("not supported by retrieved evidence")));
    let outside_evidence_used_for_decision = outside_evidence_marker
        && (final_response_verifier_contract_marker_for_tools(
                response_tools,
                "/diagnostic_markers/final_response_verifier/bounded_answer_signals",
                &full,
            )
            || full.contains("bottom line: choose")
            || full.contains(" choose ")
            || full.contains(" recommend "))
        && !workflow_final_answer_explicitly_refuses_unsupported_recommendation(&full);
    if outside_evidence_used_for_decision {
        return Some(
            "final_response_verifier_contract:outside_evidence_used_for_decision".to_string(),
        );
    }
    let missing_citation_signal = response_tools_have_recorded_evidence_refs(response_tools)
        && !response_has_evidence_tags(&cleaned)
        && !response_has_public_source_signal(&cleaned)
        && !response_tools_can_project_compact_source_signal(response_tools);
    if missing_citation_signal {
        return Some(
            "final_response_verifier_contract:missing_citation_or_source_signal".to_string(),
        );
    }
    if status_first && !bounded_answer_first {
        return Some("final_response_verifier_contract:status_before_answer".to_string());
    }
    let missing_lanes = response_missing_required_entity_lanes(&cleaned, response_tools);
    if !missing_lanes.is_empty() {
        return Some(format!(
            "final_response_verifier_contract:missing_coverage_lanes={}",
            missing_lanes.join(", ")
        ));
    }
    if response_tools_have_recorded_evidence_refs(response_tools)
        && !response_tools_have_answer_ready_evidence_packets(response_tools)
        && response_has_answer_unit_precision_traceability_violation(&cleaned, response_tools)
    {
        return Some(
            "final_response_verifier_contract:answer_units_not_traceable_to_evidence".to_string(),
        );
    }
    if response_tools_have_answer_ready_evidence_packets(response_tools)
        && response_has_answer_unit_traceability_violation(&cleaned, response_tools)
    {
        return Some(
            "final_response_verifier_contract:answer_units_not_traceable_to_evidence".to_string(),
        );
    }
    None
}

fn response_underuses_available_tool_evidence(response_text: &str, response_tools: &[Value]) -> bool {
    let available_units = evidence_packet_answer_units(response_tools, 4)
        .into_iter()
        .filter(|unit| {
            let (answer, _) = fallback_answer_unit_text_and_source(unit);
            !answer.is_empty()
                && evidence_packet_text_is_answer_claim(&answer)
                && !workflow_answer_unit_is_process_or_metadata_fact(&answer)
                && !workflow_answer_unit_contains_ui_or_source_shell(&answer)
                && !workflow_answer_unit_looks_like_source_title_fragment(&answer)
                && !workflow_answer_unit_looks_like_datestamped_headline_shell(&answer)
        })
        .take(3)
        .count();
    if available_units < 2 {
        return false;
    }
    let answer_units = workflow_answer_text_units(response_text)
        .into_iter()
        .filter(|unit| {
            let cleaned = clean_text(unit, 520);
            !cleaned.is_empty()
                && !workflow_answer_unit_is_process_or_metadata_fact(&cleaned)
                && !workflow_answer_unit_contains_ui_or_source_shell(&cleaned)
                && !workflow_answer_unit_looks_like_source_title_fragment(&cleaned)
                && !workflow_answer_unit_looks_like_datestamped_headline_shell(&cleaned)
                && evidence_packet_text_is_answer_claim(&cleaned)
        })
        .take(2)
        .count();
    if answer_units >= 2 {
        return false;
    }
    let lowered = normalize_coverage_lane_text(response_text);
    let honest_low_evidence_closure = workflow_answer_unit_is_hedged_or_gap(&lowered)
        && (lowered.contains("no usable evidence")
            || lowered.contains("not enough usable evidence")
            || lowered.contains("insufficient evidence")
            || lowered.contains("could not verify"));
    !honest_low_evidence_closure && response_text.split_whitespace().count() < 70
}

fn response_looks_like_materialization_error_as_answer(response_text: &str) -> bool {
    let lowered = normalize_coverage_lane_text(response_text);
    if lowered.is_empty() {
        return false;
    }
    let names_error_artifact = lowered.contains("error page")
        || lowered.contains("technical error")
        || lowered.contains("access denied")
        || lowered.contains("captcha")
        || lowered.contains("blocked page")
        || lowered.contains("browser challenge");
    let substitutes_for_answer = lowered.contains("returned only")
        || lowered.contains("yielding no usable")
        || lowered.contains("no usable scholarly content")
        || lowered.contains("no usable content")
        || lowered.contains("could not retrieve");
    names_error_artifact && substitutes_for_answer
}
