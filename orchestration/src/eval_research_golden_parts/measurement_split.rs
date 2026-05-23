use super::*;

pub(super) fn measurement_split_report(
    rows: &[Value],
    gate_rates: &[Value],
    gate_transition_rates: &[Value],
    live: bool,
    workflow_gate_pass_min: f64,
    research_success_rate: f64,
    research_success_min: f64,
    safety_ok: bool,
) -> Value {
    let total_cases = rows.len() as u64;
    let hard_failure_cases = rows
        .iter()
        .filter(|row| str_at(row, &["failure_classification"], "") == "hard")
        .count() as u64;
    let soft_failure_cases = rows
        .iter()
        .filter(|row| str_at(row, &["failure_classification"], "") == "soft")
        .count() as u64;
    let transport_failure_cases = rows
        .iter()
        .filter(|row| str_at(row, &["failure_classification"], "") == "transport")
        .count() as u64;
    let pass_cases = rows
        .iter()
        .filter(|row| bool_at(row, &["pass"], false))
        .count() as u64;
    let non_transport_cases = total_cases.saturating_sub(transport_failure_cases);
    let transport_adjusted_pass_cases = rows
        .iter()
        .filter(|row| {
            bool_at(row, &["pass"], false)
                && str_at(row, &["failure_classification"], "") != "transport"
        })
        .count() as u64;
    let transport_adjusted_research_success_rate =
        ratio(transport_adjusted_pass_cases, non_transport_cases);
    let workflow_path_ok = gate_rates
        .iter()
        .all(|row| row.get("ok").and_then(Value::as_bool).unwrap_or(false));
    let transition_path_ok = gate_transition_rates
        .iter()
        .all(|row| f64_at(row, &["pass_rate"], 0.0) >= workflow_gate_pass_min);
    let tool_execution_rate = checkpoint_rate(gate_transition_rates, "5a_tool_execution_recorded");
    let raw_provider_rate =
        checkpoint_rate(gate_transition_rates, "5b_raw_provider_result_present");
    let packaged_result_rate =
        checkpoint_rate(gate_transition_rates, "5c_packaged_tool_result_present");
    let evidence_rate = checkpoint_rate(gate_transition_rates, "5d_evidence_refs_extracted");
    let evidence_context_rate =
        checkpoint_rate(gate_transition_rates, "5e_agent_received_evidence_context");
    let synthesis_rate = checkpoint_rate(
        gate_transition_rates,
        "6a_synthesis_uses_evidence_or_low_evidence_fallback",
    );
    let hard_rows = failure_rows_for_classification(rows, "hard");
    let soft_rows = failure_rows_for_classification(rows, "soft");
    let retrieval_soft_cases = rows
        .iter()
        .filter(|row| {
            str_at(row, &["failure_classification"], "") == "soft"
                && case_has_retrieval_quality_signal(row)
        })
        .count() as u64;
    let retrieval_quality_counts = retrieval_quality_status_counts(rows);
    let usable_retrieval_quality_cases =
        retrieval_quality_counts.get("usable").copied().unwrap_or(0);
    let low_evidence_or_degraded_cases = rows
        .iter()
        .filter(|row| {
            matches!(
                str_at(row, &["retrieval_quality", "status"], "").as_str(),
                "low_signal"
                    | "no_results"
                    | "provider_degraded"
                    | "no_evidence"
                    | "raw_provider_absent"
            )
        })
        .count() as u64;
    let excellent_blocked_by_retrieval_quality = rows
        .iter()
        .filter(|row| {
            if str_at(row, &["failure_classification"], "") == "transport" {
                return false;
            }
            row.get("excellent_blockers")
                .and_then(Value::as_array)
                .map(|blockers| {
                    blockers
                        .iter()
                        .filter_map(Value::as_str)
                        .any(|blocker| blocker.starts_with("retrieval_quality:"))
                })
                .unwrap_or(false)
                || !bool_at(
                    row,
                    &[
                        "excellent_diagnostics",
                        "subgates",
                        "excellent_2_citable_evidence_available",
                    ],
                    true,
                )
        })
        .count() as u64;
    let query_metadata_eligible_cases = rows
        .iter()
        .filter(|row| {
            bool_at(
                row,
                &[
                    "query_metadata_diagnostics",
                    "eligible_web_retrieval_request",
                ],
                false,
            )
        })
        .count() as u64;
    let batch_query_metadata_eligible_cases = rows
        .iter()
        .filter(|row| {
            bool_at(
                row,
                &["query_metadata_diagnostics", "eligible_batch_query_request"],
                false,
            )
        })
        .count() as u64;
    let query_metadata_present_cases = rows
        .iter()
        .filter(|row| {
            bool_at(
                row,
                &["query_metadata_diagnostics", "metadata_present"],
                false,
            )
        })
        .count() as u64;
    let rich_query_pack_or_marker_cases = rows
        .iter()
        .filter(|row| {
            bool_at(
                row,
                &[
                    "query_metadata_diagnostics",
                    "rich_query_pack_or_narrow_marker",
                ],
                false,
            )
        })
        .count() as u64;
    let citation_ready_cases = rows
        .iter()
        .filter(|row| u64_at(row, &["citation_behavior", "evidence_count"], 0) > 0)
        .count() as u64;
    let citation_signal_cases = rows
        .iter()
        .filter(|row| bool_at(row, &["citation_behavior", "citation_signal"], false))
        .count() as u64;
    let synthesis_ignored_citable_evidence_cases = rows
        .iter()
        .filter(|row| {
            bool_at(
                row,
                &["citation_behavior", "synthesis_ignored_citable_evidence"],
                false,
            )
        })
        .count() as u64;
    let generic_response_contract_pass_cases = rows
        .iter()
        .filter(|row| {
            bool_at(
                row,
                &[
                    "response_grading_layers",
                    "generic_response_contract",
                    "pass",
                ],
                false,
            )
        })
        .count() as u64;
    let tool_backed_evidence_contract_pass_cases = rows
        .iter()
        .filter(|row| {
            bool_at(
                row,
                &[
                    "response_grading_layers",
                    "tool_backed_evidence_contract",
                    "pass",
                ],
                false,
            )
        })
        .count() as u64;
    let workflow_specific_rubric_pass_cases = rows
        .iter()
        .filter(|row| {
            bool_at(
                row,
                &[
                    "response_grading_layers",
                    "workflow_specific_rubric",
                    "pass",
                ],
                false,
            )
        })
        .count() as u64;
    let soft_quality_smoke_pass_cases = rows
        .iter()
        .filter(|row| bool_at(row, &["soft_quality_smoke", "pass"], false))
        .count() as u64;
    let soft_quality_smoke_flagged_cases =
        total_cases.saturating_sub(soft_quality_smoke_pass_cases);
    let answer_unit_alignment_evaluated_cases = rows
        .iter()
        .filter(|row| bool_at(row, &["answer_unit_evidence_alignment", "evaluated"], false))
        .count() as u64;
    let answer_unit_alignment_pass_cases = rows
        .iter()
        .filter(|row| bool_at(row, &["answer_unit_evidence_alignment", "pass"], false))
        .count() as u64;
    let answer_unit_alignment_evaluated_pass_cases = rows
        .iter()
        .filter(|row| {
            bool_at(row, &["answer_unit_evidence_alignment", "evaluated"], false)
                && bool_at(row, &["answer_unit_evidence_alignment", "pass"], false)
        })
        .count() as u64;
    let answer_unit_alignment_flagged_cases = rows
        .iter()
        .filter(|row| {
            bool_at(row, &["answer_unit_evidence_alignment", "evaluated"], false)
                && !bool_at(row, &["answer_unit_evidence_alignment", "pass"], true)
        })
        .count() as u64;
    let answer_unit_alignment_support_rate_average = if answer_unit_alignment_evaluated_cases == 0 {
        0.0
    } else {
        rows.iter()
            .filter(|row| bool_at(row, &["answer_unit_evidence_alignment", "evaluated"], false))
            .map(|row| {
                f64_at(
                    row,
                    &["answer_unit_evidence_alignment", "term_support_rate"],
                    0.0,
                )
            })
            .sum::<f64>()
            / answer_unit_alignment_evaluated_cases as f64
    };
    let mut answer_unit_alignment_blockers = BTreeMap::<String, u64>::new();
    for row in rows {
        for blocker in row
            .pointer("/answer_unit_evidence_alignment/blockers")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            *answer_unit_alignment_blockers
                .entry(blocker.to_string())
                .or_insert(0) += 1;
        }
    }
    let top_answer_unit_alignment_blocker = answer_unit_alignment_blockers
        .iter()
        .max_by_key(|(_, count)| **count)
        .map(|(blocker, count)| {
            json!({
                "name": blocker,
                "count": count
            })
        })
        .unwrap_or_else(|| {
            json!({
                "name": "none",
                "count": 0
            })
        });
    let answer_unit_usefulness_evaluated_cases = rows
        .iter()
        .filter(|row| bool_at(row, &["answer_unit_usefulness", "evaluated"], false))
        .count() as u64;
    let answer_unit_usefulness_pass_cases = rows
        .iter()
        .filter(|row| bool_at(row, &["answer_unit_usefulness", "pass"], false))
        .count() as u64;
    let answer_unit_usefulness_flagged_cases = rows
        .iter()
        .filter(|row| {
            bool_at(row, &["answer_unit_usefulness", "evaluated"], false)
                && !bool_at(row, &["answer_unit_usefulness", "pass"], true)
        })
        .count() as u64;
    let mut answer_unit_usefulness_blockers = BTreeMap::<String, u64>::new();
    for row in rows {
        for blocker in row
            .pointer("/answer_unit_usefulness/blockers")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            *answer_unit_usefulness_blockers
                .entry(blocker.to_string())
                .or_insert(0) += 1;
        }
    }
    let top_answer_unit_usefulness_blocker = answer_unit_usefulness_blockers
        .iter()
        .max_by_key(|(_, count)| **count)
        .map(|(blocker, count)| {
            json!({
                "name": blocker,
                "count": count
            })
        })
        .unwrap_or_else(|| {
            json!({
                "name": "none",
                "count": 0
            })
        });
    let mut soft_quality_smoke_blockers = BTreeMap::<String, u64>::new();
    for row in rows {
        for blocker in row
            .pointer("/soft_quality_smoke/blockers")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            *soft_quality_smoke_blockers
                .entry(blocker.to_string())
                .or_insert(0) += 1;
        }
    }
    let top_soft_quality_smoke_blocker = soft_quality_smoke_blockers
        .iter()
        .max_by_key(|(_, count)| **count)
        .map(|(blocker, count)| {
            json!({
                "name": blocker,
                "count": count
            })
        })
        .unwrap_or_else(|| {
            json!({
                "name": "none",
                "count": 0
            })
        });
    let query_satisfaction_total = rows
        .iter()
        .map(|row| u64_at(row, &["query_satisfaction", "score"], 0))
        .sum::<u64>();
    let query_satisfaction_cases = rows
        .iter()
        .filter(|row| u64_at(row, &["query_satisfaction", "score"], 0) >= 7)
        .count() as u64;
    let excellent_quality = excellent_quality_report(rows);
    let upstream_failure_localization = upstream_failure_localization_report(rows);
    let retrieval_status = if !live {
        "not_live"
    } else if transport_failure_cases > 0 {
        "transport_failures_present"
    } else if tool_execution_rate < workflow_gate_pass_min || hard_failure_cases > 0 {
        "blocked_by_upstream_path"
    } else if raw_provider_rate < workflow_gate_pass_min
        || packaged_result_rate < workflow_gate_pass_min
        || evidence_rate < workflow_gate_pass_min
        || evidence_context_rate < workflow_gate_pass_min
    {
        "degraded_pipeline"
    } else if retrieval_soft_cases > 0 {
        "noisy_retrieval_or_coverage"
    } else {
        "healthy"
    };
    json!({
        "schema_version": 1,
        "purpose": "split deterministic workflow health from live retrieval variance and end-to-end research quality",
        "deterministic_workflow_path": {
            "ok": workflow_path_ok && transition_path_ok && safety_ok && hard_failure_cases == 0,
            "workflow_gate_path_ok": workflow_path_ok,
            "transition_path_ok": transition_path_ok,
            "safety_ok": safety_ok,
            "hard_failure_cases": hard_failure_cases,
            "transport_failure_cases": transport_failure_cases,
            "min_rate": workflow_gate_pass_min,
            "note": if live {
                "computed from deterministic gates over live payloads; transport failures are reported separately because they do not expose a workflow payload to grade"
            } else {
                "computed from recorded responses; suitable for deterministic replay stability"
            }
        },
        "live_retrieval_health": {
            "status": retrieval_status,
            "live": live,
            "tool_execution_rate": tool_execution_rate,
            "raw_provider_result_rate": raw_provider_rate,
            "packaged_result_rate": packaged_result_rate,
            "evidence_extraction_rate": evidence_rate,
            "evidence_context_rate": evidence_context_rate,
            "retrieval_quality_counts": retrieval_quality_counts,
            "usable_retrieval_quality_cases": usable_retrieval_quality_cases,
            "low_evidence_or_degraded_cases": low_evidence_or_degraded_cases,
            "excellent_blocked_by_retrieval_quality": excellent_blocked_by_retrieval_quality,
            "soft_retrieval_or_coverage_cases": retrieval_soft_cases,
            "transport_failure_cases": transport_failure_cases,
            "note": "this lane measures evidence availability and coverage; it should move with provider/data quality and cache state"
        },
        "query_metadata_planning": {
            "eligible_web_retrieval_requests": query_metadata_eligible_cases,
            "eligible_batch_query_requests": batch_query_metadata_eligible_cases,
            "metadata_present_cases": query_metadata_present_cases,
            "rich_query_pack_or_narrow_marker_cases": rich_query_pack_or_marker_cases,
            "metadata_present_rate": ratio(query_metadata_present_cases, query_metadata_eligible_cases),
            "rich_query_pack_or_narrow_marker_rate": ratio(rich_query_pack_or_marker_cases, query_metadata_eligible_cases),
            "note": "measures whether live web-retrieval requests exercised the CD-declared query metadata primitive instead of silently falling back to minimal query/source/aperture"
        },
        "answer_quality": {
            "citation_ready_cases": citation_ready_cases,
            "citation_signal_cases": citation_signal_cases,
            "citation_signal_rate": ratio(citation_signal_cases, citation_ready_cases),
            "synthesis_ignored_citable_evidence_cases": synthesis_ignored_citable_evidence_cases,
            "query_satisfaction_cases": query_satisfaction_cases,
            "query_satisfaction_rate": ratio(query_satisfaction_cases, total_cases),
            "query_satisfaction_average": ratio(query_satisfaction_total, total_cases),
            "answer_unit_alignment_evaluated_cases": answer_unit_alignment_evaluated_cases,
            "answer_unit_alignment_flagged_cases": answer_unit_alignment_flagged_cases,
            "answer_unit_usefulness_evaluated_cases": answer_unit_usefulness_evaluated_cases,
            "answer_unit_usefulness_flagged_cases": answer_unit_usefulness_flagged_cases,
            "note": "measures whether the final answer satisfied the original query and exposed compact citation/source signal; retrieval failures remain counted separately in live_retrieval_health"
        },
        "answer_unit_evidence_alignment": {
            "evaluated_cases": answer_unit_alignment_evaluated_cases,
            "pass_cases": answer_unit_alignment_pass_cases,
            "pass_rate": ratio(answer_unit_alignment_pass_cases, total_cases),
            "evaluated_pass_cases": answer_unit_alignment_evaluated_pass_cases,
            "evaluated_pass_rate": ratio(answer_unit_alignment_evaluated_pass_cases, answer_unit_alignment_evaluated_cases),
            "flagged_cases": answer_unit_alignment_flagged_cases,
            "flagged_rate": ratio(answer_unit_alignment_flagged_cases, total_cases),
            "average_term_support_rate": answer_unit_alignment_support_rate_average,
            "top_blocker": top_answer_unit_alignment_blocker,
            "blocker_counts": answer_unit_alignment_blockers,
            "note": "Soft generic evidence-alignment lane. It asks whether concrete answer units in the final response can be traced to retrieved evidence/citation artifacts, without assuming any domain or query shape."
        },
        "answer_unit_usefulness": {
            "evaluated_cases": answer_unit_usefulness_evaluated_cases,
            "pass_cases": answer_unit_usefulness_pass_cases,
            "pass_rate": ratio(answer_unit_usefulness_pass_cases, total_cases),
            "flagged_cases": answer_unit_usefulness_flagged_cases,
            "flagged_rate": ratio(answer_unit_usefulness_flagged_cases, total_cases),
            "top_blocker": top_answer_unit_usefulness_blocker,
            "blocker_counts": answer_unit_usefulness_blockers,
            "note": "Soft generic prompt-usefulness lane. It asks whether evidenced answer units directly answer the user's requested semantic object instead of merely surfacing source metadata or administrative facts."
        },
        "response_grading_layers": {
            "generic_response_contract_pass_cases": generic_response_contract_pass_cases,
            "generic_response_contract_pass_rate": ratio(generic_response_contract_pass_cases, total_cases),
            "tool_backed_evidence_contract_pass_cases": tool_backed_evidence_contract_pass_cases,
            "tool_backed_evidence_contract_pass_rate": ratio(tool_backed_evidence_contract_pass_cases, total_cases),
            "workflow_specific_rubric_pass_cases": workflow_specific_rubric_pass_cases,
            "workflow_specific_rubric_pass_rate": ratio(workflow_specific_rubric_pass_cases, total_cases),
            "note": "Separates general answer quality, evidence-use discipline, and the research-specific rubric so the grader can stay format-flexible while still measuring workflow-specific usefulness."
        },
        "soft_quality_smoke": {
            "pass_cases": soft_quality_smoke_pass_cases,
            "pass_rate": ratio(soft_quality_smoke_pass_cases, total_cases),
            "flagged_cases": soft_quality_smoke_flagged_cases,
            "flagged_rate": ratio(soft_quality_smoke_flagged_cases, total_cases),
            "top_blocker": top_soft_quality_smoke_blocker,
            "blocker_counts": soft_quality_smoke_blockers,
            "note": "A non-authoritative UX smoke lane that flags obviously bad answers for manual review even when structural metrics look healthy."
        },
        "upstream_failure_localization": upstream_failure_localization,
        "excellent_quality": excellent_quality,
        "end_to_end_golden": {
            "ok": research_success_rate >= research_success_min,
            "mode": if live { "live_noisy_single_run" } else { "recorded_replay" },
            "passed_cases": pass_cases,
            "total_cases": total_cases,
            "research_success_rate": research_success_rate,
            "raw_live_research_success_rate": research_success_rate,
            "transport_adjusted_passed_cases": transport_adjusted_pass_cases,
            "transport_adjusted_cases": non_transport_cases,
            "transport_adjusted_research_success_rate": transport_adjusted_research_success_rate,
            "transport_adjusted_ok": transport_adjusted_research_success_rate >= research_success_min,
            "research_success_min": research_success_min,
            "synthesis_gate_rate": synthesis_rate,
            "note": if live {
                "treat one-run movement as noisy unless deterministic gates or hard failures move with it"
            } else {
                "recorded replay should be stable and is the better signal for workflow contract regressions"
            }
        },
        "failure_classification": {
            "hard_failure_cases": hard_failure_cases,
            "soft_failure_cases": soft_failure_cases,
            "transport_failure_cases": transport_failure_cases,
            "hard_failures": hard_rows,
            "soft_failures": soft_rows,
            "transport_failures": failure_rows_for_classification(rows, "transport")
        }
    })
}
