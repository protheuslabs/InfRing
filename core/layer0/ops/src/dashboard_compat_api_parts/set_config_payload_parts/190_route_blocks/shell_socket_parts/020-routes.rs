fn handle_shell_socket_routes(
    root: &Path,
    method: &str,
    path: &str,
    path_only: &str,
    body: &[u8],
    headers: &[(&str, &str)],
    snapshot: &Value,
    requester_agent: &str,
    request_host: &str,
) -> Option<CompatApiResponse> {
    if !path_only.starts_with(SHELL_SOCKET_PREFIX) {
        return None;
    }
    let parts = shell_socket_path_parts(path_only);
    if method == "GET" && parts == ["runtime-status"] {
        return Some(CompatApiResponse { status: 200, payload: shell_socket_runtime_status(root, snapshot, request_host) });
    }
    if method == "GET" && parts == ["agents"] {
        return Some(CompatApiResponse { status: 200, payload: shell_socket_agent_roster(root, path, snapshot) });
    }
    if method == "GET" && parts.len() == 3 && parts[0] == "agents" && parts[2] == "sessions" {
        return Some(CompatApiResponse { status: 200, payload: shell_socket_session_list(root, &parts[1], path) });
    }
    if method == "GET" && parts.len() == 3 && parts[0] == "sessions" && parts[2] == "messages" {
        return Some(CompatApiResponse { status: 200, payload: shell_socket_message_window(root, &parts[1], path) });
    }
    if method == "GET" && parts.len() >= 2 && parts[0] == "details" {
        let detail_ref = parts[1..].join("/");
        return Some(match shell_socket_detail_projection(root, &detail_ref, path) {
            Some(payload) => CompatApiResponse { status: 200, payload },
            None => CompatApiResponse {
                status: 404,
                payload: json!({
                    "detail_id": clean_text(&detail_ref, 180),
                    "detail_kind": "unknown",
                    "requested_view": query_value(path, "view").unwrap_or_else(|| "summary".to_string()),
                    "detail_projection": json!({}),
                    "size_bound": json!({"max_response_bytes": 65536}),
                    "next_cursor": Value::Null,
                    "receipt_ref": shell_socket_receipt_ref("get_message_detail", &json!({"detail_ref": detail_ref})),
                    "correlation_id": "shell_socket.message_detail"
                }),
            },
        });
    }
    if method == "GET" && parts.len() == 3 && parts[0] == "sessions" && parts[2] == "events" {
        return Some(CompatApiResponse { status: 200, payload: shell_socket_event_projection(root, &parts[1]) });
    }
    if method == "GET" && parts == ["search"] {
        return Some(CompatApiResponse { status: 200, payload: shell_socket_search(root, path) });
    }
    if method == "GET" && parts == ["models"] {
        let mut payload = crate::dashboard_model_catalog::catalog_payload(root, snapshot);
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("next_cursor".to_string(), Value::Null);
            obj.insert("detail_refs".to_string(), json!({"models": "model_catalog:default"}));
            obj.insert(
                "receipt_ref".to_string(),
                json!(shell_socket_receipt_ref("list_models", &json!({"path": path}))),
            );
            obj.insert("correlation_id".to_string(), json!("shell_socket.model_catalog"));
        }
        return Some(CompatApiResponse { status: 200, payload });
    }
    if method == "GET" && parts == ["providers"] {
        let mut payload = crate::dashboard_provider_runtime::providers_payload(root, snapshot);
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("next_cursor".to_string(), Value::Null);
            obj.insert("detail_refs".to_string(), json!({"providers": "provider_catalog:default"}));
            obj.insert(
                "receipt_ref".to_string(),
                json!(shell_socket_receipt_ref("list_providers", &json!({"path": path}))),
            );
            obj.insert("correlation_id".to_string(), json!("shell_socket.provider_catalog"));
        }
        return Some(CompatApiResponse { status: 200, payload });
    }
    if method == "POST" && parts == ["models", "discover"] {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let input = clean_text(
            request
                .get("input")
                .and_then(Value::as_str)
                .or_else(|| request.get("api_key").and_then(Value::as_str))
                .unwrap_or(""),
            4096,
        );
        let mut payload = crate::dashboard_provider_runtime::discover_models(root, &input);
        let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(false);
        if let Some(obj) = payload.as_object_mut() {
            let input_kind = obj.get("input_kind").cloned().unwrap_or(Value::Null);
            obj.insert(
                "receipt_ref".to_string(),
                json!(shell_socket_receipt_ref("discover_models", &json!({"input_kind": input_kind}))),
            );
            obj.insert("correlation_id".to_string(), json!("shell_socket.model_discovery"));
        }
        return Some(CompatApiResponse {
            status: if ok { 200 } else { 400 },
            payload,
        });
    }
    if method == "POST" && parts == ["models", "download"] {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let provider = clean_text(
            request
                .get("provider")
                .and_then(Value::as_str)
                .unwrap_or(""),
            80,
        );
        let model = clean_text(
            request.get("model").and_then(Value::as_str).unwrap_or(""),
            240,
        );
        let mut payload = crate::dashboard_provider_runtime::download_model(root, &provider, &model);
        let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(false);
        if let Some(obj) = payload.as_object_mut() {
            let receipt_seed = json!({"provider": provider.clone(), "model": model.clone()});
            obj.insert("provider".to_string(), json!(provider));
            obj.insert("model".to_string(), json!(model));
            obj.insert(
                "receipt_ref".to_string(),
                json!(shell_socket_receipt_ref("download_model", &receipt_seed)),
            );
            obj.insert("correlation_id".to_string(), json!("shell_socket.model_download"));
        }
        return Some(CompatApiResponse {
            status: if ok { 200 } else { 400 },
            payload,
        });
    }
    if method == "POST" && parts == ["models", "custom"] {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let provider = clean_text(
            request
                .get("provider")
                .and_then(Value::as_str)
                .unwrap_or("openrouter"),
            80,
        );
        let model = clean_text(
            request
                .get("id")
                .or_else(|| request.get("model"))
                .or_else(|| request.get("model_ref"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            240,
        );
        let context_window = request
            .get("context_window")
            .and_then(Value::as_i64)
            .unwrap_or(128_000);
        let max_output_tokens = request
            .get("max_output_tokens")
            .and_then(Value::as_i64)
            .unwrap_or(8192);
        let authority = crate::dashboard_provider_runtime::add_custom_model(
            root,
            &provider,
            &model,
            context_window,
            max_output_tokens,
        );
        let accepted = authority.get("ok").and_then(Value::as_bool).unwrap_or(false);
        let reason_code = if accepted {
            "accepted"
        } else {
            authority
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("custom_model_rejected")
        };
        return Some(CompatApiResponse {
            status: if accepted { 202 } else { 400 },
            payload: shell_socket_ingress_ack(
                "upsert_custom_model",
                accepted,
                reason_code,
                &json!({"provider": provider, "model_ref": model, "authority": authority}),
            ),
        });
    }
    if method == "POST" && parts == ["models", "custom", "delete"] {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let model_ref = clean_text(
            request
                .get("model_ref")
                .or_else(|| request.get("id"))
                .or_else(|| request.get("model"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            240,
        );
        let authority = crate::dashboard_provider_runtime::delete_custom_model(root, &model_ref);
        let accepted = authority.get("ok").and_then(Value::as_bool).unwrap_or(false);
        let reason_code = if accepted {
            "accepted"
        } else {
            authority
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("custom_model_delete_rejected")
        };
        return Some(CompatApiResponse {
            status: if accepted { 202 } else { 400 },
            payload: shell_socket_ingress_ack(
                "delete_custom_model",
                accepted,
                reason_code,
                &json!({"model_ref": model_ref, "authority": authority}),
            ),
        });
    }
    if method == "POST" && parts.len() == 3 && parts[0] == "providers" && parts[2] == "key" {
        let provider_id = normalize_provider_route_id(&parts[1]);
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let key = clean_text(request.get("key").and_then(Value::as_str).unwrap_or(""), 4096);
        let authority = crate::dashboard_provider_runtime::save_provider_key(root, &provider_id, &key);
        let ok = authority.get("ok").and_then(Value::as_bool).unwrap_or(false);
        return Some(CompatApiResponse {
            status: if ok { 200 } else { 400 },
            payload: json!({
                "ok": ok,
                "provider": authority.get("provider").cloned().unwrap_or_else(|| json!(provider_id)),
                "auth_status": authority.get("auth_status").cloned().unwrap_or(Value::Null),
                "switched_default": authority.get("switched_default").cloned().unwrap_or_else(|| json!(false)),
                "message": authority.get("message").cloned().unwrap_or(Value::Null),
                "error": authority.get("error").cloned().unwrap_or(Value::Null),
                "receipt_ref": shell_socket_receipt_ref("save_provider_key", &json!({"provider": provider_id})),
                "correlation_id": "shell_socket.save_provider_key"
            }),
        });
    }
    if method == "POST" && parts.len() == 4 && parts[0] == "providers" && parts[2] == "key" && parts[3] == "remove" {
        let provider_id = normalize_provider_route_id(&parts[1]);
        let authority = crate::dashboard_provider_runtime::remove_provider_key(root, &provider_id);
        let ok = authority.get("ok").and_then(Value::as_bool).unwrap_or(false);
        return Some(CompatApiResponse {
            status: if ok { 200 } else { 400 },
            payload: json!({
                "ok": ok,
                "provider": authority.get("provider").cloned().unwrap_or_else(|| json!(provider_id)),
                "auth_status": authority.get("auth_status").cloned().unwrap_or(Value::Null),
                "error": authority.get("error").cloned().unwrap_or(Value::Null),
                "receipt_ref": shell_socket_receipt_ref("remove_provider_key", &json!({"provider": provider_id})),
                "correlation_id": "shell_socket.remove_provider_key"
            }),
        });
    }
    if method == "POST" && parts.len() == 3 && parts[0] == "providers" && parts[2] == "test" {
        let provider_id = normalize_provider_route_id(&parts[1]);
        let authority = crate::dashboard_provider_runtime::test_provider(root, &provider_id);
        let ok = authority.get("ok").and_then(Value::as_bool).unwrap_or(false);
        return Some(CompatApiResponse {
            status: 200,
            payload: json!({
                "ok": ok,
                "status": authority.get("status").cloned().unwrap_or_else(|| if ok { json!("ok") } else { json!("error") }),
                "provider": authority.get("provider").cloned().unwrap_or_else(|| json!(provider_id)),
                "latency_ms": authority.get("latency_ms").cloned().unwrap_or_else(|| json!(0)),
                "error": authority.get("error").cloned().unwrap_or(Value::Null),
                "receipt_ref": shell_socket_receipt_ref("test_provider", &json!({"provider": provider_id})),
                "correlation_id": "shell_socket.test_provider"
            }),
        });
    }
    if method == "POST" && parts.len() == 3 && parts[0] == "providers" && parts[2] == "complete" {
        let provider_id = normalize_provider_route_id(&parts[1]);
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let authority = crate::dashboard_provider_runtime::complete_provider(root, &provider_id, &request);
        let ok = authority.get("ok").and_then(Value::as_bool).unwrap_or(false);
        return Some(CompatApiResponse {
            status: if ok { 200 } else { 400 },
            payload: json!({
                "ok": ok,
                "status": authority.get("status").cloned().unwrap_or_else(|| if ok { json!("ok") } else { json!("error") }),
                "provider": authority.get("provider").cloned().unwrap_or_else(|| json!(provider_id)),
                "model": authority.get("model").cloned().unwrap_or(Value::Null),
                "output": authority.get("output").cloned().unwrap_or(Value::Null),
                "usage_tokens": authority.get("usage_tokens").cloned().unwrap_or_else(|| json!(0)),
                "latency_ms": authority.get("latency_ms").cloned().unwrap_or_else(|| json!(0)),
                "error": authority.get("error").cloned().unwrap_or(Value::Null),
                "receipt_ref": shell_socket_receipt_ref("complete_provider", &json!({"provider": provider_id})),
                "correlation_id": "shell_socket.complete_provider"
            }),
        });
    }
    if method == "POST" && parts.len() == 3 && parts[0] == "providers" && parts[2] == "url" {
        let provider_id = normalize_provider_route_id(&parts[1]);
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let base_url = clean_text(request.get("base_url").and_then(Value::as_str).unwrap_or(""), 400);
        let authority = crate::dashboard_provider_runtime::set_provider_url(root, &provider_id, &base_url);
        let ok = authority.get("ok").and_then(Value::as_bool).unwrap_or(false);
        return Some(CompatApiResponse {
            status: if ok { 200 } else { 400 },
            payload: json!({
                "ok": ok,
                "provider": authority.get("provider").cloned().unwrap_or_else(|| json!(provider_id)),
                "reachable": authority.get("reachable").cloned().unwrap_or_else(|| json!(false)),
                "latency_ms": authority.get("latency_ms").cloned().unwrap_or_else(|| json!(0)),
                "error": authority.get("error").cloned().unwrap_or(Value::Null),
                "receipt_ref": shell_socket_receipt_ref("set_provider_url", &json!({"provider": provider_id})),
                "correlation_id": "shell_socket.set_provider_url"
            }),
        });
    }
    if method == "POST" && parts.len() == 4 && parts[0] == "providers" && parts[2] == "oauth" && parts[3] == "start" {
        let provider_id = normalize_provider_route_id(&parts[1]);
        if provider_id != "github-copilot" {
            return Some(CompatApiResponse {
                status: 400,
                payload: json!({
                    "ok": false,
                    "provider": provider_id,
                    "status": "error",
                    "error": "provider_oauth_unsupported",
                    "receipt_ref": shell_socket_receipt_ref("start_provider_oauth", &json!({"provider": provider_id})),
                    "correlation_id": "shell_socket.start_provider_oauth"
                }),
            });
        }
        let legacy = dashboard_compat_api_settings_ops::handle(
            root,
            "POST",
            "/api/providers/github-copilot/oauth/start",
            body,
        );
        let authority = legacy.map(|response| response.payload).unwrap_or_else(|| json!({"ok": false, "status": "error", "error": "provider_oauth_unavailable"}));
        let ok = authority.get("ok").and_then(Value::as_bool).unwrap_or(false);
        return Some(CompatApiResponse {
            status: if ok { 200 } else { 400 },
            payload: json!({
                "ok": ok,
                "provider": authority.get("provider").cloned().unwrap_or_else(|| json!("github-copilot")),
                "status": authority.get("status").cloned().unwrap_or_else(|| if ok { json!("pending") } else { json!("error") }),
                "poll_id": authority.get("poll_id").cloned().unwrap_or(Value::Null),
                "user_code": authority.get("user_code").cloned().unwrap_or(Value::Null),
                "verification_uri": authority.get("verification_uri").cloned().unwrap_or(Value::Null),
                "interval": authority.get("interval").cloned().unwrap_or_else(|| json!(5)),
                "expires_in": authority.get("expires_in").cloned().unwrap_or(Value::Null),
                "error": authority.get("error").cloned().unwrap_or(Value::Null),
                "receipt_ref": shell_socket_receipt_ref("start_provider_oauth", &json!({"provider": "github-copilot"})),
                "correlation_id": "shell_socket.start_provider_oauth"
            }),
        });
    }
    if method == "POST" && parts.len() == 4 && parts[0] == "providers" && parts[2] == "oauth" && parts[3] == "poll" {
        let provider_id = normalize_provider_route_id(&parts[1]);
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let poll_id = clean_text(request.get("poll_id").and_then(Value::as_str).unwrap_or(""), 120);
        if provider_id != "github-copilot" || poll_id.is_empty() {
            return Some(CompatApiResponse {
                status: 400,
                payload: json!({
                    "ok": false,
                    "provider": provider_id,
                    "status": "error",
                    "poll_id": poll_id,
                    "error": if poll_id.is_empty() { "poll_id_required" } else { "provider_oauth_unsupported" },
                    "receipt_ref": shell_socket_receipt_ref("poll_provider_oauth", &json!({"provider": provider_id, "poll_id": poll_id})),
                    "correlation_id": "shell_socket.poll_provider_oauth"
                }),
            });
        }
        let legacy_path = format!("/api/providers/github-copilot/oauth/poll/{poll_id}");
        let authority = dashboard_compat_api_settings_ops::handle(root, "GET", &legacy_path, body)
            .map(|response| response.payload)
            .unwrap_or_else(|| json!({"ok": false, "status": "error", "error": "provider_oauth_unavailable"}));
        let ok = authority.get("ok").and_then(Value::as_bool).unwrap_or(false);
        return Some(CompatApiResponse {
            status: if ok { 200 } else { 400 },
            payload: json!({
                "ok": ok,
                "provider": "github-copilot",
                "status": authority.get("status").cloned().unwrap_or_else(|| if ok { json!("pending") } else { json!("error") }),
                "poll_id": poll_id,
                "interval": authority.get("interval").cloned().unwrap_or_else(|| json!(5)),
                "error": authority.get("error").cloned().unwrap_or(Value::Null),
                "receipt_ref": shell_socket_receipt_ref("poll_provider_oauth", &json!({"provider": "github-copilot", "poll_id": poll_id})),
                "correlation_id": "shell_socket.poll_provider_oauth"
            }),
        });
    }
    if method == "POST" && parts == ["config", "set"] {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let authority = set_config_payload(root, snapshot, &request);
        let ok = authority.get("ok").and_then(Value::as_bool).unwrap_or(false);
        let path_value = clean_text(
            request
                .get("path")
                .and_then(Value::as_str)
                .or_else(|| authority.get("path").and_then(Value::as_str))
                .unwrap_or(""),
            120,
        );
        let display_value = authority
            .get("value")
            .cloned()
            .or_else(|| request.get("value").cloned())
            .unwrap_or(Value::Null);
        return Some(CompatApiResponse {
            status: if ok { 200 } else { 400 },
            payload: json!({
                "ok": ok,
                "path": path_value,
                "value": display_value,
                "provider": authority.get("provider").cloned().unwrap_or(Value::Null),
                "auth_status": authority.get("auth_status").cloned().unwrap_or(Value::Null),
                "switched_default": authority.get("switched_default").cloned().unwrap_or(Value::Null),
                "message": authority.get("message").cloned().unwrap_or(Value::Null),
                "error": authority.get("error").cloned().unwrap_or(Value::Null),
                "receipt_ref": shell_socket_receipt_ref("set_config", &json!({"path": path_value})),
                "correlation_id": "shell_socket.set_config"
            }),
        });
    }
    if method == "POST" && parts == ["input"] {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let agent_id = clean_agent_id(request.get("agent_id").or_else(|| request.get("target_agent_id")).and_then(Value::as_str).unwrap_or(""));
        let message = clean_chat_text(
            request.get("message").or_else(|| request.get("text")).or_else(|| request.get("input")).and_then(Value::as_str).unwrap_or(""),
            24_000,
        );
        if agent_id.is_empty() || message.is_empty() {
            return Some(CompatApiResponse {
                status: 400,
                payload: shell_socket_ingress_ack("submit_input", false, "agent_id_and_message_required", &request),
            });
        }
        let legacy_path = format!("/api/agents/{agent_id}/message");
        let legacy_body = serde_json::to_vec(&json!({"message": message})).unwrap_or_default();
        let legacy = handle_agent_scope_routes(root, "POST", &legacy_path, &legacy_path, &legacy_body, headers, snapshot, requester_agent)?;
        return Some(shell_socket_ack_from_legacy("submit_input", legacy));
    }
    if method == "POST" && parts.len() == 3 && parts[0] == "agents" && parts[2] == "message" {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let agent_id = clean_agent_id(&parts[1]);
        let message = clean_chat_text(
            request.get("message").or_else(|| request.get("text")).or_else(|| request.get("input")).and_then(Value::as_str).unwrap_or(""),
            24_000,
        );
        if agent_id.is_empty() || message.is_empty() {
            return Some(CompatApiResponse {
                status: 400,
                payload: shell_socket_ingress_ack("submit_message_result", false, "agent_id_and_message_required", &request),
            });
        }
        let mut legacy_request = request.as_object().cloned().unwrap_or_default();
        legacy_request.insert("message".to_string(), json!(message));
        let legacy_path = format!("/api/agents/{agent_id}/message");
        let legacy_body = serde_json::to_vec(&Value::Object(legacy_request)).unwrap_or_default();
        let legacy = handle_agent_scope_routes(root, "POST", &legacy_path, &legacy_path, &legacy_body, headers, snapshot, requester_agent)?;
        return Some(shell_socket_message_result_projection(legacy));
    }
    if method == "POST" && parts == ["issues"] {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let agent_id = clean_agent_id(request.get("agent_id").and_then(Value::as_str).unwrap_or(""));
        if agent_id.is_empty() {
            return Some(CompatApiResponse {
                status: 400,
                payload: shell_socket_ingress_ack("submit_issue", false, "agent_id_required", &request),
            });
        }
        let legacy_path = format!("/api/agents/{agent_id}/eval-feedback/report-issue");
        let legacy = handle_agent_scope_routes(root, "POST", &legacy_path, &legacy_path, body, headers, snapshot, requester_agent)?;
        return Some(shell_socket_ack_from_legacy("submit_issue", legacy));
    }
    if method == "POST" && parts == ["auth", "login"] {
        let legacy = dashboard_compat_api_reference_parity::handle(
            root,
            "POST",
            "/api/auth/login",
            "/api/auth/login",
            headers,
            body,
            snapshot,
        )?;
        return Some(shell_socket_auth_projection("login_session", legacy));
    }
    if method == "POST" && parts == ["auth", "logout"] {
        let legacy = dashboard_compat_api_reference_parity::handle(
            root,
            "POST",
            "/api/auth/logout",
            "/api/auth/logout",
            headers,
            body,
            snapshot,
        )?;
        return Some(shell_socket_auth_projection("logout_session", legacy));
    }
    if method == "POST" && parts == ["session-index", "rebuild"] {
        let payload = rebuild_indexed_session_states(
            root,
            &serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({})),
        );
        let accepted = payload.get("ok").and_then(Value::as_bool).unwrap_or(false);
        return Some(CompatApiResponse {
            status: if accepted { 202 } else { 400 },
            payload: shell_socket_ingress_ack(
                "rebuild_session_indexes",
                accepted,
                if accepted {
                    "accepted"
                } else {
                    "session_index_rebuild_failed"
                },
                &payload,
            ),
        });
    }
    if method == "POST" && parts.len() == 3 && parts[0] == "approvals" && parts[2] == "decision" {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let decision = clean_text(
            request
                .get("decision")
                .or_else(|| request.get("action"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            40,
        );
        let reason = clean_text(
            request
                .get("reason")
                .or_else(|| request.get("deny_reason"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            400,
        );
        let authority = crate::approval_gate_kernel::apply_approval_decision(
            root,
            &parts[1],
            &decision,
            if reason.is_empty() { None } else { Some(&reason) },
        );
        let accepted = authority.get("ok").and_then(Value::as_bool).unwrap_or(false);
        let reason_code = if accepted {
            "accepted"
        } else {
            authority
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("approval_gateway_rejected")
        };
        return Some(CompatApiResponse {
            status: if accepted { 202 } else { 400 },
            payload: shell_socket_ingress_ack(
                "submit_approval_decision",
                accepted,
                reason_code,
                &json!({"approval_id": parts[1], "request": request, "authority": authority}),
            ),
        });
    }
    if method == "POST" && parts.len() == 3 && parts[0] == "agents" && parts[2] == "model" {
        let legacy_path = format!("/api/agents/{}/model", clean_agent_id(&parts[1]));
        let legacy = handle_agent_scope_routes(root, "PUT", &legacy_path, &legacy_path, body, headers, snapshot, requester_agent)?;
        return Some(shell_socket_ack_from_legacy("set_model", legacy));
    }
    if method == "POST" && parts.len() == 3 && parts[0] == "agents" && parts[2] == "config" {
        let legacy_path = format!("/api/agents/{}/config", clean_agent_id(&parts[1]));
        let legacy = handle_agent_scope_routes(root, "PATCH", &legacy_path, &legacy_path, body, headers, snapshot, requester_agent)?;
        return Some(shell_socket_agent_mutation_projection("update_agent_config", legacy));
    }
    if method == "POST" && parts.len() == 3 && parts[0] == "agents" && parts[2] == "mode" {
        let legacy_path = format!("/api/agents/{}/mode", clean_agent_id(&parts[1]));
        let legacy = handle_agent_scope_routes(root, "PUT", &legacy_path, &legacy_path, body, headers, snapshot, requester_agent)?;
        return Some(shell_socket_agent_mutation_projection("update_agent_mode", legacy));
    }
    if method == "POST" && parts.len() == 3 && parts[0] == "agents" && parts[2] == "tools" {
        let legacy_path = format!("/api/agents/{}/tools", clean_agent_id(&parts[1]));
        let legacy = handle_agent_scope_routes(root, "PUT", &legacy_path, &legacy_path, body, headers, snapshot, requester_agent)?;
        return Some(shell_socket_agent_mutation_projection("update_agent_tools", legacy));
    }
    if method == "POST" && parts.len() == 2 && parts[0] == "agents" && parts[1] == "create" {
        let legacy_path = "/api/agents";
        let legacy = handle_primary_dashboard_routes(root, "POST", legacy_path, legacy_path, body, headers, snapshot, requester_agent)?;
        return Some(shell_socket_agent_lifecycle_projection("create_agent", legacy));
    }
    if method == "POST" && parts.len() == 3 && parts[0] == "agents" && parts[2] == "archive" {
        let legacy_path = format!("/api/agents/{}", clean_agent_id(&parts[1]));
        let legacy = handle_agent_scope_routes(root, "DELETE", &legacy_path, &legacy_path, body, headers, snapshot, requester_agent)?;
        return Some(shell_socket_agent_lifecycle_projection("archive_agent", legacy));
    }
    if method == "POST" && parts.len() == 3 && parts[0] == "agents" && parts[2] == "revive" {
        let legacy_path = format!("/api/agents/{}/revive", clean_agent_id(&parts[1]));
        let legacy = handle_primary_dashboard_routes(root, "POST", &legacy_path, &legacy_path, body, headers, snapshot, requester_agent)?;
        return Some(shell_socket_agent_lifecycle_projection("revive_agent", legacy));
    }
    if method == "POST" && parts.len() == 3 && parts[0] == "agents" && parts[2] == "clone" {
        let legacy_path = format!("/api/agents/{}/clone", clean_agent_id(&parts[1]));
        let legacy = handle_agent_scope_routes(root, "POST", &legacy_path, &legacy_path, body, headers, snapshot, requester_agent)?;
        return Some(shell_socket_agent_lifecycle_projection("clone_agent", legacy));
    }
    if method == "POST" && parts.len() == 4 && parts[0] == "agents" && parts[2] == "history" && parts[3] == "clear" {
        let legacy_path = format!("/api/agents/{}/history", clean_agent_id(&parts[1]));
        let legacy = handle_agent_scope_routes(root, "DELETE", &legacy_path, &legacy_path, body, headers, snapshot, requester_agent)?;
        return Some(shell_socket_agent_lifecycle_projection("clear_agent_history", legacy));
    }
    if method == "POST" && parts.len() == 4 && parts[0] == "agents" && parts[2] == "archived" && parts[3] == "delete" {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let contract_id = request
            .get("contract_id")
            .and_then(Value::as_str)
            .map(|value| clean_text(value, 120))
            .filter(|value| !value.is_empty());
        let legacy_path = if let Some(contract_id) = contract_id {
            format!(
                "/api/agents/terminated/{}?contract_id={}",
                clean_agent_id(&parts[1]),
                contract_id
            )
        } else {
            format!("/api/agents/terminated/{}", clean_agent_id(&parts[1]))
        };
        let legacy_path_only = format!("/api/agents/terminated/{}", clean_agent_id(&parts[1]));
        let legacy = handle_primary_dashboard_routes(root, "DELETE", &legacy_path, &legacy_path_only, body, headers, snapshot, requester_agent)?;
        return Some(shell_socket_agent_lifecycle_projection("delete_archived_agent", legacy));
    }
    if method == "POST" && parts.len() == 3 && parts[0] == "agents" && parts[1] == "archived" && parts[2] == "delete-all" {
        let legacy_path = "/api/agents/terminated?all=1";
        let legacy_path_only = "/api/agents/terminated";
        let legacy = handle_primary_dashboard_routes(root, "DELETE", legacy_path, legacy_path_only, body, headers, snapshot, requester_agent)?;
        return Some(shell_socket_agent_lifecycle_projection("delete_all_archived_agents", legacy));
    }
    if method == "POST" && parts.len() == 2 && parts[0] == "agents" && parts[1] == "archive-all" {
        let legacy_path = "/api/agents/archive-all";
        let legacy = handle_primary_dashboard_routes(root, "POST", legacy_path, legacy_path, body, headers, snapshot, requester_agent)?;
        return Some(shell_socket_agent_lifecycle_projection("archive_all_agents", legacy));
    }
    if method == "POST" && parts.len() == 3 && parts[0] == "agents" && parts[2] == "stop" {
        let legacy_path = format!("/api/agents/{}/stop", clean_agent_id(&parts[1]));
        let legacy = handle_agent_scope_routes(root, "POST", &legacy_path, &legacy_path, body, headers, snapshot, requester_agent)?;
        return Some(shell_socket_agent_lifecycle_projection("stop_agent", legacy));
    }
    if method == "POST" && parts.len() == 3 && parts[0] == "agents" && parts[2] == "sessions" {
        let legacy_path = format!("/api/agents/{}/sessions", clean_agent_id(&parts[1]));
        let legacy = handle_agent_scope_routes(root, "POST", &legacy_path, &legacy_path, body, headers, snapshot, requester_agent)?;
        return Some(shell_socket_session_lifecycle_projection("create_session", legacy));
    }
    if method == "POST" && parts.len() == 5 && parts[0] == "agents" && parts[2] == "sessions" && parts[4] == "switch" {
        let legacy_path = format!(
            "/api/agents/{}/sessions/{}/switch",
            clean_agent_id(&parts[1]),
            clean_text(&parts[3], 120)
        );
        let legacy = handle_agent_scope_routes(root, "POST", &legacy_path, &legacy_path, body, headers, snapshot, requester_agent)?;
        return Some(shell_socket_session_lifecycle_projection("switch_session", legacy));
    }
    if method == "POST" && parts.len() == 5 && parts[0] == "agents" && parts[2] == "sessions" && parts[4] == "delete" {
        let agent_id = clean_agent_id(&parts[1]);
        let session_id = clean_text(&parts[3], 120);
        let payload = crate::dashboard_agent_state::delete_session(root, &agent_id, &session_id);
        return Some(shell_socket_session_lifecycle_projection(
            "delete_session",
            CompatApiResponse {
                status: if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    200
                } else {
                    400
                },
                payload,
            },
        ));
    }
    if method == "POST" && parts.len() == 3 && parts[0] == "agents" && parts[2] == "suggestions" {
        let legacy_path = format!("/api/agents/{}/suggestions", clean_agent_id(&parts[1]));
        let legacy = handle_agent_scope_routes(root, "POST", &legacy_path, &legacy_path, body, headers, snapshot, requester_agent)?;
        return Some(shell_socket_suggestion_projection("request_agent_suggestions", legacy));
    }
    if method == "POST" && parts.len() == 5 && parts[0] == "agents" && parts[2] == "artifacts" && parts[3] == "file" && parts[4] == "read" {
        let legacy_path = format!("/api/agents/{}/file/read", clean_agent_id(&parts[1]));
        let legacy = handle_agent_scope_routes(root, "POST", &legacy_path, &legacy_path, body, headers, snapshot, requester_agent)?;
        return Some(shell_socket_artifact_projection("read_agent_file_artifact", legacy));
    }
    if method == "POST" && parts.len() == 5 && parts[0] == "agents" && parts[2] == "files" && parts[4] == "save" {
        let file_name = clean_text(&parts[3], 300);
        let legacy_path = format!(
            "/api/agents/{}/files/{}",
            clean_agent_id(&parts[1]),
            urlencoding::encode(&file_name)
        );
        let legacy = handle_agent_scope_routes(root, "PUT", &legacy_path, &legacy_path, body, headers, snapshot, requester_agent)?;
        return Some(shell_socket_file_save_projection("save_agent_file_artifact", legacy, &file_name));
    }
    if method == "POST" && parts.len() == 5 && parts[0] == "agents" && parts[2] == "artifacts" && parts[3] == "folder" && parts[4] == "export" {
        let legacy_path = format!("/api/agents/{}/folder/export", clean_agent_id(&parts[1]));
        let legacy = handle_agent_scope_routes(root, "POST", &legacy_path, &legacy_path, body, headers, snapshot, requester_agent)?;
        return Some(shell_socket_artifact_projection("export_agent_folder_artifact", legacy));
    }
    if method == "POST" && parts == ["workflows"] {
        let legacy_path = "/api/workflows";
        let legacy = dashboard_compat_api_sidebar_ops::handle(root, "POST", legacy_path, body, snapshot)?;
        return Some(shell_socket_workflow_projection("create_workflow", legacy));
    }
    if method == "POST" && parts.len() == 3 && parts[0] == "workflows" && parts[2] == "update" {
        let workflow_id = clean_text(&parts[1], 120);
        let legacy_path = format!("/api/workflows/{workflow_id}");
        let legacy = dashboard_compat_api_sidebar_ops::handle(root, "PUT", &legacy_path, body, snapshot)?;
        return Some(shell_socket_workflow_projection("update_workflow", legacy));
    }
    if method == "POST" && parts.len() == 3 && parts[0] == "workflows" && parts[2] == "delete" {
        let workflow_id = clean_text(&parts[1], 120);
        let legacy_path = format!("/api/workflows/{workflow_id}");
        let legacy = dashboard_compat_api_sidebar_ops::handle(root, "DELETE", &legacy_path, body, snapshot)?;
        return Some(shell_socket_workflow_projection("delete_workflow", legacy));
    }
    if method == "POST" && parts.len() == 3 && parts[0] == "workflows" && parts[2] == "run" {
        let workflow_id = clean_text(&parts[1], 120);
        let legacy_path = format!("/api/workflows/{workflow_id}/run");
        let legacy = dashboard_compat_api_sidebar_ops::handle(root, "POST", &legacy_path, body, snapshot)?;
        return Some(shell_socket_workflow_projection("run_workflow", legacy));
    }
    if method == "POST" && parts == ["scheduler", "jobs"] {
        let legacy_path = "/api/cron/jobs";
        let legacy = dashboard_compat_api_sidebar_ops::handle(root, "POST", legacy_path, body, snapshot)?;
        return Some(shell_socket_scheduler_projection("create_cron_job", legacy));
    }
    if method == "POST" && parts.len() == 4 && parts[0] == "scheduler" && parts[1] == "jobs" && parts[3] == "enable" {
        let job_id = clean_text(&parts[2], 120);
        let legacy_path = format!("/api/cron/jobs/{job_id}/enable");
        let legacy = dashboard_compat_api_sidebar_ops::handle(root, "PUT", &legacy_path, body, snapshot)?;
        return Some(shell_socket_scheduler_projection("set_cron_job_enabled", legacy));
    }
    if method == "POST" && parts.len() == 4 && parts[0] == "scheduler" && parts[1] == "jobs" && parts[3] == "delete" {
        let job_id = clean_text(&parts[2], 120);
        let legacy_path = format!("/api/cron/jobs/{job_id}");
        let legacy = dashboard_compat_api_sidebar_ops::handle(root, "DELETE", &legacy_path, body, snapshot)?;
        return Some(shell_socket_scheduler_projection("delete_cron_job", legacy));
    }
    if method == "POST" && parts.len() == 4 && parts[0] == "scheduler" && parts[1] == "jobs" && parts[3] == "run" {
        let job_id = clean_text(&parts[2], 120);
        let legacy_path = format!("/api/schedules/{job_id}/run");
        let legacy = dashboard_compat_api_sidebar_ops::handle(root, "POST", &legacy_path, body, snapshot)?;
        return Some(shell_socket_scheduler_projection("run_schedule", legacy));
    }
    if method == "POST" && parts.len() == 4 && parts[0] == "scheduler" && parts[1] == "triggers" && parts[3] == "enable" {
        let trigger_id = clean_text(&parts[2], 120);
        let legacy_path = format!("/api/triggers/{trigger_id}");
        let legacy = dashboard_compat_api_sidebar_ops::handle(root, "PUT", &legacy_path, body, snapshot)?;
        return Some(shell_socket_scheduler_projection("set_trigger_enabled", legacy));
    }
    if method == "POST" && parts.len() == 4 && parts[0] == "scheduler" && parts[1] == "triggers" && parts[3] == "delete" {
        let trigger_id = clean_text(&parts[2], 120);
        let legacy_path = format!("/api/triggers/{trigger_id}");
        let legacy = dashboard_compat_api_sidebar_ops::handle(root, "DELETE", &legacy_path, body, snapshot)?;
        return Some(shell_socket_scheduler_projection("delete_trigger", legacy));
    }
    if method == "POST" && parts.len() == 4 && parts[0] == "channels" && parts[2] == "qr" && parts[3] == "start" {
        let channel_id = clean_text(&parts[1], 120);
        let legacy_path = format!("/api/channels/{channel_id}/qr/start");
        let legacy = if channel_id == "whatsapp" {
            dashboard_compat_api_channels::handle(root, "POST", "/api/channels/whatsapp/qr/start", body)?
        } else {
            dashboard_compat_api_channels::handle(root, "POST", &legacy_path, body)?
        };
        return Some(shell_socket_channel_projection("start_channel_qr", &channel_id, legacy));
    }
    if method == "POST" && parts.len() == 3 && parts[0] == "channels" && parts[2] == "configure" {
        let channel_id = clean_text(&parts[1], 120);
        let legacy_path = format!("/api/channels/{channel_id}/configure");
        let legacy = dashboard_compat_api_channels::handle(root, "POST", &legacy_path, body)?;
        return Some(shell_socket_channel_projection("configure_channel", &channel_id, legacy));
    }
    if method == "POST" && parts.len() == 3 && parts[0] == "channels" && parts[2] == "test" {
        let channel_id = clean_text(&parts[1], 120);
        let legacy_path = format!("/api/channels/{channel_id}/test");
        let legacy = dashboard_compat_api_channels::handle(root, "POST", &legacy_path, body)?;
        return Some(shell_socket_channel_projection("test_channel", &channel_id, legacy));
    }
    if method == "POST" && parts.len() == 4 && parts[0] == "channels" && parts[2] == "configure" && parts[3] == "remove" {
        let channel_id = clean_text(&parts[1], 120);
        let legacy_path = format!("/api/channels/{channel_id}/configure");
        let legacy = dashboard_compat_api_channels::handle(root, "DELETE", &legacy_path, body)?;
        return Some(shell_socket_channel_projection("remove_channel_config", &channel_id, legacy));
    }
    if method == "GET" && parts == ["migration", "detect"] {
        let legacy = dashboard_compat_api_settings_ops::handle(root, "GET", "/api/migrate/detect", body)?;
        return Some(shell_socket_migration_projection("detect_migration_source", legacy));
    }
    if method == "POST" && parts == ["migration", "scan"] {
        let legacy = dashboard_compat_api_settings_ops::handle(root, "POST", "/api/migrate/scan", body)?;
        return Some(shell_socket_migration_projection("scan_migration_source", legacy));
    }
    if method == "POST" && parts == ["migration", "run"] {
        let legacy = dashboard_compat_api_settings_ops::handle(root, "POST", "/api/migrate", body)?;
        return Some(shell_socket_migration_projection("run_migration", legacy));
    }
    if method == "POST" && parts.len() == 3 && parts[0] == "hands" && parts[2] == "install-deps" {
        let hand_id = clean_text(&parts[1], 120);
        let legacy_path = format!("/api/hands/{hand_id}/install-deps");
        let legacy = dashboard_compat_api_hands::handle(root, "POST", &legacy_path, body, snapshot)?;
        return Some(shell_socket_hand_projection("install_hand_dependencies", legacy));
    }
    if method == "POST" && parts.len() == 3 && parts[0] == "hands" && parts[2] == "check-deps" {
        let hand_id = clean_text(&parts[1], 120);
        let legacy_path = format!("/api/hands/{hand_id}/check-deps");
        let legacy = dashboard_compat_api_hands::handle(root, "POST", &legacy_path, body, snapshot)?;
        return Some(shell_socket_hand_projection("check_hand_dependencies", legacy));
    }
    if method == "POST" && parts.len() == 3 && parts[0] == "hands" && parts[2] == "activate" {
        let hand_id = clean_text(&parts[1], 120);
        let legacy_path = format!("/api/hands/{hand_id}/activate");
        let legacy = dashboard_compat_api_hands::handle(root, "POST", &legacy_path, body, snapshot)?;
        return Some(shell_socket_hand_projection("activate_hand", legacy));
    }
    if method == "POST" && parts.len() == 4 && parts[0] == "hands" && parts[1] == "instances" && parts[3] == "pause" {
        let instance_id = clean_text(&parts[2], 120);
        let legacy_path = format!("/api/hands/instances/{instance_id}/pause");
        let legacy = dashboard_compat_api_hands::handle(root, "POST", &legacy_path, body, snapshot)?;
        return Some(shell_socket_hand_projection("pause_hand_instance", legacy));
    }
    if method == "POST" && parts.len() == 4 && parts[0] == "hands" && parts[1] == "instances" && parts[3] == "resume" {
        let instance_id = clean_text(&parts[2], 120);
        let legacy_path = format!("/api/hands/instances/{instance_id}/resume");
        let legacy = dashboard_compat_api_hands::handle(root, "POST", &legacy_path, body, snapshot)?;
        return Some(shell_socket_hand_projection("resume_hand_instance", legacy));
    }
    if method == "POST" && parts.len() == 4 && parts[0] == "hands" && parts[1] == "instances" && parts[3] == "deactivate" {
        let instance_id = clean_text(&parts[2], 120);
        let legacy_path = format!("/api/hands/instances/{instance_id}");
        let legacy = dashboard_compat_api_hands::handle(root, "DELETE", &legacy_path, body, snapshot)?;
        return Some(shell_socket_hand_projection("deactivate_hand_instance", legacy));
    }
    if method == "POST" && parts == ["skills", "install"] {
        let legacy = dashboard_skills_marketplace::handle(root, "POST", "/api/clawhub/install", snapshot, body)?;
        return Some(shell_socket_skill_projection("install_skill", legacy));
    }
    if method == "POST" && parts == ["skills", "uninstall"] {
        let legacy = dashboard_skills_marketplace::handle(root, "POST", "/api/skills/uninstall", snapshot, body)?;
        return Some(shell_socket_skill_projection("uninstall_skill", legacy));
    }
    if method == "POST" && parts == ["skills", "create"] {
        let legacy = dashboard_skills_marketplace::handle(root, "POST", "/api/skills/create", snapshot, body)?;
        return Some(shell_socket_skill_projection("create_skill", legacy));
    }
    if method == "POST" && parts == ["comms", "send"] {
        let legacy = dashboard_compat_api_comms::handle(root, "POST", "/api/comms/send", "/api/comms/send", body, snapshot)?;
        return Some(shell_socket_comms_projection("send_comms_message", legacy));
    }
    if method == "POST" && parts == ["comms", "task"] {
        let legacy = dashboard_compat_api_comms::handle(root, "POST", "/api/comms/task", "/api/comms/task", body, snapshot)?;
        return Some(shell_socket_comms_projection("post_comms_task", legacy));
    }
    if method == "POST" && parts == ["eyes"] {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        return Some(shell_socket_upsert_eye(root, &request));
    }
    if method == "POST" && parts.len() == 3 && parts[0] == "agents" && parts[2] == "git-tree" {
        let legacy_path = format!("/api/agents/{}/git-tree/switch", clean_agent_id(&parts[1]));
        let legacy = handle_agent_scope_routes(root, "POST", &legacy_path, &legacy_path, body, headers, snapshot, requester_agent)?;
        return Some(shell_socket_ack_from_legacy("set_git_tree", legacy));
    }
    if method == "POST" && parts.len() == 3 && parts[0] == "agents" && parts[2] == "fresh-session" {
        let agent_id = clean_agent_id(&parts[1]);
        let legacy_path = format!("/api/agents/{agent_id}/session/reset");
        let legacy = handle_agent_scope_routes(root, "POST", &legacy_path, &legacy_path, body, headers, snapshot, requester_agent)?;
        return Some(shell_socket_ack_from_legacy("fresh_session", legacy));
    }
    if method == "POST" && parts.len() == 3 && parts[0] == "agents" && parts[2] == "compact-session" {
        let agent_id = clean_agent_id(&parts[1]);
        let legacy_path = format!("/api/agents/{agent_id}/session/compact");
        let legacy = handle_agent_scope_routes(root, "POST", &legacy_path, &legacy_path, body, headers, snapshot, requester_agent)?;
        return Some(shell_socket_ack_from_legacy("compact_session", legacy));
    }
    if parts.len() >= 4 && parts[0] == "agents" && parts[2] == "memory" && parts[3] == "kv" {
        let agent_id = resolve_agent_id_alias(root, &clean_agent_id(&parts[1]));
        if !requester_agent.is_empty()
            && requester_agent != agent_id
            && !actor_can_manage_target(root, snapshot, &requester_agent, &agent_id)
        {
            return Some(CompatApiResponse {
                status: 403,
                payload: json!({
                    "ok": false,
                    "error": "agent_manage_forbidden",
                    "actor_agent_id": requester_agent,
                    "target_agent_id": agent_id,
                    "correlation_id": "shell_socket.memory_kv_forbidden"
                }),
            });
        }
        if method == "GET" && parts.len() == 4 {
            let mut payload = crate::dashboard_agent_state::memory_kv_pairs(root, &agent_id);
            let receipt_ref = shell_socket_receipt_ref("list_memory_kv", &payload);
            if let Some(object) = payload.as_object_mut() {
                object.insert("receipt_ref".to_string(), json!(receipt_ref));
                object.insert(
                    "correlation_id".to_string(),
                    json!("shell_socket.list_memory_kv"),
                );
            }
            return Some(CompatApiResponse { status: 200, payload });
        }
        if method == "POST" && parts.len() >= 5 {
            let key = decode_path_segment(&parts[4..].join("/"));
            if parts.last().map(|part| part == "delete").unwrap_or(false) && parts.len() >= 6 {
                let key = decode_path_segment(&parts[4..parts.len() - 1].join("/"));
                let payload = crate::dashboard_agent_state::memory_kv_delete(root, &agent_id, &key);
                return Some(shell_socket_memory_kv_projection("delete_memory_kv", payload));
            }
            let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
            let value = request.get("value").cloned().unwrap_or(Value::Null);
            let payload = crate::dashboard_agent_state::memory_kv_set(root, &agent_id, &key, &value);
            return Some(shell_socket_memory_kv_projection("set_memory_kv", payload));
        }
    }
    if method == "POST"
        && parts.len() == 4
        && parts[0] == "agents"
        && parts[2] == "session-index"
        && parts[3] == "rebuild"
    {
        let agent_id = clean_agent_id(&parts[1]);
        let legacy_path = format!("/api/agents/{agent_id}/session/index/rebuild");
        let legacy = handle_agent_scope_routes(
            root,
            "POST",
            &legacy_path,
            &legacy_path,
            body,
            headers,
            snapshot,
            requester_agent,
        )?;
        return Some(shell_socket_ack_from_legacy("rebuild_session_index", legacy));
    }
    if method == "POST" && parts == ["terminal", "commands"] {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let agent_id = clean_agent_id(request.get("agent_id").and_then(Value::as_str).unwrap_or(""));
        if agent_id.is_empty() {
            return Some(CompatApiResponse {
                status: 400,
                payload: shell_socket_ingress_ack("submit_terminal_command", false, "agent_id_required", &request),
            });
        }
        let legacy_path = format!("/api/agents/{agent_id}/terminal");
        let legacy = handle_agent_scope_routes(root, "POST", &legacy_path, &legacy_path, body, headers, snapshot, requester_agent)?;
        return Some(shell_socket_ack_from_legacy("submit_terminal_command", legacy));
    }
    Some(CompatApiResponse { status: 404, payload: json!({"error": "shell_socket_route_not_found"}) })
}
