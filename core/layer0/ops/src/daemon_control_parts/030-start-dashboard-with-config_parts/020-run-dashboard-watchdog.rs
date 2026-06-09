
fn run_dashboard_watchdog(root: &Path, argv: &[String]) -> i32 {
    let cfg = parse_dashboard_launch_config(argv, "start");
    let current_pid = std::process::id();
    if !cfg.enabled {
        print_json_line(&json!({
            "ok": true,
            "type": "dashboard_watchdog",
            "running": false,
            "reason": "dashboard_disabled",
            "host": cfg.host,
            "port": cfg.port,
        }));
        return 0;
    }
    if let Some(duplicate_runtime) = dashboard_runtime_duplicate_guard(root, &cfg) {
        append_watchdog_log(
            root,
            &json!({
                "ok": false,
                "type": "dashboard_watchdog",
                "event": "authority_guard_blocked",
                "pid": current_pid,
                "duplicate_runtime": duplicate_runtime,
                "port": cfg.port,
            }),
        );
        print_json_line(&json!({
            "ok": false,
            "type": "dashboard_watchdog",
            "running": false,
            "reason": "dashboard_duplicate_runtime_detected",
            "pid": current_pid,
            "duplicate_runtime": duplicate_runtime,
            "port": cfg.port,
        }));
        return 2;
    }
    let _ = fs::write(
        dashboard_watchdog_pid_path(root),
        format!("{}\n", current_pid),
    );
    let watchdog_pids = dashboard_watchdog_runtime_pids(&cfg);
    let other_watchdogs = watchdog_pids
        .iter()
        .copied()
        .filter(|pid| *pid != current_pid)
        .collect::<Vec<_>>();
    if !other_watchdogs.is_empty() {
        append_watchdog_log(
            root,
            &json!({
                "ok": false,
                "type": "dashboard_watchdog",
                "event": "duplicate_watchdog_preempted",
                "pid": current_pid,
                "other_watchdogs": other_watchdogs,
                "watchdog_pids": watchdog_pids,
                "port": cfg.port,
            }),
        );
        print_json_line(&json!({
            "ok": false,
            "type": "dashboard_watchdog",
            "running": false,
            "reason": "duplicate_watchdog_running",
            "pid": current_pid,
            "other_watchdogs": other_watchdogs,
            "watchdog_pids": watchdog_pids,
            "port": cfg.port,
        }));
        return 2;
    }
    append_watchdog_log(
        root,
        &json!({
            "ok": true,
            "type": "dashboard_watchdog",
            "event": "started",
            "pid": current_pid,
            "host": cfg.host,
            "port": cfg.port,
            "interval_ms": cfg.watchdog_interval_ms,
            "fail_streak_threshold": DASHBOARD_WATCHDOG_FAIL_STREAK_THRESHOLD,
            "node_binary": cfg.node_binary,
        }),
    );
    let mut fail_streak = 0usize;
    let mut last_health: Option<bool> = None;
    let mut exit_reason = "stop_latch";
    let mut exit_code = 0i32;
    loop {
        let watchdog_pids = dashboard_watchdog_runtime_pids(&cfg);
        if watchdog_pids.len() > 1 {
            let leader = watchdog_pids.first().copied().unwrap_or(current_pid);
            if leader != current_pid {
                append_watchdog_log(
                    root,
                    &json!({
                        "ok": false,
                        "type": "dashboard_watchdog",
                        "event": "duplicate_watchdog_preempted",
                        "pid": current_pid,
                        "leader_pid": leader,
                        "watchdog_pids": watchdog_pids,
                        "port": cfg.port,
                    }),
                );
                exit_reason = "duplicate_watchdog_running";
                exit_code = 2;
                break;
            }
        }
        if dashboard_stop_latch_active(root) {
            if dashboard_desired_state_active(root) {
                clear_dashboard_stop_latch(root);
                append_watchdog_log(
                    root,
                    &json!({
                        "ok": true,
                        "type": "dashboard_watchdog",
                        "event": "stop_latch_cleared",
                        "reason": "desired_state_active",
                    }),
                );
            } else {
                break;
            }
        }
        let health_snapshot = dashboard_health_snapshot_fast(cfg.host.as_str(), cfg.port);
        let healthy = health_snapshot.healthy;
        let dashboard_pid = read_pid_file(&dashboard_pid_path(root));
        let listener_pids = dashboard_listener_pids(cfg.port);
        let process_active =
            dashboard_pid.map(pid_running).unwrap_or(false) || !listener_pids.is_empty();
        if last_health != Some(healthy) {
            append_watchdog_log(
                root,
                &json!({
                    "ok": true,
                    "type": "dashboard_watchdog",
                    "event": "health_transition",
                    "healthy": healthy,
                    "health": health_snapshot.to_json(),
                    "fail_streak": fail_streak,
                    "dashboard_pid": dashboard_pid,
                    "listeners": listener_pids,
                    "process_active": process_active,
                }),
            );
            last_health = Some(healthy);
        }
        if healthy {
            fail_streak = 0;
        } else {
            fail_streak = fail_streak.saturating_add(1);
        }
        let restart_threshold = if !healthy && !process_active {
            1usize
        } else if health_snapshot.is_wedged() {
            DASHBOARD_WATCHDOG_WEDGE_FAIL_STREAK_THRESHOLD
        } else {
            DASHBOARD_WATCHDOG_FAIL_STREAK_THRESHOLD
        };
        if fail_streak >= restart_threshold {
            let diagnostic = json!({
                "ok": true,
                "type": "dashboard_watchdog",
                "event": "restart_triggered",
                "fail_streak": fail_streak,
                "restart_threshold": restart_threshold,
                "process_active": process_active,
                "health": health_snapshot.to_json(),
                "diagnostic_path": dashboard_watchdog_diagnostic_path(root).to_string_lossy().to_string(),
            });
            append_watchdog_log(root, &diagnostic);
            write_dashboard_watchdog_diagnostic(root, &diagnostic);
            let restarted = restart_dashboard_for_watchdog(root, &cfg);
            append_watchdog_log(
                root,
                &json!({
                    "ok": true,
                    "type": "dashboard_watchdog",
                    "event": "restart_result",
                    "payload": restarted,
                }),
            );
            if restarted.get("running").and_then(Value::as_bool) == Some(true) {
                fail_streak = 0;
            } else {
                std::thread::sleep(Duration::from_millis(1_500));
            }
        }
        std::thread::sleep(Duration::from_millis(cfg.watchdog_interval_ms));
    }
    let _ = fs::remove_file(dashboard_watchdog_pid_path(root));
    append_watchdog_log(
        root,
        &json!({
            "ok": exit_code == 0,
            "type": "dashboard_watchdog",
            "event": "stopped",
            "reason": exit_reason,
            "host": cfg.host,
            "port": cfg.port,
        }),
    );
    print_json_line(&json!({
        "ok": exit_code == 0,
        "type": "dashboard_watchdog",
        "running": false,
        "reason": exit_reason,
        "host": cfg.host,
        "port": cfg.port,
    }));
    exit_code
}

fn ensure_gateway_supervisor(
    root: &Path,
    cfg: &DashboardLaunchConfig,
    dashboard: &mut Value,
    force_refresh: bool,
) -> Value {
    if !cfg.persistent_supervisor {
        return json!({
            "ok": true,
            "active": false,
            "running": false,
            "platform": "local_watchdog_or_manual",
            "skipped": true,
            "reason": "gateway_persist_disabled"
        });
    }
    let existing = gateway_supervisor::status(root);
    let existing_healthy = supervisor_payload_healthy(&existing.payload);
    let supervisor = if should_refresh_supervisor(force_refresh, existing_healthy) {
        gateway_supervisor_enable(root, cfg)
    } else {
        existing
    };
    let supervisor_healthy = supervisor_payload_healthy(&supervisor.payload);
    if !supervisor_healthy {
        let fallback = spawn_dashboard_watchdog(root, cfg);
        dashboard["watchdog_fallback"] = match fallback {
            Ok(pid) => json!({
                "ok": true,
                "running": true,
                "pid": pid,
                "mode": "local_watchdog_fallback",
            }),
            Err(err) => json!({
                "ok": false,
                "running": false,
                "error": err,
                "mode": "local_watchdog_fallback",
            }),
        };
    }
    supervisor.payload
}

fn supervisor_payload_running(supervisor: &Value) -> bool {
    supervisor
        .get("running")
        .and_then(Value::as_bool)
        .or_else(|| supervisor.get("active").and_then(Value::as_bool))
        .unwrap_or(false)
}

fn supervisor_payload_healthy(supervisor: &Value) -> bool {
    supervisor
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && supervisor
            .get("active")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        && supervisor_payload_running(supervisor)
}

fn should_refresh_supervisor(force_refresh: bool, supervisor_healthy: bool) -> bool {
    force_refresh || !supervisor_healthy
}
