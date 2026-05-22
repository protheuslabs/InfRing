fn read_state(path: &Path) -> SecretBrokerState {
    lane_utils::read_json(path)
        .and_then(|value| serde_json::from_value::<SecretBrokerState>(value).ok())
        .unwrap_or_else(|| SecretBrokerState {
            version: "1.1".to_string(),
            issued: BTreeMap::new(),
        })
}

fn write_state(path: &Path, state: &SecretBrokerState) -> Result<(), String> {
    let payload = serde_json::to_value(state)
        .map_err(|err| format!("secret_broker_kernel_state_encode_failed:{err}"))?;
    lane_utils::write_json(path, &payload)
}

fn provider_type_name(provider: &ProviderConfig) -> &'static str {
    match provider {
        ProviderConfig::Env { .. } => "env",
        ProviderConfig::JsonFile { .. } => "json_file",
        ProviderConfig::EncryptedFile { .. } => "encrypted_file",
        ProviderConfig::Command { .. } => "command",
    }
}

fn command_provider_ref(command: &CommandSpec) -> String {
    match command {
        CommandSpec::Argv(argv) => argv.first().cloned().unwrap_or_default(),
        CommandSpec::Shell(shell) => shell.clone(),
    }
}

fn provider_env(provider: &ProviderConfig) -> Option<Value> {
    let ProviderConfig::Env {
        env,
        rotated_at_env,
        ..
    } = provider
    else {
        return None;
    };
    let value = std::env::var(env).ok()?.trim().to_string();
    if value.is_empty() {
        return None;
    }
    let rotated_at = if rotated_at_env.trim().is_empty() {
        Value::Null
    } else {
        std::env::var(rotated_at_env)
            .ok()
            .filter(|row| !row.trim().is_empty())
            .map(Value::String)
            .unwrap_or(Value::Null)
    };
    Some(json!({
        "ok": true,
        "value": value,
        "rotated_at": rotated_at,
        "provider_type": "env",
        "provider_ref": env,
        "external": true
    }))
}

fn provider_json_file(root: &Path, secret_id: &str, provider: &ProviderConfig) -> Option<Value> {
    let ProviderConfig::JsonFile {
        paths,
        field,
        rotated_at_field,
        ..
    } = provider
    else {
        return None;
    };
    for raw_path in paths {
        let resolved = resolve_template(root, raw_path, secret_id);
        let resolved_path = PathBuf::from(&resolved);
        if !resolved_path.exists() {
            continue;
        }
        let Ok(text) = fs::read_to_string(&resolved_path) else {
            continue;
        };
        let Ok(payload) = serde_json::from_str::<Value>(&text) else {
            continue;
        };
        let Some(value) = get_path_value(&payload, field).and_then(Value::as_str) else {
            continue;
        };
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }
        let rotated_at = get_path_value(&payload, rotated_at_field)
            .cloned()
            .unwrap_or(Value::Null);
        return Some(json!({
            "ok": true,
            "value": trimmed,
            "rotated_at": rotated_at,
            "provider_type": "json_file",
            "provider_ref": resolved,
            "external": false
        }));
    }
    None
}

fn encrypted_file_aad(secret_id: &str) -> String {
    format!("secret_broker_encrypted_file_v1:{secret_id}")
}

fn encrypted_file_key(root: &Path) -> Result<[u8; 32], String> {
    let key = secret_broker_key(root)?;
    let digest = Sha256::digest(key.as_bytes());
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest[..32]);
    Ok(out)
}

fn encrypt_secret_payload(root: &Path, secret_id: &str, payload: &Value) -> Result<Value, String> {
    let encoded =
        serde_json::to_vec(payload).map_err(|err| format!("secret_payload_encode_failed:{err}"))?;
    let key_bytes = encrypted_file_key(root)?;
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let mut nonce_bytes = [0u8; 12];
    rand::rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let aad = encrypted_file_aad(secret_id);
    let ciphertext = cipher
        .encrypt(
            nonce,
            aes_gcm::aead::Payload {
                msg: &encoded,
                aad: aad.as_bytes(),
            },
        )
        .map_err(|err| format!("secret_payload_encrypt_failed:{err}"))?;
    let ciphertext_b64 = BASE64_STANDARD.encode(ciphertext);
    Ok(json!({
        "version": "secret_broker_encrypted_file_v1",
        "secret_id": secret_id,
        "updated_at": now_iso(),
        "encryption": {
            "alg": "aes-256-gcm",
            "key_source": "secret_broker_local_key",
            "aad": aad,
            "nonce_b64": BASE64_STANDARD.encode(nonce_bytes)
        },
        "ciphertext_b64": ciphertext_b64,
        "ciphertext_sha256": hex::encode(Sha256::digest(ciphertext_b64.as_bytes()))
    }))
}

fn decrypt_secret_payload(root: &Path, secret_id: &str, envelope: &Value) -> Result<Value, String> {
    let encryption = envelope
        .get("encryption")
        .and_then(Value::as_object)
        .ok_or_else(|| "encrypted_file_encryption_missing".to_string())?;
    let alg = encryption
        .get("alg")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if alg != "aes-256-gcm" {
        return Err("encrypted_file_alg_unsupported".to_string());
    }
    let aad = encryption
        .get("aad")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let expected_aad = encrypted_file_aad(secret_id);
    if aad != expected_aad {
        return Err("encrypted_file_aad_mismatch".to_string());
    }
    let nonce_b64 = encryption
        .get("nonce_b64")
        .and_then(Value::as_str)
        .ok_or_else(|| "encrypted_file_nonce_missing".to_string())?;
    let nonce_bytes = BASE64_STANDARD
        .decode(nonce_b64.as_bytes())
        .map_err(|err| format!("encrypted_file_nonce_decode_failed:{err}"))?;
    if nonce_bytes.len() != 12 {
        return Err("encrypted_file_nonce_len_invalid".to_string());
    }
    let ciphertext_b64 = envelope
        .get("ciphertext_b64")
        .and_then(Value::as_str)
        .ok_or_else(|| "encrypted_file_ciphertext_missing".to_string())?;
    let ciphertext = BASE64_STANDARD
        .decode(ciphertext_b64.as_bytes())
        .map_err(|err| format!("encrypted_file_ciphertext_decode_failed:{err}"))?;
    let key_bytes = encrypted_file_key(root)?;
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let plain = cipher
        .decrypt(
            Nonce::from_slice(&nonce_bytes),
            aes_gcm::aead::Payload {
                msg: &ciphertext,
                aad: aad.as_bytes(),
            },
        )
        .map_err(|err| format!("encrypted_file_decrypt_failed:{err}"))?;
    serde_json::from_slice::<Value>(&plain)
        .map_err(|err| format!("encrypted_file_plaintext_decode_failed:{err}"))
}

fn write_encrypted_file(path: &Path, envelope: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("secret_broker_kernel_create_secret_dir_failed:{err}"))?;
    }
    let encoded = serde_json::to_vec_pretty(envelope)
        .map_err(|err| format!("secret_broker_kernel_secret_encode_failed:{err}"))?;
    fs::write(path, encoded).map_err(|err| format!("secret_broker_kernel_write_secret_failed:{err}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

fn provider_encrypted_file(root: &Path, secret_id: &str, provider: &ProviderConfig) -> Option<Value> {
    let ProviderConfig::EncryptedFile {
        paths,
        field,
        rotated_at_field,
        ..
    } = provider
    else {
        return None;
    };
    for raw_path in paths {
        let resolved = resolve_template(root, raw_path, secret_id);
        let resolved_path = PathBuf::from(&resolved);
        if !resolved_path.exists() {
            continue;
        }
        let Ok(text) = fs::read_to_string(&resolved_path) else {
            return Some(json!({
                "ok": false,
                "reason": "encrypted_file_read_failed",
                "provider_type": "encrypted_file",
                "provider_ref": resolved
            }));
        };
        let Ok(envelope) = serde_json::from_str::<Value>(&text) else {
            return Some(json!({
                "ok": false,
                "reason": "encrypted_file_json_invalid",
                "provider_type": "encrypted_file",
                "provider_ref": resolved
            }));
        };
        let payload = match decrypt_secret_payload(root, secret_id, &envelope) {
            Ok(payload) => payload,
            Err(err) => {
                return Some(json!({
                    "ok": false,
                    "reason": err,
                    "provider_type": "encrypted_file",
                    "provider_ref": resolved
                }));
            }
        };
        let Some(value) = get_path_value(&payload, field).and_then(Value::as_str) else {
            return Some(json!({
                "ok": false,
                "reason": "encrypted_file_value_missing",
                "provider_type": "encrypted_file",
                "provider_ref": resolved
            }));
        };
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Some(json!({
                "ok": false,
                "reason": "encrypted_file_value_empty",
                "provider_type": "encrypted_file",
                "provider_ref": resolved
            }));
        }
        let rotated_at = get_path_value(&payload, rotated_at_field)
            .cloned()
            .unwrap_or(Value::Null);
        return Some(json!({
            "ok": true,
            "value": trimmed,
            "rotated_at": rotated_at,
            "provider_type": "encrypted_file",
            "provider_ref": resolved,
            "external": false
        }));
    }
    None
}

fn provider_command(secret_id: &str, provider: &ProviderConfig) -> Option<Value> {
    let ProviderConfig::Command {
        command,
        parse_json,
        value_path,
        rotated_at_path,
        env,
        ..
    } = provider
    else {
        return None;
    };
    let mut command_builder = match command {
        CommandSpec::Argv(argv) if !argv.is_empty() => {
            let mut builder = Command::new(&argv[0]);
            builder.args(&argv[1..]);
            builder
        }
        CommandSpec::Shell(shell) => {
            let mut builder = Command::new("/bin/sh");
            builder.args(["-lc", shell]);
            builder
        }
        _ => return None,
    };
    command_builder.env("SECRET_ID", secret_id);
    command_builder.env("SECRET_BROKER_SECRET_ID", secret_id);
    for (key, value) in env {
        command_builder.env(key, value);
    }
    let output = command_builder.output().ok()?;
    if !output.status.success() {
        return Some(json!({
            "ok": false,
            "reason": "command_exit_nonzero",
            "code": output.status.code().unwrap_or(1),
            "stderr": String::from_utf8_lossy(&output.stderr).trim().chars().take(200).collect::<String>(),
        }));
    }
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        return Some(json!({
            "ok": false,
            "reason": "command_empty_stdout",
        }));
    }
    if !parse_json {
        return Some(json!({
            "ok": true,
            "value": stdout,
            "rotated_at": Value::Null,
            "provider_type": "command",
            "provider_ref": command_provider_ref(command),
            "external": true
        }));
    }
    let Ok(payload) = serde_json::from_str::<Value>(&stdout) else {
        return Some(json!({
            "ok": false,
            "reason": "command_json_invalid"
        }));
    };
    let value = get_path_value(&payload, value_path)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    if value.is_empty() {
        return Some(json!({
            "ok": false,
            "reason": "command_value_missing"
        }));
    }
    let rotated_at = get_path_value(&payload, rotated_at_path)
        .cloned()
        .unwrap_or(Value::Null);
    Some(json!({
        "ok": true,
        "value": value,
        "rotated_at": rotated_at,
        "provider_type": "command",
        "provider_ref": command_provider_ref(command),
        "external": true
    }))
}

fn evaluate_rotation(
    rotation_cfg: &RotationConfig,
    rotated_at: Option<&Value>,
    now_ms: i64,
) -> RotationHealth {
    let rotated_at_ms = rotated_at.and_then(parse_ts_ms);
    if rotated_at_ms.is_none() {
        return RotationHealth {
            status: if rotation_cfg.require_rotated_at {
                "critical".to_string()
            } else {
                "unknown".to_string()
            },
            reason: "rotated_at_missing".to_string(),
            rotated_at: None,
            age_days: None,
            warn_after_days: rotation_cfg.warn_after_days,
            max_after_days: rotation_cfg.max_after_days,
            require_rotated_at: rotation_cfg.require_rotated_at,
            enforce_on_issue: rotation_cfg.enforce_on_issue,
        };
    }
    let rotated_at_ms = rotated_at_ms.unwrap_or(now_ms);
    let age_days = ((now_ms - rotated_at_ms).max(0) as f64) / 86_400_000f64;
    let (status, reason) = if age_days > rotation_cfg.max_after_days {
        ("critical", "rotation_age_exceeded")
    } else if age_days > rotation_cfg.warn_after_days {
        ("warn", "rotation_age_warning")
    } else {
        ("ok", "rotation_fresh")
    };
    RotationHealth {
        status: status.to_string(),
        reason: reason.to_string(),
        rotated_at: Some(iso_from_ms(rotated_at_ms)),
        age_days: Some((age_days * 1000.0).round() / 1000.0),
        warn_after_days: rotation_cfg.warn_after_days,
        max_after_days: rotation_cfg.max_after_days,
        require_rotated_at: rotation_cfg.require_rotated_at,
        enforce_on_issue: rotation_cfg.enforce_on_issue,
    }
}

fn load_secret_by_id(
    root: &Path,
    payload: &Map<String, Value>,
    policy: &SecretBrokerPolicy,
    audit_path: &Path,
    with_audit: bool,
) -> LoadedSecret {
    let secret_id = text(payload.get("secret_id"), 160);
    let now = now_ms(payload);
    let Some(spec) = policy.secrets.get(&secret_id) else {
        return LoadedSecret {
            ok: false,
            secret_id,
            error: Some("secret_id_unsupported".to_string()),
            ..LoadedSecret::default()
        };
    };
    let mut provider_errors = Vec::new();
    for provider in &spec.providers {
        let enabled = match provider {
            ProviderConfig::Env { enabled, .. }
            | ProviderConfig::JsonFile { enabled, .. }
            | ProviderConfig::EncryptedFile { enabled, .. }
            | ProviderConfig::Command { enabled, .. } => *enabled,
        };
        if !enabled {
            continue;
        }
        let result = match provider {
            ProviderConfig::Env { .. } => provider_env(provider),
            ProviderConfig::JsonFile { .. } => provider_json_file(root, &secret_id, provider),
            ProviderConfig::EncryptedFile { .. } => {
                provider_encrypted_file(root, &secret_id, provider)
            }
            ProviderConfig::Command { .. } => provider_command(&secret_id, provider),
        };
        let Some(result) = result else {
            provider_errors.push(json!({
                "provider_type": provider_type_name(provider),
                "reason": "provider_failed"
            }));
            continue;
        };
        if result.get("ok").and_then(Value::as_bool) != Some(true) {
            provider_errors.push(json!({
                "provider_type": result.get("provider_type").and_then(Value::as_str).unwrap_or(provider_type_name(provider)),
                "reason": result.get("reason").and_then(Value::as_str).unwrap_or("provider_failed"),
                "code": result.get("code").cloned().unwrap_or(Value::Null),
                "ref": result.get("provider_ref").cloned().unwrap_or(Value::Null)
            }));
            continue;
        }
        let value = text(result.get("value"), 8192);
        if value.is_empty() {
            provider_errors.push(json!({
                "provider_type": result.get("provider_type").and_then(Value::as_str).unwrap_or("unknown"),
                "reason": "value_empty"
            }));
            continue;
        }
        let rotation = evaluate_rotation(&spec.rotation, result.get("rotated_at"), now);
        let backend = ResolvedBackend {
            provider_type: text(result.get("provider_type"), 64),
            provider_ref: {
                let v = text(result.get("provider_ref"), 240);
                if v.is_empty() {
                    None
                } else {
                    Some(v)
                }
            },
            external: bool_value(result.get("external"), false),
        };
        if with_audit {
            let _ = append_audit(
                audit_path,
                json!({
                    "type": "secret_value_loaded",
                    "secret_id": secret_id,
                    "provider_type": backend.provider_type,
                    "provider_ref": if policy.include_backend_details { backend.provider_ref.clone() } else { None },
                    "external_backend": backend.external,
                    "value_hash": sha16(&value),
                    "rotation_status": rotation.status,
                    "rotation_age_days": rotation.age_days,
                }),
            );
        }
        return LoadedSecret {
            ok: true,
            secret_id: secret_id.clone(),
            value: value.clone(),
            value_hash: sha16(&value),
            backend: Some(backend),
            rotation: Some(rotation),
            error: None,
            provider_errors: Vec::new(),
        };
    }
    if with_audit {
        let _ = append_audit(
            audit_path,
            json!({
                "type": "secret_value_load_failed",
                "secret_id": secret_id,
                "reason": "all_providers_failed",
                "provider_errors": provider_errors,
            }),
        );
    }
    LoadedSecret {
        ok: false,
        secret_id,
        error: Some("secret_value_missing".to_string()),
        provider_errors,
        ..LoadedSecret::default()
    }
}

fn put_secret_by_id(
    root: &Path,
    payload: &Map<String, Value>,
    policy: &SecretBrokerPolicy,
    audit_path: &Path,
    with_audit: bool,
) -> Value {
    let secret_id = text(payload.get("secret_id"), 160);
    let value = text(
        payload.get("value").or_else(|| payload.get("api_key")),
        8192,
    );
    if value.is_empty() {
        return json!({
            "ok": false,
            "secret_id": secret_id,
            "error": "secret_value_missing"
        });
    }
    let Some(spec) = policy.secrets.get(&secret_id) else {
        return json!({
            "ok": false,
            "secret_id": secret_id,
            "error": "secret_id_unsupported"
        });
    };
    let mut target: Option<(String, String, String)> = None;
    for provider in &spec.providers {
        if let ProviderConfig::EncryptedFile {
            enabled,
            paths,
            field,
            rotated_at_field,
        } = provider
        {
            if *enabled {
                if let Some(path) = paths.first() {
                    target = Some((path.clone(), field.clone(), rotated_at_field.clone()));
                    break;
                }
            }
        }
    }
    let Some((path, field, rotated_at_field)) = target else {
        return json!({
            "ok": false,
            "secret_id": secret_id,
            "error": "encrypted_file_provider_missing"
        });
    };
    let rotated_at = payload
        .get("rotated_at")
        .and_then(Value::as_str)
        .filter(|row| !row.trim().is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(now_iso);
    let mut secret_payload = Map::new();
    secret_payload.insert(field, Value::String(value.clone()));
    secret_payload.insert(rotated_at_field, Value::String(rotated_at));
    let envelope = match encrypt_secret_payload(root, &secret_id, &Value::Object(secret_payload)) {
        Ok(envelope) => envelope,
        Err(err) => {
            return json!({
                "ok": false,
                "secret_id": secret_id,
                "error": err
            });
        }
    };
    let resolved = resolve_template(root, &path, &secret_id);
    let resolved_path = PathBuf::from(&resolved);
    if let Err(err) = write_encrypted_file(&resolved_path, &envelope) {
        return json!({
            "ok": false,
            "secret_id": secret_id,
            "error": err
        });
    }
    if with_audit {
        let _ = append_audit(
            audit_path,
            json!({
                "type": "secret_value_stored",
                "secret_id": secret_id,
                "provider_type": "encrypted_file",
                "provider_ref": if policy.include_backend_details { Some(resolved.clone()) } else { None },
                "external_backend": false,
                "value_hash": sha16(&value)
            }),
        );
    }
    json!({
        "ok": true,
        "type": "secret_value_stored",
        "secret_id": secret_id,
        "provider_type": "encrypted_file",
        "provider_ref": if policy.include_backend_details { Value::String(resolved) } else { Value::Null },
        "value_hash": sha16(&value)
    })
}
