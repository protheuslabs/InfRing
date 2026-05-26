    #[test]
    fn tool_evidence_fallback_returns_bounded_user_visible_answer() {
        let response = fallback_final_response_from_tool_evidence(
            "Compare LangGraph, CrewAI, AutoGen, and OpenHands for agentic research workflows.",
            &[json!({
                "name": "batch_query",
                "status": "ok",
                "is_error": false,
                "result": "Key findings: AutoGen - Microsoft Research: AutoGen is an open-source framework for building AI agents.",
                "evidence_refs": [
                    {
                        "title": "LangGraph overview",
                        "snippet": "LangGraph focuses on long-running stateful agent workflows."
                    },
                    {
                        "title": "CrewAI docs",
                        "snippet": "CrewAI emphasizes role-based multi-agent orchestration."
                    },
                    {
                        "title": "OpenHands docs",
                        "snippet": "OpenHands is oriented toward software-development task execution."
                    }
                ]
            })],
        );
        assert!(response.starts_with("LangGraph focuses on"), "{response}");
        assert!(response.contains("LangGraph focuses on"), "{response}");
        assert!(!response.contains("Sources:"), "{response}");
        assert!(!response.contains("tool_evidence_runtime_fallback_suppressed"));
        assert!(!response.contains("Recorded evidence so far"));
    }

    #[test]
    fn tool_evidence_fallback_uses_answer_ready_evidence_packets() {
        let response = fallback_final_response_from_tool_evidence(
            "What are some scientific breakthroughs in 2026?",
            &[json!({
                "name": "batch_query",
                "status": "ok",
                "is_error": false,
                "evidence_pack": [{
                    "pack_version": "evidence_pack_v1",
                    "source_kind": "research_news",
                    "source_class": "scholarly_or_research",
                    "title": "Battery milestone report",
                    "locator": "https://example.test/battery-2026",
                    "source_domain": "example.test",
                    "relevant_extract": "A research group reported a solid-state battery chemistry milestone with improved cycle stability in 2026.",
                    "why_relevant_to_query": "It is a source-backed example of a scientific breakthrough reported in the requested year.",
                    "claim_hints": [
                        "A 2026 solid-state battery chemistry milestone improved cycle stability."
                    ],
                    "counts_as_usable_evidence": true
                }]
            })],
        );
        assert!(
            response.starts_with("A 2026 solid-state battery chemistry milestone"),
            "{response}"
        );
        assert!(
            response.contains("solid-state battery chemistry milestone"),
            "{response}"
        );
        assert!(!response.contains("Sources:"), "{response}");
        assert!(
            !response.contains("strongest supported answer"),
            "{response}"
        );
        assert!(!response.contains("Here's what I found"), "{response}");
        assert!(!response.contains("Recorded evidence so far"), "{response}");
        assert!(!response.contains("From web retrieval"), "{response}");
    }

    #[test]
    fn tool_evidence_fallback_filters_process_metadata_when_goal_needs_substantive_units() {
        let response = fallback_final_response_from_tool_evidence(
            "What are some scientific breakthroughs in 2026?",
            &[json!({
                "name": "batch_query",
                "status": "ok",
                "is_error": false,
                "evidence_pack": [
                    {
                        "title": "Nobel Prize schedule",
                        "locator": "https://example.test/nobel-schedule",
                        "source_domain": "example.test",
                        "relevant_extract": "The 2026 Nobel Prize announcements are scheduled for October.",
                        "claim_hints": ["The 2026 Nobel Prize announcements are scheduled for October."],
                        "counts_as_usable_evidence": true
                    },
                    {
                        "title": "Battery milestone report",
                        "locator": "https://example.test/battery-2026",
                        "source_domain": "example.test",
                        "relevant_extract": "A research group reported a solid-state battery chemistry milestone with improved cycle stability in 2026.",
                        "claim_hints": ["A 2026 solid-state battery chemistry milestone improved cycle stability."],
                        "counts_as_usable_evidence": true
                    }
                ]
            })],
        );
        assert!(
            response.contains("solid-state battery chemistry milestone"),
            "{response}"
        );
        assert!(
            !response.contains("Nobel Prize announcements are scheduled"),
            "{response}"
        );
    }

    #[test]
    fn tool_evidence_fallback_filters_source_inventory_fragments() {
        let response = fallback_final_response_from_tool_evidence(
            "Research home backup options versus portable power stations for outage resilience.",
            &[json!({
                "name": "batch_query",
                "status": "ok",
                "is_error": false,
                "evidence_pack": [
                    {
                        "title": "User Guide - Portable Power Shop",
                        "source_domain": "example.test",
                        "summary": "User Guide / Shop Solar Generators Portable Power Stations Accessories Gift Card",
                        "counts_as_usable_evidence": true
                    },
                    {
                        "title": "Backup comparison",
                        "source_domain": "example.test",
                        "relevant_extract": "Whole-home batteries can keep hardwired circuits running during outages, while portable power stations are easier to move but usually cover fewer loads.",
                        "counts_as_usable_evidence": true
                    }
                ]
            })],
        );
        assert!(
            response.contains("Whole-home batteries can keep hardwired circuits running"),
            "{response}"
        );
        assert!(!response.contains("User Guide / Shop"), "{response}");
        assert!(!response.contains("Gift Card"), "{response}");
    }

    #[test]
    fn tool_evidence_fallback_filters_affiliate_disclosure_shell() {
        let response = fallback_final_response_from_tool_evidence(
            "Compare current cordless vacuum options for a small apartment with pets across Dyson, Shark, Tineco, and Miele.",
            &[json!({
                "name": "batch_query",
                "status": "ok",
                "is_error": false,
                "query_metadata": {
                    "required_coverage": {
                        "entities": ["Dyson", "Shark", "Tineco", "Miele"]
                    }
                },
                "evidence_pack": [
                    {
                        "title": "Home Vacuum Zone review shell",
                        "source_domain": "example.test",
                        "claim_hints": ["Affiliate Disclosure: Home Vacuum Zone is reader-supported."],
                        "counts_as_usable_evidence": true
                    },
                    {
                        "title": "Generic cordless overview",
                        "source_domain": "example.test",
                        "claim_hints": ["Cordless vacuums have evolved dramatically in the last few years, offering stronger suction and longer battery life."],
                        "counts_as_usable_evidence": true
                    }
                ]
            })],
        );
        assert!(!response.contains("Affiliate Disclosure"), "{response}");
        assert!(!response.contains("reader-supported"), "{response}");
        assert!(
            response.contains("reliable comparison") || response.contains("coverage gaps remain"),
            "{response}"
        );
    }

    #[test]
    fn tool_evidence_fallback_filters_article_title_fragments() {
        let response = fallback_final_response_from_tool_evidence(
            "Research the current evidence on creatine supplementation for women.",
            &[json!({
                "name": "batch_query",
                "status": "ok",
                "is_error": false,
                "evidence_pack": [{
                    "title": "Web result from evidentianutrition.org",
                    "source_domain": "evidentianutrition.org",
                    "relevant_extract": "Articles / Creatine and women: what does the evidence actually show",
                    "claim_hints": ["Articles / Creatine and women: what does the evidence actually show"],
                    "counts_as_usable_evidence": true
                }]
            })],
        );
        assert!(!response.contains("Articles / Creatine"), "{response}");
        assert!(
            !response.contains("what does the evidence actually show"),
            "{response}"
        );
    }

    #[test]
    fn tool_evidence_fallback_filters_preview_title_shell() {
        let response = fallback_final_response_from_tool_evidence(
            "Research data-residency and sovereignty requirements that matter for SaaS buyers in 2026 for selling into Europe and the US public sector, with practical compliance picture.",
            &[json!({
                "name": "batch_query",
                "status": "ok",
                "is_error": false,
                "query_metadata": {
                    "required_coverage": {
                        "entities": ["GDPR", "Schrems II", "FedRAMP", "StateRAMP", "CJIS", "ITAR", "DFARS"]
                    }
                },
                "evidence_pack": [
                    {
                        "title": "FedRAMP consolidated rules",
                        "source_domain": "example.test",
                        "claim_hints": ["The Consolidated Rules will have Agency Use of FedRAMP Certified Cloud Services (Needs Review) - FedRAMP Consolidated Rules for 2026 Public Preview."],
                        "counts_as_usable_evidence": true
                    },
                    {
                        "title": "EDPB guidance",
                        "source_domain": "example.test",
                        "claim_hints": ["EDPB guidance emphasizes controllers cannot rely on SCCs alone when data flows to jurisdictions where public authorities may access data beyond necessary levels."],
                        "counts_as_usable_evidence": true
                    }
                ]
            })],
        );
        assert!(!response.contains("Needs Review"), "{response}");
        assert!(!response.contains("Public Preview"), "{response}");
        assert!(
            response.contains("reliable comparison") || response.contains("coverage gaps remain"),
            "{response}"
        );
    }

    #[test]
    fn tool_evidence_fallback_filters_doc_action_chrome_and_prefers_substantive_units() {
        let response = fallback_final_response_from_tool_evidence(
            "Compare LlamaIndex workflows versus LangGraph for document-heavy research assistants.",
            &[json!({
                "name": "batch_query",
                "status": "ok",
                "is_error": false,
                "evidence_pack": [
                    {
                        "title": "LlamaAgents Agent Workflows",
                        "source_domain": "developers.llamaindex.ai",
                        "claim_hints": [
                            "LlamaAgents Agent Workflows Introduction Copy Markdown Open in Claude Open in ChatGPT Open in Cursor View as Markdown Introduction What is a workflow."
                        ],
                        "counts_as_usable_evidence": true
                    },
                    {
                        "title": "LlamaIndex ingestion guide",
                        "source_domain": "developers.llamaindex.ai",
                        "claim_hints": [
                            "LlamaIndex has introduced async metadata extraction for ingestion pipelines."
                        ],
                        "counts_as_usable_evidence": true
                    }
                ]
            })],
        );
        assert!(
            response.starts_with("LlamaIndex has introduced async metadata extraction"),
            "{response}"
        );
        assert!(!response.contains("Copy Markdown"), "{response}");
        assert!(!response.contains("Open in ChatGPT"), "{response}");
        assert!(!response.contains("View as Markdown"), "{response}");
    }

    #[test]
    fn tool_evidence_fallback_keeps_process_metadata_when_goal_asks_for_schedule() {
        let response = fallback_final_response_from_tool_evidence(
            "When are the 2026 Nobel Prize announcements scheduled?",
            &[json!({
                "name": "batch_query",
                "status": "ok",
                "is_error": false,
                "evidence_pack": [{
                    "title": "Nobel Prize schedule",
                    "locator": "https://example.test/nobel-schedule",
                    "source_domain": "example.test",
                    "relevant_extract": "The 2026 Nobel Prize announcements are scheduled for October.",
                    "claim_hints": ["The 2026 Nobel Prize announcements are scheduled for October."],
                    "counts_as_usable_evidence": true
                }]
            })],
        );
        assert!(
            response.contains("Nobel Prize announcements are scheduled"),
            "{response}"
        );
    }

    #[test]
    fn tool_evidence_fallback_avoids_off_topic_units_when_required_entity_lanes_are_uncovered() {
        let response = fallback_final_response_from_tool_evidence(
            "Compare LlamaIndex workflows versus LangGraph for document-heavy research assistants.",
            &[json!({
                "name": "batch_query",
                "status": "ok",
                "is_error": false,
                "query_metadata": {
                    "required_coverage": {
                        "entities": ["LlamaIndex", "LangGraph"]
                    }
                },
                "result": "Retrieved a governed AI evidence-retrieval paper that does not directly compare the requested frameworks.",
                "evidence_pack": [{
                    "title": "Kura governed AI",
                    "source_domain": "example.test",
                    "relevant_extract": "Kura focuses on governed evidence retrieval for AI systems.",
                    "claim_hints": ["Kura focuses on governed evidence retrieval for AI systems."],
                    "counts_as_usable_evidence": true
                }]
            })],
        );
        assert!(!response.starts_with("Kura focuses on"), "{response}");
        assert!(
            response.contains("reliable comparison")
                || response.contains("partial comparison")
                || response.contains("coverage gaps remain"),
            "{response}"
        );
        assert!(
            response.contains("LlamaIndex")
                || response.contains("LangGraph")
                || response.contains("coverage gaps remain"),
            "{response}"
        );
    }

    #[test]
    fn workflow_hedge_detector_accepts_tentative_and_unsettled_language() {
        assert!(workflow_answer_unit_is_hedged_or_gap(
            "what remains unsettled causality and dose response are still tentative"
        ));
        assert!(workflow_answer_unit_is_hedged_or_gap(
            "critical limitation my evidence this turn is thin and i would need to verify"
        ));
    }

    #[test]
    fn tool_evidence_fallback_requires_multiple_required_entities_for_comparison_answers() {
        let response = fallback_final_response_from_tool_evidence(
            "Research the current conversation-intelligence market for B2B sales teams. Compare Gong, Chorus, Clari Copilot, and Avoma.",
            &[json!({
                "name": "batch_query",
                "status": "ok",
                "is_error": false,
                "query_metadata": {
                    "required_coverage": {
                        "entities": ["Gong", "Chorus", "Clari Copilot", "Avoma"]
                    }
                },
                "tool_result_quality": {
                    "status": "low_signal",
                    "flags": ["comparison_evidence_insufficient"]
                },
                "evidence_pack": [{
                    "title": "Gong market overview",
                    "source_domain": "example.test",
                    "relevant_extract": "Gong is the deepest, most mature conversation-intelligence platform in the market.",
                    "claim_hints": ["Gong is the deepest, most mature conversation-intelligence platform in the market."],
                    "counts_as_usable_evidence": true
                }]
            })],
        );
        assert!(response.contains("partial comparison"), "{response}");
        assert!(response.contains("Gong"), "{response}");
        assert!(response.contains("best-supported option"), "{response}");
        assert!(
            response.contains("Chorus")
                || response.contains("Clari Copilot")
                || response.contains("Avoma")
                || response.contains("coverage"),
            "{response}"
        );
        assert!(!response.starts_with("For Gong, Chorus"), "{response}");
    }

    #[test]
    fn tool_evidence_fallback_prefers_clean_claims_over_source_title_fragments() {
        let response = fallback_final_response_from_tool_evidence(
            "Research the current legal-ops AI market for contract review and internal legal workflows. What looks operationally real versus mostly demo-driven?",
            &[json!({
                "name": "batch_query",
                "status": "ok",
                "is_error": false,
                "query_metadata": {
                    "required_coverage": {
                        "entities": ["legal ops AI"]
                    }
                },
                "evidence_pack": [
                    {
                        "title": "Flex Legal reshaped contract review",
                        "source_domain": "example.test",
                        "claim_hints": [
                            "2023 Value Champion Flex and DocJuris Value Through Diversity 8 Days to 5 Minutes: How Flex Legal Reshaped Contract Review Contract management is a common organizational challenge."
                        ],
                        "counts_as_usable_evidence": true
                    },
                    {
                        "title": "AI contract management market",
                        "source_domain": "example.test",
                        "claim_hints": [
                            "The AI contract management market has matured significantly in 2025, with clear leaders emerging across different use cases and organizational sizes."
                        ],
                        "counts_as_usable_evidence": true
                    },
                    {
                        "title": "Contract review deployment evidence",
                        "source_domain": "example.test",
                        "claim_hints": [
                            "Contract review evidence shows operational deployments are stronger when sources describe review queues, clause extraction controls, and integrations with internal legal intake rather than demo-only positioning."
                        ],
                        "counts_as_usable_evidence": true
                    }
                ]
            })],
        );
        assert!(
            response.starts_with("Contract review evidence shows operational deployments"),
            "{response}"
        );
        assert!(
            !response.contains("Value Champion Flex and DocJuris"),
            "{response}"
        );
        assert!(!response.contains("market has matured"), "{response}");
    }
