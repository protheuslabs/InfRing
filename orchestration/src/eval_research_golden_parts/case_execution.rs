use super::*;

pub(super) struct ResearchGoldenCaseRunInput<'a> {
    pub(super) cases: &'a [Value],
    pub(super) limit: usize,
    pub(super) live: bool,
    pub(super) confirm_pending_tool: bool,
    pub(super) setup_failures: &'a [String],
    pub(super) isolate_tool_cache: bool,
    pub(super) fresh_agent_per_case: bool,
    pub(super) cleanup_fresh_agents: bool,
    pub(super) base_url: &'a str,
    pub(super) agent_id: &'a str,
    pub(super) fresh_agent_model: Option<&'a str>,
    pub(super) timeout_seconds: u64,
    pub(super) timeout_recovery_seconds: u64,
    pub(super) responses_by_case: &'a BTreeMap<String, Value>,
    pub(super) pass_score: u64,
    pub(super) excellent_score: u64,
    pub(super) progress_path: &'a str,
    pub(super) partial_out_path: &'a str,
    pub(super) live_agent_bootstrap: &'a Value,
}

pub(super) struct ResearchGoldenRunState {
    pub(super) rows: Vec<Value>,
    pub(super) failure_events: Vec<Value>,
    pub(super) gate_pass_counts: BTreeMap<String, u64>,
    pub(super) gate_total_counts: BTreeMap<String, u64>,
    pub(super) transition_pass_counts: BTreeMap<String, u64>,
    pub(super) transition_total_counts: BTreeMap<String, u64>,
    pub(super) web_gate_pass_counts: BTreeMap<String, u64>,
    pub(super) web_gate_total_counts: BTreeMap<String, u64>,
    pub(super) dimension_totals: BTreeMap<String, u64>,
    pub(super) passed_cases: u64,
    pub(super) excellent_cases: u64,
    pub(super) total_score: u64,
    pub(super) empty_responses: u64,
    pub(super) raw_tool_leaks: u64,
    pub(super) tool_choice_final_responses: u64,
    pub(super) unsupported_claims: u64,
    pub(super) transport_failures: u64,
}

pub(super) fn run_research_golden_cases(
    input: ResearchGoldenCaseRunInput<'_>,
) -> ResearchGoldenRunState {
    let ResearchGoldenCaseRunInput {
        cases,
        limit,
        live,
        confirm_pending_tool,
        setup_failures,
        isolate_tool_cache,
        fresh_agent_per_case,
        cleanup_fresh_agents,
        base_url,
        agent_id,
        fresh_agent_model,
        timeout_seconds,
        timeout_recovery_seconds,
        responses_by_case,
        pass_score,
        excellent_score,
        progress_path,
        partial_out_path,
        live_agent_bootstrap,
    } = input;
    let mut rows = Vec::new();
    let mut failure_events = Vec::new();
    let mut gate_pass_counts: BTreeMap<String, u64> = BTreeMap::new();
    let mut gate_total_counts: BTreeMap<String, u64> = BTreeMap::new();
    let mut transition_pass_counts: BTreeMap<String, u64> = BTreeMap::new();
    let mut transition_total_counts: BTreeMap<String, u64> = BTreeMap::new();
    let mut web_gate_pass_counts: BTreeMap<String, u64> = BTreeMap::new();
    let mut web_gate_total_counts: BTreeMap<String, u64> = BTreeMap::new();
    let mut dimension_totals: BTreeMap<String, u64> = BTreeMap::new();
    let mut passed_cases = 0_u64;
    let mut excellent_cases = 0_u64;
    let mut total_score = 0_u64;
    let mut empty_responses = 0_u64;
    let mut raw_tool_leaks = 0_u64;
    let mut tool_choice_final_responses = 0_u64;
    let mut unsupported_claims = 0_u64;
    let mut transport_failures = 0_u64;
    let total_planned_cases = cases.iter().take(limit).count() as u64;
    let run_started_at = now_iso_like();
    write_research_golden_progress(
        &progress_path,
        json!({
            "event": "run_start",
            "generated_at": run_started_at,
            "mode": if live { "live_dashboard" } else { "offline_responses" },
            "cases_planned": total_planned_cases,
            "timeout_seconds": timeout_seconds,
            "timeout_recovery_seconds": timeout_recovery_seconds,
            "fresh_agent_per_case": fresh_agent_per_case,
            "live_agent_bootstrap": live_agent_bootstrap
        }),
    );
    write_partial_research_golden_report(
        &partial_out_path,
        "running",
        live,
        total_planned_cases,
        &rows,
        &setup_failures,
        None,
    );

    for (case_index, case) in cases.iter().take(limit).enumerate() {
        let case_started = Instant::now();
        let case_id = str_at(case, &["id"], "unknown_case");
        let prompt = str_at(case, &["prompt"], "");
        eprintln!(
            "research_golden: case {}/{} start {}",
            case_index + 1,
            total_planned_cases,
            case_id
        );
        write_research_golden_progress(
            &progress_path,
            json!({
                "event": "case_start",
                "case_index": case_index + 1,
                "cases_planned": total_planned_cases,
                "case_id": case_id,
                "generated_at": now_iso_like()
            }),
        );
        let mut case_agent_id = agent_id.to_string();
        let mut case_setup_failures = setup_failures.to_vec();
        let mut cache_isolation = json!({
            "ok": true,
            "type": "research_golden_cache_isolation",
            "applied": false
        });
        if live && setup_failures.is_empty() && isolate_tool_cache {
            cache_isolation = isolate_batch_query_cache_for_eval();
            if !cache_isolation
                .get("ok")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                case_setup_failures.push("batch_query_cache_isolation_failed".to_string());
            }
        }
        if live && setup_failures.is_empty() && fresh_agent_per_case {
            match create_live_agent(
                &base_url,
                case_id.as_str(),
                agent_id,
                fresh_agent_model.as_deref(),
                timeout_seconds,
            ) {
                Some(created_agent_id) => case_agent_id = created_agent_id,
                None => case_setup_failures.push("fresh_agent_create_failed".to_string()),
            }
        }
        let source_payload = responses_by_case
            .get(&case_id)
            .cloned()
            .unwrap_or_else(|| json!({}));
        let post_tool_setup_prompt = live
            .then(|| post_tool_web_tooling_setup_prompt(case))
            .flatten();
        let mut post_tool_setup_payload_used = false;
        let initial_payload = if live && case_setup_failures.is_empty() {
            if let Some(setup_prompt) = post_tool_setup_prompt.as_deref() {
                post_tool_setup_payload_used = true;
                post_agent_message(
                    &base_url,
                    &case_agent_id,
                    &json!({ "message": setup_prompt }),
                    timeout_seconds,
                    timeout_recovery_seconds,
                )
            } else {
                post_agent_message(
                    &base_url,
                    &case_agent_id,
                    &json!({ "message": prompt }),
                    timeout_seconds,
                    timeout_recovery_seconds,
                )
            }
        } else {
            response_sequence_payload(&source_payload, 0).unwrap_or(source_payload.clone())
        };
        let initial_pending_tool_confirmation =
            payload_has_pending_tool_confirmation(&initial_payload);
        let mut payload =
            if post_tool_setup_payload_used && !payload_is_transport_failure(&initial_payload) {
                post_agent_message(
                    &base_url,
                    &case_agent_id,
                    &json!({ "message": prompt }),
                    timeout_seconds,
                    timeout_recovery_seconds,
                )
            } else {
                initial_payload.clone()
            };
        let mut confirmation_payload_used = false;
        let mut confirmation_sent = false;
        let mut confirmation_fixture_used = false;
        if confirm_pending_tool
            && case_setup_failures.is_empty()
            && initial_pending_tool_confirmation
        {
            if live {
                confirmation_sent = true;
                payload = post_agent_message(
                    &base_url,
                    &case_agent_id,
                    &json!({ "message": "confirm" }),
                    timeout_seconds,
                    timeout_recovery_seconds,
                );
                confirmation_payload_used = true;
            } else if let Some(confirmed_payload) = response_sequence_payload(&source_payload, 1) {
                payload = confirmed_payload;
                confirmation_payload_used = true;
                confirmation_fixture_used = true;
            }
        }
        if live && fresh_agent_per_case && cleanup_fresh_agents && case_agent_id != agent_id {
            let _ = delete_live_agent(&base_url, &case_agent_id, timeout_seconds);
        }
        let transition_diagnostics = gate_transition_diagnostics_for_sequence(
            case,
            &initial_payload,
            &payload,
            confirmation_payload_used,
        );
        let transport_timeout_failure = payload_is_transport_failure(&payload);
        let lifecycle_gate_path_complete =
            transition_first_failed_checkpoint(&transition_diagnostics).is_none();
        let grade = grade_case(case, &payload, pass_score, excellent_score);
        let web_tooling_payload = if post_tool_setup_payload_used {
            &initial_payload
        } else {
            &payload
        };
        let web_tooling_retrieval_quality = if post_tool_setup_payload_used {
            grade_case(case, web_tooling_payload, pass_score, excellent_score).retrieval_quality
        } else {
            grade.retrieval_quality.clone()
        };
        let query_metadata_diagnostics = query_metadata_diagnostics(web_tooling_payload);
        let web_tool_gate_diagnostics = web_retrieval_gate_diagnostics(
            web_tooling_payload,
            &web_tooling_retrieval_quality,
            &query_metadata_diagnostics,
            &transition_diagnostics,
        );
        let web_tooling_measurement_eligible = web_tooling_measurement_eligible_case(
            case,
            web_tooling_payload,
            &web_tooling_retrieval_quality,
        );
        let mut excellent_blockers = grade.excellent_blockers.clone();
        let mut excellent_diagnostics = grade.excellent_diagnostics.clone();
        append_web_tooling_excellent_readiness(
            &mut excellent_diagnostics,
            &mut excellent_blockers,
            &web_tool_gate_diagnostics,
            web_tooling_measurement_eligible,
        );
        let mut case_failures = grade.failures.clone();
        if transport_timeout_failure {
            case_failures.push("transport_failure".to_string());
        }
        if let Some(checkpoint) = transition_first_failed_checkpoint(&transition_diagnostics) {
            case_failures.push(format!("research_lifecycle_gate_failed:{checkpoint}"));
        }
        case_failures.sort();
        case_failures.dedup();
        let case_pass =
            grade.pass && lifecycle_gate_path_complete && case_setup_failures.is_empty();
        let case_excellent = grade.excellent
            && excellent_blockers.is_empty()
            && lifecycle_gate_path_complete
            && case_setup_failures.is_empty();
        let failure_classification = case_failure_classification(
            case_pass,
            &case_failures,
            &case_setup_failures,
            &transition_diagnostics,
            grade.empty_response,
            grade.raw_tool_leak,
            grade.tool_choice_final_response,
        );
        let initial_response_text = assistant_text(&initial_payload);
        if transport_timeout_failure {
            transport_failures = transport_failures.saturating_add(1);
        } else {
            record_gate_counts(&grade.gates, &mut gate_total_counts, &mut gate_pass_counts);
            record_checkpoint_counts(
                &transition_diagnostics,
                &mut transition_total_counts,
                &mut transition_pass_counts,
            );
            if web_tooling_measurement_eligible {
                record_web_retrieval_gate_counts(
                    &web_tool_gate_diagnostics,
                    &mut web_gate_total_counts,
                    &mut web_gate_pass_counts,
                );
            }
        }
        let web_tooling_measurement_exclusion = web_tooling_measurement_exclusion_reason_case(
            case,
            web_tooling_payload,
            &web_tooling_retrieval_quality,
        )
        .unwrap_or("none");
        for (dimension, score) in grade.dimension_scores.iter() {
            *dimension_totals.entry(dimension.clone()).or_insert(0) += *score;
        }
        total_score = total_score.saturating_add(grade.score);
        if case_pass {
            passed_cases = passed_cases.saturating_add(1);
        }
        if case_excellent {
            excellent_cases = excellent_cases.saturating_add(1);
        }
        if grade.empty_response {
            empty_responses = empty_responses.saturating_add(1);
        }
        if grade.raw_tool_leak {
            raw_tool_leaks = raw_tool_leaks.saturating_add(1);
        }
        if grade.tool_choice_final_response {
            tool_choice_final_responses = tool_choice_final_responses.saturating_add(1);
        }
        if grade.unsupported_claim {
            unsupported_claims = unsupported_claims.saturating_add(1);
        }
        append_failure_events(
            &mut failure_events,
            case_id.as_str(),
            prompt.as_str(),
            case_agent_id.as_str(),
            live,
            &grade.response_text,
            &case_failures,
            &case_setup_failures,
        );
        let mut case_row = json!({
            "case_id": case_id,
            "category": str_at(case, &["category"], "unknown"),
            "tags": string_array_at(case, &["tags"]),
            "prompt_preview": clean_text(&prompt, 320),
            "score": grade.score,
            "score_pass": grade.pass,
            "pass": case_pass,
            "excellent": case_excellent,
            "lifecycle_gate_path_complete": lifecycle_gate_path_complete,
            "agent_id": case_agent_id,
            "gates": grade.gates,
            "dimension_scores": grade.dimension_scores,
            "failures": case_failures,
            "failure_classification": failure_classification,
            "retrieval_quality": grade.retrieval_quality,
            "citation_behavior": grade.citation_behavior,
            "query_satisfaction": grade.query_satisfaction,
            "user_stated_coverage_entities": grade.coverage_entities,
            "excellent_blockers": excellent_blockers,
            "excellent_diagnostics": excellent_diagnostics,
            "transport_failure": transport_timeout_failure,
            "setup_failures": case_setup_failures,
            "response_preview": clean_text(&grade.response_text, 500),
            "response_diagnostics": response_diagnostics(&payload, &grade.response_text),
            "query_metadata_diagnostics": query_metadata_diagnostics,
            "web_tool_gate_diagnostics": web_tool_gate_diagnostics,
            "web_tooling_diagnostic_source": if post_tool_setup_payload_used {
                "post_tool_setup_turn"
            } else {
                "final_turn"
            },
            "web_tooling_retrieval_quality": web_tooling_retrieval_quality,
            "web_tooling_measurement_exclusion": web_tooling_measurement_exclusion,
            "gate_transition_diagnostics": transition_diagnostics,
            "turn_sequence": {
                "confirm_pending_tool": confirm_pending_tool,
                "initial_pending_tool_confirmation": initial_pending_tool_confirmation,
                "confirmation_sent": confirmation_sent,
                "confirmation_fixture_used": confirmation_fixture_used,
                "confirmation_payload_used": confirmation_payload_used,
                "post_tool_setup_payload_used": post_tool_setup_payload_used,
                "cache_isolation": cache_isolation,
                "final_payload_source": if confirmation_payload_used {
                    "confirmation_turn"
                } else if post_tool_setup_payload_used {
                    "post_tool_synthesis_turn"
                } else {
                    "initial_turn"
                },
                "initial_response_diagnostics": response_diagnostics(
                    &initial_payload,
                    &initial_response_text
                ),
                "initial_gate_transition_diagnostics": gate_transition_diagnostics(
                    case,
                    &initial_payload
                )
            },
        });
        if let Some(object) = case_row.as_object_mut() {
            object.insert("prompt".to_string(), Value::String(prompt.clone()));
            object.insert(
                "expected_gate_path".to_string(),
                case.get("expected_gate_path")
                    .cloned()
                    .unwrap_or_else(|| json!({})),
            );
            object.insert(
                "required_entities".to_string(),
                json!(string_array_at(case, &["required_entities"])),
            );
            object.insert(
                "required_facets".to_string(),
                json!(string_array_at(case, &["required_facets"])),
            );
            object.insert(
                "response_full".to_string(),
                Value::String(clean_text(&grade.response_text, 16_000)),
            );
            object.insert(
                "response_grading_layers".to_string(),
                grade.response_grading_layers,
            );
            object.insert("soft_quality_smoke".to_string(), grade.soft_quality_smoke);
            object.insert(
                "answer_unit_evidence_alignment".to_string(),
                grade.answer_unit_evidence_alignment,
            );
            object.insert(
                "citation_artifacts".to_string(),
                citation_artifact_summary(&payload),
            );
        }
        let localization = upstream_failure_localization(&case_row);
        if let Some(object) = case_row.as_object_mut() {
            object.insert("upstream_failure_localization".to_string(), localization);
        }
        rows.push(case_row.clone());
        let case_elapsed_ms = case_started.elapsed().as_millis() as u64;
        eprintln!(
            "research_golden: case {}/{} done {} pass={} excellent={} score={} elapsed_ms={}",
            case_index + 1,
            total_planned_cases,
            case_id,
            case_pass,
            case_excellent,
            grade.score,
            case_elapsed_ms
        );
        write_research_golden_progress(
            &progress_path,
            json!({
                "event": "case_done",
                "case_index": case_index + 1,
                "cases_planned": total_planned_cases,
                "case_id": case_id,
                "generated_at": now_iso_like(),
                "elapsed_ms": case_elapsed_ms,
                "pass": case_pass,
                "excellent": case_excellent,
                "score": grade.score,
                "transport_failure": transport_timeout_failure,
                "failure_classification": failure_classification
            }),
        );
        write_partial_research_golden_report(
            &partial_out_path,
            "running",
            live,
            total_planned_cases,
            &rows,
            &setup_failures,
            Some(&case_row),
        );
    }

    ResearchGoldenRunState {
        rows,
        failure_events,
        gate_pass_counts,
        gate_total_counts,
        transition_pass_counts,
        transition_total_counts,
        web_gate_pass_counts,
        web_gate_total_counts,
        dimension_totals,
        passed_cases,
        excellent_cases,
        total_score,
        empty_responses,
        raw_tool_leaks,
        tool_choice_final_responses,
        unsupported_claims,
        transport_failures,
    }
}

fn append_web_tooling_excellent_readiness(
    excellent_diagnostics: &mut Value,
    excellent_blockers: &mut Vec<String>,
    web_tool_gate_diagnostics: &Value,
    measured_web_tooling_case: bool,
) {
    if !measured_web_tooling_case {
        return;
    }

    let source_quality_ready = bool_at(
        web_tool_gate_diagnostics,
        &["evidence_quality", "source_quality_ready"],
        false,
    );
    let answerability_ready = bool_at(
        web_tool_gate_diagnostics,
        &["evidence_quality", "answerability_ready"],
        false,
    );
    let evidence_packet_ready = bool_at(
        web_tool_gate_diagnostics,
        &["evidence_quality", "evidence_packet_contract_ready"],
        false,
    );

    if let Some(subgates) = excellent_diagnostics
        .get_mut("subgates")
        .and_then(Value::as_object_mut)
    {
        subgates.insert(
            "excellent_12_web_tooling_source_quality_ready".to_string(),
            json!(source_quality_ready),
        );
        subgates.insert(
            "excellent_13_web_tooling_answerability_ready".to_string(),
            json!(answerability_ready),
        );
        subgates.insert(
            "excellent_14_web_tooling_evidence_packet_ready".to_string(),
            json!(evidence_packet_ready),
        );
    }

    for blocker in [
        (!source_quality_ready).then_some("web_tooling_source_quality_not_ready"),
        (!answerability_ready).then_some("web_tooling_answerability_not_ready"),
        (!evidence_packet_ready).then_some("web_tooling_evidence_packet_not_ready"),
    ]
    .into_iter()
    .flatten()
    {
        if !excellent_blockers
            .iter()
            .any(|existing| existing == blocker)
        {
            excellent_blockers.push(blocker.to_string());
        }
    }

    if let Some(object) = excellent_diagnostics.as_object_mut() {
        object.insert("blockers".to_string(), json!(excellent_blockers.clone()));
        object.insert(
            "top_blocker".to_string(),
            Value::String(
                excellent_blockers
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "none".to_string()),
            ),
        );
        object.insert(
            "web_tooling_readiness".to_string(),
            json!({
                "measured": true,
                "source_quality_ready": source_quality_ready,
                "answerability_ready": answerability_ready,
                "evidence_packet_contract_ready": evidence_packet_ready,
                "first_failed_web_gate": web_tool_gate_diagnostics
                    .get("first_failed_gate")
                    .cloned()
                    .unwrap_or(Value::Null),
                "note": "Excellent research answers require answer-ready web evidence when the web tooling lane was measured."
            }),
        );
    }
}
