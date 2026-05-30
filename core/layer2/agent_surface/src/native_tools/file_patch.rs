use crate::native_tools::export_guard::ensure_no_export_removal;
use crate::native_tools::hashing::sha256_hex;
use crate::native_tools::paths::required_abs_path;
use serde_json::{json, Value};
use std::fs;

pub fn file_patch(args: &Value) -> Result<Value, String> {
    let path = required_abs_path(args)?;
    let content =
        fs::read_to_string(&path).map_err(|error| format!("file_patch_read_failed:{error}"))?;
    if args.get("old").is_none()
        && args.get("find").is_none()
        && args.get("search").is_none()
        && args.get("before").is_none()
        && args.get("original").is_none()
    {
        if let Some(patch) = args
            .get("patch")
            .or_else(|| args.get("patch_content"))
            .or_else(|| args.get("diff"))
            .and_then(Value::as_str)
        {
            let previous_hash = sha256_hex(content.as_bytes());
            let patched = apply_unified_patch_payload(&content, patch)?;
            ensure_no_export_removal(&path, &content, &patched, args)?;
            fs::write(&path, &patched)
                .map_err(|error| format!("file_patch_write_failed:{error}"))?;
            return Ok(json!({
                "path": path.display().to_string(),
                "replacement_count": 1,
                "patch_format": "unified_diff",
                "previous_content_hash": previous_hash,
                "new_content_hash": sha256_hex(patched.as_bytes()),
            }));
        }
    }
    let old = args
        .get("old")
        .or_else(|| args.get("find"))
        .or_else(|| args.get("search"))
        .or_else(|| args.get("before"))
        .or_else(|| args.get("original"))
        .and_then(Value::as_str)
        .ok_or_else(|| "old_required".to_string())?;
    let new = args
        .get("new")
        .or_else(|| args.get("replace"))
        .or_else(|| args.get("replacement"))
        .or_else(|| args.get("after"))
        .or_else(|| args.get("updated"))
        .and_then(Value::as_str)
        .ok_or_else(|| "new_required".to_string())?;
    let allow_multiple = args
        .get("allow_multiple")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let count = content.matches(old).count();
    if count == 0 {
        return Err("patch_old_text_not_found".to_string());
    }
    if count > 1 && !allow_multiple {
        return Err("patch_old_text_not_unique".to_string());
    }
    let previous_hash = sha256_hex(content.as_bytes());
    let patched = if allow_multiple {
        content.replace(old, new)
    } else {
        content.replacen(old, new, 1)
    };
    ensure_no_export_removal(&path, &content, &patched, args)?;
    fs::write(&path, &patched).map_err(|error| format!("file_patch_write_failed:{error}"))?;
    Ok(json!({
        "path": path.display().to_string(),
        "replacement_count": if allow_multiple { count } else { 1 },
        "previous_content_hash": previous_hash,
        "new_content_hash": sha256_hex(patched.as_bytes()),
    }))
}

fn apply_unified_patch_payload(content: &str, patch: &str) -> Result<String, String> {
    let mut lines = content.lines().map(ToOwned::to_owned).collect::<Vec<_>>();
    let mut hunk_starts = Vec::new();
    for (idx, line) in patch.lines().enumerate() {
        if line.starts_with("@@") {
            hunk_starts.push(idx);
        }
    }
    if hunk_starts.is_empty() {
        return Err("patch_old_text_not_found".to_string());
    }
    let patch_lines = patch.lines().collect::<Vec<_>>();
    let mut offset: isize = 0;
    for (hunk_idx, start_idx) in hunk_starts.iter().enumerate() {
        let header = patch_lines[*start_idx];
        let (old_start, old_count) =
            parse_unified_hunk_header(header).ok_or_else(|| "patch_hunk_header_invalid".to_string())?;
        let end_idx = hunk_starts
            .get(hunk_idx + 1)
            .copied()
            .unwrap_or(patch_lines.len());
        let mut old_hunk = Vec::<String>::new();
        let mut new_hunk = Vec::<String>::new();
        for line in &patch_lines[*start_idx + 1..end_idx] {
            if line.starts_with("diff --git ") || line.starts_with("--- ") || line.starts_with("+++ ") {
                continue;
            }
            if let Some(text) = line.strip_prefix(' ') {
                old_hunk.push(text.to_string());
                new_hunk.push(text.to_string());
            } else if let Some(text) = line.strip_prefix('-') {
                old_hunk.push(text.to_string());
            } else if let Some(text) = line.strip_prefix('+') {
                new_hunk.push(text.to_string());
            }
        }
        let header_index = ((old_start.saturating_sub(1)) as isize + offset).max(0) as usize;
        let (replace_start, replace_len) =
            if !old_hunk.is_empty() && old_hunk.len() >= old_count.min(lines.len()) {
                let Some(found) = find_line_sequence(&lines, &old_hunk) else {
                    return Err("patch_old_text_not_found".to_string());
                };
                (found, old_hunk.len())
            } else {
                let replace_start = header_index.min(lines.len());
                let replace_end = replace_start.saturating_add(old_count).min(lines.len());
                (replace_start, replace_end.saturating_sub(replace_start))
            };
        lines.splice(replace_start..replace_start + replace_len, new_hunk.clone());
        offset += new_hunk.len() as isize - replace_len as isize;
    }
    let mut patched = lines.join("\n");
    if content.ends_with('\n') {
        patched.push('\n');
    }
    Ok(patched)
}

fn parse_unified_hunk_header(header: &str) -> Option<(usize, usize)> {
    let old_spec = header.split_whitespace().find(|part| part.starts_with('-'))?;
    let old_spec = old_spec.trim_start_matches('-');
    let (start, count) = old_spec
        .split_once(',')
        .map(|(start, count)| (start, count))
        .unwrap_or((old_spec, "1"));
    Some((start.parse().ok()?, count.parse().ok()?))
}

fn find_line_sequence(lines: &[String], needle: &[String]) -> Option<usize> {
    if needle.is_empty() || needle.len() > lines.len() {
        return None;
    }
    lines.windows(needle.len()).position(|window| window == needle)
}
