fn infer_materialization_quality(
    source_kind: &str,
    permissions: &str,
    snippet: &str,
) -> Option<String> {
    let source_kind = normalize_for_compare(source_kind);
    let permissions = normalize_for_compare(permissions);
    if source_kind.is_empty() && permissions.is_empty() {
        return None;
    }
    let snippet_rich = content_rich_text(snippet);
    let materialized = source_kind.contains("materialized")
        || source_kind.contains("page_enriched")
        || source_kind.contains("document_page_artifact")
        || source_kind.contains("reader_output")
        || source_kind.contains("rendered_page")
        || source_kind.contains("page_artifact")
        || permissions.contains("browser_materialized");
    if materialized {
        return Some(if snippet_rich {
            "full_materialized".to_string()
        } else {
            "partial_materialized".to_string()
        });
    }
    let fetch_like = source_kind.contains("direct_fetch")
        || source_kind.contains("fetch_candidate")
        || permissions.contains("fetch_materialized");
    if fetch_like {
        return Some(if snippet_rich {
            "partial_materialized".to_string()
        } else {
            "failed_materialization".to_string()
        });
    }
    let trusted_structured_feed = source_kind.contains("rss")
        || source_kind.contains("feed")
        || source_kind.contains("api")
        || permissions.contains("structured_feed")
        || permissions.contains("headline_feed");
    if trusted_structured_feed {
        return Some("trusted_structured_feed".to_string());
    }
    Some("candidate_only".to_string())
}

fn content_rich_text(raw: &str) -> bool {
    let cleaned = clean_text(raw, 1_800);
    if cleaned.split_whitespace().count() < 22 {
        return false;
    }
    let lowered = cleaned.to_ascii_lowercase();
    ![
        "no results",
        "no usable result",
        "no usable results",
        "low signal",
        "low-signal",
        "retrieval-quality miss",
        "please narrow",
        "retry with",
        "verify you are human",
        "captcha",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn tool_rows(payload: &Value) -> Vec<&Value> {
    let mut rows = Vec::new();
    if let Some(items) = payload.get("tools").and_then(Value::as_array) {
        rows.extend(items.iter());
    }
    if let Some(items) = payload
        .pointer("/response_finalization/tool_completion/tool_attempts")
        .and_then(Value::as_array)
    {
        rows.extend(items.iter());
    }
    rows
}

fn count_content_items(value: &Value) -> u64 {
    match value {
        Value::Null => 0,
        Value::Bool(raw) => u64::from(*raw),
        Value::Number(_) => 1,
        Value::String(raw) => u64::from(substantive_text(raw)),
        Value::Array(rows) => rows
            .iter()
            .filter(|row| value_has_substantive_content(row))
            .count() as u64,
        Value::Object(map) => u64::from(object_has_substantive_content(map)),
    }
}

fn value_has_substantive_content(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(raw) => *raw,
        Value::Number(_) => true,
        Value::String(raw) => substantive_text(raw),
        Value::Array(rows) => rows.iter().any(value_has_substantive_content),
        Value::Object(map) => object_has_substantive_content(map),
    }
}

fn object_has_substantive_content(map: &serde_json::Map<String, Value>) -> bool {
    if map.is_empty() || object_is_status_or_error_only(map) {
        return false;
    }
    let direct_semantic_keys = [
        "title",
        "url",
        "link",
        "locator",
        "source_url",
        "source_domain",
        "snippet",
        "summary",
        "content",
        "markdown",
        "text",
        "body",
        "description",
        "abstract",
        "claim_hints",
        "claims",
        "extracted_claims",
        "claim_candidates",
        "key_findings",
        "findings",
        "citations",
        "sources",
    ];
    if direct_semantic_keys.iter().any(|key| {
        map.get(*key)
            .map(value_has_substantive_content)
            .unwrap_or(false)
    }) {
        return true;
    }
    semantic_child_values(map).any(value_has_substantive_content)
}

fn object_is_status_or_error_only(map: &serde_json::Map<String, Value>) -> bool {
    let has_error_marker = ["error", "failure", "failure_reason", "status"]
        .iter()
        .any(|key| {
            map.get(*key)
                .map(value_has_substantive_content)
                .unwrap_or(false)
        });
    has_error_marker
        && map.iter().all(|(key, value)| {
            operational_or_error_key(key) || !value_has_substantive_content(value)
        })
}

fn semantic_child_values<'a>(
    map: &'a serde_json::Map<String, Value>,
) -> impl Iterator<Item = &'a Value> {
    map.iter()
        .filter(|(key, _)| !operational_or_error_key(key))
        .map(|(_, value)| value)
}

fn operational_or_error_key(key: &str) -> bool {
    let normalized = normalize_for_compare(&key.replace(['_', '-'], " "));
    [
        "status",
        "state",
        "error",
        "failure",
        "failure reason",
        "failure class",
        "provider",
        "tool",
        "name",
        "query",
        "queries",
        "input",
        "aperture",
        "request",
        "request payload",
        "metadata",
        "query metadata policy",
        "quality flags",
        "quality reasons",
        "blocker taxonomy",
    ]
    .iter()
    .any(|needle| normalized == *needle || normalized.ends_with(&format!(" {needle}")))
}

fn substantive_text(raw: &str) -> bool {
    let cleaned = raw.trim();
    if cleaned.is_empty() {
        return false;
    }
    let normalized = normalize_for_compare(cleaned);
    ![
        "error",
        "failed",
        "tool execution failed",
        "no results",
        "no_results",
        "none",
        "null",
        "unknown",
    ]
    .iter()
    .any(|marker| normalized == *marker)
}

fn tool_status_marker_text(payload: &Value) -> String {
    tool_rows(payload)
        .iter()
        .flat_map(|row| {
            [
                str_at(row, &["name"], ""),
                str_at(row, &["status"], ""),
                str_at(row, &["completion_state"], ""),
                str_at(row, &["state"], ""),
                str_at(row, &["outcome"], ""),
                str_at(row, &["error"], ""),
                str_at(row, &["failure"], ""),
                str_at(row, &["failure_class"], ""),
                str_at(row, &["failure_reason"], ""),
                str_at(row, &["status_code"], ""),
                str_at(row, &["http_status"], ""),
                row.get("quality_lanes")
                    .map(Value::to_string)
                    .unwrap_or_default(),
                row.get("quality_reasons")
                    .map(Value::to_string)
                    .unwrap_or_default(),
                row.get("quality_flags")
                    .map(Value::to_string)
                    .unwrap_or_default(),
            ]
        })
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}
