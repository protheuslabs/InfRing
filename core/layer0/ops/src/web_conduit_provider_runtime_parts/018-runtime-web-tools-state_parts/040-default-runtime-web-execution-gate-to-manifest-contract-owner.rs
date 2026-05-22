
fn default_runtime_web_execution_gate() -> Value {
    json!({
        "should_execute": false,
        "mode": "blocked",
        "reason": "unknown",
        "retry_recommended": true,
        "retry_lane": "repair_tool_surface"
    })
}

pub(crate) fn runtime_web_execution_gate(
    tool_surface_status: &str,
    tool_surface_ready: bool,
    allow_fallback: bool,
    blocking_reason: &str,
) -> Value {
    let status = clean_text(tool_surface_status, 40).to_ascii_lowercase();
    let reason = clean_text(blocking_reason, 120).to_ascii_lowercase();
    if status == "ready" && (tool_surface_ready || allow_fallback) {
        return json!({
            "should_execute": true,
            "mode": "allow",
            "reason": "none",
            "retry_recommended": false,
            "retry_lane": "none"
        });
    }
    if status == "degraded" {
        if tool_surface_ready || allow_fallback {
            return json!({
                "should_execute": true,
                "mode": "degraded_allow",
                "reason": if reason.is_empty() { "degraded_but_fallback_available" } else { reason.as_str() },
                "retry_recommended": true,
                "retry_lane": "degraded_backoff"
            });
        }
        return json!({
            "should_execute": false,
            "mode": "blocked",
            "reason": if reason.is_empty() { "degraded_without_fallback" } else { reason.as_str() },
            "retry_recommended": true,
            "retry_lane": "repair_tool_surface"
        });
    }
    if status == "unavailable" {
        return json!({
            "should_execute": false,
            "mode": "blocked",
            "reason": if reason.is_empty() { "tool_surface_unavailable" } else { reason.as_str() },
            "retry_recommended": true,
            "retry_lane": "repair_tool_surface"
        });
    }
    json!({
        "should_execute": false,
        "mode": "blocked",
        "reason": if reason.is_empty() { "unknown" } else { reason.as_str() },
        "retry_recommended": true,
        "retry_lane": "repair_tool_surface"
    })
}

fn runtime_web_family_health(
    family: WebProviderFamily,
    selected_provider: Option<&str>,
    selected_provider_key_source: &Value,
    selection_fallback_reason: Option<&str>,
    diagnostics: &[Value],
) -> Value {
    let selected_provider_requires_credential = selected_provider
        .and_then(|provider| provider_descriptor(provider, family))
        .map(|descriptor| !descriptor.env_keys.is_empty())
        .unwrap_or(false);
    let selected_provider_credential_state = match (
        selected_provider_key_source.as_str(),
        selected_provider_requires_credential,
    ) {
        (Some("config" | "env" | "secret_broker" | "not_required"), _) => "resolved",
        (Some("missing"), true) => "missing",
        (Some("missing"), false) => "not_required",
        _ => "unknown",
    };
    let selected_provider_ready = selected_provider.is_some()
        && (selected_provider_credential_state == "resolved"
            || selected_provider_credential_state == "not_required");
    let status = if selected_provider.is_none() {
        "unavailable"
    } else if selection_fallback_reason == Some("credential_unresolved")
        || (selected_provider_requires_credential
            && selected_provider_credential_state == "missing")
    {
        "degraded"
    } else {
        "ready"
    };
    let blocking_reason = if selected_provider.is_none() {
        "no_selected_provider"
    } else if selection_fallback_reason == Some("credential_unresolved") {
        "configured_provider_credential_unresolved"
    } else if selected_provider_requires_credential
        && selected_provider_credential_state == "missing"
    {
        "selected_provider_credential_missing"
    } else {
        "none"
    };
    json!({
        "status": status,
        "selected_provider_ready": selected_provider_ready,
        "selected_provider_requires_credential": selected_provider_requires_credential,
        "selected_provider_credential_state": selected_provider_credential_state,
        "blocking_reason": blocking_reason,
        "available_provider_count": builtin_provider_descriptors(family).len(),
        "diagnostic_count": diagnostics.len()
    })
}

fn invalid_provider_code(family: WebProviderFamily) -> &'static str {
    match family {
        WebProviderFamily::Search => "WEB_SEARCH_PROVIDER_INVALID_AUTODETECT",
        WebProviderFamily::Fetch => "WEB_FETCH_PROVIDER_INVALID_AUTODETECT",
    }
}

fn auto_detect_code(family: WebProviderFamily) -> &'static str {
    match family {
        WebProviderFamily::Search => "WEB_SEARCH_AUTODETECT_SELECTED",
        WebProviderFamily::Fetch => "WEB_FETCH_AUTODETECT_SELECTED",
    }
}

fn fallback_used_code(family: WebProviderFamily) -> &'static str {
    match family {
        WebProviderFamily::Search => "WEB_SEARCH_KEY_UNRESOLVED_FALLBACK_USED",
        WebProviderFamily::Fetch => "WEB_FETCH_PROVIDER_KEY_UNRESOLVED_FALLBACK_USED",
    }
}

fn no_fallback_code(family: WebProviderFamily) -> &'static str {
    match family {
        WebProviderFamily::Search => "WEB_SEARCH_KEY_UNRESOLVED_NO_FALLBACK",
        WebProviderFamily::Fetch => "WEB_FETCH_PROVIDER_KEY_UNRESOLVED_NO_FALLBACK",
    }
}

fn configured_scope_path(provider: &str, family: WebProviderFamily) -> String {
    match family {
        WebProviderFamily::Search => format!("/web_conduit/search_provider_config/{provider}"),
        WebProviderFamily::Fetch => format!("/web_conduit/fetch_provider_config/{provider}"),
    }
}

fn config_surface_snapshot(
    policy: &Value,
    provider: Option<&str>,
    family: WebProviderFamily,
) -> Value {
    let Some(provider_id) = provider else {
        return Value::Null;
    };
    match family {
        WebProviderFamily::Search => {
            let section = search_provider_config_section(policy, provider_id);
            let inline_present = section
                .and_then(|row| row.get("api_key"))
                .and_then(Value::as_str)
                .map(|raw| !clean_text(raw, 600).is_empty())
                .unwrap_or(false);
            let env_name = section
                .and_then(|row| row.get("api_key_env"))
                .and_then(Value::as_str)
                .map(|raw| clean_text(raw, 160))
                .filter(|value| !value.is_empty());
            json!({
                "path": configured_scope_path(provider_id, family),
                "configured": section.is_some(),
                "has_inline_api_key": inline_present,
                "has_api_key_env": env_name.is_some(),
                "api_key_env": env_name
            })
        }
        WebProviderFamily::Fetch => json!({
            "path": configured_scope_path(provider_id, family),
            "configured": false
        }),
    }
}

fn manifest_contract_owner(provider: Option<&str>, family: WebProviderFamily) -> Value {
    provider
        .map(|provider_id| {
            json!({
                "kind": "built_in",
                "provider": provider_id,
                "family": provider_family_name(family)
            })
        })
        .unwrap_or(Value::Null)
}
