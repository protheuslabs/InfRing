
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
    let locator_bonus = if candidate.locator.is_empty() {
        0.0
    } else {
        0.2
    };
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

fn retrieve_web_candidates_for_query_with_timeout(
    root: &Path,
    query: &str,
    policy: &Value,
    search_scope: &BatchQuerySearchScope,
    fetch_budget: PageExtractionFetchBudget,
    timeout: Duration,
) -> (Vec<Candidate>, Vec<String>, Vec<Value>) {
    let (tx, rx) = std::sync::mpsc::channel::<(Vec<Candidate>, Vec<String>, Vec<Value>)>();
    let root_buf = root.to_path_buf();
    let query_buf = query.to_string();
    let policy_buf = policy.clone();
    let search_scope_buf = search_scope.clone();
    let fetch_budget_buf = fetch_budget.clone();
    let spawned = thread::Builder::new()
        .name("batch-query-retrieve".to_string())
        .spawn(move || {
            let out = retrieve_web_candidates_for_query(
                &root_buf,
                &query_buf,
                &policy_buf,
                &search_scope_buf,
                fetch_budget_buf,
            );
            let _ = tx.send(out);
        });
    if spawned.is_err() {
        return (
            Vec::new(),
            vec!["query_worker_spawn_failed".to_string()],
            Vec::new(),
        );
    }
    match rx.recv_timeout(timeout) {
        Ok(out) => out,
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => (
            Vec::new(),
            vec![format!("query_timeout_ms_{}", timeout.as_millis())],
            Vec::new(),
        ),
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => (
            Vec::new(),
            vec!["query_worker_disconnected".to_string()],
            Vec::new(),
        ),
    }
}

fn is_benign_partial_failure(detail: &str) -> bool {
    let lowered = clean_text(detail, 320).to_ascii_lowercase();
    if lowered.contains("anti_bot_challenge") {
        return false;
    }
    lowered.contains("candidate_low_relevance")
        || lowered.contains("fetch_candidate_low_relevance")
        || lowered.contains("no_usable_summary")
        || lowered.contains("fixture_missing")
}

fn is_diagnostic_only_after_complete_coverage(detail: &str) -> bool {
    let lowered = clean_text(detail, 320).to_ascii_lowercase();
    lowered.contains("page_extraction_global_budget_exhausted")
        || lowered.contains("page_extraction_candidate_prefetch_rejected")
        || lowered.contains("trusted_primary_lane_preserved")
}
