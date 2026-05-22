    #[test]
    fn nonblocking_provider_failures_do_not_mask_claim_extraction_gap() {
        let payload = json!({
            "pending_tool_request": {
                "tool_key": "batch_query",
                "input": {
                    "query": "Compare Alpha and Beta",
                    "queries": ["Compare Alpha and Beta"],
                    "keywords": ["Alpha", "Beta"]
                }
            },
            "tools": [{
                "status": "partial"
            }],
            "partial_failure_details": [
                "provider_circuit_open",
                "provider_error",
                "missing api key",
                "rate_limited",
                "captcha challenge"
            ],
            "provider_results": [{
                "provider_raw_count": 40,
                "candidate_rows": 12,
                "synthesis_candidate_count": 6,
                "result_quality": "provider_error"
            }],
            "evidence_refs": [{
                "title": "Alpha and Beta overview",
                "snippet": "The source names Alpha and Beta but does not expose a usable claim."
            }]
        });
        let retrieval_quality = json!({
            "status": "low_signal",
            "candidate_count": 12,
            "evidence_count": 4,
            "content_rich_candidate_count": 4,
            "materialized_candidate_count": 4,
            "claim_hint_count": 0,
            "usable_evidence": false
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
        let gates = diag
            .get("gates")
            .and_then(Value::as_array)
            .cloned()
            .expect("gates");
        for name in [
            "web_3b1_provider_quota_not_rate_limited",
            "web_3b2_no_bot_challenge_or_waf",
            "web_3b5_provider_configuration_available",
            "web_3b_access_not_blocked_or_throttled",
            "web_4b_search_provider_circuit_closed",
            "web_4c_search_provider_surface_ready",
        ] {
            let gate = gates
                .iter()
                .find(|row| row.get("gate").and_then(Value::as_str) == Some(name))
                .cloned()
                .unwrap_or_else(|| panic!("missing {name}"));
            assert_eq!(
                gate.get("status").and_then(Value::as_str),
                Some("pass"),
                "{name}: {gate:#?}"
            );
        }
        assert_eq!(
            diag.pointer("/first_failed_gate").and_then(Value::as_str),
            Some("web_5c_claim_extraction_present")
        );
    }

    #[test]
    fn operator_metrics_surface_materialization_failure_reason() {
        let payload = json!({
            "pending_tool_request": {
                "tool_key": "batch_query",
                "input": {
                    "query": "Find recent benchmarks comparing agent frameworks"
                }
            },
            "tools": [{
                "status": "low_signal"
            }]
        });
        let retrieval_quality = json!({
            "status": "low_signal",
            "candidate_count": 12,
            "evidence_count": 4,
            "content_rich_candidate_count": 0,
            "materialized_candidate_count": 0,
            "claim_hint_count": 0,
            "usable_evidence": false,
            "materialization_failure_report": {
                "top_reason": {"reason": "content_too_thin", "count": 4},
                "reason_rows": [
                    {"reason": "content_too_thin", "count": 4}
                ]
            }
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
        assert_eq!(
            diag.pointer("/operator_metrics/materialization/top_failure_reason")
                .and_then(Value::as_str),
            Some("content_too_thin")
        );
    }

    #[test]
    fn browser_materialization_gate_ignores_nonblocking_enrichment_failures() {
        let payload = json!({
            "pending_tool_request": {
                "tool_key": "batch_query",
                "input": {
                    "query": "Compare browser agents"
                }
            },
            "tools": [{
                "status": "partial"
            }]
        });
        let retrieval_quality = json!({
            "status": "low_signal",
            "candidate_count": 18,
            "evidence_count": 3,
            "content_rich_candidate_count": 0,
            "materialized_candidate_count": 0,
            "claim_hint_count": 0,
            "usable_evidence": false,
            "browser_materialization": {
                "attempted": true,
                "failed": true
            },
            "materialization_failure_report": {
                "top_reason": {"reason": "prefetch_rejected_off_intent", "count": 8},
                "reason_rows": [
                    {"reason": "prefetch_rejected_off_intent", "count": 8},
                    {"reason": "browser_materialization_failed", "count": 2}
                ]
            }
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
        let gate_3d = diag
            .get("gates")
            .and_then(Value::as_array)
            .and_then(|rows| {
                rows.iter().find(|row| {
                    row.get("gate").and_then(Value::as_str)
                        == Some("web_3d_browser_materialization_not_failed")
                })
            })
            .cloned()
            .expect("web_3d gate");
        assert_eq!(gate_3d.get("status").and_then(Value::as_str), Some("pass"));
    }
