fn dashboard_health_probe_once(host: &str, port: u16) -> bool {
    dashboard_health_probe_snapshot_once(host, port, DASHBOARD_IO_TIMEOUT_MS).healthy
}

fn dashboard_health_probe_snapshot_once(
    host: &str,
    port: u16,
    io_timeout_ms: u64,
) -> DashboardHealthSnapshot {
    let addr = format!("{host}:{port}");
    let mut resolved = match addr.to_socket_addrs() {
        Ok(addrs) => addrs,
        Err(_) => {
            return DashboardHealthSnapshot::not_ready(
                "dashboard_healthz_resolve_failed",
                false,
                false,
                false,
                0,
                None,
            );
        }
    };
    let Some(sock_addr) = resolved.next() else {
        return DashboardHealthSnapshot::not_ready(
            "dashboard_healthz_resolve_empty",
            false,
            false,
            false,
            0,
            None,
        );
    };
    let mut stream = match TcpStream::connect_timeout(
        &sock_addr,
        Duration::from_millis(DASHBOARD_CONNECT_TIMEOUT_MS),
    ) {
        Ok(s) => s,
        Err(_) => {
            return DashboardHealthSnapshot::not_ready(
                "dashboard_healthz_listener_unreachable",
                false,
                false,
                false,
                0,
                None,
            );
        }
    };
    let bounded_io_timeout_ms = io_timeout_ms.clamp(250, DASHBOARD_IO_TIMEOUT_MS);
    let _ = stream.set_read_timeout(Some(Duration::from_millis(bounded_io_timeout_ms)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(bounded_io_timeout_ms)));
    if stream
        .write_all(
            format!("GET /healthz HTTP/1.1\r\nHost: {host}:{port}\r\nConnection: close\r\n\r\n")
                .as_bytes(),
        )
        .is_err()
    {
        return DashboardHealthSnapshot::not_ready(
            "dashboard_healthz_write_failed",
            true,
            false,
            false,
            0,
            None,
        );
    }

    let mut collected = Vec::<u8>::new();
    let mut buf = [0u8; 1024];
    let mut timed_out = false;
    let mut response_capped = false;
    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                collected.extend_from_slice(&buf[..n]);
                if collected.len() > DASHBOARD_HEALTH_MAX_BYTES {
                    collected.truncate(DASHBOARD_HEALTH_MAX_BYTES);
                    response_capped = true;
                }
                if dashboard_health_response_ok(&collected) {
                    return dashboard_health_snapshot_from_response(
                        &collected,
                        false,
                        response_capped,
                    );
                }
            }
            Err(err)
                if err.kind() == ErrorKind::WouldBlock || err.kind() == ErrorKind::TimedOut =>
            {
                timed_out = true;
                break;
            }
            Err(_) => {
                return DashboardHealthSnapshot::not_ready(
                    "dashboard_healthz_read_failed",
                    true,
                    false,
                    false,
                    collected.len(),
                    dashboard_health_response_status_code(&collected),
                );
            }
        }
    }
    dashboard_health_snapshot_from_response(&collected, timed_out, response_capped)
}

fn dashboard_health_response_status_code(bytes: &[u8]) -> Option<u16> {
    let raw = String::from_utf8_lossy(bytes);
    let first_line = raw.lines().next()?;
    let mut parts = first_line.split_whitespace();
    let _protocol = parts.next()?;
    parts.next()?.parse::<u16>().ok()
}

fn dashboard_health_snapshot_from_response(
    bytes: &[u8],
    timed_out: bool,
    response_capped: bool,
) -> DashboardHealthSnapshot {
    let status_code = dashboard_health_response_status_code(bytes);
    if dashboard_health_response_ok(bytes) {
        return DashboardHealthSnapshot::ready(status_code, bytes.len());
    }
    if let Some(code) = status_code {
        return DashboardHealthSnapshot::not_ready(
            "dashboard_healthz_non_2xx",
            true,
            timed_out,
            !response_capped && !timed_out,
            bytes.len(),
            Some(code),
        );
    }
    if timed_out {
        return DashboardHealthSnapshot::not_ready(
            "dashboard_healthz_timeout",
            true,
            true,
            false,
            bytes.len(),
            None,
        );
    }
    if bytes.is_empty() {
        return DashboardHealthSnapshot::not_ready(
            "dashboard_healthz_no_response",
            true,
            false,
            false,
            0,
            None,
        );
    }
    DashboardHealthSnapshot::not_ready(
        "dashboard_healthz_incomplete",
        true,
        false,
        !response_capped,
        bytes.len(),
        None,
    )
}

fn dashboard_health_response_ok(bytes: &[u8]) -> bool {
    let raw = String::from_utf8_lossy(bytes);
    if let Some(first_line) = raw.lines().next() {
        let mut parts = first_line.split_whitespace();
        let _protocol = parts.next().unwrap_or("");
        if let Some(code_raw) = parts.next() {
            if let Ok(code) = code_raw.parse::<u16>() {
                if (200..300).contains(&code) {
                    return true;
                }
                return false;
            }
        }
    }
    raw.contains("200 OK")
}

fn dashboard_web_tooling_response_ready(bytes: &[u8]) -> bool {
    if !dashboard_health_response_ok(bytes) {
        return false;
    }
    let raw = String::from_utf8_lossy(bytes).to_ascii_lowercase();
    let body = raw.split("\r\n\r\n").nth(1).unwrap_or("");
    body.contains("\"any_present\":true")
        || body.contains("\"auth_any_present\":true")
        || body.contains("\"readiness\":\"ready\"")
        || (body.contains("\"auth_sources\":") && !body.contains("\"auth_sources\":[]"))
}

fn dashboard_web_tooling_status_ok(host: &str, port: u16) -> bool {
    let addr = format!("{host}:{port}");
    let mut resolved = match addr.to_socket_addrs() {
        Ok(addrs) => addrs,
        Err(_) => return false,
    };
    let Some(sock_addr) = resolved.next() else {
        return false;
    };
    let mut stream = match TcpStream::connect_timeout(
        &sock_addr,
        Duration::from_millis(DASHBOARD_CONNECT_TIMEOUT_MS),
    ) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(DASHBOARD_IO_TIMEOUT_MS)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(DASHBOARD_IO_TIMEOUT_MS)));
    if stream
        .write_all(
            format!(
                "GET /api/comms/web-tooling/status HTTP/1.1\r\nHost: {host}:{port}\r\nConnection: close\r\n\r\n"
            )
            .as_bytes(),
        )
        .is_err()
    {
        return false;
    }
    let mut collected = Vec::<u8>::new();
    let mut buf = [0u8; 1024];
    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                collected.extend_from_slice(&buf[..n]);
                if collected.len() > DASHBOARD_HEALTH_MAX_BYTES {
                    collected.truncate(DASHBOARD_HEALTH_MAX_BYTES);
                }
            }
            Err(err)
                if err.kind() == ErrorKind::WouldBlock || err.kind() == ErrorKind::TimedOut =>
            {
                break;
            }
            Err(_) => return false,
        }
    }
    dashboard_web_tooling_response_ready(&collected)
}

fn dashboard_web_tooling_strict_enabled() -> bool {
    std::env::var("INFRING_DASHBOARD_WEB_TOOLING_STRICT")
        .ok()
        .map(|raw| matches!(raw.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
        .unwrap_or(false)
}

fn dashboard_health_ok_with_retry(
    host: &str,
    port: u16,
    retry_attempts: usize,
    retry_backoff_ms: u64,
) -> bool {
    let attempts = retry_attempts.max(1);
    for idx in 0..attempts {
        if dashboard_health_probe_once(host, port) {
            return true;
        }
        if idx + 1 < attempts && retry_backoff_ms > 0 {
            std::thread::sleep(Duration::from_millis(retry_backoff_ms));
        }
    }
    false
}

fn dashboard_health_ok(host: &str, port: u16) -> bool {
    let healthy = dashboard_health_ok_with_retry(
        host,
        port,
        DASHBOARD_HEALTH_RETRY_ATTEMPTS,
        DASHBOARD_HEALTH_RETRY_BACKOFF_MS.max(1_000),
    );
    if !healthy {
        return false;
    }
    if dashboard_web_tooling_strict_enabled() {
        return dashboard_web_tooling_status_ok(host, port);
    }
    true
}

fn dashboard_health_ok_fast(host: &str, port: u16) -> bool {
    dashboard_health_ok_with_retry(host, port, 1, 0)
}

fn dashboard_health_snapshot_fast(host: &str, port: u16) -> DashboardHealthSnapshot {
    dashboard_health_probe_snapshot_once(host, port, DASHBOARD_WATCHDOG_HEALTH_IO_TIMEOUT_MS)
}

fn wait_for_dashboard(host: &str, port: u16, attempts: usize) -> bool {
    for _ in 0..attempts {
        if dashboard_health_ok_fast(host, port) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(150));
    }
    false
}

fn wait_for_dashboard_stable(
    host: &str,
    port: u16,
    attempts: usize,
    required_successes: usize,
) -> bool {
    let needed = required_successes.max(1);
    let mut ok_streak = 0usize;
    for _ in 0..attempts {
        if dashboard_health_ok_fast(host, port) {
            ok_streak += 1;
            if ok_streak >= needed {
                return true;
            }
        } else {
            ok_streak = 0;
        }
        std::thread::sleep(Duration::from_millis(150));
    }
    false
}

fn dashboard_wait_attempts(cfg: &DashboardLaunchConfig) -> usize {
    let ticks = (cfg.ready_timeout_ms / 150).max(1);
    usize::try_from(ticks).unwrap_or(240).clamp(1, 1200)
}

fn dashboard_log_tail(root: &Path, lines: usize) -> String {
    let log_path = dashboard_log_path(root);
    let raw = fs::read_to_string(log_path).unwrap_or_default();
    let mut tail = raw
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .rev()
        .take(lines)
        .collect::<Vec<_>>();
    tail.reverse();
    tail.join("\n")
}

fn open_browser(url: &str) -> bool {
    #[cfg(target_os = "macos")]
    {
        return Command::new("open")
            .arg(url)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
    }
    #[cfg(target_os = "linux")]
    {
        return Command::new("xdg-open")
            .arg(url)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
    }
    #[cfg(target_os = "windows")]
    {
        return Command::new("cmd")
            .arg("/C")
            .arg("start")
            .arg("")
            .arg(url)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
    }
    #[allow(unreachable_code)]
    false
}

fn clear_dashboard_stop_latch(root: &Path) {
    let _ = fs::remove_file(dashboard_stop_latch_path(root));
}

fn set_dashboard_stop_latch(root: &Path) {
    let latch = dashboard_stop_latch_path(root);
    if let Some(parent) = latch.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(latch, b"stop\n");
}

fn dashboard_stop_latch_active(root: &Path) -> bool {
    dashboard_stop_latch_path(root).exists()
}

fn set_dashboard_desired_state(root: &Path, active: bool) {
    let path = dashboard_desired_state_path(root);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(path, if active { b"1\n" } else { b"0\n" });
}

fn dashboard_desired_state_active(root: &Path) -> bool {
    fs::read_to_string(dashboard_desired_state_path(root))
        .map(|raw| raw.trim() == "1")
        .unwrap_or(false)
}

fn append_watchdog_log(root: &Path, payload: &Value) {
    let path = dashboard_watchdog_log_path(root);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(
            file,
            "{}",
            serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_string())
        );
    }
}

fn dashboard_watchdog_diagnostic_path(root: &Path) -> PathBuf {
    dashboard_state_dir(root).join("dashboard_watchdog_last_diagnostic.json")
}

fn write_dashboard_watchdog_diagnostic(root: &Path, payload: &Value) {
    let path = dashboard_watchdog_diagnostic_path(root);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(
        path,
        serde_json::to_string_pretty(payload).unwrap_or_else(|_| "{}".to_string()),
    );
}

fn push_dashboard_pid_candidate(pids: &mut Vec<u32>, pid: u32) {
    if pid == 0 || !pid_running(pid) || pids.contains(&pid) {
        return;
    }
    pids.push(pid);
}

fn dashboard_stop_candidate_pids(root: &Path, cfg: &DashboardLaunchConfig) -> Vec<u32> {
    let mut candidate_pids = Vec::<u32>::new();
    if let Some(pid) = read_pid_file(&dashboard_pid_path(root)) {
        push_dashboard_pid_candidate(&mut candidate_pids, pid);
    }
    for pid in dashboard_listener_pids(cfg.port) {
        push_dashboard_pid_candidate(&mut candidate_pids, pid);
    }
    for pid in dashboard_host_process_pids(cfg) {
        push_dashboard_pid_candidate(&mut candidate_pids, pid);
    }
    candidate_pids.sort_unstable();
    candidate_pids.dedup();
    candidate_pids
}

fn kill_dashboard_process(root: &Path, cfg: &DashboardLaunchConfig) -> Value {
    let pid_path = dashboard_pid_path(root);
    let mut candidate_pids = dashboard_stop_candidate_pids(root, cfg);
    let initial_candidate_pids = candidate_pids.clone();

    let mut killed_pids = Vec::<u32>::new();
    let mut convergence_passes = Vec::<Value>::new();
    for pass in 0..3 {
        let pass_pids = dashboard_stop_candidate_pids(root, cfg);
        if pass_pids.is_empty() {
            convergence_passes.push(json!({
                "phase": "term",
                "pass": pass + 1,
                "pids": [],
                "converged": true
            }));
            break;
        }
        for pid in &pass_pids {
            if !candidate_pids.contains(pid) {
                candidate_pids.push(*pid);
            }
            if kill_pid(*pid) && !killed_pids.contains(pid) {
                killed_pids.push(*pid);
            }
        }
        convergence_passes.push(json!({
            "phase": "term",
            "pass": pass + 1,
            "pids": pass_pids,
            "converged": false
        }));
        std::thread::sleep(Duration::from_millis(250));
    }
    for _ in 0..5 {
        let live_candidates = dashboard_stop_candidate_pids(root, cfg);
        if live_candidates.is_empty() && candidate_pids.iter().all(|pid| !pid_running(*pid)) {
            break;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    let survivor_pids = dashboard_stop_candidate_pids(root, cfg);
    let mut force_killed_pids = Vec::<u32>::new();
    for pid in &survivor_pids {
        if pid_running(*pid) && kill_pid_force(*pid) {
            force_killed_pids.push(*pid);
        }
    }
    if !force_killed_pids.is_empty() {
        std::thread::sleep(Duration::from_millis(250));
    }

    let _ = fs::remove_file(pid_path);
    let still_running = wait_for_dashboard(cfg.host.as_str(), cfg.port, 5);
    let stale_host_pids_after = dashboard_host_process_pids(cfg);
    let any_killed = !killed_pids.is_empty() || !force_killed_pids.is_empty();
    json!({
        "ok": true,
        "stopped": any_killed && !still_running && stale_host_pids_after.is_empty(),
        "any_killed": any_killed,
        "initial_candidate_pids": initial_candidate_pids,
        "candidate_pids": candidate_pids,
        "convergence_passes": convergence_passes,
        "survivor_pids_before_force": survivor_pids,
        "killed_pids": killed_pids,
        "force_killed_pids": force_killed_pids,
        "host": cfg.host.as_str(),
        "port": cfg.port,
        "still_running": still_running,
        "stale_host_pids_after": stale_host_pids_after,
        "reason": if killed_pids.is_empty() { "no_pid_killed" } else { "pid_killed" }
    })
}

include!("020-dashboard-health-ok_parts/010-duplicate-runtime-guard.rs");

fn restart_dashboard_for_watchdog(root: &Path, cfg: &DashboardLaunchConfig) -> Value {
    let duplicate_before_restart = dashboard_runtime_duplicate_guard(root, cfg);
    let previous_pid = read_pid_file(&dashboard_pid_path(root));
    let previous_running = previous_pid.map(pid_running).unwrap_or(false);
    let listeners_before = dashboard_listener_pids(cfg.port);
    let had_listener = !listeners_before.is_empty();
    let host_processes_before = dashboard_host_process_pids(cfg);
    let had_host_process = !host_processes_before.is_empty();

    let stop_attempt = if previous_running || had_listener || had_host_process || duplicate_before_restart.is_some() {
        kill_dashboard_process(root, cfg)
    } else {
        json!({
            "ok": true,
            "stopped": false,
            "reason": "nothing_to_stop",
            "killed_pids": []
        })
    };

    let wait_attempts = dashboard_wait_attempts(cfg);
    let spawn = spawn_dashboard(root, cfg);
    let spawned_pid = spawn.as_ref().ok().copied();
    let spawn_error = spawn.as_ref().err().cloned();
    let launched = spawn.is_ok();
    let running = if launched {
        wait_for_dashboard_stable(
            cfg.host.as_str(),
            cfg.port,
            wait_attempts,
            DASHBOARD_WATCHDOG_STABLE_RETRIES,
        )
    } else {
        false
    };
    let pid = read_pid_file(&dashboard_pid_path(root)).or(spawned_pid);
    let mut out = json!({
        "ok": true,
        "running": running,
        "launched": launched,
        "pid": pid,
        "previous_pid": previous_pid,
        "previous_running": previous_running,
        "listeners_before": listeners_before,
        "host_processes_before": host_processes_before,
        "duplicate_before_restart": duplicate_before_restart,
        "stop_attempt": stop_attempt,
    });
    if let Some(err) = spawn_error {
        out["spawn_error"] = Value::String(err);
    }
    if !running {
        let tail = dashboard_log_tail(root, 8);
        if !tail.is_empty() {
            out["log_tail"] = Value::String(tail);
        }
    }
    out
}

include!("020-dashboard-health-ok_parts/020-health-tests.rs");

fn spawn_dashboard(root: &Path, cfg: &DashboardLaunchConfig) -> Result<u32, String> {
    if !crate::contract_lane_utils::node_binary_usable(cfg.node_binary.as_str()) {
        return Err(format!(
            "dashboard_spawn_failed:node_binary_unavailable:{}",
            cfg.node_binary
        ));
    }
    fs::create_dir_all(dashboard_state_dir(root))
        .map_err(|err| format!("dashboard_state_dir_create_failed:{err}"))?;
    let log_path = dashboard_log_path(root);
    let log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|err| format!("dashboard_log_open_failed:{err}"))?;
    let log_err = log
        .try_clone()
        .map_err(|err| format!("dashboard_log_clone_failed:{err}"))?;
    // Canonical dashboard surface: TypeScript pipeline serving the Infring browser UI.
    // Keep a single browser surface wired to the Rust API lane.
    let mut cmd = Command::new(cfg.node_binary.as_str());
    cmd.arg("client/runtime/lib/ts_entrypoint.ts")
        .arg("client/runtime/systems/ui/infring_dashboard.ts")
        .arg("serve")
        .arg(format!("--host={}", cfg.host))
        .arg(format!("--port={}", cfg.port))
        .arg(format!("--team={}", cfg.team))
        .arg(format!("--refresh-ms={}", cfg.refresh_ms))
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(log_err))
        .env("INFRING_OPS_ALLOW_STALE", "1")
        .env("INFRING_NPM_ALLOW_STALE", "1")
        .env("INFRING_NODE_BINARY", cfg.node_binary.as_str());
    if let Some(bin_hint) = dashboard_backend_binary_hint() {
        cmd.env("INFRING_NPM_BINARY", bin_hint);
    }
    let child = cmd
        .spawn()
        .map_err(|err| format!("dashboard_spawn_failed:{err}"))?;
    let _ = fs::write(dashboard_pid_path(root), format!("{}\n", child.id()));
    Ok(child.id())
}

fn dashboard_watchdog_status(root: &Path) -> Value {
    let pid_path = dashboard_watchdog_pid_path(root);
    let pid = read_pid_file(&pid_path);
    let running = pid.map(pid_running).unwrap_or(false);
    if !running {
        let _ = fs::remove_file(pid_path);
    }
    json!({
        "pid": pid,
        "running": running,
        "log_path": dashboard_watchdog_log_path(root).to_string_lossy().to_string(),
        "diagnostic_path": dashboard_watchdog_diagnostic_path(root).to_string_lossy().to_string(),
        "stop_latch_active": dashboard_stop_latch_active(root),
        "desired_active": dashboard_desired_state_active(root),
    })
}

fn stop_dashboard_watchdog(root: &Path) -> Value {
    let pid_path = dashboard_watchdog_pid_path(root);
    let pid = read_pid_file(&pid_path);
    let stopped = pid.map(kill_pid).unwrap_or(false);
    let _ = fs::remove_file(pid_path);
    json!({
        "ok": true,
        "stopped": stopped,
        "pid": pid,
        "stop_latch_active": dashboard_stop_latch_active(root),
        "desired_active": dashboard_desired_state_active(root),
    })
}

fn spawn_dashboard_watchdog(root: &Path, cfg: &DashboardLaunchConfig) -> Result<u32, String> {
    fs::create_dir_all(dashboard_state_dir(root))
        .map_err(|err| format!("dashboard_state_dir_create_failed:{err}"))?;
    if let Some(err) = dashboard_duplicate_spawn_error(root, cfg) {
        return Err(err);
    }
    let status = dashboard_watchdog_status(root);
    if status.get("running").and_then(Value::as_bool) == Some(true) {
        if let Some(pid) = status
            .get("pid")
            .and_then(Value::as_u64)
            .and_then(|value| u32::try_from(value).ok())
        {
            return Ok(pid);
        }
    }
    let log_path = dashboard_watchdog_log_path(root);
    let log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|err| format!("dashboard_watchdog_log_open_failed:{err}"))?;
    let log_err = log
        .try_clone()
        .map_err(|err| format!("dashboard_watchdog_log_clone_failed:{err}"))?;
    let current_exe = std::env::current_exe()
        .map_err(|err| format!("dashboard_watchdog_current_exe_failed:{err}"))?;
    let executable = resolve_dashboard_executable(&current_exe);
    let child = Command::new(executable)
        .arg("daemon-control")
        .arg("watchdog")
        .arg(format!(
            "--gateway-persist={}",
            if cfg.persistent_supervisor { 1 } else { 0 }
        ))
        .arg(format!("--dashboard-host={}", cfg.host))
        .arg(format!("--dashboard-port={}", cfg.port))
        .arg(format!("--dashboard-team={}", cfg.team))
        .arg(format!("--dashboard-refresh-ms={}", cfg.refresh_ms))
        .arg(format!(
            "--dashboard-ready-timeout-ms={}",
            cfg.ready_timeout_ms
        ))
        .arg(format!(
            "--dashboard-watchdog-interval-ms={}",
            cfg.watchdog_interval_ms
        ))
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(log_err))
        .spawn()
        .map_err(|err| format!("dashboard_watchdog_spawn_failed:{err}"))?;
    let _ = fs::write(
        dashboard_watchdog_pid_path(root),
        format!("{}\n", child.id()),
    );
    Ok(child.id())
}
