// Layer ownership: core/layer0/ops::browser-serp-link-utils (authoritative)

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
