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
    fn tool_evidence_fallback_filters_date_stamped_headline_shells() {
        let response = fallback_final_response_from_tool_evidence(
            "Research current approaches to reducing meeting overload on remote teams. What interventions have stronger evidence or operational support than vague productivity advice?",
            &[json!({
                "name": "batch_query",
                "status": "ok",
                "is_error": false,
                "query_metadata": {
                    "required_coverage": {
                        "entities": ["meeting overload", "remote teams"]
                    }
                },
                "evidence_pack": [
                    {
                        "title": "Meeting-free-day article",
                        "source_domain": "example.test",
                        "claim_hints": [
                            "Meeting-Free Days: What the Data Actually Shows June 14, 2026."
                        ],
                        "counts_as_usable_evidence": true
                    },
                    {
                        "title": "Field experiment summary",
                        "source_domain": "example.test",
                        "claim_hints": [
                            "For remote teams, a large-scale field experiment found that prompting attendees to define meeting goals in advance improved meeting effectiveness and reduced meeting overload."
                        ],
                        "counts_as_usable_evidence": true
                    }
                ]
            })],
        );
        assert!(
            response.starts_with("A large-scale field experiment found"),
            "{response}"
        );
        assert!(!response.contains("Meeting-Free Days"), "{response}");
        assert!(!response.contains("What the Data Actually Shows"), "{response}");
    }

    #[test]
    fn tool_evidence_fallback_filters_published_source_dateline_shells() {
        let response = fallback_final_response_from_tool_evidence(
            "Research family-friendly neighborhoods to stay in Chicago for museums, transit access, and walkability. Compare a few options and tradeoffs.",
            &[json!({
                "name": "batch_query",
                "status": "ok",
                "is_error": false,
                "query_metadata": {
                    "required_coverage": {
                        "entities": ["Lincoln Park", "Hyde Park", "Wicker Park", "South Loop"]
                    }
                },
                "evidence_pack": [
                    {
                        "title": "Chicago Parent Hyde Park guide",
                        "source_domain": "chicagoparent.com",
                        "claim_hints": [
                            "Family Guide to Hyde Park: Things to Do with Kids Chicago Parent Published: Tue, 17 Sep 2024 07:00:00 GMT. Source: Chicago Parent (www.chicagoparent.com)."
                        ],
                        "counts_as_usable_evidence": true
                    },
                    {
                        "title": "Neighborhood tradeoff note",
                        "source_domain": "example.test",
                        "claim_hints": [
                            "Hyde Park gives families strong museum access near MSI, but the current evidence set does not yet support a fair comparison against Lincoln Park, Wicker Park, or South Loop."
                        ],
                        "counts_as_usable_evidence": true
                    }
                ]
            })],
        );
        assert!(!response.contains("Published"), "{response}");
        assert!(!response.contains("GMT"), "{response}");
        assert!(!response.contains("Chicago Parent"), "{response}");
        assert!(response.contains("Hyde Park"), "{response}");
    }

    #[test]
    fn tool_evidence_fallback_turns_single_lane_title_shell_into_bounded_partial_answer() {
        let response = fallback_final_response_from_tool_evidence(
            "Research family-friendly neighborhoods to stay in Chicago for museums, transit access, and walkability. Compare a few options and tradeoffs.",
            &[json!({
                "name": "batch_query",
                "status": "ok",
                "is_error": false,
                "query_metadata": {
                    "required_coverage": {
                        "entities": ["Lincoln Park", "Hyde Park", "Near North Side", "The Loop", "Wicker Park"]
                    }
                },
                "evidence_pack": [
                    {
                        "title": "Family Guide to Hyde Park: Things to Do with Kids - Chicago Parent",
                        "source_domain": "chicagoparent.com",
                        "snippet": "Family Guide to Hyde Park: Things to Do with Kids Chicago Parent Published: Tue, 17 Sep 2024 07:00:00 GMT. Source: Chicago Parent (www.chicagoparent.com).",
                        "counts_as_usable_evidence": true
                    }
                ]
            })],
        );
        assert!(response.contains("Hyde Park"), "{response}");
        assert!(response.contains("Chicago Parent"), "{response}");
        assert!(
            response.contains("direct source-backed coverage")
                || response.contains("fair comparison"),
            "{response}"
        );
        assert!(!response.contains("Published"), "{response}");
        assert!(!response.contains("GMT"), "{response}");
    }

    #[test]
    fn tool_evidence_fallback_drops_goal_less_findings_shell_and_emits_bounded_insufficiency() {
        let response = fallback_final_response_from_tool_evidence(
            "Research travel-friendly noise-canceling headphones in 2026 for calls, comfort, battery life, and reliability. I want a shortlist I can buy with confidence.",
            &[json!({
                "name": "batch_query",
                "status": "ok",
                "is_error": false,
                "result": "Web findings: newsroom.",
                "query_metadata": {
                    "required_coverage": {
                        "entities": [
                            "Sony WH-1000XM6",
                            "Bose QuietComfort Ultra",
                            "Apple AirPods Max",
                            "Sennheiser Momentum 4",
                            "Shure AONIC 50"
                        ],
                        "facets": [
                            "call quality",
                            "comfort",
                            "battery life",
                            "reliability"
                        ]
                    }
                }
            })],
        );
        assert!(!response.contains("Here's what I found"), "{response}");
        assert!(!response.contains("web retrieval"), "{response}");
        assert!(!response.contains("newsroom"), "{response}");
        assert!(
            response.contains("reliable recommendation")
                || response.contains("confident conclusion"),
            "{response}"
        );
        assert!(response.contains("Sony WH-1000XM6"), "{response}");
        assert!(response.contains("call quality"), "{response}");
    }

    #[test]
    fn tool_evidence_fallback_turns_multi_lane_title_shells_into_partial_comparison() {
        let response = fallback_final_response_from_tool_evidence(
            "Research family-friendly neighborhoods to stay in Chicago for museums, transit access, and walkability, and compare options and tradeoffs.",
            &[json!({
                "name": "batch_query",
                "status": "ok",
                "is_error": false,
                "query_metadata": {
                    "required_coverage": {
                        "entities": ["Lincoln Park", "Hyde Park", "South Loop", "River North", "Loop"]
                    }
                },
                "evidence_pack": [
                    {
                        "title": "Family Guide to Hyde Park: Things to Do with Kids - Chicago Parent",
                        "source_domain": "chicagoparent.com",
                        "snippet": "Family Guide to Hyde Park: Things to Do with Kids Chicago Parent Published: Tue, 17 Sep 2024 07:00:00 GMT. Source: Chicago Parent (www.chicagoparent.com).",
                        "counts_as_usable_evidence": true
                    },
                    {
                        "title": "South Loop family travel guide - Example Travel",
                        "source_domain": "example.travel",
                        "snippet": "South Loop family travel guide Example Travel Published: Mon, 12 Aug 2024 07:00:00 GMT. Source: Example Travel (example.travel).",
                        "counts_as_usable_evidence": true
                    }
                ]
            })],
        );
        assert!(
            response.contains("partial comparison")
                || response.contains("fair comparison")
                || response.contains("source-backed coverage"),
            "{response}"
        );
        assert!(response.contains("Hyde Park"), "{response}");
        assert!(response.contains("South Loop"), "{response}");
        assert!(!response.contains("Published"), "{response}");
        assert!(!response.contains("GMT"), "{response}");
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
    fn tool_evidence_fallback_prefers_bounded_evidence_sketch_over_coverage_state_note() {
        let response = fallback_final_response_from_tool_evidence(
            "Research current approaches to reducing meeting overload on remote teams. What interventions have stronger evidence or operational support than vague productivity advice?",
            &[json!({
                "name": "batch_query",
                "status": "ok",
                "is_error": false,
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
            })],
        );
        assert!(
            response.contains("cut meeting time by 60% using async boards"),
            "{response}"
        );
        assert!(
            !response.starts_with("My recommendation is to treat the current evidence as insufficient"),
            "{response}"
        );
        assert!(!response.contains("Coverage state:"), "{response}");
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

    #[test]
    fn tool_evidence_fallback_filters_headline_byline_shells() {
        let response = fallback_final_response_from_tool_evidence(
            "Explain the current data residency and sovereignty requirements that matter to SaaS buyers selling in Europe and the public sector.",
            &[json!({
                "name": "batch_query",
                "status": "ok",
                "is_error": false,
                "evidence_pack": [
                    {
                        "title": "EU data residency article",
                        "source_domain": "example.test",
                        "claim_hints": [
                            "Self-Hosted EU Data Residency Laws Are Breaking Your SaaS Stack (Here's How to Fix It) /author/michael/ Michael Soto 12 Feb 2026 •"
                        ],
                        "counts_as_usable_evidence": true
                    },
                    {
                        "title": "EU data residency guidance",
                        "source_domain": "example.test",
                        "claim_hints": [
                            "For SaaS buyers selling into Europe or the public sector, the practical requirements are data location controls, subprocessor transparency, and clear jurisdiction boundaries for regulated workloads."
                        ],
                        "counts_as_usable_evidence": true
                    }
                ]
            })],
        );
        assert!(!response.contains("/author/michael/"), "{response}");
        assert!(
            !response.contains(
                "Self-Hosted EU Data Residency Laws Are Breaking Your SaaS Stack"
            ),
            "{response}"
        );
    }
