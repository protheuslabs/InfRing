
fn selected_provider_key_source(
    policy: &Value,
    provider: Option<&str>,
    family: WebProviderFamily,
) -> Value {
    let Some(provider_id) = provider else {
        return Value::Null;
    };
    let raw = match family {
        WebProviderFamily::Search => {
            resolve_provider_credential_source_with_env(policy, provider_id, family, |key| {
                std::env::var(key).ok()
            })
        }
        WebProviderFamily::Fetch => "not_required".to_string(),
    };
    let normalized = match raw.as_str() {
        "policy_inline" => "config",
        "policy_secret_ref" => "secret_broker",
        "policy_env" | "env" => "env",
        "not_required" => "not_required",
        _ => "missing",
    };
    Value::String(normalized.to_string())
}
