// SRS: V12-SYNTHETIC-USER-HARNESS-001
use serde_json::{json, Value};
use std::path::PathBuf;
use std::time::Instant;

#[path = "eval_agent_work_success.rs"]
mod eval_agent_work_success;
#[path = "eval_synthetic_user_chat_harness_utils.rs"]
mod eval_synthetic_user_chat_harness_utils;
use eval_agent_work_success::*;
use eval_synthetic_user_chat_harness_utils::*;

const DEFAULT_CASES_PATH: &str = "validation/evals/fixtures/synthetic_user_chat_harness_cases.json";
const DEFAULT_OUT_PATH: &str = "core/local/artifacts/synthetic_user_chat_harness_current.json";
const DEFAULT_OUT_LATEST_PATH: &str = "artifacts/synthetic_user_chat_harness_latest.json";
const DEFAULT_MARKDOWN_PATH: &str =
    "local/workspace/reports/SYNTHETIC_USER_CHAT_HARNESS_CURRENT.md";
const DEFAULT_FAILURES_PATH: &str = "local/state/ops/synthetic_user_chat_harness/failures.jsonl";
const DEFAULT_ATTENTION_DIR: &str = "local/state/ops/eval_agent_feedback";
const DEFAULT_LIVE_MONITOR_LATEST_PATH: &str = "local/state/ops/eval_live_monitor/latest.json";
const DEFAULT_MISTY_AGENT_ID: &str = "agent-5bc62b0875a9";

pub fn run_synthetic_user_chat_harness(args: &[String]) -> i32 {
    let strict = parse_bool_flag(args, "strict", false);
    let live = parse_bool_flag(args, "live", false);
    let allow_remote = parse_bool_flag(args, "allow-remote", false);
    let cases_path = parse_flag(args, "cases").unwrap_or_else(|| DEFAULT_CASES_PATH.to_string());
    let out_path = parse_flag(args, "out").unwrap_or_else(|| DEFAULT_OUT_PATH.to_string());
    let out_latest_path =
        parse_flag(args, "out-latest").unwrap_or_else(|| DEFAULT_OUT_LATEST_PATH.to_string());
    let markdown_path =
        parse_flag(args, "out-markdown").unwrap_or_else(|| DEFAULT_MARKDOWN_PATH.to_string());
    let failures_path =
        parse_flag(args, "failures-out").unwrap_or_else(|| DEFAULT_FAILURES_PATH.to_string());
    let live_monitor_latest_path = parse_flag(args, "live-monitor-latest")
        .unwrap_or_else(|| DEFAULT_LIVE_MONITOR_LATEST_PATH.to_string());
    let require_live_monitor_freshness =
        parse_bool_flag(args, "require-live-monitor-freshness", live);
    let attention_dir = PathBuf::from(
        parse_flag(args, "attention-dir").unwrap_or_else(|| DEFAULT_ATTENTION_DIR.to_string()),
    );
    let base_url =
        parse_flag(args, "base-url").unwrap_or_else(|| "http://127.0.0.1:4173".to_string());
    let timeout_seconds = parse_u64_flag(args, "timeout-seconds", 45).clamp(1, 600);

    let input = read_json(&cases_path);
    let cases = input
        .get("cases")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let thresholds = input
        .get("thresholds")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let min_cases = u64_at(&thresholds, &["min_cases"], 1);
    let min_pass_rate = f64_at(&thresholds, &["min_pass_rate"], 1.0);
    let max_failures = u64_at(&thresholds, &["max_failures"], 0);

    let mut rows = Vec::new();
    let mut route_stage_deltas = Vec::new();
    let mut failure_events = Vec::new();
    let mut passed_turns = 0_u64;
    let mut total_turns = 0_u64;
    let mut previous_response = String::new();
    let mut setup_failures = Vec::new();
    let initial_live_monitor_marker = live_monitor_marker(&live_monitor_latest_path);
    let mut observed_live_monitor_markers = Vec::new();
    if live && !allow_remote && !is_local_dashboard_url(&base_url) {
        setup_failures.push("remote_dashboard_url_requires_allow_remote".to_string());
    }

    for case in cases.iter() {
        let case_id = str_at(case, &["id"], "unknown_case");
        let agent_id = normalize_agent_id(
            parse_flag(args, "agent-id")
                .as_deref()
                .or_else(|| str_opt(case, &["agent_id"]))
                .or_else(|| str_opt(&input, &["defaults", "agent_id"]))
                .unwrap_or("synthetic-harness-agent"),
        );
        let turns = case
            .get("turns")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for (idx, turn) in turns.iter().enumerate() {
            total_turns = total_turns.saturating_add(1);
            let turn_id = str_at(turn, &["turn_id"], &format!("turn_{idx}"));
            let user_message = str_opt(turn, &["user_message"])
                .or_else(|| str_opt(turn, &["synthetic_user", "message"]))
                .unwrap_or("")
                .trim()
                .to_string();
            let request = json!({ "message": user_message });
            let turn_start = Instant::now();
            let response_payload = if live && setup_failures.is_empty() {
                post_agent_message(&base_url, &agent_id, &request, timeout_seconds)
            } else {
                turn.get("mock_response")
                    .cloned()
                    .unwrap_or_else(|| json!({}))
            };
            if live {
                if let Some(marker) = live_monitor_marker_from_payload(&response_payload) {
                    observed_live_monitor_markers.push(marker);
                }
            }
            let elapsed_ms = turn_start.elapsed().as_millis();
            let latency_ms = if elapsed_ms > u64::MAX as u128 {
                u64::MAX
            } else {
                elapsed_ms as u64
            };
            let response_text = assistant_text(&response_payload);
            let response_token_count = estimate_response_tokens(&response_text);
            let workflow_stage_count = workflow_stage_count(&response_payload);
            let workflow_budget_stage_count =
                workflow_budget_stage_count(&response_payload, workflow_stage_count);
            let route_error_code = route_error_code(&response_payload);
            let failures = evaluate_turn(TurnEvaluation {
                live,
                turn,
                thresholds: &thresholds,
                user_message: &user_message,
                response_text: &response_text,
                previous_response: &previous_response,
                payload: &response_payload,
                route_error_code: route_error_code.as_deref(),
                latency_ms,
                response_token_count,
                workflow_stage_count,
            });
            let route_stage_delta = route_stage_delta(
                turn,
                &response_payload,
                route_error_code.as_deref(),
                &response_text,
                workflow_stage_count,
                &failures,
            );
            if route_stage_delta
                .get("diverged")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                route_stage_deltas.push(route_stage_delta.clone());
            }
            if failures.is_empty() && setup_failures.is_empty() {
                passed_turns = passed_turns.saturating_add(1);
            }
            for reason in failures.iter().chain(setup_failures.iter()) {
                failure_events.push(failure_event(FailureEventInput {
                    agent_id: &agent_id,
                    case_id: &case_id,
                    turn_id: &turn_id,
                    reason,
                    user: &user_message,
                    response: &response_text,
                    cases_path: &cases_path,
                    live,
                    base_url: &base_url,
                }));
            }
            rows.push(json!({
                "case_id": case_id.clone(),
                "turn_id": turn_id.clone(),
                "agent_id": agent_id.clone(),
                "synthetic_user_role": "user",
                "request_body_keys": ["message"],
                "normal_user_message_route_only": true,
                "user_message_preview": clean_text(&user_message, 240),
                "assistant_response_preview": clean_text(&response_text, 320),
                "capability_domain": str_opt(turn, &["capability", "domain"]),
                "capability_level": turn.pointer("/capability/level").and_then(Value::as_u64),
                "grader_contract": turn.get("grader").cloned().unwrap_or_else(|| json!({})),
                "route_error_code": route_error_code,
                "latency_ms": latency_ms,
                "response_token_count": response_token_count,
                "workflow_stage_count": workflow_stage_count,
                "workflow_budget_stage_count": workflow_budget_stage_count,
                "route_stage_delta": route_stage_delta,
                "response_payload_diagnostics": response_payload_diagnostics(&response_payload),
                "workflow_visible": workflow_visible(&response_payload),
                "live_eval_chat_injection_allowed": response_payload
                    .pointer("/live_eval_monitor/chat_injection_allowed")
                    .and_then(Value::as_bool),
                "pass": failures.is_empty() && setup_failures.is_empty(),
                "failures": failures,
            }));
            previous_response = response_text;
        }
    }

    let final_live_monitor_marker = live_monitor_marker(&live_monitor_latest_path);
    let live_monitor_freshness = live_monitor_freshness_report(
        live,
        require_live_monitor_freshness,
        initial_live_monitor_marker.as_deref(),
        final_live_monitor_marker.as_deref(),
        &observed_live_monitor_markers,
        &live_monitor_latest_path,
    );
    if !live_monitor_freshness
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(true)
    {
        let reason = "live_eval_monitor_timestamp_not_advanced";
        setup_failures.push(reason.to_string());
        let monitor_agent_id = normalize_agent_id(
            parse_flag(args, "agent-id")
                .as_deref()
                .unwrap_or(DEFAULT_MISTY_AGENT_ID),
        );
        let monitor_response = live_monitor_freshness.to_string();
        failure_events.push(failure_event(FailureEventInput {
            agent_id: &monitor_agent_id,
            case_id: "live_eval_monitor",
            turn_id: "freshness",
            reason,
            user: "live eval monitor freshness check",
            response: &monitor_response,
            cases_path: &cases_path,
            live,
            base_url: &base_url,
        }));
    }

    let failure_count = failure_events.len() as u64;
    let pass_rate = ratio(passed_turns, total_turns);
    let agent_work_success = agent_work_success_report(&rows, &thresholds);
    let agent_work_success_ok = bool_at(&agent_work_success, &["overall", "ok"], true);
    let ok = cases.len() as u64 >= min_cases
        && pass_rate >= min_pass_rate
        && failure_count <= max_failures
        && agent_work_success_ok
        && setup_failures.is_empty();
    let report = json!({
        "type": "synthetic_user_chat_harness",
        "schema_version": 1,
        "generated_at": now_iso_like(),
        "ok": ok,
        "mode": if live { "live_dashboard" } else { "offline_mock" },
        "transport_contract": {
            "normal_user_message_route_only": true,
            "target_route": "/api/agents/{agent_id}/message",
            "request_body_keys": ["message"],
            "synthetic_user_may_set_system_or_assistant_role": false,
        },
        "summary": {
            "cases": cases.len(),
            "total_turns": total_turns,
            "passed_turns": passed_turns,
            "pass_rate": pass_rate,
            "failure_count": failure_count,
            "min_cases": min_cases,
            "min_pass_rate": min_pass_rate,
            "max_failures": max_failures,
            "simple_direct_budgets": {
                "max_latency_ms": u64_at(&thresholds, &["simple_direct_max_latency_ms"], 0),
                "live_max_latency_ms": u64_at(
                    &thresholds,
                    &["simple_direct_live_max_latency_ms"],
                    u64_at(&thresholds, &["simple_direct_max_latency_ms"], 0)
                ),
                "max_response_tokens": u64_at(&thresholds, &["simple_direct_max_response_tokens"], 0),
                "max_stage_count": u64_at(&thresholds, &["simple_direct_max_stage_count"], 0)
            },
            "agent_work_success": agent_work_success
        },
        "setup_failures": setup_failures,
        "live_monitor_freshness": live_monitor_freshness,
        "turns": rows,
        "route_stage_deltas": route_stage_deltas,
        "failure_events": failure_events,
        "sources": { "cases": cases_path }
    });
    let markdown = markdown_report(&report);
    let writes_ok = write_json(&out_path, &report).is_ok()
        && write_json(&out_latest_path, &report).is_ok()
        && write_text(&markdown_path, &markdown).is_ok()
        && append_jsonl(&failures_path, &failure_events).is_ok()
        && write_attention_events(&attention_dir, &failure_events).is_ok();
    if !writes_ok {
        eprintln!("synthetic_user_chat_harness: failed to write one or more outputs");
        return 2;
    }
    print_json_line(&report);
    if strict && !ok {
        1
    } else {
        0
    }
}

pub fn run_misty_live_health_gate(args: &[String]) -> i32 {
    let agent_id =
        parse_flag(args, "agent-id").unwrap_or_else(|| DEFAULT_MISTY_AGENT_ID.to_string());
    let mut harness_args = vec![
        "--live=1".to_string(),
        format!("--agent-id={}", normalize_agent_id(&agent_id)),
        "--strict=1".to_string(),
        "--require-live-monitor-freshness=1".to_string(),
    ];
    for key in [
        "cases",
        "out",
        "out-latest",
        "out-markdown",
        "failures-out",
        "attention-dir",
        "base-url",
        "timeout-seconds",
        "live-monitor-latest",
        "allow-remote",
    ] {
        if let Some(value) = parse_flag(args, key) {
            harness_args.push(format!("--{key}={value}"));
        }
    }
    run_synthetic_user_chat_harness(&harness_args)
}

#[cfg(test)]
pub fn misty_live_health_gate_required_command(agent_id: &str) -> String {
    format!(
        "cargo run --quiet --manifest-path orchestration/Cargo.toml --bin eval_runtime -- synthetic-user-chat-harness --live=1 --agent-id={} --strict=1",
        normalize_agent_id(agent_id)
    )
}

struct TurnEvaluation<'a> {
    live: bool,
    turn: &'a Value,
    thresholds: &'a Value,
    user_message: &'a str,
    response_text: &'a str,
    previous_response: &'a str,
    payload: &'a Value,
    route_error_code: Option<&'a str>,
    latency_ms: u64,
    response_token_count: u64,
    workflow_stage_count: u64,
}

fn evaluate_turn(input: TurnEvaluation<'_>) -> Vec<String> {
    let TurnEvaluation {
        live,
        turn,
        thresholds,
        user_message,
        response_text,
        previous_response,
        payload,
        route_error_code,
        latency_ms,
        response_token_count,
        workflow_stage_count,
    } = input;
    let mut failures = Vec::new();
    if user_message.trim().is_empty() {
        failures.push("missing_synthetic_user_message".to_string());
    }
    if let Some(code) = route_error_code {
        failures.push(format!("message_route_error:{code}"));
    } else if response_text.trim().is_empty() && !payload_has_pending_tool(payload) {
        failures.push("empty_assistant_response".to_string());
    }
    if !response_text.trim().is_empty()
        && normalize_for_compare(response_text) == normalize_for_compare(previous_response)
    {
        failures.push("repeated_assistant_response".to_string());
    }
    for needle in default_forbidden_phrases()
        .into_iter()
        .chain(string_array_at(turn, &["expect", "forbidden_substrings"]))
    {
        if response_text
            .to_ascii_lowercase()
            .contains(&needle.to_ascii_lowercase())
        {
            failures.push(format!("forbidden_visible_text:{needle}"));
        }
    }
    for needle in string_array_at(turn, &["expect", "required_substrings"]) {
        let normalized_response = normalize_for_compare(response_text).replace(['_', '-'], " ");
        let normalized_needle = normalize_for_compare(&needle).replace(['_', '-'], " ");
        if !normalized_response.contains(&normalized_needle) {
            failures.push(format!("missing_required_visible_text:{needle}"));
        }
    }
    if bool_at(turn, &["expect", "require_workflow_visibility"], false)
        && !workflow_visible(payload)
    {
        failures.push("missing_workflow_visibility_payload".to_string());
    }
    if payload
        .pointer("/live_eval_monitor/chat_injection_allowed")
        .and_then(Value::as_bool)
        == Some(true)
    {
        failures.push("live_eval_monitor_allows_chat_injection".to_string());
    }
    if bool_at(turn, &["expect", "require_live_eval_monitor"], false)
        && payload.pointer("/live_eval_monitor").is_none()
    {
        failures.push("missing_live_eval_monitor_payload".to_string());
    }
    if bool_at(turn, &["expect", "forbid_gate_choice_leakage"], false)
        && visible_workflow_gate_choice_leakage(response_text, payload)
    {
        failures.push("visible_workflow_gate_choice_leakage".to_string());
    }
    failures.extend(workflow_eval_failure_modes(
        response_text,
        previous_response,
        payload,
        latency_ms,
        thresholds,
    ));
    let has_tool_progress = payload_has_tool_progress(payload);
    if bool_at(turn, &["expect", "require_tool_progress"], false) && !has_tool_progress {
        failures.push("missing_tool_progress_evidence".to_string());
    }
    let has_tool_execution = payload_has_tool_execution_evidence(payload);
    if bool_at(turn, &["expect", "require_tool_execution_evidence"], false) && !has_tool_execution {
        failures.push("missing_tool_execution_evidence".to_string());
    }
    if bool_at(turn, &["expect", "require_final_synthesis"], false)
        && !payload_final_llm_synthesized(payload)
    {
        failures.push("missing_final_synthesis_status".to_string());
    }
    if bool_at(turn, &["expect", "require_tool_result_synthesis"], false)
        && has_tool_execution
        && (!payload_final_llm_synthesized(payload) || response_text.trim().is_empty())
    {
        failures.push("tool_execution_without_final_synthesis".to_string());
    }
    if bool_at(turn, &["expect", "require_useful_plaintext_answer"], false)
        && !response_is_useful_plaintext_answer(user_message, response_text)
    {
        failures.push("missing_useful_plaintext_answer".to_string());
    }
    if bool_at(
        turn,
        &["expect", "forbid_unresolved_tool_need_without_progress"],
        false,
    ) && !has_tool_progress
        && response_exposes_unresolved_tool_need(response_text)
    {
        failures.push("unresolved_tool_need_without_progress".to_string());
    }
    if !has_tool_progress && response_claims_tool_progress_without_evidence(response_text) {
        failures.push("unsupported_tool_claim_without_progress".to_string());
    }
    if bool_at(turn, &["expect", "simple_direct_conversation"], false) {
        let default_max_latency_ms = if live {
            u64_at(
                thresholds,
                &["simple_direct_live_max_latency_ms"],
                u64_at(thresholds, &["simple_direct_max_latency_ms"], 0),
            )
        } else {
            u64_at(thresholds, &["simple_direct_max_latency_ms"], 0)
        };
        let max_latency_ms = u64_at(turn, &["expect", "max_latency_ms"], default_max_latency_ms);
        let max_response_tokens = u64_at(
            turn,
            &["expect", "max_response_tokens"],
            u64_at(thresholds, &["simple_direct_max_response_tokens"], 0),
        );
        let max_stage_count = u64_at(
            turn,
            &["expect", "max_stage_count"],
            u64_at(thresholds, &["simple_direct_max_stage_count"], 0),
        );
        if max_latency_ms > 0 && latency_ms > max_latency_ms {
            failures.push(format!(
                "simple_direct_latency_over_budget:{latency_ms}>{max_latency_ms}"
            ));
        }
        if max_response_tokens > 0 && response_token_count > max_response_tokens {
            failures.push(format!(
                "simple_direct_response_tokens_over_budget:{response_token_count}>{max_response_tokens}"
            ));
        }
        let workflow_budget_stage_count =
            workflow_budget_stage_count(payload, workflow_stage_count);
        if max_stage_count > 0 && workflow_budget_stage_count > max_stage_count {
            failures.push(format!(
                "simple_direct_stage_count_over_budget:{workflow_budget_stage_count}>{max_stage_count}"
            ));
        }
        if payload
            .get("tools")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false)
        {
            failures.push("simple_direct_recorded_tool_calls".to_string());
        }
        if payload
            .pointer("/response_workflow/tool_gate/should_call_tools")
            .and_then(Value::as_bool)
            == Some(true)
        {
            failures.push("simple_direct_tool_gate_should_call_tools".to_string());
        }
    }
    failures
}

fn workflow_eval_failure_modes(
    response_text: &str,
    previous_response: &str,
    payload: &Value,
    latency_ms: u64,
    thresholds: &Value,
) -> Vec<String> {
    let mut failures = Vec::new();
    let normalized = normalize_for_compare(response_text);
    let pending_tool = payload_has_pending_tool(payload);
    if normalized.is_empty() && !pending_tool {
        failures.push("empty_direct_reply".to_string());
    }
    if normalized.matches("need tools? yes/no").count() > 1 {
        failures.push("repeated_gate_prompt".to_string());
    }
    if visible_workflow_gate_choice_leakage(response_text, payload) {
        failures.push("gate_token_leakage".to_string());
    }
    if hidden_second_pass_suspected(payload) {
        failures.push("hidden_second_pass_call".to_string());
    }
    let pending_limit = u64_at(thresholds, &["pending_tool_stuck_max_latency_ms"], 30_000);
    if pending_tool && pending_limit > 0 && latency_ms > pending_limit {
        failures.push(format!(
            "pending_tool_stuck_too_long:{latency_ms}>{pending_limit}"
        ));
    }
    if payload_has_tool_result_without_synthesis(payload, response_text) {
        failures.push("tool_result_without_synthesis".to_string());
    }
    if !previous_response.trim().is_empty()
        && normalize_for_compare(previous_response) == normalized
        && normalized.contains("need tools? yes/no")
    {
        failures.push("repeated_gate_prompt".to_string());
    }
    failures.sort();
    failures.dedup();
    failures
}

fn payload_has_tool_progress(payload: &Value) -> bool {
    payload
        .get("tools")
        .and_then(Value::as_array)
        .map(|rows| !rows.is_empty())
        .unwrap_or(false)
        || payload
            .pointer("/pending_tool_request/status")
            .and_then(Value::as_str)
            == Some("pending_confirmation")
        || payload
            .pointer("/response_workflow/pending_tool_request/status")
            .and_then(Value::as_str)
            == Some("pending_confirmation")
        || payload
            .pointer("/response_workflow/manual_toolbox_pending_tool_request/status")
            .and_then(Value::as_str)
            == Some("pending_confirmation")
        || payload
            .pointer("/response_finalization/pending_tool_request/status")
            .and_then(Value::as_str)
            == Some("pending_confirmation")
        || payload
            .pointer("/response_finalization/tooling_invariant/tool_attempted")
            .and_then(Value::as_bool)
            == Some(true)
        || payload
            .pointer("/response_finalization/web_invariant/tool_attempted")
            .and_then(Value::as_bool)
            == Some(true)
        || payload
            .pointer("/response_finalization/tool_completion/tool_attempts")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false)
}

fn payload_has_tool_execution_evidence(payload: &Value) -> bool {
    payload
        .get("tools")
        .and_then(Value::as_array)
        .map(|rows| !rows.is_empty())
        .unwrap_or(false)
        || payload
            .pointer("/response_finalization/tool_completion/tool_attempts")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false)
}

fn payload_final_llm_synthesized(payload: &Value) -> bool {
    payload
        .pointer("/response_workflow/final_llm_response/status")
        .and_then(Value::as_str)
        == Some("synthesized")
        || payload
            .pointer("/workflow_visibility/finalization_diagnostics/final_llm_status")
            .and_then(Value::as_str)
            == Some("synthesized")
}

fn payload_has_pending_tool(payload: &Value) -> bool {
    [
        "/pending_tool_request/status",
        "/response_workflow/pending_tool_request/status",
        "/response_workflow/manual_toolbox_pending_tool_request/status",
        "/response_finalization/pending_tool_request/status",
    ]
    .iter()
    .any(|pointer| payload.pointer(pointer).and_then(Value::as_str) == Some("pending_confirmation"))
}

fn hidden_second_pass_suspected(payload: &Value) -> bool {
    if payload
        .pointer("/response_finalization/workflow_system_fallback_used")
        .and_then(Value::as_bool)
        == Some(true)
        || payload
            .pointer("/response_workflow/final_llm_response/fallback_guard_multi_stage")
            .and_then(Value::as_bool)
            == Some(true)
    {
        return true;
    }
    let attempt_count = payload
        .pointer("/response_workflow/final_llm_response/attempt_count")
        .and_then(Value::as_u64)
        .unwrap_or(1);
    attempt_count > 1 && !payload_final_llm_synthesized(payload)
}

fn payload_has_tool_result_without_synthesis(payload: &Value, response_text: &str) -> bool {
    let has_tool_result = payload
        .get("tools")
        .and_then(Value::as_array)
        .map(|rows| !rows.is_empty())
        .unwrap_or(false)
        || payload
            .pointer("/response_finalization/tool_completion/tool_attempts")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false);
    if !has_tool_result {
        return false;
    }
    let synthesized = payload
        .pointer("/response_workflow/final_llm_response/status")
        .and_then(Value::as_str)
        == Some("synthesized");
    !synthesized || response_text.trim().is_empty()
}

fn response_exposes_unresolved_tool_need(response_text: &str) -> bool {
    let lowered = normalize_for_compare(response_text);
    if lowered.is_empty() {
        return false;
    }
    [
        "i don't have current web search results",
        "i do not have current web search results",
        "i don't have usable tool findings",
        "i do not have usable tool findings",
        "i'll need to perform a web search",
        "i will need to perform a web search",
        "let me run that search",
        "if you'd like me to search",
        "if you would like me to search",
        "after the tool returns",
        "would choose the web search",
        "would choose web search",
        "would choose the workspace search",
        "would choose workspace search",
    ]
    .iter()
    .any(|needle| lowered.contains(*needle))
}

fn response_claims_tool_progress_without_evidence(response_text: &str) -> bool {
    let lowered = normalize_for_compare(response_text);
    if lowered.is_empty() {
        return false;
    }
    let hypothetical_only = [
        "i would use",
        "i would choose",
        "i'd use",
        "i'd choose",
        "would use",
        "would choose",
    ]
    .iter()
    .any(|needle| lowered.contains(*needle));
    let claims_result = [
        "web search returned",
        "web search didn't return",
        "web search did not return",
        "search returned",
        "search didn't return",
        "search did not return",
        "tool returned",
        "tool didn't return",
        "tool did not return",
        "i searched",
        "i ran",
        "i used the tool",
        "i called",
        "i executed",
        "workspace search returned",
        "workspace search didn't return",
        "workspace search did not return",
    ]
    .iter()
    .any(|needle| lowered.contains(*needle));
    claims_result && !hypothetical_only
}

fn response_is_useful_plaintext_answer(user_message: &str, response_text: &str) -> bool {
    let normalized = normalize_for_compare(response_text);
    if normalized.is_empty()
        || normalized.split_whitespace().count() < 6
        || response_looks_like_raw_tool_or_provider_fragment(&normalized)
    {
        return false;
    }
    let terms = significant_user_terms(user_message);
    let overlap = terms
        .iter()
        .filter(|term| normalized.contains(term.as_str()))
        .count();
    let bounded = [
        "can't make a reliable",
        "cannot make a reliable",
        "could not make a reliable",
        "can't verify",
        "cannot verify",
        "does not support",
        "did not return",
        "not enough reliable",
        "not enough source backed",
        "insufficient evidence",
        "no reliable",
    ]
    .iter()
    .any(|needle| normalized.contains(*needle));
    if bounded {
        return overlap > 0 || terms.is_empty();
    }
    overlap >= terms.len().min(2).max(1)
}

fn response_looks_like_raw_tool_or_provider_fragment(normalized_response: &str) -> bool {
    let raw_markers = [
        "tool trace complete",
        "web search from web retrieval",
        "recorded evidence so far here's what i found",
        "no usable evidence claims",
        "provider starved",
        "provider_starved",
        "runtime_tool_evidence_fallback",
        "receipt backed synthesis",
    ];
    raw_markers
        .iter()
        .any(|needle| normalized_response.contains(*needle))
}

fn significant_user_terms(user_message: &str) -> Vec<String> {
    normalize_for_compare(user_message)
        .split_whitespace()
        .filter_map(|term| {
            let cleaned = term
                .trim_matches(|ch: char| !ch.is_ascii_alphanumeric())
                .to_string();
            if cleaned.len() < 4 || USER_TERM_STOPWORDS.contains(&cleaned.as_str()) {
                None
            } else {
                Some(cleaned)
            }
        })
        .fold(Vec::<String>::new(), |mut acc, term| {
            if !acc.iter().any(|existing| existing == &term) {
                acc.push(term);
            }
            acc
        })
}

const USER_TERM_STOPWORDS: &[&str] = &[
    "about",
    "after",
    "again",
    "answer",
    "because",
    "before",
    "current",
    "directly",
    "does",
    "from",
    "give",
    "have",
    "into",
    "just",
    "like",
    "only",
    "please",
    "query",
    "request",
    "same",
    "search",
    "some",
    "tell",
    "that",
    "their",
    "them",
    "then",
    "there",
    "this",
    "turn",
    "what",
    "which",
    "with",
    "work",
    "would",
    "your",
];

fn visible_workflow_gate_choice_leakage(response_text: &str, payload: &Value) -> bool {
    if !workflow_visible(payload) {
        return false;
    }
    let normalized = normalize_for_compare(response_text);
    let normalized = normalized.trim();
    let compact = normalized
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch.is_whitespace() {
                ch
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if normalized == "respond directly" || normalized.contains("need tools? yes/no") {
        return true;
    }
    if normalized.starts_with("yes. tool family:")
        || normalized.starts_with("yes. tool:")
        || compact.starts_with("this kind of work is respond directly")
        || compact.starts_with(&format!(
            "this kind of work is direct answer {}",
            "conversation"
        ))
        || compact.starts_with("this kind of work is planning from current context")
        || compact.starts_with("this kind of work is web research")
        || compact.starts_with("this kind of work is workspace files")
        || compact.starts_with("this kind of work is code execution terminal")
        || compact.starts_with("this kind of work is agent management")
        || compact.starts_with("this kind of work is memory notes")
        || compact.starts_with("this kind of work is external apps integrations")
        || (normalized.contains("category:")
            && (normalized.contains("tool family:")
                || normalized.contains("tool:")
                || normalized.contains("request payload:")
                || normalized.contains("payload:")))
        || (normalized.starts_with("yes. ")
            && (normalized.contains("request payload:")
                || normalized.contains("tool family:")
                || normalized.contains("workspace search")
                || normalized.contains("web search")))
    {
        return true;
    }
    normalized.starts_with("no. ")
        && (normalized.contains("would use")
            || normalized.contains("answer directly")
            || normalized.contains("web search")
            || normalized.contains("workspace search")
            || normalized.contains("file_read")
            || normalized.contains("read_file")
            || normalized.contains("tool"))
}

fn route_stage_delta(
    turn: &Value,
    payload: &Value,
    route_error_code: Option<&str>,
    response_text: &str,
    actual_stage_count: u64,
    failures: &[String],
) -> Value {
    let baseline = turn.get("mock_response").unwrap_or(&Value::Null);
    let baseline_response = assistant_text(baseline);
    let baseline_stage_count = workflow_stage_count(baseline);
    let baseline_workflow_visible = workflow_visible(baseline);
    let baseline_live_monitor = baseline.pointer("/live_eval_monitor").is_some();
    let actual_workflow_visible = workflow_visible(payload);
    let actual_live_monitor = payload.pointer("/live_eval_monitor").is_some();
    let first_missing_stage = if let Some(code) = route_error_code {
        format!("message_route_error:{code}")
    } else if baseline_workflow_visible && !actual_workflow_visible {
        "workflow_library_selection_or_payload_assembly".to_string()
    } else if baseline_stage_count > actual_stage_count {
        "workflow_stage_progression".to_string()
    } else if !baseline_response.trim().is_empty() && response_text.trim().is_empty() {
        "final_response".to_string()
    } else if baseline_live_monitor && !actual_live_monitor {
        "live_eval_monitor_attachment".to_string()
    } else if failures.is_empty() {
        "none".to_string()
    } else {
        owner_component_for_failure(failures[0].as_str()).to_string()
    };
    json!({
        "contract": "route_stage_delta_v1",
        "diverged": first_missing_stage != "none",
        "first_missing_stage": first_missing_stage,
        "baseline": {
            "workflow_visible": baseline_workflow_visible,
            "workflow_stage_count": baseline_stage_count,
            "response_empty": baseline_response.trim().is_empty(),
            "live_eval_monitor_present": baseline_live_monitor
        },
        "actual": {
            "workflow_visible": actual_workflow_visible,
            "workflow_stage_count": actual_stage_count,
            "response_empty": response_text.trim().is_empty(),
            "live_eval_monitor_present": actual_live_monitor,
            "route_error_code": route_error_code
        },
        "failures": failures
    })
}

fn response_payload_diagnostics(payload: &Value) -> Value {
    json!({
        "top_keys": payload
            .as_object()
            .map(|obj| obj.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default(),
        "workflow_stage_statuses": payload
            .pointer("/response_workflow/stage_statuses")
            .cloned()
            .unwrap_or_else(|| json!([])),
        "workflow_control": payload
            .pointer("/response_workflow/workflow_control")
            .cloned()
            .unwrap_or_else(|| json!({})),
        "tool_gate": payload
            .pointer("/response_workflow/tool_gate")
            .cloned()
            .unwrap_or_else(|| json!({})),
        "pending_tool_request": payload
            .get("pending_tool_request")
            .or_else(|| payload.pointer("/response_workflow/pending_tool_request"))
            .or_else(|| payload.pointer("/response_workflow/manual_toolbox_pending_tool_request"))
            .or_else(|| payload.pointer("/response_finalization/pending_tool_request"))
            .cloned()
            .unwrap_or(Value::Null),
        "tools": payload.get("tools").cloned().unwrap_or_else(|| json!([])),
        "latent_tool_candidates": payload
            .get("latent_tool_candidates")
            .cloned()
            .unwrap_or_else(|| json!([])),
        "final_llm_response": payload
            .pointer("/response_workflow/final_llm_response")
            .cloned()
            .unwrap_or_else(|| json!({})),
        "response_finalization": {
            "outcome": payload
                .pointer("/response_finalization/outcome")
                .and_then(Value::as_str)
                .map(clean_debug_string),
            "visible_response_source": payload
                .pointer("/response_finalization/visible_response_source")
                .and_then(Value::as_str)
                .map(clean_debug_string),
            "web_invariant": payload
                .pointer("/response_finalization/web_invariant")
                .cloned()
                .unwrap_or_else(|| json!({})),
            "tooling_invariant": payload
                .pointer("/response_finalization/tooling_invariant")
                .cloned()
                .unwrap_or_else(|| json!({}))
        },
        "routing_trace": payload
            .get("routing_trace")
            .cloned()
            .unwrap_or_else(|| json!({}))
    })
}

fn clean_debug_string(raw: &str) -> String {
    clean_text(raw, 1_000)
}

fn estimate_response_tokens(text: &str) -> u64 {
    clean_text(text, 12_000).split_whitespace().count() as u64
}

fn workflow_stage_count(payload: &Value) -> u64 {
    for pointer in [
        "/response_workflow/stage_statuses",
        "/response_workflow/trace_streams/workflow_state",
        "/workflow_trace/stage_statuses",
        "/workflow_state/stage_statuses",
    ] {
        if let Some(rows) = payload.pointer(pointer).and_then(Value::as_array) {
            return rows.len() as u64;
        }
    }
    0
}

fn workflow_budget_stage_count(payload: &Value, fallback_stage_count: u64) -> u64 {
    for pointer in [
        "/response_workflow/stage_statuses",
        "/response_workflow/trace_streams/workflow_state",
        "/workflow_trace/stage_statuses",
        "/workflow_state/stage_statuses",
    ] {
        let Some(rows) = payload.pointer(pointer).and_then(Value::as_array) else {
            continue;
        };
        let gate_rows = rows
            .iter()
            .filter(|row| {
                row.get("stage")
                    .and_then(Value::as_str)
                    .map(|stage| stage.starts_with("gate_"))
                    .unwrap_or(false)
            })
            .count() as u64;
        if gate_rows > 0 {
            return gate_rows;
        }
        return rows.len() as u64;
    }
    fallback_stage_count
}

struct FailureEventInput<'a> {
    agent_id: &'a str,
    case_id: &'a str,
    turn_id: &'a str,
    reason: &'a str,
    user: &'a str,
    response: &'a str,
    cases_path: &'a str,
    live: bool,
    base_url: &'a str,
}

fn failure_event(input: FailureEventInput<'_>) -> Value {
    let FailureEventInput {
        agent_id,
        case_id,
        turn_id,
        reason,
        user,
        response,
        cases_path,
        live,
        base_url,
    } = input;
    let seed = format!(
        "{agent_id}:{case_id}:{turn_id}:{reason}:{}",
        clean_text(response, 120)
    );
    let owner_component = owner_component_for_failure(reason);
    let replay_command = synthetic_harness_replay_command(cases_path, agent_id, live, base_url);
    json!({
        "source_type": "synthetic_user_chat_harness_failure",
        "source": format!("agent:{agent_id}"),
        "owner_component": owner_component,
        "severity": if reason.contains("forbidden") || reason.contains("empty") || reason.contains("repeated") { "high" } else { "warn" },
        "attention_key": format!("synthetic_user:{agent_id}:{}", stable_hash_hex(&seed)),
        "summary": format!("Synthetic user harness failure in {case_id}/{turn_id}: {reason}"),
        "replay_command": replay_command,
        "raw_event": {
            "agent_id": agent_id,
            "case_id": case_id,
            "turn_id": turn_id,
            "reason": reason,
            "owner_component": owner_component,
            "replay_command": replay_command,
            "user_message_preview": clean_text(user, 240),
            "assistant_response_preview": clean_text(response, 320),
        }
    })
}

fn synthetic_harness_replay_command(
    cases_path: &str,
    agent_id: &str,
    live: bool,
    base_url: &str,
) -> String {
    let mut command = format!(
        "cargo run --quiet --manifest-path orchestration/Cargo.toml --bin eval_runtime -- synthetic-user-chat-harness --live={} --agent-id={} --strict=1 --cases={}",
        if live { "1" } else { "0" },
        normalize_agent_id(agent_id),
        cases_path
    );
    if live {
        command.push_str(&format!(" --base-url={}", clean_text(base_url, 500)));
    }
    command
}

fn owner_component_for_failure(reason: &str) -> &'static str {
    let lowered = reason.to_ascii_lowercase();
    if lowered.contains("workflow") || lowered.contains("stage") || lowered.contains("tool_gate") {
        "orchestration.workflow"
    } else if lowered.contains("live_eval")
        || lowered.contains("monitor")
        || lowered.contains("telemetry")
    {
        "orchestration.telemetry"
    } else if lowered.contains("tool") || lowered.contains("web") {
        "orchestration.tool_routing"
    } else if lowered.contains("empty")
        || lowered.contains("forbidden")
        || lowered.contains("repeated")
        || lowered.contains("response")
    {
        "orchestration.finalization"
    } else {
        "orchestration.eval"
    }
}

fn live_monitor_marker(path: &str) -> Option<String> {
    let value = read_json(path);
    live_monitor_marker_from_payload(&value)
}

fn live_monitor_marker_from_payload(value: &Value) -> Option<String> {
    for pointer in [
        "/generated_at",
        "/updated_at",
        "/last_turn_at",
        "/latest_turn/generated_at",
        "/monitor/generated_at",
    ] {
        if let Some(raw) = value.pointer(pointer).and_then(Value::as_str) {
            let marker = clean_text(raw, 160);
            if !marker.is_empty() {
                return Some(marker);
            }
        }
    }
    value
        .get("turn_receipt_id")
        .or_else(|| value.get("receipt"))
        .and_then(Value::as_str)
        .map(|raw| clean_text(raw, 160))
        .filter(|raw| !raw.is_empty())
}

fn live_monitor_freshness_report(
    live: bool,
    required: bool,
    before: Option<&str>,
    after: Option<&str>,
    observed: &[String],
    source_path: &str,
) -> Value {
    let observed_advanced = observed
        .iter()
        .any(|marker| !marker.is_empty() && Some(marker.as_str()) != before);
    let file_advanced = after.is_some() && after != before;
    let ok = !live || !required || observed_advanced || file_advanced;
    json!({
        "contract": "live_eval_monitor_freshness_v1",
        "ok": ok,
        "live": live,
        "required": required,
        "initial_marker": before,
        "final_marker": after,
        "observed_markers": observed,
        "advanced": observed_advanced || file_advanced,
        "source_path": source_path,
        "failure_reason": if ok { Value::Null } else { json!("live_eval_monitor_timestamp_not_advanced") }
    })
}

fn default_forbidden_phrases() -> Vec<String> {
    [
        "I hit a response finalization edge",
        "workflow finalization edge",
        "completed the workflow gate",
        "final workflow state was unexpected",
        "please retry so I can rerun",
        "[source:workflow_route_classification]",
        "[source:tool_gate]",
        "[source:workflow_gate]",
    ]
    .iter()
    .map(|value| value.to_string())
    .collect()
}

#[cfg(test)]
#[path = "eval_synthetic_user_chat_harness_tests.rs"]
mod eval_synthetic_user_chat_harness_tests;
