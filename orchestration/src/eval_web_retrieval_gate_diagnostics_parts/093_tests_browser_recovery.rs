    #[test]
    fn browser_materialization_gate_allows_usable_retrieval_even_if_browser_failed() {
        let payload = json!({
            "pending_tool_request": {
                "tool_key": "batch_query",
                "input": {
                    "query": "Compare OpenAI Agents SDK with LangChain/LangGraph"
                }
            },
            "tools": [{
                "status": "partial"
            }],
            "evidence_refs": [{
                "title": "Web research API comparison",
                "locator": "https://docs.example.com/web-research-api-comparison",
                "source_domain": "docs.example.com",
                "snippet": "The comparison describes differences between web research APIs, including search result structure, crawling support, and source citation handling for agent workflows.",
                "claim_hints": [
                    "The comparison describes differences between web research APIs across result structure, crawling support, and citation handling for agent workflows."
                ]
            }]
        });
        let retrieval_quality = json!({
            "status": "usable",
            "candidate_count": 14,
            "evidence_count": 8,
            "content_rich_candidate_count": 11,
            "materialized_candidate_count": 8,
            "claim_hint_count": 10,
            "usable_evidence": true,
            "browser_materialization": {
                "attempted": true,
                "failed": true
            },
            "materialization_failure_report": {
                "top_reason": {"reason": "browser_materialization_failed", "count": 22},
                "reason_rows": [
                    {"reason": "browser_materialization_failed", "count": 22}
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

    #[test]
    fn browser_materialization_gate_allows_relevance_failure_after_content_arrived() {
        let payload = json!({
            "pending_tool_request": {
                "tool_key": "batch_query",
                "input": {
                    "query": "Compare Firecrawl, Tavily, and Exa"
                }
            },
            "tools": [{
                "status": "partial"
            }],
            "evidence_refs": [{
                "title": "Web research API comparison",
                "locator": "https://docs.example.com/web-research-api-comparison",
                "source_domain": "docs.example.com",
                "snippet": "The comparison describes differences between web research APIs, including search result structure, crawling support, and source citation handling for agent workflows.",
                "claim_hints": [
                    "The comparison describes differences between web research APIs across result structure, crawling support, and citation handling for agent workflows."
                ]
            }]
        });
        let retrieval_quality = json!({
            "status": "low_relevance",
            "candidate_count": 41,
            "evidence_count": 1,
            "content_rich_candidate_count": 1,
            "materialized_candidate_count": 1,
            "claim_hint_count": 1,
            "usable_evidence": false,
            "browser_materialization": {
                "attempted": true,
                "failed": true
            },
            "materialization_failure_report": {
                "top_reason": {"reason": "browser_materialization_failed", "count": 9},
                "reason_rows": [
                    {"reason": "browser_materialization_failed", "count": 9}
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
        assert_eq!(
            diag.get("first_failed_gate").and_then(Value::as_str),
            Some("web_7_usable_evidence_available")
        );
    }

    #[test]
    fn access_and_anti_bot_gates_allow_recovered_usable_retrieval() {
        let payload = json!({
            "pending_tool_request": {
                "tool_key": "batch_query",
                "input": {
                    "query": "Research MCP maturity"
                }
            },
            "tools": [{
                "status": "partial",
                "result": "Unfortunately, bots use DuckDuckGo too. Please complete the following challenge to verify you are human."
            }]
        });
        let retrieval_quality = json!({
            "status": "usable",
            "candidate_count": 20,
            "evidence_count": 12,
            "content_rich_candidate_count": 9,
            "materialized_candidate_count": 6,
            "claim_hint_count": 7,
            "usable_evidence": true,
            "browser_materialization": {
                "attempted": true,
                "failed": false
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
        let gates = diag
            .get("gates")
            .and_then(Value::as_array)
            .cloned()
            .expect("gates");
        let gate_3b2 = gates
            .iter()
            .find(|row| {
                row.get("gate").and_then(Value::as_str) == Some("web_3b2_no_bot_challenge_or_waf")
            })
            .cloned()
            .expect("web_3b2 gate");
        let gate_3b = gates
            .iter()
            .find(|row| {
                row.get("gate").and_then(Value::as_str)
                    == Some("web_3b_access_not_blocked_or_throttled")
            })
            .cloned()
            .expect("web_3b access gate");
        assert_eq!(gate_3b2.get("status").and_then(Value::as_str), Some("pass"));
        assert_eq!(gate_3b.get("status").and_then(Value::as_str), Some("pass"));
    }
