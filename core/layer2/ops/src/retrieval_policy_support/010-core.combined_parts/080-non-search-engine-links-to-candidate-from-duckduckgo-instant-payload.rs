// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/ops (retrieval policy support).

fn percent_decode_wrapper_component(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out = String::new();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = &raw[i + 1..i + 3];
            if let Ok(v) = u8::from_str_radix(hex, 16) {
                out.push(v as char);
                i += 3;
                continue;
            }
        }
        if bytes[i] == b'+' {
            out.push(' ');
        } else {
            out.push(bytes[i] as char);
        }
        i += 1;
    }
    out
}

fn extract_http_candidate_from_wrapper_text(text: &str) -> Option<String> {
    let lowered = text.to_ascii_lowercase();
    let start = lowered
        .find("https://")
        .or_else(|| lowered.find("http://"))?;
    let tail = &text[start..];
    let end = tail
        .find(|ch: char| ch.is_whitespace() || matches!(ch, '"' | '\'' | '<' | '>'))
        .unwrap_or(tail.len());
    let out = clean_text(&tail[..end], 2_200);
    if out.starts_with("http://") || out.starts_with("https://") {
        Some(out)
    } else {
        None
    }
}

fn first_decoded_wrapper_url(raw: &str) -> Option<String> {
    let decoded = percent_decode_wrapper_component(raw);
    let mut candidate =
        extract_http_candidate_from_wrapper_text(&decoded).unwrap_or_else(|| clean_text(&decoded, 2_200));
    if !(candidate.starts_with("http://") || candidate.starts_with("https://")) {
        return None;
    }
    let lowered = candidate.to_ascii_lowercase();
    let mut split_at = None;
    for marker in ["https://", "http://"] {
        let start = marker.len();
        if let Some(pos) = lowered
            .get(start..)
            .and_then(|tail| tail.find(marker).map(|offset| start + offset))
        {
            split_at = Some(split_at.map_or(pos, |current: usize| current.min(pos)));
        }
    }
    if let Some(pos) = split_at {
        candidate.truncate(pos);
    }
    let candidate = clean_text(
        candidate.trim_matches(|ch: char| matches!(ch, ')' | ']' | '}' | ',' | ';')),
        2_200,
    );
    if candidate.starts_with("http://") || candidate.starts_with("https://") {
        Some(candidate)
    } else {
        None
    }
}

fn decode_wrapper_base64_candidate(token: &str) -> Option<String> {
    use base64::engine::general_purpose::{STANDARD, URL_SAFE, URL_SAFE_NO_PAD};
    use base64::Engine;

    let trimmed = token.trim().trim_matches('/');
    for decoder in [&URL_SAFE_NO_PAD, &URL_SAFE, &STANDARD] {
        if let Ok(bytes) = decoder.decode(trimmed.as_bytes()) {
            let decoded = String::from_utf8_lossy(&bytes).to_string();
            if let Some(url) = extract_http_candidate_from_wrapper_text(&decoded) {
                return Some(url);
            }
        }
    }
    for pad in ["=", "==", "==="] {
        let padded = format!("{trimmed}{pad}");
        if let Ok(bytes) = URL_SAFE.decode(padded.as_bytes()) {
            let decoded = String::from_utf8_lossy(&bytes).to_string();
            if let Some(url) = extract_http_candidate_from_wrapper_text(&decoded) {
                return Some(url);
            }
        }
    }
    None
}

fn decode_wrapper_query_param(url: &str, include_continue: bool) -> Option<String> {
    let (_, query) = url.split_once('?')?;
    for part in query.split('&') {
        let mut chunks = part.splitn(2, '=');
        let key = chunks.next().unwrap_or_default();
        let value = chunks.next().unwrap_or_default();
        let key_allowed = matches!(key, "url" | "u" | "q" | "href" | "target")
            || (include_continue && key == "continue")
            || key == "uddg";
        if key_allowed {
            if let Some(candidate) = first_decoded_wrapper_url(value) {
                return Some(candidate);
            }
        }
    }
    None
}

fn decode_citation_wrapper_once(url: &str) -> Option<String> {
    let cleaned = clean_text(url, 2_200);
    if cleaned.is_empty() {
        return None;
    }
    let (_, host, path_raw, query) = parse_page_extraction_http_url(&cleaned)?;
    let host = host.trim_start_matches("www.").to_ascii_lowercase();
    let path = path_raw.to_ascii_lowercase();
    let query_lower = query.unwrap_or("").to_ascii_lowercase();

    if host == "news.google.com" {
        if let Some(decoded) = decode_wrapper_query_param(&cleaned, true) {
            return Some(decoded);
        }
        if path.contains("/rss/articles/") || path.contains("/articles/") || path.contains("/read/")
        {
            let token = path_raw
                .split('/')
                .filter(|segment| !segment.is_empty())
                .next_back()
                .unwrap_or_default();
            if let Some(decoded) = decode_wrapper_base64_candidate(token) {
                return Some(decoded);
            }
        }
    }

    if (host == "google.com" || host == "www.google.com")
        && (path.contains("/url") || query_lower.contains("url=") || query_lower.contains("q=http"))
    {
        return decode_wrapper_query_param(&cleaned, false);
    }

    if host == "duckduckgo.com" && (path.contains("/l/") || query_lower.contains("uddg=")) {
        return decode_wrapper_query_param(&cleaned, false);
    }

    if social_share_wrapper_host(&host)
        && (path.contains("/l.php")
            || path.contains("/share")
            || path.contains("/sharer")
            || query_lower.contains("url=")
            || query_lower.contains("u=")
            || query_lower.contains("href=")
            || query_lower.contains("target="))
    {
        return decode_wrapper_query_param(&cleaned, false);
    }

    None
}

fn social_share_wrapper_host(host: &str) -> bool {
    matches!(
        host.trim_start_matches("www."),
        "facebook.com" | "m.facebook.com" | "l.facebook.com" | "lm.facebook.com"
            | "twitter.com" | "x.com" | "linkedin.com"
    )
}

fn decode_citation_wrapper_url(url: &str, max_depth: usize) -> Option<String> {
    let mut current = clean_text(url, 2_200);
    if current.is_empty() {
        return None;
    }
    for _ in 0..max_depth.max(1) {
        let Some(decoded) = decode_citation_wrapper_once(&current) else {
            break;
        };
        if decoded == current {
            break;
        }
        current = decoded;
    }
    if current.starts_with("http://") || current.starts_with("https://") {
        Some(current)
    } else {
        None
    }
}

fn canonical_search_result_locator(primary: &str, fallbacks: &[&str]) -> String {
    let primary_clean = clean_text(primary, 2_200);
    let primary_is_wrapper = citation_wrapper_link(&primary_clean);
    if !primary_clean.is_empty() && !primary_is_wrapper {
        return primary_clean;
    }
    if let Some(decoded) = decode_citation_wrapper_url(&primary_clean, 4) {
        if !citation_wrapper_link(&decoded) {
            return decoded;
        }
    }
    for fallback in fallbacks {
        let cleaned = clean_text(fallback, 2_200);
        if cleaned.is_empty() {
            continue;
        }
        if !citation_wrapper_link(&cleaned) {
            return cleaned;
        }
        if let Some(decoded) = decode_citation_wrapper_url(&cleaned, 4) {
            if !citation_wrapper_link(&decoded) {
                return decoded;
            }
        }
    }
    primary_clean
}

fn non_search_engine_links(payload: &Value, max_links: usize) -> Vec<String> {
    if max_links == 0 {
        return Vec::new();
    }
    let mut out = Vec::<String>::new();
    let mut seen = HashSet::<String>::new();
    let push_link = |raw: &str, out: &mut Vec<String>, seen: &mut HashSet<String>| {
        let link = canonical_search_result_locator(raw, &[]);
        let Some(link) = normalize_document_candidate_link(&link) else {
            return;
        };
        if link.is_empty() || !seen.insert(link.to_ascii_lowercase()) {
            return;
        }
        let domain = extract_domains_from_text(&link, 1)
            .into_iter()
            .next()
            .unwrap_or_default();
        if domain.is_empty() || is_search_engine_domain(&domain) {
            return;
        }
        out.push(link);
    };
    for row in payload
        .get("links")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        push_link(row.as_str().unwrap_or(""), &mut out, &mut seen);
        if out.len() >= max_links.max(1) {
            break;
        }
    }
    if out.len() < max_links.max(1) {
        let origin = payload_base_origin(payload);
        for link in payload_text_links(payload, max_links.saturating_mul(4).max(max_links), origin.as_deref()) {
            push_link(&link, &mut out, &mut seen);
            if out.len() >= max_links.max(1) {
                break;
            }
        }
    }
    out
}

fn payload_base_origin(payload: &Value) -> Option<String> {
    let requested_url = payload
        .get("requested_url")
        .or_else(|| payload.pointer("/receipt/requested_url"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let (scheme, host, _, _) = parse_page_extraction_http_url(requested_url)?;
    Some(format!("{scheme}://{host}"))
}

fn payload_text_links(payload: &Value, max_links: usize, origin: Option<&str>) -> Vec<String> {
    if max_links == 0 {
        return Vec::new();
    }
    let mut out = Vec::<String>::new();
    let mut seen = HashSet::<String>::new();
    let text = clean_text(
        &[
            payload.get("summary").and_then(Value::as_str).unwrap_or(""),
            payload.get("content").and_then(Value::as_str).unwrap_or(""),
            payload
                .get("content_preview")
                .and_then(Value::as_str)
                .unwrap_or(""),
            payload.get("markdown").and_then(Value::as_str).unwrap_or(""),
            payload.get("text").and_then(Value::as_str).unwrap_or(""),
        ]
        .join(" "),
        8_000,
    );
    for link in http_links_from_text(&text)
        .into_iter()
        .chain(relative_links_from_text(&text, origin).into_iter())
    {
        if seen.insert(link.to_ascii_lowercase()) {
            out.push(link);
            if out.len() >= max_links.max(1) {
                break;
            }
        }
    }
    out
}

fn http_links_from_text(text: &str) -> Vec<String> {
    static URL_RE: OnceLock<Regex> = OnceLock::new();
    let re = URL_RE.get_or_init(|| Regex::new(r#"https?://[^\s<>\)\]\}"']+"#).expect("url"));
    re.find_iter(text)
        .map(|matched| {
            matched
                .as_str()
                .trim_matches(|ch: char| matches!(ch, ',' | '.' | ';' | ':' | ')' | ']' | '}'))
                .to_string()
        })
        .collect()
}

fn relative_links_from_text(text: &str, origin: Option<&str>) -> Vec<String> {
    let Some(origin) = origin else {
        return Vec::new();
    };
    text.split_whitespace()
        .filter_map(|token| {
            let trimmed = token.trim_matches(|ch: char| {
                matches!(ch, '(' | ')' | '[' | ']' | '{' | '}' | '"' | '\'' | ',' | '.' | ';' | ':')
            });
            if !trimmed.starts_with('/') || trimmed.starts_with("//") || trimmed.len() < 6 {
                return None;
            }
            if !page_extraction_link_has_article_like_path(&format!("{origin}{trimmed}")) {
                return None;
            }
            Some(format!("{origin}{trimmed}"))
        })
        .collect()
}

fn normalize_document_candidate_link(link: &str) -> Option<String> {
    let mut cleaned = clean_text(link, 2_200);
    if cleaned.is_empty() {
        return None;
    }
    let lowered = cleaned.to_ascii_lowercase();
    if !(lowered.starts_with("http://") || lowered.starts_with("https://")) {
        return None;
    }
    if let Some((without_fragment, _)) = cleaned.split_once('#') {
        cleaned = without_fragment.to_string();
    }
    let without_query = cleaned
        .split_once('?')
        .map(|(value, _)| value)
        .unwrap_or(cleaned.as_str())
        .to_ascii_lowercase();
    let excluded_extensions = [
        ".png", ".jpg", ".jpeg", ".gif", ".webp", ".svg", ".ico", ".css", ".js", ".woff", ".woff2",
        ".ttf", ".mp3", ".mp4", ".avi", ".mov", ".zip", ".gz", ".tar", ".dmg", ".exe",
    ];
    if excluded_extensions
        .iter()
        .any(|extension| without_query.ends_with(extension))
    {
        return None;
    }
    Some(cleaned)
}
