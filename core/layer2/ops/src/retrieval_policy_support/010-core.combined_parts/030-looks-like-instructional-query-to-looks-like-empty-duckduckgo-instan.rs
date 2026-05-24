// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/ops (retrieval policy support).

fn looks_like_instructional_query(query: &str) -> bool {
    let base = clean_text(query, 600);
    if base.is_empty() {
        return false;
    }
    let word_count = base.split_whitespace().count();
    if word_count < 9 {
        return false;
    }
    instruction_frame_regex().is_match(&base)
}

fn sha256_hex(raw: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    let digest = hasher.finalize();
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

fn looks_like_ack_only(text: &str) -> bool {
    let lowered = clean_text(text, 800).to_ascii_lowercase();
    if lowered.is_empty() {
        return true;
    }
    lowered.contains("web search completed")
        || lowered.contains("tool completed")
        || lowered.contains("searched the internet")
        || lowered == "search completed."
}

fn looks_like_low_signal_search_summary(text: &str) -> bool {
    let cleaned = clean_text(text, 3_200);
    if cleaned.is_empty() {
        return true;
    }
    if looks_like_empty_duckduckgo_instant_shell_text(&cleaned) {
        return true;
    }
    let lowered = cleaned.to_ascii_lowercase();
    if lowered.contains("unfortunately, bots use duckduckgo too")
        || lowered.contains("please complete the following challenge")
        || lowered.contains("anomaly-modal")
        || lowered.contains("no usable search results")
        || lowered.contains("no source-backed findings")
        || lowered.contains("no source-backed conclusions")
        || lowered.contains("low-signal results")
    {
        return true;
    }
    let marker_hits = [
        "all regions",
        "safe search",
        "any time",
        "at duckduckgo",
        "viewing ads is privacy protected by duckduckgo",
        "ad clicks are managed by microsoft",
    ]
    .iter()
    .filter(|marker| lowered.contains(**marker))
    .count();
    marker_hits >= 2
}

fn looks_like_empty_duckduckgo_instant_shell_text(text: &str) -> bool {
    let cleaned = clean_text(text, 3_200);
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
