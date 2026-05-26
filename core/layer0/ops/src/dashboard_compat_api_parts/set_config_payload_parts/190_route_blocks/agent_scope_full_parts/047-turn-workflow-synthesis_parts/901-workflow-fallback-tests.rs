    #[test]
    fn direct_llm_no_tool_answer_updates_array_gates_without_panic() {
        let mut workflow = json!({
            "tool_gate": {
                "gates": [
                    {"gate_id": "gate_1_work_category_menu", "submission_status": "presented"}
                ]
            },
            "stage_statuses": [
                {"stage": "gate_1_work_category_menu", "status": "presented"}
            ]
        });

        mark_workflow_direct_llm_no_tool_answer(&mut workflow);

        assert_eq!(
            workflow
                .pointer("/tool_gate/gates/0/submission_status")
                .and_then(Value::as_str),
            Some("submitted")
        );
        assert_eq!(
            workflow
                .pointer("/tool_gate/gates/0/decision_source")
                .and_then(Value::as_str),
            Some("llm_direct_answer")
        );
        assert_eq!(
            workflow
                .pointer("/stage_statuses/0/status")
                .and_then(Value::as_str),
            Some("answered_no_tool_category")
        );
    }

    #[test]
    fn manual_toolbox_selection_parses_pending_web_request() {
        let pending = manual_toolbox_pending_request_from_response(
            "Category: Web research. Tool family: Web research. Tool: web_search. Request payload: {\"query\":\"compare infring\",\"aperture\":\"medium\"}.",
            "Compare this platform to a current external tool category.",
        )
        .expect("pending request");

        assert_eq!(
            pending.get("status").and_then(Value::as_str),
            Some("pending_confirmation")
        );
        assert_eq!(
            pending.get("tool_name").and_then(Value::as_str),
            Some("web_search")
        );
        assert_eq!(
            pending.pointer("/input/query").and_then(Value::as_str),
            Some("compare infring")
        );
        assert_eq!(
            pending
                .get("execution_claim_allowed")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert!(pending
            .get("receipt_binding")
            .and_then(Value::as_str)
            .map(|value| !value.is_empty())
            .unwrap_or(false));
    }

    #[test]
    fn manual_toolbox_selection_requires_explicit_payload_submission() {
        let pending = manual_toolbox_pending_request_from_response(
            "Category: Web research. Tool family: Web research. Tool: web_search.",
            "Compare this platform to a current external tool category.",
        );
        assert!(pending.is_none());
    }

    #[test]
    fn manual_toolbox_selection_rejects_non_catalog_tool_names() {
        let pending = manual_toolbox_pending_request_from_response(
            "Category: Web research. Tool family: Search. Tool: Keyword search. Request payload: {\"keywords\":\"infring frameworks\"}.",
            "Compare this platform to current external tools.",
        );
        assert!(pending.is_none());
    }

    #[test]
    fn manual_toolbox_selection_parses_json_tool_request() {
        let pending = manual_toolbox_pending_request_from_response(
            "{\"tool_family\": \"Web research\", \"tool\": \"web_search\", \"request_payload\": {\"query\": \"compare infring to top agentic frameworks\", \"aperture\":\"medium\"}, \"selection_source\": \"unit_test\"}",
            "Compare infring to top agentic frameworks.",
        )
        .expect("pending request");

        assert_eq!(
            pending.get("status").and_then(Value::as_str),
            Some("pending_confirmation")
        );
        assert_eq!(
            pending.get("tool_name").and_then(Value::as_str),
            Some("web_search")
        );
        assert_eq!(
            pending.pointer("/input/aperture").and_then(Value::as_str),
            Some("medium")
        );
        assert_eq!(
            pending.get("source").and_then(Value::as_str),
            Some("unit_test")
        );
    }

    #[test]
    fn invalid_visible_gate_like_draft_does_not_skip_private_gate_recovery() {
        let workflow = json!({});
        assert!(!initial_visible_gate_choice_submission_allowed(
            &[],
            false,
            &workflow
        ));
    }

    #[test]
    fn recovered_pending_request_allows_gate_recovery_shortcut() {
        let workflow = json!({
            "manual_toolbox_pending_tool_request": {
                "tool_name": "web_search",
                "input": {
                    "query": "mastra langgraph typescript",
                    "aperture": "medium"
                }
            }
        });
        assert!(initial_visible_gate_choice_submission_allowed(
            &[],
            false,
            &workflow
        ));
    }

    #[test]
    fn workflow_gate_stability_rows_score_direct_llm_response_as_final() {
        let workflow = json!({
            "selected_workflow": {
                "name": "simple_conversation_v1"
            },
            "workflow_control": {
                "direct_response_path": "first_gate_unresolved"
            },
            "tool_gate": {
                "selected_work_category": "respond_directly"
            },
            "tool_count": 0,
            "response": "Hey! How can I help you today?",
            "final_llm_response": {
                "used": true,
                "required": false,
                "status": "direct_llm_response"
            },
            "stage_statuses": [
                {
                    "stage": "gate_1_work_category_menu",
                    "status": "answered_no_tool_category"
                },
                {
                    "stage": "gate_6_llm_final_output",
                    "status": "skipped_not_required"
                }
            ]
        });
        let rows = workflow_gate_stability_rows(&workflow);

        assert_eq!(
            rows.iter()
                .find(|row| row.get("gate").and_then(Value::as_str)
                    == Some("gate_6_llm_final_output"))
                .and_then(|row| row.get("status").and_then(Value::as_str)),
            Some("passed")
        );
        assert_eq!(
            rows.iter()
                .find(|row| row.get("gate").and_then(Value::as_str)
                    == Some("gate_2_tool_family_menu"))
                .and_then(|row| row.get("status").and_then(Value::as_str)),
            Some("not_applicable")
        );
    }
    #[test]
    fn workflow_gate_stability_version_ring_keeps_latest_three_versions() {
        let root = std::env::temp_dir().join(format!(
            "workflow-gate-stability-ring-{}",
            crate::deterministic_receipt_hash(&json!({
                "test": "workflow_gate_stability_version_ring_keeps_latest_three_versions",
                "ts": crate::now_iso()
            }))
        ));
        let rows = vec![
            json!({
                "gate": "gate_1_work_category_menu",
                "status": "passed"
            }),
            json!({
                "gate": "gate_6_llm_final_output",
                "status": "failed"
            }),
        ];

        for (index, version_hash) in ["v1", "v2", "v3", "v4"].iter().enumerate() {
            let snapshot = json!({
                "name": "simple_conversation_v1",
                "workflow_version": version_hash
            });
            workflow_gate_stability_update_version_ring(
                &root,
                "simple_conversation_v1",
                version_hash,
                &snapshot,
                &rows,
                &format!("ts-{index}"),
            );
        }
        let v3_snapshot = json!({
            "name": "simple_conversation_v1",
            "workflow_version": "v3"
        });
        workflow_gate_stability_update_version_ring(
            &root,
            "simple_conversation_v1",
            "v3",
            &v3_snapshot,
            &rows,
            "ts-4",
        );

        let ring_path = root.join("local/state/ops/workflow_gate_stability/versions_ring.json");
        let ring = read_json_loose(&ring_path).expect("version ring json");
        let versions = ring
            .get("versions")
            .and_then(Value::as_array)
            .expect("versions array");

        assert_eq!(versions.len(), 3);
        assert_eq!(
            ring.get("current_version_hash").and_then(Value::as_str),
            Some("v3")
        );
        assert_eq!(
            versions
                .first()
                .and_then(|value| value.get("workflow_version_hash"))
                .and_then(Value::as_str),
            Some("v3")
        );
        assert_eq!(
            versions
                .first()
                .and_then(|value| value.get("turn_count"))
                .and_then(Value::as_u64),
            Some(2)
        );
        assert!(!versions.iter().any(|value| {
            value.get("workflow_version_hash").and_then(Value::as_str) == Some("v1")
        }));
        assert!(versions.iter().all(|value| {
            value
                .get("workflow_json")
                .and_then(|snapshot| snapshot.get("name"))
                .and_then(Value::as_str)
                == Some("simple_conversation_v1")
        }));
        assert!(root
            .join("local/state/ops/workflow_gate_stability/workflow_versions/v3.workflow.json")
            .exists());
        assert!(!root
            .join("local/state/ops/workflow_gate_stability/workflow_versions/v1.workflow.json")
            .exists());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn manual_toolbox_selection_rejects_json_request_without_payload() {
        let pending = manual_toolbox_pending_request_from_response(
            "{\"tool_family\": \"Web research\", \"tool\": \"web_search\"}",
            "Compare infring to top agentic frameworks.",
        );
        assert!(pending.is_none());
    }

    #[test]
    fn direct_llm_no_tool_answer_marks_trace_as_no_tool_category() {
        let mut workflow = json!({
            "workflow_control": {},
            "tool_gate": {
                "gates": {
                    "gate_1": {}
                }
            },
            "stage_statuses": [
                {"stage": "gate_1_work_category_menu", "status": "presented"},
                {"stage": "final_llm_response", "status": "pending_final_llm"}
            ]
        });

        mark_workflow_direct_llm_no_tool_answer(&mut workflow);

        assert_eq!(
            workflow
                .pointer("/workflow_control/direct_response_path")
                .and_then(Value::as_str),
            Some("first_gate_no_tool_category")
        );
        assert_eq!(
            workflow
                .pointer("/tool_gate/gate_1_submission_status")
                .and_then(Value::as_str),
            Some("submitted")
        );
        assert_eq!(
            workflow
                .pointer("/stage_statuses/0/status")
                .and_then(Value::as_str),
            Some("answered_no_tool_category")
        );
    }

    #[test]
    fn natural_language_tool_preference_does_not_create_pending_request() {
        let mut workflow = json!({
            "workflow_control": {},
            "system_events": []
        });
        record_manual_toolbox_pending_request(
            &mut workflow,
            "I would use web search to compare infring to other frameworks.",
            "Compare this platform to a current external tool category.",
        );
        assert!(workflow
            .get("manual_toolbox_pending_tool_request")
            .is_none());
    }

    #[test]
    fn manual_toolbox_candidate_menu_is_not_reported_as_no_tool_category() {
        let workflow = turn_workflow_metadata(
            "normal_turn",
            &[],
            &[turn_workflow_event(
                "manual_toolbox_candidate_menu",
                json!({"candidate_count": 1}),
            )],
            "",
            "Use web search for the exact comparison topic supplied by the user.",
        );

        assert_eq!(
            workflow
                .pointer("/workflow_control/direct_response_path")
                .and_then(Value::as_str),
            Some("first_gate_pending_llm_tool_choice")
        );
    }

    #[test]
    fn exact_tool_gate_submission_updates_workflow_path() {
        let mut workflow = turn_workflow_metadata(
            "normal_turn",
            &[],
            &[turn_workflow_event(
                "manual_toolbox_candidate_menu",
                json!({"candidate_count": 1}),
            )],
            "",
            "Use web search for the exact comparison topic supplied by the user.",
        );
        record_manual_toolbox_pending_request(
            &mut workflow,
            "Category: Web research. Tool family: Web research. Tool: web_search. Request payload: {\"query\":\"compare infring to agent frameworks\",\"aperture\":\"medium\"}.",
            "Use web search for the exact comparison topic supplied by the user.",
        );

        assert_eq!(
            workflow
                .pointer("/manual_toolbox_pending_tool_request/status")
                .and_then(Value::as_str),
            Some("pending_confirmation")
        );
        assert_eq!(
            workflow
                .pointer("/workflow_control/direct_response_path")
                .and_then(Value::as_str),
            Some("first_gate_pending_tool_confirmation")
        );
    }

    #[test]
    fn narrated_tool_choice_does_not_create_pending_request_even_with_menu() {
        let mut workflow = turn_workflow_metadata(
            "normal_turn",
            &[],
            &[turn_workflow_event(
                "manual_toolbox_candidate_menu",
                json!({"candidate_count": 1}),
            )],
            "",
            "Use web search for the exact comparison topic supplied by the user.",
        );
        record_manual_toolbox_pending_request(
            &mut workflow,
            "I would choose a menu item.",
            "Use web search for the exact comparison topic supplied by the user.",
        );

        assert!(workflow
            .pointer("/manual_toolbox_pending_tool_request")
            .is_none());
        assert_eq!(
            workflow
                .pointer("/workflow_control/direct_response_path")
                .and_then(Value::as_str),
            Some("first_gate_pending_llm_tool_choice")
        );
    }

    #[test]
    fn manual_toolbox_candidate_menu_detection_reads_kind_field() {
        let workflow = json!({
            "workflow_control": {},
            "system_events": [
                turn_workflow_event("manual_toolbox_candidate_menu", json!({"candidate_count": 1}))
            ]
        });

        assert!(workflow_has_manual_toolbox_candidate_menu(&workflow));
    }

