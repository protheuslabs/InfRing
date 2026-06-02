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
        assert!(answer_unit_alignment_hard_failure(&alignment), "{alignment}");
    }

    #[test]
    fn answer_unit_alignment_minor_traceability_warnings_do_not_hard_fail() {
        let alignment = json!({
            "evaluated": true,
            "pass": false,
            "usable_evidence": true,
            "term_support_rate": 0.66,
            "unsupported_unit_count": 3,
            "unsupported_units": [{
                "unsupported_terms": ["active", "enforced", "allocated"]
            }]
        });

        assert!(
            !answer_unit_alignment_hard_failure(&alignment),
            "{alignment}"
        );
    }

    #[test]
    fn answer_unit_alignment_single_high_support_gap_does_not_hard_fail() {
        let alignment = json!({
            "evaluated": true,
            "pass": false,
            "usable_evidence": true,
            "term_support_rate": 0.91,
            "unsupported_unit_count": 1,
            "unsupported_units": [{
                "unsupported_terms": ["viability"]
            }]
        });

        assert!(
            !answer_unit_alignment_hard_failure(&alignment),
            "{alignment}"
        );
    }

    #[test]
    fn answer_unit_alignment_single_low_risk_wording_gap_is_not_significant() {
        let significant = answer_unit_unsupported_is_significant(
            &normalize_for_compare(
                "Consolidate to one central cloud storage solution.",
            ),
            &[],
            &[],
            &["consolidate".to_string()],
        );

        assert!(!significant);
    }

    #[test]
    fn answer_unit_specific_terms_ignore_connective_words() {
        let terms = answer_unit_specific_terms(
            "Briggs & Riley - Along with Away, it excelled in Consumer Reports data.",
        );

        assert!(!terms.iter().any(|term| term == "along"), "{terms:?}");
    }

    #[test]
    fn answer_unit_alignment_supports_related_adjective_forms() {
        let evidence = vec![normalize_for_compare(
            "Our evidence for witchcraft in Europe comes almost exclusively from hostile sources.",
        )];

        assert!(evidence_texts_support_term(&evidence, "european"));
    }

    #[test]
    fn answer_unit_alignment_generic_action_verbs_do_not_hard_fail() {
        let alignment = json!({
            "evaluated": true,
            "pass": false,
            "usable_evidence": true,
            "term_support_rate": 0.25,
            "unsupported_unit_count": 1,
            "unsupported_units": [{
                "unsupported_terms": ["securing", "implementing", "establishing", "wherever"]
            }]
        });

        assert!(
            !answer_unit_alignment_hard_failure(&alignment),
            "{alignment}"
        );
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
    fn direct_evidence_claim_relevance_uses_linked_source_context() {
        let payload = json!({
            "evidence_claims": [{
                "claim": "Efficiency progress is real in controlled lab settings.",
                "source_ref": "source_a",
                "locator": "fixture://source-a",
                "confidence": "usable"
            }],
            "evidence_pack_quality": {
                "status": "usable",
                "usable_count": 1,
                "content_rich_item_count": 1
            },
            "tools": [{
                "name": "batch_query",
                "status": "ok",
                "candidate_count": 1,
                "materialized_candidate_count": 1,
                "content_rich_candidate_count": 1,
                "claim_hint_count": 1,
                "evidence_pack": [{
                    "id": "source_a",
                    "locator": "fixture://source-a",
                    "relevant_extract": "Perovskite-silicon tandem solar cells have higher lab conversion efficiency than silicon-only cells, but the evidence remains a controlled-lab result.",
                    "claim_hints": ["Efficiency progress is real in controlled lab settings."]
                }],
                "evidence_refs": [{
                    "id": "source_a",
                    "locator": "fixture://source-a",
                    "title": "Tandem cell lab note"
                }]
            }]
        });

        let quality = retrieval_provider_quality(
            &payload,
            &normalize_for_compare("Research progress on perovskite-silicon tandem solar cells."),
        );

        assert_eq!(
            quality.get("status").and_then(Value::as_str),
            Some("usable"),
            "{:#?}",
            quality
        );
        assert_eq!(
            quality
                .pointer("/prompt_relevance/topic_relevant_evidence")
                .and_then(Value::as_bool),
            Some(true),
            "{:#?}",
            quality
        );
    }

    #[test]
    fn direct_evidence_claim_relevance_matches_short_acronym_and_adjective_family() {
        let payload = json!({
            "evidence_claims": [{
                "claim": "Governance and reliability are central buyer concerns.",
                "source_ref": "source_a",
                "locator": "fixture://source-a",
                "confidence": "usable"
            }],
            "evidence_pack_quality": {
                "status": "usable",
                "usable_count": 1,
                "content_rich_item_count": 1
            },
            "tools": [{
                "name": "batch_query",
                "status": "ok",
                "candidate_count": 1,
                "materialized_candidate_count": 1,
                "content_rich_candidate_count": 1,
                "claim_hint_count": 1,
                "evidence_pack": [{
                    "id": "source_a",
                    "locator": "fixture://source-a",
                    "relevant_extract": "The enterprise-ai brief says agent adoption is moving toward constrained workflows with approvals, audit logs, and human review.",
                    "claim_hints": ["Agentic capability without boundaries is not the current enterprise norm."]
                }]
            }]
        });

        let quality = retrieval_provider_quality(
            &payload,
            &normalize_for_compare("Give me an update on the AI agentic landscape."),
        );

        assert_eq!(
            quality.get("status").and_then(Value::as_str),
            Some("usable"),
            "{:#?}",
            quality
        );
    }

    #[test]
    fn answer_alignment_ignores_sentence_initial_generic_action_words() {
        let payload = json!({
            "response": "Prioritize enacted laws over headlines. Pay special attention to high-risk automated decisions. Specific harms include deepfakes and child safety.",
            "tools": [{
                "name": "batch_query",
                "status": "ok",
                "candidate_count": 1,
                "materialized_candidate_count": 1,
                "content_rich_candidate_count": 1,
                "claim_hint_count": 1,
                "evidence_refs": [{
                    "title": "State AI legislation tracker",
                    "locator": "https://example.test/state-ai",
                    "snippet": "The tracker says startups should monitor enacted laws, high-risk automated decisions, deepfakes, and child safety rules rather than relying on legislative headlines.",
                    "claim_hints": ["Bill status and enacted-law status must be kept separate."]
                }]
            }]
        });
        let retrieval_quality = retrieval_provider_quality(&payload, "state ai regulation");
        let alignment = answer_unit_evidence_alignment(
            &payload,
            "Prioritize enacted laws over headlines. Pay special attention to high-risk automated decisions. Specific harms include deepfakes and child safety.",
            &retrieval_quality,
        );

        assert_eq!(alignment.get("pass").and_then(Value::as_bool), Some(true));
        assert_eq!(
            alignment
                .get("unsupported_unit_count")
                .and_then(Value::as_u64),
            Some(0),
            "{:#?}",
            alignment
        );
    }

    #[test]
    fn answer_alignment_ignores_checklist_verbs_as_claim_terms() {
        let payload = json!({
            "response": "The highest-priority areas are: - Maintaining an accurate data inventory - Ensuring privacy notices match product practices - Putting vendor terms in place - Offering clear opt-outs - Keeping request records - Use durable file formats - Continue preserving originals - Look for maintenance friction - Factor in location - Set a repeatable workflow - Save ordinary file formats.",
            "tools": [{
                "name": "batch_query",
                "status": "ok",
                "candidate_count": 1,
                "materialized_candidate_count": 1,
                "content_rich_candidate_count": 1,
                "claim_hint_count": 1,
                "evidence_refs": [{
                    "title": "Privacy compliance checklist",
                    "locator": "https://example.test/privacy",
                    "snippet": "Mid-size internet businesses should maintain a data inventory, align privacy notices with actual practices, use vendor data-processing terms, offer opt-outs, and keep records of consumer requests.",
                    "claim_hints": ["Operational readiness matters more than tracking every statutory nuance."]
                }]
            }]
        });
        let retrieval_quality = retrieval_provider_quality(&payload, "consumer privacy law");
        let alignment = answer_unit_evidence_alignment(
            &payload,
            "The highest-priority areas are: - Maintaining an accurate data inventory - Ensuring privacy notices match product practices - Putting vendor terms in place - Offering clear opt-outs - Keeping request records - Use durable file formats - Continue preserving originals - Look for maintenance friction - Factor in location - Set a repeatable workflow - Save ordinary file formats.",
            &retrieval_quality,
        );

        assert_eq!(alignment.get("pass").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn answer_alignment_ignores_list_markers_and_scaffold_adjectives() {
        let terms = answer_unit_specific_terms(
            "Your immediate priorities should be: 1. Economic dimensions: Unresolved conflicts over land and labor shaped the collapse.",
        );

        assert!(!terms.contains(&"1".to_string()), "{terms:?}");
        assert!(!terms.contains(&"unresolved".to_string()), "{terms:?}");
        assert!(!terms.contains(&"unresolv".to_string()), "{terms:?}");
    }

    #[test]
    fn answer_alignment_does_not_treat_common_country_scope_words_as_claim_terms() {
        let terms = answer_unit_specific_terms(
            "The women’s suffrage movement in the United States contained distinct strategic camps.",
        );

        assert!(!terms.contains(&"united".to_string()), "{terms:?}");
        assert!(!terms.contains(&"states".to_string()), "{terms:?}");
    }

    #[test]
    fn answer_alignment_ignores_checklist_and_interface_scaffold_terms() {
        let terms = answer_unit_specific_terms(
            "Lock in the move date, begin address changes, initiate postal forwarding, evaluate bids, insist on contract terms, and use API-driven internal tools. During the same period, whichever lane you use, measure each class separately. **Target:** the easiest option. Named the strongest pick. **Why it is on the list:** a useful fit. Ueno offers older Tokyo atmosphere and value.",
        );

        assert!(!terms.contains(&"lock".to_string()), "{terms:?}");
        assert!(!terms.contains(&"during".to_string()), "{terms:?}");
        assert!(!terms.contains(&"begin".to_string()), "{terms:?}");
        assert!(!terms.contains(&"initiat".to_string()), "{terms:?}");
        assert!(!terms.contains(&"initiate".to_string()), "{terms:?}");
        assert!(!terms.contains(&"evaluat".to_string()), "{terms:?}");
        assert!(!terms.contains(&"evaluate".to_string()), "{terms:?}");
        assert!(!terms.contains(&"insist".to_string()), "{terms:?}");
        assert!(!terms.contains(&"api".to_string()), "{terms:?}");
        assert!(!terms.contains(&"whichever".to_string()), "{terms:?}");
        assert!(!terms.contains(&"target".to_string()), "{terms:?}");
        assert!(!terms.contains(&"want".to_string()), "{terms:?}");
        assert!(!terms.contains(&"named".to_string()), "{terms:?}");
        assert!(!terms.contains(&"why".to_string()), "{terms:?}");
        assert!(!terms.contains(&"offer".to_string()), "{terms:?}");
        assert!(!terms.contains(&"offers".to_string()), "{terms:?}");
    }

    #[test]
    fn answer_alignment_keeps_target_as_named_entity_when_not_a_label() {
        let terms = answer_unit_specific_terms(
            "The retailer Target announced new pickup policies for urban stores.",
        );

        assert!(terms.contains(&"target".to_string()), "{terms:?}");
    }

    #[test]
    fn answer_alignment_ignores_acronym_expansion_scaffold_terms() {
        let terms = answer_unit_specific_terms(
            "Employer EAPs: Employee Assistance Programs can provide short-term counseling.",
        );

        assert!(terms.contains(&"eaps".to_string()), "{terms:?}");
        assert!(!terms.contains(&"assistance".to_string()), "{terms:?}");
        assert!(!terms.contains(&"programs".to_string()), "{terms:?}");
    }

    #[test]
    fn answer_alignment_ignores_outline_and_pronoun_scaffold_terms() {
        let terms = answer_unit_specific_terms(
            "Inclusion criteria: They should add supported items rather than unsupported detail.",
        );

        assert!(!terms.contains(&"inclusion".to_string()), "{terms:?}");
        assert!(!terms.contains(&"they".to_string()), "{terms:?}");
        assert!(!terms.contains(&"add".to_string()), "{terms:?}");
        assert!(!terms.contains(&"rather".to_string()), "{terms:?}");
    }

    #[test]
    fn answer_alignment_ignores_generic_words_inside_expanded_law_names() {
        let terms = answer_unit_specific_terms(
            "The EU Digital Services Act includes dedicated protections for minors.",
        );

        assert!(!terms.contains(&"digital".to_string()), "{terms:?}");
        assert!(!terms.contains(&"services".to_string()), "{terms:?}");
    }

    #[test]
    fn answer_alignment_ignores_heading_and_scope_words() {
        let terms = answer_unit_specific_terms(
            "LNG Market Dynamics: Global LNG markets are influenced by export terminal capacity.",
        );

        assert!(!terms.contains(&"dynamics".to_string()), "{terms:?}");
        assert!(!terms.contains(&"global".to_string()), "{terms:?}");
    }

    #[test]
    fn answer_alignment_ignores_generic_actor_and_scope_adjectives() {
        let terms = answer_unit_specific_terms(
            "Researchers and scholars reported that Western institutions were highly influential in the policy context.",
        );

        assert!(!terms.contains(&"researcher".to_string()), "{terms:?}");
        assert!(!terms.contains(&"researchers".to_string()), "{terms:?}");
        assert!(!terms.contains(&"scholar".to_string()), "{terms:?}");
        assert!(!terms.contains(&"scholars".to_string()), "{terms:?}");
        assert!(!terms.contains(&"western".to_string()), "{terms:?}");
        assert!(!terms.contains(&"highly".to_string()), "{terms:?}");
    }

    #[test]
    fn answer_alignment_expands_common_scope_initialisms() {
        let payload = json!({
            "response": "For a first-time visitor to New York City, the Upper West Side is a useful base.",
            "pending_tool_request": {
                "input": {
                    "query": "Compare NYC neighborhoods for a first-time visitor."
                }
            },
            "tools": [{
                "name": "batch_query",
                "status": "ok",
                "candidate_count": 3,
                "content_rich_candidate_count": 3,
                "claim_hint_count": 3,
                "evidence_refs": [{
                    "title": "NYC first-time visitor neighborhoods",
                    "locator": "https://example.test/nyc",
                    "snippet": "Upper West Side works well for museums and subway access while avoiding Times Square.",
                    "claim_hints": ["Upper West Side is a useful first-time visitor base."]
                }]
            }]
        });
        let retrieval_quality = retrieval_provider_quality(&payload, "nyc neighborhoods");
        let alignment = answer_unit_evidence_alignment(
            &payload,
            "For a first-time visitor to New York City, the Upper West Side is a useful base.",
            &retrieval_quality,
        );

        assert_eq!(alignment.get("pass").and_then(Value::as_bool), Some(true));
        assert_eq!(
            alignment
                .get("top_blocker")
                .and_then(Value::as_str),
            Some("none")
        );
    }

    #[test]
    fn answer_alignment_ignores_generic_action_verbs_as_claim_terms() {
        let payload = json!({
            "response": "What looks supported: Individualized screening and management tailors care around symptom flares. Pacing specifically supports patients whose symptoms worsen after exertion. Where people are overgeneralizing: Prescribing uniform exercise, pushing fixed escalation, applying aggressive graded exercise, or treating patient reports as trial data all overstate the evidence.",
            "tools": [{
                "name": "batch_query",
                "status": "ok",
                "candidate_count": 3,
                "materialized_candidate_count": 3,
                "content_rich_candidate_count": 3,
                "claim_hint_count": 3,
                "evidence_refs": [{
                    "title": "Long COVID clinical management review",
                    "locator": "https://example.test/long-covid",
                    "snippet": "The review supports individualized symptom management, screening for post-exertional symptom exacerbation, rehabilitation matched to tolerance, and pacing for patients whose symptoms worsen after exertion. It warns against one-size-fits-all exercise prescriptions.",
                    "claim_hints": ["Pacing is supported for patients with post-exertional symptom worsening."]
                }, {
                    "title": "Rehabilitation caution statement",
                    "locator": "https://example.test/rehab",
                    "snippet": "The guidance distinguishes graded rehabilitation for some deconditioned patients from aggressive graded exercise for patients with post-exertional malaise. Programs should monitor setbacks and stop escalation when symptoms flare, rather than forcing progression on a fixed schedule.",
                    "claim_hints": ["Fixed escalation can be harmful for some patients."]
                }, {
                    "title": "Patient advocacy evidence synthesis",
                    "locator": "https://example.test/patient-reports",
                    "snippet": "Patient reports highlight relapses after overexertion and practical management harms. Anecdotal reports are not trials but are important for identifying harms and practical management needs.",
                    "claim_hints": ["Patient experience should inform caution without replacing clinical evidence."]
                }]
            }]
        });
        let retrieval_quality = retrieval_provider_quality(&payload, "long covid pacing exercise");
        let alignment = answer_unit_evidence_alignment(
            &payload,
            "What looks supported: Individualized screening and management tailors care around symptom flares. Pacing specifically supports patients whose symptoms worsen after exertion. Where people are overgeneralizing: Prescribing uniform exercise, pushing fixed escalation, applying aggressive graded exercise, or treating patient reports as trial data all overstate the evidence.",
            &retrieval_quality,
        );

        assert_eq!(alignment.get("pass").and_then(Value::as_bool), Some(true), "{alignment}");
    }

    #[test]
    fn answer_alignment_treats_negative_timeline_caveats_as_hedged() {
        let payload = json!({
            "response": "These accounts do not claim a simple, direct pipeline from the 1920s to the 1950s and 1960s.",
            "tools": [{
                "name": "batch_query",
                "status": "ok",
                "candidate_count": 1,
                "materialized_candidate_count": 1,
                "content_rich_candidate_count": 1,
                "claim_hint_count": 1,
                "evidence_refs": [{
                    "title": "Civil-rights movement continuity essay",
                    "locator": "https://example.test/harlem",
                    "snippet": "The essay links Harlem Renaissance networks, publications, patronage, and public intellectual life to later civil-rights organizing and Black political consciousness. It avoids claiming a simple direct pipeline, instead describing a cultural and institutional foundation that later movements drew from.",
                    "claim_hints": ["A useful answer should avoid a simplistic cause-effect chain."]
                }]
            }]
        });
        let retrieval_quality = retrieval_provider_quality(&payload, "harlem renaissance political impact");
        let alignment = answer_unit_evidence_alignment(
            &payload,
            "These accounts do not claim a simple, direct pipeline from the 1920s to the 1950s and 1960s.",
            &retrieval_quality,
        );

        assert_eq!(alignment.get("pass").and_then(Value::as_bool), Some(true), "{alignment}");
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
