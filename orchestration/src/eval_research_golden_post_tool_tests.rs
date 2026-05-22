use super::*;
use std::fs;
use std::path::{Path, PathBuf};

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("{name}_{}", now_iso_like().replace(':', "_")))
}

fn write_json_file(path: &Path, payload: &Value) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("temp parent");
    }
    fs::write(
        path,
        format!("{}\n", serde_json::to_string_pretty(payload).unwrap()),
    )
    .expect("write temp json");
}

fn dataset() -> Value {
    json!({
        "reliability_thresholds": {
            "min_cases_for_reliability_claim": 1,
            "workflow_gate_pass_min": 0.95,
            "research_success_min": 0.85,
            "max_empty_responses": 0,
            "max_raw_tool_leaks": 0,
            "max_tool_choice_as_final_response": 0,
            "max_unsupported_factual_claims": 0
        },
        "scoring_contract": {
            "pass_score": 85,
            "excellent_score": 95
        },
        "cases": [{
            "id": "research_gold_test",
            "category": "comparison",
            "prompt": "Use web research to compare Infring with LangGraph.",
            "expected_gate_path": {
                "gate_1": "tool_required",
                "gate_2": "web_research",
                "gate_3": "web_search",
                "gate_4_required_fields": ["query", "aperture"]
            },
            "required_entities": ["Infring", "LangGraph"]
        }]
    })
}

fn runner_args(root: &Path, cases: &Path, responses: &Path, strict: bool) -> Vec<String> {
    vec![
        format!("--cases={}", cases.display()),
        format!("--responses={}", responses.display()),
        format!("--out={}", root.join("out.json").display()),
        format!("--out-latest={}", root.join("latest.json").display()),
        format!("--out-markdown={}", root.join("report.md").display()),
        format!("--failures-out={}", root.join("failures.jsonl").display()),
        format!(
            "--observation-ledger-out={}",
            root.join("observation_events.jsonl").display()
        ),
        format!(
            "--observation-hot-out={}",
            root.join("observation_hot.json").display()
        ),
        format!(
            "--observation-archive-out={}",
            root.join("observation_archive.json").display()
        ),
        format!(
            "--observation-summary-out={}",
            root.join("observation_summary.json").display()
        ),
        format!("--strict={}", if strict { "1" } else { "0" }),
    ]
}

#[test]
fn research_golden_splits_low_signal_tool_result_after_execution() {
    let root = temp_path("research_golden_low_signal_tool");
    let cases = root.join("cases.json");
    let responses = root.join("responses.json");
    write_json_file(&cases, &dataset());
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_gold_test",
                "response_payload": {
                    "response": "The search was low signal and did not produce enough source coverage. Please narrow the query to one framework pair.",
                    "pending_tool_request": {
                        "status": "executed",
                        "tool_name": "web_search",
                        "selected_tool_family": "Web Search / Fetch",
                        "input": {
                            "query": "Infring LangGraph comparison current docs",
                            "aperture": "medium"
                        }
                    },
                    "tools": [{
                        "name": "web_search",
                        "status": "no_results",
                        "result": "Search did not produce enough source coverage for the requested comparison."
                    }],
                    "response_finalization": {
                        "tool_completion": {
                            "completion_state": "reported_no_findings",
                            "findings_available": false,
                            "final_no_findings": true
                        }
                    }
                }
            }]
        }),
    );
    let code = run_research_golden(&runner_args(&root, &cases, &responses, false));
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    assert_eq!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/first_failed_checkpoint")
            .and_then(Value::as_str),
        Some("5b_raw_provider_result_present")
    );
    assert_eq!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/inferred_failure_boundary")
            .and_then(Value::as_str),
        Some("raw_provider_result_absent_or_low_signal")
    );
}

#[test]
fn research_golden_splits_missing_evidence_extraction_after_usable_result() {
    let root = temp_path("research_golden_missing_evidence");
    let cases = root.join("cases.json");
    let responses = root.join("responses.json");
    write_json_file(&cases, &dataset());
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_gold_test",
                "response_payload": {
                    "response": "According to current source evidence, Infring and LangGraph differ in workflow control and graph orchestration. The practical tradeoff is that LangGraph fits production teams that need durable graph execution, while Infring fits teams experimenting with editable workflow gates and audit-friendly rollback. My recommendation is to use LangGraph for conventional production assistants and keep Infring for gate-inspection experiments.",
                    "pending_tool_request": {
                        "status": "executed",
                        "tool_name": "web_search",
                        "selected_tool_family": "Web Search / Fetch",
                        "input": {
                            "query": "Infring LangGraph comparison current docs",
                            "aperture": "medium"
                        }
                    },
                    "tools": [{
                        "name": "web_search",
                        "status": "ok",
                        "raw_results": [{
                            "title": "LangGraph durable graph docs",
                            "snippet": "LangGraph documents graph orchestration, durable execution, and state-machine patterns for agent workflows."
                        }],
                        "result": "Current source evidence says LangGraph documents graph orchestration and durable execution, while Infring notes emphasize workflow CD gates and rollback."
                    }],
                    "response_finalization": {
                        "tool_completion": {
                            "completion_state": "reported_findings",
                            "findings_available": true
                        }
                    }
                }
            }]
        }),
    );
    let code = run_research_golden(&runner_args(&root, &cases, &responses, false));
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    assert_eq!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/first_failed_checkpoint")
            .and_then(Value::as_str),
        Some("5d_evidence_refs_extracted")
    );
    assert_eq!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/inferred_failure_boundary")
            .and_then(Value::as_str),
        Some("packaged_result_not_extracted_to_evidence")
    );
}

#[test]
fn research_golden_counts_low_signal_packaged_result_before_evidence_extraction() {
    let root = temp_path("research_golden_low_signal_packaged");
    let cases = root.join("cases.json");
    let responses = root.join("responses.json");
    write_json_file(&cases, &dataset());
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_gold_test",
                "response_payload": {
                    "response": "The current source coverage is limited. The retrieved results point to official docs and release notes, but they are not strong enough yet for a confident comparison. The next useful action is to add one narrower query per framework and then synthesize from the combined evidence.",
                    "pending_tool_request": {
                        "status": "executed",
                        "tool_name": "web_search",
                        "selected_tool_family": "Web Search / Fetch",
                        "input": {
                            "query": "Infring LangGraph comparison current docs",
                            "aperture": "medium"
                        }
                    },
                    "tools": [{
                        "name": "web_search",
                        "status": "ok",
                        "raw_results": [{
                            "title": "LangGraph release notes",
                            "snippet": "LangGraph release notes discuss graph execution, persistence, and workflow reliability improvements."
                        }],
                        "result": "The current source coverage is limited. Retrieved results point to docs and release notes, but they are not yet strong enough for a confident comparison."
                    }],
                    "response_finalization": {
                        "tool_completion": {
                            "completion_state": "reported_no_findings",
                            "findings_available": false,
                            "final_no_findings": true
                        }
                    }
                }
            }]
        }),
    );
    let code = run_research_golden(&runner_args(&root, &cases, &responses, false));
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    assert_eq!(
        report.pointer(
            "/cases/0/gate_transition_diagnostics/post_tool_pipeline/packaged_tool_result_present"
        ),
        Some(&Value::Bool(true))
    );
    assert_eq!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/post_tool_pipeline/packaged_tool_result_paths/0")
            .and_then(Value::as_str),
        Some("tools.0.result")
    );
    assert_eq!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/first_failed_checkpoint")
            .and_then(Value::as_str),
        Some("5d_evidence_refs_extracted")
    );
}

#[test]
fn research_golden_counts_error_status_tool_artifacts_before_evidence_extraction() {
    let root = temp_path("research_golden_error_status_tool_artifacts");
    let cases = root.join("cases.json");
    let responses = root.join("responses.json");
    write_json_file(&cases, &dataset());
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_gold_test",
                "response_payload": {
                    "response": "The search failed, but the recorded provider artifact still shows which query ran and why it degraded. That is enough to explain the limitation and propose a narrower retry.",
                    "pending_tool_request": {
                        "status": "executed",
                        "tool_name": "batch_query",
                        "selected_tool_family": "Web Search / Fetch",
                        "input": {
                            "query": "Find recent benchmarks comparing agent frameworks",
                            "aperture": "medium"
                        }
                    },
                    "tools": [{
                        "name": "batch_query",
                        "status": "error",
                        "provider_results": [{
                            "provider": "bing_rss",
                            "query": "Find recent benchmarks comparing agent frameworks",
                            "summary": "Search provider returned no comparison-grade benchmark rows.",
                            "error": "web_search_tool_surface_degraded"
                        }],
                        "result": "The recorded tool outcome shows a failed search attempt with degraded provider coverage."
                    }],
                    "response_finalization": {
                        "tool_completion": {
                            "completion_state": "reported_no_findings",
                            "findings_available": false,
                            "final_no_findings": true
                        }
                    }
                }
            }]
        }),
    );
    let code = run_research_golden(&runner_args(&root, &cases, &responses, false));
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    assert_eq!(
        report.pointer(
            "/cases/0/gate_transition_diagnostics/post_tool_pipeline/raw_provider_result_present"
        ),
        Some(&Value::Bool(true))
    );
    assert_eq!(
        report.pointer(
            "/cases/0/gate_transition_diagnostics/post_tool_pipeline/packaged_tool_result_present"
        ),
        Some(&Value::Bool(true))
    );
    assert_eq!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/first_failed_checkpoint")
            .and_then(Value::as_str),
        Some("5d_evidence_refs_extracted")
    );
}

#[test]
fn research_golden_splits_weak_synthesis_after_evidence_extraction() {
    let root = temp_path("research_golden_weak_synthesis");
    let cases = root.join("cases.json");
    let responses = root.join("responses.json");
    write_json_file(&cases, &dataset());
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_gold_test",
                "response_payload": {
                    "response": "I found sources. Please narrow the query.",
                    "pending_tool_request": {
                        "status": "executed",
                        "tool_name": "web_search",
                        "selected_tool_family": "Web Search / Fetch",
                        "input": {
                            "query": "Infring LangGraph comparison current docs",
                            "aperture": "medium"
                        }
                    },
                    "tools": [{
                        "name": "web_search",
                        "status": "ok",
                        "raw_results": [{
                            "title": "LangGraph durable graph docs",
                            "snippet": "LangGraph documents graph orchestration, durable execution, and state-machine patterns for agent workflows."
                        }],
                        "result": "Current source evidence says LangGraph documents graph orchestration and durable execution, while Infring notes emphasize workflow CD gates and rollback."
                    }],
                    "response_workflow": {
                        "evidence_refs": ["evidence:langgraph-docs", "evidence:infring-notes"]
                    },
                    "response_finalization": {
                        "tool_completion": {
                            "completion_state": "reported_findings",
                            "findings_available": true
                        }
                    }
                }
            }]
        }),
    );
    let code = run_research_golden(&runner_args(&root, &cases, &responses, false));
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    assert_eq!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/first_failed_checkpoint")
            .and_then(Value::as_str),
        Some("6a_synthesis_uses_evidence_or_low_evidence_fallback")
    );
    assert_eq!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/inferred_failure_boundary")
            .and_then(Value::as_str),
        Some("post_tool_synthesis_not_useful")
    );
    assert_eq!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/synthesis_failure_hardness")
            .and_then(Value::as_str),
        Some("soft")
    );
    assert_eq!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/synthesis_failure_class")
            .and_then(Value::as_str),
        Some("evidence_entity_coverage_gap")
    );
}

#[test]
fn research_golden_counts_tool_row_evidence_refs_as_extracted_evidence() {
    let root = temp_path("research_golden_tool_row_evidence");
    let cases = root.join("cases.json");
    let responses = root.join("responses.json");
    write_json_file(&cases, &dataset());
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_gold_test",
                "response_payload": {
                    "response": "According to the current evidence, LangGraph documents durable execution while Infring still appears sparse in public sources. The comparison remains limited, but the retrieved evidence supports that one workflow favors mature graph orchestration and the other favors editable gate experimentation.",
                    "pending_tool_request": {
                        "status": "executed",
                        "tool_name": "web_search",
                        "selected_tool_family": "Web Search / Fetch",
                        "input": {
                            "query": "Infring LangGraph comparison current docs",
                            "aperture": "medium"
                        }
                    },
                    "tools": [{
                        "name": "web_search",
                        "status": "ok",
                        "raw_results": [{
                            "title": "LangGraph durable graph docs",
                            "snippet": "LangGraph documents graph orchestration, durable execution, and state-machine patterns for agent workflows."
                        }],
                        "result": "Current source evidence says LangGraph documents graph orchestration and durable execution, while Infring notes emphasize workflow CD gates and rollback.",
                        "evidence_refs": [
                            {"title": "LangGraph durable graph docs", "locator": "https://docs.langchain.com/langgraph", "score": 0.91}
                        ]
                    }],
                    "response_finalization": {
                        "tool_completion": {
                            "completion_state": "reported_findings",
                            "findings_available": true
                        }
                    }
                }
            }]
        }),
    );
    let code = run_research_golden(&runner_args(&root, &cases, &responses, false));
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    assert_eq!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/post_tool_pipeline/evidence_extracted"),
        Some(&Value::Bool(true))
    );
    assert_ne!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/first_failed_checkpoint")
            .and_then(Value::as_str),
        Some("5d_evidence_refs_extracted")
    );
}

#[test]
fn research_golden_grades_low_signal_evidence_as_low_evidence_synthesis() {
    let root = temp_path("research_golden_low_signal_evidence_lane");
    let cases = root.join("cases.json");
    let responses = root.join("responses.json");
    write_json_file(&cases, &dataset());
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_gold_test",
                "response_payload": {
                    "response": "The returned results were low-signal, so there is no source-backed winner. This retrieval miss supports only that LangGraph has clearer public docs for durable graph execution; it does not support a complete Infring vs LangGraph comparison. Bounded conclusion: evaluate LangGraph first for public evidence maturity, and treat Infring as needing direct docs or repo inspection before selection.",
                    "pending_tool_request": {
                        "status": "executed",
                        "tool_name": "web_search",
                        "selected_tool_family": "Web Search / Fetch",
                        "input": {
                            "query": "Infring LangGraph comparison current docs",
                            "aperture": "medium"
                        }
                    },
                    "tools": [{
                        "name": "web_search",
                        "status": "low_signal",
                        "raw_results": [{
                            "title": "LangGraph durable graph docs",
                            "snippet": "LangGraph documents graph orchestration, durable execution, and state-machine patterns for agent workflows."
                        }],
                        "result": "Low signal: one relevant LangGraph source surfaced, but no comparable public Infring source surfaced.",
                        "evidence_refs": [
                            {"title": "LangGraph durable graph docs", "locator": "https://docs.langchain.com/langgraph", "score": 0.91}
                        ]
                    }],
                    "response_finalization": {
                        "outcome": "workflow_authored+tool_completion:low_signal+synthesized",
                        "tool_completion": {
                            "completion_state": "low_signal",
                            "findings_available": true
                        }
                    }
                }
            }]
        }),
    );
    let code = run_research_golden(&runner_args(&root, &cases, &responses, false));
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    let checkpoints = report
        .pointer("/cases/0/gate_transition_diagnostics/checkpoints")
        .and_then(Value::as_array)
        .expect("checkpoints");
    let gate_6a = checkpoints
        .iter()
        .find(|row| {
            row.get("checkpoint").and_then(Value::as_str)
                == Some("6a_synthesis_uses_evidence_or_low_evidence_fallback")
        })
        .expect("6a checkpoint");
    assert_eq!(gate_6a.get("status").and_then(Value::as_str), Some("pass"));
    assert_ne!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/first_failed_checkpoint")
            .and_then(Value::as_str),
        Some("6a_synthesis_uses_evidence_or_low_evidence_fallback")
    );
}

#[test]
fn research_golden_accepts_bounded_gap_language_for_low_signal_evidence() {
    let root = temp_path("research_golden_low_signal_gap_language");
    let cases = root.join("cases.json");
    let responses = root.join("responses.json");
    write_json_file(&cases, &dataset());
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_gold_test",
                "response_payload": {
                    "response": "Here is a bounded comparison based on the retrieved evidence, with clear separation between what the evidence directly supports and where inference fills gaps. LangGraph has source-backed support for durable graph orchestration and stateful agent flows. Infring is present in the task, but the retrieved snippets do not contain direct Infring production evidence, so a direct source-backed winner is not available. Practical recommendation: use LangGraph when public documentation maturity matters now, and keep Infring in the evaluation path only after direct docs or repository evidence are available.",
                    "pending_tool_request": {
                        "status": "executed",
                        "tool_name": "web_search",
                        "selected_tool_family": "Web Search / Fetch",
                        "input": {
                            "query": "Infring LangGraph comparison current docs",
                            "aperture": "medium"
                        }
                    },
                    "tools": [{
                        "name": "web_search",
                        "status": "low_signal",
                        "raw_results": [{
                            "title": "LangGraph durable graph docs",
                            "snippet": "LangGraph documents graph orchestration, durable execution, and state-machine patterns for agent workflows."
                        }],
                        "result": "Low-signal retrieval: relevant LangGraph documentation surfaced, but direct Infring evidence was not found.",
                        "evidence_refs": [
                            {"title": "LangGraph durable graph docs", "locator": "https://docs.langchain.com/langgraph", "score": 0.91}
                        ]
                    }],
                    "response_finalization": {
                        "outcome": "workflow_authored+tool_completion:low_signal+synthesized",
                        "tool_completion": {
                            "completion_state": "low_signal",
                            "findings_available": true
                        }
                    }
                }
            }]
        }),
    );
    let code = run_research_golden(&runner_args(&root, &cases, &responses, false));
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    assert_eq!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/first_failed_checkpoint")
            .and_then(Value::as_str),
        None
    );
    assert_eq!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/synthesis_failure_class")
            .and_then(Value::as_str),
        Some("none")
    );
}

#[test]
fn research_golden_blocks_excellent_for_low_signal_fallback() {
    let root = temp_path("research_golden_low_signal_not_excellent");
    let cases = root.join("cases.json");
    let responses = root.join("responses.json");
    write_json_file(&cases, &dataset());
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_gold_test",
                "response_payload": {
                    "response": "Bounded conclusion: the retrieved evidence is low-signal, so this should be treated as a provisional comparison rather than a source-backed winner. Infring appears useful where editable workflow CDs, gate inspection, and rollback discipline matter. LangGraph has clearer public docs for durable graph orchestration and state-machine workflows. The evidence supports using LangGraph first when public documentation maturity matters, while treating Infring-specific production claims as requiring direct docs or repository inspection. The caveat is that this retrieval pass surfaced only partial source coverage, so the decision should not be framed as fully proven.",
                    "pending_tool_request": {
                        "status": "executed",
                        "tool_name": "web_search",
                        "selected_tool_family": "Web Search / Fetch",
                        "input": {
                            "query": "Infring LangGraph comparison current docs",
                            "aperture": "medium"
                        }
                    },
                    "tools": [{
                        "name": "web_search",
                        "status": "low_signal",
                        "raw_results": [{
                            "title": "LangGraph durable graph docs",
                            "snippet": "LangGraph documents graph orchestration, durable execution, and state-machine patterns for agent workflows."
                        }],
                        "result": "Low signal: one relevant LangGraph source surfaced, but comparable Infring source coverage did not.",
                        "evidence_refs": [
                            {"title": "LangGraph durable graph docs", "locator": "https://docs.langchain.com/langgraph", "score": 0.91}
                        ]
                    }],
                    "response_workflow": {
                        "evidence_refs": ["evidence:langgraph-docs"],
                        "final_llm_response": {
                            "status": "synthesized",
                            "evidence_refs_used": ["evidence:langgraph-docs"]
                        }
                    },
                    "response_finalization": {
                        "outcome": "workflow_authored+tool_completion:low_signal+synthesized",
                        "tool_completion": {
                            "completion_state": "low_signal",
                            "findings_available": true,
                            "evidence_refs_used": ["evidence:langgraph-docs"]
                        }
                    }
                }
            }]
        }),
    );
    let code = run_research_golden(&runner_args(&root, &cases, &responses, false));
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    assert_eq!(report.pointer("/cases/0/pass"), Some(&Value::Bool(true)));
    assert_eq!(
        report.pointer("/cases/0/retrieval_quality/status"),
        Some(&Value::String("low_signal".to_string()))
    );
    assert_eq!(
        report.pointer("/cases/0/retrieval_quality/allows_excellent"),
        Some(&Value::Bool(false))
    );
    assert_eq!(
        report.pointer("/cases/0/excellent"),
        Some(&Value::Bool(false))
    );
    assert!(report
        .pointer("/cases/0/excellent_blockers")
        .and_then(Value::as_array)
        .unwrap()
        .iter()
        .any(|row| row.as_str() == Some("retrieval_quality_not_excellent_ready")));
    assert_eq!(
        report.pointer(
            "/cases/0/excellent_diagnostics/subgates/excellent_2_citable_evidence_available"
        ),
        Some(&Value::Bool(false))
    );
    assert_eq!(
        report
            .pointer("/measurement_split/live_retrieval_health/retrieval_quality_counts/low_signal")
            .and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        report
            .pointer(
                "/measurement_split/live_retrieval_health/excellent_blocked_by_retrieval_quality"
            )
            .and_then(Value::as_u64),
        Some(1)
    );
}

#[test]
fn research_golden_accepts_no_results_retrieval_failure_as_low_evidence_synthesis() {
    let root = temp_path("research_golden_no_results_low_evidence");
    let cases = root.join("cases.json");
    let responses = root.join("responses.json");
    write_json_file(&cases, &dataset());
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_gold_test",
                "response_payload": {
                    "response": "Bounded conclusion: use this only as a low-evidence signal, retry with narrower source targets, and avoid treating the absence of retrievable evidence as a product judgment. The web search returned no directly relevant results for this query and the evidence is limited. That retrieval failure is not evidence that Infring and LangGraph are equivalent. The attempted search covered official docs and comparison terms, but returned zero candidate snippets or evidence refs, so I cannot cite a source-backed winner.",
                    "pending_tool_request": {
                        "status": "executed",
                        "tool_name": "web_search",
                        "selected_tool_family": "Web Search / Fetch",
                        "input": {
                            "query": "Infring LangGraph comparison current docs",
                            "aperture": "medium"
                        }
                    },
                    "tools": [{
                        "name": "web_search",
                        "status": "no_results",
                        "raw_results": [{
                            "title": "No usable result",
                            "snippet": "No results were returned for the query."
                        }],
                        "result": "No results: provider returned no usable source snippets.",
                        "evidence_refs": [
                            {"title": "No usable result", "locator": "tool:no-results", "score": 0.0}
                        ]
                    }],
                    "response_finalization": {
                        "outcome": "workflow_authored+tool_completion:no_results+synthesized",
                        "tool_completion": {
                            "completion_state": "no_results",
                            "findings_available": false,
                            "evidence_refs_used": ["tool:no-results"]
                        }
                    }
                }
            }]
        }),
    );
    let code = run_research_golden(&runner_args(&root, &cases, &responses, false));
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    let checkpoints = report
        .pointer("/cases/0/gate_transition_diagnostics/checkpoints")
        .and_then(Value::as_array)
        .expect("checkpoints");
    let gate_6a = checkpoints
        .iter()
        .find(|row| {
            row.get("checkpoint").and_then(Value::as_str)
                == Some("6a_synthesis_uses_evidence_or_low_evidence_fallback")
        })
        .expect("6a checkpoint");
    assert_eq!(gate_6a.get("status").and_then(Value::as_str), Some("pass"));
    assert_ne!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/synthesis_failure_class")
            .and_then(Value::as_str),
        Some("low_signal_not_acknowledged")
    );
}

#[test]
fn research_golden_does_not_mark_provider_empty_from_final_answer_caveat_text() {
    let root = temp_path("research_golden_no_results_phrase_not_provider_status");
    let cases = root.join("cases.json");
    let responses = root.join("responses.json");
    write_json_file(&cases, &dataset());
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_gold_test",
                "response_payload": {
                    "response": "Based on the retrieved source evidence, LangGraph has stronger public documentation for durable graph workflows while Infring should be treated as the local comparison target. There were no results for one narrow sub-question, but the run still returned usable source snippets and evidence refs for the main comparison. Bounded conclusion: use the available evidence for directional comparison and avoid over-claiming unsupported gaps.",
                    "pending_tool_request": {
                        "status": "executed",
                        "tool_name": "batch_query",
                        "tool_key": "batch_query",
                        "selected_tool_key": "batch_query",
                        "selected_tool_family": "web_research",
                        "input": {
                            "query": "Infring LangGraph comparison current docs",
                            "queries": [
                                "Infring LangGraph comparison current docs",
                                "LangGraph durable execution docs"
                            ],
                            "keywords": ["Infring", "LangGraph", "durable execution"],
                            "required_coverage": ["official docs", "current source evidence"],
                            "query_metadata_policy": {
                                "classification": "expanded_query_pack"
                            },
                            "aperture": "medium",
                            "source": "web"
                        }
                    },
                    "tools": [{
                        "name": "batch_query",
                        "status": "done",
                        "raw_results": [
                            {
                                "title": "LangGraph durable execution docs",
                                "snippet": "LangGraph documents durable execution and graph-based agent orchestration."
                            },
                            {
                                "title": "Infring workflow policy",
                                "snippet": "Infring workflow CDs define gates, transitions, and final-output contracts."
                            }
                        ],
                        "result": "Returned usable source snippets for the main comparison.",
                        "evidence_refs": [
                            {"title": "LangGraph durable execution docs", "locator": "https://docs.example/langgraph", "score": 0.92},
                            {"title": "Infring workflow policy", "locator": "workspace:docs/workflow_json_format_policy.md", "score": 0.88}
                        ]
                    }],
                    "response_workflow": {
                        "evidence_refs": ["evidence:langgraph", "evidence:infring"],
                        "final_llm_response": {
                            "status": "synthesized",
                            "evidence_refs_used": ["evidence:langgraph", "evidence:infring"]
                        }
                    },
                    "response_finalization": {
                        "outcome": "workflow_authored+tool_completion:synthesized",
                        "tool_completion": {
                            "completion_state": "synthesized",
                            "findings_available": true,
                            "evidence_refs_used": ["evidence:langgraph", "evidence:infring"],
                            "tool_attempts": [{
                                "tool_name": "batch_query",
                                "status": "done",
                                "raw_results": [
                                    {
                                        "title": "LangGraph durable execution docs",
                                        "snippet": "LangGraph documents durable execution and graph-based agent orchestration."
                                    }
                                ],
                                "evidence_refs": ["evidence:langgraph"]
                            }]
                        }
                    }
                }
            }]
        }),
    );
    let code = run_research_golden(&runner_args(&root, &cases, &responses, false));
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    assert_eq!(
        report.pointer("/cases/0/retrieval_quality/status"),
        Some(&Value::String("usable".to_string()))
    );
    assert_eq!(
        report.pointer("/cases/0/retrieval_quality/usable_evidence"),
        Some(&Value::Bool(true))
    );
    assert_eq!(
        report
            .pointer("/cases/0/retrieval_quality/classification_inputs/status_marker_source")
            .and_then(Value::as_str),
        Some("structured_tool_status_fields_only")
    );

    let web_6 = report
        .pointer("/cases/0/web_tool_gate_diagnostics/gates")
        .and_then(Value::as_array)
        .unwrap()
        .iter()
        .find(|row| {
            row.get("gate").and_then(Value::as_str) == Some("web_6_provider_not_empty_or_degraded")
        })
        .unwrap()
        .get("status")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    assert_eq!(web_6, "pass");
}

#[test]
fn research_golden_6a_lets_usable_tool_quality_override_stale_low_signal_markers() {
    let root = temp_path("research_golden_usable_quality_overrides_stale_low_signal");
    let cases = root.join("cases.json");
    let responses = root.join("responses.json");
    write_json_file(&cases, &dataset());
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_gold_test",
                "response_payload": {
                    "response": "Based on the retrieved source evidence, LangGraph has source-backed support for durable graph orchestration while Infring remains the local comparison target with evidence refs available in this run. Decision boundary: prefer LangGraph when public documentation and mature graph execution matter, and evaluate Infring when the workspace workflow-CD model is the core requirement. Practical recommendation: use the evidence as a directional comparison, cite the retrieved docs, and avoid over-claiming beyond the covered sources.",
                    "pending_tool_request": {
                        "status": "executed",
                        "tool_name": "batch_query",
                        "tool_key": "batch_query",
                        "selected_tool_key": "batch_query",
                        "selected_tool_family": "web_research",
                        "input": {
                            "query": "Infring LangGraph comparison current docs",
                            "queries": [
                                "Infring LangGraph comparison current docs",
                                "LangGraph durable execution docs"
                            ],
                            "keywords": ["Infring", "LangGraph", "durable execution"],
                            "required_coverage": ["official docs", "current source evidence"],
                            "query_metadata_policy": {
                                "classification": "expanded_query_pack"
                            },
                            "aperture": "medium",
                            "source": "web"
                        }
                    },
                    "tools": [{
                        "name": "batch_query",
                        "status": "done",
                        "raw_results": [{
                            "title": "LangGraph durable execution docs",
                            "snippet": "LangGraph documents durable execution and graph-based agent orchestration."
                        }],
                        "result": "Returned usable source snippets for the main comparison.",
                        "evidence_refs": [
                            {"title": "LangGraph durable execution docs", "locator": "https://docs.example/langgraph", "score": 0.92},
                            {"title": "Infring workflow policy", "locator": "workspace:docs/workflow_json_format_policy.md", "score": 0.88}
                        ],
                        "tool_result_quality": {
                            "status": "usable",
                            "usable_evidence": true,
                            "evidence_count": 2,
                            "content_rich_candidate_count": 2,
                            "claim_hint_count": 2
                        }
                    }],
                    "response_workflow": {
                        "evidence_refs": ["evidence:langgraph", "evidence:infring"],
                        "final_llm_response": {
                            "status": "synthesized",
                            "evidence_refs_used": ["evidence:langgraph", "evidence:infring"]
                        }
                    },
                    "response_finalization": {
                        "outcome": "workflow_authored+tool_completion:low_signal+synthesized",
                        "tool_completion": {
                            "completion_state": "low_signal",
                            "reasoning": "One early provider lane had limited evidence, but the normalized tool quality was usable.",
                            "findings_available": true,
                            "evidence_refs_used": ["evidence:langgraph", "evidence:infring"],
                            "tool_attempts": [{
                                "tool_name": "batch_query",
                                "status": "done",
                                "raw_results": [{
                                    "title": "LangGraph durable execution docs",
                                    "snippet": "LangGraph documents durable execution and graph-based agent orchestration."
                                }],
                                "evidence_refs": ["evidence:langgraph", "evidence:infring"],
                                "tool_result_quality": {
                                    "status": "usable",
                                    "usable_evidence": true,
                                    "evidence_count": 2,
                                    "content_rich_candidate_count": 2,
                                    "claim_hint_count": 2
                                }
                            }]
                        }
                    }
                }
            }]
        }),
    );
    let code = run_research_golden(&runner_args(&root, &cases, &responses, false));
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    let gate_6a = report
        .pointer("/cases/0/gate_transition_diagnostics/checkpoints")
        .and_then(Value::as_array)
        .unwrap()
        .iter()
        .find(|row| {
            row.get("checkpoint").and_then(Value::as_str)
                == Some("6a_synthesis_uses_evidence_or_low_evidence_fallback")
        })
        .expect("6a checkpoint");
    assert_eq!(gate_6a.get("status").and_then(Value::as_str), Some("pass"));
    assert_eq!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/synthesis_failure_class")
            .and_then(Value::as_str),
        Some("none")
    );
}

#[test]
fn research_golden_reports_conflicting_provider_state_when_no_results_status_has_artifacts() {
    let root = temp_path("research_golden_conflicting_provider_state");
    let cases = root.join("cases.json");
    let responses = root.join("responses.json");
    write_json_file(&cases, &dataset());
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_gold_test",
                "response_payload": {
                    "response": "Bounded conclusion: the retrieval state is internally inconsistent. It reports no results while also carrying candidate and evidence artifacts for Infring and LangGraph, so this should be treated as a tooling diagnostic before making a comparison claim.",
                    "pending_tool_request": {
                        "status": "executed",
                        "tool_name": "batch_query",
                        "tool_key": "batch_query",
                        "selected_tool_key": "batch_query",
                        "selected_tool_family": "web_research",
                        "input": {
                            "query": "Infring LangGraph comparison current docs",
                            "queries": ["Infring LangGraph comparison current docs"],
                            "keywords": ["Infring", "LangGraph"],
                            "required_coverage": ["official docs"],
                            "query_metadata_policy": {
                                "classification": "expanded_query_pack"
                            },
                            "aperture": "medium",
                            "source": "web"
                        }
                    },
                    "tools": [{
                        "name": "batch_query",
                        "status": "no_results",
                        "raw_results": [{
                            "title": "LangGraph docs",
                            "snippet": "A real candidate row survived packaging."
                        }],
                        "evidence_refs": [
                            {"title": "LangGraph docs", "locator": "https://docs.example/langgraph", "score": 0.8}
                        ]
                    }],
                    "response_finalization": {
                        "outcome": "workflow_authored+tool_completion:no_results+synthesized",
                        "tool_completion": {
                            "completion_state": "no_results",
                            "findings_available": true,
                            "evidence_refs_used": ["evidence:langgraph"],
                            "tool_attempts": [{
                                "tool_name": "batch_query",
                                "status": "no_results",
                                "raw_results": [{
                                    "title": "LangGraph docs",
                                    "snippet": "A real candidate row survived packaging."
                                }],
                                "evidence_refs": ["evidence:langgraph"]
                            }]
                        }
                    }
                }
            }]
        }),
    );
    let code = run_research_golden(&runner_args(&root, &cases, &responses, false));
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    assert_eq!(
        report.pointer("/cases/0/retrieval_quality/status"),
        Some(&Value::String("conflicting_provider_state".to_string()))
    );
    assert_eq!(
        report
            .pointer("/cases/0/retrieval_quality/classification_inputs/evidence_artifact_conflict"),
        Some(&Value::Bool(true))
    );
    assert!(report
        .pointer("/cases/0/retrieval_quality/quality_flags")
        .and_then(Value::as_array)
        .unwrap()
        .iter()
        .any(|row| row.as_str() == Some("evidence_artifact_conflict")));
    assert_eq!(
        report
            .pointer("/cases/0/web_tool_gate_diagnostics/first_failed_gate")
            .and_then(Value::as_str),
        Some("web_6_provider_not_empty_or_degraded")
    );
}

#[test]
fn research_golden_reports_web_tooling_gate_splits_separately() {
    let root = temp_path("research_golden_web_tooling_gates");
    let cases = root.join("cases.json");
    let responses = root.join("responses.json");
    write_json_file(&cases, &dataset());
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_gold_test",
                "response_payload": {
                    "response": "Bounded conclusion: this retrieval attempt produced low-evidence source coverage, so treat the result as a diagnostic rather than a source-backed framework comparison. The provider returned no directly relevant results, which supports only the operational conclusion that the query should be expanded or retargeted. It does not prove anything about Infring or LangGraph. The useful takeaway is to retry with official docs, repository pages, and dated release sources before making a product decision.",
                    "pending_tool_request": {
                        "status": "executed",
                        "tool_name": "batch_query",
                        "tool_key": "batch_query",
                        "selected_tool_key": "batch_query",
                        "selected_tool_family": "web_research",
                        "input": {
                            "query": "Infring LangGraph comparison current docs",
                            "aperture": "medium",
                            "source": "web"
                        }
                    },
                    "tools": [{
                        "name": "batch_query",
                        "status": "no_results",
                        "provider_results": [{
                            "title": "No usable result",
                            "snippet": "No directly relevant source snippets were returned."
                        }],
                        "search_results": [{
                            "title": "No usable result",
                            "snippet": "No directly relevant source snippets were returned."
                        }],
                        "result": "No results: provider returned no usable source snippets.",
                        "evidence_refs": [
                            {"title": "No usable result", "locator": "tool:no-results", "score": 0.0}
                        ]
                    }],
                    "response_finalization": {
                        "outcome": "workflow_authored+tool_completion:no_results+synthesized",
                        "tool_completion": {
                            "completion_state": "no_results",
                            "findings_available": false,
                            "evidence_refs": ["tool:no-results"],
                            "evidence_refs_used": ["tool:no-results"],
                            "tool_attempts": [{
                                "tool_name": "batch_query",
                                "status": "no_results",
                                "provider_results": [{
                                    "title": "No usable result",
                                    "snippet": "No directly relevant source snippets were returned."
                                }],
                                "search_results": [{
                                    "title": "No usable result",
                                    "snippet": "No directly relevant source snippets were returned."
                                }],
                                "evidence_refs": ["tool:no-results"]
                            }]
                        }
                    }
                }
            }]
        }),
    );
    let code = run_research_golden(&runner_args(&root, &cases, &responses, false));
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    assert_eq!(
        report
            .pointer("/cases/0/web_tool_gate_diagnostics/first_failed_gate")
            .and_then(Value::as_str),
        Some("web_2_query_metadata_present")
    );
    assert_eq!(
        report
            .pointer("/cases/0/web_tool_gate_diagnostics/operator_metrics/primary_bottleneck")
            .and_then(Value::as_str),
        Some("query_planning_metadata_missing")
    );
    assert_eq!(
        report
            .pointer("/cases/0/web_tool_gate_diagnostics/operator_metrics/layer_bottleneck")
            .and_then(Value::as_str),
        Some("query_planning")
    );
    assert_eq!(
        report
            .pointer("/cases/0/web_tool_gate_diagnostics/operator_metrics/candidate_supply/raw_candidate_count")
            .and_then(Value::as_u64),
        Some(4)
    );
    assert_eq!(
        report
            .pointer("/cases/0/web_tool_gate_diagnostics/operator_metrics/packaging/evidence_count")
            .and_then(Value::as_u64),
        Some(3)
    );
    assert_eq!(
        report
            .pointer(
                "/measurement_split/web_tooling/first_failure_counts/web_2_query_metadata_present"
            )
            .and_then(Value::as_u64),
        Some(1)
    );

    let gates = report
        .get("web_tool_gate_pass_rates")
        .and_then(Value::as_array)
        .expect("web gate rates");
    let gate_status = |name: &str| {
        gates
            .iter()
            .find(|row| row.get("gate").and_then(Value::as_str) == Some(name))
            .and_then(|row| row.get("pass_rate").and_then(Value::as_f64))
            .unwrap_or(-1.0)
    };
    assert_eq!(gate_status("web_1_request_shape_present"), 1.0);
    assert_eq!(gate_status("web_2_query_metadata_present"), 0.0);
    assert_eq!(gate_status("web_4_raw_candidates_present"), 1.0);
    assert_eq!(gate_status("web_5_packaged_evidence_present"), 1.0);
    assert_eq!(gate_status("web_6_provider_not_empty_or_degraded"), 0.0);
    assert_eq!(gate_status("web_7_usable_evidence_available"), 0.0);

    let metrics = report
        .pointer("/measurement_split/web_tooling/gate_metrics")
        .and_then(Value::as_array)
        .expect("web gate metrics");
    let query_metric = metrics
        .iter()
        .find(|row| row.get("gate").and_then(Value::as_str) == Some("web_2_query_metadata_present"))
        .expect("query metadata metric");
    assert_eq!(query_metric.get("total").and_then(Value::as_u64), Some(1));
    assert_eq!(query_metric.get("passed").and_then(Value::as_u64), Some(0));
    assert_eq!(query_metric.get("failed").and_then(Value::as_u64), Some(1));
    assert_eq!(
        query_metric
            .get("first_failure_count")
            .and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        query_metric.get("artifact_present").and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        query_metric.get("pass_rate").and_then(Value::as_f64),
        Some(0.0)
    );
    assert_eq!(
        report
            .pointer("/measurement_split/web_tooling/operator_metrics/top_first_failure/name")
            .and_then(Value::as_str),
        Some("web_2_query_metadata_present")
    );
    assert_eq!(
        report
            .pointer(
                "/measurement_split/web_tooling/operator_metrics/averages/raw_candidates_per_case"
            )
            .and_then(Value::as_f64),
        Some(4.0)
    );
    assert_eq!(
        report
            .pointer("/measurement_split/web_tooling/operator_metrics/top_layer")
            .and_then(Value::as_str),
        Some("query_planning")
    );
    assert_eq!(
        report
            .pointer("/measurement_split/web_tooling/operator_metrics/conversion_rates/usable_evidence_case_rate")
            .and_then(Value::as_f64),
        Some(0.0)
    );
}

#[test]
fn research_golden_web_search_metadata_moves_failure_to_provider_quality() {
    let root = temp_path("research_golden_web_search_metadata_provider_quality");
    let cases = root.join("cases.json");
    let responses = root.join("responses.json");
    write_json_file(&cases, &dataset());
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_gold_test",
                "response_payload": {
                    "response": "Bounded conclusion: this narrow web lookup ran with explicit query metadata, but the returned candidates were too weak to support a source-backed answer.",
                    "pending_tool_request": {
                        "status": "executed",
                        "tool_name": "web_search",
                        "tool_key": "web_search",
                        "selected_tool_key": "web_search",
                        "selected_tool_family": "web_research",
                        "input": {
                            "query": "latest Rust 2026 release notes",
                            "keywords": ["Rust", "2026", "release notes"],
                            "required_coverage": {
                                "entities": ["Rust"],
                                "facets": ["release notes", "current version"]
                            },
                            "query_metadata_policy": {
                                "classification": "narrow_lookup_or_initial_discovery"
                            },
                            "aperture": "medium"
                        }
                    },
                    "tools": [{
                        "name": "web_search",
                        "status": "no_results",
                        "provider_results": [{
                            "title": "No usable result",
                            "snippet": "No directly relevant source snippets were returned."
                        }],
                        "search_results": [{
                            "title": "No usable result",
                            "snippet": "No directly relevant source snippets were returned."
                        }],
                        "evidence_refs": [
                            {"title": "No usable result", "locator": "tool:no-results", "score": 0.0}
                        ]
                    }],
                    "response_finalization": {
                        "outcome": "workflow_authored+tool_completion:no_results+synthesized",
                        "tool_completion": {
                            "completion_state": "no_results",
                            "findings_available": false,
                            "evidence_refs": ["tool:no-results"],
                            "evidence_refs_used": ["tool:no-results"],
                            "tool_attempts": [{
                                "tool_name": "web_search",
                                "status": "no_results",
                                "provider_results": [{
                                    "title": "No usable result",
                                    "snippet": "No directly relevant source snippets were returned."
                                }],
                                "search_results": [{
                                    "title": "No usable result",
                                    "snippet": "No directly relevant source snippets were returned."
                                }],
                                "evidence_refs": ["tool:no-results"]
                            }]
                        }
                    }
                }
            }]
        }),
    );
    let code = run_research_golden(&runner_args(&root, &cases, &responses, false));
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    assert_eq!(
        report
            .pointer("/cases/0/query_metadata_diagnostics/eligible_web_retrieval_request")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        report
            .pointer("/cases/0/query_metadata_diagnostics/query_lane_count")
            .and_then(Value::as_u64),
        Some(0)
    );
    assert_eq!(
        report
            .pointer("/cases/0/query_metadata_diagnostics/keyword_count")
            .and_then(Value::as_u64),
        Some(3)
    );
    assert_eq!(
        report
            .pointer("/cases/0/query_metadata_diagnostics/required_coverage_entities_count")
            .and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        report
            .pointer("/cases/0/query_metadata_diagnostics/required_coverage_facets_count")
            .and_then(Value::as_u64),
        Some(2)
    );
    assert_eq!(
        report
            .pointer(
                "/cases/0/web_tool_gate_diagnostics/operator_metrics/query_planning/keyword_count"
            )
            .and_then(Value::as_u64),
        Some(3)
    );
    assert_eq!(
        report
            .pointer("/cases/0/web_tool_gate_diagnostics/operator_metrics/query_planning/multi_query_present")
            .and_then(Value::as_bool),
        Some(false)
    );
    let gates = report
        .get("web_tool_gate_pass_rates")
        .and_then(Value::as_array)
        .expect("web gate rates");
    let gate_status = |name: &str| {
        gates
            .iter()
            .find(|row| row.get("gate").and_then(Value::as_str) == Some(name))
            .and_then(|row| row.get("pass_rate").and_then(Value::as_f64))
            .unwrap_or(-1.0)
    };
    assert_eq!(gate_status("web_2_query_metadata_present"), 1.0);
    assert_eq!(
        report
            .pointer("/cases/0/web_tool_gate_diagnostics/first_failed_gate")
            .and_then(Value::as_str),
        Some("web_6_provider_not_empty_or_degraded")
    );
    assert_eq!(
        report
            .pointer("/cases/0/web_tool_gate_diagnostics/operator_metrics/primary_bottleneck")
            .and_then(Value::as_str),
        Some("provider_empty_or_degraded")
    );
    assert_eq!(
        report
            .pointer("/measurement_split/query_metadata_planning/eligible_web_retrieval_requests")
            .and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        report
            .pointer("/measurement_split/query_metadata_planning/metadata_present_rate")
            .and_then(Value::as_f64),
        Some(1.0)
    );
}

#[test]
fn research_golden_web_tooling_gates_split_thin_rows_from_materialized_evidence() {
    let root = temp_path("research_golden_web_tooling_thin_materialization");
    let cases = root.join("cases.json");
    let responses = root.join("responses.json");
    write_json_file(&cases, &dataset());
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_gold_test",
                "response_payload": {
                    "response": "Bounded conclusion: the retrieval returned candidate rows for Infring and LangGraph, but the available snippets are too thin to support a useful comparison. Treat this as a tooling materialization miss, not evidence about the frameworks.",
                    "pending_tool_request": {
                        "status": "executed",
                        "tool_name": "batch_query",
                        "tool_key": "batch_query",
                        "selected_tool_key": "batch_query",
                        "selected_tool_family": "web_research",
                        "input": {
                            "query": "Infring LangGraph comparison current docs",
                            "queries": [
                                "Infring LangGraph comparison current docs",
                                "LangGraph official docs durable execution"
                            ],
                            "keywords": ["Infring", "LangGraph", "official docs"],
                            "required_coverage": ["current docs", "workflow reliability"],
                            "query_metadata_policy": {
                                "classification": "expanded_query_pack"
                            },
                            "aperture": "medium",
                            "source": "web"
                        }
                    },
                    "tools": [{
                        "name": "batch_query",
                        "status": "done",
                        "raw_results": [
                            {
                                "title": "LangGraph docs",
                                "locator": "https://docs.example/langgraph",
                                "snippet": "LangGraph docs."
                            },
                            {
                                "title": "Infring notes",
                                "locator": "https://docs.example/infring",
                                "snippet": "Infring notes."
                            }
                        ],
                        "evidence_refs": [
                            {"title": "LangGraph docs", "locator": "https://docs.example/langgraph", "score": 0.82},
                            {"title": "Infring notes", "locator": "https://docs.example/infring", "score": 0.8}
                        ]
                    }],
                    "response_finalization": {
                        "outcome": "workflow_authored+tool_completion:synthesized",
                        "tool_completion": {
                            "completion_state": "synthesized",
                            "findings_available": true,
                            "evidence_refs_used": ["evidence:langgraph", "evidence:infring"],
                            "tool_attempts": [{
                                "tool_name": "batch_query",
                                "status": "done",
                                "raw_results": [
                                    {
                                        "title": "LangGraph docs",
                                        "locator": "https://docs.example/langgraph",
                                        "snippet": "LangGraph docs."
                                    }
                                ],
                                "evidence_refs": ["evidence:langgraph"]
                            }]
                        }
                    }
                }
            }]
        }),
    );
    let code = run_research_golden(&runner_args(&root, &cases, &responses, false));
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    assert_eq!(
        report.pointer("/cases/0/retrieval_quality/status"),
        Some(&Value::String("low_signal".to_string()))
    );
    assert_eq!(
        report.pointer("/cases/0/retrieval_quality/materialized_candidate_count"),
        Some(&Value::Number(0.into()))
    );
    assert_eq!(
        report.pointer("/cases/0/retrieval_quality/content_rich_candidate_count"),
        Some(&Value::Number(0.into()))
    );
    assert_eq!(
        report.pointer("/cases/0/retrieval_quality/claim_hint_count"),
        Some(&Value::Number(0.into()))
    );
    assert_eq!(
        report
            .pointer("/cases/0/web_tool_gate_diagnostics/operator_metrics/query_planning/query_lane_count")
            .and_then(Value::as_u64),
        Some(2)
    );
    assert_eq!(
        report
            .pointer("/cases/0/web_tool_gate_diagnostics/operator_metrics/query_planning/followup_query_count")
            .and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        report
            .pointer("/cases/0/web_tool_gate_diagnostics/operator_metrics/candidate_supply/unique_source_domains")
            .and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        report
            .pointer("/cases/0/web_tool_gate_diagnostics/first_failed_gate")
            .and_then(Value::as_str),
        Some("web_5b_content_rich_candidates_present")
    );
    assert_eq!(
        report
            .pointer("/cases/0/web_tool_gate_diagnostics/inferred_failure_boundary")
            .and_then(Value::as_str),
        Some("candidate_content_materialization_missing")
    );
}

#[test]
fn research_golden_reports_web_access_blockers_separately() {
    let root = temp_path("research_golden_web_access_blocker_gate");
    let cases = root.join("cases.json");
    let responses = root.join("responses.json");
    write_json_file(&cases, &dataset());
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_gold_test",
                "response_payload": {
                    "response": "The web retrieval attempt was blocked before usable source evidence was available. The only grounded conclusion is that access was rate-limited, so this run should be treated as an access diagnostic rather than evidence about the requested comparison.",
                    "pending_tool_request": {
                        "status": "executed",
                        "tool_name": "batch_query",
                        "tool_key": "batch_query",
                        "selected_tool_key": "batch_query",
                        "selected_tool_family": "web_research",
                        "input": {
                            "query": "Infring LangGraph comparison current docs",
                            "aperture": "medium",
                            "queries": [
                                "Infring LangGraph comparison current docs",
                                "LangGraph official docs durable execution",
                                "Infring workflow CD gates research"
                            ],
                            "keywords": ["Infring", "LangGraph", "workflow", "durable execution"],
                            "required_coverage": ["official docs", "current source evidence"],
                            "query_metadata_policy": {
                                "classification": "expanded_query_pack"
                            }
                        }
                    },
                    "tools": [{
                        "name": "batch_query",
                        "status": "error",
                        "status_code": 429,
                        "error": "HTTP 429 Too Many Requests",
                        "result": "Provider returned 429 Too Many Requests with Retry-After: 60. The page showed a Cloudflare CAPTCHA / verify you are human challenge, so no source snippets were available."
                    }],
                    "response_finalization": {
                        "outcome": "workflow_authored+tool_completion:access_blocked+synthesized",
                        "tool_completion": {
                            "completion_state": "provider_error",
                            "findings_available": false,
                            "tool_attempts": [{
                                "tool_name": "batch_query",
                                "status": "error",
                                "status_code": 429,
                                "error": "HTTP 429 Too Many Requests",
                                "result": "Cloudflare CAPTCHA challenge and Retry-After throttling blocked candidate retrieval."
                            }]
                        }
                    }
                }
            }]
        }),
    );
    let code = run_research_golden(&runner_args(&root, &cases, &responses, false));
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    assert_eq!(
        report
            .pointer("/cases/0/web_tool_gate_diagnostics/first_failed_gate")
            .and_then(Value::as_str),
        Some("web_3b1_provider_quota_not_rate_limited")
    );
    assert_eq!(
        report
            .pointer("/cases/0/web_tool_gate_diagnostics/inferred_failure_boundary")
            .and_then(Value::as_str),
        Some("provider_rate_limited_or_quota_exhausted")
    );
    assert_eq!(
        report.pointer("/cases/0/web_tool_gate_diagnostics/access_blocker/detected"),
        Some(&Value::Bool(true))
    );
    assert_eq!(
        report
            .pointer("/cases/0/web_tool_gate_diagnostics/access_blocker/kind")
            .and_then(Value::as_str),
        Some("anti_bot_or_throttle")
    );
    assert_eq!(
        report
            .pointer("/measurement_split/web_tooling/access_blocker_counts/anti_bot_or_throttle")
            .and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        report
            .pointer(
                "/measurement_split/web_tooling/access_blocker_class_counts/rate_limit_or_quota"
            )
            .and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        report
            .pointer(
                "/measurement_split/web_tooling/access_blocker_class_counts/anti_bot_challenge"
            )
            .and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        report
            .pointer("/measurement_split/web_tooling/access_blocker_signal_counts/http_status_429")
            .and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        report
            .pointer("/measurement_split/web_tooling/measured_cases")
            .and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        report
            .pointer("/measurement_split/web_tooling/transport_excluded_cases")
            .and_then(Value::as_u64),
        Some(0)
    );

    let gates = report
        .get("web_tool_gate_pass_rates")
        .and_then(Value::as_array)
        .expect("web gate rates");
    let access_gate_rate = gates
        .iter()
        .find(|row| {
            row.get("gate").and_then(Value::as_str)
                == Some("web_3b1_provider_quota_not_rate_limited")
        })
        .and_then(|row| row.get("pass_rate").and_then(Value::as_f64));
    assert_eq!(access_gate_rate, Some(0.0));

    let metrics = report
        .get("web_tool_gate_metrics")
        .and_then(Value::as_array)
        .expect("top-level web gate metrics");
    let access_metric = metrics
        .iter()
        .find(|row| {
            row.get("gate").and_then(Value::as_str)
                == Some("web_3b1_provider_quota_not_rate_limited")
        })
        .expect("access blocker metric");
    assert_eq!(access_metric.get("failed").and_then(Value::as_u64), Some(1));
    assert_eq!(
        access_metric
            .get("first_failure_count")
            .and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        access_metric
            .get("artifact_present_failures")
            .and_then(Value::as_u64),
        Some(1)
    );
}

#[test]
fn research_golden_rejects_tool_status_overlead_without_bounded_answer() {
    let root = temp_path("research_golden_status_overlead");
    let cases = root.join("cases.json");
    let responses = root.join("responses.json");
    write_json_file(&cases, &dataset());
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_gold_test",
                "response_payload": {
                    "response": "The web search returned no directly relevant results for this query and the evidence is limited. I cannot make a source-backed comparison, and the right next step is to retry with a narrower query. The retrieval failure does not prove the tools are equivalent, but it also does not support choosing one.",
                    "pending_tool_request": {
                        "status": "executed",
                        "tool_name": "web_search",
                        "selected_tool_family": "Web Search / Fetch",
                        "input": {
                            "query": "Infring LangGraph comparison current docs",
                            "aperture": "medium"
                        }
                    },
                    "tools": [{
                        "name": "web_search",
                        "status": "no_results",
                        "raw_results": [{
                            "title": "No usable result",
                            "snippet": "No results were returned for the query."
                        }],
                        "result": "No results: provider returned no usable source snippets.",
                        "evidence_refs": [
                            {"title": "No usable result", "locator": "tool:no-results", "score": 0.0}
                        ]
                    }],
                    "response_finalization": {
                        "outcome": "workflow_authored+tool_completion:no_results+synthesized",
                        "tool_completion": {
                            "completion_state": "no_results",
                            "findings_available": false,
                            "evidence_refs_used": ["tool:no-results"]
                        }
                    }
                }
            }]
        }),
    );
    let code = run_research_golden(&runner_args(&root, &cases, &responses, false));
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    assert_eq!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/first_failed_checkpoint")
            .and_then(Value::as_str),
        Some("6a_synthesis_uses_evidence_or_low_evidence_fallback")
    );
}

#[test]
fn research_golden_allows_post_tool_synthesis_without_fresh_request_candidate() {
    let root = temp_path("research_golden_post_tool_no_fresh_candidate");
    let cases = root.join("cases.json");
    let responses = root.join("responses.json");
    write_json_file(
        &cases,
        &json!({
            "reliability_thresholds": {
                "min_cases_for_reliability_claim": 1,
                "workflow_gate_pass_min": 0.95,
                "research_success_min": 0.85
            },
            "scoring_contract": {
                "pass_score": 85,
                "excellent_score": 95
            },
            "cases": [{
                "id": "research_gold_post_tool",
                "category": "post_tool_synthesis",
                "prompt": "After the web tool returns low-signal results for Infring, synthesize a useful answer anyway.",
                "expected_gate_path": {
                    "gate_1": "tool_required_or_pending_tool_result",
                    "gate_2": "web_research",
                    "gate_3": "web_search",
                    "gate_4_required_fields": ["query", "aperture"],
                    "post_tool": "must_synthesize_from_low_signal_evidence"
                },
                "required_entities": ["Infring"]
            }]
        }),
    );
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_gold_post_tool",
                "response_payload": {
                    "response": "The first result was low signal, so no source-backed conclusion is available yet. What we know is that public evidence about Infring is sparse. What we do not know is how it compares head to head with other frameworks from current source material. The next useful action is to narrow the query to one competitor at a time."
                }
            }]
        }),
    );
    let code = run_research_golden(&runner_args(&root, &cases, &responses, false));
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    let checkpoints = report
        .pointer("/cases/0/gate_transition_diagnostics/checkpoints")
        .and_then(Value::as_array)
        .expect("checkpoints");
    let gate_4a = checkpoints
        .iter()
        .find(|row| {
            row.get("checkpoint").and_then(Value::as_str) == Some("4a_request_template_signaled")
        })
        .expect("4a checkpoint");
    assert_eq!(gate_4a.get("status").and_then(Value::as_str), Some("pass"));
    let gate_4b = checkpoints
        .iter()
        .find(|row| {
            row.get("checkpoint").and_then(Value::as_str)
                == Some("4b_tool_request_candidate_present")
        })
        .expect("4b checkpoint");
    assert_eq!(gate_4b.get("status").and_then(Value::as_str), Some("pass"));
    let gate_5a = checkpoints
        .iter()
        .find(|row| {
            row.get("checkpoint").and_then(Value::as_str) == Some("5a_tool_execution_recorded")
        })
        .expect("5a checkpoint");
    assert_eq!(gate_5a.get("status").and_then(Value::as_str), Some("pass"));
    let gate_5b = checkpoints
        .iter()
        .find(|row| {
            row.get("checkpoint").and_then(Value::as_str) == Some("5b_raw_provider_result_present")
        })
        .expect("5b checkpoint");
    assert_eq!(gate_5b.get("status").and_then(Value::as_str), Some("pass"));
    let gate_5c = checkpoints
        .iter()
        .find(|row| {
            row.get("checkpoint").and_then(Value::as_str) == Some("5c_packaged_tool_result_present")
        })
        .expect("5c checkpoint");
    assert_eq!(gate_5c.get("status").and_then(Value::as_str), Some("pass"));
    let gate_5d = checkpoints
        .iter()
        .find(|row| {
            row.get("checkpoint").and_then(Value::as_str) == Some("5d_evidence_refs_extracted")
        })
        .expect("5d checkpoint");
    assert_eq!(gate_5d.get("status").and_then(Value::as_str), Some("pass"));
    let gate_5e = checkpoints
        .iter()
        .find(|row| {
            row.get("checkpoint").and_then(Value::as_str)
                == Some("5e_agent_received_evidence_context")
        })
        .expect("5e checkpoint");
    assert_eq!(gate_5e.get("status").and_then(Value::as_str), Some("pass"));
    assert_ne!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/first_failed_checkpoint")
            .and_then(Value::as_str),
        Some("4a_request_template_signaled")
    );
    assert_ne!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/first_failed_checkpoint")
            .and_then(Value::as_str),
        Some("4b_tool_request_candidate_present")
    );
    assert_ne!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/first_failed_checkpoint")
            .and_then(Value::as_str),
        Some("5a_tool_execution_recorded")
    );
}

#[test]
fn research_golden_allows_bounded_missing_tool_context_fallback_at_6a() {
    let root = temp_path("research_golden_missing_tool_context_fallback");
    let cases = root.join("cases.json");
    let responses = root.join("responses.json");
    write_json_file(
        &cases,
        &json!({
            "reliability_thresholds": {
                "min_cases_for_reliability_claim": 1,
                "workflow_gate_pass_min": 0.95,
                "research_success_min": 0.85
            },
            "scoring_contract": {
                "pass_score": 85,
                "excellent_score": 95
            },
            "cases": [{
                "id": "research_gold_post_tool_missing_context",
                "category": "post_tool_synthesis",
                "prompt": "After the web tool returns several source snippets about agent frameworks, synthesize the tradeoffs and cite evidence refs without dumping the raw payload.",
                "expected_gate_path": {
                    "gate_1": "tool_required_or_pending_tool_result",
                    "gate_2": "web_research",
                    "gate_3": "web_search",
                    "gate_4_required_fields": ["query", "aperture"],
                    "post_tool": "must_synthesize_from_evidence_refs"
                },
                "required_entities": ["agent frameworks"]
            }]
        }),
    );
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_gold_post_tool_missing_context",
                "response_payload": {
                    "response": "No returned tool result is available in this turn, so no source-backed synthesis is available yet. What we know is that agent frameworks usually involve a tradeoff between autonomy, observability, and deployment maturity. What we do not know is which current frameworks the missing snippets supported, and that would require live research. The next best search query is agent framework tradeoff benchmark observability deployment 2026."
                }
            }]
        }),
    );
    let code = run_research_golden(&runner_args(&root, &cases, &responses, false));
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    let checkpoints = report
        .pointer("/cases/0/gate_transition_diagnostics/checkpoints")
        .and_then(Value::as_array)
        .expect("checkpoints");
    let gate_6a = checkpoints
        .iter()
        .find(|row| {
            row.get("checkpoint").and_then(Value::as_str)
                == Some("6a_synthesis_uses_evidence_or_low_evidence_fallback")
        })
        .expect("6a checkpoint");
    assert_eq!(gate_6a.get("status").and_then(Value::as_str), Some("pass"));
    assert_ne!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/first_failed_checkpoint")
            .and_then(Value::as_str),
        Some("6a_synthesis_uses_evidence_or_low_evidence_fallback")
    );
}

#[test]
fn research_golden_allows_explicit_missing_tool_fallback_when_tools_array_is_empty() {
    let root = temp_path("research_golden_missing_tool_empty_tools");
    let cases = root.join("cases.json");
    let responses = root.join("responses.json");
    write_json_file(
        &cases,
        &json!({
            "reliability_thresholds": {
                "min_cases_for_reliability_claim": 1,
                "workflow_gate_pass_min": 0.95,
                "research_success_min": 0.85
            },
            "scoring_contract": {
                "pass_score": 85,
                "excellent_score": 95
            },
            "cases": [{
                "id": "research_gold_post_tool_missing_context_empty_tools",
                "category": "post_tool_synthesis",
                "prompt": "After the web tool returns several source snippets about agent frameworks, synthesize the tradeoffs and cite evidence refs without dumping the raw payload.",
                "expected_gate_path": {
                    "gate_1": "tool_required_or_pending_tool_result",
                    "gate_2": "web_research",
                    "gate_3": "web_search",
                    "gate_4_required_fields": ["query", "aperture"],
                    "post_tool": "must_synthesize_from_evidence_refs"
                },
                "required_entities": ["agent frameworks"]
            }]
        }),
    );
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_gold_post_tool_missing_context_empty_tools",
                "response_payload": {
                    "tools": [],
                    "response": "No returned tool result is available in this turn, so no source-backed synthesis is available yet. What we know is that agent frameworks usually involve a tradeoff between autonomy, observability, and deployment maturity, but no returned tool result, snippets, or evidence refs are present in this turn. What we do not know is which agent frameworks the missing snippets supported or what evidence refs they would justify, so no source-backed comparison is available yet. My recommendation is to rerun one focused source-backed comparison before drawing conclusions. The next best search query is agent framework tradeoff benchmark observability deployment 2026."
                }
            }]
        }),
    );
    let code = run_research_golden(&runner_args(&root, &cases, &responses, false));
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    let checkpoints = report
        .pointer("/cases/0/gate_transition_diagnostics/checkpoints")
        .and_then(Value::as_array)
        .expect("checkpoints");
    let gate_6a = checkpoints
        .iter()
        .find(|row| {
            row.get("checkpoint").and_then(Value::as_str)
                == Some("6a_synthesis_uses_evidence_or_low_evidence_fallback")
        })
        .expect("6a checkpoint");
    assert_eq!(gate_6a.get("status").and_then(Value::as_str), Some("pass"));
    assert_ne!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/first_failed_checkpoint")
            .and_then(Value::as_str),
        Some("6a_synthesis_uses_evidence_or_low_evidence_fallback")
    );
}

#[test]
fn research_golden_rejects_internal_runtime_context_as_post_tool_evidence() {
    let root = temp_path("research_golden_internal_context_not_evidence");
    let cases = root.join("cases.json");
    let responses = root.join("responses.json");
    write_json_file(
        &cases,
        &json!({
            "reliability_thresholds": {
                "min_cases_for_reliability_claim": 1,
                "workflow_gate_pass_min": 0.95,
                "research_success_min": 0.85
            },
            "scoring_contract": {
                "pass_score": 85,
                "excellent_score": 95
            },
            "cases": [{
                "id": "research_gold_post_tool_internal_context",
                "category": "post_tool_synthesis",
                "prompt": "After the web tool returns low-signal results for Infring, synthesize a useful answer anyway.",
                "expected_gate_path": {
                    "gate_1": "tool_required_or_pending_tool_result",
                    "gate_2": "web_research",
                    "gate_3": "web_search",
                    "gate_4_required_fields": ["query", "aperture"],
                    "post_tool": "must_synthesize_from_low_signal_evidence"
                },
                "required_entities": ["Infring"]
            }]
        }),
    );
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_gold_post_tool_internal_context",
                "response_payload": {
                    "response": "What we know is the identity context: Infring is the platform hosting this conversation, evident from system instructions and the agent name in the runtime. That platform identity suggests orchestration features, but it is not tied to any returned external evidence. The next step would be to search for public framework comparisons."
                }
            }]
        }),
    );
    let code = run_research_golden(&runner_args(&root, &cases, &responses, false));
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    assert_eq!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/first_failed_checkpoint")
            .and_then(Value::as_str),
        Some("6a_synthesis_uses_evidence_or_low_evidence_fallback")
    );
    assert_eq!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/inferred_failure_boundary")
            .and_then(Value::as_str),
        Some("post_tool_synthesis_not_useful")
    );
}

#[test]
fn research_golden_allows_domain_discussion_of_system_instructions_when_labeled_inference() {
    let root = temp_path("research_golden_system_instruction_domain_content");
    let cases = root.join("cases.json");
    let responses = root.join("responses.json");
    write_json_file(
        &cases,
        &json!({
            "reliability_thresholds": {
                "min_cases_for_reliability_claim": 1,
                "workflow_gate_pass_min": 0.95,
                "research_success_min": 0.85
            },
            "scoring_contract": {
                "pass_score": 85,
                "excellent_score": 95
            },
            "cases": [{
                "id": "research_gold_browser_agent_security",
                "category": "current_state",
                "prompt": "Research current security concerns around AI browser agents. Focus on prompt injection, credential handling, and approval boundaries.",
                "expected_gate_path": {
                    "gate_1": "tool_required",
                    "gate_2": "web_research",
                    "gate_3": "web_search",
                    "gate_4_required_fields": ["query", "aperture"]
                },
                "required_entities": ["prompt injection", "credential handling", "approval boundaries"]
            }]
        }),
    );
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_gold_browser_agent_security",
                "response_payload": {
                    "response": "Current evidence from this turn is limited: the search returned no usable source coverage due to provider degradation. What I can offer is clearly labeled secondary inference, not source-backed findings. Prompt injection is a core risk because browser agents may process untrusted page text alongside system instructions and task instructions. Credential handling should be treated as high-risk because sessions, cookies, and form-fill data can cross approval boundaries. Approval boundaries matter most around navigation, downloads, form submission, payments, and account changes. Bounded guidance: keep sensitive actions gated, separate retrieved evidence from inference, and require explicit confirmation for irreversible or credential-bearing steps.",
                    "pending_tool_request": {
                        "status": "executed",
                        "tool_name": "web_search",
                        "selected_tool_family": "Web Search / Fetch",
                        "input": {
                            "query": "AI browser agent security prompt injection credential handling approval boundaries",
                            "aperture": "medium"
                        }
                    },
                    "tools": [{
                        "name": "web_search",
                        "status": "no_results",
                        "result": "Provider degradation: no usable source snippets were returned.",
                        "evidence_refs": [
                            {"title": "No usable result", "locator": "tool:no-results", "score": 0.0}
                        ]
                    }],
                    "response_finalization": {
                        "outcome": "workflow_authored+tool_completion:no_results+synthesized",
                        "tool_completion": {
                            "completion_state": "no_results",
                            "findings_available": false,
                            "evidence_refs_used": ["tool:no-results"]
                        }
                    }
                }
            }]
        }),
    );
    let code = run_research_golden(&runner_args(&root, &cases, &responses, false));
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    assert_ne!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/synthesis_failure_class")
            .and_then(Value::as_str),
        Some("internal_context_used_as_evidence")
    );
    assert_ne!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/first_failed_checkpoint")
            .and_then(Value::as_str),
        Some("6a_synthesis_uses_evidence_or_low_evidence_fallback")
    );
}
