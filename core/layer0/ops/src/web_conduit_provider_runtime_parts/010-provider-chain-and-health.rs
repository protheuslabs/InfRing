// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)
// Web provider runtime: chain selection + provider health + local search/fetch provider catalogs.

use chrono::Utc;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

const PROVIDER_HEALTH_REL: &str = "client/runtime/local/state/web_conduit/provider_health.json";
const SEARCH_CACHE_REL: &str = "client/runtime/local/state/web_conduit/search_cache.json";
const SEARCH_CACHE_MAX_ENTRIES: usize = 256;
const SEARCH_CACHE_TTL_SUCCESS_SECS: i64 = 8 * 60;
const SEARCH_CACHE_TTL_NO_RESULTS_SECS: i64 = 90;

const DEFAULT_SEARCH_PROVIDER_CHAIN: &[&str] = &[
    "tavily",
    "exa",
    "brave",
    "serperdev",
    "google_news_rss",
    "bing_rss",
    "duckduckgo_lite",
    "duckduckgo",
];
const DEFAULT_FETCH_PROVIDER_CHAIN: &[&str] = &["direct_http"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WebProviderFamily {
    Search,
    Fetch,
}

#[derive(Debug, Clone, Copy)]
struct WebProviderDescriptor {
    family: WebProviderFamily,
    provider: &'static str,
    aliases: &'static [&'static str],
    source_kind: &'static str,
    env_keys: &'static [&'static str],
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct CircuitPolicy {
    pub enabled: bool,
    pub failure_threshold: u64,
    pub open_for_secs: i64,
}

fn clean_text(raw: &str, max_len: usize) -> String {
    raw.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max_len.max(1))
        .collect::<String>()
}

fn runtime_state_path(root: &Path, rel: &str) -> PathBuf {
    root.join(rel)
}

fn builtin_provider_descriptors(family: WebProviderFamily) -> &'static [WebProviderDescriptor] {
    const SEARCH: &[WebProviderDescriptor] = &[
        WebProviderDescriptor { family: WebProviderFamily::Search, provider: "tavily", aliases: &["tavily", "tavily_search", "tvly"], source_kind: "structured_api", env_keys: &["INFRING_TAVILY_API_KEY", "TAVILY_API_KEY"] },
        WebProviderDescriptor { family: WebProviderFamily::Search, provider: "exa", aliases: &["exa", "exa_search", "exaai", "exa_ai"], source_kind: "structured_api", env_keys: &["INFRING_EXA_API_KEY", "EXA_API_KEY"] },
        WebProviderDescriptor { family: WebProviderFamily::Search, provider: "brave", aliases: &["brave", "brave_search", "brave-search"], source_kind: "structured_api", env_keys: &["INFRING_BRAVE_SEARCH_API_KEY", "BRAVE_SEARCH_API_KEY", "BRAVE_API_KEY"] },
        WebProviderDescriptor { family: WebProviderFamily::Search, provider: "serperdev", aliases: &["serper", "serperdev"], source_kind: "structured_api", env_keys: &["INFRING_SERPERDEV_API_KEY", "SERPERDEV_API_KEY", "INFRING_SERPER_API_KEY", "SERPER_API_KEY"] },
        WebProviderDescriptor { family: WebProviderFamily::Search, provider: "browser_serp", aliases: &["browser_serp", "browser-serp", "browser_search", "browser-search", "serp_browser", "serp-browser"], source_kind: "browser_search", env_keys: &[] },
        WebProviderDescriptor { family: WebProviderFamily::Search, provider: "duckduckgo", aliases: &["duckduckgo", "ddg"], source_kind: "html_search", env_keys: &[] },
        WebProviderDescriptor { family: WebProviderFamily::Search, provider: "duckduckgo_lite", aliases: &["duckduckgo_lite", "ddg_lite", "duckduckgo-lite", "ddg-lite", "lite"], source_kind: "html_search", env_keys: &[] },
        WebProviderDescriptor { family: WebProviderFamily::Search, provider: "google_news_rss", aliases: &["google_news", "google-news", "google_news_rss", "google-news-rss", "news_rss", "news-rss", "gnews"], source_kind: "news_rss_feed", env_keys: &[] },
        WebProviderDescriptor { family: WebProviderFamily::Search, provider: "bing_rss", aliases: &["bing", "bing_rss"], source_kind: "rss_feed", env_keys: &[] },
    ];
    const FETCH: &[WebProviderDescriptor] = &[WebProviderDescriptor { family: WebProviderFamily::Fetch, provider: "direct_http", aliases: &["direct_http", "direct-http", "curl", "http", "fetch"], source_kind: "http_get", env_keys: &[] }];
    match family {
        WebProviderFamily::Search => SEARCH,
        WebProviderFamily::Fetch => FETCH,
    }
}

fn provider_family_name(family: WebProviderFamily) -> &'static str {
    match family {
        WebProviderFamily::Search => "search",
        WebProviderFamily::Fetch => "fetch",
    }
}

fn default_provider_chain_vec(family: WebProviderFamily) -> Vec<String> {
    let defaults = match family { WebProviderFamily::Search => DEFAULT_SEARCH_PROVIDER_CHAIN, WebProviderFamily::Fetch => DEFAULT_FETCH_PROVIDER_CHAIN };
    defaults.iter().map(|row| row.to_string()).collect::<Vec<_>>()
}

fn default_provider_health_state() -> Value {
    json!({"version": 1, "providers": {}})
}

fn default_search_cache_state() -> Value {
    json!({"version": 1, "entries": {}})
}

fn read_json_or(path: &Path, fallback: Value) -> Value {
    match fs::read_to_string(path) {
        Ok(raw) => serde_json::from_str::<Value>(&raw).unwrap_or(fallback),
        Err(_) => fallback,
    }
}

fn write_json_atomic(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("web_conduit_runtime_create_parent_failed:{err}"))?;
    }
    let tmp = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        Utc::now().timestamp_millis()
    ));
    let encoded = serde_json::to_vec_pretty(value)
        .map_err(|err| format!("web_conduit_runtime_encode_failed:{err}"))?;
    fs::write(&tmp, encoded)
        .map_err(|err| format!("web_conduit_runtime_tmp_write_failed:{err}"))?;
    fs::rename(&tmp, path).map_err(|err| format!("web_conduit_runtime_rename_failed:{err}"))?;
    Ok(())
}

fn normalize_provider_token_for_family(raw: &str, family: WebProviderFamily) -> Option<String> {
    let lowered = clean_text(raw, 60).to_ascii_lowercase();
    for descriptor in builtin_provider_descriptors(family) {
        if descriptor.provider == lowered || descriptor.aliases.iter().any(|alias| *alias == lowered) {
            return Some(descriptor.provider.to_string());
        }
    }
    None
}

fn normalize_provider_token(raw: &str) -> Option<String> {
    normalize_provider_token_for_family(raw, WebProviderFamily::Search)
}

fn provider_descriptor(
    provider: &str,
    family: WebProviderFamily,
) -> Option<&'static WebProviderDescriptor> {
    let provider_id = normalize_provider_token_for_family(provider, family)?;
    builtin_provider_descriptors(family)
        .iter()
        .find(|descriptor| descriptor.provider == provider_id)
}

fn provider_env_keys(provider: &str, family: WebProviderFamily) -> &'static [&'static str] {
    provider_descriptor(provider, family)
        .map(|descriptor| descriptor.env_keys)
        .unwrap_or(&[])
}

pub(crate) fn provider_requires_credential(provider: &str, family: WebProviderFamily) -> bool {
    !provider_env_keys(provider, family).is_empty()
}

fn provider_aliases(provider: &str, family: WebProviderFamily) -> &'static [&'static str] {
    provider_descriptor(provider, family)
        .map(|descriptor| descriptor.aliases)
        .unwrap_or(&[])
}

fn provider_source_kind(provider: &str, family: WebProviderFamily) -> &'static str {
    provider_descriptor(provider, family)
        .map(|descriptor| descriptor.source_kind)
        .unwrap_or("unknown")
}

fn provider_has_runtime_credential_with<F>(
    provider: &str,
    family: WebProviderFamily,
    resolve_env: F,
) -> bool
where
    F: Fn(&str) -> Option<String>,
{
    let keys = provider_env_keys(provider, family);
    if keys.is_empty() {
        return true;
    }
    keys.iter().any(|key| {
        resolve_env(key)
            .map(|raw| !clean_text(&raw, 600).is_empty())
            .unwrap_or(false)
    })
}

fn parse_provider_list_for_family(raw: &Value, family: WebProviderFamily) -> Vec<String> {
    let rows = if let Some(array) = raw.as_array() {
        array
            .iter()
            .filter_map(|row| row.as_str().map(ToString::to_string))
            .collect::<Vec<_>>()
    } else if let Some(single) = raw.as_str() {
        single
            .split(|ch: char| ch == ',' || ch.is_ascii_whitespace())
            .map(str::trim)
            .filter(|row| !row.is_empty())
            .map(ToString::to_string)
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    rows.into_iter()
        .filter_map(|row| normalize_provider_token_for_family(&row, family))
        .collect::<Vec<_>>()
}

pub(crate) fn request_prefers_runtime_provider(request: &Value) -> bool {
    request
        .get("prefer_runtime_provider")
        .and_then(Value::as_bool)
        .or_else(|| {
            request
                .get("prefer_runtime_providers")
                .and_then(Value::as_bool)
        })
        .or_else(|| request.get("preferRuntimeProvider").and_then(Value::as_bool))
        .or_else(|| request.get("preferRuntimeProviders").and_then(Value::as_bool))
        .or_else(|| {
            request
                .pointer("/runtime_web_search/prefer_runtime_provider")
                .and_then(Value::as_bool)
        })
        .or_else(|| {
            request
                .pointer("/runtime_web_search/prefer_runtime_providers")
                .and_then(Value::as_bool)
        })
        .or_else(|| {
            request
                .pointer("/runtimeWebSearch/preferRuntimeProvider")
                .and_then(Value::as_bool)
        })
        .or_else(|| {
            request
                .pointer("/runtimeWebSearch/preferRuntimeProviders")
                .and_then(Value::as_bool)
        })
        .or_else(|| {
            request
                .pointer("/runtime_web_fetch/prefer_runtime_provider")
                .and_then(Value::as_bool)
        })
        .or_else(|| {
            request
                .pointer("/runtimeWebFetch/preferRuntimeProvider")
                .and_then(Value::as_bool)
        })
        .unwrap_or(false)
}

fn request_provider_chain_value<'a>(
    request: &'a Value,
    family: WebProviderFamily,
) -> Option<&'a Value> {
    match family {
        WebProviderFamily::Search => request
            .get("provider_chain")
            .or_else(|| request.get("search_provider_chain"))
            .or_else(|| request.get("providerChain"))
            .or_else(|| request.get("searchProviderChain")),
        WebProviderFamily::Fetch => request
            .get("fetch_provider_chain")
            .or_else(|| request.get("provider_chain"))
            .or_else(|| request.get("fetchProviderChain"))
            .or_else(|| request.get("providerChain")),
    }
}

fn request_provider_chain_is_strict(request: &Value, family: WebProviderFamily) -> bool {
    let raw = match family {
        WebProviderFamily::Search => request
            .get("provider_chain_strict")
            .or_else(|| request.get("search_provider_chain_strict"))
            .or_else(|| request.get("providerChainStrict"))
            .or_else(|| request.get("searchProviderChainStrict")),
        WebProviderFamily::Fetch => request
            .get("provider_chain_strict")
            .or_else(|| request.get("fetch_provider_chain_strict"))
            .or_else(|| request.get("providerChainStrict"))
            .or_else(|| request.get("fetchProviderChainStrict")),
    };
    raw.and_then(|value| {
        value
            .as_bool()
            .or_else(|| value.as_str().map(|raw| matches!(raw.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on")))
    })
    .unwrap_or(false)
}

pub(crate) fn request_provider_chain_for_family(
    request: &Value,
    family: WebProviderFamily,
) -> Vec<String> {
    request_provider_chain_value(request, family)
        .map(|raw| parse_provider_list_for_family(raw, family))
        .unwrap_or_default()
}

pub(crate) fn runtime_selected_provider_from_request(
    request: &Value,
    family: WebProviderFamily,
) -> Option<String> {
    let raw = match family {
        WebProviderFamily::Search => request
            .pointer("/runtime_web_search/selected_provider")
            .or_else(|| request.pointer("/runtime_web_search/provider_configured"))
            .or_else(|| request.pointer("/runtimeWebSearch/selectedProvider"))
            .or_else(|| request.pointer("/runtimeWebSearch/providerConfigured"))
            .or_else(|| request.get("runtime_search_provider"))
            .or_else(|| request.get("runtimeSearchProvider"))
            .and_then(Value::as_str),
        WebProviderFamily::Fetch => request
            .pointer("/runtime_web_fetch/selected_provider")
            .or_else(|| request.pointer("/runtime_web_fetch/provider_configured"))
            .or_else(|| request.pointer("/runtimeWebFetch/selectedProvider"))
            .or_else(|| request.pointer("/runtimeWebFetch/providerConfigured"))
            .or_else(|| request.get("runtime_fetch_provider"))
            .or_else(|| request.get("runtimeFetchProvider"))
            .and_then(Value::as_str),
    };
    raw.and_then(|value| normalize_provider_token_for_family(value, family))
}

fn dedupe_preserve(rows: Vec<String>) -> Vec<String> {
    rows.into_iter().fold(Vec::<String>::new(), |mut acc, row| {
        if !acc.iter().any(|existing| existing == &row) {
            acc.push(row);
        }
        acc
    })
}

fn provider_chain_from_request_with_env<F>(
    provider_hint: &str,
    request: &Value,
    policy: &Value,
    resolve_env: F,
) -> Vec<String>
where
    F: Fn(&str) -> Option<String> + Copy,
{
    let hint = clean_text(provider_hint, 60).to_ascii_lowercase();
    let request_chain = request_provider_chain_for_family(request, WebProviderFamily::Search);
    let request_chain_explicit = !request_chain.is_empty();
    let strict_request_chain =
        request_chain_explicit && request_provider_chain_is_strict(request, WebProviderFamily::Search);
    let runtime_selected_provider =
        runtime_selected_provider_from_request(request, WebProviderFamily::Search);
    let prefer_runtime_provider =
        request_prefers_runtime_provider(request) || runtime_selected_provider.is_some();
    let policy_chain = policy
        .pointer("/web_conduit/search_provider_order")
        .or_else(|| policy.get("search_provider_order"))
        .map(|raw| parse_provider_list_for_family(raw, WebProviderFamily::Search))
        .unwrap_or_default();
    let configured = if request_chain.is_empty() {
        policy_chain
    } else {
        request_chain.clone()
    };
    let configured = if configured.is_empty() { default_provider_chain_vec(WebProviderFamily::Search) } else { configured };

    let mut prefix = Vec::<String>::new();
    match hint.as_str() {
        "bing" | "bing_rss" => return vec!["bing_rss".to_string()],
        "google_news" | "google-news" | "google_news_rss" | "google-news-rss" | "news_rss" | "news-rss" | "gnews" => return vec!["google_news_rss".to_string()],
        "duckduckgo" | "ddg" => prefix.extend(["duckduckgo", "duckduckgo_lite", "bing_rss"].into_iter().map(str::to_string)),
        "duckduckgo_lite" | "ddg_lite" | "duckduckgo-lite" | "ddg-lite" | "lite" => {
            prefix.extend(["duckduckgo_lite", "duckduckgo", "bing_rss"].into_iter().map(str::to_string))
        }
        "tavily" | "tavily_search" | "tvly" => prefix.push("tavily".to_string()),
        "exa" | "exa_search" | "exaai" | "exa_ai" => prefix.push("exa".to_string()),
        "brave" | "brave_search" | "brave-search" => prefix.push("brave".to_string()),
        "serper" | "serperdev" => prefix.push("serperdev".to_string()),
        "browser_serp" | "browser-serp" | "browser_search" | "browser-search"
        | "serp_browser" | "serp-browser" => prefix.push("browser_serp".to_string()),
        _ => {}
    }
    let hint_explicit = matches!(
        hint.as_str(),
        "bing" | "bing_rss" | "google_news" | "google-news" | "google_news_rss"
            | "google-news-rss" | "news_rss" | "news-rss" | "gnews"
            | "duckduckgo" | "ddg" | "tavily" | "tavily_search" | "tvly"
            | "duckduckgo_lite" | "ddg_lite" | "duckduckgo-lite" | "ddg-lite" | "lite"
            | "exa" | "exa_search" | "exaai" | "exa_ai" | "brave" | "brave_search"
            | "brave-search" | "serper" | "serperdev" | "browser_serp"
            | "browser-serp" | "browser_search" | "browser-search" | "serp_browser"
            | "serp-browser"
    );
    let mut merged = prefix;
    if prefer_runtime_provider && !hint_explicit {
        if let Some(runtime_provider) = runtime_selected_provider {
            merged.push(runtime_provider);
        }
    }
    merged.extend(configured);
    merged.extend(default_provider_chain_vec(WebProviderFamily::Search));
    let deduped = dedupe_preserve(merged);
    if hint_explicit || ((request_chain_explicit && !prefer_runtime_provider) && strict_request_chain) {
        return if strict_request_chain {
            dedupe_preserve(request_chain)
        } else {
            deduped
        };
    }
    if request_chain_explicit && !prefer_runtime_provider {
        return deduped;
    }
    let mut credential_ready = Vec::<String>::new();
    let mut missing_credential = Vec::<String>::new();
    for provider in deduped {
        if provider_has_configured_secret_ref(policy, &provider, WebProviderFamily::Search)
            || provider_has_runtime_credential_with(&provider, WebProviderFamily::Search, resolve_env)
        {
            credential_ready.push(provider);
        } else {
            missing_credential.push(provider);
        }
    }
    credential_ready.extend(missing_credential);
    credential_ready
}

fn fetch_provider_chain_from_request_with_env<F>(
    provider_hint: &str,
    request: &Value,
    policy: &Value,
    resolve_env: F,
) -> Vec<String>
where
    F: Fn(&str) -> Option<String> + Copy,
{
    let explicit = normalize_provider_token_for_family(provider_hint, WebProviderFamily::Fetch);
    let request_chain = request_provider_chain_for_family(request, WebProviderFamily::Fetch);
    let runtime_selected_provider =
        runtime_selected_provider_from_request(request, WebProviderFamily::Fetch);
    let prefer_runtime_provider =
        request_prefers_runtime_provider(request) || runtime_selected_provider.is_some();
    let policy_chain = policy
        .pointer("/web_conduit/fetch_provider_order")
        .or_else(|| policy.get("fetch_provider_order"))
        .map(|raw| parse_provider_list_for_family(raw, WebProviderFamily::Fetch))
        .unwrap_or_default();
    let configured = if request_chain.is_empty() { policy_chain } else { request_chain };
    let configured = if configured.is_empty() { default_provider_chain_vec(WebProviderFamily::Fetch) } else { configured };

    let mut merged = Vec::<String>::new();
    if let Some(provider) = explicit {
        merged.push(provider);
    } else if prefer_runtime_provider {
        if let Some(runtime_provider) = runtime_selected_provider {
            merged.push(runtime_provider);
        }
    }
    merged.extend(configured);
    merged.extend(default_provider_chain_vec(WebProviderFamily::Fetch));
    let deduped = dedupe_preserve(merged);
    let mut credential_ready = Vec::<String>::new();
    let mut missing_credential = Vec::<String>::new();
    for provider in deduped {
        if provider_has_configured_secret_ref(policy, &provider, WebProviderFamily::Fetch)
            || provider_has_runtime_credential_with(&provider, WebProviderFamily::Fetch, resolve_env)
        {
            credential_ready.push(provider);
        } else {
            missing_credential.push(provider);
        }
    }
    credential_ready.extend(missing_credential);
    credential_ready
}

pub(crate) fn provider_chain_from_request(
    provider_hint: &str,
    request: &Value,
    policy: &Value,
) -> Vec<String> {
    provider_chain_from_request_with_env(provider_hint, request, policy, |key| std::env::var(key).ok())
}

pub(crate) fn fetch_provider_chain_from_request(
    provider_hint: &str,
    request: &Value,
    policy: &Value,
) -> Vec<String> {
    fetch_provider_chain_from_request_with_env(provider_hint, request, policy, |key| std::env::var(key).ok())
}

fn validate_explicit_provider_hint_for_family(
    provider_hint: &str,
    family: WebProviderFamily,
) -> Option<String> {
    let trimmed = clean_text(provider_hint, 60).to_ascii_lowercase();
    if trimmed.is_empty() || trimmed == "auto" {
        return None;
    }
    if normalize_provider_token_for_family(&trimmed, family).is_some() {
        None
    } else {
        Some(trimmed)
    }
}

pub(crate) fn validate_explicit_provider_hint(provider_hint: &str) -> Option<String> {
    validate_explicit_provider_hint_for_family(provider_hint, WebProviderFamily::Search)
}

pub(crate) fn validate_explicit_fetch_provider_hint(provider_hint: &str) -> Option<String> {
    validate_explicit_provider_hint_for_family(provider_hint, WebProviderFamily::Fetch)
}

pub(crate) fn circuit_policy(policy: &Value) -> CircuitPolicy {
    let scope = policy.pointer("/web_conduit/provider_circuit_breaker").or_else(|| policy.get("provider_circuit_breaker")).cloned().unwrap_or_else(|| json!({}));
    let enabled = scope.get("enabled").and_then(Value::as_bool).unwrap_or(true);
    let failure_threshold = scope.get("failure_threshold").and_then(Value::as_u64).unwrap_or(3).clamp(1, 12);
    let open_for_secs = scope.get("open_for_secs").and_then(Value::as_i64).unwrap_or(5 * 60).clamp(30, 4 * 60 * 60);
    CircuitPolicy { enabled, failure_threshold, open_for_secs }
}

fn provider_health_path(root: &Path) -> PathBuf {
    runtime_state_path(root, PROVIDER_HEALTH_REL)
}

fn load_provider_health(root: &Path) -> Value {
    read_json_or(&provider_health_path(root), default_provider_health_state())
}

fn write_provider_health(root: &Path, state: &Value) {
    let _ = write_json_atomic(&provider_health_path(root), state);
}

pub(crate) fn provider_circuit_open_until(
    root: &Path,
    provider: &str,
    policy: &Value,
) -> Option<i64> {
    let breaker = circuit_policy(policy);
    if !breaker.enabled {
        return None;
    }
    let now_ts = Utc::now().timestamp();
    let provider_id = normalize_provider_token(provider)?;
    let mut state = load_provider_health(root);
    let open_until = state
        .pointer(&format!("/providers/{provider_id}/circuit_open_until"))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let last_error = state
        .pointer(&format!("/providers/{provider_id}/last_error"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let last_failure_class = state
        .pointer(&format!("/providers/{provider_id}/last_failure_class"))
        .and_then(Value::as_str)
        .unwrap_or("");
    if open_until > 0
        && (last_failure_class == "query_quality"
            || last_failure_class == "configuration"
            || provider_error_is_query_quality_failure(last_error)
            || provider_error_is_configuration_failure(last_error))
    {
        let cleared_failure_class = if last_failure_class == "configuration"
            || provider_error_is_configuration_failure(last_error)
        {
            "configuration"
        } else {
            "query_quality"
        };
        if let Some(obj) = state
            .get_mut("providers")
            .and_then(Value::as_object_mut)
            .and_then(|providers| providers.get_mut(&provider_id))
            .and_then(Value::as_object_mut)
        {
            obj.insert("consecutive_failures".to_string(), json!(0));
            obj.insert("circuit_open_until".to_string(), json!(0));
            obj.insert(
                "last_failure_class".to_string(),
                json!(cleared_failure_class),
            );
        }
        write_provider_health(root, &state);
        return None;
    }
    if open_until > now_ts {
        return Some(open_until);
    }
    if open_until > 0 {
        if let Some(obj) = state
            .get_mut("providers")
            .and_then(Value::as_object_mut)
            .and_then(|providers| providers.get_mut(&provider_id))
            .and_then(Value::as_object_mut)
        {
            obj.insert("circuit_open_until".to_string(), json!(0));
        }
        write_provider_health(root, &state);
    }
    None
}

pub(crate) fn record_provider_attempt(
    root: &Path,
    provider: &str,
    success: bool,
    error: &str,
    policy: &Value,
) {
    let provider_id = match normalize_provider_token(provider) {
        Some(value) => value,
        None => return,
    };
    let breaker = circuit_policy(policy);
    let now = crate::now_iso();
    let now_ts = Utc::now().timestamp();
    let mut state = load_provider_health(root);
    let providers = state
        .get_mut("providers")
        .and_then(Value::as_object_mut)
        .cloned()
        .unwrap_or_default();
    let mut providers = providers;
    let mut row = providers
        .get(&provider_id)
        .cloned()
        .unwrap_or_else(|| json!({}));
    if !row.is_object() {
        row = json!({});
    }
    let mut failures = row
        .get("consecutive_failures")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if success {
        if let Some(obj) = row.as_object_mut() {
            obj.insert("consecutive_failures".to_string(), json!(0));
            obj.insert("circuit_open_until".to_string(), json!(0));
            obj.insert("last_success_at".to_string(), json!(now));
            obj.insert("last_error".to_string(), Value::String(String::new()));
            obj.insert("last_failure_class".to_string(), Value::String(String::new()));
        }
    } else {
        let failure_class = provider_failure_class(error);
        if failure_class == "query_quality" || failure_class == "configuration" {
            if provider_error_is_query_quality_failure(
                row.get("last_error").and_then(Value::as_str).unwrap_or(""),
            ) || provider_error_is_configuration_failure(
                row.get("last_error").and_then(Value::as_str).unwrap_or(""),
            ) || row
                .get("last_failure_class")
                .and_then(Value::as_str)
                .unwrap_or("")
                == failure_class
            {
                failures = 0;
            }
            if let Some(obj) = row.as_object_mut() {
                obj.insert("consecutive_failures".to_string(), json!(failures));
                if failures == 0 {
                    obj.insert("circuit_open_until".to_string(), json!(0));
                }
                if failure_class == "query_quality" {
                    obj.insert("last_query_quality_at".to_string(), json!(now));
                    obj.insert(
                        "last_query_quality_error".to_string(),
                        json!(clean_text(error, 280)),
                    );
                } else {
                    obj.insert("last_configuration_error_at".to_string(), json!(now));
                    obj.insert(
                        "last_configuration_error".to_string(),
                        json!(clean_text(error, 280)),
                    );
                }
                obj.insert("last_error".to_string(), json!(clean_text(error, 280)));
                obj.insert("last_failure_class".to_string(), json!(failure_class));
            }
            providers.insert(provider_id, row);
            state["version"] = json!(1);
            state["providers"] = Value::Object(providers);
            write_provider_health(root, &state);
            return;
        }
        failures = failures.saturating_add(1);
        let mut open_until = row
            .get("circuit_open_until")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        if breaker.enabled && (failures >= breaker.failure_threshold || failure_class == "access_or_throttle") {
            open_until = now_ts + breaker.open_for_secs;
        }
        if let Some(obj) = row.as_object_mut() {
            obj.insert("consecutive_failures".to_string(), json!(failures));
            obj.insert("circuit_open_until".to_string(), json!(open_until.max(0)));
            obj.insert("last_failure_at".to_string(), json!(now));
            obj.insert("last_error".to_string(), json!(clean_text(error, 280)));
            obj.insert("last_failure_class".to_string(), json!(failure_class));
        }
    }
    providers.insert(provider_id, row);
    state["version"] = json!(1);
    state["providers"] = Value::Object(providers);
    write_provider_health(root, &state);
}

fn provider_error_is_configuration_failure(error: &str) -> bool {
    let lowered = clean_text(error, 320).to_ascii_lowercase();
    [
        "api_key_missing",
        "api key missing",
        "credential_missing",
        "credential_unresolved",
        "credential unresolved",
        "key_unresolved",
        "missing credential",
        "missing api key",
        "web_conduit_policy_denied",
        "policy_denied",
        "policy_blocked",
        "policy blocked",
        "provider_network_policy_blocked",
        "not admitted by policy",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn provider_error_is_query_quality_failure(error: &str) -> bool {
    let lowered = clean_text(error, 320).to_ascii_lowercase();
    [
        "query_result_mismatch",
        "low_signal_search_payload",
        "low-signal search payload",
        "no_usable_summary",
        "no usable summary",
        "search_providers_exhausted",
        "no_relevant_results",
        "no relevant results",
        "low_relevance",
        "low relevance",
        "no_results",
        "no results",
        "off-topic results",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn provider_failure_class(error: &str) -> &'static str {
    let lowered = clean_text(error, 320).to_ascii_lowercase();
    if provider_error_is_configuration_failure(&lowered) {
        return "configuration";
    }
    if provider_error_is_query_quality_failure(&lowered) {
        return "query_quality";
    }
    if [
        "429",
        "rate limit",
        "rate_limited",
        "too many requests",
        "retry-after",
        "captcha",
        "cloudflare",
        "verify you are human",
        "checking your browser",
        "bot wall",
        "anti_bot",
        "anti-bot",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
    {
        "access_or_throttle"
    } else if lowered.contains("timeout") || lowered.contains("timed out") {
        "timeout"
    } else {
        "provider_failure"
    }
}
