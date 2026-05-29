fn fallback_final_response_from_tool_evidence(message: &str, response_tools: &[Value]) -> String {
    let required_entity_lanes = hard_required_entity_lanes_for_tools(response_tools, 8);
    let goal_terms = workflow_answer_unit_goal_terms(message);
    let answer_units = evidence_packet_answer_units_for_goal(message, response_tools, 4);
    let partial_decision_hint =
        synthesis_partial_comparison_decision_hint(message, response_tools);
    let comparison_intent = synthesis_message_is_comparison_intent(message);
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
            let mut parts = vec![if comparison_intent && covered_required_entity_lanes.is_empty() {
                "The current evidence does not yet support a reliable comparison across the requested entities.".to_string()
            } else if comparison_intent {
                "The current evidence supports only a partial comparison across the requested entities.".to_string()
            } else if covered_required_entity_lanes.is_empty() {
                "The current evidence supports only a partial answer to the request so far.".to_string()
            } else {
                "The current evidence supports only a partial answer to the request so far.".to_string()
            }];
            if partial_decision_hint.is_empty() {
                let support_parts = if comparison_intent {
                    lane_scoped_answer_parts.iter().collect::<Vec<_>>()
                } else {
                    answer_parts.iter().collect::<Vec<_>>()
                };
                parts.extend(
                    support_parts
                        .into_iter()
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
        let evidence_sketch = clean_text(
            &bounded_evidence_sketch_for_rejected_fallback(message, response_tools, ""),
            1_200,
        );
        if !evidence_sketch.is_empty() {
            let mut parts = vec![workflow_finish_visible_sentence(&evidence_sketch)];
            if !partial_decision_hint.is_empty() {
                parts.push(workflow_finish_visible_sentence(&partial_decision_hint));
            }
            let coverage_gap_note = fallback_user_visible_coverage_note(response_tools);
            if !coverage_gap_note.is_empty() {
                parts.push(workflow_finish_visible_sentence(&coverage_gap_note));
            }
            return clean_text(&parts.join(" "), 1_600);
        }
        let snippet_sketch = clean_text(
            &fallback_evidence_snippet_sentence_from_tools(message, response_tools, ""),
            1_200,
        );
        if !snippet_sketch.is_empty() {
            let mut parts = vec![workflow_finish_visible_sentence(&snippet_sketch)];
            if !partial_decision_hint.is_empty() {
                parts.push(workflow_finish_visible_sentence(&partial_decision_hint));
            }
            let coverage_gap_note = fallback_user_visible_coverage_note(response_tools);
            if !coverage_gap_note.is_empty() {
                parts.push(workflow_finish_visible_sentence(&coverage_gap_note));
            }
            return clean_text(&parts.join(" "), 1_600);
        }
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
    let salvage_source_text = if !last_invalid_response_text.trim().is_empty() {
        last_invalid_response_text
    } else {
        last_invalid_excerpt
    };
    let fallback_response = clean_text(
        &fallback_final_response_from_tool_evidence(message, response_tools),
        3_000,
    );
    let salvaged_response = clean_text(
        &salvaged_rejected_answer_units_for_fallback(
            message,
            salvage_source_text,
            response_tools,
            last_reject_reason,
        ),
        3_000,
    );
    let excerpt_sentence_salvage = clean_text(
        &fallback_excerpt_sentence_from_rejected_response(
            message,
            last_invalid_excerpt,
            response_tools,
            last_reject_reason,
        ),
        3_000,
    );
    let evidence_sketch_salvage = clean_text(
        &bounded_evidence_sketch_for_rejected_fallback(
            message,
            response_tools,
            last_reject_reason,
        ),
        3_000,
    );
    let preferred_response = if !salvaged_response.is_empty() {
        salvaged_response
    } else if !excerpt_sentence_salvage.is_empty() {
        excerpt_sentence_salvage
    } else if !evidence_sketch_salvage.is_empty() {
        evidence_sketch_salvage
    } else {
        fallback_response
    };
    if preferred_response.is_empty() {
        return false;
    }
    apply_tool_evidence_fallback_response(
        workflow,
        response_tools,
        &preferred_response,
        "tool_evidence_runtime_fallback_after_verifier_reject",
        "rejected_response_replaced_from_tool_evidence",
        Some(last_reject_reason),
        Some(last_invalid_excerpt),
        "tool_evidence_verifier_reject_rewritten",
        "synthesis_failure_diagnostic",
    );
    true
}

fn workflow_final_llm_diagnostic_text(
    workflow: &Value,
    pointers: &[&str],
    max_len: usize,
) -> String {
    pointers
        .iter()
        .find_map(|pointer| {
            let cleaned = clean_text(workflow.pointer(pointer).and_then(Value::as_str).unwrap_or(""), max_len);
            (!cleaned.is_empty()).then_some(cleaned)
        })
        .unwrap_or_default()
}

fn response_is_low_information_coverage_fallback(response_text: &str) -> bool {
    let lowered = clean_text(response_text, 1_200).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let coverage_shell = (lowered.contains("coverage state:")
        && lowered.contains("usable evidence is present"))
        || lowered.contains("coverage gaps still matter for:");
    coverage_shell
        && (lowered.starts_with("my recommendation is to treat the current evidence as insufficient")
            || lowered.starts_with("the practical answer is that the current evidence supports only a partial conclusion"))
}

fn response_is_low_information_tool_evidence_fallback(response_text: &str) -> bool {
    let cleaned = clean_text(response_text, 1_200);
    if cleaned.is_empty() {
        return false;
    }
    if response_is_low_information_coverage_fallback(&cleaned) {
        return true;
    }
    let word_count = cleaned.split_whitespace().count();
    let headline_candidate = cleaned.trim_end_matches(|ch: char| matches!(ch, '.' | '!' | '?'));
    let title_shell = {
        let alpha_tokens = headline_candidate
            .split_whitespace()
            .map(|token| {
                token.trim_matches(|ch: char| {
                    !ch.is_ascii_alphanumeric() && ch != '-' && ch != '.' && ch != '/'
                })
            })
            .filter(|token| token.chars().any(|ch| ch.is_ascii_alphabetic()))
            .collect::<Vec<_>>();
        let title_like_words = alpha_tokens
            .iter()
            .filter(|token| workflow_answer_unit_token_looks_title_like(token))
            .count();
        let leading_lowercase_content_words = alpha_tokens
            .iter()
            .filter(|token| {
                token.chars().next().map(|ch| ch.is_ascii_lowercase()).unwrap_or(false)
                    && !workflow_answer_unit_source_title_style_stopword(
                        &token.to_ascii_lowercase(),
                    )
            })
            .count();
        let title_ratio = if alpha_tokens.is_empty() {
            0.0
        } else {
            title_like_words as f64 / alpha_tokens.len() as f64
        };
        (4..=18).contains(&alpha_tokens.len())
            && title_like_words >= 3
            && title_ratio >= 0.60
            && leading_lowercase_content_words <= 2
    };
    (word_count <= 18 || !cleaned.contains('.'))
        && (workflow_answer_unit_looks_like_source_title_fragment(&cleaned)
            || workflow_answer_unit_looks_like_source_title_fragment(headline_candidate)
            || workflow_answer_unit_looks_like_datestamped_headline_shell(&cleaned)
            || workflow_text_prefix_looks_like_headline(headline_candidate)
            || title_shell)
}

fn workflow_response_sentence_is_gap_or_status_preface(raw: &str) -> bool {
    let lowered = clean_text(raw, 520).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    [
        "i don't have sufficient specific evidence",
        "i dont have sufficient specific evidence",
        "i do not have sufficient specific evidence",
        "i don't have usable evidence",
        "i dont have usable evidence",
        "i do not have usable evidence",
        "i don't have enough usable evidence",
        "i dont have enough usable evidence",
        "i do not have enough usable evidence",
        "current evidence is insufficient",
        "recorded evidence is insufficient",
        "the recorded evidence for",
        "coverage state:",
        "the current evidence supports only a partial conclusion",
        "my recommendation is to treat the current evidence as insufficient",
    ]
    .iter()
    .any(|needle| lowered.contains(needle))
}

fn maybe_repair_runtime_tool_evidence_fallback_from_reject_excerpt(
    workflow: &mut Value,
    message: &str,
    response_tools: &[Value],
) -> bool {
    if workflow_final_response_status(workflow) != "tool_evidence_fallback_used" {
        return false;
    }
    let current_response = clean_text(workflow.get("response").and_then(Value::as_str).unwrap_or(""), 3_000);
    if current_response.is_empty()
        || !response_is_low_information_tool_evidence_fallback(&current_response)
    {
        return false;
    }
    let reject_excerpt = workflow_final_llm_diagnostic_text(
        workflow,
        &[
            "/final_llm_response/original_reject_excerpt",
            "/final_llm_response/diagnostic_invalid_excerpt",
            "/final_llm_response/error",
        ],
        600,
    );
    let reject_reason = workflow_final_llm_diagnostic_text(
        workflow,
        &[
            "/final_llm_response/original_reject_reason",
            "/final_llm_response/diagnostic_reject_reason",
            "/final_llm_response/last_reject_reason",
        ],
        240,
    );
    let repaired = clean_text(
        &fallback_excerpt_sentence_from_rejected_response(
            message,
            &reject_excerpt,
            response_tools,
            &reject_reason,
        ),
        3_000,
    );
    let repaired = if repaired.is_empty() {
        clean_text(
            &bounded_evidence_sketch_for_rejected_fallback(
                message,
                response_tools,
                &reject_reason,
            ),
            3_000,
        )
    } else {
        repaired
    };
    let repaired = if repaired.is_empty() {
        clean_text(
            &fallback_evidence_snippet_sentence_from_tools(
                message,
                response_tools,
                &reject_reason,
            ),
            3_000,
        )
    } else {
        repaired
    };
    if repaired.is_empty() || repaired == current_response {
        return false;
    }
    let cleaned = persist_workflow_visible_response(workflow, &repaired);
    if cleaned.is_empty() {
        return false;
    }
    workflow["quality_telemetry"]["runtime_visible_fallback_source"] =
        Value::String("tool_evidence_reject_excerpt_repair".to_string());
    workflow["final_llm_response"]["replacement_response_used"] = Value::Bool(true);
    workflow["final_llm_response"]["replacement_response_excerpt"] =
        Value::String(first_sentence(&cleaned, 240));
    workflow["final_llm_response"]["visible_response_repaired_from_reject_excerpt"] =
        Value::Bool(true);
    true
}

fn salvaged_rejected_answer_units_for_fallback(
    message: &str,
    rejected_response_text: &str,
    response_tools: &[Value],
    reject_reason: &str,
) -> String {
    let lowered_reason = reject_reason.to_ascii_lowercase();
    if !lowered_reason.contains("missing_coverage_lanes=")
        && !lowered_reason.contains("answer_units_not_useful_for_prompt")
    {
        return String::new();
    }
    let goal_terms = workflow_answer_unit_goal_terms(message);
    let mut parts = Vec::<String>::new();
    for unit in workflow_answer_text_units(rejected_response_text) {
        let cleaned = clean_text(&unit, 520);
        if cleaned.is_empty()
            || response_looks_like_retrieval_recap_substituted_for_answer(&cleaned)
            || workflow_answer_unit_contains_ui_or_source_shell(&cleaned)
            || workflow_answer_unit_looks_like_source_title_fragment(&cleaned)
            || workflow_answer_unit_looks_like_datestamped_headline_shell(&cleaned)
            || workflow_answer_unit_is_process_or_metadata_fact(&cleaned)
            || workflow_answer_unit_goal_overlap_count(&cleaned, &goal_terms) == 0
            || response_has_answer_unit_traceability_violation(&cleaned, response_tools)
        {
            continue;
        }
        let finished = workflow_finish_visible_sentence(&cleaned);
        if !finished.is_empty() && !parts.iter().any(|existing| existing == &finished) {
            parts.push(finished);
        }
        if parts.len() >= 2 {
            break;
        }
    }
    if parts.is_empty() {
        return String::new();
    }
    let coverage_note = fallback_user_visible_coverage_note(response_tools);
    if lowered_reason.contains("missing_coverage_lanes=") && !coverage_note.is_empty() {
        parts.push(workflow_finish_visible_sentence(&coverage_note));
    }
    clean_text(&parts.join(" "), 1_600)
}

fn fallback_excerpt_sentence_from_rejected_response(
    message: &str,
    rejected_excerpt: &str,
    response_tools: &[Value],
    reject_reason: &str,
) -> String {
    let cleaned = clean_text(&first_sentence(rejected_excerpt, 420), 520);
    if cleaned.is_empty()
        || workflow_answer_unit_is_process_or_metadata_fact(&cleaned)
        || workflow_answer_unit_contains_ui_or_source_shell(&cleaned)
        || workflow_answer_unit_looks_like_source_title_fragment(&cleaned)
        || workflow_answer_unit_looks_like_datestamped_headline_shell(&cleaned)
        || workflow_response_sentence_is_gap_or_status_preface(&cleaned)
    {
        return String::new();
    }
    let goal_terms = workflow_answer_unit_goal_terms(message);
    let goal_overlap = workflow_answer_unit_goal_overlap_count(&cleaned, &goal_terms);
    if goal_overlap == 0 {
        return String::new();
    }
    let mut parts = vec![workflow_finish_visible_sentence(&cleaned)];
    if reject_reason
        .to_ascii_lowercase()
        .contains("missing_coverage_lanes=")
    {
        let coverage_note = fallback_user_visible_coverage_note(response_tools);
        if !coverage_note.is_empty() {
            parts.push(workflow_finish_visible_sentence(&coverage_note));
        }
    }
    clean_text(&parts.join(" "), 1_200)
}

fn bounded_evidence_sketch_for_rejected_fallback(
    message: &str,
    response_tools: &[Value],
    reject_reason: &str,
) -> String {
    let primary_units = evidence_packet_answer_units_for_goal(message, response_tools, 2);
    let evidence_sketch = clean_text(
        &synthesis_safe_bounded_sketch_from_evidence(message, response_tools, &primary_units),
        1_200,
    );
    if evidence_sketch.is_empty()
        || response_looks_like_retrieval_recap_substituted_for_answer(&evidence_sketch)
        || workflow_answer_unit_contains_ui_or_source_shell(&evidence_sketch)
        || workflow_answer_unit_looks_like_source_title_fragment(&evidence_sketch)
        || workflow_answer_unit_is_process_or_metadata_fact(&evidence_sketch)
    {
        return String::new();
    }
    let mut parts = vec![workflow_finish_visible_sentence(&evidence_sketch)];
    if reject_reason
        .to_ascii_lowercase()
        .contains("missing_coverage_lanes=")
    {
        let coverage_note = fallback_user_visible_coverage_note(response_tools);
        if !coverage_note.is_empty() {
            parts.push(workflow_finish_visible_sentence(&coverage_note));
        }
    }
    clean_text(&parts.join(" "), 1_600)
}

fn fallback_evidence_snippet_sentence_from_tools(
    message: &str,
    response_tools: &[Value],
    reject_reason: &str,
) -> String {
    let goal_terms = workflow_answer_unit_goal_terms(message);
    let comparison_intent = synthesis_message_is_comparison_intent(message);
    let required_entity_lanes = if comparison_intent {
        hard_required_entity_lanes_for_tools(response_tools, 8)
    } else {
        Vec::new()
    };
    let mut try_candidate = |raw: &str, parts: &mut Vec<String>| {
        let cleaned = clean_text(&first_sentence(raw, 240), 300);
        if cleaned.is_empty()
            || workflow_answer_unit_is_process_or_metadata_fact(&cleaned)
            || workflow_answer_unit_contains_ui_or_source_shell(&cleaned)
            || workflow_answer_unit_looks_like_source_title_fragment(&cleaned)
            || workflow_answer_unit_looks_like_datestamped_headline_shell(&cleaned)
            || workflow_response_sentence_is_gap_or_status_preface(&cleaned)
            || workflow_answer_unit_goal_overlap_count(&cleaned, &goal_terms) == 0
        {
            return;
        }
        if comparison_intent
            && !required_entity_lanes.is_empty()
            && text_matches_required_entity_lanes(&cleaned, &required_entity_lanes).is_empty()
        {
            return;
        }
        let finished = workflow_finish_visible_sentence(&cleaned);
        if !finished.is_empty() && !parts.iter().any(|existing| existing == &finished) {
            parts.push(finished);
        }
    };
    let mut parts = Vec::<String>::new();
    for tool in response_tools.iter().take(4) {
        for key in [
            "evidence_pack",
            "evidence_refs",
            "evidence_pack_candidates",
            "search_results",
            "provider_results",
        ] {
            for row in tool_hidden_array(tool, key).into_iter().take(6) {
                for field in [
                    "relevant_extract",
                    "support_snippet",
                    "snippet",
                    "content",
                    "summary",
                ] {
                    let raw = clean_text(row.get(field).and_then(Value::as_str).unwrap_or(""), 320);
                    if raw.is_empty() {
                        continue;
                    }
                    try_candidate(&raw, &mut parts);
                    if parts.len() >= 2 {
                        break;
                    }
                }
                if parts.len() >= 2 {
                    break;
                }
            }
            if parts.len() >= 2 {
                break;
            }
        }
        if parts.is_empty() {
            for snippet in response_tool_evidence_snippets_for_user(tool, 4) {
                try_candidate(&snippet, &mut parts);
                if parts.len() >= 2 {
                    break;
                }
            }
        }
        if parts.len() >= 2 {
            break;
        }
    }
    if parts.is_empty() {
        return String::new();
    }
    if reject_reason
        .to_ascii_lowercase()
        .contains("missing_coverage_lanes=")
    {
        let coverage_note = fallback_user_visible_coverage_note(response_tools);
        if !coverage_note.is_empty() {
            parts.push(workflow_finish_visible_sentence(&coverage_note));
        }
    }
    clean_text(&parts.join(" "), 1_400)
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
