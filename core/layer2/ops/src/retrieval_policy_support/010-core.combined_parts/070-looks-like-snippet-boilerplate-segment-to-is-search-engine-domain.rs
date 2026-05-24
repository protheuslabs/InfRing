
// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/ops (retrieval policy support).

fn search_domain_capture_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)\b(?:https?://)?(?:www\.)?([a-z0-9][a-z0-9.-]*\.[a-z]{2,})(?:/[^\s]*)?")
            .expect("search-domain-regex")
    })
}

fn extract_domains_from_text(text: &str, max_domains: usize) -> Vec<String> {
    if max_domains == 0 {
        return Vec::new();
    }
    let mut out = Vec::<String>::new();
    let mut seen = HashSet::<String>::new();
    for capture in search_domain_capture_regex().captures_iter(text) {
        let host = capture
            .get(1)
            .map(|row| row.as_str())
            .unwrap_or("")
            .trim()
            .trim_matches('.')
            .to_ascii_lowercase();
        if host.is_empty() || host == "duckduckgo.com" || host.ends_with(".duckduckgo.com") {
            continue;
        }
        if !seen.insert(host.clone()) {
            continue;
        }
        out.push(host);
        if out.len() >= max_domains {
            break;
        }
    }
    out
}

fn is_search_engine_domain(domain: &str) -> bool {
    let normalized = clean_text(domain, 120).to_ascii_lowercase();
    matches!(
        normalized.as_str(),
        "duckduckgo.com"
            | "lite.duckduckgo.com"
            | "bing.com"
            | "www.bing.com"
            | "google.com"
            | "www.google.com"
            | "search.yahoo.com"
            | "yahoo.com"
            | "search.brave.com"
            | "brave.com"
    )
}
