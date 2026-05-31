// SRS: V12-SYNTHETIC-USER-HARNESS-001
use super::*;
use std::fs;
use std::path::Path;

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("{name}_{}", now_iso_like().replace(':', "_")))
}

fn write_case_file(root: &Path, payload: &Value) -> PathBuf {
    let path = root.join("cases.json");
    fs::create_dir_all(root).expect("temp root");
    write_json(path.to_str().unwrap(), payload).expect("case write");
    path
}

fn harness_args(root: &Path, cases: &Path, strict: bool) -> Vec<String> {
    vec![
        format!("--cases={}", cases.display()),
        format!("--out={}", root.join("out.json").display()),
        format!("--out-latest={}", root.join("latest.json").display()),
        format!("--out-markdown={}", root.join("report.md").display()),
        format!("--failures-out={}", root.join("failures.jsonl").display()),
        format!("--attention-dir={}", root.join("attention").display()),
        format!("--strict={}", if strict { "1" } else { "0" }),
    ]
}

#[test]
fn synthetic_user_harness_preserves_normal_user_message_contract() {
    let root = temp_path("synthetic_user_harness_pass");
    let cases = write_case_file(
        &root,
        &json!({
            "thresholds": {
                "min_cases": 1,
                "min_pass_rate": 1.0,
                "max_failures": 0,
                "simple_direct_max_latency_ms": 5000,
                "simple_direct_max_response_tokens": 24,
                "simple_direct_max_stage_count": 2
            },
            "defaults": {"agent_id": "agent-synthetic"},
            "cases": [{
                "id": "hello",
                "turns": [{
                    "turn_id": "t1",
                    "user_message": "hey",
                    "mock_response": {
                        "response": "Hey, I am here.",
                        "response_workflow": {"stage_statuses": [{"stage": "gate_1_need_tool_access_menu", "status": "answered_no"}]},
                        "live_eval_monitor": {"chat_injection_allowed": false}
                    },
                    "expect": {
                        "required_substrings": ["Hey"],
                        "require_workflow_visibility": true,
                        "require_live_eval_monitor": true,
                        "simple_direct_conversation": true
                    }
                }]
            }]
        }),
    );
    let code = run_synthetic_user_chat_harness(&harness_args(&root, &cases, true));
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    assert_eq!(report.get("ok").and_then(Value::as_bool), Some(true));
    assert_eq!(
        report
            .pointer("/transport_contract/normal_user_message_route_only")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        report
            .pointer("/turns/0/request_body_keys/0")
            .and_then(Value::as_str),
        Some("message")
    );
    assert_eq!(
        report
            .pointer("/turns/0/workflow_stage_count")
            .and_then(Value::as_u64),
        Some(1)
    );
}

#[test]
fn synthetic_user_harness_reports_agent_work_success_by_domain() {
    let root = temp_path("synthetic_user_harness_work_success");
    let cases = write_case_file(
        &root,
        &json!({
            "thresholds": {
                "min_cases": 1,
                "min_pass_rate": 0.0,
                "max_failures": 99,
                "agent_work_success": {
                    "min_domain_success_rate": 1.0,
                    "required_domains": ["coding", "planning", "research"]
                }
            },
            "cases": [
                {
                    "id": "research_ok",
                    "turns": [{
                        "turn_id": "r1",
                        "capability": {"domain": "research", "level": 1},
                        "user_message": "compare with evidence",
                        "mock_response": {
                            "response": "The evidence supports a concise comparison.",
                            "response_workflow": {"stage_statuses": [{"stage": "gate_6_llm_final_output"}]},
                            "live_eval_monitor": {"chat_injection_allowed": false}
                        },
                        "expect": {"required_substrings": ["evidence"]}
                    }]
                },
                {
                    "id": "coding_bad",
                    "turns": [{
                        "turn_id": "c1",
                        "capability": {"domain": "coding", "level": 1},
                        "user_message": "name the patch",
                        "mock_response": {
                            "response": "I need to inspect the repo first.",
                            "response_workflow": {"stage_statuses": [{"stage": "gate_4_request_payload_input"}]},
                            "live_eval_monitor": {"chat_injection_allowed": false}
                        },
                        "expect": {"required_substrings": ["patch"]}
                    }]
                }
            ]
        }),
    );
    let code = run_synthetic_user_chat_harness(&harness_args(&root, &cases, false));
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    let domains = report
        .pointer("/summary/agent_work_success/by_domain")
        .and_then(Value::as_array)
        .expect("domain work success rows");
    let domain_row = |name: &str| {
        domains
            .iter()
            .find(|row| row.get("domain").and_then(Value::as_str) == Some(name))
            .unwrap_or_else(|| panic!("missing domain row {name}: {domains:?}"))
    };
    assert_eq!(
        domain_row("research").get("ok").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        domain_row("coding").get("ok").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        domain_row("planning")
            .get("total_turns")
            .and_then(Value::as_u64),
        Some(0)
    );
    assert_eq!(
        report
            .pointer("/summary/agent_work_success/overall/ok")
            .and_then(Value::as_bool),
        Some(false)
    );
    let markdown = fs::read_to_string(root.join("report.md")).expect("markdown report");
    assert!(
        markdown.contains("Agent Work Success By Category"),
        "{markdown}"
    );
}

#[test]
fn synthetic_user_harness_enforces_simple_direct_budgets() {
    let root = temp_path("synthetic_user_harness_budget");
    let cases = write_case_file(
        &root,
        &json!({
            "thresholds": {
                "min_cases": 1,
                "min_pass_rate": 1.0,
                "max_failures": 0,
                "simple_direct_max_response_tokens": 3,
                "simple_direct_max_stage_count": 2
            },
            "defaults": {"agent_id": "agent-budget"},
            "cases": [{
                "id": "slow_direct",
                "turns": [{
                    "turn_id": "t1",
                    "user_message": "hey",
                    "mock_response": {
                        "response": "This direct response is intentionally too verbose for the tiny budget.",
                        "tools": [{"name": "web_search"}],
                        "response_workflow": {
                            "tool_gate": {"should_call_tools": true},
                            "stage_statuses": [
                                {"stage": "gate_1_need_tool_access_menu"},
                                {"stage": "gate_2_tool_family_menu"},
                                {"stage": "gate_3_tool_menu"}
                            ]
                        },
                        "live_eval_monitor": {"chat_injection_allowed": false}
                    },
                    "expect": {
                        "require_workflow_visibility": true,
                        "simple_direct_conversation": true
                    }
                }]
            }]
        }),
    );
    let code = run_synthetic_user_chat_harness(&harness_args(&root, &cases, false));
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    let failures = report
        .pointer("/turns/0/failures")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|value| value.as_str().map(ToString::to_string))
        .collect::<Vec<_>>();
    assert!(
        failures
            .iter()
            .any(|row| row.starts_with("simple_direct_response_tokens_over_budget")),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|row| row.starts_with("simple_direct_stage_count_over_budget")),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|row| row == "simple_direct_recorded_tool_calls"),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|row| row == "simple_direct_tool_gate_should_call_tools"),
        "{failures:?}"
    );
}

#[test]
fn synthetic_user_harness_flags_visible_gate_choice_leakage() {
    let turn = json!({
        "user_message": "Dry run only: tell me which file tool you would use, but do not run tools yet.",
        "expect": {
            "forbid_gate_choice_leakage": true
        }
    });
    let thresholds = json!({});
    let payload = json!({
        "response": "No. I would use the read_file tool.",
        "response_workflow": {
            "stage_statuses": [{"stage": "gate_1_need_tool_access_menu"}]
        },
        "live_eval_monitor": {"chat_injection_allowed": false}
    });

    let failures = evaluate_turn(TurnEvaluation {
        live: false,
        turn: &turn,
        thresholds: &thresholds,
        user_message:
            "Dry run only: tell me which file tool you would use, but do not run tools yet.",
        response_text: "No. I would use the read_file tool.",
        previous_response: "",
        payload: &payload,
        route_error_code: None,
        latency_ms: 100,
        response_token_count: 7,
        workflow_stage_count: 1,
    });

    assert!(
        failures
            .iter()
            .any(|row| row == "visible_workflow_gate_choice_leakage"),
        "{failures:?}"
    );

    let category_payload = json!({
        "response": "Respond directly. Category: Respond directly. Tool family: None. Request payload: {}",
        "response_workflow": {
            "stage_statuses": [{"stage": "gate_1_work_category_menu"}]
        },
        "live_eval_monitor": {"chat_injection_allowed": false}
    });
    let failures = evaluate_turn(TurnEvaluation {
        live: false,
        turn: &turn,
        thresholds: &thresholds,
        user_message: "hey",
        response_text:
            "Respond directly. Category: Respond directly. Tool family: None. Request payload: {}",
        previous_response: "",
        payload: &category_payload,
        route_error_code: None,
        latency_ms: 100,
        response_token_count: 12,
        workflow_stage_count: 1,
    });

    assert!(
        failures
            .iter()
            .any(|row| row == "visible_workflow_gate_choice_leakage"),
        "{failures:?}"
    );

    let described_category_payload = json!({
        "response": "This kind of work is `Respond directly`.",
        "response_workflow": {
            "stage_statuses": [{"stage": "gate_1_work_category_menu"}]
        },
        "live_eval_monitor": {"chat_injection_allowed": false}
    });
    let failures = evaluate_turn(TurnEvaluation {
        live: false,
        turn: &turn,
        thresholds: &thresholds,
        user_message: "hey",
        response_text: "This kind of work is `Respond directly`.",
        previous_response: "",
        payload: &described_category_payload,
        route_error_code: None,
        latency_ms: 100,
        response_token_count: 8,
        workflow_stage_count: 1,
    });

    assert!(
        failures
            .iter()
            .any(|row| row == "visible_workflow_gate_choice_leakage"),
        "{failures:?}"
    );
}

#[test]
fn synthetic_user_harness_flags_workflow_infra_failure_modes() {
    let turn = json!({
        "user_message": "hey",
        "expect": {}
    });
    let thresholds = json!({"pending_tool_stuck_max_latency_ms": 10});
    let payload = json!({
        "response": "Need tools? Yes/No Need tools? Yes/No",
        "response_finalization": {
            "workflow_system_fallback_used": true,
            "pending_tool_request": {"status": "pending_confirmation"},
            "tool_completion": {
                "tool_attempts": [{"name": "web_search", "status": "ok"}]
            }
        },
        "response_workflow": {
            "final_llm_response": {
                "status": "skipped",
                "attempt_count": 2,
                "fallback_guard_multi_stage": true
            },
            "stage_statuses": [{"stage": "gate_1_need_tool_access_menu"}]
        },
        "tools": [{"name": "web_search"}],
        "live_eval_monitor": {"chat_injection_allowed": false}
    });

    let failures = evaluate_turn(TurnEvaluation {
        live: false,
        turn: &turn,
        thresholds: &thresholds,
        user_message: "hey",
        response_text: "Need tools? Yes/No Need tools? Yes/No",
        previous_response: "Need tools? Yes/No Need tools? Yes/No",
        payload: &payload,
        route_error_code: None,
        latency_ms: 20,
        response_token_count: 4,
        workflow_stage_count: 1,
    });

    for expected in [
        "gate_token_leakage",
        "hidden_second_pass_call",
        "repeated_gate_prompt",
        "tool_result_without_synthesis",
    ] {
        assert!(
            failures.iter().any(|row| row == expected),
            "missing {expected}: {failures:?}"
        );
    }
    assert!(
        failures
            .iter()
            .any(|row| row.starts_with("pending_tool_stuck_too_long")),
        "{failures:?}"
    );
}

#[test]
fn synthetic_user_harness_accepts_useful_plaintext_without_magic_words() {
    let turn = json!({
        "user_message": "Use web search to compare infring to other major agentic frameworks in April 2026.",
        "expect": {"require_useful_plaintext_answer": true}
    });
    let thresholds = json!({});
    let payload = json!({
        "response_workflow": {
            "final_llm_response": {
                "status": "synthesized",
                "attempt_count": 2
            }
        },
        "live_eval_monitor": {"chat_injection_allowed": false}
    });
    let useful_failures = evaluate_turn(TurnEvaluation {
        live: false,
        turn: &turn,
        thresholds: &thresholds,
        user_message: "Use web search to compare infring to other major agentic frameworks in April 2026.",
        response_text: "I can't make a reliable comparison of Infring against other agentic frameworks because the retrieved evidence did not return source-backed coverage for the requested comparison.",
        previous_response: "",
        payload: &payload,
        route_error_code: None,
        latency_ms: 100,
        response_token_count: 21,
        workflow_stage_count: 1,
    });
    assert!(
        !useful_failures
            .iter()
            .any(|row| row == "missing_useful_plaintext_answer"
                || row == "hidden_second_pass_call"),
        "{useful_failures:?}"
    );

    let fragment_failures = evaluate_turn(TurnEvaluation {
        live: false,
        turn: &turn,
        thresholds: &thresholds,
        user_message: "Use web search to compare infring to other major agentic frameworks in April 2026.",
        response_text: "Tool trace complete. Web search from web retrieval: provider_starved and no usable evidence claims.",
        previous_response: "",
        payload: &payload,
        route_error_code: None,
        latency_ms: 100,
        response_token_count: 11,
        workflow_stage_count: 1,
    });
    assert!(
        fragment_failures
            .iter()
            .any(|row| row == "missing_useful_plaintext_answer"),
        "{fragment_failures:?}"
    );

    let recovery_turn = json!({
        "user_message": "what? why are you repeating the same fallback text?",
        "expect": {"require_useful_plaintext_answer": true}
    });
    let recovery_failures = evaluate_turn(TurnEvaluation {
        live: false,
        turn: &recovery_turn,
        thresholds: &thresholds,
        user_message: "what? why are you repeating the same fallback text?",
        response_text: "You are right to call that out. The repeated fallback text happened because workflow telemetry was being surfaced as chat text instead of staying internal. I should answer your actual request directly and keep those diagnostics out of the visible reply.",
        previous_response: "",
        payload: &payload,
        route_error_code: None,
        latency_ms: 100,
        response_token_count: 34,
        workflow_stage_count: 1,
    });
    assert!(
        !recovery_failures
            .iter()
            .any(|row| row == "missing_useful_plaintext_answer"),
        "{recovery_failures:?}"
    );
}

#[test]
fn synthetic_user_harness_flags_empty_direct_reply() {
    let turn = json!({"user_message": "hey", "expect": {}});
    let failures = evaluate_turn(TurnEvaluation {
        live: false,
        turn: &turn,
        thresholds: &json!({}),
        user_message: "hey",
        response_text: "",
        previous_response: "",
        payload: &json!({"response": "", "response_workflow": {}, "live_eval_monitor": {"chat_injection_allowed": false}}),
        route_error_code: None,
        latency_ms: 1,
        response_token_count: 0,
        workflow_stage_count: 0,
    });
    assert!(
        failures.iter().any(|row| row == "empty_direct_reply"),
        "{failures:?}"
    );
}

#[test]
fn synthetic_user_harness_requires_real_work_tool_progress() {
    let turn = json!({
        "user_message": "Use web search to compare infring to other major agentic frameworks in April 2026.",
        "expect": {
            "require_tool_progress": true,
            "forbid_unresolved_tool_need_without_progress": true
        }
    });
    let thresholds = json!({});
    let payload = json!({
        "response": "I don't have current web search results. I can provide a comparison if you'd like me to search.",
        "response_workflow": {
            "stage_statuses": [
                {"stage": "gate_1_need_tool_access_menu", "status": "presented"},
                {"stage": "final_llm_response", "status": "synthesized"}
            ]
        },
        "tools": [],
        "live_eval_monitor": {"chat_injection_allowed": false}
    });

    let failures = evaluate_turn(TurnEvaluation {
        live: false,
        turn: &turn,
        thresholds: &thresholds,
        user_message: "Use web search to compare infring to other major agentic frameworks in April 2026.",
        response_text: "I don't have current web search results. I can provide a comparison if you'd like me to search.",
        previous_response: "",
        payload: &payload,
        route_error_code: None,
        latency_ms: 100,
        response_token_count: 17,
        workflow_stage_count: 2,
    });

    assert!(
        failures
            .iter()
            .any(|row| row == "missing_tool_progress_evidence"),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|row| row == "unresolved_tool_need_without_progress"),
        "{failures:?}"
    );
}

#[test]
fn synthetic_user_harness_accepts_pending_tool_progress() {
    let turn = json!({
        "user_message": "Use web search to compare infring to other major agentic frameworks in April 2026.",
        "expect": {
            "require_tool_progress": true,
            "forbid_unresolved_tool_need_without_progress": true
        }
    });
    let thresholds = json!({});
    let payload = json!({
        "response": "",
        "pending_tool_request": {
            "status": "pending_confirmation",
            "tool_name": "web_search",
            "execution_claim_allowed": false
        },
        "response_workflow": {
            "stage_statuses": [
                {"stage": "gate_1_need_tool_access_menu", "status": "presented"},
                {"stage": "final_llm_response", "status": "synthesized"}
            ]
        },
        "tools": [],
        "live_eval_monitor": {"chat_injection_allowed": false}
    });

    let failures = evaluate_turn(TurnEvaluation {
        live: false,
        turn: &turn,
        thresholds: &thresholds,
        user_message:
            "Use web search to compare infring to other major agentic frameworks in April 2026.",
        response_text: "",
        previous_response: "",
        payload: &payload,
        route_error_code: None,
        latency_ms: 100,
        response_token_count: 0,
        workflow_stage_count: 2,
    });

    assert!(
        !failures
            .iter()
            .any(|row| row == "missing_tool_progress_evidence"
                || row == "unresolved_tool_need_without_progress"),
        "{failures:?}"
    );
}

#[test]
fn synthetic_user_harness_requires_executed_tool_synthesis_when_requested() {
    let turn = json!({
        "user_message": "Search the web and summarize what changed.",
        "expect": {
            "require_tool_execution_evidence": true,
            "require_final_synthesis": true,
            "require_tool_result_synthesis": true
        }
    });
    let thresholds = json!({});
    let pending_only_payload = json!({
        "response": "",
        "pending_tool_request": {"status": "pending_confirmation", "tool_name": "web_search"},
        "response_workflow": {
            "final_llm_response": {"status": "synthesized"},
            "stage_statuses": [{"stage": "gate_2_tool_family_menu", "status": "selected_web_search"}]
        },
        "tools": [],
        "live_eval_monitor": {"chat_injection_allowed": false}
    });

    let pending_failures = evaluate_turn(TurnEvaluation {
        live: false,
        turn: &turn,
        thresholds: &thresholds,
        user_message: "Search the web and summarize what changed.",
        response_text: "",
        previous_response: "",
        payload: &pending_only_payload,
        route_error_code: None,
        latency_ms: 100,
        response_token_count: 0,
        workflow_stage_count: 1,
    });
    assert!(
        pending_failures
            .iter()
            .any(|row| row == "missing_tool_execution_evidence"),
        "{pending_failures:?}"
    );

    let executed_payload = json!({
        "response": "The search result says the framework added native workflow traces.",
        "response_workflow": {
            "final_llm_response": {"status": "synthesized"},
            "stage_statuses": [{"stage": "final_llm_response", "status": "synthesized"}]
        },
        "tools": [{"name": "web_search", "status": "success", "receipt": "tool-receipt-1"}],
        "live_eval_monitor": {"chat_injection_allowed": false}
    });
    let executed_failures = evaluate_turn(TurnEvaluation {
        live: false,
        turn: &turn,
        thresholds: &thresholds,
        user_message: "Search the web and summarize what changed.",
        response_text: "The search result says the framework added native workflow traces.",
        previous_response: "",
        payload: &executed_payload,
        route_error_code: None,
        latency_ms: 100,
        response_token_count: 10,
        workflow_stage_count: 1,
    });
    assert!(
        !executed_failures
            .iter()
            .any(|row| row == "missing_tool_execution_evidence"
                || row == "missing_final_synthesis_status"
                || row == "tool_execution_without_final_synthesis"),
        "{executed_failures:?}"
    );
}

#[test]
fn synthetic_user_harness_flags_unbacked_tool_result_claims() {
    let turn = json!({
        "user_message": "what? why are you repeating the same fallback text?",
        "expect": {
            "required_substrings": ["answer directly"]
        }
    });
    let thresholds = json!({});
    let payload = json!({
        "response": "I will answer directly. The tool returned no new results beyond the previous fallback text.",
        "response_workflow": {
            "stage_statuses": [
                {"stage": "gate_1_need_tool_access_menu", "status": "presented"},
                {"stage": "final_llm_response", "status": "synthesized"}
            ]
        },
        "tools": [],
        "live_eval_monitor": {"chat_injection_allowed": false}
    });

    let failures = evaluate_turn(TurnEvaluation {
        live: false,
        turn: &turn,
        thresholds: &thresholds,
        user_message: "what? why are you repeating the same fallback text?",
        response_text:
            "I will answer directly. The tool returned no new results beyond the previous fallback text.",
        previous_response: "",
        payload: &payload,
        route_error_code: None,
        latency_ms: 100,
        response_token_count: 15,
        workflow_stage_count: 2,
    });

    assert!(
        failures
            .iter()
            .any(|row| row == "unsupported_tool_claim_without_progress"),
        "{failures:?}"
    );
}

#[test]
fn synthetic_user_harness_uses_separate_live_latency_budget() {
    let turn = json!({
        "user_message": "hey",
        "expect": {
            "simple_direct_conversation": true
        }
    });
    let thresholds = json!({
        "simple_direct_max_latency_ms": 5000,
        "simple_direct_live_max_latency_ms": 15000,
        "simple_direct_max_response_tokens": 24,
        "simple_direct_max_stage_count": 2
    });
    let payload = json!({
        "response": "Hey, I am here.",
        "response_workflow": {
            "tool_gate": {"should_call_tools": false},
            "stage_statuses": [
                {"stage": "gate_1_need_tool_access_menu"},
                {"stage": "gate_6_llm_final_output"}
            ]
        },
        "tools": [],
        "live_eval_monitor": {"chat_injection_allowed": false}
    });

    let offline_failures = evaluate_turn(TurnEvaluation {
        live: false,
        turn: &turn,
        thresholds: &thresholds,
        user_message: "hey",
        response_text: "Hey, I am here.",
        previous_response: "",
        payload: &payload,
        route_error_code: None,
        latency_ms: 8_000,
        response_token_count: 4,
        workflow_stage_count: 2,
    });
    assert!(
        offline_failures
            .iter()
            .any(|row| row == "simple_direct_latency_over_budget:8000>5000"),
        "{offline_failures:?}"
    );

    let live_failures = evaluate_turn(TurnEvaluation {
        live: true,
        turn: &turn,
        thresholds: &thresholds,
        user_message: "hey",
        response_text: "Hey, I am here.",
        previous_response: "",
        payload: &payload,
        route_error_code: None,
        latency_ms: 8_000,
        response_token_count: 4,
        workflow_stage_count: 2,
    });
    assert!(
        !live_failures
            .iter()
            .any(|row| row.starts_with("simple_direct_latency_over_budget")),
        "{live_failures:?}"
    );
}

#[test]
fn synthetic_user_harness_flags_fallback_text_and_writes_attention() {
    let root = temp_path("synthetic_user_harness_failure");
    let cases = write_case_file(
        &root,
        &json!({
            "thresholds": {"min_cases": 1, "min_pass_rate": 1.0, "max_failures": 0},
            "defaults": {"agent_id": "agent-loop"},
            "cases": [{
                "id": "fallback_loop",
                "turns": [{
                    "turn_id": "t1",
                    "user_message": "what is going on?",
                    "mock_response": {
                        "response": "I hit a response finalization edge on that turn.",
                        "live_eval_monitor": {"chat_injection_allowed": false}
                    }
                }]
            }]
        }),
    );
    let code = run_synthetic_user_chat_harness(&harness_args(&root, &cases, false));
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    assert_eq!(report.get("ok").and_then(Value::as_bool), Some(false));
    assert!(root.join("attention/agent-loop.attention.jsonl").exists());
    let attention = fs::read_to_string(root.join("attention/agent-loop.attention.jsonl"))
        .expect("attention jsonl");
    let event: Value = serde_json::from_str(attention.lines().next().unwrap()).expect("event json");
    assert_eq!(
        event.get("owner_component").and_then(Value::as_str),
        Some("orchestration.finalization")
    );
    let replay = event
        .get("replay_command")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(
        replay.contains("synthetic-user-chat-harness --live=0 --agent-id=agent-loop --strict=1"),
        "{replay}"
    );
    assert!(
        replay.contains(&format!("--cases={}", cases.display())),
        "{replay}"
    );
}

#[test]
fn synthetic_user_harness_reports_first_route_stage_delta() {
    let turn = json!({
        "mock_response": {
            "response": "Hey, I am here.",
            "response_workflow": {
                "stage_statuses": [{"stage": "gate_1_need_tool_access_menu"}]
            },
            "live_eval_monitor": {"chat_injection_allowed": false}
        }
    });
    let actual = json!({"ok": true, "response": ""});
    let delta = route_stage_delta(
        &turn,
        &actual,
        None,
        "",
        workflow_stage_count(&actual),
        &["empty_assistant_response".to_string()],
    );
    assert_eq!(
        delta.get("first_missing_stage").and_then(Value::as_str),
        Some("workflow_library_selection_or_payload_assembly")
    );
    assert_eq!(delta.get("diverged").and_then(Value::as_bool), Some(true));
}

#[test]
fn synthetic_user_harness_blocks_remote_live_dashboard_by_default() {
    let root = temp_path("synthetic_user_harness_remote");
    let cases = write_case_file(
        &root,
        &json!({
            "thresholds": {"min_cases": 1},
            "cases": [{"id": "remote", "turns": [{"user_message": "hey"}]}]
        }),
    );
    let mut args = harness_args(&root, &cases, true);
    args.push("--live=1".to_string());
    args.push("--base-url=https://example.com".to_string());
    let code = run_synthetic_user_chat_harness(&args);
    assert_eq!(code, 1);
    let report = read_json(root.join("out.json").to_str().unwrap());
    assert_eq!(
        report.pointer("/setup_failures/0").and_then(Value::as_str),
        Some("remote_dashboard_url_requires_allow_remote")
    );
}

#[test]
fn synthetic_user_harness_live_monitor_freshness_contract() {
    let stale = live_monitor_freshness_report(
        true,
        true,
        Some("unix_ms:10"),
        Some("unix_ms:10"),
        &[],
        "local/state/ops/eval_live_monitor/latest.json",
    );
    assert_eq!(stale.get("ok").and_then(Value::as_bool), Some(false));
    assert_eq!(
        stale.get("failure_reason").and_then(Value::as_str),
        Some("live_eval_monitor_timestamp_not_advanced")
    );

    let fresh = live_monitor_freshness_report(
        true,
        true,
        Some("unix_ms:10"),
        Some("unix_ms:11"),
        &[],
        "local/state/ops/eval_live_monitor/latest.json",
    );
    assert_eq!(fresh.get("ok").and_then(Value::as_bool), Some(true));
}

#[test]
fn misty_live_health_gate_command_requires_live_agent_and_strict() {
    let command = misty_live_health_gate_required_command("agent-5bc62b0875a9");
    assert!(
        command.contains(
            "synthetic-user-chat-harness --live=1 --agent-id=agent-5bc62b0875a9 --strict=1"
        ),
        "{command}"
    );
}
