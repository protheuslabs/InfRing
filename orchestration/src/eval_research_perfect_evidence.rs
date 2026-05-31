use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::eval_research_golden_scoring::{grade_case, response_diagnostics};

const DEFAULT_CASES_PATH: &str =
    "validation/evals/fixtures/research_perfect_evidence_dataset_v1.json";
const DEFAULT_OUT_PATH: &str = "core/local/artifacts/research_perfect_evidence_current.json";
const DEFAULT_OUT_LATEST_PATH: &str = "artifacts/research_perfect_evidence_latest.json";
const DEFAULT_MARKDOWN_PATH: &str = "local/workspace/reports/RESEARCH_PERFECT_EVIDENCE_CURRENT.md";
const DEFAULT_TEST_MODE_RESPONSES_PATH: &str =
    "core/local/artifacts/research_perfect_evidence_test_mode_responses.json";
const DEFAULT_HANDOFF_OUT_PATH: &str =
    "core/local/artifacts/research_perfect_evidence_handoff_current.json";
const DEFAULT_HANDOFF_OUT_LATEST_PATH: &str =
    "artifacts/research_perfect_evidence_handoff_latest.json";
const DEFAULT_HANDOFF_MARKDOWN_PATH: &str =
    "local/workspace/reports/RESEARCH_PERFECT_EVIDENCE_HANDOFF_CURRENT.md";
const DEFAULT_BASE_URL: &str = "http://127.0.0.1:4173";
const DEFAULT_OLLAMA_BASE_URL: &str = "http://127.0.0.1:11434";
const DEFAULT_AGENT_ID: &str = "research-perfect-evidence-test-lane";
const DEFAULT_REPLAY_MODEL: &str = "kimi-k2.6:cloud";
const TEST_INPUT_LANE_ID: &str = "research_perfect_evidence_test_input_lane_v1";

#[derive(Debug, Clone)]
struct CaseReadiness {
    id: String,
    prompt: String,
    category: String,
    posture: String,
    ok: bool,
    blockers: Vec<String>,
    evidence_packets: usize,
    ready_packets: usize,
    source_domains: usize,
    source_kinds: usize,
    claim_hints: usize,
    replay_payload_ready: bool,
}

pub fn run_research_perfect_evidence(args: &[String]) -> i32 {
    let mode = parse_flag(args, "mode").unwrap_or_else(|| "readiness".to_string());
    if matches!(
        mode.trim(),
        "test-input-replay" | "replay" | "live-synthesis"
    ) {
        return run_test_input_replay(args);
    }
    if matches!(
        mode.trim(),
        "test-input-regrade" | "regrade" | "regrade-responses"
    ) {
        return run_test_input_regrade(args);
    }
    if matches!(
        mode.trim(),
        "production-handoff-replay" | "handoff-replay" | "handoff"
    ) {
        return run_production_handoff_replay(args);
    }

    let cases_path = parse_flag(args, "cases").unwrap_or_else(|| DEFAULT_CASES_PATH.to_string());
    let out_path = parse_flag(args, "out").unwrap_or_else(|| DEFAULT_OUT_PATH.to_string());
    let out_latest_path =
        parse_flag(args, "out-latest").unwrap_or_else(|| DEFAULT_OUT_LATEST_PATH.to_string());
    let markdown_path =
        parse_flag(args, "out-markdown").unwrap_or_else(|| DEFAULT_MARKDOWN_PATH.to_string());
    let strict = parse_bool_flag(args, "strict", true);

    let dataset = match fs::read_to_string(&cases_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
    {
        Some(value) => value,
        None => {
            eprintln!("research-perfect-evidence: failed to read cases from {cases_path}");
            return 2;
        }
    };

    let cases = dataset
        .get("cases")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let min_cases = dataset
        .get("reliability_thresholds")
        .and_then(|v| v.get("min_cases_for_reliability_claim"))
        .and_then(Value::as_u64)
        .unwrap_or(30) as usize;
    let min_categories = dataset
        .get("reliability_thresholds")
        .and_then(|v| v.get("min_categories_for_reliability_claim"))
        .and_then(Value::as_u64)
        .unwrap_or(8) as usize;

    let mut rows = Vec::new();
    let mut category_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut posture_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut source_domain_set = BTreeSet::new();
    let mut total_packets = 0usize;
    let mut total_ready_packets = 0usize;
    let mut total_claim_hints = 0usize;
    let mut exact_answer_key_cases = 0usize;

    for case in cases {
        let readiness = evaluate_case(case);
        *category_counts
            .entry(readiness.category.clone())
            .or_default() += 1;
        *posture_counts.entry(readiness.posture.clone()).or_default() += 1;
        total_packets += readiness.evidence_packets;
        total_ready_packets += readiness.ready_packets;
        total_claim_hints += readiness.claim_hints;
        if readiness
            .blockers
            .iter()
            .any(|blocker| blocker == "exact_answer_key_present")
        {
            exact_answer_key_cases += 1;
        }
        for item in case
            .get("evidence_pack")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or(&[])
        {
            if let Some(domain) = non_empty_str(item, "source_domain") {
                source_domain_set.insert(domain.to_string());
            }
        }
        rows.push(readiness);
    }

    let passed_cases = rows.iter().filter(|row| row.ok).count();
    let replay_ready_cases = rows.iter().filter(|row| row.replay_payload_ready).count();
    let categories_ready = category_counts.len() >= min_categories;
    let case_volume_ready = cases.len() >= min_cases;
    let all_cases_ready = passed_cases == cases.len() && !cases.is_empty();
    let packet_ready_rate = rate(total_ready_packets, total_packets);
    let case_pass_rate = rate(passed_cases, cases.len());
    let replay_payload_ready_rate = rate(replay_ready_cases, cases.len());
    let ok = all_cases_ready && case_volume_ready && categories_ready;

    let case_rows: Vec<Value> = rows
        .iter()
        .map(|row| {
            json!({
                "id": row.id,
                "category": row.category,
                "posture": row.posture,
                "ok": row.ok,
                "blockers": row.blockers,
                "evidence_packets": row.evidence_packets,
                "ready_packets": row.ready_packets,
                "source_domains": row.source_domains,
                "source_kinds": row.source_kinds,
                "claim_hints": row.claim_hints,
                "replay_payload_ready": row.replay_payload_ready,
                "prompt": row.prompt,
            })
        })
        .collect();

    let replay_payload_examples: Vec<Value> =
        cases.iter().take(3).map(build_replay_payload).collect();

    let report = json!({
        "type": "research_perfect_evidence_readiness",
        "schema_version": 1,
        "generated_at": now_iso_like(),
        "ok": ok,
        "summary": {
            "cases_total": cases.len(),
            "min_cases_for_reliability_claim": min_cases,
            "case_volume_ready": case_volume_ready,
            "categories_total": category_counts.len(),
            "min_categories_for_reliability_claim": min_categories,
            "categories_ready": categories_ready,
            "passed_cases": passed_cases,
            "case_pass_rate": case_pass_rate,
            "replay_ready_cases": replay_ready_cases,
            "replay_payload_ready_rate": replay_payload_ready_rate,
            "evidence_packets_total": total_packets,
            "ready_evidence_packets": total_ready_packets,
            "evidence_packet_ready_rate": packet_ready_rate,
            "claim_hints_total": total_claim_hints,
            "source_domains_total": source_domain_set.len(),
            "exact_answer_key_cases": exact_answer_key_cases,
            "category_counts": category_counts,
            "posture_counts": posture_counts,
        },
        "dataset": {
            "path": cases_path,
            "dataset_id": str_at(&dataset, "dataset_id"),
            "answer_key_policy": dataset.get("answer_key_policy").cloned().unwrap_or_else(|| json!({})),
        },
        "cases": case_rows,
        "replay_payload_examples": replay_payload_examples,
    });
    let markdown = render_markdown(&report, &rows);

    let write_ok = write_json(&out_path, &report).is_ok()
        && write_json(&out_latest_path, &report).is_ok()
        && write_text(&markdown_path, &markdown).is_ok();
    if !write_ok {
        eprintln!("research-perfect-evidence: failed to write one or more outputs");
        return 2;
    }

    print_structured(&report);
    if strict && !ok {
        1
    } else {
        0
    }
}

fn run_test_input_replay(args: &[String]) -> i32 {
    let cases_path = parse_flag(args, "cases").unwrap_or_else(|| DEFAULT_CASES_PATH.to_string());
    let out_path = parse_flag(args, "out").unwrap_or_else(|| DEFAULT_OUT_PATH.to_string());
    let out_latest_path =
        parse_flag(args, "out-latest").unwrap_or_else(|| DEFAULT_OUT_LATEST_PATH.to_string());
    let markdown_path =
        parse_flag(args, "out-markdown").unwrap_or_else(|| DEFAULT_MARKDOWN_PATH.to_string());
    let responses_out = parse_flag(args, "responses-out")
        .unwrap_or_else(|| DEFAULT_TEST_MODE_RESPONSES_PATH.to_string());
    let base_url = parse_flag(args, "base-url").unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
    let ollama_base_url =
        parse_flag(args, "ollama-base-url").unwrap_or_else(|| DEFAULT_OLLAMA_BASE_URL.to_string());
    let synthesis_engine =
        parse_flag(args, "synthesis-engine").unwrap_or_else(|| "ollama-direct".to_string());
    let requested_agent_id =
        parse_flag(args, "agent-id").unwrap_or_else(|| DEFAULT_AGENT_ID.to_string());
    let model_ref = parse_flag(args, "model");
    let limit = parse_usize_flag(args, "limit", 5);
    let timeout_seconds = parse_u64_flag(args, "timeout-seconds", 45);
    let fresh_agent_per_case = parse_bool_flag(args, "fresh-agent-per-case", true);
    let cleanup_fresh_agents = parse_bool_flag(args, "cleanup-fresh-agents", true);
    let strict = parse_bool_flag(args, "strict", false);

    let dataset = match fs::read_to_string(&cases_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
    {
        Some(value) => value,
        None => {
            eprintln!("research-perfect-evidence replay: failed to read cases from {cases_path}");
            return 2;
        }
    };
    let cases = dataset
        .get("cases")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut rows = Vec::new();
    let mut response_rows = Vec::new();
    let mut setup_failures = Vec::<String>::new();

    for case in cases.iter().take(limit) {
        let case_id = str_at(case, "id");
        eprintln!("research-perfect-evidence replay: case start {case_id}");
        let readiness = evaluate_case(case);
        let mut case_setup_failures = Vec::<String>::new();
        if !readiness.ok {
            case_setup_failures.push("perfect_evidence_case_not_ready".to_string());
        }
        let mut agent_id = requested_agent_id.clone();
        let mut agent_created = false;
        let use_dashboard_agent = synthesis_engine.trim() == "dashboard-agent";
        if case_setup_failures.is_empty() && fresh_agent_per_case && use_dashboard_agent {
            match create_live_agent(
                &base_url,
                &format!("Research Perfect Evidence {}", clean_text(&case_id, 60)),
                model_ref.as_deref(),
                timeout_seconds,
            ) {
                Some(created_agent_id) => {
                    agent_id = created_agent_id;
                    agent_created = true;
                }
                None => case_setup_failures.push("fresh_test_agent_create_failed".to_string()),
            }
        }

        let prompt = test_input_replay_prompt(case);
        let live_payload = if case_setup_failures.is_empty() && use_dashboard_agent {
            post_json(
                &base_url,
                &format!("/api/agents/{agent_id}/message"),
                &json!({ "message": prompt }),
                timeout_seconds,
            )
        } else if case_setup_failures.is_empty() {
            invoke_direct_ollama_synthesis(
                &ollama_base_url,
                model_ref.as_deref().unwrap_or(DEFAULT_REPLAY_MODEL),
                &prompt,
                timeout_seconds,
            )
        } else {
            json!({})
        };
        if agent_created && cleanup_fresh_agents {
            let _ = delete_live_agent(&base_url, &agent_id, timeout_seconds);
        }
        let transport_failure = live_payload
            .get("ok")
            .and_then(Value::as_bool)
            .map(|ok| !ok)
            .unwrap_or(false)
            || live_payload.get("transport_error").is_some();
        let response_text = assistant_text_from_payload(&live_payload);
        let live_tool_count = payload_tool_count(&live_payload);
        let live_pending_tool = payload_has_pending_tool(&live_payload);
        let production_lane_touched = live_tool_count > 0 || live_pending_tool;
        let lane_isolation_ok =
            case_setup_failures.is_empty() && !transport_failure && !production_lane_touched;
        let scoring_payload = build_test_replay_scoring_payload(
            case,
            &response_text,
            &live_payload,
            lane_isolation_ok,
            live_tool_count,
            live_pending_tool,
        );
        let grade = grade_case(case, &scoring_payload, 85, 95);
        let case_pass = grade.pass && lane_isolation_ok && case_setup_failures.is_empty();
        let case_excellent = grade.excellent && lane_isolation_ok && case_setup_failures.is_empty();
        let response_diagnostics = response_diagnostics(&scoring_payload, &grade.response_text);
        let final_llm_status = response_diagnostics
            .get("final_llm_status")
            .and_then(Value::as_str)
            .unwrap_or_else(|| {
                if grade.response_text.trim().is_empty() {
                    "empty"
                } else {
                    "synthesized"
                }
            });
        let empty_final_response =
            final_llm_status == "empty" || grade.response_text.trim().is_empty();
        let transport_or_harness_failure =
            !case_setup_failures.is_empty() || transport_failure || empty_final_response;
        let measurement_class = if production_lane_touched {
            "test_input_lane_contaminated"
        } else if transport_or_harness_failure {
            "transport_or_harness_failure"
        } else if case_pass {
            "synthesis_pass"
        } else {
            "synthesis_hard_failure"
        };
        let mut failures = grade.failures.clone();
        if !lane_isolation_ok {
            failures.push("test_input_lane_isolation_failed".to_string());
        }
        failures.extend(case_setup_failures.clone());
        failures.sort();
        failures.dedup();
        rows.push(json!({
            "case_id": case_id,
            "category": str_at(case, "category"),
            "prompt": str_at(case, "prompt"),
            "input_lane": TEST_INPUT_LANE_ID,
            "test_mode": true,
            "score": grade.score,
            "pass": case_pass,
            "excellent": case_excellent,
            "failures": failures,
            "setup_failures": case_setup_failures,
            "lane_isolation": {
                "ok": lane_isolation_ok,
                "live_payload_tools_count": live_tool_count,
                "live_payload_pending_tool_request": live_pending_tool,
                "production_web_lane_touched": production_lane_touched
            },
            "measurement_class": measurement_class,
            "synthesized_case": measurement_class == "synthesis_pass" || measurement_class == "synthesis_hard_failure",
            "transport_or_harness_failure": transport_or_harness_failure,
            "response_preview": clean_text(&grade.response_text, 700),
            "response_full": clean_text(&grade.response_text, 12_000),
            "response_diagnostics": response_diagnostics,
            "live_payload_diagnostics": live_payload_diagnostics(&live_payload),
            "soft_quality_smoke": grade.soft_quality_smoke,
            "user_facing_answer_quality": grade.user_facing_answer_quality,
            "answer_unit_evidence_alignment": grade.answer_unit_evidence_alignment,
            "answer_unit_usefulness": grade.answer_unit_usefulness,
            "query_satisfaction": grade.query_satisfaction,
            "citation_behavior": grade.citation_behavior,
            "retrieval_quality": grade.retrieval_quality,
            "live_payload_transport_error": live_payload.get("transport_error").cloned().unwrap_or(Value::Null),
        }));
        response_rows.push(json!({
            "case_id": case_id,
            "response_payload": scoring_payload
        }));
        eprintln!(
            "research-perfect-evidence replay: case done {} pass={} excellent={} score={} lane_ok={}",
            str_at(case, "id"),
            case_pass,
            case_excellent,
            grade.score,
            lane_isolation_ok
        );
    }

    if rows.is_empty() {
        setup_failures.push("no_cases_executed".to_string());
    }
    let summary = replay_summary(&rows, limit, &setup_failures);
    let ok = summary
        .get("transport_or_harness_failures")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        == 0
        && summary
            .get("test_input_lane_leaks")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            == 0
        && summary
            .get("synthesis_hard_failures")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            == 0
        && setup_failures.is_empty();
    let report = json!({
        "type": "research_perfect_evidence_test_input_replay",
        "schema_version": 1,
        "generated_at": now_iso_like(),
        "ok": ok,
        "input_lane": {
            "id": TEST_INPUT_LANE_ID,
            "test_mode": true,
            "live_web_retrieval_allowed": false,
            "production_input_lanes_allowed": false,
            "evidence_source": cases_path,
            "note": "Synthetic perfect evidence is injected only as eval prompt context and reattached only inside the test-mode scoring payload."
        },
        "summary": summary,
        "live_options": {
            "base_url": base_url,
            "ollama_base_url": ollama_base_url,
            "synthesis_engine": synthesis_engine,
            "requested_agent_id": requested_agent_id,
            "fresh_agent_per_case": fresh_agent_per_case,
            "cleanup_fresh_agents": cleanup_fresh_agents,
            "model": model_ref,
            "timeout_seconds": timeout_seconds
        },
        "cases": rows,
        "responses_fixture": responses_out
    });
    let responses = json!({
        "schema_version": 1,
        "dataset_id": "research_perfect_evidence_test_mode_responses_v1",
        "input_lane": TEST_INPUT_LANE_ID,
        "test_mode": true,
        "responses": response_rows
    });
    let markdown = render_replay_markdown(&report);
    let write_ok = write_json(&out_path, &report).is_ok()
        && write_json(&out_latest_path, &report).is_ok()
        && write_json(&responses_out, &responses).is_ok()
        && write_text(&markdown_path, &markdown).is_ok();
    if !write_ok {
        eprintln!("research-perfect-evidence replay: failed to write one or more outputs");
        return 2;
    }
    print_structured(&report);
    if strict && !ok {
        1
    } else {
        0
    }
}

fn run_test_input_regrade(args: &[String]) -> i32 {
    let cases_path = parse_flag(args, "cases").unwrap_or_else(|| DEFAULT_CASES_PATH.to_string());
    let responses_path = parse_flag(args, "responses")
        .unwrap_or_else(|| DEFAULT_TEST_MODE_RESPONSES_PATH.to_string());
    let out_path = parse_flag(args, "out").unwrap_or_else(|| DEFAULT_OUT_PATH.to_string());
    let out_latest_path =
        parse_flag(args, "out-latest").unwrap_or_else(|| DEFAULT_OUT_LATEST_PATH.to_string());
    let markdown_path =
        parse_flag(args, "out-markdown").unwrap_or_else(|| DEFAULT_MARKDOWN_PATH.to_string());
    let limit = parse_usize_flag(args, "limit", 30);
    let strict = parse_bool_flag(args, "strict", false);

    let dataset = match fs::read_to_string(&cases_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
    {
        Some(value) => value,
        None => {
            eprintln!("research-perfect-evidence regrade: failed to read cases from {cases_path}");
            return 2;
        }
    };
    let responses = match fs::read_to_string(&responses_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
    {
        Some(value) => value,
        None => {
            eprintln!(
                "research-perfect-evidence regrade: failed to read responses from {responses_path}"
            );
            return 2;
        }
    };
    let case_map = dataset
        .get("cases")
        .and_then(Value::as_array)
        .map(|cases| {
            cases
                .iter()
                .filter_map(|case| {
                    let id = str_at(case, "id");
                    (!id.is_empty()).then(|| (id, case.clone()))
                })
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();

    let mut rows = Vec::new();
    let mut setup_failures = Vec::<String>::new();
    for response_row in responses
        .get("responses")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
        .iter()
        .take(limit)
    {
        let case_id = str_at(response_row, "case_id");
        let mut case_setup_failures = Vec::<String>::new();
        let case = match case_map.get(&case_id) {
            Some(case) => case,
            None => {
                case_setup_failures.push("regrade_case_not_found".to_string());
                &Value::Null
            }
        };
        let scoring_payload = response_row
            .get("response_payload")
            .cloned()
            .unwrap_or_else(|| json!({}));
        let grade = grade_case(case, &scoring_payload, 85, 95);
        let response_diagnostics = response_diagnostics(&scoring_payload, &grade.response_text);
        let live_tool_count = scoring_payload_live_tool_count(&scoring_payload);
        let live_pending_tool = scoring_payload_live_pending_tool(&scoring_payload);
        let production_lane_touched = live_tool_count > 0 || live_pending_tool;
        let transport_failure = scoring_payload_transport_failure(&scoring_payload);
        let final_llm_status = response_diagnostics
            .get("final_llm_status")
            .and_then(Value::as_str)
            .unwrap_or_else(|| {
                if grade.response_text.trim().is_empty() {
                    "empty"
                } else {
                    "synthesized"
                }
            });
        let empty_final_response =
            final_llm_status == "empty" || grade.response_text.trim().is_empty();
        let lane_isolation_ok = case_setup_failures.is_empty()
            && !transport_failure
            && !empty_final_response
            && !production_lane_touched;
        let case_pass = grade.pass && lane_isolation_ok && case_setup_failures.is_empty();
        let case_excellent = grade.excellent && lane_isolation_ok && case_setup_failures.is_empty();
        let transport_or_harness_failure =
            !case_setup_failures.is_empty() || transport_failure || empty_final_response;
        let measurement_class = if production_lane_touched {
            "test_input_lane_contaminated"
        } else if transport_or_harness_failure {
            "transport_or_harness_failure"
        } else if case_pass {
            "synthesis_pass"
        } else {
            "synthesis_hard_failure"
        };
        let mut failures = grade.failures.clone();
        if !lane_isolation_ok {
            failures.push("test_input_lane_isolation_failed".to_string());
        }
        failures.extend(case_setup_failures.clone());
        failures.sort();
        failures.dedup();
        rows.push(json!({
            "case_id": case_id,
            "category": str_at(case, "category"),
            "prompt": str_at(case, "prompt"),
            "input_lane": TEST_INPUT_LANE_ID,
            "test_mode": true,
            "regrade_mode": true,
            "score": grade.score,
            "pass": case_pass,
            "excellent": case_excellent,
            "failures": failures,
            "setup_failures": case_setup_failures,
            "lane_isolation": {
                "ok": lane_isolation_ok,
                "live_payload_tools_count": live_tool_count,
                "live_payload_pending_tool_request": live_pending_tool,
                "production_web_lane_touched": production_lane_touched
            },
            "measurement_class": measurement_class,
            "synthesized_case": measurement_class == "synthesis_pass" || measurement_class == "synthesis_hard_failure",
            "transport_or_harness_failure": transport_or_harness_failure,
            "response_preview": clean_text(&grade.response_text, 700),
            "response_full": clean_text(&grade.response_text, 12_000),
            "response_diagnostics": response_diagnostics,
            "soft_quality_smoke": grade.soft_quality_smoke,
            "user_facing_answer_quality": grade.user_facing_answer_quality,
            "answer_unit_evidence_alignment": grade.answer_unit_evidence_alignment,
            "answer_unit_usefulness": grade.answer_unit_usefulness,
            "query_satisfaction": grade.query_satisfaction,
            "citation_behavior": grade.citation_behavior,
            "retrieval_quality": grade.retrieval_quality,
            "live_payload_transport_error": scoring_payload
                .pointer("/live_payload/transport_error")
                .cloned()
                .unwrap_or(Value::Null),
        }));
    }

    if rows.is_empty() {
        setup_failures.push("no_responses_regraded".to_string());
    }
    let summary = replay_summary(&rows, limit, &setup_failures);
    let ok = summary
        .get("transport_or_harness_failures")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        == 0
        && summary
            .get("test_input_lane_leaks")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            == 0
        && summary
            .get("synthesis_hard_failures")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            == 0
        && setup_failures.is_empty();
    let report = json!({
        "type": "research_perfect_evidence_test_input_regrade",
        "schema_version": 1,
        "generated_at": now_iso_like(),
        "ok": ok,
        "input_lane": {
            "id": TEST_INPUT_LANE_ID,
            "test_mode": true,
            "regrade_mode": true,
            "live_web_retrieval_allowed": false,
            "production_input_lanes_allowed": false,
            "evidence_source": cases_path,
            "responses_source": responses_path,
            "note": "Offline regrade of saved test-mode response payloads. No LLM call or web/provider call is made."
        },
        "summary": summary,
        "cases": rows
    });
    let markdown = render_replay_markdown(&report);
    let write_ok = write_json(&out_path, &report).is_ok()
        && write_json(&out_latest_path, &report).is_ok()
        && write_text(&markdown_path, &markdown).is_ok();
    if !write_ok {
        eprintln!("research-perfect-evidence regrade: failed to write one or more outputs");
        return 2;
    }
    print_structured(&report);
    if strict && !ok {
        1
    } else {
        0
    }
}

fn run_production_handoff_replay(args: &[String]) -> i32 {
    let cases_path = parse_flag(args, "cases").unwrap_or_else(|| DEFAULT_CASES_PATH.to_string());
    let responses_path = parse_flag(args, "responses")
        .unwrap_or_else(|| DEFAULT_TEST_MODE_RESPONSES_PATH.to_string());
    let out_path = parse_flag(args, "out").unwrap_or_else(|| DEFAULT_HANDOFF_OUT_PATH.to_string());
    let out_latest_path = parse_flag(args, "out-latest")
        .unwrap_or_else(|| DEFAULT_HANDOFF_OUT_LATEST_PATH.to_string());
    let markdown_path = parse_flag(args, "out-markdown")
        .unwrap_or_else(|| DEFAULT_HANDOFF_MARKDOWN_PATH.to_string());
    let limit = parse_usize_flag(args, "limit", 30);
    let strict = parse_bool_flag(args, "strict", false);

    let dataset = match fs::read_to_string(&cases_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
    {
        Some(value) => value,
        None => {
            eprintln!("research-perfect-evidence handoff: failed to read cases from {cases_path}");
            return 2;
        }
    };
    let responses = match fs::read_to_string(&responses_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
    {
        Some(value) => value,
        None => {
            eprintln!(
                "research-perfect-evidence handoff: failed to read responses from {responses_path}"
            );
            return 2;
        }
    };
    let case_map = dataset
        .get("cases")
        .and_then(Value::as_array)
        .map(|cases| {
            cases
                .iter()
                .filter_map(|case| {
                    let id = str_at(case, "id");
                    (!id.is_empty()).then(|| (id, case.clone()))
                })
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();

    let mut rows = Vec::new();
    let mut setup_failures = Vec::<String>::new();
    for response_row in responses
        .get("responses")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
        .iter()
        .take(limit)
    {
        let case_id = str_at(response_row, "case_id");
        let mut case_setup_failures = Vec::<String>::new();
        let case = match case_map.get(&case_id) {
            Some(case) => case,
            None => {
                case_setup_failures.push("handoff_case_not_found".to_string());
                &Value::Null
            }
        };
        let saved_payload = response_row
            .get("response_payload")
            .cloned()
            .unwrap_or_else(|| json!({}));
        let scoring_payload = build_production_handoff_replay_payload(case, &saved_payload);
        let handoff_contract = production_handoff_contract(&scoring_payload);
        let grade = grade_case(case, &scoring_payload, 85, 95);
        let response_diagnostics = response_diagnostics(&scoring_payload, &grade.response_text);
        let contract_ok = handoff_contract
            .get("ok")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let transport_failure = scoring_payload_transport_failure(&scoring_payload);
        let final_llm_status = response_diagnostics
            .get("final_llm_status")
            .and_then(Value::as_str)
            .unwrap_or_else(|| {
                if grade.response_text.trim().is_empty() {
                    "empty"
                } else {
                    "synthesized"
                }
            });
        let empty_final_response =
            final_llm_status == "empty" || grade.response_text.trim().is_empty();
        let transport_or_harness_failure =
            !case_setup_failures.is_empty() || transport_failure || empty_final_response;
        let case_pass = grade.pass
            && contract_ok
            && !transport_or_harness_failure
            && case_setup_failures.is_empty();
        let case_excellent = grade.excellent
            && contract_ok
            && !transport_or_harness_failure
            && case_setup_failures.is_empty();
        let measurement_class = if transport_or_harness_failure {
            "transport_or_harness_failure"
        } else if !contract_ok {
            "handoff_contract_failure"
        } else if case_pass {
            "handoff_synthesis_pass"
        } else {
            "handoff_synthesis_hard_failure"
        };
        let mut failures = grade.failures.clone();
        failures.extend(
            handoff_contract
                .get("failures")
                .and_then(Value::as_array)
                .map(Vec::as_slice)
                .unwrap_or(&[])
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string),
        );
        failures.extend(case_setup_failures.clone());
        failures.sort();
        failures.dedup();
        rows.push(json!({
            "case_id": case_id,
            "category": str_at(case, "category"),
            "prompt": str_at(case, "prompt"),
            "input_lane": TEST_INPUT_LANE_ID,
            "production_handoff_replay": true,
            "score": grade.score,
            "pass": case_pass,
            "excellent": case_excellent,
            "failures": failures,
            "setup_failures": case_setup_failures,
            "handoff_contract": handoff_contract,
            "measurement_class": measurement_class,
            "synthesized_case": measurement_class == "handoff_synthesis_pass" || measurement_class == "handoff_synthesis_hard_failure",
            "transport_or_harness_failure": transport_or_harness_failure,
            "lane_isolation": {
                "ok": true,
                "live_payload_tools_count": 0,
                "live_payload_pending_tool_request": false,
                "production_web_lane_touched": false
            },
            "response_preview": clean_text(&grade.response_text, 700),
            "response_full": clean_text(&grade.response_text, 12_000),
            "response_diagnostics": response_diagnostics,
            "soft_quality_smoke": grade.soft_quality_smoke,
            "user_facing_answer_quality": grade.user_facing_answer_quality,
            "answer_unit_evidence_alignment": grade.answer_unit_evidence_alignment,
            "answer_unit_usefulness": grade.answer_unit_usefulness,
            "query_satisfaction": grade.query_satisfaction,
            "citation_behavior": grade.citation_behavior,
            "retrieval_quality": grade.retrieval_quality,
            "live_payload_transport_error": scoring_payload
                .pointer("/live_payload/transport_error")
                .cloned()
                .unwrap_or(Value::Null),
        }));
    }

    if rows.is_empty() {
        setup_failures.push("no_responses_replayed".to_string());
    }
    let mut summary = replay_summary(&rows, limit, &setup_failures);
    augment_handoff_summary(&mut summary, &rows);
    let ok = summary
        .get("transport_or_harness_failures")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        == 0
        && summary
            .get("handoff_contract_failures")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            == 0
        && summary
            .get("synthesis_hard_failures")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            == 0
        && setup_failures.is_empty();
    let report = json!({
        "type": "research_perfect_evidence_production_handoff_replay",
        "schema_version": 1,
        "generated_at": now_iso_like(),
        "ok": ok,
        "input_lane": {
            "id": TEST_INPUT_LANE_ID,
            "test_mode": true,
            "production_handoff_replay": true,
            "live_web_retrieval_allowed": false,
            "production_input_lanes_allowed": false,
            "evidence_source": cases_path,
            "responses_source": responses_path,
            "note": "Offline replay that projects saved rich-evidence answers into the production finalization package shape. No LLM call or web/provider call is made."
        },
        "summary": summary,
        "cases": rows
    });
    let markdown = render_handoff_markdown(&report);
    let write_ok = write_json(&out_path, &report).is_ok()
        && write_json(&out_latest_path, &report).is_ok()
        && write_text(&markdown_path, &markdown).is_ok();
    if !write_ok {
        eprintln!("research-perfect-evidence handoff: failed to write one or more outputs");
        return 2;
    }
    print_structured(&report);
    if strict && !ok {
        1
    } else {
        0
    }
}

fn evaluate_case(case: &Value) -> CaseReadiness {
    let id = str_at(case, "id");
    let prompt = str_at(case, "prompt");
    let category = str_at(case, "category");
    let posture = str_at(case, "expected_evidence_posture");
    let evidence_pack = case
        .get("evidence_pack")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let mut blockers = Vec::new();
    let mut source_domains = BTreeSet::new();
    let mut source_kinds = BTreeSet::new();
    let mut ready_packets = 0usize;
    let mut claim_hints = 0usize;

    if id.trim().is_empty() {
        blockers.push("missing_case_id".to_string());
    }
    if prompt.trim().len() < 20 {
        blockers.push("prompt_too_thin".to_string());
    }
    if category.trim().is_empty() {
        blockers.push("missing_category".to_string());
    }
    if forbidden_answer_key_present(case) {
        blockers.push("exact_answer_key_present".to_string());
    }

    let required_packet_count = if posture == "insufficient" { 2 } else { 3 };
    if evidence_pack.len() < required_packet_count {
        blockers.push(format!("evidence_pack_lt_{required_packet_count}"));
    }

    for item in evidence_pack {
        if let Some(domain) = non_empty_str(item, "source_domain") {
            source_domains.insert(domain.to_string());
        }
        if let Some(kind) = non_empty_str(item, "source_kind") {
            source_kinds.insert(kind.to_string());
        }
        let hints = item
            .get("claim_hints")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        claim_hints += hints.iter().filter(|hint| text_len(hint) >= 16).count();
        if evidence_item_ready(item) {
            ready_packets += 1;
        }
    }

    if ready_packets < required_packet_count {
        blockers.push(format!("ready_evidence_packets_lt_{required_packet_count}"));
    }
    if posture != "insufficient" && source_domains.len() < 2 {
        blockers.push("source_domain_diversity_lt_2".to_string());
    }
    if source_kinds.len() < 2 {
        blockers.push("source_kind_diversity_lt_2".to_string());
    }
    if posture != "insufficient" && claim_hints < 3 {
        blockers.push("claim_hints_lt_3".to_string());
    }
    let replay_payload_ready = blockers.iter().all(|blocker| {
        blocker != "missing_case_id"
            && blocker != "prompt_too_thin"
            && blocker != "missing_category"
            && blocker != "exact_answer_key_present"
            && !blocker.starts_with("evidence_pack_lt_")
            && !blocker.starts_with("ready_evidence_packets_lt_")
    });

    CaseReadiness {
        id,
        prompt,
        category,
        posture,
        ok: blockers.is_empty(),
        blockers,
        evidence_packets: evidence_pack.len(),
        ready_packets,
        source_domains: source_domains.len(),
        source_kinds: source_kinds.len(),
        claim_hints,
        replay_payload_ready,
    }
}

fn evidence_item_ready(item: &Value) -> bool {
    non_empty_str(item, "id").is_some()
        && non_empty_str(item, "title").is_some()
        && non_empty_str(item, "locator").is_some()
        && non_empty_str(item, "source_domain").is_some()
        && non_empty_str(item, "source_kind").is_some()
        && text_field_len(item, "relevant_extract") >= 120
        && item
            .get("claim_hints")
            .and_then(Value::as_array)
            .map(|hints| hints.iter().any(|hint| text_len(hint) >= 16))
            .unwrap_or(false)
}

fn build_replay_payload(case: &Value) -> Value {
    let case_id = str_at(case, "id");
    let prompt = str_at(case, "prompt");
    let evidence_pack = case
        .get("evidence_pack")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let evidence_refs: Vec<Value> = evidence_pack
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            json!({
                "id": non_empty_str(item, "id").unwrap_or("evidence"),
                "source": non_empty_str(item, "title").unwrap_or("synthetic evidence source"),
                "locator": non_empty_str(item, "locator").unwrap_or("fixture://unknown"),
                "source_domain": non_empty_str(item, "source_domain").unwrap_or("unknown"),
                "rank": idx + 1,
            })
        })
        .collect();
    json!({
        "case_id": case_id,
        "user_prompt": prompt,
        "pending_tool_request": {
            "status": "executed",
            "tool_family": "web_research",
            "tool_name": "batch_query",
            "synthetic_replay": true
        },
        "tool_result_quality": {
            "status": if str_at(case, "expected_evidence_posture") == "insufficient" { "insufficient" } else { "usable" },
            "source": "research_perfect_evidence_dataset_v1"
        },
        "tools": [{
            "name": "batch_query",
            "status": "done",
            "synthetic_replay": true,
            "evidence_refs": evidence_refs,
            "evidence_pack": evidence_pack
        }]
    })
}

fn build_test_replay_scoring_payload(
    case: &Value,
    response_text: &str,
    live_payload: &Value,
    lane_isolation_ok: bool,
    live_tool_count: usize,
    live_pending_tool: bool,
) -> Value {
    let case_id = str_at(case, "id");
    let prompt = str_at(case, "prompt");
    let evidence_pack = evidence_pack_for_case(case);
    let source_refs = source_refs_for_evidence(&evidence_pack);
    let evidence_claims = evidence_claims_for_evidence(&evidence_pack);
    let raw_results = evidence_pack
        .iter()
        .map(|item| {
            json!({
                "title": str_at(item, "title"),
                "locator": str_at(item, "locator"),
                "snippet": str_at(item, "relevant_extract"),
                "source_domain": str_at(item, "source_domain")
            })
        })
        .collect::<Vec<_>>();
    let tool_result_quality = json!({
        "status": if evidence_pack.is_empty() { "no_evidence" } else { "usable" },
        "usable_evidence": !evidence_pack.is_empty(),
        "candidate_count": evidence_pack.len(),
        "evidence_count": evidence_pack.len(),
        "content_rich_candidate_count": evidence_pack.len(),
        "claim_hint_count": evidence_claims.len(),
        "materialized_candidate_count": evidence_pack.len(),
        "coverage": {
            "bucket_status": "covered",
            "missing_buckets": []
        }
    });
    json!({
        "response": response_text,
        "visible_response_source": "llm_final",
        "citations": source_refs,
        "source_refs": source_refs,
        "test_input_lane": {
            "id": TEST_INPUT_LANE_ID,
            "test_mode": true,
            "case_id": case_id,
            "lane_isolation_ok": lane_isolation_ok,
            "live_payload_tools_count": live_tool_count,
            "live_payload_pending_tool_request": live_pending_tool,
            "production_input_lanes_allowed": false,
            "live_web_retrieval_allowed": false
        },
        "pending_tool_request": {
            "status": "executed",
            "tool_name": "batch_query",
            "tool_key": "batch_query",
            "selected_tool_key": "batch_query",
            "selected_tool_family": "web_research",
            "input": {
                "query": prompt,
                "source": "perfect_evidence_test_mode",
                "aperture": "synthetic",
                "query_metadata_policy": {
                    "classification": "synthetic_perfect_evidence_replay"
                }
            },
            "synthetic_replay": true,
            "input_lane": TEST_INPUT_LANE_ID
        },
        "tools": [{
            "name": "batch_query",
            "status": "done",
            "synthetic_replay": true,
            "input_lane": TEST_INPUT_LANE_ID,
            "result": "Synthetic perfect-evidence replay payload supplied by eval test mode.",
            "raw_results": raw_results,
            "evidence_refs": source_refs,
            "evidence_pack": evidence_pack,
            "tool_result_quality": tool_result_quality
        }],
        "evidence_claims": evidence_claims,
        "evidence_pack_quality": {
            "status": "usable",
            "usable_count": evidence_pack.len(),
            "content_rich_item_count": evidence_pack.len()
        },
        "tool_result_quality": tool_result_quality,
        "response_workflow": {
            "input_lane": TEST_INPUT_LANE_ID,
            "test_mode": true,
            "evidence_refs": source_refs,
            "evidence_pack": evidence_pack,
            "citations": source_refs,
            "source_refs": source_refs,
            "final_llm_response": {
                "status": if response_text.trim().is_empty() { "empty" } else { "synthesized" },
                "text": response_text,
                "used": !response_text.trim().is_empty(),
                "source_refs": source_refs,
                "citations": source_refs,
                "visible_response_source": "llm_final",
                "provider": live_payload.get("provider").and_then(Value::as_str),
                "model": live_payload.get("model").and_then(Value::as_str),
                "runtime_model": live_payload.get("runtime_model").and_then(Value::as_str)
            }
        },
        "response_finalization": {
            "outcome": "test_input_lane:synthetic_perfect_evidence+live_llm_synthesis",
            "test_input_lane": TEST_INPUT_LANE_ID,
            "visible_response_source": "llm_final",
            "finalized_output": response_text,
            "final_output": response_text,
            "final_response": {
                "text": response_text,
                "source_refs": source_refs,
                "citations": source_refs
            },
            "final_llm_response": {
                "text": response_text,
                "source_refs": source_refs,
                "citations": source_refs
            },
            "tool_completion": {
                "completion_state": "synthetic_replay",
                "findings_available": !evidence_pack.is_empty(),
                "evidence_refs": source_refs,
                "evidence_pack": evidence_pack,
                "evidence_refs_used": source_refs,
                "tool_attempts": [{
                    "tool": "batch_query",
                    "status": "done",
                    "synthetic": true,
                    "input_lane": TEST_INPUT_LANE_ID
                }]
            }
        },
        "live_payload": {
            "response_preview": clean_text(&assistant_text_from_payload(live_payload), 700),
            "tools_count": live_tool_count,
            "pending_tool_request": live_pending_tool,
            "transport_error": live_payload.get("transport_error").cloned().unwrap_or(Value::Null)
        }
    })
}

fn build_production_handoff_replay_payload(case: &Value, saved_payload: &Value) -> Value {
    let response_text = assistant_text_from_payload(saved_payload);
    let live_payload = json!({
        "response": response_text,
        "provider": saved_payload
            .pointer("/response_workflow/final_llm_response/provider")
            .or_else(|| saved_payload.pointer("/live_payload/provider"))
            .cloned()
            .unwrap_or(Value::Null),
        "model": saved_payload
            .pointer("/response_workflow/final_llm_response/model")
            .or_else(|| saved_payload.pointer("/live_payload/model"))
            .cloned()
            .unwrap_or(Value::Null),
        "runtime_model": saved_payload
            .pointer("/response_workflow/final_llm_response/runtime_model")
            .or_else(|| saved_payload.pointer("/live_payload/runtime_model"))
            .cloned()
            .unwrap_or(Value::Null),
        "response_workflow": {
            "final_llm_response": {
                "status": if response_text.trim().is_empty() { "empty" } else { "synthesized" },
                "used": !response_text.trim().is_empty(),
                "text": response_text
            }
        },
        "response_finalization": {
            "visible_response_source": "llm_final",
            "final_response": {
                "text": response_text
            }
        }
    });
    let mut payload =
        build_test_replay_scoring_payload(case, &response_text, &live_payload, true, 0, false);
    let evidence_pack = evidence_pack_for_case(case);
    let source_refs = source_refs_for_evidence(&evidence_pack);
    let evidence_claims = evidence_claims_for_evidence(&evidence_pack);
    let tool_result_quality = json!({
        "status": if evidence_pack.is_empty() { "no_evidence" } else { "usable" },
        "usable_evidence": !evidence_pack.is_empty(),
        "candidate_count": evidence_pack.len(),
        "evidence_count": evidence_pack.len(),
        "content_rich_candidate_count": evidence_pack.len(),
        "claim_hint_count": evidence_claims.len(),
        "materialized_candidate_count": evidence_pack.len(),
        "coverage": {
            "bucket_status": "covered",
            "missing_buckets": []
        }
    });
    payload["production_handoff_replay"] = json!({
        "source": "saved_perfect_evidence_response_fixture",
        "saved_payload_had_response_finalization_text": text_at_pointer(saved_payload, "/response_finalization/final_response/text").is_some(),
        "saved_payload_had_workflow_final_llm_text": text_at_pointer(saved_payload, "/response_workflow/final_llm_response/text").is_some(),
        "saved_payload_had_visible_response_source": text_at_pointer(saved_payload, "/response_finalization/visible_response_source")
            .or_else(|| text_at_pointer(saved_payload, "/visible_response_source"))
            .is_some()
    });
    payload["response"] = json!(response_text);
    payload["visible_response_source"] = json!("llm_final");
    payload["citations"] = json!(source_refs);
    payload["source_refs"] = json!(source_refs);
    payload["evidence_claims"] = json!(evidence_claims);
    payload["evidence_pack_quality"] = json!({
        "status": if evidence_pack.is_empty() { "no_evidence" } else { "usable" },
        "usable_count": evidence_pack.len(),
        "content_rich_item_count": evidence_pack.len()
    });
    payload["tool_result_quality"] = tool_result_quality.clone();
    payload["tools"] = json!([{
        "name": "batch_query",
        "status": "done",
        "synthetic_replay": true,
        "input_lane": TEST_INPUT_LANE_ID,
        "result": "Synthetic perfect-evidence replay payload supplied by eval test mode.",
        "raw_results": evidence_pack.iter().map(|item| {
            json!({
                "title": str_at(item, "title"),
                "locator": str_at(item, "locator"),
                "snippet": str_at(item, "relevant_extract"),
                "source_domain": str_at(item, "source_domain")
            })
        }).collect::<Vec<_>>(),
        "evidence_refs": source_refs,
        "source_refs": source_refs,
        "citations": source_refs,
        "evidence_pack": evidence_pack,
        "tool_result_quality": tool_result_quality
    }]);
    payload["response_workflow"] = json!({
        "input_lane": TEST_INPUT_LANE_ID,
        "test_mode": true,
        "production_handoff_replay": true,
        "evidence_refs": source_refs,
        "source_refs": source_refs,
        "citations": source_refs,
        "evidence_pack": evidence_pack,
        "evidence_claims": evidence_claims,
        "tool_result_quality": tool_result_quality,
        "final_llm_response": {
            "status": if response_text.trim().is_empty() { "empty" } else { "synthesized" },
            "text": response_text,
            "used": !response_text.trim().is_empty(),
            "visible_response_source": "llm_final",
            "source_refs": source_refs,
            "citations": source_refs
        }
    });
    payload["response_finalization"] = json!({
        "outcome": "production_handoff_replay:synthetic_perfect_evidence+saved_llm_synthesis",
        "test_input_lane": TEST_INPUT_LANE_ID,
        "visible_response_source": "llm_final",
        "finalized_output": response_text,
        "final_output": response_text,
        "final_response": {
            "text": response_text,
            "source_refs": source_refs,
            "citations": source_refs
        },
        "final_llm_response": {
            "text": response_text,
            "source_refs": source_refs,
            "citations": source_refs
        },
        "tool_completion": {
            "completion_state": "synthetic_replay",
            "findings_available": !evidence_pack.is_empty(),
            "evidence_refs": source_refs,
            "source_refs": source_refs,
            "citations": source_refs,
            "evidence_pack": evidence_pack,
            "evidence_claims": evidence_claims,
            "evidence_refs_used": source_refs,
            "tool_attempts": [{
                "tool": "batch_query",
                "status": "done",
                "synthetic": true,
                "input_lane": TEST_INPUT_LANE_ID
            }]
        }
    });
    payload["live_payload"] = json!({
        "response_preview": clean_text(&response_text, 700),
        "tools_count": 0,
        "pending_tool_request": false,
        "transport_error": Value::Null
    });
    payload
}

fn production_handoff_contract(payload: &Value) -> Value {
    let visible_text = assistant_text_from_payload(payload);
    let finalization_text = text_at_pointer(payload, "/response_finalization/final_response/text")
        .or_else(|| text_at_pointer(payload, "/response_finalization/finalized_output"))
        .unwrap_or_default();
    let workflow_final_text =
        text_at_pointer(payload, "/response_workflow/final_llm_response/text").unwrap_or_default();
    let root_response = text_at_pointer(payload, "/response").unwrap_or_default();
    let visible_source = text_at_pointer(payload, "/response_finalization/visible_response_source")
        .or_else(|| text_at_pointer(payload, "/visible_response_source"))
        .or_else(|| {
            text_at_pointer(
                payload,
                "/response_workflow/final_llm_response/visible_response_source",
            )
        })
        .unwrap_or_default();
    let citations_count = max_array_len(
        payload,
        &[
            "/citations",
            "/response_finalization/final_response/citations",
            "/response_finalization/final_llm_response/citations",
            "/response_workflow/citations",
            "/response_workflow/final_llm_response/citations",
        ],
    );
    let source_refs_count = max_array_len(
        payload,
        &[
            "/source_refs",
            "/response_finalization/final_response/source_refs",
            "/response_finalization/final_llm_response/source_refs",
            "/response_workflow/source_refs",
            "/response_workflow/final_llm_response/source_refs",
        ],
    );
    let evidence_pack_count = max_array_len(
        payload,
        &[
            "/tools/0/evidence_pack",
            "/response_workflow/evidence_pack",
            "/response_finalization/tool_completion/evidence_pack",
        ],
    );
    let evidence_claim_count = max_array_len(
        payload,
        &[
            "/evidence_claims",
            "/response_workflow/evidence_claims",
            "/response_finalization/tool_completion/evidence_claims",
        ],
    );
    let mut failures = Vec::<String>::new();
    if visible_text.trim().len() < 80 {
        failures.push("final_visible_text_missing_or_too_thin".to_string());
    }
    if finalization_text.trim().is_empty() {
        failures.push("response_finalization_final_text_missing".to_string());
    }
    if workflow_final_text.trim().is_empty() {
        failures.push("workflow_final_llm_text_missing".to_string());
    }
    if root_response.trim().is_empty() {
        failures.push("root_response_text_missing".to_string());
    }
    if visible_source != "llm_final" {
        failures.push("visible_response_source_not_llm_final".to_string());
    }
    if citations_count == 0 {
        failures.push("citations_not_carried".to_string());
    }
    if source_refs_count == 0 {
        failures.push("source_refs_not_carried".to_string());
    }
    if evidence_pack_count > 0 && evidence_claim_count == 0 {
        failures.push("evidence_claims_not_carried".to_string());
    }
    if looks_like_source_inventory_answer(&visible_text) {
        failures.push("visible_answer_looks_like_source_inventory".to_string());
    }
    json!({
        "ok": failures.is_empty(),
        "failures": failures,
        "final_visible_text_present": visible_text.trim().len() >= 80,
        "response_finalization_final_text_present": !finalization_text.trim().is_empty(),
        "workflow_final_llm_text_present": !workflow_final_text.trim().is_empty(),
        "root_response_text_present": !root_response.trim().is_empty(),
        "visible_response_source": visible_source,
        "visible_response_source_llm_final": visible_source == "llm_final",
        "citations_count": citations_count,
        "source_refs_count": source_refs_count,
        "evidence_pack_count": evidence_pack_count,
        "evidence_claim_count": evidence_claim_count,
        "source_inventory_like": looks_like_source_inventory_answer(&visible_text),
    })
}

fn test_input_replay_prompt(case: &Value) -> String {
    let mut evidence_rows = Vec::<String>::new();
    for (idx, item) in evidence_pack_for_case(case).iter().enumerate() {
        let hints = item
            .get("claim_hints")
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter()
                    .filter_map(Value::as_str)
                    .map(|row| clean_text(row, 240))
                    .filter(|row| !row.is_empty())
                    .collect::<Vec<_>>()
                    .join("; ")
            })
            .unwrap_or_default();
        evidence_rows.push(format!(
            "[{}] {} ({}, {}): {}\nClaim hints: {}",
            idx + 1,
            clean_text(&str_at(item, "title"), 180),
            clean_text(&str_at(item, "source_kind"), 80),
            clean_text(&str_at(item, "source_domain"), 120),
            clean_text(&str_at(item, "relevant_extract"), 900),
            hints
        ));
    }
    format!(
        "TEST MODE: {TEST_INPUT_LANE_ID}\n\
This is an eval-only closed-context answer replay. Do not use web search, browser, batch_query, or any other tool. Treat the reference packets below as the only factual context for the answer. Write only the final user-facing answer, in whatever natural format best answers the original query. Do not mention this test mode, workflow internals, raw tool state, or that reference packets were supplied. Do not add extra concrete examples, dates, numbers, product capabilities, or named entities unless they appear in the reference packets or you clearly mark them as inference.\n\n\
Original user query:\n{}\n\n\
Reference packets:\n{}\n\n\
Answer the original query directly using only the reference packets above. If the reference packets are insufficient for part of the query, say that plainly and give the best bounded answer they support.",
        clean_text(&str_at(case, "prompt"), 1_200),
        evidence_rows.join("\n\n")
    )
}

fn evidence_pack_for_case(case: &Value) -> Vec<Value> {
    case.get("evidence_pack")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn source_refs_for_evidence(evidence_pack: &[Value]) -> Vec<Value> {
    evidence_pack
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            json!({
                "id": str_at(item, "id"),
                "title": str_at(item, "title"),
                "locator": str_at(item, "locator"),
                "source_domain": str_at(item, "source_domain"),
                "source_kind": str_at(item, "source_kind"),
                "rank": idx + 1
            })
        })
        .collect()
}

fn evidence_claims_for_evidence(evidence_pack: &[Value]) -> Vec<Value> {
    let mut claims = Vec::new();
    for item in evidence_pack {
        let source_id = str_at(item, "id");
        for hint in item
            .get("claim_hints")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
        {
            let claim = clean_text(hint.as_str().unwrap_or(""), 500);
            if claim.is_empty() {
                continue;
            }
            claims.push(json!({
                "claim": claim,
                "source_ref": source_id,
                "locator": str_at(item, "locator"),
                "source_domain": str_at(item, "source_domain"),
                "confidence": "usable"
            }));
        }
    }
    claims
}

fn forbidden_answer_key_present(value: &Value) -> bool {
    let forbidden = [
        "expected_answer",
        "ideal_answer",
        "ideal_response",
        "golden_answer",
        "answer_key",
        "expected_final_answer",
    ];
    match value {
        Value::Object(map) => map.iter().any(|(key, child)| {
            forbidden.contains(&key.as_str()) || forbidden_answer_key_present(child)
        }),
        Value::Array(items) => items.iter().any(forbidden_answer_key_present),
        _ => false,
    }
}

fn render_markdown(report: &Value, rows: &[CaseReadiness]) -> String {
    let summary = report.get("summary").unwrap_or(&Value::Null);
    let mut out = String::new();
    out.push_str("# Research Perfect Evidence Readiness\n\n");
    out.push_str(&format!(
        "- ok: {}\n",
        report.get("ok").and_then(Value::as_bool).unwrap_or(false)
    ));
    out.push_str(&format!(
        "- cases: {} / min {}\n",
        summary
            .get("cases_total")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        summary
            .get("min_cases_for_reliability_claim")
            .and_then(Value::as_u64)
            .unwrap_or(0)
    ));
    out.push_str(&format!(
        "- case_pass_rate: {:.3}\n",
        summary
            .get("case_pass_rate")
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
    ));
    out.push_str(&format!(
        "- evidence_packet_ready_rate: {:.3}\n",
        summary
            .get("evidence_packet_ready_rate")
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
    ));
    out.push_str(&format!(
        "- categories: {}\n",
        summary
            .get("categories_total")
            .and_then(Value::as_u64)
            .unwrap_or(0)
    ));
    out.push_str("\n## Blocked Cases\n\n");
    let mut blocked = false;
    for row in rows.iter().filter(|row| !row.ok) {
        blocked = true;
        out.push_str(&format!(
            "- `{}`: {}\n",
            row.id,
            if row.blockers.is_empty() {
                "unknown".to_string()
            } else {
                row.blockers.join(", ")
            }
        ));
    }
    if !blocked {
        out.push_str("- none\n");
    }
    out
}

fn replay_summary(rows: &[Value], limit: usize, setup_failures: &[String]) -> Value {
    let total_cases = rows.len();
    let passed_cases = rows
        .iter()
        .filter(|row| row.get("pass").and_then(Value::as_bool).unwrap_or(false))
        .count();
    let excellent_cases = rows
        .iter()
        .filter(|row| {
            row.get("excellent")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .count();
    let total_score = rows
        .iter()
        .filter_map(|row| row.get("score").and_then(Value::as_u64))
        .sum::<u64>();
    let transport_failures = rows.iter().filter(|row| row_transport_failure(row)).count();
    let empty_final_response_cases = rows
        .iter()
        .filter(|row| row_empty_final_response(row))
        .count();
    let setup_failure_cases = rows
        .iter()
        .filter(|row| {
            row.get("setup_failures")
                .and_then(Value::as_array)
                .map(|items| !items.is_empty())
                .unwrap_or(false)
        })
        .count();
    let transport_or_harness_failures = rows
        .iter()
        .filter(|row| row_transport_or_harness_failure(row))
        .count();
    let lane_leaks = rows.iter().filter(|row| row_lane_contaminated(row)).count();
    let synthesized_rows = rows
        .iter()
        .filter(|row| !row_transport_or_harness_failure(row) && !row_lane_contaminated(row))
        .collect::<Vec<_>>();
    let synthesized_cases = synthesized_rows.len();
    let synthesized_passed_cases = synthesized_rows
        .iter()
        .filter(|row| row.get("pass").and_then(Value::as_bool).unwrap_or(false))
        .count();
    let synthesized_excellent_cases = synthesized_rows
        .iter()
        .filter(|row| {
            row.get("excellent")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .count();
    let synthesized_sounds_good_cases = synthesized_rows
        .iter()
        .filter(|row| {
            row.pointer("/user_facing_answer_quality/verdict")
                .and_then(Value::as_str)
                == Some("sounds_good")
        })
        .count();
    let synthesis_hard_failure_ids = synthesized_rows
        .iter()
        .filter(|row| !row.get("pass").and_then(Value::as_bool).unwrap_or(false))
        .filter_map(|row| row.get("case_id").and_then(Value::as_str))
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let transport_or_harness_failure_ids = rows
        .iter()
        .filter(|row| row_transport_or_harness_failure(row))
        .filter_map(|row| row.get("case_id").and_then(Value::as_str))
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let lane_leak_ids = rows
        .iter()
        .filter(|row| row_lane_contaminated(row))
        .filter_map(|row| row.get("case_id").and_then(Value::as_str))
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    json!({
        "cases_total": total_cases,
        "limit": limit,
        "passed_cases": passed_cases,
        "pass_rate": rate(passed_cases, total_cases),
        "raw_pass_rate_including_transport": rate(passed_cases, total_cases),
        "excellent_cases": excellent_cases,
        "excellent_rate": rate(excellent_cases, total_cases),
        "average_score": rate(total_score as usize, total_cases),
        "transport_failures": transport_failures,
        "empty_final_response_cases": empty_final_response_cases,
        "setup_failure_cases": setup_failure_cases,
        "transport_or_harness_failures": transport_or_harness_failures,
        "transport_or_harness_failure_rate": rate(transport_or_harness_failures, total_cases),
        "transport_or_harness_failure_ids": transport_or_harness_failure_ids,
        "test_input_lane_leaks": lane_leaks,
        "test_input_lane_leak_ids": lane_leak_ids,
        "lane_isolation_rate": rate(total_cases.saturating_sub(lane_leaks), total_cases),
        "synthesized_cases": synthesized_cases,
        "synthesized_passed_cases": synthesized_passed_cases,
        "synthesized_pass_rate": rate(synthesized_passed_cases, synthesized_cases),
        "synthesized_excellent_cases": synthesized_excellent_cases,
        "synthesized_excellent_rate": rate(synthesized_excellent_cases, synthesized_cases),
        "synthesized_sounds_good_cases": synthesized_sounds_good_cases,
        "synthesized_sounds_good_rate": rate(synthesized_sounds_good_cases, synthesized_cases),
        "synthesis_hard_failures": synthesis_hard_failure_ids.len(),
        "synthesis_hard_failure_ids": synthesis_hard_failure_ids,
        "setup_failures": setup_failures,
        "note": "Raw pass/excellent rates include transport and harness failures. Synthesized rates exclude transport, empty final responses, setup failures, and actual test-lane contamination so the good-evidence-to-good-answer lane can be evaluated separately."
    })
}

fn row_transport_or_harness_failure(row: &Value) -> bool {
    row.get("transport_or_harness_failure")
        .and_then(Value::as_bool)
        .unwrap_or_else(|| row_transport_failure(row) || row_empty_final_response(row))
}

fn row_transport_failure(row: &Value) -> bool {
    value_is_present(row.get("live_payload_transport_error"))
        || row
            .pointer("/response_diagnostics/transport_error")
            .and_then(Value::as_str)
            .map(|raw| !raw.trim().is_empty())
            .unwrap_or(false)
}

fn row_empty_final_response(row: &Value) -> bool {
    row.pointer("/response_diagnostics/response_empty")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || row
            .pointer("/response_diagnostics/final_llm_status")
            .and_then(Value::as_str)
            == Some("empty")
}

fn row_lane_contaminated(row: &Value) -> bool {
    row.pointer("/lane_isolation/production_web_lane_touched")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn value_is_present(value: Option<&Value>) -> bool {
    match value {
        Some(Value::Null) | None => false,
        Some(Value::String(raw)) => !raw.trim().is_empty(),
        Some(_) => true,
    }
}

fn scoring_payload_live_tool_count(payload: &Value) -> usize {
    payload
        .pointer("/live_payload/tools_count")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize
}

fn scoring_payload_live_pending_tool(payload: &Value) -> bool {
    payload
        .pointer("/live_payload/pending_tool_request")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn scoring_payload_transport_failure(payload: &Value) -> bool {
    value_is_present(payload.pointer("/live_payload/transport_error"))
        || payload
            .pointer("/response_diagnostics/transport_error")
            .and_then(Value::as_str)
            .map(|raw| !raw.trim().is_empty())
            .unwrap_or(false)
}

fn render_replay_markdown(report: &Value) -> String {
    let summary = report.get("summary").unwrap_or(&Value::Null);
    let mut out = String::new();
    out.push_str("# Research Perfect Evidence Test Input Replay\n\n");
    out.push_str(&format!(
        "- ok: {}\n",
        report.get("ok").and_then(Value::as_bool).unwrap_or(false)
    ));
    out.push_str(&format!(
        "- input_lane: `{}`\n",
        report
            .pointer("/input_lane/id")
            .and_then(Value::as_str)
            .unwrap_or("")
    ));
    out.push_str(&format!(
        "- cases: {}\n",
        summary
            .get("cases_total")
            .and_then(Value::as_u64)
            .unwrap_or(0)
    ));
    out.push_str(&format!(
        "- pass_rate: {:.3}\n",
        summary
            .get("pass_rate")
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
    ));
    out.push_str(&format!(
        "- excellent_rate: {:.3}\n",
        summary
            .get("excellent_rate")
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
    ));
    out.push_str(&format!(
        "- synthesized_pass_rate: {:.3}\n",
        summary
            .get("synthesized_pass_rate")
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
    ));
    out.push_str(&format!(
        "- synthesized_sounds_good_rate: {:.3}\n",
        summary
            .get("synthesized_sounds_good_rate")
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
    ));
    out.push_str(&format!(
        "- transport_or_harness_failures: {}\n",
        summary
            .get("transport_or_harness_failures")
            .and_then(Value::as_u64)
            .unwrap_or(0)
    ));
    out.push_str(&format!(
        "- test_input_lane_leaks: {}\n",
        summary
            .get("test_input_lane_leaks")
            .and_then(Value::as_u64)
            .unwrap_or(0)
    ));
    out.push_str(&format!(
        "- actual_lane_isolation_rate: {:.3}\n",
        summary
            .get("lane_isolation_rate")
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
    ));
    out.push_str("\n## Transport Or Harness Failures\n\n");
    render_case_id_list(
        &mut out,
        summary
            .get("transport_or_harness_failure_ids")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or(&[]),
    );
    out.push_str("\n## Test Input Lane Leaks\n\n");
    render_case_id_list(
        &mut out,
        summary
            .get("test_input_lane_leak_ids")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or(&[]),
    );
    out.push_str("\n## Synthesized Hard Failures\n\n");
    render_case_id_list(
        &mut out,
        summary
            .get("synthesis_hard_failure_ids")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or(&[]),
    );
    out.push_str("\n## Failed Cases\n\n");
    let mut any = false;
    for row in report
        .get("cases")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
        .iter()
        .filter(|row| !row.get("pass").and_then(Value::as_bool).unwrap_or(false))
    {
        any = true;
        let case_id = row.get("case_id").and_then(Value::as_str).unwrap_or("");
        let failures = row
            .get("failures")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default();
        out.push_str(&format!("- `{case_id}`: {failures}\n"));
    }
    if !any {
        out.push_str("- none\n");
    }
    out
}

fn augment_handoff_summary(summary: &mut Value, rows: &[Value]) {
    let handoff_contract_passed = rows
        .iter()
        .filter(|row| {
            row.pointer("/handoff_contract/ok")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .count();
    let handoff_contract_failures = rows.len().saturating_sub(handoff_contract_passed);
    let finalization_text_present = count_bool_pointer(
        rows,
        "/handoff_contract/response_finalization_final_text_present",
    );
    let workflow_final_text_present =
        count_bool_pointer(rows, "/handoff_contract/workflow_final_llm_text_present");
    let visible_source_llm_final =
        count_bool_pointer(rows, "/handoff_contract/visible_response_source_llm_final");
    let citation_package_present = rows
        .iter()
        .filter(|row| {
            row.pointer("/handoff_contract/citations_count")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                > 0
        })
        .count();
    let source_refs_present = rows
        .iter()
        .filter(|row| {
            row.pointer("/handoff_contract/source_refs_count")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                > 0
        })
        .count();
    let evidence_claims_present = rows
        .iter()
        .filter(|row| {
            row.pointer("/handoff_contract/evidence_claim_count")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                > 0
        })
        .count();
    let source_inventory_like = count_bool_pointer(rows, "/handoff_contract/source_inventory_like");
    let handoff_contract_failure_ids = rows
        .iter()
        .filter(|row| {
            !row.pointer("/handoff_contract/ok")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .filter_map(|row| row.get("case_id").and_then(Value::as_str))
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let mut failure_counts = BTreeMap::<String, usize>::new();
    for failure in rows
        .iter()
        .flat_map(|row| {
            row.pointer("/handoff_contract/failures")
                .and_then(Value::as_array)
                .map(Vec::as_slice)
                .unwrap_or(&[])
                .iter()
        })
        .filter_map(Value::as_str)
    {
        *failure_counts.entry(failure.to_string()).or_default() += 1;
    }

    if let Some(map) = summary.as_object_mut() {
        map.insert(
            "handoff_contract_passed_cases".to_string(),
            json!(handoff_contract_passed),
        );
        map.insert(
            "handoff_contract_pass_rate".to_string(),
            json!(rate(handoff_contract_passed, rows.len())),
        );
        map.insert(
            "handoff_contract_failures".to_string(),
            json!(handoff_contract_failures),
        );
        map.insert(
            "handoff_contract_failure_ids".to_string(),
            json!(handoff_contract_failure_ids),
        );
        map.insert(
            "handoff_contract_failure_counts".to_string(),
            json!(failure_counts),
        );
        map.insert(
            "response_finalization_final_text_rate".to_string(),
            json!(rate(finalization_text_present, rows.len())),
        );
        map.insert(
            "workflow_final_llm_text_rate".to_string(),
            json!(rate(workflow_final_text_present, rows.len())),
        );
        map.insert(
            "visible_source_llm_final_rate".to_string(),
            json!(rate(visible_source_llm_final, rows.len())),
        );
        map.insert(
            "citation_package_present_rate".to_string(),
            json!(rate(citation_package_present, rows.len())),
        );
        map.insert(
            "source_refs_present_rate".to_string(),
            json!(rate(source_refs_present, rows.len())),
        );
        map.insert(
            "evidence_claims_present_rate".to_string(),
            json!(rate(evidence_claims_present, rows.len())),
        );
        map.insert(
            "source_inventory_like_cases".to_string(),
            json!(source_inventory_like),
        );
        map.insert(
            "source_inventory_like_rate".to_string(),
            json!(rate(source_inventory_like, rows.len())),
        );
        map.insert(
            "note".to_string(),
            json!("Production handoff replay isolates rich-evidence-to-final-package behavior. It does not call web providers or prove the UI route triggers research."),
        );
    }
}

fn count_bool_pointer(rows: &[Value], pointer: &str) -> usize {
    rows.iter()
        .filter(|row| {
            row.pointer(pointer)
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .count()
}

fn render_handoff_markdown(report: &Value) -> String {
    let summary = report.get("summary").unwrap_or(&Value::Null);
    let mut out = String::new();
    out.push_str("# Research Perfect Evidence Production Handoff Replay\n\n");
    out.push_str(&format!(
        "- ok: {}\n",
        report.get("ok").and_then(Value::as_bool).unwrap_or(false)
    ));
    out.push_str(&format!(
        "- cases: {}\n",
        summary
            .get("cases_total")
            .and_then(Value::as_u64)
            .unwrap_or(0)
    ));
    out.push_str(&format!(
        "- pass_rate: {:.3}\n",
        summary
            .get("pass_rate")
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
    ));
    out.push_str(&format!(
        "- synthesized_sounds_good_rate: {:.3}\n",
        summary
            .get("synthesized_sounds_good_rate")
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
    ));
    out.push_str(&format!(
        "- handoff_contract_pass_rate: {:.3}\n",
        summary
            .get("handoff_contract_pass_rate")
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
    ));
    out.push_str(&format!(
        "- response_finalization_final_text_rate: {:.3}\n",
        summary
            .get("response_finalization_final_text_rate")
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
    ));
    out.push_str(&format!(
        "- workflow_final_llm_text_rate: {:.3}\n",
        summary
            .get("workflow_final_llm_text_rate")
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
    ));
    out.push_str(&format!(
        "- visible_source_llm_final_rate: {:.3}\n",
        summary
            .get("visible_source_llm_final_rate")
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
    ));
    out.push_str(&format!(
        "- citation_package_present_rate: {:.3}\n",
        summary
            .get("citation_package_present_rate")
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
    ));
    out.push_str("\n## Handoff Contract Failures\n\n");
    render_case_id_list(
        &mut out,
        summary
            .get("handoff_contract_failure_ids")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or(&[]),
    );
    out.push_str("\n## Synthesis Hard Failures\n\n");
    render_case_id_list(
        &mut out,
        summary
            .get("synthesis_hard_failure_ids")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or(&[]),
    );
    out
}

fn render_case_id_list(out: &mut String, ids: &[Value]) {
    if ids.is_empty() {
        out.push_str("- none\n");
        return;
    }
    for id in ids.iter().filter_map(Value::as_str) {
        out.push_str(&format!("- `{id}`\n"));
    }
}

fn text_at_pointer(value: &Value, pointer: &str) -> Option<String> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .map(|raw| clean_text(raw, 32_000))
        .filter(|text| !text.trim().is_empty())
}

fn max_array_len(value: &Value, pointers: &[&str]) -> usize {
    pointers
        .iter()
        .filter_map(|pointer| value.pointer(pointer).and_then(Value::as_array))
        .map(Vec::len)
        .max()
        .unwrap_or(0)
}

fn looks_like_source_inventory_answer(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    let sourceish_markers = [
        "here's what i found",
        "search surfaced",
        "recorded evidence",
        "tool trace",
        "web search:",
        "provider starvation",
        "source:",
        "sources:",
    ];
    let marker_hits = sourceish_markers
        .iter()
        .filter(|marker| lower.contains(**marker))
        .count();
    let direct_answer_markers = [
        "because",
        "the main",
        "the best",
        "i'd",
        "recommend",
        "you should",
        "in short",
        "bottom line",
    ];
    let answer_marker_hits = direct_answer_markers
        .iter()
        .filter(|marker| lower.contains(**marker))
        .count();
    marker_hits >= 2 && answer_marker_hits == 0
}

fn assistant_text_from_payload(payload: &Value) -> String {
    for pointer in [
        "/response_finalization/final_response/text",
        "/response_finalization/finalized_output",
        "/response_finalization/final_output",
        "/response_workflow/final_llm_response/text",
        "/response",
        "/text",
        "/message",
        "/content",
        "/assistant/text",
    ] {
        let candidate = payload
            .pointer(pointer)
            .and_then(Value::as_str)
            .map(|raw| clean_text(raw, 12_000))
            .unwrap_or_default();
        if !candidate.is_empty() {
            return candidate;
        }
    }
    String::new()
}

fn payload_tool_count(payload: &Value) -> usize {
    payload
        .get("tools")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0)
        + payload
            .get("response_tools")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0)
}

fn payload_has_pending_tool(payload: &Value) -> bool {
    [
        "/pending_tool_request/status",
        "/response_workflow/pending_tool_request/status",
        "/response_workflow/manual_toolbox_pending_tool_request/status",
        "/response_finalization/pending_tool_request/status",
    ]
    .iter()
    .any(|pointer| payload.pointer(pointer).is_some())
}

fn live_payload_diagnostics(payload: &Value) -> Value {
    let top_keys = payload
        .as_object()
        .map(|object| object.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    json!({
        "top_keys": top_keys,
        "ok": payload.get("ok").and_then(Value::as_bool),
        "response_preview": clean_text(&assistant_text_from_payload(payload), 1_000),
        "provider": payload.get("provider").and_then(Value::as_str),
        "model": payload.get("model").and_then(Value::as_str),
        "runtime_model": payload.get("runtime_model").and_then(Value::as_str),
        "transport_error": payload.get("transport_error").cloned().unwrap_or(Value::Null),
        "response_workflow": {
            "final_llm_response": payload
                .pointer("/response_workflow/final_llm_response")
                .cloned()
                .unwrap_or(Value::Null),
            "quality_telemetry": payload
                .pointer("/response_workflow/quality_telemetry")
                .cloned()
                .unwrap_or(Value::Null),
            "workflow_control": payload
                .pointer("/response_workflow/workflow_control")
                .cloned()
                .unwrap_or(Value::Null),
            "stage_statuses": payload
                .pointer("/response_workflow/stage_statuses")
                .cloned()
                .unwrap_or(Value::Null),
            "tool_gate": payload
                .pointer("/response_workflow/tool_gate")
                .cloned()
                .unwrap_or(Value::Null)
        },
        "response_finalization": {
            "outcome": payload
                .pointer("/response_finalization/outcome")
                .cloned()
                .unwrap_or(Value::Null),
            "visible_response_source": payload
                .pointer("/response_finalization/visible_response_source")
                .cloned()
                .unwrap_or(Value::Null),
            "final_llm_response": payload
                .pointer("/response_finalization/final_llm_response")
                .cloned()
                .unwrap_or(Value::Null),
            "workflow_control": payload
                .pointer("/response_finalization/workflow_control")
                .cloned()
                .unwrap_or(Value::Null)
        }
    })
}

fn invoke_direct_ollama_synthesis(
    ollama_base_url: &str,
    model: &str,
    prompt: &str,
    timeout_seconds: u64,
) -> Value {
    let cleaned_model = clean_text(model, 240);
    let system_prompt = "You write direct, useful final answers from supplied reference material. You do not call tools, expose evaluator state, or describe the prompt machinery.";
    let request = json!({
        "model": cleaned_model,
        "stream": false,
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": prompt}
        ]
    });
    let mut raw = Value::Null;
    let mut attempts = 0_u64;
    for attempt in 0..3_u64 {
        attempts = attempt + 1;
        raw = post_json(ollama_base_url, "/api/chat", &request, timeout_seconds);
        let failed = raw.get("transport_error").is_some()
            || raw
                .get("ok")
                .and_then(Value::as_bool)
                .map(|ok| !ok)
                .unwrap_or(false);
        if !failed {
            break;
        }
        if attempt < 2 {
            sleep(Duration::from_millis(350 * (attempt + 1)));
        }
    }
    if raw.get("transport_error").is_some()
        || raw
            .get("ok")
            .and_then(Value::as_bool)
            .map(|ok| !ok)
            .unwrap_or(false)
    {
        if let Some(map) = raw.as_object_mut() {
            map.insert(
                "direct_ollama_attempts".to_string(),
                Value::Number(attempts.into()),
            );
        }
        return raw;
    }
    let response_text = raw
        .pointer("/message/content")
        .or_else(|| raw.get("response"))
        .and_then(Value::as_str)
        .map(|raw| clean_text(raw, 32_000))
        .unwrap_or_default();
    json!({
        "ok": !response_text.trim().is_empty(),
        "provider": "ollama",
        "model": cleaned_model,
        "runtime_model": cleaned_model,
        "response": response_text,
        "synthesis_engine": "ollama-direct",
        "direct_ollama_attempts": attempts,
        "response_workflow": {
            "final_llm_response": {
                "status": if response_text.trim().is_empty() { "empty" } else { "synthesized" },
                "used": !response_text.trim().is_empty(),
                "attempted": true,
                "provider": "ollama",
                "model": cleaned_model,
                "runtime_model": cleaned_model,
                "text": response_text,
                "source": "test_input_lane_direct_ollama"
            },
            "workflow_control": {
                "mode": TEST_INPUT_LANE_ID,
                "direct_response_path": "test_input_lane_direct_synthesis"
            },
            "quality_telemetry": {
                "final_fallback_used": false
            },
            "stage_statuses": [{
                "stage": "test_input_lane_direct_synthesis",
                "status": if response_text.trim().is_empty() { "empty" } else { "synthesized" }
            }]
        },
        "response_finalization": {
            "outcome": "test_input_lane_direct_ollama_synthesis",
            "visible_response_source": "llm_final",
            "final_response": {
                "text": response_text
            },
            "final_llm_response": {
                "text": response_text
            },
            "workflow_control": {
                "mode": TEST_INPUT_LANE_ID,
                "direct_response_path": "test_input_lane_direct_synthesis"
            }
        },
        "raw_provider_payload": raw
    })
}

fn create_live_agent(
    base_url: &str,
    name: &str,
    model_ref: Option<&str>,
    timeout_seconds: u64,
) -> Option<String> {
    let payload = post_json(
        base_url,
        "/api/agents",
        &json!({
            "name": clean_text(name, 120),
            "role": "analyst"
        }),
        timeout_seconds,
    );
    let agent_id = payload
        .get("agent_id")
        .or_else(|| payload.get("id"))
        .and_then(Value::as_str)
        .map(|raw| clean_text(raw, 160))
        .filter(|raw| !raw.is_empty())?;
    if let Some(model) = model_ref {
        let model = clean_text(model, 160);
        if !model.is_empty() {
            let set_model = post_json_with_method(
                base_url,
                &format!("/api/agents/{agent_id}/model"),
                "PUT",
                &json!({ "model": model }),
                timeout_seconds,
            );
            if set_model
                .get("ok")
                .and_then(Value::as_bool)
                .map(|ok| !ok)
                .unwrap_or(false)
                || set_model.get("transport_error").is_some()
            {
                return None;
            }
        }
    }
    Some(agent_id)
}

fn delete_live_agent(base_url: &str, agent_id: &str, timeout_seconds: u64) -> Value {
    post_json_with_method(
        base_url,
        &format!("/api/agents/{agent_id}"),
        "DELETE",
        &json!({}),
        timeout_seconds,
    )
}

fn post_json(base_url: &str, path: &str, payload: &Value, timeout_seconds: u64) -> Value {
    post_json_with_method(base_url, path, "POST", payload, timeout_seconds)
}

fn post_json_with_method(
    base_url: &str,
    path: &str,
    method: &str,
    payload: &Value,
    timeout_seconds: u64,
) -> Value {
    let body = match serde_json::to_vec(payload) {
        Ok(body) => body,
        Err(err) => return json!({"ok": false, "transport_error": format!("encode_failed:{err}")}),
    };
    let url = format!(
        "{}{}",
        base_url.trim_end_matches('/'),
        if path.starts_with('/') {
            path.to_string()
        } else {
            format!("/{path}")
        }
    );
    let mut child = match Command::new("curl")
        .arg("-sS")
        .arg("--max-time")
        .arg(timeout_seconds.max(1).to_string())
        .arg("-X")
        .arg(method)
        .arg("-H")
        .arg("Content-Type: application/json")
        .arg("--data-binary")
        .arg("@-")
        .arg(url)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(err) => {
            return json!({"ok": false, "transport_error": format!("curl_spawn_failed:{err}")})
        }
    };
    if let Some(stdin) = child.stdin.as_mut() {
        let _ = stdin.write_all(&body);
    }
    match child.wait_with_output() {
        Ok(output) if output.status.success() => serde_json::from_slice::<Value>(&output.stdout)
            .unwrap_or_else(
                |_| json!({"ok": false, "transport_error": "response_json_decode_failed"}),
            ),
        Ok(output) => json!({
            "ok": false,
            "transport_error": "curl_failed",
            "stderr": clean_text(&String::from_utf8_lossy(&output.stderr), 800)
        }),
        Err(err) => json!({"ok": false, "transport_error": format!("curl_wait_failed:{err}")}),
    }
}

fn clean_text(raw: &str, max_len: usize) -> String {
    let compact = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    compact.chars().take(max_len).collect()
}

fn text_len(value: &Value) -> usize {
    value.as_str().map(|text| text.trim().len()).unwrap_or(0)
}

fn text_field_len(value: &Value, key: &str) -> usize {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(|text| text.trim().len())
        .unwrap_or(0)
}

fn non_empty_str<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str).and_then(|text| {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn str_at(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string()
}

fn rate(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn parse_flag(args: &[String], key: &str) -> Option<String> {
    let inline_prefix = format!("--{key}=");
    for (idx, arg) in args.iter().enumerate() {
        if let Some(value) = arg.strip_prefix(&inline_prefix) {
            return Some(value.to_string());
        }
        if arg == &format!("--{key}") {
            if let Some(value) = args.get(idx + 1) {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn parse_bool_flag(args: &[String], key: &str, default: bool) -> bool {
    match parse_flag(args, key).as_deref() {
        Some("1" | "true" | "TRUE" | "yes" | "on") => true,
        Some("0" | "false" | "FALSE" | "no" | "off") => false,
        Some(_) | None => default,
    }
}

fn parse_u64_flag(args: &[String], key: &str, default: u64) -> u64 {
    parse_flag(args, key)
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .unwrap_or(default)
}

fn parse_usize_flag(args: &[String], key: &str, default: usize) -> usize {
    parse_flag(args, key)
        .and_then(|raw| raw.trim().parse::<usize>().ok())
        .unwrap_or(default)
}

fn write_json(path: &str, value: &Value) -> io::Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(value)?)
}

fn write_text(path: &str, content: &str) -> io::Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)
}

fn print_structured(value: &Value) {
    match serde_json::to_string_pretty(value) {
        Ok(text) => println!("{text}"),
        Err(_) => println!("{value:?}"),
    }
}

fn now_iso_like() -> String {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    format!("unix_ms:{ms}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perfect_evidence_case_rejects_exact_answer_keys() {
        let case = json!({
            "id": "case",
            "category": "general_other",
            "prompt": "Research a broad topic with outside evidence.",
            "expected_evidence_posture": "answerable",
            "expected_answer": "do not do this",
            "evidence_pack": []
        });
        let readiness = evaluate_case(&case);
        assert!(readiness
            .blockers
            .contains(&"exact_answer_key_present".to_string()));
    }

    #[test]
    fn insufficient_posture_allows_two_gap_sources() {
        let case = json!({
            "id": "case",
            "category": "general_other",
            "prompt": "Research whether a niche product has reliable public benchmarks.",
            "expected_evidence_posture": "insufficient",
            "evidence_pack": [
                evidence_item("one", "official_docs", "The available page names the product and lists marketing features, but it does not include independent benchmark measurements, test conditions, sample sizes, or reproducible methodology. The only useful answer support is that public evidence is thin and any performance ranking should be treated as unsupported."),
                evidence_item("two", "independent_review", "The independent roundup mentions the product as announced but does not publish a hands-on review, measured performance data, support history, pricing stability, or customer reliability evidence. It supports an honest insufficient-evidence answer rather than a confident recommendation.")
            ]
        });
        let readiness = evaluate_case(&case);
        assert!(readiness.ok, "{:?}", readiness.blockers);
    }

    #[test]
    fn replay_summary_splits_transport_from_synthesis_quality() {
        let rows = vec![
            json!({
                "case_id": "good",
                "score": 92,
                "pass": true,
                "excellent": false,
                "setup_failures": [],
                "transport_or_harness_failure": false,
                "live_payload_transport_error": null,
                "lane_isolation": {"production_web_lane_touched": false},
                "response_diagnostics": {"response_empty": false, "final_llm_status": "synthesized"},
                "user_facing_answer_quality": {"verdict": "sounds_good"}
            }),
            json!({
                "case_id": "curl",
                "score": 0,
                "pass": false,
                "excellent": false,
                "setup_failures": [],
                "transport_or_harness_failure": true,
                "live_payload_transport_error": "curl_failed",
                "lane_isolation": {"production_web_lane_touched": false},
                "response_diagnostics": {"response_empty": true, "final_llm_status": "empty"},
                "user_facing_answer_quality": {"verdict": "sounds_bad"}
            }),
            json!({
                "case_id": "hard",
                "score": 80,
                "pass": false,
                "excellent": false,
                "setup_failures": [],
                "transport_or_harness_failure": false,
                "live_payload_transport_error": null,
                "lane_isolation": {"production_web_lane_touched": false},
                "response_diagnostics": {"response_empty": false, "final_llm_status": "synthesized"},
                "user_facing_answer_quality": {"verdict": "borderline"}
            }),
        ];
        let summary = replay_summary(&rows, 3, &[]);
        assert_eq!(
            summary.get("transport_failures").and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            summary
                .get("transport_or_harness_failures")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            summary.get("synthesized_cases").and_then(Value::as_u64),
            Some(2)
        );
        assert_eq!(
            summary
                .get("synthesized_passed_cases")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            summary
                .get("synthesis_hard_failure_ids")
                .and_then(Value::as_array)
                .and_then(|ids| ids.first())
                .and_then(Value::as_str),
            Some("hard")
        );
    }

    fn evidence_item(id: &str, kind: &str, extract: &str) -> Value {
        json!({
            "id": id,
            "title": format!("Source {id}"),
            "locator": format!("fixture://{id}"),
            "source_domain": format!("{id}.example.test"),
            "source_kind": kind,
            "relevant_extract": extract,
            "claim_hints": ["The answer should stay bounded to what this evidence supports."]
        })
    }
}
