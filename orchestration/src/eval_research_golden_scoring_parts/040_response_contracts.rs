fn query_satisfaction(
    normalized_prompt: &str,
    normalized_response: &str,
    coverage_entities: &[String],
    entity_coverage: f64,
    final_answer_present: bool,
    source_signal: bool,
    citation_signal: bool,
    limitation_signal: bool,
) -> Value {
    let scope_covered = coverage_entities.is_empty() || entity_coverage >= 0.75;
    let coverage_gap_prevents_answer =
        response_explicitly_cannot_answer_goal_from_current_evidence(normalized_response);
    let intent_answered = response_matches_prompt_intent(normalized_prompt, normalized_response)
        && !coverage_gap_prevents_answer;
    let decision_value = (has_recommendation_signal(normalized_response)
        || response_matches_decision_prompt(normalized_prompt, normalized_response))
        && !coverage_gap_prevents_answer;
    let right_granularity = response_has_right_granularity(normalized_response);
    let evidence_aware = source_signal || citation_signal || limitation_signal;
    let score = [
        (final_answer_present, 2_u64),
        (intent_answered, 2),
        (scope_covered, 2),
        (evidence_aware, 2),
        (decision_value, 1),
        (right_granularity, 1),
    ]
    .iter()
    .filter_map(|(ok, points)| ok.then_some(*points))
    .sum::<u64>();
    json!({
        "schema_version": 1,
        "score": score,
        "max_score": 10,
        "intent_answered": intent_answered,
        "scope_covered": scope_covered,
        "user_stated_coverage_entities": coverage_entities,
        "entity_coverage": entity_coverage,
        "evidence_aware": evidence_aware,
        "decision_value": decision_value,
        "right_granularity": right_granularity,
        "coverage_gap_prevents_answer": coverage_gap_prevents_answer,
        "coverage_entity_aliases": coverage_entity_aliases(coverage_entities),
        "note": "Query satisfaction is derived from the original prompt plus available evidence behavior, not from hidden expected answers."
    })
}

fn generic_response_contract(
    response_text: &str,
    final_answer_present: bool,
    query_satisfaction: &Value,
    source_summary_without_answer: bool,
    raw_tool_leak: bool,
    internal_leak: bool,
    tool_choice_final_response: bool,
    truncated_or_incomplete_response: bool,
) -> Value {
    let intent_answered = query_satisfaction
        .get("intent_answered")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let clean_projection = !raw_tool_leak && !internal_leak && !tool_choice_final_response;
    let human_readable = normal_prose_signal(response_text) && !truncated_or_incomplete_response;
    let mut subgates = serde_json::Map::new();
    subgates.insert(
        "generic_1_final_answer_present".to_string(),
        json!(final_answer_present),
    );
    subgates.insert(
        "generic_2_answers_user_goal".to_string(),
        json!(intent_answered),
    );
    subgates.insert(
        "generic_3_no_source_summary_without_answer".to_string(),
        json!(!source_summary_without_answer),
    );
    subgates.insert(
        "generic_4_projection_clean".to_string(),
        json!(clean_projection),
    );
    subgates.insert(
        "generic_5_human_readable_shape".to_string(),
        json!(human_readable),
    );
    subgates.insert(
        "generic_6_complete_response_shape".to_string(),
        json!(!truncated_or_incomplete_response),
    );
    let ordered = [
        ("generic_1_final_answer_present", "missing_final_answer"),
        ("generic_2_answers_user_goal", "user_goal_not_answered"),
        (
            "generic_3_no_source_summary_without_answer",
            "source_summary_without_user_answer",
        ),
        (
            "generic_4_projection_clean",
            "projection_contains_internal_or_tool_state",
        ),
        (
            "generic_5_human_readable_shape",
            "response_shape_not_human_readable",
        ),
        (
            "generic_6_complete_response_shape",
            "truncated_or_incomplete_response",
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
    let score = [
        final_answer_present,
        intent_answered,
        !source_summary_without_answer,
        clean_projection,
        human_readable,
        !truncated_or_incomplete_response,
    ]
    .iter()
    .filter(|ok| **ok)
    .count() as u64
        * 4;
    json!({
        "schema_version": 1,
        "layer_id": "generic_response_contract_v1",
        "pass": blockers.is_empty(),
        "score": score,
        "max_score": 24,
        "subgates": Value::Object(subgates),
        "blockers": blockers,
        "top_blocker": blockers.first().cloned().unwrap_or_else(|| "none".to_string()),
        "note": "Generic response grading checks that the answer is actually user-facing, goal-directed, and readable without depending on a fixed visible format."
    })
}

fn tool_backed_evidence_contract(
    normalized_response: &str,
    retrieval_quality: &Value,
    citation_behavior: &Value,
    limitation_signal: bool,
    query_satisfaction: &Value,
    unsupported_claim: bool,
    outside_evidence_used_for_decision: bool,
) -> Value {
    let tool_executed = retrieval_quality
        .get("tool_executed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let usable_evidence = retrieval_quality
        .get("usable_evidence")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let retrieval_status = str_at(retrieval_quality, &["status"], "unknown");
    let evidence_count = citation_behavior
        .get("evidence_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let citation_signal = citation_behavior
        .get("citation_signal")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let response_source_signal = citation_behavior
        .get("response_source_signal")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let synthesis_ignored_citable_evidence = citation_behavior
        .get("synthesis_ignored_citable_evidence")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let scope_covered = query_satisfaction
        .get("scope_covered")
        .and_then(Value::as_bool)
        .unwrap_or(true);
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
    let denies_recorded_evidence =
        response_denies_recorded_evidence(normalized_response, evidence_count);
    let uses_recorded_evidence_when_present =
        !tool_executed || evidence_count == 0 || response_source_signal || citation_signal;
    let preserves_source_signal_when_citable =
        !usable_evidence || evidence_count == 0 || citation_signal;
    let names_limits_when_needed = !needs_gap_statement || limitation_signal;
    let mut subgates = serde_json::Map::new();
    subgates.insert(
        "evidence_1_uses_recorded_evidence_when_present".to_string(),
        json!(uses_recorded_evidence_when_present),
    );
    subgates.insert(
        "evidence_2_preserves_compact_source_signal_when_citable".to_string(),
        json!(preserves_source_signal_when_citable),
    );
    subgates.insert(
        "evidence_3_does_not_ignore_citable_evidence".to_string(),
        json!(!synthesis_ignored_citable_evidence),
    );
    subgates.insert(
        "evidence_4_does_not_overclaim_or_deny_recorded_state".to_string(),
        json!(!unsupported_claim && !denies_recorded_evidence),
    );
    subgates.insert(
        "evidence_5_names_limits_when_needed".to_string(),
        json!(names_limits_when_needed),
    );
    subgates.insert(
        "evidence_6_respects_source_boundary".to_string(),
        json!(!outside_evidence_used_for_decision),
    );
    let ordered = [
        (
            "evidence_1_uses_recorded_evidence_when_present",
            "recorded_evidence_not_used",
        ),
        (
            "evidence_2_preserves_compact_source_signal_when_citable",
            "missing_compact_source_signal",
        ),
        (
            "evidence_3_does_not_ignore_citable_evidence",
            "citable_evidence_ignored",
        ),
        (
            "evidence_4_does_not_overclaim_or_deny_recorded_state",
            "recorded_state_overclaimed_or_denied",
        ),
        (
            "evidence_5_names_limits_when_needed",
            "missing_evidence_gap_statement",
        ),
        (
            "evidence_6_respects_source_boundary",
            "outside_evidence_used_for_decision",
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
    let score = [
        uses_recorded_evidence_when_present,
        preserves_source_signal_when_citable,
        !synthesis_ignored_citable_evidence,
        !unsupported_claim && !denies_recorded_evidence,
        names_limits_when_needed,
        !outside_evidence_used_for_decision,
    ]
    .iter()
    .filter(|ok| **ok)
    .count() as u64
        * 5;
    let top_blocker = blockers
        .first()
        .cloned()
        .unwrap_or_else(|| "none".to_string());
    json!({
        "schema_version": 1,
        "layer_id": "tool_backed_evidence_contract_v1",
        "pass": blockers.is_empty(),
        "score": score,
        "max_score": 30,
        "subgates": Value::Object(subgates),
        "blockers": blockers,
        "top_blocker": top_blocker,
        "retrieval_status": retrieval_status,
        "outside_evidence_used_for_decision": outside_evidence_used_for_decision,
        "note": "Evidence-use grading is format-flexible but requires the final answer to use recorded evidence honestly when evidence exists and to keep outside-evidence inference from carrying concrete recommendations."
    })
}

fn research_workflow_specific_rubric(
    query_satisfaction: &Value,
    source_signal: bool,
    limitation_signal: bool,
    normalized_response: &str,
) -> Value {
    let query_satisfaction_score = query_satisfaction
        .get("score")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let scope_covered = query_satisfaction
        .get("scope_covered")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let decision_value = query_satisfaction
        .get("decision_value")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let right_granularity = query_satisfaction
        .get("right_granularity")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let research_structure =
        has_tradeoff_or_structure(normalized_response) || source_signal || limitation_signal;
    let mut subgates = serde_json::Map::new();
    subgates.insert(
        "rubric_1_query_satisfaction".to_string(),
        json!(query_satisfaction_score >= 7),
    );
    subgates.insert("rubric_2_scope_covered".to_string(), json!(scope_covered));
    subgates.insert(
        "rubric_3_decision_or_explanatory_value".to_string(),
        json!(decision_value || has_tradeoff_or_structure(normalized_response)),
    );
    subgates.insert(
        "rubric_4_right_granularity".to_string(),
        json!(right_granularity),
    );
    subgates.insert(
        "rubric_5_research_structure_or_grounding".to_string(),
        json!(research_structure),
    );
    let ordered = [
        (
            "rubric_1_query_satisfaction",
            "query_satisfaction_below_rubric",
        ),
        ("rubric_2_scope_covered", "requested_scope_not_covered"),
        (
            "rubric_3_decision_or_explanatory_value",
            "missing_decision_or_explanatory_value",
        ),
        ("rubric_4_right_granularity", "response_granularity_off"),
        (
            "rubric_5_research_structure_or_grounding",
            "missing_research_structure_or_grounding",
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
    let score = (query_satisfaction_score.min(10) * 2)
        + (if scope_covered { 5 } else { 0 })
        + (if decision_value || has_tradeoff_or_structure(normalized_response) {
            4
        } else {
            0
        })
        + (if right_granularity { 3 } else { 0 })
        + (if research_structure { 3 } else { 0 });
    let normalized_score = score.min(35);
    let top_blocker = blockers
        .first()
        .cloned()
        .unwrap_or_else(|| "none".to_string());
    json!({
        "schema_version": 1,
        "layer_id": "research_workflow_specific_rubric_v1",
        "pass": blockers.is_empty(),
        "score": normalized_score,
        "max_score": 35,
        "subgates": Value::Object(subgates),
        "blockers": blockers,
        "top_blocker": top_blocker,
        "note": "This layer is intentionally workflow-specific. It captures research-answer usefulness without requiring any fixed visible format."
    })
}

