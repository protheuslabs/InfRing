// Layer ownership: core/layer0/ops::batch-query-api (authoritative)
fn summary_strip_heading_fragments(raw: &str) -> String {
    let normalized = claim_hint_normalized_snippet(raw);
    let mut segments = Vec::<String>::new();
    for segment in normalized.split(|ch| matches!(ch, '.' | ';' | '\n' | '\r')) {
        let cleaned = clean_text(segment, 520);
        if cleaned.is_empty() {
            continue;
        }
        let word_count = cleaned.split_whitespace().count();
        let alpha_count = cleaned.chars().filter(|ch| ch.is_ascii_alphabetic()).count();
        let visible_count = cleaned.chars().filter(|ch| !ch.is_whitespace()).count().max(1);
        if word_count < 6
            || alpha_count < 18
            || alpha_count * 2 < visible_count
            || !claim_text_is_synthesis_safe(&cleaned)
        {
            continue;
        }
        segments.push(cleaned);
    }
    if segments.is_empty() {
        let fallback = clean_text(&normalized, 1_200);
        if claim_text_is_synthesis_safe(&fallback) {
            fallback
        } else {
            String::new()
        }
    } else {
        clean_text(&segments.join(". "), 1_200)
    }
}

fn candidate_handoff_summary_fragment(
    query: &str,
    candidate: &Candidate,
    benchmark_intent: bool,
) -> String {
    let raw_snippet = if benchmark_intent {
        extract_metric_focused_fragment(&candidate.snippet)
    } else {
        normalize_snippet_text(&candidate.snippet, query, &candidate.locator)
    };
    let snippet = summary_strip_heading_fragments(&raw_snippet);
    let title = clean_text(&candidate_title_for_relevance(candidate), 240);
    let title_is_specific = !title.is_empty();
    let snippet_lowered = snippet.to_ascii_lowercase();
    let boilerplate_hits = [
        "contact preferences",
        "cookie preferences",
        "keep in touch",
        "log in below",
        "manage your contact",
        "privacy preferences",
        "sign in to your account",
        "subscribe to",
    ]
    .iter()
    .filter(|marker| snippet_lowered.contains(**marker))
    .count();
    let snippet_is_weak = snippet.is_empty()
        || looks_like_low_signal_search_summary(&snippet)
        || looks_like_source_only_snippet(&snippet)
        || looks_like_snippet_boilerplate_segment(&snippet, &candidate.locator)
        || boilerplate_hits >= 1;
    if title_is_specific
        && (snippet_is_weak || official_lane_direct_subject_source_signal(query, candidate))
    {
        return title;
    }
    snippet
}

fn summary_insights_from_evidence_claims(evidence_claims: &Value, limit: usize) -> Vec<String> {
    let mut insights = Vec::<String>::new();
    let mut seen = HashSet::<String>::new();
    for row in evidence_claims
        .as_array()
        .into_iter()
        .flat_map(|rows| rows.iter())
    {
        let claim = clean_text(row.get("claim").and_then(Value::as_str).unwrap_or(""), 260);
        if !claim_text_is_synthesis_safe(&claim) {
            continue;
        }
        let domain = clean_text(
            row.get("source_domain").and_then(Value::as_str).unwrap_or(""),
            120,
        );
        let insight = if domain.is_empty() {
            claim
        } else {
            format!("{domain}: {claim}")
        };
        let key = insight.to_ascii_lowercase();
        if !seen.insert(key) {
            continue;
        }
        insights.push(insight);
        if insights.len() >= limit.max(1) {
            break;
        }
    }
    insights
}

pub fn api_batch_query(root: &Path, request: &Value) -> Value {
    let started = Instant::now();
    let policy = load_policy(root);
    let parallel_window = max_parallel_subqueries(&policy).max(1);
    let query_timeout = query_timeout(&policy);
    let source = normalize_source(
        request
            .get("source")
            .and_then(Value::as_str)
            .unwrap_or("web"),
    );
    let aperture = normalize_aperture(
        request
            .get("aperture")
            .and_then(Value::as_str)
            .unwrap_or("medium"),
    );
    let query = request_query_text(request, 600);
    if source == "web" && looks_like_internal_route_query(&query) {
        return json!({
            "ok": true,
            "status": "no_results",
            "summary": INTERNAL_ROUTE_HINT,
            "evidence_refs": [],
            "receipt_id": "",
            "error": "internal_route_query_requires_local_diagnostics"
        });
    }
    if !enabled_sources(&policy).iter().any(|row| row == &source) {
        return json!({"ok": false, "status": "blocked", "summary": format!("Source `{source}` is not allowed by policy."), "evidence_refs": [], "receipt_id": "", "error": "source_blocked"});
    }
    if aperture == "large" && !allow_large(&policy) {
        return json!({"ok": false, "status": "blocked", "summary": "Aperture `large` is blocked by policy.", "evidence_refs": [], "receipt_id": "", "error": "aperture_blocked"});
    }
    let budget = match aperture_budget(&aperture) {
        Some(value) => value,
        None => {
            return json!({"ok": false, "status": "blocked", "summary": "Unsupported aperture.", "evidence_refs": [], "receipt_id": "", "error": "aperture_unsupported"})
        }
    };
    if query.is_empty() {
        return json!({"ok": false, "status": "blocked", "summary": "Query is required.", "evidence_refs": [], "receipt_id": "", "error": "query_required"});
    }
    if source == "web" && is_local_subject_comparison_query(&query) {
        return json!({
            "ok": true,
            "status": "no_results",
            "summary": local_subject_comparison_summary(&query),
            "evidence_refs": [],
            "receipt_id": "",
            "error": "local_subject_requires_workspace_analysis"
        });
    }
    let nexus_connection =
        match crate::dashboard_tool_turn_loop::authorize_ingress_tool_call_with_nexus("batch_query")
        {
            Ok(meta) => meta,
            Err(err) => {
                return json!({
                    "ok": false,
                    "type": "batch_query",
                    "status": "blocked",
                    "source": source,
                    "query": query,
                    "aperture": aperture,
                    "summary": "Batch query blocked by hierarchical nexus ingress policy.",
                    "evidence_refs": [],
                    "receipt_id": "",
                    "error": "batch_query_nexus_delivery_denied",
                    "nexus_error": clean_text(&err, 240)
                })
            }
        };
    let query_plan = resolve_query_plan(&policy, request, &query, budget);
    let search_scope = batch_query_search_scope(request);
    let search_scope_value = search_scope.to_value();
    let cache_control = batch_query_cache_control(&policy, request);
    let cache_key_primary = cache_key_with_query_plan_and_scope(
        &source,
        &query,
        &aperture,
        &policy,
        &query_plan.queries,
        &search_scope,
    );
    let legacy_cache_key = cache_key(&source, &query, &aperture, &policy);
    let (cached_response, cache_lookup_key) =
        if let Some(cached) = load_cached_response(root, &cache_key_primary, &cache_control) {
            (Some(cached), cache_key_primary.clone())
        } else if search_scope.is_empty() && cache_key_primary != legacy_cache_key {
            if let Some(cached) = load_cached_response(root, &legacy_cache_key, &cache_control) {
                (Some(cached), legacy_cache_key.clone())
            } else {
                (None, cache_key_primary.clone())
            }
        } else {
            (None, cache_key_primary.clone())
        };
    if let Some(cached) = cached_response {
        let cached_status = clean_text(
            cached
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("no_results"),
            32,
        );
        let query_plan_value = cached
            .get("query_plan")
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter()
                    .filter_map(Value::as_str)
                    .map(|row| clean_text(row, 600))
                    .filter(|row| !row.is_empty())
                    .collect::<Vec<_>>()
            })
            .filter(|rows| !rows.is_empty())
            .unwrap_or_else(|| query_plan.queries.clone());
        let query_plan_source = clean_text(
            cached
                .get("query_plan_source")
                .and_then(Value::as_str)
                .unwrap_or(query_plan.query_plan_source),
            64,
        );
        let evidence_refs = cached
            .get("evidence_refs")
            .and_then(Value::as_array)
            .cloned()
            .map(Value::Array)
            .unwrap_or_else(|| json!([]));
        let search_results = cached
            .get("search_results")
            .and_then(Value::as_array)
            .cloned()
            .map(Value::Array)
            .unwrap_or_else(|| json!([]));
        let evidence_pack = cached
            .get("evidence_pack")
            .and_then(Value::as_array)
            .cloned()
            .map(Value::Array)
            .unwrap_or_else(|| json!([]));
        let evidence_coverage = cached
            .get("evidence_coverage")
            .and_then(Value::as_array)
            .cloned()
            .map(Value::Array)
            .unwrap_or_else(|| json!([]));
        let retrieval_telemetry = cached
            .get("retrieval_telemetry")
            .and_then(Value::as_array)
            .cloned()
            .map(Value::Array)
            .unwrap_or_else(|| json!([]));
        let (provider_results_rows, provider_result_dedup_count) = dedup_provider_results(
            cached
                .get("provider_results")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default(),
        );
        let provider_results = Value::Array(provider_results_rows);
        let rewrite_set = cached
            .get("rewrite_set")
            .and_then(Value::as_array)
            .cloned()
            .map(Value::Array)
            .unwrap_or_else(|| json!([]));
        let partial_failure_details = cached
            .get("partial_failure_details")
            .and_then(Value::as_array)
            .cloned()
            .map(Value::Array)
            .unwrap_or_else(|| json!([]));
        let raw_summary = clean_text(
            cached
                .get("summary")
                .and_then(Value::as_str)
                .unwrap_or(crate::tool_output_match_filter::no_findings_user_copy()),
            budget.max_summary_tokens.max(60),
        );
        let cached_summary_requires_refresh = source == "web"
            && cached_status == "ok"
            && cached_framework_summary_requires_refresh(&query, &raw_summary, &evidence_refs);
        if cached_summary_requires_refresh {
            // Recompute instead of replaying stale forum-led framework summaries.
        } else {
            let status =
                if cached_status == "ok" && looks_like_low_signal_search_summary(&raw_summary) {
                    "no_results".to_string()
                } else {
                    cached_status
                };
            let summary = rewrite_cached_batch_query_summary(
                &query,
                &source,
                &raw_summary,
                &evidence_refs,
                &partial_failure_details,
            );
            let cached_quality = cached.get("tool_result_quality").cloned();
            let cached_quality_is_current = cached_quality
                .as_ref()
                .and_then(|quality| quality.get("version"))
                .and_then(Value::as_str)
                == Some(web_tool_quality_version());
            let tool_result_quality = if cached_quality_is_current {
                cached_quality.unwrap_or_else(|| json!({}))
            } else {
                cached_web_tool_quality_report(
                    &query,
                    &status,
                    &partial_failure_details,
                    &evidence_refs,
                )
            };
            let source_class_coverage = cached
                .get("source_class_coverage")
                .cloned()
                .unwrap_or_else(|| {
                    source_class_coverage_from_evidence_pack(
                        &policy,
                        &query,
                        &evidence_pack,
                        &evidence_coverage,
                    )
                });
            let evidence_pack_quality = cached
                .get("evidence_pack_quality")
                .cloned()
                .unwrap_or_else(|| {
                    evidence_pack_quality_report(&policy, &evidence_pack, &evidence_coverage)
                });
            let evidence_claims = cached
                .get("evidence_claims")
                .and_then(Value::as_array)
                .cloned()
                .map(Value::Array)
                .unwrap_or_else(|| {
                    evidence_claims_from_pack(
                        &query_plan.query_metadata,
                        &evidence_pack,
                        budget.max_evidence * 2,
                    )
                });
            let query_lane_attribution = cached
                .get("query_lane_attribution")
                .cloned()
                .unwrap_or_else(|| {
                    json!({
                        "version": "query_lane_attribution_v1",
                        "status": "not_available_from_cache",
                        "lane_count": 0,
                        "rows": [],
                        "diagnostic_use": "telemetry_only"
                    })
                });
            let second_pass_recovery =
                cached
                    .get("second_pass_recovery")
                    .cloned()
                    .unwrap_or_else(|| {
                        json!({
                            "enabled": second_pass_recovery_enabled(&policy),
                            "used": false,
                            "reason": "none",
                            "queries": []
                        })
                    });
            let retrieval_broker = cached.get("retrieval_broker").cloned().unwrap_or_else(|| {
                retrieval_broker_report(
                    &status,
                    json!(query_plan_value.clone()),
                    cached
                        .get("query_plan")
                        .cloned()
                        .unwrap_or_else(|| json!(query_plan_value.clone())),
                    &query_plan_source,
                    second_pass_recovery.clone(),
                    &retrieval_telemetry,
                    &provider_results,
                    &evidence_pack,
                    &evidence_coverage,
                    &tool_result_quality,
                    &source_class_coverage,
                    &evidence_pack_quality,
                )
            });
            let mut retrieval_broker = retrieval_broker;
            if let Some(obj) = retrieval_broker.as_object_mut() {
                obj.insert(
                    "query_lane_attribution".to_string(),
                    query_lane_attribution.clone(),
                );
            }
            let parallel_retrieval_used = cached
                .get("parallel_retrieval_used")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let query_metadata = cached
                .get("query_metadata")
                .cloned()
                .unwrap_or_else(|| query_plan.query_metadata.to_value());
            let provider_snapshot = json!({
                "id": crate::deterministic_receipt_hash(&json!({"source": source, "query": query, "cache_key": cache_lookup_key, "search_scope": search_scope_value.clone()})),
                "source": source,
                "adapter_version": "web_conduit_v1",
                "disposable": true
            });
            let receipt = json!({
                "type": "batch_query_receipt",
                "ts": crate::now_iso(),
                "source": source,
                "query": query,
                "aperture": aperture,
                "query_timeout_ms": query_timeout.as_millis() as u64,
                "parallel_window": parallel_window,
                "rewrite_set": rewrite_set,
                "query_plan": query_plan_value,
                "query_plan_source": query_plan_source,
                "query_metadata": query_metadata.clone(),
                "query_contract": {
                    "authority": "agent_submitted",
                    "query_used": query,
                    "hidden_query_expansion": false,
                    "query_plan_source": query_plan_source,
                    "query_metadata": query_metadata.clone(),
                    "search_scope": search_scope_value.clone()
                },
                "adapter_version": "web_conduit_v1",
                "provider_snapshot": provider_snapshot,
                "snapshot_id": provider_snapshot.get("id").cloned().unwrap_or(Value::Null),
                "candidate_count": 0,
                "dedup_count": 0,
                "provider_result_count": provider_results.as_array().map(|rows| rows.len()).unwrap_or(0),
                "provider_result_dedup_count": provider_result_dedup_count,
                "evidence_count": evidence_refs.as_array().map(|rows| rows.len()).unwrap_or(0),
                "evidence_pack_count": evidence_pack.as_array().map(|rows| rows.len()).unwrap_or(0),
                "evidence_claim_count": evidence_claims.as_array().map(|rows| rows.len()).unwrap_or(0),
                "cache_status": "hit",
                "cache_mode": cache_control.mode.as_str(),
                "latency_ms": started.elapsed().as_millis() as u64,
                "token_usage": {"summary_tokens_estimate": summary.split_whitespace().count()},
                "parallel_retrieval_used": parallel_retrieval_used,
                "retrieval_telemetry": retrieval_telemetry.clone(),
                "evidence_coverage": evidence_coverage.clone(),
                "source_class_coverage": source_class_coverage.clone(),
                "evidence_pack_quality": evidence_pack_quality.clone(),
                "query_lane_attribution": query_lane_attribution.clone(),
                "retrieval_broker": retrieval_broker.clone(),
                "partial_failure_details": [],
                "tool_result_quality": tool_result_quality.clone(),
                "evidence_claims": evidence_claims.clone(),
                "status": status
            });
            let receipt_id = crate::deterministic_receipt_hash(&receipt);
            let mut receipt_with_id = receipt.clone();
            receipt_with_id["receipt_id"] = Value::String(receipt_id.clone());
            let _ = append_jsonl(&receipts_path(root), &receipt_with_id);
            let mut out = json!({
                "ok": status != "blocked",
                "type": "batch_query",
                "status": status,
                "source": source,
                "query": query,
                "aperture": aperture,
                "summary": summary,
                "evidence_refs": evidence_refs,
                "receipt_id": receipt_id,
                "parallel_retrieval_used": parallel_retrieval_used,
                "query_timeout_ms": query_timeout.as_millis() as u64,
                "parallel_window": parallel_window,
                "rewrite_set": rewrite_set,
                "query_plan": cached
                    .get("query_plan")
                    .cloned()
                    .unwrap_or_else(|| json!(query_plan.queries)),
                "query_plan_source": cached
                    .get("query_plan_source")
                    .cloned()
                    .unwrap_or_else(|| json!(query_plan.query_plan_source)),
                "query_metadata": query_metadata.clone(),
                "query_contract": {
                    "authority": "agent_submitted",
                    "query_used": query,
                    "hidden_query_expansion": false,
                    "query_plan_source": query_plan_source,
                    "query_metadata": query_metadata.clone(),
                    "search_scope": search_scope_value.clone()
                },
                "partial_failure_details": partial_failure_details,
                "retrieval_telemetry": retrieval_telemetry.clone(),
                "evidence_claims": evidence_claims,
                "tool_result_quality": tool_result_quality,
                "source_class_coverage": source_class_coverage.clone(),
                "evidence_pack_quality": evidence_pack_quality.clone(),
                "query_lane_attribution": query_lane_attribution.clone(),
                "retrieval_broker": retrieval_broker.clone(),
                "provider_result_count": provider_results.as_array().map(|rows| rows.len()).unwrap_or(0),
                "provider_result_dedup_count": provider_result_dedup_count,
                "cache_status": "hit",
                "cache_mode": cache_control.mode.as_str()
            });
            if search_results
                .as_array()
                .map(|rows| !rows.is_empty())
                .unwrap_or(false)
            {
                out["search_results"] = search_results;
            }
            if evidence_pack
                .as_array()
                .map(|rows| !rows.is_empty())
                .unwrap_or(false)
            {
                out["evidence_pack"] = evidence_pack;
            }
            if evidence_coverage
                .as_array()
                .map(|rows| !rows.is_empty())
                .unwrap_or(false)
            {
                out["evidence_coverage"] = evidence_coverage;
            }
            out["source_class_coverage"] = source_class_coverage;
            out["evidence_pack_quality"] = evidence_pack_quality;
            out["query_lane_attribution"] = query_lane_attribution;
            out["retrieval_broker"] = retrieval_broker;
            if provider_results
                .as_array()
                .map(|rows| !rows.is_empty())
                .unwrap_or(false)
            {
                out["provider_results"] = provider_results;
            }
            if retrieval_telemetry
                .as_array()
                .map(|rows| !rows.is_empty())
                .unwrap_or(false)
            {
                out["retrieval_telemetry"] = retrieval_telemetry;
            }
            if let Some(code) = no_results_error_code(&summary, &partial_failure_details) {
                out["error"] = Value::String(code.to_string());
            }
            if let Some(meta) = nexus_connection {
                out["nexus_connection"] = meta;
            }
            return out;
        }
    }

    let submitted_queries = query_plan.queries.clone();
    let queries = execution_limited_initial_queries(&policy, budget, &submitted_queries);
    let deferred_queries = submitted_queries
        .iter()
        .skip(queries.len())
        .cloned()
        .collect::<Vec<String>>();
    let query_execution_limited = !deferred_queries.is_empty();
    let query_execution_limiter = json!({
        "applied": query_execution_limited,
        "submitted_lane_count": submitted_queries.len(),
        "executed_initial_lane_count": queries.len(),
        "deferred_lane_count": deferred_queries.len(),
        "deferred_query_plan": deferred_queries,
        "max_initial_lanes": initial_query_execution_max_lanes(&policy, budget),
        "reason": if query_execution_limited {
            "bounded_initial_wave_to_preserve_provider_rate_budget"
        } else {
            "within_budget"
        }
    });
    let rewrite_set = query_plan.rewrite_set.clone();
    let parallel_allowed = source == "web" && query_plan.rewrite_applied && queries.len() > 1;
    let mut executed_queries = queries.clone();
    let mut second_pass_queries = Vec::<String>::new();
    let mut retrieval_telemetry = Vec::<Value>::new();
    let mut query_lane_sources = Vec::<QueryLaneSource>::new();
    let mut candidates = Vec::<Candidate>::new();
    let mut partial_failures = Vec::<String>::new();
    let mut provider_results = Vec::<Value>::new();
    let page_fetch_budget = PageExtractionFetchBudget::new(&policy);
    if parallel_allowed {
        let limit = parallel_window;
        let mut offset = 0usize;
        while offset < queries.len() {
            let end = (offset + limit).min(queries.len());
            let expected = end.saturating_sub(offset);
            let (tx, rx) = std::sync::mpsc::channel::<(
                usize,
                String,
                (Vec<Candidate>, Vec<String>, Vec<Value>),
            )>();
            let mut chunk_rows = std::iter::repeat_with(|| None)
                .take(expected)
                .collect::<Vec<Option<(String, (Vec<Candidate>, Vec<String>, Vec<Value>))>>>();
            for (local_idx, q) in queries[offset..end].iter().enumerate() {
                let tx_clone = tx.clone();
                let query_item = q.clone();
                let root_buf = root.to_path_buf();
                let policy_buf = policy.clone();
                let search_scope_buf = search_scope.clone();
                let fetch_budget_buf = page_fetch_budget.clone();
                let spawned = thread::Builder::new()
                    .name(format!("batch-query-{local_idx}"))
                    .spawn(move || {
                        let out = retrieve_web_candidates_for_query(
                            &root_buf,
                            &query_item,
                            &policy_buf,
                            &search_scope_buf,
                            fetch_budget_buf,
                        );
                        let _ = tx_clone.send((local_idx, query_item, out));
                    });
                if spawned.is_err() {
                    chunk_rows[local_idx] = Some((
                        q.clone(),
                        (
                            Vec::new(),
                            vec!["query_worker_spawn_failed".to_string()],
                            Vec::new(),
                        ),
                    ));
                }
            }
            drop(tx);
            let mut received = chunk_rows.iter().filter(|row| row.is_some()).count();
            while received < expected {
                match rx.recv_timeout(query_timeout) {
                    Ok((local_idx, q, out)) => {
                        if local_idx < expected && chunk_rows[local_idx].is_none() {
                            chunk_rows[local_idx] = Some((q, out));
                            received += 1;
                        }
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => break,
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
            for local_idx in 0..expected {
                let fallback_query = clean_text(&queries[offset + local_idx], 120);
                match chunk_rows[local_idx].take() {
                    Some((q, (mut rows, issues, artifacts))) => {
                        query_lane_sources
                            .push(query_lane_source(&q, "initial", &rows, &issues, &artifacts));
                        if retrieval_telemetry_enabled(&policy) {
                            retrieval_telemetry.push(retrieval_telemetry_row(
                                &q, "initial", &rows, &issues, &artifacts,
                            ));
                        }
                        provider_results.extend(artifacts);
                        let transport_only_issue = rows.is_empty()
                            && issues.iter().all(|issue| {
                                issue.starts_with("query_timeout_ms_")
                                    || issue == "query_worker_spawn_failed"
                                    || issue == "query_worker_disconnected"
                            });
                        if rows.is_empty() && issues.is_empty() {
                            partial_failures
                                .push(format!("{}:no_usable_summary", clean_text(&q, 120)));
                        } else {
                            if rows.is_empty() && !transport_only_issue {
                                partial_failures
                                    .push(format!("{}:no_usable_summary", clean_text(&q, 120)));
                            } else {
                                candidates.append(&mut rows);
                            }
                            partial_failures.extend(
                                issues
                                    .into_iter()
                                    .map(|issue| format!("{}:{issue}", clean_text(&q, 120))),
                            );
                        }
                    }
                    None => {
                        let timeout_issue =
                            format!("query_timeout_ms_{}", query_timeout.as_millis());
                        query_lane_sources.push(query_lane_source(
                            &fallback_query,
                            "initial",
                            &[],
                            &[timeout_issue.clone()],
                            &[],
                        ));
                        partial_failures.push(format!("{}:{timeout_issue}", fallback_query));
                    }
                }
            }
            offset = end;
        }
    } else {
        for q in &queries {
            let (mut rows, issues, artifacts) = retrieve_web_candidates_for_query_with_timeout(
                root,
                q,
                &policy,
                &search_scope,
                page_fetch_budget.clone(),
                query_timeout,
            );
            query_lane_sources.push(query_lane_source(q, "initial", &rows, &issues, &artifacts));
            if retrieval_telemetry_enabled(&policy) {
                retrieval_telemetry.push(retrieval_telemetry_row(
                    q, "initial", &rows, &issues, &artifacts,
                ));
            }
            provider_results.extend(artifacts);
            let transport_only_issue = rows.is_empty()
                && issues.iter().all(|issue| {
                    issue.starts_with("query_timeout_ms_")
                        || issue == "query_worker_spawn_failed"
                        || issue == "query_worker_disconnected"
                });
            if rows.is_empty() && issues.is_empty() {
                partial_failures.push(format!("{}:no_usable_summary", clean_text(q, 120)));
            } else {
                if rows.is_empty() && !transport_only_issue {
                    partial_failures.push(format!("{}:no_usable_summary", clean_text(q, 120)));
                } else {
                    candidates.append(&mut rows);
                }
                partial_failures.extend(
                    issues
                        .into_iter()
                        .map(|issue| format!("{}:{issue}", clean_text(q, 120))),
                );
            }
        }
    }

    let mut second_pass_reason = "none";
    let recovery_basis_query = clean_text(&query_plan.rerank_query, 600);
    let recovery_basis_query = if recovery_basis_query.is_empty() {
        query.clone()
    } else {
        recovery_basis_query
    };
    let first_pass_lacked_usable = !has_pack_ready_synthesis_candidate(
        &recovery_basis_query,
        &candidates,
    );
    let first_pass_lacked_source_quality =
        !has_pack_ready_synthesis_source_quality(&recovery_basis_query, &candidates);
    let first_pass_research_facets = infer_research_facets(
        &recovery_basis_query,
        &executed_queries,
        &query_plan.query_metadata,
        &policy,
        budget,
    );
    let query_pack_declares_coverage =
        !query_plan.query_metadata.entities.is_empty() || !query_plan.query_metadata.facets.is_empty();
    let mut planned_second_pass_queries = Vec::<String>::new();
    if source == "web" && query_execution_limited {
        second_pass_reason = "deferred_due_initial_query_execution_budget";
        if second_pass_recovery_enabled(&policy) && first_pass_lacked_usable {
            planned_second_pass_queries = deferred_query_recovery_queries(
                &policy,
                budget,
                &submitted_queries,
                &executed_queries,
            );
            if !planned_second_pass_queries.is_empty() {
                second_pass_reason = "deferred_query_execution_recovery";
            }
        }
        if planned_second_pass_queries.is_empty() && coverage_gap_recovery_enabled(&policy) {
            planned_second_pass_queries = coverage_gap_recovery_queries(
                &policy,
                &recovery_basis_query,
                &executed_queries,
                &first_pass_research_facets,
                &candidates,
                budget,
            );
            if !planned_second_pass_queries.is_empty() {
                second_pass_reason = "coverage_gap";
            }
        }
        if planned_second_pass_queries.is_empty()
            && second_pass_recovery_enabled(&policy)
            && broad_current_research_lacks_synthesis_breadth(
                &policy,
                &query,
                &query_plan.query_metadata,
                &candidates,
                budget,
            )
        {
            planned_second_pass_queries = deferred_query_recovery_queries(
                &policy,
                budget,
                &submitted_queries,
                &executed_queries,
            );
            if !planned_second_pass_queries.is_empty() {
                second_pass_reason = "deferred_query_breadth_recovery";
            }
        }
        if planned_second_pass_queries.is_empty()
            && second_pass_recovery_enabled(&policy)
            && first_pass_lacked_source_quality
        {
            planned_second_pass_queries = deferred_query_recovery_queries(
                &policy,
                budget,
                &submitted_queries,
                &executed_queries,
            );
            if !planned_second_pass_queries.is_empty() {
                second_pass_reason = if query_pack_declares_coverage {
                    "deferred_query_source_quality_recovery"
                } else {
                    "deferred_query_source_diversity_recovery"
                };
            }
        }
    } else {
        if source == "web"
            && second_pass_recovery_enabled(&policy)
            && (first_pass_lacked_usable || provider_results.is_empty())
        {
            planned_second_pass_queries = second_pass_recovery_queries(
                &policy,
                &recovery_basis_query,
                &executed_queries,
                budget,
            );
            if !planned_second_pass_queries.is_empty() {
                second_pass_reason = if first_pass_lacked_usable {
                    "no_usable_synthesis_candidates"
                } else {
                    "no_raw_provider_artifacts"
                };
            }
        }
        let policy_recovery_already_spent_coverage_budget = matches!(
            query_plan.query_plan_source,
            "policy_general_research_recovery" | "policy_broad_current_research_recovery"
        ) && !query_pack_declares_coverage;
        if source == "web"
            && planned_second_pass_queries.is_empty()
            && coverage_gap_recovery_enabled(&policy)
            && !policy_recovery_already_spent_coverage_budget
        {
            planned_second_pass_queries = coverage_gap_recovery_queries(
                &policy,
                &recovery_basis_query,
                &executed_queries,
                &first_pass_research_facets,
                &candidates,
                budget,
            );
            if !planned_second_pass_queries.is_empty() {
                second_pass_reason = "coverage_gap";
            }
        }
        if source == "web"
            && planned_second_pass_queries.is_empty()
            && claim_gap_recovery_enabled(&policy)
        {
            planned_second_pass_queries = claim_gap_recovery_queries(
                &policy,
                &recovery_basis_query,
                &executed_queries,
                &first_pass_research_facets,
                &candidates,
                budget,
            );
            if !planned_second_pass_queries.is_empty() {
                second_pass_reason = "claim_gap";
            }
        }
    }
    if source == "web" && !planned_second_pass_queries.is_empty() {
        for recovery_query in planned_second_pass_queries {
            let (mut rows, issues, artifacts) = retrieve_web_candidates_for_query_with_timeout(
                root,
                &recovery_query,
                &policy,
                &search_scope,
                page_fetch_budget.clone(),
                query_timeout,
            );
            query_lane_sources.push(query_lane_source(
                &recovery_query,
                "second_pass_recovery",
                &rows,
                &issues,
                &artifacts,
            ));
            if retrieval_telemetry_enabled(&policy) {
                retrieval_telemetry.push(retrieval_telemetry_row(
                    &recovery_query,
                    "second_pass_recovery",
                    &rows,
                    &issues,
                    &artifacts,
                ));
            }
            provider_results.extend(artifacts);
            let transport_only_issue = rows.is_empty()
                && issues.iter().all(|issue| {
                    issue.starts_with("query_timeout_ms_")
                        || issue == "query_worker_spawn_failed"
                        || issue == "query_worker_disconnected"
                });
            if rows.is_empty() && issues.is_empty() {
                partial_failures.push(format!(
                    "{}:no_usable_summary",
                    clean_text(&recovery_query, 120)
                ));
            } else {
                if rows.is_empty() && !transport_only_issue {
                    partial_failures.push(format!(
                        "{}:no_usable_summary",
                        clean_text(&recovery_query, 120)
                    ));
                } else {
                    candidates.append(&mut rows);
                }
                partial_failures.extend(
                    issues
                        .into_iter()
                        .map(|issue| format!("{}:{issue}", clean_text(&recovery_query, 120))),
                );
            }
            executed_queries.push(recovery_query.clone());
            second_pass_queries.push(recovery_query);
        }
    }

    let before_dedup = candidates.len();
    let mut seen = HashSet::<String>::new();
    candidates.retain(|row| {
        let key = format!(
            "{}|{}|{}",
            row.locator.to_ascii_lowercase(),
            row.title.to_ascii_lowercase(),
            row.excerpt_hash
        );
        if seen.contains(&key) {
            false
        } else {
            seen.insert(key);
            true
        }
    });
    let rerank_query = query_plan.rerank_query.clone();
    let research_facets = infer_research_facets(
        &query,
        &executed_queries,
        &query_plan.query_metadata,
        &policy,
        budget,
    );
    let facet_min_terms = facet_aware_min_terms(&policy);
    let evidence_limit = coverage_aware_max_evidence(&research_facets, budget);
    truncate_candidates_preserving_facet_coverage(
        &rerank_query,
        &research_facets,
        &mut candidates,
        budget.max_candidates,
        facet_min_terms,
    );
    let (provider_results, provider_result_dedup_count) = dedup_provider_results(provider_results);

    let benchmark_intent = is_benchmark_or_comparison_intent(&rerank_query);
    let ranked_pool = candidates
        .iter()
        .map(|row| {
            let score = if research_facets.is_empty() {
                rerank_score(&rerank_query, row)
            } else {
                coverage_aware_score(&rerank_query, &research_facets, row, facet_min_terms)
            };
            (row.clone(), score)
        })
        .collect::<Vec<_>>();
    let retained_ranked_pool = ranked_pool
        .iter()
        .filter(|(row, score)| candidate_retention_preview_eligible(&rerank_query, row, *score))
        .cloned()
        .collect::<Vec<_>>();
    let retained_ranked = select_facet_covered_ranked_candidates(
        retained_ranked_pool,
        &research_facets,
        evidence_limit.max(low_confidence_retention_max_items(&policy, budget)),
        facet_min_terms,
    );

    let min_synthesis_score = minimum_synthesis_score(benchmark_intent);
    let actionable_ranked_pool = ranked_pool
        .iter()
        .cloned()
        .filter(|(row, score)| {
            let snippet = clean_text(&row.snippet, 1_200);
            let domain = candidate_domain_hint(row);
            !snippet.is_empty()
                && !candidate_is_low_confidence_retained(row)
                && *score >= min_synthesis_score
                && !looks_like_ack_only(&snippet)
                && !looks_like_low_signal_search_summary(&snippet)
                && !looks_like_source_only_snippet(&snippet)
                && !citation_wrapper_link(&row.locator)
                && !is_search_engine_domain(&domain)
                && candidate_passes_relevance_gate(&rerank_query, row, benchmark_intent)
                && candidate_is_substantive(&rerank_query, row, benchmark_intent)
                && !(benchmark_intent && looks_like_definition_candidate(row))
                && !(benchmark_intent && looks_like_comparison_noise_candidate(row))
        })
        .collect::<Vec<_>>();
    let mut actionable_ranked = select_pack_ready_ranked_candidates(
        &rerank_query,
        actionable_ranked_pool.clone(),
        &research_facets,
        evidence_limit,
        facet_min_terms,
    );

    let comparison_entities = if benchmark_intent {
        let metadata_entities = research_facets
            .iter()
            .filter(|facet| facet.kind == "entity")
            .map(|facet| facet.requested_text.clone())
            .collect::<Vec<_>>();
        if metadata_entities.len() >= 2 {
            metadata_entities
        } else {
            comparison_entities_from_query(&query)
        }
    } else {
        Vec::new()
    };
    let (comparison_guard_search_results, comparison_guard_summary) =
        comparison_guard_failure_artifacts(
            &query,
            &comparison_entities,
            &actionable_ranked,
            &retained_ranked,
            &provider_results,
            evidence_limit,
        );
    let comparison_coverage_gap = comparison_guard_summary.is_some();
    let preserve_partial_comparison_evidence = comparison_partial_preserves_actionable_evidence(
        &rerank_query,
        &comparison_entities,
        &actionable_ranked,
        &retained_ranked,
    );
    if let Some(summary) = comparison_guard_summary.as_ref() {
        if !actionable_ranked.is_empty() {
            partial_failures.push(format!(
                "comparison_entity_coverage_gap:{}",
                clean_text(summary, 320)
            ));
        }
        if !preserve_partial_comparison_evidence {
            actionable_ranked.clear();
        }
    }
    let allow_comparison_gap_evidence_recovery =
        !comparison_coverage_gap || preserve_partial_comparison_evidence;

    let mut evidence_ranked = actionable_ranked.clone();
    let mut low_confidence_evidence_used = false;
    if allow_comparison_gap_evidence_recovery
        && evidence_ranked.is_empty()
        && low_confidence_retention_enabled(&policy)
    {
        let mut low_confidence_ranked = retained_ranked
            .iter()
            .filter(|(row, _)| candidate_is_low_confidence_retained(row))
            .cloned()
            .collect::<Vec<_>>();
        if low_confidence_ranked.is_empty() {
            low_confidence_ranked = ranked_pool
                .iter()
                .filter(|(row, _)| candidate_is_low_confidence_retained(row))
                .cloned()
                .collect::<Vec<_>>();
        }
        low_confidence_ranked = select_facet_covered_ranked_candidates(
            low_confidence_ranked,
            &research_facets,
            low_confidence_retention_max_items(&policy, budget),
            facet_min_terms,
        );
        if !low_confidence_ranked.is_empty() {
            low_confidence_evidence_used = true;
            partial_failures.push("low_confidence_raw_evidence_used_after_recovery".to_string());
            evidence_ranked = low_confidence_ranked;
        }
    }
    let low_confidence_count_before_backfill = evidence_ranked
        .iter()
        .filter(|(row, _)| candidate_is_low_confidence_retained(row))
        .count();
    let facet_backfill_count = if allow_comparison_gap_evidence_recovery {
        backfill_missing_facet_ranked_candidates(
            &rerank_query,
            &mut evidence_ranked,
            &ranked_pool,
            &research_facets,
            evidence_limit,
            facet_min_terms,
            low_confidence_retention_enabled(&policy),
        )
    } else {
        0
    };
    if facet_backfill_count > 0 {
        partial_failures.push(format!(
            "facet_coverage_backfill_used:{facet_backfill_count}"
        ));
        let low_confidence_count_after_backfill = evidence_ranked
            .iter()
            .filter(|(row, _)| candidate_is_low_confidence_retained(row))
            .count();
        if low_confidence_count_after_backfill > low_confidence_count_before_backfill {
            partial_failures.push("low_confidence_facet_backfill_used".to_string());
        }
    }
    let trusted_primary_preserved = if allow_comparison_gap_evidence_recovery {
        preserve_trusted_primary_lane_candidates(
            &rerank_query,
            &mut evidence_ranked,
            &ranked_pool,
            &query_lane_sources,
            &research_facets,
            evidence_limit,
            facet_min_terms,
        )
    } else {
        0
    };
    if trusted_primary_preserved > 0 {
        partial_failures.push(format!(
            "trusted_primary_lane_preserved:{trusted_primary_preserved}"
        ));
    }

    let evidence_refs = evidence_ranked
        .iter()
        .filter(|(row, _)| {
            candidate_counts_as_query_usable_evidence(&rerank_query, row, rerank_score(&rerank_query, row))
                || candidate_is_low_confidence_retained(row)
        })
        .map(|(row, score)| EvidenceRef {
            source_kind: row.source_kind.clone(),
            title: row.title.clone(),
            locator: row.locator.clone(),
            excerpt_hash: row.excerpt_hash.clone(),
            score: (*score * 100.0).round() / 100.0,
            timestamp: row.timestamp.clone(),
            permissions: row.permissions.clone(),
            confidence: if candidate_is_low_confidence_retained(row) {
                "low_confidence_raw".to_string()
            } else {
                "usable".to_string()
            },
            quality_flags: if candidate_is_low_confidence_retained(row) {
                vec!["low_confidence_raw".to_string()]
            } else {
                candidate_quality_flags(&rerank_query, row, *score)
            },
            coverage_facets: candidate_coverage_facets(&research_facets, row, facet_min_terms),
        })
        .collect::<Vec<_>>();
    let evidence_pack = evidence_pack_from_ranked_candidates(
        &policy,
        &rerank_query,
        &research_facets,
        facet_min_terms,
        &evidence_ranked,
        evidence_limit,
    );
    let evidence_claims = evidence_claims_from_pack(
        &query_plan.query_metadata,
        &evidence_pack,
        evidence_limit * 2,
    );
    let evidence_coverage = evidence_coverage_from_ranked_candidates(
        &rerank_query,
        &research_facets,
        &evidence_ranked,
        facet_min_terms,
    );
    let query_lane_attribution = query_lane_attribution_report(
        &query_lane_sources,
        &evidence_ranked,
        &research_facets,
        facet_min_terms,
    );
    let evidence_selection_diagnostics = evidence_selection_diagnostics(
        &rerank_query,
        &ranked_pool,
        &actionable_ranked_pool,
        &evidence_ranked,
        evidence_limit.saturating_mul(4).max(12),
    );

    let mut hard_partial_failures = partial_failures
        .iter()
        .filter(|row| !is_benign_partial_failure(row))
        .cloned()
        .collect::<Vec<_>>();
    if !evidence_refs.is_empty() {
        hard_partial_failures.retain(|row| !issue_is_access_or_throttle_failure(row));
    }
    if ranked_evidence_covers_all_facets(
        &rerank_query,
        &research_facets,
        &evidence_ranked,
        facet_min_terms,
    ) {
        hard_partial_failures.retain(|row| !is_diagnostic_only_after_complete_coverage(row));
    }
    let status = if evidence_refs.is_empty() {
        "no_results"
    } else if low_confidence_evidence_used {
        "low_signal"
    } else if hard_partial_failures.is_empty() {
        "ok"
    } else {
        "partial"
    };
    let summary = if evidence_refs.is_empty() {
        let partial_failure_value = Value::Array(
            hard_partial_failures
                .iter()
                .cloned()
                .map(Value::String)
                .collect::<Vec<_>>(),
        );
        no_results_summary_for_batch_query(
            &query,
            &source,
            &partial_failure_value,
            comparison_guard_summary,
        )
    } else if low_confidence_evidence_used {
        if source == "web" {
            "Web retrieval ran, but only low-confidence raw snippets were available in this turn. Treat retained rows as diagnostic leads, not source-backed findings."
                .to_string()
        } else {
            crate::tool_output_match_filter::no_findings_user_copy().to_string()
        }
    } else {
        let mut synthesized_insights =
            summary_insights_from_evidence_claims(&evidence_claims, evidence_limit.max(1));
        let mut seen_domains = HashSet::<String>::new();
        if synthesized_insights.is_empty() && source != "web" {
            for (candidate, _) in &evidence_ranked {
                let snippet_raw =
                    candidate_handoff_summary_fragment(&rerank_query, candidate, benchmark_intent);
                let snippet = trim_words(&snippet_raw, if benchmark_intent { 30 } else { 42 });
                if snippet.is_empty() || !claim_text_is_synthesis_safe(&snippet) {
                    continue;
                }
                if benchmark_intent {
                    if !looks_like_metric_rich_text(&snippet_raw)
                        && looks_like_instructional_query(&snippet_raw)
                    {
                        continue;
                    }
                    let comparison_haystack = clean_text(
                        &format!("{} {} {}", candidate.title, snippet_raw, candidate.locator),
                        1_600,
                    )
                    .to_ascii_lowercase();
                    let entity_hits = comparison_entities
                        .iter()
                        .filter(|entity| comparison_haystack.contains(entity.as_str()))
                        .count();
                    let comparative_copy = comparison_haystack.contains(" vs ")
                        || comparison_haystack.contains("versus")
                        || comparison_haystack.contains("compared")
                        || comparison_haystack.contains("better")
                        || comparison_haystack.contains("worse")
                        || comparison_haystack.contains("faster")
                        || comparison_haystack.contains("slower");
                    let benchmark_quality_ok = looks_like_metric_rich_text(&snippet_raw)
                        || (comparison_entities.len() >= 2
                            && entity_hits >= 1
                            && (comparative_copy || comparison_coverage_gap));
                    if !benchmark_quality_ok {
                        continue;
                    }
                }
                let domain = candidate_domain_hint(candidate);
                let domain_key = clean_text(&domain, 160).to_ascii_lowercase();
                if domain_key != "source"
                    && !domain_key.is_empty()
                    && !seen_domains.insert(domain_key)
                {
                    continue;
                }
                let insight = if domain == "source" {
                    snippet.clone()
                } else {
                    format!("{domain}: {snippet}")
                };
                if synthesized_insights
                    .iter()
                    .any(|existing| existing.eq_ignore_ascii_case(&insight))
                {
                    continue;
                }
                synthesized_insights.push(insight);
                if synthesized_insights.len() >= evidence_limit.max(1) {
                    break;
                }
            }
        }
        if is_framework_catalog_intent(&query) {
            let fallback_insights =
                framework_catalog_fallback_insights(&evidence_ranked, evidence_limit);
            let synthesized_joined = synthesized_insights.join(" ");
            let fallback_joined = fallback_insights.join(" ");
            if framework_name_hits(&synthesized_joined) < 2
                && framework_name_hits(&fallback_joined) > framework_name_hits(&synthesized_joined)
            {
                synthesized_insights = fallback_insights.clone();
            }
            if framework_summary_contains_low_signal_sources(&synthesized_insights.join(" "))
                && !fallback_insights.is_empty()
            {
                synthesized_insights = fallback_insights;
            }
        }
        if synthesized_insights.is_empty() && source != "web" && !evidence_refs.is_empty() {
            for (candidate, _) in evidence_ranked.iter().filter(|(candidate, _)| {
                candidate_counts_as_query_usable_evidence(
                    &rerank_query,
                    candidate,
                    rerank_score(&rerank_query, candidate),
                )
                    || candidate_is_low_confidence_retained(candidate)
            }) {
                let snippet_raw =
                    candidate_handoff_summary_fragment(&rerank_query, candidate, benchmark_intent);
                let snippet = if snippet_raw.is_empty() {
                    clean_text(&candidate.title, 240)
                } else {
                    trim_words(&snippet_raw, if benchmark_intent { 36 } else { 48 })
                };
                if snippet.is_empty() || !claim_text_is_synthesis_safe(&snippet) {
                    continue;
                }
                let domain = candidate_domain_hint(candidate);
                let domain_key = clean_text(&domain, 160).to_ascii_lowercase();
                if domain_key != "source" && !domain_key.is_empty() && !seen_domains.insert(domain_key)
                {
                    continue;
                }
                let insight = if domain == "source" {
                    snippet
                } else {
                    format!("{domain}: {snippet}")
                };
                if synthesized_insights
                    .iter()
                    .any(|existing| existing.eq_ignore_ascii_case(&insight))
                {
                    continue;
                }
                synthesized_insights.push(insight);
                if synthesized_insights.len() >= evidence_limit.max(1) {
                    break;
                }
            }
        }
        if synthesized_insights.is_empty() {
            if source == "web" {
                "Web retrieval ran, but only low-signal snippets were available for synthesis in this turn. Retry with a narrower query or one specific source URL for source-backed findings."
                    .to_string()
            } else {
                crate::tool_output_match_filter::no_findings_user_copy().to_string()
            }
        } else {
            let comparison_intent = comparison_entities.len() >= 2;
            let prefix = if source == "web" {
                if low_confidence_evidence_used {
                    "Low-confidence web retrieval:"
                } else if comparison_coverage_gap {
                    "Partial source coverage:"
                } else if comparison_intent {
                    "Comparison findings:"
                } else if benchmark_intent {
                    "Web benchmark synthesis:"
                } else {
                    "From web retrieval:"
                }
            } else if comparison_intent {
                "Comparison findings:"
            } else if benchmark_intent {
                "Benchmark findings:"
            } else {
                "Key findings:"
            };
            let gap_note = if comparison_coverage_gap {
                comparison_guard_summary
                    .as_ref()
                    .map(|summary| format!(" {summary}"))
                    .unwrap_or_default()
            } else {
                String::new()
            };
            trim_words(
                &format!("{prefix} {}{}", synthesized_insights.join("; "), gap_note),
                budget.max_summary_tokens,
            )
        }
    };
    let tool_result_quality = web_tool_quality_report(
        &query,
        status,
        before_dedup,
        evidence_refs.len(),
        &partial_failures,
        &hard_partial_failures,
        &evidence_ranked,
    );
    let source_class_coverage = source_class_coverage_from_ranked_candidates(
        &policy,
        &query,
        &evidence_ranked,
        &evidence_coverage,
    );
    let evidence_pack_quality =
        evidence_pack_quality_report(&policy, &evidence_pack, &evidence_coverage);
    let second_pass_recovery = json!({
        "enabled": second_pass_recovery_enabled(&policy),
        "used": !second_pass_queries.is_empty(),
        "reason": second_pass_reason,
        "queries": second_pass_queries.clone()
    });
    let retrieval_telemetry_value = Value::Array(retrieval_telemetry.clone());
    let provider_results_value = Value::Array(provider_results.clone());
    let retrieval_broker = retrieval_broker_report(
        status,
        json!(submitted_queries.clone()),
        json!(executed_queries.clone()),
        query_plan.query_plan_source,
        second_pass_recovery.clone(),
        &retrieval_telemetry_value,
        &provider_results_value,
        &evidence_pack,
        &evidence_coverage,
        &tool_result_quality,
        &source_class_coverage,
        &evidence_pack_quality,
    );
    let mut retrieval_broker = retrieval_broker;
    if let Some(obj) = retrieval_broker.as_object_mut() {
        obj.insert(
            "query_lane_attribution".to_string(),
            query_lane_attribution.clone(),
        );
    }

    let provider_snapshot = json!({
        "id": crate::deterministic_receipt_hash(&json!({"source": source, "queries": executed_queries, "search_scope": search_scope_value.clone()})),
        "source": source,
        "adapter_version": "web_conduit_v1",
        "disposable": true
    });
    let fresh_cache_status = cache_control.fresh_status();
    let receipt = json!({
        "type": "batch_query_receipt",
        "ts": crate::now_iso(),
        "source": source,
        "query": query,
        "aperture": aperture,
        "query_timeout_ms": query_timeout.as_millis() as u64,
        "parallel_window": parallel_window,
        "rewrite_set": rewrite_set,
        "query_plan": executed_queries,
        "submitted_query_plan": submitted_queries,
        "query_execution_limiter": query_execution_limiter.clone(),
        "second_pass_recovery": second_pass_recovery.clone(),
        "query_plan_source": query_plan.query_plan_source,
        "query_metadata": query_plan.query_metadata.to_value(),
        "query_contract": {
            "authority": "agent_submitted",
            "query_used": query,
            "hidden_query_expansion": false,
            "query_plan_source": query_plan.query_plan_source,
            "query_metadata": query_plan.query_metadata.to_value(),
            "search_scope": search_scope_value.clone()
        },
        "adapter_version": "web_conduit_v1",
        "provider_snapshot": provider_snapshot,
        "snapshot_id": provider_snapshot.get("id").cloned().unwrap_or(Value::Null),
        "candidate_count": before_dedup,
        "dedup_count": before_dedup.saturating_sub(candidates.len()),
        "provider_result_count": provider_results.len(),
        "provider_result_dedup_count": provider_result_dedup_count,
        "evidence_count": evidence_refs.len(),
        "evidence_pack_count": evidence_pack.as_array().map(|rows| rows.len()).unwrap_or(0),
        "evidence_claim_count": evidence_claims.as_array().map(|rows| rows.len()).unwrap_or(0),
        "cache_status": fresh_cache_status,
        "cache_mode": cache_control.mode.as_str(),
        "latency_ms": started.elapsed().as_millis() as u64,
        "token_usage": {"summary_tokens_estimate": summary.split_whitespace().count()},
        "parallel_retrieval_used": parallel_allowed,
        "retrieval_telemetry": retrieval_telemetry,
        "evidence_coverage": evidence_coverage.clone(),
        "source_class_coverage": source_class_coverage.clone(),
        "evidence_pack_quality": evidence_pack_quality.clone(),
        "evidence_selection_diagnostics": evidence_selection_diagnostics.clone(),
        "query_lane_attribution": query_lane_attribution.clone(),
        "retrieval_broker": retrieval_broker.clone(),
        "partial_failure_details": hard_partial_failures,
        "tool_result_quality": tool_result_quality.clone(),
        "status": status
    });
    let receipt_id = crate::deterministic_receipt_hash(&receipt);
    let mut receipt_with_id = receipt.clone();
    receipt_with_id["receipt_id"] = Value::String(receipt_id.clone());
    let _ = append_jsonl(&receipts_path(root), &receipt_with_id);

    let mut out = json!({
        "ok": status != "blocked",
        "type": "batch_query",
        "status": status,
        "source": source,
        "query": query,
        "aperture": aperture,
        "summary": summary.clone(),
        "evidence_refs": evidence_refs.clone(),
        "receipt_id": receipt_id,
        "parallel_retrieval_used": parallel_allowed,
        "query_timeout_ms": query_timeout.as_millis() as u64,
        "parallel_window": parallel_window,
        "rewrite_set": rewrite_set.clone(),
        "query_plan": executed_queries.clone(),
        "submitted_query_plan": submitted_queries.clone(),
        "query_execution_limiter": query_execution_limiter.clone(),
        "second_pass_recovery": second_pass_recovery.clone(),
        "query_plan_source": query_plan.query_plan_source,
        "query_metadata": query_plan.query_metadata.to_value(),
        "query_contract": {
            "authority": "agent_submitted",
            "query_used": query,
            "hidden_query_expansion": false,
            "query_plan_source": query_plan.query_plan_source,
            "query_metadata": query_plan.query_metadata.to_value(),
            "search_scope": search_scope_value.clone()
        },
        "evidence_claims": evidence_claims.clone(),
        "partial_failure_details": hard_partial_failures.clone(),
        "retrieval_telemetry": retrieval_telemetry.clone(),
        "evidence_coverage": evidence_coverage.clone(),
        "source_class_coverage": source_class_coverage.clone(),
        "evidence_pack_quality": evidence_pack_quality.clone(),
        "evidence_selection_diagnostics": evidence_selection_diagnostics.clone(),
        "query_lane_attribution": query_lane_attribution.clone(),
        "retrieval_broker": retrieval_broker.clone(),
        "tool_result_quality": tool_result_quality.clone(),
        "provider_result_count": provider_results.len(),
        "provider_result_dedup_count": provider_result_dedup_count,
        "cache_status": fresh_cache_status,
        "cache_mode": cache_control.mode.as_str()
    });
    if comparison_guard_search_results
        .as_array()
        .map(|rows| !rows.is_empty())
        .unwrap_or(false)
    {
        out["search_results"] = comparison_guard_search_results.clone();
    }
    if evidence_pack
        .as_array()
        .map(|rows| !rows.is_empty())
        .unwrap_or(false)
    {
        out["evidence_pack"] = evidence_pack.clone();
    }
    if evidence_coverage
        .as_array()
        .map(|rows| !rows.is_empty())
        .unwrap_or(false)
    {
        out["evidence_coverage"] = evidence_coverage.clone();
    }
    out["source_class_coverage"] = source_class_coverage.clone();
    out["evidence_pack_quality"] = evidence_pack_quality.clone();
    out["evidence_selection_diagnostics"] = evidence_selection_diagnostics.clone();
    out["query_lane_attribution"] = query_lane_attribution.clone();
    out["retrieval_broker"] = retrieval_broker.clone();
    if !provider_results.is_empty() {
        out["provider_results"] = Value::Array(provider_results.clone());
    }
    store_cached_response(
        root,
        &cache_key_primary,
        &json!({
            "status": status,
            "summary": summary,
            "evidence_refs": evidence_refs,
            "evidence_pack": evidence_pack,
            "evidence_coverage": evidence_coverage,
            "search_results": comparison_guard_search_results,
            "provider_results": provider_results,
            "rewrite_set": rewrite_set,
            "query_plan": executed_queries,
            "submitted_query_plan": submitted_queries,
            "query_execution_limiter": query_execution_limiter,
            "second_pass_recovery": second_pass_recovery,
            "query_plan_source": query_plan.query_plan_source,
            "query_metadata": query_plan.query_metadata.to_value(),
            "search_scope": search_scope_value,
            "evidence_claims": evidence_claims,
            "partial_failure_details": hard_partial_failures,
            "retrieval_telemetry": retrieval_telemetry,
            "source_class_coverage": source_class_coverage,
            "evidence_pack_quality": evidence_pack_quality,
            "evidence_selection_diagnostics": evidence_selection_diagnostics,
            "query_lane_attribution": query_lane_attribution,
            "retrieval_broker": retrieval_broker,
            "tool_result_quality": tool_result_quality,
            "parallel_retrieval_used": parallel_allowed
        }),
        status,
        &cache_control,
    );
    if let Some(code) = no_results_error_code_from_failure_strings(&summary, &hard_partial_failures)
    {
        out["error"] = Value::String(code.to_string());
    }
    if let Some(meta) = nexus_connection {
        out["nexus_connection"] = meta;
    }
    out
}
