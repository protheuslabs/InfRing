fn web_operator_case_metrics(
    payload: &Value,
    request_input: Option<&Value>,
    retrieval_quality: &Value,
    query_metadata_diagnostics: &Value,
    first_failed_gate: &str,
    retrieval_status: &str,
    candidate_count: u64,
    evidence_count: u64,
    content_rich_candidate_count: u64,
    claim_hint_count: u64,
    usable_evidence: bool,
    access_blocked_or_throttled: bool,
    access_blocker: &Value,
    provider_not_empty_or_degraded: bool,
    evidence_context_to_synthesis: bool,
    evidence_quality: &Value,
) -> Value {
    let primary_bottleneck = web_failure_boundary(first_failed_gate);
    let layer_bottleneck = web_failure_layer(first_failed_gate);
    let materialized_candidate_count =
        u64_at(retrieval_quality, &["materialized_candidate_count"], 0);
    let query_lane_count = u64_at(query_metadata_diagnostics, &["query_lane_count"], 0);
    let followup_query_count = u64_at(query_metadata_diagnostics, &["followup_query_count"], 0);
    let keyword_count = u64_at(query_metadata_diagnostics, &["keyword_count"], 0);
    let alias_count = u64_at(query_metadata_diagnostics, &["alias_count"], 0);
    let negative_term_count = u64_at(query_metadata_diagnostics, &["negative_term_count"], 0);
    let required_entity_count = u64_at(
        query_metadata_diagnostics,
        &["required_coverage_entities_count"],
        0,
    );
    let required_facet_count = u64_at(
        query_metadata_diagnostics,
        &["required_coverage_facets_count"],
        0,
    );
    let source_lane_count = declared_source_preference_count(request_input);
    let unique_source_domains = unique_source_domain_count(payload);
    let unique_evidence_domains = unique_evidence_domain_count(payload);
    let source_class_count = unique_source_class_count(payload);
    let official_or_primary_source_count = official_or_primary_source_count(payload);
    let relevant_evidence_count = u64_at(
        retrieval_quality,
        &["prompt_relevance", "relevant_evidence_count"],
        0,
    );
    let materialization_failure_report = retrieval_quality
        .get("materialization_failure_report")
        .cloned()
        .unwrap_or(Value::Null);
    let top_materialization_failure_reason = materialization_failure_report
        .pointer("/top_reason/reason")
        .and_then(Value::as_str)
        .unwrap_or("none");
    let topic_relevant_evidence = retrieval_quality
        .pointer("/prompt_relevance/topic_relevant_evidence")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let direct_claim_contract_present = bool_at(
        retrieval_quality,
        &["classification_inputs", "direct_contract_present"],
        false,
    );
    let direct_evidence_claim_count = u64_at(
        retrieval_quality,
        &["classification_inputs", "direct_evidence_claim_count"],
        0,
    );
    json!({
        "schema_version": 1,
        "readout": web_operator_case_readout(primary_bottleneck, retrieval_status),
        "primary_bottleneck": primary_bottleneck,
        "layer_bottleneck": layer_bottleneck,
        "next_action": web_operator_next_action(primary_bottleneck),
        "query_planning": {
            "query_lane_count": query_lane_count,
            "followup_query_count": followup_query_count,
            "multi_query_present": query_lane_count > 1,
            "keyword_count": keyword_count,
            "alias_count": alias_count,
            "negative_term_count": negative_term_count,
            "required_entity_count": required_entity_count,
            "required_facet_count": required_facet_count,
            "declared_source_lane_count": source_lane_count,
            "metadata_present": bool_at(query_metadata_diagnostics, &["metadata_present"], false),
            "rich_query_pack_or_narrow_marker": bool_at(
                query_metadata_diagnostics,
                &["rich_query_pack_or_narrow_marker"],
                false
            )
        },
        "candidate_supply": {
            "raw_candidate_count": candidate_count,
            "provider_status": retrieval_status,
            "provider_not_empty_or_degraded": provider_not_empty_or_degraded,
            "unique_source_domains": unique_source_domains,
            "unique_evidence_domains": unique_evidence_domains,
            "source_class_count": source_class_count,
            "official_or_primary_source_count": official_or_primary_source_count,
            "relevant_evidence_count": relevant_evidence_count,
            "topic_relevant_evidence": topic_relevant_evidence,
            "relevant_evidence_per_candidate_rate": ratio(relevant_evidence_count, candidate_count)
        },
        "packaging": {
            "evidence_count": evidence_count,
            "evidence_per_candidate_rate": ratio(evidence_count, candidate_count)
        },
        "materialization": {
            "materialized_candidate_count": materialized_candidate_count,
            "content_rich_candidate_count": content_rich_candidate_count,
            "content_rich_per_candidate_rate": ratio(content_rich_candidate_count, candidate_count),
            "top_failure_reason": top_materialization_failure_reason,
            "failure_report": materialization_failure_report
        },
        "claim_extraction": {
            "claim_hint_count": claim_hint_count,
            "direct_claim_contract_present": direct_claim_contract_present,
            "direct_evidence_claim_count": direct_evidence_claim_count,
            "claim_hints_per_evidence_rate": ratio(claim_hint_count, evidence_count)
        },
        "evidence_quality": {
            "source_quality_ready": bool_at(evidence_quality, &["source_quality_ready"], false),
            "source_authority_sensitive": bool_at(
                evidence_quality,
                &["source_authority_sensitive"],
                false
            ),
            "source_authority_ready": bool_at(
                evidence_quality,
                &["source_authority_ready"],
                true
            ),
            "authority_grade_source_domain_count": u64_at(
                evidence_quality,
                &["authority_grade_source_domain_count"],
                0
            ),
            "claim_quality_ready": bool_at(evidence_quality, &["claim_quality_ready"], false),
            "handoff_claim_quality_ready": bool_at(
                evidence_quality,
                &["handoff_claim_quality_ready"],
                false
            ),
            "citation_renderability_ready": bool_at(
                evidence_quality,
                &["citation_renderability_ready"],
                false
            ),
            "citation_titles_clean": bool_at(
                evidence_quality,
                &["citation_titles_clean"],
                true
            ),
            "malformed_citation_title_count": u64_at(
                evidence_quality,
                &["malformed_citation_title_count"],
                0
            ),
            "answerability_ready": bool_at(evidence_quality, &["answerability_ready"], false),
            "evidence_packet_contract_ready": bool_at(
                evidence_quality,
                &["evidence_packet_contract_ready"],
                false
            ),
            "evidence_packet_ready_item_count": u64_at(
                evidence_quality,
                &["evidence_packet_contract", "ready_item_count"],
                0
            ),
            "evidence_packet_ready_rate": f64_at(
                evidence_quality,
                &["evidence_packet_contract", "ready_rate"],
                0.0
            ),
            "evidence_packet_missing_fields": evidence_quality
                .pointer("/evidence_packet_contract/missing_fields")
                .cloned()
                .unwrap_or_else(|| json!([])),
            "clean_evidence_rate": f64_at(evidence_quality, &["clean_evidence_rate"], 0.0),
            "concrete_claim_rate": f64_at(evidence_quality, &["concrete_claim_rate"], 0.0),
            "citation_ready_claim_rate": f64_at(
                evidence_quality,
                &["citation_ready_claim_rate"],
                0.0
            ),
            "handoff_concrete_claim_rate": f64_at(
                evidence_quality,
                &["handoff_concrete_claim_rate"],
                0.0
            ),
            "handoff_low_quality_claim_rate": f64_at(
                evidence_quality,
                &["handoff_low_quality_claim_rate"],
                0.0
            ),
            "handoff_citation_ready_claim_rate": f64_at(
                evidence_quality,
                &["handoff_citation_ready_claim_rate"],
                0.0
            ),
            "evidence_item_count": u64_at(evidence_quality, &["evidence_item_count"], 0),
            "low_quality_evidence_item_count": u64_at(
                evidence_quality,
                &["low_quality_evidence_item_count"],
                0
            ),
            "claim_count": u64_at(evidence_quality, &["claim_count"], 0),
            "concrete_claim_count": u64_at(evidence_quality, &["concrete_claim_count"], 0),
            "citation_ready_claim_count": u64_at(
                evidence_quality,
                &["citation_ready_claim_count"],
                0
            ),
            "handoff_claim_count": u64_at(evidence_quality, &["handoff_claim_count"], 0),
            "handoff_concrete_claim_count": u64_at(
                evidence_quality,
                &["handoff_concrete_claim_count"],
                0
            ),
            "handoff_low_quality_claim_count": u64_at(
                evidence_quality,
                &["handoff_low_quality_claim_count"],
                0
            ),
            "handoff_citation_ready_claim_count": u64_at(
                evidence_quality,
                &["handoff_citation_ready_claim_count"],
                0
            )
        },
        "usable_evidence": {
            "observed": usable_evidence,
            "case_rate": if usable_evidence { 1.0 } else { 0.0 }
        },
        "access": {
            "blocked_or_throttled": access_blocked_or_throttled,
            "kind": access_blocker
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("none"),
            "classes": access_blocker
                .get("classes")
                .cloned()
                .unwrap_or_else(|| json!({})),
            "signals": access_blocker
                .get("signals")
                .cloned()
                .unwrap_or_else(|| json!([]))
        },
        "synthesis_handoff": {
            "observed": evidence_context_to_synthesis
        }
    })
}
