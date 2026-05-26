    #[test]
    fn current_agentic_framework_comparison_does_not_auto_select_web_candidate() {
        let candidates = latent_tool_candidates_for_message(
            "try again to do a real source-backed comparison for the topic I asked about",
            &[],
        );
        assert!(
            !candidates.iter().any(|candidate| {
                candidate
                    .get("tool")
                    .and_then(Value::as_str)
                    .map(|tool| tool == "web_search")
                    .unwrap_or(false)
            }),
            "latent tooling must not auto-select web_search; the workflow CD must present the menu and wait for the LLM"
        );
    }

    #[test]
    fn ordinary_lookup_and_search_intents_do_not_create_latent_candidates() {
        for message in [
            "look up recent changes in the relevant frameworks",
            "search the web for public evidence about a named system",
            "use web research to compare current options",
        ] {
            let candidates = latent_tool_candidates_for_message(message, &[]);
            assert!(candidates.is_empty(), "{message}: {candidates:?}");
        }
    }

    #[test]
    fn time_scoped_update_requests_do_not_create_latent_candidates() {
        for message in [
            "give me an update on the agentic landscape in May 2026",
            "summarize the current state of synthetic biology in 2026",
            "brief me on the electric vehicle market landscape this year",
        ] {
            let candidates = latent_tool_candidates_for_message(message, &[]);
            assert!(candidates.is_empty(), "{message}: {candidates:?}");
        }
    }

    #[test]
    fn evaluative_web_research_prompts_do_not_create_latent_candidates() {
        let candidates = latent_tool_candidates_for_message(
            "What is the best agentic framework in 2026? Search first, but do not trust marketing pages blindly. Give me a defensible answer.",
            &[],
        );
        assert!(candidates.is_empty(), "{candidates:?}");
    }

    #[test]
    fn runtime_temporal_context_declares_past_future_rule() {
        let prompt = agent_runtime_temporal_context_prompt();
        assert!(prompt.contains("current date/time"));
        assert!(prompt.contains("Dates before this timestamp are in the past"));
        assert!(prompt.contains("dates after it are in the future"));
    }

    #[test]
    fn unresolved_tool_need_without_progress_is_rejected_signal() {
        assert!(manual_toolbox_response_exposes_unresolved_tool_need(
            "I don't have current web search results, but I can compare if you'd like me to search."
        ));
        assert!(manual_toolbox_response_exposes_unresolved_tool_need(
            "Web search returned limited results for this specific comparison. I can provide a ranked table."
        ));
        assert!(!manual_toolbox_response_exposes_unresolved_tool_need(
            "I would choose a menu item for the user's current topic."
        ));
    }

    #[test]
    fn stale_tool_intent_draft_for_simple_greeting_is_withheld() {
        let message = "hey";
        let response = "I need to perform a web search for the user's comparison topic. Let me start that process. [tool:Web Research]";
        assert!(workflow_response_requests_more_tooling(response));
        assert!(response_contains_unrequested_content_without_tool_evidence(
            message,
            response,
            &[],
        ));
        assert!(response_current_turn_dominance_violation(
            message,
            response,
            &[],
        ));
    }

    #[test]
    fn stale_mixed_tool_draft_for_simple_greeting_requires_fresh_synthesis() {
        let response = "I will use web search for the user's comparison topic. Please hold while I gather details. Meanwhile, let's inspect the tiny fixture repo and identify a small bugfix. 1 = Respond directly";
        assert!(workflow_response_requests_more_tooling(response));
        assert!(turn_workflow_requires_final_llm(&[], &[], response));
    }

    #[test]
    fn unsupported_tool_claim_guard_ignores_later_hypothetical_offer() {
        assert!(response_claims_tool_success_without_current_turn_evidence(
            "Use web search for the exact comparison topic supplied by the user.",
            "Web search didn't return specific April 2026 comparisons. I can provide a source-backed ranked table if you name specific frameworks.",
            &[],
        ));
        assert!(!response_claims_tool_success_without_current_turn_evidence(
            "Use web search for the exact comparison topic supplied by the user.",
            "I would choose a menu item for the user's current topic.",
            &[],
        ));
    }

    #[test]
    fn recorded_low_signal_tool_result_counts_as_visible_answer() {
        let tools = vec![json!({
            "name": "batch_query",
            "status": "no_results",
            "result": "Search did not produce enough source coverage for the requested comparison."
        })];

        assert!(response_answers_tool_confirmation_with_recorded_result(
            "The search did not find enough relevant source coverage for that comparison.",
            &tools,
        ));
        assert!(!response_answers_tool_confirmation_with_recorded_result(
            "I searched the web.",
            &tools,
        ));
        assert!(!response_answers_tool_confirmation_with_recorded_result(
            "", &tools,
        ));
    }

    #[test]
    fn final_verifier_rejects_tool_status_overlead_before_answer() {
        let tools = vec![json!({
            "name": "batch_query",
            "status": "no_results",
            "result": "Search did not produce enough source coverage for the requested comparison.",
            "tool_result_quality": {
                "flags": ["low_signal"],
                "evidence_count": 0
            }
        })];

        assert!(response_violates_tool_backed_final_verifier(
            "The web search results are too thin and provider-degraded to answer. I cannot give a useful conclusion.",
            &tools,
        ));
        assert!(!response_violates_tool_backed_final_verifier(
            "Bottom line: treat the topic as unverified from this retrieval turn and avoid making a source-backed choice until a better source lane is available. The search result was low-signal, so this is bounded guidance rather than retrieved evidence.",
            &tools,
        ));
    }

    #[test]
    fn final_verifier_rejects_status_overlead_variants_from_research_turns() {
        let tools = vec![json!({
            "name": "batch_query",
            "status": "low_signal",
            "result": "Retrieval returned only partial source coverage.",
            "tool_result_quality": {
                "flags": ["low_signal"],
                "evidence_count": 0
            }
        })];

        assert!(response_violates_tool_backed_final_verifier(
            "I ran a batch search, but the results were low-signal, so I cannot answer.",
            &tools,
        ));
        assert!(response_violates_tool_backed_final_verifier(
            "Based on the search attempt, there is not enough retrieved evidence to decide.",
            &tools,
        ));
        assert!(!response_violates_tool_backed_final_verifier(
            "The practical answer is to treat the choice as unverified by this turn and avoid a strong recommendation until source coverage improves. The retrieval attempt was low-signal, so this is bounded guidance.",
            &tools,
        ));
    }

    #[test]
    fn final_verifier_rejects_missing_evidence_claim_when_refs_exist() {
        let tools = vec![json!({
            "name": "web_search",
            "status": "ok",
            "result": "Official docs say the library supports typed agent outputs.",
            "evidence_refs": [{
                "title": "Official docs",
                "locator": "https://example.test/docs"
            }],
            "tool_result_quality": {
                "evidence_count": 1
            }
        })];

        assert!(response_violates_tool_backed_final_verifier(
            "No evidence is available for this question, so I cannot answer.",
            &tools,
        ));
        assert!(!response_violates_tool_backed_final_verifier(
            "Bottom line: the recorded source supports typed agent outputs, but it does not prove production maturity. Treat the evidence as useful for capability fit and still verify operations, support, and deployment references.",
            &tools,
        ));
    }

    #[test]
    fn final_verifier_rejects_outside_evidence_decision_basis() {
        let tools = vec![json!({
            "name": "web_search",
            "status": "ok",
            "result": "Retrieved partial evidence for the requested comparison.",
            "evidence_refs": [{
                "title": "Partial comparison source",
                "locator": "https://example.test/partial"
            }],
            "tool_result_quality": {
                "evidence_count": 1
            }
        })];

        assert_eq!(
            tool_backed_final_verifier_violation_reason(
                "The retrieved evidence does not support a direct comparison. General knowledge, not source-backed in this turn: Alpha is known for reliability and Beta is known for flexibility. Bottom line: choose Alpha for production.",
                &tools,
            ),
            Some("final_response_verifier_contract:outside_evidence_used_for_decision".to_string())
        );
        assert_eq!(
            tool_backed_final_verifier_violation_reason(
                "The retrieved evidence does not support a direct comparison. General knowledge would be outside retrieved evidence here, so there is no source-backed basis to recommend Alpha or Beta.",
                &tools,
            ),
            None
        );
    }

    #[test]
    fn final_verifier_rejects_answer_unit_wording_that_drifts_beyond_evidence() {
        let tools = vec![json!({
            "name": "web_search",
            "status": "ok",
            "evidence_pack_candidates": [{
                "title": "OpenAI Research",
                "locator": "https://openai.com/index/model-disproves-discrete-geometry-conjecture/",
                "claim_hints": [
                    "OpenAI Research said a model disproved the unit distance problem in discrete geometry."
                ],
                "relevant_extract": "An OpenAI model disproved a central conjecture in discrete geometry known as the unit distance problem.",
                "score": 91.0,
                "confidence": "usable"
            }]
        })];

        assert_eq!(
            tool_backed_final_verifier_violation_reason(
                "Here are notable scientific developments: Mathematics / AI-assisted research - An OpenAI model disproved the 80-year-old unit distance problem. Published May 20, 2026.",
                &tools,
            ),
            Some("final_response_verifier_contract:answer_units_not_traceable_to_evidence".to_string())
        );
        assert_eq!(
            tool_backed_final_verifier_violation_reason(
                "OpenAI Research reported that a model disproved the unit distance problem in discrete geometry.",
                &tools,
            ),
            None
        );
    }

    #[test]
    fn final_verifier_counts_tool_result_text_as_recorded_evidence() {
        let tools = vec![json!({
            "name": "batch_query",
            "status": "ok",
            "result": "A critical phase of machine assembly has begun at ITER's construction site in Cadarache, southern France, according to a May 2026 report from Nucnet.",
            "evidence_refs": [{
                "title": "ITER European fusion project now on budget and on schedule after turnaround",
                "locator": "https://www.nucnet.org/news/iter-european-fusion-project-now-on-budget-and-on-schedule-after-turnaround-5-5-2026"
            }]
        })];

        assert_eq!(
            tool_backed_final_verifier_violation_reason(
                "A critical phase of machine assembly has begun at ITER's construction site in Cadarache, southern France, according to a May 2026 report from Nucnet.",
                &tools,
            ),
            None
        );
    }

    #[test]
    fn final_verifier_treats_materialized_candidates_as_recorded_evidence() {
        let tools = vec![json!({
            "name": "browser_materialize_page",
            "status": "ok",
            "result": "Rendered page extracted through materialization.",
            "evidence_pack_candidates": [{
                "source_kind": "browser_materialized_page",
                "title": "Rendered source",
                "locator": "https://example.test/rendered",
                "snippet": "The rendered page provides enough text for normal synthesis consumption.",
                "claim_hints": ["Rendered source supports the research claim."],
                "score": 76.0,
                "confidence": "usable"
            }]
        })];

        assert!(response_violates_tool_backed_final_verifier(
            "No evidence was found for this question.",
            &tools,
        ));

        let synthesis_input = workflow_synthesis_input_for_final_response(
            "research the rendered source",
            &tools,
            &json!({}),
        );
        assert_eq!(
            synthesis_input
                .pointer("/evidence_pack/0/source")
                .and_then(Value::as_str),
            Some("evidence_pack_candidate"),
            "{synthesis_input:#?}"
        );
        assert_eq!(
            synthesis_input
                .pointer("/evidence_pack/0/source_kind")
                .and_then(Value::as_str),
            Some("browser_materialized_page"),
            "{synthesis_input:#?}"
        );
    }

    #[test]
    fn final_verifier_rejects_missing_named_coverage_lanes() {
        let tools = vec![json!({
            "name": "batch_query",
            "status": "ok",
            "result": "Retrieved evidence across the comparison request.",
            "query_metadata": {
                "required_coverage": {
                    "entities": ["Infring", "LangGraph", "CrewAI", "AutoGen", "OpenHands"]
                }
            },
            "evidence_refs": [{
                "title": "Framework comparison",
                "locator": "https://example.test/frameworks"
            }]
        })];

        assert!(response_violates_tool_backed_final_verifier(
            "Bottom line: Infring, LangGraph, and CrewAI have enough evidence for a provisional comparison, but the ranking remains bounded.",
            &tools,
        ));
        assert!(!response_violates_tool_backed_final_verifier(
            "Bottom line: Infring, LangGraph, and CrewAI have enough evidence for a provisional comparison. AutoGen and OpenHands remain weakly covered in this retrieval turn, so treat their tradeoffs as explicit coverage gaps rather than source-backed conclusions.",
            &tools,
        ));
    }

    #[test]
    fn final_verifier_does_not_hard_require_expanded_query_alias_lanes() {
        let tools = vec![json!({
            "name": "batch_query",
            "status": "ok",
            "result": "Retrieved source-backed evidence for a broad landscape update.",
            "input": json!({
                "query": "Give me an update on the AI agentic landscape in May 2026",
                "queries": ["autonomous AI agents enterprise adoption May 2026"],
                "required_coverage": {
                    "entities": ["AI agents", "agentic AI", "autonomous agents", "multi-agent systems"]
                }
            }).to_string(),
            "evidence_refs": [{
                "title": "Agentic AI landscape",
                "locator": "https://example.test/agentic-ai-landscape",
                "snippet": "Agentic AI adoption and orchestration are changing in 2026."
            }],
            "tool_result_quality": {
                "coverage": {
                    "bucket_status": "covered",
                    "missing_buckets": []
                },
                "evidence_count": 1
            }
        })];

        assert_eq!(
            tool_backed_final_verifier_violation_reason(
                "Bottom line: according to the retrieved source, the agentic AI landscape in May 2026 is centered on enterprise adoption, orchestration, and platform/infrastructure maturation.",
                &tools,
            ),
            None
        );
    }

    #[test]
    fn final_verifier_accepts_package_backed_source_signal_when_evidence_exists() {
        let tools = vec![json!({
            "name": "batch_query",
            "status": "ok",
            "result": "Retrieved source-backed evidence for the requested comparison.",
            "evidence_refs": [{
                "title": "Framework comparison source",
                "locator": "https://example.test/frameworks",
                "snippet": "Substantive citable evidence for the answer."
            }]
        })];

        assert_eq!(
            tool_backed_final_verifier_violation_reason(
                "Bottom line: Alpha is better for production while Beta is better for prototypes.",
                &tools,
            ),
            None
        );
        assert_eq!(
            tool_backed_final_verifier_violation_reason(
                "Bottom line: according to the retrieved project docs, Alpha is better for production while Beta is better for prototypes.",
                &tools,
            ),
            None
        );
        assert_eq!(
            tool_backed_final_verifier_violation_reason(
                "Bottom line: Alpha is better for production while Beta is better for prototypes (langchain.com).",
                &tools,
            ),
            None
        );
    }

    #[test]
    fn final_synthesis_retry_guidance_names_missing_coverage_lane_behavior() {
        let prompt = workflow_final_synthesis_retry_prompt_context(
            "final_response_verifier_contract:missing_coverage_lanes=AutoGen, OpenHands",
            "Bottom line: Infring, LangGraph, and CrewAI are covered.",
        );
        let lowered = prompt.to_ascii_lowercase();

        assert!(lowered.contains("missing coverage lanes"));
        assert!(lowered.contains("cover each named lane"));
        assert!(lowered.contains("weak or missing"));
        assert!(lowered.contains("required output format"));
    }

    #[test]
    fn final_synthesis_retry_guidance_is_internal_and_format_free() {
        let prompt = workflow_final_synthesis_retry_prompt_context(
            "final_response_verifier_contract:missing_citation_or_source_signal",
            "The web search results are too thin.",
        );
        let lowered = prompt.to_ascii_lowercase();

        assert!(lowered.contains("internal final-response verifier retry"));
        assert!(lowered.contains("lead with the best bounded answer"));
        assert!(lowered.contains("source grounding"));
        assert!(lowered.contains("do not mention this verifier"));
        assert!(lowered.contains("required output format"));
    }

    #[test]
    fn final_synthesis_retry_guidance_explains_prompt_analysis_leak() {
        let prompt = workflow_final_synthesis_retry_prompt_context(
            "workflow_prompt_analysis_leak",
            "The user wants a comparison of robot vacuums.",
        );
        let lowered = prompt.to_ascii_lowercase();

        assert!(lowered.contains("narrated the prompt instead of answering"));
        assert!(lowered.contains("start with the subject-matter answer"));
        assert!(lowered.contains("the user wants"));
        assert!(lowered.contains("the user asks"));
    }

    #[test]
    fn latent_tool_candidates_do_not_force_prompt_only_gate() {
        let message = "what? why are you repeating the same fallback text?";
        let latent_tool_candidates = json!([{"tool": "web_search"}]);
        let no_tool_minimal_final_turn = message_explicitly_disallows_tool_calls(message);
        let manual_toolbox_prompt_only_turn = !no_tool_minimal_final_turn
            && response_tools_prompt_only_gate_required(message, &latent_tool_candidates);

        assert!(!no_tool_minimal_final_turn);
        assert!(!manual_toolbox_prompt_only_turn);
    }
