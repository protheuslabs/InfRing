fn candidate_from_search_payload(query: &str, payload: &Value) -> Result<Candidate, String> {
    if !payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        return Err(clean_text(
            payload
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("adapter_failed"),
            200,
        ));
    }
    let raw_summary = clean_text(
        payload.get("summary").and_then(Value::as_str).unwrap_or(""),
        1800,
    );
    let content = clean_text(
        payload.get("content").and_then(Value::as_str).unwrap_or(""),
        6_000,
    );
    let mut locator = first_non_search_engine_link(payload);
    if locator.is_empty() {
        locator = canonical_search_result_locator(
            payload
                .get("requested_url")
                .or_else(|| payload.pointer("/receipt/requested_url"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            &[
                payload
                    .get("source_url")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                payload
                    .get("resolved_url")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                payload
                    .get("final_url")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
            ],
        );
    }
    let content_normalized = normalize_snippet_text(
        &normalize_htmlish_content_for_snippet(&content),
        query,
        &locator,
    );
    let summary = normalize_snippet_text(&raw_summary, query, &locator);
    let summary_low_signal = looks_like_low_signal_search_summary(&summary);
    let content_empty_duckduckgo_shell =
        looks_like_empty_duckduckgo_instant_shell_text(&content_normalized);
    let summary_defers_to_content = summary_should_defer_to_content(&raw_summary);
    let domains = extract_domains_from_text(
        if content.is_empty() {
            &raw_summary
        } else {
            &content
        },
        5,
    );
    let mut snippet = if !summary.is_empty()
        && !summary_defers_to_content
        && !looks_like_ack_only(&summary)
        && !summary_low_signal
    {
        summary.clone()
    } else {
        String::new()
    };
    if snippet.is_empty()
        && !content_normalized.is_empty()
        && !looks_like_ack_only(&content_normalized)
        && !content_empty_duckduckgo_shell
        && !looks_like_source_only_snippet(&content_normalized)
    {
        snippet = trim_words(&content_normalized, 56);
    }
    if snippet.is_empty()
        && !summary.is_empty()
        && !summary_defers_to_content
        && !looks_like_ack_only(&summary)
        && !summary_low_signal
    {
        snippet = trim_words(&summary, 56);
    }
    if snippet.is_empty()
        && !domains.is_empty()
        && (summary_low_signal
            || looks_like_domain_list_noise(&content_normalized)
            || looks_like_url_dump_segment(&content_normalized))
    {
        let query_hint = clean_text(query, 140);
        let domain_list = domains
            .iter()
            .take(5)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        let query_suffix = if query_hint.is_empty() {
            String::new()
        } else {
            format!(" for query \"{query_hint}\"")
        };
        snippet = format!(
            "From web retrieval, candidate domains include {domain_list}. The search summary was low-signal chrome{query_suffix}; domains were extracted from full payload content for follow-up synthesis. These domains are candidate leads and require direct page verification before final claims."
        );
    }
    if snippet.is_empty() {
        return Err("no_usable_summary".to_string());
    }
    if looks_like_source_only_snippet(&snippet) {
        return Err("no_usable_summary".to_string());
    }
    let locator_domain = extract_domains_from_text(&locator, 1)
        .into_iter()
        .next()
        .unwrap_or_default();
    let title = if !locator_domain.is_empty() && !is_search_engine_domain(&locator_domain) {
        format!("Web result from {}", clean_text(&locator_domain, 120))
    } else if let Some(first_domain) = domains.first() {
        format!("Web result from {}", clean_text(first_domain, 120))
    } else if locator.is_empty() {
        format!("Web result for {}", clean_text(query, 120))
    } else {
        format!("Web result from {}", clean_text(&locator, 120))
    };
    let explicit_source_kind = clean_text(
        payload
            .get("source_kind")
            .or_else(|| payload.get("sourceKind"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        80,
    );
    let payload_type = clean_text(payload.get("type").and_then(Value::as_str).unwrap_or(""), 80);
    let source_kind = if !explicit_source_kind.is_empty() {
        explicit_source_kind
    } else if !payload_type.is_empty() {
        payload_type
    } else if payload.get("results").is_none()
        && payload.get("links").is_none()
        && (200..400).contains(&payload.get("status_code").and_then(Value::as_i64).unwrap_or(0))
        && !locator_domain.is_empty()
        && !is_search_engine_domain(&locator_domain)
    {
        "web_conduit_fetch".to_string()
    } else {
        "web".to_string()
    };
    Ok(Candidate {
        source_kind: if source_kind.is_empty() {
            "web".to_string()
        } else {
            source_kind
        },
        title,
        locator,
        snippet: snippet.clone(),
        excerpt_hash: sha256_hex(&snippet),
        timestamp: Some(crate::now_iso()),
        permissions: Some("public_web".to_string()),
        status_code: payload
            .get("status_code")
            .and_then(Value::as_i64)
            .unwrap_or(0),
    })
}
