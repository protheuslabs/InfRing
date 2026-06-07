// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (thin adapter to core/layer2/ops retrieval policy).

#[derive(Clone, Debug)]
struct ResearchFacet {
    id: String,
    requested_text: String,
    kind: String,
    terms: HashSet<String>,
    distinctive_terms: HashSet<String>,
}

fn candidate_to_policy_value(candidate: &Candidate) -> Value {
    json!({
        "source_kind": candidate.source_kind,
        "title": candidate.title,
        "locator": candidate.locator,
        "snippet": candidate.snippet,
        "excerpt_hash": candidate.excerpt_hash,
        "timestamp": candidate.timestamp,
        "permissions": candidate.permissions,
        "status_code": candidate.status_code
    })
}

fn candidate_from_policy_value(value: &Value) -> Candidate {
    Candidate {
        source_kind: clean_text(value.get("source_kind").and_then(Value::as_str).unwrap_or("web"), 120),
        title: clean_text(value.get("title").and_then(Value::as_str).unwrap_or(""), 240),
        locator: clean_text(value.get("locator").and_then(Value::as_str).unwrap_or(""), 2_200),
        snippet: clean_text(value.get("snippet").and_then(Value::as_str).unwrap_or(""), 4_000),
        excerpt_hash: clean_text(value.get("excerpt_hash").and_then(Value::as_str).unwrap_or(""), 128),
        timestamp: value
            .get("timestamp")
            .and_then(Value::as_str)
            .map(|raw| clean_text(raw, 80))
            .filter(|raw| !raw.is_empty()),
        permissions: value
            .get("permissions")
            .and_then(Value::as_str)
            .map(|raw| clean_text(raw, 240))
            .filter(|raw| !raw.is_empty()),
        status_code: value.get("status_code").and_then(Value::as_i64).unwrap_or(0),
    }
}

fn candidate_rows_from_policy_value(value: Value) -> Vec<Candidate> {
    value
        .as_array()
        .map(|rows| rows.iter().map(candidate_from_policy_value).collect::<Vec<_>>())
        .unwrap_or_default()
}

fn ranked_to_policy_value(rows: &[(Candidate, f64)]) -> Value {
    Value::Array(
        rows.iter()
            .map(|(candidate, score)| {
                json!({
                    "candidate": candidate_to_policy_value(candidate),
                    "score": score
                })
            })
            .collect(),
    )
}

fn candidates_to_policy_value(rows: &[Candidate]) -> Value {
    Value::Array(rows.iter().map(candidate_to_policy_value).collect())
}

fn ranked_from_policy_value(value: Value) -> Vec<(Candidate, f64)> {
    value
        .as_array()
        .map(|rows| {
            rows.iter()
                .map(|row| {
                    let candidate = row
                        .get("candidate")
                        .map(candidate_from_policy_value)
                        .unwrap_or_else(|| candidate_from_policy_value(row));
                    let score = row.get("score").and_then(Value::as_f64).unwrap_or(0.0);
                    (candidate, score)
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn string_set_from_policy_value(value: Option<&Value>) -> HashSet<String> {
    value
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|raw| clean_text(raw, 240))
                .filter(|raw| !raw.is_empty())
                .collect::<HashSet<_>>()
        })
        .unwrap_or_default()
}

fn facet_to_policy_value(facet: &ResearchFacet) -> Value {
    json!({
        "id": facet.id,
        "requested_text": facet.requested_text,
        "kind": facet.kind,
        "terms": facet.terms.iter().cloned().collect::<Vec<_>>(),
        "distinctive_terms": facet.distinctive_terms.iter().cloned().collect::<Vec<_>>()
    })
}

fn facet_from_policy_value(value: &Value) -> ResearchFacet {
    ResearchFacet {
        id: clean_text(value.get("id").and_then(Value::as_str).unwrap_or(""), 80),
        requested_text: clean_text(
            value.get("requested_text").and_then(Value::as_str).unwrap_or(""),
            240,
        ),
        kind: clean_text(value.get("kind").and_then(Value::as_str).unwrap_or("inferred"), 80),
        terms: string_set_from_policy_value(value.get("terms")),
        distinctive_terms: string_set_from_policy_value(value.get("distinctive_terms")),
    }
}

fn facets_to_policy_value(facets: &[ResearchFacet]) -> Value {
    Value::Array(facets.iter().map(facet_to_policy_value).collect())
}

fn facets_from_policy_value(value: Value) -> Vec<ResearchFacet> {
    value
        .as_array()
        .map(|rows| rows.iter().map(facet_from_policy_value).collect::<Vec<_>>())
        .unwrap_or_default()
}

fn aperture_budget_to_policy_value(budget: ApertureBudget) -> Value {
    #[cfg(not(test))]
    {
        json!({
            "max_candidates": budget.max_candidates,
            "max_evidence": budget.max_evidence,
            "max_summary_tokens": budget.max_summary_tokens
        })
    }
    #[cfg(test)]
    {
        let mut value = json!({
            "max_candidates": budget.max_candidates,
            "max_evidence": budget.max_evidence,
            "max_summary_tokens": budget.max_summary_tokens
        });
        value["max_query_rewrites"] = json!(budget.max_query_rewrites);
        value
    }
}

fn current_web_intent(query: &str) -> bool {
    infring_nexus_core_v1::ops_core::retrieval_policy::current_web_intent_api(query)
}

fn current_year() -> String {
    infring_nexus_core_v1::ops_core::retrieval_policy::current_year_api()
}

fn candidates_from_structured_search_payload(
    query: &str,
    payload: &Value,
    max_rows: usize,
) -> Vec<Candidate> {
    candidate_rows_from_policy_value(
        infring_nexus_core_v1::ops_core::retrieval_policy::candidates_from_structured_search_payload_api(
            query, payload, max_rows,
        ),
    )
}

fn candidates_from_rendered_search_payload(
    query: &str,
    payload: &Value,
    max_rows: usize,
) -> Vec<Candidate> {
    candidate_rows_from_policy_value(
        infring_nexus_core_v1::ops_core::retrieval_policy::candidates_from_rendered_search_payload_api(
            query, payload, max_rows,
        ),
    )
}

fn infer_research_facets(
    query: &str,
    query_plan: &[String],
    query_metadata: &BatchQueryKeywordPack,
    policy: &Value,
    budget: ApertureBudget,
) -> Vec<ResearchFacet> {
    facets_from_policy_value(infring_nexus_core_v1::ops_core::retrieval_policy::infer_research_facets_api(
        query,
        query_plan,
        &query_metadata.to_value(),
        policy,
        &aperture_budget_to_policy_value(budget),
    ))
}

fn coverage_aware_max_evidence(facets: &[ResearchFacet], budget: ApertureBudget) -> usize {
    infring_nexus_core_v1::ops_core::retrieval_policy::coverage_aware_max_evidence_api(
        &facets_to_policy_value(facets),
        &aperture_budget_to_policy_value(budget),
    )
}

fn select_facet_covered_ranked_candidates(
    ranked: Vec<(Candidate, f64)>,
    facets: &[ResearchFacet],
    max_evidence: usize,
    min_terms: usize,
) -> Vec<(Candidate, f64)> {
    ranked_from_policy_value(
        infring_nexus_core_v1::ops_core::retrieval_policy::select_facet_covered_ranked_candidates_api(
            &ranked_to_policy_value(&ranked),
            &facets_to_policy_value(facets),
            max_evidence,
            min_terms,
        ),
    )
}

fn candidate_identity_key(candidate: &Candidate) -> String {
    infring_nexus_core_v1::ops_core::retrieval_policy::candidate_identity_key_api(&candidate_to_policy_value(
        candidate,
    ))
}

fn selected_candidate_coverage_ids(
    facets: &[ResearchFacet],
    candidate: &Candidate,
    min_terms: usize,
) -> Vec<String> {
    infring_nexus_core_v1::ops_core::retrieval_policy::selected_candidate_coverage_ids_api(
        &facets_to_policy_value(facets),
        &candidate_to_policy_value(candidate),
        min_terms,
    )
}

fn candidate_coverage_facets(
    facets: &[ResearchFacet],
    candidate: &Candidate,
    min_terms: usize,
) -> Vec<String> {
    infring_nexus_core_v1::ops_core::retrieval_policy::candidate_coverage_facets_api(
        &facets_to_policy_value(facets),
        &candidate_to_policy_value(candidate),
        min_terms,
    )
}

fn candidate_retention_preview_eligible(query: &str, candidate: &Candidate, score: f64) -> bool {
    infring_nexus_core_v1::ops_core::retrieval_policy::candidate_retention_preview_eligible_api(
        query,
        &candidate_to_policy_value(candidate),
        score,
    )
}

fn content_rich_text(text: &str) -> bool {
    infring_nexus_core_v1::ops_core::retrieval_policy::content_rich_text_api(text)
}

fn source_trust_adjustment(candidate: &Candidate) -> f64 {
    infring_nexus_core_v1::ops_core::retrieval_policy::source_trust_adjustment_api(&candidate_to_policy_value(
        candidate,
    ))
}

fn redundant_facet_backfill_replacement_index(
    selected: &[(Candidate, f64)],
    facets: &[ResearchFacet],
    min_terms: usize,
) -> Option<usize> {
    infring_nexus_core_v1::ops_core::retrieval_policy::redundant_facet_backfill_replacement_index_api(
        &ranked_to_policy_value(selected),
        &facets_to_policy_value(facets),
        min_terms,
    )
}

fn evidence_pack_from_ranked_candidates(
    policy: &Value,
    query: &str,
    facets: &[ResearchFacet],
    min_terms: usize,
    actionable_ranked: &[(Candidate, f64)],
    max_evidence: usize,
) -> Value {
    infring_nexus_core_v1::ops_core::retrieval_policy::evidence_pack_from_ranked_candidates_api(
        policy,
        query,
        &facets_to_policy_value(facets),
        min_terms,
        &ranked_to_policy_value(actionable_ranked),
        max_evidence,
    )
}

fn web_tool_quality_report(
    query: &str,
    status: &str,
    candidate_count: usize,
    evidence_count: usize,
    partial_failures: &[String],
    hard_partial_failures: &[String],
    actionable_ranked: &[(Candidate, f64)],
) -> Value {
    infring_nexus_core_v1::ops_core::retrieval_policy::web_tool_quality_report_api(
        query,
        status,
        candidate_count,
        evidence_count,
        partial_failures,
        hard_partial_failures,
        &ranked_to_policy_value(actionable_ranked),
    )
}

fn cached_web_tool_quality_report(
    query: &str,
    status: &str,
    partial_failure_details: &Value,
    evidence_refs: &Value,
) -> Value {
    infring_nexus_core_v1::ops_core::retrieval_policy::cached_web_tool_quality_report_api(
        query,
        status,
        partial_failure_details,
        evidence_refs,
    )
}

fn web_tool_quality_version() -> &'static str {
    "web_tool_quality_v11"
}

fn segment_has_current_signal(text: &str) -> bool {
    infring_nexus_core_v1::ops_core::retrieval_policy::segment_has_current_signal_api(text)
}

fn recency_adjustment(query: &str, candidate: &Candidate) -> f64 {
    infring_nexus_core_v1::ops_core::retrieval_policy::recency_adjustment_api(
        query,
        &candidate_to_policy_value(candidate),
    )
}

fn candidate_quality_flags(query: &str, candidate: &Candidate, score: f64) -> Vec<String> {
    infring_nexus_core_v1::ops_core::retrieval_policy::candidate_quality_flags_api(
        query,
        &candidate_to_policy_value(candidate),
        score,
    )
}

fn select_pack_ready_ranked_candidates(
    query: &str,
    ranked: Vec<(Candidate, f64)>,
    facets: &[ResearchFacet],
    max_evidence: usize,
    min_terms: usize,
) -> Vec<(Candidate, f64)> {
    ranked_from_policy_value(
        infring_nexus_core_v1::ops_core::retrieval_policy::select_pack_ready_ranked_candidates_api(
            query,
            &ranked_to_policy_value(&ranked),
            &facets_to_policy_value(facets),
            max_evidence,
            min_terms,
        ),
    )
}

fn research_facet_from_metadata_text(
    text: &str,
    index: usize,
    kind: &str,
) -> Option<ResearchFacet> {
    let value = infring_nexus_core_v1::ops_core::retrieval_policy::research_facet_from_metadata_text_api(
        text, index, kind,
    );
    (!value.is_null()).then(|| facet_from_policy_value(&value))
}

fn assign_distinctive_facet_terms(facets: &mut [ResearchFacet]) {
    let updated = facets_from_policy_value(
        infring_nexus_core_v1::ops_core::retrieval_policy::assign_distinctive_facet_terms_api(
            &facets_to_policy_value(facets),
        ),
    );
    for (target, source) in facets.iter_mut().zip(updated.into_iter()) {
        *target = source;
    }
}

fn candidate_matches_facet(
    facet: &ResearchFacet,
    candidate: &Candidate,
    min_terms: usize,
) -> bool {
    infring_nexus_core_v1::ops_core::retrieval_policy::candidate_matches_facet_api(
        &facet_to_policy_value(facet),
        &candidate_to_policy_value(candidate),
        min_terms,
    )
}

fn coverage_aware_score(
    query: &str,
    facets: &[ResearchFacet],
    candidate: &Candidate,
    min_terms: usize,
) -> f64 {
    infring_nexus_core_v1::ops_core::retrieval_policy::coverage_aware_score_api(
        query,
        &facets_to_policy_value(facets),
        &candidate_to_policy_value(candidate),
        min_terms,
    )
}

fn backfill_missing_facet_ranked_candidates(
    query: &str,
    selected: &mut Vec<(Candidate, f64)>,
    supplemental_pool: &[(Candidate, f64)],
    facets: &[ResearchFacet],
    max_evidence: usize,
    min_terms: usize,
    allow_low_confidence: bool,
) -> usize {
    let value = infring_nexus_core_v1::ops_core::retrieval_policy::backfill_missing_facet_ranked_candidates_api(
        query,
        &ranked_to_policy_value(selected),
        &ranked_to_policy_value(supplemental_pool),
        &facets_to_policy_value(facets),
        max_evidence,
        min_terms,
        allow_low_confidence,
    );
    *selected = ranked_from_policy_value(value.get("selected").cloned().unwrap_or_else(|| json!([])));
    value.get("added").and_then(Value::as_u64).unwrap_or(0) as usize
}

fn truncate_candidates_preserving_facet_coverage(
    query: &str,
    facets: &[ResearchFacet],
    candidates: &mut Vec<Candidate>,
    max_candidates: usize,
    min_terms: usize,
) {
    *candidates = candidate_rows_from_policy_value(
        infring_nexus_core_v1::ops_core::retrieval_policy::truncate_candidates_preserving_facet_coverage_api(
            query,
            &facets_to_policy_value(facets),
            &candidates_to_policy_value(candidates),
            max_candidates,
            min_terms,
        ),
    );
}

fn evidence_coverage_from_ranked_candidates(
    query: &str,
    facets: &[ResearchFacet],
    evidence_ranked: &[(Candidate, f64)],
    min_terms: usize,
) -> Value {
    infring_nexus_core_v1::ops_core::retrieval_policy::evidence_coverage_from_ranked_candidates_api(
        query,
        &facets_to_policy_value(facets),
        &ranked_to_policy_value(evidence_ranked),
        min_terms,
    )
}

fn ranked_evidence_covers_all_facets(
    query: &str,
    facets: &[ResearchFacet],
    evidence_ranked: &[(Candidate, f64)],
    min_terms: usize,
) -> bool {
    infring_nexus_core_v1::ops_core::retrieval_policy::ranked_evidence_covers_all_facets_api(
        query,
        &facets_to_policy_value(facets),
        &ranked_to_policy_value(evidence_ranked),
        min_terms,
    )
}

fn evidence_selection_diagnostics(
    query: &str,
    ranked_pool: &[(Candidate, f64)],
    actionable_ranked_pool: &[(Candidate, f64)],
    evidence_ranked: &[(Candidate, f64)],
    limit: usize,
) -> Value {
    infring_nexus_core_v1::ops_core::retrieval_policy::evidence_selection_diagnostics_api(
        query,
        &ranked_to_policy_value(ranked_pool),
        &ranked_to_policy_value(actionable_ranked_pool),
        &ranked_to_policy_value(evidence_ranked),
        limit,
    )
}

fn coverage_gap_recovery_queries(
    policy: &Value,
    query: &str,
    existing_queries: &[String],
    facets: &[ResearchFacet],
    candidates: &[Candidate],
    budget: ApertureBudget,
) -> Vec<String> {
    infring_nexus_core_v1::ops_core::retrieval_policy::coverage_gap_recovery_queries_api(
        policy,
        query,
        existing_queries,
        &facets_to_policy_value(facets),
        &candidates_to_policy_value(candidates),
        &aperture_budget_to_policy_value(budget),
    )
}

fn claim_gap_recovery_queries(
    policy: &Value,
    query: &str,
    existing_queries: &[String],
    facets: &[ResearchFacet],
    candidates: &[Candidate],
    budget: ApertureBudget,
) -> Vec<String> {
    infring_nexus_core_v1::ops_core::retrieval_policy::claim_gap_recovery_queries_api(
        policy,
        query,
        existing_queries,
        &facets_to_policy_value(facets),
        &candidates_to_policy_value(candidates),
        &aperture_budget_to_policy_value(budget),
    )
}

fn broad_current_research_lacks_synthesis_breadth(
    policy: &Value,
    query: &str,
    query_metadata: &BatchQueryKeywordPack,
    candidates: &[Candidate],
    budget: ApertureBudget,
) -> bool {
    infring_nexus_core_v1::ops_core::retrieval_policy::broad_current_research_lacks_synthesis_breadth_api(
        policy,
        query,
        &query_metadata.to_value(),
        &candidates_to_policy_value(candidates),
        &aperture_budget_to_policy_value(budget),
    )
}

fn candidate_has_non_evidence_payload(candidate: &Candidate) -> bool {
    infring_nexus_core_v1::ops_core::retrieval_policy::candidate_has_non_evidence_payload_api(
        &candidate_to_policy_value(candidate),
    )
}

fn candidate_counts_as_query_usable_evidence(
    query: &str,
    candidate: &Candidate,
    score: f64,
) -> bool {
    infring_nexus_core_v1::ops_core::retrieval_policy::candidate_counts_as_query_usable_evidence_api(
        query,
        &candidate_to_policy_value(candidate),
        score,
    )
}

fn has_pack_ready_synthesis_candidate(query: &str, candidates: &[Candidate]) -> bool {
    infring_nexus_core_v1::ops_core::retrieval_policy::has_pack_ready_synthesis_candidate_api(
        query,
        &candidates_to_policy_value(candidates),
    )
}

fn has_pack_ready_synthesis_source_quality(query: &str, candidates: &[Candidate]) -> bool {
    infring_nexus_core_v1::ops_core::retrieval_policy::has_pack_ready_synthesis_source_quality_api(
        query,
        &candidates_to_policy_value(candidates),
    )
}

fn claim_hint_normalized_snippet(snippet: &str) -> String {
    infring_nexus_core_v1::ops_core::retrieval_policy::claim_hint_normalized_snippet_api(snippet)
}

fn claim_text_is_synthesis_safe(text: &str) -> bool {
    infring_nexus_core_v1::ops_core::retrieval_policy::claim_text_is_synthesis_safe_api(text)
}

fn looks_like_link_directory_or_aggregator_shell(text: &str) -> bool {
    infring_nexus_core_v1::ops_core::retrieval_policy::looks_like_link_directory_or_aggregator_shell_api(text)
}

fn evidence_claims_from_pack(
    query_metadata: &BatchQueryKeywordPack,
    evidence_pack: &Value,
    limit: usize,
) -> Value {
    infring_nexus_core_v1::ops_core::retrieval_policy::evidence_claims_from_pack_api(
        &query_metadata.to_value(),
        evidence_pack,
        limit,
    )
}

fn evidence_pack_quality_report(
    policy: &Value,
    evidence_pack: &Value,
    evidence_coverage: &Value,
) -> Value {
    infring_nexus_core_v1::ops_core::retrieval_policy::evidence_pack_quality_report_api(
        policy,
        evidence_pack,
        evidence_coverage,
    )
}

fn source_class_coverage_from_evidence_pack(
    policy: &Value,
    query: &str,
    evidence_pack: &Value,
    evidence_coverage: &Value,
) -> Value {
    infring_nexus_core_v1::ops_core::retrieval_policy::source_class_coverage_from_evidence_pack_api(
        policy,
        query,
        evidence_pack,
        evidence_coverage,
    )
}

fn source_class_coverage_from_ranked_candidates(
    policy: &Value,
    query: &str,
    actionable_ranked: &[(Candidate, f64)],
    evidence_coverage: &Value,
) -> Value {
    infring_nexus_core_v1::ops_core::retrieval_policy::source_class_coverage_from_ranked_candidates_api(
        policy,
        query,
        &ranked_to_policy_value(actionable_ranked),
        evidence_coverage,
    )
}

fn retrieval_broker_report(
    status: &str,
    submitted_query_plan: Value,
    executed_query_plan: Value,
    query_plan_source: &str,
    second_pass_recovery: Value,
    retrieval_telemetry: &Value,
    provider_results: &Value,
    evidence_pack: &Value,
    evidence_coverage: &Value,
    tool_result_quality: &Value,
    source_class_coverage: &Value,
    evidence_pack_quality: &Value,
) -> Value {
    infring_nexus_core_v1::ops_core::retrieval_policy::retrieval_broker_report_api(
        status,
        submitted_query_plan,
        executed_query_plan,
        query_plan_source,
        second_pass_recovery,
        retrieval_telemetry,
        provider_results,
        evidence_pack,
        evidence_coverage,
        tool_result_quality,
        source_class_coverage,
        evidence_pack_quality,
    )
}

fn page_extraction_link_has_article_like_path(link: &str) -> bool {
    infring_nexus_core_v1::ops_core::retrieval_policy::page_extraction_link_has_article_like_path_api(link)
}

fn candidate_looks_like_relevant_discovery_hub(query: &str, candidate: &Candidate) -> bool {
    infring_nexus_core_v1::ops_core::retrieval_policy::candidate_looks_like_relevant_discovery_hub_api(
        query,
        &candidate_to_policy_value(candidate),
    )
}

fn page_extraction_link_candidate_with_context(link: &str, context: &str) -> Candidate {
    candidate_from_policy_value(
        &infring_nexus_core_v1::ops_core::retrieval_policy::page_extraction_link_candidate_with_context_api(
            link, context,
        ),
    )
}

fn fallback_link_score_with_context(query: &str, link: &str, context: &str) -> f64 {
    infring_nexus_core_v1::ops_core::retrieval_policy::fallback_link_score_with_context_api(
        query, link, context,
    )
}

fn ranked_payload_links_for_fallback_with_context_and_min_score(
    query: &str,
    payload: &Value,
    max_links: usize,
    min_score: f64,
) -> Vec<(String, String)> {
    infring_nexus_core_v1::ops_core::retrieval_policy::ranked_payload_links_for_fallback_with_context_and_min_score_api(
        query,
        payload,
        max_links,
        min_score,
    )
    .as_array()
    .map(|rows| {
        rows.iter()
            .map(|row| {
                (
                    clean_text(row.get("link").and_then(Value::as_str).unwrap_or(""), 2_200),
                    clean_text(row.get("context").and_then(Value::as_str).unwrap_or(""), 2_400),
                )
            })
            .filter(|(link, _)| !link.is_empty())
            .collect::<Vec<_>>()
    })
    .unwrap_or_default()
}

fn ranked_payload_links_for_fallback(
    query: &str,
    payload: &Value,
    max_links: usize,
) -> Vec<String> {
    infring_nexus_core_v1::ops_core::retrieval_policy::ranked_payload_links_for_fallback_api(
        query, payload, max_links,
    )
}

fn comparison_guard_failure_artifacts(
    query: &str,
    comparison_entities: &[String],
    actionable_ranked: &[(Candidate, f64)],
    retained_ranked: &[(Candidate, f64)],
    provider_results: &[Value],
    max_results: usize,
) -> (Value, Option<String>) {
    let value = infring_nexus_core_v1::ops_core::retrieval_policy::comparison_guard_failure_artifacts_api(
        query,
        comparison_entities,
        &ranked_to_policy_value(actionable_ranked),
        &ranked_to_policy_value(retained_ranked),
        provider_results,
        max_results,
    );
    (
        value.get("rows").cloned().unwrap_or_else(|| json!([])),
        value
            .get("summary")
            .and_then(Value::as_str)
            .map(ToString::to_string),
    )
}

fn comparison_partial_preserves_actionable_evidence(
    query: &str,
    comparison_entities: &[String],
    actionable_ranked: &[(Candidate, f64)],
    retained_ranked: &[(Candidate, f64)],
) -> bool {
    infring_nexus_core_v1::ops_core::retrieval_policy::comparison_partial_preserves_actionable_evidence_api(
        query,
        comparison_entities,
        &ranked_to_policy_value(actionable_ranked),
        &ranked_to_policy_value(retained_ranked),
    )
}
