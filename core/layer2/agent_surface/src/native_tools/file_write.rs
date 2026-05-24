// Layer ownership: Core Layer 2 (Scheduling + Execution) - agent runtime native tool file mutation.
use crate::native_tools::export_guard::{
    ensure_no_export_removal, preserve_removed_python_exports,
};
use crate::native_tools::hashing::sha256_hex;
use crate::native_tools::paths::required_abs_path;
use serde_json::{json, Value};
use std::fs;

pub fn file_write(args: &Value) -> Result<Value, String> {
    let path = required_abs_path(args)?;
    let content = args
        .get("content")
        .or_else(|| args.get("contents"))
        .or_else(|| args.get("text"))
        .or_else(|| args.get("body"))
        .or_else(|| args.get("data"))
        .and_then(Value::as_str)
        .ok_or_else(|| "content_required".to_string())?;
    let existed = path.exists();
    let overwrite = args
        .get("overwrite")
        .and_then(Value::as_bool)
        .unwrap_or(existed);
    if existed && !overwrite {
        return Err("overwrite_permission_required".to_string());
    }
    let previous_bytes = if existed {
        Some(fs::read(&path).map_err(|error| format!("pre_write_snapshot_failed:{error}"))?)
    } else {
        None
    };
    let mut content = content.to_string();
    if let Some(previous_bytes) = previous_bytes.as_ref() {
        if let Ok(previous_text) = std::str::from_utf8(previous_bytes) {
            content = preserve_removed_python_exports(&path, previous_text, &content, args);
            ensure_no_export_removal(&path, previous_text, &content, args)?;
        }
    }
    let previous_hash = previous_bytes.as_ref().map(|bytes| sha256_hex(bytes));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("parent_create_failed:{error}"))?;
    }
    fs::write(&path, content).map_err(|error| format!("file_write_failed:{error}"))?;
    let bytes = fs::read(&path).map_err(|error| format!("post_write_read_failed:{error}"))?;
    Ok(json!({
        "path": path.display().to_string(),
        "created": !existed,
        "overwritten": existed,
        "previous_content_hash": previous_hash,
        "new_content_hash": sha256_hex(&bytes),
        "bytes_written": bytes.len(),
    }))
}
