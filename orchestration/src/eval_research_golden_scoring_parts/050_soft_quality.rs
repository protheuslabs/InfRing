// Layer ownership: orchestration (research eval authority)

fn soft_quality_smoke_check(
    response_text: &str,
    normalized_response: &str,
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
    let decision_value = query_satisfaction
        .get("decision_value")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let direct_user_help = final_answer_present && intent_answered;
    let meta_process_talk = response_has_meta_process_talk(normalized_response)
        && (!direct_user_help || source_summary_without_answer);
    let delegates_research_back_to_user =
        response_delegates_research_back_to_user(normalized_response) && !intent_answered;
    let obviously_bad_shape = raw_tool_leak
        || internal_leak
        || tool_choice_final_response
        || truncated_or_incomplete_response
        || source_summary_without_answer
        || !normal_prose_signal(response_text);

    let mut subgates = serde_json::Map::new();
    subgates.insert(
        "smoke_1_no_meta_process_talk".to_string(),
        json!(!meta_process_talk),
    );
    subgates.insert(
        "smoke_2_not_source_dump_without_answer".to_string(),
        json!(!source_summary_without_answer),
    );
    subgates.insert(
        "smoke_3_not_delegating_research_back_to_user".to_string(),
        json!(!delegates_research_back_to_user),
    );
    subgates.insert(
        "smoke_4_direct_user_help_present".to_string(),
        json!(direct_user_help),
    );
    subgates.insert(
        "smoke_5_projection_not_obviously_bad".to_string(),
        json!(!obviously_bad_shape),
    );
    subgates.insert(
        "smoke_6_decision_or_explanatory_value_present".to_string(),
        json!(decision_value || has_tradeoff_or_structure(normalized_response)),
    );
    subgates.insert(
        "smoke_7_response_not_truncated".to_string(),
        json!(!truncated_or_incomplete_response),
    );
    let ordered = [
        ("smoke_1_no_meta_process_talk", "meta_process_talk_visible"),
        (
            "smoke_2_not_source_dump_without_answer",
            "source_dump_without_answer",
        ),
        (
            "smoke_3_not_delegating_research_back_to_user",
            "delegates_research_back_to_user",
        ),
        ("smoke_4_direct_user_help_present", "direct_answer_missing"),
        (
            "smoke_5_projection_not_obviously_bad",
            "projection_shape_obviously_bad",
        ),
        (
            "smoke_6_decision_or_explanatory_value_present",
            "decision_or_explanatory_value_missing",
        ),
        (
            "smoke_7_response_not_truncated",
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
    let score = subgates
        .values()
        .filter(|value| value.as_bool().unwrap_or(false))
        .count() as u64;
    json!({
        "schema_version": 1,
        "lane_id": "soft_quality_smoke_v1",
        "pass": blockers.is_empty(),
        "score": score,
        "max_score": 7,
        "subgates": Value::Object(subgates),
        "blockers": blockers,
        "top_blocker": blockers.first().cloned().unwrap_or_else(|| "none".to_string()),
        "note": "This is a soft UX smoke lane, not an authoritative grading contract. It flags answers that would likely feel obviously bad to a real user even if structural gates passed."
    })
}

fn user_facing_answer_quality_check(
    response_text: &str,
    normalized_response: &str,
    query_satisfaction: &Value,
    citation_behavior: &Value,
    soft_quality_smoke: &Value,
    answer_unit_evidence_alignment: &Value,
    answer_unit_usefulness: &Value,
    source_summary_without_answer: bool,
    raw_tool_leak: bool,
    internal_leak: bool,
    tool_choice_final_response: bool,
    truncated_or_incomplete_response: bool,
) -> Value {
    let final_answer_present = !response_text.trim().is_empty()
        && response_text.split_whitespace().count() >= 20
        && normal_prose_signal(response_text);
    let direct_user_help = final_answer_present
        && query_satisfaction
            .get("intent_answered")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    let intent_answered = query_satisfaction
        .get("intent_answered")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let decision_value = query_satisfaction
        .get("decision_value")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let right_granularity = query_satisfaction
        .get("right_granularity")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let citation_signal = citation_behavior
        .get("citation_signal")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let response_source_signal = citation_behavior
        .get("response_source_signal")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let citable_evidence_available = citation_behavior
        .get("evidence_count")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        > 0;
    let soft_smoke_pass = soft_quality_smoke
        .get("pass")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let direct_useful_units = answer_unit_usefulness
        .get("direct_useful_units")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let substantive_units = answer_unit_usefulness
        .get("substantive_units")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let process_metadata_units = answer_unit_usefulness
        .get("process_metadata_units")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let useful_units_pass = answer_unit_usefulness
        .get("pass")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let alignment_evaluated = answer_unit_evidence_alignment
        .get("evaluated")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let alignment_pass = answer_unit_evidence_alignment
        .get("pass")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let term_support_rate = answer_unit_evidence_alignment
        .get("term_support_rate")
        .and_then(Value::as_f64)
        .unwrap_or(1.0);
    let unsupported_unit_count = answer_unit_evidence_alignment
        .get("unsupported_unit_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let limitation_heavy = limitation_heavy_for_excellent(normalized_response);
    let explicit_goal_gap =
        response_explicitly_cannot_answer_goal_from_current_evidence(normalized_response);
    let explicit_recap_frame = contains_any(
        normalized_response,
        &[
            "recorded evidence so far",
            "here s what i found",
            "heres what i found",
            "the current turn does not yet support",
        ],
    );
    let meta_process_talk = response_has_meta_process_talk(normalized_response)
        && (!direct_user_help || source_summary_without_answer || explicit_recap_frame);
    let delegates_research = response_delegates_research_back_to_user(normalized_response);
    let source_title_fragment_contamination =
        response_has_source_title_fragment_contamination(response_text);
    let projection_clean = !raw_tool_leak && !internal_leak && !tool_choice_final_response;
    let bounded_recommendation_closure = direct_useful_units >= 2
        && has_recommendation_signal(normalized_response)
        && has_tradeoff_or_structure(normalized_response)
        && !delegates_research
        && !source_summary_without_answer;
    let insufficiency_without_bounded_closure =
        explicit_goal_gap && !bounded_recommendation_closure;
    let enough_substance = substantive_units >= 2
        || response_text.split_whitespace().count() >= 55
        || (intent_answered && direct_useful_units >= 1);
    let useful_substance = direct_useful_units >= 2
        || (intent_answered && (decision_value || has_tradeoff_or_structure(normalized_response)));
    let readable_and_complete =
        normal_prose_signal(response_text) && !truncated_or_incomplete_response;
    let source_signal_ok = !citable_evidence_available || citation_signal || response_source_signal;
    let not_mostly_limits = !limitation_heavy || direct_useful_units >= 2;
    let not_process_dominated =
        process_metadata_units == 0 || process_metadata_units * 2 < substantive_units.max(1);
    let traceable_enough = !alignment_evaluated || alignment_pass || term_support_rate >= 0.85;
    let specific_without_bad_overreach =
        unsupported_unit_count == 0 || (unsupported_unit_count <= 1 && term_support_rate >= 0.9);
    let direct_answer_signal = intent_answered || direct_useful_units > 0;
    let source_or_process_recap_visible =
        source_summary_without_answer || meta_process_talk || delegates_research;

    let mut subgates = serde_json::Map::new();
    subgates.insert(
        "user_1_stands_alone_as_answer".to_string(),
        json!(final_answer_present && direct_answer_signal),
    );
    subgates.insert(
        "user_2_has_substantive_user_value".to_string(),
        json!(useful_substance && enough_substance && !insufficiency_without_bounded_closure),
    );
    subgates.insert(
        "user_3_not_source_or_process_recap".to_string(),
        json!(!source_or_process_recap_visible),
    );
    subgates.insert(
        "user_4_readable_complete_shape".to_string(),
        json!(readable_and_complete),
    );
    subgates.insert(
        "user_5_not_limitation_heavy".to_string(),
        json!(not_mostly_limits),
    );
    subgates.insert(
        "user_6_source_signal_fits_evidence".to_string(),
        json!(source_signal_ok),
    );
    subgates.insert(
        "user_7_answer_units_are_prompt_useful".to_string(),
        json!(useful_units_pass && direct_useful_units > 0 && not_process_dominated),
    );
    subgates.insert(
        "user_8_concrete_units_trace_to_evidence".to_string(),
        json!(traceable_enough && specific_without_bad_overreach),
    );
    subgates.insert(
        "user_9_projection_clean".to_string(),
        json!(projection_clean && readable_and_complete && !source_summary_without_answer),
    );
    subgates.insert(
        "user_10_right_level_of_detail".to_string(),
        json!(right_granularity && enough_substance),
    );
    subgates.insert(
        "user_11_not_source_title_fragment_contamination".to_string(),
        json!(!source_title_fragment_contamination),
    );
    subgates.insert(
        "user_12_explicit_gap_still_closes_usefully".to_string(),
        json!(!insufficiency_without_bounded_closure),
    );

    let ordered = [
        ("user_1_stands_alone_as_answer", "standalone_answer_missing"),
        (
            "user_2_has_substantive_user_value",
            "substantive_user_value_missing",
        ),
        (
            "user_3_not_source_or_process_recap",
            "source_or_process_recap_visible",
        ),
        (
            "user_4_readable_complete_shape",
            "readability_or_completion_issue",
        ),
        ("user_5_not_limitation_heavy", "limitation_heavy_answer"),
        (
            "user_6_source_signal_fits_evidence",
            "source_signal_missing_for_citable_evidence",
        ),
        (
            "user_7_answer_units_are_prompt_useful",
            "answer_units_not_prompt_useful",
        ),
        (
            "user_8_concrete_units_trace_to_evidence",
            "concrete_units_not_traceable_enough",
        ),
        ("user_9_projection_clean", "projection_or_smoke_issue"),
        ("user_10_right_level_of_detail", "wrong_level_of_detail"),
        (
            "user_11_not_source_title_fragment_contamination",
            "source_title_fragment_contamination",
        ),
        (
            "user_12_explicit_gap_still_closes_usefully",
            "insufficiency_without_bounded_closure",
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
    let score = subgates
        .values()
        .filter(|value| value.as_bool().unwrap_or(false))
        .count() as u64;
    let pass = score >= 9
        && soft_smoke_pass
        && !blockers
            .iter()
            .any(|blocker| user_facing_answer_quality_fatal_blocker(blocker));
    let verdict = if pass {
        "sounds_good"
    } else if score >= 7 {
        "borderline"
    } else {
        "sounds_bad"
    };
    json!({
        "schema_version": 1,
        "lane_id": "user_facing_answer_quality_v1",
        "pass": pass,
        "verdict": verdict,
        "score": score,
        "max_score": 11,
        "subgates": Value::Object(subgates),
        "blockers": blockers,
        "top_blocker": blockers.first().cloned().unwrap_or_else(|| "none".to_string()),
        "signals": {
            "direct_useful_units": direct_useful_units,
            "substantive_units": substantive_units,
            "process_metadata_units": process_metadata_units,
            "term_support_rate": term_support_rate,
            "unsupported_unit_count": unsupported_unit_count,
            "limitation_heavy": limitation_heavy,
            "soft_smoke_pass": soft_smoke_pass,
            "citable_evidence_available": citable_evidence_available,
            "citation_signal": citation_signal,
            "response_source_signal": response_source_signal,
            "explicit_goal_gap": explicit_goal_gap,
            "bounded_recommendation_closure": bounded_recommendation_closure,
            "insufficiency_without_bounded_closure": insufficiency_without_bounded_closure,
            "source_or_process_recap_visible": source_or_process_recap_visible,
            "source_title_fragment_contamination": source_title_fragment_contamination
        },
        "note": "Soft user-facing proxy. It asks whether the final visible text would feel useful and coherent to a real user if formatting and evaluator state were ignored. It remains diagnostic, but excellent should not outrun it and obviously bad visible answer shapes may be treated as grading failures."
    })
}

fn user_facing_answer_quality_fatal_blocker(blocker: &str) -> bool {
    matches!(
        blocker,
        "standalone_answer_missing"
            | "substantive_user_value_missing"
            | "source_or_process_recap_visible"
            | "readability_or_completion_issue"
            | "projection_or_smoke_issue"
            | "wrong_level_of_detail"
            | "answer_units_not_prompt_useful"
            | "insufficiency_without_bounded_closure"
            | "source_title_fragment_contamination"
    )
}
