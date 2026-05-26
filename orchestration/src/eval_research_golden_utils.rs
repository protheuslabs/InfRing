use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const DASHBOARD_STATE_ROOT_ENV: &str = "INFRING_TOOLING_DASHBOARD_STATE_ROOT";
const AGENT_SESSIONS_SUBDIR: &str = "agent_sessions";
const BATCH_QUERY_CACHE_REL: &str = "client/runtime/local/state/batch_query/cache.json";

#[derive(Clone, Debug, Default)]
struct SessionSnapshot {
    total_messages: usize,
    assistant_messages: usize,
}

pub(super) fn load_responses_by_case(path: &str) -> BTreeMap<String, Value> {
    let input = read_json(path);
    let rows = input
        .get("responses")
        .or_else(|| input.get("cases"))
        .or_else(|| input.get("turns"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut by_case = BTreeMap::new();
    for row in rows {
        let case_id = str_at(&row, &["case_id"], &str_at(&row, &["id"], ""));
        if case_id.is_empty() {
            continue;
        }
        let payload = row
            .get("response_payload")
            .or_else(|| row.get("payload"))
            .or_else(|| row.get("mock_response"))
            .cloned()
            .unwrap_or(row);
        by_case.insert(case_id, payload);
    }
    by_case
}

pub(super) fn response_sequence_payload(source: &Value, index: usize) -> Option<Value> {
    source
        .get("response_sequence")
        .or_else(|| source.get("__research_golden_response_sequence"))
        .and_then(Value::as_array)
        .and_then(|rows| rows.get(index))
        .cloned()
}

pub(super) fn payload_has_pending_tool_confirmation(payload: &Value) -> bool {
    [
        "/pending_tool_request/status",
        "/response_workflow/pending_tool_request/status",
        "/response_workflow/manual_toolbox_pending_tool_request/status",
        "/response_finalization/pending_tool_request/status",
    ]
    .iter()
    .any(|pointer| {
        payload
            .pointer(pointer)
            .and_then(Value::as_str)
            .map(|status| status == "pending_confirmation")
            .unwrap_or(false)
    })
}

pub(super) fn create_live_agent(
    base_url: &str,
    case_id: &str,
    parent_agent_id: &str,
    model_ref: Option<&str>,
    timeout_seconds: u64,
) -> Option<String> {
    let name = format!("Research Golden {}", clean_text(case_id, 80));
    let payload = post_json(
        base_url,
        "/api/agents",
        &json!({
            "name": name,
            "role": "analyst",
            "parent_agent_id": parent_agent_id
        }),
        timeout_seconds,
    );
    str_opt(&payload, &["agent_id"])
        .or_else(|| str_opt(&payload, &["id"]))
        .map(normalize_agent_id)
        .filter(|agent_id| !agent_id.is_empty())
        .and_then(|agent_id| {
            if set_live_agent_model(base_url, &agent_id, model_ref, timeout_seconds) {
                Some(agent_id)
            } else {
                None
            }
        })
}

pub(super) fn create_root_live_agent(
    base_url: &str,
    name_hint: &str,
    model_ref: Option<&str>,
    timeout_seconds: u64,
) -> Option<String> {
    let name = if name_hint.is_empty() {
        "Research Golden Live".to_string()
    } else {
        format!("Research Golden {}", clean_text(name_hint, 80))
    };
    let payload = post_json(
        base_url,
        "/api/agents",
        &json!({
            "name": name,
            "role": "analyst"
        }),
        timeout_seconds,
    );
    str_opt(&payload, &["agent_id"])
        .or_else(|| str_opt(&payload, &["id"]))
        .map(normalize_agent_id)
        .filter(|agent_id| !agent_id.is_empty())
        .and_then(|agent_id| {
            if set_live_agent_model(base_url, &agent_id, model_ref, timeout_seconds) {
                Some(agent_id)
            } else {
                None
            }
        })
}

pub(super) fn ensure_live_eval_agent(
    base_url: &str,
    requested_agent_id: &str,
    model_ref: Option<&str>,
    timeout_seconds: u64,
) -> Result<(String, bool, String), String> {
    if let Some(reason) = current_live_agent_reason(base_url, requested_agent_id, timeout_seconds) {
        return Ok((requested_agent_id.to_string(), false, reason));
    }
    create_root_live_agent(base_url, requested_agent_id, model_ref, timeout_seconds)
        .map(|agent_id| {
            (
                agent_id,
                true,
                if requested_agent_id.is_empty() {
                    "bootstrapped_live_agent_missing_request".to_string()
                } else {
                    "bootstrapped_live_agent_from_missing_or_inactive_request".to_string()
                },
            )
        })
        .ok_or_else(|| "live_agent_bootstrap_failed".to_string())
}

fn current_live_agent_reason(
    base_url: &str,
    requested_agent_id: &str,
    timeout_seconds: u64,
) -> Option<String> {
    let requested = clean_text(requested_agent_id, 240);
    if requested.is_empty() {
        return None;
    }
    let registry = curl_json("GET", base_url, "/api/agents", &json!({}), timeout_seconds);
    let row = live_agent_registry_rows(&registry)
        .into_iter()
        .find(|row| {
            clean_text(
                row.get("agent_id")
                    .or_else(|| row.get("id"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                240,
            ) == requested
        })?;
    live_agent_row_active(row).then_some("requested_live_agent_active".to_string())
}

fn live_agent_registry_rows(registry: &Value) -> Vec<&Value> {
    if let Some(rows) = registry.as_array() {
        return rows.iter().collect();
    }
    registry
        .get("agents")
        .and_then(Value::as_array)
        .map(|rows| rows.iter().collect())
        .unwrap_or_default()
}

fn live_agent_row_active(row: &Value) -> bool {
    let mut states = Vec::<String>::new();
    for path in [
        &["state"][..],
        &["sidebar_status_state"][..],
        &["contract", "status"][..],
    ] {
        if let Some(raw) = str_opt(row, path) {
            let cleaned = clean_text(raw, 64).to_ascii_lowercase();
            if !cleaned.is_empty() {
                states.push(cleaned);
            }
        }
    }
    if states.is_empty() {
        return false;
    }
    states
        .iter()
        .any(|state| matches!(state.as_str(), "active" | "running"))
        && !states
            .iter()
            .any(|state| matches!(state.as_str(), "terminated" | "expired" | "failed"))
}

fn set_live_agent_model(
    base_url: &str,
    agent_id: &str,
    model_ref: Option<&str>,
    timeout_seconds: u64,
) -> bool {
    let Some(model_ref) = model_ref.map(|raw| clean_text(raw, 240)) else {
        return true;
    };
    if model_ref.is_empty() {
        return true;
    }
    let resolved_model_ref = resolve_live_agent_model_ref(base_url, &model_ref, timeout_seconds);
    let response = curl_json(
        "PUT",
        base_url,
        &format!("/api/agents/{agent_id}/model"),
        &json!({ "model": resolved_model_ref }),
        timeout_seconds,
    );
    response.get("ok").and_then(Value::as_bool).unwrap_or(false)
}

fn resolve_live_agent_model_ref(base_url: &str, model_ref: &str, timeout_seconds: u64) -> String {
    let cleaned = clean_text(model_ref, 240);
    if cleaned.is_empty() || cleaned.contains('/') {
        return cleaned;
    }
    let catalog = curl_json("GET", base_url, "/api/models", &json!({}), timeout_seconds);
    resolve_live_agent_model_ref_from_catalog(&cleaned, &catalog)
}

fn resolve_live_agent_model_ref_from_catalog(model_ref: &str, catalog: &Value) -> String {
    let cleaned = clean_text(model_ref, 240);
    if cleaned.is_empty() || cleaned.contains('/') {
        return cleaned;
    }
    let Some(models) = catalog.get("models").and_then(Value::as_array) else {
        return cleaned;
    };

    let mut exact_available_non_auto = None::<String>;
    let mut exact_non_auto = None::<String>;
    for row in models {
        let provider = clean_text(
            row.get("provider")
                .or_else(|| row.get("runtime_provider"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        );
        let model = clean_text(
            row.get("model")
                .or_else(|| row.get("runtime_model"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            240,
        );
        let matches_requested = model == cleaned
            || clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 300) == cleaned;
        if !matches_requested || provider.is_empty() || provider.eq_ignore_ascii_case("auto") {
            continue;
        }
        let resolved = format!("{provider}/{model}");
        if row
            .get("available")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            exact_available_non_auto = Some(resolved);
            break;
        }
        if exact_non_auto.is_none() {
            exact_non_auto = Some(resolved);
        }
    }

    exact_available_non_auto
        .or(exact_non_auto)
        .unwrap_or(cleaned)
}

pub(super) fn delete_live_agent(base_url: &str, agent_id: &str, timeout_seconds: u64) -> Value {
    curl_json(
        "DELETE",
        base_url,
        &format!("/api/agents/{agent_id}"),
        &json!({}),
        timeout_seconds,
    )
}

pub(super) fn isolate_batch_query_cache_for_eval() -> Value {
    let path = repo_root().join(BATCH_QUERY_CACHE_REL);
    if !path.exists() {
        return json!({
            "ok": true,
            "type": "research_golden_cache_isolation",
            "cache_path": path.display().to_string(),
            "removed": false
        });
    }
    match fs::remove_file(&path) {
        Ok(_) => json!({
            "ok": true,
            "type": "research_golden_cache_isolation",
            "cache_path": path.display().to_string(),
            "removed": true
        }),
        Err(err) => json!({
            "ok": false,
            "type": "research_golden_cache_isolation",
            "cache_path": path.display().to_string(),
            "removed": false,
            "error": format!("remove_batch_query_cache_failed:{err}")
        }),
    }
}

pub(super) fn post_agent_message(
    base_url: &str,
    agent_id: &str,
    request: &Value,
    timeout_seconds: u64,
    timeout_recovery_seconds: u64,
) -> Value {
    let path = format!("/api/agents/{agent_id}/message");
    let expected_user_message = clean_text(
        request.get("message").and_then(Value::as_str).unwrap_or(""),
        4_000,
    );
    let timeout_recovery = dashboard_state_root().map(|root| {
        (
            root.clone(),
            session_snapshot_from_state_root(&root, agent_id),
        )
    });
    let response = post_json(base_url, &path, request, timeout_seconds);
    if is_retryable_curl_timeout(&response) {
        if let Some((dashboard_state_root, baseline_snapshot)) = timeout_recovery.as_ref() {
            if let Some(recovered) = recover_timed_out_response_from_state(
                dashboard_state_root,
                agent_id,
                baseline_snapshot,
                &expected_user_message,
                timeout_recovery_seconds,
            ) {
                return recovered;
            }
        }
        return structured_timeout_failure_payload(response, timeout_recovery.is_some());
    }
    response
}

fn post_json(base_url: &str, path: &str, request: &Value, timeout_seconds: u64) -> Value {
    curl_json("POST", base_url, path, request, timeout_seconds)
}

fn is_retryable_curl_timeout(payload: &Value) -> bool {
    payload.get("transport_error").and_then(Value::as_str) == Some("curl_failed")
        && payload
            .get("stderr")
            .and_then(Value::as_str)
            .map(|stderr| stderr.to_ascii_lowercase().contains("timed out"))
            .unwrap_or(false)
}

pub(super) fn payload_is_transport_timeout_failure(payload: &Value) -> bool {
    payload
        .pointer("/response_finalization/structured_failure/kind")
        .and_then(Value::as_str)
        == Some("transport_timeout")
        || payload
            .pointer("/response_workflow/final_llm_response/status")
            .and_then(Value::as_str)
            == Some("transport_timeout")
        || is_retryable_curl_timeout(payload)
}

pub(super) fn payload_is_transport_failure(payload: &Value) -> bool {
    if payload_is_transport_timeout_failure(payload) {
        return true;
    }
    if payload
        .as_object()
        .map(|map| map.is_empty())
        .unwrap_or(false)
    {
        return true;
    }
    let transport_error = str_at(payload, &["transport_error"], "");
    if !transport_error.is_empty() {
        return true;
    }
    let error = normalize_for_compare(&str_at(payload, &["error"], ""));
    [
        "socket hang up",
        "connection reset",
        "connection refused",
        "failed to connect",
        "couldn't connect",
        "response_json_decode_failed",
        "curl_failed",
        "network error",
        "econnreset",
        "econnrefused",
    ]
    .iter()
    .any(|needle| error.contains(*needle))
}

fn structured_timeout_failure_payload(response: Value, recovery_ready: bool) -> Value {
    let mut payload = if response.is_object() {
        response
    } else {
        json!({
            "ok": false,
            "raw_transport_payload": response
        })
    };
    let response_text = "The live dashboard request timed out before the workflow produced a final answer. This is a transport failure, not a research result.";
    if let Some(object) = payload.as_object_mut() {
        object.insert("ok".to_string(), Value::Bool(false));
        object.insert(
            "response".to_string(),
            Value::String(response_text.to_string()),
        );
        object.insert("timeout_recovery_attempted".to_string(), Value::Bool(true));
        object.insert(
            "timeout_recovery_source".to_string(),
            Value::String("agent_session_state".to_string()),
        );
        object.insert(
            "timeout_recovery_ready".to_string(),
            Value::Bool(recovery_ready),
        );
        object.insert(
            "response_finalization".to_string(),
            json!({
                "outcome": "structured_failure+transport_timeout+timeout_recovery_failed",
                "structured_failure": {
                    "kind": "transport_timeout",
                    "reason": "live_dashboard_request_timed_out_before_final_answer",
                    "retryable": true,
                    "source": "eval_transport"
                }
            }),
        );
        object.insert(
            "response_workflow".to_string(),
            json!({
                "final_llm_response": {
                    "status": "transport_timeout",
                    "attempted": false,
                    "used": false
                }
            }),
        );
    }
    payload
}

fn dashboard_state_root() -> Option<PathBuf> {
    if let Ok(raw) = env::var(DASHBOARD_STATE_ROOT_ENV) {
        let candidate = PathBuf::from(raw.trim());
        if !candidate.as_os_str().is_empty() && candidate.exists() {
            return Some(candidate);
        }
    }
    let candidate = repo_root().join("client/runtime/local/state/ui/infring_dashboard");
    candidate.exists().then_some(candidate)
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")))
        .to_path_buf()
}

fn agent_sessions_dir(dashboard_state_root: &Path) -> PathBuf {
    if dashboard_state_root
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name == AGENT_SESSIONS_SUBDIR)
        .unwrap_or(false)
    {
        return dashboard_state_root.to_path_buf();
    }
    dashboard_state_root.join(AGENT_SESSIONS_SUBDIR)
}

fn session_path_from_state_root(dashboard_state_root: &Path, agent_id: &str) -> PathBuf {
    agent_sessions_dir(dashboard_state_root).join(format!("{}.json", normalize_agent_id(agent_id)))
}

fn session_messages_from_state_root(dashboard_state_root: &Path, agent_id: &str) -> Vec<Value> {
    let state = read_json_path(&session_path_from_state_root(
        dashboard_state_root,
        agent_id,
    ));
    let active_id = clean_text(
        state
            .get("active_session_id")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    );
    let sessions = state
        .get("sessions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let row = sessions
        .iter()
        .find(|session| {
            clean_text(
                session
                    .get("session_id")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                120,
            ) == active_id
        })
        .or_else(|| sessions.first());
    row.and_then(|session| session.get("messages"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn session_snapshot_from_state_root(
    dashboard_state_root: &Path,
    agent_id: &str,
) -> SessionSnapshot {
    let messages = session_messages_from_state_root(dashboard_state_root, agent_id);
    let assistant_messages = messages
        .iter()
        .filter(|row| message_role(row).eq_ignore_ascii_case("assistant"))
        .count();
    SessionSnapshot {
        total_messages: messages.len(),
        assistant_messages,
    }
}

fn recover_timed_out_response_from_state(
    dashboard_state_root: &Path,
    agent_id: &str,
    baseline_snapshot: &SessionSnapshot,
    expected_user_message: &str,
    recovery_budget_seconds: u64,
) -> Option<Value> {
    if recovery_budget_seconds == 0 {
        return None;
    }
    let deadline = Instant::now() + Duration::from_secs(recovery_budget_seconds);
    let mut latest_recovered = None;
    loop {
        if let Some(recovered) = recovered_payload_from_state(
            dashboard_state_root,
            agent_id,
            baseline_snapshot,
            expected_user_message,
        ) {
            if recovered_payload_has_structured_turn_artifacts(&recovered) {
                return Some(recovered);
            }
            latest_recovered = Some(recovered);
        }
        if Instant::now() >= deadline {
            return latest_recovered;
        }
        sleep(Duration::from_millis(1500));
    }
}

fn recovered_payload_has_structured_turn_artifacts(payload: &Value) -> bool {
    payload.get("pending_tool_request").is_some()
        || payload.get("response_workflow").is_some()
        || payload.get("response_finalization").is_some()
        || payload.get("process_summary").is_some()
        || payload.get("turn_transaction").is_some()
        || payload
            .get("tools")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().any(tool_row_has_substantive_artifact))
            .unwrap_or(false)
}

fn tool_row_has_substantive_artifact(row: &Value) -> bool {
    row.get("input").is_some()
        || row.get("result").is_some()
        || row.get("provider_results").is_some()
        || row.get("search_results").is_some()
        || row.get("evidence_refs").is_some()
        || row.get("tool_result_quality").is_some()
}

fn recovered_payload_from_state(
    dashboard_state_root: &Path,
    agent_id: &str,
    baseline_snapshot: &SessionSnapshot,
    expected_user_message: &str,
) -> Option<Value> {
    let messages = session_messages_from_state_root(dashboard_state_root, agent_id);
    if messages.len() <= baseline_snapshot.total_messages {
        return None;
    }
    let assistant_rows_seen = messages
        .iter()
        .filter(|row| message_role(row).eq_ignore_ascii_case("assistant"))
        .count();
    if assistant_rows_seen <= baseline_snapshot.assistant_messages {
        return None;
    }
    let assistant_row = messages
        .iter()
        .enumerate()
        .rev()
        .find(|(idx, row)| {
            *idx >= baseline_snapshot.total_messages
                && message_role(row).eq_ignore_ascii_case("assistant")
                && assistant_row_matches_expected_user_message(
                    &messages,
                    *idx,
                    baseline_snapshot.total_messages,
                    expected_user_message,
                )
                && !clean_text(
                    row.get("text")
                        .or_else(|| row.get("content"))
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                    64_000,
                )
                .is_empty()
        })
        .map(|(_, row)| row.clone())?;
    let mut payload = assistant_row_to_payload(&assistant_row)?;
    hydrate_recovered_payload_from_session_artifacts(dashboard_state_root, agent_id, &mut payload);
    Some(payload)
}

fn assistant_row_matches_expected_user_message(
    messages: &[Value],
    assistant_idx: usize,
    baseline_total_messages: usize,
    expected_user_message: &str,
) -> bool {
    let expected = clean_text(expected_user_message, 4_000);
    if expected.is_empty() {
        return true;
    }
    if assistant_idx == 0 {
        return false;
    }
    for idx in (baseline_total_messages..assistant_idx).rev() {
        let row = &messages[idx];
        if !message_role(row).eq_ignore_ascii_case("user") {
            continue;
        }
        let user_text = clean_text(
            row.get("text")
                .or_else(|| row.get("content"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            4_000,
        );
        return !user_text.is_empty()
            && normalize_for_compare(&user_text) == normalize_for_compare(&expected);
    }
    false
}

fn assistant_row_to_payload(row: &Value) -> Option<Value> {
    let response_text = row
        .get("text")
        .or_else(|| row.get("content"))
        .and_then(Value::as_str)
        .map(|raw| clean_text(raw, 64_000))
        .unwrap_or_default();
    let has_structured_artifacts = row.get("detail_refs").is_some()
        || row.get("tools").is_some()
        || row.get("response_workflow").is_some()
        || row.get("response_finalization").is_some()
        || row.get("process_summary").is_some()
        || row.get("workflow_visibility").is_some()
        || row.get("turn_transaction").is_some()
        || row.get("terminal_transcript").is_some();
    if response_text.is_empty() && !has_structured_artifacts {
        return None;
    }
    let mut payload = json!({
        "ok": true,
        "response": response_text,
        "text": response_text,
        "message": response_text,
        "recovered_from_timeout": true,
        "recovered_artifact_only_response": response_text.is_empty(),
        "recovery_source": "agent_session_state"
    });
    if let Some(object) = row.as_object() {
        for key in [
            "detail_refs",
            "tools",
            "response_workflow",
            "response_finalization",
            "process_summary",
            "workflow_visibility",
            "turn_transaction",
            "terminal_transcript",
            "agent_health_snapshot",
            "live_eval_monitor",
            "dashboard_health_indicator",
        ] {
            if let Some(value) = object.get(key) {
                payload[key] = value.clone();
            }
        }
    }
    if let Some(pending_request) = row
        .pointer("/response_workflow/pending_tool_request")
        .or_else(|| row.pointer("/response_workflow/manual_toolbox_pending_tool_request"))
        .or_else(|| row.pointer("/response_finalization/pending_tool_request"))
        .cloned()
    {
        payload["pending_tool_request"] = pending_request;
    }
    if let Some(provider) = row
        .pointer("/response_workflow/final_llm_response/provider")
        .and_then(Value::as_str)
    {
        payload["provider"] = Value::String(clean_text(provider, 160));
    }
    if let Some(model) = row
        .pointer("/response_workflow/final_llm_response/model")
        .and_then(Value::as_str)
    {
        payload["model"] = Value::String(clean_text(model, 240));
    }
    if let Some(runtime_model) = row
        .pointer("/response_workflow/final_llm_response/runtime_model")
        .and_then(Value::as_str)
    {
        payload["runtime_model"] = Value::String(clean_text(runtime_model, 240));
    }
    Some(payload)
}

fn hydrate_recovered_payload_from_session_artifacts(
    dashboard_state_root: &Path,
    agent_id: &str,
    payload: &mut Value,
) {
    if payload.get("response_finalization").is_none() {
        if let Some(finalization) = payload
            .pointer("/detail_refs/response_finalization/ref")
            .and_then(Value::as_str)
            .and_then(|detail_ref| {
                load_session_artifact_detail_ref(dashboard_state_root, agent_id, detail_ref)
            })
        {
            payload["response_finalization"] = finalization;
        }
    }
    if payload.get("response_workflow").is_none() {
        if let Some(workflow) = payload
            .pointer("/detail_refs/response_workflow/ref")
            .and_then(Value::as_str)
            .and_then(|detail_ref| {
                load_session_artifact_detail_ref(dashboard_state_root, agent_id, detail_ref)
            })
        {
            payload["response_workflow"] = workflow;
        }
    }
    let Some(tool_rows) = payload.get("tools").and_then(Value::as_array).cloned() else {
        return;
    };
    let mut hydrated_tools = Vec::<Value>::new();
    let mut loaded_any = false;
    for row in tool_rows {
        if let Some(artifact) =
            row.get("detail_ref")
                .and_then(Value::as_str)
                .and_then(|detail_ref| {
                    load_session_artifact_detail_ref(dashboard_state_root, agent_id, detail_ref)
                })
        {
            loaded_any = true;
            hydrated_tools.push(artifact);
        } else {
            hydrated_tools.push(row);
        }
    }
    if !loaded_any {
        return;
    }
    payload["tools"] = Value::Array(hydrated_tools.clone());
    payload["recovered_session_artifacts_hydrated"] = Value::Bool(true);

    if payload.get("pending_tool_request").is_none() {
        if let Some(pending_request) = pending_request_from_hydrated_tools(&hydrated_tools) {
            payload["pending_tool_request"] = pending_request;
        }
    }

    if payload.get("response_finalization").is_none() {
        payload["response_finalization"] = json!({
            "outcome": "recovered_from_session_artifacts",
            "tool_completion": {
                "tool_attempts": hydrated_tools
            }
        });
    }
    if let Some(finalized_text) = payload
        .pointer("/response_finalization/finalized_output")
        .or_else(|| payload.pointer("/response_finalization/final_output"))
        .or_else(|| payload.pointer("/response_finalization/final_response/text"))
        .or_else(|| payload.pointer("/response_workflow/final_llm_response/text"))
        .and_then(Value::as_str)
        .map(|raw| clean_text(raw, 64_000))
        .filter(|raw| !raw.is_empty())
    {
        payload["response"] = Value::String(finalized_text.clone());
        payload["text"] = Value::String(finalized_text.clone());
        payload["message"] = Value::String(finalized_text);
    }
}

fn load_session_artifact_detail_ref(
    dashboard_state_root: &Path,
    fallback_agent_id: &str,
    detail_ref: &str,
) -> Option<Value> {
    let mut parts = detail_ref.split(':');
    if parts.next()? != "session_artifact" {
        return None;
    }
    let raw_agent_id = parts.next().unwrap_or(fallback_agent_id);
    let raw_kind = parts.next()?;
    let raw_hash = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    let agent_id = safe_artifact_segment(raw_agent_id)?;
    let kind = safe_artifact_segment(raw_kind)?;
    let hash = safe_artifact_segment(raw_hash)?;
    let path = dashboard_state_root
        .join("session_artifacts")
        .join(agent_id)
        .join(format!("{kind}-{hash}.json"));
    let value = read_json_path(&path);
    value.as_object()?;
    Some(value)
}

fn safe_artifact_segment(raw: &str) -> Option<String> {
    let cleaned = raw.trim();
    if cleaned.is_empty()
        || !cleaned
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        return None;
    }
    Some(cleaned.to_string())
}

fn pending_request_from_hydrated_tools(tools: &[Value]) -> Option<Value> {
    let tool = tools
        .iter()
        .find(|row| row.get("input").is_some() || row.get("name").is_some())?;
    let tool_name = clean_text(
        tool.get("name")
            .or_else(|| tool.get("tool"))
            .and_then(Value::as_str)
            .unwrap_or("tool"),
        120,
    );
    let input = parse_tool_input_value(tool.get("input"))?;
    Some(json!({
        "status": "executed",
        "source": "timeout_recovery_session_artifact",
        "tool_name": tool_name,
        "tool_key": tool_name,
        "selected_tool_key": tool_name,
        "selected_tool_family": recovered_tool_family(&tool_name),
        "input": input
    }))
}

fn parse_tool_input_value(input: Option<&Value>) -> Option<Value> {
    match input? {
        Value::String(raw) => serde_json::from_str::<Value>(raw)
            .ok()
            .or_else(|| Some(json!({ "raw": clean_text(raw, 8_000) }))),
        Value::Object(_) => input.cloned(),
        _ => None,
    }
}

fn recovered_tool_family(tool_name: &str) -> &'static str {
    match tool_name {
        "batch_query" | "web_search" | "web_fetch" => "web_research",
        _ => "unknown",
    }
}

fn read_json_path(path: &Path) -> Value {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or_else(|| json!({}))
}

fn message_role(row: &Value) -> String {
    let raw =
        clean_text(row.get("role").and_then(Value::as_str).unwrap_or(""), 24).to_ascii_lowercase();
    if raw.is_empty() {
        "assistant".to_string()
    } else {
        raw
    }
}

fn curl_json(
    method: &str,
    base_url: &str,
    path: &str,
    request: &Value,
    timeout_seconds: u64,
) -> Value {
    let url = format!("{}{}", base_url.trim_end_matches('/'), path);
    let Ok(body) = serde_json::to_string(request) else {
        return json!({"ok": false, "transport_error": "request_json_encode_failed"});
    };
    let mut child = match Command::new("curl")
        .args([
            "-sS",
            "--max-time",
            &timeout_seconds.to_string(),
            "-H",
            "Content-Type: application/json",
            "-X",
            method,
            "--data-binary",
            "@-",
            &url,
        ])
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
        let _ = stdin.write_all(body.as_bytes());
    }
    match child.wait_with_output() {
        Ok(output) if output.status.success() => serde_json::from_slice(&output.stdout)
            .unwrap_or_else(
                |_| json!({"ok": false, "transport_error": "response_json_decode_failed"}),
            ),
        Ok(output) => json!({
            "ok": false,
            "transport_error": "curl_failed",
            "stderr": clean_text(&String::from_utf8_lossy(&output.stderr), 500),
        }),
        Err(err) => json!({"ok": false, "transport_error": format!("curl_wait_failed:{err}")}),
    }
}

pub(super) fn assistant_text(payload: &Value) -> String {
    for path in [
        &["response"][..],
        &["text"][..],
        &["message"][..],
        &["content"][..],
        &["assistant", "text"][..],
    ] {
        if let Some(raw) = str_opt(payload, path) {
            return clean_text(raw, 12_000);
        }
    }
    String::new()
}

pub(super) fn parse_flag(args: &[String], key: &str) -> Option<String> {
    let inline = format!("--{key}=");
    for (idx, arg) in args.iter().enumerate() {
        if let Some(value) = arg.strip_prefix(&inline) {
            return Some(value.to_string());
        }
        if arg == &format!("--{key}") {
            return args.get(idx + 1).cloned();
        }
    }
    None
}

pub(super) fn parse_bool_flag(args: &[String], key: &str, default: bool) -> bool {
    parse_flag(args, key)
        .map(|raw| matches!(raw.trim(), "1" | "true" | "TRUE" | "yes" | "on"))
        .unwrap_or(default)
}

pub(super) fn parse_u64_flag(args: &[String], key: &str, default: u64) -> u64 {
    parse_flag(args, key)
        .and_then(|raw| raw.parse::<u64>().ok())
        .unwrap_or(default)
}

pub(super) fn read_json(path: &str) -> Value {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or_else(|| json!({}))
}

pub(super) fn write_json(path: &str, value: &Value) -> io::Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, format!("{}\n", serde_json::to_string_pretty(value)?))
}

pub(super) fn write_text(path: &str, content: &str) -> io::Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)
}

pub(super) fn append_jsonl(path: &str, rows: &[Value]) -> io::Result<()> {
    if rows.is_empty() {
        return Ok(());
    }
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    for row in rows {
        writeln!(file, "{}", serde_json::to_string(row)?)?;
    }
    Ok(())
}

pub(super) fn str_opt<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    let mut cursor = value;
    for segment in path {
        cursor = cursor.get(*segment)?;
    }
    cursor.as_str().map(str::trim).filter(|raw| !raw.is_empty())
}

pub(super) fn str_at(value: &Value, path: &[&str], default: &str) -> String {
    str_opt(value, path).unwrap_or(default).to_string()
}

pub(super) fn string_array_at(value: &Value, path: &[&str]) -> Vec<String> {
    let mut cursor = value;
    for segment in path {
        let Some(next) = cursor.get(*segment) else {
            return Vec::new();
        };
        cursor = next;
    }
    cursor
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(|raw| clean_text(raw, 500))
        .collect()
}

pub(super) fn bool_at(value: &Value, path: &[&str], default: bool) -> bool {
    value
        .pointer(&format!("/{}", path.join("/")))
        .and_then(Value::as_bool)
        .unwrap_or(default)
}

pub(super) fn u64_at(value: &Value, path: &[&str], default: u64) -> u64 {
    value
        .pointer(&format!("/{}", path.join("/")))
        .and_then(Value::as_u64)
        .unwrap_or(default)
}

pub(super) fn f64_at(value: &Value, path: &[&str], default: f64) -> f64 {
    value
        .pointer(&format!("/{}", path.join("/")))
        .and_then(Value::as_f64)
        .unwrap_or(default)
}

pub(super) fn ratio(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

pub(super) fn clean_text(raw: &str, max_len: usize) -> String {
    raw.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max_len)
        .collect()
}

pub(super) fn normalize_for_compare(raw: &str) -> String {
    clean_text(&raw.to_ascii_lowercase(), 4_000)
}

pub(super) fn normalize_agent_id(raw: &str) -> String {
    clean_text(raw, 160)
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-' || *ch == '_')
        .collect()
}

pub(super) fn is_local_dashboard_url(raw: &str) -> bool {
    let lower = raw.trim().to_ascii_lowercase();
    lower.starts_with("http://127.0.0.1")
        || lower.starts_with("http://localhost")
        || lower.starts_with("http://[::1]")
}

pub(super) fn now_iso_like() -> String {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("unix_ms:{ms}")
}

pub(super) fn print_json_line(value: &Value) {
    let _ = writeln!(
        io::stdout(),
        "{}",
        serde_json::to_string(value).unwrap_or_default()
    );
}

#[cfg(test)]
mod eval_research_golden_utils_tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_path(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "eval-research-golden-utils-{}-{}",
            name,
            now_iso_like()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("temp root");
        root
    }

    #[test]
    fn retryable_curl_timeout_matches_timeout_stderr() {
        let payload = json!({
            "ok": false,
            "transport_error": "curl_failed",
            "stderr": "curl: (28) Operation timed out after 45010 milliseconds with 0 bytes received"
        });
        assert!(is_retryable_curl_timeout(&payload));
    }

    #[test]
    fn retryable_curl_timeout_rejects_non_timeout_failures() {
        let payload = json!({
            "ok": false,
            "transport_error": "curl_failed",
            "stderr": "curl: (7) Failed to connect to 127.0.0.1 port 4173"
        });
        assert!(!is_retryable_curl_timeout(&payload));
    }

    #[test]
    fn timeout_failure_payload_is_structured_terminal_artifact() {
        let payload = structured_timeout_failure_payload(
            json!({
                "ok": false,
                "transport_error": "curl_failed",
                "stderr": "curl: (28) Operation timed out after 60004 milliseconds with 0 bytes received"
            }),
            true,
        );
        let response = payload
            .get("response")
            .and_then(Value::as_str)
            .expect("response");
        assert!(!response.trim().is_empty());
        assert_eq!(
            payload.get("transport_error").and_then(Value::as_str),
            Some("curl_failed")
        );
        assert_eq!(
            payload
                .get("timeout_recovery_attempted")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            payload
                .get("timeout_recovery_ready")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/outcome")
                .and_then(Value::as_str),
            Some("structured_failure+transport_timeout+timeout_recovery_failed")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/structured_failure/kind")
                .and_then(Value::as_str),
            Some("transport_timeout")
        );
        assert!(payload_is_transport_timeout_failure(&payload));
    }

    #[test]
    fn assistant_row_to_payload_recovers_workflow_metadata() {
        let row = json!({
            "role": "assistant",
            "text": "Recovered answer",
            "tools": [{"name": "web_search", "status": "ok"}],
            "response_workflow": {
                "final_llm_response": {
                    "status": "synthesized",
                    "provider": "ollama",
                    "model": "kimi-k2.6:cloud",
                    "runtime_model": "kimi-k2.6:cloud"
                },
                "pending_tool_request": {"status": "pending_confirmation", "tool_name": "web_search"}
            },
            "response_finalization": {
                "outcome": "workflow_authored+workflow:synthesized"
            },
            "process_summary": {"contract": "turn_process_summary_v1"},
            "workflow_visibility": {"current_stage_status": "synthesized"}
        });
        let payload = assistant_row_to_payload(&row).expect("payload");
        assert_eq!(
            payload.get("response").and_then(Value::as_str),
            Some("Recovered answer")
        );
        assert_eq!(
            payload
                .pointer("/response_workflow/final_llm_response/status")
                .and_then(Value::as_str),
            Some("synthesized")
        );
        assert_eq!(
            payload.get("runtime_model").and_then(Value::as_str),
            Some("kimi-k2.6:cloud")
        );
        assert_eq!(
            payload
                .pointer("/pending_tool_request/tool_name")
                .and_then(Value::as_str),
            Some("web_search")
        );
        assert_eq!(
            payload
                .get("recovered_from_timeout")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn assistant_row_to_payload_recovers_artifact_only_turns() {
        let row = json!({
            "role": "assistant",
            "text": "",
            "tools": [{
                "name": "batch_query",
                "status": "ok",
                "input": {"query": "robot vacuums"}
            }],
            "turn_transaction": {"id": "turn-artifact-only"},
            "detail_refs": {
                "response_workflow": {
                    "ref": "session_artifact:agent:response_workflow:abc"
                }
            }
        });
        let payload = assistant_row_to_payload(&row).expect("artifact-only payload");
        assert_eq!(payload.get("response").and_then(Value::as_str), Some(""));
        assert_eq!(
            payload
                .get("recovered_artifact_only_response")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert!(recovered_payload_has_structured_turn_artifacts(&payload));
    }

    #[test]
    fn timeout_recovery_waits_for_structured_turn_artifacts_when_available() {
        let text_only = assistant_row_to_payload(&json!({
            "role": "assistant",
            "text": "Recovered answer"
        }))
        .expect("text-only payload");
        assert!(!recovered_payload_has_structured_turn_artifacts(&text_only));

        let with_tools = assistant_row_to_payload(&json!({
            "role": "assistant",
            "text": "Recovered answer",
            "tools": [{"name": "batch_query", "status": "done", "input": {"query": "q"}}],
            "turn_transaction": {"id": "turn-1"}
        }))
        .expect("structured payload");
        assert!(recovered_payload_has_structured_turn_artifacts(&with_tools));
    }

    #[test]
    fn recovered_payload_uses_first_new_assistant_turn_after_baseline() {
        let root = temp_path("session-recovery");
        let sessions_dir = root.join(AGENT_SESSIONS_SUBDIR);
        fs::create_dir_all(&sessions_dir).expect("sessions dir");
        let agent_id = "agent-recovery";
        let session = json!({
            "agent_id": agent_id,
            "active_session_id": "default",
            "sessions": [{
                "session_id": "default",
                "label": "Session",
                "created_at": "2026-05-08T00:00:00Z",
                "updated_at": "2026-05-08T00:00:00Z",
                "messages": [
                    {"role": "user", "text": "old question"},
                    {"role": "assistant", "text": "old answer", "response_workflow": {"final_llm_response": {"status": "synthesized"}}},
                    {"role": "user", "text": "new question"},
                    {"role": "assistant", "text": "new answer", "response_workflow": {"final_llm_response": {"status": "synthesized", "model": "kimi-k2.6:cloud"}}}
                ]
            }],
            "memory_kv": {}
        });
        fs::write(
            sessions_dir.join(format!("{}.json", agent_id)),
            format!(
                "{}\n",
                serde_json::to_string_pretty(&session).expect("json")
            ),
        )
        .expect("session write");
        let baseline = SessionSnapshot {
            total_messages: 2,
            assistant_messages: 1,
        };
        let payload = recovered_payload_from_state(&root, agent_id, &baseline, "new question")
            .expect("recovered");
        assert_eq!(
            payload.get("response").and_then(Value::as_str),
            Some("new answer")
        );
        assert_eq!(
            payload.get("model").and_then(Value::as_str),
            Some("kimi-k2.6:cloud")
        );
    }

    #[test]
    fn recovered_payload_hydrates_session_artifact_tool_details() {
        let root = temp_path("session-artifact-recovery");
        let sessions_dir = root.join(AGENT_SESSIONS_SUBDIR);
        let artifacts_dir = root.join("session_artifacts").join("agent-recovery");
        fs::create_dir_all(&sessions_dir).expect("sessions dir");
        fs::create_dir_all(&artifacts_dir).expect("artifacts dir");
        let agent_id = "agent-recovery";
        fs::write(
            artifacts_dir.join("tool_0-abc123.json"),
            serde_json::to_string_pretty(&json!({
                "name": "batch_query",
                "status": "done",
                "input": "{\"query\":\"Infring LangGraph comparison\",\"aperture\":\"medium\",\"source\":\"web\",\"keywords\":[\"Infring\",\"LangGraph\"],\"required_coverage\":{\"entities\":[\"Infring\",\"LangGraph\"]}}",
                "provider_results": [{"title": "Result", "snippet": "Useful snippet"}],
                "evidence_refs": [{"locator": "https://example.com"}],
                "tool_result_quality": {"status": "usable", "claim_hint_count": 1}
            }))
            .expect("artifact json"),
        )
        .expect("artifact write");
        fs::write(
            artifacts_dir.join("response_finalization-def456.json"),
            serde_json::to_string_pretty(&json!({
                "outcome": "finalized",
                "citations": [{"locator": "https://example.com"}],
                "finalized_output": "Recovered final answer with source signal from example.com."
            }))
            .expect("finalization json"),
        )
        .expect("finalization write");
        fs::write(
            artifacts_dir.join("response_workflow-ghi789.json"),
            serde_json::to_string_pretty(&json!({
                "final_llm_response": {
                    "text": "Recovered workflow answer with source signal from example.com.",
                    "citations": [{"locator": "https://example.com"}]
                }
            }))
            .expect("workflow json"),
        )
        .expect("workflow write");
        let session = json!({
            "agent_id": agent_id,
            "active_session_id": "default",
            "sessions": [{
                "session_id": "default",
                "label": "Session",
                "created_at": "2026-05-08T00:00:00Z",
                "updated_at": "2026-05-08T00:00:00Z",
                "messages": [
                    {"role": "user", "text": "old question"},
                    {"role": "assistant", "text": "old answer"},
                    {"role": "user", "text": "new question"},
                    {
                        "role": "assistant",
                        "text": "new answer",
                        "detail_refs": {
                            "response_finalization": {
                                "ref": "session_artifact:agent-recovery:response_finalization:def456"
                            },
                            "response_workflow": {
                                "ref": "session_artifact:agent-recovery:response_workflow:ghi789"
                            }
                        },
                        "tools": [{
                            "name": "batch_query",
                            "status": "done",
                            "detail_ref": "session_artifact:agent-recovery:tool_0:abc123"
                        }]
                    }
                ]
            }],
            "memory_kv": {}
        });
        fs::write(
            sessions_dir.join(format!("{}.json", agent_id)),
            serde_json::to_string_pretty(&session).expect("session json"),
        )
        .expect("session write");
        let baseline = SessionSnapshot {
            total_messages: 2,
            assistant_messages: 1,
        };
        let payload = recovered_payload_from_state(&root, agent_id, &baseline, "new question")
            .expect("recovered");
        assert_eq!(
            payload
                .pointer("/pending_tool_request/input/query")
                .and_then(Value::as_str),
            Some("Infring LangGraph comparison")
        );
        assert_eq!(
            payload
                .pointer("/tools/0/tool_result_quality/status")
                .and_then(Value::as_str),
            Some("usable")
        );
        assert_eq!(
            payload
                .get("recovered_session_artifacts_hydrated")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/citations/0/locator")
                .and_then(Value::as_str),
            Some("https://example.com")
        );
        assert_eq!(
            payload
                .pointer("/response_workflow/final_llm_response/citations/0/locator")
                .and_then(Value::as_str),
            Some("https://example.com")
        );
        assert_eq!(
            payload.get("response").and_then(Value::as_str),
            Some("Recovered final answer with source signal from example.com.")
        );
        assert!(recovered_payload_has_structured_turn_artifacts(&payload));
    }

    #[test]
    fn recovered_payload_ignores_late_assistant_row_for_previous_user_message() {
        let root = temp_path("session-recovery-message-match");
        let sessions_dir = root.join(AGENT_SESSIONS_SUBDIR);
        fs::create_dir_all(&sessions_dir).expect("sessions dir");
        let agent_id = "agent-recovery";
        let session = json!({
            "agent_id": agent_id,
            "active_session_id": "default",
            "sessions": [{
                "session_id": "default",
                "label": "Session",
                "created_at": "2026-05-08T00:00:00Z",
                "updated_at": "2026-05-08T00:00:00Z",
                "messages": [
                    {"role": "user", "text": "old question"},
                    {"role": "assistant", "text": "old answer"},
                    {"role": "assistant", "text": "late old answer", "response_workflow": {"final_llm_response": {"status": "synthesized"}}},
                    {"role": "user", "text": "new question"},
                    {"role": "assistant", "text": "new answer", "response_workflow": {"final_llm_response": {"status": "synthesized"}}}
                ]
            }],
            "memory_kv": {}
        });
        fs::write(
            sessions_dir.join(format!("{}.json", agent_id)),
            serde_json::to_string_pretty(&session).expect("session json"),
        )
        .expect("session write");
        let baseline = SessionSnapshot {
            total_messages: 2,
            assistant_messages: 1,
        };

        let payload = recovered_payload_from_state(&root, agent_id, &baseline, "new question")
            .expect("recovered");
        assert_eq!(
            payload.get("response").and_then(Value::as_str),
            Some("new answer")
        );
    }

    #[test]
    fn resolve_live_agent_model_ref_prefers_available_non_auto_provider_match() {
        let catalog = json!({
            "models": [
                {
                    "id": "auto/kimi-k2.6:cloud",
                    "provider": "auto",
                    "model": "kimi-k2.6:cloud",
                    "available": false
                },
                {
                    "id": "ollama/kimi-k2.6:cloud",
                    "provider": "ollama",
                    "model": "kimi-k2.6:cloud",
                    "available": true
                }
            ]
        });
        let resolved = resolve_live_agent_model_ref_from_catalog("kimi-k2.6:cloud", &catalog);
        assert_eq!(resolved, "ollama/kimi-k2.6:cloud");
    }

    #[test]
    fn resolve_live_agent_model_ref_keeps_explicit_provider_ref() {
        assert_eq!(
            resolve_live_agent_model_ref_from_catalog(
                "ollama/kimi-k2.6:cloud",
                &json!({"models": []})
            ),
            "ollama/kimi-k2.6:cloud"
        );
    }

    #[test]
    fn live_agent_row_active_accepts_active_and_running_states() {
        assert!(live_agent_row_active(&json!({
            "state": "Running",
            "contract": { "status": "active" }
        })));
    }

    #[test]
    fn live_agent_row_active_rejects_terminated_state() {
        assert!(!live_agent_row_active(&json!({
            "state": "Running",
            "contract": { "status": "terminated" }
        })));
    }

    #[test]
    fn live_agent_registry_rows_supports_array_and_object_shapes() {
        let array_shape = json!([{ "agent_id": "a" }]);
        let object_shape = json!({ "agents": [{ "agent_id": "b" }] });
        assert_eq!(live_agent_registry_rows(&array_shape).len(), 1);
        assert_eq!(live_agent_registry_rows(&object_shape).len(), 1);
    }
}
