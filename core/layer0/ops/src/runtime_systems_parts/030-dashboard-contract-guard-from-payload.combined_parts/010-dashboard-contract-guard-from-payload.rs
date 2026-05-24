// Layer ownership: core/layer0/ops (authoritative)

fn dashboard_contract_guard_from_payload(payload: &Value) -> Value {
    let input_text = payload_string(payload, "input_text", "");
    let lowered = input_text.to_ascii_lowercase();
    let recent_messages = payload_u64(payload, "recent_messages", 0).min(2_000_000);
    let max_per_min =
        payload_u64(payload, "rogue_message_rate_max_per_min", 20).clamp(1, 1_000_000);

    let contains_any = |terms: &[&str]| -> bool { terms.iter().any(|term| lowered.contains(term)) };
    let normalized_words = lowered
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { ' ' })
        .collect::<String>();
    let has_word = |word: &str| {
        normalized_words
            .split_whitespace()
            .any(|token| token == word)
    };
    let has_any_word = |terms: &[&str]| terms.iter().any(|term| has_word(term));
    let sensitive_target = contains_any(&[
        "api key",
        "private key",
        "customer data",
        "user data",
        "access token",
    ]) || has_any_word(&[
        "secret",
        "secrets",
        "credential",
        "credentials",
        "password",
        "passwords",
        "token",
        "tokens",
        "pii",
    ]);
    let malicious_secret_action = has_any_word(&["exfiltrate", "steal"])
        || contains_any(&["data exfil"])
        || (has_any_word(&["dump", "leak", "expose", "extract"]) && sensitive_target);

    let mut reason = String::new();
    let mut detail = String::new();
    if contains_any(&["ignore", "bypass", "disable", "override"])
        && contains_any(&["contract", "safety", "receipt", "policy"])
    {
        reason = "contract_override_attempt".to_string();
        detail = "input_requested_contract_bypass".to_string();
    } else if malicious_secret_action {
        reason = "data_exfiltration_attempt".to_string();
        detail = "input_requested_exfiltration".to_string();
    } else if contains_any(&["extend", "increase"])
        && contains_any(&["expiry", "ttl", "time to live", "contract"])
    {
        reason = "self_extension_attempt".to_string();
        detail = "input_requested_expiry_extension".to_string();
    } else if recent_messages > max_per_min {
        reason = "message_rate_spike".to_string();
        detail = format!("recent_messages={recent_messages}");
    }

    json!({
        "authority": "rust_runtime_systems",
        "policy": "V6-DASHBOARD-007.3",
        "violation": !reason.is_empty(),
        "reason": reason,
        "detail": detail,
        "recent_messages": recent_messages,
        "rogue_message_rate_max_per_min": max_per_min,
        "input_sha256": sha256_hex(input_text.as_bytes())
    })
}
