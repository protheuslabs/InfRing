fn official_domain_probe_enabled(policy: &Value) -> bool {
    page_extraction_enabled(policy)
        && policy
            .pointer("/batch_query/page_extraction/official_domain_probe/enabled")
            .and_then(Value::as_bool)
            .unwrap_or(true)
}

fn official_domain_probe_max_urls(policy: &Value) -> usize {
    policy
        .pointer("/batch_query/page_extraction/official_domain_probe/max_urls")
        .and_then(Value::as_u64)
        .unwrap_or(8)
        .clamp(0, 18) as usize
}

fn official_domain_probe_tlds(policy: &Value) -> Vec<String> {
    let configured = policy
        .pointer("/batch_query/page_extraction/official_domain_probe/tlds")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| {
                    clean_text(
                        row.trim().trim_start_matches('.').to_ascii_lowercase().as_str(),
                        16,
                    )
                })
                .filter(|row| {
                    row.len() >= 2
                        && row.len() <= 12
                        && row.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if !configured.is_empty() {
        return configured;
    }
    ["com", "ai", "dev", "io", "app", "org"]
        .iter()
        .map(|row| row.to_string())
        .collect()
}

fn official_domain_probe_slug(raw: &str) -> Option<String> {
    let slug = raw
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .collect::<String>();
    if slug.len() >= 3 && slug.len() <= 48 {
        Some(slug)
    } else {
        None
    }
}

fn official_domain_probe_subject_slugs(query: &str) -> Vec<String> {
    let mut out = Vec::<String>::new();
    let mut seen = HashSet::<String>::new();
    for phrase in query_subject_phrases(query) {
        if let Some(slug) = official_domain_probe_slug(&phrase) {
            if seen.insert(slug.clone()) {
                out.push(slug);
            }
        }
        if let Some(first_token) = phrase.split_whitespace().next() {
            if let Some(slug) = official_domain_probe_slug(first_token) {
                if seen.insert(slug.clone()) {
                    out.push(slug);
                }
            }
        }
    }
    out
}

fn official_domain_probe_urls(policy: &Value, query: &str) -> Vec<String> {
    if !is_official_source_query_lane(query) || !official_domain_probe_enabled(policy) {
        return Vec::new();
    }
    let max_urls = official_domain_probe_max_urls(policy);
    if max_urls == 0 {
        return Vec::new();
    }
    let slugs = official_domain_probe_subject_slugs(query);
    let tlds = official_domain_probe_tlds(policy);
    let mut out = Vec::<String>::new();
    let mut seen = HashSet::<String>::new();
    for slug in slugs {
        for tld in &tlds {
            let url = format!("https://{slug}.{tld}/");
            if seen.insert(url.clone()) {
                out.push(url);
                if out.len() >= max_urls {
                    return out;
                }
            }
        }
    }
    out
}

fn force_candidate_locator_to_fetch_url(candidate: &mut Candidate, payload: &Value) {
    let locator = payload
        .get("final_url")
        .or_else(|| payload.get("resolved_url"))
        .or_else(|| payload.get("requested_url"))
        .and_then(Value::as_str)
        .map(|row| clean_text(row, 2_200))
        .filter(|row| row.starts_with("http://") || row.starts_with("https://"));
    if let Some(locator) = locator {
        candidate.locator = locator;
    }
}

fn recover_official_domain_probe_candidates(
    root: &Path,
    query: &str,
    policy: &Value,
    benchmark_intent: bool,
    fetch_budget: &PageExtractionFetchBudget,
) -> (Vec<Candidate>, Vec<String>, Vec<Value>) {
    let mut candidates = Vec::<Candidate>::new();
    let mut issues = Vec::<String>::new();
    let mut provider_results = Vec::<Value>::new();
    let urls = official_domain_probe_urls(policy, query);
    if urls.is_empty() || !fetch_budget.has_remaining() {
        return (candidates, issues, provider_results);
    }
    issues.push("official_domain_probe:attempted".to_string());
    for url in urls {
        match fetch_budget.reserve(policy, &url, true) {
            PageExtractionFetchReservation::Reserved => {}
            PageExtractionFetchReservation::Duplicate => continue,
            PageExtractionFetchReservation::Exhausted => {
                issues.push("official_domain_probe:fetch_budget_exhausted".to_string());
                break;
            }
        }
        let fetch_payload = stage_fetch_payload(
            root,
            "official_domain_probe",
            &url,
            &page_extraction_extract_mode(policy),
        );
        let mut stage_issues = Vec::<String>::new();
        let mut promoted = 0usize;
        if !fetch_payload
            .get("ok")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            stage_issues.push(format!(
                "official_domain_probe:fetch:{}",
                stage_error(&fetch_payload, "web_fetch_failed")
            ));
        } else {
            match candidate_from_search_payload(query, &fetch_payload) {
                Ok(mut candidate) => {
                    force_candidate_locator_to_fetch_url(&mut candidate, &fetch_payload);
                    mark_candidate_as_page_enriched(&mut candidate);
                    if candidate_is_synthesis_eligible(query, &candidate, benchmark_intent)
                        && official_lane_direct_subject_source_signal(query, &candidate)
                    {
                        promoted = 1;
                        candidates.push(candidate);
                    } else {
                        stage_issues
                            .push("official_domain_probe:candidate_low_relevance".to_string());
                    }
                }
                Err(err) => {
                    stage_issues.push(format!("official_domain_probe:fetch_candidate:{err}"));
                }
            }
        }
        if let Some(value) = hidden_provider_result_artifact(
            "official_domain_probe",
            query,
            &fetch_payload,
            promoted,
            &stage_issues,
        ) {
            provider_results.push(value);
        }
        issues.extend(stage_issues);
        if provider_recovery_satisfied(query, &candidates, benchmark_intent) {
            break;
        }
    }
    (candidates, issues, provider_results)
}
