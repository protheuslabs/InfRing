// Layer ownership: core/layer0/ops::browser-serp-search-provider (authoritative)

fn render_browser_serp_materialization(
    engine: &str,
    requested_url: &str,
    materialized: &Value,
    allowed_domains: &[String],
    exclude_subdomains: bool,
    top_k: usize,
    max_response_bytes: usize,
) -> Value {
    let page = materialized
        .get("materialized_page")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let page_text = clean_text(
        page.get("main_text_or_markdown")
            .and_then(Value::as_str)
            .unwrap_or(""),
        max_response_bytes.min(120_000),
    );
    let links_summary = page
        .get("links_summary")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let blocker = page
        .get("blocker_classification")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let challenge = looks_like_search_challenge_payload(
        page.get("title").and_then(Value::as_str).unwrap_or(""),
        &page_text,
    ) || blocker
        .get("status")
        .and_then(Value::as_str)
        .map(|status| status.contains("blocked") || status.contains("challenge"))
        .unwrap_or(false);
    let mut lines = Vec::<String>::new();
    let mut links = Vec::<String>::new();
    let mut domains = Vec::<String>::new();
    let mut results = Vec::<Value>::new();
    let mut seen = Vec::<String>::new();
    let mut rejection_counts = serde_json::Map::<String, Value>::new();
    let mut rejection_samples = Vec::<Value>::new();
    for row in &links_summary {
        if challenge {
            break;
        }
        let raw_href = row.get("href").and_then(Value::as_str).unwrap_or("");
        let title = clean_text(row.get("text").and_then(Value::as_str).unwrap_or(""), 220);
        if raw_href.trim().is_empty() {
            browser_serp_record_rejection(
                &mut rejection_counts,
                &mut rejection_samples,
                "missing_href",
                raw_href,
                "",
                &title,
            );
            continue;
        }
        let link = browser_serp_normalize_result_link(raw_href);
        if link.is_empty() {
            browser_serp_record_rejection(
                &mut rejection_counts,
                &mut rejection_samples,
                "empty_normalized_url",
                raw_href,
                &link,
                &title,
            );
            continue;
        }
        if browser_serp_is_search_navigation_url(&link) {
            browser_serp_record_rejection(
                &mut rejection_counts,
                &mut rejection_samples,
                "search_navigation_or_non_external",
                raw_href,
                &link,
                &title,
            );
            continue;
        }
        if !domain_allowed_for_scope(&link, allowed_domains, exclude_subdomains) {
            browser_serp_record_rejection(
                &mut rejection_counts,
                &mut rejection_samples,
                "domain_scope_rejected",
                raw_href,
                &link,
                &title,
            );
            continue;
        }
        if seen.iter().any(|existing| existing == &link) {
            browser_serp_record_rejection(
                &mut rejection_counts,
                &mut rejection_samples,
                "duplicate_url",
                raw_href,
                &link,
                &title,
            );
            continue;
        }
        if browser_serp_link_text_is_navigation(&title) {
            browser_serp_record_rejection(
                &mut rejection_counts,
                &mut rejection_samples,
                "navigation_text",
                raw_href,
                &link,
                &title,
            );
            continue;
        }
        let snippet = browser_serp_snippet_from_page_text(&page_text, &title);
        let rendered = render_search_row(&title, &snippet, &link);
        if rendered.is_empty() {
            browser_serp_record_rejection(
                &mut rejection_counts,
                &mut rejection_samples,
                "rendered_row_empty",
                raw_href,
                &link,
                &title,
            );
            continue;
        }
        seen.push(link.clone());
        lines.push(rendered);
        links.push(link.clone());
        push_unique_link_domain(&mut domains, &link);
        results.push(json!({
            "title": title,
            "url": link,
            "snippet": snippet,
            "source": "browser_serp",
            "engine": clean_text(engine, 80),
            "rank": results.len() + 1
        }));
        if lines.len() >= top_k.max(1) {
            break;
        }
    }
    let content = clean_text(&lines.join("\n"), max_response_bytes.min(120_000));
    let ok = !content.is_empty();
    let materialization_error = clean_text(
        materialized.get("error").and_then(Value::as_str).unwrap_or(""),
        220,
    );
    let outcome_classification = browser_serp_outcome_classification(
        ok,
        challenge,
        &materialization_error,
        links_summary.len(),
        lines.len(),
        &rejection_counts,
    );
    let diagnostic_text = if challenge {
        clean_text(&page_text, 1_200)
    } else {
        String::new()
    };
    json!({
        "ok": ok,
        "requested_url": clean_text(requested_url, 2200),
        "status_code": page.get("status_code").cloned().unwrap_or_else(|| json!(0)),
        "content_type": "text/html",
        "summary": if ok {
            summarize_text(&content, 900)
        } else if challenge && !diagnostic_text.is_empty() {
            summarize_text(&diagnostic_text, 900)
        } else {
            crate::tool_output_match_filter::no_findings_user_copy().to_string()
        },
        "content": if ok { content } else { diagnostic_text },
        "links": links,
        "results": results,
        "content_domains": domains,
        "provider_raw_count": links_summary.len(),
        "provider_filtered_count": lines.len(),
        "browser_serp": {
            "engine": clean_text(engine, 80),
            "materialization_ok": materialized.get("ok").and_then(Value::as_bool).unwrap_or(false),
            "challenge_detected": challenge,
            "blocker_classification": blocker,
            "outcome_classification": outcome_classification,
            "materialization_error": materialization_error,
            "link_rejection_counts": rejection_counts,
            "link_rejection_samples": rejection_samples
        },
        "error": if ok {
            Value::Null
        } else if challenge {
            Value::String("anti_bot_challenge".to_string())
        } else if materialization_error.is_empty() {
            Value::String("browser_serp_no_results".to_string())
        } else {
            Value::String(materialization_error)
        }
    })
}

fn merge_browser_serp_payloads(payloads: &[Value], top_k: usize, summary_only: bool) -> Value {
    let mut lines = Vec::<String>::new();
    let mut links = Vec::<String>::new();
    let mut domains = Vec::<String>::new();
    let mut results = Vec::<Value>::new();
    let mut diagnostics = Vec::<Value>::new();
    let mut raw_count = 0usize;
    let mut filtered_count = 0usize;
    let mut challenge_detected = false;
    let mut rejection_counts = serde_json::Map::<String, Value>::new();
    let mut rejection_samples = Vec::<Value>::new();
    let mut first_requested_url = String::new();
    let mut last_error = String::new();
    let mut diagnostic_content = String::new();
    for payload in payloads {
        if first_requested_url.is_empty() {
            first_requested_url = clean_text(
                payload.get("requested_url").and_then(Value::as_str).unwrap_or(""),
                2200,
            );
        }
        raw_count += payload
            .get("provider_raw_count")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize;
        filtered_count += payload
            .get("provider_filtered_count")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize;
        if payload
            .pointer("/browser_serp/challenge_detected")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            challenge_detected = true;
        }
        let error = clean_text(payload.get("error").and_then(Value::as_str).unwrap_or(""), 220);
        if !error.is_empty() {
            last_error = error;
        }
        let content = clean_text(payload.get("content").and_then(Value::as_str).unwrap_or(""), 120_000);
        if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            for row in content.lines() {
                let row = clean_text(row, 1_200);
                if !row.is_empty() && !lines.iter().any(|existing| existing == &row) {
                    lines.push(row);
                }
                if lines.len() >= top_k.max(1) {
                    break;
                }
            }
        } else if diagnostic_content.is_empty() && !content.is_empty() {
            diagnostic_content = content;
        }
        if let Some(rows) = payload.get("links").and_then(Value::as_array) {
            for row in rows.iter().filter_map(Value::as_str) {
                let link = clean_text(row, 2200);
                if !link.is_empty() && !links.iter().any(|existing| existing == &link) {
                    links.push(link);
                }
                if links.len() >= top_k.max(1) {
                    break;
                }
            }
        }
        if let Some(rows) = payload.get("content_domains").and_then(Value::as_array) {
            for row in rows.iter().filter_map(Value::as_str) {
                let domain = clean_text(row, 220);
                if !domain.is_empty() && !domains.iter().any(|existing| existing == &domain) {
                    domains.push(domain);
                }
            }
        }
        if let Some(rows) = payload.get("results").and_then(Value::as_array) {
            for row in rows {
                if results.len() >= top_k.max(1) {
                    break;
                }
                results.push(row.clone());
            }
        }
        diagnostics.push(
            payload
                .get("browser_serp")
                .cloned()
                .unwrap_or_else(|| json!({})),
        );
        browser_serp_merge_rejection_counts(
            &mut rejection_counts,
            payload.pointer("/browser_serp/link_rejection_counts"),
        );
        browser_serp_merge_rejection_samples(
            &mut rejection_samples,
            payload.pointer("/browser_serp/link_rejection_samples"),
        );
        if lines.len() >= top_k.max(1) {
            break;
        }
    }
    let content = clean_text(&lines.join("\n"), 120_000);
    let ok = !content.is_empty();
    json!({
        "ok": ok,
        "requested_url": first_requested_url,
        "status_code": if ok { 200 } else { 0 },
        "content_type": "text/html",
        "summary": if ok {
            summarize_text(&content, 900)
        } else if challenge_detected && !diagnostic_content.is_empty() {
            summarize_text(&diagnostic_content, 900)
        } else {
            crate::tool_output_match_filter::no_findings_user_copy().to_string()
        },
        "content": if summary_only && ok { Value::String(String::new()) } else if ok { Value::String(content) } else { Value::String(diagnostic_content) },
        "links": links,
        "results": results,
        "content_domains": domains,
        "provider_raw_count": raw_count,
        "provider_filtered_count": filtered_count,
        "provider": "browser_serp",
        "browser_serp_diagnostics": diagnostics,
        "browser_serp_link_rejection_counts": rejection_counts,
        "browser_serp_link_rejection_samples": rejection_samples,
        "error": if ok {
            Value::Null
        } else if challenge_detected {
            Value::String("anti_bot_challenge".to_string())
        } else if last_error.is_empty() {
            Value::String("browser_serp_no_results".to_string())
        } else {
            Value::String(last_error)
        }
    })
}

fn api_search_browser_serp(
    root: &Path,
    query: &str,
    summary_only: bool,
    allowed_domains: &[String],
    exclude_subdomains: bool,
    top_k: usize,
    timeout_ms: u64,
) -> Value {
    let max_response_bytes = 350_000usize;
    let mut payloads = Vec::<Value>::new();
    for (engine, requested_url) in browser_serp_engine_urls(query, top_k) {
        let materialized = api_browser_materialize_page(
            root,
            &json!({
                "url": requested_url.clone(),
                "admission_ref": "browser_serp_search_provider",
                "extract_mode": "text",
                "timeout_ms": timeout_ms.clamp(5_000, 45_000),
                "max_response_bytes": max_response_bytes
            }),
        );
        let rendered = render_browser_serp_materialization(
            engine,
            &requested_url,
            &materialized,
            allowed_domains,
            exclude_subdomains,
            top_k,
            max_response_bytes,
        );
        let enough = rendered
            .get("provider_filtered_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            >= top_k.min(6).max(1) as u64;
        payloads.push(rendered);
        if enough {
            break;
        }
    }
    merge_browser_serp_payloads(&payloads, top_k, summary_only)
}
