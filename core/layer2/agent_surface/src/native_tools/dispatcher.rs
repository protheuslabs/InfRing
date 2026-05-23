use crate::native_tools::command_run::command_run;
use crate::native_tools::file_discovery::{file_list, file_stat};
use crate::native_tools::file_patch::file_patch;
use crate::native_tools::file_read::{file_read, file_read_many};
use crate::native_tools::file_write::file_write;
use crate::native_tools::protocol::NativeToolCall;
use crate::native_tools::receipts::NativeToolReceipt;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::time::Instant;

#[derive(Clone, Debug)]
pub struct NativeToolDispatcher {
    allowed_tools: BTreeSet<String>,
}

impl NativeToolDispatcher {
    pub fn new(tools: &[String]) -> Self {
        Self {
            allowed_tools: tools.iter().map(|tool| normalize_tool_name(tool)).collect(),
        }
    }

    pub fn has_native_tools(&self) -> bool {
        self.allowed_tools.iter().any(|tool| {
            matches!(
                tool.as_str(),
                "file_list"
                    | "file_stat"
                    | "file_read"
                    | "file_read_many"
                    | "file_write"
                    | "file_patch"
                    | "command_run"
            )
        })
    }

    pub fn tool_protocol_prompt(&self) -> String {
        let mut tools = self.allowed_tools.iter().cloned().collect::<Vec<_>>();
        tools.sort();
        format!(
            "Native Infring local coding tools are available when needed. To call tools, return only JSON in this shape: {{\"tool_calls\":[{{\"id\":\"call_1\",\"name\":\"file_list|file_stat|file_read|file_read_many|file_write|file_patch|command_run\",\"args\":{{...}}}}]}}. Supported tools for this run: {}. file_list args: {{\"path\":\"/absolute/directory\",\"recursive\":false,\"max_entries\":200}}. file_stat args: {{\"path\":\"/absolute/path\"}}. file_read args: {{\"path\":\"/absolute/path\",\"start_line\":1,\"end_line\":20}}. file_write args: {{\"path\":\"/absolute/path\",\"content\":\"text\",\"overwrite\":false}}. file_patch args: {{\"path\":\"/absolute/path\",\"old\":\"exact text\",\"new\":\"replacement text\",\"allow_multiple\":false}}. command_run args: {{\"cwd\":\"/absolute/project\",\"cmd\":[\"python3\",\"-m\",\"pytest\",\"-q\"],\"timeout_seconds\":120,\"max_output_bytes\":12000}}. Use file_list/file_stat to discover what exists before reading likely target files. For existing-project code edits, read the likely source owner file and adjacent tests before mutating when they exist. When adding or changing tests for a public API, mutate the source owner module that defines or exports that API before validation; a test-only change is not a complete implementation. If validation reports an import, undefined-name, or missing-symbol failure, repair the owner source module or test import directly, then rerun validation. Use command_run after edits when tests, validation, or test status are requested; inspect failures and patch before finalizing. After tool results, either call more tools or return final text. Do not claim a file operation succeeded unless a tool receipt says status=ok, and do not claim validation passed unless command_run returns success=true.",
            tools.join(", ")
        )
    }

    pub fn dispatch(&self, call: NativeToolCall) -> NativeToolReceipt {
        let started = Instant::now();
        let tool_name = normalize_tool_name(&call.name);
        let result = if self.allowed_tools.contains(&tool_name) {
            dispatch_allowed_tool(&tool_name, &call.args)
        } else {
            Err(format!("native_tool_not_allowed:{tool_name}"))
        };
        let duration_ms = started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
        match result {
            Ok(result) => NativeToolReceipt {
                call_id: call.id,
                tool_name,
                status: "ok".to_string(),
                duration_ms,
                result,
                error: None,
            },
            Err(error) => NativeToolReceipt {
                call_id: call.id,
                tool_name,
                status: "error".to_string(),
                duration_ms,
                result: native_tool_error_diagnostics(&call.args),
                error: Some(error),
            },
        }
    }
}

fn native_tool_error_diagnostics(args: &Value) -> Value {
    let Some(object) = args.as_object() else {
        return json!({
            "args_type": value_kind(args),
        });
    };

    json!({
        "args_keys": object.keys().cloned().collect::<Vec<_>>(),
        "path": object
            .get("path")
            .or_else(|| object.get("file_path"))
            .or_else(|| object.get("filepath"))
            .or_else(|| object.get("target_path"))
            .or_else(|| object.get("target"))
            .or_else(|| object.get("file"))
            .or_else(|| object.get("output_path"))
            .or_else(|| object.get("destination"))
            .or_else(|| object.get("dest"))
            .or_else(|| object.get("filename"))
            .cloned()
            .unwrap_or(Value::Null),
        "paths": object
            .get("paths")
            .or_else(|| object.get("files"))
            .cloned()
            .unwrap_or(Value::Null),
        "command": object
            .get("cmd")
            .or_else(|| object.get("command"))
            .cloned()
            .unwrap_or(Value::Null),
    })
}

fn value_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn dispatch_allowed_tool(tool_name: &str, args: &Value) -> Result<Value, String> {
    match tool_name {
        "file_list" => file_list(args),
        "file_stat" => file_stat(args),
        "file_read" => file_read(args),
        "file_read_many" => file_read_many(args),
        "file_write" => file_write(args),
        "file_patch" => file_patch(args),
        "command_run" => command_run(args),
        _ => Err(format!("native_tool_not_implemented:{tool_name}")),
    }
}

fn normalize_tool_name(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "list_files" | "workspace.list" | "workspace_list" => "file_list".to_string(),
        "stat_file" | "file_exists" | "workspace.stat" | "workspace_stat" => {
            "file_stat".to_string()
        }
        "read_file" | "workspace.read" | "workspace_read" => "file_read".to_string(),
        "read_many_files" | "workspace.read_many" | "workspace_read_many" => {
            "file_read_many".to_string()
        }
        "write_file" | "workspace.write" | "workspace_write" => "file_write".to_string(),
        "apply_patch" | "patch_file" | "workspace.patch" | "workspace_patch" => {
            "file_patch".to_string()
        }
        "run_command" | "command.run" | "command_run" | "shell.run" | "shell_run" => {
            "command_run".to_string()
        }
        other => other.to_string(),
    }
}
