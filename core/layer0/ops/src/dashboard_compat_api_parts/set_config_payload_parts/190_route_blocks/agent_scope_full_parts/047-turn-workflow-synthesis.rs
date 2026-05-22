fn turn_workflow_event(kind: &str, detail: Value) -> Value {
    json!({
        "kind": clean_text(kind, 80),
        "detail": detail
    })
}

fn bump_workflow_quality_counter(workflow: &mut Value, key: &str) {
    let pointer = format!("/quality_telemetry/{key}");
    let current = workflow
        .pointer(&pointer)
        .and_then(Value::as_u64)
        .unwrap_or(0);
    workflow["quality_telemetry"][key] = json!(current + 1);
}

fn response_tool_workflow_events(response_tools: &[Value]) -> Vec<Value> {
    let mut events = Vec::<Value>::new();
    let mut seen = HashSet::<String>::new();
    for tool in response_tools.iter().take(8) {
        let tool_name =
            normalize_tool_name(tool.get("name").and_then(Value::as_str).unwrap_or("tool"));
        if tool_name.is_empty() {
            continue;
        }
        let status = clean_text(tool.get("status").and_then(Value::as_str).unwrap_or(""), 80)
            .to_ascii_lowercase();
        let blocked = tool
            .get("blocked")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let is_error = tool
            .get("is_error")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let result = clean_text(
            tool.get("result").and_then(Value::as_str).unwrap_or(""),
            600,
        );
        let attempt_reason = clean_text(
            tool.pointer("/tool_attempt_receipt/reason")
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        );
        let attempt_backend = clean_text(
            tool.pointer("/tool_attempt_receipt/backend")
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        );
        let low_signal = !result.is_empty()
            && (response_looks_like_tool_ack_without_findings(&result)
                || response_is_no_findings_placeholder(&result)
                || response_looks_like_unsynthesized_web_snippet_dump(&result)
                || response_looks_like_raw_web_artifact_dump(&result));
        let event_kind = if blocked || matches!(status.as_str(), "blocked" | "policy_denied") {
            "tool_blocked"
        } else if matches!(status.as_str(), "timeout") {
            "tool_timeout"
        } else if is_error || matches!(status.as_str(), "error" | "failed" | "execution_error") {
            "tool_failed"
        } else if low_signal || matches!(status.as_str(), "no_results") {
            "tool_low_signal"
        } else {
            "tool_completed"
        };
        let key = format!("{tool_name}:{event_kind}:{status}:{attempt_reason}");
        if !seen.insert(key) {
            continue;
        }
        events.push(turn_workflow_event(
            event_kind,
            json!({
                "tool_name": tool_name,
                "status": status,
                "blocked": blocked,
                "is_error": is_error,
                "reason": attempt_reason,
                "backend": attempt_backend,
                "result_excerpt": first_sentence(&result, 220)
            }),
        ));
    }
    events
}

fn build_turn_workflow_events(
    response_tools: &[Value],
    pending_confirmation: Option<&Value>,
    replayed_pending_confirmation: bool,
) -> Vec<Value> {
    let mut events = response_tool_workflow_events(response_tools);
    if let Some(pending) = pending_confirmation {
        let tool_name = clean_text(
            pending
                .get("tool_name")
                .or_else(|| pending.get("tool"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        );
        let source = clean_text(
            pending.get("source").and_then(Value::as_str).unwrap_or(""),
            80,
        );
        events.push(turn_workflow_event(
            "pending_confirmation_required",
            json!({
                "tool_name": tool_name,
                "source": source
            }),
        ));
    }
    if replayed_pending_confirmation {
        events.push(turn_workflow_event(
            "pending_confirmation_replayed",
            json!({"ok": true}),
        ));
    }
    events
}

fn workflow_final_response_status(workflow: &Value) -> String {
    clean_text(
        workflow
            .pointer("/final_llm_response/status")
            .and_then(Value::as_str)
            .unwrap_or(""),
        80,
    )
}

fn workflow_final_response_used(workflow: &Value) -> bool {
    workflow
        .pointer("/final_llm_response/used")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && workflow
            .get("response")
            .and_then(Value::as_str)
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
}

fn workflow_diagnostic_marker_count(response_text: &str, marker_key: &str) -> usize {
    let lowered = clean_text(response_text, 8_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return 0;
    }
    let marker_key = clean_text(marker_key, 120);
    if marker_key.is_empty() {
        return 0;
    }
    let pointer = format!("/diagnostic_markers/{marker_key}");
    default_workflow_tool_menu_contract()
        .pointer(&pointer)
        .and_then(Value::as_array)
        .map(|markers| {
            markers
                .iter()
                .filter_map(Value::as_str)
                .map(|marker| clean_text(marker, 240).to_ascii_lowercase())
                .filter(|marker| !marker.is_empty())
                .filter(|marker| lowered.contains(marker))
                .count()
        })
        .unwrap_or(0)
}

fn response_contains_route_classification_retry_template(lowered: &str) -> bool {
    workflow_diagnostic_marker_count(lowered, "legacy_retry_templates") > 0
}

fn workflow_response_repetition_breaker_active(latest_assistant_text: &str) -> bool {
    response_contains_unexpected_state_retry_boilerplate(latest_assistant_text)
}

fn recent_assistant_retry_loop_detected(active_messages: &[Value]) -> bool {
    let mut assistant_turns_scanned = 0usize;
    let mut retry_boilerplate_turns = 0usize;
    for row in active_messages.iter().rev() {
        let role = clean_text(row.get("role").and_then(Value::as_str).unwrap_or(""), 24)
            .to_ascii_lowercase();
        if role != "assistant" && role != "agent" {
            continue;
        }
        let text = clean_chat_text(
            row.get("text")
                .or_else(|| row.get("content"))
                .or_else(|| row.get("message"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            32_000,
        );
        if text.is_empty() {
            continue;
        }
        assistant_turns_scanned += 1;
        if response_contains_unexpected_state_retry_boilerplate(&text)
            || workflow_response_repetition_breaker_active(&text)
        {
            retry_boilerplate_turns += 1;
        }
        if assistant_turns_scanned >= 3 {
            break;
        }
    }
    retry_boilerplate_turns >= 2
}

fn workflow_retry_macro_signal_count(lowered: &str) -> usize {
    workflow_diagnostic_marker_count(lowered, "legacy_retry_templates")
}

fn response_contains_unexpected_state_retry_boilerplate(response_text: &str) -> bool {
    workflow_retry_macro_signal_count(response_text) > 0
}

fn tooling_failure_diagnostic_detected(
    message: &str,
    finalized_response: &str,
    latest_assistant_response: &str,
) -> bool {
    let failure_shaped = response_is_no_findings_placeholder(finalized_response)
        || response_looks_like_tool_ack_without_findings(finalized_response)
        || response_mentions_context_guard(finalized_response);
    if !failure_shaped {
        return false;
    }
    let asks_diagnosis = message_requests_tooling_failure_diagnosis(message);
    let repeated_placeholder = !latest_assistant_response.trim().is_empty()
        && response_is_no_findings_placeholder(latest_assistant_response)
        && normalize_placeholder_signature(latest_assistant_response)
            == normalize_placeholder_signature(finalized_response);
    asks_diagnosis || repeated_placeholder || response_mentions_context_guard(finalized_response)
}

fn workflow_policy_block_summary(response_tools: &[Value]) -> String {
    for row in response_tools {
        let blocked = row.get("blocked").and_then(Value::as_bool).unwrap_or(false);
        let status = clean_text(row.get("status").and_then(Value::as_str).unwrap_or(""), 240)
            .to_ascii_lowercase();
        let result = clean_text(row.get("result").and_then(Value::as_str).unwrap_or(""), 480);
        let error = clean_text(row.get("error").and_then(Value::as_str).unwrap_or(""), 480);
        let result_lower = result.to_ascii_lowercase();
        let error_lower = error.to_ascii_lowercase();
        let domain_boundary_block = status.contains("client_ingress_domain_boundary")
            || status.contains("domain_boundary")
            || result_lower.contains("client_ingress_domain_boundary")
            || result_lower.contains("domain_boundary")
            || error_lower.contains("client_ingress_domain_boundary")
            || error_lower.contains("domain_boundary");
        let file_list_boundary_block = result_lower.contains("file_list")
            && (result_lower.contains("ingress delivery policy")
                || result_lower.contains("domain_boundary")
                || result_lower.contains("lease_denied"));
        let is_policy_like = blocked
            || status.contains("lease_denied")
            || status.contains("policy_denied")
            || result_lower.contains("lease_denied")
            || error_lower.contains("lease_denied")
            || domain_boundary_block
            || file_list_boundary_block;
        if !is_policy_like {
            continue;
        }
        let tool_name =
            normalize_tool_name(row.get("name").and_then(Value::as_str).unwrap_or("tool"));
        let reason = if file_list_boundary_block {
            "file_list blocked by ingress delivery policy boundary".to_string()
        } else if domain_boundary_block {
            "workspace/file tooling blocked by ingress domain-boundary policy".to_string()
        } else if result.is_empty() {
            if error.is_empty() {
                "policy gate denied tool execution".to_string()
            } else {
                first_sentence(&error, 140)
            }
        } else {
            first_sentence(&result, 140)
        };
        if tool_name.is_empty() {
            return reason;
        }
        return format!("{tool_name}: {reason}");
    }
    String::new()
}

fn workflow_turn_has_policy_block(response_tools: &[Value]) -> bool {
    response_tools.iter().any(|row| {
        row.get("blocked").and_then(Value::as_bool).unwrap_or(false)
            || row
                .get("status")
                .and_then(Value::as_str)
                .map(|raw| raw.to_lowercase().contains("lease_denied"))
                .unwrap_or(false)
            || row
                .get("result")
                .and_then(Value::as_str)
                .map(|raw| raw.to_lowercase().contains("lease_denied"))
                .unwrap_or(false)
            || row
                .get("error")
                .and_then(Value::as_str)
                .map(|raw| {
                    let lowered = raw.to_ascii_lowercase();
                    lowered.contains("lease_denied")
                        || lowered.contains("domain_boundary")
                        || lowered.contains("client_ingress_domain_boundary")
                })
                .unwrap_or(false)
            || row
                .get("result")
                .and_then(Value::as_str)
                .map(|raw| {
                    let lowered = raw.to_ascii_lowercase();
                    lowered.contains("domain_boundary")
                        || lowered.contains("client_ingress_domain_boundary")
                        || (lowered.contains("file_list")
                            && lowered.contains("ingress delivery policy"))
                })
                .unwrap_or(false)
    })
}

fn workflow_turn_has_domain_boundary_block(response_tools: &[Value]) -> bool {
    response_tools.iter().any(|row| {
        row.get("status")
            .and_then(Value::as_str)
            .map(|raw| {
                let lowered = raw.to_ascii_lowercase();
                lowered.contains("domain_boundary")
                    || lowered.contains("client_ingress_domain_boundary")
            })
            .unwrap_or(false)
            || row
                .get("result")
                .and_then(Value::as_str)
                .map(|raw| {
                    let lowered = raw.to_ascii_lowercase();
                    lowered.contains("domain_boundary")
                        || lowered.contains("client_ingress_domain_boundary")
                        || (lowered.contains("file_list")
                            && lowered.contains("ingress delivery policy"))
                })
                .unwrap_or(false)
            || row
                .get("error")
                .and_then(Value::as_str)
                .map(|raw| {
                    let lowered = raw.to_ascii_lowercase();
                    lowered.contains("domain_boundary")
                        || lowered.contains("client_ingress_domain_boundary")
                })
                .unwrap_or(false)
    })
}

fn normalized_response_similarity_key(text: &str) -> String {
    let lowered = clean_text(text, 8_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return String::new();
    }
    let mut out = String::with_capacity(lowered.len());
    let mut previous_space = false;
    for ch in lowered.chars() {
        let mapped = if ch.is_ascii_alphanumeric() { ch } else { ' ' };
        if mapped == ' ' {
            if !previous_space {
                out.push(' ');
                previous_space = true;
            }
            continue;
        }
        out.push(mapped);
        previous_space = false;
    }
    out.trim().to_string()
}

fn response_repeats_latest_assistant_copy(
    response_text: &str,
    latest_assistant_text: &str,
) -> bool {
    let cleaned_response = sanitize_workflow_visible_response_text(response_text);
    let cleaned_latest = sanitize_workflow_visible_response_text(latest_assistant_text);
    let normalized_response = normalized_response_similarity_key(&cleaned_response);
    let normalized_latest = normalized_response_similarity_key(&cleaned_latest);
    let compact_response = normalized_response.replace(' ', "");
    let compact_latest = normalized_latest.replace(' ', "");
    let response_first_sentence = first_sentence(&cleaned_response, 200);
    let latest_first_sentence = first_sentence(&cleaned_latest, 200);
    let normalized_contains = normalized_response.len() >= 48
        && normalized_latest.len() >= 48
        && (normalized_response.contains(&normalized_latest)
            || normalized_latest.contains(&normalized_response));
    let compact_contains = compact_response.len() >= 48
        && compact_latest.len() >= 48
        && (compact_response.contains(&compact_latest)
            || compact_latest.contains(&compact_response));
    let first_sentence_match = response_first_sentence.len() >= 40
        && latest_first_sentence.len() >= 40
        && response_first_sentence.eq_ignore_ascii_case(&latest_first_sentence);
    !cleaned_response.is_empty()
        && !cleaned_latest.is_empty()
        && !normalized_response.is_empty()
        && !normalized_latest.is_empty()
        && cleaned_response.len() >= 24
        && (cleaned_response.eq_ignore_ascii_case(&cleaned_latest)
            || normalized_response == normalized_latest
            || normalized_contains
            || compact_response == compact_latest
            || compact_contains
            || first_sentence_match)
}

fn should_record_workflow_failure_diagnostic(
    last_reject_reason: &str,
    last_invalid_excerpt: &str,
    latest_assistant_text: &str,
    response_tools: &[Value],
    recent_retry_loop_detected: bool,
) -> bool {
    last_reject_reason == "unexpected_state_retry_boilerplate"
        || response_contains_unexpected_state_retry_boilerplate(last_invalid_excerpt)
        || workflow_response_repetition_breaker_active(latest_assistant_text)
        || recent_retry_loop_detected
        || workflow_turn_has_policy_block(response_tools)
}

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
    workflow["response"] = Value::String(replacement_response.clone());
    workflow["text"] = Value::String(replacement_response.clone());
    workflow["message"] = Value::String(replacement_response.clone());
    workflow["response_finalization"]["finalized_output"] =
        Value::String(replacement_response.clone());
    workflow["response_finalization"]["final_output"] =
        Value::String(replacement_response.clone());
    workflow["response_finalization"]["final_response"]["text"] =
        Value::String(replacement_response.clone());
    workflow["response_workflow"]["final_llm_response"]["text"] =
        Value::String(replacement_response.clone());
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

fn apply_final_empty_response_diagnostic(
    workflow: &mut Value,
    message: &str,
    latest_assistant_text: &str,
    response_tools: &[Value],
) {
    let response_text = clean_text(
        workflow
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or(""),
        32_000,
    );
    if !response_text.is_empty() {
        return;
    }
    let _ = latest_assistant_text;
    let fallback_response = clean_text(
        &fallback_final_response_from_tool_evidence(message, response_tools),
        3_000,
    );
    if !fallback_response.is_empty() {
        apply_tool_evidence_fallback_response(
            workflow,
            response_tools,
            &fallback_response,
            "tool_evidence_runtime_fallback",
            "empty_response_replaced_from_tool_evidence",
            None,
            None,
            "tool_evidence_runtime_fallback_used",
            "final_presence_diagnostic",
        );
        return;
    }

    workflow["quality_telemetry"]["final_fallback_used"] = Value::Bool(false);
    workflow["final_llm_response"]["used"] = Value::Bool(false);
    workflow["final_llm_response"]["status"] = Value::String("empty_llm_response".to_string());
    workflow["final_llm_response"]["runtime_interference_disabled"] = Value::Bool(true);
    workflow["final_llm_response"]["visible_response_preserved"] = Value::Bool(false);
    workflow["final_llm_response"]["error"] = Value::String("empty_response".to_string());
    workflow["final_llm_response"]["last_reject_reason"] =
        Value::String("diagnostic_only_presence".to_string());
    record_workflow_diagnostic_event(
        workflow,
        "empty_response_presence_diagnostic",
        "final_presence_diagnostic",
    );
    set_turn_workflow_final_stage_status(workflow, "empty_llm_response");
}

fn fallback_coverage_lane_sentence(response_tools: &[Value]) -> String {
    let lanes = synthesis_coverage_lanes_for_tools(response_tools, 12);
    if lanes.is_empty() {
        return String::new();
    }
    let lane_label = |row: &Value| -> String {
        let kind = clean_text(row.get("kind").and_then(Value::as_str).unwrap_or(""), 80);
        let requested = clean_text(
            row.get("requested_text")
                .and_then(Value::as_str)
                .unwrap_or(""),
            180,
        );
        if requested.is_empty() {
            String::new()
        } else if kind == "entity" {
            requested
        } else {
            requested
        }
    };
    let covered = lanes
        .iter()
        .filter(|row| {
            matches!(
                row.get("status").and_then(Value::as_str),
                Some("covered") | Some("usable")
            )
        })
        .filter_map(|row| {
            let label = lane_label(row);
            if label.is_empty() {
                None
            } else {
                Some(label)
            }
        })
        .take(4)
        .collect::<Vec<_>>();
    let weak_or_missing = lanes
        .iter()
        .filter(|row| {
            !matches!(
                row.get("status").and_then(Value::as_str),
                Some("covered") | Some("usable")
            )
        })
        .filter_map(|row| {
            let label = lane_label(row);
            if label.is_empty() {
                None
            } else {
                Some(label)
            }
        })
        .take(8)
        .collect::<Vec<_>>();
    if !covered.is_empty() && !weak_or_missing.is_empty() {
        format!(
            "Coverage state: usable evidence is present for {}; weak or missing coverage remains for {}.",
            covered.join(", "),
            weak_or_missing.join(", ")
        )
    } else if !covered.is_empty() {
        format!(
            "Coverage state: usable evidence is present for {}.",
            covered.join(", ")
        )
    } else if !weak_or_missing.is_empty() {
        format!(
            "Coverage gaps still matter for: {}.",
            weak_or_missing.join(", ")
        )
    } else {
        String::new()
    }
}

fn response_tools_have_recorded_material(response_tools: &[Value]) -> bool {
    response_tools.iter().any(|tool| {
        !clean_text(tool.get("result").and_then(Value::as_str).unwrap_or(""), 240).is_empty()
            || tool_hidden_array_len(tool, "search_results") > 0
            || tool_hidden_array_len(tool, "provider_results") > 0
            || tool_hidden_array_len(tool, "evidence_refs") > 0
            || tool_hidden_array_len(tool, "evidence_pack") > 0
            || tool_hidden_array_len(tool, "evidence_pack_candidates") > 0
    })
}

fn tool_evidence_outcome_posture(response_tools: &[Value]) -> &'static str {
    if response_tools.is_empty() || !response_tools_have_recorded_material(response_tools) {
        return "evidence_insufficient_answer";
    }
    let weak_or_missing_lane_count = synthesis_coverage_lanes_for_tools(response_tools, 16)
        .iter()
        .filter(|row| {
            !matches!(
                row.get("status").and_then(Value::as_str),
                Some("covered") | Some("usable")
            )
        })
        .count();
    let has_low_signal_or_failure = response_tools.iter().any(|tool| {
        let status = tool.get("status").and_then(Value::as_str).unwrap_or("");
        let quality_flags = tool_result_quality_object(tool)
            .map(|quality| tool_quality_string_array(quality, "/flags", 16))
            .unwrap_or_default();
        matches!(status, "low_signal" | "no_results" | "error" | "failed" | "timeout" | "blocked")
            || tool_quality_retry_recommended(tool)
            || quality_flags.iter().any(|flag| {
                matches!(
                    flag.as_str(),
                    "insufficient_evidence"
                        | "low_signal"
                        | "low_relevance_filtered"
                        | "comparison_evidence_insufficient"
                        | "weak_single_source"
                )
            })
    });
    if has_low_signal_or_failure || weak_or_missing_lane_count > 0 {
        "bounded_partial_answer"
    } else {
        "supported_answer"
    }
}

fn evidence_packet_text_field(row: &Value, keys: &[&str], max_len: usize) -> String {
    for key in keys {
        let value = clean_text(row.get(*key).and_then(Value::as_str).unwrap_or(""), max_len);
        if !value.is_empty() {
            return value;
        }
    }
    String::new()
}

fn evidence_packet_first_string(value: Option<&Value>, max_len: usize) -> String {
    match value {
        Some(Value::String(raw)) => clean_text(raw, max_len),
        Some(Value::Array(rows)) => rows
            .iter()
            .find_map(|row| {
                let value = evidence_packet_first_string(Some(row), max_len);
                (!value.is_empty()).then_some(value)
            })
            .unwrap_or_default(),
        Some(Value::Object(map)) => {
            for key in ["claim", "text", "summary", "snippet", "relevant_extract"] {
                let value = clean_text(map.get(key).and_then(Value::as_str).unwrap_or(""), max_len);
                if !value.is_empty() {
                    return value;
                }
            }
            String::new()
        }
        _ => String::new(),
    }
}

fn evidence_packet_claim_text(row: &Value) -> String {
    let claim = evidence_packet_first_string(row.get("claim_hints"), 260);
    if !claim.is_empty() {
        return claim;
    }
    let claim = evidence_packet_first_string(row.get("evidence_claims"), 260);
    if !claim.is_empty() {
        return claim;
    }
    evidence_packet_text_field(row, &["claim", "finding", "summary"], 260)
}

fn evidence_packet_source_label(row: &Value) -> String {
    let title = evidence_packet_text_field(row, &["title", "source_title", "source_ref"], 120);
    let domain = evidence_packet_text_field(row, &["source_domain", "domain"], 80);
    let locator = evidence_packet_text_field(row, &["locator", "url", "link"], 160);
    if !title.is_empty() && !domain.is_empty() {
        format!("{title}, {domain}")
    } else if !title.is_empty() {
        title
    } else if !domain.is_empty() {
        domain
    } else {
        locator
    }
}

fn evidence_packet_counts_as_usable(row: &Value) -> bool {
    if row.get("counts_as_usable_evidence").and_then(Value::as_bool) == Some(false) {
        return false;
    }
    let confidence = clean_text(row.get("confidence").and_then(Value::as_str).unwrap_or(""), 80)
        .to_ascii_lowercase();
    !matches!(
        confidence.as_str(),
        "candidate_only" | "low_confidence_raw" | "rejected"
    )
}

fn evidence_packet_answer_unit(row: &Value) -> Option<String> {
    if !evidence_packet_counts_as_usable(row) {
        return None;
    }
    let claim = evidence_packet_claim_text(row);
    let extract = evidence_packet_text_field(
        row,
        &[
            "relevant_extract",
            "support_snippet",
            "snippet",
            "summary",
            "content",
        ],
        360,
    );
    let answer_text = if !claim.is_empty() {
        claim
    } else {
        first_sentence(&extract, 260)
    };
    if answer_text.is_empty() {
        return None;
    }
    let source = evidence_packet_source_label(row);
    let unit = if source.is_empty() {
        answer_text
    } else {
        format!("{answer_text} Source: {source}.")
    };
    Some(clean_text(&unit, 520))
}

fn evidence_packet_answer_units(response_tools: &[Value], limit: usize) -> Vec<String> {
    let mut units = Vec::<String>::new();
    let mut seen = std::collections::HashSet::<String>::new();
    let limit = limit.clamp(1, 8);
    for tool in response_tools {
        for key in ["evidence_pack", "evidence_refs", "evidence_pack_candidates"] {
            for row in tool_hidden_array(tool, key) {
                let Some(unit) = evidence_packet_answer_unit(&row) else {
                    continue;
                };
                let dedupe_key = unit.to_ascii_lowercase();
                if seen.insert(dedupe_key) {
                    units.push(unit);
                }
                if units.len() >= limit {
                    return units;
                }
            }
        }
    }
    units
}

fn response_tools_have_answer_ready_evidence_packets(response_tools: &[Value]) -> bool {
    !evidence_packet_answer_units(response_tools, 1).is_empty()
}

fn annotate_final_evidence_outcome_posture(workflow: &mut Value, response_tools: &[Value]) {
    let posture = tool_evidence_outcome_posture(response_tools);
    workflow["final_llm_response"]["evidence_outcome_posture"] =
        Value::String(posture.to_string());
    workflow["quality_telemetry"]["evidence_outcome_posture"] =
        Value::String(posture.to_string());
}

fn fallback_final_response_from_tool_evidence(message: &str, response_tools: &[Value]) -> String {
    let _ = message;
    let answer_units = evidence_packet_answer_units(response_tools, 4);
    if !answer_units.is_empty() {
        let coverage_note = clean_text(
            &first_sentence(&fallback_coverage_lane_sentence(response_tools), 280),
            320,
        );
        let mut parts = vec![
            "Based on the retrieved evidence, the strongest supported answer is:".to_string(),
        ];
        for unit in answer_units {
            parts.push(format!("- {unit}"));
        }
        if !coverage_note.is_empty() {
            parts.push(format!("Limit: {coverage_note}"));
        }
        return clean_text(&parts.join("\n"), 2_400);
    }
    let failure_reason = clean_text(
        &first_sentence(&response_tools_failure_reason_for_user(response_tools, 4), 320),
        360,
    );
    let findings = clean_text(
        &first_sentence(&response_tools_summary_for_user(response_tools, 4), 420),
        480,
    );
    if findings.is_empty() && failure_reason.is_empty() {
        return String::new();
    }
    let coverage_note = clean_text(
        &first_sentence(&fallback_coverage_lane_sentence(response_tools), 280),
        320,
    );
    let opening = if !findings.is_empty() {
        "The practical answer is that the current evidence supports only a partial conclusion."
    } else {
        "My recommendation is to treat the current evidence as insufficient for a direct source-backed conclusion."
    };
    let mut parts = vec![opening.to_string()];
    if !failure_reason.is_empty() {
        parts.push(failure_reason);
    }
    if !findings.is_empty() {
        parts.push(findings);
    }
    if !coverage_note.is_empty() {
        parts.push(coverage_note);
    }
    clean_text(&parts.join(" "), 900)
}

fn apply_tool_evidence_fallback_response(
    workflow: &mut Value,
    response_tools: &[Value],
    fallback_response: &str,
    fallback_source: &str,
    error_code: &str,
    original_reject_reason: Option<&str>,
    original_reject_excerpt: Option<&str>,
    diagnostic_reason: &str,
    diagnostic_stage: &str,
) {
    let cleaned_response = clean_text(fallback_response, 3_000);
    if cleaned_response.is_empty() {
        return;
    }
    workflow["quality_telemetry"]["final_fallback_used"] = Value::Bool(true);
    workflow["quality_telemetry"]["final_fallback_suppressed"] = Value::Bool(false);
    workflow["final_llm_response"]["used"] = Value::Bool(true);
    workflow["response"] = Value::String(cleaned_response.clone());
    workflow["text"] = Value::String(cleaned_response.clone());
    workflow["message"] = Value::String(cleaned_response.clone());
    workflow["response_finalization"]["finalized_output"] = Value::String(cleaned_response.clone());
    workflow["response_finalization"]["final_output"] = Value::String(cleaned_response.clone());
    workflow["response_finalization"]["final_response"]["text"] =
        Value::String(cleaned_response.clone());
    workflow["response_workflow"]["final_llm_response"]["text"] =
        Value::String(cleaned_response.clone());
    workflow["final_llm_response"]["status"] =
        Value::String("tool_evidence_fallback_used".to_string());
    workflow["final_llm_response"]["runtime_interference_disabled"] = Value::Bool(true);
    workflow["final_llm_response"]["visible_response_preserved"] = Value::Bool(false);
    workflow["final_llm_response"]["fallback_source"] =
        Value::String(clean_text(fallback_source, 120));
    workflow["final_llm_response"]["replacement_response_used"] = Value::Bool(true);
    workflow["final_llm_response"]["replacement_response_excerpt"] =
        Value::String(first_sentence(&cleaned_response, 240));
    workflow["final_llm_response"]["error"] = Value::String(clean_text(error_code, 160));
    workflow["final_llm_response"]["last_reject_reason"] =
        Value::String("rewritten_user_visible_response".to_string());
    annotate_final_evidence_outcome_posture(workflow, response_tools);
    if let Some(reason) = original_reject_reason {
        let cleaned = clean_text(reason, 240);
        if !cleaned.is_empty() {
            workflow["final_llm_response"]["original_reject_reason"] = Value::String(cleaned);
        }
    }
    if let Some(excerpt) = original_reject_excerpt {
        let cleaned = clean_text(excerpt, 600);
        if !cleaned.is_empty() {
            workflow["final_llm_response"]["original_reject_excerpt"] = Value::String(cleaned);
        }
    }
    record_workflow_diagnostic_event(workflow, diagnostic_reason, diagnostic_stage);
    set_turn_workflow_final_stage_status(workflow, "guard_violation_rewritten");
}

fn maybe_apply_rejected_tool_evidence_fallback(
    workflow: &mut Value,
    message: &str,
    response_tools: &[Value],
    last_invalid_response_text: &str,
    last_invalid_excerpt: &str,
    last_reject_reason: &str,
) -> bool {
    if response_tools.is_empty() || last_invalid_excerpt.trim().is_empty() {
        return false;
    }
    if maybe_preserve_rejected_synthesis_with_coverage_note(
        workflow,
        response_tools,
        last_invalid_response_text,
        last_invalid_excerpt,
        last_reject_reason,
    ) {
        return true;
    }
    let fallback_response = clean_text(
        &fallback_final_response_from_tool_evidence(message, response_tools),
        3_000,
    );
    if fallback_response.is_empty() {
        return false;
    }
    apply_tool_evidence_fallback_response(
        workflow,
        response_tools,
        &fallback_response,
        "tool_evidence_runtime_fallback_after_verifier_reject",
        "rejected_response_replaced_from_tool_evidence",
        Some(last_reject_reason),
        Some(last_invalid_excerpt),
        "tool_evidence_verifier_reject_rewritten",
        "synthesis_failure_diagnostic",
    );
    true
}

fn final_verifier_reject_reason_missing_coverage_lanes(reason: &str) -> bool {
    clean_text(reason, 240)
        .to_ascii_lowercase()
        .starts_with("final_response_verifier_contract:missing_coverage_lanes=")
}

fn missing_coverage_lanes_from_reject_reason(reason: &str) -> Vec<String> {
    let cleaned = clean_text(reason, 1_000);
    let Some((_, lanes)) = cleaned.split_once("missing_coverage_lanes=") else {
        return Vec::new();
    };
    lanes
        .split(',')
        .map(|lane| clean_text(lane, 120))
        .filter(|lane| !lane.is_empty())
        .take(8)
        .collect()
}

fn missing_coverage_lane_note_from_reject_reason(reason: &str) -> String {
    let lanes = missing_coverage_lanes_from_reject_reason(reason);
    if lanes.is_empty() {
        return String::new();
    }
    clean_text(
        &format!(
            "Coverage note: the available evidence did not separately support {}; treat those as weak or folded-in lanes rather than independently proven claims.",
            lanes.join(", ")
        ),
        500,
    )
}

fn rejected_synthesized_response_is_salvageable(
    response_text: &str,
    response_tools: &[Value],
) -> bool {
    let cleaned = clean_chat_text(response_text, 32_000);
    if cleaned.is_empty()
        || response_is_no_findings_placeholder(&cleaned)
        || response_looks_like_tool_ack_without_findings(&cleaned)
        || response_is_deferred_execution_preamble(&cleaned)
        || response_is_deferred_retry_prompt(&cleaned)
        || response_contains_unexpected_state_retry_boilerplate(&cleaned)
        || response_looks_like_raw_tool_payload_dump(&cleaned)
        || response_looks_like_unsynthesized_web_snippet_dump(&cleaned)
        || response_looks_like_raw_web_artifact_dump(&cleaned)
        || response_looks_like_retrieval_recap_substituted_for_answer(&cleaned)
        || response_contains_prompt_scaffold(&cleaned)
    {
        return false;
    }
    response_tools_have_recorded_evidence_refs(response_tools)
        || response_has_public_source_signal(&cleaned)
}

fn maybe_preserve_rejected_synthesis_with_coverage_note(
    workflow: &mut Value,
    response_tools: &[Value],
    last_invalid_response_text: &str,
    last_invalid_excerpt: &str,
    last_reject_reason: &str,
) -> bool {
    if !final_verifier_reject_reason_missing_coverage_lanes(last_reject_reason)
        || !rejected_synthesized_response_is_salvageable(last_invalid_response_text, response_tools)
    {
        return false;
    }
    let note = missing_coverage_lane_note_from_reject_reason(last_reject_reason);
    let mut preserved = clean_chat_text(last_invalid_response_text, 32_000);
    if !note.is_empty() {
        preserved = clean_text(&format!("{preserved}\n\n{note}"), 4_000);
    }
    if preserved.is_empty() {
        return false;
    }
    workflow["quality_telemetry"]["final_fallback_used"] = Value::Bool(false);
    workflow["quality_telemetry"]["final_fallback_suppressed"] = Value::Bool(true);
    workflow["final_llm_response"]["used"] = Value::Bool(true);
    workflow["response"] = Value::String(preserved.clone());
    workflow["text"] = Value::String(preserved.clone());
    workflow["message"] = Value::String(preserved.clone());
    workflow["response_finalization"]["finalized_output"] = Value::String(preserved.clone());
    workflow["response_finalization"]["final_output"] = Value::String(preserved.clone());
    workflow["response_finalization"]["final_response"]["text"] = Value::String(preserved.clone());
    workflow["response_workflow"]["final_llm_response"]["text"] = Value::String(preserved.clone());
    workflow["final_llm_response"]["status"] =
        Value::String("synthesized_with_coverage_note".to_string());
    workflow["final_llm_response"]["runtime_interference_disabled"] = Value::Bool(true);
    workflow["final_llm_response"]["visible_response_preserved"] = Value::Bool(true);
    workflow["final_llm_response"]["replacement_response_used"] = Value::Bool(false);
    workflow["final_llm_response"]["coverage_note_appended"] = Value::Bool(!note.is_empty());
    workflow["final_llm_response"]["error"] =
        Value::String("coverage_note_appended_after_verifier_reject".to_string());
    workflow["final_llm_response"]["last_reject_reason"] =
        Value::String(clean_text(last_reject_reason, 240));
    workflow["final_llm_response"]["original_reject_reason"] =
        Value::String(clean_text(last_reject_reason, 240));
    workflow["final_llm_response"]["original_reject_excerpt"] =
        Value::String(clean_text(last_invalid_excerpt, 600));
    annotate_final_evidence_outcome_posture(workflow, response_tools);
    record_workflow_diagnostic_event(
        workflow,
        "tool_evidence_verifier_reject_preserved_with_coverage_note",
        "synthesis_failure_diagnostic",
    );
    set_turn_workflow_final_stage_status(workflow, "synthesized_with_coverage_note");
    true
}

fn replacement_response_for_retry_boilerplate(message: &str, response_tools: &[Value]) -> String {
    let _ = message;
    let failure_reason = clean_text(
        &first_sentence(&response_tools_failure_reason_for_user(response_tools, 4), 280),
        320,
    );
    let coverage_note = clean_text(&fallback_coverage_lane_sentence(response_tools), 320);
    let opening = "The retrieved evidence in this turn was not strong enough to support a clean source-backed conclusion across all requested lanes.";
    if !failure_reason.is_empty() && !coverage_note.is_empty() {
        clean_text(&format!("{opening} {failure_reason} {coverage_note}"), 800)
    } else if !failure_reason.is_empty() {
        clean_text(&format!("{opening} {failure_reason}"), 800)
    } else if !coverage_note.is_empty() {
        clean_text(&format!("{opening} {coverage_note}"), 800)
    } else {
        opening.to_string()
    }
}

fn agent_runtime_temporal_context_prompt() -> String {
    let current_utc = crate::now_iso();
    clean_text(
        &format!(
            "Runtime temporal context: current date/time is {current_utc} (UTC). Treat this runtime timestamp as authoritative for this turn. Dates before this timestamp are in the past; dates after it are in the future. If the user supplies a local date/time correction for the active turn, reconcile against it instead of relying on model training cutoff memory."
        ),
        800,
    )
}

fn tool_completion_report_for_response(
    response_text: &str,
    response_tools: &[Value],
    outcome: &str,
) -> Value {
    let cleaned = clean_chat_text(response_text, 32_000);
    let findings = clean_text(&response_tools_summary_for_user(response_tools, 4), 4_000);
    let failure_reason = clean_text(
        &response_tools_failure_reason_for_user(response_tools, 4),
        4_000,
    );
    let reasoning_source = if !cleaned.is_empty() {
        cleaned.clone()
    } else if !failure_reason.is_empty() {
        failure_reason.clone()
    } else {
        findings.clone()
    };
    let completion_state = if response_tools.is_empty() {
        "not_applicable"
    } else if !failure_reason.is_empty() {
        "reported_reason"
    } else if !findings.is_empty() {
        "reported_findings"
    } else {
        "reported_no_findings"
    };
    let deferred_execution = response_is_deferred_execution_preamble(&cleaned)
        || response_is_deferred_retry_prompt(&cleaned);
    json!({
        "completion_state": completion_state,
        "findings_available": !findings.is_empty(),
        "final_ack_only": response_looks_like_tool_ack_without_findings(&cleaned),
        "final_no_findings": response_is_no_findings_placeholder(&cleaned),
        "final_deferred_execution": deferred_execution,
        "final_requests_more_tooling": workflow_response_requests_more_tooling(&cleaned),
        "reasoning": first_sentence(&reasoning_source, 220),
        "outcome": clean_text(outcome, 200)
    })
}

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
    if response_answers_user_early(message, response_text) {
        return true;
    }
    false
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
    response_tools.iter().any(|row| {
        ["result", "input", "name"].iter().any(|field| {
            clean_text(row.get(*field).and_then(Value::as_str).unwrap_or(""), 2_000)
                .split(|ch: char| !ch.is_ascii_alphanumeric())
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
    );
    let outside_evidence_used_for_decision = outside_evidence_marker
        && final_response_verifier_contract_marker_for_tools(
            response_tools,
            "/diagnostic_markers/final_response_verifier/bounded_answer_signals",
            &full,
        )
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
    None
}

fn response_looks_like_retrieval_recap_substituted_for_answer(response_text: &str) -> bool {
    let normalized = clean_text(response_text, 8_000).to_ascii_lowercase();
    if normalized.is_empty() {
        return false;
    }
    let first = first_sentence(&normalized, 700);
    let trace_marker_count = [
        "recorded evidence so far",
        "here's what i found",
        "here s what i found",
        "web search:",
        "from web retrieval",
        "tool trace complete",
        "search surfaced",
        "retrieval state",
        "coverage is fragmented",
        "provider timeouts",
        "provider starvation",
        "results are incomplete",
    ]
    .iter()
    .filter(|marker| normalized.contains(**marker))
    .count();
    let opens_as_status_or_inventory = [
        "the safest bounded answer",
        "i found some",
        "here's what",
        "here s what",
        "the current retrieval",
        "recorded evidence",
        "web search",
    ]
    .iter()
    .any(|marker| first.contains(*marker));
    trace_marker_count >= 2 || (trace_marker_count >= 1 && opens_as_status_or_inventory)
}

fn workflow_final_answer_explicitly_refuses_unsupported_recommendation(normalized: &str) -> bool {
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

fn response_has_public_source_signal(response_text: &str) -> bool {
    let normalized = clean_text(response_text, 8_000).to_ascii_lowercase();
    [
        "http://",
        "https://",
        "source:",
        "sources:",
        "citation",
        "citations",
        "according to",
        "recorded source",
        "recorded evidence",
        "retrieved evidence",
        "source supports",
        "evidence supports",
        "source-backed",
        "the docs",
        "official docs",
        "release notes",
        "changelog",
        "paper",
        "study",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
        || response_text_contains_domain_like_source_marker(&normalized)
}

fn response_text_contains_domain_like_source_marker(text: &str) -> bool {
    text.split_whitespace().any(|token| {
        let cleaned = token
            .trim_matches(|ch: char| {
                !ch.is_ascii_alphanumeric() && ch != '.' && ch != '/' && ch != ':' && ch != '-'
            })
            .trim_start_matches("http://")
            .trim_start_matches("https://")
            .trim_start_matches("www.");
        let host = cleaned
            .split('/')
            .next()
            .unwrap_or("")
            .chars()
            .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '.' || *ch == '-')
            .collect::<String>();
        let labels = host
            .split('.')
            .filter(|label| !label.is_empty())
            .collect::<Vec<_>>();
        if labels.len() < 2 {
            return false;
        }
        let tld = labels.last().copied().unwrap_or("");
        if !(2..=24).contains(&tld.len()) || !tld.chars().all(|ch| ch.is_ascii_alphabetic()) {
            return false;
        }
        labels
            .iter()
            .any(|label| label.chars().any(|ch| ch.is_ascii_alphabetic()))
    })
}

fn primary_query_texts_for_coverage_tool(tool: &Value) -> Vec<String> {
    let mut out = Vec::<String>::new();
    for key in ["query", "primary_query", "user_goal", "prompt", "question"] {
        let value = clean_text(tool.get(key).and_then(Value::as_str).unwrap_or(""), 1_200);
        if !value.is_empty() {
            out.push(value);
        }
    }
    let raw_input = clean_text(tool.get("input").and_then(Value::as_str).unwrap_or(""), 4_000);
    if raw_input.is_empty() {
        return out;
    }
    match serde_json::from_str::<Value>(&raw_input) {
        Ok(Value::Object(map)) => {
            for key in ["query", "primary_query", "user_goal", "prompt", "question"] {
                let value = clean_text(map.get(key).and_then(Value::as_str).unwrap_or(""), 1_200);
                if !value.is_empty() {
                    out.push(value);
                }
            }
        }
        _ => out.push(raw_input),
    }
    out
}

fn coverage_lane_should_be_hard_required(
    requested: &str,
    response_tools: &[Value],
) -> bool {
    let mut saw_primary_query = false;
    for tool in response_tools {
        for query in primary_query_texts_for_coverage_tool(tool) {
            let normalized_query = normalize_coverage_lane_text(&query);
            if normalized_query.is_empty() {
                continue;
            }
            saw_primary_query = true;
            if normalized_response_covers_coverage_lane(&normalized_query, requested) {
                return true;
            }
        }
    }
    !saw_primary_query
}

fn response_missing_required_entity_lanes(
    response_text: &str,
    response_tools: &[Value],
) -> Vec<String> {
    let normalized_response = normalize_coverage_lane_text(response_text);
    if normalized_response.is_empty() {
        return Vec::new();
    }
    let mut missing = Vec::<String>::new();
    for lane in synthesis_coverage_lanes_for_tools(response_tools, 24) {
        let kind = clean_text(lane.get("kind").and_then(Value::as_str).unwrap_or(""), 80)
            .to_ascii_lowercase();
        if kind != "entity" {
            continue;
        }
        let requested = clean_text(
            lane.get("requested_text")
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        );
        if requested.is_empty()
            || missing
                .iter()
                .any(|row| row.eq_ignore_ascii_case(&requested))
        {
            continue;
        }
        if !coverage_lane_should_be_hard_required(&requested, response_tools) {
            continue;
        }
        if !normalized_response_covers_coverage_lane(&normalized_response, &requested) {
            missing.push(requested);
        }
    }
    missing.into_iter().take(8).collect()
}

fn normalized_response_covers_coverage_lane(normalized_response: &str, lane: &str) -> bool {
    let normalized_lane = normalize_coverage_lane_text(lane);
    if normalized_lane.is_empty() {
        return false;
    }
    if normalized_response.contains(&normalized_lane)
        || normalized_response.contains(&simple_coverage_plural_variant(&normalized_lane))
        || normalized_response.contains(&simple_coverage_singular_variant(&normalized_lane))
    {
        return true;
    }
    let tokens = normalized_lane
        .split_whitespace()
        .filter(|token| token.len() > 2)
        .collect::<Vec<_>>();
    !tokens.is_empty()
        && tokens
            .iter()
            .all(|token| coverage_token_or_simple_variant_present(normalized_response, token))
}

fn coverage_token_or_simple_variant_present(normalized_response: &str, token: &str) -> bool {
    normalized_response.contains(token)
        || normalized_response.contains(&simple_coverage_plural_variant(token))
        || normalized_response.contains(&simple_coverage_singular_variant(token))
}

fn simple_coverage_plural_variant(value: &str) -> String {
    if value.ends_with('s') {
        value.to_string()
    } else {
        format!("{value}s")
    }
}

fn simple_coverage_singular_variant(value: &str) -> String {
    value.strip_suffix('s').unwrap_or(value).to_string()
}

fn normalize_coverage_lane_text(value: &str) -> String {
    clean_text(value, 4_000)
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn workflow_final_synthesis_retry_prompt_context(
    last_reject_reason: &str,
    last_invalid_excerpt: &str,
) -> String {
    if last_reject_reason.trim().is_empty() {
        return String::new();
    }
    clean_text(
        &format!(
            "Internal final-response verifier retry. The previous candidate failed `{}`. Previous excerpt: {}. Produce the user-facing answer from the same recorded evidence and user goal. Lead with the best bounded answer the evidence supports, then state limits or gaps. If the failure names missing coverage lanes, cover each named lane or explicitly mark its evidence as weak or missing. If the failure names missing citation/source signal, preserve compact source grounding for claims supported by recorded evidence, using whatever natural wording fits the answer. If the failure names retrieval recap or the previous candidate opened by reporting tool/search/retrieval status, convert EvidencePacket claim_hints/relevant_extract/source refs into answer units instead of listing sources or tool status. Do not mention this verifier, workflow gates, tool traces, or a required output format.",
            clean_text(last_reject_reason, 120),
            clean_text(last_invalid_excerpt, 240)
        ),
        1_000,
    )
}

fn mark_workflow_pending_gate_without_final_synthesis(
    workflow: &mut Value,
    status: &str,
    diagnostic_source: &str,
    gate_attempt_count: u64,
) {
    let visible_response_preserved = workflow
        .get("response")
        .and_then(Value::as_str)
        .map(|raw| !clean_text(raw, 1_000).is_empty())
        .unwrap_or(false);
    workflow["final_llm_response"]["required"] = Value::Bool(false);
    workflow["final_llm_response"]["attempted"] = Value::Bool(false);
    workflow["final_llm_response"]["used"] = Value::Bool(false);
    workflow["final_llm_response"]["attempt_count"] = json!(0);
    workflow["final_llm_response"]["gate_attempt_count"] = json!(gate_attempt_count);
    workflow["final_llm_response"]["status"] = Value::String(clean_text(status, 80));
    workflow["final_llm_response"]["diagnostic_source"] =
        Value::String(clean_text(diagnostic_source, 120));
    workflow["final_llm_response"]["runtime_interference_disabled"] = Value::Bool(true);
    workflow["final_llm_response"]["visible_response_preserved"] =
        Value::Bool(visible_response_preserved);
    set_turn_workflow_final_stage_status(workflow, status);
}

fn workflow_final_synthesis_attempt_limit(workflow: &Value, response_tools: &[Value]) -> u64 {
    if response_tools.is_empty() {
        return 1;
    }
    workflow
        .pointer("/selected_workflow/tool_menu_interface_contract/final_synthesis_attempt_limit")
        .and_then(Value::as_u64)
        .unwrap_or(1)
        .clamp(1, 3)
}

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
    let initial_visible_gate_choice_submission =
        initial_visible_gate_choice_submission_allowed(
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
            "User message:\n{message}\n\n{tool_state_summary}{missing_turn_tool_context_block}"
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
                        "User message:\n{message}\n\n{tool_state_summary}{missing_turn_tool_context_block}"
                    ),
                    20_000,
                )
            } else {
                clean_text(
                    &format!(
                        "User message:\n{message}\n\n{tool_state_summary}{missing_turn_tool_context_block}\n\nSynthesis input envelope:\n{synthesis_input_json}\n\nRecorded tool outcomes:\n{tool_rows_json}"
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
    let mut manual_toolbox_no_selected = false;
    let mut manual_toolbox_selected_category_key = String::new();
    let mut manual_toolbox_selected_category_label = String::new();
    let mut manual_toolbox_selected_family_key = String::new();
    let mut manual_toolbox_selected_family_label = String::new();
    let mut manual_toolbox_selected_tool_key = String::new();
    let mut manual_toolbox_selected_tool_label = String::new();
    let mut last_error = String::new();
    let mut last_invalid_response_text = String::new();
    let mut last_invalid_excerpt = String::new();
    let mut last_reject_reason = String::new();
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
    // Track synthesis iterations separately from gate routing iterations so that
    // final_llm_response.attempt_count only reflects synthesis retries, not gate steps.
    let mut synthesis_attempt_count: u64 = 0;
    for attempt in 1..=max_attempts {
        let current_manual_toolbox_gate_id = manual_toolbox_active_gate_id(
            &manual_toolbox_selected_category_key,
            &manual_toolbox_selected_family_key,
            &manual_toolbox_selected_tool_key,
        );
        if manual_toolbox_gate_turn {
            workflow["gate_trace"]["attempt_count"] = json!(attempt);
            workflow["gate_trace"]["current_step"] =
                Value::String(current_manual_toolbox_gate_id.to_string());
        } else {
            workflow["gate_trace"]["final_synthesis_attempt_count"] = json!(attempt);
        }
        let active_manual_toolbox_category_turn = manual_toolbox_gate_turn
            && !manual_toolbox_no_selected
            && manual_toolbox_selected_category_key.is_empty();
        let active_manual_toolbox_family_turn = manual_toolbox_gate_turn
            && !manual_toolbox_no_selected
            && !manual_toolbox_selected_category_key.is_empty()
            && manual_toolbox_selected_family_key.is_empty();
        let active_manual_toolbox_tool_turn = manual_toolbox_gate_turn
            && !manual_toolbox_no_selected
            && !manual_toolbox_selected_category_key.is_empty()
            && !manual_toolbox_selected_family_key.is_empty()
            && manual_toolbox_selected_tool_key.is_empty();
        let active_manual_toolbox_payload_turn = manual_toolbox_gate_turn
            && !manual_toolbox_no_selected
            && !manual_toolbox_selected_category_key.is_empty()
            && !manual_toolbox_selected_family_key.is_empty()
            && !manual_toolbox_selected_tool_key.is_empty();
        let active_manual_toolbox_private_gate_turn = active_manual_toolbox_category_turn
            || active_manual_toolbox_family_turn
            || active_manual_toolbox_tool_turn
            || active_manual_toolbox_payload_turn;
        if !active_manual_toolbox_private_gate_turn {
            synthesis_attempt_count += 1;
        }
        workflow["final_llm_response"]["attempt_count"] = json!(synthesis_attempt_count.max(1));
        let compact_tool_retry = attempt > 1 && !response_tools.is_empty();
        let attempt_system_prompt = if active_manual_toolbox_category_turn {
            system_prompt.clone()
        } else if active_manual_toolbox_family_turn {
            workflow_tool_family_prompt_context(
                &manual_toolbox_selected_category_key,
                &manual_toolbox_selected_category_label,
            )
        } else if active_manual_toolbox_tool_turn {
            workflow_tool_selection_prompt_context(
                &manual_toolbox_selected_family_key,
                &manual_toolbox_selected_family_label,
            )
        } else if active_manual_toolbox_payload_turn {
            workflow_tool_payload_prompt_context(
                &manual_toolbox_selected_family_key,
                &manual_toolbox_selected_tool_key,
                &manual_toolbox_selected_tool_label,
            )
        } else if manual_toolbox_no_selected || compact_tool_retry {
            clean_text(&final_answer_instruction, 2_000)
        } else {
            system_prompt.clone()
        };
        let gate_context_user_prompt = clean_text(
            &manual_toolbox_gate_context_user_prompt(message),
            8_000,
        );
        let gate_retry_guidance = if active_manual_toolbox_private_gate_turn
            && attempt > 1
            && (!last_invalid_excerpt.is_empty() || !last_reject_reason.is_empty())
        {
            workflow_private_gate_retry_prompt_context(
                current_manual_toolbox_gate_id,
                message,
                &last_reject_reason,
                &last_invalid_excerpt,
            )
        } else {
            String::new()
        };
        let final_synthesis_retry_guidance = if !active_manual_toolbox_private_gate_turn
            && attempt > 1
            && !last_reject_reason.is_empty()
        {
            workflow_final_synthesis_retry_prompt_context(
                &last_reject_reason,
                &last_invalid_excerpt,
            )
        } else {
            String::new()
        };
        let attempt_user_prompt = if active_manual_toolbox_category_turn {
            gate_context_user_prompt.clone()
        } else if !gate_retry_guidance.is_empty() {
            gate_retry_guidance
        } else if active_manual_toolbox_family_turn
            || active_manual_toolbox_tool_turn
            || active_manual_toolbox_payload_turn
        {
            gate_context_user_prompt
        } else if manual_toolbox_no_selected {
            clean_text(
                &format!(
                    "User message:\n{message}\n\n{tool_state_summary}{missing_turn_tool_context_block}"
                ),
                8_000,
            )
        } else if compact_tool_retry {
            clean_text(
                &format!(
                    "User message:\n{message}\n\n{tool_state_summary}{missing_turn_tool_context_block}\n\nSynthesis input envelope:\n{synthesis_input_json}\n\nRecorded tool outcomes:\n{tool_rows_json}"
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
            user_prompt.clone()
        };
        let attempt_provider = cleaned_provider.clone();
        let attempt_model = cleaned_model.clone();
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
                    &mut workflow,
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
                    &mut manual_toolbox_no_selected,
                    &mut manual_toolbox_selected_category_key,
                    &mut manual_toolbox_selected_category_label,
                    &mut manual_toolbox_selected_family_key,
                    &mut manual_toolbox_selected_family_label,
                    &mut manual_toolbox_selected_tool_key,
                    &mut manual_toolbox_selected_tool_label,
                    &mut last_invalid_excerpt,
                    &mut last_reject_reason,
                ) {
                    match gate_outcome {
                        ManualToolboxPrivateGateOutcome::Continue => continue,
                        ManualToolboxPrivateGateOutcome::Finalize => {
                            return finalize_workflow_gate_stability(root, workflow, message);
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
                    && !direct_gate_recovery_turn
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
                    response_contains_workflow_prompt_analysis_leak(&retried_text);
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
                        bump_workflow_quality_counter(&mut workflow, reject_counter);
                    }
                    last_reject_reason = if reject_reason == "final_response_verifier_contract"
                        && !final_verifier_contract_violation_reason.is_empty()
                    {
                        final_verifier_contract_violation_reason.clone()
                    } else {
                        reject_reason.to_string()
                    };
                    last_invalid_response_text = retried_text.clone();
                    last_invalid_excerpt = first_sentence(&retried_text, 240);
                    workflow["final_llm_response"]["runtime_interference_disabled"] =
                        Value::Bool(true);
                    workflow["final_llm_response"]["diagnostic_reject_reason"] =
                        Value::String(last_reject_reason.clone());
                    workflow["final_llm_response"]["diagnostic_invalid_excerpt"] =
                        Value::String(last_invalid_excerpt.clone());
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
                annotate_final_evidence_outcome_posture(&mut workflow, response_tools);
                workflow["provider"] = Value::String(response_provider);
                workflow["model"] = Value::String(response_model.clone());
                workflow["runtime_model"] = Value::String(response_model);
                if response_tools.is_empty()
                    && enriched_workflow_events.is_empty()
                    && !manual_toolbox_gate_turn
                {
                    mark_workflow_direct_llm_no_tool_answer(&mut workflow);
                }
                set_turn_workflow_final_stage_status(&mut workflow, "synthesized");
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
                    response_workflow_quality_rate(&workflow, "off_topic_reject");
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
                workflow["response"] = Value::String(retried_text);
                return finalize_workflow_gate_stability(root, workflow, message);
            }
            Err(err) => {
                last_error = clean_text(&err, 240);
            }
        }
    }
    if manual_toolbox_gate_turn && response_tools.is_empty() && !last_reject_reason.is_empty() {
        workflow["workflow_control"]["direct_response_path"] = Value::String(
            manual_toolbox_pending_direct_response_path(
                &manual_toolbox_selected_category_key,
                &manual_toolbox_selected_family_key,
                &manual_toolbox_selected_tool_key,
            )
            .to_string(),
        );
        workflow["final_llm_response"]["last_reject_reason"] =
            Value::String(last_reject_reason.clone());
        workflow["final_llm_response"]["error"] = Value::String(last_invalid_excerpt.clone());
        mark_workflow_pending_gate_without_final_synthesis(
            &mut workflow,
            manual_toolbox_pending_stage_status(
                &manual_toolbox_selected_category_key,
                &manual_toolbox_selected_family_key,
                &manual_toolbox_selected_tool_key,
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
        &last_invalid_response_text,
        &last_invalid_excerpt,
        &last_reject_reason,
    ) {
        return finalize_workflow_gate_stability(root, workflow, message);
    }
    workflow["final_llm_response"]["used"] = Value::Bool(false);
    if !last_invalid_excerpt.is_empty() {
        workflow["final_llm_response"]["status"] = Value::String("synthesis_failed".to_string());
        set_turn_workflow_final_stage_status(&mut workflow, "synthesis_failed");
        workflow["final_llm_response"]["error"] = Value::String(last_invalid_excerpt.clone());
        if !last_reject_reason.is_empty() {
            workflow["final_llm_response"]["last_reject_reason"] =
                Value::String(last_reject_reason.clone());
        }
    } else {
        workflow["final_llm_response"]["status"] = Value::String("invoke_failed".to_string());
        set_turn_workflow_final_stage_status(&mut workflow, "invoke_failed");
        workflow["final_llm_response"]["error"] = Value::String(last_error);
    }
    if should_record_workflow_failure_diagnostic(
        &last_reject_reason,
        &last_invalid_excerpt,
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

fn mark_workflow_direct_llm_no_tool_answer(workflow: &mut Value) {
    let contract = default_workflow_tool_menu_contract();
    let first_gate_id = workflow_first_gate_id(&contract);
    let Some(direct_option) = workflow_gate_options(&contract, &first_gate_id)
        .into_iter()
        .find(|option| option.get("has_tools").and_then(Value::as_bool) == Some(false))
    else {
        workflow["final_llm_response"]["direct_answer_marker_error"] =
            Value::String("workflow_cd_missing_no_tool_option".to_string());
        return;
    };
    let direct_key = workflow_option_key(&direct_option);
    let direct_label = workflow_option_label(&direct_option);
    let gate_submission = json!({
        "accepted": true,
        "gate_id": first_gate_id.clone(),
        "llm_submission": direct_label,
        "resume_token": workflow_gate_resume_token(&first_gate_id, "submitted"),
        "decision_source": "llm_direct_answer"
    });
    workflow["workflow_control"]["direct_response_path"] =
        Value::String("first_gate_no_tool_category".to_string());
    workflow["tool_gate"]["selected_work_category"] = Value::String(direct_key);
    workflow["tool_gate"]["selected_tool_family"] = Value::String("none".to_string());
    workflow["tool_gate"]["gate_1_submission_status"] = Value::String("submitted".to_string());
    workflow["tool_gate"]["gate_1_decision_source"] =
        Value::String("llm_direct_answer".to_string());
    workflow["tool_gate"]["gate_submission"] = gate_submission.clone();
    mark_workflow_gate_row_submission(
        workflow,
        &first_gate_id,
        "submitted",
        "llm_direct_answer",
        gate_submission,
    );
    workflow["tool_gate"]["info_source"] = Value::String("llm_direct_answer".to_string());
    if let Some(rows) = workflow
        .get_mut("stage_statuses")
        .and_then(Value::as_array_mut)
    {
        for row in rows.iter_mut() {
            if row
                .get("stage")
                .and_then(Value::as_str)
                .map(|stage| stage == "gate_1_work_category_menu")
                .unwrap_or(false)
            {
                row["status"] = Value::String("answered_no_tool_category".to_string());
                row["decision_source"] = Value::String("llm_direct_answer".to_string());
            }
        }
    }
}

fn mark_workflow_gate_row_submission(
    workflow: &mut Value,
    gate_id: &str,
    submission_status: &str,
    decision_source: &str,
    gate_submission: Value,
) {
    let Some(gates) = workflow
        .get_mut("tool_gate")
        .and_then(|tool_gate| tool_gate.get_mut("gates"))
    else {
        return;
    };
    if let Some(gate_map) = gates.as_object_mut() {
        let gate_row = gate_map
            .entry(gate_id.to_string())
            .or_insert_with(|| json!({}));
        gate_row["submission_status"] = Value::String(submission_status.to_string());
        gate_row["decision_source"] = Value::String(decision_source.to_string());
        gate_row["gate_submission"] = gate_submission;
        return;
    }
    if let Some(gate_rows) = gates.as_array_mut() {
        let mut updated = false;
        for row in gate_rows.iter_mut() {
            let row_gate_id = row
                .get("gate_id")
                .or_else(|| row.get("id"))
                .and_then(Value::as_str)
                .unwrap_or("");
            if row_gate_id == gate_id {
                row["submission_status"] = Value::String(submission_status.to_string());
                row["decision_source"] = Value::String(decision_source.to_string());
                row["gate_submission"] = gate_submission.clone();
                updated = true;
            }
        }
        if !updated {
            gate_rows.push(json!({
                "gate_id": gate_id,
                "submission_status": submission_status,
                "decision_source": decision_source,
                "gate_submission": gate_submission
            }));
        }
    }
}

#[cfg(test)]
mod workflow_fallback_tests {
    use super::*;

    #[test]
    fn direct_llm_no_tool_answer_updates_array_gates_without_panic() {
        let mut workflow = json!({
            "tool_gate": {
                "gates": [
                    {"gate_id": "gate_1_work_category_menu", "submission_status": "presented"}
                ]
            },
            "stage_statuses": [
                {"stage": "gate_1_work_category_menu", "status": "presented"}
            ]
        });

        mark_workflow_direct_llm_no_tool_answer(&mut workflow);

        assert_eq!(
            workflow
                .pointer("/tool_gate/gates/0/submission_status")
                .and_then(Value::as_str),
            Some("submitted")
        );
        assert_eq!(
            workflow
                .pointer("/tool_gate/gates/0/decision_source")
                .and_then(Value::as_str),
            Some("llm_direct_answer")
        );
        assert_eq!(
            workflow
                .pointer("/stage_statuses/0/status")
                .and_then(Value::as_str),
            Some("answered_no_tool_category")
        );
    }

    #[test]
    fn manual_toolbox_selection_parses_pending_web_request() {
        let pending = manual_toolbox_pending_request_from_response(
            "Category: Web research. Tool family: Web research. Tool: web_search. Request payload: {\"query\":\"compare infring\",\"aperture\":\"medium\"}.",
            "Compare this platform to a current external tool category.",
        )
        .expect("pending request");

        assert_eq!(
            pending.get("status").and_then(Value::as_str),
            Some("pending_confirmation")
        );
        assert_eq!(
            pending.get("tool_name").and_then(Value::as_str),
            Some("web_search")
        );
        assert_eq!(
            pending.pointer("/input/query").and_then(Value::as_str),
            Some("compare infring")
        );
        assert_eq!(
            pending
                .get("execution_claim_allowed")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert!(pending
            .get("receipt_binding")
            .and_then(Value::as_str)
            .map(|value| !value.is_empty())
            .unwrap_or(false));
    }

    #[test]
    fn manual_toolbox_selection_requires_explicit_payload_submission() {
        let pending = manual_toolbox_pending_request_from_response(
            "Category: Web research. Tool family: Web research. Tool: web_search.",
            "Compare this platform to a current external tool category.",
        );
        assert!(pending.is_none());
    }

    #[test]
    fn manual_toolbox_selection_rejects_non_catalog_tool_names() {
        let pending = manual_toolbox_pending_request_from_response(
            "Category: Web research. Tool family: Search. Tool: Keyword search. Request payload: {\"keywords\":\"infring frameworks\"}.",
            "Compare this platform to current external tools.",
        );
        assert!(pending.is_none());
    }

    #[test]
    fn manual_toolbox_selection_parses_json_tool_request() {
        let pending = manual_toolbox_pending_request_from_response(
            "{\"tool_family\": \"Web research\", \"tool\": \"web_search\", \"request_payload\": {\"query\": \"compare infring to top agentic frameworks\", \"aperture\":\"medium\"}, \"selection_source\": \"unit_test\"}",
            "Compare infring to top agentic frameworks.",
        )
        .expect("pending request");

        assert_eq!(
            pending.get("status").and_then(Value::as_str),
            Some("pending_confirmation")
        );
        assert_eq!(
            pending.get("tool_name").and_then(Value::as_str),
            Some("web_search")
        );
        assert_eq!(
            pending.pointer("/input/aperture").and_then(Value::as_str),
            Some("medium")
        );
        assert_eq!(
            pending.get("source").and_then(Value::as_str),
            Some("unit_test")
        );
    }

    #[test]
    fn invalid_visible_gate_like_draft_does_not_skip_private_gate_recovery() {
        let workflow = json!({});
        assert!(!initial_visible_gate_choice_submission_allowed(
            &[],
            false,
            &workflow
        ));
    }

    #[test]
    fn recovered_pending_request_allows_gate_recovery_shortcut() {
        let workflow = json!({
            "manual_toolbox_pending_tool_request": {
                "tool_name": "web_search",
                "input": {
                    "query": "mastra langgraph typescript",
                    "aperture": "medium"
                }
            }
        });
        assert!(initial_visible_gate_choice_submission_allowed(
            &[],
            false,
            &workflow
        ));
    }

    #[test]
    fn workflow_gate_stability_rows_score_direct_llm_response_as_final() {
        let workflow = json!({
            "selected_workflow": {
                "name": "simple_conversation_v1"
            },
            "workflow_control": {
                "direct_response_path": "first_gate_unresolved"
            },
            "tool_gate": {
                "selected_work_category": "respond_directly"
            },
            "tool_count": 0,
            "response": "Hey! How can I help you today?",
            "final_llm_response": {
                "used": true,
                "required": false,
                "status": "direct_llm_response"
            },
            "stage_statuses": [
                {
                    "stage": "gate_1_work_category_menu",
                    "status": "answered_no_tool_category"
                },
                {
                    "stage": "gate_6_llm_final_output",
                    "status": "skipped_not_required"
                }
            ]
        });
        let rows = workflow_gate_stability_rows(&workflow);

        assert_eq!(
            rows.iter()
                .find(|row| row.get("gate").and_then(Value::as_str)
                    == Some("gate_6_llm_final_output"))
                .and_then(|row| row.get("status").and_then(Value::as_str)),
            Some("passed")
        );
        assert_eq!(
            rows.iter()
                .find(|row| row.get("gate").and_then(Value::as_str)
                    == Some("gate_2_tool_family_menu"))
                .and_then(|row| row.get("status").and_then(Value::as_str)),
            Some("not_applicable")
        );
    }
    #[test]
    fn workflow_gate_stability_version_ring_keeps_latest_three_versions() {
        let root = std::env::temp_dir().join(format!(
            "workflow-gate-stability-ring-{}",
            crate::deterministic_receipt_hash(&json!({
                "test": "workflow_gate_stability_version_ring_keeps_latest_three_versions",
                "ts": crate::now_iso()
            }))
        ));
        let rows = vec![
            json!({
                "gate": "gate_1_work_category_menu",
                "status": "passed"
            }),
            json!({
                "gate": "gate_6_llm_final_output",
                "status": "failed"
            }),
        ];

        for (index, version_hash) in ["v1", "v2", "v3", "v4"].iter().enumerate() {
            let snapshot = json!({
                "name": "simple_conversation_v1",
                "workflow_version": version_hash
            });
            workflow_gate_stability_update_version_ring(
                &root,
                "simple_conversation_v1",
                version_hash,
                &snapshot,
                &rows,
                &format!("ts-{index}"),
            );
        }
        let v3_snapshot = json!({
            "name": "simple_conversation_v1",
            "workflow_version": "v3"
        });
        workflow_gate_stability_update_version_ring(
            &root,
            "simple_conversation_v1",
            "v3",
            &v3_snapshot,
            &rows,
            "ts-4",
        );

        let ring_path = root.join("local/state/ops/workflow_gate_stability/versions_ring.json");
        let ring = read_json_loose(&ring_path).expect("version ring json");
        let versions = ring
            .get("versions")
            .and_then(Value::as_array)
            .expect("versions array");

        assert_eq!(versions.len(), 3);
        assert_eq!(
            ring.get("current_version_hash").and_then(Value::as_str),
            Some("v3")
        );
        assert_eq!(
            versions
                .first()
                .and_then(|value| value.get("workflow_version_hash"))
                .and_then(Value::as_str),
            Some("v3")
        );
        assert_eq!(
            versions
                .first()
                .and_then(|value| value.get("turn_count"))
                .and_then(Value::as_u64),
            Some(2)
        );
        assert!(!versions.iter().any(|value| {
            value.get("workflow_version_hash").and_then(Value::as_str) == Some("v1")
        }));
        assert!(versions.iter().all(|value| {
            value
                .get("workflow_json")
                .and_then(|snapshot| snapshot.get("name"))
                .and_then(Value::as_str)
                == Some("simple_conversation_v1")
        }));
        assert!(root
            .join("local/state/ops/workflow_gate_stability/workflow_versions/v3.workflow.json")
            .exists());
        assert!(!root
            .join("local/state/ops/workflow_gate_stability/workflow_versions/v1.workflow.json")
            .exists());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn manual_toolbox_selection_rejects_json_request_without_payload() {
        let pending = manual_toolbox_pending_request_from_response(
            "{\"tool_family\": \"Web research\", \"tool\": \"web_search\"}",
            "Compare infring to top agentic frameworks.",
        );
        assert!(pending.is_none());
    }

    #[test]
    fn direct_llm_no_tool_answer_marks_trace_as_no_tool_category() {
        let mut workflow = json!({
            "workflow_control": {},
            "tool_gate": {
                "gates": {
                    "gate_1": {}
                }
            },
            "stage_statuses": [
                {"stage": "gate_1_work_category_menu", "status": "presented"},
                {"stage": "final_llm_response", "status": "pending_final_llm"}
            ]
        });

        mark_workflow_direct_llm_no_tool_answer(&mut workflow);

        assert_eq!(
            workflow
                .pointer("/workflow_control/direct_response_path")
                .and_then(Value::as_str),
            Some("first_gate_no_tool_category")
        );
        assert_eq!(
            workflow
                .pointer("/tool_gate/gate_1_submission_status")
                .and_then(Value::as_str),
            Some("submitted")
        );
        assert_eq!(
            workflow
                .pointer("/stage_statuses/0/status")
                .and_then(Value::as_str),
            Some("answered_no_tool_category")
        );
    }

    #[test]
    fn natural_language_tool_preference_does_not_create_pending_request() {
        let mut workflow = json!({
            "workflow_control": {},
            "system_events": []
        });
        record_manual_toolbox_pending_request(
            &mut workflow,
            "I would use web search to compare infring to other frameworks.",
            "Compare this platform to a current external tool category.",
        );
        assert!(workflow
            .get("manual_toolbox_pending_tool_request")
            .is_none());
    }

    #[test]
    fn manual_toolbox_candidate_menu_is_not_reported_as_no_tool_category() {
        let workflow = turn_workflow_metadata(
            "normal_turn",
            &[],
            &[turn_workflow_event(
                "manual_toolbox_candidate_menu",
                json!({"candidate_count": 1}),
            )],
            "",
            "Use web search for the exact comparison topic supplied by the user.",
        );

        assert_eq!(
            workflow
                .pointer("/workflow_control/direct_response_path")
                .and_then(Value::as_str),
            Some("first_gate_pending_llm_tool_choice")
        );
    }

    #[test]
    fn exact_tool_gate_submission_updates_workflow_path() {
        let mut workflow = turn_workflow_metadata(
            "normal_turn",
            &[],
            &[turn_workflow_event(
                "manual_toolbox_candidate_menu",
                json!({"candidate_count": 1}),
            )],
            "",
            "Use web search for the exact comparison topic supplied by the user.",
        );
        record_manual_toolbox_pending_request(
            &mut workflow,
            "Category: Web research. Tool family: Web research. Tool: web_search. Request payload: {\"query\":\"compare infring to agent frameworks\",\"aperture\":\"medium\"}.",
            "Use web search for the exact comparison topic supplied by the user.",
        );

        assert_eq!(
            workflow
                .pointer("/manual_toolbox_pending_tool_request/status")
                .and_then(Value::as_str),
            Some("pending_confirmation")
        );
        assert_eq!(
            workflow
                .pointer("/workflow_control/direct_response_path")
                .and_then(Value::as_str),
            Some("first_gate_pending_tool_confirmation")
        );
    }

    #[test]
    fn narrated_tool_choice_does_not_create_pending_request_even_with_menu() {
        let mut workflow = turn_workflow_metadata(
            "normal_turn",
            &[],
            &[turn_workflow_event(
                "manual_toolbox_candidate_menu",
                json!({"candidate_count": 1}),
            )],
            "",
            "Use web search for the exact comparison topic supplied by the user.",
        );
        record_manual_toolbox_pending_request(
            &mut workflow,
            "I would choose a menu item.",
            "Use web search for the exact comparison topic supplied by the user.",
        );

        assert!(workflow
            .pointer("/manual_toolbox_pending_tool_request")
            .is_none());
        assert_eq!(
            workflow
                .pointer("/workflow_control/direct_response_path")
                .and_then(Value::as_str),
            Some("first_gate_pending_llm_tool_choice")
        );
    }

    #[test]
    fn manual_toolbox_candidate_menu_detection_reads_kind_field() {
        let workflow = json!({
            "workflow_control": {},
            "system_events": [
                turn_workflow_event("manual_toolbox_candidate_menu", json!({"candidate_count": 1}))
            ]
        });

        assert!(workflow_has_manual_toolbox_candidate_menu(&workflow));
    }

    #[test]
    fn current_agentic_framework_comparison_does_not_auto_select_web_candidate() {
        let candidates = latent_tool_candidates_for_message(
            "try again to do a real source-backed comparison for the topic I asked about",
            &[],
        );
        assert!(
            !candidates.iter().any(|candidate| {
                candidate
                    .get("tool")
                    .and_then(Value::as_str)
                    .map(|tool| tool == "web_search")
                    .unwrap_or(false)
            }),
            "latent tooling must not auto-select web_search; the workflow CD must present the menu and wait for the LLM"
        );
    }

    #[test]
    fn ordinary_lookup_and_search_intents_do_not_create_latent_candidates() {
        for message in [
            "look up recent changes in the relevant frameworks",
            "search the web for public evidence about a named system",
            "use web research to compare current options",
        ] {
            let candidates = latent_tool_candidates_for_message(message, &[]);
            assert!(candidates.is_empty(), "{message}: {candidates:?}");
        }
    }

    #[test]
    fn time_scoped_update_requests_do_not_create_latent_candidates() {
        for message in [
            "give me an update on the agentic landscape in May 2026",
            "summarize the current state of synthetic biology in 2026",
            "brief me on the electric vehicle market landscape this year",
        ] {
            let candidates = latent_tool_candidates_for_message(message, &[]);
            assert!(candidates.is_empty(), "{message}: {candidates:?}");
        }
    }

    #[test]
    fn evaluative_web_research_prompts_do_not_create_latent_candidates() {
        let candidates = latent_tool_candidates_for_message(
            "What is the best agentic framework in 2026? Search first, but do not trust marketing pages blindly. Give me a defensible answer.",
            &[],
        );
        assert!(candidates.is_empty(), "{candidates:?}");
    }

    #[test]
    fn runtime_temporal_context_declares_past_future_rule() {
        let prompt = agent_runtime_temporal_context_prompt();
        assert!(prompt.contains("current date/time"));
        assert!(prompt.contains("Dates before this timestamp are in the past"));
        assert!(prompt.contains("dates after it are in the future"));
    }

    #[test]
    fn unresolved_tool_need_without_progress_is_rejected_signal() {
        assert!(manual_toolbox_response_exposes_unresolved_tool_need(
            "I don't have current web search results, but I can compare if you'd like me to search."
        ));
        assert!(manual_toolbox_response_exposes_unresolved_tool_need(
            "Web search returned limited results for this specific comparison. I can provide a ranked table."
        ));
        assert!(!manual_toolbox_response_exposes_unresolved_tool_need(
            "I would choose a menu item for the user's current topic."
        ));
    }

    #[test]
    fn stale_tool_intent_draft_for_simple_greeting_is_withheld() {
        let message = "hey";
        let response = "I need to perform a web search for the user's comparison topic. Let me start that process. [tool:Web Research]";
        assert!(workflow_response_requests_more_tooling(response));
        assert!(response_contains_unrequested_content_without_tool_evidence(
            message,
            response,
            &[],
        ));
        assert!(response_current_turn_dominance_violation(
            message,
            response,
            &[],
        ));
    }

    #[test]
    fn stale_mixed_tool_draft_for_simple_greeting_requires_fresh_synthesis() {
        let response = "I will use web search for the user's comparison topic. Please hold while I gather details. Meanwhile, let's inspect the tiny fixture repo and identify a small bugfix. 1 = Respond directly";
        assert!(workflow_response_requests_more_tooling(response));
        assert!(turn_workflow_requires_final_llm(&[], &[], response));
    }

    #[test]
    fn unsupported_tool_claim_guard_ignores_later_hypothetical_offer() {
        assert!(response_claims_tool_success_without_current_turn_evidence(
            "Use web search for the exact comparison topic supplied by the user.",
            "Web search didn't return specific April 2026 comparisons. I can provide a source-backed ranked table if you name specific frameworks.",
            &[],
        ));
        assert!(!response_claims_tool_success_without_current_turn_evidence(
            "Use web search for the exact comparison topic supplied by the user.",
            "I would choose a menu item for the user's current topic.",
            &[],
        ));
    }

    #[test]
    fn recorded_low_signal_tool_result_counts_as_visible_answer() {
        let tools = vec![json!({
            "name": "batch_query",
            "status": "no_results",
            "result": "Search did not produce enough source coverage for the requested comparison."
        })];

        assert!(response_answers_tool_confirmation_with_recorded_result(
            "The search did not find enough relevant source coverage for that comparison.",
            &tools,
        ));
        assert!(!response_answers_tool_confirmation_with_recorded_result(
            "I searched the web.",
            &tools,
        ));
        assert!(!response_answers_tool_confirmation_with_recorded_result(
            "", &tools,
        ));
    }

    #[test]
    fn final_verifier_rejects_tool_status_overlead_before_answer() {
        let tools = vec![json!({
            "name": "batch_query",
            "status": "no_results",
            "result": "Search did not produce enough source coverage for the requested comparison.",
            "tool_result_quality": {
                "flags": ["low_signal"],
                "evidence_count": 0
            }
        })];

        assert!(response_violates_tool_backed_final_verifier(
            "The web search results are too thin and provider-degraded to answer. I cannot give a useful conclusion.",
            &tools,
        ));
        assert!(!response_violates_tool_backed_final_verifier(
            "Bottom line: treat the topic as unverified from this retrieval turn and avoid making a source-backed choice until a better source lane is available. The search result was low-signal, so this is bounded guidance rather than retrieved evidence.",
            &tools,
        ));
    }

    #[test]
    fn final_verifier_rejects_status_overlead_variants_from_research_turns() {
        let tools = vec![json!({
            "name": "batch_query",
            "status": "low_signal",
            "result": "Retrieval returned only partial source coverage.",
            "tool_result_quality": {
                "flags": ["low_signal"],
                "evidence_count": 0
            }
        })];

        assert!(response_violates_tool_backed_final_verifier(
            "I ran a batch search, but the results were low-signal, so I cannot answer.",
            &tools,
        ));
        assert!(response_violates_tool_backed_final_verifier(
            "Based on the search attempt, there is not enough retrieved evidence to decide.",
            &tools,
        ));
        assert!(!response_violates_tool_backed_final_verifier(
            "The practical answer is to treat the choice as unverified by this turn and avoid a strong recommendation until source coverage improves. The retrieval attempt was low-signal, so this is bounded guidance.",
            &tools,
        ));
    }

    #[test]
    fn final_verifier_rejects_missing_evidence_claim_when_refs_exist() {
        let tools = vec![json!({
            "name": "web_search",
            "status": "ok",
            "result": "Official docs say the library supports typed agent outputs.",
            "evidence_refs": [{
                "title": "Official docs",
                "locator": "https://example.test/docs"
            }],
            "tool_result_quality": {
                "evidence_count": 1
            }
        })];

        assert!(response_violates_tool_backed_final_verifier(
            "No evidence is available for this question, so I cannot answer.",
            &tools,
        ));
        assert!(!response_violates_tool_backed_final_verifier(
            "Bottom line: the recorded source supports typed agent outputs, but it does not prove production maturity. Treat the evidence as useful for capability fit and still verify operations, support, and deployment references.",
            &tools,
        ));
    }

    #[test]
    fn final_verifier_rejects_outside_evidence_decision_basis() {
        let tools = vec![json!({
            "name": "web_search",
            "status": "ok",
            "result": "Retrieved partial evidence for the requested comparison.",
            "evidence_refs": [{
                "title": "Partial comparison source",
                "locator": "https://example.test/partial"
            }],
            "tool_result_quality": {
                "evidence_count": 1
            }
        })];

        assert_eq!(
            tool_backed_final_verifier_violation_reason(
                "The retrieved evidence does not support a direct comparison. General knowledge, not source-backed in this turn: Alpha is known for reliability and Beta is known for flexibility. Bottom line: choose Alpha for production.",
                &tools,
            ),
            Some("final_response_verifier_contract:outside_evidence_used_for_decision".to_string())
        );
        assert_eq!(
            tool_backed_final_verifier_violation_reason(
                "The retrieved evidence does not support a direct comparison. General knowledge would be outside retrieved evidence here, so there is no source-backed basis to recommend Alpha or Beta.",
                &tools,
            ),
            None
        );
    }

    #[test]
    fn final_verifier_treats_materialized_candidates_as_recorded_evidence() {
        let tools = vec![json!({
            "name": "browser_materialize_page",
            "status": "ok",
            "result": "Rendered page extracted through materialization.",
            "evidence_pack_candidates": [{
                "source_kind": "browser_materialized_page",
                "title": "Rendered source",
                "locator": "https://example.test/rendered",
                "snippet": "The rendered page provides enough text for normal synthesis consumption.",
                "claim_hints": ["Rendered source supports the research claim."],
                "score": 76.0,
                "confidence": "usable"
            }]
        })];

        assert!(response_violates_tool_backed_final_verifier(
            "No evidence was found for this question.",
            &tools,
        ));

        let synthesis_input = workflow_synthesis_input_for_final_response(
            "research the rendered source",
            &tools,
            &json!({}),
        );
        assert_eq!(
            synthesis_input
                .pointer("/evidence_pack/0/source")
                .and_then(Value::as_str),
            Some("evidence_pack_candidate"),
            "{synthesis_input:#?}"
        );
        assert_eq!(
            synthesis_input
                .pointer("/evidence_pack/0/source_kind")
                .and_then(Value::as_str),
            Some("browser_materialized_page"),
            "{synthesis_input:#?}"
        );
    }

    #[test]
    fn final_verifier_rejects_missing_named_coverage_lanes() {
        let tools = vec![json!({
            "name": "batch_query",
            "status": "ok",
            "result": "Retrieved evidence across the comparison request.",
            "query_metadata": {
                "required_coverage": {
                    "entities": ["Infring", "LangGraph", "CrewAI", "AutoGen", "OpenHands"]
                }
            },
            "evidence_refs": [{
                "title": "Framework comparison",
                "locator": "https://example.test/frameworks"
            }]
        })];

        assert!(response_violates_tool_backed_final_verifier(
            "Bottom line: Infring, LangGraph, and CrewAI have enough evidence for a provisional comparison, but the ranking remains bounded.",
            &tools,
        ));
        assert!(!response_violates_tool_backed_final_verifier(
            "Bottom line: Infring, LangGraph, and CrewAI have enough evidence for a provisional comparison. AutoGen and OpenHands remain weakly covered in this retrieval turn, so treat their tradeoffs as explicit coverage gaps rather than source-backed conclusions.",
            &tools,
        ));
    }

    #[test]
    fn final_verifier_does_not_hard_require_expanded_query_alias_lanes() {
        let tools = vec![json!({
            "name": "batch_query",
            "status": "ok",
            "result": "Retrieved source-backed evidence for a broad landscape update.",
            "input": json!({
                "query": "Give me an update on the AI agentic landscape in May 2026",
                "queries": ["autonomous AI agents enterprise adoption May 2026"],
                "required_coverage": {
                    "entities": ["AI agents", "agentic AI", "autonomous agents", "multi-agent systems"]
                }
            }).to_string(),
            "evidence_refs": [{
                "title": "Agentic AI landscape",
                "locator": "https://example.test/agentic-ai-landscape",
                "snippet": "Agentic AI adoption and orchestration are changing in 2026."
            }],
            "tool_result_quality": {
                "coverage": {
                    "bucket_status": "covered",
                    "missing_buckets": []
                },
                "evidence_count": 1
            }
        })];

        assert_eq!(
            tool_backed_final_verifier_violation_reason(
                "Bottom line: according to the retrieved source, the agentic AI landscape in May 2026 is centered on enterprise adoption, orchestration, and platform/infrastructure maturation.",
                &tools,
            ),
            None
        );
    }

    #[test]
    fn final_verifier_accepts_package_backed_source_signal_when_evidence_exists() {
        let tools = vec![json!({
            "name": "batch_query",
            "status": "ok",
            "result": "Retrieved source-backed evidence for the requested comparison.",
            "evidence_refs": [{
                "title": "Framework comparison source",
                "locator": "https://example.test/frameworks",
                "snippet": "Substantive citable evidence for the answer."
            }]
        })];

        assert_eq!(
            tool_backed_final_verifier_violation_reason(
                "Bottom line: Alpha is better for production while Beta is better for prototypes.",
                &tools,
            ),
            None
        );
        assert_eq!(
            tool_backed_final_verifier_violation_reason(
                "Bottom line: according to the retrieved project docs, Alpha is better for production while Beta is better for prototypes.",
                &tools,
            ),
            None
        );
        assert_eq!(
            tool_backed_final_verifier_violation_reason(
                "Bottom line: Alpha is better for production while Beta is better for prototypes (langchain.com).",
                &tools,
            ),
            None
        );
    }

    #[test]
    fn final_synthesis_retry_guidance_names_missing_coverage_lane_behavior() {
        let prompt = workflow_final_synthesis_retry_prompt_context(
            "final_response_verifier_contract:missing_coverage_lanes=AutoGen, OpenHands",
            "Bottom line: Infring, LangGraph, and CrewAI are covered.",
        );
        let lowered = prompt.to_ascii_lowercase();

        assert!(lowered.contains("missing coverage lanes"));
        assert!(lowered.contains("cover each named lane"));
        assert!(lowered.contains("weak or missing"));
        assert!(lowered.contains("required output format"));
    }

    #[test]
    fn final_synthesis_retry_guidance_is_internal_and_format_free() {
        let prompt = workflow_final_synthesis_retry_prompt_context(
            "final_response_verifier_contract:missing_citation_or_source_signal",
            "The web search results are too thin.",
        );
        let lowered = prompt.to_ascii_lowercase();

        assert!(lowered.contains("internal final-response verifier retry"));
        assert!(lowered.contains("lead with the best bounded answer"));
        assert!(lowered.contains("source grounding"));
        assert!(lowered.contains("do not mention this verifier"));
        assert!(lowered.contains("required output format"));
    }

    #[test]
    fn latent_tool_candidates_do_not_force_prompt_only_gate() {
        let message = "what? why are you repeating the same fallback text?";
        let latent_tool_candidates = json!([{"tool": "web_search"}]);
        let no_tool_minimal_final_turn = message_explicitly_disallows_tool_calls(message);
        let manual_toolbox_prompt_only_turn = !no_tool_minimal_final_turn
            && response_tools_prompt_only_gate_required(message, &latent_tool_candidates);

        assert!(!no_tool_minimal_final_turn);
        assert!(!manual_toolbox_prompt_only_turn);
    }

    #[test]
    fn single_workflow_only_latent_candidate_can_recover_pending_tool_request() {
        let candidates = json!([{
            "workflow_only": true,
            "selected_tool_family": "web_research",
            "selected_tool_key": "batch_query",
            "selected_tool_label": "Research query pack",
            "input": {
                "source": "web",
                "query": "compare retrieval tools",
                "aperture": "medium"
            }
        }]);

        let pending = manual_toolbox_pending_request_from_latent_candidates(
            &candidates,
            "Research Firecrawl, Tavily, and Exa.",
        )
        .expect("single valid latent candidate");

        assert_eq!(
            pending.get("tool_name").and_then(Value::as_str),
            Some("batch_query")
        );
        assert_eq!(
            pending.get("selected_tool_key").and_then(Value::as_str),
            Some("batch_query")
        );
        assert_eq!(
            pending.get("source").and_then(Value::as_str),
            Some("latent_candidate_recovery")
        );
        assert_eq!(
            pending.pointer("/input/query").and_then(Value::as_str),
            Some("compare retrieval tools")
        );
        assert!(pending.pointer("/input/keywords").is_none(), "{pending:?}");
        assert!(
            pending.pointer("/input/required_coverage").is_none(),
            "{pending:?}"
        );
        assert!(
            pending.pointer("/input/query_metadata_policy").is_none(),
            "{pending:?}"
        );
    }

    #[test]
    fn latent_candidate_recovery_preserves_input_without_forcing_metadata() {
        let candidates = json!([{
            "workflow_only": true,
            "selected_tool_family": "web_research",
            "selected_tool_key": "batch_query",
            "selected_tool_label": "Research query pack",
            "input": {
                "source": "web",
                "query": "weather in Denver today",
                "aperture": "medium"
            }
        }]);

        let pending = manual_toolbox_pending_request_from_latent_candidates(
            &candidates,
            "weather in Denver today",
        )
        .expect("single valid latent candidate");

        assert!(
            pending.pointer("/input/query_metadata_policy").is_none(),
            "{pending:?}"
        );
    }

    #[test]
    fn latent_candidates_do_not_classify_research_from_question_shape() {
        let research_candidates = latent_tool_candidates_for_message(
            "Research Firecrawl, Tavily, and Exa as data tools for AI research agents.",
            &[],
        );
        assert!(research_candidates.is_empty(), "{research_candidates:?}");

        let trivia_candidates = latent_tool_candidates_for_message("what is 2+2?", &[]);
        assert!(trivia_candidates.is_empty(), "{trivia_candidates:?}");
    }

    #[test]
    fn terminal_invariant_can_require_single_latent_candidate_promotion() {
        let workflow = json!({
            "selected_workflow": {
                "tool_menu_interface_contract": {
                    "terminal_invariant_contract": {
                        "valid_latent_candidate_without_tool_attempt_policy": "promote_single_required_candidate_or_structured_failure_before_final_answer",
                        "required_latent_candidate_flag": "requires_tool_attempt_before_final_answer"
                    }
                }
            }
        });
        let candidates = json!([{
            "workflow_only": true,
            "selected_tool_family": "web_research",
            "selected_tool_key": "batch_query",
            "requires_tool_attempt_before_final_answer": true,
            "input": {"source": "web", "query": "compare retrieval tools", "aperture": "medium"}
        }]);

        assert!(
            workflow_latent_candidate_recovery_required_by_terminal_invariant(
                &workflow,
                &candidates
            )
        );
    }

    #[test]
    fn latent_candidate_recovery_refuses_ambiguous_candidates() {
        let candidates = json!([
            {
                "workflow_only": true,
                "selected_tool_family": "web_research",
                "selected_tool_key": "batch_query",
                "input": {"source": "web", "query": "first", "aperture": "medium"}
            },
            {
                "workflow_only": true,
                "selected_tool_family": "web_research",
                "selected_tool_key": "web_search",
                "input": {"source": "web", "query": "second", "aperture": "medium"}
            }
        ]);

        assert!(manual_toolbox_pending_request_from_latent_candidates(
            &candidates,
            "Research something current.",
        )
        .is_none());
    }

    #[test]
    fn latent_candidate_recovery_requires_missing_evidence_or_gate_diagnostic() {
        let missing_evidence_workflow = json!({
            "response": "No source-backed synthesis is available because I don't have recorded tool results for this turn."
        });
        let direct_answer_workflow = json!({
            "response": "The answer is 4."
        });
        let gate_diagnostic_workflow = json!({
            "workflow_control": {"direct_response_path": "gate_4_pending_llm_tool_request"}
        });
        let first_gate_private_workflow = json!({
            "workflow_control": {"direct_response_path": "first_gate_pending_llm_tool_choice"}
        });
        let rejected_private_gate_workflow = json!({
            "final_llm_response": {"last_reject_reason": "visible_gate_choice_reply"}
        });

        assert!(workflow_latent_candidate_recovery_needed(
            &missing_evidence_workflow,
            ""
        ));
        assert!(!workflow_latent_candidate_recovery_needed(
            &direct_answer_workflow,
            ""
        ));
        assert!(workflow_latent_candidate_recovery_needed(
            &gate_diagnostic_workflow,
            ""
        ));
        assert!(workflow_latent_candidate_recovery_needed(
            &first_gate_private_workflow,
            ""
        ));
        assert!(workflow_latent_candidate_recovery_needed(
            &rejected_private_gate_workflow,
            ""
        ));
    }

    #[test]
    fn meta_control_recovery_accepts_direct_fallback_loop_answer() {
        let message = "what? why are you repeating the same fallback text?";
        assert!(direct_gate_recovery_response_answers_user(
            message,
            "The repeated fallback text came from a response-finalization loop; I will answer directly now.",
            true,
        ));
        assert!(!direct_gate_recovery_response_answers_user(
            message,
            "I will answer directly now.",
            true,
        ));
    }

    #[test]
    fn direct_llm_response_preservation_has_no_runtime_fallback_path() {
        let mut workflow = json!({
            "final_llm_response": {}
        });
        preserve_direct_llm_response_without_fallback(&mut workflow, "Direct response");
        assert_eq!(
            workflow.get("response").and_then(Value::as_str),
            Some("Direct response")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/source")
                .and_then(Value::as_str),
            Some("initial_llm_response")
        );
        assert!(workflow
            .pointer("/final_llm_response/fallback_source")
            .is_none());
    }

    #[test]
    fn workflow_unexpected_state_retry_boilerplate_detector_catches_legacy_copy() {
        let retry_boilerplate = "I completed the workflow gate, but the final workflow state was unexpected. Please retry so I can rerun the chain cleanly.";
        assert!(response_contains_unexpected_state_retry_boilerplate(
            retry_boilerplate
        ));
    }

    #[test]
    fn workflow_unexpected_state_retry_boilerplate_detector_catches_next_actions_template() {
        let retry_boilerplate = "I completed the workflow gate, but the final workflow state was unexpected. Please retry so I can rerun the chain cleanly.\n\nNext actions: 1) clarify the exact outcome you want 2) run one targeted tool call 3) return a concise answer from current context";
        assert!(response_contains_unexpected_state_retry_boilerplate(
            retry_boilerplate
        ));
    }

    #[test]
    fn workflow_unexpected_state_retry_boilerplate_detector_catches_paraphrased_macro_bundle() {
        let retry_boilerplate = "Workflow gate completed but the final workflow state was unexpected. Next actions: run one targeted tool call, then provide a concise answer from current context.";
        assert!(response_contains_unexpected_state_retry_boilerplate(
            retry_boilerplate
        ));
    }

    #[test]
    fn workflow_unexpected_state_retry_boilerplate_detector_does_not_flag_plain_retry_offer() {
        let normal_text =
            "I can retry the query if you want, or I can answer directly from current context.";
        assert!(!response_contains_unexpected_state_retry_boilerplate(
            normal_text
        ));
    }

    #[test]
    fn workflow_unexpected_state_retry_boilerplate_detector_catches_policy_gate_outage_template() {
        let retry_boilerplate = "The File List step was blocked before I could finish the answer: This is a policy gate, not a web-provider outage.";
        assert!(response_contains_unexpected_state_retry_boilerplate(
            retry_boilerplate
        ));
        assert!(workflow_response_repetition_breaker_active(
            retry_boilerplate
        ));
    }

    #[test]
    fn workflow_unexpected_state_retry_boilerplate_detector_catches_runtime_capability_surface_template(
    ) {
        let retry_boilerplate = "I can access runtime telemetry, persistent memory, workspace files, channels, and approved command surfaces in this session.";
        assert!(response_contains_unexpected_state_retry_boilerplate(
            retry_boilerplate
        ));
        assert!(workflow_response_repetition_breaker_active(
            retry_boilerplate
        ));
    }

    #[test]
    fn workflow_unexpected_state_retry_boilerplate_detector_catches_route_classification_template()
    {
        let retry_boilerplate = "The first gate (\"workflow_route\") is still classifying this as an \"info\" route rather than a \"task\" route, which means it's still seeing this as a conversational exchange rather than a tool operation request. The system needs explicit tool-related phrasing to trigger the task classification path.";
        assert!(response_contains_unexpected_state_retry_boilerplate(
            retry_boilerplate
        ));
        assert!(workflow_response_repetition_breaker_active(
            retry_boilerplate
        ));
    }

    #[test]
    fn manual_toolbox_gate_context_prompt_is_context_only() {
        let prompt = manual_toolbox_gate_context_user_prompt(
            "Research Mastra for TypeScript agent workflows.",
        );

        assert!(prompt.contains("Context-only user message."));
        assert!(prompt.contains("Do not answer it directly."));
        assert!(prompt.contains("Research Mastra for TypeScript agent workflows."));
    }

    #[test]
    fn workflow_failure_diagnostic_records_when_retry_boilerplate_reject_was_seen() {
        let tools = vec![json!({
            "name": "file_list",
            "blocked": false
        })];
        assert!(should_record_workflow_failure_diagnostic(
            "unexpected_state_retry_boilerplate",
            "",
            "",
            &tools,
            false,
        ));
    }

    #[test]
    fn workflow_failure_diagnostic_records_when_policy_block_tool_is_present() {
        let tools = vec![json!({
            "name": "file_list",
            "blocked": true,
            "result": "lease_denied:client_ingress_domain_boundary"
        })];
        assert!(should_record_workflow_failure_diagnostic(
            "", "", "", &tools, false
        ));
    }

    #[test]
    fn workflow_failure_diagnostic_records_when_latest_reply_is_legacy_copy() {
        let tools = vec![json!({
            "name": "file_list",
            "blocked": false
        })];
        assert!(should_record_workflow_failure_diagnostic(
            "",
            "",
            "I completed the workflow gate, but the final workflow state was unexpected. Please retry so I can rerun the chain cleanly.",
            &tools,
            false,
        ));
    }

    #[test]
    fn workflow_failure_diagnostic_records_when_invalid_excerpt_has_retry_boilerplate() {
        let tools = vec![json!({
            "name": "file_list",
            "blocked": false
        })];
        assert!(should_record_workflow_failure_diagnostic(
            "",
            "final reply did not render; please retry so i can rerun the chain cleanly",
            "",
            &tools,
            false,
        ));
    }

    #[test]
    fn recent_assistant_retry_loop_detector_triggers_on_two_of_last_three_assistant_turns() {
        let messages = vec![
            json!({"role": "assistant", "text": "I completed the workflow gate, but the final workflow state was unexpected. Please retry so I can rerun the chain cleanly."}),
            json!({"role": "user", "text": "what?"}),
            json!({"role": "assistant", "text": "Workflow gate completed but the final workflow state was unexpected. Next actions: run one targeted tool call, then provide a concise answer from current context."}),
            json!({"role": "assistant", "text": "Normal answer now."}),
        ];
        assert!(recent_assistant_retry_loop_detected(&messages));
    }

    #[test]
    fn recent_assistant_retry_loop_detector_ignores_single_retry_like_turn() {
        let messages = vec![
            json!({"role": "assistant", "text": "I completed the workflow gate, but the final workflow state was unexpected. Please retry so I can rerun the chain cleanly."}),
            json!({"role": "assistant", "text": "I can answer directly from current context."}),
            json!({"role": "assistant", "text": "Here is the direct answer."}),
        ];
        assert!(!recent_assistant_retry_loop_detected(&messages));
    }

    #[test]
    fn workflow_failure_diagnostic_records_when_recent_loop_detected() {
        let tools = vec![json!({
            "name": "file_list",
            "blocked": false
        })];
        assert!(should_record_workflow_failure_diagnostic(
            "",
            "",
            "normal latest",
            &tools,
            true
        ));
    }

    #[test]
    fn direct_response_preservation_returns_none_for_empty_draft() {
        assert!(direct_llm_response_from_initial_draft("").is_none());
    }

    #[test]
    fn direct_response_preservation_keeps_clean_llm_text() {
        assert_eq!(
            direct_llm_response_from_initial_draft("All checks look healthy."),
            Some("All checks look healthy.".to_string())
        );
    }

    #[test]
    fn direct_response_preservation_withholds_private_gate_choice() {
        assert!(direct_llm_response_from_initial_draft("Need tools? Yes").is_none());
    }

    #[test]
    fn direct_response_preservation_extracts_structured_gate_final_answer() {
        assert_eq!(
            direct_llm_response_from_initial_draft(
                r#"{"gate":"2","final_answer":"Synthesized tradeoffs."}"#
            ),
            Some("Synthesized tradeoffs.".to_string())
        );
    }

    #[test]
    fn response_repeat_detector_catches_near_duplicate_formatting_variants() {
        let latest = "I'm not hard-locked. The previous fallback repeated, so I'm switching to a plain direct response path and avoiding extra tool calls unless you explicitly request one.";
        let response = "Im not hard locked - the previous fallback repeated so im switching to a plain direct response path and avoiding extra tool calls unless you explicitly request one";
        assert!(response_repeats_latest_assistant_copy(response, latest));
    }

    #[test]
    fn direct_llm_response_preservation_rejects_private_gate_tokens() {
        let mut workflow = json!({
            "final_llm_response": {}
        });
        preserve_direct_llm_response_without_fallback(&mut workflow, "Need tools? Yes");
        assert_eq!(
            workflow
                .pointer("/final_llm_response/used")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert!(workflow.get("response").is_none());
        assert!(workflow
            .pointer("/final_llm_response/fallback_source")
            .is_none());
    }

    #[test]
    fn workflow_final_visible_response_text_unwraps_final_answer_gate_payload() {
        assert_eq!(
            workflow_final_visible_response_text(
                r#"{"gate":"4","final_answer":"Based on the results: ..."}"#
            ),
            "Based on the results: ..."
        );
        assert_eq!(
            workflow_final_visible_response_text(
                r#"{"tool_family":"web_research","tool":"web_search","request_payload":{"query":"compare frameworks","aperture":"medium"}}"#
            ),
            ""
        );
        assert_eq!(
            workflow_final_visible_response_text(
                r#"{"tool_family":"web_research","tool":"batch_query","request_payload":{"source":"web","query":"Compare LangGraph vs CrewAI on reliability and deployment.","queries":["LangGraph official docs reliability deployment","CrewAI official docs reliability deployment"],"aperture":"medium"}}"#
            ),
            ""
        );
        assert_eq!(
            workflow_final_visible_response_text("Plain natural language answer."),
            "Plain natural language answer."
        );
    }

    #[test]
    fn final_retry_boilerplate_diagnostic_rewrites_response_and_sets_metadata() {
        let mut workflow = json!({
            "response": "I completed the workflow gate, but the final workflow state was unexpected. Please retry so I can rerun the chain cleanly.",
            "quality_telemetry": {},
            "final_llm_response": {
                "used": false,
                "status": "synthesis_failed"
            }
        });
        let tools = vec![json!({
            "name": "file_list",
            "blocked": true,
            "result": "lease_denied:client_ingress_domain_boundary"
        })];
        apply_final_retry_boilerplate_diagnostic(
            &mut workflow,
            "hello",
            "I completed the workflow gate, but the final workflow state was unexpected. Please retry so I can rerun the chain cleanly.",
            &tools,
        );
        let response = workflow
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        assert!(
            response.contains("retrieved evidence in this turn was not strong enough"),
            "{response}"
        );
        assert!(!response.contains("please retry"), "{response}");
        assert!(!response.contains("workflow gate"), "{response}");
        assert_eq!(
            workflow
                .pointer("/final_llm_response/status")
                .and_then(Value::as_str),
            Some("guard_violation_rewritten")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/visible_response_preserved")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/replacement_response_used")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_reason")
                .and_then(Value::as_str),
            Some("retry_boilerplate_diagnostic")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_last_stage")
                .and_then(Value::as_str),
            Some("final_retry_diagnostic")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_reasons/0")
                .and_then(Value::as_str),
            Some("retry_boilerplate_diagnostic")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_events/0/stage")
                .and_then(Value::as_str),
            Some("final_retry_diagnostic")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_stages/0")
                .and_then(Value::as_str),
            Some("final_retry_diagnostic")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_multi_stage")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_summary/trigger_count")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_summary/severity")
                .and_then(Value::as_str),
            Some("low")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_summary/requires_operator_review")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_summary/escalation_reason")
                .and_then(Value::as_str),
            Some("single_guard_activation")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_summary/recommended_action")
                .and_then(Value::as_str),
            Some("continue_direct_mode")
        );
        assert_eq!(
            workflow
                .pointer("/quality_telemetry/diagnostic_event_trigger_count")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            workflow
                .pointer("/quality_telemetry/diagnostic_event_stage_final_retry_diagnostic")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            workflow
                .pointer("/quality_telemetry/diagnostic_event_reason_retry_boilerplate_diagnostic")
                .and_then(Value::as_u64),
            Some(1)
        );
    }

    #[test]
    fn final_retry_boilerplate_diagnostic_rewrites_nested_visible_response_fields() {
        let mut workflow = json!({
            "response": "",
            "text": "",
            "message": "",
            "response_finalization": {
                "final_response": {
                    "text": "This retrieval attempt did not produce enough relevant evidence to answer the question well. Recorded evidence so far: Here's what I found: web search returned low-signal snippets."
                }
            },
            "response_workflow": {
                "final_llm_response": {
                    "text": "This retrieval attempt did not produce enough relevant evidence to answer the question well. Recorded evidence so far: Here's what I found: web search returned low-signal snippets."
                }
            },
            "quality_telemetry": {},
            "final_llm_response": {
                "used": false,
                "status": "synthesis_failed"
            }
        });
        let tools = vec![json!({
            "name": "batch_query",
            "status": "ok",
            "result": "Web retrieval ran, but only low-signal snippets were available for synthesis in this turn.",
            "tool_result_quality": {
                "status": "low_signal"
            }
        })];
        apply_final_retry_boilerplate_diagnostic(
            &mut workflow,
            "Find recent benchmarks comparing agent frameworks.",
            "",
            &tools,
        );
        let response = workflow
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        assert!(response.contains("retrieved evidence in this turn was not strong enough"));
        assert_eq!(
            workflow
                .pointer("/response_finalization/final_response/text")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_ascii_lowercase(),
            response
        );
        assert_eq!(
            workflow
                .pointer("/response_workflow/final_llm_response/text")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_ascii_lowercase(),
            response
        );
    }

    #[test]
    fn final_empty_response_diagnostic_records_metadata() {
        let mut workflow = json!({
            "response": "",
            "quality_telemetry": {},
            "final_llm_response": {
                "used": false,
                "status": "synthesis_failed"
            }
        });
        let tools = Vec::<Value>::new();
        apply_final_empty_response_diagnostic(&mut workflow, "hello", "", &tools);
        let response = workflow
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(response.trim().is_empty());
        assert_eq!(
            workflow
                .pointer("/final_llm_response/status")
                .and_then(Value::as_str),
            Some("empty_llm_response")
        );
        assert!(workflow
            .pointer("/final_llm_response/fallback_source")
            .is_none());
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_reason")
                .and_then(Value::as_str),
            Some("empty_response_presence_diagnostic")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_last_stage")
                .and_then(Value::as_str),
            Some("final_presence_diagnostic")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_reasons/0")
                .and_then(Value::as_str),
            Some("empty_response_presence_diagnostic")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_events/0/stage")
                .and_then(Value::as_str),
            Some("final_presence_diagnostic")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_stages/0")
                .and_then(Value::as_str),
            Some("final_presence_diagnostic")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_multi_stage")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_summary/trigger_count")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_summary/severity")
                .and_then(Value::as_str),
            Some("low")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_summary/requires_operator_review")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_summary/escalation_reason")
                .and_then(Value::as_str),
            Some("single_guard_activation")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_summary/recommended_action")
                .and_then(Value::as_str),
            Some("continue_direct_mode")
        );
        assert_eq!(
            workflow
                .pointer("/quality_telemetry/diagnostic_event_trigger_count")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            workflow
                .pointer("/quality_telemetry/diagnostic_event_stage_final_presence_diagnostic")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            workflow
                .pointer(
                    "/quality_telemetry/diagnostic_event_reason_empty_response_presence_diagnostic"
                )
                .and_then(Value::as_u64),
            Some(1)
        );
    }

    #[test]
    fn final_empty_response_diagnostic_uses_generic_tool_evidence_fallback_when_findings_exist() {
        let mut workflow = json!({
            "response": "",
            "quality_telemetry": {},
            "final_llm_response": {
                "used": false,
                "status": "synthesis_failed"
            }
        });
        let tools = vec![json!({
            "name": "web_search",
            "status": "ok",
            "is_error": false,
            "blocked": false,
            "result": "Top findings: OpenHands is an AI coding agent platform with strong automation capabilities."
        })];

        apply_final_empty_response_diagnostic(
            &mut workflow,
            "Compare two documentation tools for a small team.",
            "",
            &tools,
        );
        let response = workflow
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(response.starts_with("The practical answer is"), "{response}");
        assert_eq!(
            workflow
                .pointer("/final_llm_response/used")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/status")
                .and_then(Value::as_str),
            Some("tool_evidence_fallback_used")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_source")
                .and_then(Value::as_str),
            Some("tool_evidence_runtime_fallback")
        );
        assert_eq!(
            workflow
                .pointer("/quality_telemetry/final_fallback_used")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            workflow
                .pointer("/quality_telemetry/final_fallback_suppressed")
                .and_then(Value::as_bool),
            Some(false)
        );
    }

    #[test]
    fn final_empty_response_diagnostic_uses_tool_evidence_fallback_when_failure_exists() {
        let mut workflow = json!({
            "response": "",
            "quality_telemetry": {},
            "final_llm_response": {
                "used": false,
                "status": "synthesized"
            }
        });
        let tools = vec![json!({
            "name": "web_search",
            "status": "error",
            "is_error": true,
            "blocked": false,
            "error": "search service returned timeout"
        })];

        apply_final_empty_response_diagnostic(
            &mut workflow,
            "Find latest agentic frameworks",
            "",
            &tools,
        );
        let response = workflow
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(!response.trim().is_empty());
        assert!(response.contains("search service returned timeout"), "{response}");
        assert_eq!(
            workflow
                .pointer("/final_llm_response/status")
                .and_then(Value::as_str),
            Some("tool_evidence_fallback_used")
        );
    }

    #[test]
    fn tool_evidence_fallback_returns_bounded_user_visible_answer() {
        let response = fallback_final_response_from_tool_evidence(
            "Compare LangGraph, CrewAI, AutoGen, and OpenHands for agentic research workflows.",
            &[json!({
                "name": "batch_query",
                "status": "ok",
                "is_error": false,
                "result": "Key findings: AutoGen - Microsoft Research: AutoGen is an open-source framework for building AI agents.",
                "evidence_refs": [
                    {
                        "title": "LangGraph overview",
                        "snippet": "LangGraph focuses on long-running stateful agent workflows."
                    },
                    {
                        "title": "CrewAI docs",
                        "snippet": "CrewAI emphasizes role-based multi-agent orchestration."
                    },
                    {
                        "title": "OpenHands docs",
                        "snippet": "OpenHands is oriented toward software-development task execution."
                    }
                ]
            })],
        );
        assert!(
            response.starts_with("Based on the retrieved evidence"),
            "{response}"
        );
        assert!(response.contains("LangGraph focuses on"), "{response}");
        assert!(
            response.contains("Source: LangGraph overview."),
            "{response}"
        );
        assert!(!response.contains("tool_evidence_runtime_fallback_suppressed"));
        assert!(!response.contains("Recorded evidence so far"));
    }

    #[test]
    fn tool_evidence_fallback_uses_answer_ready_evidence_packets() {
        let response = fallback_final_response_from_tool_evidence(
            "What are some scientific breakthroughs in 2026?",
            &[json!({
                "name": "batch_query",
                "status": "ok",
                "is_error": false,
                "evidence_pack": [{
                    "pack_version": "evidence_pack_v1",
                    "source_kind": "research_news",
                    "source_class": "scholarly_or_research",
                    "title": "Battery milestone report",
                    "locator": "https://example.test/battery-2026",
                    "source_domain": "example.test",
                    "relevant_extract": "A research group reported a solid-state battery chemistry milestone with improved cycle stability in 2026.",
                    "why_relevant_to_query": "It is a source-backed example of a scientific breakthrough reported in the requested year.",
                    "claim_hints": [
                        "A 2026 solid-state battery chemistry milestone improved cycle stability."
                    ],
                    "counts_as_usable_evidence": true
                }]
            })],
        );
        assert!(
            response.starts_with("Based on the retrieved evidence"),
            "{response}"
        );
        assert!(response.contains("solid-state battery chemistry milestone"), "{response}");
        assert!(response.contains("Source: Battery milestone report, example.test."), "{response}");
        assert!(!response.contains("Here's what I found"), "{response}");
        assert!(!response.contains("Recorded evidence so far"), "{response}");
        assert!(!response.contains("From web retrieval"), "{response}");
    }

    #[test]
    fn final_verifier_rejects_retrieval_recap_when_answer_packets_exist() {
        let tools = vec![json!({
            "name": "batch_query",
            "status": "ok",
            "is_error": false,
            "evidence_pack": [{
                "pack_version": "evidence_pack_v1",
                "source_kind": "official_docs",
                "source_class": "official_or_primary",
                "title": "LangGraph docs",
                "locator": "https://docs.example.test/langgraph",
                "source_domain": "docs.example.test",
                "relevant_extract": "LangGraph supports stateful graph-based agent orchestration for long-running workflows.",
                "why_relevant_to_query": "It directly supports the requested comparison of agent workflow frameworks.",
                "claim_hints": [
                    "LangGraph supports stateful graph-based agent orchestration for long-running workflows."
                ],
                "counts_as_usable_evidence": true
            }]
        })];
        let bad_response = "The safest bounded answer is that the current retrieval state does not support a source-backed conclusion yet. Recorded evidence so far: Here's what I found: web search: From web retrieval: docs.example.test: LangGraph docs.";
        assert_eq!(
            tool_backed_final_verifier_violation_reason(bad_response, &tools).as_deref(),
            Some("final_response_verifier_contract:retrieval_recap_substituted_for_answer")
        );
    }

    #[test]
    fn rejected_retrieval_recap_is_rewritten_from_evidence_packets() {
        let mut workflow = json!({
            "response": "",
            "text": "",
            "message": "",
            "quality_telemetry": {},
            "final_llm_response": {
                "used": false,
                "status": "synthesis_failed"
            }
        });
        let tools = vec![json!({
            "name": "batch_query",
            "status": "ok",
            "is_error": false,
            "evidence_pack": [{
                "pack_version": "evidence_pack_v1",
                "source_kind": "official_docs",
                "source_class": "official_or_primary",
                "title": "CrewAI docs",
                "locator": "https://docs.example.test/crewai",
                "source_domain": "docs.example.test",
                "relevant_extract": "CrewAI emphasizes role-based multi-agent orchestration for collaborative agent workflows.",
                "why_relevant_to_query": "It directly supports a comparison of agent workflow frameworks.",
                "claim_hints": [
                    "CrewAI emphasizes role-based multi-agent orchestration for collaborative agent workflows."
                ],
                "counts_as_usable_evidence": true
            }]
        })];
        let bad_response = "The safest bounded answer is that the current retrieval state does not support a source-backed conclusion yet. Recorded evidence so far: Here's what I found: web search: From web retrieval: docs.example.test: CrewAI docs.";
        assert!(maybe_apply_rejected_tool_evidence_fallback(
            &mut workflow,
            "Compare LangGraph and CrewAI.",
            &tools,
            bad_response,
            bad_response,
            "final_response_verifier_contract:retrieval_recap_substituted_for_answer",
        ));
        let response = workflow.get("response").and_then(Value::as_str).unwrap_or("");
        assert!(response.contains("CrewAI emphasizes role-based"), "{response}");
        assert!(!response.contains("Here's what I found"), "{response}");
        assert!(!response.contains("Recorded evidence so far"), "{response}");
        assert_eq!(
            workflow
                .pointer("/final_llm_response/status")
                .and_then(Value::as_str),
            Some("tool_evidence_fallback_used")
        );
    }

    #[test]
    fn tool_evidence_fallback_names_required_coverage_lanes() {
        let tools = vec![json!({
            "name": "batch_query",
            "status": "no_results",
            "is_error": false,
            "result": "Search providers ran but did not return usable evidence.",
            "query_metadata": {
                "required_coverage": {
                    "entities": ["PydanticAI", "LangGraph", "CrewAI", "LangChain", "OpenAI Agents SDK"],
                    "facets": ["production readiness", "type safety"]
                }
            },
            "tool_result_quality": {
                "status": "low_signal",
                "flags": ["insufficient_evidence"],
                "retry": {
                    "recommended": true,
                    "reason": "coverage_gap"
                }
            }
        })];
        let response = fallback_final_response_from_tool_evidence(
            "Is PydanticAI a serious production option for structured agent workflows in Python?",
            &tools,
        );

        assert!(response.contains("Coverage"), "{response}");
        assert!(response.contains("PydanticAI"), "{response}");
        assert!(!response.starts_with("This retrieval attempt"), "{response}");

        let synthesis_input =
            workflow_synthesis_input_for_final_response("research PydanticAI", &tools, &json!({}));
        assert_eq!(
            synthesis_input
                .pointer("/coverage_gaps/0/requested_text")
                .and_then(Value::as_str),
            Some("PydanticAI"),
            "{synthesis_input:#?}"
        );
        assert_eq!(
            synthesis_input
                .pointer("/coverage_gaps/0/kind")
                .and_then(Value::as_str),
            Some("entity"),
            "{synthesis_input:#?}"
        );
    }

    #[test]
    fn tool_evidence_outcome_posture_distinguishes_supported_partial_and_insufficient() {
        let supported = vec![json!({
            "name": "batch_query",
            "status": "ok",
            "result": "Key findings: LangGraph and CrewAI differ in orchestration style.",
            "evidence_refs": [{
                "title": "LangGraph docs",
                "locator": "https://example.com/langgraph",
                "score": 0.9
            }]
        })];
        assert_eq!(tool_evidence_outcome_posture(&supported), "supported_answer");

        let partial = vec![json!({
            "name": "batch_query",
            "status": "ok",
            "result": "Search returned some findings but facet coverage is incomplete.",
            "query_metadata": {
                "required_coverage": {
                    "entities": ["LangGraph", "CrewAI"]
                }
            },
            "evidence_refs": [{
                "title": "LangGraph docs",
                "locator": "https://example.com/langgraph",
                "score": 0.9
            }],
            "tool_result_quality": {
                "status": "low_signal",
                "flags": ["insufficient_evidence"]
            }
        })];
        assert_eq!(
            tool_evidence_outcome_posture(&partial),
            "bounded_partial_answer"
        );

        let insufficient = vec![json!({
            "name": "batch_query",
            "status": "error",
            "error": "provider timeout"
        })];
        assert_eq!(
            tool_evidence_outcome_posture(&insufficient),
            "evidence_insufficient_answer"
        );
    }

    #[test]
    fn tool_evidence_fallback_records_evidence_outcome_posture() {
        let mut workflow = json!({
            "response": "",
            "quality_telemetry": {},
            "final_llm_response": {
                "used": false,
                "status": "synthesis_failed"
            }
        });
        let tools = vec![json!({
            "name": "batch_query",
            "status": "ok",
            "result": "Search returned some findings but major coverage gaps remain.",
            "query_metadata": {
                "required_coverage": {
                    "entities": ["AlphaTool", "BetaTool"]
                }
            },
            "evidence_refs": [{
                "title": "AlphaTool docs",
                "locator": "https://example.com/alpha",
                "score": 0.8
            }],
            "tool_result_quality": {
                "status": "low_signal",
                "flags": ["insufficient_evidence"]
            }
        })];

        apply_final_empty_response_diagnostic(&mut workflow, "Compare AlphaTool and BetaTool.", "", &tools);

        assert_eq!(
            workflow
                .pointer("/final_llm_response/evidence_outcome_posture")
                .and_then(Value::as_str),
            Some("bounded_partial_answer")
        );
        assert_eq!(
            workflow
                .pointer("/quality_telemetry/evidence_outcome_posture")
                .and_then(Value::as_str),
            Some("bounded_partial_answer")
        );
    }

    #[test]
    fn apply_final_empty_response_diagnostic_uses_tool_evidence_fallback() {
        let mut workflow = json!({
            "response": "",
            "response_finalization": {
                "final_response": {}
            },
            "response_workflow": {
                "final_llm_response": {}
            },
            "final_llm_response": {}
        });
        let tools = vec![json!({
            "name": "batch_query",
            "status": "ok",
            "result": "Key findings: LangGraph is production-oriented while CrewAI is stronger for rapid prototyping.",
            "evidence_refs": [{
                "title": "Framework comparison",
                "snippet": "LangGraph is production-oriented while CrewAI is stronger for rapid prototyping."
            }]
        })];

        apply_final_empty_response_diagnostic(
            &mut workflow,
            "Compare LangGraph and CrewAI.",
            "",
            &tools,
        );

        let response = workflow
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(
            response.starts_with("Based on the retrieved evidence"),
            "{response}"
        );
        assert!(response.contains("LangGraph is production-oriented"), "{response}");
        assert!(
            response.contains("Source: Framework comparison."),
            "{response}"
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/status")
                .and_then(Value::as_str),
            Some("tool_evidence_fallback_used")
        );
        assert_eq!(
            workflow
                .pointer("/quality_telemetry/final_fallback_used")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn rejected_tool_backed_response_preserves_synthesis_with_coverage_note() {
        let mut workflow = json!({
            "response": "",
            "quality_telemetry": {},
            "final_llm_response": {
                "used": false,
                "status": "synthesis_failed"
            }
        });
        let tools = vec![json!({
            "name": "batch_query",
            "status": "ok",
            "result": "Top findings: pgvector is simple for small teams; Weaviate is a managed vector database with additional operational surface.",
            "evidence_refs": [{
                "title": "pgvector",
                "snippet": "pgvector is simple for small teams."
            }],
            "query_metadata": {
                "required_coverage": {
                    "entities": ["LlamaIndex", "LangChain", "pgvector", "Weaviate", "Chroma"]
                }
            }
        })];
        let rejected_response =
            "Based on the available evidence, here's a pragmatic recommendation for a small team's RAG stack: pgvector is the simplest default for small teams, while Weaviate has more managed operational surface according to the recorded source.";

        let rewritten = maybe_apply_rejected_tool_evidence_fallback(
            &mut workflow,
            "Research current RAG stack options for a small team.",
            &tools,
            rejected_response,
            rejected_response,
            "final_response_verifier_contract:missing_coverage_lanes=Weaviate, Chroma",
        );

        assert!(rewritten);
        let response = workflow
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(response.contains("pragmatic recommendation"), "{response}");
        assert!(response.contains("Coverage"), "{response}");
        assert!(response.contains("Weaviate"), "{response}");
        assert!(response.contains("Chroma"), "{response}");
        assert_eq!(
            workflow
                .pointer("/final_llm_response/status")
                .and_then(Value::as_str),
            Some("synthesized_with_coverage_note")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/replacement_response_used")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/original_reject_reason")
                .and_then(Value::as_str),
            Some("final_response_verifier_contract:missing_coverage_lanes=Weaviate, Chroma")
        );
    }

    #[test]
    fn tool_input_required_coverage_becomes_synthesis_lanes() {
        let input = json!({
            "query": "Compare AlphaTool and BetaTool for current research workflows.",
            "required_coverage": {
                "entities": ["AlphaTool", "BetaTool"],
                "facets": ["current fit", "operational risk"]
            },
            "query_metadata_policy": {
                "classification": "expanded_query_pack"
            }
        })
        .to_string();
        let tools = vec![json!({
            "name": "batch_query",
            "status": "ok",
            "is_error": false,
            "input": input,
            "evidence_refs": [{
                "title": "AlphaTool docs",
                "snippet": "AlphaTool has current public documentation."
            }]
        })];

        let lanes = synthesis_coverage_lanes_for_tools(&tools, 8);
        assert!(
            lanes.iter().any(|row| {
                row.get("kind").and_then(Value::as_str) == Some("entity")
                    && row.get("requested_text").and_then(Value::as_str) == Some("AlphaTool")
            }),
            "{lanes:#?}"
        );
        assert!(
            lanes.iter().any(|row| {
                row.get("kind").and_then(Value::as_str) == Some("facet")
                    && row.get("requested_text").and_then(Value::as_str) == Some("current fit")
            }),
            "{lanes:#?}"
        );

        let missing = response_missing_required_entity_lanes("AlphaTool has evidence.", &tools);
        assert!(missing.iter().any(|row| row == "BetaTool"), "{missing:#?}");
        assert!(response_missing_required_entity_lanes(
            "AlphaTool and BetaTool both need source-backed treatment.",
            &tools
        )
        .is_empty());
    }

    #[test]
    fn tool_quality_covered_marks_query_metadata_coverage_usable() {
        let input = json!({
            "query": "Give me a broad market landscape update.",
            "required_coverage": {
                "entities": ["expanded alias lane"],
                "facets": ["adoption"]
            }
        })
        .to_string();
        let tools = vec![json!({
            "name": "batch_query",
            "status": "ok",
            "is_error": false,
            "input": input,
            "evidence_refs": [{
                "title": "Market landscape source",
                "snippet": "A substantive source-backed landscape result."
            }],
            "tool_result_quality": {
                "coverage": {
                    "bucket_status": "covered",
                    "missing_buckets": []
                }
            }
        })];

        let lanes = synthesis_coverage_lanes_for_tools(&tools, 8);
        assert!(
            lanes.iter().any(|row| {
                row.get("requested_text").and_then(Value::as_str) == Some("expanded alias lane")
                    && row.get("status").and_then(Value::as_str) == Some("usable")
            }),
            "{lanes:#?}"
        );
    }

    #[test]
    fn record_workflow_diagnostic_event_tracks_history_and_counter() {
        let mut workflow = json!({
            "final_llm_response": {},
            "quality_telemetry": {}
        });
        record_workflow_diagnostic_event(
            &mut workflow,
            "retry_boilerplate_diagnostic",
            "final_retry_diagnostic",
        );
        record_workflow_diagnostic_event(
            &mut workflow,
            "empty_response_presence_diagnostic",
            "final_presence_diagnostic",
        );
        record_workflow_diagnostic_event(
            &mut workflow,
            "retry_boilerplate_diagnostic",
            "synthesis_failure_diagnostic",
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_reason")
                .and_then(Value::as_str),
            Some("retry_boilerplate_diagnostic")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_last_stage")
                .and_then(Value::as_str),
            Some("synthesis_failure_diagnostic")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_reasons/0")
                .and_then(Value::as_str),
            Some("retry_boilerplate_diagnostic")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_reasons/1")
                .and_then(Value::as_str),
            Some("empty_response_presence_diagnostic")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_events/0/stage")
                .and_then(Value::as_str),
            Some("final_retry_diagnostic")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_events/1/stage")
                .and_then(Value::as_str),
            Some("final_presence_diagnostic")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_events/2/stage")
                .and_then(Value::as_str),
            Some("synthesis_failure_diagnostic")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_stages/0")
                .and_then(Value::as_str),
            Some("final_retry_diagnostic")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_stages/1")
                .and_then(Value::as_str),
            Some("final_presence_diagnostic")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_stages/2")
                .and_then(Value::as_str),
            Some("synthesis_failure_diagnostic")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_multi_stage")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_summary/trigger_count")
                .and_then(Value::as_u64),
            Some(3)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_summary/distinct_reason_count")
                .and_then(Value::as_u64),
            Some(2)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_summary/distinct_stage_count")
                .and_then(Value::as_u64),
            Some(3)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_summary/multi_stage")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_summary/severity")
                .and_then(Value::as_str),
            Some("high")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_summary/requires_operator_review")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_summary/escalation_reason")
                .and_then(Value::as_str),
            Some("high_trigger_or_stage_diversity")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_summary/recommended_action")
                .and_then(Value::as_str),
            Some("operator_review_recommended")
        );
        assert_eq!(
            workflow
                .pointer("/quality_telemetry/diagnostic_event_trigger_count")
                .and_then(Value::as_u64),
            Some(3)
        );
        assert_eq!(
            workflow
                .pointer("/quality_telemetry/diagnostic_event_stage_final_retry_diagnostic")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            workflow
                .pointer("/quality_telemetry/diagnostic_event_stage_final_presence_diagnostic")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            workflow
                .pointer("/quality_telemetry/diagnostic_event_stage_synthesis_failure_diagnostic")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            workflow
                .pointer("/quality_telemetry/diagnostic_event_reason_retry_boilerplate_diagnostic")
                .and_then(Value::as_u64),
            Some(2)
        );
        assert_eq!(
            workflow
                .pointer(
                    "/quality_telemetry/diagnostic_event_reason_empty_response_presence_diagnostic"
                )
                .and_then(Value::as_u64),
            Some(1)
        );
    }

    #[test]
    fn workflow_diagnostic_stage_counter_key_sanitizes_non_alnum_stage_tokens() {
        assert_eq!(
            workflow_diagnostic_stage_counter_key("Final Presence Guard!!"),
            "diagnostic_event_stage_final_presence_diagnostic"
        );
        assert_eq!(
            workflow_diagnostic_stage_counter_key("___"),
            "diagnostic_event_stage_unknown"
        );
    }

    #[test]
    fn workflow_diagnostic_reason_counter_key_sanitizes_non_alnum_reason_tokens() {
        assert_eq!(
            workflow_diagnostic_reason_counter_key("Retry Boilerplate Guard!!"),
            "diagnostic_event_reason_retry_boilerplate_diagnostic"
        );
        assert_eq!(
            workflow_diagnostic_reason_counter_key("___"),
            "diagnostic_event_reason_unknown"
        );
    }

    #[test]
    fn workflow_diagnostic_summary_classification_escalates_with_counts() {
        assert_eq!(
            workflow_diagnostic_summary_classification(1, 1),
            (
                "low",
                false,
                "single_guard_activation",
                "continue_direct_mode"
            )
        );
        assert_eq!(
            workflow_diagnostic_summary_classification(2, 1),
            (
                "moderate",
                false,
                "repeated_or_multi_stage_guard_activity",
                "monitor_and_continue_direct_mode",
            )
        );
        assert_eq!(
            workflow_diagnostic_summary_classification(1, 3),
            (
                "high",
                true,
                "high_trigger_or_stage_diversity",
                "operator_review_recommended",
            )
        );
    }

    #[test]
    fn final_synthesis_attempt_limit_is_cd_owned_for_tool_backed_answers() {
        let workflow = json!({
            "selected_workflow": {
                "tool_menu_interface_contract": {
                    "final_synthesis_attempt_limit": 2
                }
            }
        });
        let tools = vec![json!({
            "name": "batch_query",
            "status": "ok",
            "result": "source-backed finding"
        })];
        assert_eq!(workflow_final_synthesis_attempt_limit(&workflow, &tools), 2);
        assert_eq!(workflow_final_synthesis_attempt_limit(&workflow, &[]), 1);
    }

    #[test]
    fn final_synthesis_attempt_limit_is_bounded_runtime_execution() {
        let workflow = json!({
            "selected_workflow": {
                "tool_menu_interface_contract": {
                    "final_synthesis_attempt_limit": 99
                }
            }
        });
        let tools = vec![json!({"name": "batch_query"})];
        assert_eq!(workflow_final_synthesis_attempt_limit(&workflow, &tools), 3);
    }

    #[test]
    fn final_empty_response_diagnostic_preserves_non_empty_response() {
        let mut workflow = json!({
            "response": "Answer already present.",
            "quality_telemetry": {},
            "final_llm_response": {
                "used": true,
                "status": "synthesized"
            }
        });
        let tools = vec![json!({
            "name": "file_list",
            "blocked": false
        })];
        apply_final_empty_response_diagnostic(&mut workflow, "hello", "", &tools);
        assert_eq!(
            workflow.get("response").and_then(Value::as_str),
            Some("Answer already present.")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/status")
                .and_then(Value::as_str),
            Some("synthesized")
        );
    }

    #[test]
    fn successful_tool_result_answer_is_not_rejected_as_missing_direct_answer() {
        let tools = vec![json!({
            "name": "batch_query",
            "status": "ok",
            "is_error": false,
            "blocked": false,
            "result": "Key findings: OpenHands is an AI agent platform for software development."
        })];

        assert!(response_answers_successful_tool_result(
            "Use web search to find one current source about OpenHands agent framework, then summarize it in one sentence.",
            "OpenHands is an AI agent platform focused on software-development automation.",
            &tools,
        ));
    }

    #[test]
    fn successful_tool_result_answer_rejects_ack_without_findings() {
        let tools = vec![json!({
            "name": "batch_query",
            "status": "ok",
            "is_error": false,
            "blocked": false,
            "result": "Key findings: OpenHands is an AI agent platform for software development."
        })];

        assert!(!response_answers_successful_tool_result(
            "Use web search to find one current source about OpenHands agent framework, then summarize it in one sentence.",
            "I found some results and will summarize them now.",
            &tools,
        ));
    }

    #[test]
    fn successful_tool_result_answer_rejects_unrelated_context_dump() {
        let tools = vec![json!({
            "name": "batch_query",
            "status": "ok",
            "is_error": false,
            "blocked": false,
            "result": "Key findings: From web retrieval: openhands.dev describes OpenHands as an AI agent platform for software development."
        })];

        assert!(!response_answers_successful_tool_result(
            "Use web search to find one current source about OpenHands agent framework, then summarize it in one sentence.",
            "# 第一章\n\n社会管理创新，是指在现有社会管理条件下，运用现有的资源和经验。",
            &tools,
        ));
    }

    #[test]
    fn raw_tool_payload_dump_is_rejected_before_visible_chat() {
        assert!(response_looks_like_raw_tool_payload_dump(
            "<?xml version=\"1.0\"?><CustomMetadata xmlns=\"urn:test\"></CustomMetadata>"
        ));
        assert!(response_looks_like_raw_tool_payload_dump(
            "<function=web_search>{\"query\":\"x\"}</function>"
        ));
        assert!(response_looks_like_raw_tool_payload_dump(
            "{\"tool\":\"web_search\",\"query\":\"latest web frameworks\"}"
        ));
        assert!(response_looks_like_raw_tool_payload_dump(
            "{\"name\":\"batch_query\",\"status\":\"ok\",\"result\":\"items\"}"
        ));
        assert!(response_looks_like_raw_tool_payload_dump(
            "{\"query\":\"agentic frameworks\",\"source\":\"web\",\"results\":[\"a\",\"b\"]}"
        ));
        assert!(response_looks_like_raw_tool_payload_dump(
            "[{\"tool\":\"web_search\",\"query\":\"foo\"},{\"tool\":\"web_fetch\",\"query\":\"bar\"}]"
        ));
        assert!(response_looks_like_raw_tool_payload_dump(
            "<tool>web_search</tool><query>foo</query>"
        ));
        assert!(response_looks_like_raw_tool_payload_dump(
            "{\"choices\":[{\"finish_reason\":\"length\",\"index\":0}],\"usage\":{\"prompt_tokens\":11,\"completion_tokens\":30,\"total_tokens\":41},\"refusal\":\"I am Kimi, an AI assistant created by Moonshot AI.\"}"
        ));
        assert!(!response_looks_like_raw_tool_payload_dump(
            "OpenHands is an AI agent platform for software development."
        ));
        assert!(!response_looks_like_raw_tool_payload_dump(
            "{\"answer\":\"OpenHands is an AI agent platform for software development.\"}"
        ));
    }

    #[test]
    fn workflow_prompt_analysis_is_rejected_before_visible_chat() {
        assert!(response_contains_workflow_prompt_analysis_leak(
            "According to the instructions, the gate is What kind of work is this? The user asks to respond directly, so we answer normally."
        ));
        assert!(response_contains_workflow_prompt_analysis_leak(
            "We are in the runtime context of 2026-05-02T06:14:40Z. The user asks for a reply in exactly five words. We must reply in one short sentence."
        ));
        assert_eq!(
            direct_llm_response_from_initial_draft(
                "According to the instructions, the gate is What kind of work is this? The user asks to respond directly, so we answer normally."
            ),
            Some("According to the instructions, the gate is What kind of work is this? The user asks to respond directly, so we answer normally.".to_string())
        );
    }

    #[test]
    fn numeric_workflow_gate_submission_selects_json_alias_category() {
        assert!(response_is_tool_bearing_category_gate_submission("3"));
        let (category_key, category_label) =
            workflow_category_selection(&default_workflow_tool_menu_contract(), "3", Some(true))
                .expect("numeric web research alias");

        assert_eq!(category_key, "web_research");
        assert_eq!(category_label, "Web research");
    }

    #[test]
    fn structured_response_gate_fragment_selects_json_alias_category() {
        let response = r#""response_gate": "3"}"#;

        assert!(response_is_tool_bearing_category_gate_submission(response));
        let (category_key, category_label) = workflow_category_selection(
            &default_workflow_tool_menu_contract(),
            response,
            Some(true),
        )
        .expect("response_gate web research alias");

        assert_eq!(category_key, "web_research");
        assert_eq!(category_label, "Web research");
        assert!(response_is_tool_bearing_category_gate_submission(
            r#"{"gate": 3}"#
        ));
        assert!(response_is_tool_bearing_category_gate_submission(
            r#""workflow_gate": 3}"#
        ));
    }

    #[test]
    fn structured_gate_submission_accepts_combined_option_and_label_token() {
        let response = r#"{"gate":"4 = Workspace/files"}"#;

        assert!(response_is_tool_bearing_category_gate_submission(response));
        let (category_key, category_label) = workflow_category_selection(
            &default_workflow_tool_menu_contract(),
            response,
            Some(true),
        )
        .expect("combined category token");

        assert_eq!(category_key, "workspace_files");
        assert_eq!(category_label, "Workspace/files");
    }

    #[test]
    fn structured_no_tool_gate_submission_preserves_llm_final_answer() {
        let response = r#"{"gate":1,"token":"1","final_answer":"Hey there - I'm here and ready."}"#;

        assert!(response_is_exact_no_tool_gate_submission(response));
        assert_eq!(
            workflow_structured_gate_final_answer(response),
            Some("Hey there - I'm here and ready.".to_string())
        );
        assert_eq!(
            workflow_structured_gate_final_answer(r#""gate_6_final_answer": "Hey, I'm here!""#),
            Some("Hey, I'm here!".to_string())
        );
    }
}
