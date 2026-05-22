pub(super) fn response_diagnostics(payload: &Value, response_text: &str) -> Value {
    json!({
        "top_keys": payload
            .as_object()
            .map(|obj| obj.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default(),
        "pending_tool_request": pending_tool_request(payload).cloned().unwrap_or(Value::Null),
        "tools_present": has_tool_execution(payload),
        "provider": payload.get("provider").and_then(Value::as_str),
        "model": payload.get("model").and_then(Value::as_str),
        "runtime_model": payload.get("runtime_model").and_then(Value::as_str),
        "initial_invoke_error": payload.get("initial_invoke_error").and_then(Value::as_bool),
        "error": payload
            .get("error")
            .and_then(Value::as_str)
            .map(sanitize_backend_error),
        "transport_error": payload.get("transport_error").and_then(Value::as_str),
        "stderr": payload
            .get("stderr")
            .and_then(Value::as_str)
            .map(|raw| clean_text(raw, 500)),
        "response_empty": response_text.trim().is_empty(),
        "final_llm_status": payload
            .pointer("/response_workflow/final_llm_response/status")
            .and_then(Value::as_str),
        "evidence_outcome_posture": payload
            .pointer("/response_workflow/final_llm_response/evidence_outcome_posture")
            .or_else(|| payload.pointer("/response_finalization/final_llm_response/evidence_outcome_posture"))
            .and_then(Value::as_str),
    })
}

fn sanitize_backend_error(raw: &str) -> String {
    let mut cleaned = clean_text(raw, 800);
    let lower = cleaned.to_ascii_lowercase();
    let marker = "incorrect api key provided:";
    if let Some(idx) = lower.find(marker) {
        let secret_start = idx + marker.len();
        let secret_end = cleaned[secret_start..]
            .find('.')
            .map(|offset| secret_start + offset)
            .unwrap_or_else(|| cleaned.len());
        cleaned.replace_range(secret_start..secret_end, " [redacted]");
    }
    cleaned
}

pub(super) fn gate_rate_rows(
    total_counts: &BTreeMap<String, u64>,
    pass_counts: &BTreeMap<String, u64>,
    min_rate: f64,
) -> Vec<Value> {
    total_counts
        .iter()
        .map(|(gate, total)| {
            let passed = *pass_counts.get(gate).unwrap_or(&0);
            let rate = ratio(passed, *total);
            json!({
                "gate": gate,
                "passed": passed,
                "total": total,
                "pass_rate": rate,
                "min_rate": min_rate,
                "ok": rate >= min_rate
            })
        })
        .collect()
}

pub(super) fn dimension_average_rows(
    totals: &BTreeMap<String, u64>,
    total_cases: u64,
) -> Vec<Value> {
    totals
        .iter()
        .map(|(dimension, total)| {
            json!({
                "dimension": dimension,
                "average": ratio(*total, total_cases)
            })
        })
        .collect()
}

fn gate_results(case: &Value, payload: &Value) -> BTreeMap<String, bool> {
    let mut gates = BTreeMap::new();
    let serialized = payload.to_string().to_ascii_lowercase();
    let tool_request = pending_tool_request(payload);
    let synthesis_only_without_new_candidate =
        case_allows_existing_tool_state_without_new_candidate(case);
    let expected_gate_2 =
        normalize_for_compare(&str_at(case, &["expected_gate_path", "gate_2"], ""));
    let expected_gate_3 =
        normalize_for_compare(&str_at(case, &["expected_gate_path", "gate_3"], ""));
    let required_gate_4_fields =
        string_array_at(case, &["expected_gate_path", "gate_4_required_fields"]);
    let gate_2 = expected_gate_2.is_empty()
        || tool_request
            .map(|request| {
                let family = normalize_for_compare(&format!(
                    "{} {}",
                    str_at(request, &["selected_tool_family"], ""),
                    str_at(request, &["selected_tool_label"], "")
                ));
                (family.contains("web") || family.contains("research"))
                    && (family.contains("search") || family.contains("fetch"))
            })
            .unwrap_or_else(|| {
                (serialized.contains("web") || serialized.contains("research"))
                    && (serialized.contains("search") || serialized.contains("fetch"))
            });
    let gate_3 = expected_gate_3.is_empty()
        || tool_request
            .map(|request| {
                gate_3_tool_matches(
                    &format!(
                        "{} {} {}",
                        str_at(request, &["tool_name"], ""),
                        str_at(request, &["tool_key"], ""),
                        str_at(request, &["selected_tool_key"], "")
                    ),
                    &expected_gate_3,
                )
            })
            .unwrap_or_else(|| gate_3_tool_matches(&serialized, &expected_gate_3))
        || (synthesis_only_without_new_candidate && gate_2);
    let gate_4 = required_gate_4_fields.iter().all(|field| {
        let field = normalize_for_compare(field);
        tool_request
            .and_then(|request| {
                request
                    .get("input")
                    .or_else(|| request.get("request_payload"))
                    .or_else(|| request.get("payload"))
            })
            .and_then(Value::as_object)
            .map(|input| input.keys().any(|key| normalize_for_compare(key) == field))
            .unwrap_or_else(|| serialized.contains(&format!("\"{field}\"")))
    });
    let gate_1 = has_pending_tool(payload)
        || has_tool_execution(payload)
        || gate_2
        || gate_3
        || gate_4
        || serialized.contains("tool_required")
        || serialized.contains("answered_yes")
        || serialized.contains("should_call_tools\":true");
    gates.insert("gate_1_tool_need".to_string(), gate_1);
    gates.insert("gate_2_tool_family".to_string(), gate_2);
    gates.insert("gate_3_tool_key".to_string(), gate_3);
    gates.insert("gate_4_request_template".to_string(), gate_4);
    gates
}

fn has_pending_tool(payload: &Value) -> bool {
    [
        "/pending_tool_request/status",
        "/response_workflow/pending_tool_request/status",
        "/response_workflow/manual_toolbox_pending_tool_request/status",
        "/response_finalization/pending_tool_request/status",
    ]
    .iter()
    .any(|pointer| payload.pointer(pointer).and_then(Value::as_str) == Some("pending_confirmation"))
}

fn pending_tool_request(payload: &Value) -> Option<&Value> {
    payload
        .get("pending_tool_request")
        .or_else(|| payload.pointer("/response_workflow/pending_tool_request"))
        .or_else(|| payload.pointer("/response_workflow/manual_toolbox_pending_tool_request"))
        .or_else(|| payload.pointer("/response_finalization/pending_tool_request"))
}

fn case_allows_existing_tool_state_without_new_candidate(case: &Value) -> bool {
    let gate_1 = normalize_for_compare(&str_at(case, &["expected_gate_path", "gate_1"], ""));
    let post_tool = normalize_for_compare(&str_at(case, &["expected_gate_path", "post_tool"], ""));
    gate_1.contains("pending_tool_result") || post_tool.starts_with("must_synthesize_from")
}

fn gate_3_tool_matches(actual_raw: &str, expected_raw: &str) -> bool {
    let actual = normalize_for_compare(actual_raw);
    let expected = normalize_for_compare(expected_raw);
    if expected.is_empty() {
        return true;
    }
    if actual.contains(&expected) {
        return true;
    }
    matches!(
        expected.as_str(),
        "web_search" | "batch_query" | "batch query"
    ) && (actual.contains("web_search")
        || actual.contains("batch_query")
        || actual.contains("batch query"))
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

fn has_source_signal(response_text: &str, retrieval_quality: &Value) -> bool {
    if retrieval_quality
        .get("usable_evidence")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return true;
    }
    let normalized = normalize_for_compare(response_text);
    [
        "source",
        "evidence",
        "according",
        "docs",
        "release",
        "changelog",
        "citation",
        "http://",
        "https://",
    ]
    .iter()
    .any(|needle| normalized.contains(*needle))
}

fn citation_behavior(payload: &Value, response_text: &str, retrieval_quality: &Value) -> Value {
    let citation_count = response_citation_count(payload);
    let evidence_count = retrieval_quality
        .get("evidence_count")
        .and_then(Value::as_u64)
        .unwrap_or_else(|| provider_evidence_count(payload));
    let usable_evidence = retrieval_quality
        .get("usable_evidence")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let response_source_signal = response_has_inline_citation_signal(response_text);
    let citation_signal = citation_count > 0 || response_source_signal;
    let synthesis_ignored_citable_evidence =
        usable_evidence && evidence_count > 0 && !citation_signal;
    json!({
        "schema_version": 1,
        "citation_count": citation_count,
        "evidence_count": evidence_count,
        "usable_evidence": usable_evidence,
        "response_source_signal": response_source_signal,
        "citation_signal": citation_signal,
        "synthesis_ignored_citable_evidence": synthesis_ignored_citable_evidence,
        "note": "Measures whether the final artifact/prose exposes compact citation or source-reference signal separately from whether retrieval found evidence."
    })
}

fn response_citation_count(payload: &Value) -> u64 {
    [
        "/citations",
        "/sources",
        "/source_refs",
        "/response_workflow/citations",
        "/response_workflow/sources",
        "/response_workflow/source_refs",
        "/response_workflow/final_llm_response/citations",
        "/response_workflow/final_llm_response/sources",
        "/response_workflow/final_llm_response/source_refs",
        "/response_finalization/citations",
        "/response_finalization/sources",
        "/response_finalization/source_refs",
        "/response_finalization/final_response/citations",
        "/response_finalization/final_response/sources",
        "/response_finalization/final_response/source_refs",
        "/response_finalization/final_llm_response/citations",
        "/response_finalization/final_llm_response/sources",
        "/response_finalization/final_llm_response/source_refs",
        "/response_finalization/tool_completion/citations",
        "/response_finalization/tool_completion/source_refs",
    ]
    .iter()
    .map(|pointer| count_content_items(payload.pointer(pointer).unwrap_or(&Value::Null)))
    .sum::<u64>()
}

const CITATION_ARTIFACT_POINTERS: &[(&str, &str)] = &[
    ("/citations", "citations"),
    ("/sources", "sources"),
    ("/source_refs", "source_refs"),
    ("/evidence", "evidence"),
    ("/evidence_refs", "evidence_refs"),
    ("/evidence_pack", "evidence_pack"),
    (
        "/response_workflow/final_llm_response/citations",
        "final_llm_response.citations",
    ),
    (
        "/response_workflow/final_llm_response/source_refs",
        "final_llm_response.source_refs",
    ),
    (
        "/response_finalization/citations",
        "response_finalization.citations",
    ),
    (
        "/response_finalization/source_refs",
        "response_finalization.source_refs",
    ),
    (
        "/response_finalization/final_response/citations",
        "final_response.citations",
    ),
    (
        "/response_finalization/final_response/source_refs",
        "final_response.source_refs",
    ),
    (
        "/response_finalization/final_llm_response/citations",
        "final_llm_response.citations",
    ),
    (
        "/response_finalization/final_llm_response/source_refs",
        "final_llm_response.source_refs",
    ),
    (
        "/response_finalization/tool_completion/citations",
        "tool_completion.citations",
    ),
    (
        "/response_finalization/tool_completion/source_refs",
        "tool_completion.source_refs",
    ),
    (
        "/response_finalization/tool_completion/evidence_refs",
        "tool_completion.evidence_refs",
    ),
    (
        "/response_finalization/tool_completion/evidence_pack",
        "tool_completion.evidence_pack",
    ),
    (
        "/response_finalization/tool_completion/evidence_pack_candidates",
        "tool_completion.evidence_pack_candidates",
    ),
];

