fn retrieval_provider_quality(payload: &Value, normalized_prompt: &str) -> Value {
    let tool_executed = has_tool_execution(payload);
    let candidate_count = provider_candidate_count(payload).max(provider_explicit_quality_metric(
        payload,
        &[
            "candidate_count",
            "provider_raw_count",
            "provider_result_count",
            "provider_result_dedup_count",
        ],
    ));
    let evidence_count = provider_evidence_count(payload);
    let materialized_candidate_count = provider_materialized_candidate_count(payload);
    let content_rich_candidate_count = provider_content_rich_candidate_count(payload);
    let direct_claim_contract_present = payload.get("evidence_claims").is_some();
    let direct_evidence_claim_count = direct_evidence_claim_count(payload);
    let claim_hint_count = if direct_claim_contract_present {
        direct_evidence_claim_count
    } else {
        provider_claim_hint_count(payload)
    };
    let materialization_failure_report =
        provider_explicit_quality_value(payload, &["materialization_failure_report"]);
    let prompt_relevance = if direct_claim_contract_present {
        evidence_prompt_relevance_from_texts(
            normalized_prompt,
            direct_evidence_claim_texts(payload),
            "Checks prompt relevance against first-class evidence_claims only, so candidate titles or non-citable refs cannot make weak claims look usable.",
            true,
        )
    } else {
        evidence_prompt_relevance(payload, normalized_prompt)
    };
    let topic_relevant_evidence = prompt_relevance
        .get("topic_relevant_evidence")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let relevant_evidence_count = prompt_relevance
        .get("relevant_evidence_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let status_text = tool_status_marker_text(payload);
    let explicit_no_results = contains_any(
        &status_text,
        &[
            "no_results",
            "no results",
            "no usable result",
            "no usable results",
            "zero evidence",
            "zero snippets",
            "zero candidate snippets",
            "empty_feed",
        ],
    );
    let explicit_provider_degraded = contains_any(
        &status_text,
        &[
            "provider degradation",
            "provider degraded",
            "provider_error",
            "provider error",
            "transport_error",
            "execution_error",
            "error",
            "timeout",
            "blocked",
            "anti_bot",
            "anti-bot",
            "proxy_error",
            "failed",
        ],
    );
    let explicit_low_signal = contains_any(
        &status_text,
        &[
            "low_signal",
            "low signal",
            "low-signal",
            "low relevance",
            "low-relevance",
            "weak evidence",
            "limited evidence",
            "limited source coverage",
            "retrieval gap",
            "retrieval miss",
            "irrelevant",
            "off target",
            "off-topic",
        ],
    );
    let direct_quality_flags = direct_tool_quality_flags(payload);
    let direct_contract_present = payload.get("tool_result_quality").is_some()
        || payload.get("evidence_pack_quality").is_some()
        || direct_claim_contract_present;
    let direct_pack_status = str_at(payload, &["evidence_pack_quality", "status"], "");
    let direct_pack_thin = matches!(
        direct_pack_status.as_str(),
        "thin" | "empty" | "low_signal" | "no_results"
    );
    let direct_provider_degraded = direct_quality_flags.iter().any(|flag| {
        matches!(
            flag.as_str(),
            "provider_starved"
                | "provider_timeout"
                | "provider_degraded"
                | "provider_error"
                | "rate_limited"
                | "quota_exhausted"
        )
    });
    let provider_degraded_observed = explicit_provider_degraded || direct_provider_degraded;
    let provider_degradation_blocks_supply = provider_degraded_observed
        && (candidate_count == 0 || evidence_count == 0 || materialized_candidate_count == 0);
    let direct_low_signal = direct_pack_thin
        || direct_quality_flags.iter().any(|flag| {
            matches!(
                flag.as_str(),
                "claim_hints_missing"
                    | "comparison_evidence_insufficient"
                    | "content_rich_evidence_missing"
                    | "low_signal"
                    | "low_relevance"
            )
        })
        || (direct_claim_contract_present && direct_evidence_claim_count == 0);
    let evidence_artifact_conflict =
        explicit_no_results && (candidate_count > 0 || evidence_count > 0);
    let materialized_evidence_available = materialized_candidate_count > 0 && claim_hint_count > 0;
    let status = if !tool_executed {
        "not_attempted"
    } else if provider_degradation_blocks_supply {
        "provider_degraded"
    } else if evidence_artifact_conflict {
        "conflicting_provider_state"
    } else if explicit_no_results {
        "no_results"
    } else if evidence_count == 0 {
        "no_evidence"
    } else if candidate_count == 0 {
        "raw_provider_absent"
    } else if evidence_count > 0 && !topic_relevant_evidence {
        "low_relevance"
    } else if materialized_candidate_count == 0 || claim_hint_count == 0 {
        "low_signal"
    } else if explicit_low_signal || direct_low_signal {
        "low_signal"
    } else {
        "usable"
    };
    let usable_evidence = status == "usable"
        && (!direct_contract_present
            || (direct_evidence_claim_count > 0 && topic_relevant_evidence && !direct_low_signal));
    let allows_excellent = usable_evidence
        && content_rich_candidate_count > 0
        && claim_hint_count > 0
        && relevant_evidence_count >= 2;
    let mut flags = Vec::new();
    if !tool_executed {
        flags.push("tool_not_executed");
    }
    if explicit_no_results {
        flags.push("explicit_no_results_marker");
    }
    if explicit_provider_degraded {
        flags.push("explicit_provider_degraded_marker");
    }
    if explicit_low_signal {
        flags.push("explicit_low_signal_marker");
    }
    if direct_contract_present {
        flags.push("direct_tool_evidence_contract_present");
    }
    if direct_provider_degraded {
        flags.push("direct_tool_provider_degraded_marker");
    }
    if provider_degraded_observed && !provider_degradation_blocks_supply {
        flags.push("provider_degradation_nonblocking");
    }
    if direct_low_signal {
        flags.push("direct_tool_low_signal_marker");
    }
    if direct_claim_contract_present && direct_evidence_claim_count == 0 {
        flags.push("direct_evidence_claims_absent");
    }
    if evidence_artifact_conflict {
        flags.push("evidence_artifact_conflict");
    }
    if evidence_count == 0 {
        flags.push("no_evidence_refs");
    }
    if candidate_count == 0 {
        flags.push("raw_provider_absent");
    }
    if tool_executed && evidence_count > 0 && materialized_candidate_count == 0 {
        flags.push("materialized_evidence_absent");
    }
    if tool_executed && evidence_count > 0 && content_rich_candidate_count == 0 {
        flags.push("content_rich_candidates_absent");
    }
    if tool_executed && evidence_count > 0 && claim_hint_count == 0 {
        flags.push("claim_hints_absent");
    }
    if tool_executed && evidence_count > 0 && !topic_relevant_evidence {
        flags.push("topic_relevance_absent");
    }
    flags.sort_unstable();
    flags.dedup();
    json!({
        "status": status,
        "tool_executed": tool_executed,
        "candidate_count": candidate_count,
        "evidence_count": evidence_count,
        "materialized_candidate_count": materialized_candidate_count,
        "content_rich_candidate_count": content_rich_candidate_count,
        "claim_hint_count": claim_hint_count,
        "materialization_failure_report": materialization_failure_report,
        "materialized_evidence_available": materialized_evidence_available,
        "usable_evidence": usable_evidence,
        "allows_excellent": allows_excellent,
        "quality_flags": flags,
        "prompt_relevance": prompt_relevance,
        "classification_inputs": {
            "explicit_no_results_marker": explicit_no_results,
            "explicit_provider_degraded_marker": explicit_provider_degraded,
            "explicit_low_signal_marker": explicit_low_signal,
            "direct_contract_present": direct_contract_present,
            "direct_evidence_claim_count": direct_evidence_claim_count,
            "provider_degraded_observed": provider_degraded_observed,
            "provider_degradation_blocks_supply": provider_degradation_blocks_supply,
            "direct_provider_degraded_marker": direct_provider_degraded,
            "direct_low_signal_marker": direct_low_signal,
            "evidence_artifact_conflict": evidence_artifact_conflict,
            "materialized_candidate_count": materialized_candidate_count,
            "content_rich_candidate_count": content_rich_candidate_count,
            "claim_hint_count": claim_hint_count,
            "relevant_evidence_count": relevant_evidence_count,
            "topic_relevant_evidence": topic_relevant_evidence,
            "status_marker_source": "structured_tool_status_fields_only"
        },
        "note": "Excellent requires usable retrieval/provider evidence; low-evidence fallbacks may pass but cannot earn excellent."
    })
}
