fn start_dashboard_with_config(
    root: &Path,
    cfg: &DashboardLaunchConfig,
    allow_browser: bool,
    spawn_local_watchdog: bool,
) -> Value {
    let url = cfg.url();
    if !cfg.enabled {
        return json!({
            "enabled": false,
            "running": dashboard_health_ok(cfg.host.as_str(), cfg.port),
            "opened_browser": false,
            "url": url
        });
    }
    let duplicate_recovery = dashboard_runtime_duplicate_guard(root, cfg).map(|duplicate_runtime| {
        let stop_attempt = kill_dashboard_process(root, cfg);
        json!({
            "duplicate_runtime": duplicate_runtime,
            "stop_attempt": stop_attempt,
        })
    });
    if dashboard_health_ok_fast(cfg.host.as_str(), cfg.port) {
        return json!({
            "enabled": true,
            "running": true,
            "launched": false,
            "opened_browser": false,
            "url": url,
            "ready_timeout_ms": cfg.ready_timeout_ms,
            "duplicate_recovery": duplicate_recovery
        });
    }

    let wait_attempts = dashboard_wait_attempts(&cfg);
    let first_spawn = spawn_dashboard(root, &cfg);
    if let Err(err) = first_spawn.as_ref() {
        let mut out = json!({
            "enabled": true,
            "running": false,
            "launched": false,
            "pid": Value::Null,
            "opened_browser": false,
            "url": url,
            "node_binary": cfg.node_binary,
            "log_path": dashboard_log_path(root).to_string_lossy().to_string(),
            "ready_timeout_ms": cfg.ready_timeout_ms,
            "recovery_attempted": false,
            "recovery": Value::Null,
            "spawn_error": err,
            "error": "dashboard_spawn_failed"
        });
        if err.contains("node_binary_unavailable") {
            out["error_code"] = Value::String("dashboard_node_binary_unavailable".to_string());
        }
        let tail = dashboard_log_tail(root, 8);
        if !tail.is_empty() {
            out["log_tail"] = Value::String(tail);
        }
        out["watchdog"] = dashboard_watchdog_status(root);
        return out;
    }
    let mut running = wait_for_dashboard_stable(
        cfg.host.as_str(),
        cfg.port,
        wait_attempts,
        DASHBOARD_WATCHDOG_STABLE_RETRIES,
    );
    let mut launched = first_spawn.is_ok();
    let mut pid = first_spawn.as_ref().ok().copied();
    let mut recovery_attempted = false;
    let mut recovery = Value::Null;

    if !running {
        recovery_attempted = true;
        let stopped = kill_dashboard_process(root, &cfg);
        let second_spawn = spawn_dashboard(root, &cfg);
        running = wait_for_dashboard_stable(
            cfg.host.as_str(),
            cfg.port,
            wait_attempts,
            DASHBOARD_WATCHDOG_STABLE_RETRIES,
        );
        launched = second_spawn.is_ok();
        pid = second_spawn.as_ref().ok().copied();
        let mut recovery_payload = json!({
            "attempted": true,
            "stopped": stopped,
            "launched": second_spawn.is_ok()
        });
        if let Err(err) = second_spawn {
            recovery_payload["error"] = Value::String(err);
        }
        recovery = recovery_payload;
    }

    let mut out = json!({
        "enabled": true,
        "running": running,
        "launched": launched,
        "pid": pid,
        "opened_browser": false,
        "url": url,
        "node_binary": cfg.node_binary,
        "log_path": dashboard_log_path(root).to_string_lossy().to_string(),
        "ready_timeout_ms": cfg.ready_timeout_ms,
        "recovery_attempted": recovery_attempted,
        "recovery": recovery
    });
    if allow_browser && cfg.open_browser && running {
        out["opened_browser"] = Value::Bool(open_browser(cfg.url().as_str()));
    }
    if let Some(duplicate_recovery) = duplicate_recovery {
        out["duplicate_recovery"] = duplicate_recovery;
    }
    if let Err(err) = first_spawn {
        out["spawn_error"] = Value::String(err);
    }
    if !running {
        out["error"] = Value::String("dashboard_healthz_not_ready".to_string());
        let tail = dashboard_log_tail(root, 8);
        if !tail.is_empty() {
            out["log_tail"] = Value::String(tail);
        }
    }
    if running && spawn_local_watchdog {
        let watchdog = spawn_dashboard_watchdog(root, cfg);
        out["watchdog"] = match watchdog {
            Ok(pid) => json!({
                "running": true,
                "pid": pid,
                "interval_ms": cfg.watchdog_interval_ms,
                "log_path": dashboard_watchdog_log_path(root).to_string_lossy().to_string(),
            }),
            Err(err) => json!({
                "running": false,
                "error": err,
                "interval_ms": cfg.watchdog_interval_ms,
                "log_path": dashboard_watchdog_log_path(root).to_string_lossy().to_string(),
            }),
        };
    } else {
        out["watchdog"] = dashboard_watchdog_status(root);
    }
    out
}
