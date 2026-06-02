pub(super) fn web_retrieval_measurement_report(
    rows: &[Value],
    gate_rates: &[Value],
    gate_metrics: &[Value],
) -> Value {
    let mut first_failure_counts = BTreeMap::<String, u64>::new();
    let mut materialization_failure_reason_counts = BTreeMap::<String, u64>::new();
    let mut access_blocker_counts = BTreeMap::<String, u64>::new();
    let mut access_blocker_class_counts = BTreeMap::<String, u64>::new();
    let mut access_blocker_signal_counts = BTreeMap::<String, u64>::new();
    let mut browser_materialization_recovery_counts = BTreeMap::<String, u64>::new();
    let measured_rows = web_tooling_measured_rows(rows);
    let measured_cases = measured_rows.len() as u64;
    let transport_excluded_cases = rows
        .iter()
        .filter(|row| bool_at(row, &["transport_failure"], false))
        .count() as u64;
    let post_tool_context_excluded_cases = rows
        .iter()
        .filter(|row| {
            web_tooling_measurement_exclusion_reason_row(row)
                == Some("post_tool_context_not_seeded")
        })
        .count() as u64;
    let measurement_excluded_cases = rows.len() as u64 - measured_cases;
    let mut candidate_count_total = 0_u64;
    let mut evidence_count_total = 0_u64;
    let mut content_rich_candidate_count_total = 0_u64;
    let mut claim_hint_count_total = 0_u64;
    let mut usable_evidence_cases = 0_u64;
    let mut provider_starved_cases = 0_u64;
    let mut access_blocked_cases = 0_u64;
    let mut synthesis_handoff_cases = 0_u64;
    let mut query_lane_count_total = 0_u64;
    let mut followup_query_count_total = 0_u64;
    let mut keyword_count_total = 0_u64;
    let mut required_entity_count_total = 0_u64;
    let mut required_facet_count_total = 0_u64;
    let mut multi_query_cases = 0_u64;
    let mut unique_source_domains_total = 0_u64;
    let mut unique_evidence_domains_total = 0_u64;
    let mut source_class_count_total = 0_u64;
    let mut official_or_primary_cases = 0_u64;
    let mut relevant_evidence_count_total = 0_u64;
    let mut topic_relevant_cases = 0_u64;
    let mut source_quality_ready_cases = 0_u64;
    let mut claim_quality_ready_cases = 0_u64;
    let mut citation_renderability_ready_cases = 0_u64;
    let mut answerability_ready_cases = 0_u64;
    let mut evidence_packet_contract_ready_cases = 0_u64;
    let mut low_quality_evidence_item_count_total = 0_u64;
    let mut evidence_item_count_total = 0_u64;
    let mut concrete_claim_count_total = 0_u64;
    let mut citation_ready_claim_count_total = 0_u64;
    let mut quality_claim_count_total = 0_u64;
    let mut handoff_claim_count_total = 0_u64;
    let mut handoff_concrete_claim_count_total = 0_u64;
    let mut handoff_low_quality_claim_count_total = 0_u64;
    let mut handoff_citation_ready_claim_count_total = 0_u64;
    for row in measured_rows {
        let gate = row
            .pointer("/web_tool_gate_diagnostics/first_failed_gate")
            .and_then(Value::as_str)
            .unwrap_or("none");
        *first_failure_counts.entry(gate.to_string()).or_insert(0) += 1;
        let materialization_reason = str_at(
            row,
            &[
                "web_tool_gate_diagnostics",
                "operator_metrics",
                "materialization",
                "top_failure_reason",
            ],
            "none",
        );
        if !materialization_reason.is_empty() && materialization_reason != "none" {
            *materialization_failure_reason_counts
                .entry(materialization_reason)
                .or_insert(0) += 1;
        }
        candidate_count_total = candidate_count_total.saturating_add(u64_at(
            row,
            &["web_tool_gate_diagnostics", "candidate_count"],
            0,
        ));
        evidence_count_total = evidence_count_total.saturating_add(u64_at(
            row,
            &["web_tool_gate_diagnostics", "evidence_count"],
            0,
        ));
        content_rich_candidate_count_total =
            content_rich_candidate_count_total.saturating_add(u64_at(
                row,
                &["web_tool_gate_diagnostics", "content_rich_candidate_count"],
                0,
            ));
        claim_hint_count_total = claim_hint_count_total.saturating_add(u64_at(
            row,
            &["web_tool_gate_diagnostics", "claim_hint_count"],
            0,
        ));
        query_lane_count_total = query_lane_count_total.saturating_add(u64_at(
            row,
            &[
                "web_tool_gate_diagnostics",
                "operator_metrics",
                "query_planning",
                "query_lane_count",
            ],
            0,
        ));
        followup_query_count_total = followup_query_count_total.saturating_add(u64_at(
            row,
            &[
                "web_tool_gate_diagnostics",
                "operator_metrics",
                "query_planning",
                "followup_query_count",
            ],
            0,
        ));
        keyword_count_total = keyword_count_total.saturating_add(u64_at(
            row,
            &[
                "web_tool_gate_diagnostics",
                "operator_metrics",
                "query_planning",
                "keyword_count",
            ],
            0,
        ));
        required_entity_count_total = required_entity_count_total.saturating_add(u64_at(
            row,
            &[
                "web_tool_gate_diagnostics",
                "operator_metrics",
                "query_planning",
                "required_entity_count",
            ],
            0,
        ));
        required_facet_count_total = required_facet_count_total.saturating_add(u64_at(
            row,
            &[
                "web_tool_gate_diagnostics",
                "operator_metrics",
                "query_planning",
                "required_facet_count",
            ],
            0,
        ));
        if bool_at(
            row,
            &[
                "web_tool_gate_diagnostics",
                "operator_metrics",
                "query_planning",
                "multi_query_present",
            ],
            false,
        ) {
            multi_query_cases = multi_query_cases.saturating_add(1);
        }
        unique_source_domains_total = unique_source_domains_total.saturating_add(u64_at(
            row,
            &[
                "web_tool_gate_diagnostics",
                "operator_metrics",
                "candidate_supply",
                "unique_source_domains",
            ],
            0,
        ));
        unique_evidence_domains_total = unique_evidence_domains_total.saturating_add(u64_at(
            row,
            &[
                "web_tool_gate_diagnostics",
                "operator_metrics",
                "candidate_supply",
                "unique_evidence_domains",
            ],
            0,
        ));
        source_class_count_total = source_class_count_total.saturating_add(u64_at(
            row,
            &[
                "web_tool_gate_diagnostics",
                "operator_metrics",
                "candidate_supply",
                "source_class_count",
            ],
            0,
        ));
        relevant_evidence_count_total = relevant_evidence_count_total.saturating_add(u64_at(
            row,
            &[
                "web_tool_gate_diagnostics",
                "operator_metrics",
                "candidate_supply",
                "relevant_evidence_count",
            ],
            0,
        ));
        if bool_at(
            row,
            &[
                "web_tool_gate_diagnostics",
                "operator_metrics",
                "candidate_supply",
                "topic_relevant_evidence",
            ],
            false,
        ) {
            topic_relevant_cases = topic_relevant_cases.saturating_add(1);
        }
        if bool_at(
            row,
            &[
                "web_tool_gate_diagnostics",
                "operator_metrics",
                "evidence_quality",
                "source_quality_ready",
            ],
            false,
        ) {
            source_quality_ready_cases = source_quality_ready_cases.saturating_add(1);
        }
        if bool_at(
            row,
            &[
                "web_tool_gate_diagnostics",
                "operator_metrics",
                "evidence_quality",
                "claim_quality_ready",
            ],
            false,
        ) {
            claim_quality_ready_cases = claim_quality_ready_cases.saturating_add(1);
        }
        if bool_at(
            row,
            &[
                "web_tool_gate_diagnostics",
                "operator_metrics",
                "evidence_quality",
                "citation_renderability_ready",
            ],
            false,
        ) {
            citation_renderability_ready_cases =
                citation_renderability_ready_cases.saturating_add(1);
        }
        if bool_at(
            row,
            &[
                "web_tool_gate_diagnostics",
                "operator_metrics",
                "evidence_quality",
                "answerability_ready",
            ],
            false,
        ) {
            answerability_ready_cases = answerability_ready_cases.saturating_add(1);
        }
        if bool_at(
            row,
            &[
                "web_tool_gate_diagnostics",
                "operator_metrics",
                "evidence_quality",
                "evidence_packet_contract_ready",
            ],
            false,
        ) {
            evidence_packet_contract_ready_cases =
                evidence_packet_contract_ready_cases.saturating_add(1);
        }
        evidence_item_count_total = evidence_item_count_total.saturating_add(u64_at(
            row,
            &[
                "web_tool_gate_diagnostics",
                "operator_metrics",
                "evidence_quality",
                "evidence_item_count",
            ],
            0,
        ));
        low_quality_evidence_item_count_total = low_quality_evidence_item_count_total
            .saturating_add(u64_at(
                row,
                &[
                    "web_tool_gate_diagnostics",
                    "operator_metrics",
                    "evidence_quality",
                    "low_quality_evidence_item_count",
                ],
                0,
            ));
        quality_claim_count_total = quality_claim_count_total.saturating_add(u64_at(
            row,
            &[
                "web_tool_gate_diagnostics",
                "operator_metrics",
                "evidence_quality",
                "claim_count",
            ],
            0,
        ));
        concrete_claim_count_total = concrete_claim_count_total.saturating_add(u64_at(
            row,
            &[
                "web_tool_gate_diagnostics",
                "operator_metrics",
                "evidence_quality",
                "concrete_claim_count",
            ],
            0,
        ));
        citation_ready_claim_count_total = citation_ready_claim_count_total.saturating_add(u64_at(
            row,
            &[
                "web_tool_gate_diagnostics",
                "operator_metrics",
                "evidence_quality",
                "citation_ready_claim_count",
            ],
            0,
        ));
        handoff_claim_count_total = handoff_claim_count_total.saturating_add(u64_at(
            row,
            &[
                "web_tool_gate_diagnostics",
                "operator_metrics",
                "evidence_quality",
                "handoff_claim_count",
            ],
            0,
        ));
        handoff_concrete_claim_count_total =
            handoff_concrete_claim_count_total.saturating_add(u64_at(
                row,
                &[
                    "web_tool_gate_diagnostics",
                    "operator_metrics",
                    "evidence_quality",
                    "handoff_concrete_claim_count",
                ],
                0,
            ));
        handoff_low_quality_claim_count_total =
            handoff_low_quality_claim_count_total.saturating_add(u64_at(
                row,
                &[
                    "web_tool_gate_diagnostics",
                    "operator_metrics",
                    "evidence_quality",
                    "handoff_low_quality_claim_count",
                ],
                0,
            ));
        handoff_citation_ready_claim_count_total =
            handoff_citation_ready_claim_count_total.saturating_add(u64_at(
                row,
                &[
                    "web_tool_gate_diagnostics",
                    "operator_metrics",
                    "evidence_quality",
                    "handoff_citation_ready_claim_count",
                ],
                0,
            ));
        if u64_at(
            row,
            &[
                "web_tool_gate_diagnostics",
                "operator_metrics",
                "candidate_supply",
                "official_or_primary_source_count",
            ],
            0,
        ) > 0
        {
            official_or_primary_cases = official_or_primary_cases.saturating_add(1);
        }
        if bool_at(
            row,
            &["web_tool_gate_diagnostics", "usable_evidence"],
            false,
        ) {
            usable_evidence_cases = usable_evidence_cases.saturating_add(1);
        }
        if str_at(
            row,
            &[
                "web_tool_gate_diagnostics",
                "operator_metrics",
                "primary_bottleneck",
            ],
            "",
        ) == "provider_empty_or_degraded"
            || str_at(row, &["web_tool_gate_diagnostics", "retrieval_status"], "")
                .contains("provider")
        {
            provider_starved_cases = provider_starved_cases.saturating_add(1);
        }
        let blocker = row
            .pointer("/web_tool_gate_diagnostics/access_blocker/kind")
            .and_then(Value::as_str)
            .unwrap_or("none");
        *access_blocker_counts
            .entry(blocker.to_string())
            .or_insert(0) += 1;
        if let Some(classes) = row
            .pointer("/web_tool_gate_diagnostics/access_blocker/classes")
            .and_then(Value::as_object)
        {
            for (class, active) in classes {
                if active.as_bool().unwrap_or(false) {
                    *access_blocker_class_counts
                        .entry(class.to_string())
                        .or_insert(0) += 1;
                }
            }
        }
        if let Some(signals) = row
            .pointer("/web_tool_gate_diagnostics/access_blocker/signals")
            .and_then(Value::as_array)
        {
            for signal in signals.iter().filter_map(Value::as_str) {
                *access_blocker_signal_counts
                    .entry(signal.to_string())
                    .or_insert(0) += 1;
            }
        }
        if blocker != "none" {
            access_blocked_cases = access_blocked_cases.saturating_add(1);
        }
        let recovery = if bool_at(
            row,
            &[
                "web_tool_gate_diagnostics",
                "browser_materialization_recovery",
                "attempted",
            ],
            false,
        ) {
            "attempted"
        } else if bool_at(
            row,
            &[
                "web_tool_gate_diagnostics",
                "browser_materialization_recovery",
                "recommended_when_policy_allows",
            ],
            false,
        ) {
            "recommended_when_policy_allows"
        } else if bool_at(
            row,
            &[
                "web_tool_gate_diagnostics",
                "browser_materialization_recovery",
                "capability_declared",
            ],
            false,
        ) {
            "capability_declared"
        } else {
            "not_visible"
        };
        *browser_materialization_recovery_counts
            .entry(recovery.to_string())
            .or_insert(0) += 1;
        if bool_at(
            row,
            &[
                "web_tool_gate_diagnostics",
                "operator_metrics",
                "synthesis_handoff",
                "observed",
            ],
            false,
        ) {
            synthesis_handoff_cases = synthesis_handoff_cases.saturating_add(1);
        }
    }
    let weakest_gates = gate_rates
        .iter()
        .filter(|row| f64_at(row, &["pass_rate"], 1.0) < 0.95)
        .cloned()
        .collect::<Vec<_>>();
    let operator_metrics = web_operator_aggregate_metrics(
        measured_cases,
        transport_excluded_cases,
        candidate_count_total,
        evidence_count_total,
        content_rich_candidate_count_total,
        claim_hint_count_total,
        query_lane_count_total,
        followup_query_count_total,
        keyword_count_total,
        required_entity_count_total,
        required_facet_count_total,
        multi_query_cases,
        unique_source_domains_total,
        unique_evidence_domains_total,
        source_class_count_total,
        official_or_primary_cases,
        relevant_evidence_count_total,
        topic_relevant_cases,
        usable_evidence_cases,
        provider_starved_cases,
        access_blocked_cases,
        synthesis_handoff_cases,
        source_quality_ready_cases,
        claim_quality_ready_cases,
        citation_renderability_ready_cases,
        answerability_ready_cases,
        evidence_packet_contract_ready_cases,
        evidence_item_count_total,
        low_quality_evidence_item_count_total,
        quality_claim_count_total,
        concrete_claim_count_total,
        citation_ready_claim_count_total,
        handoff_claim_count_total,
        handoff_concrete_claim_count_total,
        handoff_low_quality_claim_count_total,
        handoff_citation_ready_claim_count_total,
        &first_failure_counts,
        gate_metrics,
    );
    json!({
        "schema_version": 1,
        "purpose": "make web retrieval less opaque by measuring request planning, provider return, packaging, quality, and synthesis handoff separately",
        "measured_cases": measured_cases,
        "measurement_excluded_cases": measurement_excluded_cases,
        "transport_excluded_cases": transport_excluded_cases,
        "post_tool_context_excluded_cases": post_tool_context_excluded_cases,
        "first_failure_counts": first_failure_counts,
        "materialization_failure_reason_counts": materialization_failure_reason_counts,
        "top_materialization_failure_reason": top_count_row(&materialization_failure_reason_counts),
        "access_blocker_counts": access_blocker_counts,
        "access_blocker_class_counts": access_blocker_class_counts,
        "access_blocker_signal_counts": access_blocker_signal_counts,
        "browser_materialization_recovery_counts": browser_materialization_recovery_counts,
        "operator_metrics": operator_metrics,
        "gate_metrics": gate_metrics,
        "weakest_gates": weakest_gates,
        "note": "This diagnostic lane is intentionally separate from research workflow gates; it identifies web-tooling quality bottlenecks without changing workflow pass/fail semantics."
    })
}
