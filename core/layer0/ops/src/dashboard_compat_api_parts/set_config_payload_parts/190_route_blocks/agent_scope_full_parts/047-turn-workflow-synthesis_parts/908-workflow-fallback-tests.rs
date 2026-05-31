    #[test]
    fn record_workflow_diagnostic_event_tracks_history_and_counter() {
        let mut workflow = json!({
            "final_llm_response": {},
            "quality_telemetry": {}
        });
        record_workflow_diagnostic_event(
            &mut workflow,
            "retry_boilerplate_diagnostic",
            "final_retry_diagnostic",
        );
        record_workflow_diagnostic_event(
            &mut workflow,
            "empty_response_presence_diagnostic",
            "final_presence_diagnostic",
        );
        record_workflow_diagnostic_event(
            &mut workflow,
            "retry_boilerplate_diagnostic",
            "synthesis_failure_diagnostic",
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_reason")
                .and_then(Value::as_str),
            Some("retry_boilerplate_diagnostic")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_last_stage")
                .and_then(Value::as_str),
            Some("synthesis_failure_diagnostic")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_reasons/0")
                .and_then(Value::as_str),
            Some("retry_boilerplate_diagnostic")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_reasons/1")
                .and_then(Value::as_str),
            Some("empty_response_presence_diagnostic")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_events/0/stage")
                .and_then(Value::as_str),
            Some("final_retry_diagnostic")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_events/1/stage")
                .and_then(Value::as_str),
            Some("final_presence_diagnostic")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_events/2/stage")
                .and_then(Value::as_str),
            Some("synthesis_failure_diagnostic")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_stages/0")
                .and_then(Value::as_str),
            Some("final_retry_diagnostic")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_stages/1")
                .and_then(Value::as_str),
            Some("final_presence_diagnostic")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_stages/2")
                .and_then(Value::as_str),
            Some("synthesis_failure_diagnostic")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_multi_stage")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_summary/trigger_count")
                .and_then(Value::as_u64),
            Some(3)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_summary/distinct_reason_count")
                .and_then(Value::as_u64),
            Some(2)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_summary/distinct_stage_count")
                .and_then(Value::as_u64),
            Some(3)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_summary/multi_stage")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_summary/severity")
                .and_then(Value::as_str),
            Some("high")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_summary/requires_operator_review")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_summary/escalation_reason")
                .and_then(Value::as_str),
            Some("high_trigger_or_stage_diversity")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_summary/recommended_action")
                .and_then(Value::as_str),
            Some("operator_review_recommended")
        );
        assert_eq!(
            workflow
                .pointer("/quality_telemetry/diagnostic_event_trigger_count")
                .and_then(Value::as_u64),
            Some(3)
        );
        assert_eq!(
            workflow
                .pointer("/quality_telemetry/diagnostic_event_stage_final_retry_diagnostic")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            workflow
                .pointer("/quality_telemetry/diagnostic_event_stage_final_presence_diagnostic")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            workflow
                .pointer("/quality_telemetry/diagnostic_event_stage_synthesis_failure_diagnostic")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            workflow
                .pointer("/quality_telemetry/diagnostic_event_reason_retry_boilerplate_diagnostic")
                .and_then(Value::as_u64),
            Some(2)
        );
        assert_eq!(
            workflow
                .pointer(
                    "/quality_telemetry/diagnostic_event_reason_empty_response_presence_diagnostic"
                )
                .and_then(Value::as_u64),
            Some(1)
        );
    }

    #[test]
    fn workflow_diagnostic_stage_counter_key_sanitizes_non_alnum_stage_tokens() {
        assert_eq!(
            workflow_diagnostic_stage_counter_key("Final Presence Guard!!"),
            "diagnostic_event_stage_final_presence_diagnostic"
        );
        assert_eq!(
            workflow_diagnostic_stage_counter_key("___"),
            "diagnostic_event_stage_unknown"
        );
    }

    #[test]
    fn workflow_diagnostic_reason_counter_key_sanitizes_non_alnum_reason_tokens() {
        assert_eq!(
            workflow_diagnostic_reason_counter_key("Retry Boilerplate Guard!!"),
            "diagnostic_event_reason_retry_boilerplate_diagnostic"
        );
        assert_eq!(
            workflow_diagnostic_reason_counter_key("___"),
            "diagnostic_event_reason_unknown"
        );
    }

    #[test]
    fn workflow_diagnostic_summary_classification_escalates_with_counts() {
        assert_eq!(
            workflow_diagnostic_summary_classification(1, 1),
            (
                "low",
                false,
                "single_guard_activation",
                "continue_direct_mode"
            )
        );
        assert_eq!(
            workflow_diagnostic_summary_classification(2, 1),
            (
                "moderate",
                false,
                "repeated_or_multi_stage_guard_activity",
                "monitor_and_continue_direct_mode",
            )
        );
        assert_eq!(
            workflow_diagnostic_summary_classification(1, 3),
            (
                "high",
                true,
                "high_trigger_or_stage_diversity",
                "operator_review_recommended",
            )
        );
    }

    #[test]
    fn final_synthesis_attempt_limit_is_cd_owned_for_tool_backed_answers() {
        let workflow = json!({
            "selected_workflow": {
                "tool_menu_interface_contract": {
                    "final_synthesis_attempt_limit": 2
                }
            }
        });
        let tools = vec![json!({
            "name": "batch_query",
            "status": "ok",
            "result": "source-backed finding"
        })];
        assert_eq!(workflow_final_synthesis_attempt_limit(&workflow, &tools), 2);
        assert_eq!(workflow_final_synthesis_attempt_limit(&workflow, &[]), 1);
    }

    #[test]
    fn final_synthesis_attempt_limit_is_bounded_runtime_execution() {
        let workflow = json!({
            "selected_workflow": {
                "tool_menu_interface_contract": {
                    "final_synthesis_attempt_limit": 99
                }
            }
        });
        let tools = vec![json!({"name": "batch_query"})];
        assert_eq!(workflow_final_synthesis_attempt_limit(&workflow, &tools), 3);
    }

    #[test]
    fn final_empty_response_diagnostic_preserves_non_empty_response() {
        let mut workflow = json!({
            "response": "Answer already present.",
            "quality_telemetry": {},
            "final_llm_response": {
                "used": true,
                "status": "synthesized"
            }
        });
        let tools = vec![json!({
            "name": "file_list",
            "blocked": false
        })];
        apply_final_empty_response_diagnostic(&mut workflow, "hello", "", &tools);
        assert_eq!(
            workflow.get("response").and_then(Value::as_str),
            Some("Answer already present.")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/status")
                .and_then(Value::as_str),
            Some("synthesized")
        );
    }

    #[test]
    fn finish_turn_workflow_final_response_after_attempts_never_finishes_blank() {
        let root = tempfile::tempdir().expect("tempdir");
        let workflow = turn_workflow_metadata("research_synthesize_verify", &[], &[], "", "hello");
        let gate_state = WorkflowFinalResponseGateState::default();
        let diagnostics = WorkflowFinalResponseDiagnostics {
            last_error: "invoke_failed".to_string(),
            ..Default::default()
        };

        let finalized = finish_turn_workflow_final_response_after_attempts(
            root.path(),
            workflow,
            "hello",
            "",
            &[],
            false,
            1,
            false,
            &gate_state,
            &diagnostics,
        );

        let response = workflow_visible_response_candidate(&finalized);
        assert!(!response.trim().is_empty(), "{response}");
        assert_eq!(
            finalized
                .pointer("/final_llm_response/status")
                .and_then(Value::as_str),
            Some("terminal_presence_fallback_used")
        );
    }

    #[test]
    fn runtime_visible_fallback_blocker_only_blocks_tool_evidence_suppression() {
        assert!(workflow_status_blocks_runtime_visible_fallback(
            "tool_evidence_fallback_suppressed"
        ));
        assert!(!workflow_status_blocks_runtime_visible_fallback(
            "tool_evidence_fallback_used"
        ));
        assert!(!workflow_status_blocks_runtime_visible_fallback(
            "synthesized"
        ));
        assert!(!workflow_status_blocks_runtime_visible_fallback(
            "synthesis_failed"
        ));
    }

    #[test]
    fn successful_tool_result_answer_is_not_rejected_as_missing_direct_answer() {
        let tools = vec![json!({
            "name": "batch_query",
            "status": "ok",
            "is_error": false,
            "blocked": false,
            "result": "Key findings: OpenHands is an AI agent platform for software development."
        })];

        assert!(response_answers_successful_tool_result(
            "Use web search to find one current source about OpenHands agent framework, then summarize it in one sentence.",
            "OpenHands is an AI agent platform focused on software-development automation.",
            &tools,
        ));
    }

    #[test]
    fn successful_tool_result_answer_rejects_ack_without_findings() {
        let tools = vec![json!({
            "name": "batch_query",
            "status": "ok",
            "is_error": false,
            "blocked": false,
            "result": "Key findings: OpenHands is an AI agent platform for software development."
        })];

        assert!(!response_answers_successful_tool_result(
            "Use web search to find one current source about OpenHands agent framework, then summarize it in one sentence.",
            "I found some results and will summarize them now.",
            &tools,
        ));
    }

    #[test]
    fn successful_tool_result_answer_rejects_unrelated_context_dump() {
        let tools = vec![json!({
            "name": "batch_query",
            "status": "ok",
            "is_error": false,
            "blocked": false,
            "result": "Key findings: From web retrieval: openhands.dev describes OpenHands as an AI agent platform for software development."
        })];

        assert!(!response_answers_successful_tool_result(
            "Use web search to find one current source about OpenHands agent framework, then summarize it in one sentence.",
            "# 第一章\n\n社会管理创新，是指在现有社会管理条件下，运用现有的资源和经验。",
            &tools,
        ));
    }

    #[test]
    fn raw_tool_payload_dump_is_rejected_before_visible_chat() {
        assert!(response_looks_like_raw_tool_payload_dump(
            "<?xml version=\"1.0\"?><CustomMetadata xmlns=\"urn:test\"></CustomMetadata>"
        ));
        assert!(response_looks_like_raw_tool_payload_dump(
            "<function=web_search>{\"query\":\"x\"}</function>"
        ));
        assert!(response_looks_like_raw_tool_payload_dump(
            "{\"tool\":\"web_search\",\"query\":\"latest web frameworks\"}"
        ));
        assert!(response_looks_like_raw_tool_payload_dump(
            "{\"name\":\"batch_query\",\"status\":\"ok\",\"result\":\"items\"}"
        ));
        assert!(response_looks_like_raw_tool_payload_dump(
            "{\"query\":\"agentic frameworks\",\"source\":\"web\",\"results\":[\"a\",\"b\"]}"
        ));
        assert!(response_looks_like_raw_tool_payload_dump(
            "[{\"tool\":\"web_search\",\"query\":\"foo\"},{\"tool\":\"web_fetch\",\"query\":\"bar\"}]"
        ));
        assert!(response_looks_like_raw_tool_payload_dump(
            "<tool>web_search</tool><query>foo</query>"
        ));
        assert!(response_looks_like_raw_tool_payload_dump(
            "{\"choices\":[{\"finish_reason\":\"length\",\"index\":0}],\"usage\":{\"prompt_tokens\":11,\"completion_tokens\":30,\"total_tokens\":41},\"refusal\":\"I am Kimi, an AI assistant created by Moonshot AI.\"}"
        ));
        assert!(!response_looks_like_raw_tool_payload_dump(
            "OpenHands is an AI agent platform for software development."
        ));
        assert!(!response_looks_like_raw_tool_payload_dump(
            "{\"answer\":\"OpenHands is an AI agent platform for software development.\"}"
        ));
    }

    #[test]
    fn workflow_prompt_analysis_is_rejected_before_visible_chat() {
        assert!(response_contains_workflow_prompt_analysis_leak(
            "According to the instructions, the gate is What kind of work is this? The user asks to respond directly, so we answer normally."
        ));
        assert!(response_contains_workflow_prompt_analysis_leak(
            "We are in the runtime context of 2026-05-02T06:14:40Z. The user asks for a reply in exactly five words. We must reply in one short sentence."
        ));
        assert!(response_contains_workflow_prompt_analysis_leak(
            "I need to analyze the recorded evidence state: - `recorded_evidence_available`: true - `recorded_tool_result_quality`: \"partial_or_low_signal_evidence\"."
        ));
        assert!(response_contains_workflow_prompt_analysis_leak(
            "The synthesis input envelope shows evidence_claims is empty, so choose bounded_partial_answer."
        ));
        assert_eq!(
            direct_llm_response_from_initial_draft(
                "According to the instructions, the gate is What kind of work is this? The user asks to respond directly, so we answer normally."
            ),
            Some("According to the instructions, the gate is What kind of work is this? The user asks to respond directly, so we answer normally.".to_string())
        );
    }

    #[test]
    fn dry_run_hypothetical_tool_choice_is_not_prompt_analysis_leak() {
        let message = "Dry run only: tell me which file tool you would use, but do not run tools yet.";
        let response = "The user asks for a file operation in dry-run mode, so the appropriate tool would typically be a filesystem/file tool.";

        assert!(response_contains_workflow_prompt_analysis_leak(response));
        assert!(!response_contains_workflow_prompt_analysis_leak_for_message(
            message,
            response,
        ));
        assert!(response_contains_workflow_prompt_analysis_leak_for_message(
            message,
            "According to the instructions, the workflow gate says I would use the file tool.",
        ));
    }

    #[test]
    fn numeric_workflow_gate_submission_selects_json_alias_category() {
        assert!(response_is_tool_bearing_category_gate_submission("3"));
        let (category_key, category_label) =
            workflow_category_selection(&default_workflow_tool_menu_contract(), "3", Some(true))
                .expect("numeric web research alias");

        assert_eq!(category_key, "web_research");
        assert_eq!(category_label, "Web research");
    }

    #[test]
    fn structured_response_gate_fragment_selects_json_alias_category() {
        let response = r#""response_gate": "3"}"#;

        assert!(response_is_tool_bearing_category_gate_submission(response));
        let (category_key, category_label) = workflow_category_selection(
            &default_workflow_tool_menu_contract(),
            response,
            Some(true),
        )
        .expect("response_gate web research alias");

        assert_eq!(category_key, "web_research");
        assert_eq!(category_label, "Web research");
        assert!(response_is_tool_bearing_category_gate_submission(
            r#"{"gate": 3}"#
        ));
        assert!(response_is_tool_bearing_category_gate_submission(
            r#""workflow_gate": 3}"#
        ));
    }

    #[test]
    fn structured_gate_submission_accepts_combined_option_and_label_token() {
        let response = r#"{"gate":"4 = Workspace/files"}"#;

        assert!(response_is_tool_bearing_category_gate_submission(response));
        let (category_key, category_label) = workflow_category_selection(
            &default_workflow_tool_menu_contract(),
            response,
            Some(true),
        )
        .expect("combined category token");

        assert_eq!(category_key, "workspace_files");
        assert_eq!(category_label, "Workspace/files");
    }
