// Layer ownership: core/layer0/ops::web-conduit-provider-parsers (authoritative)
fn render_serper_payload(
    body: &str,
    allowed_domains: &[String],
    exclude_subdomains: bool,
    top_k: usize,
    max_response_bytes: usize,
) -> Value {
    let parsed = match serde_json::from_str::<Value>(body) {
        Ok(value) => value,
        Err(_) => {
            return json!({
                "ok": false,
                "error": "serper_decode_failed",
                "summary": "",
                "content": "",
                "links": [],
                "content_domains": [],
                "provider_raw_count": 0,
                "provider_filtered_count": 0
            });
        }
    };
    let organic = parsed
        .get("organic")
        .or_else(|| parsed.get("results"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut lines = Vec::<String>::new();
    let mut links = Vec::<String>::new();
    let mut domains = Vec::<String>::new();
    for row in &organic {
        let link = normalize_search_result_link(
            row.get("link").and_then(Value::as_str).unwrap_or(""),
        );
        if link.is_empty() || !domain_allowed_for_scope(&link, allowed_domains, exclude_subdomains)
        {
            continue;
        }
        let rendered = render_search_row(
            row.get("title").and_then(Value::as_str).unwrap_or(""),
            row.get("snippet").and_then(Value::as_str).unwrap_or(""),
            &link,
        );
        if rendered.is_empty() {
            continue;
        }
        lines.push(rendered);
        links.push(link.clone());
        push_unique_link_domain(&mut domains, &link);
        if lines.len() >= top_k.max(1) {
            break;
        }
    }
    let content = clean_text(&lines.join("\n"), max_response_bytes.min(120_000));
    let ok = !content.is_empty();
    json!({
        "ok": ok,
        "summary": if ok {
            summarize_text(&content, 900)
        } else {
            crate::tool_output_match_filter::no_findings_user_copy().to_string()
        },
        "content": content,
        "links": links,
        "results": organic,
        "content_domains": domains,
        "provider_raw_count": organic.len(),
        "provider_filtered_count": lines.len(),
        "error": if ok {
            Value::Null
        } else {
            Value::String("no_relevant_results".to_string())
        }
    })
}

fn push_json_search_row(
    lines: &mut Vec<String>,
    links: &mut Vec<String>,
    domains: &mut Vec<String>,
    title: &str,
    snippet: &str,
    link: &str,
    allowed_domains: &[String],
    exclude_subdomains: bool,
) {
    let link = normalize_search_result_link(link);
    if link.is_empty() || !domain_allowed_for_scope(&link, allowed_domains, exclude_subdomains) {
        return;
    }
    let rendered = render_search_row(title, snippet, &link);
    if rendered.is_empty() {
        return;
    }
    lines.push(rendered);
    links.push(link.clone());
    push_unique_link_domain(domains, &link);
}

fn search_row_text(row: &Value, keys: &[&str], max_chars: usize) -> String {
    for key in keys {
        if let Some(text) = row.get(*key).and_then(Value::as_str) {
            let cleaned = clean_text(text, max_chars);
            if !cleaned.is_empty() {
                return cleaned;
            }
        }
    }
    String::new()
}

fn search_row_array_text(row: &Value, key: &str, max_chars: usize) -> String {
    row.get(key)
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|text| clean_text(text, max_chars / 2))
                .filter(|text| !text.is_empty())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .map(|text| clean_text(&text, max_chars))
        .unwrap_or_default()
}

fn render_tavily_payload(
    body: &str,
    allowed_domains: &[String],
    exclude_subdomains: bool,
    top_k: usize,
    max_response_bytes: usize,
) -> Value {
    let parsed = match serde_json::from_str::<Value>(body) {
        Ok(value) => value,
        Err(_) => {
            return json!({
                "ok": false,
                "error": "tavily_decode_failed",
                "summary": "",
                "content": "",
                "links": [],
                "content_domains": [],
                "provider_raw_count": 0,
                "provider_filtered_count": 0
            });
        }
    };
    let results = parsed
        .get("results")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut lines = Vec::<String>::new();
    let mut links = Vec::<String>::new();
    let mut domains = Vec::<String>::new();
    for row in &results {
        let snippet = search_row_text(row, &["content", "raw_content"], 900);
        push_json_search_row(
            &mut lines,
            &mut links,
            &mut domains,
            row.get("title").and_then(Value::as_str).unwrap_or(""),
            &snippet,
            row.get("url").and_then(Value::as_str).unwrap_or(""),
            allowed_domains,
            exclude_subdomains,
        );
        if lines.len() >= top_k.max(1) {
            break;
        }
    }
    let content = clean_text(&lines.join("\n"), max_response_bytes.min(120_000));
    let ok = !content.is_empty();
    json!({
        "ok": ok,
        "summary": if ok { summarize_text(&content, 900) } else { crate::tool_output_match_filter::no_findings_user_copy().to_string() },
        "content": content,
        "links": links,
        "results": results,
        "content_domains": domains,
        "provider_raw_count": results.len(),
        "provider_filtered_count": lines.len(),
        "provider_answer_present": parsed.get("answer").and_then(Value::as_str).map(|text| !clean_text(text, 200).is_empty()).unwrap_or(false),
        "provider_request_id": parsed.get("request_id").cloned().unwrap_or(Value::Null),
        "error": if ok { Value::Null } else { Value::String("no_relevant_results".to_string()) }
    })
}

fn render_exa_payload(
    body: &str,
    allowed_domains: &[String],
    exclude_subdomains: bool,
    top_k: usize,
    max_response_bytes: usize,
) -> Value {
    let parsed = match serde_json::from_str::<Value>(body) {
        Ok(value) => value,
        Err(_) => {
            return json!({
                "ok": false,
                "error": "exa_decode_failed",
                "summary": "",
                "content": "",
                "links": [],
                "content_domains": [],
                "provider_raw_count": 0,
                "provider_filtered_count": 0
            });
        }
    };
    let results = parsed
        .get("results")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut lines = Vec::<String>::new();
    let mut links = Vec::<String>::new();
    let mut domains = Vec::<String>::new();
    for row in &results {
        let mut snippet = search_row_text(row, &["summary", "text"], 900);
        if snippet.is_empty() {
            snippet = search_row_array_text(row, "highlights", 900);
        }
        push_json_search_row(
            &mut lines,
            &mut links,
            &mut domains,
            row.get("title").and_then(Value::as_str).unwrap_or(""),
            &snippet,
            row.get("url").and_then(Value::as_str).unwrap_or(""),
            allowed_domains,
            exclude_subdomains,
        );
        if lines.len() >= top_k.max(1) {
            break;
        }
    }
    let content = clean_text(&lines.join("\n"), max_response_bytes.min(120_000));
    let ok = !content.is_empty();
    json!({
        "ok": ok,
        "summary": if ok { summarize_text(&content, 900) } else { crate::tool_output_match_filter::no_findings_user_copy().to_string() },
        "content": content,
        "links": links,
        "results": results,
        "content_domains": domains,
        "provider_raw_count": results.len(),
        "provider_filtered_count": lines.len(),
        "provider_request_id": parsed.get("requestId").cloned().unwrap_or(Value::Null),
        "error": if ok { Value::Null } else { Value::String("no_relevant_results".to_string()) }
    })
}

fn render_brave_payload(
    body: &str,
    allowed_domains: &[String],
    exclude_subdomains: bool,
    top_k: usize,
    max_response_bytes: usize,
) -> Value {
    let parsed = match serde_json::from_str::<Value>(body) {
        Ok(value) => value,
        Err(_) => {
            return json!({
                "ok": false,
                "error": "brave_decode_failed",
                "summary": "",
                "content": "",
                "links": [],
                "content_domains": [],
                "provider_raw_count": 0,
                "provider_filtered_count": 0
            });
        }
    };
    let results = parsed
        .pointer("/web/results")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut lines = Vec::<String>::new();
    let mut links = Vec::<String>::new();
    let mut domains = Vec::<String>::new();
    for row in &results {
        let mut snippet = search_row_text(row, &["description"], 900);
        let extra = search_row_array_text(row, "extra_snippets", 900);
        if !extra.is_empty() {
            snippet = clean_text(format!("{snippet} {extra}").as_str(), 900);
        }
        push_json_search_row(
            &mut lines,
            &mut links,
            &mut domains,
            row.get("title").and_then(Value::as_str).unwrap_or(""),
            &snippet,
            row.get("url").and_then(Value::as_str).unwrap_or(""),
            allowed_domains,
            exclude_subdomains,
        );
        if lines.len() >= top_k.max(1) {
            break;
        }
    }
    let content = clean_text(&lines.join("\n"), max_response_bytes.min(120_000));
    let ok = !content.is_empty();
    json!({
        "ok": ok,
        "summary": if ok { summarize_text(&content, 900) } else { crate::tool_output_match_filter::no_findings_user_copy().to_string() },
        "content": content,
        "links": links,
        "results": results,
        "content_domains": domains,
        "provider_raw_count": results.len(),
        "provider_filtered_count": lines.len(),
        "more_results_available": parsed.pointer("/query/more_results_available").cloned().unwrap_or(Value::Null),
        "error": if ok { Value::Null } else { Value::String("no_relevant_results".to_string()) }
    })
}

fn decode_xml_entities(raw: &str) -> String {
    raw.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

fn extract_xml_tag_value(block: &str, tag: &str) -> String {
    let pattern = format!(r"(?is)<{tag}[^>]*>(.*?)</{tag}>");
    let Ok(re) = Regex::new(&pattern) else {
        return String::new();
    };
    let Some(captures) = re.captures(block) else {
        return String::new();
    };
    let raw = captures.get(1).map(|m| m.as_str()).unwrap_or("");
    let trimmed = raw
        .trim()
        .trim_start_matches("<![CDATA[")
        .trim_end_matches("]]>");
    clean_html_content(&decode_xml_entities(trimmed), 2_400)
}

fn extract_xml_tag_attr_value(block: &str, tag: &str, attr: &str) -> String {
    let pattern = format!(r#"(?is)<{tag}\b[^>]*\b{attr}\s*=\s*["']([^"']+)["'][^>]*>"#);
    let Ok(re) = Regex::new(&pattern) else {
        return String::new();
    };
    let Some(captures) = re.captures(block) else {
        return String::new();
    };
    clean_text(
        &decode_xml_entities(captures.get(1).map(|m| m.as_str()).unwrap_or("")),
        2_200,
    )
}

fn render_google_news_rss_payload(
    body: &str,
    allowed_domains: &[String],
    exclude_subdomains: bool,
    top_k: usize,
    max_response_bytes: usize,
) -> Value {
    static ITEM_RE: OnceLock<Regex> = OnceLock::new();
    let item_re =
        ITEM_RE.get_or_init(|| Regex::new(r"(?is)<item\b[^>]*>(.*?)</item>").expect("item regex"));
    let mut lines = Vec::<String>::new();
    let mut links = Vec::<String>::new();
    let mut domains = Vec::<String>::new();
    let mut results = Vec::<Value>::new();
    let mut raw_count = 0usize;
    for captures in item_re.captures_iter(body) {
        raw_count += 1;
        let item = captures.get(1).map(|m| m.as_str()).unwrap_or("");
        let link = normalize_search_result_link(&extract_xml_tag_value(item, "link"));
        let title = extract_xml_tag_value(item, "title");
        let source = extract_xml_tag_value(item, "source");
        let source_url = extract_xml_tag_attr_value(item, "source", "url");
        let source_domain = extract_domain(&source_url);
        if link.is_empty() || !domain_allowed_for_scope(&link, allowed_domains, exclude_subdomains)
        {
            continue;
        }
        let mut snippet = extract_xml_tag_value(item, "description");
        let pub_date = extract_xml_tag_value(item, "pubDate");
        let source_hint = if source.is_empty() && source_domain.is_empty() {
            String::new()
        } else if source_domain.is_empty() {
            format!("Source: {source}.")
        } else if source.is_empty() {
            format!("Source domain: {source_domain}.")
        } else {
            format!("Source: {source} ({source_domain}).")
        };
        if !pub_date.is_empty() {
            snippet = clean_text(format!("{snippet} Published: {pub_date}.").as_str(), 900);
        }
        if !source_hint.is_empty() {
            snippet = clean_text(format!("{snippet} {source_hint}").as_str(), 900);
        }
        let rendered = render_search_row(&title, &snippet, &link);
        if rendered.is_empty() {
            continue;
        }
        lines.push(rendered);
        links.push(link.clone());
        results.push(json!({
            "title": title,
            "url": link,
            "snippet": snippet,
            "source_name": source,
            "source_url": source_url,
            "source_domain": source_domain,
            "published": pub_date
        }));
        if source_domain.is_empty() {
            push_unique_link_domain(&mut domains, &link);
        } else if !domains.iter().any(|existing| existing == &source_domain) {
            domains.push(source_domain);
        }
        if lines.len() >= top_k.max(1) {
            break;
        }
    }
    let content = clean_text(&lines.join("\n"), max_response_bytes.min(120_000));
    let ok = !content.is_empty();
    json!({
        "ok": ok,
        "summary": if ok {
            summarize_text(&content, 900)
        } else {
            crate::tool_output_match_filter::no_findings_user_copy().to_string()
        },
        "content": content,
        "links": links,
        "results": results,
        "content_domains": domains,
        "provider_raw_count": raw_count,
        "provider_filtered_count": lines.len(),
        "error": if ok {
            Value::Null
        } else {
            Value::String("no_relevant_results".to_string())
        }
    })
}

fn render_bing_rss_payload(
    body: &str,
    allowed_domains: &[String],
    exclude_subdomains: bool,
    top_k: usize,
    max_response_bytes: usize,
) -> Value {
    static ITEM_RE: OnceLock<Regex> = OnceLock::new();
    let item_re =
        ITEM_RE.get_or_init(|| Regex::new(r"(?is)<item\b[^>]*>(.*?)</item>").expect("item regex"));
    let mut lines = Vec::<String>::new();
    let mut links = Vec::<String>::new();
    let mut domains = Vec::<String>::new();
    let mut raw_count = 0usize;
    for captures in item_re.captures_iter(body) {
        raw_count += 1;
        let item = captures.get(1).map(|m| m.as_str()).unwrap_or("");
        let link = normalize_search_result_link(&extract_xml_tag_value(item, "link"));
        if link.is_empty() || !domain_allowed_for_scope(&link, allowed_domains, exclude_subdomains)
        {
            continue;
        }
        let rendered = render_search_row(
            &extract_xml_tag_value(item, "title"),
            &extract_xml_tag_value(item, "description"),
            &link,
        );
        if rendered.is_empty() {
            continue;
        }
        lines.push(rendered);
        links.push(link.clone());
        push_unique_link_domain(&mut domains, &link);
        if lines.len() >= top_k.max(1) {
            break;
        }
    }
    let content = clean_text(&lines.join("\n"), max_response_bytes.min(120_000));
    let ok = !content.is_empty();
    json!({
        "ok": ok,
        "summary": if ok {
            summarize_text(&content, 900)
        } else {
            crate::tool_output_match_filter::no_findings_user_copy().to_string()
        },
        "content": content,
        "links": links,
        "content_domains": domains,
        "provider_raw_count": raw_count,
        "provider_filtered_count": lines.len(),
        "error": if ok {
            Value::Null
        } else {
            Value::String("no_relevant_results".to_string())
        }
    })
}

fn looks_like_search_challenge_payload(summary: &str, content: &str) -> bool {
    let combined = format!("{summary}\n{content}").to_ascii_lowercase();
    if combined.is_empty() {
        return false;
    }
    ["unfortunately, bots use duckduckgo too", "please complete the following challenge", "select all squares containing a duck", "anomaly-modal", "images not loading?", "error-lite@duckduckgo.com"]
        .iter()
        .any(|marker| combined.contains(marker))
}

fn looks_like_empty_duckduckgo_instant_shell_text(text: &str) -> bool {
    let cleaned = clean_text(text, 6_000);
    let start = match cleaned.find('{') {
        Some(idx) => idx,
        None => return looks_like_truncated_duckduckgo_instant_shell(&cleaned),
    };
    let end = match cleaned.rfind('}') {
        Some(idx) if idx > start => idx,
        _ => return looks_like_truncated_duckduckgo_instant_shell(&cleaned[start..]),
    };
    let decoded = serde_json::from_str::<Value>(&cleaned[start..=end]).unwrap_or(Value::Null);
    looks_like_empty_duckduckgo_instant_shell(&decoded)
        || looks_like_truncated_duckduckgo_instant_shell(&cleaned[start..=end])
}

fn looks_like_empty_duckduckgo_instant_shell(decoded: &Value) -> bool {
    let Some(obj) = decoded.as_object() else {
        return false;
    };
    let metadata_keys = ["Abstract", "AbstractSource", "AbstractText", "AbstractURL", "Answer", "AnswerType", "Definition", "DefinitionSource", "DefinitionURL", "Entity", "Heading", "RelatedTopics", "Results", "Type"];
    let metadata_hits = metadata_keys
        .iter()
        .filter(|key| obj.contains_key(**key))
        .count();
    if metadata_hits < 5 {
        return false;
    }
    let has_usable_primary_text = ["AbstractText", "Answer", "Definition", "Heading"]
        .iter()
        .any(|key| {
            clean_text(
                obj.get(*key).and_then(Value::as_str).unwrap_or(""),
                400,
            )
            .len()
                > 1
        });
    if has_usable_primary_text {
        return false;
    }
    let has_related_topics = obj
        .get("RelatedTopics")
        .and_then(Value::as_array)
        .map(|rows| !rows.is_empty())
        .unwrap_or(false);
    if has_related_topics {
        return false;
    }
    let has_results = obj
        .get("Results")
        .and_then(Value::as_array)
        .map(|rows| !rows.is_empty())
        .unwrap_or(false);
    !has_results
}

fn looks_like_truncated_duckduckgo_instant_shell(text: &str) -> bool {
    let lowered = clean_text(text, 6_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let empty_markers = ["\"abstract\":\"\"", "\"abstracttext\":\"\"", "\"answer\":\"\"", "\"definition\":\"\"", "\"heading\":\"\"", "\"entity\":\"\"", "\"relatedtopics\":[]", "\"results\":[]"]
        .iter()
        .filter(|marker| lowered.contains(**marker))
        .count();
    empty_markers >= 4
}

fn payload_looks_like_search_challenge(payload: &Value) -> bool {
    let summary = clean_text(
        payload.get("summary").and_then(Value::as_str).unwrap_or(""),
        2_400,
    );
    let content = clean_text(
        payload.get("content").and_then(Value::as_str).unwrap_or(""),
        4_000,
    );
    looks_like_search_challenge_payload(&summary, &content)
}

fn looks_like_low_signal_search_payload(summary: &str, content: &str) -> bool {
    let lowered = clean_text(&format!("{summary}\n{content}"), 6_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return true;
    }
    if looks_like_search_challenge_payload(summary, content) {
        return true;
    }
    if looks_like_empty_duckduckgo_instant_shell_text(summary)
        || looks_like_empty_duckduckgo_instant_shell_text(content)
    {
        return true;
    }
    if lowered.contains("key findings for") && lowered.contains("potential sources:") {
        return true;
    }
    let marker_hits = ["duckduckgo all regions", "all regions argentina", "all regions australia", "all regions canada", "safe search", "any time", "at duckduckgo"]
        .iter()
        .filter(|marker| lowered.contains(**marker))
        .count();
    marker_hits >= 2
}

fn payload_looks_low_signal_search(payload: &Value) -> bool {
    let summary = clean_text(
        payload.get("summary").and_then(Value::as_str).unwrap_or(""),
        2_400,
    );
    let content = clean_text(
        payload.get("content").and_then(Value::as_str).unwrap_or(""),
        4_000,
    );
    looks_like_low_signal_search_payload(&summary, &content)
}

fn search_lite_fallback_reason(
    used_lite_fallback: bool,
    initial_selected_provider: &str,
    provider_errors: &[Value],
) -> (&'static str, bool, bool, String) {
    if !used_lite_fallback {
        return ("none", false, false, "none".to_string());
    }
    let primary_challenge = provider_errors.iter().any(|row| {
        row.get("provider").and_then(Value::as_str) == Some("duckduckgo")
            && row.get("challenge").and_then(Value::as_bool).unwrap_or(false)
    });
    let primary_low_signal = provider_errors.iter().any(|row| {
        row.get("provider").and_then(Value::as_str) == Some("duckduckgo")
            && row.get("low_signal").and_then(Value::as_bool).unwrap_or(false)
    });
    let reason = if primary_challenge {
        "duckduckgo_challenge"
    } else if primary_low_signal {
        "duckduckgo_low_signal"
    } else if initial_selected_provider != "duckduckgo_lite" {
        "provider_chain_fallback"
    } else {
        "requested_duckduckgo_lite"
    };
    let trigger_provider = if primary_challenge || primary_low_signal {
        "duckduckgo".to_string()
    } else if reason == "provider_chain_fallback" {
        clean_text(
            provider_errors
                .iter()
                .find_map(|row| row.get("provider").and_then(Value::as_str))
                .unwrap_or(initial_selected_provider),
            80,
        )
    } else {
        "duckduckgo_lite".to_string()
    };
    (reason, primary_challenge, primary_low_signal, trigger_provider)
}

fn search_bing_fallback_reason(
    used_bing_fallback: bool,
    initial_selected_provider: &str,
    provider_errors: &[Value],
) -> (&'static str, bool, bool, String) {
    if !used_bing_fallback {
        return ("none", false, false, "none".to_string());
    }
    let duck_challenge_provider = provider_errors.iter().find_map(|row| {
        if matches!(
            row.get("provider").and_then(Value::as_str),
            Some("duckduckgo") | Some("duckduckgo_lite")
        ) && row.get("challenge").and_then(Value::as_bool).unwrap_or(false)
        {
            row.get("provider").and_then(Value::as_str)
        } else {
            None
        }
    });
    let duck_chain_challenge = provider_errors.iter().any(|row| {
        matches!(
            row.get("provider").and_then(Value::as_str),
            Some("duckduckgo") | Some("duckduckgo_lite")
        ) && row.get("challenge").and_then(Value::as_bool).unwrap_or(false)
    });
    let duck_low_signal_provider = provider_errors.iter().find_map(|row| {
        if matches!(
            row.get("provider").and_then(Value::as_str),
            Some("duckduckgo") | Some("duckduckgo_lite")
        ) && row.get("low_signal").and_then(Value::as_bool).unwrap_or(false)
        {
            row.get("provider").and_then(Value::as_str)
        } else {
            None
        }
    });
    let duck_chain_low_signal = provider_errors.iter().any(|row| {
        matches!(
            row.get("provider").and_then(Value::as_str),
            Some("duckduckgo") | Some("duckduckgo_lite")
        ) && row.get("low_signal").and_then(Value::as_bool).unwrap_or(false)
    });
    let reason = if duck_chain_challenge {
        "duckduckgo_chain_challenge"
    } else if duck_chain_low_signal {
        "duckduckgo_chain_low_signal"
    } else if initial_selected_provider != "bing_rss" {
        "provider_chain_fallback"
    } else {
        "requested_bing_rss"
    };
    let trigger_provider = if let Some(provider) = duck_challenge_provider {
        clean_text(provider, 80)
    } else if let Some(provider) = duck_low_signal_provider {
        clean_text(provider, 80)
    } else if reason == "provider_chain_fallback" {
        clean_text(
            provider_errors
                .iter()
                .find_map(|row| row.get("provider").and_then(Value::as_str))
                .unwrap_or(initial_selected_provider),
            80,
        )
    } else {
        "bing_rss".to_string()
    };
    (
        reason,
        duck_chain_challenge,
        duck_chain_low_signal,
        trigger_provider,
    )
}

fn search_provider_failure_mode(
    ok: bool,
    query_mismatch_only_failure: bool,
    challenge_like_failure: bool,
    tool_surface_status: &str,
    tool_surface_ready: bool,
    attempted_provider_count: usize,
) -> &'static str {
    if ok {
        "none"
    } else if query_mismatch_only_failure {
        "query_result_mismatch"
    } else if challenge_like_failure {
        "challenge_or_low_signal_response"
    } else if tool_surface_status == "unavailable" {
        "tool_surface_unavailable"
    } else if tool_surface_status == "degraded"
        && (attempted_provider_count == 0 || !tool_surface_ready)
    {
        "tool_surface_degraded"
    } else if attempted_provider_count == 0 {
        "provider_chain_not_attempted"
    } else {
        "provider_chain_exhausted"
    }
}

fn search_cache_skip_reason_from_failure_mode(search_provider_failure_mode: &str) -> &'static str {
    match search_provider_failure_mode {
        "query_result_mismatch" => "query_result_mismatch",
        "challenge_or_low_signal_response" => "challenge_or_low_signal_response",
        "tool_surface_unavailable" => "tool_surface_unavailable",
        "tool_surface_degraded" => "tool_surface_degraded",
        "provider_chain_not_attempted" => "provider_chain_not_attempted",
        "provider_chain_exhausted" => "provider_chain_exhausted",
        _ => "",
    }
}

fn fetch_with_curl(
    url: &str,
    timeout_ms: u64,
    max_response_bytes: usize,
    user_agent: &str,
    allow_rfc2544_benchmark_range: bool,
) -> Value {
    let mut current_url = clean_text(url, 2200);
    let mut redirect_count = 0usize;
    for _ in 0..=5 {
        let ssrf_guard =
            evaluate_fetch_ssrf_guard(&current_url, allow_rfc2544_benchmark_range, None);
        if !ssrf_guard
            .get("ok")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            return json!({
                "ok": false,
                "status_code": 0,
                "content_type": "",
                "body": "",
                "stderr": clean_text(ssrf_guard.get("error").and_then(Value::as_str).unwrap_or("blocked_private_network_target"), 220),
                "user_agent": clean_text(user_agent, 260),
                "effective_url": current_url,
                "ssrf_guard": ssrf_guard,
                "redirect_count": redirect_count,
                "accept_header": FETCH_MARKDOWN_ACCEPT_HEADER
            });
        }
        let mut current = run_curl_fetch_once(&current_url, timeout_ms, max_response_bytes, user_agent);
        let status_code = current
            .get("status_code")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let location = clean_text(
            current.get("location").and_then(Value::as_str).unwrap_or(""),
            2200,
        );
        if matches!(status_code, 301 | 302 | 303 | 307 | 308) && !location.is_empty() {
            let Some(next_url) = resolve_fetch_redirect_url(&current_url, &location) else {
                if let Some(obj) = current.as_object_mut() {
                    obj.insert(
                        "stderr".to_string(),
                        Value::String("invalid_redirect_target".to_string()),
                    );
                    obj.insert("ok".to_string(), Value::Bool(false));
                    obj.insert("redirect_count".to_string(), json!(redirect_count));
                }
                return current;
            };
            let redirect_guard =
                evaluate_fetch_ssrf_guard(&next_url, allow_rfc2544_benchmark_range, None);
            if !redirect_guard
                .get("ok")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                if let Some(obj) = current.as_object_mut() {
                    obj.insert("ok".to_string(), Value::Bool(false));
                    obj.insert(
                        "stderr".to_string(),
                        Value::String("blocked_private_network_redirect".to_string()),
                    );
                    obj.insert("redirect_target".to_string(), Value::String(next_url));
                    obj.insert("ssrf_guard".to_string(), redirect_guard);
                    obj.insert("redirect_count".to_string(), json!(redirect_count + 1));
                }
                return current;
            }
            current_url = next_url;
            redirect_count += 1;
            continue;
        }
        let effective_url = clean_text(
            current
                .get("effective_url")
                .and_then(Value::as_str)
                .unwrap_or(current_url.as_str()),
            2200,
        );
        if let Some(obj) = current.as_object_mut() {
            obj.insert("redirect_count".to_string(), json!(redirect_count));
            obj.insert("effective_url".to_string(), Value::String(effective_url));
            obj.insert("ssrf_guard".to_string(), ssrf_guard);
        }
        return current;
    }
    json!({
        "ok": false,
        "status_code": 0,
        "content_type": "",
        "body": "",
        "stderr": "too_many_redirects",
        "user_agent": clean_text(user_agent, 260),
        "effective_url": clean_text(url, 2200),
        "redirect_count": redirect_count,
        "accept_header": FETCH_MARKDOWN_ACCEPT_HEADER
    })
}

fn fetch_serper_with_curl(
    api_key: &str,
    query: &str,
    timeout_ms: u64,
    max_response_bytes: usize,
    user_agent: &str,
    top_k: usize,
) -> Value {
    let timeout_sec = ((timeout_ms as f64) / 1000.0).ceil() as u64;
    let payload = json!({
        "q": clean_text(query, 900),
        "num": top_k.clamp(1, 12)
    });
    let payload_raw = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
    let output = Command::new("curl")
        .arg("-sS")
        .arg("-L")
        .arg("--compressed")
        .arg("--proto")
        .arg("=http,https")
        .arg("--connect-timeout")
        .arg(timeout_sec.max(1).to_string())
        .arg("--max-time")
        .arg(timeout_sec.max(1).to_string())
        .arg("-A")
        .arg(clean_text(user_agent, 260))
        .arg("-H")
        .arg("Accept: application/json")
        .arg("-H")
        .arg("Content-Type: application/json")
        .arg("-H")
        .arg(format!("X-API-KEY: {}", clean_text(api_key, 600)))
        .arg("-d")
        .arg(payload_raw)
        .arg("-w")
        .arg("\n__STATUS__:%{http_code}\n__CTYPE__:%{content_type}")
        .arg(SERPER_SEARCH_URL)
        .output();
    match output {
        Ok(run) => {
            let stdout = String::from_utf8_lossy(&run.stdout).to_string();
            let stderr = clean_text(&String::from_utf8_lossy(&run.stderr), 320);
            let status_marker = "\n__STATUS__:";
            let ctype_marker = "\n__CTYPE__:";
            let (body_and_status, content_type) = match stdout.rsplit_once(ctype_marker) {
                Some((left, right)) => (left.to_string(), clean_text(right, 120)),
                None => (stdout, String::new()),
            };
            let (body_raw, status_raw) = match body_and_status.rsplit_once(status_marker) {
                Some((left, right)) => (left.to_string(), clean_text(right, 12)),
                None => (body_and_status, "0".to_string()),
            };
            let status_code = status_raw.parse::<i64>().unwrap_or(0);
            let body = clip_bytes(&body_raw, max_response_bytes.max(256));
            let status_ok = (200..300).contains(&status_code);
            json!({
                "ok": run.status.success() && status_ok,
                "status_code": status_code,
                "content_type": content_type,
                "body": body,
                "stderr": if stderr.is_empty() { Value::Null } else { Value::String(stderr) },
                "user_agent": clean_text(user_agent, 260)
            })
        }
        Err(err) => json!({
            "ok": false,
            "status_code": 0,
            "content_type": "",
            "body": "",
            "stderr": format!("serper_curl_spawn_failed:{err}"),
            "user_agent": clean_text(user_agent, 260)
        }),
    }
}

fn is_retryable_fetch_result(row: &Value) -> bool {
    let status = row.get("status_code").and_then(Value::as_i64).unwrap_or(0);
    if matches!(status, 408 | 425 | 429 | 500 | 502 | 503 | 504) {
        return true;
    }
    let error = clean_text(row.get("stderr").and_then(Value::as_str).unwrap_or(""), 220)
        .to_ascii_lowercase();
    error.contains("timed out")
        || error.contains("timeout")
        || error.contains("econnreset")
        || error.contains("temporarily unavailable")
        || error.contains("could not resolve host")
        || error.contains("empty reply")
}
