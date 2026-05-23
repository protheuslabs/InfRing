pub(super) fn web_retrieval_gate_diagnostics(
    payload: &Value,
    retrieval_quality: &Value,
    query_metadata_diagnostics: &Value,
    transition_diagnostics: &Value,
) -> Value {
    let request = web_pending_request(payload);
    let request_input = request.and_then(request_input_object);
    let tool_attempted = has_tool_execution(payload);
    let tool_transport_failure = payload_is_transport_failure(payload);
    let tool_transport_completed = tool_attempted && !tool_transport_failure;
    let candidate_count = u64_at(retrieval_quality, &["candidate_count"], 0);
    let evidence_count = u64_at(retrieval_quality, &["evidence_count"], 0);
    let materialized_candidate_count =
        u64_at(retrieval_quality, &["materialized_candidate_count"], 0);
    let content_rich_candidate_count =
        u64_at(retrieval_quality, &["content_rich_candidate_count"], 0);
    let claim_hint_count = u64_at(retrieval_quality, &["claim_hint_count"], 0);
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
    let usable_evidence = bool_at(retrieval_quality, &["usable_evidence"], false);
    let retrieval_status = str_at(retrieval_quality, &["status"], "unknown");
    let request_shape_present = request_input
        .map(input_has_query_or_locator)
        .unwrap_or(tool_attempted);
    let query_metadata_present =
        bool_at(
            query_metadata_diagnostics,
            &["rich_query_pack_or_narrow_marker"],
            false,
        ) || bool_at(query_metadata_diagnostics, &["metadata_present"], false);
    let raw_candidates_present = candidate_count > 0;
    let packaged_evidence_present = evidence_count > 0;
    let content_rich_candidates_present =
        content_rich_candidate_count > 0 && materialized_candidate_count > 0;
    let claim_extraction_present = if direct_claim_contract_present {
        direct_evidence_claim_count > 0
    } else {
        claim_hint_count > 0
    };
    let provider_not_empty_or_degraded = !matches!(
        retrieval_status.as_str(),
        "not_attempted"
            | "no_results"
            | "provider_degraded"
            | "conflicting_provider_state"
            | "raw_provider_absent"
            | "no_evidence"
    );
    let evidence_context_to_synthesis = tool_attempted
        && checkpoint_passed(transition_diagnostics, "5e_agent_received_evidence_context");
    let access_blocker = web_access_blocker_diagnostics(payload, retrieval_quality);
    let access_blocked_or_throttled = bool_at(&access_blocker, &["detected"], false);
    let access_blocker_kind = access_blocker
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or("none")
        .to_string();
    let rate_limited = bool_at(&access_blocker, &["classes", "rate_limit_or_quota"], false);
    let anti_bot_challenge = bool_at(&access_blocker, &["classes", "anti_bot_challenge"], false);
    let permission_or_auth = bool_at(&access_blocker, &["classes", "permission_or_auth"], false);
    let access_denied = bool_at(
        &access_blocker,
        &["classes", "access_denied_or_forbidden"],
        false,
    );
    let access_provider_config_missing = bool_at(
        &access_blocker,
        &["classes", "provider_configuration_missing"],
        false,
    );
    let browser_materialization_recovery =
        browser_materialization_recovery_diagnostics(payload, retrieval_quality);
    let browser_materialization_failed =
        bool_at(&browser_materialization_recovery, &["failed"], false);
    let materialization_top_reason = str_at(
        retrieval_quality,
        &["materialization_failure_report", "top_reason", "reason"],
        "none",
    );
    let recovered_usable_retrieval = usable_evidence
        && packaged_evidence_present
        && content_rich_candidates_present
        && claim_extraction_present
        && provider_not_empty_or_degraded;
    let provider_supply = web_provider_supply_diagnostics(payload, retrieval_quality);
    let provider_config_missing = access_provider_config_missing
        || bool_at(&provider_supply, &["missing_configuration_detected"], false);
    let provider_config_usable = bool_at(&provider_supply, &["configuration_usable"], true);
    let provider_circuit_open_detected =
        bool_at(&provider_supply, &["circuit_open_detected"], false);
    let provider_surface_degraded = bool_at(&provider_supply, &["tool_surface_degraded"], false);
    let provider_raw_rows_available =
        u64_at(&provider_supply, &["raw_row_count"], 0) > 0 || raw_candidates_present;
    let provider_candidates_survive_filtering =
        u64_at(&provider_supply, &["candidate_row_count"], 0) > 0 || candidate_count > 0;
    let retrieval_continued_past_access = provider_raw_rows_available
        && provider_candidates_survive_filtering
        && packaged_evidence_present;
    let retrieval_continued_past_browser_materialization =
        retrieval_continued_past_access && content_rich_candidates_present;
    let provider_config_missing_hard = provider_config_missing && !provider_config_usable;
    let evidence_quality = web_evidence_quality_diagnostics(payload, retrieval_quality);
    let source_quality_ready = bool_at(&evidence_quality, &["source_quality_ready"], false);
    let claim_quality_ready = bool_at(&evidence_quality, &["claim_quality_ready"], false);
    let citation_renderability_ready =
        bool_at(&evidence_quality, &["citation_renderability_ready"], false);
    let evidence_packet_contract_ready =
        bool_at(&evidence_quality, &["evidence_packet_contract_ready"], false);
    let answerability_ready = bool_at(&evidence_quality, &["answerability_ready"], false);
    let rate_limited_hard =
        rate_limited && !recovered_usable_retrieval && !retrieval_continued_past_access;
    let anti_bot_challenge_hard =
        anti_bot_challenge && !recovered_usable_retrieval && !retrieval_continued_past_access;
    let access_blocked_or_throttled_hard = access_blocked_or_throttled
        && !recovered_usable_retrieval
        && !retrieval_continued_past_access;
    let browser_materialization_failed_hard = browser_materialization_failed
        && !usable_evidence
        && !retrieval_continued_past_browser_materialization
        && (access_blocked_or_throttled
            || materialization_top_reason == "browser_materialization_failed");
    let provider_circuits_closed =
        !provider_circuit_open_detected || retrieval_continued_past_access;
    let provider_surface_ready = !provider_surface_degraded
        || (retrieval_continued_past_access && content_rich_candidates_present);
    let blocker_recovery_lane_visible =
        bool_at(
            &browser_materialization_recovery,
            &["recommended_when_policy_allows"],
            false,
        ) || bool_at(&browser_materialization_recovery, &["attempted"], false)
            || bool_at(
                &browser_materialization_recovery,
                &["capability_declared"],
                false,
            );

    let mut gates = Vec::<Value>::new();
    include!("011_request_access_gates.rs");
    include!("012_provider_supply_gates.rs");
    include!("013_evidence_quality_gates.rs");
    let first_failed_gate = gates
        .iter()
        .find(|row| row.get("status").and_then(Value::as_str) == Some("fail"))
        .and_then(|row| row.get("gate").and_then(Value::as_str))
        .unwrap_or("")
        .to_string();
    let operator_metrics = web_operator_case_metrics(
        payload,
        request_input,
        retrieval_quality,
        query_metadata_diagnostics,
        &first_failed_gate,
        retrieval_status.as_str(),
        candidate_count,
        evidence_count,
        content_rich_candidate_count,
        claim_hint_count,
        usable_evidence,
        access_blocked_or_throttled,
        &access_blocker,
        provider_not_empty_or_degraded,
        evidence_context_to_synthesis,
        &evidence_quality,
    );
    json!({
        "schema_version": 1,
        "purpose": "diagnose the web retrieval/tooling path below the research workflow gates",
        "first_failed_gate": if first_failed_gate.is_empty() {
            Value::Null
        } else {
            Value::String(first_failed_gate.clone())
        },
        "inferred_failure_boundary": web_failure_boundary(&first_failed_gate),
        "request_tool_key": request
            .map(|row| {
                let tool = str_at(row, &["selected_tool_key"], "");
                if tool.is_empty() {
                    str_at(row, &["tool_key"], "")
                } else {
                    tool
                }
            })
            .unwrap_or_default(),
        "retrieval_status": retrieval_status,
        "candidate_count": candidate_count,
        "evidence_count": evidence_count,
        "content_rich_candidate_count": content_rich_candidate_count,
        "claim_hint_count": claim_hint_count,
        "direct_claim_contract_present": direct_claim_contract_present,
        "direct_evidence_claim_count": direct_evidence_claim_count,
        "usable_evidence": usable_evidence,
        "access_blocker": access_blocker,
        "browser_materialization_recovery": browser_materialization_recovery,
        "provider_supply": provider_supply,
        "evidence_quality": evidence_quality,
        "web_blocker_classification": access_blocker_kind,
        "operator_metrics": operator_metrics,
        "gates": gates
    })
}
