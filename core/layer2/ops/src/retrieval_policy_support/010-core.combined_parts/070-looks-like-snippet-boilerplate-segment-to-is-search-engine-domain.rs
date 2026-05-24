
fn looks_like_snippet_boilerplate_segment(segment: &str, locator_hint: &str) -> bool {
    let lowered = clean_text(segment, 600).to_ascii_lowercase();
    if lowered.is_empty() {
        return true;
    }
    if looks_like_url_dump_segment(&lowered) {
        return true;
    }
    if lowered.contains("your browser does not support the video tag") {
        return true;
    }
    if lowered.starts_with("security notice:")
        || lowered.starts_with("source: web fetch")
        || lowered.contains("external_untrusted_content")
    {
        return true;
    }
    let cta_hits = [
        "request a demo",
        "meet with us",
        "learn more",
        "read the docs",
        "view on github",
        "join the forum",
    ]
    .iter()
    .filter(|marker| lowered.contains(**marker))
    .count();
    if framework_name_hits(&lowered) == 0
        && !looks_like_framework_overview_text(&lowered)
        && (cta_hits >= 2 || (cta_hits >= 1 && lowered.split_whitespace().count() <= 18))
    {
        return true;
    }
    let github_like = locator_hint.to_ascii_lowercase().contains("github.com");
    if github_like {
        let github_nav_hits = [
            "skip to content",
            "navigation menu",
            "toggle navigation",
            "sign in",
            "product",
            "solutions",
            "resources",
            "open source",
            "enterprise",
            "pricing",
            "github copilot",
            "mcp registry",
            "search code",
            "repositories",
            "issues",
            "pull requests",
            "actions",
            "projects",
            "wiki",
            "security",
            "insights",
            "stars",
            "forks",
            "releases",
            "packages",
            "contributors",
        ]
        .iter()
        .filter(|marker| lowered.contains(**marker))
        .count();
        if github_nav_hits >= 3 && framework_name_hits(&lowered) == 0 {
            return true;
        }
        if framework_name_hits(&lowered) == 0
            && !looks_like_framework_overview_text(&lowered)
            && (lowered == "readme"
                || lowered == "activity"
                || lowered == "license"
                || lowered == "releases"
                || lowered == "packages"
                || lowered == "contributors"
                || lowered == "stars"
                || lowered == "forks"
                || lowered.contains("mit license")
                || lowered.contains("apache-2.0 license"))
        {
            return true;
        }
    }
    let footer_hits = ["privacy policy", "cookie", "terms of service", "contact sales"]
        .iter()
        .filter(|marker| lowered.contains(**marker))
        .count();
    footer_hits >= 2
}

fn summary_should_defer_to_content(summary: &str) -> bool {
    let cleaned = clean_text(summary, 1_800);
    if cleaned.is_empty() {
        return false;
    }
    let lowered = cleaned.to_ascii_lowercase();
    lowered.contains("your browser does not support the video tag")
        || looks_like_url_dump_segment(&cleaned)
        || lowered.starts_with("security notice:")
        || (lowered.starts_with("extracted ")
            && lowered.contains(" characters from ")
            && (lowered.contains("pdf") || lowered.contains("document")))
}

fn normalize_snippet_text(raw: &str, query: &str, locator_hint: &str) -> String {
    let mut cleaned = clean_text(raw, 12_000);
    if cleaned.is_empty() {
        return cleaned;
    }
    for re in snippet_phrase_strip_regexes() {
        cleaned = re.replace_all(&cleaned, " ").to_string();
    }
    cleaned = clean_text(&cleaned, 12_000);
    if cleaned.is_empty() {
        return cleaned;
    }
    let segments = snippet_split_regex()
        .split(&cleaned)
        .map(|row| {
            clean_text(
                row.trim()
                    .trim_start_matches(|ch| matches!(ch, '-' | '—' | '–'))
                    .trim(),
                400,
            )
        })
        .filter(|row| !row.is_empty())
        .filter(|row| !looks_like_snippet_boilerplate_segment(row, locator_hint))
        .take(8)
        .collect::<Vec<_>>();
    if segments.is_empty() {
        return cleaned;
    }
    let mut preferred = Vec::<String>::new();
    if is_framework_catalog_intent(query) {
        for row in &segments {
            let combined = format!("{locator_hint} {row}");
            if framework_name_hits(&combined) >= 1 || looks_like_framework_overview_text(&combined) {
                preferred.push(row.clone());
            }
            if preferred.len() >= 2 {
                break;
            }
        }
    }
    let selected = if preferred.is_empty() { segments } else { preferred };
    trim_words(&selected.into_iter().take(2).collect::<Vec<_>>().join(". "), 72)
}

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
