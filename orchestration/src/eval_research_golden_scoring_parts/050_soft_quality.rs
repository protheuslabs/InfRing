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

