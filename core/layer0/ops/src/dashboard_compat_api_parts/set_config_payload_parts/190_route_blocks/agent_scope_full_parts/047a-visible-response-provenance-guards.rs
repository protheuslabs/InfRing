fn user_message_allows_code_context(user_message: &str) -> bool {
    let lowered = clean_text(user_message, 1_200).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    [
        "show me the code",
        "source code",
        "code snippet",
        "read file",
        "open file",
        "inspect file",
        "file content",
        "patch",
        "diff",
        "implementation",
        "function",
        "class",
        "php",
        "laravel",
        "typescript",
        "javascript",
        "rust",
        "python",
    ]
    .iter()
    .any(|needle| lowered.contains(*needle))
}

fn response_code_context_marker_count(response_text: &str) -> usize {
    response_text
        .lines()
        .filter(|line| {
            let lowered = line.trim_start().to_ascii_lowercase();
            lowered.starts_with("<?php")
                || lowered.starts_with("namespace ")
                || lowered.starts_with("use ")
                || lowered.starts_with("import ")
                || lowered.starts_with("from ")
                || lowered.starts_with("export ")
                || lowered.starts_with("def ")
                || lowered.starts_with("class ")
                || lowered.starts_with("public class ")
                || lowered.starts_with("public function ")
                || lowered.starts_with("private function ")
                || lowered.starts_with("protected function ")
                || lowered.starts_with("fn ")
                || lowered.starts_with("impl ")
                || lowered.starts_with("const ")
                || lowered.starts_with("let ")
                || lowered.starts_with("if (")
                || lowered.starts_with("$this->")
                || lowered.starts_with("return ")
        })
        .count()
}

fn response_contains_stale_code_context_dump(user_message: &str, response_text: &str) -> bool {
    let cleaned = clean_text(response_text, 32_000);
    if user_message_allows_code_context(user_message) {
        return false;
    }
    let lowered = cleaned.to_ascii_lowercase();
    if lowered.contains("<?php")
        && (lowered.contains("namespace ")
            || lowered.contains("session_start")
            || lowered.contains("class ")
            || lowered.contains("require_once"))
    {
        return true;
    }
    if cleaned.len() < 160 {
        return false;
    }
    let fenced_code = lowered.contains("```php")
        || lowered.contains("```ts")
        || lowered.contains("```js")
        || lowered.contains("```rust")
        || lowered.contains("```python");
    let php_dump = lowered.contains("<?php")
        && (lowered.contains("namespace ") || lowered.contains("extends serviceprovider"));
    let framework_dump = lowered.contains("class ")
        && lowered.contains("public function ")
        && (lowered.contains("namespace ") || lowered.contains("use "));
    php_dump
        || framework_dump
        || (fenced_code && response_code_context_marker_count(&cleaned) >= 2)
        || response_code_context_marker_count(&cleaned) >= 4
}

fn user_message_explicitly_requests_memory_context(message: &str) -> bool {
    let lowered = clean_text(message, 1_200).to_ascii_lowercase();
    memory_recall_requested(&lowered)
        || lowered.contains("memory")
        || lowered.contains("context")
        || lowered.contains("earlier")
        || lowered.contains("previous")
        || lowered.contains("what did we")
        || lowered.contains("what was decided")
        || lowered.contains("remember")
        || lowered.contains("recall")
}

fn simple_direct_chat_suppresses_passive_context(
    message: &str,
    inline_tools_allowed: bool,
) -> bool {
    (!inline_tools_allowed || message_explicitly_disallows_tool_calls(message))
        && !user_message_explicitly_requests_memory_context(message)
}

fn response_has_current_turn_tool_evidence(response_tools: &[Value]) -> bool {
    response_tools.iter().any(|row| {
        let name = normalize_tool_name(row.get("name").and_then(Value::as_str).unwrap_or(""));
        !name.is_empty() && name != "thought_process"
    })
}

fn response_claims_tool_success_without_current_turn_evidence(
    _user_message: &str,
    response_text: &str,
    response_tools: &[Value],
) -> bool {
    if response_has_current_turn_tool_evidence(response_tools) {
        return false;
    }
    let response = clean_chat_text(response_text, 32_000);
    let lowered = response.to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let contract = default_workflow_tool_menu_contract();
    let mentions_tool_surface = workflow_message_matches_contract_markers(
        &contract,
        "/diagnostic_markers/unsupported_tool_claim/tool_surface_terms",
        &lowered,
    );
    let claims_execution = workflow_message_matches_contract_markers(
        &contract,
        "/diagnostic_markers/unsupported_tool_claim/execution_claim_phrases",
        &lowered,
    );
    let claims_execution_action = [
        "i searched",
        "i ran",
        "i used",
        "i called",
        "i executed",
        "i opened",
        "i read",
        "i inspected",
        "i scanned",
        "i found",
        "tool ran",
        "tool succeeded",
        "tool completed",
        "search returned",
        "search found",
        "returned no findings",
        "returned no results",
        "found no results",
        "found no findings",
        "returned these",
        "returned the following",
    ]
    .iter()
    .any(|phrase| lowered.contains(phrase));
    let claims_empty_results = workflow_message_matches_contract_markers(
        &contract,
        "/diagnostic_markers/unsupported_tool_claim/empty_result_claim_phrases",
        &lowered,
    ) && workflow_message_matches_contract_markers(
        &contract,
        "/diagnostic_markers/unsupported_tool_claim/result_context_terms",
        &lowered,
    );
    let claims_listings = workflow_message_matches_contract_markers(
        &contract,
        "/diagnostic_markers/unsupported_tool_claim/listing_claim_phrases",
        &lowered,
    );
    let claims_tool_result = (mentions_tool_surface
        && ((claims_execution && claims_execution_action) || claims_empty_results))
        || claims_listings;
    let hypothetical = workflow_message_matches_contract_markers(
        &contract,
        "/diagnostic_markers/unsupported_tool_claim/hypothetical_phrases",
        &lowered,
    );
    if hypothetical && !claims_tool_result {
        return false;
    }
    claims_tool_result
}

fn response_has_gate_choice_prefix_leakage(response_text: &str) -> bool {
    let lowered = clean_text(response_text, 2_000).to_ascii_lowercase();
    let trimmed = lowered.trim();
    if trimmed.is_empty() {
        return false;
    }
    let compact = trimmed
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch.is_whitespace() {
                ch
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if workflow_message_matches_contract_markers(
        &default_workflow_tool_menu_contract(),
        "/diagnostic_markers/gate_choice_prefix_leakage_phrases",
        &compact,
    ) {
        return true;
    }
    let contract = default_workflow_tool_menu_contract();
    let tool_request_labels = workflow_tool_request_all_field_labels(&contract)
        .into_iter()
        .map(|label| label.to_ascii_lowercase())
        .collect::<Vec<_>>();
    !tool_request_labels.is_empty()
        && tool_request_labels
            .iter()
            .filter(|label| trimmed.contains(label.as_str()))
            .count()
            >= 3
}

fn response_contains_workflow_prompt_analysis_leak(response_text: &str) -> bool {
    let lowered = clean_text(response_text, 4_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    if workflow_message_matches_contract_markers(
        &default_workflow_tool_menu_contract(),
        "/diagnostic_markers/prompt_analysis_leak_phrases",
        &lowered,
    ) {
        return true;
    }
    [
        "i need to analyze the recorded evidence state",
        "recorded tool/evidence state",
        "recorded evidence state",
        "recorded_tool_",
        "recorded_evidence_",
        "recorded_quality_flags",
        "recorded_retry_",
        "recorded_answer_units",
        "synthesis input envelope",
        "tool boundary signals",
        "evidence_outcome_posture",
        "outcome posture",
        "supported_answer",
        "bounded_partial_answer",
        "evidence_insufficient_answer",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn response_contains_unrequested_content_without_tool_evidence(
    user_message: &str,
    response_text: &str,
    response_tools: &[Value],
) -> bool {
    if response_has_current_turn_tool_evidence(response_tools)
        || user_message_allows_code_context(user_message)
    {
        return false;
    }
    let cleaned = clean_chat_text(response_text, 32_000);
    if response_contains_short_unrelated_project_title(user_message, &cleaned) {
        return true;
    }
    let lowered = cleaned.to_ascii_lowercase();
    if lowered.contains("[tool:") || lowered.contains("<tool:") {
        return true;
    }
    if cleaned.len() < 160 {
        return false;
    }
    if response_claims_tool_success_without_current_turn_evidence(
        user_message,
        &cleaned,
        response_tools,
    ) {
        return true;
    }
    response_contains_stale_code_context_dump(user_message, &cleaned)
        || response_is_unrelated_context_dump(user_message, &cleaned)
        || response_contains_project_dump_sections(&cleaned)
        || response_contains_tool_telemetry_dump(&cleaned)
        || ((lowered.contains("```") || lowered.contains("<?php"))
            && response_code_context_marker_count(&cleaned) >= 1)
        || response_code_context_marker_count(&cleaned) >= 3
}

fn strict_current_turn_question_shape(user_message: &str) -> bool {
    let lowered = clean_text(user_message, 1_000).to_ascii_lowercase();
    lowered.contains('?')
        || lowered.starts_with("what")
        || lowered.starts_with("why")
        || lowered.starts_with("how")
        || lowered.starts_with("did")
        || lowered.starts_with("can")
        || lowered.starts_with("could")
        || lowered.starts_with("would")
        || lowered.contains("status")
        || lowered.contains("compare")
}

fn current_turn_dominance_payload(
    user_message: &str,
    response_text: &str,
    response_tools: &[Value],
) -> Value {
    let response = clean_chat_text(response_text, 32_000);
    let message_terms = important_memory_terms(user_message, 24)
        .into_iter()
        .collect::<HashSet<_>>();
    let response_terms = important_memory_terms(&response, 72)
        .into_iter()
        .collect::<HashSet<_>>();
    let overlap_count = message_terms.intersection(&response_terms).count();
    let no_tool_evidence = !response_has_current_turn_tool_evidence(response_tools);
    let contamination_detected = response_contains_unrequested_content_without_tool_evidence(
        user_message,
        &response,
        response_tools,
    );
    let low_overlap = no_tool_evidence
        && response.len() > 180
        && message_terms.len() >= 2
        && !response_terms.is_empty()
        && overlap_count == 0;
    let strict_answer_failure = no_tool_evidence
        && response.len() > 120
        && strict_current_turn_question_shape(user_message)
        && !response_answers_user_early(user_message, &response);
    let violation = !response.is_empty()
        && (contamination_detected
            || response_is_unrelated_context_dump(user_message, &response)
            || low_overlap
            || strict_answer_failure);
    json!({
        "contract": "current_turn_dominance_v1",
        "dominant": !violation,
        "violation": violation,
        "message_term_count": message_terms.len(),
        "response_term_count": response_terms.len(),
        "overlap_count": overlap_count,
        "current_turn_tool_evidence": !no_tool_evidence,
        "contamination_detected": contamination_detected,
        "low_overlap": low_overlap,
        "strict_answer_failure": strict_answer_failure
    })
}

fn response_current_turn_dominance_violation(
    user_message: &str,
    response_text: &str,
    response_tools: &[Value],
) -> bool {
    current_turn_dominance_payload(user_message, response_text, response_tools)
        .get("violation")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn response_guard_bool(report: &Value, key: &str) -> bool {
    report.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn final_response_guard_report(
    user_message: &str,
    response_text: &str,
    response_tools: &[Value],
    repair_candidate_contamination: bool,
) -> Value {
    let current_turn_dominance =
        current_turn_dominance_payload(user_message, response_text, response_tools);
    let current_turn_dominance_violation =
        response_guard_bool(&current_turn_dominance, "violation");
    let unsupported_content_contamination =
        response_contains_unrequested_content_without_tool_evidence(
            user_message,
            response_text,
            response_tools,
        );
    let unsupported_tool_success_claim = response_claims_tool_success_without_current_turn_evidence(
        user_message,
        response_text,
        response_tools,
    );
    let final_contamination_violation = repair_candidate_contamination
        || response_contains_stale_code_context_dump(user_message, response_text)
        || response_is_unrelated_context_dump(user_message, response_text)
        || unsupported_content_contamination;
    let visible_gate_choice_leakage = response_is_visible_workflow_gate_choice(response_text)
        || response_has_gate_choice_prefix_leakage(response_text);
    let final_contract_violation = response_fails_base_final_answer_contract(response_text)
        || (workflow_response_requests_more_tooling(response_text)
            && !response_is_manual_toolbox_gate_choice(response_text))
        || response_contains_unexpected_state_retry_boilerplate(response_text)
        || visible_gate_choice_leakage
        || unsupported_tool_success_claim
        || final_contamination_violation
        || current_turn_dominance_violation;
    json!({
        "current_turn_dominance": current_turn_dominance,
        "current_turn_dominance_violation": current_turn_dominance_violation,
        "visible_gate_choice_leakage": visible_gate_choice_leakage,
        "contamination_guard": {
            "contract": "unrequested_content_without_tool_evidence_v1",
            "detected": final_contamination_violation,
            "unsupported_content_detected": unsupported_content_contamination,
            "unsupported_tool_success_claim": unsupported_tool_success_claim,
            "current_turn_tool_evidence": response_has_current_turn_tool_evidence(response_tools)
        },
        "unsupported_tool_success_claim": unsupported_tool_success_claim,
        "final_contamination_violation": final_contamination_violation,
        "final_contract_violation": final_contract_violation
    })
}

fn final_response_guard_outcome(report: &Value) -> &'static str {
    if response_guard_bool(report, "unsupported_tool_success_claim") {
        "unsupported_tool_success_claim_flagged"
    } else if response_guard_bool(report, "final_contamination_violation") {
        "visible_response_contamination_flagged"
    } else if response_guard_bool(report, "current_turn_dominance_violation") {
        "current_turn_dominance_flagged"
    } else {
        "final_contract_violation_flagged"
    }
}

fn apply_response_guard_payloads(response_finalization: &mut Value, report: &Value) {
    response_finalization["current_turn_dominance"] = report
        .get("current_turn_dominance")
        .cloned()
        .unwrap_or_else(|| json!({}));
    response_finalization["contamination_guard"] = report
        .get("contamination_guard")
        .cloned()
        .unwrap_or_else(|| json!({}));
}
