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
- Prefer file_patch when exact observed text is available.\n\
- Use file_write only when replacing a small full file is safer than patching.\n\
- Do not run validation or probes in this lane.\n\
- Do not call read/list/stat/command tools in this lane.\n\
- If mutation is unsafe from the loaded context, return {{\"tool_calls\":[]}}."
    )
}

fn first_mutation_artifact_lane_v1_context_packet(receipts: &[NativeToolReceipt]) -> String {
    let mut files = Vec::new();
    for receipt in receipts {
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
        "contract": {
            "context_already_loaded": true,
            "allowed_tools": ["file_patch", "file_write"],
            "forbidden_tools": ["file_list", "file_stat", "file_read", "file_read_many", "command_run"],
            "must_mutate_before_validation_or_final": true
        }
    })
    .to_string()
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
    if let Some(object) = metadata.as_object_mut() {
        object.insert("provider_timeout_seconds".to_string(), json!(45));
        object.insert("provider_stream_until_tool_calls".to_string(), json!(true));
        object.insert("omit_ollama_thinking_flags".to_string(), json!(true));
    }
    metadata
}
