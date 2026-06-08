fn web_provider_supply_diagnostics(payload: &Value, retrieval_quality: &Value) -> Value {
    let mut state = ProviderSupplyScan::default();
    scan_provider_supply(payload, "payload", &mut state);
    scan_provider_supply(retrieval_quality, "retrieval_quality", &mut state);
    state.signals.sort_unstable();
    state.signals.dedup();
    state.refs.sort_unstable();
    state.refs.dedup();

    let missing_config = state.signals.iter().any(|signal| {
        matches!(
            signal.as_str(),
            "serper_api_key_missing"
                | "missing_api_key"
                | "invalid_api_key"
                | "strong_search_provider_missing"
                | "provider_not_configured"
                | "missing_provider_credentials"
        )
    });
    let circuit_open = state
        .signals
        .iter()
        .any(|signal| signal == "provider_circuit_open");
    let surface_degraded = state.signals.iter().any(|signal| {
        matches!(
            signal.as_str(),
            "web_search_tool_surface_degraded"
                | "tool_surface_degraded"
                | "provider_degraded"
                | "provider_error"
                | "transport_error"
        )
    });
    let provider_blocked = state.signals.iter().any(|signal| {
        matches!(
            signal.as_str(),
            "anti_bot_challenge"
                | "web_conduit_policy_denied"
                | "access_denied"
                | "rate_limited"
                | "query_result_mismatch"
        )
    });
    let raw_row_count = state.provider_raw_rows.max(state.provider_result_count);
    let candidate_row_count = state.candidate_rows;
    let filtered_row_count = state.filtered_rows;
    let configuration_usable = !missing_config || candidate_row_count > 0 || raw_row_count > 0;
    state.browser_serp_outcome_classes.sort_unstable();
    state.browser_serp_outcome_classes.dedup();
    let browser_serp_shell_or_no_organic = state.browser_serp_shell_without_organic
        || (state.browser_serp_attempted
            && state.browser_serp_raw_rows > 0
            && state.browser_serp_external_url_rows == 0
            && !state.browser_serp_challenge_detected);
    json!({
        "schema_version": 1,
        "configuration_usable": configuration_usable,
        "missing_configuration_detected": missing_config,
        "circuit_open_detected": circuit_open,
        "tool_surface_degraded": surface_degraded,
        "provider_blocked_or_denied": provider_blocked,
        "raw_row_count": raw_row_count,
        "provider_result_count": state.provider_result_count,
        "provider_raw_row_count": state.provider_raw_rows,
        "candidate_row_count": candidate_row_count,
        "synthesis_candidate_row_count": state.synthesis_candidate_rows,
        "filtered_or_rejected_row_count": filtered_row_count,
        "low_confidence_raw_row_count": state.low_confidence_raw_rows,
        "browser_serp_attempted": state.browser_serp_attempted,
        "browser_serp_raw_row_count": state.browser_serp_raw_rows,
        "browser_serp_external_url_count": state.browser_serp_external_url_rows,
        "browser_serp_challenge_detected": state.browser_serp_challenge_detected,
        "browser_serp_shell_or_no_organic": browser_serp_shell_or_no_organic,
        "browser_serp_outcome_classes": state.browser_serp_outcome_classes,
        "signals": state.signals,
        "artifact_refs": state.refs,
        "note": "Separates provider supply into configuration, circuit-breaker, surface readiness, raw-row availability, and candidate-promotion signals."
    })
}

#[derive(Default)]
struct ProviderSupplyScan {
    provider_result_count: u64,
    provider_raw_rows: u64,
    candidate_rows: u64,
    synthesis_candidate_rows: u64,
    filtered_rows: u64,
    low_confidence_raw_rows: u64,
    browser_serp_attempted: bool,
    browser_serp_raw_rows: u64,
    browser_serp_external_url_rows: u64,
    browser_serp_challenge_detected: bool,
    browser_serp_shell_without_organic: bool,
    browser_serp_outcome_classes: Vec<String>,
    signals: Vec<String>,
    refs: Vec<String>,
}

fn scan_provider_supply(value: &Value, path: &str, state: &mut ProviderSupplyScan) {
    if provider_supply_declarative_path(path) {
        return;
    }
    match value {
        Value::Null | Value::Bool(_) => {}
        Value::Number(raw) => {
            if let Some(number) = raw.as_u64() {
                let normalized_path = normalize_for_compare(&path.replace(['.', '_', '-'], " "));
                if normalized_path.contains("provider result count")
                    || normalized_path.contains("provider result dedup count")
                {
                    state.provider_result_count = state.provider_result_count.max(number);
                    state.refs.push(path.to_string());
                } else if normalized_path.contains("provider raw rows")
                    || normalized_path.contains("provider raw row")
                    || normalized_path.contains("provider raw count")
                {
                    state.provider_raw_rows = state.provider_raw_rows.max(number);
                    state.refs.push(path.to_string());
                } else if normalized_path.contains("synthesis candidate rows")
                    || normalized_path.contains("synthesis candidate row")
                {
                    state.synthesis_candidate_rows = state.synthesis_candidate_rows.max(number);
                    state.refs.push(path.to_string());
                } else if normalized_path.contains("candidate rows")
                    || normalized_path.contains("candidate row")
                    || normalized_path.contains("candidate count")
                {
                    state.candidate_rows = state.candidate_rows.max(number);
                    state.refs.push(path.to_string());
                } else if normalized_path.contains("filtered or rejected")
                    || normalized_path.contains("filtered rows")
                    || normalized_path.contains("rejected rows")
                {
                    state.filtered_rows = state.filtered_rows.max(number);
                    state.refs.push(path.to_string());
                } else if normalized_path.contains("low confidence raw rows")
                    || normalized_path.contains("low confidence raw row")
                {
                    state.low_confidence_raw_rows = state.low_confidence_raw_rows.max(number);
                    state.refs.push(path.to_string());
                }
            }
        }
        Value::String(raw) => scan_provider_supply_text(raw, path, state),
        Value::Array(rows) => {
            for (index, row) in rows.iter().enumerate() {
                scan_provider_supply(row, &format!("{path}.{index}"), state);
            }
        }
        Value::Object(map) => {
            scan_browser_serp_supply_object(map, path, state);
            for (key, child) in map {
                scan_provider_supply(child, &format!("{path}.{key}"), state);
            }
        }
    }
}

fn scan_browser_serp_supply_object(
    map: &serde_json::Map<String, Value>,
    path: &str,
    state: &mut ProviderSupplyScan,
) {
    let provider_is_browser_serp = ["provider", "selected_provider", "provider_hint", "requested_provider_hint"]
        .iter()
        .filter_map(|key| map.get(*key).and_then(Value::as_str))
        .any(is_browser_serp_provider_value);
    let has_browser_serp_diagnostics = map.get("browser_serp_diagnostics").is_some()
        || map.get("browser_serp").is_some();
    if !(provider_is_browser_serp || has_browser_serp_diagnostics) {
        return;
    }
    state.browser_serp_attempted = true;
    state.refs.push(path.to_string());
    let raw_rows = ["provider_raw_count", "provider_raw_rows", "provider_raw_row_count"]
        .iter()
        .filter_map(|key| map.get(*key).and_then(Value::as_u64))
        .max()
        .unwrap_or(0);
    let external_rows = ["provider_filtered_count", "provider_filtered_rows", "candidate_rows", "candidate_count"]
        .iter()
        .filter_map(|key| map.get(*key).and_then(Value::as_u64))
        .max()
        .unwrap_or(0);
    state.browser_serp_raw_rows = state.browser_serp_raw_rows.max(raw_rows);
    state.browser_serp_external_url_rows =
        state.browser_serp_external_url_rows.max(external_rows);
    scan_browser_serp_diagnostics_value(&Value::Object(map.clone()), state);
}

fn scan_browser_serp_diagnostics_value(value: &Value, state: &mut ProviderSupplyScan) {
    match value {
        Value::Array(rows) => {
            for row in rows {
                scan_browser_serp_diagnostics_value(row, state);
            }
        }
        Value::Object(map) => {
            scan_browser_serp_diagnostic_object(map, state);
            for key in ["browser_serp", "browser_serp_diagnostics", "outcome_classification"] {
                if let Some(child) = map.get(key) {
                    scan_browser_serp_diagnostics_value(child, state);
                }
            }
        }
        _ => {}
    }
}

fn scan_browser_serp_diagnostic_object(
    map: &serde_json::Map<String, Value>,
    state: &mut ProviderSupplyScan,
) {
    if map
        .get("challenge_detected")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || map
            .get("browser_serp")
            .and_then(|value| value.get("challenge_detected"))
            .and_then(Value::as_bool)
            .unwrap_or(false)
    {
        state.browser_serp_challenge_detected = true;
    }
    let outcome_class = map
        .get("outcome_class")
        .or_else(|| map.get("outcome"))
        .and_then(Value::as_str)
        .map(|raw| clean_text(raw, 120));
    if let Some(outcome_class) = outcome_class {
        if !outcome_class.is_empty()
            && !state
                .browser_serp_outcome_classes
                .iter()
                .any(|row| row == &outcome_class)
        {
            state.browser_serp_outcome_classes.push(outcome_class.clone());
        }
        if matches!(
            outcome_class.as_str(),
            "serp_shell_without_organic_results" | "no_organic_results_extracted"
        ) {
            state.browser_serp_shell_without_organic = true;
            state
                .signals
                .push("browser_serp_shell_without_organic_results".to_string());
        }
    }
}

fn is_browser_serp_provider_value(raw: &str) -> bool {
    matches!(
        normalize_for_compare(raw).as_str(),
        "browser serp" | "browser search" | "serp browser"
    ) || raw.trim().eq_ignore_ascii_case("browser_serp")
        || raw.trim().eq_ignore_ascii_case("browser-serp")
}

fn provider_supply_declarative_path(path: &str) -> bool {
    let normalized = normalize_for_compare(&path.replace(['.', '_', '-'], " "));
    [
        "blocker taxonomy",
        "recommended next capability",
        "query refinement signals",
        "non goals",
        "tool cd",
        "tooling cd",
        "capability contract",
        "request contract supports filters",
        "input contract",
        "plain english",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn scan_provider_supply_text(raw: &str, path: &str, state: &mut ProviderSupplyScan) {
    let normalized = normalize_for_compare(raw);
    let markers = [
        ("serper_api_key_missing", "serper_api_key_missing"),
        ("serper api key missing", "serper_api_key_missing"),
        ("missing api key", "missing_api_key"),
        ("api key missing", "missing_api_key"),
        ("invalid api key", "invalid_api_key"),
        ("strong search provider", "strong_search_provider_missing"),
        ("strong_search_provider", "strong_search_provider_missing"),
        (
            "missing provider credentials",
            "missing_provider_credentials",
        ),
        ("provider not configured", "provider_not_configured"),
        ("provider_circuit_open", "provider_circuit_open"),
        ("provider circuit open", "provider_circuit_open"),
        (
            "web_search_tool_surface_degraded",
            "web_search_tool_surface_degraded",
        ),
        ("tool surface degraded", "tool_surface_degraded"),
        ("provider degraded", "provider_degraded"),
        ("provider_degraded", "provider_degraded"),
        ("provider_error", "provider_error"),
        ("provider error", "provider_error"),
        ("transport_error", "transport_error"),
        ("transport error", "transport_error"),
        ("anti_bot_challenge", "anti_bot_challenge"),
        ("anti bot challenge", "anti_bot_challenge"),
        ("web_conduit_policy_denied", "web_conduit_policy_denied"),
        ("policy denied", "web_conduit_policy_denied"),
        ("access denied", "access_denied"),
        ("rate_limited", "rate_limited"),
        ("rate limited", "rate_limited"),
        ("query_result_mismatch", "query_result_mismatch"),
        ("low_signal_search_payload", "low_signal_search_payload"),
    ];
    for (needle, signal) in markers {
        if normalized.contains(needle) {
            state.signals.push(signal.to_string());
            state.refs.push(path.to_string());
        }
    }
}
