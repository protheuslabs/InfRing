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

fn dashboard_auto_route_from_payload(payload: &Value) -> Value {
    let input_text = payload_string(payload, "input_text", "");
    let lowered = input_text.to_ascii_lowercase();
    let token_count = payload_u64(payload, "token_count", (input_text.len() as u64 / 4).max(1))
        .clamp(1, 8_000_000);
    let has_vision = payload_bool(payload, "has_vision", false);
    let asks_speed = payload_bool(
        payload,
        "asks_speed",
        lowered.contains("fast") || lowered.contains("speed"),
    );
    let asks_cost = payload_bool(
        payload,
        "asks_cost",
        lowered.contains("cheap") || lowered.contains("cost"),
    );
    let asks_quality = payload_bool(
        payload,
        "asks_quality",
        lowered.contains("quality") || lowered.contains("best"),
    );
    let asks_long_context = payload_bool(
        payload,
        "asks_long_context",
        token_count >= 100_000 || lowered.contains("long context"),
    );

    let preferred_provider = payload_string(payload, "preferred_provider", "ollama");
    let preferred_model = payload_string(payload, "preferred_model", "llama3.2:3b");
    let fallback_provider = payload_string(payload, "fallback_provider", "cloud");
    let fallback_model = payload_string(payload, "fallback_model", "kimi2.5:cloud");

    let mut raw_candidates = payload_array(payload, "candidates");
    if raw_candidates.is_empty() {
        raw_candidates.push(json!({
            "runtime_provider": preferred_provider,
            "runtime_model": preferred_model
        }));
        raw_candidates.push(json!({
            "runtime_provider": fallback_provider,
            "runtime_model": fallback_model
        }));
    }

    let runtime_success = payload_f64(payload, "spine_success_rate", 0.90).clamp(0.0, 1.0);
    let mut scored = Vec::<Value>::new();
    for candidate in raw_candidates {
        let provider = payload_string(
            &candidate,
            "runtime_provider",
            payload_string(&candidate, "provider", "ollama").as_str(),
        );
        let model = payload_string(
            &candidate,
            "runtime_model",
            payload_string(&candidate, "model", "llama3.2:3b").as_str(),
        );
        let model_lower = model.to_ascii_lowercase();
        let (prior_latency, prior_cost, prior_success): (f64, f64, f64) =
            match provider.to_ascii_lowercase().as_str() {
                "ollama" => (120.0_f64, 0.0_f64, 0.92_f64),
                "groq" => (65.0_f64, 0.2_f64, 0.90_f64),
                "openai" => (90.0_f64, 0.55_f64, 0.95_f64),
                "frontier_provider" => (105.0_f64, 0.7_f64, 0.95_f64),
                "google" => (95.0_f64, 0.6_f64, 0.94_f64),
                "cloud" => (80.0_f64, 0.3_f64, 0.93_f64),
                _ => (110.0_f64, 0.45_f64, 0.90_f64),
            };
        let model_is_small = model_lower.contains("3b")
            || model_lower.contains("mini")
            || model_lower.contains("small");
        let latency_ms =
            (prior_latency * if model_is_small { 0.85_f64 } else { 1.0_f64 }).max(1.0_f64);
        let cost_per_1k =
            (prior_cost * if model_is_small { 0.7_f64 } else { 1.0_f64 }).max(0.0_f64);
        let context_window = candidate
            .get("context_window")
            .and_then(Value::as_u64)
            .unwrap_or(8192)
            .clamp(1024, 8_000_000);
        let context_score = if token_count <= context_window {
            1.0
        } else {
            (context_window as f64 / token_count as f64).clamp(0.1, 1.0)
        };
        let latency_score = 1.0 / (1.0 + (latency_ms / 120.0));
        let cost_score = 1.0 / (1.0 + cost_per_1k);
        let success_rate = ((prior_success * 0.65) + (runtime_success * 0.35)).clamp(0.2, 0.99);
        let supports_vision = candidate
            .get("supports_vision")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let vision_penalty = if has_vision && !supports_vision {
            0.55
        } else {
            0.0
        };
        let speed_weight = if asks_speed { 1.55 } else { 1.05 };
        let cost_weight = if asks_cost { 1.35 } else { 0.75 };
        let quality_weight = if asks_quality { 1.8 } else { 1.3 };
        let context_weight = if asks_long_context { 1.45 } else { 1.1 };
        let score = (latency_score * speed_weight)
            + (cost_score * cost_weight)
            + (success_rate * quality_weight)
            + (context_score * context_weight)
            - vision_penalty;
        scored.push(json!({
            "provider": provider,
            "model": model,
            "score": (score * 1_000_000.0).round() / 1_000_000.0,
            "latency_ms": latency_ms.round() as u64,
            "cost_per_1k": ((cost_per_1k * 10_000.0).round()) / 10_000.0,
            "success_rate": (success_rate * 10_000.0).round() / 10_000.0,
            "context_window": context_window,
            "supports_vision": supports_vision
        }));
    }

    scored.sort_by(|a, b| {
        let lhs = b.get("score").and_then(Value::as_f64).unwrap_or(0.0);
        let rhs = a.get("score").and_then(Value::as_f64).unwrap_or(0.0);
        lhs.partial_cmp(&rhs).unwrap_or(std::cmp::Ordering::Equal)
    });

    let selected = scored.first().cloned().unwrap_or_else(|| {
        json!({
            "provider": preferred_provider,
            "model": preferred_model,
            "score": 0.0,
            "latency_ms": 120,
            "cost_per_1k": 0.0,
            "success_rate": 0.9,
            "context_window": 8192,
            "supports_vision": false
        })
    });
    let selected_provider = payload_string(&selected, "provider", "ollama");
    let selected_model = payload_string(&selected, "model", "llama3.2:3b");
    let selected_context_window = selected
        .get("context_window")
        .and_then(Value::as_u64)
        .unwrap_or(8192);
    let reason = format!(
        "rust auto-route selected {} / {} by weighted latency-cost-success-context scoring",
        selected_provider, selected_model
    );
    let fallback_chain = scored
        .iter()
        .skip(1)
        .take(3)
        .map(|row| {
            json!({
                "provider": payload_string(row, "provider", ""),
                "model": payload_string(row, "model", ""),
                "score": row.get("score").and_then(Value::as_f64).unwrap_or(0.0)
            })
        })
        .collect::<Vec<_>>();

    let mut decision = json!({
        "authority": "rust_runtime_systems",
        "policy": "V6-DASHBOARD-008.1",
        "route_lane": "runtime-systems.run",
        "selected_provider": selected_provider,
        "selected_model": selected_model,
        "selected_model_id": format!(
            "{}/{}",
            payload_string(&selected, "provider", "ollama"),
            payload_string(&selected, "model", "llama3.2:3b")
        ),
        "selected_context_window": selected_context_window,
        "reason": reason,
        "context": {
            "token_count": token_count,
            "has_vision": has_vision,
            "asks_speed": asks_speed,
            "asks_cost": asks_cost,
            "asks_quality": asks_quality,
            "asks_long_context": asks_long_context
        },
        "fallback_chain": fallback_chain,
        "candidates": scored,
        "runtime_sync": {
            "spine_success_rate": runtime_success,
            "receipt_latency_p99_ms": payload_f64(payload, "receipt_latency_p99_ms", 0.0).max(0.0)
        }
    });
    let hash = receipt_hash(&decision);
    decision["receipt_hash"] = Value::String(hash);
    decision
}

fn payload_string_list(payload: &Value, key: &str) -> Vec<String> {
    payload_array(payload, key)
        .into_iter()
        .map(|row| match row {
            Value::String(text) => text,
            other => other
                .as_str()
                .map(|text| text.to_string())
                .unwrap_or_default(),
        })
        .filter(|row| !row.trim().is_empty())
        .map(|row| row.to_ascii_lowercase())
        .collect::<Vec<_>>()
}

fn push_if_disabled(violations: &mut Vec<String>, enabled: bool, code: &str) {
    if !enabled {
        violations.push(code.to_string());
    }
}

fn dashboard_message_stack_guard_from_payload(payload: &Value) -> (Value, Vec<String>) {
    let metadata_hover_scope = payload_string(payload, "metadata_hover_scope", "message_only");
    let hover_pushdown_layout_enabled =
        payload_bool(payload, "hover_pushdown_layout_enabled", true);
    let stack_interrupts_on_notifications =
        payload_bool(payload, "stack_interrupts_on_notifications", true);
    let messages = payload_array(payload, "messages");
    let mut previous_key = String::new();
    let mut source_runs = 0u64;
    let mut notification_rows = 0u64;
    for row in messages.iter() {
        let source = payload_string(
            row,
            "source",
            payload_string(row, "role", "unknown").as_str(),
        );
        let kind = payload_string(row, "kind", "message").to_ascii_lowercase();
        let is_notification = kind.contains("notification")
            || kind.contains("notice")
            || kind.contains("name_changed")
            || kind.contains("model_changed")
            || kind.contains("system_event");
        if is_notification {
            notification_rows = notification_rows.saturating_add(1);
        }
        let key = if is_notification {
            format!("notification::{source}")
        } else {
            format!("source::{source}")
        };
        if key != previous_key {
            source_runs = source_runs.saturating_add(1);
            previous_key = key;
        }
    }
    let expected_min_source_runs = payload_u64(
        payload,
        "expected_min_source_runs",
        if messages.is_empty() {
            0
        } else if notification_rows > 0 {
            2
        } else {
            1
        },
    );
    let mut violations = Vec::<String>::new();
    if metadata_hover_scope != "message_only" {
        violations.push(format!(
            "specific_dashboard_metadata_hover_scope_mismatch:{metadata_hover_scope}"
        ));
    }
    if !hover_pushdown_layout_enabled {
        violations.push("specific_dashboard_metadata_pushdown_disabled".to_string());
    }
    if notification_rows > 0 && !stack_interrupts_on_notifications {
        violations.push("specific_dashboard_notifications_must_interrupt_stack".to_string());
    }
    if source_runs < expected_min_source_runs {
        violations.push(format!(
            "specific_dashboard_source_run_count_too_low:{source_runs}<{}",
            expected_min_source_runs
        ));
    }

    (
        json!({
            "authority": "rust_runtime_systems",
            "policy": "V6-DASHBOARD-009.1",
            "metadata_hover_scope": metadata_hover_scope,
            "hover_pushdown_layout_enabled": hover_pushdown_layout_enabled,
            "stack_interrupts_on_notifications": stack_interrupts_on_notifications,
            "source_run_count": source_runs,
            "expected_min_source_runs": expected_min_source_runs,
            "notification_rows": notification_rows,
            "messages_seen": messages.len()
        }),
        violations,
    )
}

fn dashboard_boot_retry_guard_from_payload(payload: &Value) -> (Value, Vec<String>) {
    let boot_retry_enabled = payload_bool(payload, "boot_retry_enabled", true);
    let boot_retry_max_attempts = payload_u64(payload, "boot_retry_max_attempts", 5).clamp(1, 20);
    let boot_retry_backoff_ms =
        payload_u64(payload, "boot_retry_backoff_ms", 1000).clamp(1, 60_000);
    let startup_failed = payload_bool(payload, "startup_failed", false);
    let server_status_emitted = payload_bool(payload, "server_status_emitted", !startup_failed);
    let server_status_path = payload_string(
        payload,
        "server_status_path",
        "local/state/ops/daemon_control/server_status.json",
    );
    let status_error_code = payload_string(payload, "status_error_code", "");
    let mut violations = Vec::<String>::new();
    if !boot_retry_enabled {
        violations.push("specific_dashboard_boot_retry_disabled".to_string());
    }
    if boot_retry_max_attempts < 2 {
        violations.push(format!(
            "specific_dashboard_boot_retry_attempts_too_low:{boot_retry_max_attempts}"
        ));
    }
    if boot_retry_backoff_ms < 1000 {
        violations.push(format!(
            "specific_dashboard_boot_retry_backoff_too_low:{boot_retry_backoff_ms}"
        ));
    }
    if startup_failed && !server_status_emitted {
        violations.push("specific_dashboard_server_status_missing_on_failure".to_string());
    }
    if startup_failed && status_error_code.trim().is_empty() {
        violations.push("specific_dashboard_failure_missing_error_code".to_string());
    }
    if startup_failed && server_status_path.trim().is_empty() {
        violations.push("specific_dashboard_failure_missing_status_path".to_string());
    }

    (
        json!({
            "authority": "rust_runtime_systems",
            "policy": "V6-DASHBOARD-009.2",
            "boot_retry_enabled": boot_retry_enabled,
            "boot_retry_max_attempts": boot_retry_max_attempts,
            "boot_retry_backoff_ms": boot_retry_backoff_ms,
            "startup_failed": startup_failed,
            "server_status_emitted": server_status_emitted,
            "server_status_path": server_status_path,
            "status_error_code": status_error_code
        }),
        violations,
    )
}
