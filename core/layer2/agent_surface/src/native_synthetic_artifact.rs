// Layer ownership: Core Layer 2 (Scheduling + Execution) - native synthetic artifact assembly.
use crate::native_evidence::{
    native_tool_changed_paths, native_tool_prompt_evidence_gaps, native_tool_requirement_lines,
    native_tool_successful_receipt_refs,
};
use crate::native_tools::NativeToolReceipt;
use crate::provider::ProviderResponse;
use serde_json::{json, Map, Value};

fn native_tool_synthetic_artifact_policy(metadata: &Value) -> Option<&Value> {
    metadata
        .get("native_synthetic_final_artifact_policy")
        .or_else(|| metadata.pointer("/workflow/native_synthetic_final_artifact_policy"))
}

fn native_tool_synthetic_artifact_string(metadata: &Value, key: &str, fallback: &str) -> String {
    native_tool_synthetic_artifact_policy(metadata)
        .and_then(|value| value.get(key))
        .and_then(Value::as_str)
        .unwrap_or(fallback)
        .to_string()
}

fn native_tool_synthetic_artifact_array(
    metadata: &Value,
    key: &str,
    fallback: &[&str],
) -> Vec<String> {
    native_tool_synthetic_artifact_policy(metadata)
        .and_then(|value| value.get(key))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|items| !items.is_empty())
        .unwrap_or_else(|| fallback.iter().map(|item| item.to_string()).collect())
}

pub(crate) fn native_tool_synthetic_micro_final_response(
    previous_response: &ProviderResponse,
    metadata: &Value,
    original_prompt: &str,
    receipts: &[NativeToolReceipt],
) -> ProviderResponse {
    let changed_paths = native_tool_changed_paths(receipts);
    let receipt_refs = receipts
        .iter()
        .filter(|receipt| {
            receipt.status == "ok"
                && matches!(receipt.tool_name.as_str(), "file_write" | "file_patch")
        })
        .map(|receipt| {
            let path = receipt
                .result
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or("");
            format!("{}:{}:{}", receipt.call_id, receipt.tool_name, path)
        })
        .collect::<Vec<_>>();

    let workflow_id =
        native_tool_synthetic_artifact_string(metadata, "workflow_id", "native_tool_runtime");
    let heading =
        native_tool_synthetic_artifact_string(metadata, "micro_heading", "Task completed.");
    let status_field =
        native_tool_synthetic_artifact_string(metadata, "completion_status_field", "status");
    let changed_files_field =
        native_tool_synthetic_artifact_string(metadata, "changed_files_field", "changed_files");
    let validation_field =
        native_tool_synthetic_artifact_string(metadata, "validation_summary_field", "validation");
    let checkpoint_field =
        native_tool_synthetic_artifact_string(metadata, "checkpoint_field", "checkpoint");
    let trace_field =
        native_tool_synthetic_artifact_string(metadata, "public_trace_field", "trace");
    let trace_protocol =
        native_tool_synthetic_artifact_string(metadata, "public_trace_protocol", "trace_v1");
    let rollup_field = native_tool_synthetic_artifact_string(metadata, "rollup_field", "rollup");
    let rollup_protocol =
        native_tool_synthetic_artifact_string(metadata, "rollup_protocol", "rollup_v1");
    let child_refs_field = native_tool_synthetic_artifact_string(
        metadata,
        "child_trace_refs_field",
        "child_trace_refs",
    );
    let redaction_field =
        native_tool_synthetic_artifact_string(metadata, "redaction_field", "redaction");
    let redaction_value =
        native_tool_synthetic_artifact_string(metadata, "redaction_policy", "public_summary_only");
    let validation_note = native_tool_synthetic_artifact_string(
        metadata,
        "micro_validation_note",
        "Successful native mutation receipt observed.",
    );

    let mut validation = Map::new();
    validation.insert("status".to_string(), json!("receipt_verified"));
    validation.insert("note".to_string(), json!(validation_note.clone()));

    let mut checkpoint = Map::new();
    checkpoint.insert("kind".to_string(), json!("completed_checkpoint"));
    checkpoint.insert(
        "summary".to_string(),
        json!(original_prompt
            .lines()
            .next()
            .unwrap_or("direct write task")
            .chars()
            .take(160)
            .collect::<String>()),
    );

    let mut trace = Map::new();
    trace.insert("protocol".to_string(), json!(trace_protocol));
    trace.insert(
        "task_summary".to_string(),
        json!(native_tool_synthetic_artifact_string(
            metadata,
            "micro_task_summary",
            "Task completed with native file mutation."
        )),
    );
    trace.insert(
        "plan_summary".to_string(),
        json!(native_tool_synthetic_artifact_string(
            metadata,
            "micro_plan_summary",
            "Use the explicit target from the prompt and report receipt-backed completion."
        )),
    );
    trace.insert(
        "decisions".to_string(),
        json!(native_tool_synthetic_artifact_array(
            metadata,
            "micro_decisions",
            &["Selected the direct mutation path for the local task."]
        )),
    );
    trace.insert(
        "actions".to_string(),
        json!(native_tool_synthetic_artifact_array(
            metadata,
            "micro_actions",
            &["Executed native file mutation."]
        )),
    );
    trace.insert(changed_files_field.clone(), json!(changed_paths.clone()));
    trace.insert("validation_summary".to_string(), json!(validation_note));
    trace.insert("risks".to_string(), json!(Vec::<String>::new()));
    trace.insert("blockers".to_string(), json!(Vec::<String>::new()));
    trace.insert("confidence".to_string(), json!("high"));
    trace.insert("evidence_refs".to_string(), json!(receipt_refs.clone()));
    trace.insert("tool_receipt_refs".to_string(), json!(receipt_refs.clone()));
    trace.insert("child_trace_refs".to_string(), json!(Vec::<String>::new()));
    trace.insert(redaction_field.clone(), json!(redaction_value.clone()));

    let mut rollup = Map::new();
    rollup.insert("protocol".to_string(), json!(rollup_protocol));
    rollup.insert("status".to_string(), json!("complete"));
    rollup.insert(
        "summary".to_string(),
        json!(native_tool_synthetic_artifact_string(
            metadata,
            "micro_rollup_summary",
            "The requested local change was completed through native tooling."
        )),
    );
    rollup.insert(changed_files_field.clone(), json!(changed_paths.clone()));
    rollup.insert("evidence_refs".to_string(), json!(receipt_refs.clone()));
    rollup.insert("blockers".to_string(), json!(Vec::<String>::new()));
    rollup.insert(redaction_field.clone(), json!(redaction_value.clone()));

    let mut artifact = Map::new();
    artifact.insert("workflow_id".to_string(), json!(workflow_id));
    artifact.insert(status_field, json!("success"));
    artifact.insert(changed_files_field, json!(changed_paths));
    artifact.insert(validation_field, Value::Object(validation));
    artifact.insert(checkpoint_field, Value::Object(checkpoint));
    artifact.insert(trace_field, Value::Object(trace));
    artifact.insert(rollup_field, Value::Object(rollup));
    artifact.insert(child_refs_field, json!(Vec::<String>::new()));
    artifact.insert(redaction_field, json!(redaction_value));

    let output = format!(
        "{}\n\n```json\n{}\n```",
        heading,
        serde_json::to_string_pretty(&Value::Object(artifact)).unwrap_or_else(|_| "{}".to_string())
    );
    ProviderResponse {
        provider: previous_response.provider.clone(),
        model: previous_response.model.clone(),
        output,
        usage_tokens: previous_response.usage_tokens,
        raw: previous_response.raw.clone(),
    }
}

pub(crate) fn native_tool_synthetic_completion_evidence_response(
    previous_response: &ProviderResponse,
    metadata: &Value,
    original_prompt: &str,
    receipts: &[NativeToolReceipt],
    reason: &str,
) -> ProviderResponse {
    let changed_paths = native_tool_changed_paths(receipts);
    let receipt_refs = native_tool_successful_receipt_refs(receipts);
    let evidence_gaps = native_tool_prompt_evidence_gaps(original_prompt, receipts);
    let mut requirements = native_tool_requirement_lines(original_prompt);
    if requirements.is_empty() {
        requirements.push("Complete the requested task.".to_string());
    }
    let has_unresolved_gaps = changed_paths.is_empty() || !evidence_gaps.is_empty();
    let item_status = if has_unresolved_gaps {
        "blocked"
    } else {
        "covered"
    };
    let blockers = if has_unresolved_gaps {
        vec![format!(
            "{}; unresolved_evidence_gaps={}",
            reason,
            evidence_gaps.join(",")
        )]
    } else {
        Vec::<String>::new()
    };
    let checklist = requirements
        .iter()
        .enumerate()
        .map(|(idx, requirement)| {
            json!({
                "id": format!("requirement_{}", idx + 1),
                "requirement": requirement,
                "status": item_status,
                "evidence_refs": receipt_refs.clone(),
                "blocker_reason": if has_unresolved_gaps { Value::String(blockers.join("; ")) } else { Value::Null },
            })
        })
        .collect::<Vec<_>>();
    let completion_status = if has_unresolved_gaps {
        "partial_or_blocked"
    } else if reason.contains("timeout") {
        "success_receipt_backed_finalization_timeout"
    } else {
        "success_receipt_backed_runtime_synthesized"
    };

    let workflow_id =
        native_tool_synthetic_artifact_string(metadata, "workflow_id", "native_tool_runtime");
    let heading = native_tool_synthetic_artifact_string(
        metadata,
        "completion_heading",
        "Runtime synthesized finalization.",
    );
    let status_field =
        native_tool_synthetic_artifact_string(metadata, "completion_status_field", "status");
    let changed_files_field =
        native_tool_synthetic_artifact_string(metadata, "changed_files_field", "changed_files");
    let validation_field =
        native_tool_synthetic_artifact_string(metadata, "validation_summary_field", "validation");
    let checkpoint_field =
        native_tool_synthetic_artifact_string(metadata, "checkpoint_field", "checkpoint");
    let trace_field =
        native_tool_synthetic_artifact_string(metadata, "public_trace_field", "trace");
    let trace_protocol =
        native_tool_synthetic_artifact_string(metadata, "public_trace_protocol", "trace_v1");
    let rollup_field = native_tool_synthetic_artifact_string(metadata, "rollup_field", "rollup");
    let rollup_protocol =
        native_tool_synthetic_artifact_string(metadata, "rollup_protocol", "rollup_v1");
    let child_refs_field = native_tool_synthetic_artifact_string(
        metadata,
        "child_trace_refs_field",
        "child_trace_refs",
    );
    let checklist_field = native_tool_synthetic_artifact_string(
        metadata,
        "task_checklist_field",
        "requirement_checklist",
    );
    let redaction_field =
        native_tool_synthetic_artifact_string(metadata, "redaction_field", "redaction");
    let redaction_value =
        native_tool_synthetic_artifact_string(metadata, "redaction_policy", "public_summary_only");
    let validation_note = native_tool_synthetic_artifact_string(
        metadata,
        "completion_validation_note",
        "Receipt-backed runtime synthesis.",
    );

    let mut validation = Map::new();
    validation.insert("status".to_string(), json!("receipt_backed"));
    validation.insert("note".to_string(), json!(validation_note.clone()));
    validation.insert("synthesis_reason".to_string(), json!(reason));

    let mut checkpoint = Map::new();
    checkpoint.insert(
        "kind".to_string(),
        json!(if has_unresolved_gaps {
            "structured_blocker"
        } else {
            "completed_checkpoint"
        }),
    );
    checkpoint.insert("summary".to_string(), json!(reason));

    let mut trace = Map::new();
    trace.insert("protocol".to_string(), json!(trace_protocol));
    trace.insert(
        "task_summary".to_string(),
        json!(native_tool_synthetic_artifact_string(
            metadata,
            "completion_task_summary",
            "Task summarized from native receipts."
        )),
    );
    trace.insert(
        "plan_summary".to_string(),
        json!(native_tool_synthetic_artifact_string(
            metadata,
            "completion_plan_summary",
            "Map requested work to receipt-backed evidence."
        )),
    );
    trace.insert(
        "decisions".to_string(),
        json!(native_tool_synthetic_artifact_array(
            metadata,
            "completion_decisions",
            &["Used runtime synthesis because finalization evidence was missing."]
        )),
    );
    trace.insert(
        "actions".to_string(),
        json!(native_tool_synthetic_artifact_array(
            metadata,
            "completion_actions",
            &["Collected changed-file paths from successful receipts."]
        )),
    );
    trace.insert(changed_files_field.clone(), json!(changed_paths.clone()));
    trace.insert("validation_summary".to_string(), json!(validation_note));
    trace.insert(
        "risks".to_string(),
        json!(native_tool_synthetic_artifact_array(
            metadata,
            "completion_risks",
            &["Runtime synthesis is limited to available receipts."]
        )),
    );
    trace.insert("blockers".to_string(), json!(blockers.clone()));
    trace.insert(
        "confidence".to_string(),
        json!(if has_unresolved_gaps { "low" } else { "medium" }),
    );
    trace.insert("evidence_refs".to_string(), json!(receipt_refs.clone()));
    trace.insert("tool_receipt_refs".to_string(), json!(receipt_refs.clone()));
    trace.insert("child_trace_refs".to_string(), json!(Vec::<String>::new()));
    trace.insert(checklist_field.clone(), Value::Array(checklist.clone()));
    trace.insert(redaction_field.clone(), json!(redaction_value.clone()));

    let mut rollup = Map::new();
    rollup.insert("protocol".to_string(), json!(rollup_protocol));
    rollup.insert("status".to_string(), json!(completion_status));
    rollup.insert(
        "summary".to_string(),
        json!(native_tool_synthetic_artifact_string(
            metadata,
            "completion_rollup_summary",
            "Runtime produced a receipt-backed evidence map."
        )),
    );
    rollup.insert(changed_files_field.clone(), json!(changed_paths.clone()));
    rollup.insert("evidence_refs".to_string(), json!(receipt_refs));
    rollup.insert(checklist_field, Value::Array(checklist));
    rollup.insert("blockers".to_string(), json!(blockers));
    rollup.insert(redaction_field.clone(), json!(redaction_value.clone()));

    let mut artifact = Map::new();
    artifact.insert("workflow_id".to_string(), json!(workflow_id));
    artifact.insert(status_field, json!(completion_status));
    artifact.insert(changed_files_field, json!(changed_paths));
    artifact.insert(validation_field, Value::Object(validation));
    artifact.insert(checkpoint_field, Value::Object(checkpoint));
    artifact.insert(trace_field, Value::Object(trace));
    artifact.insert(rollup_field, Value::Object(rollup));
    artifact.insert(child_refs_field, json!(Vec::<String>::new()));
    artifact.insert(redaction_field, json!(redaction_value));

    let output = format!(
        "{}\n\n```json\n{}\n```",
        heading,
        serde_json::to_string_pretty(&Value::Object(artifact)).unwrap_or_else(|_| "{}".to_string())
    );
    ProviderResponse {
        provider: previous_response.provider.clone(),
        model: previous_response.model.clone(),
        output,
        usage_tokens: previous_response.usage_tokens,
        raw: previous_response.raw.clone(),
    }
}
