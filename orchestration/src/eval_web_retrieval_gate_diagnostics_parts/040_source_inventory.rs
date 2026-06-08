fn declared_source_preference_count(request_input: Option<&Value>) -> u64 {
    request_input
        .and_then(Value::as_object)
        .and_then(|map| map.get("source_preferences"))
        .and_then(Value::as_array)
        .map(|rows| rows.len() as u64)
        .unwrap_or(0)
}

fn unique_source_domain_count(payload: &Value) -> u64 {
    unique_domain_inventory(payload, false).len() as u64
}

fn unique_evidence_domain_count(payload: &Value) -> u64 {
    unique_domain_inventory(payload, true).len() as u64
}

fn unique_source_class_count(payload: &Value) -> u64 {
    let mut classes = Vec::<String>::new();
    for object in source_like_objects(payload) {
        if let Some(class) = source_class_value(object) {
            push_unique_case_insensitive(&mut classes, &class);
        }
    }
    classes.len() as u64
}

fn official_or_primary_source_count(payload: &Value) -> u64 {
    source_like_objects(payload)
        .iter()
        .filter_map(|object| source_class_value(object))
        .filter(|class| {
            let normalized = normalize_for_compare(class);
            normalized.contains("official") || normalized.contains("primary")
        })
        .count() as u64
}

fn unique_domain_inventory(payload: &Value, evidence_only: bool) -> Vec<String> {
    let mut domains = Vec::<String>::new();
    for object in source_like_objects(payload) {
        if evidence_only && !object_looks_like_evidence(object) {
            continue;
        }
        if let Some(domain) = source_domain_value(object) {
            push_unique_case_insensitive(&mut domains, &domain);
        }
    }
    domains
}

fn source_like_objects<'a>(payload: &'a Value) -> Vec<&'a serde_json::Map<String, Value>> {
    let mut out = Vec::<&serde_json::Map<String, Value>>::new();
    collect_source_like_objects(payload, &mut out, 0);
    out
}

fn collect_source_like_objects<'a>(
    value: &'a Value,
    out: &mut Vec<&'a serde_json::Map<String, Value>>,
    depth: usize,
) {
    if depth > 8 {
        return;
    }
    match value {
        Value::Array(rows) => {
            for row in rows {
                collect_source_like_objects(row, out, depth + 1);
            }
        }
        Value::Object(map) => {
            if object_looks_like_source_row(map) {
                out.push(map);
            }
            for child in map.values() {
                collect_source_like_objects(child, out, depth + 1);
            }
        }
        _ => {}
    }
}

fn object_looks_like_source_row(map: &serde_json::Map<String, Value>) -> bool {
    [
        "title",
        "source_domain",
        "source_class",
        "source_kind",
        "locator",
        "url",
        "source_url",
        "link",
        "snippet",
        "summary",
        "content",
        "markdown",
        "text",
        "claim_hints",
    ]
    .iter()
    .any(|key| map.get(*key).map(value_has_content).unwrap_or(false))
}

fn object_looks_like_evidence(map: &serde_json::Map<String, Value>) -> bool {
    [
        "claim_hints",
        "summary",
        "content",
        "markdown",
        "text",
        "snippet",
        "evidence_ref",
        "citation",
        "source_domain",
        "source_class",
    ]
    .iter()
    .any(|key| map.get(*key).map(value_has_content).unwrap_or(false))
}

fn source_class_value(map: &serde_json::Map<String, Value>) -> Option<String> {
    ["source_class", "source_kind", "class"]
        .iter()
        .find_map(|key| map.get(*key).and_then(Value::as_str))
        .map(|raw| clean_text(raw, 120))
        .filter(|raw| !raw.is_empty())
}

fn source_domain_value(map: &serde_json::Map<String, Value>) -> Option<String> {
    map.get("source_domain")
        .and_then(Value::as_str)
        .map(|raw| clean_text(raw, 160))
        .filter(|raw| !raw.is_empty())
        .or_else(|| {
            ["locator", "url", "source_url", "link"]
                .iter()
                .find_map(|key| map.get(*key).and_then(Value::as_str))
                .and_then(extract_domain_like_host)
        })
}

fn extract_domain_like_host(raw: &str) -> Option<String> {
    let cleaned = clean_text(raw, 240);
    if cleaned.is_empty() {
        return None;
    }
    let hostish = cleaned
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .trim_start_matches("www.")
        .split('/')
        .next()
        .unwrap_or("")
        .trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '.' && ch != '-');
    if hostish.is_empty() || !hostish.contains('.') {
        return None;
    }
    Some(hostish.to_ascii_lowercase())
}

fn push_unique_case_insensitive(values: &mut Vec<String>, candidate: &str) {
    let cleaned = clean_text(candidate, 160);
    if cleaned.is_empty() {
        return;
    }
    let normalized = cleaned.to_ascii_lowercase();
    if values
        .iter()
        .any(|existing| existing.to_ascii_lowercase() == normalized)
    {
        return;
    }
    values.push(cleaned);
}

fn top_count_row(counts: &BTreeMap<String, u64>) -> Value {
    let mut best_name = "none".to_string();
    let mut best_count = 0_u64;
    for (name, count) in counts {
        if *count > best_count {
            best_name = name.clone();
            best_count = *count;
        }
    }
    json!({
        "name": best_name,
        "count": best_count,
        "boundary": web_failure_boundary(&best_name)
    })
}

pub(super) fn web_failure_boundary(gate: &str) -> &'static str {
    match gate {
        "" | "none" => "no_web_tooling_failure_detected",
        "web_1_request_shape_present" => "web_request_shape_missing",
        "web_2_query_metadata_present" => "query_planning_metadata_missing",
        "web_3_tool_attempt_recorded" => "web_tool_attempt_missing",
        "web_3a_tool_transport_completed" => "tool_transport_failed",
        "web_3b1_provider_quota_not_rate_limited" => "provider_rate_limited_or_quota_exhausted",
        "web_3b2_no_bot_challenge_or_waf" => "anti_bot_challenge_or_waf",
        "web_3b3_no_permission_or_auth_block" => "permission_or_auth_block",
        "web_3b4_no_access_denied_or_forbidden" => "access_denied_or_forbidden",
        "web_3b5_provider_configuration_available" => "provider_configuration_missing",
        "web_3b_access_not_blocked_or_throttled" => "access_blocked_or_throttled",
        "web_3c_blocker_recovery_lane_visible" => "access_blocker_recovery_lane_missing",
        "web_3d_browser_materialization_not_failed" => "browser_materialization_failed",
        "web_4a_search_provider_configuration_usable" => "search_provider_configuration_unusable",
        "web_4b_search_provider_circuit_closed" => "search_provider_circuit_open",
        "web_4c_search_provider_surface_ready" => "search_provider_surface_degraded",
        "web_4d_provider_raw_rows_available" => "provider_raw_rows_absent",
        "web_4e_browser_serp_external_urls_extracted" => {
            "browser_serp_no_external_organic_urls"
        }
        "web_4e_provider_candidates_survive_filtering" => {
            "provider_rows_filtered_before_candidate_promotion"
        }
        "web_4_raw_candidates_present" => "provider_candidates_absent",
        "web_5_packaged_evidence_present" => "candidate_packaging_missing",
        "web_5b_content_rich_candidates_present" => "candidate_content_materialization_missing",
        "web_5c_claim_extraction_present" => "claim_extraction_missing",
        "web_5d_source_quality_ready" => "source_quality_not_ready",
        "web_5e_claim_quality_ready" => "claim_quality_not_ready",
        "web_5f_citation_renderability_ready" => "citation_renderability_not_ready",
        "web_5g_answerability_ready" => "answerability_not_ready",
        "web_5h_evidence_packet_contract_ready" => "evidence_packet_contract_not_ready",
        "web_5i_malformed_evidence_absent" => "malformed_evidence_fragments_present",
        "web_5j_citation_titles_clean" => "malformed_citation_titles_present",
        "web_6_provider_not_empty_or_degraded" => "provider_empty_or_degraded",
        "web_7_usable_evidence_available" => "retrieval_quality_not_usable",
        "web_8_evidence_context_to_synthesis" => "evidence_context_handoff_missing",
        _ => "unknown_web_tooling_failure",
    }
}

fn web_pending_request(payload: &Value) -> Option<&Value> {
    payload
        .get("pending_tool_request")
        .or_else(|| payload.pointer("/response_workflow/pending_tool_request"))
        .or_else(|| payload.pointer("/response_workflow/manual_toolbox_pending_tool_request"))
        .or_else(|| payload.pointer("/response_finalization/pending_tool_request"))
}

fn request_input_object(request: &Value) -> Option<&Value> {
    request
        .get("input")
        .or_else(|| request.get("request_payload"))
        .or_else(|| request.get("payload"))
}

fn input_has_query_or_locator(input: &Value) -> bool {
    [
        "query", "queries", "keyword", "keywords", "url", "urls", "locator", "locators", "source",
    ]
    .iter()
    .any(|key| value_has_content(input.get(*key).unwrap_or(&Value::Null)))
}

fn request_shape_refs(input: Option<&Value>) -> Vec<String> {
    let Some(input) = input.and_then(Value::as_object) else {
        return vec!["pending_tool_request.input".to_string()];
    };
    input
        .keys()
        .map(|key| format!("pending_tool_request.input.{key}"))
        .collect()
}

fn metadata_refs(query_metadata_diagnostics: &Value) -> Vec<String> {
    let fields = query_metadata_diagnostics
        .get("fields_present")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|field| format!("pending_tool_request.input.{field}"))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if fields.is_empty() {
        vec!["query_metadata_diagnostics.fields_present".to_string()]
    } else {
        fields
    }
}

fn checkpoint_passed(diagnostics: &Value, checkpoint: &str) -> bool {
    diagnostics
        .get("checkpoints")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter().any(|row| {
                row.get("checkpoint").and_then(Value::as_str) == Some(checkpoint)
                    && row.get("status").and_then(Value::as_str) == Some("pass")
            })
        })
        .unwrap_or(false)
}

fn has_tool_execution(payload: &Value) -> bool {
    payload
        .get("tools")
        .and_then(Value::as_array)
        .map(|rows| !rows.is_empty())
        .unwrap_or(false)
        || payload
            .pointer("/response_finalization/tool_completion/tool_attempts")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false)
}

fn value_has_content(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(raw) => *raw,
        Value::Number(_) => true,
        Value::String(raw) => !raw.trim().is_empty(),
        Value::Array(rows) => rows.iter().any(value_has_content),
        Value::Object(map) => map.values().any(value_has_content),
    }
}
