// Layer ownership: core/layer0/ops::browser-serp-search-provider-diagnostics (authoritative)

fn browser_serp_record_rejection(
    counts: &mut serde_json::Map<String, Value>,
    samples: &mut Vec<Value>,
    reason: &str,
    raw_href: &str,
    normalized_link: &str,
    text: &str,
) {
    let next_count = counts
        .get(reason)
        .and_then(Value::as_u64)
        .unwrap_or(0)
        .saturating_add(1);
    counts.insert(reason.to_string(), json!(next_count));
    if samples.len() >= 8 {
        return;
    }
    samples.push(json!({
        "reason": clean_text(reason, 80),
        "href": clean_text(raw_href, 320),
        "normalized_url": clean_text(normalized_link, 320),
        "text": clean_text(text, 180),
        "normalized_domain": extract_domain(normalized_link)
    }));
}

fn browser_serp_merge_rejection_counts(
    target: &mut serde_json::Map<String, Value>,
    source: Option<&Value>,
) {
    let Some(source) = source.and_then(Value::as_object) else {
        return;
    };
    for (reason, count) in source {
        let merged = target
            .get(reason)
            .and_then(Value::as_u64)
            .unwrap_or(0)
            .saturating_add(count.as_u64().unwrap_or(0));
        target.insert(reason.clone(), json!(merged));
    }
}

fn browser_serp_merge_rejection_samples(target: &mut Vec<Value>, source: Option<&Value>) {
    let Some(source) = source.and_then(Value::as_array) else {
        return;
    };
    for row in source {
        if target.len() >= 8 {
            break;
        }
        target.push(row.clone());
    }
}

fn browser_serp_outcome_classification(
    ok: bool,
    challenge: bool,
    materialization_error: &str,
    raw_count: usize,
    filtered_count: usize,
    rejection_counts: &serde_json::Map<String, Value>,
) -> Value {
    if ok {
        return json!({
            "version": "browser_serp_outcome_classification_v1",
            "outcome_class": "organic_results_extracted",
            "evidence_impact": "usable",
            "retryable": false,
            "recommended_next_capability": "none"
        });
    }
    if challenge {
        return json!({
            "version": "browser_serp_outcome_classification_v1",
            "outcome_class": "anti_bot_or_access_challenge",
            "evidence_impact": "rejected",
            "retryable": true,
            "recommended_next_capability": "alternate_provider_or_permission_boundary"
        });
    }
    if !materialization_error.is_empty() {
        return json!({
            "version": "browser_serp_outcome_classification_v1",
            "outcome_class": "materialization_error",
            "evidence_impact": "rejected",
            "retryable": true,
            "recommended_next_capability": "browser_materialization_recovery"
        });
    }
    let navigation_rejections = rejection_counts
        .get("search_navigation_or_non_external")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    if raw_count > 0 && filtered_count == 0 && navigation_rejections >= raw_count {
        return json!({
            "version": "browser_serp_outcome_classification_v1",
            "outcome_class": "serp_shell_without_organic_results",
            "evidence_impact": "rejected",
            "retryable": true,
            "recommended_next_capability": "serp_dom_rendering_or_alternate_search_provider"
        });
    }
    json!({
        "version": "browser_serp_outcome_classification_v1",
        "outcome_class": "no_organic_results_extracted",
        "evidence_impact": "rejected",
        "retryable": true,
        "recommended_next_capability": "serp_extraction_or_alternate_search_provider"
    })
}
