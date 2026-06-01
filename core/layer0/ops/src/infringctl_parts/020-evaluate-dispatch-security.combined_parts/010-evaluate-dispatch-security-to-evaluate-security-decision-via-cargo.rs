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

fn bool_env_with_infring_alias(primary: &str, alias: &str, fallback: bool) -> bool {
    bool_env(primary, bool_env(alias, fallback))
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
