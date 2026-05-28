    #[test]
    fn generic_required_noun_phrases_do_not_become_entity_coverage_requirements() {
        let case = json!({
            "prompt": "What is the best agentic framework in 2026? Search first, but do not trust marketing pages blindly. Give me a defensible answer.",
            "required_entities": ["agentic framework"]
        });
        let payload = json!({
            "response": "Based on the retrieved evidence, LangGraph is the most defensible production default in 2026. The current evidence favors it on reliability and cost, while other frameworks look better for narrower use cases or prototypes.",
            "tools": [{
                "name": "web_search",
                "status": "ok",
                "candidate_count": 4,
                "content_rich_candidate_count": 4,
                "claim_hint_count": 3,
                "evidence_refs": [{
                    "title": "Framework comparison",
                    "locator": "https://example.test/framework-comparison",
                    "snippet": "LangGraph, CrewAI, and AutoGen are compared for production tradeoffs in 2026.",
                    "claim_hints": ["LangGraph is the most production-ready default among the compared frameworks."]
                }]
            }]
        });

        let grade = grade_case(&case, &payload, 85, 95);
        assert!(
            grade.coverage_entities.is_empty(),
            "{:#?}",
            grade.coverage_entities
        );
        assert!(
            !grade
                .failures
                .iter()
                .any(|failure| failure.starts_with("entity_coverage_low")),
            "{:#?}",
            grade.failures
        );
    }

    #[test]
    fn lowercase_hyphenated_product_names_still_count_as_specific_entities() {
        let case = json!({
            "prompt": "Compare browser-use with Playwright for browser agent workflows.",
            "required_entities": ["browser-use", "Playwright"]
        });
        let payload = json!({
            "response": "For browser-agent workflows, browser-use is more agent-native while Playwright is stronger for deterministic automation and testability.",
            "tools": [{
                "name": "web_search",
                "status": "ok",
                "candidate_count": 2,
                "materialized_candidate_count": 2,
                "content_rich_candidate_count": 2,
                "claim_hint_count": 2,
                "evidence_refs": [{
                    "title": "Browser automation comparison",
                    "locator": "https://example.test/browser-compare",
                    "snippet": "browser-use and Playwright serve different needs in browser-agent systems.",
                    "claim_hints": ["browser-use is more agent-native while Playwright is more deterministic."]
                }]
            }]
        });

        let grade = grade_case(&case, &payload, 85, 95);
        assert_eq!(
            grade.coverage_entities,
            vec!["browser-use".to_string(), "Playwright".to_string()]
        );
    }

    #[test]
    fn citation_behavior_separates_available_evidence_from_final_citation_signal() {
        let payload = json!({
            "response": "The answer gives a recommendation without naming supporting material.",
            "tools": [{
                "name": "web_search",
                "status": "ok",
                "candidate_count": 1,
                "materialized_candidate_count": 1,
                "content_rich_candidate_count": 1,
                "claim_hint_count": 1,
                "evidence_refs": [{
                    "title": "Usable source",
                    "locator": "https://example.test/source",
                    "snippet": "This source has enough content to be usable evidence for a research answer and includes concrete findings that should be cited.",
                    "claim_hints": ["A concrete source-backed claim."]
                }]
            }]
        });
        let retrieval_quality =
            retrieval_provider_quality(&payload, "research agent workflow evidence");
        let behavior = citation_behavior(
            &payload,
            "The answer gives a recommendation without naming supporting material.",
            &retrieval_quality,
        );
        assert_eq!(
            behavior.get("usable_evidence").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            behavior.get("citation_signal").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            behavior
                .get("synthesis_ignored_citable_evidence")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn citation_behavior_accepts_final_package_source_refs() {
        let payload = json!({
            "response": "The answer gives a recommendation while citations are carried as final-package metadata.",
            "response_finalization": {
                "source_refs": [{
                    "citation_id": "source_1",
                    "title": "Usable source",
                    "locator": "https://example.test/source"
                }]
            },
            "tools": [{
                "name": "web_search",
                "status": "ok",
                "candidate_count": 1,
                "materialized_candidate_count": 1,
                "content_rich_candidate_count": 1,
                "claim_hint_count": 1,
                "evidence_refs": [{
                    "title": "Usable source",
                    "locator": "https://example.test/source",
                    "snippet": "This source has enough content to be usable evidence for a research answer and includes concrete findings that should be cited.",
                    "claim_hints": ["A concrete source-backed claim."]
                }]
            }]
        });
        let retrieval_quality =
            retrieval_provider_quality(&payload, "research agent workflow evidence");
        let behavior = citation_behavior(
            &payload,
            "The answer gives a recommendation while citations are carried as final-package metadata.",
            &retrieval_quality,
        );
        assert_eq!(
            behavior.get("citation_signal").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            behavior
                .get("synthesis_ignored_citable_evidence")
                .and_then(Value::as_bool),
            Some(false)
        );
    }

    #[test]
    fn citation_artifact_summary_carries_final_package_refs() {
        let payload = json!({
            "response_finalization": {
                "source_refs": [{
                    "citation_id": "source_1",
                    "title": "Alpha source",
                    "locator": "https://example.test/alpha",
                    "snippet": "Alpha source-backed finding."
                }],
                "tool_completion": {
                    "evidence_refs": [{
                        "citation_id": "evidence_1",
                        "title": "Beta evidence",
                        "locator": "https://example.test/beta",
                        "snippet": "Beta evidence-backed finding."
                    }]
                }
            }
        });

        let summary = citation_artifact_summary(&payload);
        assert_eq!(
            summary.get("retained_count").and_then(Value::as_u64),
            Some(2)
        );
        let rendered = summary.to_string();
        assert!(rendered.contains("Alpha source"), "{rendered}");
        assert!(rendered.contains("Beta evidence"), "{rendered}");
    }

    #[test]
    fn answer_unit_alignment_flags_untraced_specific_answer_unit() {
        let payload = json!({
            "response": "Alpha launched Beta in 2026. Alpha also launched PhantomX in 2026.",
            "tools": [{
                "name": "web_search",
                "status": "ok",
                "candidate_count": 1,
                "materialized_candidate_count": 1,
                "content_rich_candidate_count": 1,
                "claim_hint_count": 1,
                "evidence_refs": [{
                    "title": "Alpha launched Beta",
                    "locator": "https://example.test/alpha-beta",
                    "snippet": "Alpha launched Beta in 2026 after a public release.",
                    "claim_hints": ["Alpha launched Beta in 2026."]
                }]
            }]
        });
        let retrieval_quality = retrieval_provider_quality(&payload, "alpha beta launch");
        let alignment = answer_unit_evidence_alignment(
            &payload,
            "Alpha launched Beta in 2026. Alpha also launched PhantomX in 2026.",
            &retrieval_quality,
        );

        assert_eq!(
            alignment.get("evaluated").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(alignment.get("pass").and_then(Value::as_bool), Some(false));
        assert_eq!(
            alignment.get("top_blocker").and_then(Value::as_str),
            Some("unsupported_answer_units")
        );
        assert!(alignment.to_string().contains("phantomx"), "{}", alignment);
    }

    #[test]
    fn answer_unit_alignment_allows_explicitly_hedged_gap_units() {
        let payload = json!({
            "response": "Alpha launched Beta in 2026. Alpha may also be associated with PhantomX, but current evidence does not confirm it.",
            "tools": [{
                "name": "web_search",
                "status": "ok",
                "candidate_count": 1,
                "materialized_candidate_count": 1,
                "content_rich_candidate_count": 1,
                "claim_hint_count": 1,
                "evidence_refs": [{
                    "title": "Alpha launched Beta",
                    "locator": "https://example.test/alpha-beta",
                    "snippet": "Alpha launched Beta in 2026 after a public release.",
                    "claim_hints": ["Alpha launched Beta in 2026."]
                }]
            }]
        });
        let retrieval_quality = retrieval_provider_quality(&payload, "alpha beta launch");
        let alignment = answer_unit_evidence_alignment(
            &payload,
            "Alpha launched Beta in 2026. Alpha may also be associated with PhantomX, but current evidence does not confirm it.",
            &retrieval_quality,
        );

        assert_eq!(
            alignment.get("evaluated").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(alignment.get("pass").and_then(Value::as_bool), Some(true));
        assert_eq!(
            alignment
                .get("unsupported_unit_count")
                .and_then(Value::as_u64),
            Some(0)
        );
    }

    #[test]
    fn answer_unit_alignment_still_evaluates_when_retrieval_is_weak() {
        let payload = json!({
            "response": "Alpha launched Beta in 2026. Alpha also launched PhantomX in 2026.",
            "response_finalization": {
                "source_refs": [{
                    "title": "Alpha launched Beta",
                    "locator": "https://example.test/alpha-beta",
                    "snippet": "Alpha launched Beta in 2026 after a public release."
                }]
            }
        });
        let retrieval_quality = json!({
            "usable_evidence": false,
            "allows_excellent": false,
            "status": "low_signal"
        });
        let alignment = answer_unit_evidence_alignment(
            &payload,
            "Alpha launched Beta in 2026. Alpha also launched PhantomX in 2026.",
            &retrieval_quality,
        );

        assert_eq!(
            alignment.get("evaluated").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            alignment.get("usable_evidence").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(alignment.get("pass").and_then(Value::as_bool), Some(false));
        assert_eq!(
            alignment.get("top_blocker").and_then(Value::as_str),
            Some("unsupported_answer_units")
        );
    }

    #[test]
    fn answer_unit_alignment_allows_source_backed_refusal_scope_terms() {
        let payload = json!({
            "response": "I can't give you a source-backed comparison of Dyson, Roborock, and iRobot for pet hair in apartments. The search returned only headline-level roundups and missing entity details.",
            "pending_tool_request": {
                "input": {
                    "query": "Compare Dyson, Roborock, and iRobot for pet hair in apartments",
                    "keywords": ["Dyson", "Roborock", "iRobot", "pet hair", "apartments"],
                    "required_coverage": {
                        "entities": ["Dyson", "Roborock", "iRobot"],
                        "facets": ["pet hair", "apartments"]
                    }
                }
            },
            "tools": [{
                "name": "web_search",
                "status": "ok",
                "candidate_count": 3,
                "materialized_candidate_count": 1,
                "content_rich_candidate_count": 1,
                "claim_hint_count": 0,
                "evidence_refs": [{
                    "title": "Best robot vacuums for pet hair",
                    "locator": "https://example.test/pet-hair-vacuums",
                    "snippet": "Headline-level roundup with no direct comparison of the requested brands."
                }]
            }]
        });
        let retrieval_quality = retrieval_provider_quality(&payload, "robot vacuum pet hair");
        let alignment = answer_unit_evidence_alignment(
            &payload,
            "I can't give you a source-backed comparison of Dyson, Roborock, and iRobot for pet hair in apartments. The search returned only headline-level roundups and missing entity details.",
            &retrieval_quality,
        );

        assert_eq!(
            alignment.get("evaluated").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(alignment.get("pass").and_then(Value::as_bool), Some(true));
        assert_eq!(
            alignment
                .get("unsupported_unit_count")
                .and_then(Value::as_u64),
            Some(0)
        );
    }

    #[test]
    fn answer_unit_usefulness_flags_admin_facts_when_prompt_needs_substantive_answer_units() {
        let case = json!({
            "prompt": "What are some scientific breakthroughs 2026?",
            "expected_gate_path": {
                "gate_1": "tool_required",
                "gate_2": "web_research",
                "gate_3": "batch_query",
                "gate_4_required_fields": ["query", "aperture"]
            }
        });
        let payload = json!({
            "response": "Based on the retrieved evidence, here are concrete 2026 scientific developments. Particle physics: CERN reported the first observation of a new meson state in 2026. Nobel Prize cycle: The 2026 Nobel Prize announcements are scheduled for October. Peace Prize nominations closed with 287 candidates.",
            "pending_tool_request": {
                "status": "executed",
                "selected_tool_family": "web_research",
                "tool_name": "batch_query",
                "tool_key": "batch_query",
                "input": {
                    "query": "scientific breakthroughs 2026",
                    "aperture": "medium"
                }
            },
            "tools": [{
                "name": "batch_query",
                "status": "ok",
                "candidate_count": 3,
                "materialized_candidate_count": 3,
                "content_rich_candidate_count": 3,
                "claim_hint_count": 3,
                "evidence_refs": [
                    {
                        "title": "CERN meson observation",
                        "locator": "https://example.test/cern-meson",
                        "snippet": "CERN reported the first observation of a new meson state in 2026.",
                        "claim_hints": ["CERN reported the first observation of a new meson state in 2026."]
                    },
                    {
                        "title": "Nobel Prize schedule",
                        "locator": "https://example.test/nobel-schedule",
                        "snippet": "The 2026 Nobel Prize announcements are scheduled for October.",
                        "claim_hints": ["The 2026 Nobel Prize announcements are scheduled for October."]
                    },
                    {
                        "title": "Peace Prize nominations",
                        "locator": "https://example.test/peace-nominations",
                        "snippet": "Peace Prize nominations closed with 287 candidates.",
                        "claim_hints": ["Peace Prize nominations closed with 287 candidates."]
                    }
                ]
            }]
        });

        let grade = grade_case(&case, &payload, 85, 95);
        assert!(!grade.pass, "{:?}", grade.failures);
        assert!(grade
            .failures
            .contains(&"answer_units_not_useful_for_prompt".to_string()));
        assert_eq!(
            grade
                .answer_unit_usefulness
                .get("top_blocker")
                .and_then(Value::as_str),
            Some("process_metadata_units_overrepresented")
        );
    }

    #[test]
    fn answer_unit_usefulness_allows_schedule_facts_when_prompt_asks_for_schedule() {
        let retrieval_quality = json!({
            "usable_evidence": true,
            "status": "usable"
        });
        let usefulness = answer_unit_usefulness_for_prompt(
            &normalize_for_compare("When are the 2026 Nobel Prize announcements scheduled?"),
            "The 2026 Nobel Prize announcements are scheduled for October.",
            &retrieval_quality,
        );

        assert_eq!(usefulness.get("pass").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn answer_unit_usefulness_flags_transport_fallback_without_prompt_useful_units() {
        let retrieval_quality = json!({
            "usable_evidence": false,
            "status": "transport_failure"
        });
        let usefulness = answer_unit_usefulness_for_prompt(
            &normalize_for_compare(
                "Find recent benchmarks comparing agent frameworks. If the benchmark evidence is weak, explain why and suggest a practical evaluation plan.",
            ),
            "The live dashboard request timed out before the workflow produced a final answer. This is a transport failure, not a research result.",
            &retrieval_quality,
        );

        assert_eq!(usefulness.get("evaluated").and_then(Value::as_bool), Some(true));
        assert_eq!(usefulness.get("pass").and_then(Value::as_bool), Some(false));
        assert_eq!(
            usefulness.get("top_blocker").and_then(Value::as_str),
            Some("direct_answer_units_missing")
        );
        assert_eq!(
            usefulness
                .get("direct_useful_units")
                .and_then(Value::as_u64),
            Some(0)
        );
    }

    #[test]
    fn answer_unit_usefulness_allows_time_scoped_update_units_with_may_as_month() {
        let retrieval_quality = json!({
            "usable_evidence": true,
            "status": "usable"
        });
        let usefulness = answer_unit_usefulness_for_prompt(
            &normalize_for_compare("Give me an update on the AI agentic landscape in May 2026."),
            "By late May 2026, the agentic AI landscape is marked by a flurry of major model releases rather than framework or protocol breakthroughs. A practitioner roundup from May 27 frames the biggest agentic AI launches as Anthropic's Mythos, Gemini 3.5 Flash, and Qwen 3. The recorded evidence surfaces model-release activity but does not yet materialize the framework, protocol, and platform coverage needed for a full landscape update.",
            &retrieval_quality,
        );

        assert_eq!(usefulness.get("pass").and_then(Value::as_bool), Some(true));
        assert!(
            usefulness
                .get("direct_useful_units")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                >= 2,
            "{:#?}",
            usefulness
        );
        assert_ne!(
            usefulness.get("top_blocker").and_then(Value::as_str),
            Some("direct_answer_units_missing"),
            "{:#?}",
            usefulness
        );
    }

    #[test]
    fn response_truncation_detector_flags_incomplete_table_tail() {
        assert!(response_looks_truncated_or_incomplete(
            "Comparison:\n| Dimension | Best signal |\n| SDK ecosystem | Tavily (AWS"
        ));
        assert!(!response_looks_truncated_or_incomplete(
            "Comparison:\n| Dimension | Best signal |\n| SDK ecosystem | Tavily (AWS partnership). |"
        ));
    }

    #[test]
    fn excellent_diagnostics_call_out_missing_final_citation_signal() {
        let case = json!({
            "prompt": "Compare Alpha and Beta for production use.",
            "expected_gate_path": {
                "gate_1": "tool_required",
                "gate_2": "web_research",
                "gate_3": "web_search",
                "gate_4_required_fields": ["query", "aperture"]
            },
            "required_entities": ["Alpha", "Beta"]
        });
        let payload = json!({
            "response": "Alpha is the better default for production when reliability matters, while Beta is more useful for exploratory workflows. Alpha has stronger deployment and maintenance tradeoffs; Beta remains useful when speed of experimentation matters. The practical recommendation is to use Alpha for steady production and Beta for prototypes.",
            "pending_tool_request": {
                "status": "pending_confirmation",
                "selected_tool_family": "web_research",
                "selected_tool_label": "Web search",
                "tool_name": "web_search",
                "tool_key": "web_search",
                "input": {
                    "query": "Alpha Beta production comparison",
                    "aperture": "web"
                }
            },
            "tools": [{
                "name": "web_search",
                "status": "ok",
                "candidate_count": 2,
                "materialized_candidate_count": 2,
                "content_rich_candidate_count": 2,
                "claim_hint_count": 2,
                "evidence_refs": [
                    {
                        "title": "Alpha and Beta production comparison",
                        "locator": "https://example.test/alpha-beta-production",
                        "snippet": "A substantive source comparing Alpha and Beta for reliability, deployment, maintenance, and experimentation tradeoffs.",
                        "claim_hints": ["Alpha is better suited to production reliability."]
                    },
                    {
                        "title": "Alpha and Beta experimentation comparison",
                        "locator": "https://example.test/alpha-beta-experimentation",
                        "snippet": "A second substantive source comparing Alpha and Beta for experimentation speed and prototype workflows.",
                        "claim_hints": ["Beta is more useful for exploratory workflows."]
                    }
                ]
            }]
        });

        let grade = grade_case(&case, &payload, 85, 95);
        assert!(grade.pass, "{:?}", grade.failures);
        assert!(!grade.excellent);
        assert_eq!(
            grade
                .excellent_diagnostics
                .pointer("/subgates/excellent_3_citations_used_in_final")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            grade
                .excellent_diagnostics
                .get("top_blocker")
                .and_then(Value::as_str),
            Some("missing_final_citation_or_source_signal")
        );
    }
