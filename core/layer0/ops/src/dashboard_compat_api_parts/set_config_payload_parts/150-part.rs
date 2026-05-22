fn tool_error_text(payload: &Value) -> String {
    clean_text(
        payload
            .get("error")
            .or_else(|| payload.get("message"))
            .or_else(|| payload.pointer("/result/error"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        240,
    )
}

fn looks_like_domain_token(value: &str) -> bool {
    if value.is_empty() || !value.contains('.') {
        return false;
    }
    if value.starts_with('.') || value.ends_with('.') {
        return false;
    }
    if value
        .chars()
        .any(|ch| !(ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-')))
    {
        return false;
    }
    let Some(tld) = value.rsplit('.').next() else {
        return false;
    };
    (2..=24).contains(&tld.len())
}

fn extract_search_result_domains(summary: &str, max_domains: usize) -> Vec<String> {
    let mut domains = Vec::<String>::new();
    for token in clean_text(summary, 4_000).split_whitespace() {
        let stripped = token
            .trim_matches(|ch: char| {
                !ch.is_ascii_alphanumeric() && ch != '.' && ch != '-' && ch != '/'
            })
            .trim_start_matches("http://")
            .trim_start_matches("https://")
            .trim_start_matches("www.");
        let host = stripped
            .split('/')
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        if !looks_like_domain_token(&host) {
            continue;
        }
        if host == "duckduckgo.com" {
            continue;
        }
        if domains.iter().any(|existing| existing == &host) {
            continue;
        }
        domains.push(host);
        if domains.len() >= max_domains.max(1) {
            break;
        }
    }
    domains
}

fn web_search_framework_catalog_intent(query: &str) -> bool {
    let _ = query;
    false
}

fn framework_names_from_web_text(text: &str) -> Vec<&'static str> {
    let _ = text;
    Vec::new()
}

fn search_source_is_low_signal(domain: &str) -> bool {
    let lowered = clean_text(domain, 160).to_ascii_lowercase();
    lowered.contains("zhihu.com")
        || lowered.contains("reddit.com")
        || lowered.contains("quora.com")
        || lowered.contains("news.ycombinator.com")
        || lowered.ends_with(".org.cn")
        || lowered.ends_with(".com.cn")
        || lowered.contains("support.microsoft.com")
}

fn filter_framework_search_domains(query: &str, domains: Vec<String>) -> Vec<String> {
    let _ = query;
    domains
}

fn web_query_topic_terms(query: &str, max_terms: usize) -> HashSet<String> {
    let stopwords = [
        "the",
        "and",
        "for",
        "with",
        "that",
        "this",
        "from",
        "about",
        "current",
        "latest",
        "best",
        "top",
        "find",
        "search",
        "information",
        "online",
        "web",
        "framework",
        "frameworks",
        "agent",
        "agents",
    ]
    .into_iter()
    .collect::<HashSet<_>>();
    clean_text(query, 1_200)
        .to_ascii_lowercase()
        .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '.' && ch != '_')
        .filter_map(|term| {
            let cleaned = term.trim().to_string();
            if cleaned.len() < 3 || stopwords.contains(cleaned.as_str()) {
                None
            } else {
                Some(cleaned)
            }
        })
        .take(max_terms.max(1))
        .collect::<HashSet<_>>()
}

fn web_result_domain_topic_mismatch_score(query: &str, summary: &str, evidence_refs: &Value) -> f64 {
    let terms = web_query_topic_terms(query, 12);
    if terms.is_empty() {
        return 0.0;
    }
    let mut evidence_blob = clean_text(summary, 8_000);
    if let Some(rows) = evidence_refs.as_array() {
        for row in rows.iter().take(8) {
            let title = clean_text(row.get("title").and_then(Value::as_str).unwrap_or(""), 240);
            let locator = clean_text(row.get("locator").and_then(Value::as_str).unwrap_or(""), 320);
            if !title.is_empty() {
                evidence_blob.push('\n');
                evidence_blob.push_str(&title);
            }
            if !locator.is_empty() {
                evidence_blob.push('\n');
                evidence_blob.push_str(&locator);
            }
        }
    }
    let lowered = evidence_blob.to_ascii_lowercase();
    let coverage_hits = terms
        .iter()
        .filter(|term| lowered.contains(term.as_str()))
        .count() as f64;
    let coverage_ratio = if terms.is_empty() {
        0.0
    } else {
        coverage_hits / (terms.len() as f64)
    };
    let domains = extract_search_result_domains(&evidence_blob, 8);
    let strong_off_topic_domain = domains.iter().any(|domain| {
        let lowered_domain = clean_text(domain, 200).to_ascii_lowercase();
        lowered_domain.contains("qrz.com")
            || lowered_domain.contains("ham")
            || lowered_domain.contains("callsign")
    });
    let low_signal_domain_ratio = if domains.is_empty() {
        0.0
    } else {
        domains
            .iter()
            .filter(|domain| search_source_is_low_signal(domain))
            .count() as f64
            / (domains.len() as f64)
    };
    let framework_query = web_search_framework_catalog_intent(query);
    let framework_hit_count = framework_names_from_web_text(&evidence_blob).len() as f64;
    let framework_penalty = if framework_query && framework_hit_count == 0.0 {
        0.18
    } else {
        0.0
    };
    let mut score = (1.0 - coverage_ratio) * 0.62 + low_signal_domain_ratio * 0.28 + framework_penalty;
    if strong_off_topic_domain {
        score += 0.22;
    }
    score.clamp(0.0, 1.0)
}

fn web_search_off_topic_results_fallback(
    query: &str,
    mismatch_score: f64,
    domains: &[String],
) -> String {
    let query = clean_text(query, 220);
    let score = (mismatch_score * 100.0).round() / 100.0;
    let mut summary = if query.is_empty() {
        format!("Web search returned off-topic or weak source matches (mismatch score {score}).")
    } else {
        format!(
            "Web search for `{query}` returned off-topic or weak source matches (mismatch score {score})."
        )
    };
    let domains = domains
        .iter()
        .map(|row| clean_text(row, 120))
        .filter(|row| !row.is_empty())
        .take(4)
        .collect::<Vec<_>>();
    if !domains.is_empty() {
        summary.push_str(&format!(" Candidate domains seen: {}.", domains.join(", ")));
    }
    summary.push_str(" No source-backed answer should be synthesized from this result set.");
    trim_text(&summary, 1_200)
}

fn web_search_no_findings_fallback(
    query: &str,
    combined: &str,
    requested_url: &str,
    domain: &str,
) -> String {
    let query = clean_text(query, 220);
    let mut domains = extract_search_result_domains(combined, 4);
    let explicit_domain = clean_text(domain, 120);
    if !explicit_domain.is_empty()
        && !domains
            .iter()
            .any(|row| row.eq_ignore_ascii_case(&explicit_domain))
    {
        domains.push(explicit_domain);
    }
    let requested_url = clean_text(requested_url, 220);
    if domains.is_empty() && !requested_url.is_empty() {
        domains.push(requested_url);
    }
    let mut summary = if query.is_empty() {
        "Web search did not return usable source-backed findings.".to_string()
    } else {
        format!("Web search for `{query}` did not return usable source-backed findings.")
    };
    let domains = domains
        .into_iter()
        .map(|row| clean_text(&row, 120))
        .filter(|row| !row.is_empty())
        .take(4)
        .collect::<Vec<_>>();
    if !domains.is_empty() {
        summary.push_str(&format!(" Candidate sources seen: {}.", domains.join(", ")));
    }
    trim_text(&summary, 1_200)
}

fn extract_search_result_findings(summary: &str, max_items: usize) -> Vec<String> {
    if max_items == 0 {
        return Vec::new();
    }
    let mut out = Vec::<String>::new();
    let mut seen = HashSet::<String>::new();
    let normalized = clean_text(summary, 6_000);
    for line in normalized
        .split(|ch| matches!(ch, '\n' | '|' | '•'))
        .map(|row| clean_text(row, 280))
    {
        if line.is_empty() {
            continue;
        }
        if looks_like_search_engine_chrome_summary(&line) {
            continue;
        }
        let lowered = line.to_ascii_lowercase();
        if lowered.contains("duckduckgo all regions")
            || lowered.starts_with("all regions ")
            || lowered.starts_with("safe search ")
            || lowered.contains(" at duckduckgo")
            || lowered.contains("site links")
            || lowered.contains("key findings for")
            || lowered.contains("potential sources:")
        {
            continue;
        }
        if lowered.contains(" at ") && lowered.contains("duckduckgo") {
            continue;
        }
        if lowered.starts_with("bing.com:")
            || lowered.starts_with("duckduckgo.com:")
            || lowered.starts_with("google.com:")
            || lowered.starts_with("www.bing.com:")
            || lowered.starts_with("www.duckduckgo.com:")
            || lowered.starts_with("www.google.com:")
        {
            continue;
        }
        if let Some((prefix, _)) = lowered.split_once(':') {
            let domain_prefix = prefix.trim().trim_start_matches("www.");
            if looks_like_domain_token(domain_prefix) {
                continue;
            }
        }
        let has_link_hint = lowered.contains("http://")
            || lowered.contains("https://")
            || lowered.contains(".org/")
            || lowered.contains(".com/")
            || lowered.contains(".ai/")
            || lowered.contains(".dev/");
        if lowered.contains("...") && lowered.contains("all regions") {
            continue;
        }
        if !has_link_hint && line.len() < 44 {
            continue;
        }
        let compact = trim_text(&line.replace('\t', " ").replace("  ", " "), 240);
        if compact.is_empty() {
            continue;
        }
        let key = compact.to_ascii_lowercase();
        if !seen.insert(key) {
            continue;
        }
        out.push(compact);
        if out.len() >= max_items {
            break;
        }
    }
    out
}

fn filter_framework_search_findings(query: &str, findings: Vec<String>) -> Vec<String> {
    let _ = query;
    findings
}

fn rewrite_framework_web_search_summary(
    query: &str,
    raw_summary: &str,
    evidence_refs: &Value,
) -> Option<String> {
    let _ = (query, raw_summary, evidence_refs);
    None
}

fn looks_like_placeholder_fetch_content(text: &str, requested_url: &str) -> bool {
    let lowered = clean_text(text, 2_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let requested = clean_text(requested_url, 400).to_ascii_lowercase();
    if requested.contains("example.com") {
        return true;
    }
    lowered.contains("example domain")
        && lowered.contains("for use in documentation examples")
        && lowered.contains("without needing permission")
}

fn looks_like_navigation_chrome_payload(text: &str) -> bool {
    let lowered = clean_text(text, 4_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let marker_count = [
        "skip to content",
        "home",
        "news",
        "sport",
        "business",
        "technology",
        "health",
        "culture",
        "travel",
        "audio",
        "video",
        "live",
        "all regions",
    ]
    .iter()
    .filter(|marker| lowered.contains(**marker))
    .count();
    marker_count >= 5 && lowered.split_whitespace().count() >= 14
}

fn source_label_from_url(raw: &str) -> String {
    let cleaned = clean_text(raw, 2200);
    if cleaned.is_empty() {
        return String::new();
    }
    if let Some(rest) = cleaned
        .strip_prefix("https://")
        .or_else(|| cleaned.strip_prefix("http://"))
    {
        return clean_text(rest.split('/').next().unwrap_or(""), 200);
    }
    clean_text(cleaned.split('/').next().unwrap_or(""), 200)
}

fn summarize_web_fetch_payload(payload: &Value) -> String {
    if !payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        return user_facing_tool_failure_summary("web_fetch", payload).unwrap_or_default();
    }
    let requested_url = clean_text(
        payload
            .get("requested_url")
            .or_else(|| payload.pointer("/receipt/requested_url"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        2200,
    );
    let summary = clean_text(
        payload.get("summary").and_then(Value::as_str).unwrap_or(""),
        4_000,
    );
    let content = clean_text(
        payload.get("content").and_then(Value::as_str).unwrap_or(""),
        4_000,
    );
    let body = if summary.is_empty() {
        content.clone()
    } else {
        summary.clone()
    };
    let stripped_body = strip_context_guard_markers(&body);
    if response_mentions_context_guard(&body) {
        let source = source_label_from_url(&requested_url);
        let snippet = first_sentence(&stripped_body, 320);
        if !snippet.is_empty() {
            let _ = source;
            return String::new();
        }
        if source.is_empty() {
            return web_tool_context_guard_fallback("The fetched page");
        }
        return web_tool_context_guard_fallback(&format!("Fetch from {}", trim_text(&source, 120)));
    }
    if body.is_empty() {
        return String::new();
    }
    if looks_like_placeholder_fetch_content(&body, &requested_url) {
        return String::new();
    }
    if looks_like_navigation_chrome_payload(&body) || looks_like_search_engine_chrome_summary(&body)
    {
        return String::new();
    }
    let snippet = first_sentence(&body, 320);
    if snippet.is_empty() {
        return String::new();
    }
    let source = source_label_from_url(&requested_url);
    if source.is_empty() {
        snippet
    } else {
        format!("From {}: {}", trim_text(&source, 120), snippet)
    }
}

fn looks_like_search_engine_chrome_summary(summary: &str) -> bool {
    let lowered = summary.to_ascii_lowercase();
    let potential_source_mentions = lowered.matches("potential sources:").count();
    if lowered.contains("unfortunately, bots use duckduckgo too")
        || lowered.contains("please complete the following challenge")
        || lowered.contains("select all squares containing a duck")
        || lowered.contains("error-lite@duckduckgo.com")
    {
        return true;
    }
    if lowered.contains("key findings for") && potential_source_mentions >= 1 {
        return true;
    }
    if potential_source_mentions >= 1
        && !lowered.contains("http://")
        && !lowered.contains("https://")
    {
        return true;
    }
    if lowered.contains("key findings for")
        && !lowered.contains("http://")
        && !lowered.contains("https://")
    {
        return true;
    }
    let markers = [
        "duckduckgo all regions",
        "all regions argentina",
        "all regions australia",
        "all regions canada",
        "safe search",
        "any time",
    ];
    let hits = markers
        .iter()
        .filter(|marker| lowered.contains(**marker))
        .count();
    hits >= 2
}

fn user_facing_tool_failure_summary(_tool_name: &str, _payload: &Value) -> Option<String> {
    None
}

fn transient_tool_failure(payload: &Value) -> bool {
    if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        return false;
    }
    let lowered = tool_error_text(payload).to_ascii_lowercase();
    lowered.contains("aborted")
        || lowered.contains("timeout")
        || lowered.contains("timed out")
        || lowered.contains("temporar")
        || lowered.contains("unavailable")
        || lowered.contains("network")
        || lowered.contains("connection")
        || lowered.contains("retry")
        || lowered.contains("econnreset")
        || lowered.contains("request_read_failed")
        || lowered.contains("resource temporarily unavailable")
        || lowered.contains("os error 35")
}
