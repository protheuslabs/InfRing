    #[test]
    fn tool_evidence_fallback_does_not_answer_decision_prompt_from_profile_copy_only() {
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
                        "title": "Vendor profile",
                        "source_domain": "example.test",
                        "claim_hints": [
                            "Kira is a modernized version of Litera Workflow, a system that enables teams to automate the review, analysis, and management of legal documents."
                        ],
                        "counts_as_usable_evidence": true
                    },
                    {
                        "title": "Market overview",
                        "source_domain": "example.test",
                        "claim_hints": [
                            "The AI contract management market has matured significantly, with clear leaders emerging across different use cases and organizational sizes."
                        ],
                        "counts_as_usable_evidence": true
                    }
                ]
            })],
        );
        assert!(!response.contains("Kira is a modernized version"), "{response}");
        assert!(!response.contains("market has matured"), "{response}");
        assert!(
            response.contains("reliable comparison")
                || response.contains("coverage gaps remain")
                || response.contains("insufficient"),
            "{response}"
        );
    }

    #[test]
    fn tool_evidence_fallback_filters_front_matter_before_answer_units() {
        let response = fallback_final_response_from_tool_evidence(
            "Research mainstream historical interpretations of the long-term legacy of Japanese American incarceration in the United States.",
            &[json!({
                "name": "batch_query",
                "status": "ok",
                "is_error": false,
                "query_metadata": {
                    "required_coverage": {
                        "entities": ["Japanese American incarceration", "United States"]
                    }
                },
                "evidence_pack": [
                    {
                        "title": "PDF front matter",
                        "source_domain": "example.test",
                        "claim_hints": [
                            "2021 Professor Rodnyansky Rodarte 1 Abstract This paper explores the legacy of the Japanese American Redress and Reparations movement and the 1988 Civil Liberties Act."
                        ],
                        "counts_as_usable_evidence": true
                    },
                    {
                        "title": "Legacy interpretation",
                        "source_domain": "example.test",
                        "claim_hints": [
                            "The long-term legacy of Japanese American incarceration in the United States is commonly framed as a civil-liberties failure that led to redress, reparations, and public-memory institutions."
                        ],
                        "counts_as_usable_evidence": true
                    }
                ]
            })],
        );
        assert!(response.starts_with("The long-term legacy"), "{response}");
        assert!(!response.contains("Professor Rodnyansky"), "{response}");
        assert!(!response.contains("Abstract This paper explores"), "{response}");
    }

    #[test]
    fn tool_evidence_fallback_filters_headline_leads_and_dangling_teasers() {
        let response = fallback_final_response_from_tool_evidence(
            "Build a practical home inventory plan for insurance claims.",
            &[json!({
                "name": "batch_query",
                "status": "ok",
                "is_error": false,
                "query_metadata": {
                    "required_coverage": {
                        "entities": ["home inventory", "insurance claims"]
                    }
                },
                "evidence_pack": [
                    {
                        "title": "Home inventory teaser",
                        "source_domain": "example.test",
                        "claim_hints": [
                            "Why a Home Inventory Matters More Than You Think After a fire, flood, or severe storm, the last thing any homeowner wants to do is try to remember every"
                        ],
                        "counts_as_usable_evidence": true
                    },
                    {
                        "title": "Insurance inventory guidance",
                        "source_domain": "example.test",
                        "claim_hints": [
                            "A home inventory is useful for insurance claims because it documents belongings and estimated value before a loss."
                        ],
                        "counts_as_usable_evidence": true
                    }
                ]
            })],
        );
        assert!(response.starts_with("A home inventory is useful"), "{response}");
        assert!(!response.contains("Why a Home Inventory"), "{response}");
        assert!(!response.contains("remember every"), "{response}");
    }

    #[test]
    fn tool_evidence_fallback_filters_teaser_shell_claims() {
        let response = fallback_final_response_from_tool_evidence(
            "Research data-residency and sovereignty requirements that matter for SaaS buyers in 2026. I want the practical picture for selling into Europe and the US public sector.",
            &[json!({
                "name": "batch_query",
                "status": "ok",
                "is_error": false,
                "query_metadata": {
                    "required_coverage": {
                        "entities": ["data residency", "Europe", "US public sector"]
                    }
                },
                "evidence_pack": [
                    {
                        "title": "IDC survey card",
                        "source_domain": "example.test",
                        "claim_hints": [
                            "This IDC Survey examines how digital sovereignty concerns are shaping cloud strategies, application placement decisions, and technology investment priorities."
                        ],
                        "counts_as_usable_evidence": true
                    },
                    {
                        "title": "Legal article card",
                        "source_domain": "example.test",
                        "claim_hints": [
                            "Pt 2: Long term service contracts Ian Makgill Business ,Software ,Technology 27 Apr, 2026 09 Mins read If you sell long-term services into European public sector buyers, the ground is moving under your feet."
                        ],
                        "counts_as_usable_evidence": true
                    },
                    {
                        "title": "Public sector hosting controls",
                        "source_domain": "example.test",
                        "claim_hints": [
                            "European enterprise and public-sector buyers increasingly ask SaaS vendors to document data residency, transfer controls, and public-sector hosting boundaries."
                        ],
                        "counts_as_usable_evidence": true
                    }
                ]
            })],
        );
        assert!(
            response.contains("reliable comparison")
                || response.contains("partial comparison")
                || response.contains("coverage gaps remain"),
            "{response}"
        );
        assert!(!response.contains("This IDC Survey examines"), "{response}");
        assert!(!response.contains("09 Mins read"), "{response}");
        assert!(!response.contains("Pt 2:"), "{response}");
    }

    #[test]
    fn fallback_visible_answer_preserves_query_aligned_claim_text() {
        let (answer, matched) = fallback_visible_answer_for_required_lanes(
            "The contract review market has matured significantly, with clearer leaders and more operational deployments for internal legal workflows.",
            &["legal ops AI".to_string()],
            &workflow_answer_unit_goal_terms(
                "Research the current legal-ops AI market for contract review and internal legal workflows.",
            ),
        );
        assert_eq!(
            answer,
            "The contract review market has matured significantly, with clearer leaders and more operational deployments for internal legal workflows."
        );
        assert_eq!(matched, vec!["legal ops AI".to_string()]);
    }

    #[test]
    fn final_verifier_rejects_retrieval_recap_when_answer_packets_exist() {
        let tools = vec![json!({
            "name": "batch_query",
            "status": "ok",
            "is_error": false,
            "evidence_pack": [{
                "pack_version": "evidence_pack_v1",
                "source_kind": "official_docs",
                "source_class": "official_or_primary",
                "title": "LangGraph docs",
                "locator": "https://docs.example.test/langgraph",
                "source_domain": "docs.example.test",
                "relevant_extract": "LangGraph supports stateful graph-based agent orchestration for long-running workflows.",
                "why_relevant_to_query": "It directly supports the requested comparison of agent workflow frameworks.",
                "claim_hints": [
                    "LangGraph supports stateful graph-based agent orchestration for long-running workflows."
                ],
                "counts_as_usable_evidence": true
            }]
        })];
        let bad_response = "The safest bounded answer is that the current retrieval state does not support a source-backed conclusion yet. Recorded evidence so far: Here's what I found: web search: From web retrieval: docs.example.test: LangGraph docs.";
        assert_eq!(
            tool_backed_final_verifier_violation_reason(bad_response, &tools).as_deref(),
            Some("final_response_verifier_contract:retrieval_recap_substituted_for_answer")
        );
    }

    #[test]
    fn final_verifier_rejects_internal_outcome_posture_leak() {
        let tools = vec![json!({
            "name": "batch_query",
            "status": "ok",
            "is_error": false,
            "evidence_pack": [{
                "pack_version": "evidence_pack_v1",
                "source_kind": "official_docs",
                "source_class": "official_or_primary",
                "title": "Example docs",
                "locator": "https://docs.example.test/example",
                "source_domain": "docs.example.test",
                "relevant_extract": "Alpha is the better fit for production use.",
                "claim_hints": ["Alpha is the better fit for production use."],
                "counts_as_usable_evidence": true
            }]
        })];
        let bad_response =
            "**Outcome posture: bounded_partial_answer** Alpha is the better fit for production use.";
        assert_eq!(
            tool_backed_final_verifier_violation_reason(bad_response, &tools).as_deref(),
            Some("final_response_verifier_contract:internal_scaffold_leaked")
        );
    }

    #[test]
    fn synthesis_answer_units_drop_ungrounded_claim_hints_and_keep_grounded_extracts() {
        let tools = vec![json!({
            "name": "batch_query",
            "status": "ok",
            "evidence_pack": [
                {
                    "title": "Carry On Backpack | Baseline Travel Backpack | Briggs & Riley",
                    "source_domain": "briggs-riley.ca",
                    "relevant_extract": "Carry On Backpack | Baseline Travel Backpack | Briggs & Riley",
                    "claim_hints": [
                        "Briggs & Riley publishes a detailed airline carry-on size guide with exact dimensions by carrier."
                    ],
                    "counts_as_usable_evidence": true
                },
                {
                    "title": "How Away Luggage Is Tested: Our Quality Standards | Away",
                    "source_domain": "awaytravel.com",
                    "relevant_extract": "How Away Luggage Is Tested: Our Quality Standards | Away",
                    "claim_hints": [
                        "Away subjects luggage to quality testing intended to withstand rough travel."
                    ],
                    "counts_as_usable_evidence": true
                },
                {
                    "title": "Compact Carry-On Hardside Spinner | Platinum Elite by Travelpro",
                    "source_domain": "travelpro.com",
                    "relevant_extract": "Travelpro's Platinum Elite carry-on is tested to fit overhead bins on most major U.S. airlines.",
                    "claim_hints": [
                        "Travelpro's Platinum Elite carry-on is tested to fit overhead bins on most major U.S. airlines."
                    ],
                    "counts_as_usable_evidence": true
                }
            ]
        })];

        let units = evidence_packet_answer_units_for_goal(
            "Compare current carry-on luggage brands for durability and airline practicality.",
            &tools,
            8,
        );

        assert!(
            units.iter()
                .any(|unit| unit.contains("Travelpro's Platinum Elite carry-on is tested to fit overhead bins")),
            "{units:#?}"
        );
        assert!(
            !units.iter().any(|unit| unit.contains("exact dimensions by carrier")),
            "{units:#?}"
        );
        assert!(
            !units.iter().any(|unit| unit.contains("intended to withstand rough travel")),
            "{units:#?}"
        );
    }

    #[test]
    fn final_verifier_rejects_incomplete_visible_answer() {
        let tools = vec![json!({
            "name": "batch_query",
            "status": "ok",
            "is_error": false,
            "evidence_pack": [{
                "pack_version": "evidence_pack_v1",
                "source_kind": "official_docs",
                "source_class": "official_or_primary",
                "title": "Example docs",
                "locator": "https://docs.example.test/example",
                "source_domain": "docs.example.test",
                "relevant_extract": "Alpha is the better fit for production use.",
                "claim_hints": ["Alpha is the better fit for production use."],
                "counts_as_usable_evidence": true
            }]
        })];
        let bad_response = "The strongest current answer is Alpha for";
        assert_eq!(
            tool_backed_final_verifier_violation_reason(bad_response, &tools).as_deref(),
            Some("final_response_verifier_contract:incomplete_visible_answer")
        );
    }

    #[test]
    fn rejected_retrieval_recap_is_rewritten_from_evidence_packets() {
        let mut workflow = json!({
            "response": "",
            "text": "",
            "message": "",
            "quality_telemetry": {},
            "final_llm_response": {
                "used": false,
                "status": "synthesis_failed"
            }
        });
        let tools = vec![json!({
            "name": "batch_query",
            "status": "ok",
            "is_error": false,
            "evidence_pack": [{
                "pack_version": "evidence_pack_v1",
                "source_kind": "official_docs",
                "source_class": "official_or_primary",
                "title": "CrewAI docs",
                "locator": "https://docs.example.test/crewai",
                "source_domain": "docs.example.test",
                "relevant_extract": "CrewAI emphasizes role-based multi-agent orchestration for collaborative agent workflows.",
                "why_relevant_to_query": "It directly supports a comparison of agent workflow frameworks.",
                "claim_hints": [
                    "CrewAI emphasizes role-based multi-agent orchestration for collaborative agent workflows."
                ],
                "counts_as_usable_evidence": true
            }]
        })];
        let bad_response = "The safest bounded answer is that the current retrieval state does not support a source-backed conclusion yet. Recorded evidence so far: Here's what I found: web search: From web retrieval: docs.example.test: CrewAI docs.";
        assert!(maybe_apply_rejected_tool_evidence_fallback(
            &mut workflow,
            "Compare LangGraph and CrewAI.",
            &tools,
            bad_response,
            bad_response,
            "final_response_verifier_contract:retrieval_recap_substituted_for_answer",
        ));
        let response = workflow
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(response.trim().is_empty(), "{response}");
        assert!(!response.contains("Here's what I found"), "{response}");
        assert!(!response.contains("Recorded evidence so far"), "{response}");
        assert_eq!(
            workflow
                .pointer("/final_llm_response/status")
                .and_then(Value::as_str),
            Some("tool_evidence_fallback_suppressed")
        );
    }

    #[test]
    fn tool_evidence_fallback_names_required_coverage_lanes() {
        let tools = vec![json!({
            "name": "batch_query",
            "status": "no_results",
            "is_error": false,
            "result": "Search providers ran but did not return usable evidence.",
            "query_metadata": {
                "required_coverage": {
                    "entities": ["PydanticAI", "LangGraph", "CrewAI", "LangChain", "OpenAI Agents SDK"],
                    "facets": ["production readiness", "type safety"]
                }
            },
            "tool_result_quality": {
                "status": "low_signal",
                "flags": ["insufficient_evidence"],
                "retry": {
                    "recommended": true,
                    "reason": "coverage_gap"
                }
            }
        })];
        let response = fallback_final_response_from_tool_evidence(
            "Is PydanticAI a serious production option for structured agent workflows in Python?",
            &tools,
        );

        assert!(response.contains("Coverage"), "{response}");
        assert!(response.contains("PydanticAI"), "{response}");
        assert!(
            !response.starts_with("This retrieval attempt"),
            "{response}"
        );

        let synthesis_input =
            workflow_synthesis_input_for_final_response("research PydanticAI", &tools, &json!({}));
        assert_eq!(
            synthesis_input
                .pointer("/coverage_gaps/0/requested_text")
                .and_then(Value::as_str),
            Some("PydanticAI"),
            "{synthesis_input:#?}"
        );
        assert_eq!(
            synthesis_input
                .pointer("/coverage_gaps/0/kind")
                .and_then(Value::as_str),
            Some("entity"),
            "{synthesis_input:#?}"
        );
    }

    #[test]
    fn tool_evidence_outcome_posture_distinguishes_supported_partial_and_insufficient() {
        let supported = vec![json!({
            "name": "batch_query",
            "status": "ok",
            "result": "Key findings: LangGraph and CrewAI differ in orchestration style.",
            "evidence_refs": [{
                "title": "LangGraph docs",
                "locator": "https://example.com/langgraph",
                "score": 0.9
            }]
        })];
        assert_eq!(
            tool_evidence_outcome_posture(&supported),
            "supported_answer"
        );

        let partial = vec![json!({
            "name": "batch_query",
            "status": "ok",
            "result": "Search returned some findings but facet coverage is incomplete.",
            "query_metadata": {
                "required_coverage": {
                    "entities": ["LangGraph", "CrewAI"]
                }
            },
            "evidence_refs": [{
                "title": "LangGraph docs",
                "locator": "https://example.com/langgraph",
                "score": 0.9
            }],
            "tool_result_quality": {
                "status": "low_signal",
                "flags": ["insufficient_evidence"]
            }
        })];
        assert_eq!(
            tool_evidence_outcome_posture(&partial),
            "bounded_partial_answer"
        );

        let insufficient = vec![json!({
            "name": "batch_query",
            "status": "error",
            "error": "provider timeout"
        })];
        assert_eq!(
            tool_evidence_outcome_posture(&insufficient),
            "evidence_insufficient_answer"
        );
    }
