    #[test]
    fn single_workflow_only_latent_candidate_can_recover_pending_tool_request() {
        let candidates = json!([{
            "workflow_only": true,
            "selected_tool_family": "web_research",
            "selected_tool_key": "batch_query",
            "selected_tool_label": "Research query pack",
            "input": {
                "source": "web",
                "query": "compare retrieval tools",
                "aperture": "medium"
            }
        }]);

        let pending = manual_toolbox_pending_request_from_latent_candidates(
            &candidates,
            "Research Firecrawl, Tavily, and Exa.",
        )
        .expect("single valid latent candidate");

        assert_eq!(
            pending.get("tool_name").and_then(Value::as_str),
            Some("batch_query")
        );
        assert_eq!(
            pending.get("selected_tool_key").and_then(Value::as_str),
            Some("batch_query")
        );
        assert_eq!(
            pending.get("source").and_then(Value::as_str),
            Some("latent_candidate_recovery")
        );
        assert_eq!(
            pending.pointer("/input/query").and_then(Value::as_str),
            Some("compare retrieval tools")
        );
        let keywords = pending
            .pointer("/input/keywords")
            .and_then(Value::as_array)
            .expect("recovered input should carry additive keywords");
        assert!(keywords.iter().any(|value| value == "Firecrawl"), "{pending:?}");
        assert!(keywords.iter().any(|value| value == "Tavily"), "{pending:?}");
        assert!(keywords.iter().any(|value| value == "Exa"), "{pending:?}");
        let entities = pending
            .pointer("/input/required_coverage/entities")
            .and_then(Value::as_array)
            .expect("recovered input should carry additive entity coverage");
        assert!(entities.iter().any(|value| value == "Firecrawl"), "{pending:?}");
        assert!(entities.iter().any(|value| value == "Tavily"), "{pending:?}");
        assert!(entities.iter().any(|value| value == "Exa"), "{pending:?}");
        assert!(
            pending
                .pointer("/input/query_metadata_policy/recovery_contract_repair/status")
                .and_then(Value::as_str)
                == Some("metadata_hydrated"),
            "{pending:?}"
        );
    }

    #[test]
    fn latent_candidate_recovery_preserves_simple_query_while_hydrating_metadata() {
        let candidates = json!([{
            "workflow_only": true,
            "selected_tool_family": "web_research",
            "selected_tool_key": "batch_query",
            "selected_tool_label": "Research query pack",
            "input": {
                "source": "web",
                "query": "weather in Denver today",
                "aperture": "medium"
            }
        }]);

        let pending = manual_toolbox_pending_request_from_latent_candidates(
            &candidates,
            "weather in Denver today",
        )
        .expect("single valid latent candidate");

        assert_eq!(
            pending.pointer("/input/query").and_then(Value::as_str),
            Some("weather in Denver today")
        );
        assert_eq!(
            pending.pointer("/input/source").and_then(Value::as_str),
            Some("web")
        );
        assert_eq!(
            pending.pointer("/input/aperture").and_then(Value::as_str),
            Some("medium")
        );
        assert_eq!(
            pending
                .pointer("/input/query_metadata_policy/recovery_contract_repair/status")
                .and_then(Value::as_str),
            Some("metadata_hydrated"),
            "{pending:?}"
        );
        assert!(
            pending
                .pointer("/input/required_coverage/entities")
                .and_then(Value::as_array)
                .map(|entities| entities.iter().any(|value| value == "Denver"))
                .unwrap_or(false),
            "{pending:?}"
        );
    }

    #[test]
    fn latent_candidates_recover_high_confidence_external_research_but_not_trivia() {
        let research_candidates = latent_tool_candidates_for_message(
            "Research Firecrawl, Tavily, and Exa as data tools for AI research agents.",
            &[],
        );
        assert_eq!(research_candidates.len(), 1, "{research_candidates:?}");
        assert_eq!(
            research_candidates[0].get("tool").and_then(Value::as_str),
            Some("batch_query"),
            "{research_candidates:?}"
        );
        assert_eq!(
            research_candidates[0]
                .get("requires_tool_attempt_before_final_answer")
                .and_then(Value::as_bool),
            Some(true),
            "{research_candidates:?}"
        );

        let trivia_candidates = latent_tool_candidates_for_message("what is 2+2?", &[]);
        assert!(trivia_candidates.is_empty(), "{trivia_candidates:?}");
    }

    #[test]
    fn latent_candidates_keep_internal_workspace_comparisons_out_of_web_recovery() {
        let candidates =
            latent_tool_candidates_for_message("compare this system to openclaw", &[]);
        assert!(candidates.is_empty(), "{candidates:?}");
    }

    #[test]
    fn terminal_invariant_can_require_single_latent_candidate_promotion() {
        let workflow = json!({
            "selected_workflow": {
                "tool_menu_interface_contract": {
                    "terminal_invariant_contract": {
                        "valid_latent_candidate_without_tool_attempt_policy": "promote_single_required_candidate_or_structured_failure_before_final_answer",
                        "required_latent_candidate_flag": "requires_tool_attempt_before_final_answer"
                    }
                }
            }
        });
        let candidates = json!([{
            "workflow_only": true,
            "selected_tool_family": "web_research",
            "selected_tool_key": "batch_query",
            "requires_tool_attempt_before_final_answer": true,
            "input": {"source": "web", "query": "compare retrieval tools", "aperture": "medium"}
        }]);

        assert!(
            workflow_latent_candidate_recovery_required_by_terminal_invariant(
                &workflow,
                &candidates
            )
        );
    }

    #[test]
    fn latent_candidate_recovery_refuses_ambiguous_candidates() {
        let candidates = json!([
            {
                "workflow_only": true,
                "selected_tool_family": "web_research",
                "selected_tool_key": "batch_query",
                "input": {"source": "web", "query": "first", "aperture": "medium"}
            },
            {
                "workflow_only": true,
                "selected_tool_family": "web_research",
                "selected_tool_key": "web_search",
                "input": {"source": "web", "query": "second", "aperture": "medium"}
            }
        ]);

        assert!(manual_toolbox_pending_request_from_latent_candidates(
            &candidates,
            "Research something current.",
        )
        .is_none());
    }

    #[test]
    fn latent_candidate_recovery_requires_missing_evidence_or_gate_diagnostic() {
        let missing_evidence_workflow = json!({
            "response": "No source-backed synthesis is available because I don't have recorded tool results for this turn."
        });
        let direct_answer_workflow = json!({
            "response": "The answer is 4."
        });
        let gate_diagnostic_workflow = json!({
            "workflow_control": {"direct_response_path": "gate_4_pending_llm_tool_request"}
        });
        let first_gate_private_workflow = json!({
            "workflow_control": {"direct_response_path": "first_gate_pending_llm_tool_choice"}
        });
        let rejected_private_gate_workflow = json!({
            "final_llm_response": {"last_reject_reason": "visible_gate_choice_reply"}
        });

        assert!(workflow_latent_candidate_recovery_needed(
            &missing_evidence_workflow,
            ""
        ));
        assert!(!workflow_latent_candidate_recovery_needed(
            &direct_answer_workflow,
            ""
        ));
        assert!(workflow_latent_candidate_recovery_needed(
            &gate_diagnostic_workflow,
            ""
        ));
        assert!(workflow_latent_candidate_recovery_needed(
            &first_gate_private_workflow,
            ""
        ));
        assert!(workflow_latent_candidate_recovery_needed(
            &rejected_private_gate_workflow,
            ""
        ));
    }

    #[test]
    fn meta_control_recovery_accepts_direct_fallback_loop_answer() {
        let message = "what? why are you repeating the same fallback text?";
        assert!(direct_gate_recovery_response_answers_user(
            message,
            "The repeated fallback text came from a response-finalization loop; I will answer directly now.",
            true,
        ));
        assert!(!direct_gate_recovery_response_answers_user(
            message,
            "I will answer directly now.",
            true,
        ));
    }

    #[test]
    fn direct_llm_response_preservation_has_no_runtime_fallback_path() {
        let mut workflow = json!({
            "final_llm_response": {}
        });
        preserve_direct_llm_response_without_fallback(&mut workflow, "Direct response");
        assert_eq!(
            workflow.get("response").and_then(Value::as_str),
            Some("Direct response")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/source")
                .and_then(Value::as_str),
            Some("initial_llm_response")
        );
        assert!(workflow
            .pointer("/final_llm_response/fallback_source")
            .is_none());
    }

    #[test]
    fn workflow_unexpected_state_retry_boilerplate_detector_catches_legacy_copy() {
        let retry_boilerplate = "I completed the workflow gate, but the final workflow state was unexpected. Please retry so I can rerun the chain cleanly.";
        assert!(response_contains_unexpected_state_retry_boilerplate(
            retry_boilerplate
        ));
    }

    #[test]
    fn workflow_unexpected_state_retry_boilerplate_detector_catches_next_actions_template() {
        let retry_boilerplate = "I completed the workflow gate, but the final workflow state was unexpected. Please retry so I can rerun the chain cleanly.\n\nNext actions: 1) clarify the exact outcome you want 2) run one targeted tool call 3) return a concise answer from current context";
        assert!(response_contains_unexpected_state_retry_boilerplate(
            retry_boilerplate
        ));
    }

    #[test]
    fn workflow_unexpected_state_retry_boilerplate_detector_catches_paraphrased_macro_bundle() {
        let retry_boilerplate = "Workflow gate completed but the final workflow state was unexpected. Next actions: run one targeted tool call, then provide a concise answer from current context.";
        assert!(response_contains_unexpected_state_retry_boilerplate(
            retry_boilerplate
        ));
    }

    #[test]
    fn workflow_unexpected_state_retry_boilerplate_detector_does_not_flag_plain_retry_offer() {
        let normal_text =
            "I can retry the query if you want, or I can answer directly from current context.";
        assert!(!response_contains_unexpected_state_retry_boilerplate(
            normal_text
        ));
    }

    #[test]
    fn workflow_unexpected_state_retry_boilerplate_detector_catches_policy_gate_outage_template() {
        let retry_boilerplate = "The File List step was blocked before I could finish the answer: This is a policy gate, not a web-provider outage.";
        assert!(response_contains_unexpected_state_retry_boilerplate(
            retry_boilerplate
        ));
        assert!(workflow_response_repetition_breaker_active(
            retry_boilerplate
        ));
    }

    #[test]
    fn workflow_unexpected_state_retry_boilerplate_detector_catches_runtime_capability_surface_template(
    ) {
        let retry_boilerplate = "I can access runtime telemetry, persistent memory, workspace files, channels, and approved command surfaces in this session.";
        assert!(response_contains_unexpected_state_retry_boilerplate(
            retry_boilerplate
        ));
        assert!(workflow_response_repetition_breaker_active(
            retry_boilerplate
        ));
    }

    #[test]
    fn workflow_unexpected_state_retry_boilerplate_detector_catches_route_classification_template()
    {
        let retry_boilerplate = "The first gate (\"workflow_route\") is still classifying this as an \"info\" route rather than a \"task\" route, which means it's still seeing this as a conversational exchange rather than a tool operation request. The system needs explicit tool-related phrasing to trigger the task classification path.";
        assert!(response_contains_unexpected_state_retry_boilerplate(
            retry_boilerplate
        ));
        assert!(workflow_response_repetition_breaker_active(
            retry_boilerplate
        ));
    }

    #[test]
    fn manual_toolbox_gate_context_prompt_is_context_only() {
        let prompt = manual_toolbox_gate_context_user_prompt(
            "Research Mastra for TypeScript agent workflows.",
        );

        assert!(prompt.contains("Context-only user message."));
        assert!(prompt.contains("Do not answer it directly."));
        assert!(prompt.contains("Research Mastra for TypeScript agent workflows."));
    }

    #[test]
    fn workflow_failure_diagnostic_records_when_retry_boilerplate_reject_was_seen() {
        let tools = vec![json!({
            "name": "file_list",
            "blocked": false
        })];
        assert!(should_record_workflow_failure_diagnostic(
            "unexpected_state_retry_boilerplate",
            "",
            "",
            &tools,
            false,
        ));
    }

    #[test]
    fn workflow_failure_diagnostic_records_when_policy_block_tool_is_present() {
        let tools = vec![json!({
            "name": "file_list",
            "blocked": true,
            "result": "lease_denied:client_ingress_domain_boundary"
        })];
        assert!(should_record_workflow_failure_diagnostic(
            "", "", "", &tools, false
        ));
    }

    #[test]
    fn workflow_failure_diagnostic_records_when_latest_reply_is_legacy_copy() {
        let tools = vec![json!({
            "name": "file_list",
            "blocked": false
        })];
        assert!(should_record_workflow_failure_diagnostic(
            "",
            "",
            "I completed the workflow gate, but the final workflow state was unexpected. Please retry so I can rerun the chain cleanly.",
            &tools,
            false,
        ));
    }

    #[test]
    fn workflow_failure_diagnostic_records_when_invalid_excerpt_has_retry_boilerplate() {
        let tools = vec![json!({
            "name": "file_list",
            "blocked": false
        })];
        assert!(should_record_workflow_failure_diagnostic(
            "",
            "final reply did not render; please retry so i can rerun the chain cleanly",
            "",
            &tools,
            false,
        ));
    }

    #[test]
    fn recent_assistant_retry_loop_detector_triggers_on_two_of_last_three_assistant_turns() {
        let messages = vec![
            json!({"role": "assistant", "text": "I completed the workflow gate, but the final workflow state was unexpected. Please retry so I can rerun the chain cleanly."}),
            json!({"role": "user", "text": "what?"}),
            json!({"role": "assistant", "text": "Workflow gate completed but the final workflow state was unexpected. Next actions: run one targeted tool call, then provide a concise answer from current context."}),
            json!({"role": "assistant", "text": "Normal answer now."}),
        ];
        assert!(recent_assistant_retry_loop_detected(&messages));
    }

    #[test]
    fn recent_assistant_retry_loop_detector_ignores_single_retry_like_turn() {
        let messages = vec![
            json!({"role": "assistant", "text": "I completed the workflow gate, but the final workflow state was unexpected. Please retry so I can rerun the chain cleanly."}),
            json!({"role": "assistant", "text": "I can answer directly from current context."}),
            json!({"role": "assistant", "text": "Here is the direct answer."}),
        ];
        assert!(!recent_assistant_retry_loop_detected(&messages));
    }

    #[test]
    fn workflow_failure_diagnostic_records_when_recent_loop_detected() {
        let tools = vec![json!({
            "name": "file_list",
            "blocked": false
        })];
        assert!(should_record_workflow_failure_diagnostic(
            "",
            "",
            "normal latest",
            &tools,
            true
        ));
    }

    #[test]
    fn direct_response_preservation_returns_none_for_empty_draft() {
        assert!(direct_llm_response_from_initial_draft("").is_none());
    }

    #[test]
    fn direct_response_preservation_keeps_clean_llm_text() {
        assert_eq!(
            direct_llm_response_from_initial_draft("All checks look healthy."),
            Some("All checks look healthy.".to_string())
        );
    }

    #[test]
    fn direct_response_preservation_withholds_private_gate_choice() {
        assert!(direct_llm_response_from_initial_draft("Need tools? Yes").is_none());
    }

    #[test]
    fn direct_response_preservation_extracts_structured_gate_final_answer() {
        assert_eq!(
            direct_llm_response_from_initial_draft(
                r#"{"gate":"2","final_answer":"Synthesized tradeoffs."}"#
            ),
            Some("Synthesized tradeoffs.".to_string())
        );
    }

    #[test]
    fn response_repeat_detector_catches_near_duplicate_formatting_variants() {
        let latest = "I'm not hard-locked. The previous fallback repeated, so I'm switching to a plain direct response path and avoiding extra tool calls unless you explicitly request one.";
        let response = "Im not hard locked - the previous fallback repeated so im switching to a plain direct response path and avoiding extra tool calls unless you explicitly request one";
        assert!(response_repeats_latest_assistant_copy(response, latest));
    }

    #[test]
    fn direct_llm_response_preservation_rejects_private_gate_tokens() {
        let mut workflow = json!({
            "final_llm_response": {}
        });
        preserve_direct_llm_response_without_fallback(&mut workflow, "Need tools? Yes");
        assert_eq!(
            workflow
                .pointer("/final_llm_response/used")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert!(workflow.get("response").is_none());
        assert!(workflow
            .pointer("/final_llm_response/fallback_source")
            .is_none());
    }
