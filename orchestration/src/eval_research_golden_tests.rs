// Layer ownership: orchestration (research eval authority)

use super::*;
use std::collections::BTreeSet;
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
            "tags": ["comparison", "source_sensitive"],
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

fn sample_dataset(case_count: usize) -> Value {
    json!({
        "sampling_policy": {
            "auto_sample_when_limit_is_lower_than_pool": true,
            "recommended_sample_size": 3
        },
        "cases": (0..case_count).map(|idx| {
            json!({
                "id": format!("sample_case_{idx:03}"),
                "category": if idx % 2 == 0 { "software_technical" } else { "business_market" },
                "tags": if idx % 2 == 0 {
                    json!(["comparison", "current"])
                } else {
                    json!(["recommendation", "current"])
                },
                "prompt": format!("Research sample case {idx}."),
                "expected_gate_path": {
                    "gate_1": "tool_required",
                    "gate_2": "web_research",
                    "gate_3": "batch_query",
                    "gate_4_required_fields": ["source", "query", "aperture"]
                },
                "required_entities": []
            })
        }).collect::<Vec<_>>()
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
fn research_cross_domain_fixture_declares_general_other_and_shape_tags() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../validation/evals/fixtures/research_cross_domain_dataset_v1.json");
    let dataset = read_json(path.to_str().unwrap());
    let cases = dataset
        .get("cases")
        .and_then(Value::as_array)
        .expect("cases");
    assert_eq!(cases.len(), 20);
    let expected_categories = [
        "software_technical",
        "science_academic",
        "business_market",
        "policy_legal_civic",
        "consumer_product",
        "local_travel",
        "history_culture",
        "health_medical",
        "news_current_events",
        "general_other",
    ]
    .into_iter()
    .map(ToString::to_string)
    .collect::<BTreeSet<_>>();
    let declared_categories = string_array_at(&dataset, &["domain_taxonomy"])
        .into_iter()
        .collect::<BTreeSet<_>>();
    assert_eq!(declared_categories, expected_categories);
    let seen_categories = cases
        .iter()
        .map(|case| str_at(case, &["category"], ""))
        .collect::<BTreeSet<_>>();
    assert_eq!(seen_categories, expected_categories);
    assert!(seen_categories.contains("general_other"));

    let allowed_tags = string_array_at(&dataset, &["shape_tag_taxonomy"])
        .into_iter()
        .collect::<BTreeSet<_>>();
    let mut ids = BTreeSet::new();
    for case in cases {
        let case_id = str_at(case, &["id"], "");
        assert!(!case_id.is_empty());
        assert!(ids.insert(case_id.clone()), "duplicate case id: {case_id}");
        let tags = string_array_at(case, &["tags"]);
        assert!(!tags.is_empty(), "missing tags: {case_id}");
        for tag in tags {
            assert!(
                allowed_tags.contains(&tag),
                "unknown tag {tag} in {case_id}"
            );
        }
        assert_eq!(
            str_at(case, &["expected_gate_path", "gate_2"], ""),
            "web_research"
        );
        assert!(
            string_array_at(case, &["expected_gate_path", "gate_4_required_fields"])
                .contains(&"query".to_string()),
            "missing query gate field: {case_id}"
        );
    }
}

#[test]
fn research_user_prompt_pool_fixture_declares_100_cases_and_taxonomies() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../validation/evals/fixtures/research_user_prompt_pool_v1.json");
    let dataset = read_json(path.to_str().unwrap());
    let cases = dataset
        .get("cases")
        .and_then(Value::as_array)
        .expect("cases");
    assert_eq!(cases.len(), 100);
    let expected_categories = string_array_at(&dataset, &["domain_taxonomy"])
        .into_iter()
        .collect::<BTreeSet<_>>();
    assert_eq!(expected_categories.len(), 10);
    let allowed_tags = string_array_at(&dataset, &["shape_tag_taxonomy"])
        .into_iter()
        .collect::<BTreeSet<_>>();
    assert!(bool_at(
        &dataset,
        &[
            "sampling_policy",
            "auto_sample_when_limit_is_lower_than_pool"
        ],
        false
    ));
    assert_eq!(
        u64_at(&dataset, &["sampling_policy", "recommended_sample_size"], 0),
        20
    );
    let seen_categories = cases
        .iter()
        .map(|case| str_at(case, &["category"], ""))
        .collect::<BTreeSet<_>>();
    assert_eq!(seen_categories, expected_categories);
    let mut ids = BTreeSet::new();
    for case in cases {
        let case_id = str_at(case, &["id"], "");
        assert!(!case_id.is_empty());
        assert!(ids.insert(case_id.clone()), "duplicate case id: {case_id}");
        let tags = string_array_at(case, &["tags"]);
        assert!(!tags.is_empty(), "missing tags: {case_id}");
        for tag in tags {
            assert!(
                allowed_tags.contains(&tag),
                "unknown tag {tag} in {case_id}"
            );
        }
        assert_eq!(
            str_at(case, &["expected_gate_path", "gate_2"], ""),
            "web_research"
        );
        assert_eq!(
            str_at(case, &["expected_gate_path", "gate_3"], ""),
            "batch_query"
        );
        let required_fields =
            string_array_at(case, &["expected_gate_path", "gate_4_required_fields"]);
        assert!(
            required_fields.contains(&"source".to_string())
                && required_fields.contains(&"query".to_string())
                && required_fields.contains(&"aperture".to_string()),
            "missing gate 4 fields: {case_id}"
        );
    }
}

#[test]
fn research_post_tool_downstream_fixture_replays_cleanly_offline() {
    let root = temp_path("research_post_tool_downstream_fixture");
    let cases = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../validation/evals/fixtures/research_post_tool_downstream_dataset_v1.json");
    let responses = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../validation/evals/fixtures/research_post_tool_downstream_responses_v1.json");
    let code = run_research_golden(&runner_args(&root, &cases, &responses, false));
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    let rows = report.get("cases").and_then(Value::as_array).expect("cases");
    assert_eq!(rows.len(), 8);
    for row in rows {
        assert_eq!(row.get("pass"), Some(&Value::Bool(true)), "{row:#?}");
        assert_eq!(
            row.pointer("/gate_transition_diagnostics/first_failed_checkpoint"),
            Some(&Value::Null),
            "{row:#?}"
        );
        assert!(
            row.get("response_full")
                .and_then(Value::as_str)
                .map(|value| value.split_whitespace().count() >= 20)
                .unwrap_or(false),
            "{row:#?}"
        );
    }
}

#[test]
fn research_upstream_failure_localization_contract_declares_expected_layers() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../validation/evals/fixtures/research_eval_upstream_failure_localization_contract.json",
    );
    let contract = read_json(path.to_str().unwrap());
    assert_eq!(
        string_array_at(&contract, &["layer_order"]),
        vec![
            "run_stability".to_string(),
            "workflow_path".to_string(),
            "retrieval_mechanics".to_string(),
            "evidence_carrythrough".to_string(),
            "synthesis_quality".to_string(),
            "ux_smoke".to_string(),
            "none".to_string()
        ]
    );
}

#[test]
fn research_golden_case_sampling_is_seeded_and_reproducible() {
    let dataset = sample_dataset(8);
    let cases = dataset
        .get("cases")
        .and_then(Value::as_array)
        .cloned()
        .expect("cases");
    let (selected_a, meta_a) =
        select_research_golden_cases(&cases, Some(3), Some("alpha"), usize::MAX);
    let (selected_b, meta_b) =
        select_research_golden_cases(&cases, Some(3), Some("alpha"), usize::MAX);
    let (selected_c, meta_c) =
        select_research_golden_cases(&cases, Some(3), Some("beta"), usize::MAX);
    let ids_a = selected_a
        .iter()
        .map(|case| str_at(case, &["id"], ""))
        .collect::<Vec<_>>();
    let ids_b = selected_b
        .iter()
        .map(|case| str_at(case, &["id"], ""))
        .collect::<Vec<_>>();
    let ids_c = selected_c
        .iter()
        .map(|case| str_at(case, &["id"], ""))
        .collect::<Vec<_>>();
    assert_eq!(ids_a, ids_b);
    assert_ne!(ids_a, ids_c);
    assert_eq!(ids_a.len(), 3);
    assert_eq!(u64_at(&meta_a, &["pool_size"], 0), 8);
    assert_eq!(u64_at(&meta_a, &["effective_sample_size"], 0), 3);
    assert_eq!(
        str_at(&meta_a, &["selection_mode"], ""),
        "deterministic_seeded_sample"
    );
    assert_eq!(str_at(&meta_a, &["effective_sample_seed"], ""), "alpha");
    assert_eq!(
        meta_a.get("selected_case_ids"),
        meta_b.get("selected_case_ids")
    );
    assert_ne!(
        meta_a.get("selected_case_ids"),
        meta_c.get("selected_case_ids")
    );
}

#[test]
fn research_golden_auto_sampling_uses_limit_for_pool_datasets() {
    let dataset = sample_dataset(8);
    let requested = case_selection_requested_sample_size(&[], &dataset, 3);
    assert_eq!(requested, Some(3));
    let requested_unbounded = case_selection_requested_sample_size(&[], &dataset, usize::MAX);
    assert_eq!(requested_unbounded, None);
}

#[test]
fn research_golden_scores_evidenced_final_answer() {
    let root = temp_path("research_golden_pass");
    let cases = root.join("cases.json");
    let responses = root.join("responses.json");
    write_json_file(&cases, &dataset());
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_gold_test",
                "response_payload": {
                    "response": "As of the current docs and release evidence, Infring and LangGraph solve different parts of the agent workflow problem. Infring appears strongest when you want an editable workflow CD with explicit gate tradeoffs and rollback discipline, while LangGraph is best for Python teams that want durable graph orchestration and documented production patterns. My recommendation: use LangGraph for a conventional app team, and keep Infring if the priority is inspecting workflow gates, evidence refs, and the tradeoff between flexible prompts and typed control. The caveat is that public evidence for Infring is sparse, so treat Infring-specific claims as provisional until verified.",
                    "pending_tool_request": {
                        "status": "pending_confirmation",
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
                        "result": "Source evidence from current docs: LangGraph documents graph orchestration and durable state patterns; Infring evidence is limited to workflow CD and gate inspection notes."
                    }],
                    "response_workflow": {
                        "evidence_refs": ["evidence:langgraph-docs", "evidence:infring-notes"],
                        "final_llm_response": {
                            "status": "synthesized",
                            "evidence_refs_used": ["evidence:langgraph-docs", "evidence:infring-notes"]
                        },
                        "stage_statuses": [
                            {"stage": "gate_1_tool_need", "status": "answered_yes"},
                            {"stage": "gate_2_tool_family", "status": "selected_web_research"},
                            {"stage": "gate_3_tool_key", "status": "selected_web_search"},
                            {"stage": "gate_4_request_template", "status": "completed"}
                        ]
                    }
                }
            }]
        }),
    );
    let code = run_research_golden(&runner_args(&root, &cases, &responses, true));
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    assert_eq!(report.get("ok").and_then(Value::as_bool), Some(true));
    assert_eq!(
        report
            .pointer("/summary/research_success_rate")
            .and_then(Value::as_f64),
        Some(1.0)
    );
    assert_eq!(
        report.pointer("/cases/0/gate_transition_diagnostics/first_failed_checkpoint"),
        Some(&Value::Null)
    );
    assert_eq!(
        report.pointer("/cases/0/failure_classification"),
        Some(&Value::String("none".to_string()))
    );
    assert_eq!(
        report.pointer("/measurement_split/deterministic_workflow_path/ok"),
        Some(&Value::Bool(true))
    );
    assert_eq!(
        report.pointer("/cases/0/upstream_failure_localization/earliest_failure_layer"),
        Some(&Value::String("none".to_string()))
    );
    assert_eq!(
        report.pointer("/measurement_split/upstream_failure_localization/top_layer"),
        Some(&Value::String("none".to_string()))
    );
    assert_eq!(
        report.pointer("/measurement_split/end_to_end_golden/mode"),
        Some(&Value::String("recorded_replay".to_string()))
    );
    assert_eq!(
        report.pointer("/category_pass_rates/0/category"),
        Some(&Value::String("comparison".to_string()))
    );
    assert!(report
        .get("tag_pass_rates")
        .and_then(Value::as_array)
        .unwrap()
        .iter()
        .any(|row| row.get("tag").and_then(Value::as_str) == Some("source_sensitive")));
}

#[test]
fn research_golden_separates_gate_progress_from_research_success() {
    let root = temp_path("research_golden_pending");
    let cases = root.join("cases.json");
    let responses = root.join("responses.json");
    write_json_file(&cases, &dataset());
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_gold_test",
                "response_payload": {
                    "response": "",
                    "pending_tool_request": {
                        "status": "pending_confirmation",
                        "tool_name": "web_search",
                        "selected_tool_family": "Web Search / Fetch",
                        "input": {
                            "query": "Infring LangGraph comparison current docs",
                            "aperture": "medium"
                        }
                    },
                    "response_workflow": {
                        "stage_statuses": [
                            {"stage": "gate_1_tool_need", "status": "answered_yes"},
                            {"stage": "gate_2_tool_family", "status": "selected_web_research"},
                            {"stage": "gate_3_tool_key", "status": "selected_web_search"},
                            {"stage": "gate_4_request_template", "status": "completed"}
                        ]
                    }
                }
            }]
        }),
    );
    let code = run_research_golden(&runner_args(&root, &cases, &responses, false));
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    assert_eq!(report.get("ok").and_then(Value::as_bool), Some(false));
    assert_eq!(
        report
            .pointer("/summary/gate_path_ok")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        report
            .pointer("/summary/research_success_rate")
            .and_then(Value::as_f64),
        Some(0.0)
    );
    assert_eq!(
        report
            .pointer("/cases/0/failures")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .any(|row| row.as_str() == Some("empty_research_response")),
        true
    );
    assert_eq!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/first_failed_checkpoint")
            .and_then(Value::as_str),
        Some("5a_tool_execution_recorded")
    );
    assert_eq!(
        report.pointer("/cases/0/failure_classification"),
        Some(&Value::String("hard".to_string()))
    );
    assert_eq!(
        report
            .pointer("/measurement_split/failure_classification/hard_failure_cases")
            .and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        report.pointer("/cases/0/upstream_failure_localization/earliest_failure_layer"),
        Some(&Value::String("workflow_path".to_string()))
    );
    assert_eq!(
        report.pointer("/measurement_split/upstream_failure_localization/top_layer"),
        Some(&Value::String("workflow_path".to_string()))
    );
}

#[test]
fn research_golden_confirm_pending_tool_can_score_second_turn() {
    let root = temp_path("research_golden_confirm_pending");
    let cases = root.join("cases.json");
    let responses = root.join("responses.json");
    write_json_file(&cases, &dataset());
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_gold_test",
                "response_payload": {
                    "response_sequence": [
                        {
                            "response": "",
                            "pending_tool_request": {
                                "status": "pending_confirmation",
                                "tool_name": "web_search",
                                "selected_tool_family": "Web Search / Fetch",
                                "input": {
                                    "query": "Infring LangGraph comparison current docs",
                                    "aperture": "medium"
                                }
                            }
                        },
                        {
                            "response": "Based on source evidence from current docs, Infring and LangGraph fit different research workflows. LangGraph is stronger for teams that want mature graph orchestration, durable state, and production deployment patterns. Infring is more attractive when the tradeoff you care about is editable workflow gates, evidence review, and CD-style rollback discipline. My recommendation is to use LangGraph for a conventional production assistant and keep Infring for experiments where inspecting the workflow path matters most. Caveat: public Infring evidence is limited, so verify Infring-specific claims before committing.",
                            "tools": [{
                                "name": "web_search",
                                "status": "ok",
                                "raw_results": [{
                                    "title": "LangGraph durable graph docs",
                                    "snippet": "LangGraph documents graph orchestration, durable execution, and state-machine patterns for agent workflows."
                                }],
                                "result": "Source evidence from current docs: LangGraph documents graph orchestration and durable state patterns; Infring evidence is limited to workflow CD and gate inspection notes.",
                                "input": {
                                    "query": "Infring LangGraph comparison current docs",
                                    "aperture": "medium"
                                }
                            }],
                            "response_workflow": {
                                "evidence_refs": ["evidence:langgraph-docs", "evidence:infring-notes"],
                                "final_llm_response": {
                                    "status": "synthesized",
                                    "evidence_refs_used": ["evidence:langgraph-docs", "evidence:infring-notes"]
                                }
                            }
                        }
                    ]
                }
            }]
        }),
    );
    let mut args = runner_args(&root, &cases, &responses, true);
    args.push("--confirm-pending-tool=1".to_string());
    let code = run_research_golden(&args);
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    assert_eq!(report.get("ok").and_then(Value::as_bool), Some(true));
    assert_eq!(
        report.pointer("/cases/0/turn_sequence/confirmation_fixture_used"),
        Some(&Value::Bool(true))
    );
    assert_eq!(
        report
            .pointer(
                "/cases/0/turn_sequence/initial_gate_transition_diagnostics/first_failed_checkpoint"
            )
            .and_then(Value::as_str),
        Some("5a_tool_execution_recorded")
    );
    assert_eq!(
        report.pointer("/cases/0/gate_transition_diagnostics/first_failed_checkpoint"),
        Some(&Value::Null)
    );
}

#[test]
fn research_golden_identifies_unpromoted_request_candidate() {
    let root = temp_path("research_golden_unpromoted");
    let cases = root.join("cases.json");
    let responses = root.join("responses.json");
    write_json_file(&cases, &dataset());
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_gold_test",
                "response_payload": {
                    "response": "",
                    "latent_tool_candidates": [{
                        "tool_name": "web_search",
                        "request_payload": {
                            "query": "Infring LangGraph comparison current docs",
                            "aperture": "medium"
                        }
                    }],
                    "response_workflow": {
                        "final_llm_response": {"status": "empty_llm_response"}
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
        Some("4e_pending_request_promoted")
    );
    assert_eq!(
        report
            .pointer("/cases/0/gate_transition_diagnostics/inferred_failure_boundary")
            .and_then(Value::as_str),
        Some("request_candidate_not_promoted")
    );
}

#[test]
fn research_golden_accepts_batch_query_as_web_search_gate_alias() {
    let root = temp_path("research_golden_batch_query_alias");
    let cases = root.join("cases.json");
    let responses = root.join("responses.json");
    write_json_file(&cases, &dataset());
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_gold_test",
                "response_payload": {
                    "response": "Current source-backed evidence suggests Infring and LangGraph optimize for different tradeoffs. LangGraph is stronger for mature graph orchestration and durable state, while Infring is stronger when the team wants editable workflow gates and evidence review. My recommendation is LangGraph for a conventional production app and Infring when workflow inspection is the main differentiator. Caveat: public Infring evidence is still limited.",
                    "pending_tool_request": {
                        "status": "executed",
                        "tool_name": "batch_query",
                        "selected_tool_family": "web_research",
                        "input": {
                            "source": "web",
                            "query": "Compare Infring and LangGraph using current docs and release evidence.",
                            "queries": [
                                "Infring docs workflow gates",
                                "LangGraph docs durable state"
                            ],
                            "keywords": ["Infring", "LangGraph", "workflow gates", "durable state"],
                            "required_coverage": {
                                "entities": ["Infring", "LangGraph"],
                                "facets": ["workflow gates", "durable state"]
                            },
                            "aperture": "medium"
                        }
                    },
                    "tools": [{
                        "name": "batch_query",
                        "status": "ok",
                        "provider_results": [{
                            "title": "LangGraph docs",
                            "snippet": "LangGraph documents durable execution and graph orchestration."
                        }],
                        "result": "Source evidence from current docs: LangGraph documents durable graph orchestration; Infring evidence is centered on workflow gates and evidence review."
                    }],
                    "response_workflow": {
                        "evidence_refs": ["evidence:langgraph-docs", "evidence:infring-docs"],
                        "final_llm_response": {
                            "status": "synthesized",
                            "evidence_refs_used": ["evidence:langgraph-docs", "evidence:infring-docs"]
                        }
                    }
                }
            }]
        }),
    );
    let code = run_research_golden(&runner_args(&root, &cases, &responses, true));
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    assert_eq!(
        report.pointer("/cases/0/gates/gate_3_tool_key"),
        Some(&Value::Bool(true))
    );
    assert_eq!(
        report.pointer("/cases/0/gate_transition_diagnostics/first_failed_checkpoint"),
        Some(&Value::Null)
    );
    assert_eq!(
        report.pointer("/measurement_split/query_metadata_planning/metadata_present_rate"),
        Some(&json!(1.0))
    );
    assert_eq!(
        report.pointer(
            "/measurement_split/query_metadata_planning/rich_query_pack_or_narrow_marker_rate"
        ),
        Some(&json!(1.0))
    );
}

#[test]
fn research_golden_allows_gate_3_for_post_tool_synthesis_without_fresh_candidate() {
    let root = temp_path("research_golden_post_tool_gate_3");
    let cases = root.join("cases.json");
    let responses = root.join("responses.json");
    write_json_file(
        &cases,
        &json!({
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
                "id": "research_gold_post_tool_gate_3",
                "category": "post_tool_synthesis",
                "prompt": "After the web tool returns several source snippets, synthesize the tradeoffs.",
                "expected_gate_path": {
                    "gate_1": "tool_required_or_pending_tool_result",
                    "gate_2": "web_research",
                    "gate_3": "web_search",
                    "gate_4_required_fields": ["query", "aperture"],
                    "post_tool": "must_synthesize_from_evidence_refs"
                },
                "required_entities": []
            }]
        }),
    );
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_gold_post_tool_gate_3",
                "response_payload": {
                    "response": "No source-backed conclusion is available in this replay because the prior web turn is not attached here. What we can say is bounded: the intended workflow is a web research synthesis, the missing evidence should have been multiple source snippets, and the right next step is to replay the post-tool state rather than improvise unsupported claims. Until that evidence is restored, any framework tradeoff summary should be treated as provisional.",
                    "response_workflow": {
                        "final_llm_response": {
                            "status": "synthesized"
                        },
                        "workflow_hint": "post tool web research fetch replay"
                    }
                }
            }]
        }),
    );
    let code = run_research_golden(&runner_args(&root, &cases, &responses, false));
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    assert_eq!(
        report.pointer("/cases/0/gates/gate_2_tool_family"),
        Some(&Value::Bool(true))
    );
    assert_eq!(
        report.pointer("/cases/0/gates/gate_3_tool_key"),
        Some(&Value::Bool(true))
    );
}

#[test]
fn research_golden_sanitizes_backend_key_errors() {
    let root = temp_path("research_golden_backend_error");
    let cases = root.join("cases.json");
    let responses = root.join("responses.json");
    write_json_file(&cases, &dataset());
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_gold_test",
                "response_payload": {
                    "response": "",
                    "provider": "openai",
                    "model": "gpt-5",
                    "runtime_model": "gpt-5",
                    "initial_invoke_error": true,
                    "error": "model backend unavailable: Incorrect API key provided: secret-value. Check provider settings."
                }
            }]
        }),
    );
    let code = run_research_golden(&runner_args(&root, &cases, &responses, false));
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    let error = report
        .pointer("/cases/0/response_diagnostics/error")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(error.contains("[redacted]"), "{error}");
    assert!(!error.contains("secret-value"), "{error}");
}

#[test]
fn research_golden_reports_evidence_outcome_posture_in_diagnostics() {
    let root = temp_path("research_golden_evidence_outcome_posture");
    let cases = root.join("cases.json");
    let responses = root.join("responses.json");
    write_json_file(&cases, &dataset());
    write_json_file(
        &responses,
        &json!({
            "responses": [{
                "case_id": "research_gold_test",
                "response_payload": {
                    "response": "Based on the retrieved evidence, LangGraph is better documented, while Infring remains partially covered.",
                    "provider": "openai",
                    "model": "gpt-5",
                    "runtime_model": "gpt-5",
                    "tools": [{
                        "name": "web_search",
                        "status": "ok",
                        "result": "LangGraph docs are clear; Infring coverage is sparse."
                    }],
                    "response_workflow": {
                        "final_llm_response": {
                            "status": "synthesized",
                            "evidence_outcome_posture": "bounded_partial_answer"
                        }
                    }
                }
            }]
        }),
    );
    let code = run_research_golden(&runner_args(&root, &cases, &responses, false));
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    let posture = report
        .pointer("/cases/0/response_diagnostics/evidence_outcome_posture")
        .and_then(Value::as_str);
    assert_eq!(posture, Some("bounded_partial_answer"));
}

#[test]
fn research_golden_reports_transport_timeout_outside_gate_denominators() {
    let root = temp_path("research_golden_transport_timeout");
    let cases = root.join("cases.json");
    let responses = root.join("responses.json");
    let mut dataset = dataset();
    let first_case = dataset
        .get("cases")
        .and_then(Value::as_array)
        .and_then(|cases| cases.first())
        .cloned()
        .expect("case");
    let mut timeout_case = first_case.clone();
    timeout_case["id"] = json!("research_gold_timeout");
    let mut socket_case = first_case.clone();
    socket_case["id"] = json!("research_gold_socket_hangup");
    dataset["cases"] = json!([first_case, timeout_case, socket_case]);
    write_json_file(&cases, &dataset);
    write_json_file(
        &responses,
        &json!({
            "responses": [
                {
                    "case_id": "research_gold_test",
                    "response_payload": {
                        "response": "Source-backed comparison: Infring and LangGraph both target agent workflows, but the evidence says LangGraph emphasizes durable graph orchestration while Infring emphasizes workflow CDs and explicit gates. Recommendation: use LangGraph for conventional Python graph agents, and use Infring when workflow inspection, evidence refs, and rollback discipline matter. Caveat: Infring evidence remains sparse, so verify Infring-specific production claims before committing.",
                        "pending_tool_request": {
                            "status": "pending_confirmation",
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
                            "raw_results": [{"title": "LangGraph docs", "snippet": "LangGraph documents durable graph orchestration for agent workflows."}],
                            "result": "LangGraph docs describe durable graph orchestration. Infring notes describe workflow CDs and gate inspection."
                        }],
                        "response_workflow": {
                            "evidence_refs": ["evidence:langgraph-docs", "evidence:infring-notes"],
                            "final_llm_response": {
                                "status": "synthesized",
                                "evidence_refs_used": ["evidence:langgraph-docs", "evidence:infring-notes"]
                            },
                            "stage_statuses": [
                                {"stage": "gate_1_tool_need", "status": "answered_yes"},
                                {"stage": "gate_2_tool_family", "status": "selected_web_research"},
                                {"stage": "gate_3_tool_key", "status": "selected_web_search"},
                                {"stage": "gate_4_request_template", "status": "completed"}
                            ]
                        }
                    }
                },
                {
                    "case_id": "research_gold_timeout",
                    "response_payload": {
                        "ok": false,
                        "transport_error": "curl_failed",
                        "stderr": "curl: (28) Operation timed out after 75005 milliseconds with 0 bytes received",
                        "response": "The live dashboard request timed out before the workflow produced a final answer. This is a transport failure, not a research result.",
                        "response_finalization": {
                            "outcome": "structured_failure+transport_timeout+timeout_recovery_failed",
                            "structured_failure": {
                                "kind": "transport_timeout",
                                "retryable": true
                            }
                        },
                        "response_workflow": {
                            "final_llm_response": {
                                "status": "transport_timeout",
                                "attempted": false,
                                "used": false
                            }
                        }
                    }
                },
                {
                    "case_id": "research_gold_socket_hangup",
                    "response_payload": {
                        "ok": false,
                        "error": "socket hang up",
                        "response": ""
                    }
                }
            ]
        }),
    );
    let code = run_research_golden(&runner_args(&root, &cases, &responses, false));
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    let gate_3 = report
        .get("workflow_gate_pass_rates")
        .and_then(Value::as_array)
        .and_then(|rows| {
            rows.iter()
                .find(|row| row.get("gate").and_then(Value::as_str) == Some("gate_3_tool_key"))
        })
        .expect("gate 3 row");
    assert_eq!(gate_3.get("passed").and_then(Value::as_u64), Some(1));
    assert_eq!(gate_3.get("total").and_then(Value::as_u64), Some(1));
    assert_eq!(
        report
            .pointer("/summary/transport_failures")
            .and_then(Value::as_u64),
        Some(2)
    );
    assert_eq!(
        report
            .pointer("/summary/research_success_rate")
            .and_then(Value::as_f64),
        Some(1.0 / 3.0)
    );
    assert_eq!(
        report
            .pointer("/summary/transport_adjusted_research_success_rate")
            .and_then(Value::as_f64),
        Some(1.0)
    );
    assert_eq!(
        report
            .pointer("/summary/non_transport_cases")
            .and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        report
            .pointer("/cases/1/failure_classification")
            .and_then(Value::as_str),
        Some("transport")
    );
    assert_eq!(
        report
            .pointer("/cases/2/failure_classification")
            .and_then(Value::as_str),
        Some("transport")
    );
    assert_eq!(
        report
            .pointer("/measurement_split/failure_classification/transport_failure_cases")
            .and_then(Value::as_u64),
        Some(2)
    );
    assert_eq!(
        report
            .pointer("/measurement_split/end_to_end_golden/raw_live_research_success_rate")
            .and_then(Value::as_f64),
        Some(1.0 / 3.0)
    );
    assert_eq!(
        report
            .pointer(
                "/measurement_split/end_to_end_golden/transport_adjusted_research_success_rate"
            )
            .and_then(Value::as_f64),
        Some(1.0)
    );
    assert_eq!(
        report
            .pointer("/measurement_split/end_to_end_golden/transport_adjusted_cases")
            .and_then(Value::as_u64),
        Some(1)
    );
}
