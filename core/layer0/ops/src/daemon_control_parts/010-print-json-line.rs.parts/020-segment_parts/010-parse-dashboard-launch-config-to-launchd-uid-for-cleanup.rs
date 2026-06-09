fn parse_dashboard_launch_config(argv: &[String], command: &str) -> DashboardLaunchConfig {
    let start_like = matches!(command, "start" | "restart");
    let enabled = parse_bool(
        parse_flag(argv, "dashboard-autoboot")
            .or_else(|| parse_flag(argv, "dashboard"))
            .as_deref(),
        start_like,
    );
    let open_browser = parse_bool(
        parse_flag(argv, "dashboard-open")
            .or_else(|| std::env::var("INFRING_DASHBOARD_OPEN_ON_START").ok())
            .as_deref(),
        start_like,
    );
    let persistent_supervisor = parse_bool(
        parse_flag(argv, "gateway-persist")
            .or_else(|| parse_flag(argv, "gateway-supervisor"))
            .or_else(|| std::env::var("INFRING_GATEWAY_PERSIST").ok())
            .as_deref(),
        start_like,
    );
    let host = parse_flag(argv, "dashboard-host")
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "127.0.0.1".to_string());
    let port = parse_u16(parse_flag(argv, "dashboard-port").as_deref(), 4173);
    let team = parse_flag(argv, "dashboard-team")
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "ops".to_string());
    let refresh_ms = parse_u64(
        parse_flag(argv, "dashboard-refresh-ms").as_deref(),
        2000,
        800,
        60_000,
    );
    let ready_timeout_ms = parse_u64(
        parse_flag(argv, "dashboard-ready-timeout-ms")
            .or_else(|| std::env::var("INFRING_DASHBOARD_READY_TIMEOUT_MS").ok())
            .as_deref(),
        36_000,
        1_500,
        180_000,
    );
    let watchdog_interval_ms = parse_u64(
        parse_flag(argv, "dashboard-watchdog-interval-ms")
            .or_else(|| std::env::var("INFRING_DASHBOARD_WATCHDOG_INTERVAL_MS").ok())
            .as_deref(),
        DASHBOARD_WATCHDOG_INTERVAL_DEFAULT_MS,
        DASHBOARD_WATCHDOG_INTERVAL_MIN_MS,
        DASHBOARD_WATCHDOG_INTERVAL_MAX_MS,
    );
    let node_binary = parse_flag(argv, "node-binary")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(resolve_node_binary);
    DashboardLaunchConfig {
        enabled,
        open_browser,
        persistent_supervisor,
        host,
        port,
        team,
        refresh_ms,
        ready_timeout_ms,
        watchdog_interval_ms,
        node_binary,
    }
}

fn gateway_supervisor_config(cfg: &DashboardLaunchConfig) -> GatewaySupervisorConfig {
    GatewaySupervisorConfig {
        host: cfg.host.clone(),
        port: cfg.port,
        team: cfg.team.clone(),
        refresh_ms: cfg.refresh_ms,
        ready_timeout_ms: cfg.ready_timeout_ms,
        watchdog_interval_ms: cfg.watchdog_interval_ms,
        node_binary: cfg.node_binary.clone(),
    }
}

fn supervisor_executable() -> Result<PathBuf, String> {
    let current = std::env::current_exe()
        .map_err(|err| format!("gateway_supervisor_current_exe_failed:{err}"))?;
    Ok(resolve_dashboard_executable(&current))
}

fn run_platform_command(program: &str, args: &[String]) -> Value {
    let mut cmd = Command::new(program);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(err) => {
            return json!({
                "ok": false,
                "status": -1,
                "program": program,
                "args": args,
                "error": format!("spawn_failed:{err}"),
            });
        }
    };
    let started = Instant::now();
    let timeout = Duration::from_millis(DASHBOARD_PLATFORM_COMMAND_TIMEOUT_MS);
    loop {
        match child.try_wait() {
            Ok(Some(_status)) => break,
            Ok(None) => {
                if started.elapsed() >= timeout {
                    let _ = child.kill();
                    return match child.wait_with_output() {
                        Ok(out) => json!({
                            "ok": false,
                            "status": out.status.code().unwrap_or(-1),
                            "program": program,
                            "args": args,
                            "timed_out": true,
                            "timeout_ms": DASHBOARD_PLATFORM_COMMAND_TIMEOUT_MS,
                            "error": "platform_command_timeout",
                            "stdout": String::from_utf8_lossy(&out.stdout).trim().to_string(),
                            "stderr": String::from_utf8_lossy(&out.stderr).trim().to_string(),
                        }),
                        Err(err) => json!({
                            "ok": false,
                            "status": -1,
                            "program": program,
                            "args": args,
                            "timed_out": true,
                            "timeout_ms": DASHBOARD_PLATFORM_COMMAND_TIMEOUT_MS,
                            "error": format!("platform_command_timeout_wait_failed:{err}"),
                        }),
                    };
                }
                thread::sleep(Duration::from_millis(25));
            }
            Err(err) => {
                let _ = child.kill();
                return json!({
                    "ok": false,
                    "status": -1,
                    "program": program,
                    "args": args,
                    "error": format!("wait_failed:{err}"),
                });
            }
        }
    }
    match child.wait_with_output() {
        Ok(out) => json!({
            "ok": out.status.success(),
            "status": out.status.code().unwrap_or(-1),
            "program": program,
            "args": args,
            "stdout": String::from_utf8_lossy(&out.stdout).trim().to_string(),
            "stderr": String::from_utf8_lossy(&out.stderr).trim().to_string(),
        }),
        Err(err) => json!({
            "ok": false,
            "status": -1,
            "program": program,
            "args": args,
            "error": format!("output_failed:{err}"),
        }),
    }
}

fn sanitize_dashboard_host_token_for_tmp(host: &str) -> String {
    let mut out = String::with_capacity(host.len());
    for ch in host.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "127_0_0_1".to_string()
    } else {
        out
    }
}

fn tmp_root_path_for_gateway_cleanup() -> PathBuf {
    let raw = std::env::var("TMPDIR")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "/tmp".to_string());
    PathBuf::from(raw)
}

fn dashboard_tmp_pid_paths(cfg: &DashboardLaunchConfig) -> Vec<PathBuf> {
    let host_safe = sanitize_dashboard_host_token_for_tmp(&cfg.host);
    let port_safe = cfg.port.to_string();
    let tmp = tmp_root_path_for_gateway_cleanup();
    vec![
        tmp.join(format!("infring-dashboard-{host_safe}-{port_safe}.pid")),
        tmp.join(format!(
            "infring-dashboard-watchdog-{host_safe}-{port_safe}.pid"
        )),
    ]
}

fn cleanup_dashboard_pid_files(root: &Path, cfg: &DashboardLaunchConfig) -> Value {
    let mut rows = Vec::<Value>::new();
    let mut paths = vec![dashboard_pid_path(root), dashboard_watchdog_pid_path(root)];
    paths.extend(dashboard_tmp_pid_paths(cfg));
    for path in paths {
        let existed = path.exists();
        let removed = if existed {
            fs::remove_file(&path).is_ok()
        } else {
            false
        };
        rows.push(json!({
            "path": path.to_string_lossy().to_string(),
            "existed": existed,
            "removed": removed
        }));
    }
    json!({
        "ok": rows.iter().all(|row| row.get("removed").and_then(Value::as_bool) == Some(true) || row.get("existed").and_then(Value::as_bool) == Some(false)),
        "rows": rows
    })
}

#[cfg(target_os = "macos")]
fn stale_launchd_labels_for_cleanup(current_label: &str) -> Vec<String> {
    let mut labels = vec![
        "ai.infring.gateway".to_string(),
        "infring.gateway".to_string(),
        "ai.infring.gateway.legacy".to_string(),
        "ai.openclaw.gateway".to_string(),
        "openclaw.gateway".to_string(),
    ];
    labels.retain(|label| label != current_label);
    labels.sort();
    labels.dedup();
    labels
}

#[cfg(not(target_os = "macos"))]
fn stale_launchd_labels_for_cleanup(_current_label: &str) -> Vec<String> {
    Vec::new()
}

#[cfg(target_os = "macos")]
fn launchd_uid_for_cleanup() -> Option<String> {
    if let Ok(raw) = std::env::var("UID") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() && trimmed.chars().all(|ch| ch.is_ascii_digit()) {
            return Some(trimmed.to_string());
        }
    }
    let out = run_platform_command("id", &[String::from("-u")]);
    out.get("stdout")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty() && value.chars().all(|ch| ch.is_ascii_digit()))
        .map(ToString::to_string)
}
