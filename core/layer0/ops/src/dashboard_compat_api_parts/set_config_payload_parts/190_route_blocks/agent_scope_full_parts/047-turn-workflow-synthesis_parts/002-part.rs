fn workflow_workspace_tool_fallback_pattern_from_text(text: &str) -> String {
    let banned_terms = [
        "inspect",
        "identify",
        "smallest",
        "read",
        "reading",
        "before",
        "answering",
        "beforehand",
        "first",
        "after",
        "next",
        "ill",
        "make",
        "would",
        "could",
        "should",
        "workspace",
        "tool",
        "tools",
        "using",
        "use",
        "parse",
    ];
    let terms = important_memory_terms(text, 12)
        .into_iter()
        .filter(|term| !banned_terms.contains(&term.as_str()))
        .collect::<Vec<_>>();
    let pattern = terms.iter().take(5).cloned().collect::<Vec<_>>().join(" ");
    clean_text(&pattern, 220)
}

fn workflow_workspace_tool_request_inference(
    response_text: &str,
    message: &str,
    category_key: &str,
) -> Option<Value> {
    if normalized_workflow_token(category_key) != "workspace files" {
        return None;
    }
    let lowered = clean_text(&format!("{message} {response_text}"), 4_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return None;
    }
    if ![
        "inspect", "search", "read", "patch", "find", "open", "update", "fix", "bugfix",
    ]
    .iter()
    .any(|needle| lowered.contains(needle))
    {
        return None;
    }
    let tool_preference = if lowered.contains("file_read")
        || lowered.contains("read file")
        || lowered.contains("read this")
    {
        "file_read"
    } else if lowered.contains("apply patch") || lowered.contains("patch file") {
        "apply_patch"
    } else if lowered.contains("parse workspace") {
        "parse_workspace"
    } else {
        "workspace_search"
    };
    let tool_name = canonical_manual_toolbox_tool_name(category_key, tool_preference);
    if tool_name.is_empty() {
        return None;
    }
    let mut input = json!({});
    if tool_name == "workspace_search" {
        let mut pattern = workflow_workspace_tool_fallback_pattern_from_text(&format!(
            "{message} {response_text}"
        ));
        if pattern.is_empty() {
            pattern = "workspace bugfix".to_string();
        }
        input["path"] = json!(".");
        input["pattern"] = json!(pattern);
    } else if tool_name == "parse_workspace" {
        input["path"] = json!(".");
        input["operation"] = json!("inspect");
    } else if tool_name == "file_read" {
        input["path"] = json!(".");
    }
    if input.as_object().map(|obj| obj.is_empty()).unwrap_or(true) {
        return None;
    }
    Some(json!({
        "tool_family": category_key,
        "tool": tool_name,
        "source": "manual_toolbox_gate_inferred_request",
        "request_payload": input
    }))
}

fn direct_llm_response_from_initial_draft(draft_response: &str) -> Option<String> {
    if let Some(structured_final_answer) = workflow_structured_gate_final_answer(draft_response) {
        let cleaned = sanitize_workflow_visible_response_text(&structured_final_answer);
        if !cleaned.is_empty() {
            return Some(cleaned);
        }
    }
    let cleaned = sanitize_workflow_visible_response_text(draft_response);
    if cleaned.is_empty()
        || response_is_manual_toolbox_gate_choice(&cleaned)
        || response_is_visible_workflow_gate_choice(&cleaned)
        || response_has_gate_choice_prefix_leakage(&cleaned)
    {
        None
    } else {
        Some(cleaned)
    }
}

fn preserve_direct_llm_response_without_fallback(workflow: &mut Value, draft_response: &str) {
    if let Some(direct_response) = direct_llm_response_from_initial_draft(draft_response) {
        workflow["response"] = Value::String(direct_response);
        workflow["final_llm_response"]["used"] = Value::Bool(true);
        workflow["final_llm_response"]["status"] = Value::String("direct_llm_response".to_string());
        workflow["final_llm_response"]["source"] =
            Value::String("initial_llm_response".to_string());
        workflow["final_llm_response"]["runtime_interference_disabled"] = Value::Bool(true);
        workflow["final_llm_response"]["direct_response_preserved"] = Value::Bool(true);
    } else {
        workflow["final_llm_response"]["used"] = Value::Bool(false);
        workflow["final_llm_response"]["source"] = Value::String("none".to_string());
        workflow["final_llm_response"]["runtime_interference_disabled"] = Value::Bool(true);
        workflow["final_llm_response"]["direct_response_preserved"] = Value::Bool(false);
    }
}

fn workflow_has_recovered_pending_request(workflow: &Value) -> bool {
    workflow
        .get("manual_toolbox_pending_tool_request")
        .filter(|value| value.is_object())
        .is_some()
}

fn initial_visible_gate_choice_submission_allowed(
    response_tools: &[Value],
    initial_no_tool_category_submission: bool,
    workflow: &Value,
) -> bool {
    response_tools.is_empty()
        && (initial_no_tool_category_submission || workflow_has_recovered_pending_request(workflow))
}

fn record_workflow_diagnostic_event(workflow: &mut Value, reason: &str, stage: &str) {
    let cleaned_reason = clean_text(reason, 80);
    let cleaned_stage = clean_text(stage, 80);
    if cleaned_reason.is_empty() {
        return;
    }
    let mut reason_history = workflow
        .pointer("/final_llm_response/diagnostic_event_reasons")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if !reason_history.iter().any(|entry| {
        entry
            .as_str()
            .map(|value| value == cleaned_reason)
            .unwrap_or(false)
    }) {
        reason_history.push(Value::String(cleaned_reason.clone()));
        if reason_history.len() > 8 {
            let overflow = reason_history.len() - 8;
            reason_history.drain(0..overflow);
        }
    }
    let mut stage_history = workflow
        .pointer("/final_llm_response/diagnostic_event_stages")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if !cleaned_stage.is_empty()
        && !stage_history.iter().any(|entry| {
            entry
                .as_str()
                .map(|value| value == cleaned_stage)
                .unwrap_or(false)
        })
    {
        stage_history.push(Value::String(cleaned_stage.clone()));
        if stage_history.len() > 8 {
            let overflow = stage_history.len() - 8;
            stage_history.drain(0..overflow);
        }
    }
    let mut guard_events = workflow
        .pointer("/final_llm_response/diagnostic_event_events")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    guard_events.push(json!({
        "reason": cleaned_reason,
        "stage": cleaned_stage
    }));
    if guard_events.len() > 16 {
        let overflow = guard_events.len() - 16;
        guard_events.drain(0..overflow);
    }
    workflow["final_llm_response"]["diagnostic_event_reason"] = Value::String(cleaned_reason);
    workflow["final_llm_response"]["diagnostic_event_reasons"] = Value::Array(reason_history);
    workflow["final_llm_response"]["diagnostic_event_stages"] = Value::Array(stage_history.clone());
    let trigger_count = workflow
        .pointer("/quality_telemetry/diagnostic_event_trigger_count")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        + 1;
    let distinct_reason_count = workflow
        .pointer("/final_llm_response/diagnostic_event_reasons")
        .and_then(Value::as_array)
        .map(|rows| rows.len())
        .unwrap_or(0);
    let distinct_stage_count = stage_history.len();
    let multi_stage = stage_history.len() > 1;
    let (severity, requires_operator_review, escalation_reason, recommended_action) =
        workflow_diagnostic_summary_classification(trigger_count, distinct_stage_count);
    workflow["final_llm_response"]["diagnostic_event_multi_stage"] = Value::Bool(multi_stage);
    workflow["final_llm_response"]["diagnostic_event_events"] = Value::Array(guard_events);
    workflow["final_llm_response"]["diagnostic_event_last_stage"] = Value::String(cleaned_stage);
    workflow["final_llm_response"]["diagnostic_event_summary"] = json!({
        "trigger_count": trigger_count,
        "distinct_reason_count": distinct_reason_count,
        "distinct_stage_count": distinct_stage_count,
        "multi_stage": multi_stage,
        "severity": severity,
        "requires_operator_review": requires_operator_review,
        "escalation_reason": escalation_reason,
        "recommended_action": recommended_action
    });
    let stage_counter_key = workflow_diagnostic_stage_counter_key(stage);
    let reason_counter_key = workflow_diagnostic_reason_counter_key(reason);
    bump_workflow_quality_counter(workflow, &stage_counter_key);
    bump_workflow_quality_counter(workflow, &reason_counter_key);
    bump_workflow_quality_counter(workflow, "diagnostic_event_trigger_count");
}

fn workflow_diagnostic_stage_counter_key(stage: &str) -> String {
    let mut out = String::with_capacity(96);
    out.push_str("diagnostic_event_stage_");
    let mut previous_underscore = false;
    for ch in clean_text(stage, 80).chars() {
        let mapped = if ch.is_ascii_alphanumeric() { ch } else { '_' };
        if mapped == '_' {
            if !previous_underscore {
                out.push('_');
                previous_underscore = true;
            }
            continue;
        }
        out.push(mapped.to_ascii_lowercase());
        previous_underscore = false;
    }
    while out.ends_with('_') {
        out.pop();
    }
    if out.ends_with("_guard") {
        out.truncate(out.len() - "_guard".len());
    }
    if out == "diagnostic_event_stage" {
        "diagnostic_event_stage_unknown".to_string()
    } else if !out.ends_with("_diagnostic") {
        out.push_str("_diagnostic");
        out
    } else {
        out
    }
}

fn workflow_diagnostic_reason_counter_key(reason: &str) -> String {
    let mut out = String::with_capacity(96);
    out.push_str("diagnostic_event_reason_");
    let mut previous_underscore = false;
    for ch in clean_text(reason, 80).chars() {
        let mapped = if ch.is_ascii_alphanumeric() { ch } else { '_' };
        if mapped == '_' {
            if !previous_underscore {
                out.push('_');
                previous_underscore = true;
            }
            continue;
        }
        out.push(mapped.to_ascii_lowercase());
        previous_underscore = false;
    }
    while out.ends_with('_') {
        out.pop();
    }
    if out.ends_with("_guard") {
        out.truncate(out.len() - "_guard".len());
    }
    if out == "diagnostic_event_reason" {
        "diagnostic_event_reason_unknown".to_string()
    } else if !out.ends_with("_diagnostic") {
        out.push_str("_diagnostic");
        out
    } else {
        out
    }
}

fn workflow_diagnostic_summary_classification(
    trigger_count: u64,
    distinct_stage_count: usize,
) -> (&'static str, bool, &'static str, &'static str) {
    if trigger_count >= 3 || distinct_stage_count >= 3 {
        (
            "high",
            true,
            "high_trigger_or_stage_diversity",
            "operator_review_recommended",
        )
    } else if trigger_count >= 2 || distinct_stage_count >= 2 {
        (
            "moderate",
            false,
            "repeated_or_multi_stage_guard_activity",
            "monitor_and_continue_direct_mode",
        )
    } else {
        (
            "low",
            false,
            "single_guard_activation",
            "continue_direct_mode",
        )
    }
}

fn apply_final_retry_boilerplate_diagnostic(
    workflow: &mut Value,
    message: &str,
    latest_assistant_text: &str,
    response_tools: &[Value],
) {
    let response_text = workflow_visible_response_candidate(workflow);
    if response_text.is_empty()
        || !response_contains_unexpected_state_retry_boilerplate(&response_text)
    {
        return;
    }
    let _ = latest_assistant_text;
    let replacement_response = clean_text(
        &replacement_response_for_retry_boilerplate(message, response_tools),
        1_200,
    );
    workflow["quality_telemetry"]["final_fallback_used"] = Value::Bool(false);
    bump_workflow_quality_counter(workflow, "legacy_retry_template_detected");
    workflow["final_llm_response"]["used"] = Value::Bool(true);
    let replacement_response = persist_workflow_visible_response(workflow, &replacement_response);
    if replacement_response.is_empty() {
        return;
    }
    workflow["final_llm_response"]["status"] =
        Value::String("guard_violation_rewritten".to_string());
    workflow["final_llm_response"]["runtime_interference_disabled"] = Value::Bool(true);
    workflow["final_llm_response"]["visible_response_preserved"] = Value::Bool(false);
    workflow["final_llm_response"]["replacement_response_used"] = Value::Bool(true);
    workflow["final_llm_response"]["replacement_response_excerpt"] =
        Value::String(first_sentence(&replacement_response, 240));
    workflow["final_llm_response"]["error"] =
        Value::String("retry_boilerplate_detected".to_string());
    workflow["final_llm_response"]["last_reject_reason"] =
        Value::String("rewritten_user_visible_response".to_string());
    record_workflow_diagnostic_event(
        workflow,
        "retry_boilerplate_diagnostic",
        "final_retry_diagnostic",
    );
    set_turn_workflow_final_stage_status(workflow, "guard_violation_rewritten");
}

fn persist_workflow_visible_response(workflow: &mut Value, response_text: &str) -> String {
    let cleaned = workflow_final_visible_response_text(response_text);
    if cleaned.is_empty() {
        return cleaned;
    }
    workflow["response"] = Value::String(cleaned.clone());
    workflow["text"] = Value::String(cleaned.clone());
    workflow["message"] = Value::String(cleaned.clone());
    workflow["response_finalization"]["finalized_output"] = Value::String(cleaned.clone());
    workflow["response_finalization"]["final_output"] = Value::String(cleaned.clone());
    workflow["response_finalization"]["final_response"]["text"] = Value::String(cleaned.clone());
    workflow["response_workflow"]["final_llm_response"]["text"] = Value::String(cleaned.clone());
    cleaned
}

fn workflow_visible_response_candidate(workflow: &Value) -> String {
    let candidates = [
        workflow.get("response").and_then(Value::as_str),
        workflow.get("text").and_then(Value::as_str),
        workflow.get("message").and_then(Value::as_str),
        workflow
            .pointer("/response_finalization/finalized_output")
            .and_then(Value::as_str),
        workflow
            .pointer("/response_finalization/final_output")
            .and_then(Value::as_str),
        workflow
            .pointer("/response_finalization/final_response/text")
            .and_then(Value::as_str),
        workflow
            .pointer("/response_workflow/final_llm_response/text")
            .and_then(Value::as_str),
    ];
    candidates
        .into_iter()
        .flatten()
        .map(workflow_final_visible_response_text)
        .find(|candidate| !candidate.is_empty())
        .unwrap_or_default()
}

