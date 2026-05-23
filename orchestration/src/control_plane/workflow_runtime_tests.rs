use super::workflow_runtime::{
    adapt_tool_request, run_registered_replay_fixtures, run_workflow_replay,
    workflow_runtime_contract_ok, workflow_runtime_terminal_outcome_ok,
};
use super::workflow_runtime_fixtures::{WorkflowInput, WorkflowReplayFixture};

#[test]
fn runtime_replays_cover_core_tool_and_abort_paths_without_chat_injection() {
    let reports = run_registered_replay_fixtures();
    assert!(workflow_runtime_contract_ok(&reports), "{reports:#?}");
    assert_eq!(reports.len(), 5);
    assert!(reports.iter().all(|report| {
        report
            .events
            .iter()
            .all(|event| event.stream == "final_answer" || !event.visible_chat_eligible)
    }));
}

#[test]
fn runtime_exports_separated_inspector_streams_and_budget_state() {
    let reports = run_registered_replay_fixtures();
    for report in reports {
        assert_eq!(
            report.inspector.selected_graph_source,
            "json_workflow_source_of_truth_v1"
        );
        assert_eq!(
            report.inspector.source_of_truth_schema_version,
            "workflow_source_of_truth_contract_v1"
        );
        assert_eq!(report.inspector.interaction_source, "json_workflow_spec");
        assert_eq!(
            report.inspector.rust_reader_role,
            "validate_execute_trace_only"
        );
        assert!(!report.inspector.hardcoded_interaction_behavior_allowed);
        assert!(!report.graph_hash.is_empty());
        assert!(report
            .source_json_path
            .starts_with("orchestration/src/control_plane/workflows/"));
        assert!(report.source_json_path.ends_with(".workflow.json"));
        assert_eq!(
            report.contract_schema_version,
            "typed_execution_contract_v1"
        );
        assert_eq!(report.inspector.workflow_id, report.workflow_id);
        assert_eq!(report.inspector.graph_hash, report.graph_hash);
        assert_eq!(report.inspector.source_json_path, report.source_json_path);
        assert_eq!(
            report.inspector.contract_schema_version,
            report.contract_schema_version
        );
        assert!(report.budget.loop_guard_active);
        assert!(!report.budget.budget_exceeded);
        assert!(!report.budget.loop_signature_repeated);
        for stream in [
            "workflow_state",
            "agent_internal_notes",
            "tool_trace",
            "eval_trace",
            "final_answer",
        ] {
            assert!(
                report.inspector.trace_streams.contains_key(stream),
                "missing stream {stream} for {}",
                report.fixture_id
            );
        }
        assert!(!report.inspector.system_chat_injection_allowed);
        assert!(report
            .inspector
            .tool_family_diagnostics
            .iter()
            .any(|row| row.family == "workspace"));
        assert!(report.inspector.tool_family_diagnostics.iter().all(|row| {
            row.status == "menu_available_probe_required_before_execution"
                && row.reason == "workflow_reader_exposes_family_without_autoselection"
        }));
    }
}

#[test]
fn llm_menu_adapter_builds_typed_tool_request_without_recommending_tools() {
    let request = adapt_tool_request("workspace", "search_workspace", "workflow policy")
        .expect("workspace request");
    assert_eq!(request.family, "workspace");
    assert_eq!(request.request_schema, "workspace_tool_request_v1");
    assert!(request.receipt_binding_required);
    assert!(adapt_tool_request("unknown", "search", "payload").is_err());
}

#[test]
fn unregistered_workflow_ids_fail_closed_instead_of_using_private_stage_graphs() {
    let report = run_workflow_replay(&WorkflowReplayFixture {
        id: "unregistered_private_stage_graph",
        workflow_id: "private_hardcoded_chat_stage_graph",
        user_input: "run a private hardcoded workflow",
        inputs: vec![WorkflowInput::FinalAnswer("should not run")],
    });

    assert!(!report.ok, "{report:#?}");
    assert_eq!(report.terminal_state, "failed");
    assert!(report
        .failures
        .iter()
        .any(|reason| reason == "unregistered_workflow_id:private_hardcoded_chat_stage_graph"));
    assert!(report.events.iter().any(|event| {
        event.stream == "eval_trace" && event.event_kind == "workflow_selection_rejected"
    }));
}

#[test]
fn successful_tool_observation_marks_pending_final_synthesis_before_final_answer() {
    let report = run_workflow_replay(&WorkflowReplayFixture {
        id: "pending_final_synthesis",
        workflow_id: "research_synthesize_verify",
        user_input: "Search the web and synthesize the result.",
        inputs: vec![
            WorkflowInput::GateText {
                stage: "gate_1_need_tool_access_menu",
                text: "Yes",
            },
            WorkflowInput::GateText {
                stage: "gate_2_tool_family_menu",
                text: "2",
            },
            WorkflowInput::GateText {
                stage: "gate_3_tool_menu",
                text: "web_search",
            },
            WorkflowInput::GateText {
                stage: "gate_4_request_payload_input",
                text: "current agent framework comparison",
            },
            WorkflowInput::ToolObservation {
                ok: true,
                summary: "web results returned",
            },
            WorkflowInput::FinalAnswer("Synthesized web findings."),
        ],
    });

    assert!(report.ok, "{report:#?}");
    assert!(report.events.iter().any(|event| {
        event.stream == "workflow_state" && event.event_kind == "pending_final_synthesis"
    }));
    assert_eq!(report.synthesis_inputs.len(), 1, "{report:#?}");
    assert!(report.events.iter().any(|event| {
        event.stream == "workflow_state" && event.event_kind == "synthesis_input_ready"
    }));
    assert!(report.events.iter().any(|event| {
        event.stream == "final_answer"
            && event
                .payload
                .pointer("/synthesis/synthesis_input_run_id")
                .and_then(|value| value.as_str())
                .is_some()
            && event
                .payload
                .pointer("/synthesis/quality_contract_present")
                .and_then(|value| value.as_bool())
                == Some(true)
    }));
    assert!(workflow_runtime_terminal_outcome_ok(&report));
}

#[test]
fn tool_result_reaches_synthesis_input_and_final_projection_without_fixture_final_answer() {
    let report = run_workflow_replay(&WorkflowReplayFixture {
        id: "tool_result_to_synthesis_projection",
        workflow_id: "research_synthesize_verify",
        user_input: "Research current scientific breakthroughs.",
        inputs: vec![
            WorkflowInput::GateText {
                stage: "gate_1_need_tool_access_menu",
                text: "Yes",
            },
            WorkflowInput::GateText {
                stage: "gate_2_tool_family_menu",
                text: "2",
            },
            WorkflowInput::GateText {
                stage: "gate_3_tool_menu",
                text: "web_search",
            },
            WorkflowInput::GateText {
                stage: "gate_4_request_payload_input",
                text: "scientific breakthroughs 2026",
            },
            WorkflowInput::ToolObservation {
                ok: true,
                summary: r#"{
                    "tool_result_quality": "usable",
                    "evidence_pack": [{
                        "source_kind": "browser_materialized_page",
                        "title": "Source A",
                        "locator": "https://example.test/battery",
                        "source_domain": "example.test",
                        "relevant_extract": "Source A reports a battery chemistry milestone.",
                        "claim_hints": ["Source A reports a battery chemistry milestone."]
                    }],
                    "evidence_claims": [{
                        "claim": "Source A reports a battery chemistry milestone.",
                        "support_snippet": "Source A reports a battery chemistry milestone.",
                        "source_ref": {
                            "title": "Source A",
                            "locator": "https://example.test/battery"
                        }
                    }],
                    "answer_units": [
                        "Source A reports a battery chemistry milestone. Source: Source A, example.test."
                    ]
                }"#,
            },
            WorkflowInput::SynthesizeFromLatestToolResult,
        ],
    });

    assert!(report.ok, "{report:#?}");
    assert_eq!(report.synthesis_inputs.len(), 1, "{report:#?}");
    let synthesis_input = &report.synthesis_inputs[0];
    assert_eq!(synthesis_input.workflow_id, "research_synthesize_verify");
    assert_eq!(
        synthesis_input.user_goal,
        "Research current scientific breakthroughs."
    );
    assert_eq!(synthesis_input.tool_result_quality, "usable");
    assert!(!synthesis_input.tool_receipt_refs.is_empty());
    assert!(!synthesis_input.evidence_refs.is_empty());
    assert_eq!(
        synthesis_input
            .evidence_pack
            .get("schema_version")
            .and_then(|value| value.as_str()),
        Some("synthesis_evidence_pack_v1")
    );
    assert_eq!(
        synthesis_input
            .evidence_pack
            .pointer("/source")
            .and_then(|value| value.as_str()),
        Some("workflow_runtime_structured_tool_observation")
    );
    assert_eq!(
        synthesis_input
            .evidence_pack
            .pointer("/answer_units/0")
            .and_then(|value| value.as_str()),
        Some("Source A reports a battery chemistry milestone. Source: Source A, example.test.")
    );
    assert_eq!(
        synthesis_input
            .final_output_contract
            .get("source")
            .and_then(|value| value.as_str()),
        Some("workflow_cd")
    );
    assert_eq!(
        synthesis_input
            .final_output_contract
            .get("quality_contract")
            .and_then(|value| value.get("format_freedom"))
            .and_then(|value| value.as_str())
            .is_some(),
        true
    );
    assert!(report.events.iter().any(|event| {
        event.stream == "workflow_state" && event.event_kind == "synthesis_input_ready"
    }));
    assert!(report.events.iter().any(|event| {
        event.stream == "final_answer"
            && event.event_kind == "llm_final_output"
            && event.payload.get("source").and_then(|value| value.as_str())
                == Some("deterministic_replay_synthesis_stub")
            && event
                .payload
                .pointer("/synthesis/quality_contract_present")
                .and_then(|value| value.as_bool())
                == Some(true)
    }));
    assert!(workflow_runtime_terminal_outcome_ok(&report));
}

#[test]
fn usable_summary_only_tool_result_fails_closed_before_deterministic_projection() {
    let report = run_workflow_replay(&WorkflowReplayFixture {
        id: "summary_only_projection_blocked",
        workflow_id: "research_synthesize_verify",
        user_input: "Research current scientific breakthroughs.",
        inputs: vec![
            WorkflowInput::GateText {
                stage: "gate_1_need_tool_access_menu",
                text: "Yes",
            },
            WorkflowInput::GateText {
                stage: "gate_2_tool_family_menu",
                text: "2",
            },
            WorkflowInput::GateText {
                stage: "gate_3_tool_menu",
                text: "web_search",
            },
            WorkflowInput::GateText {
                stage: "gate_4_request_payload_input",
                text: "scientific breakthroughs 2026",
            },
            WorkflowInput::ToolObservation {
                ok: true,
                summary: "Source A reports a battery chemistry milestone.",
            },
            WorkflowInput::SynthesizeFromLatestToolResult,
        ],
    });

    assert!(!report.ok, "{report:#?}");
    assert!(report
        .failures
        .iter()
        .any(|reason| { reason == "usable_tool_result_not_extracted_to_evidence" }));
    assert!(report.events.iter().any(|event| {
        event.stream == "workflow_state"
            && event.event_kind == "structured_failure"
            && event.payload.get("code").and_then(|value| value.as_str())
                == Some("usable_tool_result_not_extracted_to_evidence")
    }));
}

#[test]
fn tool_observation_without_final_answer_emits_structured_failure_and_fails_closed() {
    let report = run_workflow_replay(&WorkflowReplayFixture {
        id: "tool_observation_without_final_answer",
        workflow_id: "research_synthesize_verify",
        user_input: "Search the web and summarize the result.",
        inputs: vec![
            WorkflowInput::GateText {
                stage: "gate_1_need_tool_access_menu",
                text: "Yes",
            },
            WorkflowInput::GateText {
                stage: "gate_2_tool_family_menu",
                text: "2",
            },
            WorkflowInput::GateText {
                stage: "gate_3_tool_menu",
                text: "web_search",
            },
            WorkflowInput::GateText {
                stage: "gate_4_request_payload_input",
                text: "recent agent framework updates",
            },
            WorkflowInput::ToolObservation {
                ok: true,
                summary: "web results returned",
            },
        ],
    });

    assert!(!report.ok, "{report:#?}");
    assert_eq!(report.terminal_state, "failed");
    assert!(report
        .failures
        .iter()
        .any(|reason| reason == "missing_final_answer_after_tool_observation"));
    assert!(report.events.iter().any(|event| {
        event.stream == "workflow_state" && event.event_kind == "structured_failure"
    }));
    assert!(workflow_runtime_terminal_outcome_ok(&report));
}

#[test]
fn tool_request_without_observation_emits_structured_failure_instead_of_silent_completion() {
    let report = run_workflow_replay(&WorkflowReplayFixture {
        id: "tool_request_without_observation",
        workflow_id: "research_synthesize_verify",
        user_input: "Search the web and summarize the result.",
        inputs: vec![
            WorkflowInput::GateText {
                stage: "gate_1_need_tool_access_menu",
                text: "Yes",
            },
            WorkflowInput::GateText {
                stage: "gate_2_tool_family_menu",
                text: "2",
            },
            WorkflowInput::GateText {
                stage: "gate_3_tool_menu",
                text: "web_search",
            },
            WorkflowInput::GateText {
                stage: "gate_4_request_payload_input",
                text: "recent agent framework updates",
            },
        ],
    });

    assert!(!report.ok, "{report:#?}");
    assert_eq!(report.terminal_state, "failed");
    assert!(report
        .failures
        .iter()
        .any(|reason| reason == "missing_terminal_outcome_after_tool_request"));
    assert!(report.events.iter().any(|event| {
        event.stream == "workflow_state" && event.event_kind == "structured_failure"
    }));
    assert!(workflow_runtime_terminal_outcome_ok(&report));
}
