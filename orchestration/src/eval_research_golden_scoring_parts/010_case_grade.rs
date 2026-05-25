// Layer ownership: orchestration (research eval authority)

pub(super) struct CaseGrade {
    pub(super) score: u64,
    pub(super) pass: bool,
    pub(super) excellent: bool,
    pub(super) gates: BTreeMap<String, bool>,
    pub(super) dimension_scores: BTreeMap<String, u64>,
    pub(super) failures: Vec<String>,
    pub(super) response_text: String,
    pub(super) empty_response: bool,
    pub(super) raw_tool_leak: bool,
    pub(super) tool_choice_final_response: bool,
    pub(super) unsupported_claim: bool,
    pub(super) retrieval_quality: Value,
    pub(super) excellent_blockers: Vec<String>,
    pub(super) excellent_diagnostics: Value,
    pub(super) coverage_entities: Vec<String>,
    pub(super) citation_behavior: Value,
    pub(super) query_satisfaction: Value,
    pub(super) response_grading_layers: Value,
    pub(super) soft_quality_smoke: Value,
    pub(super) user_facing_answer_quality: Value,
    pub(super) answer_unit_evidence_alignment: Value,
    pub(super) answer_unit_usefulness: Value,
}

pub(super) fn grade_case(
    case: &Value,
    payload: &Value,
    pass_score: u64,
    excellent_score: u64,
) -> CaseGrade {
    let response_text = visible_response_text_for_grading(payload);
    let normalized = normalize_for_compare(&response_text);
    let prompt = str_at(case, &["prompt"], "");
    let normalized_prompt = normalize_for_compare(&prompt);
    let required_entities = string_array_at(case, &["required_entities"]);
    let coverage_entities = user_stated_required_entities(&normalized_prompt, &required_entities);
    let gates = gate_results(case, payload);
    let raw_tool_leak = raw_tool_payload_leak(&response_text);
    let internal_leak = internal_workflow_leak(&response_text);
    let tool_choice_final_response = tool_choice_as_final_response(&response_text);
    let empty_response = response_text.trim().is_empty();
    let unsupported_claim = unsupported_claim_signal(case, &response_text);
    let truncated_or_incomplete_response = response_looks_truncated_or_incomplete(&response_text);
    let retrieval_quality = retrieval_provider_quality(payload, &normalized_prompt);
    let source_signal = has_source_signal(&response_text, &retrieval_quality);
    let citation_behavior = citation_behavior(payload, &response_text, &retrieval_quality);
    let citation_signal = citation_behavior
        .get("citation_signal")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let response_source_signal = citation_behavior
        .get("response_source_signal")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let limitation_signal = has_limitation_signal(&normalized);
    let final_answer_present = !empty_response && response_text.split_whitespace().count() >= 20;
    let entity_coverage = entity_coverage(&normalized, &coverage_entities);
    let query_satisfaction = query_satisfaction(
        &normalized_prompt,
        &normalized,
        &coverage_entities,
        entity_coverage,
        final_answer_present,
        response_source_signal,
        citation_signal,
        limitation_signal,
    );
    let query_satisfaction_score = query_satisfaction
        .get("score")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let source_summary_without_answer = source_summary_without_answer_signal(&normalized);
    let generic_response_contract = generic_response_contract(
        &response_text,
        final_answer_present,
        &query_satisfaction,
        source_summary_without_answer,
        raw_tool_leak,
        internal_leak,
        tool_choice_final_response,
        truncated_or_incomplete_response,
    );
    let evidence_use_contract = tool_backed_evidence_contract(
        &normalized,
        &retrieval_quality,
        &citation_behavior,
        limitation_signal,
        &query_satisfaction,
        unsupported_claim,
        outside_evidence_used_for_decision_signal(&normalized),
    );
    let workflow_specific_rubric = research_workflow_specific_rubric(
        &query_satisfaction,
        source_signal,
        limitation_signal,
        &normalized,
    );
    let response_grading_layers = json!({
        "schema_version": 1,
        "generic_response_contract": generic_response_contract,
        "tool_backed_evidence_contract": evidence_use_contract,
        "workflow_specific_rubric": workflow_specific_rubric,
        "note": "Separates general answer quality, evidence-use discipline, and research-specific rubric checks so format flexibility and workflow-specific semantics can evolve independently."
    });
    let soft_quality_smoke = soft_quality_smoke_check(
        &response_text,
        &normalized,
        final_answer_present,
        &query_satisfaction,
        source_summary_without_answer,
        raw_tool_leak,
        internal_leak,
        tool_choice_final_response,
        truncated_or_incomplete_response,
    );
    let answer_unit_evidence_alignment =
        answer_unit_evidence_alignment(payload, &response_text, &retrieval_quality);
    let answer_unit_usefulness =
        answer_unit_usefulness_for_prompt(&normalized_prompt, &response_text, &retrieval_quality);
    let user_facing_answer_quality = user_facing_answer_quality_check(
        &response_text,
        &normalized,
        &query_satisfaction,
        &citation_behavior,
        &soft_quality_smoke,
        &answer_unit_evidence_alignment,
        &answer_unit_usefulness,
        source_summary_without_answer,
        raw_tool_leak,
        internal_leak,
        tool_choice_final_response,
        truncated_or_incomplete_response,
    );
    let answer_unit_alignment_blocks_excellent = answer_unit_evidence_alignment
        .get("evaluated")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && !answer_unit_evidence_alignment
            .get("pass")
            .and_then(Value::as_bool)
            .unwrap_or(true);
    let answer_unit_usefulness_blocks_excellent = answer_unit_usefulness
        .get("evaluated")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && !answer_unit_usefulness
            .get("pass")
            .and_then(Value::as_bool)
            .unwrap_or(true);

    let workflow_score = gates.values().filter(|ok| **ok).count() as u64 * 5;
    let evidence_score = (if source_signal { 6 } else { 0 })
        + (if citation_signal { 6 } else { 0 })
        + (if !raw_tool_leak { 5 } else { 0 })
        + (if limitation_signal { 4 } else { 0 })
        + (if !unsupported_claim { 4 } else { 0 });
    let synthesis_score_raw = (if final_answer_present { 6 } else { 0 })
        + ((entity_coverage * 7.0).round() as u64)
        + (if has_tradeoff_or_structure(&normalized) {
            6
        } else {
            0
        })
        + (if has_recommendation_signal(&normalized) {
            4
        } else {
            0
        })
        + (if limitation_signal { 2 } else { 0 })
        + query_satisfaction_score.min(10);
    let synthesis_score =
        synthesis_score_raw.saturating_sub(if source_summary_without_answer { 8 } else { 0 });
    let projection_score = (if !raw_tool_leak { 5 } else { 0 })
        + (if !internal_leak { 5 } else { 0 })
        + (if !empty_response { 5 } else { 0 })
        + (if normal_prose_signal(&response_text) {
            5
        } else {
            0
        });
    let mut dimension_scores = BTreeMap::new();
    dimension_scores.insert("workflow_path".to_string(), workflow_score.min(20));
    dimension_scores.insert("evidence_behavior".to_string(), evidence_score.min(25));
    dimension_scores.insert("synthesis_quality".to_string(), synthesis_score.min(35));
    dimension_scores.insert("projection_safety".to_string(), projection_score.min(20));
    let score = dimension_scores.values().sum::<u64>().min(100);
    let mut failures = Vec::new();
    if !gates.values().all(|ok| *ok) {
        failures.push("workflow_gate_path_incomplete".to_string());
    }
    if empty_response {
        failures.push("empty_research_response".to_string());
    }
    if !source_signal {
        failures.push("missing_evidence_or_source_signal".to_string());
    }
    if !coverage_entities.is_empty() && entity_coverage < 0.75 {
        failures.push(format!("entity_coverage_low:{entity_coverage:.2}"));
    }
    if query_satisfaction_score < 7 {
        failures.push(format!(
            "query_satisfaction_low:{query_satisfaction_score}<7"
        ));
    }
    if source_summary_without_answer {
        failures.push("source_summary_without_user_answer".to_string());
    }
    if raw_tool_leak {
        failures.push("raw_tool_payload_leaked".to_string());
    }
    if internal_leak {
        failures.push("internal_workflow_state_leaked".to_string());
    }
    if tool_choice_final_response {
        failures.push("tool_choice_visible_as_final_response".to_string());
    }
    if truncated_or_incomplete_response {
        failures.push("truncated_or_incomplete_response".to_string());
    }
    if unsupported_claim {
        failures.push("unsupported_overconfident_claim_signal".to_string());
    }
    if outside_evidence_used_for_decision_signal(&normalized) {
        failures.push("outside_evidence_used_for_decision".to_string());
    }
    if answer_unit_alignment_hard_failure(&answer_unit_evidence_alignment) {
        failures.push("answer_units_not_traceable_to_evidence".to_string());
    }
    if answer_unit_usefulness_hard_failure(&answer_unit_usefulness) {
        failures.push("answer_units_not_useful_for_prompt".to_string());
    }
    if user_facing_answer_hard_failure(&user_facing_answer_quality)
        && !response_explicitly_cannot_answer_goal_from_current_evidence(&normalized)
    {
        failures.push("user_facing_answer_not_good_enough".to_string());
    }
    if score < pass_score {
        failures.push(format!("research_score_below_pass:{score}<{pass_score}"));
    }
    failures.sort();
    failures.dedup();
    let excellent_diagnostics = excellent_diagnostics(ExcellentDiagnosticInput {
        retrieval_quality: &retrieval_quality,
        citation_behavior: &citation_behavior,
        query_satisfaction: &query_satisfaction,
        normalized_response: &normalized,
        source_signal,
        final_answer_present,
        limitation_signal,
        raw_tool_leak,
        internal_leak,
        unsupported_claim,
        score,
        excellent_score,
        failures: &failures,
        answer_unit_evidence_alignment: &answer_unit_evidence_alignment,
        answer_unit_usefulness: &answer_unit_usefulness,
        user_facing_answer_quality: &user_facing_answer_quality,
    });
    let excellent_blockers = string_array_at(&excellent_diagnostics, &["blockers"]);
    let user_facing_quality_blocks_excellent = !user_facing_answer_quality
        .get("pass")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    CaseGrade {
        score,
        pass: score >= pass_score && failures.is_empty(),
        excellent: score >= excellent_score
            && failures.is_empty()
            && excellent_blockers.is_empty()
            && !answer_unit_alignment_blocks_excellent
            && !answer_unit_usefulness_blocks_excellent
            && !user_facing_quality_blocks_excellent,
        gates,
        dimension_scores,
        failures,
        response_text,
        empty_response,
        raw_tool_leak,
        tool_choice_final_response,
        unsupported_claim,
        retrieval_quality,
        excellent_blockers,
        excellent_diagnostics,
        coverage_entities,
        citation_behavior,
        query_satisfaction,
        response_grading_layers,
        soft_quality_smoke,
        user_facing_answer_quality,
        answer_unit_evidence_alignment,
        answer_unit_usefulness,
    }
}

fn answer_unit_usefulness_hard_failure(usefulness: &Value) -> bool {
    if !usefulness
        .get("evaluated")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return false;
    }
    if usefulness
        .get("pass")
        .and_then(Value::as_bool)
        .unwrap_or(true)
    {
        return false;
    }
    let process_metadata_units = usefulness
        .get("process_metadata_units")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let direct_useful_units = usefulness
        .get("direct_useful_units")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let usable_evidence = usefulness
        .get("usable_evidence")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    process_metadata_units >= 2 || (usable_evidence && direct_useful_units == 0)
}

fn user_facing_answer_hard_failure(answer_quality: &Value) -> bool {
    if answer_quality
        .get("pass")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return false;
    }
    let verdict = str_at(answer_quality, &["verdict"], "");
    let blockers = string_array_at(answer_quality, &["blockers"]);
    blockers.iter().any(|blocker| {
        matches!(
            blocker.as_str(),
            "standalone_answer_missing"
                | "source_or_process_recap_visible"
                | "source_title_fragment_contamination"
                | "readability_or_completion_issue"
                | "projection_or_smoke_issue"
        )
    }) || (verdict == "sounds_bad"
        && blockers
            .iter()
            .any(|blocker| blocker == "substantive_user_value_missing"))
}
