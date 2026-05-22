fn parse_ts_ms(value: &Value) -> Option<i64> {
    if let Some(n) = value.as_i64() {
        if n > 100_000_000_000 {
            return Some(n);
        }
        if n > 1_000_000_000 {
            return Some(n * 1000);
        }
    }
    if let Some(n) = value.as_u64() {
        if n > 100_000_000_000 {
            return i64::try_from(n).ok();
        }
        if n > 1_000_000_000 {
            return i64::try_from(n).ok().map(|v| v * 1000);
        }
    }
    let raw = value.as_str()?.trim();
    if raw.is_empty() {
        return None;
    }
    if let Ok(num) = raw.parse::<i64>() {
        if num > 100_000_000_000 {
            return Some(num);
        }
        if num > 1_000_000_000 {
            return Some(num * 1000);
        }
    }
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|ts| ts.timestamp_millis())
}

fn iso_from_ms(ms: i64) -> String {
    Utc.timestamp_millis_opt(ms)
        .single()
        .unwrap_or_else(Utc::now)
        .to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn now_ms(payload: &Map<String, Value>) -> i64 {
    int_value(payload.get("now_ms")).unwrap_or_else(|| Utc::now().timestamp_millis())
}

fn append_audit(audit_path: &Path, row: Value) -> Result<(), String> {
    let mut full = json!({ "ts": now_iso() });
    if let Some(map) = row.as_object() {
        let target = full.as_object_mut().expect("object");
        for (key, value) in map {
            target.insert(key.clone(), value.clone());
        }
    }
    lane_utils::append_jsonl(audit_path, &full)
}

fn get_path_value<'a>(value: &'a Value, dotted: &str) -> Option<&'a Value> {
    let mut current = value;
    for part in dotted.split('.').filter(|part| !part.trim().is_empty()) {
        let obj = current.as_object()?;
        current = obj.get(part.trim())?;
    }
    Some(current)
}

fn resolve_template(root: &Path, raw: &str, secret_id: &str) -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    let runtime = runtime_root(root).to_string_lossy().replace('\\', "/");
    let default_dir = default_secrets_dir().to_string_lossy().replace('\\', "/");
    let mut out = raw.trim().to_string();
    out = out.replace("${HOME}", &home);
    out = out.replace("${REPO_ROOT}", &runtime);
    out = out.replace("${DEFAULT_SECRETS_DIR}", &default_dir);
    out = out.replace("${SECRET_ID}", secret_id);
    if Path::new(&out).is_absolute() {
        out
    } else {
        root.join(out).to_string_lossy().replace('\\', "/")
    }
}

fn parse_command_spec(value: &Value) -> Option<CommandSpec> {
    if let Some(raw) = value.as_str() {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return Some(CommandSpec::Shell(trimmed.to_string()));
        }
    }
    let items = value
        .as_array()?
        .iter()
        .filter_map(Value::as_str)
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>();
    if items.is_empty() {
        None
    } else {
        Some(CommandSpec::Argv(items))
    }
}

fn default_policy(root: &Path) -> SecretBrokerPolicy {
    let default_dir = default_secrets_dir();
    let home = std::env::var("HOME").unwrap_or_default();
    let mut secrets = BTreeMap::new();
    secrets.insert(
        "moltbook_api_key".to_string(),
        SecretSpec {
            secret_id: "moltbook_api_key".to_string(),
            providers: vec![
                ProviderConfig::Env {
                    enabled: true,
                    env: "MOLTBOOK_TOKEN".to_string(),
                    rotated_at_env: "MOLTBOOK_TOKEN_ROTATED_AT".to_string(),
                },
                ProviderConfig::JsonFile {
                    enabled: true,
                    paths: vec![
                        default_dir
                            .join("moltbook.credentials.json")
                            .to_string_lossy()
                            .into_owned(),
                        PathBuf::from(home.clone())
                            .join(".config")
                            .join("moltbook")
                            .join("credentials.json")
                            .to_string_lossy()
                            .into_owned(),
                        root.join("config")
                            .join("moltbook")
                            .join("credentials.json")
                            .to_string_lossy()
                            .into_owned(),
                    ],
                    field: "api_key".to_string(),
                    rotated_at_field: "rotated_at".to_string(),
                },
            ],
            rotation: RotationConfig {
                warn_after_days: 30.0,
                max_after_days: 60.0,
                require_rotated_at: false,
                enforce_on_issue: false,
            },
        },
    );
    secrets.insert(
        "moltstack_api_key".to_string(),
        SecretSpec {
            secret_id: "moltstack_api_key".to_string(),
            providers: vec![
                ProviderConfig::Env {
                    enabled: true,
                    env: "MOLTSTACK_TOKEN".to_string(),
                    rotated_at_env: "MOLTSTACK_TOKEN_ROTATED_AT".to_string(),
                },
                ProviderConfig::JsonFile {
                    enabled: true,
                    paths: vec![
                        default_dir
                            .join("moltstack.credentials.json")
                            .to_string_lossy()
                            .into_owned(),
                        PathBuf::from(std::env::var("HOME").unwrap_or_default())
                            .join(".config")
                            .join("moltstack")
                            .join("credentials.json")
                            .to_string_lossy()
                            .into_owned(),
                    ],
                    field: "api_key".to_string(),
                    rotated_at_field: "rotated_at".to_string(),
                },
            ],
            rotation: RotationConfig {
                warn_after_days: 30.0,
                max_after_days: 60.0,
                require_rotated_at: false,
                enforce_on_issue: false,
            },
        },
    );
    for (secret_id, provider, env_keys) in [
        (
            "web_search_tavily_api_key",
            "tavily",
            &["INFRING_TAVILY_API_KEY", "TAVILY_API_KEY"][..],
        ),
        (
            "web_search_exa_api_key",
            "exa",
            &["INFRING_EXA_API_KEY", "EXA_API_KEY"][..],
        ),
        (
            "web_search_brave_api_key",
            "brave",
            &[
                "INFRING_BRAVE_SEARCH_API_KEY",
                "BRAVE_SEARCH_API_KEY",
                "BRAVE_API_KEY",
            ][..],
        ),
        (
            "web_search_serperdev_api_key",
            "serperdev",
            &[
                "INFRING_SERPERDEV_API_KEY",
                "SERPERDEV_API_KEY",
                "INFRING_SERPER_API_KEY",
                "SERPER_API_KEY",
            ][..],
        ),
    ] {
        let mut providers = vec![ProviderConfig::EncryptedFile {
            enabled: true,
            paths: vec![root
                .join("client")
                .join("runtime")
                .join("local")
                .join("secrets")
                .join("vault")
                .join("web")
                .join("search")
                .join(format!("{provider}.secret.json"))
                .to_string_lossy()
                .into_owned()],
            field: "api_key".to_string(),
            rotated_at_field: "rotated_at".to_string(),
        }];
        providers.extend(env_keys.iter().map(|env| ProviderConfig::Env {
            enabled: true,
            env: (*env).to_string(),
            rotated_at_env: format!("{env}_ROTATED_AT"),
        }));
        secrets.insert(
            secret_id.to_string(),
            SecretSpec {
                secret_id: secret_id.to_string(),
                providers,
                rotation: RotationConfig {
                    warn_after_days: 30.0,
                    max_after_days: 60.0,
                    require_rotated_at: false,
                    enforce_on_issue: false,
                },
            },
        );
    }
    SecretBrokerPolicy {
        version: "1.0".to_string(),
        path: default_policy_path(root).to_string_lossy().into_owned(),
        include_backend_details: true,
        command_timeout_ms: 5000,
        secrets,
    }
}

fn normalize_provider(
    root: &Path,
    secret_id: &str,
    raw: &Value,
    command_timeout_ms: i64,
) -> Option<ProviderConfig> {
    let provider_type = text(raw.get("type"), 32).to_ascii_lowercase();
    match provider_type.as_str() {
        "env" => Some(ProviderConfig::Env {
            enabled: !matches!(raw.get("enabled"), Some(Value::Bool(false))),
            env: text(raw.get("env"), 120),
            rotated_at_env: text(raw.get("rotated_at_env"), 120),
        }),
        "json_file" => {
            let mut paths = raw
                .get("paths")
                .and_then(Value::as_array)
                .map(|rows| {
                    rows.iter()
                        .filter_map(Value::as_str)
                        .map(|row| resolve_template(root, row, secret_id))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            if paths.is_empty() {
                let path_text = text(raw.get("path"), 520);
                if !path_text.is_empty() {
                    paths.push(resolve_template(root, &path_text, secret_id));
                }
            }
            Some(ProviderConfig::JsonFile {
                enabled: !matches!(raw.get("enabled"), Some(Value::Bool(false))),
                paths,
                field: {
                    let v = text(raw.get("field"), 120);
                    if v.is_empty() {
                        "api_key".to_string()
                    } else {
                        v
                    }
                },
                rotated_at_field: {
                    let v = text(raw.get("rotated_at_field"), 120);
                    if v.is_empty() {
                        "rotated_at".to_string()
                    } else {
                        v
                    }
                },
            })
        }
        "encrypted_file" | "local_encrypted_file" => {
            let mut paths = raw
                .get("paths")
                .and_then(Value::as_array)
                .map(|rows| {
                    rows.iter()
                        .filter_map(Value::as_str)
                        .map(|row| resolve_template(root, row, secret_id))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            if paths.is_empty() {
                let path_text = text(raw.get("path"), 520);
                if !path_text.is_empty() {
                    paths.push(resolve_template(root, &path_text, secret_id));
                }
            }
            Some(ProviderConfig::EncryptedFile {
                enabled: !matches!(raw.get("enabled"), Some(Value::Bool(false))),
                paths,
                field: {
                    let v = text(raw.get("field"), 120);
                    if v.is_empty() {
                        "api_key".to_string()
                    } else {
                        v
                    }
                },
                rotated_at_field: {
                    let v = text(raw.get("rotated_at_field"), 120);
                    if v.is_empty() {
                        "rotated_at".to_string()
                    } else {
                        v
                    }
                },
            })
        }
        "command" => {
            let command = parse_command_spec(raw.get("command").unwrap_or(&Value::Null))?;
            let value_path = {
                let v = text(
                    raw.get("value_path").or_else(|| raw.get("value_field")),
                    160,
                );
                if v.is_empty() {
                    "value".to_string()
                } else {
                    v
                }
            };
            let rotated_at_path = {
                let v = text(
                    raw.get("rotated_at_path")
                        .or_else(|| raw.get("rotated_at_field")),
                    160,
                );
                if v.is_empty() {
                    "rotated_at".to_string()
                } else {
                    v
                }
            };
            let env = raw
                .get("env")
                .and_then(Value::as_object)
                .map(|map| {
                    map.iter()
                        .map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_string()))
                        .collect::<BTreeMap<_, _>>()
                })
                .unwrap_or_default();
            Some(ProviderConfig::Command {
                enabled: matches!(raw.get("enabled"), Some(Value::Bool(true))),
                command,
                parse_json: !matches!(raw.get("parse_json"), Some(Value::Bool(false))),
                value_path,
                rotated_at_path,
                timeout_ms: int_value(raw.get("timeout_ms"))
                    .unwrap_or(command_timeout_ms)
                    .clamp(500, 60_000),
                env,
            })
        }
        _ => None,
    }
}

fn normalize_secret_spec(
    root: &Path,
    secret_id: &str,
    raw: Option<&Value>,
    base: Option<&SecretSpec>,
    policy_rotation: &RotationConfig,
    command_timeout_ms: i64,
) -> SecretSpec {
    let raw_obj = raw.and_then(Value::as_object);
    let base_providers = base.map(|row| row.providers.clone()).unwrap_or_default();
    let providers = if let Some(raw_providers) = raw_obj
        .and_then(|obj| obj.get("providers"))
        .and_then(Value::as_array)
    {
        raw_providers
            .iter()
            .filter_map(|provider| {
                normalize_provider(root, secret_id, provider, command_timeout_ms)
            })
            .collect::<Vec<_>>()
    } else {
        base_providers
    };
    let base_rotation = base
        .map(|row| row.rotation.clone())
        .unwrap_or_else(|| policy_rotation.clone());
    let raw_rotation = raw_obj
        .and_then(|obj| obj.get("rotation"))
        .and_then(Value::as_object);
    let rotation = RotationConfig {
        warn_after_days: number_clamped(
            raw_rotation.and_then(|row| row.get("warn_after_days")),
            1.0,
            3650.0,
            base_rotation.warn_after_days,
        ),
        max_after_days: number_clamped(
            raw_rotation.and_then(|row| row.get("max_after_days")),
            1.0,
            3650.0,
            base_rotation
                .max_after_days
                .max(base_rotation.warn_after_days),
        )
        .max(number_clamped(
            raw_rotation.and_then(|row| row.get("warn_after_days")),
            1.0,
            3650.0,
            base_rotation.warn_after_days,
        )),
        require_rotated_at: bool_value(
            raw_rotation.and_then(|row| row.get("require_rotated_at")),
            base_rotation.require_rotated_at,
        ),
        enforce_on_issue: bool_value(
            raw_rotation.and_then(|row| row.get("enforce_on_issue")),
            base_rotation.enforce_on_issue,
        ),
    };
    SecretSpec {
        secret_id: secret_id.to_string(),
        providers,
        rotation,
    }
}

fn load_policy(root: &Path, payload: &Map<String, Value>) -> SecretBrokerPolicy {
    let policy_path = resolve_path(
        root,
        payload,
        "policy_path",
        "SECRET_BROKER_POLICY_PATH",
        default_policy_path(root),
    );
    let base = default_policy(root);
    let raw = lane_utils::read_json(&policy_path).unwrap_or_else(|| json!({}));
    let raw_obj = raw.as_object();
    let include_backend_details = raw_obj
        .and_then(|obj| obj.get("audit"))
        .and_then(Value::as_object)
        .map(|audit| {
            bool_value(
                audit.get("include_backend_details"),
                base.include_backend_details,
            )
        })
        .unwrap_or(base.include_backend_details);
    let command_timeout_ms = raw_obj
        .and_then(|obj| obj.get("command_backend"))
        .and_then(Value::as_object)
        .and_then(|command| int_value(command.get("timeout_ms")))
        .unwrap_or(base.command_timeout_ms)
        .clamp(500, 60_000);
    let base_rotation = RotationConfig {
        warn_after_days: raw_obj
            .and_then(|obj| obj.get("rotation_policy"))
            .and_then(Value::as_object)
            .map(|rotation| number_clamped(rotation.get("warn_after_days"), 1.0, 3650.0, 45.0))
            .unwrap_or(45.0),
        max_after_days: raw_obj
            .and_then(|obj| obj.get("rotation_policy"))
            .and_then(Value::as_object)
            .map(|rotation| number_clamped(rotation.get("max_after_days"), 1.0, 3650.0, 90.0))
            .unwrap_or(90.0),
        require_rotated_at: raw_obj
            .and_then(|obj| obj.get("rotation_policy"))
            .and_then(Value::as_object)
            .map(|rotation| bool_value(rotation.get("require_rotated_at"), false))
            .unwrap_or(false),
        enforce_on_issue: raw_obj
            .and_then(|obj| obj.get("rotation_policy"))
            .and_then(Value::as_object)
            .map(|rotation| bool_value(rotation.get("enforce_on_issue"), false))
            .unwrap_or(false),
    };
    let mut secrets = BTreeMap::new();
    let raw_secrets = raw_obj
        .and_then(|obj| obj.get("secrets"))
        .and_then(Value::as_object);
    let secret_ids = base
        .secrets
        .keys()
        .cloned()
        .chain(
            raw_secrets
                .map(|row| row.keys().cloned().collect::<Vec<_>>())
                .unwrap_or_default(),
        )
        .collect::<std::collections::BTreeSet<_>>();
    for secret_id in secret_ids {
        let spec = normalize_secret_spec(
            root,
            &secret_id,
            raw_secrets.and_then(|row| row.get(&secret_id)),
            base.secrets.get(&secret_id),
            &base_rotation,
            command_timeout_ms,
        );
        secrets.insert(secret_id.clone(), spec);
    }
    SecretBrokerPolicy {
        version: text(raw_obj.and_then(|obj| obj.get("version")), 32).or_else_if_empty("1.0"),
        path: policy_path.to_string_lossy().into_owned(),
        include_backend_details,
        command_timeout_ms,
        secrets,
    }
}

trait OrElseIfEmpty {
    fn or_else_if_empty(self, fallback: &str) -> String;
}

impl OrElseIfEmpty for String {
    fn or_else_if_empty(self, fallback: &str) -> String {
        if self.trim().is_empty() {
            fallback.to_string()
        } else {
            self
        }
    }
}
