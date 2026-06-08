
#[derive(Clone)]
struct PageExtractionFetchBudget {
    max_total_fetches: usize,
    reserved_trusted_primary_fetches: usize,
    fetched_link_keys: std::sync::Arc<std::sync::Mutex<HashSet<String>>>,
    trusted_primary_link_keys: std::sync::Arc<std::sync::Mutex<HashSet<String>>>,
}

#[derive(Debug, PartialEq, Eq)]
enum PageExtractionFetchReservation {
    Reserved,
    Duplicate,
    Exhausted,
}

impl PageExtractionFetchBudget {
    fn new(policy: &Value) -> Self {
        Self {
            max_total_fetches: page_extraction_max_total_fetches(policy),
            reserved_trusted_primary_fetches: page_extraction_reserved_trusted_primary_fetches(policy),
            fetched_link_keys: std::sync::Arc::new(std::sync::Mutex::new(HashSet::new())),
            trusted_primary_link_keys: std::sync::Arc::new(std::sync::Mutex::new(HashSet::new())),
        }
    }

    fn has_remaining(&self) -> bool {
        self.max_total_fetches > self.reserved_count()
    }

    fn reserved_count(&self) -> usize {
        self.fetched_link_keys
            .lock()
            .map(|links| links.len())
            .unwrap_or_else(|poisoned| poisoned.into_inner().len())
    }

    fn trusted_primary_reserved_count(&self) -> usize {
        self.trusted_primary_link_keys
            .lock()
            .map(|links| links.len())
            .unwrap_or_else(|poisoned| poisoned.into_inner().len())
    }

    fn reserve(
        &self,
        policy: &Value,
        link: &str,
        trusted_primary_lane: bool,
    ) -> PageExtractionFetchReservation {
        let Some(normalized_link) = normalize_page_extraction_link(policy, link) else {
            return PageExtractionFetchReservation::Duplicate;
        };
        let dedupe_key = page_extraction_link_dedupe_key(policy, &normalized_link);
        if dedupe_key.is_empty() {
            return PageExtractionFetchReservation::Duplicate;
        }
        let mut links = self
            .fetched_link_keys
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if links.contains(&dedupe_key) {
            return PageExtractionFetchReservation::Duplicate;
        }
        if links.len() >= self.max_total_fetches {
            return PageExtractionFetchReservation::Exhausted;
        }
        if !trusted_primary_lane && self.reserved_trusted_primary_fetches > 0 {
            let trusted_reserved = self.trusted_primary_reserved_count();
            let non_trusted_reserved = links.len().saturating_sub(trusted_reserved);
            let general_budget = self
                .max_total_fetches
                .saturating_sub(self.reserved_trusted_primary_fetches);
            if non_trusted_reserved >= general_budget {
                return PageExtractionFetchReservation::Exhausted;
            }
        }
        links.insert(dedupe_key.clone());
        if trusted_primary_lane {
            self.trusted_primary_link_keys
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .insert(dedupe_key);
        }
        PageExtractionFetchReservation::Reserved
    }
}

fn hub_discovery_low_relevance_issue(stage: &str, candidate: &Candidate, suffix: &str) -> String {
    if looks_like_competitive_programming_dump(&format!(
        "{} {} {}",
        candidate.title, candidate.snippet, candidate.locator
    )) {
        format!("{stage}:hub_discovery_query_result_mismatch")
    } else {
        format!("{stage}:hub_discovery_{suffix}")
    }
}

fn try_expand_relevant_hub_payload(
    root: &Path,
    stage: &str,
    query: &str,
    policy: &Value,
    hub_payload: &Value,
    benchmark_intent: bool,
    fetch_budget: &PageExtractionFetchBudget,
    candidates: &mut Vec<Candidate>,
    issues: &mut Vec<String>,
    retained_low_confidence: &mut usize,
) -> usize {
    if !page_extraction_hub_discovery_enabled(policy) || !fetch_budget.has_remaining() {
        return 0;
    }
    let max_links = page_extraction_hub_discovery_max_links(policy);
    if max_links == 0 {
        return 0;
    }
    let (links, prefetch_rejections) = payload_links_for_page_extraction_with_rejections(
        query,
        policy,
        hub_payload,
        max_links,
    );
    for rejection in prefetch_rejections {
        issues.push(format!(
            "{stage}:hub_discovery_candidate_prefetch_rejected:{rejection}"
        ));
    }
    let mut promoted = 0usize;
    for link in links {
        let link_candidate = page_extraction_link_candidate(&link);
        let trusted_primary_lane = is_official_source_query_lane(query)
            || candidate_has_trusted_primary_source_signal(query, &link_candidate);
        match fetch_budget.reserve(policy, &link, trusted_primary_lane) {
            PageExtractionFetchReservation::Reserved => {}
            PageExtractionFetchReservation::Duplicate => continue,
            PageExtractionFetchReservation::Exhausted => {
                issues.push(format!("{stage}:hub_discovery_fetch_budget_exhausted"));
                break;
            }
        }
        let fetch_payload =
            stage_fetch_payload(root, stage, &link, &page_extraction_extract_mode(policy));
        if !fetch_payload
            .get("ok")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            issues.push(format!(
                "{stage}:hub_discovery_fetch:{}",
                stage_error(&fetch_payload, "web_fetch_failed")
            ));
            continue;
        }
        match candidate_from_search_payload(query, &fetch_payload) {
            Ok(mut candidate) => {
                if candidate.locator.is_empty()
                    || is_search_engine_domain(&candidate_domain_hint(&candidate))
                {
                    candidate.locator = link.clone();
                }
                mark_candidate_as_page_enriched(&mut candidate);
                if candidate_is_synthesis_eligible(query, &candidate, benchmark_intent) {
                    merge_or_push_page_enriched_candidate(query, policy, candidates, candidate);
                    promoted += 1;
                } else if let Some(candidate) =
                    retain_low_confidence_candidate(policy, &candidate, retained_low_confidence)
                {
                    issues.push(hub_discovery_low_relevance_issue(
                        stage,
                        &candidate,
                        "fetch_candidate_low_relevance_retained_low_confidence",
                    ));
                    candidates.push(candidate);
                } else {
                    issues.push(hub_discovery_low_relevance_issue(
                        stage,
                        &candidate,
                        "fetch_candidate_low_relevance",
                    ));
                }
            }
            Err(err) => {
                issues.push(format!("{stage}:hub_discovery_fetch_candidate:{err}"));
            }
        }
    }
    promoted
}

fn collect_candidates_from_stage_payload(
    root: &Path,
    stage: &str,
    query: &str,
    policy: &Value,
    payload: &Value,
    benchmark_intent: bool,
    fetch_budget: &PageExtractionFetchBudget,
) -> (Vec<Candidate>, Vec<String>, Option<Value>) {
    let mut candidates = Vec::<Candidate>::new();
    let mut issues = Vec::<String>::new();
    let mut retained_low_confidence = 0usize;
    let low_relevance_issue = |candidate: &Candidate, suffix: &str| {
        if looks_like_competitive_programming_dump(&format!(
            "{} {} {}",
            candidate.title, candidate.snippet, candidate.locator
        )) {
            format!("{stage}:query_result_mismatch")
        } else {
            format!("{stage}:{suffix}")
        }
    };
    if let Some(blocker_class) = payload_access_blocker_class(payload) {
        issues.push(format!("{stage}:{blocker_class}"));
        let provider_result = hidden_provider_result_artifact(stage, query, payload, 0, &issues);
        return (candidates, issues, provider_result);
    }
    if structured_results_enabled(policy) {
        for candidate in candidates_from_structured_search_payload(
            query,
            payload,
            structured_results_max_rows_per_stage(policy),
        ) {
            if candidate_is_synthesis_eligible(query, &candidate, benchmark_intent) {
                candidates.push(candidate);
            } else if let Some(candidate) =
                retain_low_confidence_candidate(policy, &candidate, &mut retained_low_confidence)
            {
                issues.push(low_relevance_issue(
                    &candidate,
                    "candidate_low_relevance_retained_low_confidence",
                ));
                candidates.push(candidate);
            } else {
                issues.push(low_relevance_issue(&candidate, "candidate_low_relevance"));
            }
        }
    }
    let rendered_rows = candidates_from_rendered_search_payload(
        query,
        payload,
        if is_framework_catalog_intent(query) { 4 } else { 2 },
    );
    for candidate in rendered_rows {
        if candidate_is_synthesis_eligible(query, &candidate, benchmark_intent) {
            candidates.push(candidate);
        } else if let Some(candidate) =
            retain_low_confidence_candidate(policy, &candidate, &mut retained_low_confidence)
        {
            issues.push(low_relevance_issue(
                &candidate,
                "candidate_low_relevance_retained_low_confidence",
            ));
            candidates.push(candidate);
        } else {
            issues.push(low_relevance_issue(&candidate, "candidate_low_relevance"));
        }
    }

    if candidates.is_empty() {
        match candidate_from_search_payload(query, payload) {
            Ok(candidate) => {
                if candidate_is_synthesis_eligible(query, &candidate, benchmark_intent) {
                    candidates.push(candidate);
                } else if let Some(candidate) =
                    retain_low_confidence_candidate(policy, &candidate, &mut retained_low_confidence)
                {
                    issues.push(low_relevance_issue(
                        &candidate,
                        "candidate_low_relevance_retained_low_confidence",
                    ));
                    candidates.push(candidate);
                } else {
                    issues.push(low_relevance_issue(&candidate, "candidate_low_relevance"));
                }
            }
            Err(err) => issues.push(format!("{stage}:{err}")),
        }
    }

    let summary = clean_text(
        payload.get("summary").and_then(Value::as_str).unwrap_or(""),
        2_400,
    );
    let content = clean_text(
        payload.get("content").and_then(Value::as_str).unwrap_or(""),
        2_400,
    );
    if contains_antibot_marker(&summary) || contains_antibot_marker(&content) {
        issues.push(format!("{stage}:anti_bot_challenge"));
    }
    if looks_like_competitive_programming_dump(&format!("{summary} {content}")) {
        issues.push(format!("{stage}:query_result_mismatch"));
    }
    if contains_web_junk_marker(&summary) || contains_web_junk_marker(&content) {
        issues.push(format!("{stage}:junk_page"));
    }
    let usable_candidate_count = candidates
        .iter()
        .filter(|candidate| !candidate_is_low_confidence_retained(candidate))
        .count();
    let trusted_primary_lane = is_official_source_query_lane(query);
    let should_fetch_links = page_extraction_enabled(policy)
        && page_extraction_max_links_per_stage(policy) > 0
        && fetch_budget.has_remaining()
        && (usable_candidate_count < page_extraction_min_usable_items_before_skip(policy)
            || looks_like_low_signal_search_summary(&summary)
            || candidates
                .iter()
                .any(|candidate| candidate_needs_link_fetch(query, policy, candidate)));
    if should_fetch_links {
        let include_substantive_candidates =
            usable_candidate_count < page_extraction_min_usable_items_before_skip(policy)
                || looks_like_low_signal_search_summary(&summary);
        let (links, prefetch_rejections) = links_for_page_extraction_with_rejections(
            query,
            policy,
            payload,
            &candidates,
            page_extraction_max_links_per_stage(policy),
            include_substantive_candidates,
        );
        for rejection in prefetch_rejections {
            issues.push(format!(
                "{stage}:page_extraction_candidate_prefetch_rejected:{rejection}"
            ));
        }
        for link in links {
            match fetch_budget.reserve(policy, &link, trusted_primary_lane) {
                PageExtractionFetchReservation::Reserved => {}
                PageExtractionFetchReservation::Duplicate => continue,
                PageExtractionFetchReservation::Exhausted => {
                    issues.push(format!("{stage}:page_extraction_global_budget_exhausted"));
                    break;
                }
            }
            let fetch_payload =
                stage_fetch_payload(root, stage, &link, &page_extraction_extract_mode(policy));
            if !fetch_payload
                .get("ok")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                let fetch_error = stage_error(&fetch_payload, "web_fetch_failed");
                issues.push(format!("{stage}:fetch:{fetch_error}"));
                if should_try_browser_materialization_for_fetch_error(&fetch_payload, &fetch_error)
                {
                    try_materialize_page_candidate(
                        root,
                        stage,
                        query,
                        policy,
                        &link,
                        benchmark_intent,
                        &mut candidates,
                        &mut issues,
                    );
                }
                continue;
            }
            match candidate_from_search_payload(query, &fetch_payload) {
                Ok(mut candidate) => {
                    if candidate.locator.is_empty()
                        || is_search_engine_domain(&candidate_domain_hint(&candidate))
                    {
                        candidate.locator = link.clone();
                    }
                    mark_candidate_as_page_enriched(&mut candidate);
                    if candidate_is_synthesis_eligible(query, &candidate, benchmark_intent) {
                        merge_or_push_page_enriched_candidate(query, policy, &mut candidates, candidate);
                    } else if citation_wrapper_link(&link)
                        && try_materialize_page_candidate(
                            root,
                            stage,
                            query,
                            policy,
                            &link,
                            benchmark_intent,
                            &mut candidates,
                            &mut issues,
                        )
                    {
                        continue;
                    } else if candidate_looks_like_relevant_discovery_hub(query, &candidate)
                        && try_expand_relevant_hub_payload(
                            root,
                            stage,
                            query,
                            policy,
                            &fetch_payload,
                            benchmark_intent,
                            fetch_budget,
                            &mut candidates,
                            &mut issues,
                            &mut retained_low_confidence,
                        ) > 0
                    {
                        continue;
                    } else if let Some(candidate) = retain_low_confidence_candidate(
                        policy,
                        &candidate,
                        &mut retained_low_confidence,
                    ) {
                        issues.push(low_relevance_issue(
                            &candidate,
                            "fetch_candidate_low_relevance_retained_low_confidence",
                        ));
                        candidates.push(candidate);
                    } else {
                        issues.push(low_relevance_issue(
                            &candidate,
                            "fetch_candidate_low_relevance",
                        ));
                    }
                }
                Err(err) => {
                    issues.push(format!("{stage}:fetch_candidate:{err}"));
                    if should_try_browser_materialization_for_candidate_error(
                        &fetch_payload,
                        &err,
                    ) || (citation_wrapper_link(&link) && err.contains("low_relevance"))
                    {
                        try_materialize_page_candidate(
                            root,
                            stage,
                            query,
                            policy,
                            &link,
                            benchmark_intent,
                            &mut candidates,
                            &mut issues,
                        );
                    }
                }
            }
        }
    }
    let synthesis_candidate_count = candidates
        .iter()
        .filter(|candidate| !candidate_is_low_confidence_retained(candidate))
        .count();
    let provider_result =
        hidden_provider_result_artifact(stage, query, payload, synthesis_candidate_count, &issues);
    (candidates, issues, provider_result)
}

fn mark_candidate_as_page_enriched(candidate: &mut Candidate) {
    if !candidate
        .source_kind
        .to_ascii_lowercase()
        .contains("page_enriched")
    {
        candidate.source_kind = format!("{}_page_enriched", candidate.source_kind);
    }
    let permissions = candidate.permissions.clone().unwrap_or_default();
    candidate.permissions = Some(if permissions.is_empty() {
        "public_web;page_enriched".to_string()
    } else if permissions.contains("page_enriched") {
        permissions
    } else {
        format!("{permissions};page_enriched")
    });
}

fn stage_browser_materialization_payload(
    root: &Path,
    stage: &str,
    url: &str,
    policy: &Value,
    reason: &str,
) -> Value {
    crate::web_conduit::api_browser_materialize_page(
        root,
        &json!({
            "url": url,
            "admission_ref": "batch_query_page_extraction_browser_materialization",
            "extract_mode": page_extraction_extract_mode(policy),
            "timeout_ms": page_extraction_browser_materialization_timeout_ms(policy),
            "max_response_bytes": page_extraction_browser_materialization_max_response_bytes(policy),
            "summary_only": false,
            "evidence_gap_reason": reason
        }),
    )
    .as_object()
    .map(|map| {
        let mut map = map.clone();
        map.insert("batch_query_stage".to_string(), json!(stage));
        Value::Object(map)
    })
    .unwrap_or_else(|| {
        json!({
            "ok": false,
            "error": "browser_materialization_non_object_payload",
            "batch_query_stage": stage
        })
    })
}

fn should_try_browser_materialization_for_fetch_error(
    fetch_payload: &Value,
    fetch_error: &str,
) -> bool {
    let lowered = clean_text(fetch_error, 240).to_ascii_lowercase();
    payload_access_blocker_class(fetch_payload).is_some()
        || issue_is_access_or_throttle_failure(&lowered)
        || lowered.contains("no_usable_summary")
        || lowered.contains("low_signal")
        || lowered.contains("web_conduit_policy_denied")
        || lowered.contains("policy_denied")
}

fn should_try_browser_materialization_for_candidate_error(
    fetch_payload: &Value,
    err: &str,
) -> bool {
    let lowered = clean_text(err, 240).to_ascii_lowercase();
    payload_access_blocker_class(fetch_payload).is_some()
        || issue_is_access_or_throttle_failure(&lowered)
        || lowered.contains("no_usable_summary")
        || lowered.contains("low_signal")
        || lowered.contains("content_too_thin")
}

fn candidate_from_browser_materialization_payload(payload: &Value) -> Result<Candidate, String> {
    if !payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        return Err(clean_text(
            payload
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("browser_materialization_failed"),
            220,
        ));
    }
    let candidate = payload
        .get("evidence_candidate")
        .or_else(|| {
            payload
                .get("evidence_pack_candidates")
                .and_then(Value::as_array)
                .and_then(|rows| rows.first())
        })
        .ok_or_else(|| "browser_materialization_missing_evidence_candidate".to_string())?;
    let decision = candidate
        .pointer("/promotion/decision")
        .and_then(Value::as_str)
        .unwrap_or("");
    let confidence = candidate
        .get("confidence")
        .and_then(Value::as_str)
        .unwrap_or("");
    if decision != "candidate_ready_for_packaging" || confidence != "usable" {
        return Err(format!(
            "browser_materialization_not_promotable:{decision}:{confidence}"
        ));
    }
    let snippet = clean_text(
        candidate.get("snippet").and_then(Value::as_str).unwrap_or(""),
        6_000,
    );
    if snippet.is_empty() || looks_like_low_signal_search_summary(&snippet) {
        return Err("browser_materialization_no_usable_summary".to_string());
    }
    let title = clean_text(
        candidate.get("title").and_then(Value::as_str).unwrap_or(""),
        220,
    );
    let locator = clean_text(
        candidate.get("locator").and_then(Value::as_str).unwrap_or(""),
        2_200,
    );
    if locator.is_empty() {
        return Err("browser_materialization_missing_locator".to_string());
    }
    let excerpt_hash = candidate
        .get("excerpt_hash")
        .and_then(Value::as_str)
        .map(|row| clean_text(row, 128))
        .filter(|row| !row.is_empty())
        .unwrap_or_else(|| sha256_hex(&snippet));
    Ok(Candidate {
        source_kind: clean_text(
            candidate
                .get("source_kind")
                .and_then(Value::as_str)
                .unwrap_or("browser_materialized_page"),
            80,
        ),
        title: if title.is_empty() {
            let domain = extract_domains_from_text(&locator, 1)
                .into_iter()
                .next()
                .unwrap_or_else(|| "source".to_string());
            format!("Browser materialized page from {}", clean_text(&domain, 120))
        } else {
            title
        },
        locator,
        snippet: snippet.clone(),
        excerpt_hash,
        timestamp: candidate
            .get("timestamp")
            .and_then(Value::as_str)
            .map(|row| clean_text(row, 80))
            .filter(|row| !row.is_empty())
            .or_else(|| Some(crate::now_iso())),
        permissions: candidate
            .get("permissions")
            .and_then(Value::as_str)
            .map(|row| clean_text(row, 120))
            .filter(|row| !row.is_empty())
            .or_else(|| Some("public_web;browser_materialized".to_string())),
        status_code: payload
            .pointer("/materialized_page/status_code")
            .and_then(Value::as_i64)
            .unwrap_or(200),
    })
}

fn try_materialize_page_candidate(
    root: &Path,
    stage: &str,
    query: &str,
    policy: &Value,
    link: &str,
    benchmark_intent: bool,
    candidates: &mut Vec<Candidate>,
    issues: &mut Vec<String>,
) -> bool {
    if !page_extraction_browser_materialization_enabled(policy) {
        issues.push(format!("{stage}:browser_materialization_disabled_by_policy"));
        return false;
    }
    issues.push(format!("{stage}:browser_materialization_attempted"));
    let payload = stage_browser_materialization_payload(
        root,
        stage,
        link,
        policy,
        "static_fetch_unusable",
    );
    match candidate_from_browser_materialization_payload(&payload) {
        Ok(mut candidate) => {
            mark_candidate_as_page_enriched(&mut candidate);
            if candidate_is_synthesis_eligible(query, &candidate, benchmark_intent) {
                merge_or_push_page_enriched_candidate(query, policy, candidates, candidate);
                issues.push(format!("{stage}:browser_materialization_recovered"));
                true
            } else {
                issues.push(format!(
                    "{stage}:browser_materialization_candidate_low_relevance"
                ));
                false
            }
        }
        Err(err) => {
            issues.push(format!("{stage}:browser_materialization:{err}"));
            false
        }
    }
}

fn page_enriched_candidate_value(query: &str, candidate: &Candidate) -> usize {
    let snippet = clean_text(&candidate.snippet, 2_400);
    let word_count = snippet.split_whitespace().count().min(220);
    let overlap = query_overlap_terms(query, candidate);
    let status_bonus = if (200..400).contains(&candidate.status_code) {
        12
    } else {
        0
    };
    let substance_bonus = if !looks_like_low_signal_search_summary(&snippet)
        && !looks_like_source_only_snippet(&snippet)
        && !looks_like_ack_only(&snippet)
    {
        24
    } else {
        0
    };
    word_count + overlap.saturating_mul(18) + status_bonus + substance_bonus
}

fn candidates_share_locator(a: &Candidate, b: &Candidate) -> bool {
    let left = clean_text(&a.locator, 2_200).to_ascii_lowercase();
    let right = clean_text(&b.locator, 2_200).to_ascii_lowercase();
    !left.is_empty() && left == right
}

fn should_replace_with_page_enriched_candidate(
    query: &str,
    policy: &Value,
    existing: &Candidate,
    enriched: &Candidate,
) -> bool {
    if !candidates_share_locator(existing, enriched) {
        return false;
    }
    if candidate_needs_link_fetch(query, policy, existing) {
        return true;
    }
    page_enriched_candidate_value(query, enriched)
        > page_enriched_candidate_value(query, existing).saturating_add(8)
}

fn merge_or_push_page_enriched_candidate(
    query: &str,
    policy: &Value,
    candidates: &mut Vec<Candidate>,
    enriched: Candidate,
) {
    if let Some(existing) = candidates.iter_mut().find(|candidate| {
        should_replace_with_page_enriched_candidate(query, policy, candidate, &enriched)
    }) {
        *existing = enriched;
        return;
    }
    candidates.push(enriched);
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

fn retain_low_confidence_candidate(
    policy: &Value,
    candidate: &Candidate,
    retained_count: &mut usize,
) -> Option<Candidate> {
    if !low_confidence_retention_enabled(policy) {
        return None;
    }
    let max_retained = policy
        .pointer("/batch_query/result_retention/max_low_confidence_items")
        .and_then(Value::as_u64)
        .unwrap_or(6)
        .clamp(1, 24) as usize;
    if *retained_count >= max_retained {
        return None;
    }
    let domain = candidate_domain_hint(candidate);
    if candidate.locator.is_empty()
        || is_search_engine_domain(&domain)
        || looks_like_low_signal_search_summary(&candidate.snippet)
        || contains_web_junk_marker(&candidate.snippet)
        || looks_like_ack_only(&candidate.snippet)
    {
        return None;
    }
    let mut candidate = candidate.clone();
    if !candidate.source_kind.contains("low_confidence") {
        candidate.source_kind = format!("{}_low_confidence_raw", candidate.source_kind);
    }
    let permissions = candidate.permissions.clone().unwrap_or_default();
    candidate.permissions = Some(if permissions.is_empty() {
        "public_web;low_confidence_raw".to_string()
    } else if permissions.contains("low_confidence_raw") {
        permissions
    } else {
        format!("{permissions};low_confidence_raw")
    });
    *retained_count += 1;
    Some(candidate)
}

fn has_usable_synthesis_candidate(candidates: &[Candidate]) -> bool {
    candidates
        .iter()
        .any(|candidate| !candidate_is_low_confidence_retained(candidate))
}

fn candidate_source_kind_is_fallback_search_surface(candidate: &Candidate) -> bool {
    let source = candidate.source_kind.to_ascii_lowercase();
    matches!(
        source.as_str(),
        "google_news_rss" | "bing_rss" | "duckduckgo" | "duckduckgo_lite" | "direct_http"
    ) || source.contains("_rss")
        || source.contains("headline_feed")
}

fn candidate_can_block_provider_recovery(
    query: &str,
    candidate: &Candidate,
    benchmark_intent: bool,
) -> bool {
    if candidate_is_low_confidence_retained(candidate)
        || candidate_source_kind_is_fallback_search_surface(candidate)
    {
        return false;
    }
    if candidate_has_non_evidence_payload(candidate) {
        return false;
    }
    if candidate
        .permissions
        .as_deref()
        .unwrap_or("")
        .to_ascii_lowercase()
        .contains("structured_feed")
    {
        return true;
    }
    if benchmark_intent
        && !content_rich_text(&format!("{} {}", candidate.title, candidate.snippet))
        && !looks_like_metric_rich_text(&candidate.snippet)
        && !candidate_has_trusted_primary_source_signal(query, candidate)
    {
        return false;
    }
    candidate_is_synthesis_eligible(query, candidate, benchmark_intent)
}

fn provider_recovery_required_entity_facets(query: &str) -> Vec<ResearchFacet> {
    if !is_benchmark_or_comparison_intent(query) {
        return Vec::new();
    }
    let mut entities = aperture_budget("medium")
        .and_then(|budget| inferred_comparison_query_pack(query, budget))
        .map(|pack| pack.entities)
        .unwrap_or_default();
    if entities.len() < 2 {
        entities = comparison_entities_from_query(query);
    }
    let mut seen = HashSet::<String>::new();
    let mut facets = Vec::<ResearchFacet>::new();
    for entity in entities {
        let cleaned = clean_text(&entity, 120);
        if cleaned.is_empty() || !seen.insert(cleaned.to_ascii_lowercase()) {
            continue;
        }
        if let Some(mut facet) = research_facet_from_metadata_text(&cleaned, facets.len(), "entity") {
            facet.id = format!("provider_recovery_entity_{:02}", facets.len() + 1);
            facets.push(facet);
        }
    }
    assign_distinctive_facet_terms(&mut facets);
    facets
}

fn provider_recovery_covered_entity_count(
    query: &str,
    candidates: &[Candidate],
    benchmark_intent: bool,
    entity_facets: &[ResearchFacet],
) -> usize {
    if entity_facets.is_empty() {
        return 0;
    }
    let mut covered = HashSet::<String>::new();
    for candidate in candidates {
        if !candidate_can_block_provider_recovery(query, candidate, benchmark_intent) {
            continue;
        }
        for facet in entity_facets {
            if candidate_matches_facet(facet, candidate, 1) {
                covered.insert(facet.id.clone());
            }
        }
    }
    covered.len()
}

fn provider_recovery_satisfied(query: &str, candidates: &[Candidate], benchmark_intent: bool) -> bool {
    let min_usable = if benchmark_intent { 2 } else { 1 };
    let min_domains = if benchmark_intent { 2 } else { 1 };
    let mut usable = 0usize;
    let mut domains = HashSet::<String>::new();
    for candidate in candidates {
        if !candidate_can_block_provider_recovery(query, candidate, benchmark_intent) {
            continue;
        }
        usable += 1;
        let domain = candidate_domain_hint(candidate).to_ascii_lowercase();
        if !domain.is_empty() && domain != "source" && !is_search_engine_domain(&domain) {
            domains.insert(domain);
        }
    }
    if usable < min_usable || domains.len() < min_domains {
        return false;
    }
    let entity_facets = provider_recovery_required_entity_facets(query);
    if entity_facets.len() >= 2
        && provider_recovery_covered_entity_count(query, candidates, benchmark_intent, &entity_facets)
            < entity_facets.len()
    {
        return false;
    }
    true
}

fn hidden_provider_result_quality(
    payload_ok: bool,
    candidate_count: usize,
    issues: &[String],
) -> &'static str {
    if candidate_count > 0 {
        return "usable";
    }
    if issues.iter().any(|issue| issue_is_access_or_throttle_failure(issue)) {
        return "blocked_or_throttled";
    }
    if !payload_ok {
        return "provider_error";
    }
    if issues.iter().any(|issue| {
        issue.contains("candidate_low_relevance")
            || issue.contains("fetch_candidate_low_relevance")
            || issue.contains("query_result_mismatch")
    }) {
        return "low_relevance";
    }
    if issues
        .iter()
        .any(|issue| issue.contains("no_usable_summary") || issue.contains("low_signal"))
    {
        return "low_signal";
    }
    "no_synthesis_candidate"
}

fn hidden_provider_result_artifact(
    stage: &str,
    query: &str,
    payload: &Value,
    candidate_count: usize,
    issues: &[String],
) -> Option<Value> {
    let query_text = clean_text(query, 600);
    let stage_name = clean_text(stage, 80);
    let provider = clean_text(
        payload
            .get("provider")
            .or_else(|| payload.get("source"))
            .and_then(Value::as_str)
            .unwrap_or(stage),
        80,
    );
    let summary = clean_text(
        payload.get("summary").and_then(Value::as_str).unwrap_or(""),
        1_200,
    );
    let content_preview = trim_words(
        &clean_text(
            payload.get("content").and_then(Value::as_str).unwrap_or(""),
            1_600,
        ),
        48,
    );
    let locator = clean_text(
        payload.get("requested_url").and_then(Value::as_str).unwrap_or(""),
        2_200,
    );
    let payload_ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(false)
        || payload
            .get("provider_payload_rejected")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    let error = hidden_provider_error(payload, !(payload_ok && candidate_count > 0));
    let links = payload_links_for_fallback(query, payload, 3);
    let result_quality = hidden_provider_result_quality(payload_ok, candidate_count, issues);
    let ok = result_quality == "usable";
    let status = clean_text(
        payload
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or(if payload_ok { "ok" } else { "error" }),
        32,
    );
    let mut out = serde_json::Map::new();
    if !query_text.is_empty() {
        out.insert("query".to_string(), Value::String(query_text));
    }
    if !stage_name.is_empty() {
        out.insert("stage".to_string(), Value::String(stage_name));
    }
    if !provider.is_empty() {
        out.insert("provider".to_string(), Value::String(provider));
    }
    out.insert("ok".to_string(), Value::Bool(ok));
    out.insert("provider_transport_ok".to_string(), Value::Bool(payload_ok));
    out.insert(
        "result_quality".to_string(),
        Value::String(result_quality.to_string()),
    );
    out.insert(
        "synthesis_candidate_count".to_string(),
        json!(candidate_count),
    );
    let provider_raw_count = hidden_provider_raw_count(payload, links.len(), &summary, &content_preview, &locator, &error);
    out.insert(
        "provider_raw_count".to_string(),
        json!(provider_raw_count),
    );
    out.insert(
        "provider_filtered_count".to_string(),
        json!(issues.len()),
    );
    if !issues.is_empty() {
        out.insert(
            "failure_reasons".to_string(),
            Value::Array(
                issues
                    .iter()
                    .map(|issue| Value::String(clean_text(issue, 180)))
                    .collect::<Vec<_>>(),
            ),
        );
    }
    if !status.is_empty() {
        out.insert("status".to_string(), Value::String(status));
    }
    if !summary.is_empty() {
        out.insert("summary".to_string(), Value::String(summary));
    }
    if !content_preview.is_empty() && content_preview != out.get("summary").and_then(Value::as_str).unwrap_or("") {
        out.insert(
            "content_preview".to_string(),
            Value::String(content_preview),
        );
    }
    if !locator.is_empty() {
        out.insert("locator".to_string(), Value::String(locator));
    }
    if !error.is_empty() {
        out.insert("error".to_string(), Value::String(error));
    } else if payload_ok && !ok {
        out.insert(
            "error".to_string(),
            Value::String(result_quality.to_string()),
        );
    }
    if !links.is_empty() {
        out.insert(
            "links".to_string(),
            Value::Array(links.into_iter().map(Value::String).collect::<Vec<_>>()),
        );
    }
    Some(Value::Object(out))
}

fn hidden_provider_error(payload: &Value, include_recovered_provider_errors: bool) -> String {
    let mut pointers = vec![
        "/error",
        "/retry/reason",
        "/policy_decision/reason",
        "/tool_execution_gate/reason",
        "/result/error",
    ];
    if include_recovered_provider_errors {
        pointers.extend(["/provider_errors/0/error", "/provider_errors/0/reason"]);
    }
    for pointer in pointers {
        let value = clean_text(payload.pointer(pointer).and_then(Value::as_str).unwrap_or(""), 240);
        if !value.is_empty() {
            return value;
        }
    }
    String::new()
}

fn hidden_provider_array_count(value: &Value, depth: usize) -> usize {
    if depth > 5 {
        return 0;
    }
    match value {
        Value::Array(rows) => rows.len(),
        Value::Object(map) => map
            .iter()
            .filter(|(key, _)| {
                matches!(
                    key.to_ascii_lowercase().as_str(),
                    "web"
                        | "news"
                        | "results"
                        | "items"
                        | "organic"
                        | "documents"
                        | "data"
                        | "links"
                        | "sources"
                )
            })
            .map(|(_, row)| hidden_provider_array_count(row, depth + 1))
            .sum(),
        _ => 0,
    }
}

fn hidden_provider_raw_count(
    payload: &Value,
    fallback_link_count: usize,
    summary: &str,
    content_preview: &str,
    locator: &str,
    error: &str,
) -> usize {
    let structured_count = hidden_provider_array_count(payload, 0);
    if structured_count > 0 {
        return structured_count;
    }
    if fallback_link_count > 0 {
        return fallback_link_count;
    }
    if !summary.is_empty() || !content_preview.is_empty() || !locator.is_empty() || !error.is_empty() {
        return 1;
    }
    0
}

fn retrieve_web_candidates_for_query(
    root: &Path,
    query: &str,
    policy: &Value,
    search_scope: &BatchQuerySearchScope,
    fetch_budget: PageExtractionFetchBudget,
) -> (Vec<Candidate>, Vec<String>, Vec<Value>) {
    let benchmark_intent = is_benchmark_or_comparison_intent(query);
    let mut candidates = Vec::<Candidate>::new();
    let mut issues = Vec::<String>::new();
    let mut provider_results = Vec::<Value>::new();

    let primary_payload = stage_search_payload(root, None, query, None, policy, search_scope);
    let (primary_candidates, primary_issues, primary_provider_result) =
        collect_candidates_from_stage_payload(
        root,
        "primary",
        query,
        policy,
        &primary_payload,
        benchmark_intent,
        &fetch_budget,
    );
    if let Some(value) = primary_provider_result {
        provider_results.push(value);
    }
    candidates.extend(primary_candidates);
    issues.extend(primary_issues);

    if !has_usable_synthesis_candidate(&candidates)
        && issues
            .iter()
            .any(|issue| skip_duckduckgo_fallback_for_error(issue))
    {
        return (Vec::new(), issues, provider_results);
    }

    if !has_usable_synthesis_candidate(&candidates) && !is_official_source_query_lane(query) {
        let bing_payload =
            stage_search_payload(root, Some("bing_rss"), query, Some("bing"), policy, search_scope);
        let (bing_candidates, bing_issues, bing_provider_result) =
            collect_candidates_from_stage_payload(
            root,
            "bing_rss",
            query,
            policy,
            &bing_payload,
            benchmark_intent,
            &fetch_budget,
        );
        if let Some(value) = bing_provider_result {
            provider_results.push(value);
        }
        candidates.extend(bing_candidates);
        issues.extend(bing_issues);
    }

    if !has_usable_synthesis_candidate(&candidates) {
        let fallback_url = duckduckgo_instant_answer_url(query);
        let fallback_payload =
            if let Some(payload) = fixture_payload_for_stage_query("duckduckgo_instant", query) {
                payload
            } else if fixture_mode_enabled() {
                fixture_missing_payload()
            } else {
                stage_fetch_payload(root, "duckduckgo_instant", &fallback_url, "text")
            };
        let mut duckduckgo_candidate_count = 0usize;
        let mut duckduckgo_issues = Vec::<String>::new();
        match candidate_from_duckduckgo_instant_payload(query, &fallback_url, &fallback_payload) {
            Ok(candidate) => {
                if candidate_is_synthesis_eligible(query, &candidate, benchmark_intent) {
                    duckduckgo_candidate_count = 1;
                    candidates.push(candidate);
                } else {
                    let mut retained_count = 0usize;
                    if let Some(candidate) =
                        retain_low_confidence_candidate(policy, &candidate, &mut retained_count)
                    {
                        duckduckgo_issues.push(
                            "duckduckgo_instant:candidate_low_relevance_retained_low_confidence"
                                .to_string(),
                        );
                        candidates.push(candidate);
                    } else {
                        duckduckgo_issues
                            .push("duckduckgo_instant:candidate_low_relevance".to_string());
                    }
                }
            }
            Err(err) => duckduckgo_issues.push(format!("duckduckgo_instant:{err}")),
        }
        if let Some(value) = hidden_provider_result_artifact(
            "duckduckgo_instant",
            query,
            &fallback_payload,
            duckduckgo_candidate_count,
            &duckduckgo_issues,
        ) {
            provider_results.push(value);
        }
        issues.extend(duckduckgo_issues);
    }

    if !provider_recovery_satisfied(query, &candidates, benchmark_intent) {
        let mut attempted_providers = provider_results
            .iter()
            .filter_map(|row| row.get("provider").and_then(Value::as_str))
            .map(|provider| provider.to_ascii_lowercase())
            .collect::<HashSet<_>>();
        for provider in provider_recovery_providers(policy, query) {
            if attempted_providers.contains(&provider) {
                continue;
            }
            attempted_providers.insert(provider.clone());
            let provider_payload =
                stage_search_payload(root, Some(&provider), query, Some(&provider), policy, search_scope);
            let (mut provider_candidates, provider_issues, provider_result) =
                collect_candidates_from_stage_payload(
                    root,
                    &provider,
                    query,
                    policy,
                    &provider_payload,
                    benchmark_intent,
                    &fetch_budget,
                );
            if let Some(value) = provider_result {
                provider_results.push(value);
            }
            issues.extend(provider_issues);
            if has_usable_synthesis_candidate(&provider_candidates) {
                candidates.append(&mut provider_candidates);
                if provider_recovery_satisfied(query, &candidates, benchmark_intent) {
                    break;
                }
            } else if !provider_candidates.is_empty() {
                candidates.append(&mut provider_candidates);
            }
        }
    }

    if is_official_source_query_lane(query)
        && !provider_recovery_satisfied(query, &candidates, benchmark_intent)
    {
        let (mut probe_candidates, probe_issues, probe_results) =
            recover_official_domain_probe_candidates(
                root,
                query,
                policy,
                benchmark_intent,
                &fetch_budget,
            );
        if !probe_candidates.is_empty() {
            candidates.append(&mut probe_candidates);
        }
        issues.extend(probe_issues);
        provider_results.extend(probe_results);
    }

    if candidates.is_empty() {
        if issues.is_empty() {
            issues.push("no_usable_summary".to_string());
        }
        (Vec::new(), issues, provider_results)
    } else {
        let mut dedup = HashSet::<String>::new();
        let mut unique = Vec::<Candidate>::new();
        for candidate in candidates {
            let key = format!(
                "{}|{}|{}",
                candidate.locator.to_ascii_lowercase(),
                candidate.title.to_ascii_lowercase(),
                candidate.excerpt_hash
            );
            if dedup.insert(key) {
                unique.push(candidate);
            }
        }
        (unique, issues, provider_results)
    }
}
