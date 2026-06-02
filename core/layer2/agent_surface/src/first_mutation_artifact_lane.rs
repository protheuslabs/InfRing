// Layer ownership: Core Layer 2 / Orchestration boundary.
//
// First mutation artifact lane v1. This stage exists to replace the broad
// open native-tool loop for the first edit in low-complexity existing-project
// tasks. It only asks for mutation tool calls and owns no validation, repair,
// final response, memory, checkpoint, or eval-specific behavior.

use crate::native_tools::NativeToolReceipt;
use serde_json::{json, Value};

pub(crate) fn first_mutation_artifact_lane_v1_enabled(metadata: &Value) -> bool {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|criteria| criteria.get("first_mutation_artifact_lane_v1_enabled"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

pub(crate) fn first_mutation_artifact_lane_v1_routes_lane(lane: &str) -> bool {
    matches!(lane, "existing_project_patch")
}

pub(crate) fn first_mutation_artifact_lane_v1_tools(tools: &[String]) -> Vec<String> {
    tools
        .iter()
        .filter(|tool| {
            matches!(
                tool.trim().to_ascii_lowercase().as_str(),
                "file_write"
                    | "write_file"
                    | "workspace.write"
                    | "workspace_write"
                    | "file_patch"
                    | "patch_file"
                    | "workspace.patch"
                    | "workspace_patch"
            )
        })
        .cloned()
        .collect()
}

pub(crate) fn first_mutation_artifact_lane_v1_system() -> String {
    "VISIBLE OUTPUT CONTRACT: the first visible byte must be `{`. Return one JSON object only: {\"tool_calls\":[...]}. You are a constrained first-mutation engine. Allowed tools: file_patch or file_write only. No thinking, prose, markdown, reads, validation, command_run, final answer, or checkpoint artifacts. Preserve observed public API behavior unless the task explicitly asks to change it.".to_string()
}

pub(crate) fn first_mutation_artifact_lane_v1_prompt(
    original_prompt: &str,
    receipts: &[NativeToolReceipt],
) -> String {
    let context_packet = first_mutation_artifact_lane_v1_context_packet(receipts);
    format!(
        "Task:\n{original_prompt}\n\n\
Loaded edit context:\n{context_packet}\n\n\
Output contract:\n\
- First visible byte must be `{{`.\n\
- Return only one JSON object.\n\
- Do not write thoughts, explanations, markdown, or final answer text.\n\
- Required shape:\n\
{{\"tool_calls\":[{{\"id\":\"first_mutation_1\",\"name\":\"file_patch\",\"args\":{{\"path\":\"/absolute/path\",\"old\":\"exact observed text\",\"new\":\"replacement text\",\"allow_multiple\":false}}}}]}}\n\n\
Mutation rules:\n\
- Mutate the smallest observed source/test files needed for the requested local code change.\n\
- If failed_validation exists, treat it as the primary repair contract and patch observed product/source files before tests.\n\
- Prefer file_patch when exact observed text is available.\n\
- Use file_write only when replacing a small full file is safer than patching.\n\
- Do not run validation or probes in this lane.\n\
- Do not call read/list/stat/command tools in this lane.\n\
- If mutation is unsafe from the loaded context, return {{\"tool_calls\":[]}}."
    )
}

fn first_mutation_artifact_lane_v1_context_packet(receipts: &[NativeToolReceipt]) -> String {
    let mut files = Vec::new();
    let mut failed_validation = Vec::new();
    for receipt in receipts {
        if let Some(validation) = first_mutation_artifact_lane_v1_failed_validation_packet(receipt)
        {
            failed_validation.push(validation);
        }
        if receipt.tool_name != "file_read" && receipt.tool_name != "file_read_many" {
            continue;
        }
        if let Some(items) = receipt.result.get("files").and_then(Value::as_array) {
            for item in items {
                if let Some(file) = first_mutation_artifact_lane_v1_file_packet(item) {
                    files.push(file);
                }
            }
        } else if let Some(file) = first_mutation_artifact_lane_v1_file_packet(&receipt.result) {
            files.push(file);
        }
    }
    json!({
        "observed_files": files,
        "failed_validation": failed_validation,
        "contract": {
            "context_already_loaded": true,
            "allowed_tools": ["file_patch", "file_write"],
            "forbidden_tools": ["file_list", "file_stat", "file_read", "file_read_many", "command_run"],
            "must_mutate_before_validation_or_final": true
        }
    })
    .to_string()
}

fn first_mutation_artifact_lane_v1_failed_validation_packet(
    receipt: &NativeToolReceipt,
) -> Option<Value> {
    if receipt.tool_name != "command_run" {
        return None;
    }
    let success = receipt
        .result
        .get("success")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if success {
        return None;
    }
    let mut lines = Vec::new();
    if let Some(error) = receipt.error.as_deref() {
        first_mutation_artifact_lane_v1_extend_validation_lines(&mut lines, error);
    }
    for key in ["stdout", "stderr", "message", "summary", "diagnostic"] {
        if let Some(text) = receipt.result.get(key).and_then(Value::as_str) {
            first_mutation_artifact_lane_v1_extend_validation_lines(&mut lines, text);
        }
    }
    if lines.is_empty() {
        return None;
    }
    Some(json!({
        "call_id": receipt.call_id,
        "tool_name": receipt.tool_name,
        "command": receipt.result.get("command").cloned().unwrap_or(Value::Null),
        "evidence_lines": lines.into_iter().take(24).collect::<Vec<_>>()
    }))
}

fn first_mutation_artifact_lane_v1_extend_validation_lines(
    lines: &mut Vec<String>,
    text: &str,
) {
    for line in text.lines() {
        let trimmed = line.trim();
        if !first_mutation_artifact_lane_v1_validation_line_is_useful(trimmed) {
            continue;
        }
        let compact = trimmed.chars().take(240).collect::<String>();
        if !lines.iter().any(|existing| existing == &compact) {
            lines.push(compact);
        }
        if lines.len() >= 32 {
            break;
        }
    }
}

fn first_mutation_artifact_lane_v1_validation_line_is_useful(line: &str) -> bool {
    if line.is_empty() || line.len() > 500 {
        return false;
    }
    let lower = line.to_ascii_lowercase();
    lower.contains("fail")
        || lower.contains("assert")
        || lower.contains("expected")
        || lower.contains("actual")
        || lower.contains("traceback")
        || lower.contains("error")
        || lower.contains("exception")
        || lower.contains("import")
        || line.starts_with("- ")
        || line.starts_with("+ ")
}

fn first_mutation_artifact_lane_v1_file_packet(value: &Value) -> Option<Value> {
    let path = value.get("path").and_then(Value::as_str)?;
    let content = value.get("content").and_then(Value::as_str).unwrap_or("");
    Some(json!({
        "path": path,
        "content": content,
        "start_line": value.get("start_line").cloned().unwrap_or(Value::Null),
        "end_line": value.get("end_line").cloned().unwrap_or(Value::Null),
        "total_lines": value.get("total_lines").cloned().unwrap_or(Value::Null)
    }))
}

pub(crate) fn first_mutation_artifact_lane_v1_metadata(metadata: &Value) -> Value {
    let mut metadata = metadata.clone();
    let timeout_seconds = first_mutation_artifact_lane_v1_provider_timeout_seconds(&metadata);
    if let Some(object) = metadata.as_object_mut() {
        object.insert("provider_timeout_seconds".to_string(), json!(timeout_seconds));
        object.insert("provider_stream_until_tool_calls".to_string(), json!(true));
        object.insert("omit_ollama_thinking_flags".to_string(), json!(false));
    }
    metadata
}

fn first_mutation_artifact_lane_v1_provider_timeout_seconds(metadata: &Value) -> u64 {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|criteria| criteria.get("first_mutation_artifact_lane_v1_provider_timeout_seconds"))
        .and_then(Value::as_u64)
        .unwrap_or(15)
        .clamp(5, 60)
}
