// Layer ownership: eval/observability (ForgeCode-derived coding safety behavior measurement).
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize)]
pub struct CodingSafetyLayerLabReport {
    pub harness_kind: &'static str,
    pub ok: bool,
    pub sandbox_root: String,
    pub checks: Vec<CodingSafetyLayerCheck>,
    pub read_receipts: Vec<SafeFileReadReceipt>,
    pub write_receipts: Vec<SafeFileWriteReceipt>,
    pub patch_receipts: Vec<SafeFilePatchReceipt>,
    pub command_receipts: Vec<ValidationCommandReceipt>,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CodingSafetyLayerCheck {
    pub id: &'static str,
    pub ok: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SafeFileReadReceipt {
    pub path: String,
    pub start_line: u64,
    pub end_line: u64,
    pub total_lines: u64,
    pub content_hash: String,
    pub bytes_read: u64,
    pub content: String,
    pub truncation_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SafeFileWriteReceipt {
    pub path: String,
    pub created_or_overwritten: &'static str,
    pub overwrite_allowed: bool,
    pub snapshot_ref: Option<String>,
    pub previous_content_hash: Option<String>,
    pub new_content_hash: String,
    pub validation_errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SafeFilePatchReceipt {
    pub path: String,
    pub operation: &'static str,
    pub match_status: &'static str,
    pub snapshot_ref: Option<String>,
    pub previous_content_hash: String,
    pub new_content_hash: String,
    pub validation_errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidationCommandReceipt {
    pub command: String,
    pub cwd: String,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub status: &'static str,
    pub description: Option<String>,
}

pub fn coding_safety_layer_lab_report() -> CodingSafetyLayerLabReport {
    let sandbox_root = lab_sandbox_root_path();
    let snapshot_root = sandbox_root.join(".safety_snapshots");
    let mut checks = Vec::new();
    let mut read_receipts = Vec::new();
    let mut write_receipts = Vec::new();
    let mut patch_receipts = Vec::new();
    let mut command_receipts = Vec::new();
    let mut failures = Vec::new();

    record_check(
        &mut checks,
        "sandbox_created",
        fs::create_dir_all(sandbox_root.join("src")).map_err(|error| error.to_string()),
    );
    record_check(
        &mut checks,
        "snapshot_dir_created",
        fs::create_dir_all(&snapshot_root).map_err(|error| error.to_string()),
    );

    let existing_path = sandbox_root.join("src/existing.rs");
    let seed = "pub fn value() -> &'static str {\n    \"before\"\n}\n";
    record_check(
        &mut checks,
        "fixture_seeded",
        fs::write(&existing_path, seed).map_err(|error| error.to_string()),
    );

    match safe_file_read(&existing_path, Some(1), Some(2), 1_048_576, 2_000, 120) {
        Ok(receipt) => {
            checks.push(CodingSafetyLayerCheck {
                id: "safe_file_read_hash_range_receipt",
                ok: receipt.start_line == 1
                    && receipt.end_line == 2
                    && receipt.total_lines == 3
                    && !receipt.content_hash.is_empty(),
                detail: format!(
                    "read {}:{}-{} hash={}",
                    receipt.path, receipt.start_line, receipt.end_line, receipt.content_hash
                ),
            });
            read_receipts.push(receipt);
        }
        Err(error) => failures.push(format!("safe_file_read_failed:{error}")),
    }

    let new_path = sandbox_root.join("src/generated.rs");
    match safe_file_write(
        &new_path,
        "pub fn generated() -> &'static str {\n    \"safe\"\n}\n",
        false,
        &snapshot_root,
    ) {
        Ok(receipt) => {
            checks.push(CodingSafetyLayerCheck {
                id: "safe_file_write_new_file_hash_receipt",
                ok: receipt.created_or_overwritten == "created"
                    && receipt.snapshot_ref.is_none()
                    && !receipt.new_content_hash.is_empty(),
                detail: format!("wrote {} hash={}", receipt.path, receipt.new_content_hash),
            });
            write_receipts.push(receipt);
        }
        Err(error) => failures.push(format!("safe_file_write_new_failed:{error}")),
    }

    match safe_file_write(
        &existing_path,
        "pub fn value() -> &'static str {\n    \"after\"\n}\n",
        true,
        &snapshot_root,
    ) {
        Ok(receipt) => {
            checks.push(CodingSafetyLayerCheck {
                id: "safe_file_write_overwrite_snapshot_receipt",
                ok: receipt.created_or_overwritten == "overwritten"
                    && receipt.snapshot_ref.is_some()
                    && receipt.previous_content_hash.is_some(),
                detail: format!(
                    "overwrote {} snapshot={}",
                    receipt.path,
                    receipt.snapshot_ref.clone().unwrap_or_default()
                ),
            });
            write_receipts.push(receipt);
        }
        Err(error) => failures.push(format!("safe_file_write_overwrite_failed:{error}")),
    }

    match safe_file_patch(&new_path, "\"safe\"", "\"patched\"", &snapshot_root) {
        Ok(receipt) => {
            checks.push(CodingSafetyLayerCheck {
                id: "safe_file_patch_exact_match_snapshot_receipt",
                ok: receipt.match_status == "matched_once"
                    && receipt.snapshot_ref.is_some()
                    && receipt.previous_content_hash != receipt.new_content_hash,
                detail: format!(
                    "patched {} {} -> {}",
                    receipt.path, receipt.previous_content_hash, receipt.new_content_hash
                ),
            });
            patch_receipts.push(receipt);
        }
        Err(error) => failures.push(format!("safe_file_patch_failed:{error}")),
    }

    match safe_file_patch(&new_path, "missing_text", "replacement", &snapshot_root) {
        Ok(_) => failures.push("safe_file_patch_missing_text_unexpectedly_succeeded".to_string()),
        Err(error) => checks.push(CodingSafetyLayerCheck {
            id: "safe_file_patch_stale_context_rejected",
            ok: error.contains("stale_context_no_match"),
            detail: error,
        }),
    }

    match validation_command_runner(
        "rustc --test src/generated.rs -o generated_test && ./generated_test",
        &sandbox_root,
        None,
        Some("compile and run generated test harness".to_string()),
    ) {
        Ok(receipt) => {
            checks.push(CodingSafetyLayerCheck {
                id: "validation_command_structured_receipt",
                ok: receipt.exit_code == Some(0)
                    && receipt.status == "passed"
                    && !receipt.command.is_empty(),
                detail: format!(
                    "command exit={:?} stdout_bytes={} stderr_bytes={}",
                    receipt.exit_code,
                    receipt.stdout.len(),
                    receipt.stderr.len()
                ),
            });
            command_receipts.push(receipt);
        }
        Err(error) => failures.push(format!("validation_command_failed:{error}")),
    }

    let ok = failures.is_empty() && checks.iter().all(|check| check.ok);

    CodingSafetyLayerLabReport {
        harness_kind: "coding_safety_layer_lab_behavior_v1",
        ok,
        sandbox_root: sandbox_root.to_string_lossy().to_string(),
        checks,
        read_receipts,
        write_receipts,
        patch_receipts,
        command_receipts,
        failures,
    }
}

pub fn safe_file_read(
    path: &Path,
    start_line: Option<u64>,
    end_line: Option<u64>,
    max_file_size_bytes: u64,
    max_read_lines: u64,
    max_line_chars: usize,
) -> Result<SafeFileReadReceipt, String> {
    assert_absolute_path(path)?;
    let metadata = fs::metadata(path).map_err(|error| format!("metadata_failed:{error}"))?;
    if metadata.len() > max_file_size_bytes {
        return Err(format!(
            "file_size_exceeds_budget:{}>{}",
            metadata.len(),
            max_file_size_bytes
        ));
    }

    let bytes = fs::read(path).map_err(|error| format!("read_failed:{error}"))?;
    if bytes.iter().any(|byte| *byte == 0) {
        return Err("binary_text_read_rejected".to_string());
    }
    let content = String::from_utf8(bytes).map_err(|error| format!("utf8_read_failed:{error}"))?;
    let content_hash = content_hash(&content);
    let lines = content.lines().collect::<Vec<_>>();
    let total_lines = lines.len() as u64;

    let start = start_line.unwrap_or(1);
    if start == 0 {
        return Err("invalid_line_range:start_zero".to_string());
    }
    let requested_end = end_line.unwrap_or_else(|| {
        if total_lines == 0 {
            1
        } else {
            total_lines.min(max_read_lines)
        }
    });
    if requested_end == 0 {
        return Err("invalid_line_range:end_zero".to_string());
    }
    if start > requested_end {
        return Err(format!(
            "invalid_line_range:start_gt_end:{start}>{requested_end}"
        ));
    }
    if total_lines > 0 && start > total_lines {
        return Err(format!(
            "invalid_line_range:start_beyond_total:{start}>{total_lines}"
        ));
    }
    if requested_end.saturating_sub(start).saturating_add(1) > max_read_lines {
        return Err("line_range_exceeds_budget".to_string());
    }

    let actual_end = if total_lines == 0 {
        requested_end
    } else {
        requested_end.min(total_lines)
    };
    let mut truncation_notes = Vec::new();
    let selected = if total_lines == 0 {
        String::new()
    } else {
        lines
            .get((start - 1) as usize..actual_end as usize)
            .unwrap_or(&[])
            .iter()
            .map(|line| truncate_line(line, max_line_chars, &mut truncation_notes))
            .collect::<Vec<_>>()
            .join("\n")
    };

    Ok(SafeFileReadReceipt {
        path: path.display().to_string(),
        start_line: start,
        end_line: actual_end,
        total_lines,
        content_hash,
        bytes_read: selected.len() as u64,
        content: selected,
        truncation_notes,
    })
}

pub fn safe_file_write(
    path: &Path,
    content: &str,
    overwrite: bool,
    snapshot_root: &Path,
) -> Result<SafeFileWriteReceipt, String> {
    assert_absolute_path(path)?;
    assert_absolute_path(snapshot_root)?;
    let file_exists = path.exists();
    if file_exists && !overwrite {
        return Err("overwrite_rejected_without_permission".to_string());
    }

    let mut previous_content_hash = None;
    let mut snapshot_ref = None;
    let target_line_ending = if file_exists {
        let existing = fs::read(path).map_err(|error| format!("read_existing_failed:{error}"))?;
        previous_content_hash = Some(content_hash_bytes(&existing));
        let snapshot = write_snapshot(snapshot_root, path, &existing)?;
        snapshot_ref = Some(snapshot.display().to_string());
        let existing_text = String::from_utf8_lossy(&existing);
        if existing_text.contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        }
    } else {
        platform_line_ending()
    };

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("create_parent_failed:{error}"))?;
    }

    let normalized_content = normalize_line_endings(content, target_line_ending);
    fs::write(path, normalized_content.as_bytes())
        .map_err(|error| format!("write_failed:{error}"))?;

    Ok(SafeFileWriteReceipt {
        path: path.display().to_string(),
        created_or_overwritten: if file_exists {
            "overwritten"
        } else {
            "created"
        },
        overwrite_allowed: overwrite,
        snapshot_ref,
        previous_content_hash,
        new_content_hash: content_hash(&normalized_content),
        validation_errors: Vec::new(),
    })
}

pub fn safe_file_patch(
    path: &Path,
    search: &str,
    replacement: &str,
    snapshot_root: &Path,
) -> Result<SafeFilePatchReceipt, String> {
    assert_absolute_path(path)?;
    if search.is_empty() {
        return Err("search_text_missing".to_string());
    }

    let source = fs::read_to_string(path).map_err(|error| format!("read_source_failed:{error}"))?;
    let previous_content_hash = content_hash(&source);
    let line_ending = if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let normalized_search = normalize_line_endings(search, line_ending);
    let normalized_replacement = normalize_line_endings(replacement, line_ending);
    let match_count = source.match_indices(&normalized_search).count();
    if match_count == 0 {
        return Err("stale_context_no_match".to_string());
    }
    if match_count > 1 {
        return Err("ambiguous_multiple_matches".to_string());
    }

    let patched = source.replacen(&normalized_search, &normalized_replacement, 1);
    let write_receipt = safe_file_write(path, &patched, true, snapshot_root)?;

    Ok(SafeFilePatchReceipt {
        path: path.display().to_string(),
        operation: "replace",
        match_status: "matched_once",
        snapshot_ref: write_receipt.snapshot_ref,
        previous_content_hash,
        new_content_hash: write_receipt.new_content_hash,
        validation_errors: write_receipt.validation_errors,
    })
}

pub fn validation_command_runner(
    command: &str,
    cwd: &Path,
    env_vars: Option<Vec<(String, String)>>,
    description: Option<String>,
) -> Result<ValidationCommandReceipt, String> {
    if command.trim().is_empty() {
        return Err("command_empty".to_string());
    }
    assert_absolute_path(cwd)?;
    if !cwd.is_dir() {
        return Err("cwd_missing_or_invalid".to_string());
    }

    let mut process = Command::new("sh");
    process.arg("-c").arg(command).current_dir(cwd);
    if let Some(env_vars) = env_vars {
        for (key, value) in env_vars {
            process.env(key, value);
        }
    }
    let output = process
        .output()
        .map_err(|error| format!("command_execution_failed:{error}"))?;
    let exit_code = output.status.code();
    let stdout = strip_ansi(&String::from_utf8_lossy(&output.stdout));
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));

    Ok(ValidationCommandReceipt {
        command: command.to_string(),
        cwd: cwd.display().to_string(),
        exit_code,
        stdout,
        stderr,
        status: if output.status.success() {
            "passed"
        } else {
            "failed"
        },
        description,
    })
}

fn record_check(
    checks: &mut Vec<CodingSafetyLayerCheck>,
    id: &'static str,
    result: Result<(), String>,
) {
    match result {
        Ok(()) => checks.push(CodingSafetyLayerCheck {
            id,
            ok: true,
            detail: "ok".to_string(),
        }),
        Err(error) => checks.push(CodingSafetyLayerCheck {
            id,
            ok: false,
            detail: error,
        }),
    }
}

fn assert_absolute_path(path: &Path) -> Result<(), String> {
    if path.is_absolute() {
        Ok(())
    } else {
        Err(format!("path_not_absolute:{}", path.display()))
    }
}

fn truncate_line(line: &str, max_line_chars: usize, notes: &mut Vec<String>) -> String {
    if line.chars().count() <= max_line_chars {
        return line.to_string();
    }
    notes.push(format!("line_truncated_to_{max_line_chars}_chars"));
    line.chars().take(max_line_chars).collect::<String>()
}

fn write_snapshot(
    snapshot_root: &Path,
    original_path: &Path,
    bytes: &[u8],
) -> Result<PathBuf, String> {
    fs::create_dir_all(snapshot_root).map_err(|error| format!("snapshot_dir_failed:{error}"))?;
    let name = sanitize_path_for_snapshot(original_path);
    let path = snapshot_root.join(format!("{}-{}.bak", name, content_hash_bytes(bytes)));
    fs::write(&path, bytes).map_err(|error| format!("snapshot_write_failed:{error}"))?;
    Ok(path)
}

fn sanitize_path_for_snapshot(path: &Path) -> String {
    path.display()
        .to_string()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn normalize_line_endings(content: &str, target_line_ending: &str) -> String {
    content
        .replace("\r\n", "\n")
        .replace('\n', target_line_ending)
}

fn platform_line_ending() -> &'static str {
    #[cfg(windows)]
    {
        "\r\n"
    }
    #[cfg(not(windows))]
    {
        "\n"
    }
}

fn content_hash(content: &str) -> String {
    content_hash_bytes(content.as_bytes())
}

fn content_hash_bytes(bytes: &[u8]) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

fn strip_ansi(input: &str) -> String {
    let mut output = String::new();
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' && chars.peek() == Some(&'[') {
            let _ = chars.next();
            for inner in chars.by_ref() {
                if inner.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            output.push(ch);
        }
    }
    output
}

fn lab_sandbox_root_path() -> PathBuf {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    std::env::temp_dir().join(format!(
        "coding-safety-layer-lab-{}-{millis}",
        std::process::id()
    ))
}

#[cfg(test)]
mod tests {
    use super::coding_safety_layer_lab_report;

    #[test]
    fn coding_safety_layer_lab_behavior_passes() {
        let report = coding_safety_layer_lab_report();
        assert!(report.ok, "{report:#?}");
        assert!(report
            .checks
            .iter()
            .any(|check| check.id == "safe_file_patch_stale_context_rejected" && check.ok));
        assert!(!report.read_receipts.is_empty());
        assert!(!report.write_receipts.is_empty());
        assert!(!report.patch_receipts.is_empty());
        assert!(!report.command_receipts.is_empty());
    }
}
