fn structured_search_freshness_filter(filters: &Value) -> String {
    clean_text(
        filters
            .get("freshness")
            .and_then(Value::as_str)
            .unwrap_or(""),
        40,
    )
    .to_ascii_lowercase()
}

fn normalized_search_freshness_window(raw: &str) -> Option<&'static str> {
    match clean_text(raw, 40).to_ascii_lowercase().as_str() {
        "day" | "daily" | "today" | "yesterday" | "24h" | "past_day" | "past-day" => Some("day"),
        "week" | "weekly" | "this_week" | "this-week" | "past_week" | "past-week" => {
            Some("week")
        }
        "month" | "monthly" | "this_month" | "this-month" | "past_month" | "past-month" => {
            Some("month")
        }
        "year" | "yearly" | "past_year" | "past-year" => Some("year"),
        _ => None,
    }
}

fn search_query_requests_news_source_lane(query: &str) -> bool {
    let lowered = clean_text(query, 900).to_ascii_lowercase();
    [
        "news",
        "headline",
        "headlines",
        "breaking",
        "incident",
        "incidents",
        "event",
        "events",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn exa_published_date_bounds_for_freshness(window: &str) -> Option<(String, String)> {
    let days = match window {
        "day" => 1,
        "week" => 7,
        "month" => 31,
        "year" => 366,
        _ => return None,
    };
    let end = chrono::Local::now().date_naive();
    let start = end - chrono::Duration::days(days);
    Some((
        start.format("%Y-%m-%d").to_string(),
        end.format("%Y-%m-%d").to_string(),
    ))
}

fn brave_freshness_param_for_window(window: &str) -> Option<&'static str> {
    match window {
        "day" => Some("pd"),
        "week" => Some("pw"),
        _ => None,
    }
}

fn brave_search_url(query: &str, top_k: usize, filters: &Value) -> String {
    let freshness = structured_search_freshness_filter(filters);
    let window = normalized_search_freshness_window(&freshness);
    let mut url = format!(
        "{}?q={}&count={}&extra_snippets=true",
        BRAVE_SEARCH_URL,
        encode_query_component(&clean_text(query, 600)),
        top_k.clamp(1, 20)
    );
    if let Some(token) = window.and_then(brave_freshness_param_for_window) {
        url.push_str("&freshness=");
        url.push_str(token);
    }
    url
}

fn structured_search_native_filter_controls(provider: &str, query: &str, filters: &Value) -> Value {
    let freshness = structured_search_freshness_filter(filters);
    let window = normalized_search_freshness_window(&freshness);
    let mut controls = json!({
        "version": "structured_search_native_filter_controls_v1",
        "freshness": if freshness.is_empty() { Value::Null } else { Value::String(freshness.clone()) },
        "normalized_window": window.map(|row| Value::String(row.to_string())).unwrap_or(Value::Null),
        "provider": clean_text(provider, 80),
        "applied_fields": []
    });
    let Some(window_value) = window else {
        return controls;
    };
    let applied = controls
        .get_mut("applied_fields")
        .and_then(Value::as_array_mut)
        .expect("applied_fields array");
    match provider {
        "tavily" => {
            applied.push(json!("time_range"));
            if search_query_requests_news_source_lane(query) {
                applied.push(json!("topic"));
            }
        }
        "exa" => {
            if exa_published_date_bounds_for_freshness(window_value).is_some() {
                applied.push(json!("startPublishedDate"));
                applied.push(json!("endPublishedDate"));
            }
        }
        "brave" => {
            if brave_freshness_param_for_window(window_value).is_some() {
                applied.push(json!("freshness"));
            }
        }
        _ => {}
    }
    controls
}

fn api_search_structured_provider(
    root: &Path,
    provider: &str,
    query: &str,
    summary_only: bool,
    human_approved: bool,
    allowed_domains: &[String],
    exclude_subdomains: bool,
    top_k: usize,
    requested_timeout_ms: u64,
    filters: &Value,
) -> Value {
    let provider = clean_text(provider, 80);
    let requested_url = match provider.as_str() {
        "tavily" => TAVILY_SEARCH_URL.to_string(),
        "exa" => EXA_SEARCH_URL.to_string(),
        "brave" => brave_search_url(query, top_k, filters),
        _ => String::new(),
    };
    let (policy, _policy_path_value) = load_policy(root);
    let credential_source = resolve_provider_credential_source_with_env(
        &policy,
        &provider,
        WebProviderFamily::Search,
        |key| std::env::var(key).ok(),
    );
    let Some(api_key) = resolve_search_provider_credential(root, &policy, &provider) else {
        return json!({
            "ok": false,
            "error": format!("{provider}_api_key_missing"),
            "requested_url": requested_url,
            "provider": provider,
            "credential_source": credential_source,
            "docs": "https://docs.openclaw.ai/tools/web"
        });
    };
    let policy_eval = crate::infring_layer1_security_bridge::evaluate_web_conduit_policy(
        root,
        &json!({
            "requested_url": requested_url,
            "domain": extract_domain(&requested_url),
            "human_approved": human_approved,
            "requests_last_minute": requests_last_minute_for_lane(root, "search")
        }),
        &policy,
    );
    let allow = policy_eval
        .get("allow")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let reason = clean_text(
        policy_eval
            .get("reason")
            .and_then(Value::as_str)
            .unwrap_or("policy_denied"),
        180,
    );
    if !allow {
        let receipt = build_receipt(
            &requested_url,
            "deny",
            None,
            0,
            &reason,
            Some("policy_denied"),
        );
        let _ = append_jsonl(&receipts_path(root), &receipt);
        return json!({
            "ok": false,
            "error": "web_conduit_policy_denied",
            "requested_url": requested_url,
            "policy_decision": policy_eval,
            "provider": provider,
            "receipt": receipt
        });
    }
    let timeout_ms = requested_timeout_ms.clamp(
        1_000,
        policy_eval
            .pointer("/policy/timeout_ms")
            .and_then(Value::as_u64)
            .unwrap_or(search_default_timeout_ms(&policy)),
    );
    let max_response_bytes = policy_eval
        .pointer("/policy/max_response_bytes")
        .and_then(Value::as_u64)
        .unwrap_or(350_000) as usize;
    let retry_attempts = policy_eval
        .pointer("/policy/retry_attempts")
        .and_then(Value::as_u64)
        .unwrap_or(2)
        .clamp(1, 4) as usize;
    let domain_filter = json!(allowed_domains);
    let freshness = structured_search_freshness_filter(filters);
    let normalized_freshness = normalized_search_freshness_window(&freshness);
    let native_filter_controls =
        structured_search_native_filter_controls(&provider, query, filters);
    let fetched = match provider.as_str() {
        "tavily" => {
            let mut payload = json!({
                "query": clean_text(query, 900),
                "search_depth": "basic",
                "max_results": top_k.clamp(1, 20),
                "include_answer": false,
                "include_raw_content": false,
                "include_favicon": false
            });
            if let Some(window) = normalized_freshness {
                payload["time_range"] = json!(window);
                if search_query_requests_news_source_lane(query) {
                    payload["topic"] = json!("news");
                }
            }
            if !allowed_domains.is_empty() {
                payload["include_domains"] = domain_filter;
            }
            fetch_json_post_with_retry(
                TAVILY_SEARCH_URL,
                &payload,
                &[("Authorization", format!("Bearer {api_key}"))],
                timeout_ms,
                max_response_bytes,
                retry_attempts,
                "tavily_curl_spawn_failed",
            )
        }
        "exa" => {
            let mut payload = json!({
                "query": clean_text(query, 900),
                "type": "auto",
                "numResults": top_k.clamp(1, 40),
                "contents": {
                    "highlights": true
                }
            });
            if let Some(window) = normalized_freshness {
                if let Some((start, end)) = exa_published_date_bounds_for_freshness(window) {
                    payload["startPublishedDate"] = json!(start);
                    payload["endPublishedDate"] = json!(end);
                }
            }
            if !allowed_domains.is_empty() {
                payload["includeDomains"] = domain_filter;
            }
            fetch_json_post_with_retry(
                EXA_SEARCH_URL,
                &payload,
                &[("x-api-key", api_key.clone())],
                timeout_ms,
                max_response_bytes,
                retry_attempts,
                "exa_curl_spawn_failed",
            )
        }
        "brave" => fetch_brave_with_retry(
            &api_key,
            query,
            timeout_ms,
            max_response_bytes,
            retry_attempts,
            top_k,
            filters,
        ),
        _ => json!({
            "ok": false,
            "status_code": 0,
            "content_type": "",
            "body": "",
            "stderr": "unsupported_structured_search_provider"
        }),
    };
    let status_code = fetched
        .get("status_code")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let content_type = clean_text(
        fetched
            .get("content_type")
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );
    let body = fetched.get("body").and_then(Value::as_str).unwrap_or("");
    let parsed = match provider.as_str() {
        "tavily" => render_tavily_payload(
            body,
            allowed_domains,
            exclude_subdomains,
            top_k,
            max_response_bytes,
        ),
        "exa" => render_exa_payload(
            body,
            allowed_domains,
            exclude_subdomains,
            top_k,
            max_response_bytes,
        ),
        "brave" => render_brave_payload(
            body,
            allowed_domains,
            exclude_subdomains,
            top_k,
            max_response_bytes,
        ),
        _ => json!({
            "ok": false,
            "error": "unsupported_structured_search_provider",
            "summary": "",
            "content": "",
            "links": [],
            "content_domains": [],
            "provider_raw_count": 0,
            "provider_filtered_count": 0
        }),
    };
    let content = clean_text(
        parsed.get("content").and_then(Value::as_str).unwrap_or(""),
        max_response_bytes,
    );
    let summary = clean_text(
        parsed.get("summary").and_then(Value::as_str).unwrap_or(""),
        900,
    );
    let response_hash = if content.is_empty() {
        String::new()
    } else {
        sha256_hex(&content)
    };
    let artifact = if response_hash.is_empty() {
        None
    } else {
        persist_artifact(root, &requested_url, &response_hash, &content)
    };
    let fetch_ok = fetched.get("ok").and_then(Value::as_bool).unwrap_or(false)
        && parsed.get("ok").and_then(Value::as_bool).unwrap_or(false)
        && !summary.is_empty();
    let mut error_value = clean_text(
        fetched.get("stderr").and_then(Value::as_str).unwrap_or(""),
        320,
    );
    if error_value.is_empty() {
        error_value = structured_search_http_error(&provider, status_code).unwrap_or_else(|| {
            clean_text(
                parsed.get("error").and_then(Value::as_str).unwrap_or(""),
                220,
            )
        });
    }
    let receipt = build_receipt(
        &requested_url,
        "allow",
        if response_hash.is_empty() {
            None
        } else {
            Some(response_hash.as_str())
        },
        status_code,
        &reason,
        if error_value.is_empty() {
            None
        } else {
            Some(error_value.as_str())
        },
    );
    let _ = append_jsonl(&receipts_path(root), &receipt);
    json!({
        "ok": fetch_ok,
        "requested_url": requested_url,
        "status_code": status_code,
        "content_type": if content_type.is_empty() { Value::String("application/json".to_string()) } else { Value::String(content_type) },
        "summary": summary,
        "content": if summary_only { Value::String(String::new()) } else { Value::String(content) },
        "links": parsed.get("links").cloned().unwrap_or_else(|| json!([])),
        "results": parsed.get("results").cloned().unwrap_or_else(|| json!([])),
        "content_domains": parsed.get("content_domains").cloned().unwrap_or_else(|| json!([])),
        "filters": filters.clone(),
        "provider_native_filters": native_filter_controls,
        "provider_raw_count": parsed.get("provider_raw_count").cloned().unwrap_or_else(|| json!(0)),
        "provider_filtered_count": parsed.get("provider_filtered_count").cloned().unwrap_or_else(|| json!(0)),
        "retry_attempts": fetched.get("retry_attempts").cloned().unwrap_or_else(|| json!(1)),
        "retry_used": fetched.get("retry_used").cloned().unwrap_or_else(|| json!(false)),
        "user_agent": fetched.get("user_agent").cloned().unwrap_or_else(|| json!(DEFAULT_WEB_USER_AGENTS[0])),
        "response_hash": response_hash,
        "artifact": artifact.clone().unwrap_or(Value::Null),
        "policy_decision": policy_eval,
        "receipt": receipt,
        "provider": provider,
        "credential_source": credential_source,
        "error": if fetch_ok {
            Value::Null
        } else if error_value.is_empty() {
            Value::String(format!("{provider}_search_failed"))
        } else {
            Value::String(error_value)
        }
    })
}

fn api_search_serper(
    root: &Path,
    query: &str,
    summary_only: bool,
    human_approved: bool,
    allowed_domains: &[String],
    exclude_subdomains: bool,
    top_k: usize,
    requested_timeout_ms: u64,
) -> Value {
    let requested_url = SERPER_SEARCH_URL.to_string();
    let (policy, _policy_path_value) = load_policy(root);
    let credential_source = resolve_provider_credential_source_with_env(
        &policy,
        "serperdev",
        WebProviderFamily::Search,
        |key| std::env::var(key).ok(),
    );
    let Some(api_key) = resolve_search_provider_credential(root, &policy, "serperdev") else {
        return json!({
            "ok": false,
            "error": "serper_api_key_missing",
            "requested_url": requested_url,
            "provider": "serperdev",
            "credential_source": credential_source,
            "docs": "https://docs.openclaw.ai/tools/web"
        });
    };
    let policy_eval = crate::infring_layer1_security_bridge::evaluate_web_conduit_policy(
        root,
        &json!({
            "requested_url": requested_url,
            "domain": extract_domain(&requested_url),
            "human_approved": human_approved,
            "requests_last_minute": requests_last_minute_for_lane(root, "search")
        }),
        &policy,
    );
    let allow = policy_eval
        .get("allow")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let reason = clean_text(
        policy_eval
            .get("reason")
            .and_then(Value::as_str)
            .unwrap_or("policy_denied"),
        180,
    );
    if !allow {
        let receipt = build_receipt(
            &requested_url,
            "deny",
            None,
            0,
            &reason,
            Some("policy_denied"),
        );
        let _ = append_jsonl(&receipts_path(root), &receipt);
        return json!({
            "ok": false,
            "error": "web_conduit_policy_denied",
            "requested_url": requested_url,
            "policy_decision": policy_eval,
            "provider": "serperdev",
            "receipt": receipt
        });
    }
    let timeout_ms = requested_timeout_ms.clamp(
        1_000,
        policy_eval
            .pointer("/policy/timeout_ms")
            .and_then(Value::as_u64)
            .unwrap_or(search_default_timeout_ms(&policy)),
    );
    let max_response_bytes = policy_eval
        .pointer("/policy/max_response_bytes")
        .and_then(Value::as_u64)
        .unwrap_or(350_000) as usize;
    let retry_attempts = policy_eval
        .pointer("/policy/retry_attempts")
        .and_then(Value::as_u64)
        .unwrap_or(2)
        .clamp(1, 4) as usize;
    let fetched = fetch_serper_with_retry(
        &api_key,
        query,
        timeout_ms,
        max_response_bytes,
        retry_attempts,
        top_k,
    );
    let status_code = fetched
        .get("status_code")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let content_type = clean_text(
        fetched
            .get("content_type")
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );
    let parsed = render_serper_payload(
        fetched.get("body").and_then(Value::as_str).unwrap_or(""),
        allowed_domains,
        exclude_subdomains,
        top_k,
        max_response_bytes,
    );
    let content = clean_text(
        parsed.get("content").and_then(Value::as_str).unwrap_or(""),
        max_response_bytes,
    );
    let summary = clean_text(
        parsed.get("summary").and_then(Value::as_str).unwrap_or(""),
        900,
    );
    let response_hash = if content.is_empty() {
        String::new()
    } else {
        sha256_hex(&content)
    };
    let materialize_artifact = true;
    let artifact = if materialize_artifact {
        persist_artifact(root, &requested_url, &response_hash, &content)
    } else {
        None
    };
    let fetch_ok = fetched.get("ok").and_then(Value::as_bool).unwrap_or(false)
        && parsed.get("ok").and_then(Value::as_bool).unwrap_or(false)
        && !summary.is_empty();
    let mut error_value = clean_text(
        fetched.get("stderr").and_then(Value::as_str).unwrap_or(""),
        320,
    );
    if error_value.is_empty() {
        error_value = structured_search_http_error("serperdev", status_code).unwrap_or_else(|| {
            clean_text(
                parsed.get("error").and_then(Value::as_str).unwrap_or(""),
                220,
            )
        });
    }
    let receipt = build_receipt(
        &requested_url,
        "allow",
        if response_hash.is_empty() {
            None
        } else {
            Some(response_hash.as_str())
        },
        status_code,
        &reason,
        if error_value.is_empty() {
            None
        } else {
            Some(error_value.as_str())
        },
    );
    let _ = append_jsonl(&receipts_path(root), &receipt);
    json!({
        "ok": fetch_ok,
        "requested_url": requested_url,
        "status_code": status_code,
        "content_type": if content_type.is_empty() { Value::String("application/json".to_string()) } else { Value::String(content_type) },
        "summary": summary,
        "content": if summary_only { Value::String(String::new()) } else { Value::String(content) },
        "links": parsed.get("links").cloned().unwrap_or_else(|| json!([])),
        "results": parsed.get("results").cloned().unwrap_or_else(|| json!([])),
        "content_domains": parsed.get("content_domains").cloned().unwrap_or_else(|| json!([])),
        "provider_raw_count": parsed.get("provider_raw_count").cloned().unwrap_or_else(|| json!(0)),
        "provider_filtered_count": parsed.get("provider_filtered_count").cloned().unwrap_or_else(|| json!(0)),
        "retry_attempts": fetched.get("retry_attempts").cloned().unwrap_or_else(|| json!(1)),
        "retry_used": fetched.get("retry_used").cloned().unwrap_or_else(|| json!(false)),
        "user_agent": fetched.get("user_agent").cloned().unwrap_or_else(|| json!(DEFAULT_WEB_USER_AGENTS[0])),
        "response_hash": response_hash,
        "artifact": artifact.clone().unwrap_or(Value::Null),
        "policy_decision": policy_eval,
        "receipt": receipt,
        "provider": "serperdev",
        "credential_source": credential_source,
        "error": if fetch_ok {
            Value::Null
        } else if error_value.is_empty() {
            Value::String("serper_search_failed".to_string())
        } else {
            Value::String(error_value)
        }
    })
}
