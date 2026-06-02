    if let Some(unknown_provider) = validate_explicit_provider_hint(&provider_hint) {
        let receipt = build_receipt(
            "",
            "deny",
            None,
            0,
            "unknown_search_provider",
            Some(&unknown_provider),
        );
        let _ = append_jsonl(&receipts_path(root), &receipt);
        return json!({
            "ok": false,
            "error": "unknown_search_provider",
            "query": query,
            "query_source": query_source,
            "query_source_kind": query_source_kind,
            "query_source_confidence": query_source_confidence,
            "query_source_recovery_mode": query_source_recovery_mode,
            "query_source_lineage": query_source_lineage,
            "requested_provider": unknown_provider,
            "supported_filters": search_provider_request_contract(&policy)
                .get("supports_filters")
                .cloned()
                .unwrap_or_else(|| json!({})),
            "tool_execution_attempted": false,
            "tool_execution_gate": {
                "should_execute": false,
                "mode": "blocked",
                "reason": "unknown_search_provider",
                "source": "request_contract"
            },
            "meta_query_blocked": false,
            "cache_status": "skipped_validation",
            "cache_store_allowed": false,
            "cache_write_attempted": false,
            "cache_skip_reason": "unknown_search_provider",
            "retry": search_retry_envelope_for_error("unknown_search_provider"),
            "provider_catalog": provider_catalog_snapshot(root, &policy),
            "process_summary": runtime_web_process_summary(
                "web_search",
                "request_contract_blocked",
                false,
                &json!({
                    "should_execute": false,
                    "mode": "blocked",
                    "reason": "unknown_search_provider",
                    "source": "request_contract"
                }),
                &json!({
                    "blocked": false,
                    "reason": "not_evaluated"
                }),
                &json!([]),
                "none",
                Some("unknown_search_provider")
            ),
            "receipt": receipt
        });
    }
    if let Some(mut unsupported) = unsupported_search_filter_response(request) {
        let receipt = build_receipt(
            "",
            "deny",
            None,
            0,
            unsupported
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("unsupported_search_filter"),
            unsupported
                .get("unsupported_filter")
                .and_then(Value::as_str),
        );
        let _ = append_jsonl(&receipts_path(root), &receipt);
        if let Some(obj) = unsupported.as_object_mut() {
            obj.insert("query".to_string(), Value::String(query.clone()));
            obj.insert("query_source".to_string(), json!(query_source));
            obj.insert("query_source_kind".to_string(), json!(query_source_kind));
            obj.insert(
                "query_source_confidence".to_string(),
                json!(query_source_confidence),
            );
            obj.insert(
                "query_source_recovery_mode".to_string(),
                json!(query_source_recovery_mode),
            );
            obj.insert("query_source_lineage".to_string(), query_source_lineage.clone());
            obj.insert(
                "query_shape_fetch_url_candidate".to_string(),
                json!(query_shape_fetch_url_candidate.clone()),
            );
            obj.insert(
                "query_shape_fetch_url_candidate_kind".to_string(),
                json!(query_shape_fetch_url_candidate_kind),
            );
            obj.insert("query_shape_error".to_string(), json!(query_shape_error));
            obj.insert(
                "query_shape_category".to_string(),
                json!(search_query_shape_category(query_shape_error)),
            );
            obj.insert(
                "query_shape_recommended_action".to_string(),
                json!(search_query_shape_recommended_action(query_shape_error)),
            );
            obj.insert(
                "query_shape_route_hint".to_string(),
                json!(search_query_shape_route_hint(query_shape_error)),
            );
            obj.insert(
                "query_shape".to_string(),
                search_query_shape_contract(
                    &query,
                    query_shape_error,
                    query_shape_override,
                    query_shape_override_source,
                ),
            );
            obj.insert(
                "suggested_next_action".to_string(),
                search_query_shape_suggested_next_action(&query, query_shape_error),
            );
            obj.insert(
                "provider_hint".to_string(),
                Value::String(provider_hint.clone()),
            );
            obj.insert("filters".to_string(), normalized_filters.clone());
            obj.insert(
                "provider_catalog".to_string(),
                provider_catalog_snapshot(root, &policy),
            );
            obj.insert(
                "supported_filters".to_string(),
                search_provider_request_contract(&policy)
                    .get("supports_filters")
                    .cloned()
                    .unwrap_or_else(|| json!({})),
            );
            obj.insert("tool_execution_attempted".to_string(), json!(false));
            obj.insert(
                "tool_execution_gate".to_string(),
                json!({
                    "should_execute": false,
                    "mode": "blocked",
                    "reason": "unsupported_search_filter",
                    "source": "request_contract"
                }),
            );
            obj.insert("meta_query_blocked".to_string(), json!(false));
            obj.insert("cache_status".to_string(), json!("skipped_validation"));
            obj.insert("cache_store_allowed".to_string(), json!(false));
            obj.insert("cache_write_attempted".to_string(), json!(false));
            obj.insert("cache_skip_reason".to_string(), json!("unsupported_search_filter"));
            obj.insert(
                "retry".to_string(),
                search_retry_envelope_for_error("unsupported_search_filter"),
            );
            obj.insert(
                "process_summary".to_string(),
                runtime_web_process_summary(
                    "web_search",
                    "request_contract_blocked",
                    false,
                    &json!({
                        "should_execute": false,
                        "mode": "blocked",
                        "reason": "unsupported_search_filter",
                        "source": "request_contract"
                    }),
                    &json!({
                        "blocked": false,
                        "reason": "not_evaluated"
                    }),
                    &json!([]),
                    "none",
                    Some("unsupported_search_filter")
                ),
            );
            obj.insert("receipt".to_string(), receipt);
        }
        return unsupported;
    }
    let top_k = resolve_search_count(request, &policy);
    let timeout_ms = resolve_search_timeout_ms(request, &policy);
    let scoped_query = scoped_search_query(&query, &allowed_domains, exclude_subdomains);
    let summary_only = ["summary_only", "summaryOnly", "summary-only", "summary"]
        .iter()
        .filter_map(|key| request.get(*key))
        .find_map(|value| {
            if let Some(flag) = value.as_bool() {
                return Some(flag);
            }
            value.as_str().map(|raw| {
                matches!(
                    raw.trim().to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "on"
                )
            })
        })
        .unwrap_or(false);
    let human_approved = request
        .get("human_approved")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let approval_id = request
        .get("approval_id")
        .and_then(Value::as_str)
        .unwrap_or("");
    let (mut provider_resolution, provider_chain, selected_provider, allow_fallback) =
        resolved_search_provider_selection(root, &policy, request, &provider_hint);
    let cache_ttl_seconds = resolve_search_cache_ttl_seconds(request, &policy, "ok");
    let cache_ttl_minutes = if cache_ttl_seconds <= 0 {
        0
    } else {
        ((cache_ttl_seconds + 59) / 60) as u64
    };
    let cache_key = search_cache_key(
        &query,
        &scoped_query,
        &allowed_domains,
        exclude_subdomains,
        top_k,
        summary_only,
        &provider_chain,
        &normalized_filters,
    );
    if cache_ttl_seconds > 0 {
        if let Some(mut cached) = load_search_cache(root, &cache_key) {
        if let Some(obj) = cached.as_object_mut() {
            obj.insert(
                "type".to_string(),
                Value::String("web_conduit_search".to_string()),
            );
            obj.insert("query".to_string(), Value::String(query.clone()));
            obj.insert(
                "effective_query".to_string(),
                Value::String(scoped_query.clone()),
            );
            obj.insert("allowed_domains".to_string(), json!(allowed_domains));
            obj.insert("exclude_subdomains".to_string(), json!(exclude_subdomains));
            obj.insert("top_k".to_string(), json!(top_k));
            obj.insert("count".to_string(), json!(top_k));
            obj.insert("timeout_ms".to_string(), json!(timeout_ms));
            obj.insert("cache_ttl_minutes".to_string(), json!(cache_ttl_minutes));
            obj.insert("filters".to_string(), normalized_filters.clone());
            obj.insert(
                "provider_hint".to_string(),
                Value::String(provider_hint.clone()),
            );
            obj.insert("query_source".to_string(), json!(query_source));
            obj.insert("query_source_kind".to_string(), json!(query_source_kind));
            obj.insert("provider_chain".to_string(), json!(provider_chain));
            obj.insert(
                "provider_resolution".to_string(),
                provider_resolution.clone(),
            );
            obj.insert(
                "query_shape_override_used".to_string(),
                json!(query_shape_override),
            );
            obj.insert(
                "query_shape_override_source".to_string(),
                json!(query_shape_override_source),
            );
            obj.insert(
                "query_shape_stats".to_string(),
                search_query_shape_stats(&query),
            );
            obj.insert(
                "query_shape_fetch_url_candidate".to_string(),
                json!(query_shape_fetch_url_candidate.clone()),
            );
            obj.insert(
                "query_shape_fetch_url_candidate_kind".to_string(),
                json!(query_shape_fetch_url_candidate_kind),
            );
            obj.insert(
                "query_shape_error".to_string(),
                json!(query_shape_error),
            );
            obj.insert(
                "query_shape_category".to_string(),
                json!(search_query_shape_category(query_shape_error)),
            );
            obj.insert(
                "query_shape_recommended_action".to_string(),
                json!(search_query_shape_recommended_action(query_shape_error)),
            );
            obj.insert(
                "query_shape_route_hint".to_string(),
                json!(search_query_shape_route_hint(query_shape_error)),
            );
            obj.insert(
                "query_shape".to_string(),
                search_query_shape_contract(
                    &query,
                    query_shape_error,
                    query_shape_override,
                    query_shape_override_source,
                ),
            );
            obj.insert(
                "suggested_next_action".to_string(),
                search_query_shape_suggested_next_action(&query, query_shape_error),
            );
            obj.insert(
                "provider_health".to_string(),
                provider_health_snapshot(root, &provider_chain),
            );
            obj.insert("cache_status".to_string(), json!("hit"));
        }
        return cached;
        }
    }
    let primary_url = web_search_url(&scoped_query);
    let lite_url = web_search_lite_url(&scoped_query);
    let mut selected = Value::Null;
    let initial_selected_provider = selected_provider.clone();
    let mut executed_provider = String::new();
    let mut attempted = Vec::<String>::new();
    let mut skipped = Vec::<Value>::new();
    let mut provider_errors = Vec::<Value>::new();
    let mut last_payload = None::<Value>;
    let tool_surface_status = provider_resolution
        .get("tool_surface_status")
        .or_else(|| provider_resolution.pointer("/tool_surface_health/status"))
        .and_then(Value::as_str)
        .unwrap_or("unavailable")
        .to_string();
