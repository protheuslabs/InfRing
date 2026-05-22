fn emit_secret_broker_error(error: &str) -> i32 {
    print_json_line(&cli_error("secret_broker_kernel", error));
    2
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let command = argv
        .first()
        .map(|row| row.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let payload_value = match payload_json(argv) {
        Ok(value) => value,
        Err(err) => return emit_secret_broker_error(&err),
    };
    let payload = payload_obj(&payload_value);
    let with_audit = bool_value(payload.get("with_audit"), true);
    let policy = load_policy(root, payload);
    let state_path = resolve_path(
        root,
        payload,
        "state_path",
        "SECRET_BROKER_STATE_PATH",
        default_state_path(root),
    );
    let audit_path = resolve_path(
        root,
        payload,
        "audit_path",
        "SECRET_BROKER_AUDIT_PATH",
        default_audit_path(root),
    );
    let result = match command.as_str() {
        "load-policy" => json!({
            "ok": true,
            "policy": policy,
        }),
        "load-secret" => serde_json::to_value(load_secret_by_id(
            root,
            payload,
            &policy,
            &audit_path,
            with_audit,
        ))
        .unwrap_or_else(|_| json!({ "ok": false, "error": "secret_value_missing" })),
        "put-secret" => put_secret_by_id(root, payload, &policy, &audit_path, with_audit),
        "rotation-health" => serde_json::to_value(rotation_health_report(
            root,
            payload,
            &policy,
            &audit_path,
            with_audit,
        ))
        .unwrap_or_else(|_| json!({ "ok": false, "error": "rotation_health_failed" })),
        "status" => secret_broker_status(root, payload, &policy, &state_path, &audit_path),
        "issue-handle" => issue_handle(root, payload, &policy, &state_path, &audit_path),
        "resolve-handle" => resolve_handle(root, payload, &policy, &state_path, &audit_path),
        _ => return emit_secret_broker_error("unknown_command"),
    };
    let ok = result.get("ok").and_then(Value::as_bool).unwrap_or(false);
    print_json_line(&cli_receipt("secret_broker_kernel", result));
    if ok {
        0
    } else {
        2
    }
}

pub(crate) fn resolve_secret_value_for_runtime(
    root: &Path,
    secret_id: &str,
    caller: &str,
) -> Value {
    let payload_value = json!({
        "secret_id": secret_id,
        "caller": caller,
        "with_audit": true
    });
    let payload = payload_obj(&payload_value);
    let policy = load_policy(root, payload);
    serde_json::to_value(load_secret_by_id(
        root,
        payload,
        &policy,
        &default_audit_path(root),
        true,
    ))
    .unwrap_or_else(|_| json!({
        "ok": false,
        "secret_id": secret_id,
        "error": "secret_value_missing"
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn temp_root() -> tempfile::TempDir {
        let dir = tempdir().expect("tempdir");
        fs::create_dir_all(dir.path().join("client/runtime/config")).expect("config");
        dir
    }

    #[test]
    fn issue_and_resolve_handle_round_trip() {
        let root = temp_root();
        let secret_dir = root.path().join(".secrets-roundtrip");
        fs::create_dir_all(&secret_dir).expect("secret_dir");
        let policy_path = root
            .path()
            .join("client/runtime/config/secret_broker_policy.json");
        fs::write(
            &policy_path,
            format!(
                "{{\"version\":\"1.0\",\"audit\":{{\"include_backend_details\":true}},\"command_backend\":{{\"timeout_ms\":5000}},\"secrets\":{{\"moltbook_api_key\":{{\"providers\":[{{\"type\":\"json_file\",\"paths\":[\"{}\"],\"field\":\"api_key\",\"rotated_at_field\":\"rotated_at\"}}]}}}}}}",
                secret_dir
                    .join("moltbook.credentials.json")
                    .to_string_lossy()
                    .replace('\\', "\\\\")
            ),
        )
        .expect("policy");
        fs::write(
            secret_dir.join("moltbook.credentials.json"),
            "{\"api_key\":\"mb-test\",\"rotated_at\":\"2026-03-01T00:00:00Z\"}",
        )
        .expect("credentials");
        std::env::set_var("SECRET_BROKER_KEY", "test-secret-key");
        let payload = json!({
            "secret_id": "moltbook_api_key",
            "scope": "scope.test",
            "caller": "caller.test",
            "ttl_sec": 60,
            "policy_path": policy_path.to_string_lossy().to_string()
        });
        let policy = load_policy(root.path(), payload_obj(&payload));
        let state_path = default_state_path(root.path());
        let audit_path = default_audit_path(root.path());
        let issued = issue_handle(
            root.path(),
            payload_obj(&payload),
            &policy,
            &state_path,
            &audit_path,
        );
        assert_eq!(issued.get("ok").and_then(Value::as_bool), Some(true));
        let resolved = resolve_handle(
            root.path(),
            payload_obj(&json!({
                "handle": issued.get("handle").and_then(Value::as_str).unwrap_or_default(),
                "scope": "scope.test",
                "caller": "caller.test"
            })),
            &policy,
            &state_path,
            &audit_path,
        );
        assert_eq!(resolved.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            resolved.get("value").and_then(Value::as_str),
            Some("mb-test")
        );
    }

    #[test]
    fn load_secret_supports_json_file_provider() {
        let root = temp_root();
        let secret_dir = root.path().join(".secrets");
        fs::create_dir_all(&secret_dir).expect("secret_dir");
        let policy_path = root
            .path()
            .join("client/runtime/config/secret_broker_policy.json");
        fs::write(
            &policy_path,
            format!(
                "{{\"version\":\"1.0\",\"audit\":{{\"include_backend_details\":true}},\"command_backend\":{{\"timeout_ms\":5000}},\"secrets\":{{\"moltbook_api_key\":{{\"providers\":[{{\"type\":\"json_file\",\"paths\":[\"{}\"],\"field\":\"api_key\",\"rotated_at_field\":\"rotated_at\"}}]}}}}}}",
                secret_dir
                    .join("moltbook.credentials.json")
                    .to_string_lossy()
                    .replace('\\', "\\\\")
            ),
        )
        .expect("policy");
        fs::write(
            secret_dir.join("moltbook.credentials.json"),
            "{\"api_key\":\"json-secret\",\"rotated_at\":\"2026-03-01T00:00:00Z\"}",
        )
        .expect("json");
        let policy = load_policy(
            root.path(),
            payload_obj(&json!({
                "policy_path": policy_path.to_string_lossy().to_string()
            })),
        );
        let loaded = load_secret_by_id(
            root.path(),
            payload_obj(&json!({ "secret_id": "moltbook_api_key" })),
            &policy,
            &default_audit_path(root.path()),
            false,
        );
        assert!(loaded.ok);
        assert_eq!(loaded.value, "json-secret");
    }

    #[test]
    fn put_secret_uses_encrypted_local_file_provider() {
        let root = temp_root();
        let policy_path = root
            .path()
            .join("client/runtime/config/secret_broker_policy.json");
        let vault_path = root
            .path()
            .join("client/runtime/local/secrets/vault/web/search/tavily.secret.json");
        fs::write(
            &policy_path,
            format!(
                "{{\"version\":\"1.0\",\"audit\":{{\"include_backend_details\":true}},\"secrets\":{{\"web_search_tavily_api_key\":{{\"providers\":[{{\"type\":\"encrypted_file\",\"paths\":[\"{}\"],\"field\":\"api_key\",\"rotated_at_field\":\"rotated_at\"}}]}}}}}}",
                vault_path.to_string_lossy().replace('\\', "\\\\")
            ),
        )
        .expect("policy");
        std::env::set_var("SECRET_BROKER_KEY", "test-secret-key");
        let policy = load_policy(
            root.path(),
            payload_obj(&json!({
                "policy_path": policy_path.to_string_lossy().to_string()
            })),
        );
        let stored = put_secret_by_id(
            root.path(),
            payload_obj(&json!({
                "secret_id": "web_search_tavily_api_key",
                "value": "tvly-test",
                "rotated_at": "2026-05-20T00:00:00Z"
            })),
            &policy,
            &default_audit_path(root.path()),
            false,
        );
        assert_eq!(stored.get("ok").and_then(Value::as_bool), Some(true));
        let raw = fs::read_to_string(&vault_path).expect("vault file");
        assert!(!raw.contains("tvly-test"));
        assert!(raw.contains("secret_broker_encrypted_file_v1"));
        let loaded = load_secret_by_id(
            root.path(),
            payload_obj(&json!({ "secret_id": "web_search_tavily_api_key" })),
            &policy,
            &default_audit_path(root.path()),
            false,
        );
        assert!(loaded.ok);
        assert_eq!(loaded.value, "tvly-test");
        assert_eq!(
            loaded.backend.as_ref().map(|row| row.provider_type.as_str()),
            Some("encrypted_file")
        );
    }
}
