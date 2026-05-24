// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/ops (retrieval policy support).

fn looks_like_empty_duckduckgo_instant_shell(decoded: &Value) -> bool {
    let Some(obj) = decoded.as_object() else {
        return false;
    };
    let metadata_keys = [
        "Abstract",
        "AbstractSource",
        "AbstractText",
        "AbstractURL",
        "Answer",
        "AnswerType",
        "Definition",
        "DefinitionSource",
        "DefinitionURL",
        "Heading",
        "RelatedTopics",
        "Results",
        "Type",
    ];
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
    let lowered = clean_text(text, 3_200).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let empty_markers = [
        "\"abstract\":\"\"",
        "\"abstracttext\":\"\"",
        "\"answer\":\"\"",
        "\"definition\":\"\"",
        "\"heading\":\"\"",
        "\"entity\":\"\"",
        "\"relatedtopics\":[]",
        "\"results\":[]",
    ]
    .iter()
    .filter(|marker| lowered.contains(**marker))
    .count();
    empty_markers >= 4
}

fn looks_like_source_only_snippet(text: &str) -> bool {
    let lowered = clean_text(text, 1_200).to_ascii_lowercase();
    if lowered.is_empty() {
        return true;
    }
    if lowered.starts_with("potential sources:")
        || lowered.starts_with("candidate sources:")
        || lowered.starts_with("found sources:")
    {
        let domain_hits = extract_domains_from_text(&lowered, 8).len();
        let word_count = lowered.split_whitespace().count();
        if domain_hits > 0 && word_count <= 28 {
            return true;
        }
    }
    false
}

fn is_benchmark_or_comparison_intent(query: &str) -> bool {
    let lowered = clean_text(query, 600).to_ascii_lowercase();
    [
        "benchmark",
        "benchmarks",
        "compare",
        "comparison",
        "competitor",
        "competitors",
        "versus",
        " vs ",
        "performance metrics",
        "latency",
        "throughput",
        "success rate",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn metric_number_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"(?i)\b\d+(?:\.\d+)?\s*(?:%|ms|s|sec|seconds|min|mins|minutes|air\s*watts?|watts?|w|x|qps|tps|ops/?sec|tokens/?s)\b",
        )
        .expect("metric-number")
    })
}

fn plain_number_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"\b\d+(?:\.\d+)?\b").expect("plain-number"))
}

fn looks_like_metric_rich_text(text: &str) -> bool {
    let lowered = clean_text(text, 1_200).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    if metric_number_regex().is_match(&lowered) {
        return true;
    }
    let metric_term_hits = [
        "latency",
        "throughput",
        "accuracy",
        "precision",
        "recall",
        "f1",
        "ops/sec",
        "tokens/s",
        "qps",
        "memory",
        "cpu",
        "cost",
        "benchmark",
    ]
    .iter()
    .filter(|marker| lowered.contains(**marker))
    .count();
    plain_number_regex().is_match(&lowered) && metric_term_hits >= 2
}
