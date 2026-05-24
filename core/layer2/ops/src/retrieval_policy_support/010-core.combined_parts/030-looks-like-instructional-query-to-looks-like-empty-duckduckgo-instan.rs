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

fn is_instruction_stop_token(token: &str) -> bool {
    matches!(
        token,
        "please"
            | "kindly"
            | "give"
            | "me"
            | "verify"
            | "check"
            | "test"
            | "research"
            | "researching"
            | "find"
            | "found"
            | "report"
            | "return"
            | "provide"
            | "show"
            | "summarize"
            | "answer"
            | "brief"
            | "briefing"
            | "concise"
            | "prioritize"
            | "group"
            | "grouped"
            | "theme"
            | "themes"
            | "cite"
            | "cited"
            | "citation"
            | "citations"
            | "source"
            | "sources"
            | "example"
            | "examples"
            | "across"
            | "question"
            | "questions"
            | "results"
            | "result"
            | "using"
            | "with"
            | "into"
            | "actual"
            | "proper"
            | "web"
            | "search"
            | "fetch"
            | "tool"
            | "tools"
            | "functionality"
            | "capabilities"
    )
}

fn normalize_instructional_query(query: &str) -> Option<String> {
    let base = clean_text(query, 600);
    if base.is_empty() {
        return None;
    }
    let lowered = base.to_ascii_lowercase();
    let focus_seed = instruction_tail_regex()
        .captures(&lowered)
        .and_then(|caps| caps.get(1).map(|row| row.as_str().to_string()))
        .unwrap_or(lowered);
    let tokens = focus_seed
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .filter(|token| token.len() > 2 || token.chars().all(|ch| ch.is_ascii_digit()))
        .map(|token| token.to_ascii_lowercase())
        .filter(|token| !is_instruction_stop_token(token.as_str()))
        .collect::<Vec<_>>();
    if tokens.len() < 3 {
        return None;
    }
    let candidate = clean_text(&tokens.join(" "), 600);
    if candidate.is_empty() {
        None
    } else {
        Some(candidate)
    }
}

fn instruction_focused_search_query(query: &str) -> Option<String> {
    let base = clean_text(query, 600);
    if base.is_empty()
        || base.contains("site:")
        || base.contains("http://")
        || base.contains("https://")
        || base.contains('"')
        || base.contains('`')
        || !looks_like_instructional_query(&base)
    {
        return None;
    }
    let normalized = normalize_instructional_query(&base)?;
    if normalized.eq_ignore_ascii_case(&base) {
        None
    } else {
        Some(normalized)
    }
}

fn deictic_framework_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)\bthis\s+(?:framework|system|platform|stack|agent\s+framework)\b")
            .expect("deictic-framework")
    })
}

fn resolve_deictic_framework_reference(query: &str) -> String {
    let cleaned = clean_text(query, 600);
    if cleaned.is_empty() {
        return cleaned;
    }
    let replaced = deictic_framework_regex().replace_all(&cleaned, "infring");
    clean_text(replaced.as_ref(), 600)
}

#[cfg(test)]
fn build_query_plan(query: &str, budget: ApertureBudget) -> (Vec<String>, Vec<String>, bool) {
    let base = resolve_deictic_framework_reference(query);
    if base.is_empty() {
        return (Vec::new(), Vec::new(), false);
    }
    let exact = is_exact_match_pattern(&base);
    if exact || budget.max_query_rewrites == 0 {
        return (vec![base], Vec::new(), false);
    }
    let rewrite = preferred_query_rewrite(&base);
    if rewrite == base {
        return (vec![base], Vec::new(), false);
    }
    (vec![base.clone(), rewrite.clone()], vec![rewrite], true)
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
