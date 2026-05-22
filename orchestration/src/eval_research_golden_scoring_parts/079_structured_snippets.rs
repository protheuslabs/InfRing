fn provider_structured_snippet_count(payload: &Value) -> u64 {
    selected_tool_contexts(payload)
        .iter()
        .map(|row| count_structured_snippet_items(row, 0))
        .sum()
}
fn count_structured_snippet_items(value: &Value, depth: usize) -> u64 {
    if depth > 7 {
        return 0;
    }
    match value {
        Value::Array(rows) => rows
            .iter()
            .map(|row| count_structured_snippet_items(row, depth + 1))
            .sum(),
        Value::Object(map) => {
            if let Some(false) = value_counts_as_usable_evidence(value) {
                return 0;
            }
            let has_locator = [
                "title",
                "url",
                "link",
                "locator",
                "source_url",
                "source_domain",
            ]
            .iter()
            .any(|key| {
                map.get(*key)
                    .and_then(Value::as_str)
                    .map(|raw| !clean_text(raw, 240).is_empty())
                    .unwrap_or(false)
            });
            let has_substantive_snippet = [
                "snippet",
                "summary",
                "content",
                "text",
                "description",
                "abstract",
            ]
            .iter()
            .any(|key| {
                map.get(*key)
                    .and_then(Value::as_str)
                    .map(structured_snippet_is_evidence_like)
                    .unwrap_or(false)
            });
            let direct = u64::from(has_locator && has_substantive_snippet);
            direct
                + semantic_child_values(map)
                    .map(|row| count_structured_snippet_items(row, depth + 1))
                    .sum::<u64>()
        }
        _ => 0,
    }
}

fn structured_snippet_is_evidence_like(raw: &str) -> bool {
    if !substantive_text(raw) {
        return false;
    }
    let meaningful_word_count = raw
        .split_whitespace()
        .filter(|word| normalize_for_compare(word).len() >= 3)
        .count();
    if meaningful_word_count < 6 {
        return false;
    }
    let normalized = normalize_for_compare(raw);
    ![
        "no results",
        "no usable result",
        "zero evidence",
        "low signal",
        "low-signal",
        "please narrow",
        "retry with",
        "verify you are human",
        "captcha",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
}
