// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

use crate::contract_lane_utils as lane_utils;
use crate::now_iso;

fn usage() {
    println!("conduit-client-security-kernel commands:");
    println!(
        "  infring-ops conduit-client-security-kernel build-security --payload-base64=<json>"
    );
    println!(
        "  infring-ops conduit-client-security-kernel build-envelope --payload-base64=<json>"
    );
    println!("  infring-ops conduit-client-security-kernel resolve-security-config --payload-base64=<json>");
    println!("  infring-ops conduit-client-security-kernel resolve-transport-policy --payload-base64=<json>");
}

fn print_json_line(value: &Value) {
    crate::contract_lane_utils::print_json_line(value);
}

fn stable_value(value: &Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.iter().map(stable_value).collect::<Vec<_>>()),
        Value::Object(map) => {
            let mut sorted = map.iter().collect::<Vec<_>>();
            sorted.sort_by(|(a, _), (b, _)| a.cmp(b));
            let mut out = Map::new();
            for (key, row) in sorted {
                out.insert(key.clone(), stable_value(row));
            }
            Value::Object(out)
        }
        _ => value.clone(),
    }
}

fn stable_json_string(value: &Value) -> String {
    serde_json::to_string(&stable_value(value)).unwrap_or_else(|_| "{}".to_string())
}

fn sign_value(key_id: &str, secret: &str, value: &Value) -> String {
    let canonical = stable_json_string(value);
    let mut hasher = Sha256::new();
    hasher.update(format!("{key_id}:{secret}:{canonical}").as_bytes());
    format!("{:x}", hasher.finalize())
}

fn required_scope(command_type: &str) -> &'static str {
    match command_type {
        "start_agent" | "stop_agent" => "agent.lifecycle",
        "query_receipt_chain" => "receipt.read",
        "list_active_agents" | "get_system_status" => "system.read",
        "apply_policy_update" => "policy.update",
        "install_extension" => "extension.install",
        _ => "system.read",
    }
}

fn field_string(map: &Map<String, Value>, key: &str, fallback: &str) -> String {
    map.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or(fallback)
        .to_string()
}

fn field_u64(map: &Map<String, Value>, key: &str, fallback: u64) -> u64 {
    map.get(key).and_then(Value::as_u64).unwrap_or(fallback)
}

fn normalized_env_value(value: &str) -> String {
    value.trim().to_lowercase()
}

fn is_production_release_channel(value: &str) -> bool {
    matches!(
        value,
        "production" | "prod" | "release" | "stable" | "ga"
    )
}

fn is_production_mode() -> bool {
    ["NODE_ENV", "INFRING_ENV", "INFRING_RUNTIME_ENV", "INFRING_PROFILE", "INFRING_RELEASE_CHANNEL"]
        .iter()
        .filter_map(|key| std::env::var(key).ok())
        .map(|value| normalized_env_value(&value))
        .any(|normalized| normalized == "production" || is_production_release_channel(&normalized))
}

fn is_dev_fallback_secret(value: &str) -> bool {
    matches!(value, "conduit-dev-signing-secret" | "conduit-dev-token-secret")
}

fn assert_not_dev_fallback_production(value: &str, kind: &str) -> Result<(), String> {
    if is_production_mode() && is_dev_fallback_secret(value) {
        return Err(format!(
            "conduit_security_fallback_secret_rejected_in_production:{kind}"
        ));
    }
    Ok(())
}

fn env_string(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn env_u64(key: &str) -> Option<u64> {
    std::env::var(key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .and_then(|v| v.parse::<u64>().ok())
}

fn resolve_transport_policy(payload: &Map<String, Value>) -> Value {
    let override_ms = payload.get("timeout_ms").and_then(Value::as_u64);
    let configured = override_ms
        .or_else(|| env_u64("INFRING_CONDUIT_STDIO_TIMEOUT_MS"))
        .or_else(|| env_u64("INFRING_CONDUIT_TIMEOUT_MS"))
        .unwrap_or(30_000)
        .clamp(1_000, 300_000);
    json!({
        "stdio_timeout_ms": configured
    })
}

fn resolve_security_config(payload: &Map<String, Value>) -> Result<Value, String> {
    let override_map = payload
        .get("override")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let client_id = override_map
        .get("client_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
        .or_else(|| env_string("CONDUIT_CLIENT_ID"))
        .unwrap_or_else(|| "ts-surface".to_string());

    let signing_key_id = override_map
        .get("signing_key_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
        .or_else(|| env_string("CONDUIT_SIGNING_KEY_ID"))
        .unwrap_or_else(|| "conduit-msg-k1".to_string());

    let signing_secret = override_map
        .get("signing_secret")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
        .or_else(|| env_string("CONDUIT_SIGNING_SECRET"))
        .unwrap_or_else(|| "conduit-dev-signing-secret".to_string());
    assert_not_dev_fallback_production(&signing_secret, "signing_secret")?;

    let token_key_id = override_map
        .get("token_key_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
        .or_else(|| env_string("CONDUIT_TOKEN_KEY_ID"))
        .unwrap_or_else(|| "conduit-token-k1".to_string());

    let token_secret = override_map
        .get("token_secret")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
        .or_else(|| env_string("CONDUIT_TOKEN_SECRET"))
        .unwrap_or_else(|| "conduit-dev-token-secret".to_string());
    assert_not_dev_fallback_production(&token_secret, "token_secret")?;

    let token_ttl_ms = override_map
        .get("token_ttl_ms")
        .and_then(Value::as_u64)
        .or_else(|| env_u64("CONDUIT_TOKEN_TTL_MS"))
        .unwrap_or(300_000);

    Ok(json!({
        "client_id": client_id,
        "signing_key_id": signing_key_id,
        "signing_secret": signing_secret,
        "token_key_id": token_key_id,
        "token_secret": token_secret,
        "token_ttl_ms": token_ttl_ms
    }))
}

fn build_security(payload: &Map<String, Value>) -> Result<Value, String> {
    let request_id = field_string(payload, "request_id", "ts-request");
    let ts_ms = field_u64(
        payload,
        "ts_ms",
        chrono::Utc::now().timestamp_millis().max(0) as u64,
    );
    let command_type = payload
        .get("command")
        .and_then(Value::as_object)
        .and_then(|obj| obj.get("type"))
        .and_then(Value::as_str)
        .unwrap_or("get_system_status");
    let security = payload
        .get("security")
        .and_then(Value::as_object)
        .ok_or_else(|| "security config missing".to_string())?;

    let client_id = field_string(security, "client_id", "ts-surface");
    let signing_key_id = field_string(security, "signing_key_id", "conduit-msg-k1");
    let signing_secret = field_string(security, "signing_secret", "conduit-dev-signing-secret");
    let token_key_id = field_string(security, "token_key_id", "conduit-token-k1");
    let token_secret = field_string(security, "token_secret", "conduit-dev-token-secret");
    assert_not_dev_fallback_production(&signing_secret, "signing_secret")?;
    assert_not_dev_fallback_production(&token_secret, "token_secret")?;
    let token_ttl_ms = field_u64(security, "token_ttl_ms", 300_000);

    let issued_at_ms = chrono::Utc::now().timestamp_millis().max(0) as u64;
    let capability = required_scope(command_type);

    let token_payload = json!({
        "token_id": format!("tok-{request_id}-{issued_at_ms}"),
        "subject": client_id,
        "capabilities": [capability],
        "issued_at_ms": issued_at_ms,
        "expires_at_ms": issued_at_ms.saturating_add(token_ttl_ms)
    });
    let token_signature = sign_value(&token_key_id, &token_secret, &token_payload);
    let capability_token = json!({
        "token_id": token_payload.get("token_id").cloned().unwrap_or(Value::String("tok".to_string())),
        "subject": token_payload.get("subject").cloned().unwrap_or(Value::String("ts-surface".to_string())),
        "capabilities": token_payload.get("capabilities").cloned().unwrap_or_else(|| json!(["system.read"])),
        "issued_at_ms": token_payload.get("issued_at_ms").cloned().unwrap_or(Value::from(issued_at_ms)),
        "expires_at_ms": token_payload.get("expires_at_ms").cloned().unwrap_or(Value::from(issued_at_ms.saturating_add(token_ttl_ms))),
        "signature": token_signature
    });

    let nonce = format!("nonce-{request_id}-{issued_at_ms}");
    let signing_payload = json!({
        "schema_id": "infring_conduit",
        "schema_version": "1.0",
        "request_id": request_id,
        "ts_ms": ts_ms,
        "command": payload.get("command").cloned().unwrap_or_else(|| json!({"type":"get_system_status"})),
        "security": {
            "client_id": client_id,
            "key_id": signing_key_id,
            "nonce": nonce,
            "capability_token": capability_token
        }
    });
    let signature = sign_value(&signing_key_id, &signing_secret, &signing_payload);

    Ok(json!({
        "client_id": signing_payload["security"]["client_id"],
        "key_id": signing_payload["security"]["key_id"],
        "nonce": nonce,
        "signature": signature,
        "capability_token": capability_token
    }))
}

fn build_envelope(payload: &Map<String, Value>) -> Result<Value, String> {
    let ts_ms = field_u64(
        payload,
        "ts_ms",
        chrono::Utc::now().timestamp_millis().max(0) as u64,
    );
    let request_default = format!("ts-{ts_ms}");
    let request_id = field_string(payload, "request_id", &request_default);
    let command = payload
        .get("command")
        .cloned()
        .unwrap_or_else(|| json!({ "type": "get_system_status" }));
    let security = payload
        .get("security")
        .and_then(Value::as_object)
        .cloned()
        .ok_or_else(|| "security config missing".to_string())?;

    let mut security_payload = Map::new();
    security_payload.insert("request_id".to_string(), Value::String(request_id.clone()));
    security_payload.insert("ts_ms".to_string(), Value::from(ts_ms));
    security_payload.insert("command".to_string(), command.clone());
    security_payload.insert("security".to_string(), Value::Object(security));
    let security_metadata = build_security(&security_payload)?;

    Ok(json!({
        "schema_id": "infring_conduit",
        "schema_version": "1.0",
        "request_id": request_id,
        "ts_ms": ts_ms,
        "command": command,
        "security": security_metadata
    }))
}

pub fn run(_root: &std::path::Path, argv: &[String]) -> i32 {
    let command = argv
        .iter()
        .find(|token| !token.trim_start().starts_with("--"))
        .map(|token| token.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "build-security".to_string());

    if command == "help" || command == "--help" || command == "-h" {
        usage();
        return 0;
    }

    if command != "build-security"
        && command != "build-envelope"
        && command != "resolve-security-config"
        && command != "resolve-transport-policy"
    {
        print_json_line(&json!({
            "ok": false,
            "type": "conduit_client_security_kernel_error",
            "error": "unknown_command",
            "command": command
        }));
        return 1;
    }

    let payload = match lane_utils::payload_json(argv, "conduit_client_security_kernel") {
        Ok(v) => v,
        Err(err) => {
            print_json_line(&json!({
                "ok": false,
                "type": "conduit_client_security_kernel_error",
                "error": err
            }));
            return 1;
        }
    };

    let payload_obj = payload.as_object().cloned().unwrap_or_default();
    let security = if command == "resolve-security-config" {
        match resolve_security_config(&payload_obj) {
            Ok(v) => v,
            Err(err) => {
                print_json_line(&json!({
                    "ok": false,
                    "type": "conduit_client_security_kernel_error",
                    "error": err
                }));
                return 1;
            }
        }
    } else if command == "resolve-transport-policy" {
        resolve_transport_policy(&payload_obj)
    } else if command == "build-envelope" {
        match build_envelope(&payload_obj) {
            Ok(v) => v,
            Err(err) => {
                print_json_line(&json!({
                    "ok": false,
                    "type": "conduit_client_security_kernel_error",
                    "error": err
                }));
                return 1;
            }
        }
    } else {
        match build_security(&payload_obj) {
            Ok(v) => v,
            Err(err) => {
                print_json_line(&json!({
                    "ok": false,
                    "type": "conduit_client_security_kernel_error",
                    "error": err
                }));
                return 1;
            }
        }
    };

    let out = json!({
        "ok": true,
        "type": "conduit_client_security",
        "payload": security,
        "ts": now_iso()
    });
    print_json_line(&out);
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::env;

    fn with_release_channel(channel: Option<&str>, assert: impl FnOnce()) {
        let key = "INFRING_RELEASE_CHANNEL";
        let previous = env::var(key).ok();
        if let Some(value) = channel {
            env::set_var(key, value);
        } else {
            env::remove_var(key);
        }
        assert();
        match previous {
            Some(value) => env::set_var(key, value),
            None => env::remove_var(key),
        }
    }

    #[test]
    fn build_security_uses_command_scope() {
        let payload = json!({
            "request_id": "req-1",
            "ts_ms": 1_700_000_000_000u64,
            "command": { "type": "start_agent", "agent_id": "a-1" },
            "security": {
                "client_id": "ts-surface",
                "signing_key_id": "sig-k1",
                "signing_secret": "sig-secret",
                "token_key_id": "tok-k1",
                "token_secret": "tok-secret",
                "token_ttl_ms": 60_000u64
            }
        });
        let out =
            build_security(payload.as_object().expect("payload object")).expect("build security");
        let scopes = out
            .get("capability_token")
            .and_then(Value::as_object)
            .and_then(|token| token.get("capabilities"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(
            scopes.first().and_then(Value::as_str),
            Some("agent.lifecycle")
        );
        assert!(out
            .get("signature")
            .and_then(Value::as_str)
            .map(|sig| !sig.is_empty())
            .unwrap_or(false));
    }

    #[test]
    fn build_envelope_wraps_schema_and_security() {
        let payload = json!({
            "request_id": "req-2",
            "ts_ms": 1_700_000_100_000u64,
            "command": { "type": "get_system_status" },
            "security": {
                "client_id": "ts-surface",
                "signing_key_id": "sig-k1",
                "signing_secret": "sig-secret",
                "token_key_id": "tok-k1",
                "token_secret": "tok-secret",
                "token_ttl_ms": 120_000u64
            }
        });
        let out =
            build_envelope(payload.as_object().expect("payload object")).expect("build envelope");
        assert_eq!(
            out.get("schema_id").and_then(Value::as_str),
            Some("infring_conduit")
        );
        assert_eq!(
            out.get("schema_version").and_then(Value::as_str),
            Some("1.0")
        );
        assert_eq!(out.get("request_id").and_then(Value::as_str), Some("req-2"));
        assert_eq!(
            out.get("security")
                .and_then(Value::as_object)
                .and_then(|sec| sec.get("client_id"))
                .and_then(Value::as_str),
            Some("ts-surface")
        );
    }

    #[test]
    fn resolve_transport_policy_clamps_timeout() {
        let payload = json!({
            "timeout_ms": 999_999u64
        });
        let out = resolve_transport_policy(payload.as_object().expect("payload object"));
        assert_eq!(
            out.get("stdio_timeout_ms").and_then(Value::as_u64),
            Some(300_000)
        );
    }

    #[test]
    fn resolve_security_config_rejects_dev_fallbacks_in_production() {
        with_release_channel(Some("stable"), || {
            let payload = json!({});
            let result = resolve_security_config(payload.as_object().expect("payload object"));
            match result {
                Ok(_) => panic!("expected production fallback rejection"),
                Err(err) => assert!(
                    err.contains("conduit_security_fallback_secret_rejected_in_production"),
                    "unexpected error: {err}"
                ),
            }
        });
    }

    #[test]
    fn build_security_rejects_dev_fallbacks_in_production() {
        with_release_channel(Some("stable"), || {
            let payload = json!({
                "request_id": "req-prod-fallback",
                "command": { "type": "get_system_status" },
                "security": {
                    "client_id": "ts-surface",
                    "signing_key_id": "sig-key",
                    "signing_secret": "conduit-dev-signing-secret",
                    "token_key_id": "tok-key",
                    "token_secret": "conduit-dev-token-secret",
                    "token_ttl_ms": 300_000u64
                }
            });
            let result = build_security(payload.as_object().expect("payload object"));
            match result {
                Ok(_) => panic!("expected production fallback rejection"),
                Err(err) => assert!(err.contains("conduit_security_fallback_secret_rejected_in_production")),
            }
        });
    }
}
