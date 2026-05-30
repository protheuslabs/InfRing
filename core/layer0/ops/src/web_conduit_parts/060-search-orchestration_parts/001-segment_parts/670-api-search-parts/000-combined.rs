// Layer ownership: core/layer0/ops::web-search-orchestration (authoritative)
{
    let (query_raw, query_source) = search_query_and_source(request);
    let query = search_strip_invisible_unicode(&clean_text(&query_raw, 600));
    let _query_invisible_unicode_removed_count =
        search_invisible_unicode_removed_count(&query_raw);
    let _query_invisible_unicode_stripped = _query_invisible_unicode_removed_count > 0;
    let query_source_kind = search_query_source_kind(query_source);
    let query_source_confidence = search_query_source_confidence(query_source_kind);
    let query_source_recovery_mode = search_query_source_recovery_mode(query_source);
    let query_source_lineage =
        search_query_source_lineage(query_source, query_source_kind, query_source_confidence);
    let provider_hint = clean_text(
        request
            .get("provider")
            .or_else(|| request.get("source"))
            .or_else(|| request.get("search_provider"))
            .or_else(|| request.get("searchProvider"))
            .and_then(Value::as_str)
            .unwrap_or("auto"),
        40,
    )
    .to_ascii_lowercase();
    let (policy, _policy_path_value) = load_policy(root);
    let query_shape_override = search_query_shape_override(&policy, request);
    let query_shape_override_source = search_query_shape_override_source(&policy, request);
    let query_shape_error = search_query_shape_error_code(&query);
    let query_shape_fetch_url_candidate = search_query_fetch_url_candidate(&query).unwrap_or_default();
    let query_shape_fetch_url_candidate_kind = search_query_fetch_url_candidate_kind(&query);
    if query_shape_error != "none" && !query_shape_override {
        let reason = query_shape_error;
        let receipt = build_receipt("", "deny", None, 0, reason, Some(reason));
        let _ = append_jsonl(&receipts_path(root), &receipt);
        let summary = if reason == "query_payload_dump_detected" {
            "Query looks like pasted output/log content instead of a concise web request. Submit a short intent-focused query."
        } else if reason == "query_prefers_fetch_url" {
            "Query is a direct URL. Use web fetch for this input instead of web search."
        } else {
            "Query shape is invalid for web search. Submit concise query text with clear keywords."
        };
        let mut out = search_early_validation_payload(
            reason,
            &query,
            Some(summary),
            &provider_hint,
            "skipped_validation",
            reason,
            reason,
            false,
            Some("submit concise query text (recommended <= 300 chars)"),
            receipt,
        );
        if let Some(obj) = out.as_object_mut() {
            obj.insert("query_shape_blocked".to_string(), json!(true));
            obj.insert("query_shape_error".to_string(), json!(reason));
            obj.insert("query_shape_stats".to_string(), search_query_shape_stats(&query));
            obj.insert(
                "query_shape_fetch_url_candidate".to_string(),
                json!(query_shape_fetch_url_candidate.clone()),
            );
            obj.insert(
                "query_shape_fetch_url_candidate_kind".to_string(),
                json!(query_shape_fetch_url_candidate_kind),
            );
            obj.insert("query_shape_override_allowed".to_string(), json!(false));
            obj.insert("query_shape_override_used".to_string(), json!(false));
            obj.insert(
                "query_shape_override_source".to_string(),
                json!(query_shape_override_source),
            );
            obj.insert(
                "query_shape_category".to_string(),
                json!(search_query_shape_category(reason)),
            );
            obj.insert(
                "query_shape_recommended_action".to_string(),
                json!(search_query_shape_recommended_action(reason)),
            );
            obj.insert(
                "query_shape_route_hint".to_string(),
                json!(search_query_shape_route_hint(reason)),
            );
            obj.insert(
                "query_shape".to_string(),
                search_query_shape_contract(&query, reason, false, query_shape_override_source),
            );
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
                "suggested_next_action".to_string(),
                search_query_shape_suggested_next_action(&query, reason),
            );
            obj.insert(
                "retry".to_string(),
                search_retry_envelope_for_error(reason),
            );
        }
        return out;
    }
    if let Some(mut early) = search_early_validation_response(root, request, &query) {
        let early_error = clean_text(
            early.get("error").and_then(Value::as_str).unwrap_or(""),
            120,
        );
        if let Some(obj) = early.as_object_mut() {
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
                "retry".to_string(),
                search_retry_envelope_for_error(&early_error),
            );
        }
        return early;
    }
    let normalized_filters = normalized_search_filters(request);
    let allowed_domains =
        normalize_allowed_domains(request.get("allowed_domains").unwrap_or(&Value::Null));
    let exclude_subdomains = request
        .get("exclude_subdomains")
        .or_else(|| request.get("exact_domain_only"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let raw_freshness = clean_text(
        request
            .get("freshness")
            .or_else(|| request.get("search_recency_filter"))
            .or_else(|| request.get("searchRecencyFilter"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        60,
    );
    let raw_date_after = clean_text(
        request
            .get("date_after")
            .or_else(|| request.get("dateAfter"))
            .or_else(|| request.get("search_after_date"))
            .or_else(|| request.get("searchAfterDate"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        40,
    );
    let raw_date_before = clean_text(
        request
            .get("date_before")
            .or_else(|| request.get("dateBefore"))
            .or_else(|| request.get("search_before_date"))
            .or_else(|| request.get("searchBeforeDate"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        40,
    );
    if !raw_freshness.is_empty() && (!raw_date_after.is_empty() || !raw_date_before.is_empty()) {
        let receipt = build_receipt(
            "",
            "deny",
            None,
            0,
            "conflicting_time_filters",
            Some("conflicting_time_filters"),
        );
        let _ = append_jsonl(&receipts_path(root), &receipt);
        return json!({
            "ok": false,
            "error": "conflicting_time_filters",
            "query": query,
            "query_source": query_source,
            "query_source_kind": query_source_kind,
            "query_source_confidence": query_source_confidence,
            "query_source_recovery_mode": query_source_recovery_mode,
            "query_source_lineage": query_source_lineage,
            "query_shape_fetch_url_candidate": query_shape_fetch_url_candidate,
            "query_shape_fetch_url_candidate_kind": query_shape_fetch_url_candidate_kind,
            "query_shape_error": query_shape_error,
            "query_shape_category": search_query_shape_category(query_shape_error),
            "query_shape_recommended_action": search_query_shape_recommended_action(query_shape_error),
            "query_shape_route_hint": search_query_shape_route_hint(query_shape_error),
            "query_shape": search_query_shape_contract(
                &query,
                query_shape_error,
                query_shape_override,
                query_shape_override_source
            ),
            "suggested_next_action": search_query_shape_suggested_next_action(&query, query_shape_error),
            "freshness": raw_freshness,
            "date_after": raw_date_after,
            "date_before": raw_date_before,
            "summary": "freshness cannot be combined with date_after/date_before. Use either freshness or an explicit date range.",
            "filters": normalized_filters.clone(),
            "provider_hint": provider_hint.clone(),
            "tool_execution_attempted": false,
            "tool_execution_gate": {
                "should_execute": false,
                "mode": "blocked",
                "reason": "conflicting_time_filters",
                "source": "request_contract"
            },
            "meta_query_blocked": false,
            "cache_status": "skipped_validation",
            "cache_store_allowed": false,
            "cache_write_attempted": false,
            "cache_skip_reason": "conflicting_time_filters",
            "retry": search_retry_envelope_for_error("conflicting_time_filters"),
            "provider_catalog": provider_catalog_snapshot(root, &policy),
            "process_summary": runtime_web_process_summary(
                "web_search",
                "request_contract_blocked",
                false,
                &json!({
                    "should_execute": false,
                    "mode": "blocked",
                    "reason": "conflicting_time_filters",
                    "source": "request_contract"
                }),
                &json!({
                    "blocked": false,
                    "reason": "not_evaluated"
                }),
                &json!([]),
                "none",
                Some("conflicting_time_filters")
            ),
            "receipt": receipt
        });
    }
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
    let tool_surface_ready = provider_resolution
        .get("tool_surface_ready")
        .or_else(|| {
            provider_resolution.pointer("/tool_surface_health/selected_provider_ready")
        })
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let tool_surface_blocking_reason = provider_resolution
        .pointer("/tool_surface_health/blocking_reason")
        .and_then(Value::as_str)
        .unwrap_or("none")
        .to_string();
    let tool_execution_gate = provider_resolution
        .get("tool_execution_gate")
        .cloned()
        .unwrap_or_else(|| {
            runtime_web_execution_gate(
                &tool_surface_status,
                tool_surface_ready,
                allow_fallback,
                &tool_surface_blocking_reason,
            )
        });
    let tool_execution_allowed = tool_execution_gate
        .get("should_execute")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let search_attempt_signature = sha256_hex(&format!(
        "{}|{}|{}|{}|{}|{}|{}",
        scoped_query,
        provider_chain.join(","),
        top_k,
        summary_only,
        timeout_ms,
        allow_fallback,
        tool_surface_status
    ));
    let replay_policy =
        runtime_web_replay_policy(&policy, request, &tool_surface_status, tool_surface_ready);
    let replay_window = replay_policy
        .get("window")
        .and_then(Value::as_u64)
        .unwrap_or(24)
        .clamp(1, 200) as usize;
    let replay_threshold = replay_policy
        .get("block_threshold")
        .and_then(Value::as_u64)
        .unwrap_or(3)
        .clamp(2, 200) as usize;
    let replay_enabled = replay_policy
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let replay_bypass = runtime_web_replay_bypass(&policy, request, human_approved);
    let replay_bypassed = replay_bypass
        .get("bypassed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let replay_cooldown_base_seconds = replay_policy
        .get("cooldown_base_seconds")
        .and_then(Value::as_u64)
        .unwrap_or(30);
    let replay_cooldown_step_seconds = replay_policy
        .get("cooldown_step_seconds")
        .and_then(Value::as_u64)
        .unwrap_or(15);
    let replay_cooldown_max_seconds = replay_policy
        .get("cooldown_max_seconds")
        .and_then(Value::as_u64)
        .unwrap_or(180);
    let attempt_replay_guard =
        if replay_enabled && !replay_bypassed {
            recent_tool_attempt_replay_guard(
                root,
                &search_attempt_signature,
                replay_window,
                replay_threshold,
                replay_cooldown_base_seconds,
                replay_cooldown_step_seconds,
                replay_cooldown_max_seconds,
            )
        } else if replay_bypassed {
            runtime_web_replay_guard_passthrough(
                "replay_guard_bypassed",
                &search_attempt_signature,
                replay_window,
                replay_threshold,
                replay_cooldown_base_seconds,
                replay_cooldown_step_seconds,
                replay_cooldown_max_seconds,
                &replay_bypass,
            )
        } else {
            runtime_web_replay_guard_passthrough(
                "replay_policy_disabled",
                &search_attempt_signature,
                replay_window,
                replay_threshold,
                replay_cooldown_base_seconds,
                replay_cooldown_step_seconds,
                replay_cooldown_max_seconds,
                &replay_bypass,
            )
        };
    let attempt_replay_blocked = attempt_replay_guard
        .get("blocked")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let replay_retry_after_seconds = attempt_replay_guard.get("retry_after_seconds");
    let replay_retry_after_seconds =
        search_retry_after_seconds_from_value(replay_retry_after_seconds);
    let replay_retry_lane = clean_text(
        attempt_replay_guard
            .get("retry_lane")
            .and_then(Value::as_str)
            .unwrap_or("change_query_or_provider"),
        80,
    );
    if !tool_execution_allowed {
        let preflight_error = if tool_surface_status == "unavailable" {
            "web_search_tool_surface_unavailable"
        } else if tool_surface_status == "degraded" {
            "web_search_tool_surface_degraded"
        } else {
            "web_search_tool_execution_blocked"
        };
        let search_url = match selected_provider.as_str() {
            "duckduckgo_lite" => lite_url.clone(),
            "google_news_rss" => web_search_google_news_rss_url(&scoped_query),
            "bing_rss" => web_search_bing_rss_url(&scoped_query),
            _ => primary_url.clone(),
        };
        let mut receipt = build_receipt(
            &search_url,
            "deny",
            None,
            0,
            "search_preflight_gate_blocked",
            Some(preflight_error),
        );
        if let Some(receipt_obj) = receipt.as_object_mut() {
            receipt_obj.insert(
                "attempt_signature".to_string(),
                Value::String(search_attempt_signature.clone()),
            );
            receipt_obj.insert(
                "gate_mode".to_string(),
                Value::String(
                    tool_execution_gate
                        .get("mode")
                        .and_then(Value::as_str)
                        .unwrap_or("blocked")
                        .to_string(),
                ),
            );
        }
        let _ = append_jsonl(&receipts_path(root), &receipt);
        let mut out = serde_json::Map::<String, Value>::new();
        out.insert("ok".to_string(), Value::Bool(false));
        out.insert("error".to_string(), Value::String(preflight_error.to_string()));
        out.insert(
            "summary".to_string(),
            Value::String(
                "Web search execution was blocked by runtime tooling gate before provider calls were attempted."
                    .to_string(),
            ),
        );
        out.insert("content".to_string(), Value::String(String::new()));
        out.insert(
            "type".to_string(),
            Value::String("web_conduit_search".to_string()),
        );
        out.insert("query".to_string(), Value::String(query.clone()));
        out.insert(
            "effective_query".to_string(),
            Value::String(scoped_query.clone()),
        );
        out.insert("allowed_domains".to_string(), json!(allowed_domains.clone()));
        out.insert("exclude_subdomains".to_string(), json!(exclude_subdomains));
        out.insert("top_k".to_string(), json!(top_k));
        out.insert("count".to_string(), json!(top_k));
        out.insert("timeout_ms".to_string(), json!(timeout_ms));
        out.insert(
            "provider_hint".to_string(),
            Value::String(provider_hint.clone()),
        );
        out.insert("query_source".to_string(), json!(query_source));
        out.insert("query_source_kind".to_string(), json!(query_source_kind));
        out.insert(
            "query_source_confidence".to_string(),
            json!(query_source_confidence),
        );
        out.insert(
            "query_source_recovery_mode".to_string(),
            json!(query_source_recovery_mode),
        );
        out.insert("query_source_lineage".to_string(), query_source_lineage.clone());
        out.insert(
            "query_shape_fetch_url_candidate".to_string(),
            json!(query_shape_fetch_url_candidate.clone()),
        );
        out.insert(
            "query_shape_fetch_url_candidate_kind".to_string(),
            json!(query_shape_fetch_url_candidate_kind),
        );
        out.insert("query_shape_error".to_string(), json!(query_shape_error));
        out.insert(
            "query_shape_category".to_string(),
            json!(search_query_shape_category(query_shape_error)),
        );
        out.insert(
            "query_shape_recommended_action".to_string(),
            json!(search_query_shape_recommended_action(query_shape_error)),
        );
        out.insert(
            "query_shape_route_hint".to_string(),
            json!(search_query_shape_route_hint(query_shape_error)),
        );
        out.insert(
            "query_shape".to_string(),
            search_query_shape_contract(
                &query,
                query_shape_error,
                query_shape_override,
                query_shape_override_source,
            ),
        );
        out.insert(
            "suggested_next_action".to_string(),
            search_query_shape_suggested_next_action(&query, query_shape_error),
        );
        out.insert("provider_chain".to_string(), json!(provider_chain.clone()));
        out.insert("provider_resolution".to_string(), provider_resolution);
        out.insert(
            "tool_surface_status".to_string(),
            Value::String(tool_surface_status.clone()),
        );
        out.insert("tool_surface_ready".to_string(), Value::Bool(tool_surface_ready));
        out.insert(
            "tool_surface_blocking_reason".to_string(),
            Value::String(tool_surface_blocking_reason.clone()),
        );
        out.insert("tool_execution_gate".to_string(), tool_execution_gate.clone());
        out.insert("replay_policy".to_string(), replay_policy.clone());
        out.insert("replay_bypass".to_string(), replay_bypass.clone());
        out.insert("attempt_replay_guard".to_string(), attempt_replay_guard.clone());
        out.insert(
            "attempt_signature".to_string(),
            Value::String(search_attempt_signature.clone()),
        );
        out.insert(
            "retry".to_string(),
            search_retry_envelope_runtime(
                if preflight_error == "web_search_tool_surface_unavailable" {
                    "restore_tool_surface_or_use_supported_provider"
                } else if preflight_error == "web_search_tool_surface_degraded" {
                    "stabilize_provider_runtime_and_retry"
                } else {
                    "change_query_or_provider"
                },
                preflight_error,
                &replay_retry_lane,
                replay_retry_after_seconds,
            ),
        );
        out.insert(
            "process_summary".to_string(),
            runtime_web_process_summary(
                "web_search",
                "preflight_blocked",
                false,
                &tool_execution_gate,
                &attempt_replay_guard,
                &json!(provider_chain.clone()),
                &selected_provider,
                Some(preflight_error),
            ),
        );
        out.insert("receipt".to_string(), receipt);
        return Value::Object(out);
    }
    if attempt_replay_blocked {
        let search_url = match selected_provider.as_str() {
            "duckduckgo_lite" => lite_url.clone(),
            "google_news_rss" => web_search_google_news_rss_url(&scoped_query),
            "bing_rss" => web_search_bing_rss_url(&scoped_query),
            _ => primary_url.clone(),
        };
        let mut receipt = build_receipt(
            &search_url,
            "deny",
            None,
            0,
            "search_replay_guard_blocked",
            Some("web_search_duplicate_attempt_suppressed"),
        );
        if let Some(receipt_obj) = receipt.as_object_mut() {
            receipt_obj.insert(
                "attempt_signature".to_string(),
                Value::String(search_attempt_signature.clone()),
            );
            receipt_obj.insert(
                "gate_mode".to_string(),
                Value::String(
                    tool_execution_gate
                        .get("mode")
                        .and_then(Value::as_str)
                        .unwrap_or("blocked")
                        .to_string(),
                ),
            );
        }
        let _ = append_jsonl(&receipts_path(root), &receipt);
        let mut out = serde_json::Map::<String, Value>::new();
        out.insert("ok".to_string(), Value::Bool(false));
        out.insert(
            "error".to_string(),
            Value::String("web_search_duplicate_attempt_suppressed".to_string()),
        );
        out.insert(
            "summary".to_string(),
            Value::String(
                "Repeated identical web search attempts were suppressed by replay guard. Adjust the query or provider constraints before retrying."
                    .to_string(),
            ),
        );
        out.insert("content".to_string(), Value::String(String::new()));
        out.insert(
            "type".to_string(),
            Value::String("web_conduit_search".to_string()),
        );
        out.insert("query".to_string(), Value::String(query.clone()));
        out.insert(
            "effective_query".to_string(),
            Value::String(scoped_query.clone()),
        );
        out.insert("allowed_domains".to_string(), json!(allowed_domains.clone()));
        out.insert("exclude_subdomains".to_string(), json!(exclude_subdomains));
        out.insert("top_k".to_string(), json!(top_k));
        out.insert("count".to_string(), json!(top_k));
        out.insert("timeout_ms".to_string(), json!(timeout_ms));
        out.insert(
            "provider_hint".to_string(),
            Value::String(provider_hint.clone()),
        );
        out.insert("query_source".to_string(), json!(query_source));
        out.insert("query_source_kind".to_string(), json!(query_source_kind));
        out.insert(
            "query_source_confidence".to_string(),
            json!(query_source_confidence),
        );
        out.insert(
            "query_source_recovery_mode".to_string(),
            json!(query_source_recovery_mode),
        );
        out.insert("query_source_lineage".to_string(), query_source_lineage.clone());
        out.insert(
            "query_shape_fetch_url_candidate".to_string(),
            json!(query_shape_fetch_url_candidate.clone()),
        );
        out.insert(
            "query_shape_fetch_url_candidate_kind".to_string(),
            json!(query_shape_fetch_url_candidate_kind),
        );
        out.insert("query_shape_error".to_string(), json!(query_shape_error));
        out.insert(
            "query_shape_category".to_string(),
            json!(search_query_shape_category(query_shape_error)),
        );
        out.insert(
            "query_shape_recommended_action".to_string(),
            json!(search_query_shape_recommended_action(query_shape_error)),
        );
        out.insert(
            "query_shape_route_hint".to_string(),
            json!(search_query_shape_route_hint(query_shape_error)),
        );
        out.insert(
            "query_shape".to_string(),
            search_query_shape_contract(
                &query,
                query_shape_error,
                query_shape_override,
                query_shape_override_source,
            ),
        );
        out.insert(
            "suggested_next_action".to_string(),
            search_query_shape_suggested_next_action(&query, query_shape_error),
        );
        out.insert("provider_chain".to_string(), json!(provider_chain.clone()));
        out.insert("provider_resolution".to_string(), provider_resolution);
        out.insert(
            "tool_surface_status".to_string(),
            Value::String(tool_surface_status.clone()),
        );
        out.insert("tool_surface_ready".to_string(), Value::Bool(tool_surface_ready));
        out.insert(
            "tool_surface_blocking_reason".to_string(),
            Value::String(tool_surface_blocking_reason.clone()),
        );
        out.insert("tool_execution_gate".to_string(), tool_execution_gate.clone());
        out.insert("replay_policy".to_string(), replay_policy.clone());
        out.insert("replay_bypass".to_string(), replay_bypass.clone());
        out.insert("attempt_replay_guard".to_string(), attempt_replay_guard.clone());
        out.insert(
            "attempt_signature".to_string(),
            Value::String(search_attempt_signature.clone()),
        );
        out.insert(
            "retry".to_string(),
            search_retry_envelope_runtime(
                "narrow_query_or_wait_for_replay_window",
                "web_search_duplicate_attempt_suppressed",
                &replay_retry_lane,
                replay_retry_after_seconds,
            ),
        );
        out.insert(
            "process_summary".to_string(),
            runtime_web_process_summary(
                "web_search",
                "replay_suppressed",
                false,
                &tool_execution_gate,
                &attempt_replay_guard,
                &json!(provider_chain.clone()),
                &selected_provider,
                Some("web_search_duplicate_attempt_suppressed"),
            ),
        );
        out.insert("receipt".to_string(), receipt);
        return Value::Object(out);
    }
    for provider in &provider_chain {
        if let Some(open_until) = provider_circuit_open_until(root, provider, &policy) {
            let provider_health = provider_health_snapshot(root, &[provider.clone()]);
            let provider_health_row = provider_health
                .as_array()
                .and_then(|rows| rows.first())
                .cloned()
                .unwrap_or_else(|| json!({}));
            let circuit_last_error = clean_text(
                provider_health_row
                    .get("last_error")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                220,
            );
            let circuit_failure_class = clean_text(
                provider_health_row
                    .get("last_failure_class")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                80,
            );
            skipped.push(json!({
                "provider": provider,
                "reason": "circuit_open",
                "open_until": open_until,
                "last_error": circuit_last_error,
                "last_failure_class": circuit_failure_class
            }));
            if !allow_fallback {
                let circuit_error = if circuit_last_error.is_empty() {
                    "provider_circuit_open".to_string()
                } else {
                    format!("provider_circuit_open:{circuit_last_error}")
                };
                last_payload = Some(json!({
                    "ok": false,
                    "error": circuit_error,
                    "summary": format!(
                        "Search provider \"{provider}\" is temporarily unavailable because its circuit breaker is open."
                    ),
                    "content": "",
                    "provider": provider,
                    "provider_unavailable_reason": "circuit_open",
                    "circuit_open_until": open_until,
                    "provider_circuit_last_error": circuit_last_error,
                    "provider_circuit_failure_class": circuit_failure_class
                }));
                break;
            }
            continue;
        }
        attempted.push(provider.clone());
        let candidate = match provider.as_str() {
            "tavily" | "exa" | "brave" => api_search_structured_provider(
                root,
                provider,
                &scoped_query,
                summary_only,
                human_approved,
                &allowed_domains,
                exclude_subdomains,
                top_k,
                timeout_ms,
            ),
            "serperdev" => api_search_serper(
                root,
                &scoped_query,
                summary_only,
                human_approved,
                &allowed_domains,
                exclude_subdomains,
                top_k,
                timeout_ms,
            ),
            "browser_serp" => api_search_browser_serp(
                root,
                &scoped_query,
                summary_only,
                &allowed_domains,
                exclude_subdomains,
                top_k,
                timeout_ms,
            ),
            "duckduckgo_lite" => api_fetch(
                root,
                &json!({
                    "url": lite_url,
                    "summary_only": summary_only,
                    "human_approved": human_approved,
                    "approval_id": approval_id,
                    "timeout_ms": timeout_ms
                }),
            ),
            "bing_rss" => api_search_bing_rss(
                &scoped_query,
                summary_only,
                &allowed_domains,
                exclude_subdomains,
                top_k,
                timeout_ms,
            ),
            "google_news_rss" => api_search_google_news_rss(
                &scoped_query,
                summary_only,
                &allowed_domains,
                exclude_subdomains,
                top_k,
                timeout_ms,
            ),
            _ => api_fetch(
                root,
                &json!({
                    "url": primary_url,
                    "summary_only": summary_only,
                    "human_approved": human_approved,
                    "approval_id": approval_id,
                    "timeout_ms": timeout_ms
                }),
            ),
        };
        if search_payload_usable_for_query(&candidate, &scoped_query) {
            record_provider_attempt(root, provider, true, "", &policy);
            executed_provider = provider.clone();
            selected = candidate;
            break;
        }
        let reason = search_payload_error_for_query(&candidate, &scoped_query);
        record_provider_attempt(root, provider, false, &reason, &policy);
        provider_errors.push(json!({
            "provider": provider,
            "error": reason,
            "challenge": payload_looks_like_search_challenge(&candidate),
            "low_signal": payload_looks_low_signal_search(&candidate),
            "query_mismatch": search_payload_query_mismatch(&candidate, &scoped_query),
            "status_code": candidate.get("status_code").and_then(Value::as_i64).unwrap_or(0)
        }));
        last_payload = Some(candidate);
        if !allow_fallback {
            break;
        }
    }
    let mut out = if !selected.is_null() {
        selected
    } else {
        last_payload.unwrap_or_else(|| {
            json!({
                "ok": false,
                "error": "search_providers_exhausted",
                "summary": "Search providers returned no usable findings. Retry with narrower query or explicit source URLs.",
                "content": ""
            })
        })
    };
    let final_selected_provider = if executed_provider.is_empty() {
        selected_provider.clone()
    } else {
        executed_provider.clone()
    };
    if let Some(obj) = provider_resolution.as_object_mut() {
        if final_selected_provider != initial_selected_provider {
            obj.insert(
                "initial_selected_provider".to_string(),
                json!(initial_selected_provider),
            );
            obj.insert("selection_fallback_used".to_string(), json!(true));
        } else {
            obj.insert("selection_fallback_used".to_string(), json!(false));
        }
        obj.insert(
            "selected_provider".to_string(),
            json!(final_selected_provider),
        );
    }
    if out
        .get("provider")
        .and_then(Value::as_str)
        .unwrap_or("")
        .is_empty()
    {
        if let Some(obj) = out.as_object_mut() {
            obj.insert(
                "provider".to_string(),
                if final_selected_provider.is_empty() {
                    Value::String("none".to_string())
                } else {
                    Value::String(final_selected_provider.clone())
                },
            );
        }
    }
    let provider_error_is_configuration_terminal = |row: &Value| {
        let error = row
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        error.contains("api_key_missing")
            || error.contains("api key missing")
            || error.contains("credential_missing")
            || error.contains("credential_unresolved")
            || error.contains("credential unresolved")
            || error.contains("key_unresolved")
            || error.contains("missing credential")
            || error.contains("missing api key")
    };
    let provider_error_is_pure_query_mismatch = |row: &Value| {
        row.get("query_mismatch")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            && !provider_error_is_configuration_terminal(row)
            && !row
                .get("challenge")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            && !row
                .get("low_signal")
                .and_then(Value::as_bool)
                .unwrap_or(false)
    };
    let provider_errors_have_configuration_terminal = provider_errors
        .iter()
        .any(|row| provider_error_is_configuration_terminal(row));
    if !out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        let query_mismatch_only_failure = !provider_errors.is_empty()
            && provider_errors
                .iter()
                .all(|row| provider_error_is_pure_query_mismatch(row));
        if let Some(obj) = out.as_object_mut() {
            let current_error = obj.get("error").and_then(Value::as_str).unwrap_or("");
            let current_error_is_configuration_terminal = {
                let lowered = current_error.to_ascii_lowercase();
                lowered.contains("api_key_missing")
                    || lowered.contains("api key missing")
                    || lowered.contains("credential_missing")
                    || lowered.contains("credential_unresolved")
                    || lowered.contains("credential unresolved")
                    || lowered.contains("key_unresolved")
                    || lowered.contains("missing credential")
                    || lowered.contains("missing api key")
            };
            let chain_has_quality_or_config_failures = provider_errors.iter().any(|row| {
                let error = row
                    .get("error")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_ascii_lowercase();
                row.get("low_signal").and_then(Value::as_bool).unwrap_or(false)
                    || row
                        .get("query_mismatch")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                    || error.contains("api_key_missing")
                    || error.contains("api key missing")
                    || error.contains("credential_missing")
                    || error.contains("credential_unresolved")
                    || error.contains("credential unresolved")
                    || error.contains("key_unresolved")
                    || error.contains("missing credential")
                    || error.contains("missing api key")
            });
            if current_error_is_configuration_terminal && chain_has_quality_or_config_failures {
                obj.insert(
                    "error".to_string(),
                    Value::String("search_providers_exhausted".to_string()),
                );
                obj.insert(
                    "summary".to_string(),
                    Value::String(
                        "Search provider chain exhausted: free providers returned low-signal or off-topic results, and credentialed providers were not configured."
                            .to_string(),
                    ),
                );
            }
            let current_error = obj.get("error").and_then(Value::as_str).unwrap_or("");
            if current_error.is_empty() || current_error == "search_providers_exhausted" {
                if provider_errors_have_configuration_terminal {
                    obj.insert(
                        "error".to_string(),
                        Value::String("search_providers_exhausted".to_string()),
                    );
                    obj.insert(
                        "summary".to_string(),
                        Value::String(
                            "Search provider chain exhausted: credentialed providers were not configured, and available fallback providers did not return usable evidence."
                                .to_string(),
                        ),
                    );
                } else if query_mismatch_only_failure {
                    obj.insert(
                        "error".to_string(),
                        Value::String("query_result_mismatch".to_string()),
                    );
                    obj.insert(
                        "summary".to_string(),
                        Value::String(
                            "Search providers returned off-topic results for this query. Retry with narrower terms or explicit source URLs."
                                .to_string(),
                        ),
                    );
                } else if tool_surface_status == "unavailable" {
                    obj.insert(
                        "error".to_string(),
                        Value::String("web_search_tool_surface_unavailable".to_string()),
                    );
                    obj.insert(
                        "summary".to_string(),
                        Value::String(
                            "Web search tool surface is currently unavailable. Retry after provider runtime is restored."
                                .to_string(),
                        ),
                    );
                } else if tool_surface_status == "degraded"
                    && (attempted.is_empty() || !tool_surface_ready)
                {
                    obj.insert(
                        "error".to_string(),
                        Value::String("web_search_tool_surface_degraded".to_string()),
                    );
                    obj.insert(
                        "summary".to_string(),
                        Value::String(
                            "Web search tooling is degraded (provider readiness mismatch). Retry after credentials or provider runtime are repaired."
                                .to_string(),
                        ),
                    );
                }
            }
            if obj
                .get("summary")
                .and_then(Value::as_str)
                .unwrap_or("")
                .is_empty()
            {
                obj.insert(
                    "summary".to_string(),
                    Value::String(
                        "Search providers returned no usable findings. Retry with narrower query or explicit source URLs.".to_string(),
                    ),
                );
            }
            if obj.get("error").is_none() {
                obj.insert(
                    "error".to_string(),
                    Value::String("search_providers_exhausted".to_string()),
                );
            }
        }
    }
    let used_lite_fallback = final_selected_provider == "duckduckgo_lite";
    let used_bing_fallback = final_selected_provider == "bing_rss";
    let (
        search_lite_fallback_reason,
        search_lite_fallback_triggered_by_challenge,
        search_lite_fallback_triggered_by_low_signal,
        search_lite_fallback_trigger_provider,
    ) = search_lite_fallback_reason(
        used_lite_fallback,
        &initial_selected_provider,
        provider_errors.as_slice(),
    );
    let (
        search_bing_fallback_reason,
        search_bing_fallback_triggered_by_challenge,
        search_bing_fallback_triggered_by_low_signal,
        search_bing_fallback_trigger_provider,
    ) = search_bing_fallback_reason(
        used_bing_fallback,
        &initial_selected_provider,
        provider_errors.as_slice(),
    );
    let tool_execution_attempted = !attempted.is_empty();
    let final_error_code = out
        .get("error")
        .and_then(Value::as_str)
        .map(|raw| clean_text(raw, 120));
    let query_mismatch_only_failure = out
        .get("ok")
        .and_then(Value::as_bool)
        .map(|ok| !ok)
        .unwrap_or(true)
        && !provider_errors.is_empty()
        && provider_errors
            .iter()
            .all(|row| provider_error_is_pure_query_mismatch(row));
    let challenge_like_failure = search_failure_is_challenge_like(&out, provider_errors.as_slice());
    let search_provider_failure_mode = search_provider_failure_mode(
        out.get("ok").and_then(Value::as_bool).unwrap_or(false),
        query_mismatch_only_failure,
        challenge_like_failure,
        &tool_surface_status,
        tool_surface_ready,
        attempted.len(),
    );
    let cache_skip_reason_runtime =
        search_cache_skip_reason_from_failure_mode(search_provider_failure_mode);
    let cache_store_allowed = cache_skip_reason_runtime.is_empty();
    let cache_write_attempted = cache_ttl_seconds > 0 && cache_store_allowed;
    if let Some(obj) = out.as_object_mut() {
        obj.insert(
            "type".to_string(),
            Value::String("web_conduit_search".to_string()),
        );
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
        obj.insert("query_source_lineage".to_string(), query_source_lineage);
        obj.insert(
            "effective_query".to_string(),
            Value::String(scoped_query.clone()),
        );
        obj.insert(
            "allowed_domains".to_string(),
            json!(allowed_domains.clone()),
        );
        obj.insert("exclude_subdomains".to_string(), json!(exclude_subdomains));
        obj.insert("top_k".to_string(), json!(top_k));
        obj.insert("count".to_string(), json!(top_k));
        obj.insert("timeout_ms".to_string(), json!(timeout_ms));
        obj.insert("cache_ttl_minutes".to_string(), json!(cache_ttl_minutes));
        obj.insert("filters".to_string(), normalized_filters.clone());
        obj.insert("provider_chain".to_string(), json!(provider_chain.clone()));
        obj.insert("providers_attempted".to_string(), json!(attempted));
        obj.insert("providers_skipped".to_string(), json!(skipped));
        obj.insert("provider_errors".to_string(), json!(provider_errors));
        obj.insert(
            "provider_resolution".to_string(),
            provider_resolution.clone(),
        );
        obj.insert("tool_execution_gate".to_string(), tool_execution_gate.clone());
        obj.insert("replay_policy".to_string(), replay_policy.clone());
        obj.insert("replay_bypass".to_string(), replay_bypass.clone());
        obj.insert(
            "attempt_signature".to_string(),
            Value::String(search_attempt_signature.clone()),
        );
        obj.insert(
            "attempt_replay_guard".to_string(),
            attempt_replay_guard.clone(),
        );
        obj.insert(
            "provider_health".to_string(),
            provider_health_snapshot(root, &provider_chain),
        );
        obj.insert(
            "tool_surface_status".to_string(),
            Value::String(tool_surface_status.clone()),
        );
        obj.insert("tool_surface_ready".to_string(), json!(tool_surface_ready));
        obj.insert(
            "tool_surface_blocking_reason".to_string(),
            Value::String(tool_surface_blocking_reason.clone()),
        );
        obj.insert(
            "search_lite_fallback".to_string(),
            json!(used_lite_fallback),
        );
        obj.insert(
            "search_lite_fallback_reason".to_string(),
            json!(search_lite_fallback_reason),
        );
        obj.insert(
            "search_lite_fallback_triggered_by_challenge".to_string(),
            json!(search_lite_fallback_triggered_by_challenge),
        );
        obj.insert(
            "search_lite_fallback_triggered_by_low_signal".to_string(),
            json!(search_lite_fallback_triggered_by_low_signal),
        );
        obj.insert(
            "search_lite_fallback_trigger_provider".to_string(),
            json!(search_lite_fallback_trigger_provider),
        );
        obj.insert(
            "search_bing_fallback".to_string(),
            json!(used_bing_fallback),
        );
        obj.insert(
            "search_bing_fallback_reason".to_string(),
            json!(search_bing_fallback_reason),
        );
        obj.insert(
            "search_bing_fallback_triggered_by_challenge".to_string(),
            json!(search_bing_fallback_triggered_by_challenge),
        );
        obj.insert(
            "search_bing_fallback_triggered_by_low_signal".to_string(),
            json!(search_bing_fallback_triggered_by_low_signal),
        );
        obj.insert(
            "search_bing_fallback_trigger_provider".to_string(),
            json!(search_bing_fallback_trigger_provider),
        );
        obj.insert(
            "search_provider_failure_mode".to_string(),
            json!(search_provider_failure_mode),
        );
        obj.insert("provider_hint".to_string(), Value::String(provider_hint));
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
            "process_summary".to_string(),
            runtime_web_process_summary(
                "web_search",
                "provider_chain_result",
                tool_execution_attempted,
                &tool_execution_gate,
                &attempt_replay_guard,
                &json!(provider_chain.clone()),
                &final_selected_provider,
                final_error_code.as_deref()
            ),
        );
        obj.insert(
            "cache_store_allowed".to_string(),
            json!(cache_store_allowed),
        );
        obj.insert(
            "cache_write_attempted".to_string(),
            json!(cache_write_attempted),
        );
        if !cache_skip_reason_runtime.is_empty() {
            obj.insert(
                "cache_skip_reason".to_string(),
                json!(cache_skip_reason_runtime),
            );
        }
        obj.insert("cache_status".to_string(), json!("miss"));
        let summary_raw = clean_text(
            obj.get("summary").and_then(Value::as_str).unwrap_or(""),
            1_400,
        );
        let content_raw = clean_text(
            obj.get("content").and_then(Value::as_str).unwrap_or(""),
            120_000,
        );
        let summary_wrapped = if summary_raw.is_empty() {
            String::new()
        } else {
            wrap_external_untrusted_content(&summary_raw, false, "Web Search")
        };
        let content_wrapped = if content_raw.is_empty() {
            String::new()
        } else {
            wrap_external_untrusted_content(&content_raw, true, "Web Search")
        };
        obj.insert(
            "summary_wrapped".to_string(),
            Value::String(summary_wrapped),
        );
        obj.insert(
            "content_wrapped".to_string(),
            Value::String(content_wrapped),
        );
        obj.insert(
            "external_content".to_string(),
            json!({
                "untrusted": true,
                "source": "web_search",
                "wrapped": true,
                "provider_chain": provider_chain.clone(),
                "tool_surface_status": tool_surface_status.clone(),
                "query_alignment_checked": true,
                "provider": if final_selected_provider.is_empty() {
                    "none"
                } else {
                    final_selected_provider.as_str()
                }
            }),
        );
    }
    let cache_status = if out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        "ok"
    } else if challenge_like_failure {
        "challenge"
    } else {
        "no_results"
    };
    let search_url = match final_selected_provider.as_str() {
        "duckduckgo_lite" => lite_url.clone(),
        "google_news_rss" => web_search_google_news_rss_url(&scoped_query),
        "bing_rss" => web_search_bing_rss_url(&scoped_query),
        _ => primary_url.clone(),
    };
    let response_hash = out
        .get("content")
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty())
        .map(sha256_hex);
    let error = out
        .get("error")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty());
    let receipt = build_receipt(
        &search_url,
        if out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            "allow"
        } else {
            "deny"
        },
        response_hash.as_deref(),
        out.get("status_code").and_then(Value::as_i64).unwrap_or(0),
        "search_provider_chain",
        error,
    );
    let mut receipt = receipt;
    if let Some(receipt_obj) = receipt.as_object_mut() {
        receipt_obj.insert(
            "attempt_signature".to_string(),
            Value::String(search_attempt_signature),
        );
        receipt_obj.insert(
            "provider".to_string(),
            Value::String(final_selected_provider.clone()),
        );
        receipt_obj.insert(
            "search_lite_fallback".to_string(),
            json!(used_lite_fallback),
        );
        receipt_obj.insert(
            "search_lite_fallback_reason".to_string(),
            json!(search_lite_fallback_reason),
        );
        receipt_obj.insert(
            "search_lite_fallback_triggered_by_challenge".to_string(),
            json!(search_lite_fallback_triggered_by_challenge),
        );
        receipt_obj.insert(
            "search_lite_fallback_triggered_by_low_signal".to_string(),
            json!(search_lite_fallback_triggered_by_low_signal),
        );
        receipt_obj.insert(
            "search_lite_fallback_trigger_provider".to_string(),
            json!(search_lite_fallback_trigger_provider),
        );
        receipt_obj.insert(
            "search_bing_fallback".to_string(),
            json!(used_bing_fallback),
        );
        receipt_obj.insert(
            "search_bing_fallback_reason".to_string(),
            json!(search_bing_fallback_reason),
        );
        receipt_obj.insert(
            "search_bing_fallback_triggered_by_challenge".to_string(),
            json!(search_bing_fallback_triggered_by_challenge),
        );
        receipt_obj.insert(
            "search_bing_fallback_triggered_by_low_signal".to_string(),
            json!(search_bing_fallback_triggered_by_low_signal),
        );
        receipt_obj.insert(
            "search_bing_fallback_trigger_provider".to_string(),
            json!(search_bing_fallback_trigger_provider),
        );
        receipt_obj.insert(
            "search_provider_failure_mode".to_string(),
            json!(search_provider_failure_mode),
        );
    }
    let _ = append_jsonl(&receipts_path(root), &receipt);
    if let Some(obj) = out.as_object_mut() {
        obj.insert("receipt".to_string(), receipt);
    }
    if cache_write_attempted {
        store_search_cache(
            root,
            &cache_key,
            &out,
            cache_status,
            Some(cache_ttl_seconds),
        );
    }
    out
}
