struct ExcellentDiagnosticInput<'a> {
    retrieval_quality: &'a Value,
    citation_behavior: &'a Value,
    query_satisfaction: &'a Value,
    answer_unit_evidence_alignment: &'a Value,
    answer_unit_usefulness: &'a Value,
    normalized_response: &'a str,
    source_signal: bool,
    final_answer_present: bool,
    limitation_signal: bool,
    raw_tool_leak: bool,
    internal_leak: bool,
    unsupported_claim: bool,
    score: u64,
    excellent_score: u64,
    failures: &'a [String],
}

fn excellent_diagnostics(input: ExcellentDiagnosticInput<'_>) -> Value {
    let retrieval_status = str_at(input.retrieval_quality, &["status"], "unknown");
    let citable_evidence_available = input
        .retrieval_quality
        .get("allows_excellent")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let citation_signal = input
        .citation_behavior
        .get("citation_signal")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let query_satisfaction_score = input
        .query_satisfaction
        .get("score")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let decision_value = input
        .query_satisfaction
        .get("decision_value")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let scope_covered = input
        .query_satisfaction
        .get("scope_covered")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let needs_gap_statement = !scope_covered
        || matches!(
            retrieval_status.as_str(),
            "low_signal"
                | "no_results"
                | "no_evidence"
                | "provider_degraded"
                | "raw_provider_absent"
                | "conflicting_provider_state"
                | "low_relevance"
        );
    let evidence_gaps_named_when_needed = !needs_gap_statement || input.limitation_signal;
    let limitation_heavy_answer = limitation_heavy_for_excellent(input.normalized_response);
    let answer_units_trace_to_evidence = !input
        .answer_unit_evidence_alignment
        .get("evaluated")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || input
            .answer_unit_evidence_alignment
            .get("pass")
            .and_then(Value::as_bool)
            .unwrap_or(true);
    let answer_units_useful_for_prompt = !input
        .answer_unit_usefulness
        .get("evaluated")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || input
            .answer_unit_usefulness
            .get("pass")
            .and_then(Value::as_bool)
            .unwrap_or(true);
    let mut subgates = serde_json::Map::new();
    subgates.insert(
        "excellent_1_query_satisfaction".to_string(),
        json!(query_satisfaction_score >= 9),
    );
    subgates.insert(
        "excellent_2_citable_evidence_available".to_string(),
        json!(citable_evidence_available),
    );
    subgates.insert(
        "excellent_3_citations_used_in_final".to_string(),
        json!(!citable_evidence_available || citation_signal),
    );
    subgates.insert(
        "excellent_4_claims_trace_to_citations".to_string(),
        json!(
            !citable_evidence_available
                || (citation_signal && input.source_signal && !input.unsupported_claim)
        ),
    );
    subgates.insert(
        "excellent_5_evidence_gaps_named_when_needed".to_string(),
        json!(evidence_gaps_named_when_needed),
    );
    subgates.insert(
        "excellent_6_decision_value_present".to_string(),
        json!(decision_value),
    );
    subgates.insert(
        "excellent_7_projection_clean".to_string(),
        json!(input.final_answer_present && !input.raw_tool_leak && !input.internal_leak),
    );
    subgates.insert(
        "excellent_8_score_threshold".to_string(),
        json!(input.score >= input.excellent_score),
    );
    subgates.insert(
        "excellent_9_no_pass_failures".to_string(),
        json!(input.failures.is_empty()),
    );
    subgates.insert(
        "excellent_10_answer_not_limitation_heavy".to_string(),
        json!(!limitation_heavy_answer),
    );
    subgates.insert(
        "excellent_11_answer_units_trace_to_evidence".to_string(),
        json!(answer_units_trace_to_evidence),
    );
    subgates.insert(
        "excellent_15_answer_units_useful_for_prompt".to_string(),
        json!(answer_units_useful_for_prompt),
    );

    let ordered = [
        (
            "excellent_2_citable_evidence_available",
            "retrieval_quality_not_excellent_ready",
        ),
        (
            "excellent_3_citations_used_in_final",
            "missing_final_citation_or_source_signal",
        ),
        (
            "excellent_4_claims_trace_to_citations",
            "claims_not_traceable_to_citation_signal",
        ),
        (
            "excellent_11_answer_units_trace_to_evidence",
            "answer_units_not_traceable_to_evidence",
        ),
        (
            "excellent_15_answer_units_useful_for_prompt",
            "answer_units_not_useful_for_prompt",
        ),
        (
            "excellent_1_query_satisfaction",
            "query_satisfaction_below_excellent",
        ),
        (
            "excellent_5_evidence_gaps_named_when_needed",
            "missing_evidence_gap_statement",
        ),
        (
            "excellent_6_decision_value_present",
            "missing_decision_value",
        ),
        ("excellent_7_projection_clean", "projection_not_clean"),
        ("excellent_8_score_threshold", "score_below_excellent"),
        ("excellent_9_no_pass_failures", "pass_failures_present"),
        (
            "excellent_10_answer_not_limitation_heavy",
            "limitation_heavy_answer_shape",
        ),
    ];
    let blockers = ordered
        .iter()
        .filter_map(|(gate, blocker)| {
            (!subgates
                .get(*gate)
                .and_then(Value::as_bool)
                .unwrap_or(false))
            .then(|| (*blocker).to_string())
        })
        .collect::<Vec<_>>();
    let top_blocker = blockers
        .first()
        .cloned()
        .unwrap_or_else(|| "none".to_string());
    json!({
        "schema_version": 1,
        "subgates": Value::Object(subgates),
        "blockers": blockers,
        "top_blocker": top_blocker,
        "retrieval_status": retrieval_status,
        "limitation_heavy_answer": limitation_heavy_answer,
        "score": input.score,
        "excellent_score": input.excellent_score,
        "note": "Excellent is diagnosed through generic quality properties, not hidden expected facts or a required visible format."
    })
}

fn provider_candidate_count(payload: &Value) -> u64 {
    tool_rows(payload)
        .iter()
        .map(|row| {
            let explicit = [
                "provider_raw_count",
                "provider_filtered_count",
                "candidate_count",
                "raw_count",
                "evidence_pack_candidate_count",
                "materialized_candidate_count",
            ]
            .iter()
            .filter_map(|key| row.get(*key).and_then(Value::as_u64))
            .max()
            .unwrap_or(0);
            let inferred = [
                "raw",
                "raw_result",
                "raw_results",
                "provider_result",
                "provider_results",
                "search_results",
                "organic_results",
                "web_results",
                "evidence_pack",
                "evidence_pack_candidates",
            ]
            .iter()
            .map(|key| count_content_items(row.get(*key).unwrap_or(&Value::Null)))
            .sum::<u64>();
            explicit.max(inferred)
        })
        .sum()
}

fn provider_evidence_count(payload: &Value) -> u64 {
    let top_level = [
        "/evidence",
        "/evidence_refs",
        "/evidence_pack",
        "/evidence_pack_candidates",
        "/sources",
        "/citations",
        "/response_workflow/evidence",
        "/response_workflow/evidence_refs",
        "/response_workflow/evidence_pack",
        "/response_workflow/evidence_pack_candidates",
        "/response_workflow/sources",
        "/response_workflow/citations",
        "/response_finalization/evidence",
        "/response_finalization/evidence_refs",
        "/response_finalization/evidence_pack",
        "/response_finalization/evidence_pack_candidates",
        "/response_finalization/tool_completion/evidence_refs",
        "/response_finalization/tool_completion/evidence_pack",
        "/response_finalization/tool_completion/evidence_pack_candidates",
        "/response_finalization/tool_completion/findings",
    ]
    .iter()
    .map(|pointer| count_content_items(payload.pointer(pointer).unwrap_or(&Value::Null)))
    .sum::<u64>();
    top_level
        + tool_rows(payload)
            .iter()
            .map(|row| {
                [
                    "evidence",
                    "evidence_refs",
                    "evidence_pack",
                    "evidence_pack_candidates",
                    "sources",
                    "citations",
                    "findings",
                ]
                .iter()
                .map(|key| count_content_items(row.get(*key).unwrap_or(&Value::Null)))
                .sum::<u64>()
            })
            .sum::<u64>()
}

fn provider_content_rich_candidate_count(payload: &Value) -> u64 {
    let explicit_materialized =
        provider_explicit_quality_metric(payload, &["materialized_candidate_count"]);
    let explicit = if explicit_materialized > 0 {
        explicit_materialized
    } else {
        provider_explicit_quality_metric(
            payload,
            &["content_rich_candidate_count", "content_rich_item_count"],
        )
    };
    let inferred = selected_tool_contexts(payload)
        .iter()
        .map(|row| count_content_rich_items(row, 0))
        .sum::<u64>();
    explicit.max(inferred)
}

fn provider_materialized_candidate_count(payload: &Value) -> u64 {
    let explicit = provider_explicit_quality_metric(payload, &["materialized_candidate_count"]);
    let inferred = selected_tool_contexts(payload)
        .iter()
        .map(|row| count_materialized_items(row, 0))
        .sum::<u64>();
    explicit.max(inferred)
}

fn provider_claim_hint_count(payload: &Value) -> u64 {
    let explicit = provider_explicit_quality_metric(
        payload,
        &[
            "claim_hint_count",
            "claim_hints_count",
            "claim_extraction_count",
            "extracted_claim_count",
        ],
    );
    let inferred = selected_tool_contexts(payload)
        .iter()
        .map(|row| count_claim_hint_items(row, 0))
        .sum::<u64>();
    explicit.max(inferred)
}

fn selected_tool_contexts(payload: &Value) -> Vec<&Value> {
    let mut rows = tool_rows(payload);
    for pointer in [
        "/tool_result_quality",
        "/evidence_pack_quality",
        "/evidence_pack",
        "/evidence_pack_candidates",
        "/evidence_refs",
        "/response_workflow/evidence_pack",
        "/response_workflow/evidence_pack_candidates",
        "/response_finalization/tool_completion/evidence_pack",
        "/response_finalization/tool_completion/evidence_pack_candidates",
    ] {
        if let Some(value) = payload.pointer(pointer) {
            rows.push(value);
        }
    }
    rows
}

fn provider_explicit_quality_metric(payload: &Value, metric_keys: &[&str]) -> u64 {
    selected_tool_contexts(payload)
        .iter()
        .map(|row| explicit_quality_metric(row, metric_keys, 0))
        .max()
        .unwrap_or(0)
}

fn provider_explicit_quality_value(payload: &Value, value_keys: &[&str]) -> Value {
    selected_tool_contexts(payload)
        .iter()
        .find_map(|row| explicit_quality_value(row, value_keys, 0))
        .unwrap_or(Value::Null)
}

fn explicit_quality_metric(value: &Value, metric_keys: &[&str], depth: usize) -> u64 {
    if depth > 7 {
        return 0;
    }
    match value {
        Value::Object(map) => {
            let direct = metric_keys
                .iter()
                .filter_map(|key| map.get(*key).and_then(Value::as_u64))
                .max()
                .unwrap_or(0);
            direct.max(
                map.values()
                    .map(|row| explicit_quality_metric(row, metric_keys, depth + 1))
                    .max()
                    .unwrap_or(0),
            )
        }
        Value::Array(rows) => rows
            .iter()
            .map(|row| explicit_quality_metric(row, metric_keys, depth + 1))
            .max()
            .unwrap_or(0),
        _ => 0,
    }
}

fn explicit_quality_value(value: &Value, value_keys: &[&str], depth: usize) -> Option<Value> {
    if depth > 7 {
        return None;
    }
    match value {
        Value::Object(map) => {
            for key in value_keys {
                if let Some(found) = map.get(*key) {
                    return Some(found.clone());
                }
            }
            map.values()
                .find_map(|row| explicit_quality_value(row, value_keys, depth + 1))
        }
        Value::Array(rows) => rows
            .iter()
            .find_map(|row| explicit_quality_value(row, value_keys, depth + 1)),
        _ => None,
    }
}

fn count_content_rich_items(value: &Value, depth: usize) -> u64 {
    if depth > 7 {
        return 0;
    }
    match value {
        Value::String(raw) => u64::from(content_rich_text(raw)),
        Value::Array(rows) => rows
            .iter()
            .map(|row| count_content_rich_items(row, depth + 1))
            .sum(),
        Value::Object(map) => {
            if let Some(false) = value_counts_as_usable_evidence(value) {
                return 0;
            }
            let direct = [
                "snippet",
                "summary",
                "content",
                "markdown",
                "text",
                "body",
                "description",
                "abstract",
                "content_preview",
                "snippet_preview",
                "result",
            ]
            .iter()
            .any(|key| {
                map.get(*key)
                    .and_then(Value::as_str)
                    .map(content_rich_text)
                    .unwrap_or(false)
            });
            if direct {
                1
            } else {
                semantic_child_values(map)
                    .map(|row| count_content_rich_items(row, depth + 1))
                    .sum()
            }
        }
        _ => 0,
    }
}

fn count_claim_hint_items(value: &Value, depth: usize) -> u64 {
    if depth > 7 {
        return 0;
    }
    match value {
        Value::Array(rows) => rows
            .iter()
            .map(|row| count_claim_hint_items(row, depth + 1))
            .sum(),
        Value::Object(map) => {
            if let Some(false) = value_counts_as_usable_evidence(value) {
                return 0;
            }
            let direct = [
                "claim_hints",
                "claims",
                "extracted_claims",
                "claim_candidates",
                "key_findings",
            ]
            .iter()
            .map(|key| count_content_items(map.get(*key).unwrap_or(&Value::Null)))
            .sum::<u64>();
            direct
                + semantic_child_values(map)
                    .map(|row| count_claim_hint_items(row, depth + 1))
                    .sum::<u64>()
        }
        _ => 0,
    }
}

fn count_materialized_items(value: &Value, depth: usize) -> u64 {
    if depth > 7 {
        return 0;
    }
    match value {
        Value::Array(rows) => rows
            .iter()
            .map(|row| count_materialized_items(row, depth + 1))
            .sum(),
        Value::Object(map) => {
            let direct = value_counts_as_usable_evidence(value)
                .filter(|eligible| *eligible)
                .map(|_| 1)
                .unwrap_or(0);
            if direct > 0 {
                direct
            } else {
                semantic_child_values(map)
                    .map(|row| count_materialized_items(row, depth + 1))
                    .sum()
            }
        }
        _ => 0,
    }
}

fn value_counts_as_usable_evidence(value: &Value) -> Option<bool> {
    let map = value.as_object()?;
    if let Some(explicit) = map
        .get("counts_as_usable_evidence")
        .and_then(Value::as_bool)
    {
        return Some(explicit);
    }
    let quality = map
        .get("materialization_quality")
        .and_then(Value::as_str)
        .map(normalize_for_compare)
        .or_else(|| {
            let source_kind = map.get("source_kind").and_then(Value::as_str).unwrap_or("");
            let permissions = map.get("permissions").and_then(Value::as_str).unwrap_or("");
            let snippet = map
                .get("snippet")
                .or_else(|| map.get("summary"))
                .or_else(|| map.get("content"))
                .or_else(|| map.get("markdown"))
                .or_else(|| map.get("text"))
                .and_then(Value::as_str)
                .unwrap_or("");
            infer_materialization_quality(source_kind, permissions, snippet)
        })?;
    Some(matches!(
        quality.as_str(),
        "full_materialized" | "partial_materialized" | "trusted_structured_feed"
    ))
}
