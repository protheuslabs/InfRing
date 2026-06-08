    #[test]
    fn access_blocker_ignores_topic_mentions_in_evidence_snippets() {
        let payload = json!({
            "tools": [{
                "result": "Browser-agent security sources discuss Cloudflare challenge flows, WAF designs, and bot-detection countermeasures.",
                "evidence_pack": [{
                    "snippet": "Cloudflare bot detection and WAF controls are common topics in browser-agent security writeups."
                }]
            }]
        });
        let retrieval_quality = json!({
            "status": "usable",
            "usable_evidence": true
        });
        let blocker = web_access_blocker_diagnostics(&payload, &retrieval_quality);
        assert_eq!(
            blocker.get("detected").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            blocker
                .pointer("/classes/anti_bot_challenge")
                .and_then(Value::as_bool),
            Some(false)
        );
    }

    #[test]
    fn access_blocker_detects_real_challenge_copy_in_result_body() {
        let payload = json!({
            "tools": [{
                "result": "Unfortunately, bots use DuckDuckGo too. Please complete the following challenge to verify you are human. Cloudflare protection is active."
            }]
        });
        let blocker = web_access_blocker_diagnostics(&payload, &json!({}));
        assert_eq!(
            blocker
                .pointer("/classes/anti_bot_challenge")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            blocker.get("kind").and_then(Value::as_str),
            Some("anti_bot_challenge")
        );
    }

    #[test]
    fn access_blocker_detects_provider_quota_snake_case_errors() {
        let payload = json!({
            "provider_results": [{
                "provider": "exa",
                "status": "error",
                "error": "provider_circuit_open:exa_provider_quota_exceeded_or_billing_required_http_402"
            }]
        });
        let blocker = web_access_blocker_diagnostics(&payload, &json!({}));
        assert_eq!(
            blocker.pointer("/classes/rate_limit_or_quota").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            blocker.get("kind").and_then(Value::as_str),
            Some("throttle_or_rate_limit")
        );
    }

    #[test]
    fn excludes_post_tool_cases_when_only_derived_fallback_request_exists() {
        let case = json!({
            "category": "post_tool_synthesis"
        });
        let payload = json!({
            "pending_tool_request": {
                "input": {
                    "query": "After the web tool returns low-signal results...",
                    "query_metadata_policy": {
                        "classification": "derived_prompt_request"
                    }
                }
            },
            "tools": [{
                "status": "blocked"
            }]
        });
        let retrieval_quality = json!({
            "tool_executed": true,
            "status": "provider_degraded"
        });
        assert_eq!(
            web_tooling_measurement_exclusion_reason_case(&case, &payload, &retrieval_quality),
            Some("post_tool_context_not_seeded")
        );
    }

    #[test]
    fn provider_surface_gate_passes_when_degraded_provider_still_yields_materialized_evidence() {
        let payload = json!({
            "pending_tool_request": {
                "tool_key": "batch_query",
                "input": {
                    "query": "Find recent benchmarks comparing agent frameworks",
                    "queries": ["Find recent benchmarks comparing agent frameworks"],
                    "keywords": ["benchmarks", "agent frameworks"],
                    "required_coverage": {"entities": ["agent frameworks"], "facets": ["benchmarks"]}
                }
            },
            "tools": [{
                "status": "low_signal"
            }],
            "provider_results": [{
                "provider_raw_count": 126,
                "result_quality": "provider_error",
                "synthesis_candidate_count": 20
            }],
            "query_lane_attribution": {
                "rows": [{
                    "provider_raw_rows": 126,
                    "candidate_rows": 20,
                    "synthesis_candidate_rows": 6,
                    "filtered_or_rejected_rows": 19
                }]
            },
            "evidence_refs": [
                {"snippet": "benchmark writeup", "claim_hints": ["coverage gap"], "source_domain": "example.com"}
            ]
        });
        let retrieval_quality = json!({
            "status": "low_signal",
            "candidate_count": 20,
            "evidence_count": 6,
            "content_rich_candidate_count": 6,
            "materialized_candidate_count": 6,
            "claim_hint_count": 2,
            "usable_evidence": false,
            "quality_flags": ["explicit_low_signal_marker"]
        });
        let query_metadata = json!({
            "metadata_present": true,
            "rich_query_pack_or_narrow_marker": true
        });
        let transitions = json!({
            "checkpoints": [{
                "checkpoint": "5e_agent_received_evidence_context",
                "status": "pass"
            }]
        });
        let diag = web_retrieval_gate_diagnostics(
            &payload,
            &retrieval_quality,
            &query_metadata,
            &transitions,
        );
        let gate_4c = diag
            .get("gates")
            .and_then(Value::as_array)
            .and_then(|rows| {
                rows.iter().find(|row| {
                    row.get("gate").and_then(Value::as_str)
                        == Some("web_4c_search_provider_surface_ready")
                })
            })
            .cloned()
            .expect("web_4c gate");
        let gate_7 = diag
            .get("gates")
            .and_then(Value::as_array)
            .and_then(|rows| {
                rows.iter().find(|row| {
                    row.get("gate").and_then(Value::as_str)
                        == Some("web_7_usable_evidence_available")
                })
            })
            .cloned()
            .expect("web_7 gate");
        assert_eq!(gate_4c.get("status").and_then(Value::as_str), Some("pass"));
        assert_eq!(gate_7.get("status").and_then(Value::as_str), Some("fail"));
    }

    #[test]
    fn browser_serp_shell_only_failure_gets_specific_candidate_supply_gate() {
        let payload = json!({
            "pending_tool_request": {
                "tool_key": "web_search",
                "input": {
                    "query": "compare Firecrawl Tavily Exa web search APIs",
                    "keywords": ["Firecrawl", "Tavily", "Exa"]
                }
            },
            "provider_errors": [{
                "provider": "browser_serp",
                "error": "browser_serp_no_results",
                "provider_raw_count": 2,
                "provider_filtered_count": 0,
                "browser_serp_diagnostics": [{
                    "engine": "bing_html",
                    "materialization_ok": true,
                    "challenge_detected": false,
                    "outcome_classification": {
                        "outcome_class": "serp_shell_without_organic_results",
                        "evidence_impact": "rejected",
                        "recommended_next_capability": "serp_dom_rendering_or_alternate_search_provider"
                    }
                }]
            }],
            "tools": [{
                "tool": "web_search",
                "status": "low_signal",
                "result": "The tool path ran but browser SERP produced no usable organic search results."
            }]
        });
        let retrieval_quality = json!({
            "tool_executed": true,
            "status": "provider_degraded",
            "candidate_count": 0,
            "evidence_count": 0,
            "quality_flags": ["query_result_mismatch"]
        });
        let query_metadata = json!({
            "metadata_present": true,
            "rich_query_pack_or_narrow_marker": true
        });
        let diag = web_retrieval_gate_diagnostics(
            &payload,
            &retrieval_quality,
            &query_metadata,
            &json!({"checkpoints": []}),
        );
        assert_eq!(
            diag.get("first_failed_gate").and_then(Value::as_str),
            Some("web_4e_browser_serp_external_urls_extracted")
        );
        assert_eq!(
            diag.get("inferred_failure_boundary").and_then(Value::as_str),
            Some("browser_serp_no_external_organic_urls")
        );
        assert_eq!(
            diag.pointer("/provider_supply/browser_serp_shell_or_no_organic")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            diag.pointer("/provider_supply/browser_serp_outcome_classes/0")
                .and_then(Value::as_str),
            Some("serp_shell_without_organic_results")
        );
    }

    #[test]
    fn browser_serp_specific_gate_does_not_penalize_non_browser_provider_runs() {
        let payload = json!({
            "pending_tool_request": {
                "tool_key": "web_search",
                "input": {
                    "query": "recent database benchmarks",
                    "keywords": ["database", "benchmarks"]
                }
            },
            "provider_results": [{
                "provider": "tavily",
                "provider_raw_count": 5,
                "provider_filtered_count": 3
            }],
            "evidence_refs": [{
                "source_domain": "example.com",
                "snippet": "Benchmark evidence",
                "claim_hints": ["database benchmark"]
            }]
        });
        let retrieval_quality = json!({
            "tool_executed": true,
            "status": "usable",
            "candidate_count": 3,
            "evidence_count": 1,
            "content_rich_candidate_count": 1,
            "materialized_candidate_count": 1,
            "claim_hint_count": 1,
            "usable_evidence": true
        });
        let query_metadata = json!({
            "metadata_present": true,
            "rich_query_pack_or_narrow_marker": true
        });
        let diag = web_retrieval_gate_diagnostics(
            &payload,
            &retrieval_quality,
            &query_metadata,
            &json!({"checkpoints": [{"checkpoint": "5e_agent_received_evidence_context", "status": "pass"}]}),
        );
        let browser_gate = diag
            .get("gates")
            .and_then(Value::as_array)
            .and_then(|rows| {
                rows.iter().find(|row| {
                    row.get("gate").and_then(Value::as_str)
                        == Some("web_4e_browser_serp_external_urls_extracted")
                })
            })
            .expect("browser SERP gate");
        assert_eq!(
            browser_gate.get("artifact_present").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(browser_gate.get("status").and_then(Value::as_str), Some("pass"));
    }
