    #[test]
    fn claim_extraction_gate_uses_direct_evidence_claim_contract_when_present() {
        let payload = json!({
            "pending_tool_request": {
                "tool_key": "batch_query",
                "input": {
                    "query": "Give me news from this week",
                    "queries": ["Give me news from this week"],
                    "keywords": ["news", "this week"]
                }
            },
            "tools": [{
                "status": "partial"
            }],
            "evidence_refs": [
                {
                    "title": "Generic current events page",
                    "snippet": "A thin row that should not count as claim-backed evidence.",
                    "claim_hints": ["Thin inferred hint"]
                }
            ]
        });
        let retrieval_quality = json!({
            "status": "usable",
            "candidate_count": 8,
            "evidence_count": 4,
            "content_rich_candidate_count": 4,
            "materialized_candidate_count": 4,
            "claim_hint_count": 6,
            "usable_evidence": false,
            "classification_inputs": {
                "direct_contract_present": true,
                "direct_evidence_claim_count": 0
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
        let gate_5c = diag
            .get("gates")
            .and_then(Value::as_array)
            .and_then(|rows| {
                rows.iter().find(|row| {
                    row.get("gate").and_then(Value::as_str)
                        == Some("web_5c_claim_extraction_present")
                })
            })
            .cloned()
            .expect("web_5c gate");
        assert_eq!(gate_5c.get("status").and_then(Value::as_str), Some("fail"));
        assert_eq!(
            diag.pointer("/first_failed_gate").and_then(Value::as_str),
            Some("web_5c_claim_extraction_present")
        );
        assert_eq!(
            diag.pointer("/operator_metrics/claim_extraction/direct_evidence_claim_count")
                .and_then(Value::as_u64),
            Some(0)
        );
    }

    #[test]
    fn evidence_quality_gates_reject_title_like_claim_fragments() {
        let payload = json!({
            "pending_tool_request": {
                "tool_key": "batch_query",
                "input": {
                    "query": "give me news from this week",
                    "queries": ["give me news from this week"],
                    "keywords": ["news", "this week"]
                }
            },
            "tools": [{
                "status": "usable"
            }],
            "evidence_refs": [{
                "title": "Cooler this Week",
                "locator": "https://www.wlns.com/weather/cooler-this-week/",
                "source_domain": "wlns.com",
                "snippet": "WLNS 6 News reported a local weather story published during the week with cooler temperatures expected in Lansing, Michigan.",
                "claim_hints": ["Cooler this Week"]
            }]
        });
        let retrieval_quality = json!({
            "status": "usable",
            "candidate_count": 4,
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
            diag.pointer("/first_failed_gate").and_then(Value::as_str),
            Some("web_5e_claim_quality_ready")
        );
        assert_eq!(
            diag.pointer("/evidence_quality/claim_quality_ready")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            diag.pointer("/evidence_quality/citation_renderability_ready")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn evidence_quality_gates_pass_clean_source_backed_claims() {
        let payload = json!({
            "pending_tool_request": {
                "tool_key": "batch_query",
                "input": {
                    "query": "compare web research APIs",
                    "queries": ["compare web research APIs"],
                    "keywords": ["web research APIs", "source links", "raw content"]
                }
            },
            "tools": [{
                "status": "usable"
            }],
            "evidence_refs": [{
                "title": "Search API documentation",
                "locator": "https://docs.example.com/search-api",
                "source_type": "official_docs",
                "source_kind": "official_docs",
                "source_domain": "docs.example.com",
                "snippet": "The documentation describes a search API that returns answer-ready result objects with source links, snippets, and raw content fields for retrieval workflows.",
                "relevant_extract": "The documentation describes a search API that returns answer-ready result objects with source links, snippets, and raw content fields for retrieval workflows.",
                "why_relevant_to_query": "It directly describes web research API result fields that matter for comparing retrieval tooling.",
                "claim_hints": [
                    "The search API returns structured result objects with source links, snippets, and raw content fields for retrieval workflows."
                ]
            }]
        });
        let retrieval_quality = json!({
            "status": "usable",
            "candidate_count": 6,
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
            diag.pointer("/first_failed_gate").and_then(Value::as_str),
            None
        );
        for gate_name in [
            "web_5d_source_quality_ready",
            "web_5e_claim_quality_ready",
            "web_5f_citation_renderability_ready",
            "web_5g_answerability_ready",
            "web_5h_evidence_packet_contract_ready",
            "web_7_usable_evidence_available",
        ] {
            let gate = diag
                .get("gates")
                .and_then(Value::as_array)
                .and_then(|rows| {
                    rows.iter()
                        .find(|row| row.get("gate").and_then(Value::as_str) == Some(gate_name))
                })
                .cloned()
                .unwrap_or_else(|| panic!("missing {gate_name}"));
            assert_eq!(
                gate.get("status").and_then(Value::as_str),
                Some("pass"),
                "{gate_name}: {gate:#?}"
            );
        }
    }

    #[test]
    fn evidence_packet_contract_gate_rejects_answerable_but_unexplained_packets() {
        let payload = json!({
            "pending_tool_request": {
                "tool_key": "batch_query",
                "input": {
                    "query": "compare web research APIs",
                    "queries": ["compare web research APIs"],
                    "keywords": ["web research APIs", "source links", "raw content"]
                }
            },
            "tools": [{
                "status": "usable"
            }],
            "evidence_refs": [{
                "title": "Search API documentation",
                "locator": "https://docs.example.com/search-api",
                "source_type": "official_docs",
                "source_kind": "official_docs",
                "source_domain": "docs.example.com",
                "snippet": "The documentation describes a search API that returns answer-ready result objects with source links, snippets, and raw content fields for retrieval workflows.",
                "relevant_extract": "The documentation describes a search API that returns answer-ready result objects with source links, snippets, and raw content fields for retrieval workflows.",
                "claim_hints": [
                    "The search API returns structured result objects with source links, snippets, and raw content fields for retrieval workflows."
                ]
            }]
        });
        let retrieval_quality = json!({
            "status": "usable",
            "candidate_count": 6,
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
            diag.pointer("/first_failed_gate").and_then(Value::as_str),
            Some("web_5h_evidence_packet_contract_ready")
        );
        assert_eq!(
            diag.pointer("/evidence_quality/evidence_packet_contract/ready")
                .and_then(Value::as_bool),
            Some(false)
        );
        let empty = Vec::new();
        assert!(
            diag.pointer("/evidence_quality/evidence_packet_contract/missing_fields")
                .and_then(Value::as_array)
                .unwrap_or(&empty)
                .iter()
                .any(|row| row.as_str() == Some("why_relevant_to_query")),
            "{diag:#?}"
        );
    }
