pub fn test_provider(root: &Path, provider_id: &str) -> Value {
    let provider = normalize_provider_id(provider_id);
    let started = Instant::now();
    if provider == "claude-code" {
        let ok = Command::new("sh")
            .arg("-lc")
            .arg("command -v claude >/dev/null 2>&1")
            .status()
            .map(|status| status.success())
            .unwrap_or(false);
        return if ok {
            json!({"ok": true, "status": "ok", "provider": provider, "latency_ms": started.elapsed().as_millis() as i64})
        } else {
            json!({"ok": false, "status": "error", "provider": provider, "error": "claude_code_cli_not_detected"})
        };
    }

    if provider == "auto" {
        let providers = provider_rows(root, &json!({}));
        let ready = providers.into_iter().any(|row| {
            row.get("is_local")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                || auth_status_configured(
                    row.get("auth_status").and_then(Value::as_str).unwrap_or(""),
                )
        });
        return if ready {
            json!({"ok": true, "status": "ok", "provider": provider, "latency_ms": started.elapsed().as_millis() as i64})
        } else {
            json!({"ok": false, "status": "error", "provider": provider, "error": "no_configured_provider"})
        };
    }

    if provider == "ollama" {
        let row = provider_row(root, &provider);
        let base_url = clean_text(
            row.get("base_url")
                .and_then(Value::as_str)
                .unwrap_or(&provider_base_url_default(&provider)),
            400,
        );
        let resolved_base = resolve_ollama_runtime_base_url(&base_url);
        let mut online = probe_ollama_runtime_online(&resolved_base);
        let discovered_models = probe_ollama_runtime_models(&resolved_base);
        if !online && !discovered_models.is_empty() {
            online = true;
        }
        let mut registry = load_registry(root);
        let row = ensure_provider_row_mut(&mut registry, &provider);
        row["base_url"] = json!(resolved_base.clone());
        row["reachable"] = json!(online);
        if !discovered_models.is_empty() {
            if !row.get("model_profiles").map(Value::is_object).unwrap_or(false) {
                row["model_profiles"] = json!({});
            }
            if let Some(profiles) = row.get_mut("model_profiles").and_then(Value::as_object_mut) {
                profiles.retain(|model_name, _| {
                    let cleaned = clean_text(model_name, 240);
                    !cleaned.is_empty() && !model_id_is_placeholder(&cleaned)
                });
                for model in &discovered_models {
                    if profiles.contains_key(model) {
                        continue;
                    }
                    profiles.insert(model.clone(), inferred_model_profile("ollama", model, true));
                }
            }
            row["detected_models"] = Value::Array(
                discovered_models
                    .iter()
                    .cloned()
                    .map(Value::String)
                    .collect::<Vec<_>>(),
            );
        }
        row["updated_at"] = json!(crate::now_iso());
        save_registry(root, registry);
        return if online {
            json!({
                "ok": true,
                "status": "ok",
                "provider": provider,
                "latency_ms": started.elapsed().as_millis() as i64,
                "detail": {
                    "base_url": resolved_base,
                    "discovered_models": discovered_models,
                }
            })
        } else {
            json!({
                "ok": false,
                "status": "error",
                "provider": provider,
                "error": "ollama_runtime_unreachable",
                "detail": {"base_url": resolved_base}
            })
        };
    }

    let row = provider_row(root, &provider);
    let base_url = clean_text(
        row.get("base_url")
            .and_then(Value::as_str)
            .unwrap_or(&provider_base_url_default(&provider)),
        400,
    );
    let mut headers = vec!["Content-Type: application/json".to_string()];
    let url = match provider.as_str() {
        "ollama" => format!("{base_url}/api/tags"),
        "google" => {
            let Some(key) = provider_key(root, &provider) else {
                return json!({"ok": false, "status": "error", "provider": provider, "error": "provider_key_missing"});
            };
            format!("{base_url}/v1beta/models?key={key}")
        }
        "frontier_provider" => {
            let Some(key) = provider_key(root, &provider) else {
                return json!({"ok": false, "status": "error", "provider": provider, "error": "provider_key_missing"});
            };
            headers.push(format!("x-api-key: {key}"));
            headers.push("anthropic-version: 2023-06-01".to_string());
            format!("{base_url}/v1/models")
        }
        _ => {
            let Some(key) = provider_key(root, &provider) else {
                return json!({"ok": false, "status": "error", "provider": provider, "error": "provider_key_missing"});
            };
            headers.push(format!("Authorization: Bearer {key}"));
            format!("{base_url}/models")
        }
    };

    match curl_json(&url, "GET", &headers, None, 20) {
        Ok((status, value)) if status >= 200 && status < 300 => {
            let mut registry = load_registry(root);
            let row = ensure_provider_row_mut(&mut registry, &provider);
            row["reachable"] = json!(true);
            let discovered_models = models_from_probe_response(&provider, &value);
            if !discovered_models.is_empty() {
                if !row.get("model_profiles").map(Value::is_object).unwrap_or(false) {
                    row["model_profiles"] = json!({});
                }
                if let Some(profiles) = row.get_mut("model_profiles").and_then(Value::as_object_mut) {
                    profiles.retain(|model_name, _| {
                        let cleaned = clean_text(model_name, 240);
                        !cleaned.is_empty() && !model_id_is_placeholder(&cleaned)
                    });
                    for model in &discovered_models {
                        if profiles.contains_key(model) {
                            continue;
                        }
                        profiles.insert(
                            model.clone(),
                            inferred_model_profile(&provider, model, provider_is_local(&provider)),
                        );
                    }
                }
                row["detected_models"] = Value::Array(
                    discovered_models
                        .iter()
                        .cloned()
                        .map(Value::String)
                        .collect::<Vec<_>>(),
                );
            }
            row["updated_at"] = json!(crate::now_iso());
            save_registry(root, registry);
            json!({
                "ok": true,
                "status": "ok",
                "provider": provider,
                "latency_ms": started.elapsed().as_millis() as i64,
                "detail": value
            })
        }
        Ok((status, value)) => {
            let mut registry = load_registry(root);
            let row = ensure_provider_row_mut(&mut registry, &provider);
            row["reachable"] = json!(false);
            row["updated_at"] = json!(crate::now_iso());
            save_registry(root, registry);
            json!({
                "ok": false,
                "status": "error",
                "provider": provider,
                "error": format!("http_{status}:{}", error_text_from_value(&value))
            })
        }
        Err(err) => {
            let mut registry = load_registry(root);
            let row = ensure_provider_row_mut(&mut registry, &provider);
            row["reachable"] = json!(false);
            row["updated_at"] = json!(crate::now_iso());
            save_registry(root, registry);
            json!({"ok": false, "status": "error", "provider": provider, "error": clean_text(&err, 280)})
        }
    }
}

pub fn complete_provider(root: &Path, provider_id: &str, request: &Value) -> Value {
    let provider = normalize_provider_id(provider_id);
    let started = Instant::now();
    let prompt = clean_text(
        request.get("prompt").and_then(Value::as_str).unwrap_or(""),
        65536,
    );
    if prompt.trim().is_empty() {
        return json!({
            "ok": false,
            "status": "error",
            "provider": provider,
            "error": "prompt_required",
            "latency_ms": started.elapsed().as_millis() as i64
        });
    }
    if provider != "claude-code" {
        return json!({
            "ok": false,
            "status": "error",
            "provider": provider,
            "error": "provider_completion_unsupported",
            "latency_ms": started.elapsed().as_millis() as i64
        });
    }

    let _ = root;
    if !command_exists("claude") {
        return json!({
            "ok": false,
            "status": "error",
            "provider": provider,
            "error": "claude_code_cli_not_detected",
            "latency_ms": started.elapsed().as_millis() as i64
        });
    }

    let system = clean_text(
        request.get("system").and_then(Value::as_str).unwrap_or(""),
        16384,
    );
    let model = clean_text(
        request
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or("claude-code/sonnet"),
        240,
    );
    let full_prompt = if system.trim().is_empty() {
        prompt
    } else {
        format!("{system}\n\n{prompt}")
    };
    let timeout_seconds = request
        .get("timeout_seconds")
        .and_then(Value::as_u64)
        .or_else(|| {
            request
                .pointer("/metadata/provider_timeout_seconds")
                .and_then(Value::as_u64)
        })
        .or_else(|| {
            std::env::var("INFRING_PROVIDER_TIMEOUT_SECONDS")
                .ok()
                .and_then(|value| value.parse::<u64>().ok())
        })
        .unwrap_or(120)
        .clamp(1, 3600);
    let binary = std::env::var("INFRING_CLAUDE_CODE_BIN")
        .unwrap_or_else(|_| "claude".to_string());
    let mut child = match Command::new(&binary)
        .arg("-p")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            return json!({
                "ok": false,
                "status": "error",
                "provider": provider,
                "model": model,
                "error": clean_text(&format!("claude_code_spawn_failed:{error}"), 280),
                "latency_ms": started.elapsed().as_millis() as i64
            });
        }
    };
    if let Some(mut stdin) = child.stdin.take() {
        if let Err(error) = stdin.write_all(full_prompt.as_bytes()) {
            let _ = child.kill();
            return json!({
                "ok": false,
                "status": "error",
                "provider": provider,
                "model": model,
                "error": clean_text(&format!("claude_code_stdin_write_failed:{error}"), 280),
                "latency_ms": started.elapsed().as_millis() as i64
            });
        }
    }
    let output = wait_for_dashboard_provider_process(
        child,
        std::time::Duration::from_secs(timeout_seconds),
    );
    match output {
        Ok(output) => {
            let stdout = clean_text(&String::from_utf8_lossy(&output.stdout), 65536);
            let stderr = clean_text(&String::from_utf8_lossy(&output.stderr), 2048);
            if output.status.success() {
                json!({
                    "ok": true,
                    "status": "ok",
                    "provider": provider,
                    "model": model,
                    "output": stdout.trim(),
                    "usage_tokens": stdout.split_whitespace().count() as i64,
                    "latency_ms": started.elapsed().as_millis() as i64
                })
            } else {
                json!({
                    "ok": false,
                    "status": "error",
                    "provider": provider,
                    "model": model,
                    "error": clean_text(
                        &format!(
                            "claude_code_completion_failed:status={}:stderr={}",
                            output.status.code().unwrap_or(-1),
                            stderr
                        ),
                        280
                    ),
                    "latency_ms": started.elapsed().as_millis() as i64
                })
            }
        }
        Err(error) => json!({
            "ok": false,
            "status": "error",
            "provider": provider,
            "model": model,
            "error": error,
            "latency_ms": started.elapsed().as_millis() as i64
        }),
    }
}

fn wait_for_dashboard_provider_process(
    child: std::process::Child,
    timeout: std::time::Duration,
) -> Result<std::process::Output, String> {
    let pid = child.id();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(child.wait_with_output());
    });
    match rx.recv_timeout(timeout) {
        Ok(result) => result.map_err(|error| clean_text(&format!("provider_wait_failed:{error}"), 280)),
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
            let _ = Command::new("kill")
                .arg("-TERM")
                .arg(pid.to_string())
                .status();
            if rx.recv_timeout(std::time::Duration::from_secs(5)).is_err() {
                let _ = Command::new("kill")
                    .arg("-KILL")
                    .arg(pid.to_string())
                    .status();
            }
            Err(format!("provider_completion_timeout:timeout_seconds={}", timeout.as_secs()))
        }
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
            Err("provider_wait_channel_disconnected".to_string())
        }
    }
}

#[cfg(test)]
mod provider_http_interop_tests {
    use super::*;
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn extract_openai_text_preserves_multiline_list_layout() {
        let payload = json!({
            "choices": [{
                "message": {
                    "content": "1. First item\n2. Second item\n   - nested detail"
                }
            }]
        });
        let text = extract_openai_text(&payload);
        assert!(text.contains("1. First item\n2. Second item"));
        assert!(text.contains("\n   - nested detail"));
    }

    #[test]
    fn parse_ollama_list_models_reads_name_column() {
        let raw = "\
NAME                             ID              SIZE      MODIFIED
qwen3:8b                         500a1f067a9f    5.2 GB    7 weeks ago
kimi-k2.5:cloud                  6d1c3246c608    -         7 weeks ago
";
        let rows = parse_ollama_list_models(raw);
        assert_eq!(
            rows,
            vec!["qwen3:8b".to_string(), "kimi-k2.5:cloud".to_string()]
        );
    }

    #[test]
    fn parse_ollama_list_models_json_reads_array_rows() {
        let raw = r#"
[
  {"name":"qwen3:8b","model":"qwen3:8b"},
  {"name":"smallthinker:latest","model":"smallthinker:latest"}
]
"#;
        let rows = parse_ollama_list_models_json(raw);
        assert_eq!(
            rows,
            vec!["qwen3:8b".to_string(), "smallthinker:latest".to_string()]
        );
    }

    #[test]
    fn ollama_base_url_candidates_include_default_loopback() {
        let rows = ollama_base_url_candidates("127.0.0.1:11434");
        assert!(rows.iter().any(|row| row == "http://127.0.0.1:11434"));
        assert!(rows.iter().any(|row| row == "http://localhost:11434"));
    }

    #[test]
    fn provider_rows_marks_ollama_reachable_when_cli_lists_models() {
        let root = tempfile::tempdir().expect("tempdir");
        let bin_dir = tempfile::tempdir().expect("tempdir");
        let ollama_path = bin_dir.path().join("ollama");
        let script = r#"#!/bin/sh
if [ "$1" = "list" ] && [ "$2" = "--json" ]; then
  printf '[{"name":"qwen3:4b","model":"qwen3:4b"},{"name":"smallthinker:latest","model":"smallthinker:latest"}]\n'
  exit 0
fi
if [ "$1" = "list" ]; then
  printf 'NAME ID SIZE MODIFIED\nqwen3:4b deadbeef 3.2GB now\n'
  exit 0
fi
exit 1
"#;
        fs::write(&ollama_path, script).expect("write ollama stub");
        #[cfg(unix)]
        {
            fs::set_permissions(&ollama_path, fs::Permissions::from_mode(0o755))
                .expect("chmod ollama stub");
        }
        let old_path = std::env::var("PATH").unwrap_or_default();
        let new_path = format!("{}:{}", bin_dir.path().display(), old_path);
        std::env::set_var("PATH", new_path);

        let rows = provider_rows(root.path(), &json!({}));
        let ollama = rows
            .iter()
            .find(|row| row.get("id").and_then(Value::as_str) == Some("ollama"))
            .cloned()
            .unwrap_or_else(|| json!({}));

        std::env::set_var("PATH", old_path);

        assert_eq!(ollama.get("reachable").and_then(Value::as_bool), Some(true));
        assert!(ollama
            .get("detected_models")
            .and_then(Value::as_array)
            .map(|models| {
                models
                    .iter()
                    .any(|row| row.as_str() == Some("qwen3:4b"))
            })
            .unwrap_or(false));
    }
}
