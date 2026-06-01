pub fn evaluate_dispatch_security(
    root: &Path,
    script_rel: &str,
    args: &[String],
) -> DispatchSecurity {
    if bool_env_with_infring_alias(
        "INFRING_CTL_SECURITY_GATE_DISABLED",
        "INFRING_CTL_SECURITY_GATE_DISABLED",
        false,
    ) {
        return DispatchSecurity {
            ok: true,
            reason: "infringctl_dispatch_gate_disabled".to_string(),
        };
    }

    let workspace_root = effective_workspace_root(root);
    let req = security_request(&workspace_root, script_rel, args);
    let persona_gate = evaluate_persona_dispatch_security(script_rel, args, &req);
    if !persona_gate.ok {
        return persona_gate;
    }
    if req
        .get("covenant_violation")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || req
            .get("tamper_signal")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    {
        return DispatchSecurity {
            ok: false,
            reason: "security_gate_blocked:local_fail_closed_signal".to_string(),
        };
    }

    let request_json = serde_json::to_string(&req).unwrap_or_else(|_| "{}".to_string());
    let request_base64 = BASE64_STANDARD.encode(request_json.as_bytes());

    let payload = match evaluate_security_decision_payload(&workspace_root, &req, &request_base64) {
        Ok(value) => value,
        Err(reason) => {
            if dispatch_security_gate_exempt(script_rel, args) {
                return DispatchSecurity {
                    ok: true,
                    reason: format!(
                        "dispatch_security_degraded_allow_read_only:{}",
                        clean(reason, 180)
                    ),
                };
            }
            return DispatchSecurity {
                ok: false,
                reason: format!("security_gate_blocked:{}", clean(reason, 220)),
            };
        }
    };

    let decision = payload.get("decision").cloned().unwrap_or(Value::Null);
    let ok = decision.get("ok").and_then(Value::as_bool).unwrap_or(false);
    let fail_closed = decision
        .get("fail_closed")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    if !ok || fail_closed {
        let reason = decision
            .get("reasons")
            .and_then(Value::as_array)
            .and_then(|rows| rows.first())
            .and_then(Value::as_str)
            .unwrap_or("dispatch_security_gate_blocked")
            .to_string();
        return DispatchSecurity {
            ok: false,
            reason: format!("security_gate_blocked:{}", clean(reason, 220)),
        };
    }

    DispatchSecurity {
        ok: true,
        reason: "ok".to_string(),
    }
}

fn bool_env_with_infring_alias(infring: &str, infring: &str, fallback: bool) -> bool {
    bool_env(infring, bool_env(infring, fallback))
}

fn dispatch_security_gate_exempt(script_rel: &str, args: &[String]) -> bool {
    if script_rel == "core://daemon-control"
        && args.first().map(String::as_str) == Some("status")
    {
        return true;
    }
    if script_rel == "client/runtime/systems/ops/infring_setup_wizard.ts"
        && args.first().map(String::as_str) == Some("status")
    {
        return true;
    }
    if matches!(
        script_rel,
        "core://unknown-command"
            | "core://install-doctor"
            | "core://command-list"
            | "core://completion"
            | "core://repl"
            | "core://version-cli"
            | "core://release-semver-contract"
    ) {
        return true;
    }
    matches!(
        script_rel,
        "client/runtime/systems/ops/infring_command_list.ts"
            | "client/runtime/systems/ops/infring_command_list.js"
    )
}

fn evaluate_security_decision_payload(
    workspace_root: &Path,
    req: &Value,
    request_base64: &str,
) -> Result<Value, String> {
    match evaluate_security_decision_embedded(req) {
        Ok(payload) => Ok(payload),
        Err(embedded_error) => {
            let cargo_fallback_disabled = bool_env_with_infring_alias(
                "INFRING_CTL_SECURITY_DISABLE_CARGO_FALLBACK",
                "INFRING_CTL_SECURITY_DISABLE_CARGO_FALLBACK",
                false,
            );
            let cargo_fallback_enabled = bool_env_with_infring_alias(
                "INFRING_CTL_SECURITY_ENABLE_CARGO_FALLBACK",
                "INFRING_CTL_SECURITY_ENABLE_CARGO_FALLBACK",
                false,
            );
            if cargo_fallback_disabled || !cargo_fallback_enabled {
                return Err(format!(
                    "embedded_checker_failed:{embedded_error}; cargo_fallback_disabled"
                ));
            }
            match evaluate_security_decision_via_cargo(workspace_root, request_base64) {
                Ok(payload) => Ok(payload),
                Err(cargo_error) => Err(format!(
                    "embedded_checker_failed:{embedded_error}; cargo_fallback_failed:{cargo_error}"
                )),
            }
        }
    }
}

fn evaluate_security_decision_embedded(_req: &Value) -> Result<Value, String> {
    Err("embedded_security_checker_not_linked_use_cargo".to_string())
}

fn evaluate_security_decision_via_cargo(
    workspace_root: &Path,
    request_base64: &str,
) -> Result<Value, String> {
    let manifest = workspace_root.join("core/layer0/security/Cargo.toml");
    if !manifest.exists() {
        return Err("manifest_missing".to_string());
    }

    let output = Command::new("cargo")
        .arg("run")
        .arg("--quiet")
        .arg("--manifest-path")
        .arg(manifest)
        .arg("--bin")
        .arg("security_core")
        .arg("--")
        .arg("check")
        .arg(format!("--request-base64={request_base64}"))
        .current_dir(workspace_root)
        .output()
        .map_err(|_| "spawn_failed".to_string())?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let msg = if stderr.trim().is_empty() {
            stdout.to_string()
        } else {
            stderr.to_string()
        };
        return Err(clean(msg, 220));
    }

    parse_json(&String::from_utf8_lossy(&output.stdout))
        .ok_or_else(|| "invalid_security_payload".to_string())
}

fn run_node_script(root: &Path, script_rel: &str, args: &[String], forward_stdin: bool) -> i32 {
    let workspace_root = effective_workspace_root(root);
    let runtime_mode = resolved_runtime_mode(&workspace_root);
    if let Some((domain, mapped_args)) = maybe_redirect_ts_wrapper_to_core_domain(script_rel, args)
    {
        return run_core_domain(&workspace_root, &domain, &mapped_args, forward_stdin);
    }
    if let Some(domain) = script_rel.strip_prefix("core://") {
        return run_core_domain(&workspace_root, domain, args, forward_stdin);
    }

    let mut script_abs = workspace_root.join(script_rel);
    if !script_abs.exists() && script_rel.ends_with(".js") {
        let ts_rel = format!("{}{}", script_rel.trim_end_matches(".js"), ".ts");
        let ts_abs = workspace_root.join(&ts_rel);
        if ts_abs.exists() {
            if runtime_mode == "dist" {
                eprintln!(
                    "{}",
                    json!({
                        "ok": false,
                        "type": "infringctl_dispatch",
                        "error": "dist_source_mismatch",
                        "detail": "runtime_mode=dist requires bundled JS entrypoints; source-only TS fallback detected",
                        "script_rel": clean(script_rel, 220),
                        "script_abs": clean(script_abs.to_string_lossy().to_string(), 500),
                        "ts_candidate_rel": ts_rel,
                        "ts_candidate_exists": true,
                        "runtime_mode": runtime_mode,
                        "node_runtime_detected": has_node_runtime(),
                        "route_found": true
                    })
                );
                return 1;
            }
            script_abs = ts_abs;
        }
    }
    if !script_abs.exists() {
        let synthetic_route = Route {
            script_rel: script_rel.to_string(),
            args: args.to_vec(),
            forward_stdin,
        };
        if let Some(status) = node_missing_fallback(&workspace_root, &synthetic_route, false) {
            return status;
        }
        if matches!(
            script_rel,
            "client/runtime/systems/ops/infring_setup_wizard.ts"
                | "client/runtime/systems/ops/infring_setup_wizard.js"
        ) {
            return run_setup_wizard_missing_script_fallback(&workspace_root, args);
        }
        let ts_candidate_rel = if script_rel.ends_with(".js") {
            Some(format!("{}{}", script_rel.trim_end_matches(".js"), ".ts"))
        } else {
            None
        };
        let ts_candidate_exists = ts_candidate_rel
            .as_ref()
            .map(|rel| workspace_root.join(rel).exists())
            .unwrap_or(false);
        let script_missing_kind =
            if runtime_mode == "dist" && script_rel.ends_with(".js") && ts_candidate_exists {
                "dist_source_mismatch"
            } else {
                "script_missing"
            };
        let detail = if script_missing_kind == "dist_source_mismatch" {
            "runtime_mode=dist requires bundled JS entrypoints; source-only TS fallback detected"
        } else {
            "resolved route target script is missing from workspace runtime"
        };
        eprintln!(
            "{}",
            json!({
                "ok": false,
                "type": "infringctl_dispatch",
                "error": script_missing_kind,
                "detail": detail,
                "script_rel": clean(script_rel, 220),
                "script_abs": clean(script_abs.to_string_lossy().to_string(), 500),
                "ts_candidate_rel": ts_candidate_rel,
                "ts_candidate_exists": ts_candidate_exists,
                "runtime_mode": runtime_mode,
                "node_runtime_detected": has_node_runtime(),
                "route_found": true
            })
        );
        return 1;
    }

    let ts_entrypoint = workspace_root.join("client/runtime/lib/ts_entrypoint.ts");
    let script_is_ts = script_abs
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("ts"))
        .unwrap_or(false);

    let mut cmd = Command::new(node_bin());
    if script_is_ts && ts_entrypoint.exists() {
        cmd.arg(ts_entrypoint).arg(&script_abs);
    } else {
        cmd.arg(&script_abs);
    }

    cmd.args(args)
        .current_dir(workspace_root)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    if forward_stdin {
        cmd.stdin(Stdio::inherit());
    } else {
        cmd.stdin(Stdio::null());
    }

    match cmd.status() {
        Ok(status) => status.code().unwrap_or(1),
        Err(err) => {
            eprintln!(
                "{}",
                json!({
                    "ok": false,
                    "type": "infringctl_dispatch",
                    "error": clean(format!("spawn_failed:{err}"), 220)
                })
            );
            1
        }
    }
}

fn run_setup_wizard_missing_script_fallback(root: &Path, args: &[String]) -> i32 {
    let state_path = root
        .join("local")
        .join("state")
        .join("ops")
        .join("infring_setup_wizard")
        .join("latest.json");
    let payload = json!({
        "type": "infring_setup_wizard_state",
        "completed": false,
        "completed_at": crate::now_iso(),
        "completion_mode": "missing_script_fallback_deferred",
        "node_runtime_detected": has_node_runtime(),
        "interaction_style": "silent",
        "notifications": "none",
        "covenant_acknowledged": false,
        "next_action": "infring setup --yes --defaults",
        "version": 1
    });
    if let Some(parent) = state_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(raw) = serde_json::to_string_pretty(&payload) {
        let _ = std::fs::write(state_path, raw);
    }
    let json_mode = args.iter().any(|arg| arg == "--json" || arg == "--json=1");
    if json_mode {
        println!(
            "{}",
            json!({
                "ok": true,
                "type": "infring_setup_wizard_fallback",
                "mode": "missing_script_fallback",
                "deferred": true,
                "next_action": "infring setup --yes --defaults",
                "path_reload_command": ". \"$HOME/.infring/env.sh\" && hash -r 2>/dev/null || true",
                "node_install_hint": "curl -fsSL https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.sh | sh -s -- --full --install-node",
                "toolchain_check_command": "cargo --version && rustc --version",
                "message": "setup wizard script missing in this runtime; wrote deferred fallback state"
            })
        );
    } else {
        println!("Setup wizard script missing in this runtime; setup was deferred.");
        println!("Deterministic recovery path:");
        println!("  1) . \"$HOME/.infring/env.sh\" && hash -r 2>/dev/null || true");
        println!("  2) infring setup --yes --defaults");
        println!("  3) infring setup status --json");
        println!("  4) infring gateway status");
        println!("  5) infring doctor --json");
        println!("If Node is missing, rerun installer with `--install-node`.");
    }
    0
}

fn has_json_flag(args: &[String]) -> bool {
    args.iter().any(|arg| arg == "--json" || arg == "--json=1")
}

fn first_positional_command(args: &[String]) -> String {
    for token in args {
        let trimmed = token.trim();
        if trimmed.is_empty() || trimmed.starts_with('-') {
            continue;
        }
        return trimmed.to_string();
    }
    String::new()
}

fn run_unknown_command_domain(args: &[String]) -> i32 {
    let json_mode = has_json_flag(args);
    let command = first_positional_command(args);
    let canonical = crate::command_list_kernel::canonical_command_name(&command);
    let hint = if canonical.is_some() {
        "Run `infring help` and retry using the canonical command token."
    } else {
        "Run `infring help` to list available commands."
    };
    if json_mode {
        println!(
            "{}",
            json!({
                "ok": false,
                "type": "infringctl_dispatch",
                "error": "command_not_found",
                "error_code": "command_not_found",
                "command": clean(command, 120),
                "canonical_command": canonical,
                "hint": hint
            })
        );
    } else if command.is_empty() {
        eprintln!("[infring] unknown command");
        print_node_free_command_list("help");
    } else {
        eprintln!("[infring] unknown command: {command}");
        print_node_free_command_list("help");
    }
    2
}

fn command_available_in_current_bin_dir(name: &str) -> bool {
    let Ok(exe) = env::current_exe() else {
        return false;
    };
    let Some(dir) = exe.parent() else {
        return false;
    };
    dir.join(name).exists()
}

fn route_integrity_ok(cmd: &str, rest: &[String], expected_script: &str) -> bool {
    resolve_core_shortcuts(cmd, rest)
        .map(|route| route.script_rel == expected_script)
        .unwrap_or(false)
}
