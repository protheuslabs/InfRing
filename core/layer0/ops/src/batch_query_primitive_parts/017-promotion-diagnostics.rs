// Layer ownership: core/layer0/ops::batch-query-promotion-diagnostics (authoritative)

fn retrieval_failure_reason_class(issue: &str) -> &'static str {
    let lowered = clean_text(issue, 320).to_ascii_lowercase();
    if lowered.is_empty() {
        return "unknown";
    }
    if lowered.contains("timeout") || lowered.contains("deadline") {
        return "fetch_timeout";
    }
    if issue_is_access_or_throttle_failure(&lowered)
        || lowered.contains("anti_bot")
        || lowered.contains("captcha")
        || lowered.contains("challenge")
    {
        return "fetch_blocked";
    }
    if lowered.contains("serp_shell_without_organic")
        || lowered.contains("browser_serp_no_results")
        || lowered.contains("no_organic_results")
    {
        return "provider_no_organic_results";
    }
    if lowered.contains("budget_exhausted") {
        return "materialization_budget_exhausted";
    }
    if lowered.contains("browser_materialization_attempted") {
        return "materialization_attempted";
    }
    if lowered.contains("browser_materialization_recovered") {
        return "materialization_recovered";
    }
    if lowered.contains("duplicate") || lowered.contains("low_priority") {
        return "duplicate_or_low_priority";
    }
    if lowered.contains("prefetch_rejected") || lowered.contains("promotion_skipped") {
        return "promotion_skipped";
    }
    if lowered.contains("browser_materialization") {
        return "materialization_failed";
    }
    if lowered.contains("content_too_thin")
        || lowered.contains("no_usable_summary")
        || lowered.contains("low_signal")
        || lowered.contains("source_only")
    {
        return "content_too_thin";
    }
    if lowered.contains("candidate_low_relevance")
        || lowered.contains("query_result_mismatch")
        || lowered.contains("junk_page")
    {
        return "candidate_low_relevance";
    }
    if lowered.contains("fetch_candidate") || lowered.contains("parser") {
        return "parser_rejected";
    }
    if lowered.contains("http")
        || lowered.contains("curl")
        || lowered.contains("dns")
        || lowered.contains("tls")
        || lowered.contains("web_fetch_failed")
        || lowered.contains("fetch:")
    {
        return "http_error";
    }
    if lowered.contains("trusted_feed_only") {
        return "trusted_feed_only";
    }
    if lowered.contains("provider_empty")
        || lowered.contains("provider_error")
        || lowered.contains("search_providers_exhausted")
    {
        return "provider_empty_or_failed";
    }
    "unknown"
}

fn retrieval_failure_reason_class_is_countable(class: &str) -> bool {
    !matches!(
        class,
        "materialization_attempted" | "materialization_recovered"
    )
}

fn retrieval_failure_reason_strings(value: &Value) -> Vec<String> {
    match value {
        Value::Array(rows) => rows
            .iter()
            .filter_map(Value::as_str)
            .map(|row| clean_text(row, 320))
            .filter(|row| !row.is_empty())
            .collect(),
        Value::String(row) => {
            let cleaned = clean_text(row, 320);
            if cleaned.is_empty() {
                Vec::new()
            } else {
                vec![cleaned]
            }
        }
        _ => Vec::new(),
    }
}

fn retrieval_failure_reason_class_counts(issues: &[String]) -> Value {
    let mut counts = Map::<String, Value>::new();
    for issue in issues {
        let class = retrieval_failure_reason_class(issue);
        if !retrieval_failure_reason_class_is_countable(class) {
            continue;
        }
        let next = counts
            .get(class)
            .and_then(Value::as_u64)
            .unwrap_or(0)
            .saturating_add(1);
        counts.insert(class.to_string(), json!(next));
    }
    Value::Object(counts)
}

fn dominant_retrieval_failure_reason_class(issues: &[String]) -> String {
    let mut counts = HashMap::<String, usize>::new();
    for issue in issues {
        let class = retrieval_failure_reason_class(issue).to_string();
        if !retrieval_failure_reason_class_is_countable(&class) {
            continue;
        }
        *counts.entry(class).or_insert(0) += 1;
    }
    counts
        .into_iter()
        .max_by(|left, right| left.1.cmp(&right.1).then_with(|| right.0.cmp(&left.0)))
        .map(|(class, _)| class)
        .unwrap_or_else(|| "none".to_string())
}

#[cfg(test)]
mod batch_query_promotion_diagnostics_tests {
    use super::*;

    #[test]
    fn retrieval_failure_reason_classes_are_normalized() {
        assert_eq!(
            retrieval_failure_reason_class("primary:anti_bot_challenge"),
            "fetch_blocked"
        );
        assert_eq!(
            retrieval_failure_reason_class("bing_rss:fetch_candidate:content_too_thin"),
            "content_too_thin"
        );
        assert_eq!(
            retrieval_failure_reason_class("browser_serp:serp_shell_without_organic_results"),
            "provider_no_organic_results"
        );
        assert_eq!(
            retrieval_failure_reason_class("primary:page_extraction_global_budget_exhausted"),
            "materialization_budget_exhausted"
        );
        assert_eq!(
            retrieval_failure_reason_class("brave:search_providers_exhausted"),
            "provider_empty_or_failed"
        );
    }

    #[test]
    fn retrieval_failure_reason_class_counts_aggregate_classes() {
        let issues = vec![
            "primary:candidate_low_relevance".to_string(),
            "bing_rss:query_result_mismatch".to_string(),
            "browser_serp:browser_serp_no_results".to_string(),
        ];
        let counts = retrieval_failure_reason_class_counts(&issues);
        assert_eq!(
            counts
                .get("candidate_low_relevance")
                .and_then(Value::as_u64),
            Some(2)
        );
        assert_eq!(
            counts
                .get("provider_no_organic_results")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            dominant_retrieval_failure_reason_class(&issues),
            "candidate_low_relevance"
        );
    }

    #[test]
    fn materialization_attempt_events_do_not_count_as_failures() {
        let issues = vec![
            "primary:browser_materialization_attempted".to_string(),
            "primary:browser_materialization:local_browser_empty_dom".to_string(),
        ];
        let counts = retrieval_failure_reason_class_counts(&issues);
        assert_eq!(
            counts
                .get("materialization_attempted")
                .and_then(Value::as_u64),
            None
        );
        assert_eq!(
            counts
                .get("materialization_failed")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            dominant_retrieval_failure_reason_class(&issues),
            "materialization_failed"
        );
    }
}
