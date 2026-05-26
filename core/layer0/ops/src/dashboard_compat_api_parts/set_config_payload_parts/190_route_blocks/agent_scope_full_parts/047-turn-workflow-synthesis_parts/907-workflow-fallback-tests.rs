    #[test]
    fn tool_evidence_fallback_records_evidence_outcome_posture() {
        let mut workflow = json!({
            "response": "",
            "quality_telemetry": {},
            "final_llm_response": {
                "used": false,
                "status": "synthesis_failed"
            }
        });
        let tools = vec![json!({
            "name": "batch_query",
            "status": "ok",
            "result": "Search returned some findings but major coverage gaps remain.",
            "query_metadata": {
                "required_coverage": {
                    "entities": ["AlphaTool", "BetaTool"]
                }
            },
            "evidence_refs": [{
                "title": "AlphaTool docs",
                "locator": "https://example.com/alpha",
                "score": 0.8
            }],
            "tool_result_quality": {
                "status": "low_signal",
                "flags": ["insufficient_evidence"]
            }
        })];

        apply_final_empty_response_diagnostic(
            &mut workflow,
            "Compare AlphaTool and BetaTool.",
            "",
            &tools,
        );

        assert_eq!(
            workflow
                .pointer("/final_llm_response/evidence_outcome_posture")
                .and_then(Value::as_str),
            Some("bounded_partial_answer")
        );
        assert_eq!(
            workflow
                .pointer("/quality_telemetry/evidence_outcome_posture")
                .and_then(Value::as_str),
            Some("bounded_partial_answer")
        );
    }

    #[test]
    fn apply_final_empty_response_diagnostic_suppresses_tool_evidence_fallback() {
        let mut workflow = json!({
            "response": "",
            "response_finalization": {
                "final_response": {}
            },
            "response_workflow": {
                "final_llm_response": {}
            },
            "final_llm_response": {}
        });
        let tools = vec![json!({
            "name": "batch_query",
            "status": "ok",
            "result": "Key findings: LangGraph is production-oriented while CrewAI is stronger for rapid prototyping.",
            "evidence_refs": [{
                "title": "Framework comparison",
                "snippet": "LangGraph is production-oriented while CrewAI is stronger for rapid prototyping."
            }]
        })];

        apply_final_empty_response_diagnostic(
            &mut workflow,
            "Compare LangGraph and CrewAI.",
            "",
            &tools,
        );

        let response = workflow
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(!response.trim().is_empty(), "{response}");
        assert!(!response.contains("Sources:"), "{response}");
        assert_eq!(
            workflow
                .pointer("/final_llm_response/status")
                .and_then(Value::as_str),
            Some("tool_evidence_fallback_used")
        );
        assert_eq!(
            workflow
                .pointer("/quality_telemetry/final_fallback_used")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn rejected_tool_backed_response_suppresses_runtime_coverage_note_rewrite() {
        let mut workflow = json!({
            "response": "",
            "quality_telemetry": {},
            "final_llm_response": {
                "used": false,
                "status": "synthesis_failed"
            }
        });
        let tools = vec![json!({
            "name": "batch_query",
            "status": "ok",
            "result": "Top findings: pgvector is simple for small teams; Weaviate is a managed vector database with additional operational surface.",
            "evidence_refs": [{
                "title": "pgvector",
                "snippet": "pgvector is simple for small teams."
            }],
            "query_metadata": {
                "required_coverage": {
                    "entities": ["LlamaIndex", "LangChain", "pgvector", "Weaviate", "Chroma"]
                }
            }
        })];
        let rejected_response =
            "Based on the available evidence, here's a pragmatic recommendation for a small team's RAG stack: pgvector is the simplest default for small teams, while Weaviate has more managed operational surface according to the recorded source.";

        let rewritten = maybe_apply_rejected_tool_evidence_fallback(
            &mut workflow,
            "Research current RAG stack options for a small team.",
            &tools,
            rejected_response,
            rejected_response,
            "final_response_verifier_contract:missing_coverage_lanes=Weaviate, Chroma",
        );

        assert!(rewritten);
        let response = workflow
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(!response.trim().is_empty(), "{response}");
        assert_eq!(
            workflow
                .pointer("/final_llm_response/status")
                .and_then(Value::as_str),
            Some("tool_evidence_fallback_used")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/replacement_response_used")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/original_reject_reason")
                .and_then(Value::as_str),
            Some("final_response_verifier_contract:missing_coverage_lanes=Weaviate, Chroma")
        );
    }

    #[test]
    fn rejected_tool_backed_response_without_required_lane_presence_uses_fallback_rewrite() {
        let mut workflow = json!({
            "response": "",
            "quality_telemetry": {},
            "final_llm_response": {
                "used": false,
                "status": "synthesis_failed"
            }
        });
        let tools = vec![json!({
            "name": "batch_query",
            "status": "ok",
            "result": "Retrieved a governed AI evidence-retrieval paper that does not directly compare the requested frameworks.",
            "query_metadata": {
                "required_coverage": {
                    "entities": ["LlamaIndex", "LangGraph"]
                }
            },
            "evidence_pack": [{
                "title": "Kura governed AI",
                "source_domain": "example.test",
                "relevant_extract": "Kura focuses on governed evidence retrieval for AI systems.",
                "claim_hints": ["Kura focuses on governed evidence retrieval for AI systems."],
                "counts_as_usable_evidence": true
            }]
        })];
        let rejected_response =
            "An answer grounding pipeline is more than RAG because it adds governed evidence routing and source-trust controls.";

        let rewritten = maybe_apply_rejected_tool_evidence_fallback(
            &mut workflow,
            "Compare LlamaIndex workflows versus LangGraph for document-heavy research assistants.",
            &tools,
            rejected_response,
            rejected_response,
            "final_response_verifier_contract:missing_coverage_lanes=LlamaIndex, LangGraph",
        );

        assert!(rewritten);
        let response = workflow
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(!response.trim().is_empty(), "{response}");
        assert!(
            !response.contains("answer grounding pipeline is more than RAG"),
            "{response}"
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/status")
                .and_then(Value::as_str),
            Some("tool_evidence_fallback_used")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/replacement_response_used")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn tool_input_required_coverage_becomes_synthesis_lanes() {
        let input = json!({
            "query": "Compare AlphaTool and BetaTool for current research workflows.",
            "required_coverage": {
                "entities": ["AlphaTool", "BetaTool"],
                "facets": ["current fit", "operational risk"]
            },
            "query_metadata_policy": {
                "classification": "expanded_query_pack"
            }
        })
        .to_string();
        let tools = vec![json!({
            "name": "batch_query",
            "status": "ok",
            "is_error": false,
            "input": input,
            "evidence_refs": [{
                "title": "AlphaTool docs",
                "snippet": "AlphaTool has current public documentation."
            }]
        })];

        let lanes = synthesis_coverage_lanes_for_tools(&tools, 8);
        assert!(
            lanes.iter().any(|row| {
                row.get("kind").and_then(Value::as_str) == Some("entity")
                    && row.get("requested_text").and_then(Value::as_str) == Some("AlphaTool")
            }),
            "{lanes:#?}"
        );
        assert!(
            lanes.iter().any(|row| {
                row.get("kind").and_then(Value::as_str) == Some("facet")
                    && row.get("requested_text").and_then(Value::as_str) == Some("current fit")
            }),
            "{lanes:#?}"
        );

        let missing = response_missing_required_entity_lanes("AlphaTool has evidence.", &tools);
        assert!(missing.iter().any(|row| row == "BetaTool"), "{missing:#?}");
        assert!(response_missing_required_entity_lanes(
            "AlphaTool and BetaTool both need source-backed treatment.",
            &tools
        )
        .is_empty());
    }

    #[test]
    fn tool_quality_covered_marks_query_metadata_coverage_usable() {
        let input = json!({
            "query": "Give me a broad market landscape update.",
            "required_coverage": {
                "entities": ["expanded alias lane"],
                "facets": ["adoption"]
            }
        })
        .to_string();
        let tools = vec![json!({
            "name": "batch_query",
            "status": "ok",
            "is_error": false,
            "input": input,
            "evidence_refs": [{
                "title": "Market landscape source",
                "snippet": "A substantive source-backed landscape result."
            }],
            "tool_result_quality": {
                "coverage": {
                    "bucket_status": "covered",
                    "missing_buckets": []
                }
            }
        })];

        let lanes = synthesis_coverage_lanes_for_tools(&tools, 8);
        assert!(
            lanes.iter().any(|row| {
                row.get("requested_text").and_then(Value::as_str) == Some("expanded alias lane")
                    && row.get("status").and_then(Value::as_str) == Some("usable")
            }),
            "{lanes:#?}"
        );
    }
