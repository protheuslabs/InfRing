use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

use super::eval_research_golden_scoring::{grade_case, response_diagnostics};
use super::eval_research_golden_utils::{
    assistant_text, bool_at, clean_text, now_iso_like, ratio, read_json, str_at,
};
use super::eval_web_retrieval_gate_diagnostics::{
    record_web_retrieval_gate_counts, web_retrieval_gate_diagnostics,
    web_retrieval_gate_metric_rows, web_retrieval_gate_rate_rows, web_retrieval_measurement_report,
    web_tooling_measurement_exclusion_reason_case,
};

mod direct_tool;
mod report;
mod request_packs;
mod synthetic;

#[cfg(test)]
mod tests;

use direct_tool::{
    direct_tool_payload_diagnostics, direct_tool_payload_sample, invoke_direct_tool,
    is_local_dashboard_url, payload_is_transport_failure,
};
use report::tooling_markdown_report;
use request_packs::{load_request_pack_index, request_pack_for_case};
use synthetic::{
    query_metadata_diagnostics, synthesize_tooling_eval_payload, synthetic_transition_diagnostics,
};

const DEFAULT_CASES_PATH: &str = "validation/evals/fixtures/research_golden_dataset_v1.json";
const DEFAULT_REPORT_REQUEST_PACKS_PATH: &str = "core/local/artifacts/research_golden_current.json";
const DEFAULT_OUT_PATH: &str = "core/local/artifacts/web_tooling_golden_current.json";
const DEFAULT_OUT_LATEST_PATH: &str = "artifacts/web_tooling_golden_latest.json";
const DEFAULT_MARKDOWN_PATH: &str = "local/workspace/reports/WEB_TOOLING_GOLDEN_CURRENT.md";
const DEFAULT_BASE_URL: &str = "http://127.0.0.1:4173";
const DEFAULT_TOOLING_SUCCESS_MIN: f64 = 0.95;
const DEFAULT_WEB_GATE_PASS_MIN: f64 = 0.95;

fn tooling_eval_request_input(tool_name: &str, request_input: &Value) -> Value {
    if tool_name != "batch_query" && tool_name != "batch-query" {
        return request_input.clone();
    }
    let mut request = request_input.clone();
    let Some(map) = request.as_object_mut() else {
        return request;
    };
    if !map.contains_key("cache_mode")
        && !map.contains_key("cache")
        && !map.contains_key("cache_policy")
    {
        map.insert("cache_mode".to_string(), json!("disabled"));
    }
    request
}

pub fn run_web_tooling_golden(args: &[String]) -> i32 {
    let strict = super::parse_bool_flag(args, "strict", false);
    let live = super::parse_bool_flag(args, "live", true);
    let allow_remote = super::parse_bool_flag(args, "allow-remote", false);
    let cases_path =
        super::parse_flag(args, "cases").unwrap_or_else(|| DEFAULT_CASES_PATH.to_string());
    let request_packs_path = super::parse_flag(args, "request-packs-from").or_else(|| {
        std::path::Path::new(DEFAULT_REPORT_REQUEST_PACKS_PATH)
            .exists()
            .then_some(DEFAULT_REPORT_REQUEST_PACKS_PATH.to_string())
    });
    let out_path = super::parse_flag(args, "out").unwrap_or_else(|| DEFAULT_OUT_PATH.to_string());
    let out_latest_path = super::parse_flag(args, "out-latest")
        .unwrap_or_else(|| DEFAULT_OUT_LATEST_PATH.to_string());
    let markdown_path = super::parse_flag(args, "out-markdown")
        .unwrap_or_else(|| DEFAULT_MARKDOWN_PATH.to_string());
    let base_url =
        super::parse_flag(args, "base-url").unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
    let timeout_seconds = super::parse_u64_flag(args, "timeout-seconds", 90).clamp(1, 600);
    let limit = super::parse_u64_flag(args, "limit", u64::MAX) as usize;
    let requested_sample_size = super::parse_u64_flag(args, "sample-size", 0) as usize;
    let requested_sample_seed = super::parse_flag(args, "sample-seed")
        .map(|raw| clean_text(&raw, 120))
        .filter(|raw| !raw.is_empty());
    let default_tool = clean_text(
        &super::parse_flag(args, "tool").unwrap_or_else(|| "batch_query".to_string()),
        80,
    )
    .to_ascii_lowercase();

    let input = read_json(&cases_path);
    let cases = input
        .get("cases")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let thresholds = input
        .get("reliability_thresholds")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let web_gate_pass_min = thresholds
        .get("workflow_gate_pass_min")
        .and_then(Value::as_f64)
        .unwrap_or(DEFAULT_WEB_GATE_PASS_MIN);
    let tooling_success_min = thresholds
        .get("web_tooling_success_min")
        .and_then(Value::as_f64)
        .unwrap_or(DEFAULT_TOOLING_SUCCESS_MIN);

    let mut setup_failures = Vec::<String>::new();
    if live && !allow_remote && !is_local_dashboard_url(&base_url) {
        setup_failures.push("remote_dashboard_url_requires_allow_remote".to_string());
    }
    if !live {
        setup_failures.push("web_tooling_golden_currently_requires_live=1".to_string());
    }

    let request_pack_index = request_packs_path
        .as_deref()
        .map(load_request_pack_index)
        .unwrap_or_default();
    let (selected_cases, case_selection) = select_tooling_cases(
        &cases,
        (requested_sample_size > 0).then_some(requested_sample_size),
        requested_sample_seed.as_deref(),
        limit,
    );

    let mut rows = Vec::<Value>::new();
    let mut web_gate_pass_counts: BTreeMap<String, u64> = BTreeMap::new();
    let mut web_gate_total_counts: BTreeMap<String, u64> = BTreeMap::new();
    let mut transport_failures = 0_u64;
    for case in selected_cases.iter().take(limit) {
        let case_id = str_at(case, &["id"], "unknown_case");
        let prompt = str_at(case, &["prompt"], "");
        let request_pack =
            request_pack_for_case(case, request_pack_index.get(&case_id), &default_tool);
        let tool_name = str_at(&request_pack, &["tool_name"], "batch_query");
        let request_input = request_pack
            .get("input")
            .cloned()
            .unwrap_or_else(|| json!({}));
        let eval_request_input = tooling_eval_request_input(&tool_name, &request_input);
        let request_source = str_at(&request_pack, &["request_pack_source"], "unknown");

        let direct_payload = if live && setup_failures.is_empty() {
            invoke_direct_tool(&base_url, &tool_name, &eval_request_input, timeout_seconds)
        } else {
            json!({
                "ok": false,
                "transport_error": "live_disabled_or_setup_failed",
                "error": "web_tooling_live_execution_skipped"
            })
        };
        let transport_failure = payload_is_transport_failure(&direct_payload);
        if transport_failure {
            transport_failures = transport_failures.saturating_add(1);
        }

        let synthetic_payload =
            synthesize_tooling_eval_payload(&tool_name, &eval_request_input, &direct_payload);
        let grade = grade_case(case, &synthetic_payload, 85, 95);
        let query_metadata_diagnostics = query_metadata_diagnostics(&synthetic_payload);
        let transition_diagnostics =
            synthetic_transition_diagnostics(&synthetic_payload, &grade.retrieval_quality);
        let web_tool_gate_diagnostics = web_retrieval_gate_diagnostics(
            &synthetic_payload,
            &grade.retrieval_quality,
            &query_metadata_diagnostics,
            &transition_diagnostics,
        );
        let mut measurement_exclusion = web_tooling_measurement_exclusion_reason_case(
            case,
            &synthetic_payload,
            &grade.retrieval_quality,
        )
        .unwrap_or("none");
        if transport_failure && measurement_exclusion == "transport_failure" {
            measurement_exclusion = "none";
        }
        let measurement_eligible = measurement_exclusion == "none";
        if measurement_eligible {
            record_web_retrieval_gate_counts(
                &web_tool_gate_diagnostics,
                &mut web_gate_total_counts,
                &mut web_gate_pass_counts,
            );
        }
        let first_failed_gate = web_tool_gate_diagnostics
            .get("first_failed_gate")
            .and_then(Value::as_str)
            .unwrap_or("");
        let tooling_pass = measurement_eligible && first_failed_gate.is_empty();
        rows.push(json!({
            "case_id": case_id,
            "category": str_at(case, &["category"], "unknown"),
            "prompt_preview": clean_text(&prompt, 320),
            "tool_name": tool_name,
            "request_pack_source": request_source,
            "tooling_request": eval_request_input,
            "tooling_pass": tooling_pass,
            "transport_failure": transport_failure,
            "response_preview": clean_text(&assistant_text(&synthetic_payload), 240),
            "response_diagnostics": response_diagnostics(&synthetic_payload, &assistant_text(&synthetic_payload)),
            "retrieval_quality": grade.retrieval_quality,
            "query_metadata_diagnostics": query_metadata_diagnostics,
            "web_tool_gate_diagnostics": web_tool_gate_diagnostics,
            "web_tooling_measurement_exclusion": measurement_exclusion,
            "gate_transition_diagnostics": transition_diagnostics,
            "direct_tool_payload_diagnostics": direct_tool_payload_diagnostics(&direct_payload),
            "direct_tool_payload_sample": direct_tool_payload_sample(&direct_payload)
        }));
    }

    let total_cases = rows.len() as u64;
    let non_transport_cases = total_cases.saturating_sub(transport_failures);
    let passed_cases = rows
        .iter()
        .filter(|row| bool_at(row, &["tooling_pass"], false))
        .count() as u64;
    let success_rate = ratio(passed_cases, total_cases);
    let transport_adjusted_success_rate = ratio(passed_cases, non_transport_cases);
    let web_tool_gate_rates =
        web_retrieval_gate_rate_rows(&web_gate_total_counts, &web_gate_pass_counts);
    let web_tool_gate_metrics = web_retrieval_gate_metric_rows(&rows, &web_tool_gate_rates);
    let web_tooling_diagnostics =
        web_retrieval_measurement_report(&rows, &web_tool_gate_rates, &web_tool_gate_metrics);
    let measured_cases = web_tooling_diagnostics
        .get("measured_cases")
        .and_then(Value::as_u64)
        .unwrap_or(non_transport_cases);
    let measurement_adjusted_success_rate = ratio(passed_cases, measured_cases);
    let ok = setup_failures.is_empty()
        && measurement_adjusted_success_rate >= tooling_success_min
        && web_tool_gate_metrics
            .iter()
            .all(|row| row.get("ok").and_then(Value::as_bool).unwrap_or(false));
    let report = json!({
        "type": "research_web_tooling_eval",
        "schema_version": 1,
        "generated_at": now_iso_like(),
        "ok": ok,
        "mode": if live { "live_direct_tool" } else { "offline" },
        "summary": {
            "cases": total_cases,
            "passed_cases": passed_cases,
            "success_rate": success_rate,
            "transport_adjusted_success_rate": transport_adjusted_success_rate,
            "measurement_adjusted_success_rate": measurement_adjusted_success_rate,
            "transport_failures": transport_failures,
            "non_transport_cases": non_transport_cases,
            "measured_cases": measured_cases,
            "tooling_success_min": tooling_success_min,
            "web_gate_pass_min": web_gate_pass_min
        },
        "measurement_split": {
            "web_tooling": web_tooling_diagnostics
        },
        "web_tool_gate_pass_rates": web_tool_gate_rates,
        "web_tool_gate_metrics": web_tool_gate_metrics,
        "cases": rows,
        "setup_failures": setup_failures,
        "case_selection": case_selection,
        "sources": {
            "cases": cases_path,
            "request_packs_from": request_packs_path,
            "base_url": if live { Some(base_url) } else { None }
        }
    });
    let markdown = tooling_markdown_report(&report);
    let write_ok = super::write_json(&out_path, &report).is_ok()
        && super::write_json(&out_latest_path, &report).is_ok()
        && super::write_text(&markdown_path, &markdown).is_ok();
    if !write_ok {
        eprintln!("eval_runtime: failed to write one or more web tooling golden outputs");
        return 2;
    }
    super::print_structured(&report);
    if strict && !ok {
        return 1;
    }
    0
}

fn select_tooling_cases(
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
            .unwrap_or_else(runtime_random_sample_seed)
    });
    let selected_cases = if let Some(seed) = effective_seed.as_deref() {
        let mut ranked = cases
            .iter()
            .cloned()
            .enumerate()
            .map(|(ordinal, case)| {
                let case_id = clean_text(&str_at(&case, &["id"], ""), 160);
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
        .map(|case| clean_text(&str_at(case, &["id"], ""), 160))
        .collect::<Vec<_>>();
    (
        selected_cases.clone(),
        json!({
            "selection_applied": selection_applied,
            "selection_mode": if selection_applied {
                if requested_seed.is_some() {
                    "deterministic_seeded_sample"
                } else {
                    "runtime_random_recorded_sample"
                }
            } else {
                "full_dataset_order"
            },
            "pool_size": pool_size,
            "requested_sample_size": requested_sample_size,
            "effective_sample_size": selected_cases.len(),
            "requested_sample_seed": requested_seed,
            "effective_sample_seed": effective_seed,
            "limit_requested": limit,
            "planned_execution_count": selected_cases.iter().take(limit).count(),
            "selected_case_ids": selected_case_ids,
            "selected_category_counts": selected_case_counts(&selected_cases, "category"),
            "selected_tag_counts": selected_case_counts(&selected_cases, "tag")
        }),
    )
}

fn runtime_random_sample_seed() -> String {
    let now_nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let raw = format!("web_tooling_golden:{now_nanos}:{}", process::id());
    format!("random:{}", stable_hash_hex(&raw))
}

fn case_selection_hash(seed: &str, case_id: &str, ordinal: usize) -> String {
    stable_hash_hex(&format!("{seed}\n{case_id}\n{ordinal}"))
}

fn stable_hash_hex(raw: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in raw.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn selected_case_counts(cases: &[Value], key: &str) -> Vec<Value> {
    let mut counts = BTreeMap::<String, u64>::new();
    for case in cases {
        let values = match key {
            "category" => vec![str_at(case, &["category"], "")],
            "tag" => case
                .get("tags")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .map(|raw| raw.to_string())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
            _ => Vec::new(),
        };
        for value in values {
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
