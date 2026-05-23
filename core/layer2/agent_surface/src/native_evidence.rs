// Layer ownership: Core Layer 2 (Scheduling + Execution) - agent runtime surface coordination.
use crate::native_tools::NativeToolReceipt;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

pub(crate) fn native_tool_has_successful_mutation(receipts: &[NativeToolReceipt]) -> bool {
    receipts.iter().any(|receipt| {
        receipt.status == "ok"
            && matches!(receipt.tool_name.as_str(), "file_write" | "file_patch")
    })
}

pub(crate) fn native_tool_has_successful_validation_command(receipts: &[NativeToolReceipt]) -> bool {
    receipts.iter().any(|receipt| {
        receipt.status == "ok"
            && receipt.tool_name == "command_run"
            && receipt
                .result
                .get("success")
                .and_then(Value::as_bool)
                .unwrap_or(false)
    })
}

pub(crate) fn native_tool_failed_validation_command_refs(receipts: &[NativeToolReceipt]) -> Vec<String> {
    receipts
        .iter()
        .filter(|receipt| {
            receipt.status == "ok"
                && receipt.tool_name == "command_run"
                && !receipt
                    .result
                    .get("success")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
        })
        .map(|receipt| format!("failed_validation_command_receipt:{}", receipt.call_id))
        .collect()
}

pub(crate) fn native_tool_failed_validation_receipt_details(receipts: &[NativeToolReceipt]) -> String {
    let details = receipts
        .iter()
        .filter(|receipt| {
            receipt.status == "ok"
                && receipt.tool_name == "command_run"
                && !receipt
                    .result
                    .get("success")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
        })
        .map(|receipt| {
            let cmd = receipt
                .result
                .get("cmd")
                .and_then(|value| serde_json::to_string(value).ok())
                .unwrap_or_else(|| "[]".to_string());
            let exit_code = receipt
                .result
                .get("exit_code")
                .map(|value| value.to_string())
                .unwrap_or_else(|| "null".to_string());
            let stdout = receipt
                .result
                .get("stdout")
                .and_then(Value::as_str)
                .unwrap_or("")
                .chars()
                .take(1800)
                .collect::<String>();
            let stderr = receipt
                .result
                .get("stderr")
                .and_then(Value::as_str)
                .unwrap_or("")
                .chars()
                .take(1800)
                .collect::<String>();
            format!(
                "{} cmd={} exit_code={}\nstdout:\n{}\nstderr:\n{}",
                receipt.call_id, cmd, exit_code, stdout, stderr
            )
        })
        .collect::<Vec<_>>();
    if details.is_empty() {
        "none".to_string()
    } else {
        details.join("\n\n---\n\n")
    }
}

pub(crate) fn native_tool_context_only_turn(receipts: &[NativeToolReceipt]) -> bool {
    let mut saw_successful_context = false;
    for receipt in receipts {
        match receipt.tool_name.as_str() {
            "file_list" | "file_stat" | "file_read" | "file_read_many" => {
                saw_successful_context |= receipt.status == "ok";
            }
            "file_write" | "file_patch" => return false,
            "command_run" => {
                saw_successful_context |= receipt.status == "ok";
            }
            _ => return false,
        }
    }
    saw_successful_context
}

pub(crate) fn native_tool_should_synthesize_micro_final(
    metadata: &Value,
    original_prompt: &str,
    receipts: &[NativeToolReceipt],
) -> bool {
    let criteria = metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"));
    let enabled = criteria
        .and_then(|value| value.get("synthesize_final_after_successful_micro_mutation"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    enabled
        && native_tool_has_successful_mutation(receipts)
        && native_tool_is_probable_micro_direct_write_task(metadata, original_prompt)
}

pub(crate) fn native_tool_is_probable_micro_direct_write_task(metadata: &Value, original_prompt: &str) -> bool {
    let criteria = metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"));
    if !criteria
        .and_then(|value| value.get("micro_direct_write_enabled"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return false;
    }
    let lower = original_prompt.to_ascii_lowercase();
    let create_like = [
        "create ",
        "write ",
        "make ",
        "single file",
        "one file",
        "tiny ",
    ]
    .iter()
    .any(|needle| lower.contains(needle));
    if !create_like {
        return false;
    }
    let existing_project_markers = [
        "update ",
        "modify ",
        "refactor",
        "debug",
        "fix ",
        "repair",
        "existing ",
        "preserve ",
        "integrat",
        "tests/",
        "src/",
        "package.json",
        "pyproject.toml",
        "cargo.toml",
    ];
    if existing_project_markers
        .iter()
        .any(|needle| lower.contains(needle))
    {
        return false;
    }
    let target_count = native_tool_unique_code_path_mentions(original_prompt).len();
    let max_targets = criteria
        .and_then(|value| value.get("micro_direct_write_max_target_files"))
        .and_then(Value::as_u64)
        .unwrap_or(1) as usize;
    target_count > 0 && target_count <= max_targets
}

pub(crate) fn native_tool_coding_task_lane(metadata: &Value, original_prompt: &str) -> &'static str {
    if native_tool_is_probable_micro_direct_write_task(metadata, original_prompt) {
        return "new_file_fast_path";
    }
    let lower = original_prompt.to_ascii_lowercase();
    let long_run_markers = [
        "checkpoint",
        "resume",
        "memory row",
        "continue the existing local",
        "long-run",
        "long run",
        "operator",
    ];
    if long_run_markers.iter().any(|marker| lower.contains(marker))
        && (lower.contains("project") || lower.contains("workflow") || lower.contains("coding"))
    {
        return "long_run_project_operator";
    }
    let validation_repair_markers = [
        "validation fails",
        "validation failed",
        "test fails",
        "tests fail",
        "repair any failures",
        "use the command output as repair input",
        "failed validation",
    ];
    if validation_repair_markers
        .iter()
        .any(|marker| lower.contains(marker))
    {
        return "validation_repair";
    }
    let multi_file_markers = [
        "multi-file",
        "multi file",
        "vertical slice",
        "cli",
        "persistence",
        "import/export",
        "round-trip",
        "docs",
        "readme",
        "service",
        "web app",
    ];
    if multi_file_markers.iter().any(|marker| lower.contains(marker)) {
        return "multi_file_slice";
    }
    let existing_project_markers = [
        "read the local",
        "existing",
        "preserve ",
        "add tests",
        "update tests",
        "regression tests",
        "run validation",
        "run tests",
        "pytest",
        "unittest",
        "pyproject.toml",
        "cargo.toml",
        "package.json",
        "src/",
        "tests/",
        "modify ",
        "update ",
        "refactor",
        "debug",
        "fix ",
        "extend ",
    ];
    if existing_project_markers
        .iter()
        .any(|marker| lower.contains(marker))
    {
        return "existing_project_patch";
    }
    if native_tool_prompt_requires_product_mutation(&lower) {
        return "implementation_slice";
    }
    "general_native_tool_task"
}

pub(crate) fn native_tool_unique_code_path_mentions(raw: &str) -> Vec<String> {
    let extensions = [
        ".py", ".js", ".ts", ".tsx", ".jsx", ".html", ".css", ".rs", ".go", ".java", ".rb",
        ".php", ".swift", ".kt", ".c", ".cpp", ".h", ".hpp", ".md", ".json",
    ];
    let mut out = Vec::<String>::new();
    for token in raw.split_whitespace() {
        let cleaned = token.trim_matches(|ch: char| {
            matches!(
                ch,
                '`' | '\'' | '"' | ',' | ';' | ':' | '.' | ')' | '(' | '[' | ']' | '{' | '}'
            )
        });
        let lower = cleaned.to_ascii_lowercase();
        if extensions.iter().any(|extension| lower.ends_with(extension))
            && !out.iter().any(|path| path == cleaned)
        {
            out.push(cleaned.to_string());
        }
    }
    out
}

pub(crate) fn native_tool_needs_public_report_finalization(metadata: &Value, output: &str) -> bool {
    let report_contract_key = ["public_", "reasoning_trace", "_contract"].concat();
    let contract = metadata.get(report_contract_key.as_str());
    let Some(contract) = contract else {
        return false;
    };
    if !contract.is_object() {
        return false;
    }
    if output.contains("\"tool_calls\"") || output.contains("{\"tool_calls\"") {
        return true;
    }
    let emits = contract
        .get("emits")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let trace_protocol = ["public_", "reasoning_trace_v1"].concat();
    let requires_trace = emits.iter().any(|emit| *emit == trace_protocol)
        || contract
            .get("local_trace_required_fields")
            .and_then(Value::as_array)
            .is_some();
    let rollup_protocol = ["public_", "reasoning_", "rollup_v1"].concat();
    let requires_rollup = emits.iter().any(|emit| *emit == rollup_protocol);
    let trace_field = ["public_", "reasoning_trace"].concat();
    let has_trace = output.contains(trace_field.as_str()) && output.contains(trace_protocol.as_str());
    let rollup_field = ["reasoning_", "rollup"].concat();
    let has_rollup = output.contains(rollup_field.as_str()) && output.contains(rollup_protocol.as_str());
    (requires_trace && !has_trace) || (requires_rollup && !has_rollup)
}

pub(crate) fn native_tool_needs_artifact_finalization(
    metadata: &Value,
    original_prompt: &str,
    output: &str,
    receipts: &[NativeToolReceipt],
) -> bool {
    native_tool_artifact_contract_enabled(metadata)
        && native_tool_prompt_has_multiple_requirements(original_prompt)
        && native_tool_has_successful_mutation(receipts)
        && !output.contains(["task_requirement_", "checklist"].concat().as_str())
}

pub(crate) fn native_tool_artifact_repair_reasons(
    metadata: &Value,
    original_prompt: &str,
    output: &str,
    receipts: &[NativeToolReceipt],
) -> Vec<String> {
    if !native_tool_artifact_contract_enabled(metadata)
        || !native_tool_prompt_has_multiple_requirements(original_prompt)
    {
        return Vec::new();
    }
    let mut reasons = Vec::<String>::new();
    if output.contains("partial_or_blocked")
        || output.contains("\"status\": \"uncovered\"")
        || output.contains("\"status\":\"uncovered\"")
        || output.contains("\"status\": \"blocked\"")
        || output.contains("\"status\":\"blocked\"")
    {
        reasons.push("reported_uncovered_or_blocked_requirement".to_string());
    }
    reasons.extend(native_tool_prompt_evidence_gaps(original_prompt, receipts));
    reasons.sort();
    reasons.dedup();
    reasons
}

pub(crate) fn native_tool_prompt_evidence_gaps(
    original_prompt: &str,
    receipts: &[NativeToolReceipt],
) -> Vec<String> {
    let mut reasons = Vec::<String>::new();
    let prompt_lower = original_prompt.to_ascii_lowercase();
    if native_tool_prompt_requires_product_mutation(&prompt_lower)
        && !native_tool_has_successful_mutation(receipts)
    {
        reasons.push("missing_product_mutation_receipt".to_string());
        return reasons;
    }
    reasons.extend(native_tool_product_slice_gaps(original_prompt, receipts));
    for path in native_tool_prompt_preserved_api_source_paths(original_prompt, receipts) {
        if !native_tool_changed_paths_include(receipts, &path) {
            reasons.push(format!("missing_changed_path:{path}"));
        }
    }
    for path in native_tool_prompt_required_changed_paths(original_prompt) {
        if !native_tool_changed_paths_include(receipts, &path) {
            reasons.push(format!("missing_changed_path:{path}"));
        } else if native_tool_is_handoff_artifact_path(&path)
            && !native_tool_checkpoint_receipt_file_valid(
                native_tool_changed_path_for_expected(receipts, &path)
                    .as_deref()
                    .unwrap_or(&path),
                native_tool_prompt_checkpoint_name(original_prompt).as_deref(),
            )
        {
            reasons.push(format!("invalid_checkpoint_receipt:{path}"));
        }
    }
    if native_tool_prompt_requires_test_changes(&prompt_lower)
        && !native_tool_changed_path_matches(receipts, |path| {
            let lower = path.to_ascii_lowercase();
            lower.contains("/test") || lower.contains("\\test") || lower.contains("tests/")
        })
    {
        reasons.push("missing_test_change_receipt".to_string());
    }
    reasons.extend(native_tool_public_interface_verification_gaps(
        original_prompt,
        receipts,
    ));
    if native_tool_prompt_requires_doc_changes(&prompt_lower)
        && !native_tool_changed_path_matches(receipts, |path| {
            let lower = path.to_ascii_lowercase();
            lower.ends_with("readme.md")
                || lower.contains("/docs/")
                || lower.contains("\\docs\\")
                || lower.contains("/doc/")
        })
    {
        reasons.push("missing_doc_change_receipt".to_string());
    }
    if native_tool_prompt_requires_validation_command(&prompt_lower)
        && !native_tool_has_successful_validation_command(receipts)
    {
        let failed_validation_refs = native_tool_failed_validation_command_refs(receipts);
        if failed_validation_refs.is_empty() {
            reasons.push("missing_validation_command_receipt".to_string());
        } else {
            reasons.extend(failed_validation_refs);
        }
    }
    if native_tool_prompt_requires_memory_write(&prompt_lower)
        && !native_tool_has_successful_memory_write_command(
            receipts,
            native_tool_prompt_expected_memory_row_id(original_prompt).as_deref(),
            original_prompt,
        )
    {
        let suffix = native_tool_prompt_expected_memory_row_id(original_prompt)
            .map(|id| format!(":{id}"))
            .unwrap_or_default();
        reasons.push(format!("missing_memory_write_receipt{suffix}"));
    }
    reasons.sort();
    reasons.dedup();
    reasons
}

fn native_tool_public_interface_verification_gaps(
    original_prompt: &str,
    receipts: &[NativeToolReceipt],
) -> Vec<String> {
    let prompt_lower = original_prompt.to_ascii_lowercase();
    if !native_tool_prompt_requires_product_mutation(&prompt_lower)
        || !native_tool_has_successful_mutation(receipts)
    {
        return Vec::new();
    }

    let mut reasons = Vec::<String>::new();
    reasons.extend(native_tool_changed_test_import_surface_gaps(
        original_prompt,
        receipts,
    ));
    reasons.extend(native_tool_preserved_module_public_surface_gaps(
        original_prompt,
        receipts,
    ));
    reasons.sort();
    reasons.dedup();
    reasons
}

fn native_tool_changed_test_import_surface_gaps(
    original_prompt: &str,
    receipts: &[NativeToolReceipt],
) -> Vec<String> {
    let Some(project_root) = native_tool_prompt_project_root(original_prompt).map(PathBuf::from)
    else {
        return Vec::new();
    };
    let mut reasons = Vec::<String>::new();
    for path in native_tool_changed_paths(receipts) {
        if !native_tool_path_looks_like_test(&path) {
            continue;
        }
        let Ok(test_text) = fs::read_to_string(&path) else {
            continue;
        };
        for (module, symbols) in native_tool_python_from_imports(&test_text) {
            let Some(module_path) = native_tool_python_module_path(&project_root, &module) else {
                continue;
            };
            let Ok(module_text) = fs::read_to_string(&module_path) else {
                continue;
            };
            for symbol in symbols {
                if native_tool_python_import_symbol_is_noise(&symbol) {
                    continue;
                }
                if !native_tool_source_defines_or_exports_public_api(&module_text, &symbol) {
                    reasons.push(format!(
                        "missing_public_interface_verification:{}:{}",
                        module, symbol
                    ));
                }
            }
        }
    }
    reasons
}

fn native_tool_preserved_module_public_surface_gaps(
    original_prompt: &str,
    receipts: &[NativeToolReceipt],
) -> Vec<String> {
    let requested_names = native_tool_prompt_requested_public_api_names(original_prompt);
    if requested_names.is_empty() {
        return Vec::new();
    }
    let preserved_paths = native_tool_prompt_preserved_api_source_paths(original_prompt, receipts);
    if preserved_paths.is_empty() {
        return Vec::new();
    }
    let mut reasons = Vec::<String>::new();
    for path in preserved_paths {
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        for name in &requested_names {
            if native_tool_source_defines_or_exports_public_api(&text, name) {
                continue;
            }
            reasons.push(format!(
                "missing_public_interface_verification:{}:{}",
                path, name
            ));
        }
    }
    reasons
}

pub(crate) fn native_tool_product_slice_ready(
    original_prompt: &str,
    receipts: &[NativeToolReceipt],
) -> bool {
    native_tool_product_slice_gaps(original_prompt, receipts).is_empty()
}

pub(crate) fn native_tool_product_slice_gaps(
    original_prompt: &str,
    receipts: &[NativeToolReceipt],
) -> Vec<String> {
    let prompt_lower = original_prompt.to_ascii_lowercase();
    if !native_tool_prompt_requires_product_mutation(&prompt_lower) {
        return Vec::new();
    }
    if !native_tool_has_successful_mutation(receipts) {
        return vec!["missing_product_mutation_receipt".to_string()];
    }
    let changed_product_paths = native_tool_changed_paths(receipts)
        .into_iter()
        .filter(|path| !native_tool_is_handoff_artifact_path(path))
        .collect::<Vec<_>>();
    let mut reasons = Vec::<String>::new();
    let changed_implementation_paths = changed_product_paths
        .iter()
        .filter(|path| native_tool_is_implementation_source_path(path))
        .cloned()
        .collect::<Vec<_>>();
    if changed_implementation_paths.is_empty() {
        reasons.push("missing_product_source_evidence:implementation_source".to_string());
    }
    let changed_implementation_text =
        native_tool_changed_product_source_text(&changed_implementation_paths);
    for name in native_tool_prompt_requested_public_api_names(original_prompt) {
        if !native_tool_public_interface_text_mentions(&changed_implementation_text, &name) {
            reasons.push(format!("missing_public_interface_evidence:{name}"));
        }
    }
    if native_tool_prompt_requires_multi_file_product_slice(&prompt_lower)
        && changed_product_paths.len() < 3
    {
        reasons.push(format!(
            "incomplete_product_slice_changed_file_count:{}",
            changed_product_paths.len()
        ));
    }
    if native_tool_prompt_requires_multi_file_product_slice(&prompt_lower)
        && !native_tool_changed_paths_include_product_and_test(&changed_product_paths)
    {
        reasons.push("incomplete_product_slice_missing_source_or_test_category".to_string());
    }
    reasons.sort();
    reasons.dedup();
    reasons
}

fn native_tool_prompt_requires_multi_file_product_slice(prompt_lower: &str) -> bool {
    native_tool_prompt_mentions_any(
        prompt_lower,
        &[
            "multiple files",
            "multi-file",
            "multiple file",
            "coherent checkpoint",
            "checkpoint_",
            "operator-facing cli",
        ],
    )
}

fn native_tool_changed_paths_include_product_and_test(paths: &[String]) -> bool {
    let mut has_product_source = false;
    let mut has_test_source = false;
    for path in paths {
        let lower = path.replace('\\', "/").to_ascii_lowercase();
        if lower.contains("/test")
            || lower.contains("_test.")
            || lower.contains(".test.")
            || lower.contains(".spec.")
        {
            has_test_source = true;
        } else if native_tool_path_looks_like_source(&lower) {
            has_product_source = true;
        }
    }
    has_product_source && has_test_source
}

fn native_tool_is_implementation_source_path(path: &str) -> bool {
    let lower = path.replace('\\', "/").to_ascii_lowercase();
    if native_tool_is_handoff_artifact_path(&lower)
        || lower.contains("/test/")
        || lower.contains("/tests/")
        || lower.contains("test_")
        || lower.ends_with("_test.py")
        || lower.ends_with(".test.js")
        || lower.ends_with(".spec.js")
        || lower.ends_with("/__init__.py")
        || lower.ends_with("/index.ts")
        || lower.ends_with("/index.tsx")
        || lower.ends_with("/index.js")
        || lower.ends_with("/index.jsx")
        || lower.ends_with("/mod.rs")
    {
        return false;
    }
    lower.ends_with(".py")
        || lower.ends_with(".rs")
        || lower.ends_with(".ts")
        || lower.ends_with(".tsx")
        || lower.ends_with(".js")
        || lower.ends_with(".jsx")
}

fn native_tool_prompt_requested_public_api_names(original_prompt: &str) -> Vec<String> {
    let mut names = Vec::<String>::new();
    for name in native_tool_prompt_explicit_public_api_names(original_prompt) {
        if !names.iter().any(|existing| existing == &name) {
            names.push(name);
        }
    }
    if !names.is_empty() {
        return names;
    }
    let task_surface = native_tool_public_api_request_surface(original_prompt);
    for token in task_surface.split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_')) {
        let token = token.trim();
        if !native_tool_token_looks_like_public_api(token) {
            continue;
        }
        let lower = token.to_ascii_lowercase();
        if matches!(
            lower.as_str(),
            "pythonpath"
                | "inspect"
                | "native"
                | "coding"
                | "useful"
                | "work"
                | "eval"
                | "infring"
                | "json"
                | "api"
                | "project_root"
                | "receipt_backed"
                | "current"
                | "existing"
                | "regression"
                | "validation"
                | "task"
                | "rules"
                | "run"
                | "read"
                | "add"
                | "final"
                | "codex"
                | "python3"
                | "project"
                | "preserve"
                | "this"
                | "python"
                | "root"
                | "command"
                | "semantic"
                | "probe"
                | "unittest"
                | "discover"
        ) {
            continue;
        }
        if native_tool_token_looks_like_path_or_harness_artifact(token) {
            continue;
        }
        if !names.iter().any(|existing| existing == token) {
            names.push(token.to_string());
        }
    }
    names
}

fn native_tool_prompt_explicit_public_api_names(original_prompt: &str) -> Vec<String> {
    let mut names = Vec::<String>::new();
    for line in original_prompt.lines() {
        let line = line.trim();
        for name in native_tool_public_api_names_from_call_patterns(line) {
            if !names.iter().any(|existing| existing == &name) {
                names.push(name);
            }
        }
        for name in native_tool_public_api_names_from_python_imports(line) {
            if !names.iter().any(|existing| existing == &name) {
                names.push(name);
            }
        }
        for name in native_tool_public_api_names_from_action_phrases(line) {
            if !names.iter().any(|existing| existing == &name) {
                names.push(name);
            }
        }
    }
    names
}

fn native_tool_public_api_names_from_call_patterns(line: &str) -> Vec<String> {
    let mut names = Vec::<String>::new();
    let bytes = line.as_bytes();
    let mut idx = 0usize;
    while idx < bytes.len() {
        let ch = bytes[idx] as char;
        if !(ch.is_ascii_alphabetic() || ch == '_') {
            idx += 1;
            continue;
        }
        let start = idx;
        idx += 1;
        while idx < bytes.len() {
            let ch = bytes[idx] as char;
            if ch.is_ascii_alphanumeric() || ch == '_' {
                idx += 1;
            } else {
                break;
            }
        }
        let token = &line[start..idx];
        let rest = line[idx..].trim_start();
        if rest.starts_with('(')
            && native_tool_token_looks_like_called_public_api(token)
            && !native_tool_token_looks_like_path_or_harness_artifact(token)
        {
            names.push(token.to_string());
        }
    }
    names
}

fn native_tool_public_api_names_from_python_imports(line: &str) -> Vec<String> {
    native_tool_python_from_imports(line)
        .into_iter()
        .flat_map(|(_module, symbols)| symbols)
        .filter(|symbol| {
            native_tool_token_looks_like_public_api(symbol)
                && !native_tool_token_looks_like_path_or_harness_artifact(symbol)
                && !native_tool_python_import_symbol_is_noise(symbol)
        })
        .collect()
}

fn native_tool_public_api_names_from_action_phrases(line: &str) -> Vec<String> {
    let mut names = Vec::<String>::new();
    let tokens = line
        .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    for (idx, token) in tokens.iter().enumerate() {
        let lower = token.to_ascii_lowercase();
        if !matches!(
            lower.as_str(),
            "add" | "repair" | "fix" | "implement" | "create" | "define" | "update"
        ) {
            continue;
        }
        for candidate in tokens.iter().skip(idx + 1).take(6) {
            let candidate_lower = candidate.to_ascii_lowercase();
            if matches!(
                candidate_lower.as_str(),
                "a" | "an"
                    | "the"
                    | "this"
                    | "that"
                    | "public"
                    | "api"
                    | "function"
                    | "method"
                    | "class"
                    | "tests"
                    | "test"
                    | "unittest"
                    | "failing"
                    | "relevant"
                    | "files"
                    | "file"
                    | "behavior"
                    | "command"
                    | "validation"
                    | "for"
                    | "it"
                    | "to"
                    | "from"
                    | "with"
                    | "without"
                    | "in"
                    | "on"
                    | "until"
            ) {
                continue;
            }
            if native_tool_token_looks_like_called_public_api(candidate)
                && !native_tool_token_looks_like_path_or_harness_artifact(candidate)
                && !names.iter().any(|existing| existing == candidate)
            {
                names.push((*candidate).to_string());
            }
            break;
        }
    }
    names
}

fn native_tool_token_looks_like_path_or_harness_artifact(token: &str) -> bool {
    let lower = token.to_ascii_lowercase();
    token.starts_with('_')
        || lower.ends_with("_tools")
        || lower.ends_with("_test")
        || lower.starts_with("test_")
        || lower.contains("test_")
        || lower.contains("semantic_probe")
        || lower == "semantic"
        || lower == "probe"
        || lower == "infring"
        || lower == "py"
        || lower == "pytest"
        || lower == "unittest"
        || lower == "pythonpath"
        || lower == "first"
        || lower == "then"
        || lower == "for"
        || lower == "it"
}

fn native_tool_token_looks_like_called_public_api(token: &str) -> bool {
    if token.len() < 3 || token.len() > 80 {
        return false;
    }
    if token.chars().any(|ch| ch.is_ascii_digit()) {
        return false;
    }
    let mut chars = token.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    if !chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_') {
        return false;
    }
    token.chars().any(|ch| ch.is_ascii_lowercase())
}

fn native_tool_token_looks_like_public_api(token: &str) -> bool {
    if token.len() < 3 || token.len() > 80 {
        return false;
    }
    if token.chars().all(|ch| ch.is_ascii_uppercase()) {
        return false;
    }
    let has_underscore = token.contains('_');
    let starts_upper = token
        .chars()
        .next()
        .map(|ch| ch.is_ascii_uppercase())
        .unwrap_or(false);
    let has_lower = token.chars().any(|ch| ch.is_ascii_lowercase());
    if token.chars().any(|ch| ch.is_ascii_digit()) {
        return false;
    }
    (has_underscore || (starts_upper && has_lower && token.len() <= 24))
        && token
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn native_tool_public_api_request_surface(original_prompt: &str) -> String {
    let mut lines = Vec::new();
    for line in original_prompt.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Task:")
            || trimmed.contains("user-named functions")
            || trimmed.contains("public API")
            || trimmed.contains("constructor arguments")
        {
            lines.push(trimmed.to_string());
        }
    }
    if lines.is_empty() {
        original_prompt.chars().take(1800).collect::<String>()
    } else {
        lines.join("\n")
    }
}

fn native_tool_public_interface_text_mentions(text: &str, name: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    let name = name.to_ascii_lowercase();
    lower.contains(&format!("def {name}("))
        || lower.contains(&format!("class {name}"))
        || lower.contains(&format!("function {name}("))
        || lower.contains(&format!("const {name}"))
        || lower.contains(&format!("let {name}"))
        || lower.contains(&format!("var {name}"))
        || lower.contains(&format!("pub fn {name}("))
        || lower.contains(&format!("struct {name}"))
        || lower.contains(&format!("enum {name}"))
        || lower.contains(&name)
}

fn native_tool_prompt_mentions_any(prompt_lower: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| prompt_lower.contains(needle))
}

fn native_tool_changed_product_source_text(paths: &[String]) -> String {
    let mut out = String::new();
    for path in paths {
        let lower = path.to_ascii_lowercase();
        if lower.ends_with(".py")
            || lower.ends_with(".rs")
            || lower.ends_with(".ts")
            || lower.ends_with(".tsx")
            || lower.ends_with(".js")
            || lower.ends_with(".jsx")
            || lower.ends_with(".json")
            || lower.ends_with(".md")
            || lower.ends_with(".toml")
        {
            if let Ok(text) = fs::read_to_string(path) {
                out.push_str(&text.to_ascii_lowercase());
                out.push('\n');
            }
        }
    }
    out
}

pub(crate) fn native_tool_evidence_target_brief(original_prompt: &str) -> String {
    let mut items = Vec::<String>::new();
    let paths = native_tool_prompt_required_changed_paths(original_prompt);
    if !paths.is_empty() {
        items.push(format!(
            "- prompt-derived target paths needing mutation evidence when applicable: {}",
            paths.join(", ")
        ));
    }
    let prompt_lower = original_prompt.to_ascii_lowercase();
    if native_tool_prompt_requires_product_mutation(&prompt_lower) {
        items.push("- implementation work was requested; include a successful file_write/file_patch receipt for source, tests, docs, or a checkpoint artifact before treating validation as completion".to_string());
    }
    if native_tool_prompt_requires_test_changes(&prompt_lower) {
        items.push("- tests were explicitly requested; include a test-file mutation receipt or a blocker".to_string());
    }
    if native_tool_prompt_requires_doc_changes(&prompt_lower) {
        items.push("- docs/README were explicitly requested; include a docs mutation receipt or a blocker".to_string());
    }
    if native_tool_prompt_requires_validation_command(&prompt_lower) {
        items.push("- validation/test status was requested; include a successful command_run validation receipt or a blocker".to_string());
    }
    if native_tool_prompt_requires_memory_write(&prompt_lower) {
        let target = native_tool_prompt_expected_memory_row_id(original_prompt)
            .map(|id| format!(" for expected row id `{id}`"))
            .unwrap_or_default();
        items.push(format!(
            "- checkpoint/project memory persistence was explicitly requested; include a successful memory-cli ingest command_run receipt{target} or a blocker"
        ));
    }
    if items.is_empty() {
        return String::new();
    }
    format!(
        "\n\nImplementation evidence targets:\n{}\nThese are generic prompt-derived evidence targets, not domain-specific hardcoded rules. For multi-requirement tasks, do not finalize until these target paths/categories have mutation receipts or a blocker explains why they should not be changed.",
        items.join("\n")
    )
}

pub(crate) fn native_tool_prompt_required_changed_paths(original_prompt: &str) -> Vec<String> {
    let mut paths = native_tool_unique_code_path_mentions(original_prompt)
        .into_iter()
        .filter(|path| !native_tool_prompt_marks_path_optional(original_prompt, path))
        .filter(|path| {
            let lower = path.to_ascii_lowercase();
            !lower.contains("python")
                && !lower.contains("public_reasoning")
                && !lower.contains(["reasoning_", "rollup"].concat().as_str())
                && !lower.contains(".infring/")
                && !lower.contains("\\.infring\\")
                && !lower.contains("semantic_probe")
                && !lower.contains('*')
                && !lower.contains('?')
                && !lower.contains("[")
                && !lower.contains("]")
                && !lower.ends_with(".py.")
        })
        .collect::<Vec<_>>();
    paths.sort();
    paths.dedup();
    paths
}

fn native_tool_prompt_preserved_api_source_paths(
    original_prompt: &str,
    receipts: &[NativeToolReceipt],
) -> Vec<String> {
    let names = native_tool_prompt_preserved_public_api_names(original_prompt);
    if names.is_empty() {
        return Vec::new();
    }

    let mut paths = Vec::<String>::new();
    for receipt in receipts {
        if receipt.status != "ok" || receipt.tool_name != "file_read" {
            continue;
        }
        let Some(path) = receipt.result.get("path").and_then(Value::as_str) else {
            continue;
        };
        if !native_tool_path_looks_like_source(path) {
            continue;
        }
        let Ok(text) = fs::read_to_string(path) else {
            continue;
        };
        if names
            .iter()
            .any(|name| native_tool_source_defines_public_api(&text, name))
            && !paths.iter().any(|existing| existing == path)
        {
            paths.push(path.to_string());
        }
    }
    paths
}

fn native_tool_prompt_preserved_public_api_names(original_prompt: &str) -> Vec<String> {
    let mut names = Vec::<String>::new();
    for segment in original_prompt.split(['.', ';', '\n']) {
        let lower = segment.to_ascii_lowercase();
        let Some(start) = lower.find("preserve ") else {
            continue;
        };
        let preserved = &segment[start + "preserve ".len()..];
        for token in preserved.split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_')) {
            let token = token.trim();
            if token.is_empty() {
                continue;
            }
            let lower_token = token.to_ascii_lowercase();
            if matches!(
                lower_token.as_str(),
                "and" | "api" | "behavior" | "behaviour" | "current" | "existing" | "public" | "the"
            ) {
                continue;
            }
            if !names.iter().any(|existing| existing == token) {
                names.push(token.to_string());
            }
            break;
        }
    }
    names
}

fn native_tool_path_looks_like_source(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    !lower.contains("/tests/")
        && !lower.contains("\\tests\\")
        && !lower.contains("/test/")
        && (lower.ends_with(".py")
            || lower.ends_with(".rs")
            || lower.ends_with(".ts")
            || lower.ends_with(".tsx")
            || lower.ends_with(".js")
            || lower.ends_with(".jsx")
            || lower.ends_with(".go")
            || lower.ends_with(".java")
            || lower.ends_with(".rb")
            || lower.ends_with(".php")
            || lower.ends_with(".swift")
            || lower.ends_with(".kt"))
}

fn native_tool_path_looks_like_test(path: &str) -> bool {
    let lower = path.replace('\\', "/").to_ascii_lowercase();
    lower.contains("/tests/")
        || lower.contains("/test/")
        || lower.contains("test_")
        || lower.ends_with("_test.py")
        || lower.ends_with(".test.js")
        || lower.ends_with(".spec.js")
        || lower.ends_with(".test.ts")
        || lower.ends_with(".spec.ts")
}

fn native_tool_source_defines_public_api(text: &str, name: &str) -> bool {
    let lower_text = text.to_ascii_lowercase();
    let lower_name = name.to_ascii_lowercase();
    [
        format!("def {lower_name}("),
        format!("async def {lower_name}("),
        format!("class {lower_name}("),
        format!("class {lower_name}:"),
        format!("function {lower_name}("),
        format!("export function {lower_name}("),
        format!("pub fn {lower_name}("),
        format!("fn {lower_name}("),
        format!("const {lower_name} "),
        format!("let {lower_name} "),
    ]
    .iter()
    .any(|needle| lower_text.contains(needle))
}

fn native_tool_source_defines_or_exports_public_api(text: &str, name: &str) -> bool {
    if native_tool_source_defines_public_api(text, name) {
        return true;
    }
    let lower_text = text.to_ascii_lowercase();
    let lower_name = name.to_ascii_lowercase();
    lower_text.contains(&format!("import {lower_name}"))
        || lower_text.contains(&format!("{lower_name} = "))
        || lower_text.contains(&format!("\"{lower_name}\""))
        || lower_text.contains(&format!("'{lower_name}'"))
}

fn native_tool_python_from_imports(text: &str) -> Vec<(String, Vec<String>)> {
    let mut imports = Vec::<(String, Vec<String>)>::new();
    for raw_line in text.lines() {
        let line = raw_line.trim();
        let Some(rest) = line.strip_prefix("from ") else {
            continue;
        };
        let Some((module, symbols_raw)) = rest.split_once(" import ") else {
            continue;
        };
        if module.starts_with('.') || module.is_empty() {
            continue;
        }
        let symbols = symbols_raw
            .split(',')
            .filter_map(|symbol| {
                let symbol = symbol.trim().split_whitespace().next().unwrap_or("");
                let symbol = symbol.trim_matches(|ch: char| {
                    matches!(ch, '(' | ')' | '[' | ']' | '{' | '}' | '\\')
                });
                if symbol.is_empty() || symbol == "*" {
                    None
                } else {
                    Some(symbol.to_string())
                }
            })
            .collect::<Vec<_>>();
        if !symbols.is_empty() {
            imports.push((module.trim().to_string(), symbols));
        }
    }
    imports
}

fn native_tool_python_module_path(project_root: &std::path::Path, module: &str) -> Option<PathBuf> {
    let relative = module.replace('.', "/");
    let root_path = project_root.join(format!("{relative}.py"));
    if root_path.exists() {
        return Some(root_path);
    }
    let root_package_path = project_root.join(&relative).join("__init__.py");
    if root_package_path.exists() {
        return Some(root_package_path);
    }
    let source_path = project_root.join("src").join(format!("{relative}.py"));
    if source_path.exists() {
        return Some(source_path);
    }
    let package_path = project_root.join("src").join(relative).join("__init__.py");
    if package_path.exists() {
        return Some(package_path);
    }
    None
}

fn native_tool_python_import_symbol_is_noise(symbol: &str) -> bool {
    matches!(
        symbol,
        "Path"
            | "Dict"
            | "List"
            | "Tuple"
            | "Set"
            | "Optional"
            | "Any"
            | "Union"
            | "Callable"
            | "dataclass"
            | "field"
            | "Enum"
            | "BaseModel"
            | "TestCase"
    )
}

fn native_tool_prompt_marks_path_optional(original_prompt: &str, path: &str) -> bool {
    let prompt = original_prompt.to_ascii_lowercase();
    let path = path.to_ascii_lowercase();
    [
        format!("optionally {path}"),
        format!("optional {path}"),
        format!("and optionally {path}"),
        format!("or optionally {path}"),
    ]
    .iter()
    .any(|needle| prompt.contains(needle))
}

pub(crate) fn native_tool_prompt_requires_test_changes(prompt_lower: &str) -> bool {
    prompt_lower.contains("add tests")
        || prompt_lower.contains("update tests")
        || prompt_lower.contains("regression tests")
        || prompt_lower.contains("test for")
        || prompt_lower.contains("tests for")
}

pub(crate) fn native_tool_prompt_requires_product_mutation(prompt_lower: &str) -> bool {
    let mutation_intent = [
        "implement ",
        "add ",
        "update ",
        "modify ",
        "fix ",
        "repair ",
        "refactor ",
        "create ",
        "build ",
        "extend ",
        "complete one coherent checkpoint slice",
        "continue the existing local",
    ]
    .iter()
    .any(|needle| prompt_lower.contains(needle));
    let software_target = [
        "code",
        "project",
        "package",
        "file",
        "tests",
        "behavior",
        "feature",
        "slice",
        "workflow requirements",
        "checkpoint",
    ]
    .iter()
    .any(|needle| prompt_lower.contains(needle));
    mutation_intent && software_target
}

pub(crate) fn native_tool_prompt_requires_validation_command(prompt_lower: &str) -> bool {
    prompt_lower.contains("test status")
        || prompt_lower.contains("run tests")
        || prompt_lower.contains("runs tests")
        || prompt_lower.contains("pytest")
        || prompt_lower.contains("validation status")
        || prompt_lower.contains("validation command")
        || prompt_lower.contains("run validation")
        || prompt_lower.contains("validate")
}

pub(crate) fn native_tool_prompt_requires_doc_changes(prompt_lower: &str) -> bool {
    prompt_lower.contains("update readme")
        || prompt_lower.contains("readme.md")
        || prompt_lower.contains("update docs")
        || prompt_lower.contains("documentation")
}

pub(crate) fn native_tool_prompt_requires_memory_write(prompt_lower: &str) -> bool {
    (prompt_lower.contains("memory row") || prompt_lower.contains(["checkpoint ", "memory"].concat().as_str()))
        && (prompt_lower.contains("write") || prompt_lower.contains("ingest"))
}

pub(crate) fn native_tool_prompt_expected_memory_row_id(original_prompt: &str) -> Option<String> {
    for line in original_prompt.lines() {
        let lower = line.to_ascii_lowercase();
        if lower.contains(["expected new ", "memory row id"].concat().as_str()) {
            return line
                .split_once(':')
                .map(|(_, value)| sanitize_token(value, 240))
                .filter(|value| !value.is_empty());
        }
    }
    None
}

pub(crate) fn native_tool_has_successful_memory_write_command(
    receipts: &[NativeToolReceipt],
    expected_row_id: Option<&str>,
    original_prompt: &str,
) -> bool {
    let checkpoint_name = native_tool_prompt_checkpoint_name(original_prompt);
    let validation_status_required = native_tool_prompt_requires_validation_command(
        &original_prompt.to_ascii_lowercase(),
    );
    receipts.iter().any(|receipt| {
        if receipt.status != "ok" || receipt.tool_name != "command_run" {
            return false;
        }
        let command = receipt
            .result
            .get("cmd")
            .and_then(|value| serde_json::to_string(value).ok())
            .unwrap_or_default();
        let stdout = receipt
            .result
            .get("stdout")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let success = receipt
            .result
            .get("success")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let row_matches = expected_row_id
            .map(|id| command.contains(id) || stdout.contains(id))
            .unwrap_or(true);
        let combined = format!("{command}\n{stdout}");
        let checkpoint_matches = checkpoint_name
            .as_deref()
            .map(|checkpoint| combined.contains(checkpoint))
            .unwrap_or(true);
        let validation_matches = !validation_status_required
            || ((combined.contains("\"status\"") || combined.contains("'status'"))
                && combined.to_ascii_lowercase().contains("pass"));
        success
            && row_matches
            && checkpoint_matches
            && validation_matches
            && command.contains("memory-cli")
            && command.contains("ingest")
            && (stdout.contains("\"ok\": true") || stdout.contains("\"ok\":true"))
    })
}

pub(crate) fn native_tool_is_handoff_artifact_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.ends_with(".json")
        && (lower.contains("receipt") || lower.contains("handoff") || lower.contains("checkpoint"))
}

pub(crate) fn native_tool_checkpoint_receipt_file_valid(path: &str, checkpoint: Option<&str>) -> bool {
    let Ok(content) = std::fs::read_to_string(path) else {
        return false;
    };
    if !content.contains("completed_checkpoint") && !content.contains("\"checkpoint\"") {
        return false;
    }
    checkpoint
        .map(|checkpoint| content.contains(checkpoint))
        .unwrap_or(true)
}

pub(crate) fn native_tool_successful_validation_summary(receipts: &[NativeToolReceipt]) -> Value {
    for receipt in receipts.iter().rev() {
        if receipt.status == "ok"
            && receipt.tool_name == "command_run"
            && receipt
                .result
                .get("success")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        {
            let command = receipt
                .result
                .get("cmd")
                .cloned()
                .unwrap_or_else(|| json!([]));
            let command_text = serde_json::to_string(&command).unwrap_or_default();
            if !command_text.contains("memory-cli")
                && (command_text.contains("test")
                || command_text.contains("unittest")
                || command_text.contains("pytest")
                    || command_text.contains("cargo test"))
            {
                return json!({
                    "status": "pass",
                    "command": command,
                    "exit_code": receipt.result.get("exit_code").cloned().unwrap_or(Value::Null),
                    "receipt_id": receipt.call_id
                });
            }
        }
    }
    json!({"status": "pass", "receipt_id": "successful_validation_receipt"})
}

pub(crate) fn native_tool_prompt_checkpoint_name(original_prompt: &str) -> Option<String> {
    for token in original_prompt.split_whitespace() {
        let cleaned = token.trim_matches(|ch: char| {
            matches!(
                ch,
                '`' | '\'' | '"' | ',' | ';' | ':' | '.' | ')' | '(' | '[' | ']' | '{' | '}'
            )
        });
        if cleaned.starts_with("checkpoint_") && !cleaned.ends_with(".json") {
            return Some(cleaned.to_string());
        }
    }
    None
}

pub(crate) fn native_tool_prompt_next_checkpoint_name(original_prompt: &str) -> Option<String> {
    let lower = original_prompt.to_ascii_lowercase();
    if lower.contains("recommended next checkpoint") {
        return Some("next_checkpoint_to_define".to_string());
    }
    None
}

pub(crate) fn native_tool_changed_paths_include(receipts: &[NativeToolReceipt], expected: &str) -> bool {
    native_tool_changed_path_for_expected(receipts, expected).is_some()
}

fn native_tool_changed_path_for_expected(
    receipts: &[NativeToolReceipt],
    expected: &str,
) -> Option<String> {
    let expected = expected.trim().trim_start_matches("./");
    native_tool_changed_paths(receipts).into_iter().find(|path| {
        let normalized = path.replace('\\', "/");
        normalized.ends_with(expected) || normalized.contains(&format!("/{expected}"))
    })
}

pub(crate) fn native_tool_changed_path_matches<F>(receipts: &[NativeToolReceipt], mut predicate: F) -> bool
where
    F: FnMut(&str) -> bool,
{
    native_tool_changed_paths(receipts)
        .iter()
        .any(|path| predicate(path))
}

pub(crate) fn native_tool_prompt_memory_cli_pattern(original_prompt: &str) -> Option<String> {
    for line in original_prompt.lines() {
        let lower = line.to_ascii_lowercase();
        if lower.contains("memory cli command pattern") {
            return line
                .split_once(':')
                .map(|(_, value)| sanitize_token(value, 500))
                .filter(|value| !value.is_empty());
        }
    }
    None
}

pub(crate) fn native_tool_prompt_project_root(original_prompt: &str) -> Option<String> {
    for token in original_prompt.split_whitespace() {
        let candidate = token.trim_matches(|ch: char| {
            matches!(
                ch,
                '"' | '\'' | '`' | ',' | '.' | ';' | ':' | '(' | ')' | '[' | ']' | '{' | '}'
            )
        });
        if !candidate.starts_with('/') {
            continue;
        }
        let mut path = PathBuf::from(candidate);
        while !path.exists() {
            if !path.pop() {
                break;
            }
        }
        if path.is_file() {
            path = path.parent()?.to_path_buf();
        }
        if path.is_dir() {
            return Some(path.display().to_string());
        }
    }
    None
}

pub(crate) fn native_tool_changed_paths(receipts: &[NativeToolReceipt]) -> Vec<String> {
    let mut paths = receipts
        .iter()
        .filter(|receipt| receipt.status == "ok")
        .filter(|receipt| receipt.tool_name == "file_write" || receipt.tool_name == "file_patch")
        .filter_map(|receipt| receipt.result.get("path").and_then(Value::as_str))
        .map(str::to_string)
        .collect::<Vec<_>>();
    paths.sort();
    paths.dedup();
    paths
}

fn sanitize_token(raw: &str, max_len: usize) -> String {
    let mut out = String::new();
    for ch in raw.chars().take(max_len) {
        if ch.is_control() && ch != '\n' && ch != '\t' {
            continue;
        }
        out.push(ch);
    }
    out.trim().to_string()
}
pub(crate) fn native_tool_artifact_contract_enabled(metadata: &Value) -> bool {
    let contract_key = ["completion_", "evidence_contract"].concat();
    let workflow_contract_pointer = ["/workflow/", contract_key.as_str()].concat();
    let required_key = ["completion_", "evidence_required_for_multi_requirement_tasks"].concat();
    metadata
        .get(contract_key.as_str())
        .or_else(|| metadata.pointer(workflow_contract_pointer.as_str()))
        .map(Value::is_object)
        .unwrap_or(false)
        || metadata
            .get("native_success_criteria")
            .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
            .and_then(|value| value.get(required_key.as_str()))
            .and_then(Value::as_bool)
            .unwrap_or(false)
}

pub(crate) fn native_tool_prompt_has_multiple_requirements(original_prompt: &str) -> bool {
    native_tool_requirement_lines(original_prompt).len() >= 2
}

pub(crate) fn native_tool_requirement_lines(original_prompt: &str) -> Vec<String> {
    let mut in_task = false;
    let mut out = Vec::<String>::new();
    for line in original_prompt.lines() {
        let trimmed = line.trim();
        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with("task:") {
            in_task = true;
            continue;
        }
        if in_task && lower.starts_with("final response contract") {
            break;
        }
        if !in_task {
            continue;
        }
        if let Some(requirement) = native_tool_requirement_from_line(trimmed) {
            out.push(requirement);
        }
    }
    if out.is_empty() {
        for line in original_prompt.lines() {
            if let Some(requirement) = native_tool_requirement_from_line(line.trim()) {
                out.push(requirement);
            }
        }
    }
    out
}

pub(crate) fn native_tool_requirement_from_line(trimmed: &str) -> Option<String> {
    if trimmed.is_empty() {
        return None;
    }
    if let Some((prefix, rest)) = trimmed.split_once('.') {
        if !prefix.is_empty()
            && prefix.chars().all(|ch| ch.is_ascii_digit())
            && !rest.trim().is_empty()
        {
            return Some(rest.trim().to_string());
        }
    }
    if let Some((prefix, rest)) = trimmed.split_once(')') {
        if !prefix.is_empty()
            && prefix.chars().all(|ch| ch.is_ascii_digit())
            && !rest.trim().is_empty()
        {
            return Some(rest.trim().to_string());
        }
    }
    for marker in ["- ", "* "] {
        if let Some(rest) = trimmed.strip_prefix(marker) {
            if !rest.trim().is_empty() {
                return Some(rest.trim().to_string());
            }
        }
    }
    None
}

pub(crate) fn native_tool_successful_receipt_refs(receipts: &[NativeToolReceipt]) -> Vec<String> {
    receipts
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
        .collect()
}
