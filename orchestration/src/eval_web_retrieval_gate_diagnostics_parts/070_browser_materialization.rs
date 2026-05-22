fn browser_materialization_recovery_diagnostics(
    payload: &Value,
    retrieval_quality: &Value,
) -> Value {
    let mut refs = Vec::<String>::new();
    let mut failure_signals = Vec::<String>::new();
    let mut recommended = false;
    let mut attempted = false;
    let mut capability_declared = false;
    scan_browser_materialization_recovery(
        payload,
        "payload",
        &mut recommended,
        &mut attempted,
        &mut capability_declared,
        &mut failure_signals,
        &mut refs,
    );
    scan_browser_materialization_recovery(
        retrieval_quality,
        "retrieval_quality",
        &mut recommended,
        &mut attempted,
        &mut capability_declared,
        &mut failure_signals,
        &mut refs,
    );
    refs.sort_unstable();
    refs.dedup();
    failure_signals.sort_unstable();
    failure_signals.dedup();
    json!({
        "schema_version": 1,
        "capability": "browser_materialize_page",
        "recommended_when_policy_allows": recommended,
        "attempted": attempted,
        "capability_declared": capability_declared,
        "failed": !failure_signals.is_empty(),
        "failure_signals": failure_signals,
        "artifact_refs": refs,
        "note": "Measures whether access-blocked runs expose an optional browser-materialization recovery lane. This does not require or default to browser execution."
    })
}

fn scan_browser_materialization_recovery(
    value: &Value,
    path: &str,
    recommended: &mut bool,
    attempted: &mut bool,
    capability_declared: &mut bool,
    failure_signals: &mut Vec<String>,
    refs: &mut Vec<String>,
) {
    if browser_materialization_declarative_path(path) {
        return;
    }
    match value {
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
        Value::String(raw) => {
            let normalized = normalize_for_compare(raw);
            if normalized.contains("browser materialization")
                || normalized.contains("browser_materialization")
                || normalized.contains("browser_materialize_page")
            {
                *capability_declared = true;
                refs.push(path.to_string());
            }
            if normalized.contains("browser_materialization_attempted")
                || normalized.contains("browser materialization attempted")
            {
                *attempted = true;
                refs.push(path.to_string());
            }
            if normalized.contains("recommended_when_policy_allows")
                || normalized.contains("browser materialization recommended")
            {
                *recommended = true;
                refs.push(path.to_string());
            }
            scan_browser_materialization_failure_text(&normalized, path, failure_signals, refs);
        }
        Value::Array(rows) => {
            for (index, row) in rows.iter().enumerate() {
                scan_browser_materialization_recovery(
                    row,
                    &format!("{path}.{index}"),
                    recommended,
                    attempted,
                    capability_declared,
                    failure_signals,
                    refs,
                );
            }
        }
        Value::Object(map) => {
            let path_normalized = normalize_for_compare(path);
            let declares_browser_capability = map
                .get("capability")
                .and_then(Value::as_str)
                .map(|raw| raw == "browser_materialize_page")
                .unwrap_or(false);
            let browser_context_object = path_normalized.contains("browser materialization")
                || path_normalized.contains("browser_materialization")
                || declares_browser_capability;
            if path_normalized.contains("browser materialization") {
                *capability_declared = true;
                refs.push(path.to_string());
            }
            if declares_browser_capability {
                *capability_declared = true;
                refs.push(path.to_string());
            }
            if browser_context_object
                && map
                    .get("recommended_when_policy_allows")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
            {
                *recommended = true;
                refs.push(format!("{path}.recommended_when_policy_allows"));
            }
            if browser_context_object
                && map
                    .get("attempted")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
            {
                *attempted = true;
                refs.push(format!("{path}.attempted"));
            }
            if browser_context_object
                && (map.get("failed").and_then(Value::as_bool).unwrap_or(false)
                    || map
                        .get("success")
                        .and_then(Value::as_bool)
                        .map(|success| !success)
                        .unwrap_or(false)
                    || map
                        .get("status")
                        .and_then(Value::as_str)
                        .map(|status| {
                            matches!(
                                normalize_for_compare(status).as_str(),
                                "failed" | "error" | "timeout" | "blocked"
                            )
                        })
                        .unwrap_or(false))
            {
                failure_signals.push("browser_materialization_failed".to_string());
                refs.push(path.to_string());
            }
            for (key, child) in map {
                scan_browser_materialization_recovery(
                    child,
                    &format!("{path}.{key}"),
                    recommended,
                    attempted,
                    capability_declared,
                    failure_signals,
                    refs,
                );
            }
        }
    }
}

fn browser_materialization_declarative_path(path: &str) -> bool {
    let normalized = normalize_for_compare(&path.replace(['.', '_', '-'], " "));
    [
        "blocker taxonomy",
        "profile compilation",
        "readiness lifecycle",
        "url safety",
        "non goals",
        "source pattern",
        "tool cd",
        "tooling cd",
        "capability contract",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn scan_browser_materialization_failure_text(
    normalized: &str,
    path: &str,
    failure_signals: &mut Vec<String>,
    refs: &mut Vec<String>,
) {
    let path_normalized = normalize_for_compare(path);
    let browser_context = path_normalized.contains("browser materialization")
        || path_normalized.contains("browser_materialization")
        || normalized.contains("browser_materialization_failed")
        || normalized.contains("browser materialization failed");
    if !browser_context {
        return;
    }
    let markers = [
        (
            "browser_materialization_failed",
            "browser_materialization_failed",
        ),
        (
            "browser materialization failed",
            "browser_materialization_failed",
        ),
        ("navigation timeout", "navigation_timeout"),
        ("timed out", "navigation_timeout"),
        ("timeout", "navigation_timeout"),
        ("extraction failed", "content_extraction_failed"),
        ("empty page", "empty_materialized_page"),
        (
            "browser_materialization_blocked",
            "browser_materialization_blocked",
        ),
        (
            "browser materialization blocked",
            "browser_materialization_blocked",
        ),
    ];
    for (needle, signal) in markers {
        if normalized.contains(needle) {
            failure_signals.push(signal.to_string());
            refs.push(path.to_string());
        }
    }
}
