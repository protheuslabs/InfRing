fn extract_domain(raw_url: &str) -> String {
    let mut url = clean_text(raw_url, 2200).to_ascii_lowercase();
    if let Some(rest) = url.strip_prefix("http://") {
        url = rest.to_string();
    } else if let Some(rest) = url.strip_prefix("https://") {
        url = rest.to_string();
    }
    let host = url
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default()
        .split('@')
        .next_back()
        .unwrap_or_default()
        .split(':')
        .next()
        .unwrap_or_default()
        .trim_matches('.');
    clean_text(host, 220).to_ascii_lowercase()
}

fn sha256_hex(raw: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    hex::encode(hasher.finalize())
}

fn clip_bytes(raw: &str, max_bytes: usize) -> String {
    if raw.len() <= max_bytes {
        return raw.to_string();
    }
    let mut out = String::new();
    let mut used = 0usize;
    for ch in raw.chars() {
        let width = ch.len_utf8();
        if used + width > max_bytes {
            break;
        }
        out.push(ch);
        used += width;
    }
    out
}

fn regex_script() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"(?is)<script[^>]*>.*?</script>").expect("regex"))
}

fn regex_style() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"(?is)<style[^>]*>.*?</style>").expect("regex"))
}

fn regex_tags() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"(?is)<[^>]+>").expect("regex"))
}

fn clean_html_content(raw: &str, max_chars: usize) -> String {
    let no_script = regex_script().replace_all(raw, " ");
    let no_style = regex_style().replace_all(&no_script, " ");
    let no_tags = regex_tags().replace_all(&no_style, " ");
    let decoded = no_tags
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'");
    clean_text(&decoded, max_chars)
}

fn summarize_text(text: &str, max_chars: usize) -> String {
    let cleaned = clean_text(text, max_chars.max(200));
    if cleaned.is_empty() {
        return String::new();
    }
    let mut sentences = Vec::<String>::new();
    let mut current = String::new();
    for ch in cleaned.chars() {
        current.push(ch);
        if matches!(ch, '.' | '!' | '?') {
            let sentence = clean_text(&current, 280);
            if !sentence.is_empty() {
                sentences.push(sentence);
            }
            current.clear();
            if sentences.len() >= 5 {
                break;
            }
        }
    }
    if sentences.is_empty() {
        return clean_text(&cleaned, 320);
    }
    clean_text(&sentences.join(" "), max_chars)
}

fn persist_artifact(
    root: &Path,
    requested_url: &str,
    response_hash: &str,
    content: &str,
) -> Option<Value> {
    if response_hash.trim().is_empty() || content.trim().is_empty() {
        return None;
    }
    let artifact_id = format!(
        "web-{}",
        response_hash
            .chars()
            .take(16)
            .collect::<String>()
            .to_ascii_lowercase()
    );
    let dir = artifacts_dir_path(root);
    if fs::create_dir_all(&dir).is_err() {
        return None;
    }
    let path = dir.join(format!("{artifact_id}.txt"));
    if !path.exists() {
        if fs::write(&path, content.as_bytes()).is_err() {
            return None;
        }
    }
    Some(json!({
        "artifact_id": artifact_id,
        "path": crate::rel_path(root, &path),
        "bytes": content.len(),
        "source_url": clean_text(requested_url, 2200)
    }))
}

fn encode_query_component(raw: &str) -> String {
    let mut out = String::new();
    for byte in raw.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(byte as char);
        } else if byte == b' ' {
            out.push('+');
        } else {
            out.push('%');
            out.push_str(&format!("{byte:02X}"));
        }
    }
    out
}

fn web_search_url(query: &str) -> String {
    format!(
        "https://duckduckgo.com/html/?q={}",
        encode_query_component(&clean_text(query, 600))
    )
}

fn web_search_lite_url(query: &str) -> String {
    format!(
        "https://lite.duckduckgo.com/lite/?q={}",
        encode_query_component(&clean_text(query, 600))
    )
}

fn web_search_bing_rss_url(query: &str) -> String {
    format!(
        "https://www.bing.com/search?q={}&format=rss&setlang=en-US",
        encode_query_component(&clean_text(query, 600))
    )
}

fn web_search_bing_html_url(query: &str, top_k: usize) -> String {
    format!(
        "https://www.bing.com/search?q={}&count={}&setlang=en-US",
        encode_query_component(&clean_text(query, 600)),
        top_k.clamp(1, 50)
    )
}

fn web_search_google_news_rss_url(query: &str) -> String {
    format!(
        "https://news.google.com/rss/search?q={}&hl=en-US&gl=US&ceid=US:en",
        encode_query_component(&clean_text(query, 600))
    )
}

fn normalize_allowed_domains(raw: &Value) -> Vec<String> {
    let rows = if let Some(array) = raw.as_array() {
        array
            .iter()
            .filter_map(|row| row.as_str().map(|v| v.to_string()))
            .collect::<Vec<_>>()
    } else if let Some(single) = raw.as_str() {
        single
            .split(|ch: char| ch == ',' || ch.is_ascii_whitespace())
            .map(str::trim)
            .filter(|row| !row.is_empty())
            .map(ToString::to_string)
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    rows.into_iter()
        .map(|v| clean_text(v.as_str(), 180).to_ascii_lowercase())
        .map(|row| {
            row.trim()
                .trim_start_matches("http://")
                .trim_start_matches("https://")
                .trim_start_matches("www.")
                .trim_start_matches("*.")
                .split('/')
                .next()
                .unwrap_or("")
                .trim()
                .to_string()
        })
        .filter(|row| {
            !row.is_empty()
                && row.contains('.')
                && row
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-'))
        })
        .fold(Vec::<String>::new(), |mut acc, row| {
            if !acc.iter().any(|existing| existing == &row) {
                acc.push(row);
            }
            acc
        })
}

fn scoped_search_query(
    query: &str,
    allowed_domains: &[String],
    exclude_subdomains: bool,
) -> String {
    let cleaned = clean_text(query, 600);
    if cleaned.is_empty() || allowed_domains.is_empty() {
        return cleaned;
    }
    let scope = allowed_domains
        .iter()
        .map(|domain| {
            if exclude_subdomains {
                format!("(site:{domain} -site:*.{domain})")
            } else {
                format!("site:{domain}")
            }
        })
        .collect::<Vec<_>>()
        .join(" OR ");
    clean_text(format!("({scope}) {cleaned}").as_str(), 900)
}

fn domain_matches_filter(domain: &str, filter: &str, exclude_subdomains: bool) -> bool {
    if domain == filter {
        return true;
    }
    if exclude_subdomains {
        return false;
    }
    domain
        .strip_suffix(filter)
        .map(|prefix| prefix.ends_with('.'))
        .unwrap_or(false)
}

fn domain_allowed_for_scope(
    raw_url: &str,
    allowed_domains: &[String],
    exclude_subdomains: bool,
) -> bool {
    if allowed_domains.is_empty() {
        return true;
    }
    let domain = extract_domain(raw_url);
    if domain.is_empty() {
        return false;
    }
    allowed_domains
        .iter()
        .any(|filter| domain_matches_filter(&domain, filter, exclude_subdomains))
}

fn render_search_row(title: &str, snippet: &str, link: &str) -> String {
    let title = clean_text(title, 220);
    let snippet = clean_text(snippet, 420);
    let link = clean_text(link, 2_200);
    if title.is_empty() && snippet.is_empty() {
        return clean_text(&link, 1_200);
    }
    if snippet.is_empty() {
        return clean_text(format!("{title} — {link}").as_str(), 1_200);
    }
    if title.is_empty() {
        return clean_text(format!("{link} — {snippet}").as_str(), 1_200);
    }
    clean_text(format!("{title} — {link} — {snippet}").as_str(), 1_200)
}

fn push_unique_link_domain(domains: &mut Vec<String>, link: &str) {
    let domain = extract_domain(link);
    if !domain.is_empty() && !domains.iter().any(|existing| existing == &domain) {
        domains.push(domain);
    }
}
