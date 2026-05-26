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

fn workflow_status_blocks_runtime_visible_fallback(status: &str) -> bool {
    matches!(
        clean_text(status, 120).as_str(),
        "tool_evidence_fallback_suppressed"
    )
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
