#[derive(Default)]
struct WebGateMetric {
    total: u64,
    passed: u64,
    failed: u64,
    artifact_present: u64,
    artifact_missing: u64,
    artifact_present_failures: u64,
    artifact_missing_failures: u64,
    first_failure_count: u64,
}

fn web_tooling_measured_rows(rows: &[Value]) -> Vec<&Value> {
    rows.iter()
        .filter(|row| web_tooling_measurement_exclusion_reason_row(row).is_none())
        .collect()
}

fn web_tooling_measurement_exclusion_reason_row(row: &Value) -> Option<&'static str> {
    if let Some(explicit) = row
        .get("web_tooling_measurement_exclusion")
        .and_then(Value::as_str)
    {
        return match explicit {
            "" | "none" => None,
            "transport_failure" => Some("transport_failure"),
            "post_tool_context_not_seeded" => Some("post_tool_context_not_seeded"),
            _ => None,
        };
    }
    if bool_at(row, &["transport_failure"], false) {
        return Some("transport_failure");
    }
    let retrieval_quality = row
        .get("web_tooling_retrieval_quality")
        .or_else(|| row.get("retrieval_quality"))
        .unwrap_or(&Value::Null);
    if str_at(row, &["category"], "") == "post_tool_synthesis"
        && !bool_at(retrieval_quality, &["tool_executed"], false)
        && str_at(retrieval_quality, &["status"], "") == "not_attempted"
    {
        return Some("post_tool_context_not_seeded");
    }
    None
}

fn unseeded_post_tool_synthesis_case(
    case: &Value,
    payload: &Value,
    retrieval_quality: &Value,
) -> bool {
    if str_at(case, &["category"], "") != "post_tool_synthesis" {
        return false;
    }
    let derived_fallback_request = str_at(
        payload,
        &[
            "pending_tool_request",
            "input",
            "query_metadata_policy",
            "classification",
        ],
        "",
    ) == "derived_prompt_request";
    (!has_tool_execution(payload)
        && web_pending_request(payload).is_none()
        && !bool_at(retrieval_quality, &["tool_executed"], false)
        && str_at(retrieval_quality, &["status"], "") == "not_attempted")
        || derived_fallback_request
}

fn web_gate(
    name: &str,
    artifact_present: bool,
    passed: bool,
    reason: &str,
    artifact_refs: Vec<String>,
) -> Value {
    json!({
        "gate": name,
        "status": if passed { "pass" } else { "fail" },
        "artifact_present": artifact_present,
        "reason": reason,
        "artifact_refs": artifact_refs
    })
}

fn access_blocker_refs(access_blocker: &Value) -> Vec<String> {
    access_blocker
        .get("artifact_refs")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|rows| !rows.is_empty())
        .unwrap_or_else(|| {
            vec![
                "tools.status".to_string(),
                "tools.result".to_string(),
                "tools.error".to_string(),
                "response_finalization.tool_completion.tool_attempts".to_string(),
            ]
        })
}

fn provider_supply_refs(provider_supply: &Value) -> Vec<String> {
    provider_supply
        .get("artifact_refs")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|rows| !rows.is_empty())
        .unwrap_or_else(|| {
            vec![
                "retrieval_telemetry".to_string(),
                "tool_result_quality.provider_attempts".to_string(),
                "provider_health".to_string(),
                "provider_errors".to_string(),
            ]
        })
}

