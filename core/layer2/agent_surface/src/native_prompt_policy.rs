// Layer ownership: Core Layer 2 (Scheduling + Execution) - native prompt policy bridge.
use crate::native_evidence::{
    native_tool_changed_paths, native_tool_coding_task_lane, native_tool_evidence_target_brief,
    native_tool_failed_validation_receipt_details, native_tool_is_probable_micro_direct_write_task,
    native_tool_prompt_checkpoint_name, native_tool_prompt_expected_memory_row_id,
    native_tool_prompt_memory_cli_pattern, native_tool_successful_receipt_refs,
    native_tool_unique_code_path_mentions,
};
use crate::native_tools::NativeToolReceipt;
use crate::native_workflow_artifact::native_tool_workflow_artifact_memory_tags;
use serde_json::{json, Value};

pub(crate) fn native_tool_initial_prompt(original_prompt: &str, metadata: &Value) -> String {
    let criteria = metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"));
    let evidence_target_brief = native_tool_evidence_target_brief(original_prompt);
    let requires_native_tool_use = criteria
        .and_then(|value| value.get("requires_native_tool_use"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let force_discovery_first = criteria
        .and_then(|value| value.get("force_discovery_first_turn"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let force_read_first = criteria
        .and_then(|value| value.get("force_read_first_turn"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let task_lane = native_tool_coding_task_lane(metadata, original_prompt);
    if requires_native_tool_use
        && !force_discovery_first
        && !force_read_first
        && task_lane == "new_file_fast_path"
    {
        return format!(
            "{original_prompt}\n\n{}{evidence_target_brief}",
            native_tool_micro_direct_write_rule(original_prompt)
        );
    }
    if force_discovery_first {
        return format!(
            "{original_prompt}\n\nNative tool-use initiation rule: before planning, editing, or final answering, return only JSON with a tool_calls array that discovers the local project shape. Start with file_list on the local project root or directory implied by the task. Use file_stat before reading any target path that may not exist. After discovery observations, classify the work as create, edit, extend, debug, or refactor; then read only relevant existing context files before writing or patching. For create/new-file tasks, do not file_read the target file unless file_stat or file_list shows it exists. If the task requires mutation, do not repeat discovery/read-only turns after sufficient context; transition to file_write/file_patch or return a structured blocker. Do not produce prose, analysis, or a final answer until native discovery observations are returned.{evidence_target_brief}"
        );
    }
    if !force_read_first {
        if requires_native_tool_use {
            let rule = native_tool_bounded_orchestration_prompt_text(
                metadata,
                "initial_tool_use_rule",
                "Native tool-use rule: choose the shortest safe native file-tool path for the task. Use discovery for unclear existing-project work, mutate only after enough context is available, validate after edits when requested, and do not claim success until native receipts prove the required file mutation and validation outcomes.",
                native_tool_initial_policy_char_budget(metadata),
            );
            let guardrails = native_tool_initial_coding_guardrails(original_prompt);
            return format!(
                "{original_prompt}\n\nNative coding task lane: {task_lane}.\n{rule}{guardrails}{evidence_target_brief}"
            );
        }
        return original_prompt.to_string();
    }
    format!(
        "{original_prompt}\n\nNative tool-use initiation rule: before planning or final answering, return only JSON with a tool_calls array that reads existing local context files relevant to the task. If a target may not exist yet, use file_stat first rather than file_read. Prefer file_read_many when multiple existing files are known. Do not produce prose, analysis, or a final answer until native file-read observations are returned.{evidence_target_brief}"
    )
}

fn native_tool_micro_direct_write_rule(original_prompt: &str) -> String {
    let targets = native_tool_unique_code_path_mentions(original_prompt);
    let target_text = if targets.is_empty() {
        "the explicit target path named by the user".to_string()
    } else {
        targets.join(", ")
    };
    format!(
        "Native coding task lane: new_file_fast_path. This looks like isolated greenfield/create-one-file work, so use the shortest safe native path. Return only JSON with a tool_calls array containing file_write for {target_text}. Do not call file_list, file_stat, file_read, command_run, or final-answer before the first successful mutation. If local files, permissions, or missing target information genuinely prevent writing, return a structured blocker instead of doing project discovery."
    )
}

pub(crate) fn native_tool_orchestration_prompt_text(
    metadata: &Value,
    key: &str,
    fallback: &str,
) -> String {
    metadata
        .get("native_runtime_prompt_policy")
        .or_else(|| metadata.pointer("/workflow/native_runtime_prompt_policy"))
        .and_then(|value| value.get(key))
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| fallback.to_string())
}

fn native_tool_bounded_orchestration_prompt_text(
    metadata: &Value,
    key: &str,
    fallback: &str,
    max_chars: usize,
) -> String {
    let text = native_tool_orchestration_prompt_text(metadata, key, fallback);
    if text.chars().count() <= max_chars {
        text
    } else {
        fallback.to_string()
    }
}

fn native_tool_initial_policy_char_budget(metadata: &Value) -> usize {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("max_initial_tool_policy_chars"))
        .and_then(Value::as_u64)
        .unwrap_or(1800)
        .clamp(400, 4000) as usize
}

fn native_tool_initial_coding_guardrails(original_prompt: &str) -> String {
    let lower = original_prompt.to_ascii_lowercase();
    let mut guardrails = Vec::new();
    if lower.contains("preserve ") {
        guardrails.push("Preservation guardrail: existing public behavior named after `preserve` is a hard baseline. Do not change its constants, return shape, or observable result to make a new feature pass. Add adjacent APIs by composing or extending the existing behavior unless the prompt explicitly asks to replace it.");
    }
    if lower.contains("add tests")
        || lower.contains("update tests")
        || lower.contains("regression tests")
        || lower.contains("tests for")
        || lower.contains("test for")
    {
        guardrails.push("Test requirement closure: when tests are requested, source/product mutation is not enough. After the first product/source mutation, write or patch a focused test file before running validation, writing handoff artifacts, memory closure, or final output.");
    }
    if guardrails.is_empty() {
        return String::new();
    }
    format!("\n\n{}", guardrails.join("\n"))
}

pub(crate) fn native_tool_completion_evidence_repair_prompt(
    metadata: &Value,
    original_prompt: &str,
    previous_output: &str,
    receipts: &[NativeToolReceipt],
    repair_reasons: &[String],
) -> String {
    let changed_paths = native_tool_changed_paths(receipts);
    let receipt_refs = native_tool_successful_receipt_refs(receipts);
    let evidence_target_brief = native_tool_evidence_target_brief(original_prompt);
    let failed_validation_details = native_tool_failed_validation_receipt_details(receipts);
    let repair_actions =
        native_tool_completion_repair_action_brief(metadata, original_prompt, repair_reasons);
    let test_change_repair_hint =
        native_tool_missing_test_change_repair_hint(receipts, repair_reasons);
    let failed_validation_repair_hint = native_tool_failed_validation_repair_hint(receipts);
    let failed_validation_repair_contract = native_tool_failed_validation_repair_contract(receipts);
    let repair_rule = native_tool_orchestration_prompt_text(
        metadata,
        "completion_evidence_repair_prompt_rule",
        "This is a bounded continuation of the same native tool task. Repair only the listed uncovered requirements. Return JSON tool calls while repairing, or a structured blocker only when local completion is genuinely blocked.",
    );
    let stage_rule = native_tool_orchestration_prompt_text(
        metadata,
        "completion_stage_controller_rule",
        "Stage controller: advance in order through source mutation, test mutation, validation, checkpoint handoff, memory closure, and final answer. Do not skip to a later stage while earlier receipt evidence is missing.",
    );
    format!(
        "{}\n\nStage controller:\n{}\n\nOriginal task:\n{}\n{}\n\nReceipt-backed changed files so far:\n{}\n\nSuccessful receipt refs:\n{}\n\nFailed validation receipt details:\n{}\n\nFailed validation repair contract:\n{}\n\nUncovered requirements detected by the runtime:\n{}\n\nRequired repair actions:\n{}\n\nTest mutation repair hint:\n{}\n\nFailed validation repair hint:\n{}\n\nPrevious output preview:\n{}",
        repair_rule,
        stage_rule,
        original_prompt.chars().take(2600).collect::<String>(),
        evidence_target_brief,
        changed_paths.join("\n"),
        receipt_refs.join("\n"),
        failed_validation_details,
        failed_validation_repair_contract,
        repair_reasons.join("\n"),
        repair_actions,
        test_change_repair_hint,
        failed_validation_repair_hint,
        previous_output.chars().take(1400).collect::<String>()
    )
}

pub(crate) fn native_tool_missing_test_change_repair_hint(
    receipts: &[NativeToolReceipt],
    repair_reasons: &[String],
) -> String {
    if !repair_reasons
        .iter()
        .any(|reason| reason == "missing_test_change_receipt")
    {
        return "<none>".to_string();
    }

    let observed_test_paths = native_tool_observed_test_paths(receipts);
    if observed_test_paths.is_empty() {
        return "The runtime still needs a successful test mutation receipt. The next response must be JSON tool_calls with file_write or file_patch targeting a focused regression test path under the project tests directory. Do not run validation, continue source edits, write checkpoint/handoff artifacts, or final-answer until that test mutation receipt exists.".to_string();
    }

    format!(
        "The runtime still needs a successful test mutation receipt. The next response must be JSON tool_calls with file_write or file_patch targeting an observed test path, or a new focused regression test under the same tests directory. Prefer the smallest regression test that imports/calls the public API named by the user. Observed test paths:\n{}",
        observed_test_paths.join("\n")
    )
}

fn native_tool_observed_test_paths(receipts: &[NativeToolReceipt]) -> Vec<String> {
    let mut paths = Vec::new();
    for receipt in receipts {
        let Some(path) = receipt.result.get("path").and_then(Value::as_str) else {
            continue;
        };
        if native_tool_path_looks_like_test(path) && !paths.iter().any(|existing| existing == path)
        {
            paths.push(path.to_string());
        }
    }
    paths
}

fn native_tool_path_looks_like_test(path: &str) -> bool {
    let lower = path.replace('\\', "/").to_ascii_lowercase();
    lower.contains("/tests/")
        || lower.ends_with("/tests")
        || lower.contains("/test/")
        || lower.contains("test_")
        || lower.ends_with("_test.py")
        || lower.ends_with(".test.js")
        || lower.ends_with(".spec.js")
        || lower.ends_with(".test.ts")
        || lower.ends_with(".spec.ts")
}

pub(crate) fn native_tool_failed_validation_repair_hint(receipts: &[NativeToolReceipt]) -> String {
    let failed_validation_details = native_tool_failed_validation_receipt_details(receipts);
    if failed_validation_details == "<none>" {
        return "<none>".to_string();
    }
    let changed_paths = native_tool_changed_paths(receipts);
    if changed_paths.is_empty() {
        return "Validation is failing, but no successful mutation receipt exists yet. Produce the smallest source/test file_write or file_patch needed before running validation again.".to_string();
    }
    format!(
        "Validation is failing after local mutations. Use the failed validation details as repair input and make the next substantive tool call a file_write or file_patch against the changed source/test file that caused the failure; do not continue read-only exploration once the failed source/test files have been inspected. Changed files:\n{}",
        changed_paths.join("\n")
    )
}

fn native_tool_failed_validation_repair_contract(receipts: &[NativeToolReceipt]) -> String {
    let failed_validation_details = native_tool_failed_validation_receipt_details(receipts);
    if failed_validation_details == "<none>" {
        return "<none>".to_string();
    }
    let changed_paths = native_tool_changed_paths(receipts);
    if changed_paths.is_empty() {
        return "Failed-validation repair is mutation-first. Return JSON tool calls only: make the smallest relevant file_write or file_patch repair, then rerun the failing validation command. Do not answer in prose while local repair remains possible.".to_string();
    }
    format!(
        "Failed-validation repair is mutation-first. Return JSON tool calls only. The next substantive call must be file_patch or file_write against one of the changed source/test files below, unless exactly one focused file_read/file_read_many is needed to inspect current contents. After the mutation, rerun the failing validation command. Do not answer in prose or ask for user input while local repair remains possible.\n{}",
        changed_paths.join("\n")
    )
}

pub(crate) fn native_tool_completion_repair_action_brief(
    metadata: &Value,
    original_prompt: &str,
    repair_reasons: &[String],
) -> String {
    let rule = native_tool_orchestration_prompt_text(
        metadata,
        "completion_repair_action_rule",
        "Continue from the existing native tool work and produce the smallest missing final deliverable. Do not restart or rewrite unrelated work.",
    );
    let target_paths = repair_reasons
        .iter()
        .filter_map(|reason| reason.strip_prefix("missing_changed_path:"))
        .collect::<Vec<_>>();
    let expected_row = repair_reasons
        .iter()
        .find_map(|reason| {
            reason
                .strip_prefix("missing_memory_write_receipt:")
                .filter(|value| !value.is_empty())
        })
        .map(str::to_string)
        .or_else(|| native_tool_prompt_expected_memory_row_id(original_prompt));
    let cli_pattern = native_tool_prompt_memory_cli_pattern(original_prompt);
    let tags = native_tool_workflow_artifact_memory_tags(metadata).join(",");
    let completed_checkpoint = native_tool_prompt_checkpoint_name(original_prompt);
    let product_mutation_action =
        native_tool_missing_product_mutation_action(original_prompt, repair_reasons);
    let missing_path_action = if target_paths.is_empty() {
        "<none>".to_string()
    } else {
        format!(
            "The runtime found required changed paths without successful mutation receipts. The next JSON tool_calls must file_write/file_patch those exact prompt-derived source, test, doc, or checkpoint paths instead of satisfying the task with unrelated sidecar files. For checkpoint/handoff paths, write a completed checkpoint artifact with completed_checkpoint, changed_files, validation_summary or validation_results, known_risks, and recommended_next_checkpoint; status alone is not enough. Missing paths:\n{}",
            target_paths.join("\n")
        )
    };
    let expected_row_text = expected_row.unwrap_or_else(|| "<none>".to_string());
    let cli_pattern_text = cli_pattern.unwrap_or_else(|| "<none>".to_string());
    let memory_action = if expected_row_text == "<none>" {
        "<none>".to_string()
    } else if cli_pattern_text == "<none>" {
        format!(
            "The task still needs a receipt-backed memory closure for row {expected_row_text}. Use the native command/file path implied by the task to persist that row, including validation status and checkpoint outcome. Do not satisfy memory closure by writing checkpoint_memory_persisted=true in a handoff artifact; the runtime needs a successful memory write receipt."
        )
    } else {
        format!(
            "The task still needs a receipt-backed memory closure for row {expected_row_text}. Run or adapt the prompt-provided memory CLI pattern and include validation status plus checkpoint outcome in the persisted row. Do not satisfy memory closure by writing checkpoint_memory_persisted=true in a handoff artifact; the runtime needs a successful memory write receipt. Pattern: {cli_pattern_text}"
        )
    };
    format!(
        "{rule}\n\nUncovered items:\n{}\n\nRequired product mutation action:\n{}\n\nRequired missing-path action:\n{}\n\nRequired memory closure action:\n{}\n\nPrompt-derived target paths:\n{}\n\nExpected memory row:\n{}\n\nMemory CLI pattern:\n{}\n\nMemory tags:\n{}\n\nPrompt-derived completed checkpoint:\n{}",
        repair_reasons.join("\n"),
        product_mutation_action.unwrap_or_else(|| "<none>".to_string()),
        missing_path_action,
        memory_action,
        target_paths.join("\n"),
        expected_row_text,
        cli_pattern_text,
        tags,
        completed_checkpoint.unwrap_or_else(|| "<none>".to_string())
    )
}

fn native_tool_missing_product_mutation_action(
    original_prompt: &str,
    repair_reasons: &[String],
) -> Option<String> {
    if repair_reasons
        .iter()
        .any(|reason| reason == "missing_product_mutation_receipt")
    {
        let project_root = crate::native_evidence::native_tool_prompt_project_root(original_prompt)
            .unwrap_or_else(|| "<project root from the prompt>".to_string());
        return Some(format!(
            "The task still has no successful file_write/file_patch receipt, so validation cannot count as completion. The next response must be JSON tool_calls with at least one file_write or file_patch that implements a coherent product slice under {project_root}. Do not call file_read, file_list, command_run, summarize, finalize, write checkpoint/handoff artifacts, or ask the user until source/test product mutations exist. Prefer a vertical slice: product code first, then tests/docs/checkpoint artifacts requested by the prompt."
        ));
    }
    if repair_reasons
        .iter()
        .any(|reason| reason == "missing_test_change_receipt")
    {
        return Some(
            "The product code has mutation receipts, but the task still lacks a successful test file_write/file_patch receipt. The next response must write or patch a focused regression test under the existing tests directory. Do not write checkpoint/handoff artifacts, run memory closure, finalize, or ask the user until a test mutation receipt exists."
                .to_string(),
        );
    }
    let product_slice_reasons = repair_reasons
        .iter()
        .filter(|reason| {
            reason.starts_with("incomplete_product_slice")
                || reason.starts_with("missing_product_source_evidence:")
                || reason.starts_with("missing_public_interface_verification:")
        })
        .cloned()
        .collect::<Vec<_>>();
    if product_slice_reasons.is_empty() {
        return None;
    }
    let repair_hint = native_tool_product_slice_repair_hint(&product_slice_reasons);
    Some(format!(
        "The current mutation is too shallow for the requested product slice or public interface. Do not write checkpoint handoff or memory closure yet. Do not keep reading context if source/test context has already been observed. Return JSON tool_calls with file_write/file_patch updates that cover the missing product evidence and expose the requested public API from the module/import surface under test: {}. {} Prefer source + tests + CLI/docs as one bounded vertical slice, then run validation.",
        product_slice_reasons.join(", "),
        repair_hint
    ))
}

fn native_tool_product_slice_repair_hint(product_slice_reasons: &[String]) -> String {
    let missing_report = product_slice_reasons
        .iter()
        .any(|reason| reason == "missing_product_source_evidence:report");
    let missing_import_export = product_slice_reasons
        .iter()
        .any(|reason| reason == "missing_product_source_evidence:import_export");
    if missing_report && missing_import_export {
        return "If persistence/model code already exists, extend that existing module into the routing service and CLI: add report-by-destination/retryable summary behavior plus import/export or round-trip commands, and add regression tests for those public surfaces.".to_string();
    }
    if missing_report {
        return "If persistence/model code already exists, add report-by-destination and retryable-failure summary behavior through service or CLI plus regression tests.".to_string();
    }
    if missing_import_export {
        return "If persistence/model code already exists, add import/export or durable round-trip behavior through CLI/service plus regression tests.".to_string();
    }
    String::new()
}

pub(crate) fn native_tool_public_reasoning_metadata(metadata: &Value) -> Value {
    let mut out = metadata.clone();
    if let Some(object) = out.as_object_mut() {
        object.insert("provider_timeout_seconds".to_string(), json!(90));
        object.insert(
            "native_public_reasoning_finalization".to_string(),
            json!(true),
        );
    }
    out
}

pub(crate) fn native_tool_public_reasoning_finalization_prompt(
    metadata: &Value,
    original_prompt: &str,
    receipts: &[NativeToolReceipt],
    previous_output: &str,
) -> String {
    let rule = native_tool_orchestration_prompt_text(
        metadata,
        "public_reasoning_finalization_rule",
        "Produce a concise receipt-backed final user response for this native coding workflow attempt. Include status, changed files when known, validation status, blockers when present, and the next useful step. Do not expose hidden chain-of-thought.",
    );
    format!(
        "{rule}\n\nOriginal task summary:\n{}\n\nNative tool receipt count: {}\n\nPrevious non-final output preview:\n{}",
        original_prompt.chars().take(2400).collect::<String>(),
        receipts.len(),
        previous_output.chars().take(1200).collect::<String>()
    )
}

pub(crate) fn native_tool_recovery_prompt(
    metadata: &Value,
    original_prompt: &str,
    reason: &str,
    changed_paths: &[String],
    receipts: &[NativeToolReceipt],
) -> String {
    let receipt_refs = receipts
        .iter()
        .filter(|receipt| receipt.status == "ok")
        .map(|receipt| {
            let path = receipt
                .result
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or("");
            format!("{}:{}:{}", receipt.call_id, receipt.tool_name, path)
        })
        .collect::<Vec<_>>()
        .join("\n");
    let recovery_rule = native_tool_orchestration_prompt_text(
        metadata,
        "partial_progress_recovery_rule",
        "Run a bounded recovery pass using receipt-backed changed files as context. Fix only obvious local errors inside those files, do not expand scope, and provide the final user response if no safe repair remains.",
    );
    format!(
        "{recovery_rule}\n\nRecovery reason:\n{reason}\n\nOriginal task:\n{}\n\nReceipt-backed changed files:\n{}\n\nSuccessful receipt refs:\n{}",
        original_prompt.chars().take(2400).collect::<String>(),
        changed_paths.join("\n"),
        receipt_refs
    )
}

pub(crate) fn native_tool_empty_retry_prompt(
    metadata: &Value,
    original_prompt: &str,
    previous_output: &str,
    retry: u64,
) -> String {
    let previous = previous_output.trim();
    let evidence_target_brief = native_tool_evidence_target_brief(original_prompt);
    let previous = if previous.is_empty() {
        "The previous response was empty.".to_string()
    } else {
        format!(
            "Previous response without native tool calls:\n{}",
            previous.chars().take(1200).collect::<String>()
        )
    };
    let rule = native_tool_orchestration_prompt_text(
        metadata,
        "empty_tool_retry_rule",
        "This run requires native tool receipts before completion. Return only JSON with a tool_calls array now, or return a structured blocker only if local files, permissions, or missing user information genuinely prevent mutation.",
    );
    let task_lane = native_tool_coding_task_lane(metadata, original_prompt);
    let fast_lane_rule =
        if native_tool_is_probable_micro_direct_write_task(metadata, original_prompt) {
            format!(
                "\n\n{}",
                native_tool_micro_direct_write_rule(original_prompt)
            )
        } else {
            String::new()
        };
    format!(
        "{original_prompt}\n\nNative coding task lane: {task_lane}.\nNative tool retry {retry}: {rule}{fast_lane_rule}\n\n{previous}{evidence_target_brief}"
    )
}

pub(crate) fn native_tool_context_to_mutation_retry_prompt(
    metadata: &Value,
    original_prompt: &str,
    previous_output: &str,
    observations: &str,
    retry: u64,
) -> String {
    let previous = previous_output.trim();
    let previous = if previous.is_empty() {
        "The previous response had no native tool calls.".to_string()
    } else {
        format!(
            "Previous response without mutation tool calls:\n{}",
            previous.chars().take(1200).collect::<String>()
        )
    };
    let rule = native_tool_bounded_orchestration_prompt_text(
        metadata,
        "context_to_mutation_transition_rule",
        "Native mutation transition retry: local context already exists, but no successful file_write or file_patch receipt exists yet. Return only JSON tool calls for the next safe mutation batch, then validate when requested. Return a structured blocker only when local context proves mutation is unsafe or impossible.",
        native_tool_mutation_entry_policy_char_budget(metadata),
    );
    let mutation_action = native_tool_missing_product_mutation_action(
        original_prompt,
        &["missing_product_mutation_receipt".to_string()],
    )
    .unwrap_or_default();
    let task_brief = native_tool_task_brief(original_prompt);
    format!(
        "{task_brief}\n\n{rule}\n\nRequired product mutation action:\n{mutation_action}\n\nImplementation-entry response format: return only JSON tool_calls with no prose, markdown, explanation, validation command, or final answer. Use the already observed project structure; batch source and requested test edits now.\n\nRetry: {retry}\n\n{previous}\n\nNative tool observations already available:\n{observations}"
    )
}

fn native_tool_mutation_entry_policy_char_budget(metadata: &Value) -> usize {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("max_mutation_entry_policy_chars"))
        .and_then(Value::as_u64)
        .unwrap_or(900)
        .clamp(300, 2400) as usize
}

fn native_tool_task_brief(original_prompt: &str) -> String {
    let mut lines = Vec::new();
    for line in original_prompt.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Task:")
            || trimmed.starts_with("Your write ownership is limited to this project root:")
            || trimmed.starts_with("6. Run this validation command")
        {
            lines.push(trimmed.to_string());
        }
    }
    if lines.is_empty() {
        original_prompt.chars().take(1600).collect::<String>()
    } else {
        lines.join("\n")
    }
}
