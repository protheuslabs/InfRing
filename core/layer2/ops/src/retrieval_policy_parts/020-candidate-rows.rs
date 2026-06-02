// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/ops (retrieval policy authority)

fn search_row_url_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"https?://[^\s)]+").expect("search-row-url"))
}

fn trim_search_row_segment(raw: &str) -> String {
    clean_text(raw, 600)
        .trim()
        .trim_matches(|ch| matches!(ch, '—' | '-' | '|' | ':' | ';'))
        .trim()
        .to_string()
}

fn candidate_from_rendered_search_row(
    _query: &str,
    row: &str,
    status_code: i64,
) -> Option<Candidate> {
    let cleaned = clean_text(row, 1_400);
    if cleaned.is_empty()
        || looks_like_ack_only(&cleaned)
        || looks_like_low_signal_search_summary(&cleaned)
        || looks_like_source_only_snippet(&cleaned)
    {
        return None;
    }
    let url_match = search_row_url_regex().find(&cleaned)?;
    let locator = canonical_search_result_locator(url_match.as_str(), &[]);
    let domain = extract_domains_from_text(&locator, 1)
        .into_iter()
        .next()
        .unwrap_or_default();
    if domain.is_empty() || is_search_engine_domain(&domain) {
        return None;
    }
    let prefix = trim_search_row_segment(&cleaned[..url_match.start()]);
    let suffix = trim_search_row_segment(&cleaned[url_match.end()..]);
    let title = if prefix.is_empty() {
        format!("Web result from {}", clean_text(&domain, 120))
    } else {
        prefix
    };
    let snippet = normalize_htmlish_content_for_snippet(&suffix);
    if snippet.is_empty() {
        return None;
    }
    let excerpt_seed = format!("{} {}", title, snippet);
    Some(Candidate {
        source_kind: "web".to_string(),
        title,
        locator,
        snippet: snippet.clone(),
        excerpt_hash: sha256_hex(&excerpt_seed),
        timestamp: Some(crate::now_iso()),
        permissions: Some("public_web".to_string()),
        status_code,
    })
}

fn structured_result_collection_key(key: &str) -> bool {
    matches!(
        key.to_ascii_lowercase().as_str(),
        "web"
            | "news"
            | "images"
            | "image"
            | "results"
            | "items"
            | "organic"
            | "documents"
            | "data"
            | "links"
    )
}

fn structured_result_source_kind(key: &str, fallback: &str) -> String {
    let key = key.to_ascii_lowercase();
    let fallback_lower = fallback.to_ascii_lowercase();
    if (fallback_lower.contains("api")
        || fallback_lower.contains("rss")
        || fallback_lower.contains("feed"))
        && matches!(
            key.as_str(),
            "web" | "news" | "results" | "items" | "organic" | "documents" | "data"
        )
    {
        return fallback.to_string();
    }
    match key.as_str() {
        "web" | "news" | "images" | "image" | "document" | "documents" => key.to_ascii_lowercase(),
        _ => fallback.to_string(),
    }
}

fn search_payload_result_source_kind(payload: &Value) -> String {
    let explicit = clean_text(
        payload
            .get("source_kind")
            .or_else(|| payload.get("sourceKind"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        80,
    )
    .to_ascii_lowercase();
    if !explicit.is_empty() {
        return explicit;
    }
    let provider = clean_text(
        payload
            .get("provider")
            .or_else(|| payload.get("selected_provider"))
            .or_else(|| payload.get("search_provider"))
            .or_else(|| payload.get("searchProvider"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        80,
    )
    .to_ascii_lowercase();
    match provider.as_str() {
        "tavily" | "exa" | "brave" | "serperdev" => {
            format!("{provider}_api_search_result")
        }
        "google_news_rss" | "bing_rss" => provider,
        _ => "web".to_string(),
    }
}

fn append_candidate_permission_flag(candidate: &mut Candidate, flag: &str) {
    if flag.is_empty() {
        return;
    }
    let existing = candidate.permissions.clone().unwrap_or_default();
    if existing
        .split(';')
        .any(|part| part.trim().eq_ignore_ascii_case(flag))
    {
        return;
    }
    candidate.permissions = Some(if existing.is_empty() {
        flag.to_string()
    } else {
        format!("{existing};{flag}")
    });
}

fn apply_search_payload_source_kind(candidate: &mut Candidate, source_kind: &str) {
    let cleaned = clean_text(source_kind, 80).to_ascii_lowercase();
    if cleaned.is_empty() || cleaned == "web" {
        return;
    }
    candidate.source_kind = cleaned.clone();
    if cleaned.contains("api") {
        append_candidate_permission_flag(candidate, "structured_feed");
    } else if cleaned.contains("rss") || cleaned.contains("feed") {
        append_candidate_permission_flag(candidate, "headline_feed");
    }
}

fn object_string_field(map: &Map<String, Value>, keys: &[&str], max_len: usize) -> String {
    for key in keys {
        if let Some(value) = map.get(*key).and_then(Value::as_str) {
            let cleaned = clean_text(value, max_len);
            if !cleaned.is_empty() {
                return cleaned;
            }
        }
    }
    String::new()
}

fn nested_metadata_string_field(map: &Map<String, Value>, keys: &[&str], max_len: usize) -> String {
    map.get("metadata")
        .and_then(Value::as_object)
        .map(|metadata| object_string_field(metadata, keys, max_len))
        .unwrap_or_default()
}

fn structured_result_locator(map: &Map<String, Value>) -> String {
    let direct = object_string_field(
        map,
        &[
            "url",
            "link",
            "href",
            "sourceURL",
            "source_url",
            "resolved_url",
            "final_url",
            "source",
            "imageUrl",
            "image_url",
            "thumbnail",
            "thumbnailUrl",
            "requested_url",
            "locator",
        ],
        2_200,
    );
    if !direct.is_empty() {
        let direct_fallback = object_string_field(
            map,
            &[
                "sourceURL",
                "source_url",
                "resolved_url",
                "final_url",
                "requested_url",
            ],
            2_200,
        );
        let nested_fallback = nested_metadata_string_field(
            map,
            &[
                "sourceURL",
                "source_url",
                "resolved_url",
                "final_url",
                "url",
                "ogUrl",
                "canonical",
                "requested_url",
            ],
            2_200,
        );
        return canonical_search_result_locator(
            &direct,
            &[direct_fallback.as_str(), nested_fallback.as_str()],
        );
    }
    let nested = nested_metadata_string_field(
        map,
        &[
            "sourceURL",
            "source_url",
            "resolved_url",
            "final_url",
            "url",
            "imageUrl",
            "image_url",
            "ogUrl",
            "canonical",
            "requested_url",
        ],
        2_200,
    );
    canonical_search_result_locator(&nested, &[])
}

fn structured_result_status_code(map: &Map<String, Value>) -> i64 {
    for key in ["status_code", "statusCode", "code"] {
        if let Some(value) = map.get(key).and_then(Value::as_i64) {
            return value;
        }
    }
    map.get("metadata")
        .and_then(Value::as_object)
        .and_then(|metadata| {
            ["status_code", "statusCode", "code"]
                .iter()
                .find_map(|key| metadata.get(*key).and_then(Value::as_i64))
        })
        .unwrap_or(0)
}

fn candidate_from_structured_result_object(
    _query: &str,
    source_kind: &str,
    map: &Map<String, Value>,
) -> Option<Candidate> {
    let locator = structured_result_locator(map);
    if locator.is_empty() {
        return None;
    }
    let domain = extract_domains_from_text(&locator, 1)
        .into_iter()
        .next()
        .unwrap_or_default();
    if domain.is_empty() || is_search_engine_domain(&domain) {
        return None;
    }
    let title = {
        let direct = object_string_field(map, &["title", "name", "headline"], 240);
        if !direct.is_empty() {
            direct
        } else {
            let metadata_title = nested_metadata_string_field(map, &["title", "ogTitle"], 240);
            if metadata_title.is_empty() {
                format!("Web result from {}", clean_text(&domain, 120))
            } else {
                metadata_title
            }
        }
    };
    let raw_snippet = object_string_field(
        map,
        &[
            "description",
            "snippet",
            "summary",
            "markdown",
            "content",
            "text",
            "answer",
            "alt",
        ],
        6_000,
    );
    let metadata_description =
        nested_metadata_string_field(map, &["description", "ogDescription"], 1_200);
    let snippet_seed = if raw_snippet.is_empty() {
        metadata_description
    } else {
        raw_snippet
    };
    let snippet = trim_words(
        &normalize_htmlish_content_for_snippet(&clean_text(&snippet_seed, 6_000)),
        72,
    );
    if snippet.is_empty()
        || looks_like_ack_only(&snippet)
        || looks_like_low_signal_search_summary(&snippet)
        || looks_like_source_only_snippet(&snippet)
    {
        return None;
    }
    let excerpt_seed = format!("{title} {snippet}");
    Some(Candidate {
        source_kind: clean_text(source_kind, 80),
        title,
        locator,
        snippet,
        excerpt_hash: sha256_hex(&excerpt_seed),
        timestamp: Some(crate::now_iso()),
        permissions: Some("public_web".to_string()),
        status_code: structured_result_status_code(map),
    })
}

fn collect_structured_search_candidates_from_value(
    query: &str,
    value: &Value,
    source_kind: &str,
    in_collection: bool,
    depth: usize,
    max_rows: usize,
    out: &mut Vec<Candidate>,
) {
    if out.len() >= max_rows || depth > 6 {
        return;
    }
    match value {
        Value::Array(rows) => {
            for row in rows {
                collect_structured_search_candidates_from_value(
                    query,
                    row,
                    source_kind,
                    in_collection,
                    depth + 1,
                    max_rows,
                    out,
                );
                if out.len() >= max_rows {
                    break;
                }
            }
        }
        Value::Object(map) => {
            if in_collection {
                if let Some(candidate) =
                    candidate_from_structured_result_object(query, source_kind, map)
                {
                    out.push(candidate);
                    if out.len() >= max_rows {
                        return;
                    }
                }
            }
            for (key, child) in map {
                if !child.is_array() && !child.is_object() {
                    continue;
                }
                let child_source_kind = structured_result_source_kind(key, source_kind);
                let child_in_collection = in_collection || structured_result_collection_key(key);
                collect_structured_search_candidates_from_value(
                    query,
                    child,
                    &child_source_kind,
                    child_in_collection,
                    depth + 1,
                    max_rows,
                    out,
                );
                if out.len() >= max_rows {
                    break;
                }
            }
        }
        _ => {}
    }
}

fn candidates_from_structured_search_payload(
    query: &str,
    payload: &Value,
    max_rows: usize,
) -> Vec<Candidate> {
    if max_rows == 0 {
        return Vec::new();
    }
    let mut out = Vec::<Candidate>::new();
    let source_kind = search_payload_result_source_kind(payload);
    collect_structured_search_candidates_from_value(
        query,
        payload,
        &source_kind,
        false,
        0,
        max_rows,
        &mut out,
    );
    let mut seen = HashSet::<String>::new();
    out.retain(|candidate| {
        let key = format!(
            "{}|{}|{}",
            candidate.locator.to_ascii_lowercase(),
            candidate.title.to_ascii_lowercase(),
            candidate.excerpt_hash
        );
        seen.insert(key)
    });
    out
}

fn candidates_from_rendered_search_payload(
    query: &str,
    payload: &Value,
    max_rows: usize,
) -> Vec<Candidate> {
    if max_rows == 0 {
        return Vec::new();
    }
    let raw_content = rendered_search_payload_text(payload);
    if raw_content.trim().is_empty() {
        return Vec::new();
    }
    let status_code = payload
        .get("status_code")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let source_kind = search_payload_result_source_kind(payload);
    let mut out = Vec::<Candidate>::new();
    let mut seen = HashSet::<String>::new();
    for row in rendered_search_payload_rows(&raw_content, &source_kind) {
        let Some(mut candidate) = candidate_from_rendered_search_row(query, &row, status_code)
        else {
            continue;
        };
        apply_search_payload_source_kind(&mut candidate, &source_kind);
        let key = format!(
            "{}|{}|{}",
            candidate.locator.to_ascii_lowercase(),
            candidate.title.to_ascii_lowercase(),
            candidate.excerpt_hash
        );
        if !seen.insert(key) {
            continue;
        }
        out.push(candidate);
        if out.len() >= max_rows {
            break;
        }
    }
    out
}

fn rendered_search_payload_rows(raw_content: &str, source_kind: &str) -> Vec<String> {
    let provider_preview = rendered_payload_source_kind_can_embed_result_records(source_kind);
    let mut rows = Vec::<String>::new();
    for line in raw_content.lines() {
        let cleaned = clean_text(line, 8_000);
        if cleaned.is_empty() {
            continue;
        }
        if provider_preview {
            let embedded = embedded_search_result_rows_from_preview(&cleaned);
            if embedded.len() > 1 {
                rows.extend(embedded);
                continue;
            }
        }
        rows.push(cleaned);
    }
    rows
}

fn rendered_payload_source_kind_can_embed_result_records(source_kind: &str) -> bool {
    let lowered = clean_text(source_kind, 120).to_ascii_lowercase();
    lowered.contains("api") || lowered.contains("rss") || lowered.contains("feed")
}

fn embedded_search_result_rows_from_preview(raw: &str) -> Vec<String> {
    let cleaned = clean_text(raw, 8_000);
    let urls = search_row_url_regex()
        .find_iter(&cleaned)
        .filter(|url_match| !search_row_url_is_markdown_inline_link(&cleaned, *url_match))
        .collect::<Vec<_>>();
    if urls.len() < 2 {
        return vec![cleaned];
    }

    let mut rows = Vec::<String>::new();
    for (index, url_match) in urls.iter().enumerate() {
        let title = if index == 0 {
            trim_search_row_segment(&cleaned[..url_match.start()])
        } else {
            embedded_search_result_title_from_between(
                &cleaned[urls[index - 1].end()..url_match.start()],
            )
        };
        if title.is_empty() {
            continue;
        }

        let raw_suffix = if let Some(next_url) = urls.get(index + 1) {
            &cleaned[url_match.end()..next_url.start()]
        } else {
            &cleaned[url_match.end()..]
        };
        let snippet = embedded_search_result_snippet_before_next_title(raw_suffix);
        let row = trim_search_row_segment(&format!(
            "{} {} {}",
            title,
            url_match.as_str(),
            snippet
        ));
        if !row.is_empty() {
            rows.push(row);
        }
    }

    if rows.len() > 1 {
        rows
    } else {
        vec![cleaned]
    }
}

fn search_row_url_is_markdown_inline_link(raw: &str, url_match: regex::Match<'_>) -> bool {
    if url_match.start() == 0 {
        return false;
    }
    raw[..url_match.start()]
        .chars()
        .next_back()
        .is_some_and(|ch| ch == '(' || ch == '[')
}

fn embedded_search_result_snippet_before_next_title(raw: &str) -> String {
    let cleaned = trim_search_row_segment(raw);
    if cleaned.is_empty() {
        return cleaned;
    }
    let next_title = embedded_search_result_title_from_between(&cleaned);
    if next_title.is_empty() {
        return cleaned;
    }
    if let Some(index) = cleaned.rfind(&next_title) {
        return trim_search_row_segment(&cleaned[..index]);
    }
    cleaned
}

fn embedded_search_result_title_from_between(raw: &str) -> String {
    let cleaned = trim_search_row_segment(raw);
    if cleaned.is_empty() {
        return String::new();
    }
    let mut words = cleaned
        .split_whitespace()
        .map(|word| {
            word.trim_matches(|ch: char| {
                matches!(
                    ch,
                    ',' | '.' | ';' | ':' | '"' | '\'' | '(' | ')' | '[' | ']'
                )
            })
        })
        .filter(|word| !word.is_empty())
        .filter(|word| {
            let lowered = word.to_ascii_lowercase();
            !lowered.contains("http://")
                && !lowered.contains("https://")
                && !lowered.contains("www.")
                && !word.contains('[')
                && !word.contains(']')
        })
        .collect::<Vec<_>>();
    while words
        .first()
        .is_some_and(|word| embedded_search_result_navigation_word(word))
    {
        words.remove(0);
    }
    if words.is_empty() {
        return String::new();
    }
    let start = words.len().saturating_sub(12);
    let mut title = clean_text(&words[start..].join(" "), 320);
    while title
        .split_whitespace()
        .next()
        .is_some_and(embedded_search_result_navigation_word)
    {
        title = clean_text(
            &title
                .split_whitespace()
                .skip(1)
                .collect::<Vec<_>>()
                .join(" "),
            320,
        );
    }
    trim_search_row_segment(&title)
}

fn embedded_search_result_navigation_word(word: &str) -> bool {
    matches!(
        clean_text(word, 40).to_ascii_lowercase().as_str(),
        "skip"
            | "to"
            | "main"
            | "content"
            | "english"
            | "korean"
            | "chinese"
            | "read"
            | "more"
            | "source"
    )
}

fn rendered_search_payload_text(payload: &Value) -> String {
    let mut rows = Vec::<String>::new();
    for key in ["content", "summary", "content_preview", "markdown", "text"] {
        if let Some(value) = payload
            .get(key)
            .and_then(Value::as_str)
            .map(|raw| clean_text(raw, 8_000))
            .filter(|raw| !raw.is_empty())
        {
            if !rows.iter().any(|row| row.eq_ignore_ascii_case(&value)) {
                rows.push(value);
            }
        }
    }
    rows.join("\n")
}

fn retained_search_results_preview(rows: &[(Candidate, f64)], limit: usize) -> Value {
    Value::Array(
        rows.iter()
            .take(limit.max(1))
            .map(|(row, score)| {
                json!({
                    "source_kind": row.source_kind,
                    "title": clean_text(&row.title, 240),
                    "locator": clean_text(&row.locator, 2_200),
                    "snippet": trim_words(&clean_text(&row.snippet, 1_200), 48),
                    "score": (*score * 100.0).round() / 100.0,
                    "timestamp": row.timestamp,
                    "permissions": row.permissions,
                    "status_code": row.status_code
                })
            })
            .collect::<Vec<_>>(),
    )
}

fn retained_provider_results_preview(query: &str, rows: &[Value], limit: usize) -> Value {
    let mut out = Vec::<Value>::new();
    for row in rows {
        if out.len() >= limit.max(1) {
            break;
        }
        let locator = clean_text(
            row.get("locator").and_then(Value::as_str).unwrap_or(""),
            2_200,
        );
        let summary = clean_text(
            row.get("summary").and_then(Value::as_str).unwrap_or(""),
            1_200,
        );
        if locator.is_empty() || summary.is_empty() {
            continue;
        }
        let domain = extract_domains_from_text(&locator, 1)
            .into_iter()
            .next()
            .unwrap_or_default();
        if domain.is_empty() || is_search_engine_domain(&domain) {
            continue;
        }
        if looks_like_ack_only(&summary)
            || looks_like_low_signal_search_summary(&summary)
            || contains_antibot_marker(&summary)
            || contains_web_junk_marker(&summary)
        {
            continue;
        }
        let candidate = Candidate {
            source_kind: "web".to_string(),
            title: format!("Web result from {domain}"),
            locator: locator.clone(),
            snippet: summary.clone(),
            excerpt_hash: sha256_hex(&summary),
            timestamp: None,
            permissions: Some("public_web".to_string()),
            status_code: row.get("status_code").and_then(Value::as_i64).unwrap_or(0),
        };
        if query_overlap_terms(query, &candidate) == 0 && source_trust_adjustment(&candidate) < 0.15
        {
            continue;
        }
        out.push(json!({
            "source_kind": candidate.source_kind,
            "title": candidate.title,
            "locator": candidate.locator,
            "snippet": trim_words(&candidate.snippet, 48),
            "score": 0.0,
            "timestamp": candidate.timestamp,
            "permissions": candidate.permissions,
            "status_code": candidate.status_code
        }));
    }
    Value::Array(out)
}

fn candidate_retention_preview_eligible(query: &str, candidate: &Candidate, score: f64) -> bool {
    let snippet = clean_text(&candidate.snippet, 1_200);
    let domain = candidate_domain_hint(candidate);
    let trusted_source = source_trust_adjustment(candidate) >= 0.15;
    let overlap = query_overlap_terms(query, candidate);
    let trusted_overlap_preview = trusted_source && overlap >= 1;
    let substantive_preview_text =
        !looks_like_source_only_snippet(&snippet) || trusted_overlap_preview;
    (score > 0.0 || trusted_overlap_preview)
        && !snippet.is_empty()
        && !looks_like_ack_only(&snippet)
        && !looks_like_low_signal_search_summary(&snippet)
        && substantive_preview_text
        && !candidate_has_non_evidence_payload(candidate)
        && !citation_wrapper_link(&candidate.locator)
        && !is_search_engine_domain(&domain)
        && !looks_like_portal_noise_candidate(candidate)
}

fn comparison_guard_failure_artifacts(
    query: &str,
    comparison_entities: &[String],
    actionable_ranked: &[(Candidate, f64)],
    retained_ranked: &[(Candidate, f64)],
    provider_results: &[Value],
    max_results: usize,
) -> (Value, Option<String>) {
    if comparison_entities.len() < 2 {
        return (json!([]), None);
    }
    let coverage_ok = comparison_entities.iter().all(|entity| {
        actionable_ranked.iter().any(|(row, score)| {
            candidate_counts_as_query_usable_evidence(query, row, *score)
                && candidate_mentions_entity(row, entity)
        })
    });
    if coverage_ok {
        return (json!([]), None);
    }
    let preview_rows = if actionable_ranked.is_empty() {
        retained_ranked
    } else {
        actionable_ranked
    };
    let search_results = if preview_rows.is_empty() {
        retained_provider_results_preview(query, provider_results, max_results)
    } else {
        retained_search_results_preview(preview_rows, max_results)
    };
    (
        search_results,
        Some(format!(
            "Search did not produce enough source coverage to compare {} in this turn. This is a retrieval-quality miss, not proof the systems are equivalent. Retry with named competitors or one specific source URL per side.",
            comparison_entities.join(" vs ")
        )),
    )
}

fn comparison_entity_coverage_count(
    query: &str,
    comparison_entities: &[String],
    actionable_ranked: &[(Candidate, f64)],
    _retained_ranked: &[(Candidate, f64)],
) -> usize {
    comparison_entities
        .iter()
        .filter(|entity| {
            actionable_ranked.iter().any(|(row, score)| {
                candidate_counts_as_query_usable_evidence(query, row, *score)
                    && candidate_mentions_entity(row, entity)
            })
        })
        .count()
}

fn comparison_retained_entity_coverage_count(
    query: &str,
    comparison_entities: &[String],
    retained_ranked: &[(Candidate, f64)],
) -> usize {
    comparison_entities
        .iter()
        .filter(|entity| {
            retained_ranked.iter().any(|(row, score)| {
                candidate_mentions_entity(row, entity)
                    && candidate_retention_preview_eligible(query, row, *score)
            })
        })
        .count()
}

fn comparison_partial_preserves_actionable_evidence(
    query: &str,
    comparison_entities: &[String],
    actionable_ranked: &[(Candidate, f64)],
    retained_ranked: &[(Candidate, f64)],
) -> bool {
    let min_covered_entities = comparison_entities.len().min(2);
    if min_covered_entities == 0 {
        return false;
    }
    let actionable_covered = comparison_entity_coverage_count(
        query,
        comparison_entities,
        actionable_ranked,
        retained_ranked,
    );
    if actionable_covered >= min_covered_entities {
        return true;
    }
    actionable_covered > 0
        && comparison_retained_entity_coverage_count(query, comparison_entities, retained_ranked)
            >= min_covered_entities
}

#[cfg(test)]
mod rendered_provider_preview_fixture_tests {
    use super::*;

    fn provider_preview_fixture_query() -> &'static str {
        "What are some source-backed scientific breakthroughs or notable research milestones from April 2026?"
    }

    fn provider_preview_fixture_payload() -> Value {
        json!({
            "ok": true,
            "provider": "tavily",
            "content_preview": "Scientific breakthroughs: 2026 emerging trends to watch — https://www.cas.org/resources/cas-insights/scientific-breakthroughs-2026-emerging-trends-watch — [Skip to main content](https://www.cas.org/resources/cas-insights/scientific-breakthroughs-2026-emerging-trends-watch#main-content). [English](https://www.cas.org/resources/cas-insights/scientific-breakthroughs-2026-emerging-trends-watch). [Korean](https://www.cas.org/ko/resources/cas-insights/scientific-breakthroughs-2026-emerging-trends-watch). [Chinese](https://www.cas.org/zh-hans/resources/cas-insights/scientific April 2026 Brief - Immunotherapy, Biomarkers, and Cancer Pathways — https://www.targetedonc.com/fda-briefs/april-2026-brief — The FDA granted breakthrough therapy designation to plixorafenib for adult patients with BRAF V600E-mutated high-grade glioma, based on data.",
            "status_code": 200
        })
    }

    #[test]
    fn rendered_provider_preview_records_become_independent_candidate_rows() {
        let query = provider_preview_fixture_query();
        let payload = provider_preview_fixture_payload();
        let candidates = candidates_from_rendered_search_payload(query, &payload, 8);

        assert!(
            candidates.iter().any(|candidate| {
                candidate
                    .locator
                    .contains("cas.org/resources/cas-insights/scientific-breakthroughs")
            }),
            "{candidates:#?}"
        );
        assert!(
            candidates.iter().any(|candidate| {
                candidate
                    .locator
                    .contains("targetedonc.com/fda-briefs/april-2026-brief")
            }),
            "{candidates:#?}"
        );
    }

    #[test]
    fn exposed_provider_preview_candidate_becomes_claim_backed_evidence() {
        let query = provider_preview_fixture_query();
        let payload = provider_preview_fixture_payload();
        let candidates = candidates_from_rendered_search_payload(query, &payload, 8);
        let candidate = candidates
            .iter()
            .find(|candidate| {
                candidate
                    .locator
                    .contains("targetedonc.com/fda-briefs/april-2026-brief")
            })
            .cloned()
            .expect("targetedonc candidate");
        let pack = evidence_pack_from_ranked_candidates(
            &default_policy(),
            query,
            &[],
            1,
            &[(candidate, 0.96)],
            1,
        );
        let first = pack
            .as_array()
            .and_then(|rows| rows.first())
            .expect("evidence row");
        let claims = first
            .get("claim_hints")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let claim_text = claims
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(" ");

        assert!(
            claim_text.contains("breakthrough therapy designation"),
            "{first:#?}"
        );
        assert_eq!(
            first
                .get("counts_as_usable_evidence")
                .and_then(Value::as_bool),
            Some(true),
            "{first:#?}"
        );
    }
}
