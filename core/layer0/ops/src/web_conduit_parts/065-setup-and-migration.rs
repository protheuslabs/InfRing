const WEB_SETUP_DOCS_URL: &str = "https://docs.openclaw.ai/tools/web";

fn provider_label(provider: &str) -> &'static str {
    match provider {
        "tavily" => "Tavily",
        "exa" => "Exa",
        "brave" => "Brave Search",
        "duckduckgo" => "DuckDuckGo",
        "duckduckgo_lite" => "DuckDuckGo Lite",
        "bing_rss" => "Bing RSS",
        "serperdev" => "Serper",
        "direct_http" => "Direct HTTP",
        _ => "Web Provider",
    }
}

fn provider_hint(provider: &str) -> &'static str {
    match provider {
        "tavily" => "AI-optimized structured search API",
        "exa" => "Neural search API with optional contents",
        "brave" => "Independent web search API",
        "duckduckgo" => "HTML search with broad web coverage",
        "duckduckgo_lite" => "Low-friction fallback HTML search",
        "bing_rss" => "RSS-backed search fallback",
        "serperdev" => "Structured Google search API",
        "direct_http" => "Direct HTTP fetch runtime",
        _ => "Built-in web tool provider",
    }
}

fn provider_signup_url(provider: &str) -> Option<&'static str> {
    match provider {
        "tavily" => Some("https://app.tavily.com"),
        "exa" => Some("https://dashboard.exa.ai"),
        "brave" => Some("https://api.search.brave.com"),
        "serperdev" => Some("https://serper.dev"),
        _ => None,
    }
}

fn provider_placeholder(provider: &str) -> Option<&'static str> {
    match provider {
        "tavily" => Some("tvly-..."),
        "exa" => Some("exa-..."),
        "brave" => Some("brave-..."),
        "serperdev" => Some("serper-..."),
        _ => None,
    }
}

fn policy_string_list(policy: &Value, pointer: &str, fallback_key: &str) -> Vec<String> {
    let raw = policy.pointer(pointer).or_else(|| policy.get(fallback_key));
    if let Some(array) = raw.and_then(Value::as_array) {
        return array
            .iter()
            .filter_map(Value::as_str)
            .map(|row| clean_text(row, 80))
            .filter(|row| !row.is_empty())
            .collect();
    }
    raw.and_then(Value::as_str)
        .map(|text| {
            text.split(|ch: char| ch == ',' || ch.is_ascii_whitespace())
                .map(str::trim)
                .filter(|row| !row.is_empty())
                .map(|row| clean_text(row, 80))
                .collect()
        })
        .unwrap_or_default()
}

fn dedupe_strings(rows: Vec<String>) -> Vec<String> {
    rows.into_iter().fold(Vec::<String>::new(), |mut acc, row| {
        if !acc.iter().any(|existing| existing == &row) {
            acc.push(row);
        }
        acc
    })
}

fn normalize_search_provider_alias(policy: &Value, raw: &str) -> Option<String> {
    if validate_explicit_provider_hint(raw).is_some() {
        return None;
    }
    provider_chain_from_request(raw, &json!({}), policy)
        .into_iter()
        .next()
}

fn normalize_fetch_provider_alias(policy: &Value, raw: &str) -> Option<String> {
    if validate_explicit_fetch_provider_hint(raw).is_some() {
        return None;
    }
    fetch_provider_chain_from_request(raw, &json!({}), policy)
        .into_iter()
        .next()
}

fn normalized_search_policy_order(root: &Path, policy: &Value) -> Vec<String> {
    let mut merged = policy_string_list(policy, "/web_conduit/search_provider_order", "search_provider_order")
        .into_iter()
        .filter_map(|raw| normalize_search_provider_alias(policy, &raw))
        .collect::<Vec<_>>();
    merged.extend(
        provider_catalog_snapshot(root, policy)
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|row| row.get("provider").and_then(Value::as_str))
            .map(ToString::to_string),
    );
    dedupe_strings(merged)
}

fn normalized_fetch_policy_order(root: &Path, policy: &Value) -> Vec<String> {
    let mut merged = policy_string_list(policy, "/web_conduit/fetch_provider_order", "fetch_provider_order")
        .into_iter()
        .filter_map(|raw| normalize_fetch_provider_alias(policy, &raw))
        .collect::<Vec<_>>();
    merged.extend(
        fetch_provider_catalog_snapshot(root, policy)
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|row| row.get("provider").and_then(Value::as_str))
            .map(ToString::to_string),
    );
    dedupe_strings(merged)
}

fn current_search_provider_configured(root: &Path, policy: &Value) -> Option<String> {
    normalized_search_policy_order(root, policy).into_iter().next()
}

fn ensure_object_value(value: &mut Value) -> &mut serde_json::Map<String, Value> {
    if !value.is_object() {
        *value = json!({});
    }
    value.as_object_mut().expect("object")
}

fn ensure_child_value<'a>(parent: &'a mut Value, key: &str) -> &'a mut Value {
    let object = ensure_object_value(parent);
    object.entry(key.to_string()).or_insert_with(|| json!({}))
}

fn set_search_provider_order(policy: &mut Value, order: &[String]) {
    let web_conduit = ensure_child_value(policy, "web_conduit");
    ensure_object_value(web_conduit)
        .insert("search_provider_order".to_string(), json!(order));
}

fn set_fetch_provider_order(policy: &mut Value, order: &[String]) {
    let web_conduit = ensure_child_value(policy, "web_conduit");
    ensure_object_value(web_conduit)
        .insert("fetch_provider_order".to_string(), json!(order));
}

fn ensure_search_provider_config_entry<'a>(policy: &'a mut Value, provider: &str) -> &'a mut Value {
    let web_conduit = ensure_child_value(policy, "web_conduit");
    let search_provider_config = ensure_child_value(web_conduit, "search_provider_config");
    ensure_child_value(search_provider_config, provider)
}

fn ensure_legacy_archive_entry<'a>(policy: &'a mut Value, family: &str, key: &str) -> &'a mut Value {
    let web_conduit = ensure_child_value(policy, "web_conduit");
    let archive = ensure_child_value(web_conduit, "legacy_migration_archive");
    let family_entry = ensure_child_value(archive, family);
    ensure_child_value(family_entry, key)
}

fn remove_legacy_tools_key(policy: &mut Value, branch: &str) {
    if let Some(tools) = policy.get_mut("tools").and_then(Value::as_object_mut) {
        if let Some(web) = tools.get_mut("web").and_then(Value::as_object_mut) {
            web.remove(branch);
            if web.is_empty() {
                tools.remove("web");
            }
        }
        if tools.is_empty() {
            if let Some(root) = policy.as_object_mut() {
                root.remove("tools");
            }
        }
    }
}

fn search_setup_option_row(row: &Value, default_provider: &str) -> Value {
    let provider = row
        .get("provider")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let requires_credential = row
        .get("requires_credential")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let credential_present = row
        .get("credential_present")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let hint = if !requires_credential {
        format!("{} · key-free", provider_hint(provider))
    } else if credential_present {
        format!("{} · configured", provider_hint(provider))
    } else {
        provider_hint(provider).to_string()
    };
    json!({
        "provider": provider,
        "label": provider_label(provider),
        "hint": hint,
        "requires_credential": requires_credential,
        "credential_present": credential_present,
        "credential_source": row.get("credential_source").cloned().unwrap_or(Value::Null),
        "aliases": row.get("aliases").cloned().unwrap_or_else(|| json!([])),
        "docs_url": WEB_SETUP_DOCS_URL,
        "signup_url": provider_signup_url(provider),
        "placeholder": provider_placeholder(provider),
        "selected_by_default": provider == default_provider
    })
}

fn default_setup_provider(root: &Path, policy: &Value) -> Option<String> {
    let options = provider_catalog_snapshot(root, policy)
        .as_array()
        .cloned()
        .unwrap_or_default();
    if let Some(configured) = current_search_provider_configured(root, policy) {
        if options.iter().any(|row| {
            row.get("provider").and_then(Value::as_str) == Some(configured.as_str())
        }) {
            return Some(configured);
        }
    }
    options
        .iter()
        .find(|row| {
            !row.get("requires_credential")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                || row
                    .get("credential_present")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
        })
        .or_else(|| options.first())
        .and_then(|row| row.get("provider").and_then(Value::as_str))
        .map(ToString::to_string)
}

fn web_setup_contract(root: &Path, policy: &Value) -> Value {
    let default_provider = default_setup_provider(root, policy).unwrap_or_default();
    let options = provider_catalog_snapshot(root, policy)
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|row| search_setup_option_row(&row, &default_provider))
        .collect::<Vec<_>>();
    json!({
        "docs_url": WEB_SETUP_DOCS_URL,
        "default_provider": if default_provider.is_empty() { Value::Null } else { Value::String(default_provider) },
        "current_provider": current_search_provider_configured(root, policy),
        "supports_apply": true,
        "supports_summary_only": true,
        "supports_inline_api_key": true,
        "supports_api_key_env": true,
        "commands": [
            "infring-ops web-conduit setup",
            "infring-ops web-conduit setup --provider=tavily --api-key-env=TAVILY_API_KEY --apply=1",
            "infring-ops web-conduit setup --provider=exa --api-key-env=EXA_API_KEY --apply=1",
            "infring-ops web-conduit setup --provider=brave --api-key-env=BRAVE_SEARCH_API_KEY --apply=1",
            "infring-ops web-conduit setup --provider=serperdev --api-key-env=SERPER_API_KEY --apply=1",
            "infring-ops web-conduit setup --provider=duckduckgo --apply=1"
        ],
        "provider_options": options
    })
}

fn setup_selected_provider(root: &Path, policy: &Value, provider_hint: &str) -> Result<String, String> {
    let cleaned = clean_text(provider_hint, 60);
    if cleaned.is_empty() {
        return default_setup_provider(root, policy)
            .ok_or_else(|| "no_search_providers_available".to_string());
    }
    if let Some(invalid) = validate_explicit_provider_hint(&cleaned) {
        return Err(format!("unsupported_search_provider:{invalid}"));
    }
    provider_chain_from_request(&cleaned, &json!({}), policy)
        .into_iter()
        .next()
        .ok_or_else(|| "no_search_providers_available".to_string())
}

fn apply_search_setup_policy(
    root: &Path,
    policy: &mut Value,
    provider: &str,
    request: &Value,
) -> Vec<String> {
    let mut changes = Vec::<String>::new();
    let mut order = vec![provider.to_string()];
    order.extend(normalized_search_policy_order(root, policy));
    let order = dedupe_strings(order);
    set_search_provider_order(policy, &order);
    changes.push(format!("Selected web search provider \"{provider}\"."));

    let api_key = clean_text(
        request
            .get("api_key")
            .or_else(|| request.get("apiKey"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        600,
    );
    let api_key_env = clean_text(
        request
            .get("api_key_env")
            .or_else(|| request.get("apiKeyEnv"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        160,
    );
    let requires_credential = provider_requires_credential(provider, WebProviderFamily::Search);
    if requires_credential {
        if !api_key.is_empty() {
            let entry = ensure_search_provider_config_entry(policy, provider);
            ensure_object_value(entry).insert("api_key".to_string(), Value::String(api_key));
            changes.push(format!(
                "Configured inline API key at /web_conduit/search_provider_config/{provider}/api_key."
            ));
        }
        if !api_key_env.is_empty() {
            let entry = ensure_search_provider_config_entry(policy, provider);
            ensure_object_value(entry)
                .insert("api_key_env".to_string(), Value::String(api_key_env.clone()));
            changes.push(format!(
                "Configured env-backed API key at /web_conduit/search_provider_config/{provider}/api_key_env."
            ));
        }
    } else if !api_key.is_empty() || !api_key_env.is_empty() {
        changes.push(format!(
            "Ignored credential input because provider \"{provider}\" is key-free."
        ));
    }
    changes
}

pub fn api_setup(root: &Path, request: &Value) -> Value {
    let (policy, policy_path_value) = load_policy(root);
    let summary_only = request
        .get("summary_only")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let contract = web_setup_contract(root, &policy);
    let provider_hint = clean_text(
        request
            .get("provider")
            .or_else(|| request.get("search_provider"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        60,
    );
    if provider_hint.is_empty() {
        return json!({
            "ok": true,
            "type": "web_conduit_setup",
            "policy_path": policy_path_value.to_string_lossy().to_string(),
            "enabled": policy.pointer("/web_conduit/enabled").and_then(Value::as_bool).unwrap_or(true),
            "setup_contract": contract
        });
    }
    let selected_provider = match setup_selected_provider(root, &policy, &provider_hint) {
        Ok(provider) => provider,
        Err(err) => {
            return json!({
                "ok": false,
                "type": "web_conduit_setup",
                "error": err,
                "provider": provider_hint,
                "setup_contract": contract
            });
        }
    };
    let apply = request.get("apply").and_then(Value::as_bool).unwrap_or(false);
    let mut next_policy = policy.clone();
    let changes = apply_search_setup_policy(root, &mut next_policy, &selected_provider, request);
    let selected_requires_credential =
        provider_requires_credential(&selected_provider, WebProviderFamily::Search);
    let ready = !selected_requires_credential
        || resolve_search_provider_credential(root, &next_policy, &selected_provider).is_some();
    if apply {
        if let Err(err) = write_json_atomic(&policy_path_value, &next_policy) {
            return json!({
                "ok": false,
                "type": "web_conduit_setup",
                "error": err,
                "provider": selected_provider,
                "changes": changes,
                "setup_contract": contract
            });
        }
    }
    let mut payload = json!({
        "ok": true,
        "type": "web_conduit_setup",
        "provider": selected_provider,
        "applied": apply,
        "ready": ready,
        "changes": changes,
        "policy_path": policy_path_value.to_string_lossy().to_string(),
        "setup_contract": contract
    });
    if !summary_only {
        payload["next_policy"] = next_policy;
    }
    payload
}
