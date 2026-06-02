fn fetch_with_curl_retry(
    url: &str,
    timeout_ms: u64,
    max_response_bytes: usize,
    max_attempts: usize,
    allow_rfc2544_benchmark_range: bool,
) -> Value {
    let mut attempts = 0usize;
    let mut best = json!({
        "ok": false,
        "status_code": 0,
        "content_type": "",
        "body": "",
        "stderr": "fetch_not_attempted"
    });
    let target_attempts = max_attempts.clamp(1, 4);
    for idx in 0..target_attempts {
        attempts += 1;
        let ua = DEFAULT_WEB_USER_AGENTS
            .get(idx % DEFAULT_WEB_USER_AGENTS.len())
            .copied()
            .unwrap_or(DEFAULT_WEB_USER_AGENTS[0]);
        let current = fetch_with_curl(
            url,
            timeout_ms,
            max_response_bytes,
            ua,
            allow_rfc2544_benchmark_range,
        );
        let current_ok = current.get("ok").and_then(Value::as_bool).unwrap_or(false);
        best = current;
        if current_ok {
            break;
        }
        if !is_retryable_fetch_result(&best) || idx + 1 >= target_attempts {
            break;
        }
        let sleep_ms = match idx {
            0 => 180_u64,
            1 => 360_u64,
            _ => 720_u64,
        };
        std::thread::sleep(std::time::Duration::from_millis(sleep_ms));
    }
    if let Some(obj) = best.as_object_mut() {
        obj.insert("retry_attempts".to_string(), json!(attempts));
        obj.insert("retry_used".to_string(), json!(attempts > 1));
    }
    best
}

fn fetch_serper_with_retry(
    api_key: &str,
    query: &str,
    timeout_ms: u64,
    max_response_bytes: usize,
    max_attempts: usize,
    top_k: usize,
) -> Value {
    let mut attempts = 0usize;
    let mut best = json!({
        "ok": false,
        "status_code": 0,
        "content_type": "",
        "body": "",
        "stderr": "serper_not_attempted"
    });
    let target_attempts = max_attempts.clamp(1, 4);
    for idx in 0..target_attempts {
        attempts += 1;
        let ua = DEFAULT_WEB_USER_AGENTS
            .get(idx % DEFAULT_WEB_USER_AGENTS.len())
            .copied()
            .unwrap_or(DEFAULT_WEB_USER_AGENTS[0]);
        let current =
            fetch_serper_with_curl(api_key, query, timeout_ms, max_response_bytes, ua, top_k);
        let current_ok = current.get("ok").and_then(Value::as_bool).unwrap_or(false);
        best = current;
        if current_ok {
            break;
        }
        if !is_retryable_fetch_result(&best) || idx + 1 >= target_attempts {
            break;
        }
        let sleep_ms = match idx {
            0 => 180_u64,
            1 => 360_u64,
            _ => 720_u64,
        };
        std::thread::sleep(std::time::Duration::from_millis(sleep_ms));
    }
    if let Some(obj) = best.as_object_mut() {
        obj.insert("retry_attempts".to_string(), json!(attempts));
        obj.insert("retry_used".to_string(), json!(attempts > 1));
    }
    best
}

fn fetch_json_post_once(
    url: &str,
    payload: &Value,
    headers: &[(&str, String)],
    timeout_ms: u64,
    max_response_bytes: usize,
    user_agent: &str,
    spawn_error_prefix: &str,
) -> Value {
    let timeout_sec = ((timeout_ms as f64) / 1000.0).ceil() as u64;
    let payload_raw = serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_string());
    let mut command = Command::new("curl");
    command
        .arg("-sS")
        .arg("-L")
        .arg("--compressed")
        .arg("--proto")
        .arg("=http,https")
        .arg("--connect-timeout")
        .arg(timeout_sec.max(1).to_string())
        .arg("--max-time")
        .arg(timeout_sec.max(1).to_string())
        .arg("-A")
        .arg(clean_text(user_agent, 260))
        .arg("-H")
        .arg("Accept: application/json")
        .arg("-H")
        .arg("Content-Type: application/json");
    for (name, value) in headers {
        command.arg("-H").arg(format!("{name}: {}", clean_text(value, 900)));
    }
    let output = command
        .arg("-d")
        .arg(payload_raw)
        .arg("-w")
        .arg("\n__STATUS__:%{http_code}\n__CTYPE__:%{content_type}")
        .arg(url)
        .output();
    match output {
        Ok(run) => {
            let stdout = String::from_utf8_lossy(&run.stdout).to_string();
            let stderr = clean_text(&String::from_utf8_lossy(&run.stderr), 320);
            let status_marker = "\n__STATUS__:";
            let ctype_marker = "\n__CTYPE__:";
            let (body_and_status, content_type) = match stdout.rsplit_once(ctype_marker) {
                Some((left, right)) => (left.to_string(), clean_text(right, 120)),
                None => (stdout, String::new()),
            };
            let (body_raw, status_raw) = match body_and_status.rsplit_once(status_marker) {
                Some((left, right)) => (left.to_string(), clean_text(right, 12)),
                None => (body_and_status, "0".to_string()),
            };
            let status_code = status_raw.parse::<i64>().unwrap_or(0);
            let body = clip_bytes(&body_raw, max_response_bytes.max(256));
            let status_ok = (200..300).contains(&status_code);
            json!({
                "ok": run.status.success() && status_ok,
                "status_code": status_code,
                "content_type": content_type,
                "body": body,
                "stderr": if stderr.is_empty() { Value::Null } else { Value::String(stderr) },
                "user_agent": clean_text(user_agent, 260)
            })
        }
        Err(err) => json!({
            "ok": false,
            "status_code": 0,
            "content_type": "",
            "body": "",
            "stderr": format!("{spawn_error_prefix}:{err}"),
            "user_agent": clean_text(user_agent, 260)
        }),
    }
}

fn fetch_json_post_with_retry(
    url: &str,
    payload: &Value,
    headers: &[(&str, String)],
    timeout_ms: u64,
    max_response_bytes: usize,
    max_attempts: usize,
    spawn_error_prefix: &str,
) -> Value {
    let mut attempts = 0usize;
    let mut best = json!({
        "ok": false,
        "status_code": 0,
        "content_type": "",
        "body": "",
        "stderr": "provider_not_attempted"
    });
    let target_attempts = max_attempts.clamp(1, 4);
    for idx in 0..target_attempts {
        attempts += 1;
        let ua = DEFAULT_WEB_USER_AGENTS
            .get(idx % DEFAULT_WEB_USER_AGENTS.len())
            .copied()
            .unwrap_or(DEFAULT_WEB_USER_AGENTS[0]);
        let current = fetch_json_post_once(
            url,
            payload,
            headers,
            timeout_ms,
            max_response_bytes,
            ua,
            spawn_error_prefix,
        );
        let current_ok = current.get("ok").and_then(Value::as_bool).unwrap_or(false);
        best = current;
        if current_ok {
            break;
        }
        if !is_retryable_fetch_result(&best) || idx + 1 >= target_attempts {
            break;
        }
        let sleep_ms = match idx {
            0 => 180_u64,
            1 => 360_u64,
            _ => 720_u64,
        };
        std::thread::sleep(std::time::Duration::from_millis(sleep_ms));
    }
    if let Some(obj) = best.as_object_mut() {
        obj.insert("retry_attempts".to_string(), json!(attempts));
        obj.insert("retry_used".to_string(), json!(attempts > 1));
    }
    best
}

fn fetch_brave_with_retry(
    api_key: &str,
    query: &str,
    timeout_ms: u64,
    max_response_bytes: usize,
    max_attempts: usize,
    top_k: usize,
    filters: &Value,
) -> Value {
    let requested_url = brave_search_url(query, top_k, filters);
    let mut attempts = 0usize;
    let mut best = json!({
        "ok": false,
        "status_code": 0,
        "content_type": "",
        "body": "",
        "stderr": "brave_not_attempted"
    });
    let target_attempts = max_attempts.clamp(1, 4);
    for idx in 0..target_attempts {
        attempts += 1;
        let ua = DEFAULT_WEB_USER_AGENTS
            .get(idx % DEFAULT_WEB_USER_AGENTS.len())
            .copied()
            .unwrap_or(DEFAULT_WEB_USER_AGENTS[0]);
        let timeout_sec = ((timeout_ms as f64) / 1000.0).ceil() as u64;
        let output = Command::new("curl")
            .arg("-sS")
            .arg("-L")
            .arg("--compressed")
            .arg("--proto")
            .arg("=http,https")
            .arg("--connect-timeout")
            .arg(timeout_sec.max(1).to_string())
            .arg("--max-time")
            .arg(timeout_sec.max(1).to_string())
            .arg("-A")
            .arg(clean_text(ua, 260))
            .arg("-H")
            .arg("Accept: application/json")
            .arg("-H")
            .arg(format!("X-Subscription-Token: {}", clean_text(api_key, 900)))
            .arg("-w")
            .arg("\n__STATUS__:%{http_code}\n__CTYPE__:%{content_type}")
            .arg(&requested_url)
            .output();
        let current = match output {
            Ok(run) => {
                let stdout = String::from_utf8_lossy(&run.stdout).to_string();
                let stderr = clean_text(&String::from_utf8_lossy(&run.stderr), 320);
                let status_marker = "\n__STATUS__:";
                let ctype_marker = "\n__CTYPE__:";
                let (body_and_status, content_type) = match stdout.rsplit_once(ctype_marker) {
                    Some((left, right)) => (left.to_string(), clean_text(right, 120)),
                    None => (stdout, String::new()),
                };
                let (body_raw, status_raw) = match body_and_status.rsplit_once(status_marker) {
                    Some((left, right)) => (left.to_string(), clean_text(right, 12)),
                    None => (body_and_status, "0".to_string()),
                };
                let status_code = status_raw.parse::<i64>().unwrap_or(0);
                let status_ok = (200..300).contains(&status_code);
                json!({
                    "ok": run.status.success() && status_ok,
                    "status_code": status_code,
                    "content_type": content_type,
                    "body": clip_bytes(&body_raw, max_response_bytes.max(256)),
                    "stderr": if stderr.is_empty() { Value::Null } else { Value::String(stderr) },
                    "user_agent": clean_text(ua, 260),
                    "requested_url": requested_url
                })
            }
            Err(err) => json!({
                "ok": false,
                "status_code": 0,
                "content_type": "",
                "body": "",
                "stderr": format!("brave_curl_spawn_failed:{err}"),
                "user_agent": clean_text(ua, 260),
                "requested_url": requested_url
            }),
        };
        let current_ok = current.get("ok").and_then(Value::as_bool).unwrap_or(false);
        best = current;
        if current_ok {
            break;
        }
        if !is_retryable_fetch_result(&best) || idx + 1 >= target_attempts {
            break;
        }
        let sleep_ms = match idx {
            0 => 180_u64,
            1 => 360_u64,
            _ => 720_u64,
        };
        std::thread::sleep(std::time::Duration::from_millis(sleep_ms));
    }
    if let Some(obj) = best.as_object_mut() {
        obj.insert("retry_attempts".to_string(), json!(attempts));
        obj.insert("retry_used".to_string(), json!(attempts > 1));
    }
    best
}

fn build_receipt(
    requested_url: &str,
    policy_decision: &str,
    response_hash: Option<&str>,
    status_code: i64,
    policy_reason: &str,
    error: Option<&str>,
) -> Value {
    let timestamp = crate::now_iso();
    let mut row = json!({
        "type": "web_conduit_receipt",
        "timestamp": timestamp,
        "requested_url": clean_text(requested_url, 2200),
        "domain": extract_domain(requested_url),
        "policy_decision": clean_text(policy_decision, 40),
        "policy_reason": clean_text(policy_reason, 160),
        "status_code": status_code,
        "response_hash": response_hash.unwrap_or(""),
        "error": clean_text(error.unwrap_or(""), 320)
    });
    row["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&row));
    row
}

pub fn api_status(root: &Path) -> Value {
    let (policy, policy_path_value) = load_policy(root);
    let recent = read_recent_receipts(root, 12);
    let denied = recent
        .iter()
        .filter(|row| row.get("policy_decision").and_then(Value::as_str) == Some("deny"))
        .count();
    let last = recent.first().cloned().unwrap_or(Value::Null);
    let default_search_provider_chain =
        crate::web_conduit_provider_runtime::resolved_search_provider_chain(
            "",
            &json!({}),
            &policy,
        );
    let default_fetch_provider_chain = fetch_provider_chain_from_request("", &json!({}), &policy);
    let search_provider_catalog = provider_catalog_snapshot(root, &policy);
    let fetch_provider_catalog = fetch_provider_catalog_snapshot(root, &policy);
    let search_request_contract = search_provider_request_contract(&policy);
    let mut tool_catalog = web_tool_catalog_snapshot(&policy);
    append_web_media_tool_entry(&mut tool_catalog, &policy);
    append_web_image_tool_entry(&mut tool_catalog, root, &policy);
    let search_provider_registration_contract = search_provider_registration_contract(&policy);
    let fetch_provider_registration_contract = fetch_provider_registration_contract(&policy);
    let public_artifact_contracts = web_provider_public_artifact_contracts();
    let runtime_web_tools_metadata = runtime_web_tools_snapshot(root, &policy);
    let image_tool_runtime = runtime_web_tools_metadata
        .get("image_tool")
        .cloned()
        .unwrap_or(Value::Null);
    let native_codex_web_search = native_codex_public_contract(root, &policy);
    let media_generation_action_contracts = media_generate_action_contracts();
    let tool_effective_inventory =
        runtime_web_tooling_effective_inventory(&runtime_web_tools_metadata, &tool_catalog);
    let tool_policy_pipeline =
        runtime_web_tooling_policy_pipeline(&runtime_web_tools_metadata, &tool_effective_inventory);
    let tooling_process_summary = runtime_web_tooling_process_summary(
        &runtime_web_tools_metadata,
        &tool_effective_inventory,
        &tool_policy_pipeline,
    );
    let tooling_decision_trace = runtime_web_tooling_decision_trace(
        &runtime_web_tools_metadata,
        &tool_effective_inventory,
        &tool_policy_pipeline,
    );
    json!({
        "ok": true,
        "enabled": policy.pointer("/web_conduit/enabled").and_then(Value::as_bool).unwrap_or(true),
        "policy_path": policy_path_value.to_string_lossy().to_string(),
        "policy": policy,
        "default_provider_chain": default_search_provider_chain.clone(),
        "default_search_provider_chain": default_search_provider_chain,
        "default_fetch_provider_chain": default_fetch_provider_chain,
        "provider_catalog": search_provider_catalog.clone(),
        "search_provider_catalog": search_provider_catalog,
        "fetch_provider_catalog": fetch_provider_catalog,
        "search_request_contract": search_request_contract,
        "search_provider_registration_contract": search_provider_registration_contract,
        "fetch_provider_registration_contract": fetch_provider_registration_contract,
        "media_request_contract": web_media_request_contract(),
        "image_tool_contract": crate::web_conduit_provider_runtime::web_image_tool_contract(root, &policy),
        "image_tool_runtime": image_tool_runtime,
        "public_artifact_contracts": public_artifact_contracts,
        "runtime_web_tools_state_path": runtime_web_tools_state_path(root).display().to_string(),
        "runtime_web_tools_metadata": runtime_web_tools_metadata,
        "tool_effective_inventory": tool_effective_inventory,
        "tool_policy_pipeline": tool_policy_pipeline,
        "tooling_process_summary": tooling_process_summary,
        "tooling_decision_trace": tooling_decision_trace,
        "native_codex_web_search": native_codex_web_search,
        "media_generation_action_contracts": media_generation_action_contracts,
        "tool_catalog": tool_catalog,
        "receipts_total": receipt_count(root),
        "recent_denied": denied,
        "recent_receipts": recent,
        "last_receipt": last
    })
}

pub fn api_providers(root: &Path) -> Value {
    let (policy, policy_path_value) = load_policy(root);
    let default_search_provider_chain =
        crate::web_conduit_provider_runtime::resolved_search_provider_chain(
            "",
            &json!({}),
            &policy,
        );
    let default_fetch_provider_chain = fetch_provider_chain_from_request("", &json!({}), &policy);
    let search_providers = provider_catalog_snapshot(root, &policy);
    let fetch_providers = fetch_provider_catalog_snapshot(root, &policy);
    let mut tool_catalog = web_tool_catalog_snapshot(&policy);
    append_web_media_tool_entry(&mut tool_catalog, &policy);
    append_web_image_tool_entry(&mut tool_catalog, root, &policy);
    let search_provider_registration_contract = search_provider_registration_contract(&policy);
    let fetch_provider_registration_contract = fetch_provider_registration_contract(&policy);
    let public_artifact_contracts = web_provider_public_artifact_contracts();
    let runtime_web_tools_metadata = runtime_web_tools_snapshot(root, &policy);
    let image_tool_runtime = runtime_web_tools_metadata
        .get("image_tool")
        .cloned()
        .unwrap_or(Value::Null);
    let native_codex_web_search = native_codex_public_contract(root, &policy);
    let media_generation_action_contracts = media_generate_action_contracts();
    let tool_effective_inventory =
        runtime_web_tooling_effective_inventory(&runtime_web_tools_metadata, &tool_catalog);
    let tool_policy_pipeline =
        runtime_web_tooling_policy_pipeline(&runtime_web_tools_metadata, &tool_effective_inventory);
    let tooling_process_summary = runtime_web_tooling_process_summary(
        &runtime_web_tools_metadata,
        &tool_effective_inventory,
        &tool_policy_pipeline,
    );
    let tooling_decision_trace = runtime_web_tooling_decision_trace(
        &runtime_web_tools_metadata,
        &tool_effective_inventory,
        &tool_policy_pipeline,
    );
    json!({
        "ok": true,
        "type": "web_conduit_providers",
        "policy_path": policy_path_value.to_string_lossy().to_string(),
        "default_provider_chain": default_search_provider_chain.clone(),
        "default_search_provider_chain": default_search_provider_chain,
        "default_fetch_provider_chain": default_fetch_provider_chain,
        "search_request_contract": search_provider_request_contract(&policy),
        "search_provider_registration_contract": search_provider_registration_contract,
        "fetch_provider_registration_contract": fetch_provider_registration_contract,
        "media_request_contract": web_media_request_contract(),
        "image_tool_contract": crate::web_conduit_provider_runtime::web_image_tool_contract(root, &policy),
        "image_tool_runtime": image_tool_runtime,
        "public_artifact_contracts": public_artifact_contracts,
        "runtime_web_tools_state_path": runtime_web_tools_state_path(root).display().to_string(),
        "runtime_web_tools_metadata": runtime_web_tools_metadata,
        "tool_effective_inventory": tool_effective_inventory,
        "tool_policy_pipeline": tool_policy_pipeline,
        "tooling_process_summary": tooling_process_summary,
        "tooling_decision_trace": tooling_decision_trace,
        "native_codex_web_search": native_codex_web_search,
        "media_generation_action_contracts": media_generation_action_contracts,
        "tool_catalog": tool_catalog,
        "providers": search_providers.clone(),
        "search_providers": search_providers,
        "fetch_providers": fetch_providers
    })
}

pub fn api_receipts(root: &Path, limit: usize) -> Value {
    json!({
        "ok": true,
        "receipts": read_recent_receipts(root, limit.clamp(1, 200))
    })
}

pub fn api_fetch(root: &Path, request: &Value) -> Value {
    execute_fetch_request(root, request)
}
