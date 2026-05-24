// Layer ownership: Core Layer 2 (Scheduling + Execution) - agent runtime native tool safety guard.
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub fn ensure_no_export_removal(
    path: &Path,
    previous: &str,
    next: &str,
    args: &Value,
) -> Result<(), String> {
    if args
        .get("allow_export_removal")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return Ok(());
    }
    if path.extension().and_then(|value| value.to_str()) != Some("py") {
        return Ok(());
    }
    let previous_exports = python_top_level_exports(previous);
    if previous_exports.is_empty() {
        return Ok(());
    }
    let next_exports = python_top_level_exports(next);
    let removed = previous_exports
        .difference(&next_exports)
        .cloned()
        .collect::<Vec<_>>();
    if removed.is_empty() {
        return Ok(());
    }
    Err(format!(
        "export_removal_requires_explicit_allowlist:{}",
        removed.join(",")
    ))
}

pub fn preserve_removed_python_exports(
    path: &Path,
    previous: &str,
    next: &str,
    args: &Value,
) -> String {
    if args
        .get("allow_export_removal")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return next.to_string();
    }
    if args.get("preserve_exports").and_then(Value::as_bool) == Some(false) {
        return next.to_string();
    }
    if path.extension().and_then(|value| value.to_str()) != Some("py") {
        return next.to_string();
    }
    let previous_blocks = python_top_level_export_blocks(previous);
    if previous_blocks.is_empty() {
        return next.to_string();
    }
    let next_exports = python_top_level_exports(next);
    let missing = previous_blocks
        .keys()
        .filter(|name| !next_exports.contains(*name))
        .cloned()
        .collect::<Vec<_>>();
    if missing.is_empty() {
        return next.to_string();
    }

    let mut merged = next.trim_end().to_string();
    merged.push_str("\n\n# Preserved public exports from the previous file version.\n");
    for name in missing {
        if let Some(block) = previous_blocks.get(&name) {
            merged.push_str(block.trim_end());
            merged.push_str("\n\n");
        }
    }
    merged
}

fn python_top_level_exports(content: &str) -> BTreeSet<String> {
    let mut exports = BTreeSet::new();
    for line in content.lines() {
        if line.starts_with(' ') || line.starts_with('\t') {
            continue;
        }
        let Some(rest) = line
            .strip_prefix("class ")
            .or_else(|| line.strip_prefix("def "))
        else {
            continue;
        };
        let name = rest
            .split(|ch: char| ch == '(' || ch == ':' || ch.is_whitespace())
            .next()
            .unwrap_or("")
            .trim();
        if !name.is_empty() && !name.starts_with('_') {
            exports.insert(name.to_string());
        }
    }
    exports
}

fn python_top_level_export_blocks(content: &str) -> BTreeMap<String, String> {
    let lines = content.lines().collect::<Vec<_>>();
    let mut starts = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if line.starts_with(' ') || line.starts_with('\t') {
            continue;
        }
        let Some(name) = python_export_name(line) else {
            continue;
        };
        let mut start = index;
        while start > 0 {
            let previous = lines[start - 1];
            if previous.starts_with('@') {
                start -= 1;
                continue;
            }
            break;
        }
        starts.push((start, index, name));
    }

    let mut blocks = BTreeMap::new();
    for (position, (start, _, name)) in starts.iter().enumerate() {
        let end = starts
            .get(position + 1)
            .map(|(next_start, _, _)| *next_start)
            .unwrap_or(lines.len());
        let block = lines[*start..end].join("\n");
        blocks.insert(name.clone(), block);
    }
    blocks
}

fn python_export_name(line: &str) -> Option<String> {
    let rest = line
        .strip_prefix("class ")
        .or_else(|| line.strip_prefix("def "))?;
    let name = rest
        .split(|ch: char| ch == '(' || ch == ':' || ch.is_whitespace())
        .next()
        .unwrap_or("")
        .trim();
    if name.is_empty() || name.starts_with('_') {
        None
    } else {
        Some(name.to_string())
    }
}
