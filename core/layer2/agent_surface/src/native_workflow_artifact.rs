// Layer ownership: Core Layer 2 (Scheduling + Execution) - native workflow artifact synthesis.
use crate::native_evidence::{
    native_tool_changed_paths, native_tool_has_successful_mutation,
    native_tool_has_successful_validation_command, native_tool_is_handoff_artifact_path,
    native_tool_product_slice_ready, native_tool_prompt_checkpoint_name,
    native_tool_prompt_evidence_gaps, native_tool_prompt_expected_memory_row_id,
    native_tool_prompt_memory_cli_pattern, native_tool_prompt_next_checkpoint_name,
    native_tool_prompt_project_root, native_tool_successful_validation_summary,
};
use crate::native_tools::{NativeToolCall, NativeToolDispatcher, NativeToolReceipt};
use serde_json::{json, Map, Value};

pub(crate) fn native_tool_auto_workflow_artifact_receipts(
    dispatcher: &NativeToolDispatcher,
    metadata: &Value,
    original_prompt: &str,
    receipts: &[NativeToolReceipt],
) -> Vec<NativeToolReceipt> {
    if !native_tool_has_successful_mutation(receipts)
        || !native_tool_has_successful_validation_command(receipts)
        || !native_tool_product_slice_ready(original_prompt, receipts)
    {
        return Vec::new();
    }
    let gaps = native_tool_prompt_evidence_gaps(original_prompt, receipts);
    if gaps.is_empty() {
        return Vec::new();
    }
    let changed_files = native_tool_changed_paths(receipts);
    let validation_results = native_tool_successful_validation_summary(receipts);
    let checkpoint = native_tool_prompt_checkpoint_name(original_prompt)
        .unwrap_or_else(|| "completed_checkpoint".to_string());
    let next_checkpoint = native_tool_prompt_next_checkpoint_name(original_prompt)
        .unwrap_or_else(|| "next_checkpoint_to_define".to_string());
    let known_risks = native_tool_workflow_artifact_known_risks(metadata);
    let schema_version = native_tool_workflow_artifact_schema_version(metadata);
    let redaction_policy = native_tool_workflow_artifact_redaction_policy(metadata);
    let completed_checkpoint_field =
        native_tool_workflow_artifact_completed_checkpoint_field(metadata);
    let alias_checkpoint_field = native_tool_workflow_artifact_alias_checkpoint_field(metadata);
    let next_checkpoint_field = native_tool_workflow_artifact_next_checkpoint_field(metadata);
    let mut payload_map = Map::new();
    payload_map.insert("schema_version".to_string(), json!(schema_version));
    payload_map.insert("status".to_string(), json!("completed"));
    payload_map.insert(completed_checkpoint_field, json!(checkpoint.clone()));
    payload_map.insert(alias_checkpoint_field, json!(checkpoint.clone()));
    payload_map.insert("changed_files".to_string(), json!(changed_files));
    payload_map.insert("validation_results".to_string(), validation_results.clone());
    payload_map.insert(
        "validation_result".to_string(),
        json!({
            "status": "pass",
            "summary": validation_results
        }),
    );
    payload_map.insert("known_risks".to_string(), json!(known_risks));
    payload_map.insert(next_checkpoint_field, json!(next_checkpoint));
    payload_map.insert("redaction_policy".to_string(), json!(redaction_policy));
    let payload = Value::Object(payload_map);
    let mut out = Vec::<NativeToolReceipt>::new();
    for reason in &gaps {
        let path = reason
            .strip_prefix("missing_changed_path:")
            .or_else(|| reason.strip_prefix("invalid_checkpoint_receipt:"));
        if let Some(path) = path {
            if native_tool_is_handoff_artifact_path(path) {
                out.push(dispatcher.dispatch(NativeToolCall {
                    id: "runtime_handoff_receipt_write".to_string(),
                    name: "file_write".to_string(),
                    args: json!({
                        "path": path,
                        "content": serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string()),
                        "overwrite": true
                    }),
                }));
            }
        }
    }
    if gaps
        .iter()
        .any(|reason| reason.starts_with("missing_memory_write_receipt"))
    {
        if let (Some(project_root), Some(expected_row_id)) = (
            native_tool_prompt_project_root(original_prompt),
            native_tool_prompt_expected_memory_row_id(original_prompt),
        ) {
            let Some(memory_command) = native_tool_workflow_artifact_memory_ingest_command(
                metadata,
                original_prompt,
                &expected_row_id,
                &payload,
            ) else {
                return out;
            };
            out.push(dispatcher.dispatch(NativeToolCall {
                id: "runtime_handoff_memory_ingest".to_string(),
                name: "command_run".to_string(),
                args: json!({
                    "cwd": project_root,
                    "cmd": memory_command,
                    "timeout_seconds": 120,
                    "max_output_bytes": 12000
                }),
            }));
        }
    }
    out
}

fn native_tool_workflow_artifact_policy(metadata: &Value) -> Option<&Value> {
    metadata
        .get("native_handoff_artifact_policy")
        .or_else(|| metadata.pointer("/workflow/native_handoff_artifact_policy"))
}

fn native_tool_workflow_artifact_schema_version(metadata: &Value) -> String {
    native_tool_workflow_artifact_policy(metadata)
        .and_then(|value| value.get("receipt_schema_version"))
        .and_then(Value::as_str)
        .unwrap_or("runtime_handoff_receipt_v1")
        .to_string()
}

fn native_tool_workflow_artifact_completed_checkpoint_field(metadata: &Value) -> String {
    native_tool_workflow_artifact_policy(metadata)
        .and_then(|value| value.get("completed_checkpoint_field"))
        .and_then(Value::as_str)
        .unwrap_or("completed_checkpoint")
        .to_string()
}

fn native_tool_workflow_artifact_alias_checkpoint_field(metadata: &Value) -> String {
    native_tool_workflow_artifact_policy(metadata)
        .and_then(|value| value.get("alias_checkpoint_field"))
        .and_then(Value::as_str)
        .unwrap_or("checkpoint")
        .to_string()
}

fn native_tool_workflow_artifact_next_checkpoint_field(metadata: &Value) -> String {
    native_tool_workflow_artifact_policy(metadata)
        .and_then(|value| value.get("next_checkpoint_field"))
        .and_then(Value::as_str)
        .unwrap_or("next_checkpoint")
        .to_string()
}

fn native_tool_workflow_artifact_redaction_policy(metadata: &Value) -> String {
    native_tool_workflow_artifact_policy(metadata)
        .and_then(|value| value.get("redaction_policy"))
        .and_then(Value::as_str)
        .unwrap_or("no_hidden_chain_of_thought")
        .to_string()
}

fn native_tool_workflow_artifact_known_risks(metadata: &Value) -> Vec<String> {
    native_tool_workflow_artifact_policy(metadata)
        .and_then(|value| value.get("synthesized_known_risks"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|items| !items.is_empty())
        .unwrap_or_else(|| {
            vec!["Runtime synthesized handoff bookkeeping from native receipts.".to_string()]
        })
}

pub(crate) fn native_tool_workflow_artifact_memory_tags(metadata: &Value) -> Vec<String> {
    native_tool_workflow_artifact_policy(metadata)
        .and_then(|value| value.get("memory_tags"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|items| !items.is_empty())
        .unwrap_or_else(|| vec!["project_context".to_string()])
}

fn native_tool_workflow_artifact_memory_ingest_command(
    metadata: &Value,
    original_prompt: &str,
    expected_row_id: &str,
    payload: &Value,
) -> Option<Vec<String>> {
    let pattern = native_tool_prompt_memory_cli_pattern(original_prompt)?;
    let tags = native_tool_workflow_artifact_memory_tags(metadata).join(",");
    let payload_text = serde_json::to_string(payload).ok()?;
    let command = format!(
        "ingest --id={} --content={} --tags={}",
        expected_row_id,
        shell_single_quote(&payload_text),
        shell_single_quote(&tags)
    );
    let command_line = if pattern.contains("<command>") {
        pattern.replace("<command>", &command)
    } else {
        format!("{pattern} {command}")
    };
    Some(vec!["sh".to_string(), "-c".to_string(), command_line])
}

fn shell_single_quote(raw: &str) -> String {
    format!("'{}'", raw.replace('\'', "'\\''"))
}
