// Layer ownership: Core Layer 2 / Orchestration boundary.
//
// Adapter from legacy native-tool receipts into coding_execution_spine_v1
// normalized evidence. This file must stay fixture-agnostic: no eval levels,
// benchmark names, or expected fixture symbols.

use crate::coding_execution_spine::{
    CodingExecutionSpine, CodingSpineDecision, CodingTaskContract, CodingTaskKind,
    ContextEvidence, MutationEvidence, PublicInterfaceEvidence, ValidationEvidence,
};
use crate::native_evidence::{
    native_tool_coding_task_lane, native_tool_prompt_requires_product_mutation,
    native_tool_prompt_requires_test_changes, native_tool_prompt_requires_validation_command,
};
use crate::native_tools::NativeToolReceipt;
use serde_json::Value;

pub(crate) fn coding_execution_spine_v1_enabled(metadata: &Value) -> bool {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|criteria| criteria.get("coding_execution_spine_v1_enabled"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

pub(crate) fn coding_execution_spine_v1_routes_lane(lane: &str) -> bool {
    matches!(
        lane,
        "new_file_fast_path"
            | "micro_direct_mutation"
            | "existing_project_patch"
            | "bounded_existing_project_edit"
            | "implementation_slice"
    )
}

pub(crate) fn coding_execution_spine_decision_from_native_receipts(
    metadata: &Value,
    original_prompt: &str,
    receipts: &[NativeToolReceipt],
) -> CodingSpineDecision {
    let contract = coding_execution_spine_contract(metadata, original_prompt);
    let mut spine = CodingExecutionSpine::new(contract);
    let mut latest_successful_mutation_index = None;

    for receipt in receipts {
        if let Some(context) = context_evidence_from_receipt(receipt) {
            spine.record_context(context);
        }
        if let Some(mutation) = mutation_evidence_from_receipt(receipt) {
            spine.record_mutation(mutation);
            if spine.mutations().last().map(|evidence| evidence.success).unwrap_or(false) {
                latest_successful_mutation_index = spine.mutations().len().checked_sub(1);
            }
        }
        if let Some(validation) =
            validation_evidence_from_receipt(receipt, latest_successful_mutation_index)
        {
            spine.record_validation(validation);
        }
        if let Some(public_interface) = public_interface_evidence_from_receipt(receipt) {
            spine.record_public_interface(public_interface);
        }
    }

    spine.decide()
}

fn coding_execution_spine_contract(
    metadata: &Value,
    original_prompt: &str,
) -> CodingTaskContract {
    let lane = native_tool_coding_task_lane(metadata, original_prompt);
    let prompt_lower = original_prompt.to_ascii_lowercase();
    let task_kind = match lane {
        "new_file_fast_path" | "micro_direct_mutation" => CodingTaskKind::CreateFile,
        "validation_repair" | "validated_repair_edit" => CodingTaskKind::DebugRepair,
        "multi_file_slice" | "project_slice" => CodingTaskKind::ProjectSlice,
        "existing_project_patch" | "bounded_existing_project_edit" | "implementation_slice" => {
            CodingTaskKind::ExistingProjectPatch
        }
        _ if native_tool_prompt_requires_product_mutation(&prompt_lower) => {
            CodingTaskKind::ExistingProjectPatch
        }
        _ => CodingTaskKind::ExplanationOnly,
    };
    let requires_mutation = metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|criteria| criteria.get("requires_successful_mutation_receipt"))
        .and_then(Value::as_bool)
        .unwrap_or_else(|| native_tool_prompt_requires_product_mutation(&prompt_lower));
    CodingTaskContract {
        task_id: "native_coding_runtime".to_string(),
        task_kind,
        requires_context: !matches!(lane, "new_file_fast_path" | "micro_direct_mutation"),
        requires_mutation,
        requires_validation: native_tool_prompt_requires_validation_command(&prompt_lower),
        requires_public_interface_check: false,
        allowed_write_roots: Vec::new(),
        target_artifacts: coding_execution_spine_target_artifacts(&prompt_lower),
        public_surface_requirements: Vec::new(),
        max_repair_turns: coding_execution_spine_max_repair_turns(metadata),
    }
}

fn coding_execution_spine_target_artifacts(prompt_lower: &str) -> Vec<String> {
    let mut artifacts = Vec::new();
    if native_tool_prompt_requires_product_mutation(prompt_lower) {
        artifacts.push("source".to_string());
    }
    if native_tool_prompt_requires_test_changes(prompt_lower) {
        artifacts.push("test".to_string());
    }
    artifacts
}

fn coding_execution_spine_max_repair_turns(metadata: &Value) -> u32 {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|criteria| {
            criteria
                .get("bounded_direct_edit_repair_attempts")
                .or_else(|| criteria.get("completion_evidence_repair_max_turns"))
        })
        .and_then(Value::as_u64)
        .unwrap_or(1)
        .clamp(0, 6) as u32
}

fn context_evidence_from_receipt(receipt: &NativeToolReceipt) -> Option<ContextEvidence> {
    if receipt.status != "ok" {
        return None;
    }
    let selected_paths = match receipt.tool_name.as_str() {
        "file_read" => receipt
            .result
            .get("path")
            .and_then(Value::as_str)
            .map(|path| vec![path.to_string()])
            .unwrap_or_default(),
        "file_read_many" => receipt
            .result
            .get("files")
            .and_then(Value::as_array)
            .map(|files| {
                files
                    .iter()
                    .filter_map(|file| file.get("path").and_then(Value::as_str))
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
        "file_list" | "file_stat" => receipt
            .result
            .get("path")
            .and_then(Value::as_str)
            .map(|path| vec![path.to_string()])
            .unwrap_or_default(),
        _ => return None,
    };
    Some(ContextEvidence {
        receipt_ref: receipt.call_id.clone(),
        sufficient_for_mutation: !selected_paths.is_empty(),
        selected_paths,
    })
}

fn mutation_evidence_from_receipt(receipt: &NativeToolReceipt) -> Option<MutationEvidence> {
    if receipt.status != "ok" || !matches!(receipt.tool_name.as_str(), "file_write" | "file_patch") {
        return None;
    }
    let mut changed_paths = Vec::new();
    if let Some(path) = receipt.result.get("path").and_then(Value::as_str) {
        changed_paths.push(path.to_string());
    }
    if let Some(paths) = receipt.result.get("paths").and_then(Value::as_array) {
        for path in paths.iter().filter_map(Value::as_str) {
            if !changed_paths.iter().any(|existing| existing == path) {
                changed_paths.push(path.to_string());
            }
        }
    }
    Some(MutationEvidence {
        receipt_ref: receipt.call_id.clone(),
        tool_name: receipt.tool_name.clone(),
        success: !changed_paths.is_empty(),
        artifact_roles: artifact_roles_for_paths(&changed_paths),
        changed_paths,
    })
}

fn artifact_roles_for_paths(paths: &[String]) -> Vec<String> {
    let mut roles = Vec::new();
    for path in paths {
        let lower = path.replace('\\', "/").to_ascii_lowercase();
        let role = if lower.contains("/tests/")
            || lower.starts_with("tests/")
            || lower.contains("/test/")
            || lower.contains("test_")
            || lower.contains("_test.")
            || lower.contains(".test.")
            || lower.contains(".spec.")
        {
            "test"
        } else if lower.ends_with(".md") || lower.contains("/docs/") || lower.contains("readme") {
            "doc"
        } else {
            "source"
        };
        if !roles.iter().any(|existing| existing == role) {
            roles.push(role.to_string());
        }
    }
    roles
}

fn validation_evidence_from_receipt(
    receipt: &NativeToolReceipt,
    latest_successful_mutation_index: Option<usize>,
) -> Option<ValidationEvidence> {
    if receipt.status != "ok" || receipt.tool_name != "command_run" {
        return None;
    }
    let command = receipt_command_text(receipt);
    if !command_looks_like_validation(&command) {
        return None;
    }
    Some(ValidationEvidence {
        receipt_ref: receipt.call_id.clone(),
        command,
        success: receipt
            .result
            .get("success")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        after_mutation_index: latest_successful_mutation_index,
    })
}

fn public_interface_evidence_from_receipt(
    receipt: &NativeToolReceipt,
) -> Option<PublicInterfaceEvidence> {
    if receipt.status != "ok" || receipt.tool_name != "command_run" {
        return None;
    }
    let command = receipt_command_text(receipt);
    if !command.to_ascii_lowercase().contains("semantic_probe") {
        return None;
    }
    let success = receipt
        .result
        .get("success")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    Some(PublicInterfaceEvidence {
        receipt_ref: receipt.call_id.clone(),
        success,
        missing_requirements: Vec::new(),
    })
}

fn receipt_command_text(receipt: &NativeToolReceipt) -> String {
    receipt
        .result
        .get("cmd")
        .or_else(|| receipt.result.get("command"))
        .and_then(|value| serde_json::to_string(value).ok())
        .unwrap_or_default()
}

fn command_looks_like_validation(command: &str) -> bool {
    let lower = command.to_ascii_lowercase();
    lower.contains("unittest")
        || lower.contains("pytest")
        || lower.contains("cargo test")
        || lower.contains("npm test")
        || lower.contains("pnpm test")
        || lower.contains("yarn test")
        || lower.contains("go test")
        || lower.contains("semantic_probe")
}
