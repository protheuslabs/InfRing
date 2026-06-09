fn dashboard_duplicate_runtime_issues(
    listener_pids: &[u32],
    watchdog_pids: &[u32],
    host_process_pids: &[u32],
    port: u16,
) -> Vec<Value> {
    let mut issues = Vec::<Value>::new();
    if listener_pids.len() > 1 {
        issues.push(json!({
            "code": "dashboard_listener_duplicate_running",
            "message": "multiple dashboard listeners detected on configured port",
            "pids": listener_pids,
            "port": port,
        }));
    }
    if host_process_pids.len() > 1 {
        issues.push(json!({
            "code": "dashboard_host_process_duplicate_running",
            "message": "multiple dashboard host processes detected for configured dashboard route",
            "pids": host_process_pids,
            "port": port,
        }));
    }
    if watchdog_pids.len() > 1 {
        issues.push(json!({
            "code": "dashboard_watchdog_duplicate_running",
            "message": "multiple dashboard watchdog processes detected for configured dashboard route",
            "pids": watchdog_pids,
            "port": port,
        }));
    }
    issues
}

fn dashboard_command_contains_arg(command: &str, key: &str, value: &str) -> bool {
    command.contains(format!("{key}={value}").as_str())
        || command.contains(format!("{key} {value}").as_str())
}

fn dashboard_host_process_command_matches(command: &str, cfg: &DashboardLaunchConfig) -> bool {
    let command = command.trim();
    if command.is_empty() {
        return false;
    }
    let ts_host = command.contains("client/runtime/systems/ui/infring_dashboard.ts")
        && command.contains(" serve");
    let legacy_host = command.contains("dashboard-ui") && command.contains(" serve");
    if !(ts_host || legacy_host) {
        return false;
    }
    let port = cfg.port.to_string();
    let host = cfg.host.as_str();
    (dashboard_command_contains_arg(command, "--port", port.as_str())
        || dashboard_command_contains_arg(command, "--dashboard-port", port.as_str()))
        && (dashboard_command_contains_arg(command, "--host", host)
            || dashboard_command_contains_arg(command, "--dashboard-host", host))
}

fn dashboard_host_process_pids(cfg: &DashboardLaunchConfig) -> Vec<u32> {
    let current_pid = std::process::id();
    let mut pids = Vec::<u32>::new();
    for pattern in ["infring_dashboard.ts", "dashboard-ui"] {
        for pid in command_pids_matching(pattern) {
            if pid == current_pid {
                continue;
            }
            if let Some(command) = pid_command_line(pid) {
                if dashboard_host_process_command_matches(command.as_str(), cfg) {
                    pids.push(pid);
                }
            }
        }
    }
    normalized_running_pids(pids)
}

fn parse_gateway_launchd_labels(raw: &str) -> Vec<String> {
    let tracked = [
        "ai.infring.gateway",
        "infring.gateway",
        "ai.infring.gateway.legacy",
        "ai.openclaw.gateway",
        "openclaw.gateway",
    ];
    let mut labels = Vec::<String>::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("PID") {
            continue;
        }
        let Some(candidate) = trimmed.split_whitespace().last() else {
            continue;
        };
        if tracked.iter().any(|label| candidate == *label) {
            labels.push(candidate.to_string());
        }
    }
    labels.sort();
    labels.dedup();
    labels
}

#[cfg(target_os = "macos")]
fn loaded_gateway_launchd_labels() -> Vec<String> {
    let output = run_platform_command("launchctl", &[String::from("list")]);
    if output.get("ok").and_then(Value::as_bool) != Some(true) {
        return Vec::new();
    }
    let stdout = output
        .get("stdout")
        .and_then(Value::as_str)
        .unwrap_or_default();
    parse_gateway_launchd_labels(stdout)
}

#[cfg(not(target_os = "macos"))]
fn loaded_gateway_launchd_labels() -> Vec<String> {
    Vec::new()
}

fn binary_file_digest(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    Some(crate::v8_kernel::sha256_hex_bytes(&bytes))
}

fn shell_wrapper_exec_target(path: &Path) -> Option<PathBuf> {
    let bytes = fs::read(path).ok()?;
    if bytes.len() > 4096 {
        return None;
    }
    let text = String::from_utf8(bytes).ok()?;
    if !text.lines().any(|line| line.trim_start().starts_with("#!")) {
        return None;
    }
    for line in text.lines().map(str::trim) {
        let Some(command) = line.strip_prefix("exec ") else {
            continue;
        };
        let Some(token) = command.split_whitespace().next() else {
            continue;
        };
        let raw = token.trim_matches('"').trim_matches('\'');
        if raw.is_empty() || raw.starts_with('$') {
            continue;
        }
        let candidate = PathBuf::from(raw);
        let resolved = if candidate.is_absolute() {
            candidate
        } else {
            path.parent()?.join(candidate)
        };
        if resolved.is_file() {
            return Some(resolved);
        }
    }
    None
}

fn canonical_dashboard_binary_path(path: &Path) -> PathBuf {
    shell_wrapper_exec_target(path).unwrap_or_else(|| path.to_path_buf())
}

fn dashboard_paths_equivalent(left: &Path, right: &Path) -> bool {
    if left == right {
        return true;
    }
    match (fs::canonicalize(left), fs::canonicalize(right)) {
        (Ok(left_canonical), Ok(right_canonical)) => left_canonical == right_canonical,
        _ => false,
    }
}

fn expected_dashboard_binary_path(root: &Path) -> Option<PathBuf> {
    let explicit = std::env::var("INFRING_DAEMON_EXPECTED_BINARY")
        .ok()
        .map(|raw| raw.trim().to_string())
        .filter(|raw| !raw.is_empty());
    if let Some(raw) = explicit {
        let candidate = PathBuf::from(raw);
        if candidate.is_absolute() {
            return Some(candidate);
        }
        return Some(root.join(candidate));
    }
    for env_key in ["INFRING_NPM_BINARY", "INFRING_OPS_BINARY"] {
        let explicit_runtime = std::env::var(env_key)
            .ok()
            .map(|raw| raw.trim().to_string())
            .filter(|raw| !raw.is_empty());
        if let Some(raw) = explicit_runtime {
            let candidate = PathBuf::from(raw);
            let path = if candidate.is_absolute() {
                candidate
            } else {
                root.join(candidate)
            };
            if path.is_file() {
                return Some(path);
            }
        }
    }
    let binary_name = if cfg!(windows) {
        "infring-ops.exe"
    } else {
        "infring-ops"
    };
    let mut candidates = Vec::<PathBuf>::new();
    if let Ok(home) = std::env::var("HOME") {
        let trimmed = home.trim();
        if !trimmed.is_empty() {
            candidates.push(
                PathBuf::from(trimmed)
                    .join(".local")
                    .join("bin")
                    .join(binary_name),
            );
        }
    }
    candidates.push(root.join("target").join("debug").join(binary_name));
    candidates.push(root.join("target").join("release").join(binary_name));
    candidates.into_iter().find(|path| path.is_file())
}

fn dashboard_binary_authority_issue(root: &Path) -> Option<Value> {
    let current_exe = std::env::current_exe().ok()?;
    let resolved = resolve_dashboard_executable(&current_exe);
    let current_name = resolved
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let deprecated_name = current_name.contains("openclaw");
    let expected = expected_dashboard_binary_path(root);
    let mut reasons = Vec::<String>::new();
    if deprecated_name {
        reasons.push("deprecated_binary_name_runtime".to_string());
    }
    let expected_launcher_resolved = expected
        .as_ref()
        .and_then(|path| shell_wrapper_exec_target(path))
        .is_some();
    let expected_canonical = expected
        .as_ref()
        .map(|path| canonical_dashboard_binary_path(path));
    if !deprecated_name {
        if let Some(expected_path) = expected_canonical.as_ref() {
            if dashboard_paths_equivalent(&resolved, expected_path) {
                return None;
            }
        }
    }
    let mut expected_digest = Value::Null;
    let mut current_digest = Value::Null;
    if let Some(expected_path) = expected_canonical.as_ref() {
        if let Some(digest) = binary_file_digest(&resolved) {
            current_digest = Value::String(digest.clone());
            if let Some(expected_value) = binary_file_digest(expected_path) {
                expected_digest = Value::String(expected_value.clone());
                if digest != expected_value {
                    reasons.push("binary_digest_mismatch_current_vs_expected".to_string());
                }
            } else {
                reasons.push("expected_binary_digest_unavailable".to_string());
            }
        } else {
            reasons.push("current_binary_digest_unavailable".to_string());
        }
    }
    if reasons.is_empty() {
        return None;
    }
    Some(json!({
        "code": "dashboard_runtime_binary_authority_mismatch",
        "message": "dashboard runtime binary did not match canonical infring authority binary",
        "current_executable": resolved.to_string_lossy().to_string(),
        "expected_executable": expected_canonical.map(|path| path.to_string_lossy().to_string()),
        "expected_launcher": expected.map(|path| path.to_string_lossy().to_string()),
        "expected_launcher_resolved": expected_launcher_resolved,
        "current_digest": current_digest,
        "expected_digest": expected_digest,
        "recovery": "repair or reinstall the canonical infring launcher, or set INFRING_DAEMON_EXPECTED_BINARY to the real runtime binary for diagnostics",
        "reasons": reasons,
    }))
}

fn dashboard_runtime_duplicate_guard(root: &Path, cfg: &DashboardLaunchConfig) -> Option<Value> {
    let listener_pids = normalized_running_pids(dashboard_listener_pids(cfg.port));
    let watchdog_pids = dashboard_watchdog_candidate_pids(root, cfg);
    let host_process_pids = dashboard_host_process_pids(cfg);
    let mut issues = dashboard_duplicate_runtime_issues(
        &listener_pids,
        &watchdog_pids,
        &host_process_pids,
        cfg.port,
    );
    let launchd_labels = loaded_gateway_launchd_labels();
    if launchd_labels.len() > 1 {
        issues.push(json!({
            "code": "gateway_launchd_duplicate_labels_loaded",
            "message": "multiple gateway launchd labels are loaded; stale labels must be booted out",
            "labels": launchd_labels,
        }));
    }
    if let Some(binary_issue) = dashboard_binary_authority_issue(root) {
        issues.push(binary_issue);
    }
    if issues.is_empty() {
        return None;
    }
    Some(json!({
        "ok": false,
        "error": "dashboard_duplicate_runtime_detected",
        "issue_count": issues.len(),
        "issues": issues,
    }))
}

fn dashboard_duplicate_restart_payload(root: &Path, cfg: &DashboardLaunchConfig) -> Option<Value> {
    dashboard_runtime_duplicate_guard(root, cfg).map(|duplicate_runtime| {
        json!({
            "ok": false,
            "running": false,
            "launched": false,
            "error": "dashboard_duplicate_runtime_detected",
            "duplicate_runtime": duplicate_runtime,
        })
    })
}

fn dashboard_duplicate_spawn_error(root: &Path, cfg: &DashboardLaunchConfig) -> Option<String> {
    dashboard_runtime_duplicate_guard(root, cfg).map(|duplicate_runtime| {
        let hash = deterministic_receipt_hash(&duplicate_runtime);
        format!("dashboard_duplicate_runtime_detected:receipt_hash={hash}")
    })
}

#[cfg(test)]
mod duplicate_runtime_guard_tests {
    use super::*;

    #[test]
    fn dashboard_duplicate_runtime_issues_empty_when_single_runtime_paths_present() {
        let issues = dashboard_duplicate_runtime_issues(&[1001], &[2001], &[3001], 4173);
        assert!(issues.is_empty());
    }

    #[test]
    fn dashboard_duplicate_runtime_issues_flags_listener_duplicates() {
        let issues = dashboard_duplicate_runtime_issues(&[1001, 1002], &[2001], &[3001], 4173);
        assert_eq!(issues.len(), 1);
        assert_eq!(
            issues[0].get("code").and_then(Value::as_str),
            Some("dashboard_listener_duplicate_running")
        );
    }

    #[test]
    fn dashboard_duplicate_runtime_issues_flags_host_process_duplicates() {
        let issues = dashboard_duplicate_runtime_issues(&[1001], &[2001], &[3001, 3002], 4173);
        assert_eq!(issues.len(), 1);
        assert_eq!(
            issues[0].get("code").and_then(Value::as_str),
            Some("dashboard_host_process_duplicate_running")
        );
    }

    #[test]
    fn dashboard_duplicate_runtime_issues_flags_watchdog_duplicates() {
        let issues = dashboard_duplicate_runtime_issues(&[1001], &[2001, 2002], &[3001], 4173);
        assert_eq!(issues.len(), 1);
        assert_eq!(
            issues[0].get("code").and_then(Value::as_str),
            Some("dashboard_watchdog_duplicate_running")
        );
    }

    #[test]
    fn dashboard_command_contains_arg_accepts_equals_and_space_forms() {
        assert!(dashboard_command_contains_arg("--port=4173", "--port", "4173"));
        assert!(dashboard_command_contains_arg("--port 4173", "--port", "4173"));
        assert!(!dashboard_command_contains_arg("--port=5173", "--port", "4173"));
    }

    #[test]
    fn canonical_dashboard_binary_path_resolves_simple_exec_wrapper() {
        let root = tempfile::tempdir().expect("temp root");
        let target = root.path().join("infring-ops-new");
        let wrapper = root.path().join("infring-ops");
        fs::write(&target, b"fake binary").expect("target");
        fs::write(
            &wrapper,
            format!("#!/usr/bin/env sh\nexec {} \"$@\"\n", target.display()),
        )
        .expect("wrapper");
        assert_eq!(canonical_dashboard_binary_path(&wrapper), target);
    }

    #[test]
    fn parse_gateway_launchd_labels_extracts_known_labels() {
        let parsed = parse_gateway_launchd_labels(
            "\
PID\tStatus\tLabel\n\
8999\t0\tai.infring.gateway\n\
-\t0\tai.openclaw.gateway\n\
",
        );
        assert_eq!(
            parsed,
            vec![
                "ai.infring.gateway".to_string(),
                "ai.openclaw.gateway".to_string()
            ]
        );
    }

    #[test]
    fn expected_dashboard_binary_path_honors_runtime_binary_env() {
        let root = tempfile::tempdir().expect("temp root");
        let bin_path = root.path().join("target").join("debug").join("infring-ops");
        fs::create_dir_all(bin_path.parent().unwrap()).expect("bin parent");
        fs::write(&bin_path, b"fake").expect("fake binary");
        let previous = std::env::var("INFRING_NPM_BINARY").ok();
        unsafe {
            std::env::set_var("INFRING_NPM_BINARY", bin_path.to_string_lossy().to_string());
        }
        let resolved = expected_dashboard_binary_path(root.path());
        match previous {
            Some(value) => unsafe {
                std::env::set_var("INFRING_NPM_BINARY", value);
            },
            None => unsafe {
                std::env::remove_var("INFRING_NPM_BINARY");
            },
        }
        assert_eq!(resolved.as_deref(), Some(bin_path.as_path()));
    }
}
