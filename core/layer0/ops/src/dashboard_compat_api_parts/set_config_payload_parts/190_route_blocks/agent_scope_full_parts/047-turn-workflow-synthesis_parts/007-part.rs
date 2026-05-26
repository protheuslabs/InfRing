fn fallback_final_response_from_tool_evidence(message: &str, response_tools: &[Value]) -> String {
    let required_entity_lanes = hard_required_entity_lanes_for_tools(response_tools, 8);
    let goal_terms = workflow_answer_unit_goal_terms(message);
    let answer_units = evidence_packet_answer_units_for_goal(message, response_tools, 4);
    let partial_decision_hint =
        synthesis_partial_comparison_decision_hint(message, response_tools);
    if !answer_units.is_empty() {
        let mut answer_parts = Vec::<String>::new();
        let mut lane_scoped_answer_parts = Vec::<String>::new();
        let mut covered_required_entity_lanes = std::collections::BTreeSet::<String>::new();
        for unit in answer_units {
            let (answer, matched_lanes) = fallback_visible_answer_for_required_lanes(
                &unit,
                &required_entity_lanes,
                &goal_terms,
            );
            if !required_entity_lanes.is_empty()
                && matched_lanes.is_empty()
                && workflow_answer_unit_goal_overlap_count(&answer, &goal_terms) == 0
            {
                continue;
            }
            if !answer.is_empty() && !answer_parts.iter().any(|existing| existing == &answer) {
                answer_parts.push(answer.clone());
            }
            if !answer.is_empty()
                && !matched_lanes.is_empty()
                && !lane_scoped_answer_parts
                    .iter()
                    .any(|existing| existing == &answer)
            {
                lane_scoped_answer_parts.push(answer.clone());
            }
            for lane in matched_lanes {
                covered_required_entity_lanes.insert(normalize_coverage_lane_text(&lane));
            }
        }
        let minimum_lane_coverage = minimum_required_entity_lane_coverage(&required_entity_lanes);
        let coverage_note = fallback_user_visible_coverage_note(response_tools);
        if minimum_lane_coverage > 0 && covered_required_entity_lanes.len() < minimum_lane_coverage
        {
            let mut parts = vec![if covered_required_entity_lanes.is_empty() {
                "The current evidence does not yet support a reliable comparison across the requested entities.".to_string()
            } else {
                "The current evidence supports only a partial comparison across the requested entities.".to_string()
            }];
            if partial_decision_hint.is_empty() {
                parts.extend(
                    lane_scoped_answer_parts
                        .iter()
                        .take(2)
                        .map(|part| workflow_finish_visible_sentence(part)),
                );
            }
            if !partial_decision_hint.is_empty() {
                parts.push(workflow_finish_visible_sentence(&partial_decision_hint));
            }
            if !coverage_note.is_empty() {
                parts.push(workflow_finish_visible_sentence(&coverage_note));
            }
            if parts.len() > 1 {
                return clean_text(&parts.join(" "), 2_400);
            }
            answer_parts.clear();
        }
        if let Some(first_answer) = answer_parts.first() {
            let mut parts = vec![workflow_finish_visible_sentence(first_answer)];
            if answer_parts.len() > 1 {
                parts.extend(
                    answer_parts[1..]
                        .iter()
                        .map(|part| workflow_finish_visible_sentence(part)),
                );
            }
            if !partial_decision_hint.is_empty() {
                parts.push(workflow_finish_visible_sentence(&partial_decision_hint));
            }
            if !coverage_note.is_empty() {
                parts.push(workflow_finish_visible_sentence(&coverage_note));
            }
            return clean_text(&parts.join("\n"), 2_400);
        }
    }
    let failure_reason = clean_text(
        &first_sentence(
            &response_tools_failure_reason_for_user(response_tools, 4),
            320,
        ),
        360,
    );
    let mut findings = clean_text(
        &first_sentence(&response_tools_summary_for_user(response_tools, 4), 420),
        480,
    );
    if !required_entity_lanes.is_empty()
        && text_matches_required_entity_lanes(&findings, &required_entity_lanes).is_empty()
    {
        findings.clear();
    }
    let coverage_note = clean_text(
        &first_sentence(&fallback_coverage_lane_sentence(response_tools), 280),
        320,
    );
    if findings.is_empty() && failure_reason.is_empty() {
        if coverage_note.is_empty() {
            return String::new();
        }
        return clean_text(
            &format!(
                "My recommendation is to treat the current evidence as insufficient for a direct source-backed conclusion. {coverage_note}"
            ),
            900,
        );
    }
    let opening = if !findings.is_empty() {
        "The practical answer is that the current evidence supports only a partial conclusion."
    } else {
        "My recommendation is to treat the current evidence as insufficient for a direct source-backed conclusion."
    };
    let mut parts = vec![opening.to_string()];
    if !failure_reason.is_empty() {
        parts.push(failure_reason);
    }
    if !findings.is_empty() {
        parts.push(findings);
    }
    if !partial_decision_hint.is_empty() {
        parts.push(partial_decision_hint);
    }
    if !coverage_note.is_empty() {
        parts.push(coverage_note);
    }
    clean_text(&parts.join(" "), 900)
}

fn apply_tool_evidence_fallback_response(
    workflow: &mut Value,
    response_tools: &[Value],
    fallback_response: &str,
    fallback_source: &str,
    error_code: &str,
    original_reject_reason: Option<&str>,
    original_reject_excerpt: Option<&str>,
    diagnostic_reason: &str,
    diagnostic_stage: &str,
) {
    let cleaned_response = persist_workflow_visible_response(
        workflow,
        &clean_text(fallback_response, 3_000),
    );
    if cleaned_response.is_empty() {
        return;
    }
    workflow["quality_telemetry"]["final_fallback_used"] = Value::Bool(true);
    workflow["quality_telemetry"]["final_fallback_suppressed"] = Value::Bool(false);
    workflow["quality_telemetry"]["runtime_visible_fallback_source"] =
        Value::String(clean_text(fallback_source, 120));
    workflow["final_llm_response"]["used"] = Value::Bool(true);
    workflow["final_llm_response"]["status"] =
        Value::String("tool_evidence_fallback_used".to_string());
    workflow["final_llm_response"]["runtime_interference_disabled"] = Value::Bool(true);
    workflow["final_llm_response"]["visible_response_preserved"] = Value::Bool(false);
    workflow["final_llm_response"]["fallback_source"] =
        Value::String(clean_text(fallback_source, 120));
    workflow["final_llm_response"]["replacement_response_used"] = Value::Bool(true);
    workflow["final_llm_response"]["replacement_response_excerpt"] =
        Value::String(first_sentence(&cleaned_response, 240));
    workflow["final_llm_response"]["error"] = Value::String(clean_text(error_code, 160));
    workflow["final_llm_response"]["last_reject_reason"] =
        Value::String("runtime_visible_tool_evidence_fallback_used".to_string());
    annotate_final_evidence_outcome_posture(workflow, response_tools);
    if let Some(reason) = original_reject_reason {
        let cleaned = clean_text(reason, 240);
        if !cleaned.is_empty() {
            workflow["final_llm_response"]["original_reject_reason"] = Value::String(cleaned);
        }
    }
    if let Some(excerpt) = original_reject_excerpt {
        let cleaned = clean_text(excerpt, 600);
        if !cleaned.is_empty() {
            workflow["final_llm_response"]["original_reject_excerpt"] = Value::String(cleaned);
        }
    }
    record_workflow_diagnostic_event(
        workflow,
        diagnostic_reason,
        diagnostic_stage,
    );
    set_turn_workflow_final_stage_status(workflow, "tool_evidence_fallback_used");
}

fn maybe_apply_rejected_tool_evidence_fallback(
    workflow: &mut Value,
    message: &str,
    response_tools: &[Value],
    last_invalid_response_text: &str,
    last_invalid_excerpt: &str,
    last_reject_reason: &str,
) -> bool {
    if response_tools.is_empty() || last_invalid_excerpt.trim().is_empty() {
        return false;
    }
    let _ = last_invalid_response_text;
    let fallback_response = clean_text(
        &fallback_final_response_from_tool_evidence(message, response_tools),
        3_000,
    );
    if fallback_response.is_empty() {
        return false;
    }
    apply_tool_evidence_fallback_response(
        workflow,
        response_tools,
        &fallback_response,
        "tool_evidence_runtime_fallback_after_verifier_reject",
        "rejected_response_replaced_from_tool_evidence",
        Some(last_reject_reason),
        Some(last_invalid_excerpt),
        "tool_evidence_verifier_reject_rewritten",
        "synthesis_failure_diagnostic",
    );
    true
}

fn replacement_response_for_retry_boilerplate(message: &str, response_tools: &[Value]) -> String {
    let _ = message;
    let failure_reason = clean_text(
        &first_sentence(
            &response_tools_failure_reason_for_user(response_tools, 4),
            280,
        ),
        320,
    );
    let coverage_note = clean_text(&fallback_coverage_lane_sentence(response_tools), 320);
    let opening = "The retrieved evidence in this turn was not strong enough to support a clean source-backed conclusion across all requested lanes.";
    if !failure_reason.is_empty() && !coverage_note.is_empty() {
        clean_text(&format!("{opening} {failure_reason} {coverage_note}"), 800)
    } else if !failure_reason.is_empty() {
        clean_text(&format!("{opening} {failure_reason}"), 800)
    } else if !coverage_note.is_empty() {
        clean_text(&format!("{opening} {coverage_note}"), 800)
    } else {
        opening.to_string()
    }
}

fn agent_runtime_temporal_context_prompt() -> String {
    let current_utc = crate::now_iso();
    clean_text(
        &format!(
            "Runtime temporal context: current date/time is {current_utc} (UTC). Treat this runtime timestamp as authoritative for this turn. Dates before this timestamp are in the past; dates after it are in the future. If the user supplies a local date/time correction for the active turn, reconcile against it instead of relying on model training cutoff memory."
        ),
        800,
    )
}

fn tool_completion_report_for_response(
    response_text: &str,
    response_tools: &[Value],
    outcome: &str,
) -> Value {
    let cleaned = clean_chat_text(response_text, 32_000);
    let findings = clean_text(&response_tools_summary_for_user(response_tools, 4), 4_000);
    let failure_reason = clean_text(
        &response_tools_failure_reason_for_user(response_tools, 4),
        4_000,
    );
    let reasoning_source = if !cleaned.is_empty() {
        cleaned.clone()
    } else if !failure_reason.is_empty() {
        failure_reason.clone()
    } else {
        findings.clone()
    };
    let completion_state = if response_tools.is_empty() {
        "not_applicable"
    } else if !failure_reason.is_empty() {
        "reported_reason"
    } else if !findings.is_empty() {
        "reported_findings"
    } else {
        "reported_no_findings"
    };
    let deferred_execution = response_is_deferred_execution_preamble(&cleaned)
        || response_is_deferred_retry_prompt(&cleaned);
    json!({
        "completion_state": completion_state,
        "findings_available": !findings.is_empty(),
        "final_ack_only": response_looks_like_tool_ack_without_findings(&cleaned),
        "final_no_findings": response_is_no_findings_placeholder(&cleaned),
        "final_deferred_execution": deferred_execution,
        "final_requests_more_tooling": workflow_response_requests_more_tooling(&cleaned),
        "reasoning": first_sentence(&reasoning_source, 220),
        "outcome": clean_text(outcome, 200)
    })
}
