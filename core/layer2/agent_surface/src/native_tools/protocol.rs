use crate::native_tools::receipts::NativeToolReceipt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NativeToolCall {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub args: Value,
}

pub fn parse_native_tool_calls(raw: &str) -> Vec<NativeToolCall> {
    let cleaned = strip_ansi(raw);
    let candidates = json_candidates(&cleaned);
    let mut placeholder_fallback = Vec::new();
    for candidate in candidates {
        if let Ok(value) = serde_json::from_str::<Value>(&candidate) {
            let calls = tool_calls_from_value(&value);
            if calls.is_empty() {
                continue;
            }
            let executable_calls = calls
                .iter()
                .filter(|call| !native_tool_call_has_placeholder_args(call))
                .cloned()
                .collect::<Vec<_>>();
            if !executable_calls.is_empty() {
                return executable_calls;
            }
            if placeholder_fallback.is_empty() {
                placeholder_fallback = calls;
            }
        }
    }
    placeholder_fallback
}

pub fn native_tool_observation_prompt(receipts: &[NativeToolReceipt]) -> String {
    json!({
        "native_tool_observations": receipts,
        "instruction": "Use these receipts as authoritative. Continue with another tool call if needed, otherwise provide the final answer."
    })
    .to_string()
}

fn tool_calls_from_value(value: &Value) -> Vec<NativeToolCall> {
    if let Some(items) = value.get("tool_calls").and_then(Value::as_array) {
        return items
            .iter()
            .enumerate()
            .flat_map(|(idx, item)| tool_calls_from_item(item, idx))
            .collect();
    }
    if let Some(items) = value.get("actions").and_then(Value::as_array) {
        let calls = items
            .iter()
            .enumerate()
            .filter_map(|(idx, item)| action_call_from_item(item, idx))
            .collect::<Vec<_>>();
        if !calls.is_empty() {
            return calls;
        }
    }
    if value.get("tool").is_some() || value.get("name").is_some() {
        return tool_calls_from_item(value, 0);
    }
    Vec::new()
}

fn action_call_from_item(value: &Value, idx: usize) -> Option<NativeToolCall> {
    let command = value
        .get("command")
        .or_else(|| value.get("cmd"))
        .and_then(Value::as_str)?
        .trim();
    if command.is_empty() || matches!(command, "..." | "…") {
        return None;
    }
    let id = value
        .get("id")
        .or_else(|| value.get("call_id"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| format!("action_{}", idx + 1));
    Some(NativeToolCall {
        id,
        name: "command_run".to_string(),
        args: json!({
            "cmd": ["sh", "-lc", command],
        }),
    })
}

fn tool_calls_from_item(value: &Value, idx: usize) -> Vec<NativeToolCall> {
    let Some(call) = tool_call_from_value(value, idx) else {
        return Vec::new();
    };
    expanded_bulk_tool_calls(call)
}

fn tool_call_from_value(value: &Value, idx: usize) -> Option<NativeToolCall> {
    let name = value
        .get("name")
        .or_else(|| value.get("tool"))
        .or_else(|| value.get("tool_name"))
        .or_else(|| value.pointer("/function/name"))
        .and_then(Value::as_str)?
        .trim()
        .to_string();
    let id = value
        .get("id")
        .or_else(|| value.get("call_id"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| format!("call_{}", idx + 1));
    let args = tool_call_args_from_value(value);
    Some(NativeToolCall { id, name, args })
}

fn expanded_bulk_tool_calls(call: NativeToolCall) -> Vec<NativeToolCall> {
    let name = call.name.trim().to_ascii_lowercase();
    if matches!(
        name.as_str(),
        "file_write" | "write_file" | "workspace.write" | "workspace_write"
    ) {
        if let Some(items) = call.args.get("files").and_then(Value::as_array) {
            return items
                .iter()
                .enumerate()
                .filter_map(|(idx, item)| {
                    let mut args = normalize_tool_args(item);
                    if args.is_object() {
                        if let Some(overwrite) = call.args.get("overwrite") {
                            args.as_object_mut()?
                                .entry("overwrite".to_string())
                                .or_insert_with(|| overwrite.clone());
                        }
                    }
                    Some(NativeToolCall {
                        id: format!("{}_{}", call.id, idx + 1),
                        name: call.name.clone(),
                        args,
                    })
                })
                .collect();
        }
    }
    if matches!(
        name.as_str(),
        "file_patch" | "patch_file" | "workspace.patch" | "workspace_patch"
    ) {
        for key in ["patches", "edits", "replacements"] {
            if let Some(items) = call.args.get(key).and_then(Value::as_array) {
                return items
                    .iter()
                    .enumerate()
                    .map(|(idx, item)| NativeToolCall {
                        id: format!("{}_{}", call.id, idx + 1),
                        name: call.name.clone(),
                        args: normalize_tool_args(item),
                    })
                    .collect();
            }
        }
    }
    vec![call]
}

fn tool_call_args_from_value(value: &Value) -> Value {
    let explicit_args = value
        .get("args")
        .or_else(|| value.get("arguments"))
        .or_else(|| value.get("input"))
        .or_else(|| value.get("parameters"))
        .or_else(|| value.get("params"))
        .or_else(|| value.get("payload"))
        .or_else(|| value.get("data"))
        .or_else(|| value.pointer("/function/arguments"));
    if let Some(args) = explicit_args {
        return normalize_tool_args(args);
    }
    let mut args = Map::new();
    for key in [
        "path",
        "file_path",
        "filepath",
        "target_path",
        "target",
        "file",
        "absolute_path",
        "full_path",
        "output_path",
        "destination",
        "dest",
        "filename",
        "files",
        "paths",
        "content",
        "contents",
        "text",
        "body",
        "overwrite",
        "old",
        "find",
        "search",
        "before",
        "original",
        "new",
        "replace",
        "replacement",
        "after",
        "updated",
        "patches",
        "edits",
        "replacements",
        "allow_multiple",
        "recursive",
        "max_entries",
        "cwd",
        "working_directory",
        "working_dir",
        "workdir",
        "directory",
        "dir",
        "project_root",
        "root",
        "cmd",
        "command",
        "resolved_command",
        "command_resolution",
        "resolution",
        "require_command_resolution",
        "command_resolution_required",
        "intent",
        "tool_intent",
        "tool_id",
        "preferred_tool",
        "binary",
        "executable",
        "candidate_binaries",
        "candidate_binary",
        "binary_names",
        "binary_name",
        "executables",
        "executable_names",
        "configured_paths",
        "configured_path",
        "candidate_paths",
        "candidate_path",
        "workspace_binaries",
        "workspace_binary",
        "allowed_execution_modes",
        "allowed_modes",
        "forbidden_execution_modes",
        "forbidden_modes",
        "preferred_execution_mode",
        "execution_mode",
        "policy",
        "execution_policy",
        "timing_comparable_required",
        "cargo_run_command",
        "cargo_command",
        "dev_fallback_command",
        "fallback_command",
        "cargo_package",
        "cargo_manifest_path",
        "cargo_bin",
        "build_command",
        "build_cmd",
        "produced_executable",
        "built_executable",
        "workspace_binary_after_build",
        "env",
        "timeout_seconds",
        "max_output_bytes",
        "start_line",
        "end_line",
    ] {
        if let Some(value) = value.get(key) {
            args.insert(key.to_string(), value.clone());
        }
    }
    Value::Object(args)
}

fn native_tool_call_has_placeholder_args(call: &NativeToolCall) -> bool {
    let placeholder_strings = [
        "/absolute/path",
        "absolute/path",
        "/path/to/file",
        "path/to/file",
        "exact observed text",
        "replacement text",
    ];
    for value in native_tool_call_arg_strings(&call.args) {
        let normalized = value.trim().to_ascii_lowercase();
        if placeholder_strings
            .iter()
            .any(|placeholder| normalized == *placeholder)
        {
            return true;
        }
    }
    false
}

fn native_tool_call_arg_strings(value: &Value) -> Vec<String> {
    let mut out = Vec::new();
    native_tool_call_arg_strings_into(value, &mut out);
    out
}

fn native_tool_call_arg_strings_into(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::String(raw) => out.push(raw.clone()),
        Value::Array(items) => {
            for item in items {
                native_tool_call_arg_strings_into(item, out);
            }
        }
        Value::Object(object) => {
            for value in object.values() {
                native_tool_call_arg_strings_into(value, out);
            }
        }
        _ => {}
    }
}

fn normalize_tool_args(args: &Value) -> Value {
    if let Some(raw) = args.as_str() {
        return serde_json::from_str::<Value>(raw).unwrap_or_else(|_| json!({}));
    }
    args.clone()
}

fn json_candidates(raw: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = raw;
    while let Some(start) = rest.find("```") {
        rest = &rest[start + 3..];
        if let Some(newline) = rest.find('\n') {
            rest = &rest[newline + 1..];
        }
        let Some(end) = rest.find("```") else {
            break;
        };
        out.push(rest[..end].trim().to_string());
        rest = &rest[end + 3..];
    }
    out.extend(balanced_objects(raw));
    out.push(raw.trim().to_string());
    out
}

fn balanced_objects(raw: &str) -> Vec<String> {
    let mut objects = Vec::new();
    let mut search_start = 0usize;
    while search_start < raw.len() && objects.len() < 64 {
        let Some((start, object)) = balanced_object_from(raw, search_start) else {
            break;
        };
        search_start = start + object.len().max(1);
        objects.push(object);
    }
    objects
}

fn balanced_object_from(raw: &str, search_start: usize) -> Option<(usize, String)> {
    let mut start = None;
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;
    for (offset, ch) in raw[search_start..].char_indices() {
        let idx = search_start + offset;
        if start.is_none() {
            if ch == '{' {
                start = Some(idx);
                depth = 1;
            }
            continue;
        }
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return start.map(|start_idx| (start_idx, raw[start_idx..=idx].to_string()));
                }
            }
            _ => {}
        }
    }
    None
}

fn strip_ansi(raw: &str) -> String {
    let mut out = String::new();
    let mut chars = raw.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            for next in chars.by_ref() {
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(ch);
        }
    }
    out
}
