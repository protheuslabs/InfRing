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
    evidence_packet_contract_ready_cases: u64,
    evidence_item_count_total: u64,
    low_quality_evidence_item_count_total: u64,
    quality_claim_count_total: u64,
    concrete_claim_count_total: u64,
    citation_ready_claim_count_total: u64,
    handoff_claim_count_total: u64,
    handoff_concrete_claim_count_total: u64,
    handoff_low_quality_claim_count_total: u64,
    handoff_citation_ready_claim_count_total: u64,
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
            "evidence_packet_contract_ready_case_rate": ratio(evidence_packet_contract_ready_cases, measured_cases),
            "low_quality_evidence_item_rate": ratio(
                low_quality_evidence_item_count_total,
                evidence_item_count_total
            ),
            "concrete_claim_rate": ratio(concrete_claim_count_total, quality_claim_count_total),
            "citation_ready_claim_rate": ratio(
                citation_ready_claim_count_total,
                quality_claim_count_total
            ),
            "handoff_concrete_claim_rate": ratio(
                handoff_concrete_claim_count_total,
                handoff_claim_count_total
            ),
            "handoff_low_quality_claim_rate": ratio(
                handoff_low_quality_claim_count_total,
                handoff_claim_count_total
            ),
            "handoff_citation_ready_claim_rate": ratio(
                handoff_citation_ready_claim_count_total,
                handoff_claim_count_total
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
            "handoff_concrete_claim_rate": "Share of promoted evidence_claims that looked like concrete answer material.",
            "handoff_low_quality_claim_rate": "Share of promoted evidence_claims that still looked like malformed fragments, headings, source labels, or boilerplate.",
            "handoff_citation_ready_claim_rate": "Share of promoted evidence_claims that retained enough locator/title/domain data to render citations.",
            "citation_renderability_ready_case_rate": "Share of cases where claim/evidence material retained enough locator/title/domain data to render citations.",
            "answerability_ready_case_rate": "Share of cases where clean evidence, concrete claims, and citation data all existed together.",
            "evidence_packet_contract_ready_case_rate": "Share of cases where answerable evidence also preserved source identity, source type, useful extract, concrete claim material, and query-relevance rationale.",
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
        "tool_transport_failed" => {
            "web tool attempt was made, but the dashboard/tool transport timed out or failed before returning a usable payload".to_string()
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
        "evidence_packet_contract_not_ready" => {
            "evidence can look answerable, but selected packets do not preserve the fields a chat response needs to cite and explain the answer".to_string()
        }
        "malformed_evidence_fragments_present" => {
            "selected evidence carries stitched title tails, page chrome, or clipped fragments that would make the final answer read like source debris".to_string()
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
        "web_3a_tool_transport_completed" => "tool_transport",
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
        | "web_4e_browser_serp_external_urls_extracted"
        | "web_4e_provider_candidates_survive_filtering"
        | "web_4_raw_candidates_present"
        | "web_6_provider_not_empty_or_degraded" => "candidate_supply",
        "web_5_packaged_evidence_present"
        | "web_5d_source_quality_ready"
        | "web_5f_citation_renderability_ready"
        | "web_5g_answerability_ready"
        | "web_5h_evidence_packet_contract_ready"
        | "web_5i_malformed_evidence_absent"
        | "web_5j_citation_titles_clean"
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
        "tool_transport_failed" => {
            "make direct web tooling calls return structured timeout/partial payloads before tuning provider ranking or synthesis"
        }
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
        "browser_serp_no_external_organic_urls" => {
            "keep browser SERP explicit/canary and improve its organic URL extraction before default promotion"
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
        "evidence_packet_contract_not_ready" => {
            "preserve source identity, source type, relevant extracts, concrete claim material, and query-relevance rationale in each selected evidence packet"
        }
        "malformed_evidence_fragments_present" => {
            "tighten extraction and evidence cleaning so selected evidence contains answer material, not stitched titles or page chrome"
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
