const INTERNAL_ROUTE_HINT: &str =
    "This looks like an internal command mapping request, not a web search query. Use local route diagnostics instead of web retrieval.";

fn fixture_payload_for_stage_url(stage: &str, url: &str) -> Option<Value> {
    let fixtures = fixture_payload_map()?;
    let stage_key = format!("{stage}::{url}");
    fixtures
        .get(&stage_key)
        .cloned()
        .or_else(|| fixtures.get(&format!("fetch::{url}")).cloned())
        .or_else(|| fixtures.get(&format!("url::{url}")).cloned())
}

fn fixture_mode_enabled() -> bool {
    std::env::var("INFRING_BATCH_QUERY_TEST_FIXTURE_JSON")
        .map(|raw| !raw.trim().is_empty())
        .unwrap_or(false)
}

fn fixture_missing_payload() -> Value {
    json!({
        "ok": false,
        "error": "fixture_missing"
    })
}

fn stage_search_payload(
    root: &Path,
    stage: Option<&str>,
    query: &str,
    provider: Option<&str>,
    policy: &Value,
    search_scope: &BatchQuerySearchScope,
) -> Value {
    if let Some(stage_name) = stage {
        if let Some(payload) = fixture_payload_for_stage_query(stage_name, query) {
            return payload;
        }
    } else if let Some(payload) = fixture_payload_for_query(query) {
        return payload;
    }
    if fixture_mode_enabled() {
        return fixture_missing_payload();
    }
    let request = stage_search_request(query, provider, policy, search_scope);
    crate::web_conduit::api_search(root, &request)
}

fn stage_search_request(
    query: &str,
    provider: Option<&str>,
    policy: &Value,
    search_scope: &BatchQuerySearchScope,
) -> Value {
    let trusted_official_lane = is_official_source_query_lane(query);
    let mut request = json!({
        "query": query,
        "summary_only": false
    });
    if std::env::var(CACHE_MODE_ENV)
        .ok()
        .map(|raw| normalize_cache_mode(&raw) == "disabled")
        .unwrap_or(false)
    {
        request["cache"] = json!(false);
    }
    if let Some(provider_name) = provider {
        request["provider"] = Value::String(provider_name.to_string());
    }
    if let Some(provider_chain) = lane_aware_search_provider_chain(query, policy) {
        request["search_provider_chain"] = json!(provider_chain);
        request["search_provider_chain_strict"] = json!(false);
    }
    if !search_scope.allowed_domains.is_empty() {
        request["allowed_domains"] = json!(search_scope.allowed_domains.clone());
        request["exclude_subdomains"] = json!(search_scope.exclude_subdomains);
    }
    request
}

fn lane_aware_search_provider_chain(query: &str, policy: &Value) -> Option<Vec<String>> {
    if !is_official_source_query_lane(query) {
        return None;
    }
    let base =
        crate::web_conduit_provider_runtime::resolved_search_provider_chain("", &json!({}), policy);
    let filtered = base
        .iter()
        .filter(|provider| !matches!(provider.as_str(), "google_news_rss" | "bing_rss"))
        .cloned()
        .collect::<Vec<_>>();
    if filtered.is_empty() {
        Some(vec![
            "tavily".to_string(),
            "exa".to_string(),
            "brave".to_string(),
            "serperdev".to_string(),
            "duckduckgo_lite".to_string(),
            "duckduckgo".to_string(),
        ])
    } else {
        Some(filtered)
    }
}

fn stage_fetch_payload(root: &Path, stage: &str, url: &str, extract_mode: &str) -> Value {
    if let Some(payload) = fixture_payload_for_stage_url(stage, url) {
        return payload;
    }
    if fixture_mode_enabled() {
        return fixture_missing_payload();
    }
    let fetch_payload = crate::web_conduit::api_fetch(
        root,
        &json!({
            "url": url,
            "extract_mode": extract_mode,
            "summary_only": false
        }),
    );
    document_lane_fetch_payload(root, url, extract_mode, &fetch_payload).unwrap_or(fetch_payload)
}

fn fetch_payload_error(fetch_payload: &Value) -> String {
    clean_text(
        fetch_payload
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or(""),
        240,
    )
    .to_ascii_lowercase()
}

fn fetch_payload_content_type(fetch_payload: &Value) -> String {
    clean_text(
        fetch_payload
            .get("content_type")
            .or_else(|| fetch_payload.get("contentType"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        160,
    )
    .to_ascii_lowercase()
    .split(';')
    .next()
    .unwrap_or("")
    .trim()
    .to_string()
}

fn fetch_payload_is_pdf_document_lane_candidate(fetch_payload: &Value) -> bool {
    let error = fetch_payload_error(fetch_payload);
    let content_type = fetch_payload_content_type(fetch_payload);
    error == "unsupported_content_type:application/pdf" || content_type == "application/pdf"
}

fn document_lane_fetch_payload(
    root: &Path,
    url: &str,
    extract_mode: &str,
    fetch_payload: &Value,
) -> Option<Value> {
    if !fetch_payload_is_pdf_document_lane_candidate(fetch_payload) {
        return None;
    }
    let pdf_payload = crate::web_conduit::api_pdf_extract(
        root,
        &json!({
            "url": url,
            "summary_only": false,
            "max_pages": 5,
            "min_text_chars": 0
        }),
    );
    document_lane_fetch_payload_from_pdf_extract(url, extract_mode, fetch_payload, &pdf_payload)
}

fn document_lane_fetch_payload_from_pdf_extract(
    url: &str,
    extract_mode: &str,
    fetch_payload: &Value,
    pdf_payload: &Value,
) -> Option<Value> {
    if !fetch_payload_is_pdf_document_lane_candidate(fetch_payload) {
        return None;
    }
    let requested_url = clean_text(
        fetch_payload
            .get("requested_url")
            .or_else(|| fetch_payload.get("final_url"))
            .and_then(Value::as_str)
            .unwrap_or(url),
        2_200,
    );
    let resolved_url = clean_text(
        pdf_payload
            .get("resolved_source")
            .or_else(|| fetch_payload.get("resolved_url"))
            .or_else(|| fetch_payload.get("final_url"))
            .and_then(Value::as_str)
            .unwrap_or(requested_url.as_str()),
        2_200,
    );
    let status_code = fetch_payload
        .get("status_code")
        .and_then(Value::as_i64)
        .unwrap_or(200);
    if !pdf_payload
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        let mut out = fetch_payload.clone();
        if let Some(obj) = out.as_object_mut() {
            obj.insert("document_lane_attempted".to_string(), json!(true));
            obj.insert("document_type".to_string(), json!("pdf"));
            obj.insert(
                "document_lane_error".to_string(),
                pdf_payload
                    .get("error")
                    .cloned()
                    .unwrap_or_else(|| json!("pdf_extract_failed")),
            );
        }
        return Some(out);
    }
    let text = clean_text(
        pdf_payload
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or(""),
        6_000,
    );
    if text.is_empty() {
        let mut out = fetch_payload.clone();
        if let Some(obj) = out.as_object_mut() {
            obj.insert("document_lane_attempted".to_string(), json!(true));
            obj.insert("document_type".to_string(), json!("pdf"));
            obj.insert(
                "document_lane_error".to_string(),
                json!("pdf_extract_empty_text"),
            );
        }
        return Some(out);
    }
    let page_count = pdf_payload
        .get("page_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let text_chars = pdf_payload
        .get("text_chars")
        .and_then(Value::as_u64)
        .unwrap_or_else(|| text.chars().count() as u64);
    let summary = if let Some(summary) = pdf_payload
        .get("summary")
        .and_then(Value::as_str)
        .map(|row| clean_text(row, 600))
        .filter(|row| !row.is_empty())
    {
        summary
    } else {
        format!("Extracted {text_chars} characters from PDF document.")
    };
    Some(json!({
        "ok": true,
        "type": "web_conduit_fetch_document_lane",
        "requested_url": requested_url,
        "resolved_url": resolved_url,
        "final_url": resolved_url,
        "provider": "web_media_pdf_extract",
        "source_kind": "document_page_artifact",
        "document_type": "pdf",
        "extract_mode": extract_mode,
        "extractor": "pdf_text",
        "status_code": status_code,
        "content_type": "application/pdf",
        "summary": summary,
        "content": text,
        "text_chars": text_chars,
        "page_count": page_count,
        "page_numbers": pdf_payload.get("page_numbers").cloned().unwrap_or_else(|| json!([])),
        "document_lane_attempted": true,
        "permissions": "public_web;document_lane",
        "external_content": {
            "untrusted": true,
            "source": "web_media_pdf_extract",
            "wrapped": false
        }
    }))
}

fn payload_links_for_fallback(query: &str, payload: &Value, max_links: usize) -> Vec<String> {
    ranked_payload_links_for_fallback(query, payload, max_links)
}

fn payload_links_for_page_extraction(
    query: &str,
    policy: &Value,
    payload: &Value,
    max_links: usize,
) -> Vec<String> {
    payload_links_for_page_extraction_with_rejections(query, policy, payload, max_links).0
}

fn payload_links_for_page_extraction_with_rejections(
    query: &str,
    policy: &Value,
    payload: &Value,
    max_links: usize,
) -> (Vec<String>, Vec<String>) {
    let (ranked, rejections) =
        ranked_payload_links_for_page_extraction_with_rejections(query, policy, payload, max_links);
    (
        select_ranked_page_extraction_links(policy, ranked, max_links.max(1)),
        rejections,
    )
}

fn ranked_payload_links_for_page_extraction_with_rejections(
    query: &str,
    policy: &Value,
    payload: &Value,
    max_links: usize,
) -> (Vec<(String, f64)>, Vec<String>) {
    let mut ranked = Vec::<(String, f64)>::new();
    let mut rejections = Vec::<String>::new();
    for (link, context) in ranked_payload_links_for_fallback_with_context_and_min_score(
        query,
        payload,
        max_links.saturating_mul(4).max(max_links),
        page_extraction_min_link_score(policy),
    )
    .into_iter()
    {
        let Some(link) = normalize_page_extraction_link(policy, &link) else {
            continue;
        };
        let candidate = page_extraction_link_candidate_with_context(&link, &context);
        if let Some(reason) =
            page_extraction_link_preflight_rejection_reason_with_context(query, &link, &context)
        {
            rejections.push(reason.to_string());
            continue;
        }
        let mut score = fallback_link_score_with_context(query, &link, &context);
        if candidate_has_trusted_primary_source_signal(query, &candidate) {
            score += 1.0;
        }
        if candidate_has_trusted_official_source_signal(query, &candidate) {
            score += 0.12;
        }
        if citation_wrapper_link(&link) && citation_wrapper_context_has_signal(query, &context) {
            score += 0.08;
        }
        ranked.push((link, score));
    }
    ranked.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });
    (ranked, rejections)
}

fn candidate_locator_links_for_page_extraction(
    query: &str,
    policy: &Value,
    candidates: &[Candidate],
    max_links: usize,
    include_substantive_candidates: bool,
) -> Vec<String> {
    candidate_locator_links_for_page_extraction_with_rejections(
        query,
        policy,
        candidates,
        max_links,
        include_substantive_candidates,
    )
    .0
}

fn candidate_locator_links_for_page_extraction_with_rejections(
    query: &str,
    policy: &Value,
    candidates: &[Candidate],
    max_links: usize,
    include_substantive_candidates: bool,
) -> (Vec<String>, Vec<String>) {
    let (ranked, rejections) = ranked_candidate_locator_links_for_page_extraction_with_rejections(
        query,
        policy,
        candidates,
        max_links,
        include_substantive_candidates,
    );
    (
        select_ranked_page_extraction_links(policy, ranked, max_links),
        rejections,
    )
}

fn ranked_candidate_locator_links_for_page_extraction_with_rejections(
    query: &str,
    policy: &Value,
    candidates: &[Candidate],
    max_links: usize,
    include_substantive_candidates: bool,
) -> (Vec<(String, f64)>, Vec<String>) {
    if !page_extraction_candidate_locator_followup_enabled(policy) || max_links == 0 {
        return (Vec::new(), Vec::new());
    }
    let mut rejections = Vec::<String>::new();
    let mut ranked = candidates
        .iter()
        .filter_map(|candidate| {
            let needs_fetch = candidate_needs_link_fetch(query, policy, candidate);
            if !needs_fetch && !include_substantive_candidates {
                return None;
            }
            let link = normalize_page_extraction_link(policy, &candidate.locator)?;
            let context = clean_text(
                &format!("{} {}", candidate.title, candidate.snippet),
                1_800,
            );
            if let Some(reason) =
                page_extraction_link_preflight_rejection_reason_with_context(query, &link, &context)
            {
                rejections.push(reason.to_string());
                return None;
            }
            let mut score =
                fallback_link_score_with_context(query, &link, &context)
                    + rerank_score(query, candidate) * 0.35;
            if needs_fetch {
                score += 0.24;
            }
            if candidate_is_low_confidence_retained(candidate) {
                score += 0.08;
            }
            if candidate_has_trusted_primary_source_signal(query, candidate) {
                score += 1.0;
            }
            if candidate_has_trusted_official_source_signal(query, candidate) {
                score += 0.12;
            }
            Some((link, score))
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });
    (ranked, rejections)
}

fn links_for_page_extraction(
    query: &str,
    policy: &Value,
    payload: &Value,
    candidates: &[Candidate],
    max_links: usize,
    include_substantive_candidates: bool,
) -> Vec<String> {
    links_for_page_extraction_with_rejections(
        query,
        policy,
        payload,
        candidates,
        max_links,
        include_substantive_candidates,
    )
    .0
}

fn links_for_page_extraction_with_rejections(
    query: &str,
    policy: &Value,
    payload: &Value,
    candidates: &[Candidate],
    max_links: usize,
    include_substantive_candidates: bool,
) -> (Vec<String>, Vec<String>) {
    let limit = max_links.max(1);
    let mut rejections = Vec::<String>::new();
    let reserve_payload_slots = usize::from(limit > 1);
    let candidate_limit = page_extraction_candidate_locator_max_per_stage(policy)
        .min(limit.saturating_sub(reserve_payload_slots).max(1));
    let mut ranked = Vec::<(String, f64)>::new();

    let (links, rejected) = ranked_candidate_locator_links_for_page_extraction_with_rejections(
        query,
        policy,
        candidates,
        candidate_limit,
        false,
    );
    rejections.extend(rejected);
    ranked.extend(links);

    let (links, rejected) = ranked_payload_links_for_page_extraction_with_rejections(
        query,
        policy,
        payload,
        limit,
    );
    rejections.extend(rejected);
    ranked.extend(links);

    if include_substantive_candidates {
        let (links, rejected) = ranked_candidate_locator_links_for_page_extraction_with_rejections(
            query,
            policy,
            candidates,
            candidate_limit,
            include_substantive_candidates,
        );
        rejections.extend(rejected);
        ranked.extend(links);
    }
    let selected = select_ranked_page_extraction_links(policy, ranked, limit);
    (selected, rejections)
}

fn select_ranked_page_extraction_links(
    policy: &Value,
    mut ranked: Vec<(String, f64)>,
    limit: usize,
) -> Vec<String> {
    ranked.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });
    let mut selected = Vec::<String>::new();
    let mut selected_by_key = HashMap::<String, usize>::new();
    for (link, _) in ranked {
        let dedupe_key = page_extraction_link_dedupe_key(policy, &link);
        if dedupe_key.is_empty() {
            continue;
        }
        if let Some(index) = selected_by_key.get(&dedupe_key).copied() {
            if should_prefer_page_extraction_link(&link, &selected[index]) {
                selected[index] = link;
            }
            continue;
        }
        if selected.len() >= limit {
            continue;
        }
        selected_by_key.insert(dedupe_key, selected.len());
        selected.push(link);
    }
    selected
}

fn normalize_page_extraction_link(policy: &Value, link: &str) -> Option<String> {
    let mut cleaned = clean_text(link, 2_200);
    if cleaned.is_empty() {
        return None;
    }
    if !page_extraction_url_hygiene_enabled(policy) {
        return Some(cleaned);
    }
    let lowered = cleaned.to_ascii_lowercase();
    if page_extraction_require_http_protocol(policy)
        && !(lowered.starts_with("http://") || lowered.starts_with("https://"))
    {
        return None;
    }
    if let Some((without_fragment, _)) = cleaned.split_once('#') {
        if page_extraction_drop_fragment_for_dedupe(policy) {
            cleaned = without_fragment.to_string();
        }
    }
    let without_query = cleaned
        .split_once('?')
        .map(|(value, _)| value)
        .unwrap_or(cleaned.as_str())
        .to_ascii_lowercase();
    if page_extraction_excluded_file_extensions(policy)
        .iter()
        .any(|extension| without_query.ends_with(extension))
    {
        return None;
    }
    let domain = extract_domains_from_text(&cleaned, 1)
        .into_iter()
        .next()
        .unwrap_or_default();
    if domain.is_empty() || is_search_engine_domain(&domain) {
        return None;
    }
    Some(cleaned)
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

fn page_extraction_link_preflight_rejection_reason(query: &str, link: &str) -> Option<&'static str> {
    page_extraction_link_preflight_rejection_reason_with_context(query, link, "")
}

fn page_extraction_link_preflight_rejection_reason_with_context(
    query: &str,
    link: &str,
    context: &str,
) -> Option<&'static str> {
    let candidate = page_extraction_link_candidate_with_context(link, context);
    let trusted_primary_source_candidate =
        candidate_has_trusted_primary_source_signal(query, &candidate);
    let trusted_official_source_candidate =
        candidate_has_trusted_official_source_signal(query, &candidate);
    let trusted_prefetch_candidate =
        trusted_primary_source_candidate || trusted_official_source_candidate;
    let query_has_distinctive_terms = query_has_distinctive_relevance_terms(query);
    let article_like_link = page_extraction_link_has_article_like_path(link);
    let contextual_bridge = substantive_prefetch_context_bridge(
        query,
        link,
        context,
        &candidate,
        trusted_prefetch_candidate,
    );
    if link_contains_collapsed_query_phrase(query, link) {
        return None;
    }
    if citation_wrapper_link(link) && citation_wrapper_context_has_signal(query, context) {
        return None;
    }
    let combined = clean_text(
        &format!("{} {} {}", candidate.title, candidate.snippet, candidate.locator),
        2_400,
    );
    if contains_web_junk_marker(&combined) {
        return Some("junk_link");
    }
    if !trusted_prefetch_candidate && looks_like_off_intent_noise_candidate(query, &candidate) {
        return Some("off_intent_link");
    }
    if !trusted_prefetch_candidate && !query_has_distinctive_terms && !article_like_link {
        if candidate_looks_like_relevant_discovery_hub(query, &candidate) || contextual_bridge {
            return None;
        }
        return Some("broad_query_non_article_link");
    }
    if contextual_bridge {
        return None;
    }
    if !trusted_prefetch_candidate
        && query_has_distinctive_terms
        && has_only_weak_query_overlap(query, &candidate)
    {
        return Some("weak_overlap_link");
    }
    if !trusted_prefetch_candidate
        && query_has_distinctive_terms
        && query_overlap_terms(query, &candidate) == 0
        && source_trust_adjustment(&candidate) <= 0.0
    {
        return Some("no_distinctive_overlap_link");
    }
    None
}

fn substantive_prefetch_context_bridge(
    query: &str,
    link: &str,
    context: &str,
    candidate: &Candidate,
    trusted_prefetch_candidate: bool,
) -> bool {
    let combined = clean_text(
        &format!("{} {} {}", candidate.title, candidate.snippet, candidate.locator),
        2_400,
    );
    if combined.is_empty()
        || contains_web_junk_marker(&combined)
        || looks_like_off_intent_noise_candidate(query, candidate)
        || looks_like_portal_noise_candidate(candidate)
        || looks_like_link_directory_or_aggregator_shell(&combined)
    {
        return false;
    }
    let phrase_match = query_subject_phrase_matches_candidate(query, candidate);
    let score = fallback_link_score_with_context(query, link, context);
    let min_score = if phrase_match { 0.08 } else { 0.18 };
    if score < min_score {
        return false;
    }
    let trusted_or_article = trusted_prefetch_candidate
        || source_trust_adjustment(candidate) >= 0.1
        || page_extraction_link_has_article_like_path(link);
    if !trusted_or_article {
        return false;
    }
    let content_rich = content_rich_text(context) || content_rich_text(&combined);
    if !content_rich {
        return false;
    }
    phrase_match || (current_web_intent(query) && segment_has_current_signal(&combined))
}

fn citation_wrapper_link(link: &str) -> bool {
    let Some((_, host, path, query)) = parse_page_extraction_http_url(link) else {
        return false;
    };
    let host = host.trim_start_matches("www.").to_ascii_lowercase();
    let path = path.to_ascii_lowercase();
    let query = query.unwrap_or("").to_ascii_lowercase();
    (host == "news.google.com"
        && (path.contains("/rss/articles/") || path.contains("/articles/") || path.contains("/read/")))
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

fn citation_wrapper_context_has_signal(query: &str, context: &str) -> bool {
    let cleaned_context = clean_text(context, 1_800);
    if cleaned_context.is_empty()
        || contains_web_junk_marker(&cleaned_context)
        || looks_like_low_signal_search_summary(&cleaned_context)
    {
        return false;
    }
    let candidate = page_extraction_link_candidate_with_context("https://example.com/wrapper", &cleaned_context);
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

fn page_extraction_link_dedupe_key(policy: &Value, link: &str) -> String {
    if !page_extraction_canonical_dedupe_enabled(policy) {
        return link.to_ascii_lowercase();
    }
    let Some((_, host, path, query)) = parse_page_extraction_http_url(link) else {
        return link.to_ascii_lowercase();
    };
    let host = host.trim_start_matches("www.").to_ascii_lowercase();
    if host.is_empty() {
        return link.to_ascii_lowercase();
    }
    let mut path = path.trim_end_matches('/').to_string();
    if path.is_empty() {
        path = "/".to_string();
    }
    match query {
        Some(query) if !query.is_empty() => format!("{host}{path}?{query}"),
        _ => format!("{host}{path}"),
    }
}

fn should_prefer_page_extraction_link(candidate: &str, current: &str) -> bool {
    let (
        Some((candidate_scheme, candidate_host, _, _)),
        Some((current_scheme, current_host, _, _)),
    ) = (
        parse_page_extraction_http_url(candidate),
        parse_page_extraction_http_url(current),
    )
    else {
        return false;
    };
    if candidate_scheme == "https" && current_scheme == "http" {
        return true;
    }
    if candidate_scheme != current_scheme {
        return false;
    }
    current_host.starts_with("www.") && !candidate_host.starts_with("www.")
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

fn query_overlap_terms(query: &str, candidate: &Candidate) -> usize {
    query_overlap_profile(query, candidate).0
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

fn candidate_is_synthesis_eligible(
    query: &str,
    candidate: &Candidate,
    benchmark_intent: bool,
) -> bool {
    let framework_catalog_intent = is_framework_catalog_intent(query);
    let snippet = clean_text(&candidate.snippet, 1_200);
    let lowered_snippet = snippet.to_ascii_lowercase();
    if lowered_snippet.contains("candidate domains include")
        && lowered_snippet.contains("require direct page verification")
    {
        return false;
    }
    if !candidate_passes_relevance_gate(query, candidate, benchmark_intent) {
        return false;
    }
    if looks_like_ack_only(&candidate.snippet)
        || looks_like_low_signal_search_summary(&candidate.snippet)
        || looks_like_source_only_snippet(&candidate.snippet)
    {
        return false;
    }
    if !candidate_is_substantive(query, candidate, benchmark_intent) {
        return false;
    }
    let domain = candidate_domain_hint(candidate);
    if is_search_engine_domain(&domain) {
        return false;
    }
    if benchmark_intent {
        if looks_like_definition_candidate(candidate)
            || looks_like_comparison_noise_candidate(candidate)
        {
            return false;
        }
        let trusted_overview_exception = source_trust_adjustment(candidate) >= 0.15
            && content_rich_text(&candidate.snippet)
            && query_overlap_terms(query, candidate) >= 1;
        if !looks_like_metric_rich_text(&candidate.snippet)
            && query_overlap_terms(query, candidate) < 2
            && !trusted_overview_exception
        {
            return false;
        }
    }
    if framework_catalog_intent
        && !looks_like_framework_catalog_text(&format!("{} {}", candidate.title, candidate.snippet))
        && !looks_like_framework_overview_text(&format!(
            "{} {}",
            candidate.title, candidate.snippet
        ))
        && query_overlap_terms(query, candidate) < 2
    {
        return false;
    }
    true
}

fn stage_error(payload: &Value, fallback: &str) -> String {
    clean_text(
        payload
            .get("error")
            .or_else(|| payload.pointer("/result/error"))
            .and_then(Value::as_str)
            .unwrap_or(fallback),
        220,
    )
}

fn issue_is_access_or_throttle_failure(detail: &str) -> bool {
    let lowered = clean_text(detail, 360).to_ascii_lowercase();
    [
        "rate_limited",
        "rate limit",
        "too many requests",
        "http_429",
        "429",
        "anti_bot_challenge",
        "captcha",
        "cloudflare",
        "verify you are human",
        "checking your browser",
        "needs_js",
        "javascript required",
        "access_denied",
        "access denied",
        "403",
        "login required",
        "request blocked",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn payload_access_blocker_class(payload: &Value) -> Option<&'static str> {
    let status = payload
        .get("status_code")
        .or_else(|| payload.get("statusCode"))
        .or_else(|| payload.pointer("/receipt/status_code"))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    if status == 429 {
        return Some("rate_limited");
    }
    if status == 403 {
        return Some("access_denied");
    }
    if status == 401 {
        return Some("auth_required");
    }
    let top_level_error = clean_text(
        payload
            .get("error")
            .or_else(|| payload.get("stderr"))
            .or_else(|| payload.pointer("/result/error"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        800,
    );
    let summary = clean_text(
        payload.get("summary").and_then(Value::as_str).unwrap_or(""),
        2_000,
    );
    let content = clean_text(
        payload.get("content").and_then(Value::as_str).unwrap_or(""),
        2_000,
    );
    let combined = format!("{top_level_error} {summary} {content}").to_ascii_lowercase();
    if combined.trim().is_empty() {
        return None;
    }
    if [
        "too many requests",
        "rate limit",
        "rate_limited",
        "retry-after",
        "quota exceeded",
        "http 429",
    ]
    .iter()
    .any(|marker| combined.contains(marker))
    {
        return Some("rate_limited");
    }
    if contains_antibot_marker(&combined) {
        return Some("anti_bot_challenge");
    }
    if combined.contains("please enable javascript") || combined.contains("javascript required") {
        return Some("needs_js");
    }
    if [
        "access denied",
        "403 forbidden",
        "login required",
        "subscribe to continue",
        "request blocked",
        "blocked by",
    ]
    .iter()
    .any(|marker| combined.contains(marker))
    {
        return Some("access_denied");
    }
    None
}
