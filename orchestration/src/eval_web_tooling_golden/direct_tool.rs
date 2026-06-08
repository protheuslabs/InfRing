use serde_json::{json, Value};
use std::io::Write;
use std::process::{Command, Stdio};

use super::super::eval_research_golden_utils::{clean_text, normalize_for_compare, str_at};

pub(super) fn invoke_direct_tool(
    base_url: &str,
    tool_name: &str,
    request: &Value,
    timeout_seconds: u64,
) -> Value {
    let path = match tool_name {
        "web_fetch" => "/api/web/fetch",
        "web_search" => "/api/web/search",
        _ => "/api/batch-query",
    };
    curl_json("POST", base_url, path, request, timeout_seconds)
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
            return json!({"ok": false, "transport_error": format!("curl_spawn_failed:{err}")});
        }
    };
    if let Some(stdin) = child.stdin.as_mut() {
        let _ = stdin.write_all(body.as_bytes());
    }
    match child.wait_with_output() {
        Ok(output) if output.status.success() => serde_json::from_slice(&output.stdout)
            .unwrap_or_else(|_| {
                json!({
                    "ok": false,
                    "transport_error": "response_json_decode_failed",
                    "stdout": clean_text(&String::from_utf8_lossy(&output.stdout), 4_000)
                })
            }),
        Ok(output) => json!({
            "ok": false,
            "transport_error": "curl_failed",
            "stderr": clean_text(&String::from_utf8_lossy(&output.stderr), 4_000),
            "stdout": clean_text(&String::from_utf8_lossy(&output.stdout), 4_000),
        }),
        Err(err) => json!({"ok": false, "transport_error": format!("curl_wait_failed:{err}")}),
    }
}

pub(super) fn payload_is_transport_failure(payload: &Value) -> bool {
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
        "timed out",
    ]
    .iter()
    .any(|needle| error.contains(*needle))
        || payload
            .get("stderr")
            .and_then(Value::as_str)
            .map(|stderr| normalize_for_compare(stderr).contains("timed out"))
            .unwrap_or(false)
}

pub(super) fn direct_tool_status(tool_name: &str, direct_payload: &Value) -> &'static str {
    if direct_payload.get("status").and_then(Value::as_str) == Some("blocked") {
        return "blocked";
    }
    if payload_is_transport_failure(direct_payload) {
        return "failed";
    }
    if tool_name == "batch_query"
        && direct_payload
            .get("status")
            .and_then(Value::as_str)
            .map(|raw| raw == "ok" || raw == "success" || raw == "done")
            .unwrap_or(false)
    {
        return "ok";
    }
    if direct_payload
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(true)
    {
        "ok"
    } else {
        "failed"
    }
}

pub(super) fn direct_tool_payload_diagnostics(payload: &Value) -> Value {
    json!({
        "top_keys": payload
            .as_object()
            .map(|obj| obj.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default(),
        "status": payload.get("status").and_then(Value::as_str),
        "ok": payload.get("ok").and_then(Value::as_bool),
        "error": payload.get("error").and_then(Value::as_str),
        "transport_error": payload.get("transport_error").and_then(Value::as_str),
        "stderr": payload.get("stderr").and_then(Value::as_str).map(|raw| clean_text(raw, 500)),
    })
}

pub(super) fn direct_tool_payload_sample(payload: &Value) -> Value {
    json!({
        "status": payload.get("status").and_then(Value::as_str),
        "ok": payload.get("ok").and_then(Value::as_bool),
        "error": payload.get("error").and_then(Value::as_str).map(|raw| clean_text(raw, 240)),
        "transport_error": payload.get("transport_error").and_then(Value::as_str),
        "stderr": payload.get("stderr").and_then(Value::as_str).map(|raw| clean_text(raw, 500)),
        "query": payload.get("query").and_then(Value::as_str).map(|raw| clean_text(raw, 500)),
        "effective_query": payload.get("effective_query").and_then(Value::as_str).map(|raw| clean_text(raw, 500)),
        "cache_status": payload.get("cache_status").and_then(Value::as_str),
        "cache_mode": payload.get("cache_mode").and_then(Value::as_str),
        "provider": payload.get("provider").and_then(Value::as_str).map(|raw| clean_text(raw, 120)),
        "provider_raw_count": payload.get("provider_raw_count").and_then(Value::as_u64),
        "provider_filtered_count": payload.get("provider_filtered_count").and_then(Value::as_u64),
        "query_plan_source": payload.get("query_plan_source").and_then(Value::as_str),
        "query_metadata": compact_value(payload.get("query_metadata"), 2_000),
        "query_plan": compact_array(payload.get("query_plan"), 12),
        "submitted_query_plan": compact_array(payload.get("submitted_query_plan"), 12),
        "query_execution_limiter": compact_value(payload.get("query_execution_limiter"), 2_000),
        "second_pass_recovery": compact_value(payload.get("second_pass_recovery"), 2_000),
        "evidence_selection_diagnostics": compact_evidence_selection_diagnostics(
            payload.get("evidence_selection_diagnostics")
        ),
        "providers_attempted": compact_array(payload.get("providers_attempted"), 8),
        "providers_skipped": compact_array(payload.get("providers_skipped"), 8),
        "provider_chain": compact_array(payload.get("provider_chain"), 8),
        "provider_errors": compact_array(payload.get("provider_errors"), 8),
        "links": compact_array(payload.get("links"), 8),
        "content_domains": compact_array(payload.get("content_domains"), 8),
        "evidence_refs": compact_rows(payload.get("evidence_refs"), 5),
        "evidence_claims": compact_rows(payload.get("evidence_claims"), 8),
        "evidence_coverage": compact_rows(payload.get("evidence_coverage"), 8),
        "provider_results": compact_rows(payload.get("provider_results"), 5),
        "search_results": compact_rows(payload.get("search_results"), 5),
        "evidence_pack": compact_rows(payload.get("evidence_pack"), 5),
        "evidence_pack_candidates": compact_rows(payload.get("evidence_pack_candidates"), 5),
        "evidence_pack_quality": compact_value(payload.get("evidence_pack_quality"), 2_000),
        "tool_result_quality": compact_value(payload.get("tool_result_quality"), 3_000),
        "summary": payload.get("summary").and_then(Value::as_str).map(|raw| clean_text(raw, 1_200)),
        "content_preview": payload.get("content").and_then(Value::as_str).map(|raw| clean_text(raw, 1_200)),
        "result_preview": payload.get("result").and_then(Value::as_str).map(|raw| clean_text(raw, 1_200)),
    })
}

fn compact_array(value: Option<&Value>, limit: usize) -> Value {
    let rows = value
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .take(limit)
                .map(|row| compact_value(Some(row), 800))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Value::Array(rows)
}

fn compact_rows(value: Option<&Value>, limit: usize) -> Value {
    let rows = value
        .and_then(Value::as_array)
        .map(|rows| rows.iter().take(limit).map(compact_row).collect::<Vec<_>>())
        .unwrap_or_default();
    Value::Array(rows)
}

fn compact_evidence_selection_diagnostics(value: Option<&Value>) -> Value {
    let Some(Value::Object(map)) = value else {
        return Value::Null;
    };
    let mut out = serde_json::Map::new();
    for key in [
        "version",
        "query",
        "ranked_pool_count",
        "actionable_pool_count",
        "selected_count",
    ] {
        if let Some(value) = map.get(key) {
            out.insert(key.to_string(), compact_value(Some(value), 1_200));
        }
    }
    let rows = map
        .get("rows")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .take(8)
                .map(compact_evidence_selection_row)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    out.insert("rows".to_string(), Value::Array(rows));
    Value::Object(out)
}

fn compact_evidence_selection_row(row: &Value) -> Value {
    let Value::Object(map) = row else {
        return compact_value(Some(row), 1_200);
    };
    let mut out = serde_json::Map::new();
    for key in [
        "title",
        "locator",
        "source_domain",
        "source_kind",
        "score",
        "selected",
        "in_actionable_pool",
        "pack_ready",
        "counts_as_usable_evidence",
        "materialization_quality",
        "freshness",
        "content_rich",
        "query_overlap_count",
        "distinctive_query_overlap_count",
        "candidate_has_source_identity",
        "candidate_has_source_type",
        "source_identity_tokens",
        "source_identity_query_anchored",
        "descriptor_has_source_identity",
        "descriptor_has_official_source_terms",
        "claim_hint_count",
        "claim_hints",
        "quality_flags",
        "blockers",
        "snippet",
    ] {
        let Some(value) = map.get(key) else { continue };
        let compacted = compact_value(Some(value), 1_200);
        if !compacted.is_null()
            && compacted
                .as_str()
                .map(|raw| !raw.trim().is_empty())
                .unwrap_or(true)
        {
            out.insert(key.to_string(), compacted);
        }
    }
    Value::Object(out)
}

fn compact_row(row: &Value) -> Value {
    match row {
        Value::String(raw) => Value::String(clean_text(raw, 1_200)),
        Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for key in [
                "title",
                "url",
                "locator",
                "source_url",
                "domain",
                "source_domain",
                "provider",
                "source_kind",
                "source_class",
                "source_type",
                "status",
                "error",
                "facet_id",
                "facet_kind",
                "requested_text",
                "evidence_count",
                "usable_evidence_count",
                "candidate_only_count",
                "low_confidence_raw_count",
                "score",
                "confidence",
                "counts_as_usable_evidence",
                "materialization_quality",
                "query_relevance",
                "snippet",
                "support_snippet",
                "snippet_preview",
                "summary",
                "content_preview",
                "extract",
                "relevant_extract",
                "why_relevant_to_query",
                "coverage_facets",
                "claim",
                "claim_text",
                "claim_hints",
                "links",
            ] {
                let Some(value) = map.get(key) else { continue };
                let compacted =
                    compact_value(Some(value), if key == "links" { 2_000 } else { 1_200 });
                if !compacted.is_null()
                    && compacted
                        .as_str()
                        .map(|raw| !raw.trim().is_empty())
                        .unwrap_or(true)
                {
                    out.insert(key.to_string(), compacted);
                }
            }
            Value::Object(out)
        }
        _ => compact_value(Some(row), 1_200),
    }
}

fn compact_value(value: Option<&Value>, max_chars: usize) -> Value {
    match value {
        Some(Value::String(raw)) => Value::String(clean_text(raw, max_chars)),
        Some(Value::Array(rows)) => Value::Array(
            rows.iter()
                .take(8)
                .map(|row| compact_value(Some(row), max_chars / 2))
                .collect(),
        ),
        Some(Value::Object(map)) => {
            let mut out = serde_json::Map::new();
            for (key, value) in map.iter().take(24) {
                out.insert(key.clone(), compact_value(Some(value), max_chars / 2));
            }
            Value::Object(out)
        }
        Some(value @ Value::Bool(_)) | Some(value @ Value::Number(_)) => value.clone(),
        _ => Value::Null,
    }
}

pub(super) fn is_local_dashboard_url(base_url: &str) -> bool {
    let lowered = base_url.trim().to_ascii_lowercase();
    lowered.starts_with("http://127.0.0.1")
        || lowered.starts_with("http://localhost")
        || lowered.starts_with("http://[::1]")
}
