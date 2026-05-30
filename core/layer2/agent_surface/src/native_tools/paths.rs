use serde_json::Value;
use std::path::PathBuf;

pub fn required_abs_path(args: &Value) -> Result<PathBuf, String> {
    let path = args
        .get("path")
        .or_else(|| args.get("file_path"))
        .or_else(|| args.get("filepath"))
        .or_else(|| args.get("target_path"))
        .or_else(|| args.get("target_file_path"))
        .or_else(|| args.get("target"))
        .or_else(|| args.get("file"))
        .or_else(|| args.get("absolute_path"))
        .or_else(|| args.get("full_path"))
        .or_else(|| args.get("output_path"))
        .or_else(|| args.get("destination"))
        .or_else(|| args.get("dest"))
        .or_else(|| args.get("filename"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "path_required".to_string())?;
    let path = PathBuf::from(path);
    if !path.is_absolute() {
        return Err("absolute_path_required".to_string());
    }
    Ok(path)
}
