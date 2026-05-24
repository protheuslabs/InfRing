// Layer ownership: Core Layer 2 (Scheduling + Execution) - agent runtime command admission.
use serde_json::{json, Value};
use std::env;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const STRICT_POLICIES: &[&str] = &["ci", "comparison", "eval", "production"];
const DEV_POLICIES: &[&str] = &["debug", "dev", "development", "local-dev"];
const MODES: &[&str] = &[
    "installed_cli_binary",
    "workspace_cli_binary",
    "dev_build_then_run",
    "cargo_run_dev_fallback",
];

pub fn command_resolve(args: &Value) -> Result<Value, String> {
    let intent = text_arg(
        args,
        &[
            "intent",
            "tool_intent",
            "purpose",
            "command_intent",
            "validation_intent",
        ],
    )
    .unwrap_or_else(|| "run_project_command".to_string());
    let tool_id = text_arg(
        args,
        &[
            "tool_id",
            "tool",
            "preferred_tool",
            "binary",
            "executable",
            "command_name",
        ],
    )
    .unwrap_or_else(|| "command".to_string());
    let policy = normalize_policy(
        text_arg(args, &["policy", "execution_policy", "gate_policy"]).unwrap_or_else(|| {
            env::var("INFRING_COMMAND_EXECUTION_POLICY")
                .unwrap_or_else(|_| "comparison".to_string())
        }),
    );
    let allowed_modes = allowed_modes(args, &policy);
    let preferred_mode = text_arg(
        args,
        &[
            "preferred_execution_mode",
            "execution_mode",
            "preferred_mode",
            "mode",
        ],
    );
    let command_args = string_array_arg(args, &["args", "argv", "default_args", "command_args"])?;
    let cwd = text_arg(
        args,
        &[
            "cwd",
            "working_directory",
            "working_dir",
            "workdir",
            "directory",
            "dir",
            "project_root",
            "root",
        ],
    );
    let mut fallback_chain = Vec::new();

    let configured_paths = string_array_arg(
        args,
        &[
            "configured_paths",
            "configured_path",
            "candidate_paths",
            "candidate_path",
            "workspace_binaries",
            "workspace_binary",
        ],
    )?;
    for path in configured_paths {
        if let Some(selected) = try_path_candidate(
            &path,
            "configured_path",
            &allowed_modes,
            preferred_mode.as_deref(),
            &command_args,
            cwd.as_deref(),
            &mut fallback_chain,
        )? {
            return Ok(resolution_receipt(
                &intent,
                &tool_id,
                &policy,
                &allowed_modes,
                preferred_mode.as_deref(),
                selected,
                fallback_chain,
                None,
            ));
        }
    }

    for binary in candidate_binary_names(args, &tool_id)? {
        if let Some(path) = path_lookup(&binary) {
            if let Some(selected) = try_path_candidate(
                &path.display().to_string(),
                "path_lookup",
                &allowed_modes,
                preferred_mode.as_deref(),
                &command_args,
                cwd.as_deref(),
                &mut fallback_chain,
            )? {
                return Ok(resolution_receipt(
                    &intent,
                    &tool_id,
                    &policy,
                    &allowed_modes,
                    preferred_mode.as_deref(),
                    selected,
                    fallback_chain,
                    None,
                ));
            }
        } else {
            fallback_chain.push(format!("path_lookup:missing:{binary}"));
        }
    }

    if allowed_modes.contains(&"dev_build_then_run".to_string()) {
        if let Some(build) = build_then_run_resolution(args, &command_args, cwd.as_deref())? {
            let reason = if build.ready_to_run {
                None
            } else {
                Some("build_required_before_run".to_string())
            };
            fallback_chain.push(format!(
                "dev_build_then_run:{}",
                if build.ready_to_run {
                    "selected_ready"
                } else {
                    "selected_pending_build"
                }
            ));
            return Ok(resolution_receipt(
                &intent,
                &tool_id,
                &policy,
                &allowed_modes,
                preferred_mode.as_deref(),
                build,
                fallback_chain,
                reason,
            ));
        }
    }

    if allowed_modes.contains(&"cargo_run_dev_fallback".to_string()) {
        if let Some(cargo_command) = cargo_run_command(args)? {
            fallback_chain.push("cargo_run_dev_fallback:selected".to_string());
            let selected = SelectedCommand {
                mode: "cargo_run_dev_fallback".to_string(),
                executable: cargo_command.first().cloned().unwrap_or_else(|| "cargo".to_string()),
                resolved_command: cargo_command,
                timing_comparable: false,
                cargo_run_used: true,
                requires_build: false,
                ready_to_run: true,
                build_command: Vec::new(),
                produced_executable: None,
                source: "cargo_run_command".to_string(),
            };
            return Ok(resolution_receipt(
                &intent,
                &tool_id,
                &policy,
                &allowed_modes,
                preferred_mode.as_deref(),
                selected,
                fallback_chain,
                Some("binary_missing_using_dev_fallback".to_string()),
            ));
        }
    } else if cargo_run_command(args)?.is_some() {
        fallback_chain.push(format!(
            "cargo_run_dev_fallback:forbidden_by_policy:{policy}"
        ));
    }

    Err(format!(
        "command_resolve_no_allowed_candidate:intent={intent}:tool={tool_id}:policy={policy}"
    ))
}

#[derive(Debug)]
struct SelectedCommand {
    mode: String,
    executable: String,
    resolved_command: Vec<String>,
    timing_comparable: bool,
    cargo_run_used: bool,
    requires_build: bool,
    ready_to_run: bool,
    build_command: Vec<String>,
    produced_executable: Option<String>,
    source: String,
}

fn resolution_receipt(
    intent: &str,
    tool_id: &str,
    policy: &str,
    allowed_modes: &[String],
    preferred_mode: Option<&str>,
    selected: SelectedCommand,
    fallback_chain: Vec<String>,
    fallback_reason: Option<String>,
) -> Value {
    json!({
        "receipt_type": "command_resolution_receipt_v1",
        "intent": intent,
        "tool_id": tool_id,
        "policy": policy,
        "allowed_by_gate": true,
        "allowed_execution_modes": allowed_modes,
        "requested_execution_mode": preferred_mode,
        "execution_mode": selected.mode,
        "resolved_executable": selected.executable,
        "resolved_source": selected.source,
        "resolved_command": selected.resolved_command,
        "requires_build": selected.requires_build,
        "ready_to_run": selected.ready_to_run,
        "build_command": selected.build_command,
        "produced_executable": selected.produced_executable,
        "fallback_chain": fallback_chain,
        "fallback_reason": fallback_reason,
        "timing_comparable": selected.timing_comparable,
        "cargo_run_used": selected.cargo_run_used,
        "resolved_at_unix_ms": unix_ms(),
    })
}

fn allowed_modes(args: &Value, policy: &str) -> Vec<String> {
    let explicit = string_array_lossy(
        args,
        &[
            "allowed_execution_modes",
            "allowed_modes",
            "execution_modes",
        ],
    );
    let mut modes = if explicit.is_empty() {
        match policy {
            value if STRICT_POLICIES.contains(&value) => vec![
                "installed_cli_binary".to_string(),
                "workspace_cli_binary".to_string(),
            ],
            value if DEV_POLICIES.contains(&value) => MODES.iter().map(|value| value.to_string()).collect(),
            _ => vec![
                "installed_cli_binary".to_string(),
                "workspace_cli_binary".to_string(),
                "dev_build_then_run".to_string(),
            ],
        }
    } else {
        explicit
    };
    let forbidden = string_array_lossy(
        args,
        &[
            "forbidden_execution_modes",
            "forbidden_modes",
            "disallowed_execution_modes",
        ],
    );
    modes.retain(|mode| MODES.contains(&mode.as_str()) && !forbidden.contains(mode));
    if args
        .get("timing_comparable_required")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        modes.retain(|mode| mode != "cargo_run_dev_fallback");
    }
    modes
}

fn try_path_candidate(
    raw_path: &str,
    source: &str,
    allowed_modes: &[String],
    preferred_mode: Option<&str>,
    args: &[String],
    cwd: Option<&str>,
    fallback_chain: &mut Vec<String>,
) -> Result<Option<SelectedCommand>, String> {
    let path = PathBuf::from(raw_path);
    if !path.exists() {
        fallback_chain.push(format!("{source}:missing:{raw_path}"));
        return Ok(None);
    }
    if !path.is_file() {
        fallback_chain.push(format!("{source}:not_file:{raw_path}"));
        return Ok(None);
    }
    let mode = if is_workspace_binary(&path, cwd) {
        "workspace_cli_binary"
    } else {
        "installed_cli_binary"
    };
    if let Some(preferred) = preferred_mode {
        if preferred != mode && allowed_modes.contains(&preferred.to_string()) {
            fallback_chain.push(format!("{source}:skipped:not_preferred_mode:{mode}"));
            return Ok(None);
        }
    }
    if !allowed_modes.contains(&mode.to_string()) {
        fallback_chain.push(format!("{source}:forbidden_mode:{mode}"));
        return Ok(None);
    }
    fallback_chain.push(format!("{source}:selected:{mode}:{raw_path}"));
    let mut resolved = vec![path.display().to_string()];
    resolved.extend(args.iter().cloned());
    Ok(Some(SelectedCommand {
        mode: mode.to_string(),
        executable: path.display().to_string(),
        resolved_command: resolved,
        timing_comparable: true,
        cargo_run_used: false,
        requires_build: false,
        ready_to_run: true,
        build_command: Vec::new(),
        produced_executable: None,
        source: source.to_string(),
    }))
}

fn build_then_run_resolution(
    args: &Value,
    command_args: &[String],
    cwd: Option<&str>,
) -> Result<Option<SelectedCommand>, String> {
    let produced = text_arg(
        args,
        &[
            "produced_executable",
            "built_executable",
            "workspace_binary_after_build",
            "build_output",
        ],
    );
    let Some(produced) = produced else {
        return Ok(None);
    };
    let build_command = string_array_arg(args, &["build_command", "build_cmd", "build"])?;
    if build_command.is_empty() {
        return Ok(None);
    }
    let path = PathBuf::from(&produced);
    let ready = path.exists() && path.is_file();
    let mut resolved = vec![produced.clone()];
    resolved.extend(command_args.iter().cloned());
    Ok(Some(SelectedCommand {
        mode: "dev_build_then_run".to_string(),
        executable: produced.clone(),
        resolved_command: resolved,
        timing_comparable: ready,
        cargo_run_used: false,
        requires_build: !ready,
        ready_to_run: ready,
        build_command,
        produced_executable: Some(produced),
        source: if is_workspace_binary(&path, cwd) {
            "dev_build_then_run:workspace_output".to_string()
        } else {
            "dev_build_then_run:configured_output".to_string()
        },
    }))
}

fn cargo_run_command(args: &Value) -> Result<Option<Vec<String>>, String> {
    let explicit = string_array_arg(
        args,
        &[
            "cargo_run_command",
            "cargo_command",
            "dev_fallback_command",
            "fallback_command",
        ],
    )?;
    if !explicit.is_empty() {
        return Ok(Some(explicit));
    }
    let Some(package) = text_arg(args, &["cargo_package", "package"]) else {
        return Ok(None);
    };
    let mut command = vec!["cargo".to_string(), "run".to_string(), "--quiet".to_string()];
    if let Some(manifest) = text_arg(args, &["cargo_manifest_path", "manifest_path"]) {
        command.extend(["--manifest-path".to_string(), manifest]);
    }
    command.extend(["--package".to_string(), package]);
    if let Some(bin) = text_arg(args, &["cargo_bin", "bin", "binary_name"]) {
        command.extend(["--bin".to_string(), bin]);
    }
    command.push("--".to_string());
    command.extend(string_array_arg(args, &["args", "argv", "command_args"])?);
    Ok(Some(command))
}

fn candidate_binary_names(args: &Value, fallback: &str) -> Result<Vec<String>, String> {
    let mut names = string_array_arg(
        args,
        &[
            "candidate_binaries",
            "candidate_binary",
            "binary_names",
            "binary_name",
            "executables",
            "executable_names",
        ],
    )?;
    if names.is_empty() && fallback != "command" {
        names.push(fallback.to_string());
    }
    Ok(names)
}

fn string_array_arg(args: &Value, keys: &[&str]) -> Result<Vec<String>, String> {
    for key in keys {
        if let Some(value) = args.get(*key) {
            return strings_from_value(value);
        }
    }
    Ok(Vec::new())
}

fn string_array_lossy(args: &Value, keys: &[&str]) -> Vec<String> {
    string_array_arg(args, keys).unwrap_or_default()
}

fn strings_from_value(value: &Value) -> Result<Vec<String>, String> {
    if let Some(raw) = value.as_str() {
        if raw.trim().is_empty() {
            return Ok(Vec::new());
        }
        return Ok(vec![raw.trim().to_string()]);
    }
    let Some(values) = value.as_array() else {
        return Err("command_resolve_string_array_expected".to_string());
    };
    let mut out = Vec::new();
    for value in values.iter().take(64) {
        let Some(text) = value.as_str() else {
            return Err("command_resolve_array_items_must_be_strings".to_string());
        };
        if text.is_empty() || text.contains('\0') {
            return Err("command_resolve_invalid_string_arg".to_string());
        }
        out.push(text.to_string());
    }
    Ok(out)
}

fn text_arg(args: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| args.get(*key).and_then(Value::as_str))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn normalize_policy(raw: String) -> String {
    raw.trim().to_ascii_lowercase().replace('_', "-")
}

fn path_lookup(binary: &str) -> Option<PathBuf> {
    if binary.contains('/') {
        let path = PathBuf::from(binary);
        return path.exists().then_some(path);
    }
    env::var_os("PATH").and_then(|paths| {
        env::split_paths(&paths)
            .map(|dir| dir.join(binary))
            .find(|path| path.exists() && path.is_file())
    })
}

fn is_workspace_binary(path: &Path, cwd: Option<&str>) -> bool {
    let components = path
        .components()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>();
    if components.iter().any(|component| component == "target")
        && components
            .iter()
            .any(|component| component == "debug" || component == "release")
    {
        return true;
    }
    let Some(cwd) = cwd else {
        return false;
    };
    let cwd = PathBuf::from(cwd);
    path.strip_prefix(cwd).is_ok()
}

fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn resolves_configured_binary_without_cargo_run() {
        let executable = std::env::current_exe().expect("current test executable");
        let cwd = std::env::current_dir().expect("current dir");
        let receipt = command_resolve(&json!({
            "intent": "run_project_validation",
            "tool_id": "self_test_binary",
            "configured_paths": [executable.display().to_string()],
            "args": ["--list"],
            "cwd": cwd.display().to_string(),
            "allowed_execution_modes": ["installed_cli_binary", "workspace_cli_binary"],
            "timing_comparable_required": true
        }))
        .expect("configured binary should resolve");

        assert_eq!(receipt["receipt_type"], "command_resolution_receipt_v1");
        assert_eq!(receipt["allowed_by_gate"], true);
        assert_eq!(receipt["cargo_run_used"], false);
        assert_eq!(receipt["timing_comparable"], true);
        assert!(matches!(
            receipt["execution_mode"].as_str(),
            Some("installed_cli_binary" | "workspace_cli_binary")
        ));
    }

    #[test]
    fn comparison_policy_forbids_cargo_run_fallback() {
        let error = command_resolve(&json!({
            "intent": "run_development_tool",
            "tool_id": "definitely_missing_binary_for_command_resolve_test",
            "candidate_binaries": ["definitely_missing_binary_for_command_resolve_test"],
            "policy": "comparison",
            "cargo_package": "xtask",
            "cargo_bin": "xtask",
            "allowed_execution_modes": ["installed_cli_binary", "workspace_cli_binary"],
            "timing_comparable_required": true
        }))
        .expect_err("comparison policy should not fall back to cargo run");

        assert!(error.contains("command_resolve_no_allowed_candidate"));
    }
}
