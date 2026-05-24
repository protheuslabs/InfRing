// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/ops (retrieval policy authority).

include!("../../../layer0/ops/src/batch_query_primitive_parts/010-core.combined.rs");
include!("../../../layer0/ops/src/batch_query_primitive_parts/015-intent-and-quality.rs");
include!("../../../layer0/ops/src/batch_query_primitive_parts/018-request-and-cache.rs");
include!("retrieval_policy_parts/010-freshness-and-relevance.rs");
include!("retrieval_policy_parts/020-candidate-rows.rs");

fn framework_official_domain(domain: &str) -> bool {
    let lowered = clean_text(domain, 160).to_ascii_lowercase();
    lowered.contains("langchain.com")
        || lowered.contains("openai.com")
        || lowered.contains("openai.github.io")
        || lowered.contains("crewai.com")
        || lowered.contains("github.com")
        || lowered.contains("microsoft.com")
}

fn page_extraction_link_candidate(link: &str) -> Candidate {
    let cleaned = clean_text(link, 2_200);
    let domain = extract_domains_from_text(&cleaned, 1)
        .into_iter()
        .next()
        .unwrap_or_default();
    Candidate {
        source_kind: "web".to_string(),
        title: format!("Web result from {domain}"),
        locator: cleaned.clone(),
        snippet: cleaned.clone(),
        excerpt_hash: sha256_hex(&cleaned),
        timestamp: None,
        permissions: Some("public_web".to_string()),
        status_code: 200,
    }
}

fn parse_page_extraction_http_url(link: &str) -> Option<(&str, &str, &str, Option<&str>)> {
    let trimmed = link.trim();
    let lowered = trimmed.to_ascii_lowercase();
    let (scheme, after_scheme) = if lowered.starts_with("https://") {
        ("https", &trimmed[8..])
    } else if lowered.starts_with("http://") {
        ("http", &trimmed[7..])
    } else {
        return None;
    };
    let host_end = after_scheme.find(['/', '?']).unwrap_or(after_scheme.len());
    let host_with_port = &after_scheme[..host_end];
    let host = host_with_port
        .rsplit_once('@')
        .map(|(_, value)| value)
        .unwrap_or(host_with_port)
        .split_once(':')
        .map(|(value, _)| value)
        .unwrap_or(host_with_port);
    if host.is_empty() {
        return None;
    }
    let remainder = &after_scheme[host_end..];
    if remainder.is_empty() {
        return Some((scheme, host, "/", None));
    }
    if let Some(query) = remainder.strip_prefix('?') {
        return Some((scheme, host, "/", Some(query)));
    }
    let (path, query) = remainder
        .split_once('?')
        .map(|(path, query)| (path, Some(query)))
        .unwrap_or((remainder, None));
    Some((scheme, host, path, query))
}

fn citation_wrapper_link(link: &str) -> bool {
    let Some((_, host, path, query)) = parse_page_extraction_http_url(link) else {
        return false;
    };
    let host = host.trim_start_matches("www.").to_ascii_lowercase();
    let path = path.to_ascii_lowercase();
    let query = query.unwrap_or("").to_ascii_lowercase();
    (host == "news.google.com"
        && (path.contains("/rss/articles/")
            || path.contains("/articles/")
            || path.contains("/read/")))
        || (host == "duckduckgo.com" && (path.contains("/l/") || query.contains("uddg=")))
        || ((host == "google.com" || host == "www.google.com")
            && (path.contains("/url") || query.contains("url=") || query.contains("q=http")))
        || (social_share_wrapper_host(&host)
            && (path.contains("/l.php")
                || path.contains("/share")
                || path.contains("/sharer")
                || query.contains("url=")
                || query.contains("u=")
                || query.contains("href=")
                || query.contains("target=")))
}

fn query_overlap_terms(query: &str, candidate: &Candidate) -> usize {
    query_overlap_profile(query, candidate).0
}

fn citation_wrapper_context_has_signal(query: &str, context: &str) -> bool {
    let cleaned_context = clean_text(context, 1_800);
    if cleaned_context.is_empty()
        || contains_web_junk_marker(&cleaned_context)
        || looks_like_low_signal_search_summary(&cleaned_context)
    {
        return false;
    }
    let candidate =
        page_extraction_link_candidate_with_context("https://example.com/wrapper", &cleaned_context);
    let (overlap, distinctive_overlap, _) = query_overlap_profile(query, &candidate);
    distinctive_overlap > 0 || overlap >= 2
}

fn link_contains_collapsed_query_phrase(query: &str, link: &str) -> bool {
    let tokens = clean_text(query, 800)
        .to_ascii_lowercase()
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .map(str::trim)
        .filter(|token| token.len() >= 3)
        .filter(|token| !is_relevance_stop_token(token))
        .filter(|token| !is_weak_relevance_token(token))
        .map(str::to_string)
        .collect::<Vec<_>>();
    if tokens.len() < 2 {
        return false;
    }
    let collapsed_link = clean_text(link, 2_200)
        .to_ascii_lowercase()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>();
    if collapsed_link.is_empty() {
        return false;
    }
    tokens.windows(2).any(|pair| {
        let phrase = format!("{}{}", pair[0], pair[1]);
        phrase.len() >= 8 && collapsed_link.contains(&phrase)
    })
}

fn candidate_is_substantive(query: &str, candidate: &Candidate, benchmark_intent: bool) -> bool {
    let snippet = clean_text(&candidate.snippet, 1_800);
    if snippet.is_empty() {
        return false;
    }
    if contains_antibot_marker(&snippet) || contains_antibot_marker(&candidate.title) {
        return false;
    }
    if contains_web_junk_marker(&snippet) || contains_web_junk_marker(&candidate.title) {
        return false;
    }
    if looks_like_off_intent_noise_candidate(query, candidate) {
        return false;
    }
    if looks_like_domain_list_noise(&snippet) {
        return false;
    }
    let word_count = snippet.split_whitespace().count();
    let overlap = query_overlap_terms(query, candidate);
    if has_only_weak_query_overlap(query, candidate) {
        return false;
    }
    if benchmark_intent {
        if word_count < 8 && overlap < 2 {
            return false;
        }
    } else if word_count < 6 && overlap < 1 {
        return false;
    }
    if is_framework_catalog_intent(query) && word_count < 8 && overlap < 2 {
        let combined = format!("{} {}", candidate.title, snippet);
        let domain = candidate_domain_hint(candidate);
        if framework_official_domain(&domain) && looks_like_framework_overview_text(&combined) {
            return true;
        }
        return false;
    }
    true
}

fn rerank_score(query: &str, candidate: &Candidate) -> f64 {
    let benchmark_intent = is_benchmark_or_comparison_intent(query);
    let framework_catalog_intent = is_framework_catalog_intent(query);
    let query_tokens = tokenize_relevance(query, 40);
    let haystack = tokenize_relevance(&candidate_relevance_text(candidate), 120);
    let overlap = query_tokens
        .iter()
        .filter(|token| haystack.contains(token.as_str()))
        .count() as f64;
    let overlap_norm = if query_tokens.is_empty() {
        0.0
    } else {
        overlap / query_tokens.len() as f64
    };
    let locator_bonus = if candidate.locator.is_empty() { 0.0 } else { 0.2 };
    let status_bonus = if (200..400).contains(&candidate.status_code) {
        0.2
    } else {
        0.0
    };
    let metric_bonus = if benchmark_intent && looks_like_metric_rich_text(&candidate.snippet) {
        0.24
    } else {
        0.0
    };
    let framework_catalog_bonus = if framework_catalog_intent
        && looks_like_framework_catalog_text(&format!("{} {}", candidate.title, candidate.snippet))
    {
        0.18
    } else {
        0.0
    };
    let framework_catalog_source_bonus = if framework_catalog_intent {
        framework_catalog_source_adjustment(candidate)
    } else {
        0.0
    };
    let direct_official_source = official_lane_direct_subject_source_signal(query, candidate);
    let direct_official_source_bonus = if direct_official_source { 0.32 } else { 0.0 };
    let official_lane_mismatch_penalty = if is_official_source_query_lane(query)
        && !direct_official_source
        && !candidate_has_trusted_primary_source_signal(query, candidate)
    {
        0.14
    } else {
        0.0
    };
    let definition_penalty = if benchmark_intent && looks_like_definition_candidate(candidate) {
        0.72
    } else {
        0.0
    };
    let comparison_noise_penalty =
        if benchmark_intent && looks_like_comparison_noise_candidate(candidate) {
            0.65
        } else {
            0.0
        };
    let low_signal_penalty = if contains_web_junk_marker(&candidate.snippet)
        || looks_like_source_only_snippet(&candidate.snippet)
        || looks_like_domain_list_noise(&candidate.snippet)
    {
        0.3
    } else {
        0.0
    };
    let off_intent_noise_penalty = if looks_like_off_intent_noise_candidate(query, candidate) {
        0.65
    } else {
        0.0
    };
    let weak_overlap_penalty = if has_only_weak_query_overlap(query, candidate) {
        0.55
    } else {
        0.0
    };
    let mut score = 0.6 * overlap_norm
        + locator_bonus
        + status_bonus
        + metric_bonus
        + framework_catalog_bonus
        + framework_catalog_source_bonus
        + direct_official_source_bonus
        + source_trust_adjustment(candidate)
        + recency_adjustment(query, candidate)
        - definition_penalty
        - comparison_noise_penalty
        - low_signal_penalty
        - off_intent_noise_penalty
        - official_lane_mismatch_penalty
        - weak_overlap_penalty;
    if benchmark_intent && !looks_like_metric_rich_text(&candidate.snippet) {
        score -= 0.12;
    }
    score.clamp(0.0, 1.0)
}

fn minimum_synthesis_score(benchmark_intent: bool) -> f64 {
    if benchmark_intent {
        0.33
    } else {
        0.18
    }
}

fn candidate_is_low_confidence_retained(candidate: &Candidate) -> bool {
    candidate
        .source_kind
        .to_ascii_lowercase()
        .contains("low_confidence")
        || candidate
            .permissions
            .as_deref()
            .unwrap_or("")
            .to_ascii_lowercase()
            .contains("low_confidence_raw")
}

fn candidate_to_value(candidate: &Candidate) -> Value {
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

fn candidate_from_value(value: &Value) -> Candidate {
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

fn ranked_from_value(value: &Value) -> Vec<(Candidate, f64)> {
    value
        .as_array()
        .map(|rows| {
            rows.iter()
                .map(|row| {
                    let candidate = row
                        .get("candidate")
                        .map(candidate_from_value)
                        .unwrap_or_else(|| candidate_from_value(row));
                    let score = row.get("score").and_then(Value::as_f64).unwrap_or(0.0);
                    (candidate, score)
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn ranked_to_value(rows: &[(Candidate, f64)]) -> Value {
    Value::Array(
        rows.iter()
            .map(|(candidate, score)| {
                json!({
                    "candidate": candidate_to_value(candidate),
                    "score": score
                })
            })
            .collect(),
    )
}

fn candidates_from_value(value: &Value) -> Vec<Candidate> {
    value
        .as_array()
        .map(|rows| rows.iter().map(candidate_from_value).collect::<Vec<_>>())
        .unwrap_or_default()
}

fn candidates_to_value(rows: &[Candidate]) -> Value {
    Value::Array(rows.iter().map(candidate_to_value).collect())
}

fn string_vec_from_value(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|raw| clean_text(raw, 240))
                .filter(|raw| !raw.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn keyword_pack_from_value(value: &Value) -> BatchQueryKeywordPack {
    let required = value.get("required_coverage").unwrap_or(&Value::Null);
    let authority = value
        .get("compilation")
        .and_then(|row| row.get("authority"))
        .and_then(Value::as_str)
        .unwrap_or("agent_submitted_request_metadata");
    BatchQueryKeywordPack {
        keywords: string_vec_from_value(value.get("keywords")),
        entities: string_vec_from_value(required.get("entities")),
        facets: string_vec_from_value(required.get("facets")),
        aliases: string_vec_from_value(value.get("aliases")),
        negative_terms: string_vec_from_value(value.get("negative_terms")),
        metadata_authority: clean_text(authority, 120),
    }
}

fn budget_from_value(value: &Value) -> ApertureBudget {
    ApertureBudget {
        max_candidates: value
            .get("max_candidates")
            .and_then(Value::as_u64)
            .unwrap_or(20) as usize,
        max_evidence: value
            .get("max_evidence")
            .and_then(Value::as_u64)
            .unwrap_or(6) as usize,
        max_summary_tokens: value
            .get("max_summary_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(350) as usize,
        #[cfg(test)]
        max_query_rewrites: value
            .get("max_query_rewrites")
            .and_then(Value::as_u64)
            .unwrap_or(1) as usize,
    }
}

fn research_facet_to_value(facet: &ResearchFacet) -> Value {
    json!({
        "id": facet.id,
        "requested_text": facet.requested_text,
        "kind": facet.kind,
        "terms": facet.terms.iter().cloned().collect::<Vec<_>>(),
        "distinctive_terms": facet.distinctive_terms.iter().cloned().collect::<Vec<_>>()
    })
}

fn research_facet_from_value(value: &Value) -> ResearchFacet {
    ResearchFacet {
        id: clean_text(value.get("id").and_then(Value::as_str).unwrap_or(""), 80),
        requested_text: clean_text(
            value.get("requested_text").and_then(Value::as_str).unwrap_or(""),
            240,
        ),
        kind: clean_text(value.get("kind").and_then(Value::as_str).unwrap_or("inferred"), 80),
        terms: string_vec_from_value(value.get("terms"))
            .into_iter()
            .collect::<HashSet<_>>(),
        distinctive_terms: string_vec_from_value(value.get("distinctive_terms"))
            .into_iter()
            .collect::<HashSet<_>>(),
    }
}

fn facets_from_value(value: &Value) -> Vec<ResearchFacet> {
    value
        .as_array()
        .map(|rows| rows.iter().map(research_facet_from_value).collect::<Vec<_>>())
        .unwrap_or_default()
}

pub fn current_web_intent_api(query: &str) -> bool {
    current_web_intent(query)
}

pub fn current_year_api() -> String {
    current_year()
}

pub fn candidates_from_structured_search_payload_api(
    query: &str,
    payload: &Value,
    max_rows: usize,
) -> Value {
    Value::Array(
        candidates_from_structured_search_payload(query, payload, max_rows)
            .iter()
            .map(candidate_to_value)
            .collect(),
    )
}

pub fn candidates_from_rendered_search_payload_api(
    query: &str,
    payload: &Value,
    max_rows: usize,
) -> Value {
    Value::Array(
        candidates_from_rendered_search_payload(query, payload, max_rows)
            .iter()
            .map(candidate_to_value)
            .collect(),
    )
}

pub fn infer_research_facets_api(
    query: &str,
    query_plan: &[String],
    query_metadata: &Value,
    policy: &Value,
    budget: &Value,
) -> Value {
    let metadata = keyword_pack_from_value(query_metadata);
    let budget = budget_from_value(budget);
    Value::Array(
        infer_research_facets(query, query_plan, &metadata, policy, budget)
            .iter()
            .map(research_facet_to_value)
            .collect(),
    )
}

pub fn coverage_aware_max_evidence_api(facets: &Value, budget: &Value) -> usize {
    coverage_aware_max_evidence(&facets_from_value(facets), budget_from_value(budget))
}

pub fn select_facet_covered_ranked_candidates_api(
    ranked: &Value,
    facets: &Value,
    max_evidence: usize,
    min_terms: usize,
) -> Value {
    let selected = select_facet_covered_ranked_candidates(
        ranked_from_value(ranked),
        &facets_from_value(facets),
        max_evidence,
        min_terms,
    );
    ranked_to_value(&selected)
}

pub fn candidate_identity_key_api(candidate: &Value) -> String {
    candidate_identity_key(&candidate_from_value(candidate))
}

pub fn selected_candidate_coverage_ids_api(
    facets: &Value,
    candidate: &Value,
    min_terms: usize,
) -> Vec<String> {
    selected_candidate_coverage_ids(&facets_from_value(facets), &candidate_from_value(candidate), min_terms)
}

pub fn candidate_coverage_facets_api(
    facets: &Value,
    candidate: &Value,
    min_terms: usize,
) -> Vec<String> {
    candidate_coverage_facets(&facets_from_value(facets), &candidate_from_value(candidate), min_terms)
}

pub fn candidate_retention_preview_eligible_api(query: &str, candidate: &Value, score: f64) -> bool {
    candidate_retention_preview_eligible(query, &candidate_from_value(candidate), score)
}

pub fn content_rich_text_api(text: &str) -> bool {
    content_rich_text(text)
}

pub fn source_trust_adjustment_api(candidate: &Value) -> f64 {
    source_trust_adjustment(&candidate_from_value(candidate))
}

pub fn redundant_facet_backfill_replacement_index_api(
    selected: &Value,
    facets: &Value,
    min_terms: usize,
) -> Option<usize> {
    let selected = ranked_from_value(selected);
    redundant_facet_backfill_replacement_index(&selected, &facets_from_value(facets), min_terms)
}

pub fn evidence_pack_from_ranked_candidates_api(
    policy: &Value,
    query: &str,
    facets: &Value,
    min_terms: usize,
    actionable_ranked: &Value,
    max_evidence: usize,
) -> Value {
    evidence_pack_from_ranked_candidates(
        policy,
        query,
        &facets_from_value(facets),
        min_terms,
        &ranked_from_value(actionable_ranked),
        max_evidence,
    )
}

pub fn web_tool_quality_report_api(
    query: &str,
    status: &str,
    candidate_count: usize,
    evidence_count: usize,
    partial_failures: &[String],
    hard_partial_failures: &[String],
    actionable_ranked: &Value,
) -> Value {
    web_tool_quality_report(
        query,
        status,
        candidate_count,
        evidence_count,
        partial_failures,
        hard_partial_failures,
        &ranked_from_value(actionable_ranked),
    )
}

pub fn cached_web_tool_quality_report_api(
    query: &str,
    status: &str,
    partial_failure_details: &Value,
    evidence_refs: &Value,
) -> Value {
    cached_web_tool_quality_report(query, status, partial_failure_details, evidence_refs)
}

pub fn web_tool_quality_version_api() -> String {
    web_tool_quality_version().to_string()
}

pub fn segment_has_current_signal_api(text: &str) -> bool {
    segment_has_current_signal(text)
}

pub fn recency_adjustment_api(query: &str, candidate: &Value) -> f64 {
    recency_adjustment(query, &candidate_from_value(candidate))
}

pub fn candidate_quality_flags_api(query: &str, candidate: &Value, score: f64) -> Vec<String> {
    candidate_quality_flags(query, &candidate_from_value(candidate), score)
}

pub fn select_pack_ready_ranked_candidates_api(
    query: &str,
    ranked: &Value,
    facets: &Value,
    max_evidence: usize,
    min_terms: usize,
) -> Value {
    let selected = select_pack_ready_ranked_candidates(
        query,
        ranked_from_value(ranked),
        &facets_from_value(facets),
        max_evidence,
        min_terms,
    );
    ranked_to_value(&selected)
}

pub fn research_facet_from_metadata_text_api(text: &str, index: usize, kind: &str) -> Value {
    research_facet_from_metadata_text(text, index, kind)
        .map(|facet| research_facet_to_value(&facet))
        .unwrap_or(Value::Null)
}

pub fn assign_distinctive_facet_terms_api(facets: &Value) -> Value {
    let mut facets = facets_from_value(facets);
    assign_distinctive_facet_terms(&mut facets);
    Value::Array(facets.iter().map(research_facet_to_value).collect())
}

pub fn candidate_matches_facet_api(facet: &Value, candidate: &Value, min_terms: usize) -> bool {
    candidate_matches_facet(
        &research_facet_from_value(facet),
        &candidate_from_value(candidate),
        min_terms,
    )
}

pub fn coverage_aware_score_api(
    query: &str,
    facets: &Value,
    candidate: &Value,
    min_terms: usize,
) -> f64 {
    coverage_aware_score(
        query,
        &facets_from_value(facets),
        &candidate_from_value(candidate),
        min_terms,
    )
}

pub fn backfill_missing_facet_ranked_candidates_api(
    query: &str,
    selected: &Value,
    supplemental_pool: &Value,
    facets: &Value,
    max_evidence: usize,
    min_terms: usize,
    allow_low_confidence: bool,
) -> Value {
    let mut selected = ranked_from_value(selected);
    let supplemental_pool = ranked_from_value(supplemental_pool);
    let added = backfill_missing_facet_ranked_candidates(
        query,
        &mut selected,
        &supplemental_pool,
        &facets_from_value(facets),
        max_evidence,
        min_terms,
        allow_low_confidence,
    );
    json!({
        "added": added,
        "selected": ranked_to_value(&selected)
    })
}

pub fn truncate_candidates_preserving_facet_coverage_api(
    query: &str,
    facets: &Value,
    candidates: &Value,
    max_candidates: usize,
    min_terms: usize,
) -> Value {
    let mut candidates = candidates_from_value(candidates);
    truncate_candidates_preserving_facet_coverage(
        query,
        &facets_from_value(facets),
        &mut candidates,
        max_candidates,
        min_terms,
    );
    candidates_to_value(&candidates)
}

pub fn evidence_coverage_from_ranked_candidates_api(
    query: &str,
    facets: &Value,
    evidence_ranked: &Value,
    min_terms: usize,
) -> Value {
    evidence_coverage_from_ranked_candidates(
        query,
        &facets_from_value(facets),
        &ranked_from_value(evidence_ranked),
        min_terms,
    )
}

pub fn ranked_evidence_covers_all_facets_api(
    query: &str,
    facets: &Value,
    evidence_ranked: &Value,
    min_terms: usize,
) -> bool {
    ranked_evidence_covers_all_facets(
        query,
        &facets_from_value(facets),
        &ranked_from_value(evidence_ranked),
        min_terms,
    )
}

pub fn evidence_selection_diagnostics_api(
    query: &str,
    ranked_pool: &Value,
    actionable_ranked_pool: &Value,
    evidence_ranked: &Value,
    limit: usize,
) -> Value {
    evidence_selection_diagnostics(
        query,
        &ranked_from_value(ranked_pool),
        &ranked_from_value(actionable_ranked_pool),
        &ranked_from_value(evidence_ranked),
        limit,
    )
}

pub fn coverage_gap_recovery_queries_api(
    policy: &Value,
    query: &str,
    existing_queries: &[String],
    facets: &Value,
    candidates: &Value,
    budget: &Value,
) -> Vec<String> {
    coverage_gap_recovery_queries(
        policy,
        query,
        existing_queries,
        &facets_from_value(facets),
        &candidates_from_value(candidates),
        budget_from_value(budget),
    )
}

pub fn claim_gap_recovery_queries_api(
    policy: &Value,
    query: &str,
    existing_queries: &[String],
    facets: &Value,
    candidates: &Value,
    budget: &Value,
) -> Vec<String> {
    claim_gap_recovery_queries(
        policy,
        query,
        existing_queries,
        &facets_from_value(facets),
        &candidates_from_value(candidates),
        budget_from_value(budget),
    )
}

pub fn broad_current_research_lacks_synthesis_breadth_api(
    policy: &Value,
    query: &str,
    query_metadata: &Value,
    candidates: &Value,
    budget: &Value,
) -> bool {
    let metadata = keyword_pack_from_value(query_metadata);
    broad_current_research_lacks_synthesis_breadth(
        policy,
        query,
        &metadata,
        &candidates_from_value(candidates),
        budget_from_value(budget),
    )
}

pub fn candidate_has_non_evidence_payload_api(candidate: &Value) -> bool {
    candidate_has_non_evidence_payload(&candidate_from_value(candidate))
}

pub fn candidate_counts_as_query_usable_evidence_api(
    query: &str,
    candidate: &Value,
    score: f64,
) -> bool {
    candidate_counts_as_query_usable_evidence(query, &candidate_from_value(candidate), score)
}

pub fn has_pack_ready_synthesis_candidate_api(query: &str, candidates: &Value) -> bool {
    has_pack_ready_synthesis_candidate(query, &candidates_from_value(candidates))
}

pub fn has_pack_ready_synthesis_source_quality_api(query: &str, candidates: &Value) -> bool {
    has_pack_ready_synthesis_source_quality(query, &candidates_from_value(candidates))
}

pub fn claim_hint_normalized_snippet_api(snippet: &str) -> String {
    claim_hint_normalized_snippet(snippet)
}

pub fn claim_text_is_synthesis_safe_api(text: &str) -> bool {
    claim_text_is_synthesis_safe(text)
}

pub fn looks_like_link_directory_or_aggregator_shell_api(text: &str) -> bool {
    looks_like_link_directory_or_aggregator_shell(text)
}

pub fn evidence_claims_from_pack_api(
    query_metadata: &Value,
    evidence_pack: &Value,
    limit: usize,
) -> Value {
    let metadata = keyword_pack_from_value(query_metadata);
    evidence_claims_from_pack(&metadata, evidence_pack, limit)
}

pub fn evidence_pack_quality_report_api(
    policy: &Value,
    evidence_pack: &Value,
    evidence_coverage: &Value,
) -> Value {
    evidence_pack_quality_report(policy, evidence_pack, evidence_coverage)
}

pub fn source_class_coverage_from_evidence_pack_api(
    policy: &Value,
    query: &str,
    evidence_pack: &Value,
    evidence_coverage: &Value,
) -> Value {
    source_class_coverage_from_evidence_pack(policy, query, evidence_pack, evidence_coverage)
}

pub fn source_class_coverage_from_ranked_candidates_api(
    policy: &Value,
    query: &str,
    actionable_ranked: &Value,
    evidence_coverage: &Value,
) -> Value {
    source_class_coverage_from_ranked_candidates(
        policy,
        query,
        &ranked_from_value(actionable_ranked),
        evidence_coverage,
    )
}

pub fn retrieval_broker_report_api(
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
    retrieval_broker_report(
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

pub fn page_extraction_link_has_article_like_path_api(link: &str) -> bool {
    page_extraction_link_has_article_like_path(link)
}

pub fn candidate_looks_like_relevant_discovery_hub_api(query: &str, candidate: &Value) -> bool {
    candidate_looks_like_relevant_discovery_hub(query, &candidate_from_value(candidate))
}

pub fn page_extraction_link_candidate_with_context_api(link: &str, context: &str) -> Value {
    candidate_to_value(&page_extraction_link_candidate_with_context(link, context))
}

pub fn fallback_link_score_with_context_api(query: &str, link: &str, context: &str) -> f64 {
    fallback_link_score_with_context(query, link, context)
}

pub fn ranked_payload_links_for_fallback_with_context_and_min_score_api(
    query: &str,
    payload: &Value,
    max_links: usize,
    min_score: f64,
) -> Value {
    Value::Array(
        ranked_payload_links_for_fallback_with_context_and_min_score(
            query, payload, max_links, min_score,
        )
        .into_iter()
        .map(|(link, context)| json!({"link": link, "context": context}))
        .collect(),
    )
}

pub fn ranked_payload_links_for_fallback_api(
    query: &str,
    payload: &Value,
    max_links: usize,
) -> Vec<String> {
    ranked_payload_links_for_fallback(query, payload, max_links)
}

pub fn comparison_guard_failure_artifacts_api(
    query: &str,
    comparison_entities: &[String],
    actionable_ranked: &Value,
    retained_ranked: &Value,
    provider_results: &[Value],
    max_results: usize,
) -> Value {
    let (rows, summary) = comparison_guard_failure_artifacts(
        query,
        comparison_entities,
        &ranked_from_value(actionable_ranked),
        &ranked_from_value(retained_ranked),
        provider_results,
        max_results,
    );
    json!({
        "rows": rows,
        "summary": summary
    })
}

pub fn comparison_partial_preserves_actionable_evidence_api(
    query: &str,
    comparison_entities: &[String],
    actionable_ranked: &Value,
    retained_ranked: &Value,
) -> bool {
    comparison_partial_preserves_actionable_evidence(
        query,
        comparison_entities,
        &ranked_from_value(actionable_ranked),
        &ranked_from_value(retained_ranked),
    )
}
