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
            "id": "research_lifecycle_test",
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

fn runner_args(root: &Path, cases: &Path, responses: &Path) -> Vec<String> {
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
        "--strict=0".to_string(),
    ]
}

#[test]
fn research_success_requires_score_and_lifecycle_gate_completion() {
    let root = temp_path("research_golden_lifecycle_metric");
    let cases = root.join("cases.json");
    let responses = root.join("responses.json");
    write_json_file(&cases, &dataset());
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_lifecycle_test",
                "response_payload": {
                    "response": "According to current docs and source evidence, Infring and LangGraph optimize for different workflow tradeoffs. Infring is better when a team wants editable workflow gates, evidence review, and rollback discipline; LangGraph is better when a Python team wants mature graph orchestration and durable state. My recommendation is LangGraph for a conventional production app and Infring when inspectable workflow CDs matter most. Caveat: public Infring evidence is limited, so verify claims before treating this as complete.",
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
    let code = run_research_golden(&runner_args(&root, &cases, &responses));
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    assert!(
        report
            .pointer("/cases/0/score")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            >= 85
    );
    assert_eq!(
        report.pointer("/cases/0/score_pass"),
        Some(&Value::Bool(true))
    );
    assert_eq!(
        report.pointer("/cases/0/prompt").and_then(Value::as_str),
        Some("Use web research to compare Infring with LangGraph.")
    );
    assert_eq!(
        report
            .pointer("/cases/0/expected_gate_path/gate_4_required_fields")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().filter_map(Value::as_str).collect::<Vec<_>>()),
        Some(vec!["query", "aperture"])
    );
    assert_eq!(
        report
            .pointer("/cases/0/required_entities")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().filter_map(Value::as_str).collect::<Vec<_>>()),
        Some(vec!["Infring", "LangGraph"])
    );
    assert_eq!(report.pointer("/cases/0/pass"), Some(&Value::Bool(false)));
    assert_eq!(
        report.pointer("/cases/0/lifecycle_gate_path_complete"),
        Some(&Value::Bool(false))
    );
    assert_eq!(
        report
            .pointer("/summary/research_success_rate")
            .and_then(Value::as_f64),
        Some(0.0)
    );
    assert_eq!(
        report
            .pointer("/summary/gate_transition_path_ok")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert!(report
        .pointer("/cases/0/failures")
        .and_then(Value::as_array)
        .unwrap()
        .iter()
        .any(|row| row.as_str()
            == Some("research_lifecycle_gate_failed:5b_raw_provider_result_present")));
    assert_eq!(
        report.pointer("/observation_lifecycle/enabled"),
        Some(&Value::Bool(true))
    );
    assert_eq!(
        report.pointer("/observation_lifecycle/ok"),
        Some(&Value::Bool(true))
    );
    assert!(root.join("observation_events.jsonl").exists());
    assert!(root.join("observation_archive.json").exists());
}
