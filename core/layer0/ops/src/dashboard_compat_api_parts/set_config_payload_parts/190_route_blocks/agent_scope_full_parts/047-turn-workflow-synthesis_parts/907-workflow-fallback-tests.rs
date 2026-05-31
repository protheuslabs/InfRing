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
    fn tool_evidence_fallback_sanitizes_provider_diagnostic_failures() {
        let mut workflow = json!({
            "response": "",
            "quality_telemetry": {},
            "final_llm_response": {}
        });
        let tools = vec![json!({
            "name": "web_search",
            "status": "no_results",
            "result": "Search provider chain exhausted: credentialed providers were not configured, and available fallback providers did not return usable evidence.",
            "tool_result_quality": {
                "status": "provider_starved",
                "flags": ["missing_strong_search_provider"]
            }
        })];

        apply_final_empty_response_diagnostic(
            &mut workflow,
            "Give me current global news from this week.",
            "",
            &tools,
        );

        let response = workflow
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(!response.trim().is_empty(), "{response}");
        assert!(!response.contains("Search provider chain exhausted"), "{response}");
        assert!(!response.contains("credentialed providers"), "{response}");
        assert!(!response.contains("Tool failures:"), "{response}");
        assert!(
            response.contains("usable evidence") || response.contains("reliable information"),
            "{response}"
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
    fn rejected_bounded_llm_answer_is_preserved_instead_of_runtime_tool_fragment() {
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
            "status": "no_results",
            "summary": "No usable evidence claims or concrete answer units were extracted from the results.",
            "evidence_refs": [{
                "title": "Off-topic framework source",
                "locator": "https://example.test/off-topic"
            }],
            "tool_result_quality": {
                "status": "low_signal",
                "flags": ["insufficient_evidence", "low_relevance_filtered"]
            }
        })];
        let rejected_response =
            "I can't make a reliable comparison from this search because the available evidence did not return source-backed coverage of the requested frameworks.";

        let rewritten = maybe_apply_rejected_tool_evidence_fallback(
            &mut workflow,
            "Compare the current agentic framework landscape.",
            &tools,
            rejected_response,
            rejected_response,
            "final_response_verifier_contract:missing_coverage_lanes=frameworks",
        );

        assert!(rewritten);
        assert_eq!(
            workflow.get("response").and_then(Value::as_str),
            Some(rejected_response)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/status")
                .and_then(Value::as_str),
            Some("synthesized")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/visible_response_preserved")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/replacement_response_used")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/verifier_reject_suppressed")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn rejected_concise_named_evidence_gap_is_preserved_as_synthesis() {
        let mut workflow = json!({
            "response": "",
            "quality_telemetry": {},
            "final_llm_response": {
                "used": false,
                "status": "synthesis_failed"
            }
        });
        let tools = vec![json!({
            "name": "web_search",
            "status": "success",
            "summary": "Search completed with low-confidence snippets.",
            "evidence_refs": [{
                "title": "Framework comparison search",
                "locator": "https://example.test/frameworks"
            }],
            "tool_result_quality": {
                "status": "low_signal",
                "flags": ["insufficient_evidence"]
            }
        })];
        let rejected_response =
            "The web search did not return source-backed evidence on agentic framework comparisons for April 2026.";

        let rewritten = maybe_apply_rejected_tool_evidence_fallback(
            &mut workflow,
            "Synthesize the current framework comparison from the tool result.",
            &tools,
            rejected_response,
            rejected_response,
            "final_response_verifier_contract:status_before_answer",
        );

        assert!(rewritten);
        assert_eq!(
            workflow.get("response").and_then(Value::as_str),
            Some(rejected_response)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/status")
                .and_then(Value::as_str),
            Some("synthesized")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/replacement_response_used")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/verifier_reject_suppressed")
                .and_then(Value::as_bool),
            Some(true)
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
    fn rejected_tool_backed_response_salvages_supported_units_before_generic_coverage_note() {
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
            "query_metadata": {
                "required_coverage": {
                    "entities": ["meeting overload", "remote teams"]
                }
            },
            "evidence_pack": [{
                "title": "Field experiment summary",
                "source_domain": "example.test",
                "relevant_extract": "For remote teams, a large-scale field experiment found that prompting attendees to define meeting goals in advance improved meeting effectiveness and reduced meeting overload.",
                "claim_hints": ["For remote teams, a large-scale field experiment found that prompting attendees to define meeting goals in advance improved meeting effectiveness and reduced meeting overload."],
                "counts_as_usable_evidence": true
            }]
        })];
        let rejected_response = "Remote and hybrid work has increased meeting load across distributed teams. For remote teams, a large-scale field experiment found that prompting attendees to define meeting goals in advance improved meeting effectiveness and reduced meeting overload.";

        let rewritten = maybe_apply_rejected_tool_evidence_fallback(
            &mut workflow,
            "Research current approaches to reducing meeting overload on remote teams. What interventions have stronger evidence or operational support than vague productivity advice?",
            &tools,
            rejected_response,
            rejected_response,
            "final_response_verifier_contract:missing_coverage_lanes=calendar audit, asynchronous communication",
        );

        assert!(rewritten);
        let response = workflow
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(response.contains("define meeting goals in advance"), "{response}");
        assert!(!response.starts_with("My recommendation is to treat the current evidence as insufficient"), "{response}");
    }

    #[test]
    fn rejected_tool_backed_response_salvages_from_excerpt_when_full_text_missing() {
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
            "query_metadata": {
                "required_coverage": {
                    "entities": ["meeting overload", "remote teams"]
                }
            },
            "evidence_pack": [{
                "title": "Meeting-free days metrics",
                "source_domain": "example.test",
                "relevant_extract": "Remote teams with structured calendar analytics recover more focus time when recurring low-value meetings are reduced.",
                "claim_hints": ["Remote teams with structured calendar analytics recover more focus time when recurring low-value meetings are reduced."],
                "counts_as_usable_evidence": true
            }]
        })];
        let rejected_excerpt =
            "Calendar analytics tools that benchmark meeting load by role and quantify focus-time recovery now offer concrete operational support for remote teams, going beyond generic productivity tips.";

        let rewritten = maybe_apply_rejected_tool_evidence_fallback(
            &mut workflow,
            "Research current approaches to reducing meeting overload on remote teams. What interventions have stronger evidence or operational support than vague productivity advice?",
            &tools,
            "",
            rejected_excerpt,
            "final_response_verifier_contract:missing_coverage_lanes=meeting overload",
        );

        assert!(rewritten);
        let response = workflow
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(
            response.contains("Calendar analytics tools that benchmark meeting load by role"),
            "{response}"
        );
        assert!(
            !response.contains("structured calendar analytics recover more focus time"),
            "{response}"
        );
        assert!(!response.starts_with("My recommendation is to treat the current evidence as insufficient"), "{response}");
    }

    #[test]
    fn traceability_reject_rebuilds_from_evidence_before_reusing_excerpt() {
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
            "evidence_pack": [
                {
                    "title": "NRC licensing modernization",
                    "source_domain": "nrc.gov",
                    "relevant_extract": "NRC modernization work has focused on updating licensing review processes for advanced reactors and small modular reactor applications.",
                    "claim_hints": ["NRC modernization work has focused on updating licensing review processes for advanced reactors and small modular reactor applications."],
                    "counts_as_usable_evidence": true
                },
                {
                    "title": "DOE reactor deployment bill",
                    "source_domain": "energy.gov",
                    "relevant_extract": "NRC licensing timelines and environmental review remain nuclear permitting bottlenecks for reactor deployment.",
                    "claim_hints": ["NRC licensing timelines and environmental review remain nuclear permitting bottlenecks for reactor deployment."],
                    "counts_as_usable_evidence": true
                }
            ]
        })];
        let rejected_excerpt =
            "Nuclear permitting reform remains largely stalled as a legislative priority, with no major statutory changes to the NRC's licensing framework or to NEPA environmental review timelines enacted in recent sessions.";

        let rewritten = maybe_apply_rejected_tool_evidence_fallback(
            &mut workflow,
            "Research the current status of nuclear permitting reform in the United States. What has actually changed, and what remains a bottleneck?",
            &tools,
            rejected_excerpt,
            rejected_excerpt,
            "final_response_verifier_contract:answer_units_not_traceable_to_evidence",
        );

        assert!(rewritten);
        let response = workflow
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(
            response.contains("NRC modernization work has focused on updating licensing review processes"),
            "{response}"
        );
        assert!(
            response.contains("NRC licensing timelines and environmental review remain nuclear permitting bottlenecks"),
            "{response}"
        );
        assert!(
            !response.contains("remains largely stalled as a legislative priority"),
            "{response}"
        );
    }

    fn low_information_tool_evidence_fallback_is_repaired_from_reject_excerpt() {
        let mut workflow = json!({
            "response": "My recommendation is to treat the current evidence as insufficient for a direct source-backed conclusion. Coverage state: usable evidence is present for remote teams, meeting overload, asynchronous communication, meeting-free days.",
            "quality_telemetry": {},
            "final_llm_response": {
                "used": true,
                "status": "tool_evidence_fallback_used",
                "diagnostic_reject_reason": "final_response_verifier_contract:missing_coverage_lanes=remote teams, meeting overload",
                "diagnostic_invalid_excerpt": "Microsoft Research published an ACM paper in April 2026 finding that ineffective meetings are pervasive, and that thinking ahead explicitly about meeting goals may improve effectiveness."
            }
        });
        let tools = vec![json!({
            "name": "batch_query",
            "status": "ok",
            "query_metadata": {
                "required_coverage": {
                    "entities": ["remote teams", "meeting overload"]
                }
            },
            "evidence_pack": [{
                "title": "Meeting effectiveness study",
                "source_domain": "example.test",
                "relevant_extract": "Microsoft Research published an ACM paper in April 2026 finding that ineffective meetings are pervasive, and that thinking ahead explicitly about meeting goals may improve effectiveness for remote teams.",
                "claim_hints": ["Microsoft Research published an ACM paper in April 2026 finding that ineffective meetings are pervasive, and that thinking ahead explicitly about meeting goals may improve effectiveness for remote teams."],
                "counts_as_usable_evidence": true
            }]
        })];

        let repaired = maybe_repair_runtime_tool_evidence_fallback_from_reject_excerpt(
            &mut workflow,
            "Research current approaches to reducing meeting overload on remote teams. What interventions have stronger evidence or operational support than vague productivity advice?",
            &tools,
        );

        assert!(repaired);
        let response = workflow
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(response.contains("thinking ahead explicitly about meeting goals may improve effectiveness"), "{response}");
        assert!(!response.starts_with("My recommendation is to treat the current evidence as insufficient"), "{response}");
        assert_eq!(
            workflow
                .pointer("/quality_telemetry/runtime_visible_fallback_source")
                .and_then(Value::as_str),
            Some("tool_evidence_reject_excerpt_repair")
        );
    }

    #[test]
    fn low_information_tool_evidence_fallback_is_repaired_from_bounded_evidence_sketch_without_excerpt() {
        let mut workflow = json!({
            "response": "My recommendation is to treat the current evidence as insufficient for a direct source-backed conclusion. Coverage state: usable evidence is present for remote teams, meeting overload, async communication, meeting-free days.",
            "quality_telemetry": {},
            "final_llm_response": {
                "used": true,
                "status": "tool_evidence_fallback_used",
                "diagnostic_reject_reason": "final_response_verifier_contract:missing_coverage_lanes=remote teams, meeting overload"
            }
        });
        let tools = vec![json!({
            "name": "batch_query",
            "status": "ok",
            "query_metadata": {
                "required_coverage": {
                    "entities": ["remote teams", "meeting overload", "async communication", "meeting-free days"]
                }
            },
            "evidence_pack": [{
                "title": "Distributed team case study",
                "source_domain": "example.test",
                "relevant_extract": "One measured case study from early 2026 documents a distributed product team that cut meeting time by 60% using async boards, with tracked methodology and outcomes.",
                "claim_hints": ["One measured case study from early 2026 documents a distributed product team that cut meeting time by 60% using async boards, with tracked methodology and outcomes."],
                "counts_as_usable_evidence": true
            }]
        })];

        let repaired = maybe_repair_runtime_tool_evidence_fallback_from_reject_excerpt(
            &mut workflow,
            "Research current approaches to reducing meeting overload on remote teams. What interventions have stronger evidence or operational support than vague productivity advice?",
            &tools,
        );

        assert!(repaired);
        let response = workflow
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(
            response.contains("cut meeting time by 60% using async boards"),
            "{response}"
        );
        assert!(
            !response.starts_with("My recommendation is to treat the current evidence as insufficient"),
            "{response}"
        );
    }

    #[test]
    fn title_shell_tool_evidence_fallback_is_repaired_from_evidence_snippet() {
        let mut workflow = json!({
            "response": "How AI is Transforming Scientific Discovery While Keeping Humans at the Center.",
            "quality_telemetry": {},
            "final_llm_response": {
                "used": true,
                "status": "tool_evidence_fallback_used",
                "diagnostic_reject_reason": "final_response_verifier_contract:answer_units_not_traceable_to_evidence",
                "diagnostic_invalid_excerpt": "The recorded evidence for May 2026 includes a Stanford HAI piece on how AI is transforming scientific discovery while keeping humans at the center."
            }
        });
        let tools = vec![json!({
            "name": "batch_query",
            "status": "ok",
            "evidence_refs": [{
                "title": "Agentic AI production trends",
                "locator": "https://example.test/agentic-ai-production",
                "snippet": "Agentic AI in May 2026 has moved past the hype cycle into enterprise deployment, with vendors emphasizing orchestration, security controls, and production reliability."
            }]
        })];

        let title_shell = "How AI is Transforming Scientific Discovery While Keeping Humans at the Center.";
        assert!(response_is_low_information_tool_evidence_fallback(title_shell));
        let salvaged = fallback_evidence_snippet_sentence_from_tools(
            "Give me an update on the AI agentic landscape in May 2026.",
            &tools,
            "final_response_verifier_contract:answer_units_not_traceable_to_evidence",
        );
        assert!(!salvaged.is_empty(), "{salvaged}");

        let repaired = maybe_repair_runtime_tool_evidence_fallback_from_reject_excerpt(
            &mut workflow,
            "Give me an update on the AI agentic landscape in May 2026.",
            &tools,
        );

        assert!(repaired);
        let response = workflow
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(
            response.contains("enterprise deployment")
                || response.contains("production reliability")
                || response.contains("orchestration"),
            "{response}"
        );
        assert_ne!(
            response,
            "How AI is Transforming Scientific Discovery While Keeping Humans at the Center."
        );
    }

    #[test]
    fn verifier_rejects_single_sentence_when_multiple_evidence_units_are_available() {
        let tools = vec![json!({
            "name": "batch_query",
            "status": "ok",
            "evidence_pack": [
                {
                    "title": "Microplastics definition",
                    "relevant_extract": "Microplastics are synthetic polymer particles now detected across multiple human tissue studies.",
                    "counts_as_usable_evidence": true
                },
                {
                    "title": "Microplastics uncertainty",
                    "relevant_extract": "Researchers still distinguish human tissue detection from proven causal health harm because exposure routes, dose, and disease mechanisms remain uncertain.",
                    "counts_as_usable_evidence": true
                },
                {
                    "title": "Microplastics evidence needs",
                    "relevant_extract": "Longitudinal epidemiology and controlled mechanistic work are still needed before firm clinical risk estimates can be made.",
                    "counts_as_usable_evidence": true
                }
            ]
        })];

        let reason = tool_backed_final_verifier_violation_reason(
            "Microplastics are now widely detected in human tissues, but the leap from detection to proven health harm remains unsettled.",
            &tools,
        );

        assert_eq!(
            reason.as_deref(),
            Some("final_response_verifier_contract:answer_underdeveloped_for_available_evidence")
        );
    }

    #[test]
    fn underdeveloped_reject_rebuilds_from_multiple_evidence_units() {
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
            "evidence_pack": [
                {
                    "title": "Microplastics definition",
                    "relevant_extract": "Microplastics are synthetic polymer particles now detected across multiple human tissue studies.",
                    "counts_as_usable_evidence": true
                },
                {
                    "title": "Microplastics uncertainty",
                    "relevant_extract": "Researchers still distinguish human tissue detection from proven causal health harm because exposure routes, dose, and disease mechanisms remain uncertain.",
                    "counts_as_usable_evidence": true
                },
                {
                    "title": "Microplastics evidence needs",
                    "relevant_extract": "Longitudinal epidemiology and controlled mechanistic work are still needed before firm clinical risk estimates can be made.",
                    "counts_as_usable_evidence": true
                }
            ]
        })];
        let rejected =
            "Microplastics are now widely detected in human tissues, but the leap from detection to proven health harm remains unsettled.";

        let rewritten = maybe_apply_rejected_tool_evidence_fallback(
            &mut workflow,
            "What is the state of evidence on microplastics and human health?",
            &tools,
            rejected,
            rejected,
            "final_response_verifier_contract:answer_underdeveloped_for_available_evidence",
        );

        assert!(rewritten);
        let response = workflow
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(response.contains("detected across multiple human tissue studies"), "{response}");
        assert!(response.contains("causal health harm"), "{response}");
        assert!(response.split('.').filter(|part| !part.trim().is_empty()).count() >= 2, "{response}");
    }

    #[test]
    fn underdeveloped_reject_adds_gap_context_when_rebuild_is_thin() {
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
            "query_metadata": {
                "required_coverage": {
                    "entities": ["clinical progress", "biological validation"]
                }
            },
            "evidence_pack": [{
                "title": "AI drug discovery signal",
                "relevant_extract": "AI drug discovery in 2026 shows platform development, but the available material does not verify clinical translation.",
                "counts_as_usable_evidence": true
            }]
        })];
        let rejected =
            "AI drug discovery in 2026 remains characterized by significant computational platform development with limited publicly traceable clinical translation.";

        let rewritten = maybe_apply_rejected_tool_evidence_fallback(
            &mut workflow,
            "Research what is actually happening in AI drug discovery in 2026. Separate platform claims from source-backed clinical or biological progress.",
            &tools,
            rejected,
            rejected,
            "final_response_verifier_contract:answer_units_not_traceable_to_evidence",
        );

        assert!(rewritten);
        let response = workflow
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(
            response.contains("coverage gaps remain")
                || response.contains("not have enough source-backed evidence")
                || response.contains("too thin"),
            "{response}"
        );
        assert_ne!(response, rejected);
        assert!(response.split('.').filter(|part| !part.trim().is_empty()).count() >= 2, "{response}");
    }

    #[test]
    fn title_shell_fallback_is_not_substantive_depth_for_traceability_reject() {
        let source_title_shell =
            "From Radiology to Drug Discovery, Survey Reveals AI Is Delivering Clear Return on Investment in Healthcare. Integrating AI and Machine Learning in Drug Discovery and Development PharmTech.";
        assert!(
            response_is_low_information_tool_evidence_fallback(source_title_shell),
            "{source_title_shell}"
        );
        assert!(
            !fallback_response_has_substantive_depth(source_title_shell),
            "{source_title_shell}"
        );
        let bounded_gap = "My recommendation is to treat the current evidence as insufficient for a direct source-backed conclusion. Coverage gaps still matter for: AI drug discovery, clinical progress, biological validation.";
        assert!(
            response_is_low_information_tool_evidence_fallback(bounded_gap),
            "{bounded_gap}"
        );
        assert!(fallback_response_has_substantive_depth(bounded_gap), "{bounded_gap}");
    }

    #[test]
    fn verifier_rejects_materialization_error_as_visible_answer() {
        let tools = vec![json!({
            "name": "batch_query",
            "status": "ok",
            "evidence_pack": [{
                "title": "Early hip hop political role",
                "relevant_extract": "Early hip-hop is often interpreted as a cultural response to urban disinvestment, policing, and media neglect in late twentieth-century New York.",
                "counts_as_usable_evidence": true
            }]
        })];

        let reason = tool_backed_final_verifier_violation_reason(
            "The search attempt returned only a technical error page, yielding no usable scholarly content on early hip-hop's political interpretation.",
            &tools,
        );

        assert_eq!(
            reason.as_deref(),
            Some("final_response_verifier_contract:materialization_error_substituted_for_answer")
        );
    }

    #[test]
    fn low_information_tool_evidence_fallback_is_repaired_from_evidence_snippet_when_excerpt_is_gap_note() {
        let mut workflow = json!({
            "response": "My recommendation is to treat the current evidence as insufficient for a direct source-backed conclusion. Coverage gaps still matter for: European Union, United States federal government, FedRAMP, StateRAMP, GDPR.",
            "quality_telemetry": {},
            "final_llm_response": {
                "used": true,
                "status": "tool_evidence_fallback_used",
                "diagnostic_reject_reason": "final_response_verifier_contract:status_before_answer",
                "diagnostic_invalid_excerpt": "I don't have sufficient specific evidence in my current retrieval results to give you a concrete, source-backed picture of 2026 data-residency and sovereignty requirements for SaaS sales into Europe and the US public sector."
            }
        });
        let tools = vec![json!({
            "name": "batch_query",
            "status": "ok",
            "evidence_refs": [{
                "title": "EU sovereignty pressure",
                "locator": "https://example.test/eu-sovereignty-pressure",
                "snippet": "European procurement and sovereignty pressure is increasing around localization expectations, transfer controls, and cloud-provider risk assessments for regulated buyers."
            }]
        })];

        let repaired = maybe_repair_runtime_tool_evidence_fallback_from_reject_excerpt(
            &mut workflow,
            "Research data-residency and sovereignty requirements that matter for SaaS buyers in 2026. I want the practical picture for selling into Europe and the US public sector.",
            &tools,
        );

        assert!(repaired);
        let response = workflow
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(
            response.contains("localization expectations")
                || response.contains("transfer controls")
                || response.contains("risk assessments"),
            "{response}"
        );
        assert!(
            !response.starts_with("My recommendation is to treat the current evidence as insufficient"),
            "{response}"
        );
    }

    #[test]
    fn bounded_search_results_do_not_yield_answer_is_preservable_synthesis() {
        assert!(rejected_llm_answer_is_visible_bounded_evidence_response(
            "The search results don't yield a usable, source-backed comparison of Infring against other major agentic frameworks for April 2026."
        ));
    }

    #[test]
    fn mixed_title_shell_and_coverage_fallback_is_repaired_from_reject_excerpt() {
        let mut workflow = json!({
            "response": "Family Guide to Hyde Park: Things to Do with Kids Chicago Parent Published: Tue, 17 Sep 2024 07:00:00 GMT. coverage gaps remain for Lincoln Park, Hyde Park, Wicker Park, South Loop, Loop, Museum Campus, family-friendly, museums.",
            "quality_telemetry": {},
            "final_llm_response": {
                "used": true,
                "status": "tool_evidence_fallback_used",
                "diagnostic_reject_reason": "final_response_verifier_contract:answer_units_not_traceable_to_evidence",
                "diagnostic_invalid_excerpt": "Hyde Park stands out as a solid family-friendly base in Chicago, with Chicago Parent highlighting kid-oriented activities in the area."
            }
        });
        let tools = vec![json!({
            "name": "batch_query",
            "status": "ok",
            "query_metadata": {
                "required_coverage": {
                    "entities": ["Lincoln Park", "Hyde Park", "Wicker Park", "South Loop", "Loop", "Museum Campus"]
                }
            },
            "evidence_refs": [{
                "title": "Chicago Parent Hyde Park guide",
                "locator": "https://www.chicagoparent.com",
                "snippet": "Family Guide to Hyde Park: Things to Do with Kids Chicago Parent Published: Tue, 17 Sep 2024 07:00:00 GMT. Source: Chicago Parent (www.chicagoparent.com)."
            }]
        })];

        assert!(response_is_low_information_tool_evidence_fallback(
            "Family Guide to Hyde Park: Things to Do with Kids Chicago Parent Published: Tue, 17 Sep 2024 07:00:00 GMT. coverage gaps remain for Lincoln Park, Hyde Park, Wicker Park, South Loop, Loop, Museum Campus, family-friendly, museums."
        ));

        let repaired = maybe_repair_runtime_tool_evidence_fallback_from_reject_excerpt(
            &mut workflow,
            "Research family-friendly neighborhoods to stay in Chicago for museums, transit access, and walkability. Compare a few options and tradeoffs.",
            &tools,
        );

        assert!(repaired);
        let response = workflow
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(response.contains("Hyde Park"), "{response}");
        assert!(!response.contains("Published"), "{response}");
        assert!(!response.contains("GMT"), "{response}");
        assert!(response.contains("coverage gaps remain"), "{response}");
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
