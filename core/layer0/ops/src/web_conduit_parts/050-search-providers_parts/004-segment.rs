fn search_payload_error(payload: &Value) -> String {
    let explicit = clean_text(
        payload.get("error").and_then(Value::as_str).unwrap_or(""),
        220,
    );
    if !explicit.is_empty() {
        if explicit == "web_conduit_policy_denied" {
            if let Some(policy_error) = search_payload_policy_denial_error(payload) {
                return policy_error;
            }
        }
        return explicit;
    }
    if payload_looks_like_search_challenge(payload) {
        return "anti_bot_challenge".to_string();
    }
    if search_payload_looks_competitive_programming_dump(payload) {
        return "query_result_mismatch".to_string();
    }
    if payload_looks_low_signal_search(payload) {
        return "low_signal_search_payload".to_string();
    }
    let summary = clean_text(
        payload.get("summary").and_then(Value::as_str).unwrap_or(""),
        1_200,
    );
    if search_summary_has_low_signal_marker(&summary) {
        return "low_signal_search_payload".to_string();
    }
    if !payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        return "search_provider_failed".to_string();
    }
    "no_usable_summary".to_string()
}

fn search_payload_policy_denial_error(payload: &Value) -> Option<String> {
    let reason = clean_text(
        payload
            .pointer("/policy_decision/reason")
            .and_then(Value::as_str)
            .unwrap_or(""),
        160,
    )
    .to_ascii_lowercase();
    match reason.as_str() {
        "rate_limit_exceeded" => Some("web_conduit_rate_limited".to_string()),
        "human_approval_required_for_sensitive_domain" => {
            Some("web_conduit_human_approval_required".to_string())
        }
        "domain_denylisted" | "domain_not_allowlisted" => {
            Some(format!("web_conduit_policy_denied:{reason}"))
        }
        _ => None,
    }
}
fn search_summary_has_low_signal_marker(summary: &str) -> bool {
    let lowered = clean_text(summary, 1_200).to_ascii_lowercase();
    if lowered.is_empty() {
        return true;
    }
    [
        "no relevant results found for that request yet",
        "couldn't produce source-backed findings in this turn",
        "don't have usable tool findings from this turn yet",
        "this turn only produced low-signal or no-result output",
        "retry with a narrower query or one specific source url",
        "search providers returned no usable findings"
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}
