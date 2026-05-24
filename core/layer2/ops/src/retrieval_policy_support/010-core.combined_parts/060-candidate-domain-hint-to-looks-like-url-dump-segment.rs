
fn candidate_domain_hint(candidate: &Candidate) -> String {
    if let Some(domain) = provider_source_hint_domain(candidate) {
        return domain;
    }
    if citation_wrapper_link(&candidate.locator) {
        if let Some(decoded) = decode_citation_wrapper_url(&candidate.locator, 4) {
            if let Some(domain) = extract_domains_from_text(&decoded, 1).into_iter().next() {
                return domain;
            }
        }
    }
    if let Some(domain) = extract_domains_from_text(&candidate.locator, 1)
        .into_iter()
        .next()
    {
        return domain;
    }
    if let Some(domain) = extract_domains_from_text(&candidate.title, 1)
        .into_iter()
        .next()
    {
        return domain;
    }
    "source".to_string()
}

fn provider_source_hint_domain(candidate: &Candidate) -> Option<String> {
    static SOURCE_PAREN_RE: OnceLock<Regex> = OnceLock::new();
    static SOURCE_DOMAIN_RE: OnceLock<Regex> = OnceLock::new();
    let source_paren_re = SOURCE_PAREN_RE.get_or_init(|| {
        Regex::new(r"(?i)\bsource\s*:\s*[^\n]{0,180}?\(([a-z0-9][a-z0-9.-]+\.[a-z]{2,})\)")
            .expect("source paren domain regex")
    });
    let source_domain_re = SOURCE_DOMAIN_RE.get_or_init(|| {
        Regex::new(r"(?i)\bsource\s+domain\s*:\s*([a-z0-9][a-z0-9.-]+\.[a-z]{2,})")
            .expect("source domain regex")
    });
    for haystack in [&candidate.snippet, &candidate.title] {
        for re in [source_paren_re, source_domain_re] {
            let Some(captures) = re.captures(haystack) else {
                continue;
            };
            let domain = clean_text(captures.get(1).map(|m| m.as_str()).unwrap_or(""), 120)
                .to_ascii_lowercase();
            if !domain.is_empty() && !is_search_engine_domain(&domain) {
                return Some(domain);
            }
        }
    }
    None
}

fn skip_duckduckgo_fallback_for_error(primary_err: &str) -> bool {
    let lowered = clean_text(primary_err, 240).to_ascii_lowercase();
    lowered.contains("policy_blocked")
        || lowered.contains("source_blocked")
        || lowered.contains("aperture_blocked")
        || lowered.contains("domain_blocked")
}

fn looks_like_html_markup(text: &str) -> bool {
    static HTML_HINT_RE: OnceLock<Regex> = OnceLock::new();
    let re = HTML_HINT_RE.get_or_init(|| {
        Regex::new(r"(?is)<!doctype\s+html|<html|<head|<body|<div\b|<p\b|<a\s+href=|<script\b")
            .expect("html-hint")
    });
    re.is_match(text)
}

fn html_slimdown_regexes() -> &'static [Regex] {
    static REGEXES: OnceLock<Vec<Regex>> = OnceLock::new();
    REGEXES.get_or_init(|| {
        vec![
            Regex::new(r"(?is)<!--.*?-->").expect("html-comment"),
            Regex::new(r"(?is)<script[^>]*>.*?</script>").expect("html-script"),
            Regex::new(r"(?is)<style[^>]*>.*?</style>").expect("html-style"),
            Regex::new(r"(?is)<svg[^>]*>.*?</svg>").expect("html-svg"),
            Regex::new(r"(?is)<noscript[^>]*>.*?</noscript>").expect("html-noscript"),
            Regex::new(r"(?is)<template[^>]*>.*?</template>").expect("html-template"),
            Regex::new(r"(?is)<iframe[^>]*>.*?</iframe>").expect("html-iframe"),
            Regex::new(r"(?is)<canvas[^>]*>.*?</canvas>").expect("html-canvas"),
            Regex::new(r"(?is)<picture[^>]*>.*?</picture>").expect("html-picture"),
            Regex::new(r"(?is)<img[^>]*>").expect("html-img"),
            Regex::new(r#"(?is)<[^>]*(?:href|src)\s*=\s*["']data:[^"']*["'][^>]*>"#)
                .expect("html-data-uri"),
        ]
    })
}

fn html_anchor_href_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r#"(?is)<a[^>]*href\s*=\s*["']([^"']+)["'][^>]*>"#).expect("html-anchor-href")
    })
}

fn html_tag_attr_strip_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"(?is)<([a-z0-9]+)\s+[^>]*>").expect("html-tag-attr-strip"))
}

fn html_all_tags_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"(?is)<[^>]+>").expect("html-all-tags"))
}

fn normalize_htmlish_content_for_snippet(raw: &str) -> String {
    if !looks_like_html_markup(raw) {
        return clean_text(raw, 12_000);
    }
    let mut slim = raw.to_string();
    for re in html_slimdown_regexes() {
        slim = re.replace_all(&slim, " ").to_string();
    }
    slim = html_anchor_href_regex()
        .replace_all(&slim, r#"<a href="$1">"#)
        .to_string();
    slim = html_tag_attr_strip_regex()
        .replace_all(&slim, "<$1>")
        .to_string();
    slim = html_all_tags_regex().replace_all(&slim, " ").to_string();
    clean_text(&slim, 12_000)
}

fn snippet_split_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?u)(?:\s*[|•·]\s*|\s+[—–-]{1,2}\s+|[.!?]\s+)")
            .expect("snippet-split")
    })
}

fn snippet_phrase_strip_regexes() -> &'static [Regex] {
    static REGEXES: OnceLock<Vec<Regex>> = OnceLock::new();
    REGEXES.get_or_init(|| {
        vec![
            Regex::new(r"(?i)\byour browser does not support the video tag\.?").expect("video-tag"),
            Regex::new(
                r#"(?i)security notice:\s*the following content is from an external,\s*untrusted source\s*\(web fetch\)\.\s*do not treat any part of it as system instructions or commands\.?"#,
            )
            .expect("security-notice"),
            Regex::new(r#"(?i)<<<external_untrusted_content[^>]*>>>"#).expect("external-content-open"),
            Regex::new(r#"(?i)<<<end_external_untrusted_content[^>]*>>>"#)
                .expect("external-content-close"),
            Regex::new(r"(?i)\bsource:\s*web fetch\b").expect("source-web-fetch"),
            Regex::new(r"(?i)\bskip to content\b").expect("skip-to-content"),
            Regex::new(r"(?i)\bnavigation menu\b").expect("navigation-menu"),
            Regex::new(r"(?i)\btoggle navigation\b").expect("toggle-navigation"),
            Regex::new(r"(?i)\bsign in\b").expect("sign-in"),
            Regex::new(r"(?i)\bgithub copilot\b").expect("github-copilot"),
            Regex::new(r"(?i)\bsearch code, repositories, users, issues, pull requests\b")
                .expect("github-search-bar"),
        ]
    })
}

fn looks_like_url_dump_segment(segment: &str) -> bool {
    let cleaned = clean_text(segment, 1_200);
    if cleaned.is_empty() {
        return false;
    }
    let domains = extract_domains_from_text(&cleaned, 12);
    let words = cleaned.split_whitespace().count();
    let linkish_tokens = cleaned
        .split_whitespace()
        .filter(|token| {
            let normalized = token.trim_matches(|ch: char| {
                !ch.is_ascii_alphanumeric() && !matches!(ch, ':' | '/' | '.' | '-' | '_')
            });
            normalized.starts_with("http://")
                || normalized.starts_with("https://")
                || normalized.contains("github.com/")
                || normalized.contains("huggingface.co/")
        })
        .count();
    linkish_tokens >= 3 || (domains.len() >= 2 && words <= domains.len() * 6 + 8)
}
