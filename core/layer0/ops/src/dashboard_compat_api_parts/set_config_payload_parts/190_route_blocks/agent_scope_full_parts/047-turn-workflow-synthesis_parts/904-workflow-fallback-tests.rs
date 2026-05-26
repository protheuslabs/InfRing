    #[test]
    fn workflow_final_visible_response_text_unwraps_final_answer_gate_payload() {
        assert_eq!(
            workflow_final_visible_response_text(
                r#"{"gate":"4","final_answer":"Based on the results: ..."}"#
            ),
            "Based on the results: ..."
        );
        assert_eq!(
            workflow_final_visible_response_text(
                r#"{"tool_family":"web_research","tool":"web_search","request_payload":{"query":"compare frameworks","aperture":"medium"}}"#
            ),
            ""
        );
        assert_eq!(
            workflow_final_visible_response_text(
                r#"{"tool_family":"web_research","tool":"batch_query","request_payload":{"source":"web","query":"Compare LangGraph vs CrewAI on reliability and deployment.","queries":["LangGraph official docs reliability deployment","CrewAI official docs reliability deployment"],"aperture":"medium"}}"#
            ),
            ""
        );
        assert_eq!(
            workflow_final_visible_response_text("Plain natural language answer."),
            "Plain natural language answer."
        );
    }

    #[test]
    fn final_retry_boilerplate_diagnostic_rewrites_response_and_sets_metadata() {
        let mut workflow = json!({
            "response": "I completed the workflow gate, but the final workflow state was unexpected. Please retry so I can rerun the chain cleanly.",
            "quality_telemetry": {},
            "final_llm_response": {
                "used": false,
                "status": "synthesis_failed"
            }
        });
        let tools = vec![json!({
            "name": "file_list",
            "blocked": true,
            "result": "lease_denied:client_ingress_domain_boundary"
        })];
        apply_final_retry_boilerplate_diagnostic(
            &mut workflow,
            "hello",
            "I completed the workflow gate, but the final workflow state was unexpected. Please retry so I can rerun the chain cleanly.",
            &tools,
        );
        let response = workflow
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        assert!(
            response.contains("retrieved evidence in this turn was not strong enough"),
            "{response}"
        );
        assert!(!response.contains("please retry"), "{response}");
        assert!(!response.contains("workflow gate"), "{response}");
        assert_eq!(
            workflow
                .pointer("/final_llm_response/status")
                .and_then(Value::as_str),
            Some("guard_violation_rewritten")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/visible_response_preserved")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/replacement_response_used")
                .and_then(Value::as_bool),
            Some(true)
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
            Some("final_retry_diagnostic")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_reasons/0")
                .and_then(Value::as_str),
            Some("retry_boilerplate_diagnostic")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_events/0/stage")
                .and_then(Value::as_str),
            Some("final_retry_diagnostic")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_stages/0")
                .and_then(Value::as_str),
            Some("final_retry_diagnostic")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_multi_stage")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_summary/trigger_count")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_summary/severity")
                .and_then(Value::as_str),
            Some("low")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_summary/requires_operator_review")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_summary/escalation_reason")
                .and_then(Value::as_str),
            Some("single_guard_activation")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_summary/recommended_action")
                .and_then(Value::as_str),
            Some("continue_direct_mode")
        );
        assert_eq!(
            workflow
                .pointer("/quality_telemetry/diagnostic_event_trigger_count")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            workflow
                .pointer("/quality_telemetry/diagnostic_event_stage_final_retry_diagnostic")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            workflow
                .pointer("/quality_telemetry/diagnostic_event_reason_retry_boilerplate_diagnostic")
                .and_then(Value::as_u64),
            Some(1)
        );
    }

    #[test]
    fn final_retry_boilerplate_diagnostic_rewrites_nested_visible_response_fields() {
        let mut workflow = json!({
            "response": "",
            "text": "",
            "message": "",
            "response_finalization": {
                "final_response": {
                    "text": "This retrieval attempt did not produce enough relevant evidence to answer the question well. Recorded evidence so far: Here's what I found: web search returned low-signal snippets."
                }
            },
            "response_workflow": {
                "final_llm_response": {
                    "text": "This retrieval attempt did not produce enough relevant evidence to answer the question well. Recorded evidence so far: Here's what I found: web search returned low-signal snippets."
                }
            },
            "quality_telemetry": {},
            "final_llm_response": {
                "used": false,
                "status": "synthesis_failed"
            }
        });
        let tools = vec![json!({
            "name": "batch_query",
            "status": "ok",
            "result": "Web retrieval ran, but only low-signal snippets were available for synthesis in this turn.",
            "tool_result_quality": {
                "status": "low_signal"
            }
        })];
        apply_final_retry_boilerplate_diagnostic(
            &mut workflow,
            "Find recent benchmarks comparing agent frameworks.",
            "",
            &tools,
        );
        let response = workflow
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        assert!(response.contains("retrieved evidence in this turn was not strong enough"));
        assert_eq!(
            workflow
                .pointer("/response_finalization/final_response/text")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_ascii_lowercase(),
            response
        );
        assert_eq!(
            workflow
                .pointer("/response_workflow/final_llm_response/text")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_ascii_lowercase(),
            response
        );
    }

    #[test]
    fn final_empty_response_diagnostic_records_metadata() {
        let mut workflow = json!({
            "response": "",
            "quality_telemetry": {},
            "final_llm_response": {
                "used": false,
                "status": "synthesis_failed"
            }
        });
        let tools = Vec::<Value>::new();
        apply_final_empty_response_diagnostic(&mut workflow, "hello", "", &tools);
        let response = workflow
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(
            response.contains("do not have enough reliable information"),
            "{response}"
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/status")
                .and_then(Value::as_str),
            Some("terminal_presence_fallback_used")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_source")
                .and_then(Value::as_str),
            Some("terminal_presence_fallback")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/replacement_response_used")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            workflow
                .pointer("/quality_telemetry/final_terminal_fallback_used")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_reason")
                .and_then(Value::as_str),
            Some("empty_response_presence_diagnostic")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_last_stage")
                .and_then(Value::as_str),
            Some("final_presence_diagnostic")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_reasons/0")
                .and_then(Value::as_str),
            Some("empty_response_presence_diagnostic")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_events/0/stage")
                .and_then(Value::as_str),
            Some("final_presence_diagnostic")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_stages/0")
                .and_then(Value::as_str),
            Some("final_presence_diagnostic")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_multi_stage")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_summary/trigger_count")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_summary/severity")
                .and_then(Value::as_str),
            Some("low")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_summary/requires_operator_review")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_summary/escalation_reason")
                .and_then(Value::as_str),
            Some("single_guard_activation")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/diagnostic_event_summary/recommended_action")
                .and_then(Value::as_str),
            Some("continue_direct_mode")
        );
        assert_eq!(
            workflow
                .pointer("/quality_telemetry/diagnostic_event_trigger_count")
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
                .pointer(
                    "/quality_telemetry/diagnostic_event_reason_empty_response_presence_diagnostic"
                )
                .and_then(Value::as_u64),
            Some(1)
        );
    }

    #[test]
    fn final_empty_response_diagnostic_uses_generic_tool_evidence_fallback_when_findings_exist()
    {
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
            "status": "ok",
            "is_error": false,
            "blocked": false,
            "result": "Top findings: OpenHands is an AI coding agent platform with strong automation capabilities."
        })];

        apply_final_empty_response_diagnostic(
            &mut workflow,
            "Compare two documentation tools for a small team.",
            "",
            &tools,
        );
        let response = workflow
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(!response.trim().is_empty(), "{response}");
        assert!(response.contains("partial conclusion") || response.contains("OpenHands"), "{response}");
        assert_eq!(
            workflow
                .pointer("/final_llm_response/used")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/status")
                .and_then(Value::as_str),
            Some("tool_evidence_fallback_used")
        );
        assert_eq!(
            workflow
                .pointer("/final_llm_response/fallback_source")
                .and_then(Value::as_str),
            Some("tool_evidence_runtime_fallback")
        );
        assert_eq!(
            workflow
                .pointer("/quality_telemetry/final_fallback_used")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            workflow
                .pointer("/quality_telemetry/final_fallback_suppressed")
                .and_then(Value::as_bool),
            Some(false)
        );
    }

    #[test]
    fn final_empty_response_diagnostic_uses_tool_evidence_fallback_when_failure_exists() {
        let mut workflow = json!({
            "response": "",
            "quality_telemetry": {},
            "final_llm_response": {
                "used": false,
                "status": "synthesized"
            }
        });
        let tools = vec![json!({
            "name": "web_search",
            "status": "error",
            "is_error": true,
            "blocked": false,
            "error": "search service returned timeout"
        })];

        apply_final_empty_response_diagnostic(
            &mut workflow,
            "Find latest agentic frameworks",
            "",
            &tools,
        );
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
                .pointer("/final_llm_response/replacement_response_excerpt")
                .and_then(Value::as_str),
            Some("My recommendation is to treat the current evidence as insufficient for a direct source-backed conclusion.")
        );
    }
