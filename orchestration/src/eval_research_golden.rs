use serde_json::{json, Value};
use std::collections::BTreeMap;

use super::eval_research_gate_diagnostics::{
    failure_boundary, gate_transition_diagnostics, gate_transition_rate_rows,
};
use super::eval_research_golden_report::{append_failure_events, markdown_report};
use super::eval_research_golden_scoring::{
    citation_artifact_summary, dimension_average_rows, gate_rate_rows, grade_case,
    response_diagnostics,
};
use super::eval_research_golden_utils::*;
use super::eval_web_retrieval_gate_diagnostics::{
    record_web_retrieval_gate_counts, web_failure_boundary, web_retrieval_gate_diagnostics,
    web_retrieval_gate_metric_rows, web_retrieval_gate_rate_rows, web_retrieval_measurement_report,
    web_tooling_measurement_eligible_case, web_tooling_measurement_exclusion_reason_case,
};
use infring_orchestration_v1::observation_lifecycle::{
    load_policy_or_default, persist_lifecycle_observations, policy_path_string,
    research_golden_observation_events, stable_hash_hex, ObservationLifecyclePaths,
    DEFAULT_ARCHIVE_PATH, DEFAULT_HOT_WINDOW_PATH, DEFAULT_LEDGER_PATH, DEFAULT_POLICY_PATH,
    DEFAULT_SUMMARY_PATH,
};
use std::env;
use std::time::Instant;

const DEFAULT_CASES_PATH: &str = "validation/evals/fixtures/research_golden_dataset_v1.json";
const DEFAULT_OUT_PATH: &str = "core/local/artifacts/research_golden_current.json";
const DEFAULT_OUT_LATEST_PATH: &str = "artifacts/research_golden_latest.json";
const DEFAULT_MARKDOWN_PATH: &str = "local/workspace/reports/RESEARCH_GOLDEN_CURRENT.md";
const DEFAULT_FAILURES_PATH: &str = "local/state/ops/research_golden/failures.jsonl";
const DEFAULT_AGENT_ID: &str = "agent-5bc62b0875a9";

#[path = "eval_research_golden_parts/case_execution.rs"]
mod case_execution;
#[path = "eval_research_golden_parts/localization.rs"]
mod localization;
#[path = "eval_research_golden_parts/measurement_split.rs"]
mod measurement_split;
#[path = "eval_research_golden_parts/query_diagnostics.rs"]
mod query_diagnostics;
#[path = "eval_research_golden_parts/reporting.rs"]
mod reporting;

use case_execution::{
    run_research_golden_cases, ResearchGoldenCaseRunInput, ResearchGoldenRunState,
};
use localization::*;
use measurement_split::*;
use query_diagnostics::*;
use reporting::*;

fn case_selection_requested_sample_size(args: &[String], input: &Value, limit: usize) -> Option<usize> {
    let explicit = parse_u64_flag(args, "sample-size", 0) as usize;
    if explicit > 0 {
        return Some(explicit);
    }
    let auto_when_truncated = bool_at(
        input,
        &["sampling_policy", "auto_sample_when_limit_is_lower_than_pool"],
        false,
    );
    auto_when_truncated
        .then_some(limit)
        .filter(|requested| *requested > 0 && *requested != usize::MAX)
}

fn case_selection_requested_seed(args: &[String]) -> Option<String> {
    parse_flag(args, "sample-seed")
        .map(|raw| clean_text(&raw, 120))
        .filter(|raw| !raw.is_empty())
}

fn case_selection_hash(seed: &str, case_id: &str, ordinal: usize) -> String {
    stable_hash_hex(
        &json!({
            "seed": seed,
            "case_id": case_id,
            "ordinal": ordinal
        })
        .to_string(),
    )
}

fn case_selection_counts(cases: &[Value], key: &str) -> Vec<Value> {
    let mut counts = BTreeMap::<String, u64>::new();
    for case in cases {
        for value in match key {
            "category" => vec![str_at(case, &["category"], "unknown")],
            "tag" => {
                let tags = string_array_at(case, &["tags"]);
                if tags.is_empty() {
                    vec!["untagged".to_string()]
                } else {
                    tags
                }
            }
            _ => Vec::new(),
        } {
            let cleaned = clean_text(&value, 120);
            if cleaned.is_empty() {
                continue;
            }
            *counts.entry(cleaned).or_insert(0) += 1;
        }
    }
    counts
        .into_iter()
        .map(|(name, count)| {
            json!({
                key: name,
                "count": count
            })
        })
        .collect()
}

fn select_research_golden_cases(
    cases: &[Value],
    requested_sample_size: Option<usize>,
    requested_seed: Option<&str>,
    limit: usize,
) -> (Vec<Value>, Value) {
    let pool_size = cases.len();
    let requested_sample_size = requested_sample_size
        .filter(|size| *size > 0)
        .map(|size| size.min(pool_size));
    let requested_seed = requested_seed
        .map(|raw| clean_text(raw, 120))
        .filter(|raw| !raw.is_empty());
    let effective_sample_size = requested_sample_size.unwrap_or(pool_size);
    let selection_applied = effective_sample_size < pool_size || requested_seed.is_some();
    let effective_seed = selection_applied.then(|| {
        requested_seed
            .clone()
            .unwrap_or_else(|| format!("auto:{}", now_iso_like()))
    });
    let selected_cases = if let Some(seed) = effective_seed.as_deref() {
        let mut ranked = cases
            .iter()
            .cloned()
            .enumerate()
            .map(|(ordinal, case)| {
                let case_id = clean_text(
                    &str_at(&case, &["id"], &format!("case_{ordinal:03}")),
                    160,
                );
                (case_selection_hash(seed, &case_id, ordinal), ordinal, case)
            })
            .collect::<Vec<_>>();
        ranked.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
        ranked
            .into_iter()
            .take(effective_sample_size)
            .map(|(_, _, case)| case)
            .collect::<Vec<_>>()
    } else {
        cases.to_vec()
    };
    let selected_case_ids = selected_cases
        .iter()
        .enumerate()
        .map(|(ordinal, case)| {
            clean_text(
                &str_at(case, &["id"], &format!("case_{ordinal:03}")),
                160,
            )
        })
        .collect::<Vec<_>>();
    let planned_execution_count = selected_cases.iter().take(limit).count();
    (
        selected_cases.clone(),
        json!({
            "selection_applied": selection_applied,
            "selection_mode": if selection_applied {
                "deterministic_seeded_sample"
            } else {
                "full_dataset_order"
            },
            "pool_size": pool_size,
            "requested_sample_size": requested_sample_size,
            "effective_sample_size": selected_cases.len(),
            "requested_sample_seed": requested_seed,
            "effective_sample_seed": effective_seed,
            "limit_requested": if limit == usize::MAX { None::<usize> } else { Some(limit) },
            "planned_execution_count": planned_execution_count,
            "selected_case_ids": selected_case_ids,
            "selected_category_counts": case_selection_counts(&selected_cases, "category"),
            "selected_tag_counts": case_selection_counts(&selected_cases, "tag")
        }),
    )
}

pub fn run_research_golden(args: &[String]) -> i32 {
    let strict = parse_bool_flag(args, "strict", false);
    let live = parse_bool_flag(args, "live", false);
    let allow_remote = parse_bool_flag(args, "allow-remote", false);
    let confirm_pending_tool = parse_bool_flag(args, "confirm-pending-tool", false);
    let cases_path = parse_flag(args, "cases").unwrap_or_else(|| DEFAULT_CASES_PATH.to_string());
    let responses_path = parse_flag(args, "responses");
    let out_path = parse_flag(args, "out").unwrap_or_else(|| DEFAULT_OUT_PATH.to_string());
    let out_latest_path =
        parse_flag(args, "out-latest").unwrap_or_else(|| DEFAULT_OUT_LATEST_PATH.to_string());
    let markdown_path =
        parse_flag(args, "out-markdown").unwrap_or_else(|| DEFAULT_MARKDOWN_PATH.to_string());
    let failures_path =
        parse_flag(args, "failures-out").unwrap_or_else(|| DEFAULT_FAILURES_PATH.to_string());
    let observation_lifecycle_enabled = parse_bool_flag(args, "observation-lifecycle", true);
    let observation_policy_path =
        parse_flag(args, "observation-policy").unwrap_or_else(|| DEFAULT_POLICY_PATH.to_string());
    let observation_policy = load_policy_or_default(&observation_policy_path);
    let observation_paths = ObservationLifecyclePaths {
        ledger_path: parse_flag(args, "observation-ledger-out").unwrap_or_else(|| {
            policy_path_string(
                &observation_policy,
                &["paths", "compact_ledger"],
                DEFAULT_LEDGER_PATH,
            )
        }),
        hot_window_path: parse_flag(args, "observation-hot-out").unwrap_or_else(|| {
            policy_path_string(
                &observation_policy,
                &["paths", "hot_ring_buffer"],
                DEFAULT_HOT_WINDOW_PATH,
            )
        }),
        archive_path: parse_flag(args, "observation-archive-out").unwrap_or_else(|| {
            policy_path_string(
                &observation_policy,
                &["paths", "failure_lifecycle_archive"],
                DEFAULT_ARCHIVE_PATH,
            )
        }),
        summary_path: parse_flag(args, "observation-summary-out").unwrap_or_else(|| {
            policy_path_string(
                &observation_policy,
                &["paths", "current_summary"],
                DEFAULT_SUMMARY_PATH,
            )
        }),
    };
    let commit_sha = parse_flag(args, "commit-sha")
        .or_else(|| env::var("INFRING_COMMIT_SHA").ok())
        .map(|raw| clean_text(&raw, 120))
        .filter(|raw| !raw.is_empty())
        .unwrap_or_else(|| "unknown".to_string());
    let requested_agent_id = normalize_agent_id(
        &parse_flag(args, "agent-id").unwrap_or_else(|| DEFAULT_AGENT_ID.to_string()),
    );
    let fresh_agent_per_case = parse_bool_flag(args, "fresh-agent-per-case", live);
    let cleanup_fresh_agents = parse_bool_flag(args, "cleanup-fresh-agents", true);
    let isolate_tool_cache = parse_bool_flag(args, "isolate-tool-cache", live);
    let fresh_agent_model = parse_flag(args, "fresh-agent-model")
        .or_else(|| env::var("INFRING_RESEARCH_GOLDEN_FRESH_MODEL").ok())
        .map(|raw| clean_text(&raw, 240))
        .filter(|raw| !raw.is_empty());
    let base_url =
        parse_flag(args, "base-url").unwrap_or_else(|| "http://127.0.0.1:4173".to_string());
    let timeout_seconds = parse_u64_flag(args, "timeout-seconds", 45).clamp(1, 600);
    let default_timeout_recovery_seconds = if live {
        timeout_seconds.saturating_add(135).clamp(90, 240)
    } else {
        timeout_seconds.saturating_add(15).clamp(15, 90)
    };
    let timeout_recovery_seconds = parse_u64_flag(
        args,
        "timeout-recovery-seconds",
        default_timeout_recovery_seconds,
    )
    .min(300);
    let limit = parse_u64_flag(args, "limit", u64::MAX) as usize;
    let partial_out_path =
        parse_flag(args, "partial-out").unwrap_or_else(|| default_partial_path(&out_path));
    let progress_path =
        parse_flag(args, "progress-out").unwrap_or_else(|| default_progress_path(&out_path));

    let input = read_json(&cases_path);
    let cases = input
        .get("cases")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let requested_sample_size = case_selection_requested_sample_size(args, &input, limit);
    let requested_sample_seed = case_selection_requested_seed(args);
    let (selected_cases, case_selection) = select_research_golden_cases(
        &cases,
        requested_sample_size,
        requested_sample_seed.as_deref(),
        limit,
    );
    let thresholds = input
        .get("reliability_thresholds")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let scoring_contract = input
        .get("scoring_contract")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let min_cases = u64_at(&thresholds, &["min_cases_for_reliability_claim"], 20);
    let workflow_gate_pass_min = f64_at(&thresholds, &["workflow_gate_pass_min"], 0.95);
    let research_success_min = f64_at(&thresholds, &["research_success_min"], 0.85);
    let pass_score = u64_at(&scoring_contract, &["pass_score"], 85);
    let excellent_score = u64_at(&scoring_contract, &["excellent_score"], 95);
    let responses_by_case = responses_path
        .as_deref()
        .map(load_responses_by_case)
        .unwrap_or_default();

    let mut setup_failures = Vec::new();
    let mut agent_id = requested_agent_id.clone();
    let mut live_agent_bootstrap = json!({
        "requested_agent_id": requested_agent_id.clone(),
        "effective_agent_id": requested_agent_id.clone(),
        "bootstrapped": false,
        "reason": if live { "not_checked" } else { "offline_mode" }
    });
    if live && !allow_remote && !is_local_dashboard_url(&base_url) {
        setup_failures.push("remote_dashboard_url_requires_allow_remote".to_string());
    }
    if !live && responses_by_case.is_empty() {
        setup_failures.push("offline_mode_requires_responses_fixture".to_string());
    }
    if live && setup_failures.is_empty() {
        match ensure_live_eval_agent(
            &base_url,
            &requested_agent_id,
            fresh_agent_model.as_deref(),
            timeout_seconds,
        ) {
            Ok((effective_agent_id, bootstrapped, reason)) => {
                agent_id = effective_agent_id.clone();
                live_agent_bootstrap = json!({
                    "requested_agent_id": requested_agent_id.clone(),
                    "effective_agent_id": effective_agent_id,
                    "bootstrapped": bootstrapped,
                    "reason": reason
                });
            }
            Err(reason) => {
                live_agent_bootstrap = json!({
                    "requested_agent_id": requested_agent_id.clone(),
                    "effective_agent_id": requested_agent_id.clone(),
                    "bootstrapped": false,
                    "reason": reason
                });
                setup_failures.push("live_agent_bootstrap_failed".to_string());
            }
        }
    }

    let run_state = run_research_golden_cases(ResearchGoldenCaseRunInput {
        cases: &selected_cases,
        limit,
        live,
        confirm_pending_tool,
        setup_failures: &setup_failures,
        isolate_tool_cache,
        fresh_agent_per_case,
        cleanup_fresh_agents,
        base_url: &base_url,
        agent_id: &agent_id,
        fresh_agent_model: fresh_agent_model.as_deref(),
        timeout_seconds,
        timeout_recovery_seconds,
        responses_by_case: &responses_by_case,
        pass_score,
        excellent_score,
        progress_path: &progress_path,
        partial_out_path: &partial_out_path,
        live_agent_bootstrap: &live_agent_bootstrap,
    });
    let ResearchGoldenRunState {
        rows,
        failure_events,
        gate_pass_counts,
        gate_total_counts,
        transition_pass_counts,
        transition_total_counts,
        web_gate_pass_counts,
        web_gate_total_counts,
        dimension_totals,
        passed_cases,
        excellent_cases,
        total_score,
        empty_responses,
        raw_tool_leaks,
        tool_choice_final_responses,
        unsupported_claims,
        transport_failures,
    } = run_state;
    if live
        && cleanup_fresh_agents
        && live_agent_bootstrap
            .get("bootstrapped")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    {
        let _ = delete_live_agent(&base_url, &agent_id, timeout_seconds);
    }

    let total_cases = rows.len() as u64;
    let avg_score = ratio(total_score, total_cases);
    let research_success_rate = ratio(passed_cases, total_cases);
    let excellent_rate = ratio(excellent_cases, total_cases);
    let non_transport_cases = total_cases.saturating_sub(transport_failures);
    let transport_adjusted_passed_cases = rows
        .iter()
        .filter(|row| {
            bool_at(row, &["pass"], false) && !bool_at(row, &["transport_failure"], false)
        })
        .count() as u64;
    let transport_adjusted_research_success_rate =
        ratio(transport_adjusted_passed_cases, non_transport_cases);
    let gate_rates = gate_rate_rows(
        &gate_total_counts,
        &gate_pass_counts,
        workflow_gate_pass_min,
    );
    let gate_path_ok = gate_rates
        .iter()
        .all(|row| row.get("ok").and_then(Value::as_bool).unwrap_or(false));
    let dimension_averages = dimension_average_rows(&dimension_totals, total_cases);
    let category_pass_rates = category_pass_rate_rows(&rows);
    let tag_pass_rates = tag_pass_rate_rows(&rows);
    let gate_transition_rates =
        gate_transition_rate_rows(&transition_total_counts, &transition_pass_counts);
    let web_tool_gate_rates =
        web_retrieval_gate_rate_rows(&web_gate_total_counts, &web_gate_pass_counts);
    let web_tool_gate_metrics = web_retrieval_gate_metric_rows(&rows, &web_tool_gate_rates);
    let gate_transition_path_ok = gate_transition_rates
        .iter()
        .all(|row| f64_at(row, &["pass_rate"], 0.0) >= workflow_gate_pass_min);
    let safety_ok = empty_responses <= u64_at(&thresholds, &["max_empty_responses"], 0)
        && raw_tool_leaks <= u64_at(&thresholds, &["max_raw_tool_leaks"], 0)
        && tool_choice_final_responses
            <= u64_at(&thresholds, &["max_tool_choice_as_final_response"], 0)
        && unsupported_claims <= u64_at(&thresholds, &["max_unsupported_factual_claims"], 0);
    let enough_cases = total_cases >= min_cases;
    let ok = setup_failures.is_empty()
        && enough_cases
        && gate_path_ok
        && gate_transition_path_ok
        && research_success_rate >= research_success_min
        && safety_ok;
    let web_tooling_diagnostics =
        web_retrieval_measurement_report(&rows, &web_tool_gate_rates, &web_tool_gate_metrics);
    let mut measurement_split = measurement_split_report(
        &rows,
        &gate_rates,
        &gate_transition_rates,
        live,
        workflow_gate_pass_min,
        research_success_rate,
        research_success_min,
        safety_ok,
    );
    if let Some(object) = measurement_split.as_object_mut() {
        object.insert("web_tooling".to_string(), web_tooling_diagnostics.clone());
    }
    let generated_at = now_iso_like();
    let run_id = parse_flag(args, "run-id").unwrap_or_else(|| {
        let seed = json!({
            "generated_at": generated_at.clone(),
            "mode": if live { "live_dashboard" } else { "offline_responses" },
            "cases_path": cases_path.clone(),
            "out_path": out_path.clone(),
            "commit_sha": commit_sha.clone()
        });
        format!("research_golden:{}", stable_hash_hex(&seed.to_string()))
    });
    let mut report = json!({
        "type": "research_golden_eval",
        "schema_version": 1,
        "generated_at": generated_at,
        "run_id": run_id,
        "ok": ok,
        "mode": if live { "live_dashboard" } else { "offline_responses" },
        "live_options": {
            "fresh_agent_per_case": fresh_agent_per_case,
            "cleanup_fresh_agents": cleanup_fresh_agents,
            "fresh_agent_model_set": fresh_agent_model.is_some(),
            "timeout_seconds": timeout_seconds,
            "timeout_recovery_seconds": timeout_recovery_seconds,
            "confirm_pending_tool": confirm_pending_tool,
            "isolate_tool_cache": isolate_tool_cache
        },
        "grader": {
            "kind": "deterministic_seed_research_grader",
            "exact_answer_matching": false,
            "score_scale": "0_to_100",
            "pass_score": pass_score,
            "excellent_score": excellent_score,
            "response_grading_layers": [
                "generic_response_contract",
                "tool_backed_evidence_contract",
                "workflow_specific_rubric"
            ]
        },
        "summary": {
            "cases": total_cases,
            "pool_cases": u64_at(&case_selection, &["pool_size"], 0),
            "selected_cases_before_limit": u64_at(&case_selection, &["effective_sample_size"], 0),
            "planned_execution_count": u64_at(&case_selection, &["planned_execution_count"], 0),
            "min_cases_for_reliability_claim": min_cases,
            "enough_cases_for_reliability_claim": enough_cases,
            "passed_cases": passed_cases,
            "transport_adjusted_passed_cases": transport_adjusted_passed_cases,
            "excellent_cases": excellent_cases,
            "average_score": avg_score,
            "research_success_rate": research_success_rate,
            "raw_live_research_success_rate": research_success_rate,
            "transport_adjusted_research_success_rate": transport_adjusted_research_success_rate,
            "excellent_rate": excellent_rate,
            "research_success_min": research_success_min,
            "workflow_gate_pass_min": workflow_gate_pass_min,
            "gate_path_ok": gate_path_ok,
            "gate_transition_path_ok": gate_transition_path_ok,
            "safety_ok": safety_ok,
            "empty_responses": empty_responses,
            "raw_tool_leaks": raw_tool_leaks,
            "tool_choice_final_responses": tool_choice_final_responses,
            "unsupported_claims": unsupported_claims,
            "transport_failures": transport_failures,
            "non_transport_cases": non_transport_cases,
            "failure_count": failure_events.len()
        },
        "measurement_split": measurement_split,
        "workflow_gate_pass_rates": gate_rates,
        "gate_transition_pass_rates": gate_transition_rates,
        "web_tool_gate_pass_rates": web_tool_gate_rates,
        "web_tool_gate_metrics": web_tool_gate_metrics,
        "web_tooling_diagnostics": web_tooling_diagnostics,
        "dimension_averages": dimension_averages,
        "category_pass_rates": category_pass_rates,
        "tag_pass_rates": tag_pass_rates,
        "case_selection": case_selection,
        "setup_failures": setup_failures,
        "cases": rows,
        "failure_events": failure_events,
        "sources": {
            "cases": cases_path,
            "responses": responses_path,
            "base_url": if live { Some(base_url) } else { None },
            "agent_id": if live { Some(agent_id) } else { None },
            "requested_agent_id": if live { Some(requested_agent_id) } else { None },
            "live_agent_bootstrap": if live { Some(live_agent_bootstrap) } else { None }
        }
    });
    let observation_lifecycle_summary = if observation_lifecycle_enabled {
        let cache_mode = if live && isolate_tool_cache {
            "isolated_tool_cache"
        } else if live {
            "shared_tool_cache"
        } else {
            "recorded_responses"
        };
        let model_ref = fresh_agent_model
            .clone()
            .unwrap_or_else(|| "selected_chat_model_or_fixture".to_string());
        let observation_meta = json!({
            "run_id": report.get("run_id").and_then(Value::as_str).unwrap_or(""),
            "commit_sha": commit_sha.clone(),
            "model_ref": model_ref,
            "cache_mode": cache_mode,
            "artifact_refs": [
                out_path.clone(),
                out_latest_path.clone(),
                markdown_path.clone(),
                failures_path.clone(),
                partial_out_path.clone(),
                progress_path.clone()
            ]
        });
        let observations = research_golden_observation_events(&report, &observation_meta);
        match persist_lifecycle_observations(
            &observation_policy,
            &observation_paths,
            &observations,
            report
                .get("generated_at")
                .and_then(Value::as_str)
                .unwrap_or(""),
        ) {
            Ok(summary) => json!({
                "enabled": true,
                "ok": true,
                "events_recorded": observations.len(),
                "summary": summary
            }),
            Err(err) => json!({
                "enabled": true,
                "ok": false,
                "error": err
            }),
        }
    } else {
        json!({
            "enabled": false,
            "ok": true
        })
    };
    if let Some(object) = report.as_object_mut() {
        object.insert(
            "observation_lifecycle".to_string(),
            observation_lifecycle_summary.clone(),
        );
    }
    let markdown = markdown_report(&report);
    let failure_rows = report
        .get("failure_events")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let writes_ok = write_json(&out_path, &report).is_ok()
        && write_json(&out_latest_path, &report).is_ok()
        && write_json(&partial_out_path, &report).is_ok()
        && write_text(&markdown_path, &markdown).is_ok()
        && append_jsonl(&failures_path, &failure_rows).is_ok()
        && observation_lifecycle_summary
            .get("ok")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    if !writes_ok {
        eprintln!("eval_runtime: failed to write one or more research golden outputs");
        return 2;
    }
    write_research_golden_progress(
        &progress_path,
        json!({
            "event": "run_done",
            "generated_at": now_iso_like(),
            "ok": ok,
            "cases": total_cases,
            "passed_cases": passed_cases,
            "excellent_cases": excellent_cases,
            "transport_failures": transport_failures
        }),
    );
    print_json_line(&report);
    if strict && !ok {
        1
    } else {
        0
    }
}
#[cfg(test)]
#[path = "eval_research_golden_lifecycle_tests.rs"]
mod eval_research_golden_lifecycle_tests;
#[cfg(test)]
#[path = "eval_research_golden_post_tool_tests.rs"]
mod eval_research_golden_post_tool_tests;
#[cfg(test)]
#[path = "eval_research_golden_tests.rs"]
mod eval_research_golden_tests;
