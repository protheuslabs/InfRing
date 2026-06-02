    if attempt_replay_blocked {
        let search_url = match selected_provider.as_str() {
            "duckduckgo_lite" => lite_url.clone(),
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
            skipped.push(json!({
                "provider": provider,
                "reason": "circuit_open",
                "open_until": open_until
            }));
            if !allow_fallback {
                last_payload = Some(json!({
                    "ok": false,
                    "error": "provider_circuit_open",
                    "summary": format!(
                        "Search provider \"{provider}\" is temporarily unavailable because its circuit breaker is open."
                    ),
                    "content": "",
                    "provider": provider,
                    "provider_unavailable_reason": "circuit_open",
                    "circuit_open_until": open_until
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
                &normalized_filters,
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
