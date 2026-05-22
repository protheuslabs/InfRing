fn api_search_bing_rss(
    query: &str,
    summary_only: bool,
    allowed_domains: &[String],
    exclude_subdomains: bool,
    top_k: usize,
    timeout_ms: u64,
) -> Value {
    let requested_url = web_search_bing_rss_url(query);
    let max_response_bytes = 280_000usize;
    let retry_attempts = 2usize;
    let fetched = fetch_with_curl_retry(
        &requested_url,
        timeout_ms,
        max_response_bytes,
        retry_attempts,
        false,
    );
    let status_code = fetched
        .get("status_code")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let content_type = clean_text(
        fetched
            .get("content_type")
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );
    let parsed = render_bing_rss_payload(
        fetched.get("body").and_then(Value::as_str).unwrap_or(""),
        allowed_domains,
        exclude_subdomains,
        top_k,
        max_response_bytes,
    );
    let content = clean_text(
        parsed.get("content").and_then(Value::as_str).unwrap_or(""),
        max_response_bytes,
    );
    let summary = clean_text(
        parsed.get("summary").and_then(Value::as_str).unwrap_or(""),
        900,
    );
    let fetch_ok = fetched.get("ok").and_then(Value::as_bool).unwrap_or(false)
        && parsed.get("ok").and_then(Value::as_bool).unwrap_or(false)
        && !summary.is_empty();
    let mut error_value = clean_text(
        fetched.get("stderr").and_then(Value::as_str).unwrap_or(""),
        320,
    );
    if error_value.is_empty() {
        error_value = clean_text(
            parsed.get("error").and_then(Value::as_str).unwrap_or(""),
            220,
        );
    }
    json!({
        "ok": fetch_ok,
        "requested_url": requested_url,
        "status_code": status_code,
        "content_type": if content_type.is_empty() { Value::String("application/rss+xml".to_string()) } else { Value::String(content_type) },
        "summary": summary,
        "content": if summary_only { Value::String(String::new()) } else { Value::String(content) },
        "links": parsed.get("links").cloned().unwrap_or_else(|| json!([])),
        "results": parsed.get("results").cloned().unwrap_or_else(|| json!([])),
        "content_domains": parsed.get("content_domains").cloned().unwrap_or_else(|| json!([])),
        "provider_raw_count": parsed.get("provider_raw_count").cloned().unwrap_or_else(|| json!(0)),
        "provider_filtered_count": parsed.get("provider_filtered_count").cloned().unwrap_or_else(|| json!(0)),
        "retry_attempts": fetched.get("retry_attempts").cloned().unwrap_or_else(|| json!(1)),
        "retry_used": fetched.get("retry_used").cloned().unwrap_or_else(|| json!(false)),
        "user_agent": fetched.get("user_agent").cloned().unwrap_or_else(|| json!(DEFAULT_WEB_USER_AGENTS[0])),
        "provider": "bing_rss",
        "error": if fetch_ok {
            Value::Null
        } else if error_value.is_empty() {
            Value::String("bing_rss_search_failed".to_string())
        } else {
            Value::String(error_value)
        }
    })
}

fn api_search_google_news_rss(
    query: &str,
    summary_only: bool,
    allowed_domains: &[String],
    exclude_subdomains: bool,
    top_k: usize,
    timeout_ms: u64,
) -> Value {
    let requested_url = web_search_google_news_rss_url(query);
    let max_response_bytes = 320_000usize;
    let retry_attempts = 2usize;
    let fetched = fetch_with_curl_retry(
        &requested_url,
        timeout_ms,
        max_response_bytes,
        retry_attempts,
        false,
    );
    let status_code = fetched
        .get("status_code")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let content_type = clean_text(
        fetched
            .get("content_type")
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );
    let parsed = render_google_news_rss_payload(
        fetched.get("body").and_then(Value::as_str).unwrap_or(""),
        allowed_domains,
        exclude_subdomains,
        top_k,
        max_response_bytes,
    );
    let content = clean_text(
        parsed.get("content").and_then(Value::as_str).unwrap_or(""),
        max_response_bytes,
    );
    let summary = clean_text(
        parsed.get("summary").and_then(Value::as_str).unwrap_or(""),
        900,
    );
    let fetch_ok = fetched.get("ok").and_then(Value::as_bool).unwrap_or(false)
        && parsed.get("ok").and_then(Value::as_bool).unwrap_or(false)
        && !summary.is_empty();
    let mut error_value = clean_text(
        fetched.get("stderr").and_then(Value::as_str).unwrap_or(""),
        320,
    );
    if error_value.is_empty() {
        error_value = clean_text(
            parsed.get("error").and_then(Value::as_str).unwrap_or(""),
            220,
        );
    }
    json!({
        "ok": fetch_ok,
        "requested_url": requested_url,
        "status_code": status_code,
        "content_type": if content_type.is_empty() { Value::String("application/rss+xml".to_string()) } else { Value::String(content_type) },
        "summary": summary,
        "content": if summary_only { Value::String(String::new()) } else { Value::String(content) },
        "links": parsed.get("links").cloned().unwrap_or_else(|| json!([])),
        "results": parsed.get("results").cloned().unwrap_or_else(|| json!([])),
        "content_domains": parsed.get("content_domains").cloned().unwrap_or_else(|| json!([])),
        "provider_raw_count": parsed.get("provider_raw_count").cloned().unwrap_or_else(|| json!(0)),
        "provider_filtered_count": parsed.get("provider_filtered_count").cloned().unwrap_or_else(|| json!(0)),
        "retry_attempts": fetched.get("retry_attempts").cloned().unwrap_or_else(|| json!(1)),
        "retry_used": fetched.get("retry_used").cloned().unwrap_or_else(|| json!(false)),
        "user_agent": fetched.get("user_agent").cloned().unwrap_or_else(|| json!(DEFAULT_WEB_USER_AGENTS[0])),
        "provider": "google_news_rss",
        "error": if fetch_ok {
            Value::Null
        } else if error_value.is_empty() {
            Value::String("google_news_rss_search_failed".to_string())
        } else {
            Value::String(error_value)
        }
    })
}

fn browser_serp_engine_urls(query: &str, top_k: usize) -> Vec<(&'static str, String)> {
    vec![("bing_html", web_search_bing_html_url(query, top_k.max(12)))]
}

fn browser_serp_query_param_raw(raw_url: &str, key: &str) -> Option<String> {
    let (_, query) = raw_url.split_once('?')?;
    for pair in query.split('&') {
        let mut chunks = pair.splitn(2, '=');
        let raw_key = chunks.next().unwrap_or_default().trim();
        let value = chunks.next().unwrap_or_default().trim();
        if raw_key == key {
            return Some(percent_decode_urlish(value));
        }
    }
    None
}

fn browser_serp_decode_base64_urlish(raw: &str) -> Option<String> {
    use base64::Engine as _;

    let cleaned = clean_text(raw, 2200);
    if cleaned.starts_with("http://") || cleaned.starts_with("https://") {
        return Some(cleaned);
    }
    let mut candidates = vec![cleaned.clone()];
    if let Some(stripped) = cleaned.strip_prefix("a1") {
        candidates.push(stripped.to_string());
    }
    for candidate in candidates {
        let mut padded = candidate.clone();
        while padded.len() % 4 != 0 {
            padded.push('=');
        }
        for encoded in [candidate.as_str(), padded.as_str()] {
            let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(encoded)
                .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(encoded))
                .or_else(|_| base64::engine::general_purpose::STANDARD.decode(encoded));
            let Ok(bytes) = decoded else {
                continue;
            };
            let text = clean_text(&String::from_utf8_lossy(&bytes), 2200);
            if text.starts_with("http://") || text.starts_with("https://") {
                return Some(text);
            }
        }
    }
    None
}

fn browser_serp_decode_bing_redirect(raw_url: &str) -> Option<String> {
    let domain = extract_domain(raw_url);
    if !matches!(domain.as_str(), "bing.com" | "www.bing.com") {
        return None;
    }
    let value = browser_serp_query_param_raw(raw_url, "u")?;
    browser_serp_decode_base64_urlish(&value)
}

fn browser_serp_normalize_result_link(raw_url: &str) -> String {
    let normalized = normalize_search_result_link(raw_url);
    browser_serp_decode_bing_redirect(&normalized).unwrap_or(normalized)
}

fn browser_serp_is_search_navigation_url(url: &str) -> bool {
    let lowered = clean_text(url, 2200).to_ascii_lowercase();
    if !(lowered.starts_with("http://") || lowered.starts_with("https://")) {
        return true;
    }
    let domain = extract_domain(&lowered);
    if domain.is_empty() {
        return true;
    }
    let search_host = matches!(
        domain.as_str(),
        "bing.com"
            | "www.bing.com"
            | "duckduckgo.com"
            | "www.duckduckgo.com"
            | "html.duckduckgo.com"
            | "lite.duckduckgo.com"
            | "google.com"
            | "www.google.com"
            | "news.google.com"
    );
    if search_host {
        return true;
    }
    lowered.starts_with("javascript:")
        || lowered.starts_with("mailto:")
        || lowered.starts_with("tel:")
        || lowered.contains("/aclick?")
        || lowered.contains("ad_domain=")
}

fn browser_serp_link_text_is_navigation(text: &str) -> bool {
    let lowered = clean_text(text, 220).to_ascii_lowercase();
    if lowered.len() < 4 {
        return true;
    }
    matches!(
        lowered.as_str(),
        "images"
            | "videos"
            | "maps"
            | "shopping"
            | "news"
            | "more"
            | "next"
            | "previous"
            | "privacy"
            | "terms"
            | "settings"
            | "sign in"
            | "cached"
            | "feedback"
    ) || lowered.contains("search settings")
        || lowered.contains("about this result")
}

fn browser_serp_snippet_from_page_text(page_text: &str, title: &str) -> String {
    let title = clean_text(title, 220);
    if title.is_empty() {
        return String::new();
    }
    let rows = page_text
        .lines()
        .map(|row| clean_text(row, 420))
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    let title_l = title.to_ascii_lowercase();
    for (idx, row) in rows.iter().enumerate() {
        if !row.to_ascii_lowercase().contains(&title_l) {
            continue;
        }
        let mut excerpt = row.clone();
        for next in rows.iter().skip(idx + 1).take(2) {
            if !excerpt.contains(next) {
                excerpt = clean_text(format!("{excerpt} {next}").as_str(), 520);
            }
        }
        let excerpt = clean_text(&excerpt, 520);
        if excerpt != title {
            return excerpt;
        }
    }
    String::new()
}

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
    for row in &links_summary {
        if challenge {
            break;
        }
        let raw_href = row.get("href").and_then(Value::as_str).unwrap_or("");
        let link = browser_serp_normalize_result_link(raw_href);
        if link.is_empty()
            || browser_serp_is_search_navigation_url(&link)
            || !domain_allowed_for_scope(&link, allowed_domains, exclude_subdomains)
            || seen.iter().any(|existing| existing == &link)
        {
            continue;
        }
        let title = clean_text(row.get("text").and_then(Value::as_str).unwrap_or(""), 220);
        if browser_serp_link_text_is_navigation(&title) {
            continue;
        }
        let snippet = browser_serp_snippet_from_page_text(&page_text, &title);
        let rendered = render_search_row(&title, &snippet, &link);
        if rendered.is_empty() {
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
            "materialization_error": materialization_error
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
fn search_payload_usable(payload: &Value) -> bool {
    if !payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        return false;
    }
    if payload_looks_like_search_challenge(payload)
        || payload_looks_low_signal_search(payload)
        || search_payload_looks_competitive_programming_dump(payload)
    {
        return false;
    }
    let summary = clean_text(
        payload.get("summary").and_then(Value::as_str).unwrap_or(""),
        1_200,
    );
    if summary.is_empty() {
        return false;
    }
    !search_summary_has_low_signal_marker(&summary)
}
fn search_payload_looks_competitive_programming_dump(payload: &Value) -> bool {
    let summary = clean_text(
        payload.get("summary").and_then(Value::as_str).unwrap_or(""),
        2_400,
    );
    let content = clean_text(
        payload.get("content").and_then(Value::as_str).unwrap_or(""),
        3_200,
    );
    let combined = format!("{summary}\n{content}").to_ascii_lowercase();
    if combined.trim().is_empty() {
        return false;
    }
    let marker_hits = [
        "given a tree",
        "input specification",
        "output specification",
        "sample input",
        "sample output",
        "#include <stdio.h>",
        "int main()",
        "public class",
        "translate the following java code",
        "csdn.net",
        "acm",
    ]
    .iter()
    .filter(|marker| combined.contains(**marker))
    .count();
    marker_hits >= 3
}
fn search_query_is_meta_diagnostic(query: &str) -> bool {
    let lowered = clean_text(query, 600).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let explicit_search_intent = ["search for ", "search the web", "web search", "find information", "finding information", "look up", "compare ", "official docs", "research online", "research on web"]
        .iter()
        .any(|marker| lowered.contains(*marker));
    if explicit_search_intent {
        return false;
    }
    if [
        "that was just a test",
        "that was a test",
        "did you do the web request",
        "did you try it",
        "where did that come from",
        "where the hell did that come from",
        "you hallucinated",
        "you returned no result",
        "answer the question",
    ]
    .iter()
    .any(|marker| lowered.contains(*marker))
    {
        return true;
    }
    if lowered.contains("did you do the web request")
        || lowered.contains("did you try it")
        || lowered.contains("why did my last prompt")
        || lowered.contains("you returned no result")
        || lowered.contains("that was just a test")
        || lowered.contains("that was a test")
        || lowered.contains("where did that come from")
    {
        return true;
    }
    let signal_terms = lowered
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| token.len() >= 3)
        .count();
    let meta_hits = ["what happened", "workflow", "tool call", "web tooling", "hallucination", "hallucinated", "training data", "context issue", "answer the question", "last response", "previous response"]
    .iter()
    .filter(|marker| lowered.contains(**marker))
    .count();
    if meta_hits == 0 {
        return false;
    }
    let web_intent_hits = ["site:", "http://", "https://", "latest ", "top ", "best ", "news", "framework", "docs", "recipe", "weather", "price"]
    .iter()
    .filter(|marker| lowered.contains(**marker))
    .count();
    if web_intent_hits > 0 {
        return false;
    }
    let research_intent_hits = [
        "technique",
        "techniques",
        "mitigation",
        "how to",
        "best practice",
        "best practices",
        "guide",
        "tutorial",
        "methods",
        "strategy",
    ]
    .iter()
    .filter(|marker| lowered.contains(**marker))
    .count();
    if meta_hits == 1 && research_intent_hits > 0 {
        return false;
    }
    meta_hits >= 2 || signal_terms <= 7
}
fn search_override_flag_enabled(value: &Value) -> bool {
    runtime_web_truthy_flag(value)
}
fn search_meta_query_override(request: &Value) -> bool {
    let pointers = [
        "/allow_meta_query_search",
        "/allowMetaQuerySearch",
        "/force_web_search",
        "/forceWebSearch",
        "/force_web_lookup",
        "/forceWebLookup",
        "/search_policy/allow_meta_query_search",
        "/searchPolicy/allowMetaQuerySearch",
        "/search_policy/force_web_search",
        "/searchPolicy/forceWebSearch",
        "/search_policy/force_web_lookup",
        "/searchPolicy/forceWebLookup",
    ];
    runtime_web_request_flag(request, &pointers)
}
