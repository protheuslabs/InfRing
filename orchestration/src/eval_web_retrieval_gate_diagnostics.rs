use super::eval_research_golden_utils::*;
use serde_json::{json, Value};
use std::collections::BTreeMap;

const WEB_GATE_TARGET_PASS_RATE: f64 = 0.95;

pub(super) fn web_retrieval_gate_diagnostics(
    payload: &Value,
    retrieval_quality: &Value,
    query_metadata_diagnostics: &Value,
    transition_diagnostics: &Value,
) -> Value {
    let request = web_pending_request(payload);
    let request_input = request.and_then(request_input_object);
    let tool_attempted = has_tool_execution(payload);
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

    let gates = vec![
        web_gate(
            "web_1_request_shape_present",
            request.is_some() || tool_attempted,
            request_shape_present,
            if request_shape_present {
                "web request carries a query, query pack, URL, locator, or equivalent executed request shape"
            } else {
                "no query, query pack, URL, locator, or equivalent request shape was visible"
            },
            request_shape_refs(request_input),
        ),
        web_gate(
            "web_2_query_metadata_present",
            request_shape_present,
            query_metadata_present,
            if query_metadata_present {
                "request includes query metadata, expansion/narrowing marker, keywords, or required coverage"
            } else if request_shape_present {
                "request used a minimal query shape without metadata, keywords, expansion marker, or required coverage"
            } else {
                "query metadata cannot be inspected without a visible request shape"
            },
            metadata_refs(query_metadata_diagnostics),
        ),
        web_gate(
            "web_3_tool_attempt_recorded",
            request_shape_present || tool_attempted,
            tool_attempted,
            if tool_attempted {
                "web tool attempt is recorded"
            } else {
                "request shape exists but no web tool attempt is recorded"
            },
            vec![
                "tools".to_string(),
                "response_finalization.tool_completion.tool_attempts".to_string(),
            ],
        ),
        web_gate(
            "web_3b1_provider_quota_not_rate_limited",
            tool_attempted,
            !rate_limited_hard,
            if rate_limited_hard {
                "provider or retrieval lane reported rate-limit, quota, Retry-After, throttling, or HTTP 429 signals"
            } else if rate_limited {
                "rate-limit or quota signal appeared on at least one lane, but retrieval still produced candidates and evidence"
            } else if tool_attempted {
                "no provider rate-limit, quota, Retry-After, throttling, or HTTP 429 signal was detected"
            } else {
                "rate-limit signals cannot be inspected before a tool attempt"
            },
            access_blocker_refs(&access_blocker),
        ),
        web_gate(
            "web_3b2_no_bot_challenge_or_waf",
            tool_attempted,
            !anti_bot_challenge_hard,
            if anti_bot_challenge_hard {
                "tool artifacts contained CAPTCHA, human-verification, Cloudflare, WAF, or bot-wall challenge signals"
            } else if anti_bot_challenge {
                "tool artifacts contained challenge signals on at least one lane, but retrieval still produced usable evidence"
            } else if tool_attempted {
                "no CAPTCHA, human-verification, Cloudflare, WAF, or bot-wall challenge signal was detected"
            } else {
                "bot-challenge signals cannot be inspected before a tool attempt"
            },
            access_blocker_refs(&access_blocker),
        ),
        web_gate(
            "web_3b3_no_permission_or_auth_block",
            tool_attempted,
            !permission_or_auth,
            if permission_or_auth {
                "tool artifacts contained login, auth-required, unauthorized, or HTTP 401 signals"
            } else if tool_attempted {
                "no login, auth-required, unauthorized, or HTTP 401 signal was detected"
            } else {
                "auth/permission signals cannot be inspected before a tool attempt"
            },
            access_blocker_refs(&access_blocker),
        ),
        web_gate(
            "web_3b4_no_access_denied_or_forbidden",
            tool_attempted,
            !access_denied,
            if access_denied {
                "tool artifacts contained access-denied, forbidden, request-blocked, or HTTP 403 signals"
            } else if tool_attempted {
                "no access-denied, forbidden, request-blocked, or HTTP 403 signal was detected"
            } else {
                "access-denied signals cannot be inspected before a tool attempt"
            },
            access_blocker_refs(&access_blocker),
        ),
        web_gate(
            "web_3b5_provider_configuration_available",
            tool_attempted,
            !provider_config_missing_hard,
            if provider_config_missing_hard {
                "tool artifacts indicate provider credentials, provider admission, or required provider configuration is missing"
            } else if provider_config_missing {
                "provider configuration gap was detected on at least one lane, but configured provider supply continued far enough to produce candidates and evidence"
            } else if tool_attempted {
                "no missing provider credential, admission, or configuration signal was detected"
            } else {
                "provider configuration signals cannot be inspected before a tool attempt"
            },
            access_blocker_refs(&access_blocker),
        ),
        web_gate(
            "web_3b_access_not_blocked_or_throttled",
            tool_attempted,
            !access_blocked_or_throttled_hard,
            if access_blocked_or_throttled_hard {
                "tool attempt appears blocked or throttled by an access, rate-limit, CAPTCHA, bot-wall, or similar web-control signal"
            } else if access_blocked_or_throttled {
                "an access blocker signal appeared on at least one lane, but retrieval still produced usable evidence"
            } else if tool_attempted {
                "no access-block, CAPTCHA, bot-wall, or rate-limit signal was detected in the tool artifacts"
            } else {
                "access blockers cannot be inspected before a tool attempt"
            },
            access_blocker_refs(&access_blocker),
        ),
        web_gate(
            "web_3c_blocker_recovery_lane_visible",
            access_blocked_or_throttled_hard,
            !access_blocked_or_throttled_hard || blocker_recovery_lane_visible,
            if !access_blocked_or_throttled_hard {
                "no access blocker was detected, so a browser-materialization recovery lane is not required"
            } else if blocker_recovery_lane_visible {
                "access blocker was detected and the payload exposes browser-materialization recovery capability, recommendation, or attempt metadata"
            } else {
                "access blocker was detected but no browser-materialization recovery lane metadata was visible"
            },
            browser_materialization_recovery
                .get("artifact_refs")
                .and_then(Value::as_array)
                .map(|rows| {
                    rows.iter()
                        .filter_map(Value::as_str)
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                })
                .filter(|rows| !rows.is_empty())
                .unwrap_or_else(|| {
                    vec![
                        "tool_result_quality.browser_materialization".to_string(),
                        "retrieval_broker.provider_attempts".to_string(),
                        "runtime_web_tools_metadata.browser_materialization".to_string(),
                    ]
                }),
        ),
        web_gate(
            "web_3d_browser_materialization_not_failed",
            blocker_recovery_lane_visible,
            !browser_materialization_failed_hard,
            if browser_materialization_failed_hard {
                "browser-materialization was the active blocking recovery lane and reported failure, timeout, navigation failure, or extraction failure"
            } else if browser_materialization_failed {
                "browser-materialization reported a non-blocking failed enrichment attempt, but another materialization failure reason was more upstream"
            } else if blocker_recovery_lane_visible {
                "browser-materialization recovery lane was visible and no recovery failure signal was detected"
            } else {
                "browser-materialization failure cannot be inspected when the recovery lane is not visible"
            },
            browser_materialization_recovery
                .get("artifact_refs")
                .and_then(Value::as_array)
                .map(|rows| {
                    rows.iter()
                        .filter_map(Value::as_str)
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                })
                .filter(|rows| !rows.is_empty())
                .unwrap_or_else(|| {
                    vec![
                        "tool_result_quality.browser_materialization".to_string(),
                        "runtime_web_tools_metadata.browser_materialization".to_string(),
                    ]
                }),
        ),
        web_gate(
            "web_4a_search_provider_configuration_usable",
            tool_attempted,
            provider_config_usable,
            if provider_config_usable {
                "search provider configuration is usable enough for this run"
            } else if tool_attempted {
                "search provider supply is constrained by missing credentials, missing strong provider, or provider admission/configuration state"
            } else {
                "provider configuration cannot be inspected before a tool attempt"
            },
            provider_supply_refs(&provider_supply),
        ),
        web_gate(
            "web_4b_search_provider_circuit_closed",
            tool_attempted,
            provider_circuits_closed,
            if provider_circuits_closed {
                if provider_circuit_open_detected {
                    "a provider circuit-open signal was present, but another provider path produced usable candidates and evidence"
                } else {
                    "no search-provider circuit-open signal was detected"
                }
            } else if tool_attempted {
                "one or more search providers were skipped by an open circuit breaker"
            } else {
                "provider circuit state cannot be inspected before a tool attempt"
            },
            provider_supply_refs(&provider_supply),
        ),
        web_gate(
            "web_4c_search_provider_surface_ready",
            tool_attempted,
            provider_surface_ready,
            if provider_surface_ready {
                if provider_surface_degraded {
                    "search surface reported non-fatal provider degradation, but usable candidates and evidence were produced"
                } else {
                    "search tool surface did not report degraded execution"
                }
            } else if tool_attempted {
                "search tool surface reported degraded execution before usable candidates could be produced"
            } else {
                "provider surface readiness cannot be inspected before a tool attempt"
            },
            provider_supply_refs(&provider_supply),
        ),
        web_gate(
            "web_4d_provider_raw_rows_available",
            tool_attempted,
            provider_raw_rows_available,
            if provider_raw_rows_available {
                "provider attempts produced raw rows before filtering or promotion"
            } else if tool_attempted {
                "provider attempts produced no raw rows"
            } else {
                "raw provider rows cannot be inspected before a tool attempt"
            },
            provider_supply_refs(&provider_supply),
        ),
        web_gate(
            "web_4e_provider_candidates_survive_filtering",
            provider_raw_rows_available || tool_attempted,
            provider_candidates_survive_filtering,
            if provider_candidates_survive_filtering {
                "some provider rows survived filtering into candidate rows"
            } else if provider_raw_rows_available {
                "provider rows existed, but all were filtered, rejected, low-confidence-only, or not promoted into candidates"
            } else {
                "candidate filtering cannot be inspected before raw provider rows"
            },
            provider_supply_refs(&provider_supply),
        ),
        web_gate(
            "web_4_raw_candidates_present",
            tool_attempted,
            raw_candidates_present,
            if raw_candidates_present {
                "provider returned raw candidates, search rows, or equivalent candidate artifacts"
            } else if tool_attempted {
                "tool ran but no raw candidates or provider rows were visible"
            } else {
                "raw candidates cannot be expected before a tool attempt"
            },
            vec![
                "retrieval_quality.candidate_count".to_string(),
                "5b_raw_provider_result_present".to_string(),
            ],
        ),
        web_gate(
            "web_5_packaged_evidence_present",
            raw_candidates_present || tool_attempted,
            packaged_evidence_present,
            if packaged_evidence_present {
                "candidate output was packaged into evidence refs, findings, sources, or equivalent artifacts"
            } else if raw_candidates_present {
                "raw candidates were present but no packaged evidence artifact was visible"
            } else {
                "packaged evidence cannot be expected before raw candidates"
            },
            vec![
                "retrieval_quality.evidence_count".to_string(),
                "5c_packaged_tool_result_present".to_string(),
                "5d_evidence_refs_extracted".to_string(),
            ],
        ),
        web_gate(
            "web_6_provider_not_empty_or_degraded",
            tool_attempted,
            provider_not_empty_or_degraded,
            if provider_not_empty_or_degraded {
                "provider status is not empty, absent, degraded, or no-results"
            } else {
                "provider status indicates no results, degraded transport, contradictory provider state, absent raw output, or no extracted evidence"
            },
            vec![
                "retrieval_quality.status".to_string(),
                "retrieval_quality.quality_flags".to_string(),
            ],
        ),
        web_gate(
            "web_5b_content_rich_candidates_present",
            packaged_evidence_present,
            content_rich_candidates_present,
            if content_rich_candidates_present {
                "packaged evidence includes materialized, content-rich candidate text rather than only thin search rows"
            } else if packaged_evidence_present {
                "packaged evidence exists but no content-rich candidate text was visible"
            } else {
                "content-rich candidates cannot be expected before packaged evidence"
            },
            vec![
                "retrieval_quality.materialized_candidate_count".to_string(),
                "retrieval_quality.content_rich_candidate_count".to_string(),
                "tool_result_quality.content_rich_candidate_count".to_string(),
                "tool_result_quality.materialized_candidate_count".to_string(),
                "evidence_pack_quality.content_rich_item_count".to_string(),
                "evidence_pack_quality.materialized_item_count".to_string(),
            ],
        ),
        web_gate(
            "web_5c_claim_extraction_present",
            packaged_evidence_present,
            claim_extraction_present,
            if claim_extraction_present {
                "packaged evidence includes extracted claim hints or equivalent claim-level facts for synthesis"
            } else if packaged_evidence_present {
                "packaged evidence exists but no claim hints or equivalent claim extraction were visible"
            } else {
                "claim extraction cannot be expected before packaged evidence"
            },
            vec![
                "retrieval_quality.claim_hint_count".to_string(),
                "retrieval_quality.classification_inputs.direct_contract_present".to_string(),
                "retrieval_quality.classification_inputs.direct_evidence_claim_count".to_string(),
                "tool_result_quality.claim_hint_count".to_string(),
                "evidence_pack.claim_hints".to_string(),
                "evidence_claims".to_string(),
            ],
        ),
        web_gate(
            "web_5d_source_quality_ready",
            packaged_evidence_present,
            source_quality_ready,
            if source_quality_ready {
                "packaged evidence contains source-backed, non-dominantly-low-quality material"
            } else if packaged_evidence_present {
                "packaged evidence exists but appears source-thin, low-confidence, boilerplate-heavy, or candidate-only"
            } else {
                "source quality cannot be assessed before packaged evidence exists"
            },
            evidence_quality_refs(&evidence_quality),
        ),
        web_gate(
            "web_5e_claim_quality_ready",
            claim_extraction_present,
            claim_quality_ready,
            if claim_quality_ready {
                "claim extraction produced concrete answer material rather than only titles, source labels, or boilerplate fragments"
            } else if claim_extraction_present {
                "claim extraction exists but the extracted strings look too thin, source-only, title-like, or boilerplate-heavy for synthesis"
            } else {
                "claim quality cannot be assessed before claim extraction exists"
            },
            evidence_quality_refs(&evidence_quality),
        ),
        web_gate(
            "web_5f_citation_renderability_ready",
            claim_extraction_present,
            citation_renderability_ready,
            if citation_renderability_ready {
                "claim-level or evidence-level material has enough source locator/title/domain data to render citations"
            } else if claim_extraction_present {
                "claims exist but do not carry enough source locator, title, or domain data for reliable citation rendering"
            } else {
                "citation renderability cannot be assessed before claim extraction exists"
            },
            evidence_quality_refs(&evidence_quality),
        ),
        web_gate(
            "web_5g_answerability_ready",
            packaged_evidence_present && claim_extraction_present,
            answerability_ready,
            if answerability_ready {
                "evidence has enough clean source material, concrete claims, and citation data for a bounded useful answer"
            } else if packaged_evidence_present && claim_extraction_present {
                "evidence and claims exist, but the package is not yet strong enough for a coherent source-backed answer"
            } else {
                "answerability cannot be assessed before evidence packaging and claim extraction both exist"
            },
            evidence_quality_refs(&evidence_quality),
        ),
        web_gate(
            "web_7_usable_evidence_available",
            packaged_evidence_present || tool_attempted,
            usable_evidence
                && content_rich_candidates_present
                && claim_extraction_present
                && answerability_ready,
            if usable_evidence
                && content_rich_candidates_present
                && claim_extraction_present
                && answerability_ready
            {
                "retrieval quality classifies the packaged, materialized, claim-bearing, citation-ready evidence as usable"
            } else {
                "packaged output exists only as thin, unmaterialized, claim-poor, citation-poor, low-signal/no-results/degraded evidence or no usable evidence was available"
            },
            vec![
                "retrieval_quality.usable_evidence".to_string(),
                "retrieval_quality.status".to_string(),
                "retrieval_quality.materialized_candidate_count".to_string(),
                "retrieval_quality.content_rich_candidate_count".to_string(),
                "retrieval_quality.claim_hint_count".to_string(),
            ],
        ),
        web_gate(
            "web_8_evidence_context_to_synthesis",
            packaged_evidence_present,
            evidence_context_to_synthesis,
            if evidence_context_to_synthesis {
                "evidence context reached the synthesis/finalization boundary"
            } else if packaged_evidence_present {
                "packaged evidence exists but synthesis context marker is absent"
            } else {
                "synthesis evidence context cannot be expected without packaged evidence"
            },
            vec!["5e_agent_received_evidence_context".to_string()],
        ),
    ];
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

pub(super) fn record_web_retrieval_gate_counts(
    diagnostics: &Value,
    total_counts: &mut BTreeMap<String, u64>,
    pass_counts: &mut BTreeMap<String, u64>,
) {
    let Some(gates) = diagnostics.get("gates").and_then(Value::as_array) else {
        return;
    };
    for gate in gates {
        let Some(name) = gate.get("gate").and_then(Value::as_str) else {
            continue;
        };
        *total_counts.entry(name.to_string()).or_insert(0) += 1;
        if gate.get("status").and_then(Value::as_str) == Some("pass") {
            *pass_counts.entry(name.to_string()).or_insert(0) += 1;
        }
    }
}

pub(super) fn web_tooling_measurement_eligible_case(
    case: &Value,
    payload: &Value,
    retrieval_quality: &Value,
) -> bool {
    web_tooling_measurement_exclusion_reason_case(case, payload, retrieval_quality).is_none()
}

pub(super) fn web_tooling_measurement_exclusion_reason_case(
    case: &Value,
    payload: &Value,
    retrieval_quality: &Value,
) -> Option<&'static str> {
    if payload_is_transport_failure(payload) {
        return Some("transport_failure");
    }
    if unseeded_post_tool_synthesis_case(case, payload, retrieval_quality) {
        return Some("post_tool_context_not_seeded");
    }
    None
}

pub(super) fn web_retrieval_gate_rate_rows(
    total_counts: &BTreeMap<String, u64>,
    pass_counts: &BTreeMap<String, u64>,
) -> Vec<Value> {
    total_counts
        .iter()
        .map(|(gate, total)| {
            let passed = *pass_counts.get(gate).unwrap_or(&0);
            json!({
                "gate": gate,
                "passed": passed,
                "total": total,
                "pass_rate": ratio(passed, *total),
                "boundary": web_failure_boundary(gate)
            })
        })
        .collect()
}

pub(super) fn web_retrieval_gate_metric_rows(rows: &[Value], gate_rates: &[Value]) -> Vec<Value> {
    let measured_rows = web_tooling_measured_rows(rows);
    let measured_cases = measured_rows.len() as u64;
    let mut metrics = BTreeMap::<String, WebGateMetric>::new();
    for gate_rate in gate_rates {
        if let Some(gate) = gate_rate.get("gate").and_then(Value::as_str) {
            metrics.entry(gate.to_string()).or_default();
        }
    }

    for row in measured_rows {
        let first_failed_gate = row
            .pointer("/web_tool_gate_diagnostics/first_failed_gate")
            .and_then(Value::as_str)
            .unwrap_or("");
        let Some(gates) = row
            .pointer("/web_tool_gate_diagnostics/gates")
            .and_then(Value::as_array)
        else {
            continue;
        };
        for gate in gates {
            let Some(name) = gate.get("gate").and_then(Value::as_str) else {
                continue;
            };
            let metric = metrics.entry(name.to_string()).or_default();
            let artifact_present = gate
                .get("artifact_present")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let passed = gate.get("status").and_then(Value::as_str) == Some("pass");
            metric.total = metric.total.saturating_add(1);
            if artifact_present {
                metric.artifact_present = metric.artifact_present.saturating_add(1);
            } else {
                metric.artifact_missing = metric.artifact_missing.saturating_add(1);
            }
            if passed {
                metric.passed = metric.passed.saturating_add(1);
            } else {
                metric.failed = metric.failed.saturating_add(1);
                if artifact_present {
                    metric.artifact_present_failures =
                        metric.artifact_present_failures.saturating_add(1);
                } else {
                    metric.artifact_missing_failures =
                        metric.artifact_missing_failures.saturating_add(1);
                }
            }
            if first_failed_gate == name {
                metric.first_failure_count = metric.first_failure_count.saturating_add(1);
            }
        }
    }

    metrics
        .into_iter()
        .map(|(gate, metric)| {
            let pass_rate = ratio(metric.passed, metric.total);
            let fail_rate = ratio(metric.failed, metric.total);
            json!({
                "gate": gate,
                "boundary": web_failure_boundary(&gate),
                "measured_cases": measured_cases,
                "target_pass_rate": WEB_GATE_TARGET_PASS_RATE,
                "ok": pass_rate >= WEB_GATE_TARGET_PASS_RATE,
                "total": metric.total,
                "passed": metric.passed,
                "failed": metric.failed,
                "pass_rate": pass_rate,
                "fail_rate": fail_rate,
                "artifact_present": metric.artifact_present,
                "artifact_missing": metric.artifact_missing,
                "artifact_present_rate": ratio(metric.artifact_present, metric.total),
                "artifact_missing_rate": ratio(metric.artifact_missing, metric.total),
                "artifact_present_failures": metric.artifact_present_failures,
                "artifact_missing_failures": metric.artifact_missing_failures,
                "first_failure_count": metric.first_failure_count,
                "first_failure_rate": ratio(metric.first_failure_count, measured_cases)
            })
        })
        .collect()
}

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
    let mut low_quality_evidence_item_count_total = 0_u64;
    let mut evidence_item_count_total = 0_u64;
    let mut concrete_claim_count_total = 0_u64;
    let mut citation_ready_claim_count_total = 0_u64;
    let mut quality_claim_count_total = 0_u64;
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
        evidence_item_count_total,
        low_quality_evidence_item_count_total,
        quality_claim_count_total,
        concrete_claim_count_total,
        citation_ready_claim_count_total,
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

#[derive(Default)]
struct WebGateMetric {
    total: u64,
    passed: u64,
    failed: u64,
    artifact_present: u64,
    artifact_missing: u64,
    artifact_present_failures: u64,
    artifact_missing_failures: u64,
    first_failure_count: u64,
}

fn web_tooling_measured_rows(rows: &[Value]) -> Vec<&Value> {
    rows.iter()
        .filter(|row| web_tooling_measurement_exclusion_reason_row(row).is_none())
        .collect()
}

fn web_tooling_measurement_exclusion_reason_row(row: &Value) -> Option<&'static str> {
    if let Some(explicit) = row
        .get("web_tooling_measurement_exclusion")
        .and_then(Value::as_str)
    {
        return match explicit {
            "" | "none" => None,
            "transport_failure" => Some("transport_failure"),
            "post_tool_context_not_seeded" => Some("post_tool_context_not_seeded"),
            _ => None,
        };
    }
    if bool_at(row, &["transport_failure"], false) {
        return Some("transport_failure");
    }
    let retrieval_quality = row
        .get("web_tooling_retrieval_quality")
        .or_else(|| row.get("retrieval_quality"))
        .unwrap_or(&Value::Null);
    if str_at(row, &["category"], "") == "post_tool_synthesis"
        && !bool_at(retrieval_quality, &["tool_executed"], false)
        && str_at(retrieval_quality, &["status"], "") == "not_attempted"
    {
        return Some("post_tool_context_not_seeded");
    }
    None
}

fn unseeded_post_tool_synthesis_case(
    case: &Value,
    payload: &Value,
    retrieval_quality: &Value,
) -> bool {
    if str_at(case, &["category"], "") != "post_tool_synthesis" {
        return false;
    }
    let derived_fallback_request = str_at(
        payload,
        &[
            "pending_tool_request",
            "input",
            "query_metadata_policy",
            "classification",
        ],
        "",
    ) == "derived_prompt_request";
    (!has_tool_execution(payload)
        && web_pending_request(payload).is_none()
        && !bool_at(retrieval_quality, &["tool_executed"], false)
        && str_at(retrieval_quality, &["status"], "") == "not_attempted")
        || derived_fallback_request
}

fn web_gate(
    name: &str,
    artifact_present: bool,
    passed: bool,
    reason: &str,
    artifact_refs: Vec<String>,
) -> Value {
    json!({
        "gate": name,
        "status": if passed { "pass" } else { "fail" },
        "artifact_present": artifact_present,
        "reason": reason,
        "artifact_refs": artifact_refs
    })
}

fn access_blocker_refs(access_blocker: &Value) -> Vec<String> {
    access_blocker
        .get("artifact_refs")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|rows| !rows.is_empty())
        .unwrap_or_else(|| {
            vec![
                "tools.status".to_string(),
                "tools.result".to_string(),
                "tools.error".to_string(),
                "response_finalization.tool_completion.tool_attempts".to_string(),
            ]
        })
}

fn provider_supply_refs(provider_supply: &Value) -> Vec<String> {
    provider_supply
        .get("artifact_refs")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|rows| !rows.is_empty())
        .unwrap_or_else(|| {
            vec![
                "retrieval_telemetry".to_string(),
                "tool_result_quality.provider_attempts".to_string(),
                "provider_health".to_string(),
                "provider_errors".to_string(),
            ]
        })
}

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
            "claim_quality_ready": bool_at(evidence_quality, &["claim_quality_ready"], false),
            "citation_renderability_ready": bool_at(
                evidence_quality,
                &["citation_renderability_ready"],
                false
            ),
            "answerability_ready": bool_at(evidence_quality, &["answerability_ready"], false),
            "clean_evidence_rate": f64_at(evidence_quality, &["clean_evidence_rate"], 0.0),
            "concrete_claim_rate": f64_at(evidence_quality, &["concrete_claim_rate"], 0.0),
            "citation_ready_claim_rate": f64_at(
                evidence_quality,
                &["citation_ready_claim_rate"],
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

fn web_operator_aggregate_metrics(
    measured_cases: u64,
    transport_excluded_cases: u64,
    candidate_count_total: u64,
    evidence_count_total: u64,
    content_rich_candidate_count_total: u64,
    claim_hint_count_total: u64,
    query_lane_count_total: u64,
    followup_query_count_total: u64,
    keyword_count_total: u64,
    required_entity_count_total: u64,
    required_facet_count_total: u64,
    multi_query_cases: u64,
    unique_source_domains_total: u64,
    unique_evidence_domains_total: u64,
    source_class_count_total: u64,
    official_or_primary_cases: u64,
    relevant_evidence_count_total: u64,
    topic_relevant_cases: u64,
    usable_evidence_cases: u64,
    provider_starved_cases: u64,
    access_blocked_cases: u64,
    synthesis_handoff_cases: u64,
    source_quality_ready_cases: u64,
    claim_quality_ready_cases: u64,
    citation_renderability_ready_cases: u64,
    answerability_ready_cases: u64,
    evidence_item_count_total: u64,
    low_quality_evidence_item_count_total: u64,
    quality_claim_count_total: u64,
    concrete_claim_count_total: u64,
    citation_ready_claim_count_total: u64,
    first_failure_counts: &BTreeMap<String, u64>,
    gate_metrics: &[Value],
) -> Value {
    let gates_below_target = gate_metrics
        .iter()
        .filter(|row| !bool_at(row, &["ok"], true))
        .count() as u64;
    let top_failure = top_count_row(first_failure_counts);
    json!({
        "schema_version": 1,
        "readout": web_operator_aggregate_readout(&top_failure),
        "top_first_failure": top_failure,
        "top_layer": top_failure
            .get("name")
            .and_then(Value::as_str)
            .map(web_failure_layer)
            .unwrap_or("none"),
        "measured_cases": measured_cases,
        "transport_excluded_cases": transport_excluded_cases,
        "gates_below_target": gates_below_target,
        "query_planning": {
            "query_lanes_per_case": ratio(query_lane_count_total, measured_cases),
            "followup_queries_per_case": ratio(followup_query_count_total, measured_cases),
            "keywords_per_case": ratio(keyword_count_total, measured_cases),
            "required_entities_per_case": ratio(required_entity_count_total, measured_cases),
            "required_facets_per_case": ratio(required_facet_count_total, measured_cases),
            "multi_query_case_rate": ratio(multi_query_cases, measured_cases)
        },
        "candidate_supply": {
            "unique_source_domains_per_case": ratio(unique_source_domains_total, measured_cases),
            "unique_evidence_domains_per_case": ratio(unique_evidence_domains_total, measured_cases),
            "source_classes_per_case": ratio(source_class_count_total, measured_cases),
            "official_or_primary_case_rate": ratio(official_or_primary_cases, measured_cases),
            "relevant_evidence_per_candidate": ratio(relevant_evidence_count_total, candidate_count_total),
            "topic_relevant_case_rate": ratio(topic_relevant_cases, measured_cases)
        },
        "averages": {
            "raw_candidates_per_case": ratio(candidate_count_total, measured_cases),
            "evidence_refs_per_case": ratio(evidence_count_total, measured_cases),
            "content_rich_candidates_per_case": ratio(content_rich_candidate_count_total, measured_cases),
            "claim_hints_per_case": ratio(claim_hint_count_total, measured_cases)
        },
        "conversion_rates": {
            "evidence_per_candidate": ratio(evidence_count_total, candidate_count_total),
            "content_rich_per_candidate": ratio(content_rich_candidate_count_total, candidate_count_total),
            "claim_hints_per_evidence": ratio(claim_hint_count_total, evidence_count_total),
            "usable_evidence_case_rate": ratio(usable_evidence_cases, measured_cases),
            "synthesis_handoff_case_rate": ratio(synthesis_handoff_cases, measured_cases)
        },
        "evidence_quality": {
            "source_quality_ready_case_rate": ratio(source_quality_ready_cases, measured_cases),
            "claim_quality_ready_case_rate": ratio(claim_quality_ready_cases, measured_cases),
            "citation_renderability_ready_case_rate": ratio(citation_renderability_ready_cases, measured_cases),
            "answerability_ready_case_rate": ratio(answerability_ready_cases, measured_cases),
            "low_quality_evidence_item_rate": ratio(
                low_quality_evidence_item_count_total,
                evidence_item_count_total
            ),
            "concrete_claim_rate": ratio(concrete_claim_count_total, quality_claim_count_total),
            "citation_ready_claim_rate": ratio(
                citation_ready_claim_count_total,
                quality_claim_count_total
            )
        },
        "blocker_rates": {
            "provider_starved_or_degraded_case_rate": ratio(provider_starved_cases, measured_cases),
            "access_blocked_or_throttled_case_rate": ratio(access_blocked_cases, measured_cases)
        },
        "plain_english": {
            "query_lanes_per_case": "How many concrete retrieval lanes each request submitted on average.",
            "followup_queries_per_case": "How many narrower follow-up query lanes each request carried beyond the first lane.",
            "keywords_per_case": "How much explicit query metadata the request preserved for retrieval.",
            "multi_query_case_rate": "Share of measured cases that used more than one explicit query lane.",
            "raw_candidates_per_case": "How many candidate URLs/rows the web tooling found before filtering.",
            "unique_source_domains_per_case": "How many distinct source domains retrieval surfaced per case.",
            "official_or_primary_case_rate": "Share of measured cases that surfaced at least one official or primary source.",
            "relevant_evidence_per_candidate": "How much of the candidate supply stayed relevant to the user's actual prompt.",
            "evidence_per_candidate": "How much of the raw candidate supply survived packaging into evidence refs.",
            "content_rich_per_candidate": "How often candidates had usable page/snippet content rather than thin search rows.",
            "claim_hints_per_evidence": "How much claim-level material synthesis received per evidence item.",
            "source_quality_ready_case_rate": "Share of cases where selected evidence was source-backed and not dominated by low-quality/candidate-only material.",
            "claim_quality_ready_case_rate": "Share of cases where extracted claims looked like concrete answer material rather than headings, source labels, or boilerplate.",
            "citation_renderability_ready_case_rate": "Share of cases where claim/evidence material retained enough locator/title/domain data to render citations.",
            "answerability_ready_case_rate": "Share of cases where clean evidence, concrete claims, and citation data all existed together.",
            "usable_evidence_case_rate": "Share of measured cases where retrieval produced evidence strong enough for synthesis.",
            "provider_starved_or_degraded_case_rate": "Share of cases where the first meaningful blocker was missing/degraded provider supply.",
            "access_blocked_or_throttled_case_rate": "Share of cases with detected bot wall, rate-limit, CAPTCHA, auth, or access-control signals."
        }
    })
}

fn web_operator_case_readout(primary_bottleneck: &str, retrieval_status: &str) -> String {
    match primary_bottleneck {
        "no_web_tooling_failure_detected" => "web tooling path completed for this case".to_string(),
        "query_planning_metadata_missing" => {
            "request is too thin: add visible keywords, query pack, or coverage metadata".to_string()
        }
        "web_tool_attempt_missing" => {
            "request was shaped, but no web tool attempt was recorded".to_string()
        }
        "provider_rate_limited_or_quota_exhausted" => {
            "candidate supply is constrained by provider quota, rate-limit, Retry-After, throttling, or HTTP 429 signals".to_string()
        }
        "anti_bot_challenge_or_waf" => {
            "candidate supply hit a CAPTCHA, human-verification, WAF, Cloudflare, or bot-wall challenge".to_string()
        }
        "permission_or_auth_block" => {
            "candidate supply requires auth, login, provider credentials, or permission that was not available".to_string()
        }
        "access_denied_or_forbidden" => {
            "candidate supply hit access-denied, forbidden, request-blocked, or HTTP 403 signals".to_string()
        }
        "provider_configuration_missing" => {
            "candidate supply is blocked by missing provider credentials, admission, or configuration".to_string()
        }
        "access_blocked_or_throttled" => {
            "candidate supply is constrained by access control, throttling, or bot-defense signals".to_string()
        }
        "browser_materialization_failed" => {
            "browser-materialization recovery was visible, but the recovery lane reported a failure".to_string()
        }
        "search_provider_configuration_unusable" => {
            "search candidate supply is blocked by missing strong-provider credentials, admission, or provider configuration".to_string()
        }
        "search_provider_circuit_open" => {
            "search candidate supply is blocked by provider circuit breakers after repeated provider failures".to_string()
        }
        "search_provider_surface_degraded" => {
            "search candidate supply is blocked because the search tool surface reported degraded execution".to_string()
        }
        "provider_raw_rows_absent" => {
            "search providers ran but produced no raw rows to filter or promote".to_string()
        }
        "provider_rows_filtered_before_candidate_promotion" => {
            "search providers produced rows, but filtering/promotion rejected them before usable candidate creation".to_string()
        }
        "provider_candidates_absent" | "provider_empty_or_degraded" => format!(
            "candidate supply is the bottleneck: provider status is {retrieval_status}"
        ),
        "candidate_packaging_missing" => {
            "raw candidates exist, but packaging did not produce evidence refs".to_string()
        }
        "candidate_content_materialization_missing" => {
            "evidence exists, but it is still thin search-row material rather than content-rich page text".to_string()
        }
        "claim_extraction_missing" => {
            "content exists, but claim-level extraction is not giving synthesis enough facts".to_string()
        }
        "source_quality_not_ready" => {
            "evidence exists, but the source material is too thin, low-confidence, or candidate-like for synthesis".to_string()
        }
        "claim_quality_not_ready" => {
            "claim strings exist, but they are mostly titles, source labels, boilerplate, or fragments rather than usable answer material".to_string()
        }
        "citation_renderability_not_ready" => {
            "claim strings exist, but they do not carry enough source locator, title, or domain data to render citations".to_string()
        }
        "answerability_not_ready" => {
            "evidence and claims exist, but the package is not yet coherent enough for a bounded useful answer".to_string()
        }
        "retrieval_quality_not_usable" => {
            "evidence reached the tool layer, but quality is too weak for source-backed synthesis".to_string()
        }
        "evidence_context_handoff_missing" => {
            "evidence exists, but the final synthesis boundary did not receive it".to_string()
        }
        _ => "web tooling failed at an unclassified boundary".to_string(),
    }
}

fn web_failure_layer(gate: &str) -> &'static str {
    match gate {
        "" | "none" => "none",
        "web_1_request_shape_present"
        | "web_2_query_metadata_present"
        | "web_3_tool_attempt_recorded" => "query_planning",
        "web_3b1_provider_quota_not_rate_limited"
        | "web_3b2_no_bot_challenge_or_waf"
        | "web_3b3_no_permission_or_auth_block"
        | "web_3b4_no_access_denied_or_forbidden"
        | "web_3b5_provider_configuration_available"
        | "web_3b_access_not_blocked_or_throttled"
        | "web_3c_blocker_recovery_lane_visible"
        | "web_3d_browser_materialization_not_failed"
        | "web_5b_content_rich_candidates_present" => "access_materialization",
        "web_4a_search_provider_configuration_usable"
        | "web_4b_search_provider_circuit_closed"
        | "web_4c_search_provider_surface_ready"
        | "web_4d_provider_raw_rows_available"
        | "web_4e_provider_candidates_survive_filtering"
        | "web_4_raw_candidates_present"
        | "web_6_provider_not_empty_or_degraded" => "candidate_supply",
        "web_5_packaged_evidence_present"
        | "web_5d_source_quality_ready"
        | "web_5f_citation_renderability_ready"
        | "web_5g_answerability_ready"
        | "web_7_usable_evidence_available"
        | "web_8_evidence_context_to_synthesis" => "usable_evidence_packaging",
        "web_5c_claim_extraction_present" | "web_5e_claim_quality_ready" => "claim_extraction",
        _ => "unknown",
    }
}

fn web_operator_next_action(primary_bottleneck: &str) -> &'static str {
    match primary_bottleneck {
        "no_web_tooling_failure_detected" => "inspect synthesis quality rather than web tooling",
        "query_planning_metadata_missing" => {
            "improve visible query metadata and coverage declaration"
        }
        "web_tool_attempt_missing" => "inspect workflow-to-tool invocation wiring",
        "provider_rate_limited_or_quota_exhausted" => {
            "reduce request pressure, use admitted quota-backed providers, or add provider backoff before tuning synthesis"
        }
        "anti_bot_challenge_or_waf" => {
            "prefer allowed APIs/source feeds or policy-compliant browser materialization for selected URLs"
        }
        "permission_or_auth_block" => {
            "route through admitted credentials or skip sources that require unavailable permission"
        }
        "access_denied_or_forbidden" => {
            "choose alternate allowed sources or provider paths before tuning synthesis"
        }
        "provider_configuration_missing" => {
            "configure/admit the provider or mark it unavailable before broad retrieval"
        }
        "access_blocked_or_throttled" => {
            "use allowed alternate provider or browser materialization when policy permits"
        }
        "browser_materialization_failed" => {
            "inspect browser recovery execution, page load, and extraction diagnostics"
        }
        "search_provider_configuration_unusable" => {
            "configure/admit a strong search provider or mark unavailable providers out of the active order"
        }
        "search_provider_circuit_open" => {
            "inspect provider circuit-breaker state, cooldown, and repeated failure signatures"
        }
        "search_provider_surface_degraded" => {
            "repair the search tool surface before tuning query planning or synthesis"
        }
        "provider_raw_rows_absent" => {
            "inspect provider execution and provider response parsing"
        }
        "provider_rows_filtered_before_candidate_promotion" => {
            "inspect candidate filters, relevance thresholds, and low-confidence promotion policy"
        }
        "provider_candidates_absent" | "provider_empty_or_degraded" => {
            "configure or admit a stronger search provider before tuning synthesis"
        }
        "candidate_packaging_missing" => "inspect candidate-to-evidence packaging",
        "candidate_content_materialization_missing" => {
            "improve page fetch/materialization and content extraction"
        }
        "claim_extraction_missing" => "improve claim extraction from evidence-pack content",
        "source_quality_not_ready" => {
            "tighten source selection and evidence packaging so selected evidence is clean, source-backed, and not candidate-only"
        }
        "claim_quality_not_ready" => {
            "tighten extraction so synthesis receives concrete claims instead of titles, labels, or boilerplate fragments"
        }
        "citation_renderability_not_ready" => {
            "preserve locator, title, and domain alongside each selected evidence claim"
        }
        "answerability_not_ready" => {
            "improve selected evidence quality and claim support before tuning synthesis style"
        }
        "retrieval_quality_not_usable" => "increase candidate quality and source diversity",
        "evidence_context_handoff_missing" => "inspect evidence-to-synthesis handoff",
        _ => "inspect raw gate rows and artifacts",
    }
}

fn web_operator_aggregate_readout(top_failure: &Value) -> String {
    let gate = top_failure
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("none");
    let count = top_failure
        .get("count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if gate == "none" || count == 0 {
        "no recurring web-tooling failure dominated this run".to_string()
    } else {
        format!(
            "top recurring web-tooling blocker: {} in {} measured case(s)",
            web_failure_boundary(gate),
            count
        )
    }
}

fn declared_source_preference_count(request_input: Option<&Value>) -> u64 {
    request_input
        .and_then(Value::as_object)
        .and_then(|map| map.get("source_preferences"))
        .and_then(Value::as_array)
        .map(|rows| rows.len() as u64)
        .unwrap_or(0)
}

fn unique_source_domain_count(payload: &Value) -> u64 {
    unique_domain_inventory(payload, false).len() as u64
}

fn unique_evidence_domain_count(payload: &Value) -> u64 {
    unique_domain_inventory(payload, true).len() as u64
}

fn unique_source_class_count(payload: &Value) -> u64 {
    let mut classes = Vec::<String>::new();
    for object in source_like_objects(payload) {
        if let Some(class) = source_class_value(object) {
            push_unique_case_insensitive(&mut classes, &class);
        }
    }
    classes.len() as u64
}

fn official_or_primary_source_count(payload: &Value) -> u64 {
    source_like_objects(payload)
        .iter()
        .filter_map(|object| source_class_value(object))
        .filter(|class| {
            let normalized = normalize_for_compare(class);
            normalized.contains("official") || normalized.contains("primary")
        })
        .count() as u64
}

fn unique_domain_inventory(payload: &Value, evidence_only: bool) -> Vec<String> {
    let mut domains = Vec::<String>::new();
    for object in source_like_objects(payload) {
        if evidence_only && !object_looks_like_evidence(object) {
            continue;
        }
        if let Some(domain) = source_domain_value(object) {
            push_unique_case_insensitive(&mut domains, &domain);
        }
    }
    domains
}

fn source_like_objects<'a>(payload: &'a Value) -> Vec<&'a serde_json::Map<String, Value>> {
    let mut out = Vec::<&serde_json::Map<String, Value>>::new();
    collect_source_like_objects(payload, &mut out, 0);
    out
}

fn collect_source_like_objects<'a>(
    value: &'a Value,
    out: &mut Vec<&'a serde_json::Map<String, Value>>,
    depth: usize,
) {
    if depth > 8 {
        return;
    }
    match value {
        Value::Array(rows) => {
            for row in rows {
                collect_source_like_objects(row, out, depth + 1);
            }
        }
        Value::Object(map) => {
            if object_looks_like_source_row(map) {
                out.push(map);
            }
            for child in map.values() {
                collect_source_like_objects(child, out, depth + 1);
            }
        }
        _ => {}
    }
}

fn object_looks_like_source_row(map: &serde_json::Map<String, Value>) -> bool {
    [
        "title",
        "source_domain",
        "source_class",
        "source_kind",
        "locator",
        "url",
        "source_url",
        "link",
        "snippet",
        "summary",
        "content",
        "markdown",
        "text",
        "claim_hints",
    ]
    .iter()
    .any(|key| map.get(*key).map(value_has_content).unwrap_or(false))
}

fn object_looks_like_evidence(map: &serde_json::Map<String, Value>) -> bool {
    [
        "claim_hints",
        "summary",
        "content",
        "markdown",
        "text",
        "snippet",
        "evidence_ref",
        "citation",
        "source_domain",
        "source_class",
    ]
    .iter()
    .any(|key| map.get(*key).map(value_has_content).unwrap_or(false))
}

fn source_class_value(map: &serde_json::Map<String, Value>) -> Option<String> {
    ["source_class", "source_kind", "class"]
        .iter()
        .find_map(|key| map.get(*key).and_then(Value::as_str))
        .map(|raw| clean_text(raw, 120))
        .filter(|raw| !raw.is_empty())
}

fn source_domain_value(map: &serde_json::Map<String, Value>) -> Option<String> {
    map.get("source_domain")
        .and_then(Value::as_str)
        .map(|raw| clean_text(raw, 160))
        .filter(|raw| !raw.is_empty())
        .or_else(|| {
            ["locator", "url", "source_url", "link"]
                .iter()
                .find_map(|key| map.get(*key).and_then(Value::as_str))
                .and_then(extract_domain_like_host)
        })
}

fn extract_domain_like_host(raw: &str) -> Option<String> {
    let cleaned = clean_text(raw, 240);
    if cleaned.is_empty() {
        return None;
    }
    let hostish = cleaned
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .trim_start_matches("www.")
        .split('/')
        .next()
        .unwrap_or("")
        .trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '.' && ch != '-');
    if hostish.is_empty() || !hostish.contains('.') {
        return None;
    }
    Some(hostish.to_ascii_lowercase())
}

fn push_unique_case_insensitive(values: &mut Vec<String>, candidate: &str) {
    let cleaned = clean_text(candidate, 160);
    if cleaned.is_empty() {
        return;
    }
    let normalized = cleaned.to_ascii_lowercase();
    if values
        .iter()
        .any(|existing| existing.to_ascii_lowercase() == normalized)
    {
        return;
    }
    values.push(cleaned);
}

fn top_count_row(counts: &BTreeMap<String, u64>) -> Value {
    let mut best_name = "none".to_string();
    let mut best_count = 0_u64;
    for (name, count) in counts {
        if *count > best_count {
            best_name = name.clone();
            best_count = *count;
        }
    }
    json!({
        "name": best_name,
        "count": best_count,
        "boundary": web_failure_boundary(counts
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(name, _)| name.as_str())
            .unwrap_or("none"))
    })
}

pub(super) fn web_failure_boundary(gate: &str) -> &'static str {
    match gate {
        "" | "none" => "no_web_tooling_failure_detected",
        "web_1_request_shape_present" => "web_request_shape_missing",
        "web_2_query_metadata_present" => "query_planning_metadata_missing",
        "web_3_tool_attempt_recorded" => "web_tool_attempt_missing",
        "web_3b1_provider_quota_not_rate_limited" => "provider_rate_limited_or_quota_exhausted",
        "web_3b2_no_bot_challenge_or_waf" => "anti_bot_challenge_or_waf",
        "web_3b3_no_permission_or_auth_block" => "permission_or_auth_block",
        "web_3b4_no_access_denied_or_forbidden" => "access_denied_or_forbidden",
        "web_3b5_provider_configuration_available" => "provider_configuration_missing",
        "web_3b_access_not_blocked_or_throttled" => "access_blocked_or_throttled",
        "web_3c_blocker_recovery_lane_visible" => "access_blocker_recovery_lane_missing",
        "web_3d_browser_materialization_not_failed" => "browser_materialization_failed",
        "web_4a_search_provider_configuration_usable" => "search_provider_configuration_unusable",
        "web_4b_search_provider_circuit_closed" => "search_provider_circuit_open",
        "web_4c_search_provider_surface_ready" => "search_provider_surface_degraded",
        "web_4d_provider_raw_rows_available" => "provider_raw_rows_absent",
        "web_4e_provider_candidates_survive_filtering" => {
            "provider_rows_filtered_before_candidate_promotion"
        }
        "web_4_raw_candidates_present" => "provider_candidates_absent",
        "web_5_packaged_evidence_present" => "candidate_packaging_missing",
        "web_5b_content_rich_candidates_present" => "candidate_content_materialization_missing",
        "web_5c_claim_extraction_present" => "claim_extraction_missing",
        "web_5d_source_quality_ready" => "source_quality_not_ready",
        "web_5e_claim_quality_ready" => "claim_quality_not_ready",
        "web_5f_citation_renderability_ready" => "citation_renderability_not_ready",
        "web_5g_answerability_ready" => "answerability_not_ready",
        "web_6_provider_not_empty_or_degraded" => "provider_empty_or_degraded",
        "web_7_usable_evidence_available" => "retrieval_quality_not_usable",
        "web_8_evidence_context_to_synthesis" => "evidence_context_handoff_missing",
        _ => "unknown_web_tooling_failure",
    }
}

fn web_pending_request(payload: &Value) -> Option<&Value> {
    payload
        .get("pending_tool_request")
        .or_else(|| payload.pointer("/response_workflow/pending_tool_request"))
        .or_else(|| payload.pointer("/response_workflow/manual_toolbox_pending_tool_request"))
        .or_else(|| payload.pointer("/response_finalization/pending_tool_request"))
}

fn request_input_object(request: &Value) -> Option<&Value> {
    request
        .get("input")
        .or_else(|| request.get("request_payload"))
        .or_else(|| request.get("payload"))
}

fn input_has_query_or_locator(input: &Value) -> bool {
    [
        "query", "queries", "keyword", "keywords", "url", "urls", "locator", "locators", "source",
    ]
    .iter()
    .any(|key| value_has_content(input.get(*key).unwrap_or(&Value::Null)))
}

fn request_shape_refs(input: Option<&Value>) -> Vec<String> {
    let Some(input) = input.and_then(Value::as_object) else {
        return vec!["pending_tool_request.input".to_string()];
    };
    input
        .keys()
        .map(|key| format!("pending_tool_request.input.{key}"))
        .collect()
}

fn metadata_refs(query_metadata_diagnostics: &Value) -> Vec<String> {
    let fields = query_metadata_diagnostics
        .get("fields_present")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|field| format!("pending_tool_request.input.{field}"))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if fields.is_empty() {
        vec!["query_metadata_diagnostics.fields_present".to_string()]
    } else {
        fields
    }
}

fn checkpoint_passed(diagnostics: &Value, checkpoint: &str) -> bool {
    diagnostics
        .get("checkpoints")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter().any(|row| {
                row.get("checkpoint").and_then(Value::as_str) == Some(checkpoint)
                    && row.get("status").and_then(Value::as_str) == Some("pass")
            })
        })
        .unwrap_or(false)
}

fn has_tool_execution(payload: &Value) -> bool {
    payload
        .get("tools")
        .and_then(Value::as_array)
        .map(|rows| !rows.is_empty())
        .unwrap_or(false)
        || payload
            .pointer("/response_finalization/tool_completion/tool_attempts")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false)
}

fn value_has_content(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(raw) => *raw,
        Value::Number(_) => true,
        Value::String(raw) => !raw.trim().is_empty(),
        Value::Array(rows) => rows.iter().any(value_has_content),
        Value::Object(map) => map.values().any(value_has_content),
    }
}

fn web_evidence_quality_diagnostics(payload: &Value, retrieval_quality: &Value) -> Value {
    let mut scan = EvidenceQualityScan::default();
    scan_evidence_quality(payload, "payload", &mut scan, 0);
    scan_evidence_quality(retrieval_quality, "retrieval_quality", &mut scan, 0);
    scan.source_domains.sort_unstable();
    scan.source_domains.dedup();
    scan.low_quality_flags.sort_unstable();
    scan.low_quality_flags.dedup();
    scan.refs.sort_unstable();
    scan.refs.dedup();

    let retrieval_usable = bool_at(retrieval_quality, &["usable_evidence"], false);
    let evidence_count = u64_at(retrieval_quality, &["evidence_count"], 0);
    let claim_hint_count = u64_at(retrieval_quality, &["claim_hint_count"], 0);
    let direct_claim_count = u64_at(
        retrieval_quality,
        &["classification_inputs", "direct_evidence_claim_count"],
        0,
    );
    let quality_pack = retrieval_quality
        .get("evidence_pack_quality")
        .unwrap_or(&Value::Null);
    let pack_usable_count = u64_at(quality_pack, &["usable_count"], 0);
    let pack_content_rich_count = u64_at(quality_pack, &["content_rich_item_count"], 0);
    let pack_claim_hint_count = u64_at(quality_pack, &["claim_hint_count"], 0);
    let pack_low_confidence_count = u64_at(quality_pack, &["low_confidence_count"], 0);
    let pack_candidate_only_count = u64_at(quality_pack, &["candidate_only_count"], 0);

    let evidence_item_count = scan
        .evidence_item_count
        .max(evidence_count)
        .max(pack_usable_count);
    let clean_evidence_count = scan
        .clean_evidence_item_count
        .max(pack_usable_count)
        .max(pack_content_rich_count.min(evidence_item_count));
    let low_quality_evidence_count = scan
        .low_quality_evidence_item_count
        .max(pack_low_confidence_count.saturating_add(pack_candidate_only_count));
    let claim_count = scan
        .claim_count
        .max(claim_hint_count)
        .max(direct_claim_count);
    let concrete_claim_count = scan.concrete_claim_count.max(direct_claim_count);
    let citation_ready_claim_count = scan.citation_ready_claim_count;
    let clean_evidence_rate = ratio(clean_evidence_count, evidence_item_count);
    let low_quality_evidence_rate = ratio(low_quality_evidence_count, evidence_item_count);
    let concrete_claim_rate = ratio(concrete_claim_count, claim_count);
    let low_quality_claim_rate = ratio(scan.low_quality_claim_count, claim_count);
    let citation_ready_claim_rate = ratio(citation_ready_claim_count, claim_count);

    let source_quality_ready = evidence_item_count > 0
        && clean_evidence_count > 0
        && (scan.citation_ready_evidence_item_count > 0
            || !scan.source_domains.is_empty()
            || retrieval_usable)
        && low_quality_evidence_count < evidence_item_count
        && clean_evidence_rate >= 0.5;
    let claim_quality_ready = claim_count > 0
        && concrete_claim_count > 0
        && scan.low_quality_claim_count < claim_count
        && concrete_claim_rate >= 0.35
        && low_quality_claim_rate < 0.75;
    let citation_renderability_ready = citation_ready_claim_count > 0
        || (scan.citation_ready_evidence_item_count > 0
            && (claim_hint_count > 0 || direct_claim_count > 0 || pack_claim_hint_count > 0));
    let answerability_ready =
        source_quality_ready && claim_quality_ready && citation_renderability_ready;

    json!({
        "schema_version": 1,
        "source_quality_ready": source_quality_ready,
        "claim_quality_ready": claim_quality_ready,
        "citation_renderability_ready": citation_renderability_ready,
        "answerability_ready": answerability_ready,
        "evidence_item_count": evidence_item_count,
        "clean_evidence_item_count": clean_evidence_count,
        "low_quality_evidence_item_count": low_quality_evidence_count,
        "clean_evidence_rate": clean_evidence_rate,
        "low_quality_evidence_rate": low_quality_evidence_rate,
        "claim_count": claim_count,
        "concrete_claim_count": concrete_claim_count,
        "low_quality_claim_count": scan.low_quality_claim_count,
        "citation_ready_claim_count": citation_ready_claim_count,
        "citation_ready_evidence_item_count": scan.citation_ready_evidence_item_count,
        "concrete_claim_rate": concrete_claim_rate,
        "low_quality_claim_rate": low_quality_claim_rate,
        "citation_ready_claim_rate": citation_ready_claim_rate,
        "source_domain_count": scan.source_domains.len() as u64,
        "source_domains": scan.source_domains,
        "low_quality_flags": scan.low_quality_flags,
        "artifact_refs": scan.refs,
        "note": "Generic evidence-quality readout: checks whether packaged evidence has clean source-backed material, concrete claim text, and citation renderability without assuming the query domain."
    })
}

fn evidence_quality_refs(evidence_quality: &Value) -> Vec<String> {
    evidence_quality
        .get("artifact_refs")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|rows| !rows.is_empty())
        .unwrap_or_else(|| {
            vec![
                "evidence_pack".to_string(),
                "evidence_refs".to_string(),
                "evidence_claims".to_string(),
                "evidence_pack_quality".to_string(),
                "retrieval_quality.classification_inputs".to_string(),
            ]
        })
}

#[derive(Default)]
struct EvidenceQualityScan {
    evidence_item_count: u64,
    clean_evidence_item_count: u64,
    low_quality_evidence_item_count: u64,
    citation_ready_evidence_item_count: u64,
    claim_count: u64,
    concrete_claim_count: u64,
    low_quality_claim_count: u64,
    citation_ready_claim_count: u64,
    source_domains: Vec<String>,
    low_quality_flags: Vec<String>,
    refs: Vec<String>,
}

fn scan_evidence_quality(value: &Value, path: &str, scan: &mut EvidenceQualityScan, depth: usize) {
    if depth > 10 || evidence_quality_declarative_path(path) {
        return;
    }
    match value {
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
        Value::Array(rows) => {
            for (index, row) in rows.iter().enumerate() {
                scan_evidence_quality(row, &format!("{path}.{index}"), scan, depth + 1);
            }
        }
        Value::Object(map) => {
            let evidence_context = evidence_quality_context_path(path);
            if evidence_context && object_looks_like_evidence(map) {
                analyze_evidence_quality_object(map, path, scan);
            }
            for (key, child) in map {
                scan_evidence_quality(child, &format!("{path}.{key}"), scan, depth + 1);
            }
        }
    }
}

fn evidence_quality_declarative_path(path: &str) -> bool {
    let normalized = normalize_for_compare(&path.replace(['.', '_', '-'], " "));
    [
        "pending tool request",
        "query metadata",
        "gate diagnostics",
        "operator metrics",
        "plain english",
        "blocker taxonomy",
        "recommended next capability",
        "capability contract",
        "tooling cd",
        "workflow cd",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn evidence_quality_context_path(path: &str) -> bool {
    let normalized = normalize_for_compare(&path.replace(['.', '_', '-'], " "));
    [
        "evidence pack",
        "evidence refs",
        "evidence ref",
        "evidence claims",
        "evidence claim",
        "findings",
        "source candidates",
        "synthesis candidates",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn analyze_evidence_quality_object(
    map: &serde_json::Map<String, Value>,
    path: &str,
    scan: &mut EvidenceQualityScan,
) {
    scan.evidence_item_count = scan.evidence_item_count.saturating_add(1);
    scan.refs.push(path.to_string());

    if let Some(domain) = source_domain_value(map) {
        push_unique_case_insensitive(&mut scan.source_domains, &domain);
    }

    let citation_ready = evidence_object_citation_ready(map);
    if citation_ready {
        scan.citation_ready_evidence_item_count =
            scan.citation_ready_evidence_item_count.saturating_add(1);
    }

    let low_quality = evidence_object_low_quality(map, path, scan);
    if low_quality {
        scan.low_quality_evidence_item_count =
            scan.low_quality_evidence_item_count.saturating_add(1);
    } else if evidence_object_has_clean_content(map) {
        scan.clean_evidence_item_count = scan.clean_evidence_item_count.saturating_add(1);
    }

    for claim in evidence_object_claim_strings(map) {
        scan.claim_count = scan.claim_count.saturating_add(1);
        let low_claim = claim_text_low_quality(&claim);
        if low_claim {
            scan.low_quality_claim_count = scan.low_quality_claim_count.saturating_add(1);
        }
        if !low_claim && claim_text_concrete(&claim) {
            scan.concrete_claim_count = scan.concrete_claim_count.saturating_add(1);
            if citation_ready {
                scan.citation_ready_claim_count = scan.citation_ready_claim_count.saturating_add(1);
            }
        }
    }
}

fn evidence_object_citation_ready(map: &serde_json::Map<String, Value>) -> bool {
    let locator_present = [
        "locator",
        "url",
        "source_url",
        "link",
        "source_locator",
        "citation",
    ]
    .iter()
    .any(|key| map.get(*key).map(value_has_content).unwrap_or(false));
    let title_or_ref_present = ["title", "source_title", "source_ref"]
        .iter()
        .any(|key| map.get(*key).map(value_has_content).unwrap_or(false));
    locator_present || (title_or_ref_present && source_domain_value(map).is_some())
}

fn evidence_object_has_clean_content(map: &serde_json::Map<String, Value>) -> bool {
    evidence_object_content_strings(map)
        .iter()
        .any(|raw| !content_text_low_quality(raw) && word_count(raw) >= 6)
}

fn evidence_object_low_quality(
    map: &serde_json::Map<String, Value>,
    path: &str,
    scan: &mut EvidenceQualityScan,
) -> bool {
    let mut low_quality = false;
    for flag in evidence_object_flag_strings(map) {
        if low_quality_flag_text(&flag) {
            low_quality = true;
            push_unique_case_insensitive(&mut scan.low_quality_flags, &flag);
        }
    }
    if let Some(confidence) = ["confidence", "score", "quality_score"]
        .iter()
        .find_map(|key| map.get(*key).and_then(Value::as_f64))
        .filter(|score| *score < 0.35)
    {
        low_quality = true;
        push_unique_case_insensitive(
            &mut scan.low_quality_flags,
            &format!("low_numeric_confidence_{confidence:.2}"),
        );
    }
    if map
        .get("counts_as_usable_evidence")
        .and_then(Value::as_bool)
        == Some(false)
    {
        low_quality = true;
        push_unique_case_insensitive(&mut scan.low_quality_flags, "not_usable_evidence");
    }
    if map.get("usable").and_then(Value::as_bool) == Some(false)
        && evidence_quality_context_path(path)
    {
        low_quality = true;
        push_unique_case_insensitive(&mut scan.low_quality_flags, "not_usable");
    }
    for raw in evidence_object_content_strings(map) {
        if content_text_low_quality(&raw) {
            low_quality = true;
            push_unique_case_insensitive(
                &mut scan.low_quality_flags,
                "boilerplate_or_source_only_text",
            );
        }
    }
    low_quality
}

fn evidence_object_flag_strings(map: &serde_json::Map<String, Value>) -> Vec<String> {
    let mut values = Vec::new();
    for key in [
        "quality_flags",
        "flags",
        "limitations",
        "failure_reasons",
        "rejection_reasons",
        "status",
        "materialization_quality",
        "result_quality",
        "source_quality",
    ] {
        collect_value_strings(map.get(key).unwrap_or(&Value::Null), &mut values);
    }
    values
}

fn evidence_object_content_strings(map: &serde_json::Map<String, Value>) -> Vec<String> {
    let mut values = Vec::new();
    for key in [
        "snippet",
        "summary",
        "content",
        "markdown",
        "text",
        "description",
        "result",
        "body",
    ] {
        collect_value_strings(map.get(key).unwrap_or(&Value::Null), &mut values);
    }
    values
}

fn evidence_object_claim_strings(map: &serde_json::Map<String, Value>) -> Vec<String> {
    let mut values = Vec::new();
    for key in [
        "claim",
        "claims",
        "claim_hint",
        "claim_hints",
        "evidence_claims",
        "findings",
    ] {
        collect_value_strings(map.get(key).unwrap_or(&Value::Null), &mut values);
    }
    values
        .into_iter()
        .map(|raw| clean_text(&raw, 500))
        .filter(|raw| !raw.is_empty())
        .collect()
}

fn collect_value_strings(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::String(raw) => {
            let cleaned = clean_text(raw, 500);
            if !cleaned.is_empty() {
                out.push(cleaned);
            }
        }
        Value::Array(rows) => {
            for row in rows {
                collect_value_strings(row, out);
            }
        }
        Value::Object(map) => {
            for child in map.values() {
                collect_value_strings(child, out);
            }
        }
        _ => {}
    }
}

fn low_quality_flag_text(raw: &str) -> bool {
    let normalized = normalize_for_compare(raw);
    [
        "low trust",
        "low_trust",
        "low confidence",
        "low_confidence",
        "low relevance",
        "low_relevance",
        "off topic",
        "off_topic",
        "thin",
        "title only",
        "title_only",
        "source only",
        "source_only",
        "candidate only",
        "candidate_only",
        "search row only",
        "search_row_only",
        "boilerplate",
        "script dump",
        "script_dump",
        "style dump",
        "style_dump",
        "html dump",
        "materialization failed",
        "materialization_failed",
        "content too thin",
        "content_too_thin",
        "link directory",
        "link_directory",
        "aggregator shell",
        "aggregator_shell",
        "social video shell",
        "social_video_shell",
        "affiliate",
        "privacy policy",
        "terms of service",
        "cookie banner",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn content_text_low_quality(raw: &str) -> bool {
    let cleaned = clean_text(raw, 500);
    let normalized = normalize_for_compare(&cleaned);
    if cleaned.len() < 40 || word_count(&cleaned) < 5 {
        return true;
    }
    if urlish_or_source_label(&normalized) {
        return true;
    }
    [
        "click here",
        "subscribe",
        "sign up",
        "privacy policy",
        "terms of service",
        "cookie",
        "javascript",
        "function ",
        "loading",
        "enable javascript",
        "checking your browser",
        "verify you are human",
        "here s what i found",
        "from web retrieval",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn claim_text_low_quality(raw: &str) -> bool {
    let cleaned = clean_text(raw, 500);
    let normalized = normalize_for_compare(&cleaned);
    if cleaned.len() < 28 || word_count(&cleaned) < 4 {
        return true;
    }
    if urlish_or_source_label(&normalized) {
        return true;
    }
    if normalized.starts_with("source ")
        || normalized.starts_with("sources ")
        || normalized.starts_with("web search ")
        || normalized.starts_with("from web retrieval")
    {
        return true;
    }
    [
        "click here",
        "subscribe",
        "sign up",
        "privacy policy",
        "terms of service",
        "cookie",
        "javascript",
        "loading",
        "current news latest news photos videos",
        "here s what i found",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn claim_text_concrete(raw: &str) -> bool {
    let cleaned = clean_text(raw, 500);
    if claim_text_low_quality(&cleaned) {
        return false;
    }
    let normalized = normalize_for_compare(&cleaned);
    let words = word_count(&cleaned);
    let has_specific_marker = cleaned.chars().any(|ch| ch.is_ascii_digit())
        || cleaned.chars().any(|ch| ch.is_ascii_uppercase())
        || normalized.contains(" compared ")
        || normalized.contains(" announced ")
        || normalized.contains(" released ")
        || normalized.contains(" reported ")
        || normalized.contains(" found ")
        || normalized.contains(" shows ")
        || normalized.contains(" offers ")
        || normalized.contains(" supports ")
        || normalized.contains(" requires ")
        || normalized.contains(" costs ")
        || normalized.contains(" changed ")
        || normalized.contains(" increased ")
        || normalized.contains(" decreased ");
    words >= 7 || (words >= 5 && has_specific_marker)
}

fn urlish_or_source_label(normalized: &str) -> bool {
    normalized.starts_with("http ")
        || normalized.starts_with("https ")
        || normalized.starts_with("www ")
        || normalized.starts_with("source ")
        || normalized.starts_with("sources ")
        || normalized.ends_with(" source")
        || normalized.contains(" com ")
        || normalized.contains(" org ")
        || normalized.contains(" net ")
        || normalized.contains(" via google news")
}

fn word_count(raw: &str) -> usize {
    raw.split_whitespace()
        .filter(|word| word.chars().any(|ch| ch.is_ascii_alphanumeric()))
        .count()
}

fn web_provider_supply_diagnostics(payload: &Value, retrieval_quality: &Value) -> Value {
    let mut state = ProviderSupplyScan::default();
    scan_provider_supply(payload, "payload", &mut state);
    scan_provider_supply(retrieval_quality, "retrieval_quality", &mut state);
    state.signals.sort_unstable();
    state.signals.dedup();
    state.refs.sort_unstable();
    state.refs.dedup();

    let missing_config = state.signals.iter().any(|signal| {
        matches!(
            signal.as_str(),
            "serper_api_key_missing"
                | "missing_api_key"
                | "invalid_api_key"
                | "strong_search_provider_missing"
                | "provider_not_configured"
                | "missing_provider_credentials"
        )
    });
    let circuit_open = state
        .signals
        .iter()
        .any(|signal| signal == "provider_circuit_open");
    let surface_degraded = state.signals.iter().any(|signal| {
        matches!(
            signal.as_str(),
            "web_search_tool_surface_degraded"
                | "tool_surface_degraded"
                | "provider_degraded"
                | "provider_error"
                | "transport_error"
        )
    });
    let provider_blocked = state.signals.iter().any(|signal| {
        matches!(
            signal.as_str(),
            "anti_bot_challenge"
                | "web_conduit_policy_denied"
                | "access_denied"
                | "rate_limited"
                | "query_result_mismatch"
        )
    });
    let raw_row_count = state.provider_raw_rows.max(state.provider_result_count);
    let candidate_row_count = state.candidate_rows;
    let filtered_row_count = state.filtered_rows;
    let configuration_usable = !missing_config || candidate_row_count > 0 || raw_row_count > 0;
    json!({
        "schema_version": 1,
        "configuration_usable": configuration_usable,
        "missing_configuration_detected": missing_config,
        "circuit_open_detected": circuit_open,
        "tool_surface_degraded": surface_degraded,
        "provider_blocked_or_denied": provider_blocked,
        "raw_row_count": raw_row_count,
        "provider_result_count": state.provider_result_count,
        "provider_raw_row_count": state.provider_raw_rows,
        "candidate_row_count": candidate_row_count,
        "synthesis_candidate_row_count": state.synthesis_candidate_rows,
        "filtered_or_rejected_row_count": filtered_row_count,
        "low_confidence_raw_row_count": state.low_confidence_raw_rows,
        "signals": state.signals,
        "artifact_refs": state.refs,
        "note": "Separates provider supply into configuration, circuit-breaker, surface readiness, raw-row availability, and candidate-promotion signals."
    })
}

#[derive(Default)]
struct ProviderSupplyScan {
    provider_result_count: u64,
    provider_raw_rows: u64,
    candidate_rows: u64,
    synthesis_candidate_rows: u64,
    filtered_rows: u64,
    low_confidence_raw_rows: u64,
    signals: Vec<String>,
    refs: Vec<String>,
}

fn scan_provider_supply(value: &Value, path: &str, state: &mut ProviderSupplyScan) {
    if provider_supply_declarative_path(path) {
        return;
    }
    match value {
        Value::Null | Value::Bool(_) => {}
        Value::Number(raw) => {
            if let Some(number) = raw.as_u64() {
                let normalized_path = normalize_for_compare(&path.replace(['.', '_', '-'], " "));
                if normalized_path.contains("provider result count")
                    || normalized_path.contains("provider result dedup count")
                {
                    state.provider_result_count = state.provider_result_count.max(number);
                    state.refs.push(path.to_string());
                } else if normalized_path.contains("provider raw rows")
                    || normalized_path.contains("provider raw row")
                    || normalized_path.contains("provider raw count")
                {
                    state.provider_raw_rows = state.provider_raw_rows.max(number);
                    state.refs.push(path.to_string());
                } else if normalized_path.contains("synthesis candidate rows")
                    || normalized_path.contains("synthesis candidate row")
                {
                    state.synthesis_candidate_rows = state.synthesis_candidate_rows.max(number);
                    state.refs.push(path.to_string());
                } else if normalized_path.contains("candidate rows")
                    || normalized_path.contains("candidate row")
                    || normalized_path.contains("candidate count")
                {
                    state.candidate_rows = state.candidate_rows.max(number);
                    state.refs.push(path.to_string());
                } else if normalized_path.contains("filtered or rejected")
                    || normalized_path.contains("filtered rows")
                    || normalized_path.contains("rejected rows")
                {
                    state.filtered_rows = state.filtered_rows.max(number);
                    state.refs.push(path.to_string());
                } else if normalized_path.contains("low confidence raw rows")
                    || normalized_path.contains("low confidence raw row")
                {
                    state.low_confidence_raw_rows = state.low_confidence_raw_rows.max(number);
                    state.refs.push(path.to_string());
                }
            }
        }
        Value::String(raw) => scan_provider_supply_text(raw, path, state),
        Value::Array(rows) => {
            for (index, row) in rows.iter().enumerate() {
                scan_provider_supply(row, &format!("{path}.{index}"), state);
            }
        }
        Value::Object(map) => {
            for (key, child) in map {
                scan_provider_supply(child, &format!("{path}.{key}"), state);
            }
        }
    }
}

fn provider_supply_declarative_path(path: &str) -> bool {
    let normalized = normalize_for_compare(&path.replace(['.', '_', '-'], " "));
    [
        "blocker taxonomy",
        "recommended next capability",
        "query refinement signals",
        "non goals",
        "tool cd",
        "tooling cd",
        "capability contract",
        "request contract supports filters",
        "input contract",
        "plain english",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn scan_provider_supply_text(raw: &str, path: &str, state: &mut ProviderSupplyScan) {
    let normalized = normalize_for_compare(raw);
    let markers = [
        ("serper_api_key_missing", "serper_api_key_missing"),
        ("serper api key missing", "serper_api_key_missing"),
        ("missing api key", "missing_api_key"),
        ("api key missing", "missing_api_key"),
        ("invalid api key", "invalid_api_key"),
        ("strong search provider", "strong_search_provider_missing"),
        ("strong_search_provider", "strong_search_provider_missing"),
        (
            "missing provider credentials",
            "missing_provider_credentials",
        ),
        ("provider not configured", "provider_not_configured"),
        ("provider_circuit_open", "provider_circuit_open"),
        ("provider circuit open", "provider_circuit_open"),
        (
            "web_search_tool_surface_degraded",
            "web_search_tool_surface_degraded",
        ),
        ("tool surface degraded", "tool_surface_degraded"),
        ("provider degraded", "provider_degraded"),
        ("provider_degraded", "provider_degraded"),
        ("provider_error", "provider_error"),
        ("provider error", "provider_error"),
        ("transport_error", "transport_error"),
        ("transport error", "transport_error"),
        ("anti_bot_challenge", "anti_bot_challenge"),
        ("anti bot challenge", "anti_bot_challenge"),
        ("web_conduit_policy_denied", "web_conduit_policy_denied"),
        ("policy denied", "web_conduit_policy_denied"),
        ("access denied", "access_denied"),
        ("rate_limited", "rate_limited"),
        ("rate limited", "rate_limited"),
        ("query_result_mismatch", "query_result_mismatch"),
        ("low_signal_search_payload", "low_signal_search_payload"),
    ];
    for (needle, signal) in markers {
        if normalized.contains(needle) {
            state.signals.push(signal.to_string());
            state.refs.push(path.to_string());
        }
    }
}

fn browser_materialization_recovery_diagnostics(
    payload: &Value,
    retrieval_quality: &Value,
) -> Value {
    let mut refs = Vec::<String>::new();
    let mut failure_signals = Vec::<String>::new();
    let mut recommended = false;
    let mut attempted = false;
    let mut capability_declared = false;
    scan_browser_materialization_recovery(
        payload,
        "payload",
        &mut recommended,
        &mut attempted,
        &mut capability_declared,
        &mut failure_signals,
        &mut refs,
    );
    scan_browser_materialization_recovery(
        retrieval_quality,
        "retrieval_quality",
        &mut recommended,
        &mut attempted,
        &mut capability_declared,
        &mut failure_signals,
        &mut refs,
    );
    refs.sort_unstable();
    refs.dedup();
    failure_signals.sort_unstable();
    failure_signals.dedup();
    json!({
        "schema_version": 1,
        "capability": "browser_materialize_page",
        "recommended_when_policy_allows": recommended,
        "attempted": attempted,
        "capability_declared": capability_declared,
        "failed": !failure_signals.is_empty(),
        "failure_signals": failure_signals,
        "artifact_refs": refs,
        "note": "Measures whether access-blocked runs expose an optional browser-materialization recovery lane. This does not require or default to browser execution."
    })
}

fn scan_browser_materialization_recovery(
    value: &Value,
    path: &str,
    recommended: &mut bool,
    attempted: &mut bool,
    capability_declared: &mut bool,
    failure_signals: &mut Vec<String>,
    refs: &mut Vec<String>,
) {
    if browser_materialization_declarative_path(path) {
        return;
    }
    match value {
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
        Value::String(raw) => {
            let normalized = normalize_for_compare(raw);
            if normalized.contains("browser materialization")
                || normalized.contains("browser_materialization")
                || normalized.contains("browser_materialize_page")
            {
                *capability_declared = true;
                refs.push(path.to_string());
            }
            if normalized.contains("browser_materialization_attempted")
                || normalized.contains("browser materialization attempted")
            {
                *attempted = true;
                refs.push(path.to_string());
            }
            if normalized.contains("recommended_when_policy_allows")
                || normalized.contains("browser materialization recommended")
            {
                *recommended = true;
                refs.push(path.to_string());
            }
            scan_browser_materialization_failure_text(&normalized, path, failure_signals, refs);
        }
        Value::Array(rows) => {
            for (index, row) in rows.iter().enumerate() {
                scan_browser_materialization_recovery(
                    row,
                    &format!("{path}.{index}"),
                    recommended,
                    attempted,
                    capability_declared,
                    failure_signals,
                    refs,
                );
            }
        }
        Value::Object(map) => {
            let path_normalized = normalize_for_compare(path);
            let declares_browser_capability = map
                .get("capability")
                .and_then(Value::as_str)
                .map(|raw| raw == "browser_materialize_page")
                .unwrap_or(false);
            let browser_context_object = path_normalized.contains("browser materialization")
                || path_normalized.contains("browser_materialization")
                || declares_browser_capability;
            if path_normalized.contains("browser materialization") {
                *capability_declared = true;
                refs.push(path.to_string());
            }
            if declares_browser_capability {
                *capability_declared = true;
                refs.push(path.to_string());
            }
            if browser_context_object
                && map
                    .get("recommended_when_policy_allows")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
            {
                *recommended = true;
                refs.push(format!("{path}.recommended_when_policy_allows"));
            }
            if browser_context_object
                && map
                    .get("attempted")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
            {
                *attempted = true;
                refs.push(format!("{path}.attempted"));
            }
            if browser_context_object
                && (map.get("failed").and_then(Value::as_bool).unwrap_or(false)
                    || map
                        .get("success")
                        .and_then(Value::as_bool)
                        .map(|success| !success)
                        .unwrap_or(false)
                    || map
                        .get("status")
                        .and_then(Value::as_str)
                        .map(|status| {
                            matches!(
                                normalize_for_compare(status).as_str(),
                                "failed" | "error" | "timeout" | "blocked"
                            )
                        })
                        .unwrap_or(false))
            {
                failure_signals.push("browser_materialization_failed".to_string());
                refs.push(path.to_string());
            }
            for (key, child) in map {
                scan_browser_materialization_recovery(
                    child,
                    &format!("{path}.{key}"),
                    recommended,
                    attempted,
                    capability_declared,
                    failure_signals,
                    refs,
                );
            }
        }
    }
}

fn browser_materialization_declarative_path(path: &str) -> bool {
    let normalized = normalize_for_compare(&path.replace(['.', '_', '-'], " "));
    [
        "blocker taxonomy",
        "profile compilation",
        "readiness lifecycle",
        "url safety",
        "non goals",
        "source pattern",
        "tool cd",
        "tooling cd",
        "capability contract",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn scan_browser_materialization_failure_text(
    normalized: &str,
    path: &str,
    failure_signals: &mut Vec<String>,
    refs: &mut Vec<String>,
) {
    let path_normalized = normalize_for_compare(path);
    let browser_context = path_normalized.contains("browser materialization")
        || path_normalized.contains("browser_materialization")
        || normalized.contains("browser_materialization_failed")
        || normalized.contains("browser materialization failed");
    if !browser_context {
        return;
    }
    let markers = [
        (
            "browser_materialization_failed",
            "browser_materialization_failed",
        ),
        (
            "browser materialization failed",
            "browser_materialization_failed",
        ),
        ("navigation timeout", "navigation_timeout"),
        ("timed out", "navigation_timeout"),
        ("timeout", "navigation_timeout"),
        ("extraction failed", "content_extraction_failed"),
        ("empty page", "empty_materialized_page"),
        (
            "browser_materialization_blocked",
            "browser_materialization_blocked",
        ),
        (
            "browser materialization blocked",
            "browser_materialization_blocked",
        ),
    ];
    for (needle, signal) in markers {
        if normalized.contains(needle) {
            failure_signals.push(signal.to_string());
            refs.push(path.to_string());
        }
    }
}

fn web_access_blocker_diagnostics(payload: &Value, retrieval_quality: &Value) -> Value {
    let mut signals = Vec::<String>::new();
    let mut refs = Vec::<String>::new();
    scan_access_blocker_signals(payload, "payload", &mut signals, &mut refs);
    scan_access_blocker_signals(
        retrieval_quality,
        "retrieval_quality",
        &mut signals,
        &mut refs,
    );
    signals.sort_unstable();
    signals.dedup();
    refs.sort_unstable();
    refs.dedup();

    let has_throttle = signals.iter().any(|signal| {
        matches!(
            signal.as_str(),
            "http_status_429"
                | "too_many_requests"
                | "rate_limit"
                | "retry_after"
                | "quota_exceeded"
                | "throttled"
        )
    });
    let has_bot_challenge = signals.iter().any(|signal| {
        matches!(
            signal.as_str(),
            "captcha_challenge"
                | "cloudflare_challenge"
                | "bot_detection"
                | "human_verification"
                | "waf_or_bot_wall"
        )
    });
    let has_auth = signals.iter().any(|signal| {
        matches!(
            signal.as_str(),
            "http_status_401" | "auth_required" | "login_required"
        )
    });
    let has_access_block = signals.iter().any(|signal| {
        matches!(
            signal.as_str(),
            "http_status_403" | "access_denied" | "request_blocked"
        )
    });
    let has_provider_config_missing = signals.iter().any(|signal| {
        matches!(
            signal.as_str(),
            "missing_api_key"
                | "invalid_api_key"
                | "missing_provider_credentials"
                | "provider_not_configured"
                | "strong_provider_missing"
        )
    });

    let kind = if has_provider_config_missing {
        "provider_configuration_missing"
    } else if has_throttle && has_bot_challenge {
        "anti_bot_or_throttle"
    } else if has_throttle {
        "throttle_or_rate_limit"
    } else if has_bot_challenge {
        "anti_bot_challenge"
    } else if has_access_block && !has_auth {
        "access_blocked"
    } else if has_auth {
        "permission_or_auth"
    } else {
        "none"
    };
    json!({
        "detected": kind != "none",
        "kind": kind,
        "classes": {
            "rate_limit_or_quota": has_throttle,
            "anti_bot_challenge": has_bot_challenge,
            "permission_or_auth": has_auth,
            "access_denied_or_forbidden": has_access_block,
            "provider_configuration_missing": has_provider_config_missing
        },
        "signals": signals,
        "artifact_refs": refs,
        "note": "General web-access blocker detection based on status/error/body markers such as 429, 403, CAPTCHA, bot-wall, WAF, Cloudflare challenge, rate limit, Retry-After, auth-required, or provider-configuration signals."
    })
}

fn scan_access_blocker_signals(
    value: &Value,
    path: &str,
    signals: &mut Vec<String>,
    refs: &mut Vec<String>,
) {
    if access_blocker_declarative_path(path) {
        return;
    }
    match value {
        Value::Null | Value::Bool(_) => {}
        Value::Number(raw) => {
            if let Some(code) = raw.as_u64().filter(|_| access_status_path(path)) {
                push_status_signal(code, path, signals, refs);
            }
        }
        Value::String(raw) => scan_access_blocker_text(raw, path, signals, refs),
        Value::Array(rows) => {
            for (index, row) in rows.iter().enumerate() {
                scan_access_blocker_signals(row, &format!("{path}.{index}"), signals, refs);
            }
        }
        Value::Object(map) => {
            for (key, child) in map {
                scan_access_blocker_signals(child, &format!("{path}.{key}"), signals, refs);
            }
        }
    }
}

fn access_blocker_declarative_path(path: &str) -> bool {
    let normalized = normalize_for_compare(&path.replace(['.', '_', '-'], " "));
    [
        "blocker taxonomy",
        "browser materialization profile compilation",
        "browser materialization readiness lifecycle",
        "browser materialization url safety",
        "browser materialization non goals",
        "source pattern",
        "tool cd",
        "tooling cd",
        "capability contract",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn scan_access_blocker_text(
    raw: &str,
    path: &str,
    signals: &mut Vec<String>,
    refs: &mut Vec<String>,
) {
    let normalized = normalize_for_compare(raw);
    let explicit_challenge_markers = [
        ("captcha", "captcha_challenge"),
        ("recaptcha", "captcha_challenge"),
        ("hcaptcha", "captcha_challenge"),
        ("cf-chl", "cloudflare_challenge"),
        ("cf-ray", "cloudflare_challenge"),
        ("checking your browser", "cloudflare_challenge"),
        ("verify you are human", "human_verification"),
        ("human verification", "human_verification"),
        (
            "please complete the following challenge",
            "human_verification",
        ),
        (
            "unfortunately bots use duckduckgo too",
            "human_verification",
        ),
        ("select all squares containing a duck", "human_verification"),
        ("unusual traffic", "bot_detection"),
        ("automated queries", "bot_detection"),
    ];
    let mut explicit_challenge_detected = false;
    for (needle, signal) in explicit_challenge_markers {
        if normalized.contains(needle) {
            explicit_challenge_detected = true;
            push_access_signal(signal, path, signals, refs);
        }
    }

    let contextual_bot_markers = [
        ("cloudflare", "cloudflare_challenge"),
        ("bot detection", "bot_detection"),
        ("anti-bot", "bot_detection"),
        ("anti bot", "bot_detection"),
        ("bot wall", "waf_or_bot_wall"),
        ("waf", "waf_or_bot_wall"),
        ("datadome", "waf_or_bot_wall"),
        ("perimeterx", "waf_or_bot_wall"),
        ("imperva", "waf_or_bot_wall"),
        ("incapsula", "waf_or_bot_wall"),
        ("distil networks", "waf_or_bot_wall"),
        ("ddos-guard", "waf_or_bot_wall"),
    ];
    if access_status_path(path) || explicit_challenge_detected {
        for (needle, signal) in contextual_bot_markers {
            if normalized.contains(needle) {
                push_access_signal(signal, path, signals, refs);
            }
        }
    }

    if !access_status_path(path) {
        return;
    }

    let status_markers = [
        ("429", "http_status_429"),
        ("too many requests", "too_many_requests"),
        ("rate limit", "rate_limit"),
        ("rate-limit", "rate_limit"),
        ("rate_limited", "rate_limit"),
        ("ratelimit", "rate_limit"),
        ("retry-after", "retry_after"),
        ("quota exceeded", "quota_exceeded"),
        ("throttled", "throttled"),
        ("throttle", "throttled"),
        ("missing api key", "missing_api_key"),
        ("api key missing", "missing_api_key"),
        ("invalid api key", "invalid_api_key"),
        (
            "missing provider credentials",
            "missing_provider_credentials",
        ),
        (
            "provider credentials missing",
            "missing_provider_credentials",
        ),
        ("provider not configured", "provider_not_configured"),
        ("strong search provider missing", "strong_provider_missing"),
        ("strong_provider_missing", "strong_provider_missing"),
        ("403", "http_status_403"),
        ("forbidden", "access_denied"),
        ("access denied", "access_denied"),
        ("request blocked", "request_blocked"),
        ("blocked by", "request_blocked"),
        ("401", "http_status_401"),
        ("unauthorized", "auth_required"),
        ("authentication required", "auth_required"),
        ("login required", "login_required"),
        ("sign in required", "login_required"),
    ];
    for (needle, signal) in status_markers {
        if normalized.contains(needle) {
            push_access_signal(signal, path, signals, refs);
        }
    }
}

fn access_status_path(path: &str) -> bool {
    let normalized = normalize_for_compare(&path.replace(['.', '_', '-'], " "));
    [
        "status",
        "status code",
        "http status",
        "http code",
        "error",
        "failure",
        "exception",
        "headers",
        "header",
        "retry after",
        "retry_after",
        "rate limit",
        "rate_limit",
        "access blocker",
        "blocker",
        "blocked",
        "provider config",
        "provider configured",
        "provider not configured",
        "api key",
        "credentials",
        "credential",
        "strong provider",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn push_status_signal(code: u64, path: &str, signals: &mut Vec<String>, refs: &mut Vec<String>) {
    match code {
        401 => push_access_signal("http_status_401", path, signals, refs),
        403 => push_access_signal("http_status_403", path, signals, refs),
        429 => push_access_signal("http_status_429", path, signals, refs),
        503 => push_access_signal("waf_or_bot_wall", path, signals, refs),
        _ => {}
    }
}

fn push_access_signal(signal: &str, path: &str, signals: &mut Vec<String>, refs: &mut Vec<String>) {
    signals.push(signal.to_string());
    refs.push(path.to_string());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn access_blocker_ignores_topic_mentions_in_evidence_snippets() {
        let payload = json!({
            "tools": [{
                "result": "Browser-agent security sources discuss Cloudflare challenge flows, WAF designs, and bot-detection countermeasures.",
                "evidence_pack": [{
                    "snippet": "Cloudflare bot detection and WAF controls are common topics in browser-agent security writeups."
                }]
            }]
        });
        let retrieval_quality = json!({
            "status": "usable",
            "usable_evidence": true
        });
        let blocker = web_access_blocker_diagnostics(&payload, &retrieval_quality);
        assert_eq!(
            blocker.get("detected").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            blocker
                .pointer("/classes/anti_bot_challenge")
                .and_then(Value::as_bool),
            Some(false)
        );
    }

    #[test]
    fn access_blocker_detects_real_challenge_copy_in_result_body() {
        let payload = json!({
            "tools": [{
                "result": "Unfortunately, bots use DuckDuckGo too. Please complete the following challenge to verify you are human. Cloudflare protection is active."
            }]
        });
        let blocker = web_access_blocker_diagnostics(&payload, &json!({}));
        assert_eq!(
            blocker
                .pointer("/classes/anti_bot_challenge")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            blocker.get("kind").and_then(Value::as_str),
            Some("anti_bot_challenge")
        );
    }

    #[test]
    fn excludes_post_tool_cases_when_only_derived_fallback_request_exists() {
        let case = json!({
            "category": "post_tool_synthesis"
        });
        let payload = json!({
            "pending_tool_request": {
                "input": {
                    "query": "After the web tool returns low-signal results...",
                    "query_metadata_policy": {
                        "classification": "derived_prompt_request"
                    }
                }
            },
            "tools": [{
                "status": "blocked"
            }]
        });
        let retrieval_quality = json!({
            "tool_executed": true,
            "status": "provider_degraded"
        });
        assert_eq!(
            web_tooling_measurement_exclusion_reason_case(&case, &payload, &retrieval_quality),
            Some("post_tool_context_not_seeded")
        );
    }

    #[test]
    fn provider_surface_gate_passes_when_degraded_provider_still_yields_materialized_evidence() {
        let payload = json!({
            "pending_tool_request": {
                "tool_key": "batch_query",
                "input": {
                    "query": "Find recent benchmarks comparing agent frameworks",
                    "queries": ["Find recent benchmarks comparing agent frameworks"],
                    "keywords": ["benchmarks", "agent frameworks"],
                    "required_coverage": {"entities": ["agent frameworks"], "facets": ["benchmarks"]}
                }
            },
            "tools": [{
                "status": "low_signal"
            }],
            "provider_results": [{
                "provider_raw_count": 126,
                "result_quality": "provider_error",
                "synthesis_candidate_count": 20
            }],
            "query_lane_attribution": {
                "rows": [{
                    "provider_raw_rows": 126,
                    "candidate_rows": 20,
                    "synthesis_candidate_rows": 6,
                    "filtered_or_rejected_rows": 19
                }]
            },
            "evidence_refs": [
                {"snippet": "benchmark writeup", "claim_hints": ["coverage gap"], "source_domain": "example.com"}
            ]
        });
        let retrieval_quality = json!({
            "status": "low_signal",
            "candidate_count": 20,
            "evidence_count": 6,
            "content_rich_candidate_count": 6,
            "materialized_candidate_count": 6,
            "claim_hint_count": 2,
            "usable_evidence": false,
            "quality_flags": ["explicit_low_signal_marker"]
        });
        let query_metadata = json!({
            "metadata_present": true,
            "rich_query_pack_or_narrow_marker": true
        });
        let transitions = json!({
            "checkpoints": [{
                "checkpoint": "5e_agent_received_evidence_context",
                "status": "pass"
            }]
        });
        let diag = web_retrieval_gate_diagnostics(
            &payload,
            &retrieval_quality,
            &query_metadata,
            &transitions,
        );
        let gate_4c = diag
            .get("gates")
            .and_then(Value::as_array)
            .and_then(|rows| {
                rows.iter().find(|row| {
                    row.get("gate").and_then(Value::as_str)
                        == Some("web_4c_search_provider_surface_ready")
                })
            })
            .cloned()
            .expect("web_4c gate");
        let gate_7 = diag
            .get("gates")
            .and_then(Value::as_array)
            .and_then(|rows| {
                rows.iter().find(|row| {
                    row.get("gate").and_then(Value::as_str)
                        == Some("web_7_usable_evidence_available")
                })
            })
            .cloned()
            .expect("web_7 gate");
        assert_eq!(gate_4c.get("status").and_then(Value::as_str), Some("pass"));
        assert_eq!(gate_7.get("status").and_then(Value::as_str), Some("fail"));
    }

    #[test]
    fn claim_extraction_gate_uses_direct_evidence_claim_contract_when_present() {
        let payload = json!({
            "pending_tool_request": {
                "tool_key": "batch_query",
                "input": {
                    "query": "Give me news from this week",
                    "queries": ["Give me news from this week"],
                    "keywords": ["news", "this week"]
                }
            },
            "tools": [{
                "status": "partial"
            }],
            "evidence_refs": [
                {
                    "title": "Generic current events page",
                    "snippet": "A thin row that should not count as claim-backed evidence.",
                    "claim_hints": ["Thin inferred hint"]
                }
            ]
        });
        let retrieval_quality = json!({
            "status": "usable",
            "candidate_count": 8,
            "evidence_count": 4,
            "content_rich_candidate_count": 4,
            "materialized_candidate_count": 4,
            "claim_hint_count": 6,
            "usable_evidence": false,
            "classification_inputs": {
                "direct_contract_present": true,
                "direct_evidence_claim_count": 0
            }
        });
        let query_metadata = json!({
            "metadata_present": true,
            "rich_query_pack_or_narrow_marker": true
        });
        let transitions = json!({
            "checkpoints": [{
                "checkpoint": "5e_agent_received_evidence_context",
                "status": "pass"
            }]
        });
        let diag = web_retrieval_gate_diagnostics(
            &payload,
            &retrieval_quality,
            &query_metadata,
            &transitions,
        );
        let gate_5c = diag
            .get("gates")
            .and_then(Value::as_array)
            .and_then(|rows| {
                rows.iter().find(|row| {
                    row.get("gate").and_then(Value::as_str)
                        == Some("web_5c_claim_extraction_present")
                })
            })
            .cloned()
            .expect("web_5c gate");
        assert_eq!(gate_5c.get("status").and_then(Value::as_str), Some("fail"));
        assert_eq!(
            diag.pointer("/first_failed_gate").and_then(Value::as_str),
            Some("web_5c_claim_extraction_present")
        );
        assert_eq!(
            diag.pointer("/operator_metrics/claim_extraction/direct_evidence_claim_count")
                .and_then(Value::as_u64),
            Some(0)
        );
    }

    #[test]
    fn evidence_quality_gates_reject_title_like_claim_fragments() {
        let payload = json!({
            "pending_tool_request": {
                "tool_key": "batch_query",
                "input": {
                    "query": "give me news from this week",
                    "queries": ["give me news from this week"],
                    "keywords": ["news", "this week"]
                }
            },
            "tools": [{
                "status": "usable"
            }],
            "evidence_refs": [{
                "title": "Cooler this Week",
                "locator": "https://www.wlns.com/weather/cooler-this-week/",
                "source_domain": "wlns.com",
                "snippet": "WLNS 6 News reported a local weather story published during the week with cooler temperatures expected in Lansing, Michigan.",
                "claim_hints": ["Cooler this Week"]
            }]
        });
        let retrieval_quality = json!({
            "status": "usable",
            "candidate_count": 4,
            "evidence_count": 1,
            "content_rich_candidate_count": 1,
            "materialized_candidate_count": 1,
            "claim_hint_count": 1,
            "usable_evidence": true
        });
        let query_metadata = json!({
            "metadata_present": true,
            "rich_query_pack_or_narrow_marker": true
        });
        let transitions = json!({
            "checkpoints": [{
                "checkpoint": "5e_agent_received_evidence_context",
                "status": "pass"
            }]
        });
        let diag = web_retrieval_gate_diagnostics(
            &payload,
            &retrieval_quality,
            &query_metadata,
            &transitions,
        );
        assert_eq!(
            diag.pointer("/first_failed_gate").and_then(Value::as_str),
            Some("web_5e_claim_quality_ready")
        );
        assert_eq!(
            diag.pointer("/evidence_quality/claim_quality_ready")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            diag.pointer("/evidence_quality/citation_renderability_ready")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn evidence_quality_gates_pass_clean_source_backed_claims() {
        let payload = json!({
            "pending_tool_request": {
                "tool_key": "batch_query",
                "input": {
                    "query": "compare web research APIs",
                    "queries": ["compare web research APIs"],
                    "keywords": ["web research APIs", "source links", "raw content"]
                }
            },
            "tools": [{
                "status": "usable"
            }],
            "evidence_refs": [{
                "title": "Search API documentation",
                "locator": "https://docs.example.com/search-api",
                "source_domain": "docs.example.com",
                "snippet": "The documentation describes a search API that returns answer-ready result objects with source links, snippets, and raw content fields for retrieval workflows.",
                "claim_hints": [
                    "The search API returns structured result objects with source links, snippets, and raw content fields for retrieval workflows."
                ]
            }]
        });
        let retrieval_quality = json!({
            "status": "usable",
            "candidate_count": 6,
            "evidence_count": 1,
            "content_rich_candidate_count": 1,
            "materialized_candidate_count": 1,
            "claim_hint_count": 1,
            "usable_evidence": true
        });
        let query_metadata = json!({
            "metadata_present": true,
            "rich_query_pack_or_narrow_marker": true
        });
        let transitions = json!({
            "checkpoints": [{
                "checkpoint": "5e_agent_received_evidence_context",
                "status": "pass"
            }]
        });
        let diag = web_retrieval_gate_diagnostics(
            &payload,
            &retrieval_quality,
            &query_metadata,
            &transitions,
        );
        assert_eq!(
            diag.pointer("/first_failed_gate").and_then(Value::as_str),
            None
        );
        for gate_name in [
            "web_5d_source_quality_ready",
            "web_5e_claim_quality_ready",
            "web_5f_citation_renderability_ready",
            "web_5g_answerability_ready",
            "web_7_usable_evidence_available",
        ] {
            let gate = diag
                .get("gates")
                .and_then(Value::as_array)
                .and_then(|rows| {
                    rows.iter()
                        .find(|row| row.get("gate").and_then(Value::as_str) == Some(gate_name))
                })
                .cloned()
                .unwrap_or_else(|| panic!("missing {gate_name}"));
            assert_eq!(
                gate.get("status").and_then(Value::as_str),
                Some("pass"),
                "{gate_name}: {gate:#?}"
            );
        }
    }

    #[test]
    fn nonblocking_provider_failures_do_not_mask_claim_extraction_gap() {
        let payload = json!({
            "pending_tool_request": {
                "tool_key": "batch_query",
                "input": {
                    "query": "Compare Alpha and Beta",
                    "queries": ["Compare Alpha and Beta"],
                    "keywords": ["Alpha", "Beta"]
                }
            },
            "tools": [{
                "status": "partial"
            }],
            "partial_failure_details": [
                "provider_circuit_open",
                "provider_error",
                "missing api key",
                "rate_limited",
                "captcha challenge"
            ],
            "provider_results": [{
                "provider_raw_count": 40,
                "candidate_rows": 12,
                "synthesis_candidate_count": 6,
                "result_quality": "provider_error"
            }],
            "evidence_refs": [{
                "title": "Alpha and Beta overview",
                "snippet": "The source names Alpha and Beta but does not expose a usable claim."
            }]
        });
        let retrieval_quality = json!({
            "status": "low_signal",
            "candidate_count": 12,
            "evidence_count": 4,
            "content_rich_candidate_count": 4,
            "materialized_candidate_count": 4,
            "claim_hint_count": 0,
            "usable_evidence": false
        });
        let query_metadata = json!({
            "metadata_present": true,
            "rich_query_pack_or_narrow_marker": true
        });
        let transitions = json!({
            "checkpoints": [{
                "checkpoint": "5e_agent_received_evidence_context",
                "status": "pass"
            }]
        });
        let diag = web_retrieval_gate_diagnostics(
            &payload,
            &retrieval_quality,
            &query_metadata,
            &transitions,
        );
        let gates = diag
            .get("gates")
            .and_then(Value::as_array)
            .cloned()
            .expect("gates");
        for name in [
            "web_3b1_provider_quota_not_rate_limited",
            "web_3b2_no_bot_challenge_or_waf",
            "web_3b5_provider_configuration_available",
            "web_3b_access_not_blocked_or_throttled",
            "web_4b_search_provider_circuit_closed",
            "web_4c_search_provider_surface_ready",
        ] {
            let gate = gates
                .iter()
                .find(|row| row.get("gate").and_then(Value::as_str) == Some(name))
                .cloned()
                .unwrap_or_else(|| panic!("missing {name}"));
            assert_eq!(
                gate.get("status").and_then(Value::as_str),
                Some("pass"),
                "{name}: {gate:#?}"
            );
        }
        assert_eq!(
            diag.pointer("/first_failed_gate").and_then(Value::as_str),
            Some("web_5c_claim_extraction_present")
        );
    }

    #[test]
    fn operator_metrics_surface_materialization_failure_reason() {
        let payload = json!({
            "pending_tool_request": {
                "tool_key": "batch_query",
                "input": {
                    "query": "Find recent benchmarks comparing agent frameworks"
                }
            },
            "tools": [{
                "status": "low_signal"
            }]
        });
        let retrieval_quality = json!({
            "status": "low_signal",
            "candidate_count": 12,
            "evidence_count": 4,
            "content_rich_candidate_count": 0,
            "materialized_candidate_count": 0,
            "claim_hint_count": 0,
            "usable_evidence": false,
            "materialization_failure_report": {
                "top_reason": {"reason": "content_too_thin", "count": 4},
                "reason_rows": [
                    {"reason": "content_too_thin", "count": 4}
                ]
            }
        });
        let query_metadata = json!({
            "metadata_present": true,
            "rich_query_pack_or_narrow_marker": true
        });
        let transitions = json!({
            "checkpoints": [{
                "checkpoint": "5e_agent_received_evidence_context",
                "status": "pass"
            }]
        });
        let diag = web_retrieval_gate_diagnostics(
            &payload,
            &retrieval_quality,
            &query_metadata,
            &transitions,
        );
        assert_eq!(
            diag.pointer("/operator_metrics/materialization/top_failure_reason")
                .and_then(Value::as_str),
            Some("content_too_thin")
        );
    }

    #[test]
    fn browser_materialization_gate_ignores_nonblocking_enrichment_failures() {
        let payload = json!({
            "pending_tool_request": {
                "tool_key": "batch_query",
                "input": {
                    "query": "Compare browser agents"
                }
            },
            "tools": [{
                "status": "partial"
            }]
        });
        let retrieval_quality = json!({
            "status": "low_signal",
            "candidate_count": 18,
            "evidence_count": 3,
            "content_rich_candidate_count": 0,
            "materialized_candidate_count": 0,
            "claim_hint_count": 0,
            "usable_evidence": false,
            "browser_materialization": {
                "attempted": true,
                "failed": true
            },
            "materialization_failure_report": {
                "top_reason": {"reason": "prefetch_rejected_off_intent", "count": 8},
                "reason_rows": [
                    {"reason": "prefetch_rejected_off_intent", "count": 8},
                    {"reason": "browser_materialization_failed", "count": 2}
                ]
            }
        });
        let query_metadata = json!({
            "metadata_present": true,
            "rich_query_pack_or_narrow_marker": true
        });
        let transitions = json!({
            "checkpoints": [{
                "checkpoint": "5e_agent_received_evidence_context",
                "status": "pass"
            }]
        });
        let diag = web_retrieval_gate_diagnostics(
            &payload,
            &retrieval_quality,
            &query_metadata,
            &transitions,
        );
        let gate_3d = diag
            .get("gates")
            .and_then(Value::as_array)
            .and_then(|rows| {
                rows.iter().find(|row| {
                    row.get("gate").and_then(Value::as_str)
                        == Some("web_3d_browser_materialization_not_failed")
                })
            })
            .cloned()
            .expect("web_3d gate");
        assert_eq!(gate_3d.get("status").and_then(Value::as_str), Some("pass"));
    }

    #[test]
    fn browser_materialization_gate_allows_usable_retrieval_even_if_browser_failed() {
        let payload = json!({
            "pending_tool_request": {
                "tool_key": "batch_query",
                "input": {
                    "query": "Compare OpenAI Agents SDK with LangChain/LangGraph"
                }
            },
            "tools": [{
                "status": "partial"
            }],
            "evidence_refs": [{
                "title": "Web research API comparison",
                "locator": "https://docs.example.com/web-research-api-comparison",
                "source_domain": "docs.example.com",
                "snippet": "The comparison describes differences between web research APIs, including search result structure, crawling support, and source citation handling for agent workflows.",
                "claim_hints": [
                    "The comparison describes differences between web research APIs across result structure, crawling support, and citation handling for agent workflows."
                ]
            }]
        });
        let retrieval_quality = json!({
            "status": "usable",
            "candidate_count": 14,
            "evidence_count": 8,
            "content_rich_candidate_count": 11,
            "materialized_candidate_count": 8,
            "claim_hint_count": 10,
            "usable_evidence": true,
            "browser_materialization": {
                "attempted": true,
                "failed": true
            },
            "materialization_failure_report": {
                "top_reason": {"reason": "browser_materialization_failed", "count": 22},
                "reason_rows": [
                    {"reason": "browser_materialization_failed", "count": 22}
                ]
            }
        });
        let query_metadata = json!({
            "metadata_present": true,
            "rich_query_pack_or_narrow_marker": true
        });
        let transitions = json!({
            "checkpoints": [{
                "checkpoint": "5e_agent_received_evidence_context",
                "status": "pass"
            }]
        });
        let diag = web_retrieval_gate_diagnostics(
            &payload,
            &retrieval_quality,
            &query_metadata,
            &transitions,
        );
        let gate_3d = diag
            .get("gates")
            .and_then(Value::as_array)
            .and_then(|rows| {
                rows.iter().find(|row| {
                    row.get("gate").and_then(Value::as_str)
                        == Some("web_3d_browser_materialization_not_failed")
                })
            })
            .cloned()
            .expect("web_3d gate");
        assert_eq!(gate_3d.get("status").and_then(Value::as_str), Some("pass"));
    }

    #[test]
    fn browser_materialization_gate_allows_relevance_failure_after_content_arrived() {
        let payload = json!({
            "pending_tool_request": {
                "tool_key": "batch_query",
                "input": {
                    "query": "Compare Firecrawl, Tavily, and Exa"
                }
            },
            "tools": [{
                "status": "partial"
            }],
            "evidence_refs": [{
                "title": "Web research API comparison",
                "locator": "https://docs.example.com/web-research-api-comparison",
                "source_domain": "docs.example.com",
                "snippet": "The comparison describes differences between web research APIs, including search result structure, crawling support, and source citation handling for agent workflows.",
                "claim_hints": [
                    "The comparison describes differences between web research APIs across result structure, crawling support, and citation handling for agent workflows."
                ]
            }]
        });
        let retrieval_quality = json!({
            "status": "low_relevance",
            "candidate_count": 41,
            "evidence_count": 1,
            "content_rich_candidate_count": 1,
            "materialized_candidate_count": 1,
            "claim_hint_count": 1,
            "usable_evidence": false,
            "browser_materialization": {
                "attempted": true,
                "failed": true
            },
            "materialization_failure_report": {
                "top_reason": {"reason": "browser_materialization_failed", "count": 9},
                "reason_rows": [
                    {"reason": "browser_materialization_failed", "count": 9}
                ]
            }
        });
        let query_metadata = json!({
            "metadata_present": true,
            "rich_query_pack_or_narrow_marker": true
        });
        let transitions = json!({
            "checkpoints": [{
                "checkpoint": "5e_agent_received_evidence_context",
                "status": "pass"
            }]
        });
        let diag = web_retrieval_gate_diagnostics(
            &payload,
            &retrieval_quality,
            &query_metadata,
            &transitions,
        );
        let gate_3d = diag
            .get("gates")
            .and_then(Value::as_array)
            .and_then(|rows| {
                rows.iter().find(|row| {
                    row.get("gate").and_then(Value::as_str)
                        == Some("web_3d_browser_materialization_not_failed")
                })
            })
            .cloned()
            .expect("web_3d gate");
        assert_eq!(gate_3d.get("status").and_then(Value::as_str), Some("pass"));
        assert_eq!(
            diag.get("first_failed_gate").and_then(Value::as_str),
            Some("web_7_usable_evidence_available")
        );
    }

    #[test]
    fn access_and_anti_bot_gates_allow_recovered_usable_retrieval() {
        let payload = json!({
            "pending_tool_request": {
                "tool_key": "batch_query",
                "input": {
                    "query": "Research MCP maturity"
                }
            },
            "tools": [{
                "status": "partial",
                "result": "Unfortunately, bots use DuckDuckGo too. Please complete the following challenge to verify you are human."
            }]
        });
        let retrieval_quality = json!({
            "status": "usable",
            "candidate_count": 20,
            "evidence_count": 12,
            "content_rich_candidate_count": 9,
            "materialized_candidate_count": 6,
            "claim_hint_count": 7,
            "usable_evidence": true,
            "browser_materialization": {
                "attempted": true,
                "failed": false
            }
        });
        let query_metadata = json!({
            "metadata_present": true,
            "rich_query_pack_or_narrow_marker": true
        });
        let transitions = json!({
            "checkpoints": [{
                "checkpoint": "5e_agent_received_evidence_context",
                "status": "pass"
            }]
        });
        let diag = web_retrieval_gate_diagnostics(
            &payload,
            &retrieval_quality,
            &query_metadata,
            &transitions,
        );
        let gates = diag
            .get("gates")
            .and_then(Value::as_array)
            .cloned()
            .expect("gates");
        let gate_3b2 = gates
            .iter()
            .find(|row| {
                row.get("gate").and_then(Value::as_str) == Some("web_3b2_no_bot_challenge_or_waf")
            })
            .cloned()
            .expect("web_3b2 gate");
        let gate_3b = gates
            .iter()
            .find(|row| {
                row.get("gate").and_then(Value::as_str)
                    == Some("web_3b_access_not_blocked_or_throttled")
            })
            .cloned()
            .expect("web_3b access gate");
        assert_eq!(gate_3b2.get("status").and_then(Value::as_str), Some("pass"));
        assert_eq!(gate_3b.get("status").and_then(Value::as_str), Some("pass"));
    }
}
