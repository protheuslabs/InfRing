fn current_web_intent(query: &str) -> bool {
    let lowered = clean_text(query, 600).to_ascii_lowercase();
    if lowered.contains(&current_year()) {
        return true;
    }
    [
        "latest",
        "current",
        "today",
        "this week",
        "this month",
        "as of",
        "recent",
        "newest",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn web_tool_quality_version() -> &'static str {
    "web_tool_quality_v11"
}

fn current_year() -> String {
    crate::now_iso().chars().take(4).collect::<String>()
}

fn segment_has_current_signal(text: &str) -> bool {
    let lowered = clean_text(text, 1_200).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let year = current_year();
    lowered.contains(&year)
        || lowered.contains("today")
        || lowered.contains("this week")
        || lowered.contains("this month")
        || lowered.contains("yesterday")
        || lowered.contains("hours ago")
        || lowered.contains("minutes ago")
}

fn push_unique_clean_string(
    out: &mut Vec<String>,
    seen: &mut HashSet<String>,
    raw: &str,
    max_len: usize,
) {
    let cleaned = clean_text(raw, max_len);
    if cleaned.is_empty() {
        return;
    }
    let key = cleaned.to_ascii_lowercase();
    if seen.insert(key) {
        out.push(cleaned);
    }
}

fn source_trust_adjustment(candidate: &Candidate) -> f64 {
    let domain = candidate_domain_hint(candidate).to_ascii_lowercase();
    let locator = clean_text(&candidate.locator, 2_200).to_ascii_lowercase();
    let combined = clean_text(
        &format!(
            "{} {} {}",
            candidate.title, candidate.snippet, candidate.locator
        ),
        2_400,
    )
    .to_ascii_lowercase();
    if domain == "source" || is_search_engine_domain(&domain) {
        return -0.18;
    }
    if contains_web_junk_marker(&combined) || looks_like_competitive_programming_dump(&combined) {
        return -0.5;
    }
    if domain.contains("reddit.com")
        || domain.contains("quora.com")
        || domain.contains("zhihu.com")
        || domain.contains("medium.com")
        || domain.contains("dev.to")
        || domain.contains("hashnode.dev")
        || domain.contains("tiktok.com")
        || domain.contains("instagram.com")
        || domain.contains("facebook.com")
        || domain.contains("pinterest.")
    {
        return -0.14;
    }
    if domain.contains("docs.")
        || locator.contains("/docs")
        || locator.contains("/documentation")
        || domain.contains("developer.")
        || domain.contains("github.com")
        || domain.contains("gitlab.com")
        || domain.contains("arxiv.org")
        || domain.contains("openai.com")
        || domain.contains("microsoft.com")
        || domain.contains("langchain.com")
        || domain.contains("huggingface.co")
        || domain.contains("crewai.com")
    {
        return 0.16;
    }
    if domain.contains("reuters.com")
        || domain.contains("apnews.com")
        || domain.contains("bloomberg.com")
        || domain.contains("wsj.com")
        || domain.contains("ft.com")
        || domain.contains("techcrunch.com")
        || domain.contains("theverge.com")
    {
        return 0.1;
    }
    0.0
}

fn recency_adjustment(query: &str, candidate: &Candidate) -> f64 {
    if !current_web_intent(query) {
        return 0.0;
    }
    let year = current_year();
    let haystack = clean_text(
        &format!(
            "{} {} {}",
            candidate.title, candidate.snippet, candidate.locator
        ),
        2_400,
    )
    .to_ascii_lowercase();
    if haystack.contains(&year) || segment_has_current_signal(&haystack) {
        0.08
    } else {
        -0.08
    }
}

fn candidate_quality_flags(query: &str, candidate: &Candidate, score: f64) -> Vec<String> {
    let mut flags = Vec::<String>::new();
    let snippet = clean_text(&candidate.snippet, 1_600);
    let trust = source_trust_adjustment(candidate);
    if trust >= 0.15 {
        flags.push("trusted_source".to_string());
    } else if trust <= -0.14 {
        flags.push("low_trust_source".to_string());
    }
    if current_web_intent(query) && recency_adjustment(query, candidate) < 0.0 {
        flags.push("freshness_unproven".to_string());
    }
    if looks_like_metric_rich_text(&snippet) {
        flags.push("metric_rich".to_string());
    }
    if query_overlap_terms(query, candidate) < 2 {
        flags.push("thin_query_overlap".to_string());
    }
    if has_only_weak_query_overlap(query, candidate) {
        flags.push("weak_query_overlap_only".to_string());
    }
    if contains_web_junk_marker(&snippet) || contains_web_junk_marker(&candidate.title) {
        flags.push("junk_marker".to_string());
    }
    if looks_like_style_or_script_dump(&snippet) {
        flags.push("style_or_script_dump".to_string());
    }
    if looks_like_social_video_shell(candidate) {
        flags.push("social_video_shell".to_string());
    }
    if looks_like_link_directory_or_aggregator_shell(&snippet) {
        flags.push("link_directory_or_aggregator_shell".to_string());
    }
    if score < 0.35 {
        flags.push("low_score".to_string());
    }
    flags.sort();
    flags.dedup();
    flags
}

fn sorted_ranked_candidates(mut ranked: Vec<(Candidate, f64)>) -> Vec<(Candidate, f64)> {
    ranked.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.title.cmp(&b.0.title))
    });
    ranked
}

fn select_diverse_ranked_candidates(
    ranked: Vec<(Candidate, f64)>,
    max_evidence: usize,
) -> Vec<(Candidate, f64)> {
    let limit = max_evidence.max(1);
    let sorted = sorted_ranked_candidates(ranked);
    let mut selected = Vec::<(Candidate, f64)>::new();
    let mut seen_domains = HashSet::<String>::new();
    for row in &sorted {
        let domain = candidate_domain_hint(&row.0).to_ascii_lowercase();
        if !domain.is_empty() && domain != "source" && !seen_domains.insert(domain) {
            continue;
        }
        selected.push(row.clone());
        if selected.len() >= limit {
            return selected;
        }
    }
    for row in sorted {
        if selected.iter().any(|(candidate, _)| {
            candidate.locator == row.0.locator && candidate.excerpt_hash == row.0.excerpt_hash
        }) {
            continue;
        }
        selected.push(row);
        if selected.len() >= limit {
            break;
        }
    }
    selected
}

fn candidate_is_pack_ready_evidence(query: &str, candidate: &Candidate, score: f64) -> bool {
    candidate_counts_as_query_usable_evidence(query, candidate, score)
        && !evidence_pack_claim_hints_for_candidate(query, candidate, 1).is_empty()
}

fn candidate_locator_identity_key(candidate: &Candidate) -> String {
    let locator = clean_text(&candidate.locator, 2_200).to_ascii_lowercase();
    if locator.is_empty() {
        return candidate_identity_key(candidate);
    }
    locator
}

fn evidence_selection_materialization_rank(candidate: &Candidate) -> f64 {
    match candidate_materialization_quality(candidate) {
        "full_materialized" => 2.0,
        "browser_materialized" => 1.6,
        "trusted_structured_feed" => 0.5,
        "candidate_only" => -0.5,
        _ => 0.0,
    }
}

fn evidence_selection_rank(query: &str, candidate: &Candidate, score: f64) -> f64 {
    let content_bonus = if content_rich_text(&candidate.snippet) {
        0.25
    } else {
        0.0
    };
    let claim_bonus = if evidence_pack_claim_hints_for_candidate(query, candidate, 1).is_empty() {
        0.0
    } else {
        0.25
    };
    score + evidence_selection_materialization_rank(candidate) + content_bonus + claim_bonus
}

fn dedupe_ranked_candidates_by_locator_for_evidence(
    query: &str,
    mut ranked: Vec<(Candidate, f64)>,
) -> Vec<(Candidate, f64)> {
    ranked.sort_by(|left, right| {
        evidence_selection_rank(query, &right.0, right.1)
            .total_cmp(&evidence_selection_rank(query, &left.0, left.1))
            .then_with(|| {
                candidate_locator_identity_key(&left.0).cmp(&candidate_locator_identity_key(&right.0))
            })
            .then_with(|| left.0.title.cmp(&right.0.title))
    });
    let mut seen = HashSet::<String>::new();
    ranked
        .into_iter()
        .filter(|(candidate, _)| seen.insert(candidate_locator_identity_key(candidate)))
        .collect::<Vec<_>>()
}

fn select_pack_ready_ranked_candidates(
    query: &str,
    ranked: Vec<(Candidate, f64)>,
    facets: &[ResearchFacet],
    max_evidence: usize,
    min_terms: usize,
) -> Vec<(Candidate, f64)> {
    let limit = max_evidence.max(1);
    let pack_ready = ranked
        .iter()
        .filter(|(candidate, score)| candidate_is_pack_ready_evidence(query, candidate, *score))
        .cloned()
        .collect::<Vec<_>>();
    if pack_ready.is_empty() {
        return select_facet_covered_ranked_candidates(
            dedupe_ranked_candidates_by_locator_for_evidence(query, ranked),
            facets,
            limit,
            min_terms,
        );
    }

    let pack_ready = dedupe_ranked_candidates_by_locator_for_evidence(query, pack_ready);
    let mut selected = if facets.is_empty() {
        select_diverse_ranked_candidates(pack_ready.clone(), limit)
    } else {
        select_facet_covered_ranked_candidates(pack_ready.clone(), facets, limit, min_terms)
    };
    let mut selected_keys = selected
        .iter()
        .map(|(candidate, _)| candidate_locator_identity_key(candidate))
        .collect::<HashSet<_>>();

    if selected.len() < limit {
        let remaining_ready = pack_ready
            .into_iter()
            .filter(|(candidate, _)| {
                !selected_keys.contains(&candidate_locator_identity_key(candidate))
            })
            .collect::<Vec<_>>();
        for row in select_diverse_ranked_candidates(remaining_ready, limit - selected.len()) {
            selected_keys.insert(candidate_locator_identity_key(&row.0));
            selected.push(row);
            if selected.len() >= limit {
                break;
            }
        }
    }

    selected
}

#[derive(Clone, Debug)]
struct ResearchFacet {
    id: String,
    kind: String,
    requested_text: String,
    terms: HashSet<String>,
    distinctive_terms: HashSet<String>,
}

fn research_facet_signature(terms: &HashSet<String>) -> String {
    let mut sorted = terms.iter().cloned().collect::<Vec<_>>();
    sorted.sort();
    sorted.join("|")
}

fn research_facet_actionable_terms(kind: &str, terms: &HashSet<String>) -> HashSet<String> {
    if kind == "entity" {
        return terms.clone();
    }
    terms
        .iter()
        .filter(|term| !is_weak_relevance_token(term))
        .cloned()
        .collect::<HashSet<_>>()
}

fn research_facet_from_text_with_required_terms(
    text: &str,
    index: usize,
    required_terms: usize,
    kind: &str,
) -> Option<ResearchFacet> {
    let requested_text = clean_text(text, 600);
    if requested_text.is_empty() {
        return None;
    }
    let terms = tokenize_relevance(&requested_text, 24);
    if terms.len() < required_terms.max(1) {
        return None;
    }
    let kind = clean_text(kind, 80);
    let distinctive_terms = research_facet_actionable_terms(&kind, &terms);
    if kind != "entity" && distinctive_terms.is_empty() {
        return None;
    }
    Some(ResearchFacet {
        id: format!("facet_{:02}", index + 1),
        kind,
        requested_text,
        terms,
        distinctive_terms,
    })
}

fn research_facet_from_text(text: &str, index: usize, min_terms: usize) -> Option<ResearchFacet> {
    research_facet_from_text_with_required_terms(text, index, min_terms, "inferred")
}

fn research_facet_from_metadata_text(
    text: &str,
    index: usize,
    kind: &str,
) -> Option<ResearchFacet> {
    research_facet_from_text_with_required_terms(text, index, 1, kind)
}

fn assign_distinctive_facet_terms(facets: &mut [ResearchFacet]) {
    if facets.len() <= 1 {
        for facet in facets {
            facet.distinctive_terms = research_facet_actionable_terms(&facet.kind, &facet.terms);
        }
        return;
    }
    let mut counts = HashMap::<String, usize>::new();
    for facet in facets.iter() {
        for term in &facet.terms {
            *counts.entry(term.clone()).or_insert(0) += 1;
        }
    }
    let facet_count = facets.len();
    for facet in facets.iter_mut() {
        facet.distinctive_terms = facet
            .terms
            .iter()
            .filter(|term| facet.kind == "entity" || !is_weak_relevance_token(term))
            .filter(|term| counts.get(*term).copied().unwrap_or(0) < facet_count)
            .cloned()
            .collect::<HashSet<_>>();
        if facet.distinctive_terms.is_empty() {
            facet.distinctive_terms = research_facet_actionable_terms(&facet.kind, &facet.terms);
        }
    }
}

fn metadata_coverage_facet_texts(query_metadata: &BatchQueryKeywordPack) -> Vec<(String, String)> {
    let mut seen = HashSet::<String>::new();
    let mut out = Vec::<(String, String)>::new();
    for (kind, rows) in [
        ("entity", query_metadata.entities.as_slice()),
        ("facet", query_metadata.facets.as_slice()),
    ] {
        for raw in rows {
            let cleaned = clean_text(raw, 240);
            if cleaned.is_empty() {
                continue;
            }
            let key = cleaned.to_ascii_lowercase();
            if seen.insert(key) {
                out.push((cleaned, kind.to_string()));
            }
        }
    }
    out
}

fn infer_research_facets(
    query: &str,
    query_plan: &[String],
    query_metadata: &BatchQueryKeywordPack,
    policy: &Value,
    budget: ApertureBudget,
) -> Vec<ResearchFacet> {
    if !facet_aware_evidence_enabled(policy) {
        return Vec::new();
    }
    let max_facets = facet_aware_max_facets(policy, budget);
    let min_terms = facet_aware_min_terms(policy);
    let mut facets = Vec::<ResearchFacet>::new();
    let mut seen = HashSet::<String>::new();
    for (text, kind) in metadata_coverage_facet_texts(query_metadata) {
        if let Some(mut facet) = research_facet_from_metadata_text(&text, facets.len(), &kind) {
            let signature = research_facet_signature(&facet.terms);
            if !seen.insert(signature) {
                continue;
            }
            facet.id = format!("facet_{:02}", facets.len() + 1);
            facets.push(facet);
        }
        if facets.len() >= max_facets {
            assign_distinctive_facet_terms(&mut facets);
            return facets;
        }
    }

    let metadata_declares_coverage =
        !query_metadata.entities.is_empty() || !query_metadata.facets.is_empty();
    if metadata_declares_coverage && !facets.is_empty() {
        assign_distinctive_facet_terms(&mut facets);
        return facets;
    }
    if query_metadata.is_empty() && query_plan.len() > 1 {
        let base_key = clean_text(query, 600).to_ascii_lowercase();
        for text in query_plan {
            let cleaned = clean_text(text, 600);
            if cleaned.is_empty() || cleaned.to_ascii_lowercase() == base_key {
                continue;
            }
            if let Some(mut facet) = research_facet_from_text(&cleaned, facets.len(), min_terms) {
                let signature = research_facet_signature(&facet.terms);
                if !seen.insert(signature) {
                    continue;
                }
                facet.id = format!("facet_{:02}", facets.len() + 1);
                facets.push(facet);
            }
            if facets.len() >= max_facets {
                assign_distinctive_facet_terms(&mut facets);
                return facets;
            }
        }
        if !facets.is_empty() {
            assign_distinctive_facet_terms(&mut facets);
            return facets;
        }
    }
    let mut texts = Vec::<String>::new();
    let base = clean_text(query, 600);
    if !base.is_empty() {
        texts.push(base.clone());
    }

    for text in texts {
        if let Some(mut facet) = research_facet_from_text(&text, facets.len(), min_terms) {
            let signature = research_facet_signature(&facet.terms);
            if !seen.insert(signature) {
                continue;
            }
            facet.id = format!("facet_{:02}", facets.len() + 1);
            facets.push(facet);
        }
        if facets.len() >= max_facets {
            break;
        }
    }
    assign_distinctive_facet_terms(&mut facets);
    facets
}

fn candidate_facet_overlap(facet: &ResearchFacet, candidate: &Candidate) -> usize {
    let haystack = candidate_facet_haystack_terms(candidate);
    let haystack_stems = haystack
        .iter()
        .map(|term| relevance_term_stem(term))
        .collect::<HashSet<_>>();
    facet
        .terms
        .iter()
        .filter(|term| facet_term_present(term, &haystack, &haystack_stems))
        .count()
}

fn candidate_facet_haystack_terms(candidate: &Candidate) -> HashSet<String> {
    tokenize_relevance(
        &format!(
            "{} {} {}",
            candidate.title, candidate.snippet, candidate.locator
        ),
        360,
    )
}

fn relevance_term_stem(term: &str) -> String {
    let lower = term.to_ascii_lowercase();
    if lower.len() > 5 && lower.ends_with("ies") {
        return format!("{}y", &lower[..lower.len().saturating_sub(3)]);
    }
    if lower.len() > 4 && lower.ends_with("es") {
        return lower[..lower.len().saturating_sub(2)].to_string();
    }
    if lower.len() > 3 && lower.ends_with('s') && !lower.ends_with("ss") {
        return lower[..lower.len().saturating_sub(1)].to_string();
    }
    lower
}

fn facet_term_present(
    term: &str,
    haystack: &HashSet<String>,
    haystack_stems: &HashSet<String>,
) -> bool {
    haystack.contains(term) || haystack_stems.contains(&relevance_term_stem(term))
}

fn candidate_matches_facet(facet: &ResearchFacet, candidate: &Candidate, min_terms: usize) -> bool {
    let overlap = candidate_facet_overlap(facet, candidate);
    if overlap == 0 {
        return false;
    }
    if !facet.distinctive_terms.is_empty() {
        let haystack = candidate_facet_haystack_terms(candidate);
        let haystack_stems = haystack
            .iter()
            .map(|term| relevance_term_stem(term))
            .collect::<HashSet<_>>();
        if !facet
            .distinctive_terms
            .iter()
            .any(|term| facet_term_present(term, &haystack, &haystack_stems))
        {
            return false;
        }
    }
    let required = min_terms.min(facet.terms.len()).max(1);
    overlap >= required || (facet.kind == "entity" && facet.terms.len() <= 2 && overlap >= 1)
}

fn coverage_aware_score(
    base_query: &str,
    facets: &[ResearchFacet],
    candidate: &Candidate,
    min_terms: usize,
) -> f64 {
    let mut best = rerank_score(base_query, candidate);
    for facet in facets {
        let overlap = candidate_facet_overlap(facet, candidate);
        if overlap == 0 {
            continue;
        }
        let mut score = rerank_score(&facet.requested_text, candidate);
        if overlap >= min_terms.min(facet.terms.len()).max(1) {
            score += 0.08;
        } else {
            score += 0.03;
        }
        best = best.max(score);
    }
    best.clamp(0.0, 1.0)
}

fn candidate_identity_key(candidate: &Candidate) -> String {
    format!(
        "{}|{}|{}",
        candidate.locator.to_ascii_lowercase(),
        candidate.title.to_ascii_lowercase(),
        candidate.excerpt_hash
    )
}

fn select_facet_covered_ranked_candidates(
    ranked: Vec<(Candidate, f64)>,
    facets: &[ResearchFacet],
    max_evidence: usize,
    min_terms: usize,
) -> Vec<(Candidate, f64)> {
    if facets.is_empty() {
        return select_diverse_ranked_candidates(ranked, max_evidence);
    }
    let limit = max_evidence.max(1);
    let sorted = sorted_ranked_candidates(ranked);
    let mut selected = Vec::<(Candidate, f64)>::new();
    let mut selected_keys = HashSet::<String>::new();
    for facet in facets {
        if let Some(row) = sorted.iter().find(|(candidate, _)| {
            !selected_keys.contains(&candidate_identity_key(candidate))
                && candidate_matches_facet(facet, candidate, min_terms)
        }) {
            selected_keys.insert(candidate_identity_key(&row.0));
            selected.push(row.clone());
            if selected.len() >= limit {
                return selected;
            }
        }
    }
    let remaining = sorted
        .into_iter()
        .filter(|(candidate, _)| !selected_keys.contains(&candidate_identity_key(candidate)))
        .collect::<Vec<_>>();
    for row in select_diverse_ranked_candidates(remaining, limit.saturating_sub(selected.len())) {
        selected_keys.insert(candidate_identity_key(&row.0));
        selected.push(row);
        if selected.len() >= limit {
            break;
        }
    }
    selected
}

fn coverage_aware_max_evidence(facets: &[ResearchFacet], budget: ApertureBudget) -> usize {
    let required_entity_count = facets.iter().filter(|facet| facet.kind == "entity").count();
    let coverage_floor = if required_entity_count >= 2 {
        required_entity_count
    } else {
        0
    };
    budget
        .max_evidence
        .max(coverage_floor)
        .min(budget.max_candidates.max(1))
        .max(1)
}

fn ranked_candidate_already_selected(
    selected_keys: &HashSet<String>,
    candidate: &Candidate,
) -> bool {
    selected_keys.contains(&candidate_identity_key(candidate))
}

fn selected_candidate_coverage_ids(
    facets: &[ResearchFacet],
    candidate: &Candidate,
    min_terms: usize,
) -> Vec<String> {
    facets
        .iter()
        .filter(|facet| candidate_matches_facet(facet, candidate, min_terms))
        .map(|facet| facet.id.clone())
        .collect::<Vec<_>>()
}

fn redundant_facet_backfill_replacement_index(
    selected: &[(Candidate, f64)],
    facets: &[ResearchFacet],
    min_terms: usize,
) -> Option<usize> {
    let mut counts = HashMap::<String, usize>::new();
    let coverage_by_index = selected
        .iter()
        .map(|(candidate, _)| {
            let ids = selected_candidate_coverage_ids(facets, candidate, min_terms);
            for id in &ids {
                *counts.entry(id.clone()).or_insert(0) += 1;
            }
            ids
        })
        .collect::<Vec<_>>();
    selected
        .iter()
        .enumerate()
        .filter(|(idx, _)| {
            let ids = &coverage_by_index[*idx];
            ids.is_empty()
                || ids
                    .iter()
                    .all(|id| counts.get(id).copied().unwrap_or(0) > 1)
        })
        .min_by(|(_, left), (_, right)| {
            let left_rank = facet_backfill_replacement_rank(&left.0, left.1);
            let right_rank = facet_backfill_replacement_rank(&right.0, right.1);
            left_rank.total_cmp(&right_rank)
        })
        .map(|(idx, _)| idx)
}

fn facet_backfill_replacement_rank(candidate: &Candidate, score: f64) -> f64 {
    let low_confidence_penalty = if candidate_is_low_confidence_retained(candidate) {
        -2.0
    } else {
        0.0
    };
    let thin_penalty = if content_rich_text(&candidate.snippet) {
        0.0
    } else {
        -1.0
    };
    score + low_confidence_penalty + thin_penalty
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
    if facets.is_empty() || supplemental_pool.is_empty() || max_evidence == 0 {
        return 0;
    }
    let mut added = 0usize;
    let mut selected_keys = selected
        .iter()
        .map(|(candidate, _)| candidate_identity_key(candidate))
        .collect::<HashSet<_>>();
    for facet in facets {
        let already_covered = selected
            .iter()
            .any(|(candidate, _)| candidate_matches_facet(facet, candidate, min_terms));
        if already_covered {
            continue;
        }
        let mut candidates = supplemental_pool
            .iter()
            .filter(|(candidate, score)| {
                !ranked_candidate_already_selected(&selected_keys, candidate)
                    && candidate_matches_facet(facet, candidate, min_terms)
                    && (allow_low_confidence || !candidate_is_low_confidence_retained(candidate))
                    && candidate_retention_preview_eligible(query, candidate, *score)
            })
            .cloned()
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| {
            let left_low = candidate_is_low_confidence_retained(&left.0);
            let right_low = candidate_is_low_confidence_retained(&right.0);
            left_low
                .cmp(&right_low)
                .then_with(|| {
                    content_rich_text(&right.0.snippet).cmp(&content_rich_text(&left.0.snippet))
                })
                .then_with(|| right.1.total_cmp(&left.1))
                .then_with(|| candidate_domain_hint(&left.0).cmp(&candidate_domain_hint(&right.0)))
        });
        let Some(candidate) = candidates.into_iter().next() else {
            continue;
        };
        if selected.len() >= max_evidence {
            let Some(replace_index) =
                redundant_facet_backfill_replacement_index(selected, facets, min_terms)
            else {
                continue;
            };
            selected_keys.remove(&candidate_identity_key(&selected[replace_index].0));
            selected[replace_index] = candidate.clone();
        } else {
            selected.push(candidate.clone());
        }
        selected_keys.insert(candidate_identity_key(&candidate.0));
        added += 1;
    }
    added
}

fn truncate_candidates_preserving_facet_coverage(
    query: &str,
    facets: &[ResearchFacet],
    candidates: &mut Vec<Candidate>,
    max_candidates: usize,
    min_terms: usize,
) {
    if candidates.len() <= max_candidates {
        return;
    }
    if facets.is_empty() {
        candidates.truncate(max_candidates);
        return;
    }
    let ranked = candidates
        .iter()
        .map(|candidate| {
            (
                candidate.clone(),
                coverage_aware_score(query, facets, candidate, min_terms),
            )
        })
        .collect::<Vec<_>>();
    let selected =
        select_facet_covered_ranked_candidates(ranked, facets, max_candidates, min_terms);
    if selected.is_empty() {
        candidates.truncate(max_candidates);
    } else {
        *candidates = selected
            .into_iter()
            .map(|(candidate, _)| candidate)
            .collect::<Vec<_>>();
    }
}

fn candidate_coverage_facets(
    facets: &[ResearchFacet],
    candidate: &Candidate,
    min_terms: usize,
) -> Vec<String> {
    facets
        .iter()
        .filter(|facet| candidate_matches_facet(facet, candidate, min_terms))
        .map(|facet| facet.id.clone())
        .collect::<Vec<_>>()
}

fn evidence_coverage_from_ranked_candidates(
    query: &str,
    facets: &[ResearchFacet],
    evidence_ranked: &[(Candidate, f64)],
    min_terms: usize,
) -> Value {
    if facets.is_empty() {
        return json!([]);
    }
    Value::Array(
        facets
            .iter()
            .map(|facet| {
                let matching = evidence_ranked
                    .iter()
                    .filter(|(candidate, _)| candidate_matches_facet(facet, candidate, min_terms))
                    .collect::<Vec<_>>();
                let usable_count = matching
                    .iter()
                    .filter(|(candidate, score)| {
                        candidate_counts_as_query_usable_evidence(query, candidate, *score)
                    })
                    .count();
                let low_confidence_count = matching
                    .iter()
                    .filter(|(candidate, _)| candidate_is_low_confidence_retained(candidate))
                    .count();
                let candidate_only_count = matching
                    .iter()
                    .filter(|(candidate, _)| {
                        !candidate_is_low_confidence_retained(candidate)
                            && !candidate_counts_as_query_usable_evidence(
                                query,
                                candidate,
                                rerank_score(query, candidate),
                            )
                    })
                    .count();
                let mut domains = matching
                    .iter()
                    .map(|(candidate, _)| candidate_domain_hint(candidate).to_ascii_lowercase())
                    .filter(|domain| !domain.is_empty() && domain != "source")
                    .collect::<Vec<_>>();
                domains.sort();
                domains.dedup();
                let status = if usable_count > 0 {
                    "covered"
                } else if low_confidence_count > 0 || candidate_only_count > 0 {
                    "weak"
                } else {
                    "missing"
                };
                json!({
                    "facet_id": facet.id,
                    "facet_kind": facet.kind,
                    "requested_text": facet.requested_text,
                    "status": status,
                    "evidence_count": matching.len(),
                    "usable_evidence_count": usable_count,
                    "low_confidence_raw_count": low_confidence_count,
                    "candidate_only_count": candidate_only_count,
                    "source_domain_count": domains.len(),
                    "source_domains": domains
                })
            })
            .collect::<Vec<_>>(),
    )
}

fn ranked_evidence_covers_all_facets(
    query: &str,
    facets: &[ResearchFacet],
    evidence_ranked: &[(Candidate, f64)],
    min_terms: usize,
) -> bool {
    if facets.is_empty() {
        return !evidence_ranked.is_empty();
    }
    facets.iter().all(|facet| {
        evidence_ranked.iter().any(|(candidate, score)| {
            candidate_matches_facet(facet, candidate, min_terms)
                && candidate_counts_as_query_usable_evidence(query, candidate, *score)
        })
    })
}

fn evidence_selection_diagnostic_blockers(
    query: &str,
    candidate: &Candidate,
    score: f64,
    selected: bool,
    actionable: bool,
    benchmark_intent: bool,
    min_synthesis_score: f64,
) -> Vec<String> {
    if selected {
        return Vec::new();
    }
    let mut blockers = Vec::<String>::new();
    let snippet = clean_text(&candidate.snippet, 1_200);
    let domain = candidate_domain_hint(candidate);
    if snippet.is_empty() {
        blockers.push("empty_snippet".to_string());
    }
    if candidate_is_low_confidence_retained(candidate) {
        blockers.push("low_confidence_retained".to_string());
    }
    if score < min_synthesis_score {
        blockers.push("below_min_synthesis_score".to_string());
    }
    if looks_like_ack_only(&snippet) {
        blockers.push("ack_only".to_string());
    }
    if looks_like_low_signal_search_summary(&snippet) {
        blockers.push("low_signal_search_summary".to_string());
    }
    if looks_like_source_only_snippet(&snippet) {
        blockers.push("source_only_snippet".to_string());
    }
    if citation_wrapper_link(&candidate.locator) {
        blockers.push("citation_wrapper_link".to_string());
    }
    if is_search_engine_domain(&domain) {
        blockers.push("search_engine_domain".to_string());
    }
    if !candidate_passes_relevance_gate(query, candidate, benchmark_intent) {
        blockers.push("relevance_gate_failed".to_string());
    }
    if !candidate_is_substantive(query, candidate, benchmark_intent) {
        blockers.push("not_substantive".to_string());
    }
    if benchmark_intent && looks_like_definition_candidate(candidate) {
        blockers.push("definition_noise".to_string());
    }
    if benchmark_intent && looks_like_comparison_noise_candidate(candidate) {
        blockers.push("comparison_noise".to_string());
    }
    let quality_flags = candidate_quality_flags(query, candidate, score);
    let claim_hints = evidence_pack_claim_hints_for_candidate(query, candidate, 1);
    if !candidate_counts_as_usable_evidence(candidate) {
        blockers.push("not_materialized_or_non_evidence_payload".to_string());
    }
    if quality_flags_block_query_usable_evidence(query, candidate, &quality_flags) {
        blockers.push("quality_flags_block_usable_evidence".to_string());
    }
    if claim_hints.is_empty() {
        blockers.push("missing_claim_hints".to_string());
    }
    if actionable && !candidate_is_pack_ready_evidence(query, candidate, score) {
        blockers.push("not_pack_ready_evidence".to_string());
    }
    if actionable && blockers.is_empty() {
        blockers.push("not_selected_capacity_or_diversity".to_string());
    }
    blockers.sort();
    blockers.dedup();
    blockers
}

fn evidence_selection_diagnostics(
    query: &str,
    ranked_pool: &[(Candidate, f64)],
    actionable_ranked_pool: &[(Candidate, f64)],
    evidence_ranked: &[(Candidate, f64)],
    limit: usize,
) -> Value {
    let benchmark_intent = is_benchmark_or_comparison_intent(query);
    let min_synthesis_score = minimum_synthesis_score(benchmark_intent);
    let actionable_keys = actionable_ranked_pool
        .iter()
        .map(|(candidate, _)| candidate_locator_identity_key(candidate))
        .collect::<HashSet<_>>();
    let selected_keys = evidence_ranked
        .iter()
        .map(|(candidate, _)| candidate_locator_identity_key(candidate))
        .collect::<HashSet<_>>();
    let mut rows = ranked_pool
        .iter()
        .map(|(candidate, score)| {
            let key = candidate_locator_identity_key(candidate);
            let selected = selected_keys.contains(&key);
            let actionable = actionable_keys.contains(&key);
            let quality_flags = if candidate_is_low_confidence_retained(candidate) {
                vec!["low_confidence_raw".to_string()]
            } else {
                candidate_quality_flags(query, candidate, *score)
            };
            let claim_hints = evidence_pack_claim_hints_for_candidate(query, candidate, 2);
            let materialization_quality = candidate_materialization_quality(candidate);
            let blockers = evidence_selection_diagnostic_blockers(
                query,
                candidate,
                *score,
                selected,
                actionable,
                benchmark_intent,
                min_synthesis_score,
            );
            json!({
                "title": clean_text(&candidate.title, 240),
                "locator": clean_text(&candidate.locator, 2_200),
                "source_domain": candidate_domain_hint(candidate),
                "source_kind": candidate.source_kind.clone(),
                "score": (*score * 100.0).round() / 100.0,
                "selected": selected,
                "in_actionable_pool": actionable,
                "pack_ready": candidate_is_pack_ready_evidence(query, candidate, *score),
                "counts_as_usable_evidence": candidate_counts_as_query_usable_evidence(query, candidate, *score),
                "materialization_quality": materialization_quality,
                "freshness": evidence_pack_freshness_status(query, candidate),
                "content_rich": content_rich_text(&candidate.snippet),
                "claim_hint_count": claim_hints.len(),
                "claim_hints": claim_hints,
                "quality_flags": quality_flags,
                "blockers": blockers,
                "snippet": trim_words(&clean_text(&candidate.snippet, 1_200), 40)
            })
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        let left_selected = left
            .get("selected")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let right_selected = right
            .get("selected")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        right_selected
            .cmp(&left_selected)
            .then_with(|| {
                let left_actionable = left
                    .get("in_actionable_pool")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let right_actionable = right
                    .get("in_actionable_pool")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                right_actionable.cmp(&left_actionable)
            })
            .then_with(|| {
                let left_score = left.get("score").and_then(Value::as_f64).unwrap_or(0.0);
                let right_score = right.get("score").and_then(Value::as_f64).unwrap_or(0.0);
                right_score.total_cmp(&left_score)
            })
    });
    rows.truncate(limit.max(1));
    json!({
        "version": "evidence_selection_diagnostics_v1",
        "query": clean_text(query, 360),
        "ranked_pool_count": ranked_pool.len(),
        "actionable_pool_count": actionable_ranked_pool.len(),
        "selected_count": evidence_ranked.len(),
        "rows": rows
    })
}

fn usable_ranked_candidates_for_coverage(
    query: &str,
    facets: &[ResearchFacet],
    candidates: &[Candidate],
    min_terms: usize,
) -> Vec<(Candidate, f64)> {
    let benchmark_intent = is_benchmark_or_comparison_intent(query);
    let min_score = minimum_synthesis_score(benchmark_intent);
    candidates
        .iter()
        .map(|candidate| {
            let score = if facets.is_empty() {
                rerank_score(query, candidate)
            } else {
                coverage_aware_score(query, facets, candidate, min_terms)
            };
            (candidate.clone(), score)
        })
        .filter(|(candidate, score)| {
            candidate_counts_as_query_usable_evidence(query, candidate, *score)
                &&
            *score >= min_score
                && candidate_passes_relevance_gate(query, candidate, benchmark_intent)
                && candidate_is_substantive(query, candidate, benchmark_intent)
        })
        .collect::<Vec<_>>()
}

fn missing_research_facets_for_coverage(
    query: &str,
    facets: &[ResearchFacet],
    candidates: &[Candidate],
    min_terms: usize,
) -> Vec<ResearchFacet> {
    if facets.is_empty() {
        return Vec::new();
    }
    let usable = usable_ranked_candidates_for_coverage(query, facets, candidates, min_terms);
    facets
        .iter()
        .filter(|facet| {
            !usable
                .iter()
                .any(|(candidate, _)| candidate_matches_facet(facet, candidate, min_terms))
        })
        .cloned()
        .collect::<Vec<_>>()
}

fn coverage_gap_recovery_needed(
    policy: &Value,
    query: &str,
    facets: &[ResearchFacet],
    candidates: &[Candidate],
    budget: ApertureBudget,
) -> bool {
    if !coverage_gap_recovery_enabled(policy) || facets.is_empty() || candidates.is_empty() {
        return false;
    }
    let min_terms = facet_aware_min_terms(policy);
    let usable = usable_ranked_candidates_for_coverage(query, facets, candidates, min_terms);
    let min_usable = coverage_gap_recovery_min_usable_evidence(policy, budget);
    if usable.len() < min_usable {
        return true;
    }
    let covered = facets
        .iter()
        .filter(|facet| {
            usable
                .iter()
                .any(|(candidate, _)| candidate_matches_facet(facet, candidate, min_terms))
        })
        .count();
    covered < coverage_gap_recovery_min_covered_facets(policy, facets.len(), budget)
}

fn expand_recovery_template(
    template: &str,
    query: &str,
    facet: &str,
    entities: &str,
) -> Option<String> {
    let expanded = template
        .replace("{query}", query)
        .replace("{facet}", facet)
        .replace("{entities}", entities)
        .replace("{subjects}", entities);
    let cleaned = clean_text(&expanded, 600);
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

fn quote_recovery_subject(raw: &str) -> Option<String> {
    let cleaned = clean_text(raw, 160).replace('"', "");
    if cleaned.is_empty() {
        return None;
    }
    if cleaned.split_whitespace().count() > 1 {
        Some(format!("\"{cleaned}\""))
    } else {
        Some(cleaned)
    }
}

fn compact_recovery_entities(facets: &[ResearchFacet], missing_facet: &ResearchFacet) -> String {
    if missing_facet.kind == "entity" {
        return quote_recovery_subject(&missing_facet.requested_text).unwrap_or_default();
    }
    let mut seen = HashSet::<String>::new();
    let entities = facets
        .iter()
        .filter(|facet| facet.kind == "entity")
        .filter_map(|facet| quote_recovery_subject(&facet.requested_text))
        .filter(|entity| seen.insert(entity.to_ascii_lowercase()))
        .take(3)
        .collect::<Vec<_>>();
    clean_text(&entities.join(" "), 360)
}

fn coverage_gap_recovery_queries(
    policy: &Value,
    query: &str,
    existing_queries: &[String],
    facets: &[ResearchFacet],
    candidates: &[Candidate],
    budget: ApertureBudget,
) -> Vec<String> {
    if !coverage_gap_recovery_needed(policy, query, facets, candidates, budget) {
        return Vec::new();
    }
    let min_terms = facet_aware_min_terms(policy);
    let max_queries = coverage_gap_recovery_max_queries(policy, budget);
    let missing = missing_research_facets_for_coverage(query, facets, candidates, min_terms);
    if missing.is_empty() {
        return Vec::new();
    }
    let mut seen = existing_queries
        .iter()
        .map(|row| clean_text(row, 600).to_ascii_lowercase())
        .collect::<HashSet<_>>();
    let mut out = Vec::<String>::new();
    let templates = coverage_gap_recovery_templates(policy);
    for template in templates {
        for facet in &missing {
            let template = if template.contains("{facet}") {
                template.clone()
            } else {
                template.replace("{query}", "{facet}")
            };
            let entities = compact_recovery_entities(facets, facet);
            if template.contains("{entities}") && entities.is_empty() {
                continue;
            }
            if let Some(candidate) = expand_recovery_template(
                &template,
                query,
                &facet.requested_text,
                &entities,
            ) {
                if seen.insert(candidate.to_ascii_lowercase()) {
                    out.push(candidate);
                }
            }
            if out.len() >= max_queries {
                return out;
            }
        }
    }
    out
}

fn usable_candidate_claim_hint_count(query: &str, candidate: &Candidate) -> usize {
    let score = rerank_score(query, candidate);
    if !candidate_counts_as_query_usable_evidence(query, candidate, score) {
        return 0;
    }
    evidence_pack_claim_hints_for_candidate(query, candidate, 2).len()
}

fn weak_research_facets_for_claim_support(
    query: &str,
    facets: &[ResearchFacet],
    candidates: &[Candidate],
    min_terms: usize,
) -> Vec<ResearchFacet> {
    if facets.is_empty() {
        return Vec::new();
    }
    facets
        .iter()
        .filter(|facet| {
            let claim_hints = candidates
                .iter()
                .filter(|candidate| {
                    candidate_counts_as_query_usable_evidence(
                        query,
                        candidate,
                        rerank_score(query, candidate),
                    )
                        && candidate_matches_facet(facet, candidate, min_terms)
                })
                .map(|candidate| usable_candidate_claim_hint_count(query, candidate))
                .sum::<usize>();
            claim_hints == 0
        })
        .cloned()
        .collect::<Vec<_>>()
}

fn claim_gap_recovery_needed(
    policy: &Value,
    query: &str,
    facets: &[ResearchFacet],
    candidates: &[Candidate],
    budget: ApertureBudget,
) -> bool {
    if !claim_gap_recovery_enabled(policy) || candidates.is_empty() {
        return false;
    }
    let min_terms = facet_aware_min_terms(policy);
    let usable = usable_ranked_candidates_for_coverage(query, facets, candidates, min_terms);
    if usable.is_empty() {
        return false;
    }
    if usable.len() < claim_gap_recovery_min_materialized_evidence(policy, budget) {
        return true;
    }
    let claim_hint_count = usable
        .iter()
        .map(|(candidate, _)| usable_candidate_claim_hint_count(query, candidate))
        .sum::<usize>();
    claim_hint_count < claim_gap_recovery_min_claim_hints(policy, budget)
}

fn claim_gap_recovery_queries(
    policy: &Value,
    query: &str,
    existing_queries: &[String],
    facets: &[ResearchFacet],
    candidates: &[Candidate],
    budget: ApertureBudget,
) -> Vec<String> {
    if !claim_gap_recovery_needed(policy, query, facets, candidates, budget) {
        return Vec::new();
    }
    let min_terms = facet_aware_min_terms(policy);
    let weak_facets = weak_research_facets_for_claim_support(query, facets, candidates, min_terms);
    let max_queries = claim_gap_recovery_max_queries(policy, budget);
    let mut seen = existing_queries
        .iter()
        .map(|row| clean_text(row, 600).to_ascii_lowercase())
        .collect::<HashSet<_>>();
    let mut out = Vec::<String>::new();
    let templates = claim_gap_recovery_templates(policy);
    for template in &templates {
        if !weak_facets.is_empty() {
            for facet in &weak_facets {
                let entities = compact_recovery_entities(facets, facet);
                if template.contains("{entities}") && entities.is_empty() {
                    continue;
                }
                if let Some(candidate) = expand_recovery_template(
                    template,
                    query,
                    &facet.requested_text,
                    &entities,
                ) {
                    if seen.insert(candidate.to_ascii_lowercase()) {
                        out.push(candidate);
                    }
                }
                if out.len() >= max_queries {
                    return out;
                }
            }
        } else if let Some(candidate) = expand_recovery_template(template, query, query, "") {
            if seen.insert(candidate.to_ascii_lowercase()) {
                out.push(candidate);
            }
            if out.len() >= max_queries {
                return out;
            }
        }
    }
    out
}

fn evidence_pack_enabled(policy: &Value) -> bool {
    policy
        .pointer("/batch_query/evidence_pack/enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

fn evidence_pack_max_items(policy: &Value, max_evidence: usize) -> usize {
    if !evidence_pack_enabled(policy) {
        return 0;
    }
    policy
        .pointer("/batch_query/evidence_pack/max_items")
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or(max_evidence)
        .clamp(1, max_evidence.max(1))
}

fn evidence_pack_max_snippet_words(policy: &Value) -> usize {
    policy
        .pointer("/batch_query/evidence_pack/max_snippet_words")
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or(72)
        .clamp(16, 160)
}

fn policy_string_list(policy: &Value, pointer: &str) -> Vec<String> {
    policy
        .pointer(pointer)
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| clean_text(row, 160).to_ascii_lowercase())
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn locator_path_hint(raw: &str) -> String {
    let locator = clean_text(raw, 2_200).to_ascii_lowercase();
    let after_host = locator
        .split_once("://")
        .and_then(|(_, rest)| rest.find('/').map(|index| &rest[index..]))
        .unwrap_or(locator.as_str());
    let path = after_host.split(['?', '#']).next().unwrap_or(after_host);
    clean_text(path, 1_200)
}

fn evidence_pack_source_class(policy: &Value, candidate: &Candidate) -> String {
    let source_kind = clean_text(&candidate.source_kind, 80).to_ascii_lowercase();
    let locator_path = locator_path_hint(&candidate.locator);
    let domain = candidate_domain_hint(candidate).to_ascii_lowercase();
    let title = clean_text(&candidate.title, 600).to_ascii_lowercase();
    let snippet = clean_text(&candidate.snippet, 1_800).to_ascii_lowercase();
    let text = clean_text(&format!("{title} {snippet}"), 2_400).to_ascii_lowercase();
    if let Some(rules) = policy
        .pointer("/batch_query/evidence_pack/source_class_rules")
        .and_then(Value::as_array)
    {
        for rule in rules {
            let class = clean_text(rule.get("class").and_then(Value::as_str).unwrap_or(""), 80);
            if class.is_empty() {
                continue;
            }
            let source_kind_matches = policy_string_list(rule, "/source_kinds")
                .iter()
                .any(|value| value == &source_kind);
            let host_suffix_matches = policy_string_list(rule, "/host_suffixes")
                .iter()
                .any(|suffix| domain.ends_with(suffix));
            let host_contains_matches = policy_string_list(rule, "/host_contains")
                .iter()
                .any(|needle| domain.contains(needle));
            let path_contains_matches = policy_string_list(rule, "/path_contains")
                .iter()
                .any(|needle| locator_path.contains(needle));
            let title_contains_matches = policy_string_list(rule, "/title_contains")
                .iter()
                .any(|needle| title.contains(needle));
            let snippet_contains_matches = policy_string_list(rule, "/snippet_contains")
                .iter()
                .any(|needle| snippet.contains(needle));
            let text_contains_matches = policy_string_list(rule, "/text_contains")
                .iter()
                .any(|needle| text.contains(needle));
            if source_kind_matches
                || host_suffix_matches
                || host_contains_matches
                || path_contains_matches
                || title_contains_matches
                || snippet_contains_matches
                || text_contains_matches
            {
                return class;
            }
        }
    }
    if source_kind.is_empty() || source_kind == "web" {
        "general_web".to_string()
    } else {
        source_kind
    }
}

fn evidence_pack_freshness_status(query: &str, candidate: &Candidate) -> String {
    if !current_web_intent(query) {
        return "not_time_sensitive".to_string();
    }
    if recency_adjustment(query, candidate) >= 0.0 {
        "current_signal_present".to_string()
    } else {
        "freshness_unproven".to_string()
    }
}

fn materialization_quality_from_fields(
    source_kind: &str,
    permissions: &str,
    snippet: &str,
) -> &'static str {
    let source_kind = clean_text(source_kind, 120).to_ascii_lowercase();
    let permissions = clean_text(permissions, 240).to_ascii_lowercase();
    let snippet_rich = content_rich_text(snippet);
    let materialized = source_kind.contains("materialized")
        || source_kind.contains("page_enriched")
        || source_kind.contains("document_page_artifact")
        || source_kind.contains("reader_output")
        || source_kind.contains("rendered_page")
        || source_kind.contains("page_artifact")
        || permissions.contains("browser_materialized");
    if materialized {
        return if snippet_rich {
            "full_materialized"
        } else {
            "partial_materialized"
        };
    }
    let fetch_like = source_kind.contains("direct_fetch")
        || source_kind.contains("web_conduit_fetch")
        || source_kind.contains("http_get")
        || source_kind.contains("fetch_candidate")
        || permissions.contains("fetch_materialized");
    if fetch_like {
        return if snippet_rich {
            "partial_materialized"
        } else {
            "failed_materialization"
        };
    }
    let trusted_structured_feed = source_kind.contains("rss")
        || source_kind.contains("feed")
        || source_kind.contains("api")
        || permissions.contains("structured_feed")
        || permissions.contains("headline_feed");
    if trusted_structured_feed {
        return "trusted_structured_feed";
    }
    "candidate_only"
}

fn materialization_quality_counts_as_usable_evidence(materialization_quality: &str) -> bool {
    matches!(
        materialization_quality,
        "full_materialized" | "partial_materialized" | "trusted_structured_feed"
    )
}

fn candidate_materialization_quality(candidate: &Candidate) -> &'static str {
    materialization_quality_from_fields(
        &candidate.source_kind,
        candidate.permissions.as_deref().unwrap_or(""),
        &candidate.snippet,
    )
}

fn looks_like_style_or_script_dump(text: &str) -> bool {
    let lowered = clean_text(text, 2_400).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    contains_web_junk_marker(&lowered)
        || lowered.contains("copyright zendesk, inc")
        || lowered.contains("use of this source code is governed")
        || (lowered.contains(":root {") && lowered.contains("--"))
        || (lowered.contains("function(") && lowered.contains("var "))
        || (lowered.contains("document.addeventlistener") && lowered.contains("window."))
}

fn looks_like_hashtag_keyword_shell(text: &str) -> bool {
    let cleaned = clean_text(text, 1_800);
    let lowered = cleaned.to_ascii_lowercase();
    let hashtag_count = cleaned
        .split_whitespace()
        .filter(|token| token.starts_with('#'))
        .count();
    hashtag_count >= 3
        || (hashtag_count >= 1
            && lowered.contains("keywords:")
            && cleaned.split_whitespace().count() < 120)
}

fn looks_like_media_embed_shell(text: &str) -> bool {
    let lowered = clean_text(text, 2_400).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    lowered.contains("video player is not supported")
        || lowered.contains("your browser does not support the video tag")
        || (lowered.contains("social share")
            && lowered.contains("embed")
            && (lowered.contains("ad link") || lowered.contains("share this ad")))
}

fn looks_like_social_video_shell(candidate: &Candidate) -> bool {
    let combined = format!("{} {}", candidate.title, candidate.snippet);
    if looks_like_media_embed_shell(&combined) {
        return true;
    }
    let domain = candidate_domain_hint(candidate).to_ascii_lowercase();
    let social_video_domain = domain.contains("tiktok.com")
        || domain.contains("instagram.com")
        || domain.contains("facebook.com")
        || domain.contains("pinterest.");
    if !social_video_domain {
        return false;
    }
    looks_like_hashtag_keyword_shell(&format!(
        "{} {}",
        candidate.title, candidate.snippet
    ))
}

fn candidate_has_non_evidence_payload(candidate: &Candidate) -> bool {
    let combined = clean_text(
        &format!("{} {}", candidate.title, candidate.snippet),
        2_400,
    );
    contains_web_junk_marker(&combined)
        || looks_like_style_or_script_dump(&candidate.snippet)
        || looks_like_social_video_shell(candidate)
        || looks_like_link_directory_or_aggregator_shell(&candidate.snippet)
}

fn candidate_counts_as_usable_evidence(candidate: &Candidate) -> bool {
    !candidate_is_low_confidence_retained(candidate)
        && !candidate_has_non_evidence_payload(candidate)
        && materialization_quality_counts_as_usable_evidence(candidate_materialization_quality(
            candidate,
        ))
}

fn quality_flags_block_query_usable_evidence(
    query: &str,
    candidate: &Candidate,
    quality_flags: &[String],
) -> bool {
    if quality_flags.iter().any(|flag| {
        matches!(
            flag.as_str(),
                "junk_marker"
                    | "style_or_script_dump"
                    | "social_video_shell"
                    | "link_directory_or_aggregator_shell"
                    | "weak_query_overlap_only"
                    | "low_score"
        )
    }) {
        return true;
    }
    let thin_overlap = quality_flags
        .iter()
        .any(|flag| flag == "thin_query_overlap");
    if current_web_intent(query)
        && quality_flags
            .iter()
            .any(|flag| flag == "freshness_unproven")
    {
        return true;
    }
    if !thin_overlap {
        return false;
    }
    let materialization_quality = candidate_materialization_quality(candidate);
    is_benchmark_or_comparison_intent(query)
        || matches!(
            materialization_quality,
            "candidate_only" | "trusted_structured_feed"
        )
}

fn candidate_counts_as_query_usable_evidence(
    query: &str,
    candidate: &Candidate,
    score: f64,
) -> bool {
    if !candidate_counts_as_usable_evidence(candidate) {
        return false;
    }
    let quality_flags = candidate_quality_flags(query, candidate, score);
    !quality_flags_block_query_usable_evidence(query, candidate, &quality_flags)
}

fn evidence_row_materialization_quality(row: &Value) -> String {
    if let Some(explicit) = row.get("materialization_quality").and_then(Value::as_str) {
        let cleaned = clean_text(explicit, 80);
        if !cleaned.is_empty() {
            return cleaned;
        }
    }
    materialization_quality_from_fields(
        row.get("source_kind").and_then(Value::as_str).unwrap_or(""),
        row.get("permissions").and_then(Value::as_str).unwrap_or(""),
        row.get("snippet")
            .or_else(|| row.get("summary"))
            .or_else(|| row.get("content"))
            .or_else(|| row.get("markdown"))
            .or_else(|| row.get("text"))
            .and_then(Value::as_str)
            .unwrap_or(""),
    )
    .to_string()
}

fn evidence_row_counts_as_usable_evidence(row: &Value) -> bool {
    if let Some(explicit) = row
        .get("counts_as_usable_evidence")
        .and_then(Value::as_bool)
    {
        return explicit;
    }
    let low_confidence =
        row.get("confidence").and_then(Value::as_str) == Some("low_confidence_raw");
    let quality_flags = row
        .get("quality_flags")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| row.to_ascii_lowercase())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    !low_confidence
        && !quality_flags.iter().any(|flag| {
            matches!(
                flag.as_str(),
                "junk_marker"
                    | "style_or_script_dump"
                    | "social_video_shell"
                    | "weak_query_overlap_only"
                    | "thin_query_overlap"
                    | "low_score"
            )
        })
        && materialization_quality_counts_as_usable_evidence(&evidence_row_materialization_quality(
            row,
        ))
}

fn claim_hint_normalized_snippet(snippet: &str) -> String {
    let mut normalized = clean_text(snippet, 2_400)
        .replace("&#x27;", "'")
        .replace("&#39;", "'")
        .replace("&#039;", "'")
        .replace("&quot;", "\"")
        .replace("&amp;", "&")
        .replace("&mdash;", "-")
        .replace("&ndash;", "-")
        .replace("â", "'")
        .replace("â", "'")
        .replace("â", "\"")
        .replace("â", "\"")
        .replace("â", "-")
        .replace("â", "-")
        .replace(" Share Save Add as preferred on Google ", ". ")
        .replace(" Share Save.", ".")
        .replace(" Share Save ", ". ")
        .replace(" Add as preferred on Google ", ". ")
        .replace(" - https://", ". https://")
        .replace(" — https://", ". https://")
        .replace(" - http://", ". http://")
        .replace(" — http://", ". http://")
        .replace(" - www.", ". www.")
        .replace(" — www.", ". www.")
        .replace("[...]", ".")
        .replace("[…]", ".")
        .replace('…', ".")
        .replace("https://", ". ")
        .replace("http://", ". ")
        .replace("www.", ". ");
    for marker in ["####", "###", "##", "#"] {
        normalized = normalized.replace(marker, ".");
    }
    normalized
}

fn claim_text_has_urlish_fragment(text: &str) -> bool {
    let lowered = clean_text(text, 520).to_ascii_lowercase();
    lowered.contains("http://")
        || lowered.contains("https://")
        || lowered.contains("www.")
        || lowered.contains(".com/")
        || lowered.contains(".org/")
        || lowered.contains(".net/")
        || lowered.contains(".dev/")
        || lowered.contains(".ai/")
        || lowered.contains(".io/")
        || lowered.contains("com/")
        || lowered.contains("org/")
        || lowered.contains("net/")
        || lowered.contains("dev/")
        || lowered.contains("ai/")
        || lowered.contains("io/")
}

fn claim_text_looks_like_page_or_subscription_boilerplate(text: &str) -> bool {
    let lowered = clean_text(text, 520).to_ascii_lowercase();
    [
        "affiliate link",
        "affiliate links",
        "all rights reserved",
        "account today",
        "amazon associate",
        "by clicking",
        "cookie policy",
        "earn from qualifying purchases",
        "enter email",
        "explore ways to use",
        "for your next news release",
        "load more",
        "latest news, sport and opinion",
        "live updates",
        "privacy policy",
        "published:",
        "share save",
        "see the latest features",
        "sign up",
        "skip to main content",
        "source:",
        "subscribe",
        "terms of service",
        "tos and privacy",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn claim_text_has_dangling_tail(text: &str) -> bool {
    let cleaned = clean_text(text, 520);
    let Some(last) = cleaned
        .split_whitespace()
        .last()
        .map(|token| token.trim_matches(|ch: char| !ch.is_ascii_alphanumeric()))
    else {
        return true;
    };
    matches!(
        last.to_ascii_lowercase().as_str(),
        "a" | "an"
            | "and"
            | "as"
            | "at"
            | "by"
            | "for"
            | "from"
            | "her"
            | "his"
            | "in"
            | "of"
            | "or"
            | "the"
            | "their"
            | "to"
            | "with"
    )
}

fn claim_text_is_synthesis_safe(text: &str) -> bool {
    let cleaned = clean_text(text, 520);
    !cleaned.is_empty()
        && !claim_text_has_urlish_fragment(&cleaned)
        && !claim_text_looks_like_title_bar_or_source_separator(&cleaned)
        && !claim_text_looks_like_page_or_subscription_boilerplate(&cleaned)
        && !looks_like_link_directory_or_aggregator_shell(&cleaned)
        && !claim_text_has_dangling_tail(&cleaned)
}

fn claim_text_looks_like_title_bar_or_source_separator(text: &str) -> bool {
    let cleaned = clean_text(text, 520);
    cleaned.contains(" | ")
        || cleaned.ends_with('—')
        || cleaned.ends_with(" -")
        || cleaned.ends_with('-')
}

fn claim_hint_segment_is_substantive(segment: &str) -> bool {
    let cleaned = clean_text(segment, 420);
    if cleaned.split_whitespace().count() < 6 {
        return false;
    }
    let lowered = cleaned.to_ascii_lowercase();
    if lowered.contains("[...]")
        || lowered.contains("[…]")
        || lowered.contains("####")
        || lowered.contains("###")
        || lowered.contains("##")
        || !claim_text_is_synthesis_safe(&cleaned)
    {
        return false;
    }
    let alpha_count = cleaned.chars().filter(|ch| ch.is_ascii_alphabetic()).count();
    let visible_count = cleaned.chars().filter(|ch| !ch.is_whitespace()).count().max(1);
    if alpha_count < 18 || alpha_count * 2 < visible_count {
        return false;
    }
    [
        " is ",
        " are ",
        " was ",
        " were ",
        " has ",
        " have ",
        " had ",
        " can ",
        " could ",
        " should ",
        " will ",
        " would ",
        " supports ",
        " includes ",
        " covers ",
        " compares ",
        " describes ",
        " documents ",
        " documented ",
        " emphasizes ",
        " explains ",
        " features ",
        " provides ",
        " produces ",
        " uses ",
        " identifies ",
        " recognizes ",
        " recognises ",
        " avoids ",
        " removes ",
        " tackles ",
        " detects ",
        " senses ",
        " navigates ",
        " cleans ",
        " picks up ",
        " de-tangles ",
        " detangles ",
        " engineered ",
        " designed ",
        " built ",
        " rated ",
        " publish ",
        " publishes ",
        " published ",
        " reports ",
        " reported ",
        " found ",
        " shows ",
        " says ",
        " said ",
        " warns ",
        " warned ",
        " according to ",
        " producing ",
        " rebuilding ",
        " urges ",
        " urged ",
        " condemns ",
        " condemned ",
        " agrees ",
        " agreed ",
        " meets ",
        " met ",
        " reviews ",
        " reviewed ",
        " holds ",
        " held ",
        " joins ",
        " joined ",
        " launches ",
        " launched ",
        " approves ",
        " approved ",
        " confirms ",
        " confirmed ",
        " blocks ",
        " blocked ",
        " shifts ",
        " shifted ",
        " excel ",
        " excels ",
        " cleaned ",
        " announced ",
        " released ",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn evidence_pack_claim_hints(query: &str, snippet: &str, limit: usize) -> Vec<String> {
    let query_terms = tokenize_relevance(query, 40);
    let broad_current_query_without_distinctive_terms =
        current_web_intent(query) && !query_has_distinctive_relevance_terms(query);
    let mut out = Vec::<String>::new();
    let normalized = claim_hint_normalized_snippet(snippet);
    let snippet_has_current_signal = segment_has_current_signal(&normalized);
    for segment in normalized.split(|ch| matches!(ch, '.' | '\n' | '\r')) {
        let cleaned = clean_text(segment, 420);
        if !claim_hint_segment_is_substantive(&cleaned) {
            continue;
        }
        let segment_terms = tokenize_relevance(&cleaned, 80);
        let has_query_overlap = query_terms.is_empty()
            || query_terms
                .iter()
                .any(|term| segment_terms.contains(term.as_str()))
            || (broad_current_query_without_distinctive_terms
                && segment_has_current_signal(&cleaned));
        let has_query_overlap = has_query_overlap
            || (broad_current_query_without_distinctive_terms && snippet_has_current_signal);
        if !has_query_overlap {
            continue;
        }
        let mut claim = trim_words(&cleaned, 48);
        if !claim_text_is_synthesis_safe(&claim) && claim_text_is_synthesis_safe(&cleaned) {
            claim = cleaned.clone();
        }
        if !claim_text_is_synthesis_safe(&claim) {
            continue;
        }
        out.push(claim);
        if out.len() >= limit.max(1) {
            break;
        }
    }
    out
}

fn evidence_pack_fallback_claim_hints_for_candidate(
    query: &str,
    candidate: &Candidate,
    limit: usize,
) -> Vec<String> {
    if !candidate_counts_as_query_usable_evidence(query, candidate, rerank_score(query, candidate))
    {
        return Vec::new();
    }
    let trusted_source = source_trust_adjustment(candidate) >= 0.15;
    let snippet = claim_hint_normalized_snippet(&candidate.snippet);
    if snippet.is_empty()
        || looks_like_low_signal_search_summary(&snippet)
        || looks_like_source_only_snippet(&snippet)
        || contains_web_junk_marker(&snippet)
        || looks_like_style_or_script_dump(&snippet)
        || looks_like_hashtag_keyword_shell(&snippet)
        || looks_like_media_embed_shell(&snippet)
        || looks_like_link_directory_or_aggregator_shell(&snippet)
    {
        return Vec::new();
    }

    let query_terms = tokenize_relevance(query, 40);
    let mut out = Vec::<String>::new();
    for segment in snippet.split(|ch| matches!(ch, '.' | ';' | '\n' | '\r')) {
        let cleaned = clean_text(segment, 420);
        let word_count = cleaned.split_whitespace().count();
        let alpha_count = cleaned.chars().filter(|ch| ch.is_ascii_alphabetic()).count();
        let visible_count = cleaned.chars().filter(|ch| !ch.is_whitespace()).count().max(1);
        if word_count < 7
            || alpha_count < 24
            || alpha_count * 2 < visible_count
            || !claim_text_is_synthesis_safe(&cleaned)
            || claim_text_looks_like_page_or_subscription_boilerplate(&cleaned)
            || looks_like_low_signal_search_summary(&cleaned)
            || looks_like_source_only_snippet(&cleaned)
            || looks_like_link_directory_or_aggregator_shell(&cleaned)
        {
            continue;
        }
        let segment_terms = tokenize_relevance(&cleaned, 80);
        let has_query_overlap = query_terms.is_empty()
            || query_terms
                .iter()
                .any(|term| segment_terms.contains(term.as_str()));
        if !has_query_overlap && !trusted_source {
            continue;
        }
        let claim = trim_words(&cleaned, 48);
        if claim_text_is_synthesis_safe(&claim) {
            out.push(claim);
        }
        if out.len() >= limit.max(1) {
            break;
        }
    }
    out
}

fn evidence_pack_claim_hints_for_candidate(
    query: &str,
    candidate: &Candidate,
    limit: usize,
) -> Vec<String> {
    let mut context = clean_text(&candidate.snippet, 2_400);
    let locator = clean_text(&candidate.locator, 2_200);
    if !locator.is_empty() {
        context.push(' ');
        context.push_str(&locator);
    }
    if let Some(timestamp) = candidate.timestamp.as_deref() {
        let timestamp = clean_text(timestamp, 80);
        if !timestamp.is_empty() {
            context.push(' ');
            context.push_str(&timestamp);
        }
    }
    let hints = evidence_pack_claim_hints(query, &context, limit);
    if hints.is_empty() {
        evidence_pack_fallback_claim_hints_for_candidate(query, candidate, limit)
    } else {
        hints
    }
}

fn content_rich_text(text: &str) -> bool {
    let cleaned = clean_text(text, 1_800);
    if cleaned.split_whitespace().count() < 18 {
        return false;
    }
    !looks_like_low_signal_search_summary(&cleaned)
        && !looks_like_source_only_snippet(&cleaned)
        && !contains_web_junk_marker(&cleaned)
        && !looks_like_style_or_script_dump(&cleaned)
        && !looks_like_hashtag_keyword_shell(&cleaned)
        && !looks_like_media_embed_shell(&cleaned)
        && !looks_like_link_directory_or_aggregator_shell(&cleaned)
        && !looks_like_ack_only(&cleaned)
}

fn looks_like_link_directory_or_aggregator_shell(text: &str) -> bool {
    let cleaned = clean_text(text, 2_400);
    if cleaned.is_empty() {
        return false;
    }
    let word_count = cleaned.split_whitespace().count();
    if word_count < 28 {
        return false;
    }
    let domain_count = extract_domains_from_text(&cleaned, 8).len();
    let link_separator_count = cleaned.matches(" — http").count()
        + cleaned.matches(" - http").count()
        + cleaned.matches(" — www.").count()
        + cleaned.matches(" - www.").count();
    if domain_count >= 2 && link_separator_count >= 1 {
        return true;
    }
    let lowered = cleaned.to_ascii_lowercase();
    let navigation_marker_hits = [
        "menu",
        "sections",
        "top stories",
        "latest",
        "newsletter",
        "subscribe",
        "follow us",
        "more news",
    ]
    .iter()
    .filter(|marker| lowered.contains(**marker))
    .count();
    if navigation_marker_hits >= 2 && word_count < 260 {
        return true;
    }
    let index_marker_hits = [
        " the latest",
        " latest ",
        " top stories",
        " headlines",
        " more ",
        " sections",
        " articles",
        " results",
        " posts",
    ]
    .iter()
    .filter(|marker| lowered.contains(**marker))
    .count();
    if word_count < 260
        && index_marker_hits >= 1
        && repeated_leading_topic_phrase_in_index_text(&lowered)
    {
        return true;
    }
    if word_count < 260 && index_marker_hits >= 2 && segment_has_current_signal(&cleaned) {
        return true;
    }
    let relative_time_hits = [
        " minutes ago",
        " minute ago",
        " hours ago",
        " hour ago",
        " days ago",
        " day ago",
        " yesterday",
    ]
    .iter()
    .map(|marker| lowered.matches(marker).count())
    .sum::<usize>();
    if relative_time_hits >= 2
        && word_count < 260
        && [" news", "latest", "top ", "international", "world"]
            .iter()
            .any(|marker| lowered.contains(marker))
    {
        return true;
    }
    let directory_marker_hits = [
        "candidate sources:",
        "found sources:",
        "headlines:",
        "recent editions",
        "source:",
        "top stories",
    ]
    .iter()
    .filter(|marker| lowered.contains(**marker))
    .count();
    domain_count >= 2 && directory_marker_hits >= 1 && word_count < 220
}

fn repeated_leading_topic_phrase_in_index_text(lowered: &str) -> bool {
    let words: Vec<String> = lowered
        .split_whitespace()
        .take(18)
        .map(|word| {
            word.trim_matches(|ch: char| !ch.is_ascii_alphanumeric())
                .to_string()
        })
        .filter(|word| word.len() >= 3)
        .collect();
    if words.len() < 5 {
        return false;
    }
    for phrase_len in 2..=3 {
        if words.len() < phrase_len * 2 {
            continue;
        }
        for i in 0..=words.len().saturating_sub(phrase_len * 2) {
            let phrase = &words[i..i + phrase_len];
            if phrase.iter().all(|word| {
                matches!(
                    word.as_str(),
                    "and" | "for" | "from" | "latest" | "more" | "news" | "the" | "top"
                )
            }) {
                continue;
            }
            for j in i + phrase_len..=words.len().saturating_sub(phrase_len) {
                if words[j..j + phrase_len] == *phrase {
                    return true;
                }
            }
        }
    }
    false
}

fn evidence_pack_term_hints(query: &str, candidate: &Candidate, limit: usize) -> Vec<String> {
    let mut seen = HashSet::<String>::new();
    let mut out = Vec::<String>::new();
    for term in tokenize_relevance(
        &format!(
            "{} {} {}",
            candidate.title, candidate.snippet, candidate.locator
        ),
        160,
    ) {
        if term.len() < 4 {
            continue;
        }
        if query.to_ascii_lowercase().contains(term.as_str()) {
            continue;
        }
        if seen.insert(term.clone()) {
            out.push(term);
        }
        if out.len() >= limit.max(1) {
            break;
        }
    }
    out
}

fn locator_has_credentials(locator: &str) -> bool {
    let lowered = clean_text(locator, 2_200).to_ascii_lowercase();
    if let Some(rest) = lowered
        .strip_prefix("https://")
        .or_else(|| lowered.strip_prefix("http://"))
    {
        let authority = rest.split('/').next().unwrap_or("");
        return authority.contains('@');
    }
    false
}

fn locator_has_internal_host_hint(locator: &str) -> bool {
    let lowered = clean_text(locator, 2_200).to_ascii_lowercase();
    let host = lowered
        .strip_prefix("https://")
        .or_else(|| lowered.strip_prefix("http://"))
        .unwrap_or(&lowered)
        .split('/')
        .next()
        .unwrap_or("")
        .split('@')
        .last()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("");
    host == "localhost"
        || host == "0.0.0.0"
        || host.starts_with("127.")
        || host.starts_with("10.")
        || host.starts_with("192.168.")
        || host.starts_with("169.254.")
        || host.starts_with("172.16.")
        || host.starts_with("172.17.")
        || host.starts_with("172.18.")
        || host.starts_with("172.19.")
        || host.starts_with("172.20.")
        || host.starts_with("172.21.")
        || host.starts_with("172.22.")
        || host.starts_with("172.23.")
        || host.starts_with("172.24.")
        || host.starts_with("172.25.")
        || host.starts_with("172.26.")
        || host.starts_with("172.27.")
        || host.starts_with("172.28.")
        || host.starts_with("172.29.")
        || host.starts_with("172.30.")
        || host.starts_with("172.31.")
}

fn locator_scheme(locator: &str) -> String {
    let lowered = clean_text(locator, 2_200).to_ascii_lowercase();
    lowered
        .split_once("://")
        .map(|(scheme, _)| clean_text(scheme, 40))
        .filter(|scheme| !scheme.is_empty())
        .unwrap_or_else(|| "none".to_string())
}

fn materialization_url_safety_assessment(locator: &str) -> Value {
    let cleaned = clean_text(locator, 2_200);
    let lowered = cleaned.to_ascii_lowercase();
    let http_https = lowered.starts_with("http://") || lowered.starts_with("https://");
    let credentials_in_url = locator_has_credentials(&cleaned);
    let internal_host_hint = locator_has_internal_host_hint(&cleaned);
    let mut rejection_reasons = Vec::<&str>::new();
    if cleaned.is_empty() {
        rejection_reasons.push("missing_locator");
    }
    if !http_https {
        rejection_reasons.push("non_http_https_scheme");
    }
    if credentials_in_url {
        rejection_reasons.push("credentials_in_url");
    }
    if internal_host_hint {
        rejection_reasons.push("internal_host_hint");
    }
    rejection_reasons.sort();
    rejection_reasons.dedup();
    let materialization_allowed = rejection_reasons.is_empty();
    let status = if materialization_allowed {
        "allowed_public_http_https"
    } else if internal_host_hint {
        "blocked_internal_host_hint"
    } else if credentials_in_url {
        "blocked_url_credentials"
    } else if !http_https {
        "blocked_non_http_https_scheme"
    } else {
        "blocked_by_policy"
    };
    json!({
        "version": "url_safety_assessment_v1",
        "status": status,
        "scheme": locator_scheme(&cleaned),
        "materialization_allowed": materialization_allowed,
        "http_https": http_https,
        "credentials_in_url": credentials_in_url,
        "internal_host_hint": internal_host_hint,
        "rejection_reasons": rejection_reasons,
        "redirect_revalidation_required": materialization_allowed,
        "final_url_must_match_safety_contract": true,
        "raw_payload_chat_visible": false
    })
}

fn materialization_url_allowed(locator: &str) -> bool {
    materialization_url_safety_assessment(locator)
        .get("materialization_allowed")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn materialization_candidate_safety_refs(
    actionable_ranked: &[(Candidate, f64)],
    limit: usize,
) -> Vec<Value> {
    actionable_ranked
        .iter()
        .take(limit.max(1))
        .map(|(candidate, score)| {
            json!({
                "locator": clean_text(&candidate.locator, 240),
                "domain": candidate_domain_hint(candidate),
                "score": (*score * 100.0).round() / 100.0,
                "url_safety": materialization_url_safety_assessment(&candidate.locator)
            })
        })
        .collect()
}

fn browser_profile_compilation_report() -> Value {
    json!({
        "version": "browser_profile_compilation_v1",
        "status": "contract_ready_default_off",
        "profile_source": "tool_cd_policy",
        "default_profile": "stateless_public_materialization",
        "effective_profile_required_before_launch": true,
        "raw_launch_args_accepted_from_caller": false,
        "denied_caller_fields": [
            "raw_browser_args",
            "raw_launch_args",
            "remote_debugging_flags",
            "certificate_bypass_flags",
            "local_file_access_flags",
            "extension_load_flags",
            "proxy_session_fields_without_capability",
            "raw_scripts_or_cdp_commands"
        ],
        "separately_admitted_capabilities": [
            "proxy",
            "persistent_session",
            "humanized_interaction",
            "service_pool"
        ],
        "chat_visibility": "telemetry_only",
        "non_goals": [
            "do_not_make_browser_materialization_default",
            "do_not_accept_raw_browser_flags_from_workflow",
            "do_not_smuggle_proxy_or_session_authority_through_request_shape"
        ]
    })
}

fn browser_capability_readiness_lifecycle_report() -> Value {
    json!({
        "version": "browser_capability_readiness_lifecycle_v1",
        "status": "not_configured_default_off",
        "ordinary_research_may_install_dependency": false,
        "ordinary_research_may_launch_browser": false,
        "states": [
            "not_configured",
            "not_installed",
            "version_mismatch",
            "ready",
            "degraded",
            "blocked",
            "cleanup_required"
        ],
        "cheap_status_probe_allowed": true,
        "install_or_update_requires_explicit_capability_action": true,
        "cleanup_state_must_be_reported_separately": true,
        "chat_visibility": "telemetry_only",
        "non_goals": [
            "do_not_surprise_install_browser_binary_during_research",
            "do_not_treat_missing_browser_as_search_failure",
            "do_not_hide_provider_readiness_state_inside_low_signal_quality"
        ]
    })
}

fn evidence_promotion_assessment(
    query: &str,
    candidate: &Candidate,
    score: f64,
    quality_flags: &[String],
    claim_hints: &[String],
    coverage_facets: &[String],
) -> Value {
    let locator = clean_text(&candidate.locator, 2_200);
    let http_https = locator.starts_with("http://") || locator.starts_with("https://");
    let credentials_in_url = locator_has_credentials(&locator);
    let internal_host_hint = locator_has_internal_host_hint(&locator);
    let content_rich = content_rich_text(&candidate.snippet);
    let query_overlap_count = query_overlap_terms(query, candidate);
    let blocker_absent = !quality_flags
        .iter()
        .any(|flag| matches!(flag.as_str(), "junk_marker" | "low_trust_source"));
    let mut caveats = Vec::<&str>::new();
    if candidate_is_low_confidence_retained(candidate) {
        caveats.push("low_confidence_raw_retained");
    }
    if !content_rich {
        caveats.push("content_not_rich");
    }
    if claim_hints.is_empty() {
        caveats.push("claim_hints_missing");
    }
    if query_overlap_count < 2 {
        caveats.push("thin_query_overlap");
    }
    if quality_flags
        .iter()
        .any(|flag| flag == "weak_query_overlap_only")
    {
        caveats.push("weak_query_overlap_only");
    }
    if credentials_in_url {
        caveats.push("url_credentials_present");
    }
    if internal_host_hint {
        caveats.push("internal_host_hint");
    }
    if !http_https {
        caveats.push("non_http_locator");
    }
    for flag in quality_flags {
        if matches!(
            flag.as_str(),
            "freshness_unproven" | "low_score" | "low_trust_source" | "junk_marker"
        ) && !caveats.iter().any(|existing| existing == flag)
        {
            caveats.push(flag);
        }
    }
    caveats.sort();
    caveats.dedup();
    let safety_status = if !http_https || credentials_in_url || internal_host_hint {
        "unsafe_or_internal_hint"
    } else {
        "public_http_https_candidate"
    };
    let decision = if candidate_is_low_confidence_retained(candidate) {
        "retained_low_confidence"
    } else if quality_flags
        .iter()
        .any(|flag| flag == "weak_query_overlap_only")
    {
        "rejected_weak_query_overlap"
    } else if safety_status != "public_http_https_candidate"
        || !content_rich
        || claim_hints.is_empty()
        || quality_flags.iter().any(|flag| flag == "junk_marker")
    {
        "promoted_with_caveats"
    } else {
        "promoted"
    };
    json!({
        "version": "evidence_promotion_v1",
        "decision": decision,
        "safety": {
            "status": safety_status,
            "http_https": http_https,
            "credentials_in_url": credentials_in_url,
            "internal_host_hint": internal_host_hint,
            "raw_payload_chat_visible": false,
            "url_safety": materialization_url_safety_assessment(&locator)
        },
        "components": {
            "query_overlap_terms": query_overlap_count,
            "content_rich": content_rich,
            "claim_hint_count": claim_hints.len(),
            "coverage_facet_count": coverage_facets.len(),
            "source_trust_delta": (source_trust_adjustment(candidate) * 100.0).round() / 100.0,
            "freshness_status": evidence_pack_freshness_status(query, candidate),
            "score": (score * 100.0).round() / 100.0,
            "blocker_absent": blocker_absent,
            "permissions": candidate.permissions.clone().unwrap_or_else(|| "unknown".to_string())
        },
        "caveats": caveats,
        "non_goals": [
            "promotion_metadata_is_not_final_answer_format",
            "browser_enrichment_is_not_a_trust_override",
            "raw_payload_is_not_chat_visible"
        ]
    })
}

fn evidence_pack_from_ranked_candidates(
    policy: &Value,
    query: &str,
    facets: &[ResearchFacet],
    min_terms: usize,
    actionable_ranked: &[(Candidate, f64)],
    max_evidence: usize,
) -> Value {
    let max_items = evidence_pack_max_items(policy, max_evidence);
    if max_items == 0 {
        return json!([]);
    }
    let max_snippet_words = evidence_pack_max_snippet_words(policy);
    Value::Array(
        actionable_ranked
            .iter()
            .take(max_items)
            .map(|(candidate, score)| {
                let domain = candidate_domain_hint(candidate);
                let quality_flags = if candidate_is_low_confidence_retained(candidate) {
                    vec!["low_confidence_raw".to_string()]
                } else {
                    candidate_quality_flags(query, candidate, *score)
                };
                let coverage_facets = candidate_coverage_facets(facets, candidate, min_terms);
                let claim_hints = evidence_pack_claim_hints_for_candidate(query, candidate, 2);
                let counts_as_usable =
                    candidate_counts_as_query_usable_evidence(query, candidate, *score)
                        && !claim_hints.is_empty();
                let confidence = if candidate_is_low_confidence_retained(candidate) {
                    "low_confidence_raw"
                } else if counts_as_usable {
                    "usable"
                } else {
                    "candidate_only"
                };
                let term_hints = evidence_pack_term_hints(query, candidate, 8);
                let promotion = evidence_promotion_assessment(
                    query,
                    candidate,
                    *score,
                    &quality_flags,
                    &claim_hints,
                    &coverage_facets,
                );
                let materialization_quality = candidate_materialization_quality(candidate);
                json!({
                    "pack_version": "evidence_pack_v1",
                    "source_kind": candidate.source_kind.clone(),
                    "source_class": evidence_pack_source_class(policy, candidate),
                    "title": clean_text(&candidate.title, 240),
                    "locator": clean_text(&candidate.locator, 2_200),
                    "source_scope": domain,
                    "source_domain": domain,
                    "snippet": trim_words(&clean_text(&candidate.snippet, 1_800), max_snippet_words),
                    "claim_hints": claim_hints,
                    "term_hints": term_hints,
                    "excerpt_hash": candidate.excerpt_hash.clone(),
                    "score": (*score * 100.0).round() / 100.0,
                    "score_components": {
                        "relevance": (*score * 100.0).round() / 100.0,
                        "source_trust_delta": (source_trust_adjustment(candidate) * 100.0).round() / 100.0,
                        "freshness_delta": (recency_adjustment(query, candidate) * 100.0).round() / 100.0
                    },
                    "confidence": confidence,
                    "materialization_quality": materialization_quality,
                    "counts_as_usable_evidence": counts_as_usable,
                    "quality_flags": quality_flags,
                    "coverage_facets": coverage_facets,
                    "freshness": {
                        "status": evidence_pack_freshness_status(query, candidate),
                        "current_intent": current_web_intent(query)
                    },
                    "timestamp": candidate.timestamp.clone(),
                    "permissions": candidate.permissions.clone(),
                    "promotion": promotion,
                    "content_authority": "retrieved_public_web",
                    "visibility": "synthesis_context"
                })
            })
            .collect::<Vec<_>>(),
    )
}

fn evidence_claim_entities(query_metadata: &BatchQueryKeywordPack, row: &Value) -> Vec<String> {
    let haystack = clean_text(
        &format!(
            "{} {} {}",
            row.get("title").and_then(Value::as_str).unwrap_or(""),
            row.get("snippet").and_then(Value::as_str).unwrap_or(""),
            row.get("locator").and_then(Value::as_str).unwrap_or(""),
        ),
        2_400,
    )
    .to_ascii_lowercase();
    let mut entities = Vec::<String>::new();
    let mut seen = HashSet::<String>::new();
    for entity in &query_metadata.entities {
        let cleaned = clean_text(entity, 160);
        if cleaned.is_empty() {
            continue;
        }
        if haystack.contains(&cleaned.to_ascii_lowercase())
            && seen.insert(cleaned.to_ascii_lowercase())
        {
            entities.push(cleaned);
        }
    }
    entities
}

fn evidence_claim_limitations(row: &Value) -> Vec<String> {
    let mut limitations = Vec::<String>::new();
    let mut seen = HashSet::<String>::new();
    let materialization_quality = evidence_row_materialization_quality(row);
    match materialization_quality.as_str() {
        "partial_materialized" => {
            push_unique_clean_string(&mut limitations, &mut seen, "partial_materialization", 120);
        }
        "trusted_structured_feed" => {
            push_unique_clean_string(
                &mut limitations,
                &mut seen,
                "headline_or_structured_feed_level",
                120,
            );
        }
        "candidate_only" => {
            push_unique_clean_string(
                &mut limitations,
                &mut seen,
                "candidate_only_not_claim_backed",
                120,
            );
        }
        "failed_materialization" => {
            push_unique_clean_string(&mut limitations, &mut seen, "materialization_failed", 120);
        }
        _ => {}
    }
    if let Some(flags) = row
        .get("quality_flags")
        .or_else(|| row.get("flags"))
        .and_then(Value::as_array)
    {
        for flag in flags.iter().filter_map(Value::as_str).take(6) {
            if matches!(
                flag,
                "freshness_unproven"
                    | "low_trust_source"
                    | "low_score"
                    | "primary_source_needed"
                    | "weak_query_overlap_only"
                    | "thin_query_overlap"
            ) {
                push_unique_clean_string(&mut limitations, &mut seen, flag, 120);
            }
        }
    }
    limitations
}

fn evidence_claims_from_pack(
    query_metadata: &BatchQueryKeywordPack,
    evidence_pack: &Value,
    limit: usize,
) -> Value {
    let mut claims = Vec::<Value>::new();
    let max_claims = limit.max(1);
    if let Some(rows) = evidence_pack.as_array() {
        for row in rows {
            if !evidence_row_counts_as_usable_evidence(row) {
                continue;
            }
            let support_snippet = clean_text(
                row.get("snippet").and_then(Value::as_str).unwrap_or(""),
                420,
            );
            let entities = evidence_claim_entities(query_metadata, row);
            let limitations = evidence_claim_limitations(row);
            let coverage_facets = row
                .get("coverage_facets")
                .cloned()
                .unwrap_or_else(|| json!([]));
            let source_ref = json!({
                "title": clean_text(row.get("title").and_then(Value::as_str).unwrap_or(""), 180),
                "locator": clean_text(row.get("locator").and_then(Value::as_str).unwrap_or(""), 260),
                "source_domain": clean_text(row.get("source_domain").and_then(Value::as_str).unwrap_or(""), 120),
                "source_kind": clean_text(row.get("source_kind").and_then(Value::as_str).unwrap_or(""), 80),
                "materialization_quality": evidence_row_materialization_quality(row)
            });
            if let Some(hints) = row.get("claim_hints").and_then(Value::as_array) {
                for hint in hints.iter().filter_map(Value::as_str) {
                    let claim = clean_text(hint, 240);
                    let source_title =
                        clean_text(source_ref.get("title").and_then(Value::as_str).unwrap_or(""), 180);
                    if claim.is_empty()
                        || !claim_text_is_synthesis_safe(&claim)
                        || (!source_title.is_empty()
                            && claim.eq_ignore_ascii_case(&source_title))
                    {
                        continue;
                    }
                    claims.push(json!({
                        "claim": claim,
                        "title": source_ref.get("title").cloned().unwrap_or_else(|| json!("")),
                        "locator": source_ref.get("locator").cloned().unwrap_or_else(|| json!("")),
                        "source_title": source_ref.get("title").cloned().unwrap_or_else(|| json!("")),
                        "source_locator": source_ref.get("locator").cloned().unwrap_or_else(|| json!("")),
                        "source_domain": source_ref.get("source_domain").cloned().unwrap_or_else(|| json!("")),
                        "source_ref": source_ref.clone(),
                        "support_snippet": support_snippet.clone(),
                        "date": row.get("timestamp").cloned().unwrap_or(Value::Null),
                        "entities": entities.clone(),
                        "coverage_facets": coverage_facets.clone(),
                        "confidence": row.get("confidence").cloned().unwrap_or_else(|| json!("unknown")),
                        "limitations": limitations.clone()
                    }));
                    if claims.len() >= max_claims {
                        return Value::Array(claims);
                    }
                }
            }
        }
    }
    Value::Array(claims)
}

fn value_array_len(value: &Value) -> usize {
    value.as_array().map(Vec::len).unwrap_or(0)
}

fn policy_usize_at(
    policy: &Value,
    pointer: &str,
    default_value: usize,
    min: usize,
    max: usize,
) -> usize {
    policy
        .pointer(pointer)
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or(default_value)
        .clamp(min, max)
}

fn source_class_diversity_min(policy: &Value, query: &str) -> usize {
    let default_value = if current_web_intent(query) { 2 } else { 1 };
    policy_usize_at(
        policy,
        "/batch_query/source_diversification/min_source_classes",
        default_value,
        1,
        8,
    )
}

fn evidence_pack_quality_report(
    policy: &Value,
    evidence_pack: &Value,
    evidence_coverage: &Value,
) -> Value {
    let min_usable = policy_usize_at(
        policy,
        "/batch_query/evidence_pack_quality/min_usable_items",
        2,
        1,
        12,
    );
    let min_domains = policy_usize_at(
        policy,
        "/batch_query/evidence_pack_quality/min_source_domains",
        2,
        1,
        12,
    );
    let mut usable_count = 0usize;
    let mut low_confidence_count = 0usize;
    let mut materialized_count = 0usize;
    let mut trusted_structured_feed_count = 0usize;
    let mut candidate_only_count = 0usize;
    let mut failed_materialization_count = 0usize;
    let mut content_rich_count = 0usize;
    let mut claim_hint_count = 0usize;
    let mut evidence_claim_count = 0usize;
    let mut domains = HashSet::<String>::new();
    let mut source_classes = HashSet::<String>::new();
    if let Some(rows) = evidence_pack.as_array() {
        for row in rows {
            let materialization_quality = evidence_row_materialization_quality(row);
            let counts_as_usable = evidence_row_counts_as_usable_evidence(row);
            let confidence = clean_text(
                row.get("confidence").and_then(Value::as_str).unwrap_or(""),
                80,
            );
            if confidence == "low_confidence_raw" {
                low_confidence_count += 1;
            }
            if counts_as_usable {
                usable_count += 1;
            }
            match materialization_quality.as_str() {
                "full_materialized" | "partial_materialized" => materialized_count += 1,
                "trusted_structured_feed" => trusted_structured_feed_count += 1,
                "candidate_only" => candidate_only_count += 1,
                "failed_materialization" => failed_materialization_count += 1,
                _ => {}
            }
            if counts_as_usable
                && row
                    .get("snippet")
                    .and_then(Value::as_str)
                    .map(content_rich_text)
                    .unwrap_or(false)
            {
                content_rich_count += 1;
            }
            if counts_as_usable {
                let row_claim_count = row
                    .get("claim_hints")
                    .and_then(Value::as_array)
                    .map(|rows| {
                        rows.iter()
                            .filter(|row| {
                                row.as_str()
                                    .map(|raw| !raw.trim().is_empty())
                                    .unwrap_or(false)
                            })
                            .count()
                    })
                    .unwrap_or(0);
                claim_hint_count += row_claim_count;
                evidence_claim_count += row_claim_count;
            }
            let domain = clean_text(
                row.get("source_domain")
                    .or_else(|| row.get("source_scope"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                180,
            )
            .to_ascii_lowercase();
            if !domain.is_empty() && domain != "source" {
                domains.insert(domain);
            }
            let source_class = clean_text(
                row.get("source_class")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                120,
            )
            .to_ascii_lowercase();
            if !source_class.is_empty() {
                source_classes.insert(source_class);
            }
        }
    }
    let mut missing_facets = 0usize;
    let mut weak_facets = 0usize;
    if let Some(rows) = evidence_coverage.as_array() {
        for row in rows {
            match row.get("status").and_then(Value::as_str).unwrap_or("") {
                "missing" => missing_facets += 1,
                "weak" => weak_facets += 1,
                _ => {}
            }
        }
    }
    let item_count = value_array_len(evidence_pack);
    let status = if item_count == 0 {
        "absent"
    } else if usable_count == 0 && low_confidence_count > 0 && candidate_only_count == 0 {
        "low_confidence_only"
    } else if usable_count < min_usable
        || domains.len() < min_domains
        || missing_facets > 0
        || weak_facets > 0
        || content_rich_count == 0
        || claim_hint_count == 0
    {
        "thin"
    } else {
        "usable"
    };
    json!({
        "version": "evidence_pack_quality_v1",
        "status": status,
        "item_count": item_count,
        "usable_count": usable_count,
        "low_confidence_count": low_confidence_count,
        "materialized_item_count": materialized_count,
        "trusted_structured_feed_count": trusted_structured_feed_count,
        "candidate_only_count": candidate_only_count,
        "failed_materialization_count": failed_materialization_count,
        "content_rich_item_count": content_rich_count,
        "claim_hint_count": claim_hint_count,
        "evidence_claim_count": evidence_claim_count,
        "source_domain_count": domains.len(),
        "source_class_count": source_classes.len(),
        "missing_facet_count": missing_facets,
        "weak_facet_count": weak_facets,
        "thresholds": {
            "min_usable_items": min_usable,
            "min_source_domains": min_domains,
            "min_content_rich_items": 1,
            "min_claim_hint_items": 1
        },
        "synthesis_boundary": "quality metadata is calibration context, not citable evidence or final answer structure"
    })
}

fn source_class_coverage_from_evidence_pack(
    policy: &Value,
    query: &str,
    evidence_pack: &Value,
    evidence_coverage: &Value,
) -> Value {
    let mut class_counts = HashMap::<String, usize>::new();
    let mut domains = HashSet::<String>::new();
    let mut low_confidence_count = 0usize;
    if let Some(rows) = evidence_pack.as_array() {
        for row in rows {
            let source_class = clean_text(
                row.get("source_class")
                    .and_then(Value::as_str)
                    .unwrap_or("general_web"),
                120,
            )
            .to_ascii_lowercase();
            if !source_class.is_empty() {
                *class_counts.entry(source_class).or_insert(0) += 1;
            }
            let domain = clean_text(
                row.get("source_domain")
                    .or_else(|| row.get("source_scope"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                180,
            )
            .to_ascii_lowercase();
            if !domain.is_empty() && domain != "source" {
                domains.insert(domain);
            }
            if row.get("confidence").and_then(Value::as_str) == Some("low_confidence_raw") {
                low_confidence_count += 1;
            }
        }
    }
    let mut classes = class_counts
        .iter()
        .map(|(class, count)| json!({"source_class": class, "evidence_count": count}))
        .collect::<Vec<_>>();
    classes.sort_by(|a, b| {
        b.get("evidence_count")
            .and_then(Value::as_u64)
            .cmp(&a.get("evidence_count").and_then(Value::as_u64))
            .then_with(|| {
                a.get("source_class")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .cmp(b.get("source_class").and_then(Value::as_str).unwrap_or(""))
            })
    });
    let preferred = policy_string_list(
        policy,
        "/batch_query/source_diversification/preferred_source_classes",
    );
    let missing_preferred = preferred
        .iter()
        .filter(|class| !class_counts.contains_key(*class))
        .cloned()
        .collect::<Vec<_>>();
    let mut missing_facets = 0usize;
    if let Some(rows) = evidence_coverage.as_array() {
        missing_facets = rows
            .iter()
            .filter(|row| row.get("status").and_then(Value::as_str) == Some("missing"))
            .count();
    }
    let min_classes = source_class_diversity_min(policy, query);
    let item_count = value_array_len(evidence_pack);
    let status = if item_count == 0 {
        "absent"
    } else if class_counts.len() < min_classes {
        "limited"
    } else if missing_facets > 0 {
        "coverage_gaps"
    } else {
        "diverse"
    };
    json!({
        "version": "source_class_coverage_v1",
        "status": status,
        "source_class_count": class_counts.len(),
        "source_domain_count": domains.len(),
        "low_confidence_count": low_confidence_count,
        "classes": classes,
        "preferred_source_classes_missing": missing_preferred,
        "missing_facet_count": missing_facets,
        "thresholds": {
            "min_source_classes": min_classes
        },
        "non_goal": "do_not_require_any_specific_source_class_for_all_research"
    })
}

fn source_class_coverage_from_ranked_candidates(
    policy: &Value,
    query: &str,
    evidence_ranked: &[(Candidate, f64)],
    evidence_coverage: &Value,
) -> Value {
    let pack_like = Value::Array(
        evidence_ranked
            .iter()
            .map(|(candidate, _)| {
                json!({
                    "source_class": evidence_pack_source_class(policy, candidate),
                    "source_domain": candidate_domain_hint(candidate),
                    "confidence": if candidate_is_low_confidence_retained(candidate) {
                        "low_confidence_raw"
                    } else {
                        "usable"
                    }
                })
            })
            .collect::<Vec<_>>(),
    );
    source_class_coverage_from_evidence_pack(policy, query, &pack_like, evidence_coverage)
}

fn provider_attempts_from_retrieval_telemetry(retrieval_telemetry: &Value) -> Value {
    let attempts = retrieval_telemetry
        .as_array()
        .map(|rows| {
            rows.iter()
                .map(|row| {
                    let provider_count = row
                        .get("provider_count")
                        .and_then(Value::as_u64)
                        .unwrap_or(0);
                    let provider_raw_rows = row
                        .get("provider_raw_rows")
                        .and_then(Value::as_u64)
                        .unwrap_or(0);
                    let synthesis_rows = row
                        .get("synthesis_candidate_rows")
                        .and_then(Value::as_u64)
                        .unwrap_or(0);
                    let low_confidence_rows = row
                        .get("low_confidence_raw_rows")
                        .and_then(Value::as_u64)
                        .unwrap_or(0);
                    let failure_count = row
                        .get("failure_reasons")
                        .and_then(Value::as_array)
                        .map(Vec::len)
                        .unwrap_or(0);
                    let status = if synthesis_rows > 0 {
                        "usable"
                    } else if low_confidence_rows > 0 {
                        "low_confidence"
                    } else if provider_raw_rows > 0 {
                        "filtered"
                    } else if provider_count > 0 || failure_count > 0 {
                        "failed_or_empty"
                    } else {
                        "not_recorded"
                    };
                    json!({
                        "query": row.get("query").cloned().unwrap_or_else(|| json!("")),
                        "phase": row.get("phase").cloned().unwrap_or_else(|| json!("unknown")),
                        "status": status,
                        "provider_count": provider_count,
                        "provider_raw_rows": provider_raw_rows,
                        "candidate_rows": row.get("candidate_rows").cloned().unwrap_or_else(|| json!(0)),
                        "synthesis_candidate_rows": synthesis_rows,
                        "low_confidence_raw_rows": low_confidence_rows,
                        "failure_reasons": row.get("failure_reasons").cloned().unwrap_or_else(|| json!([]))
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Value::Array(attempts)
}

fn provider_attempts_from_provider_results(provider_results: &Value) -> Value {
    let attempts = provider_results
        .as_array()
        .map(|rows| {
            rows.iter()
                .map(|row| {
                    let raw_count = row
                        .get("provider_raw_count")
                        .and_then(Value::as_u64)
                        .unwrap_or(0);
                    let synthesis_count = row
                        .get("synthesis_candidate_count")
                        .and_then(Value::as_u64)
                        .unwrap_or(0);
                    let quality = row
                        .get("result_quality")
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    let ok = row.get("ok").and_then(Value::as_bool).unwrap_or(false);
                    let status = if quality == "usable" || synthesis_count > 0 {
                        "usable"
                    } else if quality == "low_signal" || quality == "low_confidence_raw" {
                        "low_confidence"
                    } else if raw_count > 0 && ok {
                        "filtered"
                    } else {
                        "failed_or_empty"
                    };
                    json!({
                        "query": row.get("query").cloned().unwrap_or_else(|| json!("")),
                        "phase": row.get("stage").cloned().unwrap_or_else(|| json!("provider_result")),
                        "provider": row.get("provider").cloned().unwrap_or_else(|| json!("unknown")),
                        "status": status,
                        "provider_count": 1,
                        "provider_raw_rows": raw_count,
                        "candidate_rows": raw_count,
                        "synthesis_candidate_rows": synthesis_count,
                        "low_confidence_raw_rows": if status == "low_confidence" { raw_count } else { 0 },
                        "failure_reasons": row.get("failure_reasons").cloned().unwrap_or_else(|| {
                            row.get("error")
                                .and_then(Value::as_str)
                                .map(|error| json!([error]))
                                .unwrap_or_else(|| json!([]))
                        })
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Value::Array(attempts)
}

fn provider_results_candidate_enrichment_count(provider_results: &Value) -> usize {
    provider_results
        .as_array()
        .map(|rows| {
            rows.iter()
                .filter(|row| {
                    row.get("content_preview")
                        .and_then(Value::as_str)
                        .map(|value| !value.trim().is_empty())
                        .unwrap_or(false)
                        || row
                            .get("links")
                            .and_then(Value::as_array)
                            .map(|links| !links.is_empty())
                            .unwrap_or(false)
                        || row
                            .get("failure_reasons")
                            .and_then(Value::as_array)
                            .map(|reasons| {
                                reasons.iter().any(|reason| {
                                    reason
                                        .as_str()
                                        .map(|value| {
                                            value.contains(":fetch:")
                                                || value.contains("fetch_candidate")
                                                || value.contains("page_extraction")
                                        })
                                        .unwrap_or(false)
                                })
                            })
                            .unwrap_or(false)
                })
                .count()
        })
        .unwrap_or(0)
}

fn provider_normalization_report(provider_attempts: &Value) -> Value {
    let mut status_counts = HashMap::<String, usize>::new();
    let mut phase_counts = HashMap::<String, usize>::new();
    let mut failure_classes = HashSet::<String>::new();
    let mut raw_rows = 0usize;
    let mut synthesis_rows = 0usize;
    let mut low_confidence_rows = 0usize;
    if let Some(rows) = provider_attempts.as_array() {
        for row in rows {
            let status = clean_text(
                row.get("status")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown"),
                80,
            );
            *status_counts.entry(status).or_insert(0) += 1;
            let phase = clean_text(
                row.get("phase")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown"),
                120,
            );
            *phase_counts.entry(phase).or_insert(0) += 1;
            raw_rows += row
                .get("provider_raw_rows")
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize;
            synthesis_rows += row
                .get("synthesis_candidate_rows")
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize;
            low_confidence_rows += row
                .get("low_confidence_raw_rows")
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize;
            if let Some(reasons) = row.get("failure_reasons").and_then(Value::as_array) {
                let reason_strings = reasons
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ToString::to_string)
                    .collect::<Vec<_>>();
                for class in issue_quality_flags(&reason_strings) {
                    failure_classes.insert(class);
                }
            }
        }
    }
    let attempt_count = value_array_len(provider_attempts);
    let mut status_rows = status_counts
        .iter()
        .map(|(status, count)| json!({"status": status, "count": count}))
        .collect::<Vec<_>>();
    status_rows.sort_by(|a, b| {
        a.get("status")
            .and_then(Value::as_str)
            .unwrap_or("")
            .cmp(b.get("status").and_then(Value::as_str).unwrap_or(""))
    });
    let mut phase_rows = phase_counts
        .iter()
        .map(|(phase, count)| json!({"phase": phase, "count": count}))
        .collect::<Vec<_>>();
    phase_rows.sort_by(|a, b| {
        a.get("phase")
            .and_then(Value::as_str)
            .unwrap_or("")
            .cmp(b.get("phase").and_then(Value::as_str).unwrap_or(""))
    });
    let mut failure_class_rows = failure_classes.into_iter().collect::<Vec<_>>();
    failure_class_rows.sort();
    let status = if attempt_count == 0 {
        "not_observed"
    } else if synthesis_rows > 0 {
        "normalized_with_usable_candidates"
    } else if raw_rows > 0 || low_confidence_rows > 0 {
        "normalized_but_filtered_or_low_confidence"
    } else {
        "normalized_failure_only"
    };
    json!({
        "version": "provider_normalization_v1",
        "status": status,
        "attempt_count": attempt_count,
        "provider_raw_rows": raw_rows,
        "synthesis_candidate_rows": synthesis_rows,
        "low_confidence_raw_rows": low_confidence_rows,
        "status_counts": status_rows,
        "phase_counts": phase_rows,
        "failure_classes": failure_class_rows,
        "normalized_candidate_model": [
            "provider",
            "phase",
            "status",
            "provider_raw_rows",
            "synthesis_candidate_rows",
            "low_confidence_raw_rows",
            "failure_reasons"
        ],
        "chat_visibility": "telemetry_only_until_synthesized",
        "raw_payload_chat_visible": false
    })
}

fn string_array_values(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn count_artifact_like_refs(value: &Value) -> usize {
    match value {
        Value::Array(rows) => rows.iter().map(count_artifact_like_refs).sum(),
        Value::Object(map) => map
            .iter()
            .map(|(key, value)| {
                let lowered = key.to_ascii_lowercase();
                let current = if (lowered.contains("artifact") || lowered.contains("raw_payload"))
                    && (lowered.ends_with("ref") || lowered.ends_with("refs"))
                {
                    1
                } else {
                    0
                };
                current + count_artifact_like_refs(value)
            })
            .sum(),
        _ => 0,
    }
}

fn artifact_quarantine_report(
    provider_results: &Value,
    evidence_pack: &Value,
    tool_result_quality: &Value,
) -> Value {
    let provider_result_count = value_array_len(provider_results);
    let evidence_pack_count = value_array_len(evidence_pack);
    let artifact_ref_count =
        count_artifact_like_refs(provider_results) + count_artifact_like_refs(evidence_pack);
    let promotion_rows = evidence_pack
        .as_array()
        .map(|rows| {
            rows.iter()
                .map(|row| {
                    json!({
                        "locator": row.get("locator").cloned().unwrap_or_else(|| json!("")),
                        "promotion_decision": row.pointer("/promotion/decision").cloned().unwrap_or_else(|| json!("unknown")),
                        "safety_status": row.pointer("/promotion/safety/status").cloned().unwrap_or_else(|| json!("unknown")),
                        "raw_payload_chat_visible": row.pointer("/promotion/safety/raw_payload_chat_visible").cloned().unwrap_or_else(|| json!(false))
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let status = if artifact_ref_count > 0 {
        "raw_artifacts_quarantined"
    } else if provider_result_count > 0 || evidence_pack_count > 0 {
        "no_raw_artifacts_projected"
    } else {
        "no_artifacts_observed"
    };
    json!({
        "version": "artifact_quarantine_v1",
        "status": status,
        "artifact_ref_count": artifact_ref_count,
        "provider_result_count": provider_result_count,
        "evidence_pack_count": evidence_pack_count,
        "raw_payload_chat_visible": false,
        "lanes": [
            {
                "lane": "provider_results",
                "visibility": "telemetry_or_receipt_only",
                "raw_payload_chat_visible": false
            },
            {
                "lane": "evidence_pack",
                "visibility": "synthesis_context_after_promotion",
                "raw_payload_chat_visible": false
            },
            {
                "lane": "tool_result_quality",
                "visibility": tool_result_quality
                    .get("chat_visibility")
                    .cloned()
                    .unwrap_or_else(|| json!("telemetry_or_synthesis_context")),
                "raw_payload_chat_visible": false
            }
        ],
        "evidence_promotions": promotion_rows,
        "non_goals": [
            "do_not_expose_raw_html_or_provider_payloads_to_chat",
            "do_not_make_artifact_presence_source_truth",
            "do_not_depend_on_specific_answer_format"
        ]
    })
}

fn retry_stop_conditions_report(
    status: &str,
    submitted_query_count: usize,
    executed_query_count: usize,
    provider_attempt_count: usize,
    provider_success_count: usize,
    second_pass_recovery: &Value,
    tool_result_quality: &Value,
    source_class_coverage: &Value,
    evidence_pack_quality: &Value,
    provider_normalization: &Value,
) -> Value {
    let retrieval_decision = tool_result_quality
        .pointer("/retrieval_decision/decision")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let action_status = tool_result_quality
        .pointer("/retrieval_decision/action_status")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let retry_reason = tool_result_quality
        .pointer("/retry/reason")
        .and_then(Value::as_str)
        .unwrap_or("none");
    let retry_recommended = tool_result_quality
        .pointer("/retry/recommended")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let second_pass_enabled = second_pass_recovery
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let second_pass_used = second_pass_recovery
        .get("used")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let second_pass_query_count = second_pass_recovery
        .get("queries")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    let failure_classes = string_array_values(provider_normalization.get("failure_classes"));
    let repeated_failure_signature =
        provider_attempt_count > 1 && provider_success_count == 0 && failure_classes.len() == 1;
    let retry_budget_observed_exhausted =
        retry_recommended && second_pass_enabled && second_pass_used && second_pass_query_count > 0;
    let capability_required = action_status.starts_with("requires_");
    let report_status = match retrieval_decision {
        "synthesize_from_evidence" => "stop_ready_for_synthesis",
        "structured_low_evidence" => "stop_structured_low_evidence",
        "alternate_provider" => "continue_with_alternate_provider_if_admitted",
        "browser_materialize_candidate" => "continue_with_browser_materialization_if_admitted",
        "direct_fetch_candidate" => "continue_with_direct_fetch_if_budget_remains",
        "agent_refine_query_pack" => {
            if retry_budget_observed_exhausted {
                "stop_or_escalate_after_retry_budget"
            } else {
                "continue_with_agent_query_refinement_if_budget_remains"
            }
        }
        _ => "observe_without_control_decision",
    };
    json!({
        "version": "retry_stop_conditions_v1",
        "status": report_status,
        "retrieval_decision": retrieval_decision,
        "action_status": action_status,
        "retry_reason": retry_reason,
        "stop_conditions": {
            "evidence_sufficient": retrieval_decision == "synthesize_from_evidence",
            "structured_low_evidence_terminal": retrieval_decision == "structured_low_evidence",
            "retry_budget_observed_exhausted": retry_budget_observed_exhausted,
            "repeated_failure_signature": repeated_failure_signature,
            "capability_required": capability_required,
            "provider_success_available": provider_success_count > 0
        },
        "budgets_observed": {
            "submitted_query_count": submitted_query_count,
            "executed_query_count": executed_query_count,
            "provider_attempt_count": provider_attempt_count,
            "second_pass_enabled": second_pass_enabled,
            "second_pass_used": second_pass_used,
            "second_pass_query_count": second_pass_query_count
        },
        "quality_state": {
            "status": status,
            "source_class_coverage": source_class_coverage
                .get("status")
                .cloned()
                .unwrap_or_else(|| json!("unknown")),
            "evidence_pack_quality": evidence_pack_quality
                .get("status")
                .cloned()
                .unwrap_or_else(|| json!("unknown")),
            "provider_normalization": provider_normalization
                .get("status")
                .cloned()
                .unwrap_or_else(|| json!("unknown")),
            "failure_classes": failure_classes
        },
        "authority": {
            "tool": "reports_stop_conditions_and_retrieval_state",
            "agent": "chooses_next_query_or_final_synthesis",
            "gateway": "admits_provider_or_browser_capabilities"
        },
        "non_goals": [
            "do_not_loop_without_budget",
            "do_not_generate_hidden_queries",
            "do_not_force_final_answer_format"
        ]
    })
}

fn page_readiness_and_extraction_report(
    provider_results: &Value,
    evidence_pack: &Value,
    tool_result_quality: &Value,
) -> Value {
    let provider_result_count = value_array_len(provider_results);
    let evidence_pack_count = value_array_len(evidence_pack);
    let mut content_preview_count = 0usize;
    let mut substantive_preview_count = 0usize;
    let mut blocker_shell_count = 0usize;
    let mut link_count = 0usize;
    let mut extraction_signal_count = 0usize;
    if let Some(rows) = provider_results.as_array() {
        for row in rows {
            if let Some(preview) = row.get("content_preview").and_then(Value::as_str) {
                let cleaned = clean_text(preview, 1_800);
                if !cleaned.is_empty() {
                    content_preview_count += 1;
                    extraction_signal_count += 1;
                    if content_rich_text(&cleaned) {
                        substantive_preview_count += 1;
                    }
                    let lowered = cleaned.to_ascii_lowercase();
                    if contains_web_junk_marker(&cleaned)
                        || text_has_any_marker(
                            &lowered,
                            &[
                                "please enable javascript",
                                "verify you are human",
                                "checking your browser",
                                "access denied",
                                "captcha",
                                "cloudflare",
                            ],
                        )
                    {
                        blocker_shell_count += 1;
                    }
                }
            }
            if let Some(links) = row.get("links").and_then(Value::as_array) {
                link_count += links.len();
                if !links.is_empty() {
                    extraction_signal_count += 1;
                }
            }
            if row
                .get("failure_reasons")
                .and_then(Value::as_array)
                .map(|reasons| {
                    reasons.iter().any(|reason| {
                        reason
                            .as_str()
                            .map(|value| {
                                value.contains("page_extraction")
                                    || value.contains("content_materialization")
                                    || value.contains("fetch_candidate")
                            })
                            .unwrap_or(false)
                    })
                })
                .unwrap_or(false)
            {
                extraction_signal_count += 1;
            }
        }
    }
    let blocker_class = tool_result_quality
        .pointer("/blocker_taxonomy/primary_class")
        .and_then(Value::as_str)
        .unwrap_or("none");
    let retrieval_decision = tool_result_quality
        .pointer("/retrieval_decision/decision")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let status = if evidence_pack_count > 0 {
        "evidence_packaged"
    } else if blocker_shell_count > 0 {
        "blocked_or_shell_detected"
    } else if substantive_preview_count > 0 {
        "extraction_ready_for_promotion_review"
    } else if extraction_signal_count > 0 {
        "extraction_observed_but_thin"
    } else if matches!(
        retrieval_decision,
        "browser_materialize_candidate" | "direct_fetch_candidate"
    ) {
        "materialization_or_fetch_needed"
    } else if provider_result_count > 0 {
        "retrieval_observed_without_extraction_signal"
    } else {
        "not_observed"
    };
    json!({
        "version": "page_readiness_extraction_v1",
        "status": status,
        "provider_result_count": provider_result_count,
        "content_preview_count": content_preview_count,
        "substantive_preview_count": substantive_preview_count,
        "blocker_shell_count": blocker_shell_count,
        "link_count": link_count,
        "extraction_signal_count": extraction_signal_count,
        "evidence_pack_count": evidence_pack_count,
        "blocker_class": blocker_class,
        "retrieval_decision": retrieval_decision,
        "readiness_contract": {
            "ready_requires_substantive_main_text": true,
            "blocker_shell_is_not_evidence": true,
            "redirect_final_url_safety_required": true,
            "bounded_extraction_required": true
        },
        "extraction_contract": {
            "metadata_first": true,
            "main_text_or_markdown_required_for_promotion": true,
            "raw_html_artifact_only": true,
            "console_or_network_logs_artifact_only": true
        },
        "non_goals": [
            "do_not_wait_indefinitely_for_page_readiness",
            "do_not_promote_shell_text_as_evidence",
            "do_not_expose_raw_render_payloads_to_chat"
        ]
    })
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
    let mut provider_attempts = provider_attempts_from_retrieval_telemetry(retrieval_telemetry);
    if value_array_len(&provider_attempts) == 0 {
        provider_attempts = provider_attempts_from_provider_results(provider_results);
    }
    let provider_attempt_count = value_array_len(&provider_attempts);
    let provider_normalization = provider_normalization_report(&provider_attempts);
    let provider_success_count = provider_attempts
        .as_array()
        .map(|rows| {
            rows.iter()
                .filter(|row| row.get("status").and_then(Value::as_str) == Some("usable"))
                .count()
        })
        .unwrap_or(0);
    let evidence_status = evidence_pack_quality
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("absent");
    let provider_status = if provider_attempt_count == 0 {
        "not_recorded"
    } else if provider_success_count > 0 {
        "usable"
    } else {
        "degraded"
    };
    let submitted_query_count = value_array_len(&submitted_query_plan);
    let executed_query_count = value_array_len(&executed_query_plan);
    let provider_result_count = value_array_len(provider_results);
    let evidence_pack_count = value_array_len(evidence_pack);
    let coverage_facet_count = value_array_len(evidence_coverage);
    let enrichment_count = provider_results_candidate_enrichment_count(provider_results);
    let retry_stop_conditions = retry_stop_conditions_report(
        status,
        submitted_query_count,
        executed_query_count,
        provider_attempt_count,
        provider_success_count,
        &second_pass_recovery,
        tool_result_quality,
        source_class_coverage,
        evidence_pack_quality,
        &provider_normalization,
    );
    let artifact_quarantine =
        artifact_quarantine_report(provider_results, evidence_pack, tool_result_quality);
    let page_readiness_extraction =
        page_readiness_and_extraction_report(provider_results, evidence_pack, tool_result_quality);
    let enrichment_status = if enrichment_count > 0 {
        "attempted"
    } else if evidence_pack_count > 0 {
        "not_needed_or_not_recorded"
    } else {
        "not_recorded"
    };
    json!({
        "version": "web_research_broker_report_v1",
        "primitive": "web_research",
        "authority": "tool_cd_policy_runtime_observation",
        "chat_visibility": "hidden_until_synthesized",
        "status": status,
        "query_planning": {
            "submitted_query_plan": submitted_query_plan,
            "executed_query_plan": executed_query_plan,
            "query_plan_source": query_plan_source,
            "hidden_query_expansion": false
        },
        "lanes": [
            {
                "lane": "query_planning",
                "status": "complete",
                "submitted_query_count": submitted_query_count,
                "executed_query_count": executed_query_count
            },
            {
                "lane": "provider_retrieval",
                "status": provider_status,
                "attempt_count": provider_attempt_count,
                "usable_attempt_count": provider_success_count,
                "provider_result_count": provider_result_count
            },
            {
                "lane": "candidate_enrichment",
                "status": enrichment_status,
                "enrichment_signal_count": enrichment_count
            },
            {
                "lane": "evidence_packaging",
                "status": evidence_status,
                "evidence_pack_count": evidence_pack_count
            },
            {
                "lane": "coverage_review",
                "status": source_class_coverage
                    .get("status")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown"),
                "coverage_facet_count": coverage_facet_count
            }
        ],
        "provider_attempts": provider_attempts,
        "provider_normalization": provider_normalization,
        "retry_stop_conditions": retry_stop_conditions,
        "artifact_quarantine": artifact_quarantine,
        "page_readiness_extraction": page_readiness_extraction,
        "second_pass_recovery": second_pass_recovery,
        "source_class_coverage": source_class_coverage,
        "evidence_pack_quality": evidence_pack_quality,
        "tool_result_quality_flags": tool_result_quality
            .get("flags")
            .cloned()
            .unwrap_or_else(|| json!([])),
        "synthesis_boundary": "broker diagnostics guide retrieval calibration but are not final-answer text or citable source evidence"
    })
}

fn page_extraction_link_has_article_like_path(link: &str) -> bool {
    let Some((_, _, path, _)) = parse_page_extraction_http_url(link) else {
        return false;
    };
    let path_lower = clean_text(path, 1_800).to_ascii_lowercase();
    let segments = path_lower
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    if segments.is_empty() {
        return false;
    }
    let has_year_segment = segments.iter().any(|segment| {
        segment.len() == 4
            && segment.starts_with("20")
            && segment.chars().all(|ch| ch.is_ascii_digit())
    });
    let has_date_slug = segments.iter().any(|segment| {
        segment.len() >= 10
            && segment
                .chars()
                .take(10)
                .enumerate()
                .all(|(index, ch)| matches!(index, 4 | 7) && ch == '-' || ch.is_ascii_digit())
            && segment.starts_with("20")
    });
    let has_article_marker = path_lower.contains("/article")
        || path_lower.contains("/articles/")
        || path_lower.contains("/story")
        || path_lower.contains("/stories/")
        || path_lower.contains("/news/");
    let has_long_slug = segments.iter().any(|segment| {
        segment.len() >= 16
            && segment.contains('-')
            && segment.chars().any(|ch| ch.is_ascii_alphabetic())
    });
    (has_year_segment && segments.len() >= 2)
        || has_date_slug
        || (has_article_marker && (segments.len() >= 3 || has_long_slug))
        || (segments.len() >= 3 && has_long_slug)
}

fn candidate_looks_like_relevant_discovery_hub(query: &str, candidate: &Candidate) -> bool {
    let combined = clean_text(
        &format!(
            "{} {} {}",
            candidate.title, candidate.snippet, candidate.locator
        ),
        2_400,
    );
    if combined.is_empty()
        || contains_web_junk_marker(&combined)
        || looks_like_low_signal_search_summary(&combined)
        || looks_like_off_intent_noise_candidate(query, candidate)
    {
        return false;
    }
    let separator_hits = combined.matches(" · ").count() + combined.matches(" | ").count();
    let section_context_signal = separator_hits >= 2 && combined.split_whitespace().count() >= 12;
    if !looks_like_link_directory_or_aggregator_shell(&combined) && !section_context_signal {
        return false;
    }
    let source_signal = source_trust_adjustment(candidate) >= 0.1
        || query_overlap_terms(query, candidate) >= 1;
    if !source_signal {
        return false;
    }
    if query_has_distinctive_relevance_terms(query) {
        query_overlap_terms(query, candidate) >= 1 || source_trust_adjustment(candidate) >= 0.1
    } else {
        current_web_intent(query)
    }
}

fn fallback_link_score(query: &str, link: &str) -> f64 {
    let cleaned = clean_text(link, 2_200);
    let lowered = cleaned.to_ascii_lowercase();
    let domain = extract_domains_from_text(&cleaned, 1)
        .into_iter()
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if domain.is_empty() || is_search_engine_domain(&domain) || contains_web_junk_marker(&lowered) {
        return -1.0;
    }
    let mut score: f64 = 0.0;
    let candidate = Candidate {
        source_kind: "web".to_string(),
        title: format!("Web result from {domain}"),
        locator: cleaned.clone(),
        snippet: cleaned.clone(),
        excerpt_hash: sha256_hex(&cleaned),
        timestamp: None,
        permissions: Some("public_web".to_string()),
        status_code: 200,
    };
    score += source_trust_adjustment(&candidate);
    if current_web_intent(query) {
        score += recency_adjustment(query, &candidate);
    }
    let article_like_link = page_extraction_link_has_article_like_path(&cleaned);
    if article_like_link {
        score += 0.16;
    }
    if current_web_intent(query) && !query_has_distinctive_relevance_terms(query) {
        if article_like_link {
            score += 0.28;
        } else {
            score -= 0.28;
        }
    }
    if link_contains_collapsed_query_phrase(query, &cleaned) {
        score += 0.35;
    }
    let query_tokens = tokenize_relevance(query, 40);
    let link_tokens = tokenize_relevance(&cleaned, 80);
    if !query_tokens.is_empty() {
        let overlap = query_tokens.intersection(&link_tokens).count() as f64;
        score += 0.4 * (overlap / query_tokens.len() as f64);
        if has_only_weak_query_overlap(query, &candidate) {
            score -= 0.45;
        }
        if overlap == 0.0 && source_trust_adjustment(&candidate) <= 0.0 {
            score -= 0.25;
        }
    }
    if lowered.contains("/docs") || lowered.contains("/blog") || lowered.contains("/news") {
        score += 0.04;
    }
    if lowered.contains("login") || lowered.contains("signup") || lowered.contains("account") {
        score -= 0.2;
    }
    score
}

fn link_context_window(text: &str, link: &str, radius: usize) -> String {
    let cleaned_text = clean_text(text, 8_000);
    let cleaned_link = clean_text(link, 2_200);
    if cleaned_text.is_empty() || cleaned_link.is_empty() {
        return String::new();
    }
    let lowered_text = cleaned_text.to_ascii_lowercase();
    let lowered_link = cleaned_link.to_ascii_lowercase();
    let Some(pos) = lowered_text.find(&lowered_link) else {
        return trim_words(&cleaned_text, 96);
    };
    let mut start = pos.saturating_sub(radius);
    while start > 0 && !cleaned_text.is_char_boundary(start) {
        start -= 1;
    }
    let mut end = (pos + cleaned_link.len() + radius).min(cleaned_text.len());
    while end < cleaned_text.len() && !cleaned_text.is_char_boundary(end) {
        end += 1;
    }
    trim_words(&cleaned_text[start..end], 120)
}

fn payload_context_for_link(payload: &Value, link: &str) -> String {
    let summary = payload.get("summary").and_then(Value::as_str).unwrap_or("");
    let content = payload.get("content").and_then(Value::as_str).unwrap_or("");
    link_context_window(&format!("{summary} {content}"), link, 720)
}

fn page_extraction_link_candidate_with_context(link: &str, context: &str) -> Candidate {
    let mut candidate = page_extraction_link_candidate(link);
    let cleaned_context = clean_text(context, 1_800);
    if !cleaned_context.is_empty() {
        candidate.snippet =
            clean_text(&format!("{} {}", cleaned_context, candidate.locator), 2_200);
    }
    candidate
}

fn fallback_link_score_with_context(query: &str, link: &str, context: &str) -> f64 {
    let base_score = fallback_link_score(query, link);
    let cleaned_context = clean_text(context, 1_800);
    if cleaned_context.is_empty() {
        return base_score;
    }
    let candidate = page_extraction_link_candidate_with_context(link, &cleaned_context);
    let context_score = rerank_score(query, &candidate);
    let query_tokens = tokenize_relevance(query, 40);
    let context_tokens = tokenize_relevance(&cleaned_context, 120);
    let overlap_bonus = if query_tokens.is_empty() {
        0.0
    } else {
        0.34 * (query_tokens.intersection(&context_tokens).count() as f64
            / query_tokens.len() as f64)
    };
    base_score
        .max(context_score + overlap_bonus)
        .clamp(-1.0, 1.0)
}

fn ranked_payload_links_for_fallback_with_context_and_min_score(
    query: &str,
    payload: &Value,
    max_links: usize,
    min_score: f64,
) -> Vec<(String, String)> {
    let mut ranked = non_search_engine_links(payload, max_links.saturating_mul(4).max(max_links))
        .into_iter()
        .map(|link| {
            let context = payload_context_for_link(payload, &link);
            let score = fallback_link_score_with_context(query, &link, &context);
            (link, context, score)
        })
        .filter(|(_, _, score)| *score > -1.0 && *score >= min_score)
        .collect::<Vec<_>>();
    ranked.sort_by(|a, b| {
        b.2.partial_cmp(&a.2)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });
    ranked
        .into_iter()
        .take(max_links.max(1))
        .map(|(link, context, _)| (link, context))
        .collect::<Vec<_>>()
}

fn ranked_payload_links_for_fallback_with_min_score(
    query: &str,
    payload: &Value,
    max_links: usize,
    min_score: f64,
) -> Vec<String> {
    ranked_payload_links_for_fallback_with_context_and_min_score(
        query, payload, max_links, min_score,
    )
    .into_iter()
    .map(|(link, _)| link)
    .collect::<Vec<_>>()
}

fn ranked_payload_links_for_fallback(
    query: &str,
    payload: &Value,
    max_links: usize,
) -> Vec<String> {
    ranked_payload_links_for_fallback_with_min_score(query, payload, max_links, -1.0)
}

fn issue_quality_flags(partial_failures: &[String]) -> Vec<String> {
    let mut flags = Vec::<String>::new();
    for failure in partial_failures {
        let lowered = clean_text(failure, 320).to_ascii_lowercase();
        if lowered.contains("anti_bot_challenge")
            || lowered.contains("captcha")
            || lowered.contains("cloudflare")
            || lowered.contains("verify you are human")
        {
            flags.push("anti_bot_filtered".to_string());
        }
        if lowered.contains("needs_js") || lowered.contains("javascript required") {
            flags.push("needs_js".to_string());
        }
        if lowered.contains("rate_limited")
            || lowered.contains("rate limit")
            || lowered.contains("http_429")
            || lowered.contains("429")
        {
            flags.push("rate_limited".to_string());
        }
        if lowered.contains("access_denied")
            || lowered.contains("access denied")
            || lowered.contains("403")
            || lowered.contains("login required")
            || lowered.contains("subscription")
            || lowered.contains("region block")
        {
            flags.push("access_denied".to_string());
        }
        if lowered.contains("junk_page") {
            flags.push("junk_filtered".to_string());
        }
        if lowered.contains("candidate_low_relevance")
            || lowered.contains("fetch_candidate_low_relevance")
        {
            flags.push("low_relevance_filtered".to_string());
        }
        if lowered.contains("query_timeout") {
            flags.push("provider_timeout".to_string());
        }
        if lowered.contains("tool_surface_degraded")
            || lowered.contains("provider readiness mismatch")
        {
            flags.push("provider_degraded".to_string());
        }
        if lowered.contains("provider_circuit_open")
            || lowered.contains("serper_api_key_missing")
            || lowered.contains("_api_key_missing")
            || lowered.contains("api key missing")
            || lowered.contains("credential_missing")
            || lowered.contains("configured_provider_credential_unresolved")
            || lowered.contains("credential_unresolved")
            || lowered.contains("search_providers_exhausted")
        {
            flags.push("provider_starved".to_string());
        }
        if lowered.contains("no_usable_summary") || lowered.contains("fixture_missing") {
            flags.push("low_signal".to_string());
        }
        if lowered.contains("query_result_mismatch") {
            flags.push("query_result_mismatch".to_string());
        }
        if lowered.contains("comparison_entity_coverage_gap")
            || lowered.contains("source coverage to compare")
        {
            flags.push("comparison_evidence_insufficient".to_string());
        }
    }
    flags.sort();
    flags.dedup();
    flags
}

fn normalized_materialization_failure_reason(reason: &str) -> Option<&'static str> {
    let lowered = clean_text(reason, 320).to_ascii_lowercase();
    if lowered.is_empty() {
        return None;
    }
    if lowered.contains("page_extraction_global_budget_exhausted") {
        return Some("fetch_budget_exhausted");
    }
    if lowered.contains("page_extraction_candidate_prefetch_rejected:junk_link")
        || lowered.contains("junk_page")
    {
        return Some("prefetch_rejected_junk");
    }
    if lowered.contains("page_extraction_candidate_prefetch_rejected:off_intent_link")
        || lowered.contains("query_result_mismatch")
    {
        return Some("prefetch_rejected_off_intent");
    }
    if lowered.contains("page_extraction_candidate_prefetch_rejected:weak_overlap_link")
        || lowered.contains("page_extraction_candidate_prefetch_rejected:no_distinctive_overlap_link")
        || lowered.contains("fetch_candidate_low_relevance")
        || lowered.contains("browser_materialization_candidate_low_relevance")
    {
        return Some("prefetch_or_promotion_low_relevance");
    }
    if lowered.contains("query_timeout_ms_") || lowered.contains("query_timeout") {
        return Some("fetch_timeout");
    }
    if lowered.contains("rate_limited") || lowered.contains("http_429") || lowered.contains(":429")
    {
        return Some("fetch_rate_limited");
    }
    if lowered.contains("anti_bot_challenge")
        || lowered.contains("captcha")
        || lowered.contains("cloudflare")
        || lowered.contains("verify you are human")
    {
        return Some("fetch_blocked_or_waf");
    }
    if lowered.contains("access_denied")
        || lowered.contains("login required")
        || lowered.contains("subscription")
        || lowered.contains("region block")
        || lowered.contains(":403")
        || lowered.ends_with("403")
    {
        return Some("fetch_denied_or_auth");
    }
    if lowered.contains("browser_materialization") {
        return Some("browser_materialization_failed");
    }
    if lowered.contains("provider readiness mismatch")
        || lowered.contains("tool_surface_degraded")
        || lowered.contains("provider_degraded")
    {
        return Some("provider_surface_degraded");
    }
    if lowered.contains("content_too_thin")
        || lowered.contains("no_usable_summary")
        || lowered.contains("low_signal")
        || lowered.contains("page_extraction")
    {
        return Some("content_too_thin");
    }
    if lowered.contains("fetch:") || lowered.contains("fetch_candidate:") {
        return Some("fetch_or_parse_failed");
    }
    None
}

fn materialization_failure_report(
    diagnostic_failures: &[String],
    evidence_count: usize,
    materialized_candidate_count: usize,
    candidate_only_count: usize,
    failed_materialization_count: usize,
) -> Value {
    let mut reason_counts = HashMap::<String, u64>::new();
    let mut sample_reasons = HashMap::<String, String>::new();
    for reason in diagnostic_failures {
        let Some(normalized) = normalized_materialization_failure_reason(reason) else {
            continue;
        };
        *reason_counts.entry(normalized.to_string()).or_insert(0) += 1;
        sample_reasons
            .entry(normalized.to_string())
            .or_insert_with(|| clean_text(reason, 220));
    }
    if candidate_only_count > 0 && materialized_candidate_count == 0 {
        *reason_counts
            .entry("candidate_only_unfetched".to_string())
            .or_insert(0) += candidate_only_count as u64;
        sample_reasons
            .entry("candidate_only_unfetched".to_string())
            .or_insert_with(|| {
                "candidate rows stayed at title/snippet level and never materialized into source text"
                    .to_string()
            });
    }
    if failed_materialization_count > 0 {
        *reason_counts
            .entry("failed_materialization_candidate".to_string())
            .or_insert(0) += failed_materialization_count as u64;
        sample_reasons
            .entry("failed_materialization_candidate".to_string())
            .or_insert_with(|| {
                "candidate fetch or promotion attempted but did not yield promotable source content"
                    .to_string()
            });
    }
    let mut reason_rows = reason_counts
        .iter()
        .map(|(reason, count)| {
            json!({
                "reason": reason,
                "count": count,
                "sample": sample_reasons.get(reason).cloned().unwrap_or_default()
            })
        })
        .collect::<Vec<_>>();
    reason_rows.sort_by(|a, b| {
        b.get("count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            .cmp(&a.get("count").and_then(Value::as_u64).unwrap_or(0))
            .then_with(|| {
                a.get("reason")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .cmp(b.get("reason").and_then(Value::as_str).unwrap_or(""))
            })
    });
    let total_issue_count = reason_rows
        .iter()
        .map(|row| row.get("count").and_then(Value::as_u64).unwrap_or(0))
        .sum::<u64>();
    let status = if materialized_candidate_count > 0 {
        "materialized_candidates_present"
    } else if !reason_rows.is_empty() {
        "materialization_gap_diagnosed"
    } else if evidence_count == 0 {
        "no_packaged_evidence"
    } else {
        "not_recorded"
    };
    let top_reason = reason_rows.first().cloned().unwrap_or(Value::Null);
    json!({
        "version": "materialization_failure_report_v1",
        "status": status,
        "total_issue_count": total_issue_count,
        "distinct_reason_count": reason_rows.len(),
        "top_reason": top_reason,
        "reason_rows": reason_rows,
        "candidate_state_counts": {
            "materialized_candidate_count": materialized_candidate_count,
            "candidate_only_count": candidate_only_count,
            "failed_materialization_count": failed_materialization_count
        },
        "note": "Normalizes why candidate rows failed to become materialized source evidence so provider-surface failures can be debugged upstream."
    })
}

fn text_has_any_marker(text: &str, markers: &[&str]) -> bool {
    markers.iter().any(|marker| text.contains(*marker))
}

fn blocker_taxonomy_row(
    class: &str,
    present: bool,
    retryable: bool,
    recommended_next_capability: &str,
    evidence_impact: &str,
) -> Value {
    json!({
        "class": class,
        "present": present,
        "retryable": retryable,
        "recommended_next_capability": recommended_next_capability,
        "evidence_impact": evidence_impact
    })
}

fn browser_materialization_blocker_taxonomy(
    flags: &[String],
    partial_failures: &[String],
) -> Value {
    let combined = partial_failures
        .iter()
        .map(|failure| clean_text(failure, 320).to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join(" ");
    let has_flag = |needle: &str| flags.iter().any(|flag| flag == needle);
    let anti_bot = has_flag("anti_bot_filtered")
        || text_has_any_marker(
            &combined,
            &[
                "captcha",
                "cloudflare",
                "verify you are human",
                "checking your browser",
                "bot wall",
                "cf-challenge",
            ],
        );
    let needs_js = has_flag("needs_js")
        || text_has_any_marker(
            &combined,
            &[
                "needs_js",
                "javascript required",
                "please enable javascript",
            ],
        );
    let rate_limited = has_flag("rate_limited")
        || text_has_any_marker(
            &combined,
            &["rate limit", "rate_limited", "http_429", "429"],
        );
    let access_denied = has_flag("access_denied")
        || text_has_any_marker(
            &combined,
            &[
                "access denied",
                "403",
                "forbidden",
                "login required",
                "subscribe to continue",
                "region block",
            ],
        );
    let provider_degraded = has_flag("provider_degraded") || has_flag("provider_timeout");
    let provider_starved = has_flag("provider_starved");
    let content_materialization_missing =
        has_flag("content_rich_evidence_missing") || has_flag("claim_hints_missing");
    let off_intent_noise = has_flag("low_relevance_filtered")
        || has_flag("query_result_mismatch")
        || has_flag("junk_filtered");
    let low_signal = has_flag("low_signal") || has_flag("insufficient_evidence");
    let classes = vec![
        blocker_taxonomy_row(
            "anti_bot_challenge",
            anti_bot,
            true,
            "browser_materialize_page_when_policy_allows",
            "raw blocker page is not evidence",
        ),
        blocker_taxonomy_row(
            "needs_js",
            needs_js,
            true,
            "browser_materialize_page_when_policy_allows",
            "static result may be shell-only",
        ),
        blocker_taxonomy_row(
            "rate_limited",
            rate_limited,
            true,
            "retry_or_alternate_provider",
            "provider returned throttling signal",
        ),
        blocker_taxonomy_row(
            "access_denied",
            access_denied,
            false,
            "alternate_source_or_permission_boundary",
            "denied page is not evidence",
        ),
        blocker_taxonomy_row(
            "provider_starved",
            provider_starved,
            true,
            "configure_or_admit_stronger_search_provider",
            "candidate supply is unavailable or limited to degraded fallback providers",
        ),
        blocker_taxonomy_row(
            "provider_degraded",
            provider_degraded,
            true,
            "alternate_provider_or_runtime_repair",
            "provider health limits evidence confidence",
        ),
        blocker_taxonomy_row(
            "content_materialization_missing",
            content_materialization_missing,
            true,
            "browser_materialize_page_when_policy_allows",
            "candidate exists but extracted content is too thin",
        ),
        blocker_taxonomy_row(
            "off_intent_noise",
            off_intent_noise,
            true,
            "refine_query_pack_or_filter_terms",
            "lexical overlap did not produce relevant evidence",
        ),
        blocker_taxonomy_row(
            "low_signal",
            low_signal,
            true,
            "refine_query_pack_or_alternate_provider",
            "retrieval produced insufficient citable signal",
        ),
    ];
    let primary = classes
        .iter()
        .find(|row| row.get("present").and_then(Value::as_bool) == Some(true))
        .and_then(|row| row.get("class").and_then(Value::as_str))
        .unwrap_or("none");
    json!({
        "version": "web_blocker_taxonomy_v1",
        "primary_class": primary,
        "classes": classes,
        "decision_authority": "tool_diagnostics_and_policy",
        "chat_visibility": "telemetry_only_until_synthesized"
    })
}

fn browser_materialization_recovery_report(
    flags: &[String],
    partial_failures: &[String],
    retry_reason: &str,
    blocker_taxonomy: &Value,
    actionable_ranked: &[(Candidate, f64)],
) -> Value {
    let combined_failures = partial_failures
        .iter()
        .map(|failure| clean_text(failure, 320).to_ascii_lowercase())
        .collect::<Vec<_>>();
    let has_access_or_render_blocker = flags.iter().any(|flag| {
        matches!(
            flag.as_str(),
            "anti_bot_filtered"
                | "content_rich_evidence_missing"
                | "claim_hints_missing"
                | "provider_degraded"
        )
    }) || combined_failures.iter().any(|failure| {
        [
            "captcha",
            "cloudflare",
            "verify you are human",
            "checking your browser",
            "needs_js",
            "javascript required",
            "bot wall",
            "waf",
            "access denied",
            "request blocked",
        ]
        .iter()
        .any(|marker| failure.contains(marker))
    });
    let attempted = combined_failures
        .iter()
        .any(|failure| failure.contains("browser_materialization_attempted"));
    let recommended_when_policy_allows =
        has_access_or_render_blocker && retry_reason != "none" && !attempted;
    let reason = if attempted {
        "already_attempted"
    } else if recommended_when_policy_allows {
        retry_reason
    } else {
        "not_needed_or_not_observed"
    };
    let blocker_class = blocker_taxonomy
        .get("primary_class")
        .and_then(Value::as_str)
        .unwrap_or("none");
    let candidate_safety = materialization_candidate_safety_refs(actionable_ranked, 3);
    let materializable_candidate_count = actionable_ranked
        .iter()
        .filter(|(candidate, _)| materialization_url_allowed(&candidate.locator))
        .count();
    json!({
        "version": "browser_materialization_recovery_v1",
        "capability": "browser_materialize_page",
        "blocker_class": blocker_class,
        "decision_authority": "tool_cd_and_gateway_policy",
        "chat_visibility": "telemetry_only",
        "recommended_when_policy_allows": recommended_when_policy_allows,
        "attempted": attempted,
        "availability": "not_observed_in_batch_query",
        "availability_source": "web_conduit_status_or_tool_cd",
        "reason": reason,
        "url_safety": {
            "version": "browser_materialization_url_safety_v1",
            "candidate_count": actionable_ranked.len(),
            "materializable_candidate_count": materializable_candidate_count,
            "candidate_refs": candidate_safety,
            "pre_navigation_safety_required": true,
            "post_redirect_safety_required": true,
            "raw_payload_chat_visible": false
        },
        "profile_compilation": browser_profile_compilation_report(),
        "readiness_lifecycle": browser_capability_readiness_lifecycle_report(),
        "guardrails": [
            "not_default_search",
            "public_http_https_only",
            "ssrf_and_redirect_safety_required",
            "no_proxy_or_persistent_session_without_separate_admission"
        ],
        "evidence_handoff": {
            "target_lane": "candidate_enrichment",
            "promotion_requires": [
                "safe_final_url",
                "substantive_main_text",
                "query_relevance",
                "not_blocker_shell"
            ],
            "raw_payload_chat_visible": false,
            "browser_success_is_not_source_truth_without_evidence_packaging": true
        }
    })
}

fn flags_include(flags: &[String], needle: &str) -> bool {
    flags.iter().any(|flag| flag == needle)
}

fn candidate_url_state(candidate_count: usize, actionable_ranked: &[(Candidate, f64)]) -> String {
    let has_safe_candidate = actionable_ranked
        .iter()
        .any(|(candidate, _)| materialization_url_allowed(&candidate.locator));
    let has_http_candidate = actionable_ranked.iter().any(|(candidate, _)| {
        candidate.locator.starts_with("http://") || candidate.locator.starts_with("https://")
    });
    if has_safe_candidate {
        "candidate_url_ref_available".to_string()
    } else if has_http_candidate {
        "candidate_url_ref_blocked_by_safety".to_string()
    } else if candidate_count > 0 {
        "candidate_count_observed_without_projected_url_ref".to_string()
    } else {
        "absent".to_string()
    }
}

fn top_candidate_refs(actionable_ranked: &[(Candidate, f64)], limit: usize) -> Vec<Value> {
    actionable_ranked
        .iter()
        .take(limit.max(1))
        .map(|(candidate, score)| {
            json!({
                "title": clean_text(&candidate.title, 120),
                "locator": clean_text(&candidate.locator, 240),
                "domain": candidate_domain_hint(candidate),
                "score": (*score * 100.0).round() / 100.0,
                "url_safety_status": materialization_url_safety_assessment(&candidate.locator)
                    .get("status")
                    .cloned()
                    .unwrap_or_else(|| json!("unknown"))
            })
        })
        .collect()
}

fn query_refinement_signals(
    query: &str,
    flags: &[String],
    missing_buckets: &[String],
    blocker_taxonomy: &Value,
    actionable_ranked: &[(Candidate, f64)],
) -> Value {
    let mut preserve_terms = tokenize_relevance(query, 32)
        .into_iter()
        .filter(|term| term.len() >= 4)
        .collect::<Vec<_>>();
    preserve_terms.sort();
    preserve_terms.truncate(10);

    let mut term_hints = Vec::<String>::new();
    for (candidate, _) in actionable_ranked.iter().take(4) {
        for term in evidence_pack_term_hints(query, candidate, 4) {
            if !term_hints.iter().any(|existing| existing == &term) {
                term_hints.push(term);
            }
            if term_hints.len() >= 10 {
                break;
            }
        }
        if term_hints.len() >= 10 {
            break;
        }
    }

    let primary_blocker = blocker_taxonomy
        .get("primary_class")
        .and_then(Value::as_str)
        .unwrap_or("none");
    let mut strategy_signals = Vec::<&str>::new();
    if current_web_intent(query) {
        strategy_signals.push("prefer_current_or_recent_source_classes");
    }
    if flags_include(flags, "comparison_evidence_insufficient") {
        strategy_signals.push("split_by_entities_or_comparison_facets");
    }
    if flags_include(flags, "weak_single_source")
        || flags_include(flags, "source_diversity_limited")
    {
        strategy_signals.push("add_independent_source_or_source_class");
    }
    if matches!(primary_blocker, "off_intent_noise" | "low_signal") {
        strategy_signals.push("remove_terms_that_only_matched_off_intent_rows");
    }
    if matches!(
        primary_blocker,
        "anti_bot_challenge" | "needs_js" | "content_materialization_missing"
    ) {
        strategy_signals
            .push("try_candidate_specific_or_direct_source_queries_before_user_narrowing");
    }
    if strategy_signals.is_empty() {
        strategy_signals.push("preserve_user_goal_and_add_missing_coverage_facets");
    }
    strategy_signals.sort();
    strategy_signals.dedup();

    json!({
        "version": "query_refinement_signals_v1",
        "authority": "tool_diagnostics_only_agent_authors_queries",
        "hidden_query_generation": false,
        "preserve_terms": preserve_terms,
        "term_hints_from_candidates": term_hints,
        "missing_coverage_buckets": missing_buckets,
        "blocker_class": primary_blocker,
        "strategy_signals": strategy_signals,
        "non_goals": [
            "do_not_generate_hidden_queries",
            "do_not_hardcode_research_domain",
            "do_not_force_answer_format"
        ]
    })
}

fn retrieval_decision_lattice(
    status: &str,
    candidate_count: usize,
    evidence_count: usize,
    coverage_status: &str,
    flags: &[String],
    retry_reason: &str,
    blocker_taxonomy: &Value,
    browser_materialization: &Value,
    actionable_ranked: &[(Candidate, f64)],
) -> Value {
    let primary_blocker = blocker_taxonomy
        .get("primary_class")
        .and_then(Value::as_str)
        .unwrap_or("none");
    let browser_recommended = browser_materialization
        .get("recommended_when_policy_allows")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let url_state = candidate_url_state(candidate_count, actionable_ranked);
    let candidate_available = url_state != "absent";
    let candidate_url_ref_available = url_state == "candidate_url_ref_available";
    let enough_evidence =
        retry_reason == "none" && evidence_count > 0 && coverage_status == "covered";
    let quality_refinement_needed = flags_include(flags, "comparison_evidence_insufficient")
        || flags_include(flags, "weak_single_source")
        || primary_blocker == "off_intent_noise";

    let (decision, action_status, fallback_decision, confidence, rationale) = if enough_evidence {
        (
            "synthesize_from_evidence",
            "ready_for_synthesis",
            "none",
            "high",
            vec!["coverage_is_covered", "no_retry_reason"],
        )
    } else if browser_recommended
        && candidate_url_ref_available
        && matches!(primary_blocker, "anti_bot_challenge" | "needs_js")
    {
        (
            "browser_materialize_candidate",
            "requires_capability_admission",
            "agent_refine_query_pack",
            "medium",
            vec!["candidate_observed", "render_or_access_blocker_detected"],
        )
    } else if browser_recommended
        && candidate_url_ref_available
        && primary_blocker == "content_materialization_missing"
        && !quality_refinement_needed
    {
        (
            "browser_materialize_candidate",
            "requires_capability_admission",
            "agent_refine_query_pack",
            "medium",
            vec!["candidate_observed", "content_materialization_gap_detected"],
        )
    } else if matches!(primary_blocker, "anti_bot_challenge" | "needs_js") {
        (
            "alternate_provider",
            "requires_admitted_alternate_provider_or_browser_retrieval_capability",
            "agent_refine_query_pack",
            "medium",
            vec!["render_or_access_blocker_detected_without_candidate_url"],
        )
    } else if matches!(
        primary_blocker,
        "rate_limited" | "provider_starved" | "provider_degraded"
    ) {
        (
            "alternate_provider",
            "requires_admitted_alternate_provider",
            "agent_refine_query_pack",
            "medium",
            vec!["provider_or_throttle_blocker_detected"],
        )
    } else if primary_blocker == "access_denied" {
        (
            "alternate_provider",
            "requires_alternate_source_or_permission_boundary",
            "structured_low_evidence",
            "low",
            vec!["access_denied_is_not_evidence"],
        )
    } else if quality_refinement_needed {
        (
            "agent_refine_query_pack",
            "requires_agent_query",
            "structured_low_evidence",
            "medium",
            vec!["coverage_or_relevance_gap_detected"],
        )
    } else if candidate_available
        && evidence_count == 0
        && !matches!(
            primary_blocker,
            "anti_bot_challenge" | "needs_js" | "access_denied"
        )
    {
        (
            "direct_fetch_candidate",
            "requires_fetch_budget",
            "agent_refine_query_pack",
            "medium",
            vec!["candidate_observed_without_promoted_evidence"],
        )
    } else if retry_reason != "none" && status != "ok" {
        (
            "agent_refine_query_pack",
            "requires_agent_query",
            "structured_low_evidence",
            "low",
            vec!["tool_result_did_not_yield_usable_evidence"],
        )
    } else if retry_reason != "none" {
        (
            "agent_refine_query_pack",
            "requires_agent_query",
            "structured_low_evidence",
            "medium",
            vec!["quality_gate_requested_retry"],
        )
    } else {
        (
            "structured_low_evidence",
            "terminal_if_budget_exhausted",
            "none",
            "low",
            vec!["no_higher_value_retrieval_action_identified"],
        )
    };

    json!({
        "version": "retrieval_decision_lattice_v1",
        "decision": decision,
        "action_status": action_status,
        "fallback_decision": fallback_decision,
        "confidence": confidence,
        "rationale": rationale,
        "inputs": {
            "status": status,
            "candidate_count": candidate_count,
            "evidence_count": evidence_count,
            "coverage_status": coverage_status,
            "retry_reason": retry_reason,
            "primary_blocker": primary_blocker,
            "candidate_url_state": url_state,
            "browser_materialization_recommended_when_policy_allows": browser_recommended
        },
        "candidate_refs": top_candidate_refs(actionable_ranked, 3),
        "authority": {
            "tool": "classifies_and_recommends_next_capability",
            "agent": "authors_queries_and_final_synthesis",
            "gateway": "admits_browser_or_provider_capabilities"
        },
        "non_goals": [
            "do_not_hide_tool_generated_queries",
            "do_not_treat_browser_success_as_source_truth",
            "do_not_force_answer_format"
        ]
    })
}

fn strong_evidence_available(query: &str, actionable_ranked: &[(Candidate, f64)]) -> bool {
    actionable_ranked.iter().any(|(candidate, score)| {
        if *score < 0.75 || candidate_is_low_confidence_retained(candidate) {
            return false;
        }
        if !content_rich_text(&candidate.snippet) {
            return false;
        }
        let flags = candidate_quality_flags(query, candidate, *score);
        !flags.iter().any(|flag| {
            matches!(
                flag.as_str(),
                "low_trust_source"
                    | "thin_query_overlap"
                    | "freshness_unproven"
                    | "junk_marker"
                    | "low_score"
            )
        })
    })
}

fn potential_source_conflict(actionable_ranked: &[(Candidate, f64)]) -> bool {
    let mut positive_claim = false;
    let mut limiting_claim = false;
    for (candidate, _) in actionable_ranked {
        let text = clean_text(&format!("{} {}", candidate.title, candidate.snippet), 1_600)
            .to_ascii_lowercase();
        positive_claim |= [
            "best",
            "better",
            "outperform",
            "leads",
            "faster",
            "stronger",
            "higher",
        ]
        .iter()
        .any(|marker| text.contains(marker));
        limiting_claim |= [
            "worse",
            "weaker",
            "slower",
            "lower",
            "limitation",
            "trade-off",
            "tradeoff",
            "lacks",
        ]
        .iter()
        .any(|marker| text.contains(marker));
    }
    positive_claim && limiting_claim
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
    let quality_failures = if evidence_count > 0 {
        hard_partial_failures
    } else {
        partial_failures
    };
    let mut flags = issue_quality_flags(quality_failures);
    if status == "no_results" || evidence_count == 0 {
        flags.push("insufficient_evidence".to_string());
    } else if status == "low_signal" {
        flags.push("low_signal".to_string());
    } else if status == "partial" {
        flags.push("partial_results".to_string());
    } else if evidence_count >= 2 && hard_partial_failures.is_empty() {
        flags.push("high_confidence".to_string());
    }
    if actionable_ranked
        .iter()
        .any(|(candidate, _)| candidate_is_low_confidence_retained(candidate))
    {
        flags.push("low_confidence_raw_evidence".to_string());
    }
    let materialized_candidate_count = actionable_ranked
        .iter()
        .filter(|(candidate, score)| {
            candidate_counts_as_query_usable_evidence(query, candidate, *score)
        })
        .count();
    let trusted_structured_feed_count = actionable_ranked
        .iter()
        .filter(|(candidate, _)| {
            candidate_materialization_quality(candidate) == "trusted_structured_feed"
        })
        .count();
    let candidate_only_count = actionable_ranked
        .iter()
        .filter(|(candidate, _)| candidate_materialization_quality(candidate) == "candidate_only")
        .count();
    let failed_materialization_count = actionable_ranked
        .iter()
        .filter(|(candidate, _)| {
            candidate_materialization_quality(candidate) == "failed_materialization"
        })
        .count();
    let content_rich_candidate_count = actionable_ranked
        .iter()
        .filter(|(candidate, _)| {
            candidate_counts_as_query_usable_evidence(
                query,
                candidate,
                rerank_score(query, candidate),
            ) && content_rich_text(&candidate.snippet)
        })
        .count();
    let claim_hint_count = actionable_ranked
        .iter()
        .filter(|(candidate, score)| {
            candidate_counts_as_query_usable_evidence(query, candidate, *score)
        })
        .map(|(candidate, _)| evidence_pack_claim_hints_for_candidate(query, candidate, 2).len())
        .sum::<usize>();
    let evidence_claim_count = claim_hint_count;
    let materialization_failure_report = materialization_failure_report(
        partial_failures,
        evidence_count,
        materialized_candidate_count,
        candidate_only_count,
        failed_materialization_count,
    );
    if evidence_count > 0 && materialized_candidate_count == 0 {
        flags.push("materialized_evidence_missing".to_string());
    }
    if evidence_count > 0 && candidate_only_count > 0 && materialized_candidate_count == 0 {
        flags.push("candidate_only_evidence".to_string());
    }
    if evidence_count > 0 && content_rich_candidate_count == 0 {
        flags.push("content_rich_evidence_missing".to_string());
    }
    if evidence_count > 0 && claim_hint_count == 0 {
        flags.push("claim_hints_missing".to_string());
    }
    if evidence_count >= 2 && content_rich_candidate_count > 0 && claim_hint_count > 0 {
        let hard_access_or_transport_blocker = flags.iter().any(|flag| {
            matches!(
                flag.as_str(),
                "anti_bot_filtered"
                    | "needs_js"
                    | "rate_limited"
                    | "access_denied"
                    | "provider_degraded"
                    | "provider_timeout"
                    | "insufficient_evidence"
            )
        });
        if !hard_access_or_transport_blocker && flags.iter().any(|flag| flag == "provider_starved")
        {
            flags.retain(|flag| flag != "provider_starved");
            flags.push("credentialed_provider_unavailable_nonblocking".to_string());
        }
    }
    if status == "ok"
        && evidence_count > 0
        && claim_hint_count > 0
        && hard_partial_failures.is_empty()
        && !flags.iter().any(|flag| {
            matches!(
                flag.as_str(),
                "anti_bot_filtered"
                    | "needs_js"
                    | "rate_limited"
                    | "access_denied"
                    | "provider_degraded"
                    | "insufficient_evidence"
            )
        })
    {
        flags.retain(|flag| flag != "low_signal");
    }
    let mut domains = HashSet::<String>::new();
    for (candidate, _) in actionable_ranked {
        let domain = candidate_domain_hint(candidate).to_ascii_lowercase();
        if !domain.is_empty() && domain != "source" {
            domains.insert(domain);
        }
    }
    if evidence_count > 1 && domains.len() < evidence_count {
        flags.push("source_diversity_limited".to_string());
    }
    if is_benchmark_or_comparison_intent(query) && evidence_count < 2 {
        flags.push("comparison_evidence_insufficient".to_string());
    }
    if is_benchmark_or_comparison_intent(query) && evidence_count > 1 {
        flags.push("comparative_synthesis_required".to_string());
    }
    if potential_source_conflict(actionable_ranked) {
        flags.push("potential_source_conflict".to_string());
    }
    if status == "ok" && strong_evidence_available(query, actionable_ranked) {
        flags.retain(|flag| flag != "low_signal");
    }
    flags.sort();
    flags.dedup();
    let candidate_quality = actionable_ranked
        .iter()
        .take(6)
        .map(|(candidate, score)| {
            json!({
                "title": clean_text(&candidate.title, 160),
                "domain": candidate_domain_hint(candidate),
                "locator": clean_text(&candidate.locator, 240),
                "snippet_preview": trim_words(&clean_text(&candidate.snippet, 600), 32),
                "score": (*score * 100.0).round() / 100.0,
                "flags": candidate_quality_flags(query, candidate, *score)
            })
        })
        .collect::<Vec<_>>();
    let weak_single_source = evidence_count == 1
        && actionable_ranked.first().is_some_and(|(candidate, score)| {
            let candidate_flags = candidate_quality_flags(query, candidate, *score);
            *score < 0.65
                && candidate_flags.iter().any(|flag| {
                    matches!(
                        flag.as_str(),
                        "thin_query_overlap"
                            | "low_score"
                            | "low_trust_source"
                            | "freshness_unproven"
                            | "junk_marker"
                    )
                })
        });
    if weak_single_source {
        flags.push("weak_single_source".to_string());
    }
    flags.sort();
    flags.dedup();
    let coverage_status = if evidence_count == 0 {
        "missing"
    } else if weak_single_source || flags.iter().any(|flag| flag == "low_signal") {
        "weak"
    } else {
        "covered"
    };
    let mut missing_buckets = Vec::<String>::new();
    if evidence_count == 0 {
        missing_buckets.push("usable_evidence".to_string());
    }
    if evidence_count > 0 && materialized_candidate_count == 0 {
        missing_buckets.push("materialized_source_content".to_string());
    }
    if weak_single_source {
        missing_buckets.push("source_diversity_or_confidence".to_string());
    }
    if flags
        .iter()
        .any(|flag| flag == "comparison_evidence_insufficient")
    {
        missing_buckets.push("comparison_coverage".to_string());
    }
    if flags.iter().any(|flag| flag == "freshness_unproven") {
        missing_buckets.push("freshness_signal".to_string());
    }
    if flags.iter().any(|flag| flag == "provider_starved") {
        missing_buckets.push("strong_search_provider".to_string());
    }
    missing_buckets.sort();
    missing_buckets.dedup();
    let retry_reason = if flags.iter().any(|flag| flag == "anti_bot_filtered") {
        "anti_bot_filtered"
    } else if flags.iter().any(|flag| flag == "needs_js") {
        "needs_js"
    } else if flags.iter().any(|flag| flag == "rate_limited") {
        "rate_limited"
    } else if flags.iter().any(|flag| flag == "access_denied") {
        "access_denied"
    } else if flags.iter().any(|flag| flag == "provider_starved") {
        "provider_starved"
    } else if flags.iter().any(|flag| flag == "provider_degraded") {
        "provider_degraded"
    } else if flags.iter().any(|flag| flag == "insufficient_evidence") {
        "insufficient_evidence"
    } else if flags
        .iter()
        .any(|flag| flag == "comparison_evidence_insufficient")
    {
        "comparison_evidence_insufficient"
    } else if flags.iter().any(|flag| flag == "weak_single_source") {
        "weak_single_source"
    } else if flags.iter().any(|flag| flag == "low_signal") {
        "low_signal"
    } else {
        "none"
    };
    let blocker_taxonomy = browser_materialization_blocker_taxonomy(&flags, quality_failures);
    let browser_materialization = browser_materialization_recovery_report(
        &flags,
        quality_failures,
        retry_reason,
        &blocker_taxonomy,
        actionable_ranked,
    );
    let retrieval_decision = retrieval_decision_lattice(
        status,
        candidate_count,
        evidence_count,
        coverage_status,
        &flags,
        retry_reason,
        &blocker_taxonomy,
        &browser_materialization,
        actionable_ranked,
    );
    let query_refinement = query_refinement_signals(
        query,
        &flags,
        &missing_buckets,
        &blocker_taxonomy,
        actionable_ranked,
    );
    json!({
        "version": web_tool_quality_version(),
        "status": status,
        "flags": flags,
        "candidate_count": candidate_count,
        "evidence_count": evidence_count,
        "coverage": {
            "bucket_status": coverage_status,
            "missing_buckets": missing_buckets,
            "source_domain_count": domains.len(),
            "basis": "usable_evidence_source_diversity_and_quality_flags"
        },
        "freshness": {
            "current_intent": current_web_intent(query),
            "current_year": current_year()
        },
        "content_rich_candidate_count": content_rich_candidate_count,
        "claim_hint_count": claim_hint_count,
        "evidence_claim_count": evidence_claim_count,
        "materialized_candidate_count": materialized_candidate_count,
        "trusted_structured_feed_count": trusted_structured_feed_count,
        "candidate_only_count": candidate_only_count,
        "failed_materialization_count": failed_materialization_count,
        "materialization_failure_report": materialization_failure_report,
        "candidate_quality": candidate_quality,
        "blocker_taxonomy": blocker_taxonomy,
        "browser_materialization": browser_materialization,
        "retrieval_decision": retrieval_decision,
        "synthesis_contract": {
            "authority": "agent_authored",
            "must_not_claim_more_than_evidence": true,
            "use_candidate_snippet_previews_only_as_grounding": true
        },
        "retry": {
            "recommended": retry_reason != "none",
            "reason": retry_reason,
            "input_contract": {
                "authority": "agent_submitted",
                "input_kind": "query_or_query_pack",
                "required_fields": ["query"],
                "optional_fields": ["queries", "source_url"],
                "hidden_query_expansion": false
            },
            "query_strategy_hints": [
                "preserve the user's original research objective",
                "split broad requests into entity-specific or aspect-specific searches",
                "target primary, official, source-backed, or directly citable pages when possible",
                "use partial snippets, failure reasons, and off-topic signals to remove weak terms",
                "for current or recent research, prefer current/recent source-class searches such as changelog, release notes, repository, publication, announcement, security advisory, or methodology over broad stale year ranges",
                "for one named entity, keep the exact entity name in every retry query and vary source class or decision aspect rather than replacing it with loose synonyms",
                "prefer another agent-submitted query pack before asking the user to narrow"
            ],
            "query_refinement_signals": query_refinement,
            "next_action": if retry_reason == "none" {
                "synthesize_from_evidence"
            } else {
                "agent_refine_query_pack_and_retry_if_budget_remains"
            }
        }
    })
}

fn cached_web_tool_quality_report(
    query: &str,
    status: &str,
    partial_failure_details: &Value,
    evidence_refs: &Value,
) -> Value {
    let failures = partial_failure_details
        .as_array()
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let evidence_count = evidence_refs.as_array().map(Vec::len).unwrap_or(0);
    let mut report =
        web_tool_quality_report(query, status, 0, evidence_count, &failures, &failures, &[]);
    let weak_cached_single_source = evidence_count == 1
        && evidence_refs
            .as_array()
            .and_then(|rows| rows.first())
            .and_then(|row| row.get("score"))
            .and_then(Value::as_f64)
            .map(|score| score < 0.65)
            .unwrap_or(false);
    if weak_cached_single_source {
        if let Some(flags) = report.get_mut("flags").and_then(Value::as_array_mut) {
            if !flags.iter().any(|flag| flag == "weak_single_source") {
                flags.push(json!("weak_single_source"));
            }
        }
        report["retry"]["recommended"] = json!(true);
        report["retry"]["reason"] = json!("weak_single_source");
        report["retry"]["next_action"] =
            json!("agent_refine_query_pack_and_retry_if_budget_remains");
    }
    report
}
