const SEARCH_PUBLIC_ARTIFACT_CANDIDATES: &[&str] = &[
    "web-search-contract-api.js",
    "web-search-provider.js",
    "web-search.js",
];
const FETCH_PUBLIC_ARTIFACT_CANDIDATES: &[&str] = &[
    "web-fetch-contract-api.js",
    "web-fetch-provider.js",
    "web-fetch.js",
];

fn fetch_provider_request_contract() -> Value {
    json!({
        "extract_modes": ["text", "markdown"],
        "supports_summary_only": true,
        "supports_timeout_ms": true,
        "supports_cache_ttl_minutes": true
    })
}

fn browser_materialization_enabled(policy: &Value) -> bool {
    policy
        .pointer("/web_conduit/browser_materialization/enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn browser_materialization_request_contract(policy: &Value) -> Value {
    let config = policy
        .pointer("/web_conduit/browser_materialization")
        .cloned()
        .unwrap_or_else(|| json!({}));
    json!({
        "capability_id": "browser_materialize_page",
        "enabled": browser_materialization_enabled(policy),
        "permission_class": config
            .get("permission_class")
            .and_then(Value::as_str)
            .unwrap_or("public_web_dynamic_read"),
        "requires_explicit_admission": config
            .get("requires_explicit_admission")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        "input_contract": {
            "required_fields": config
                .pointer("/request_contract/required_fields")
                .cloned()
                .unwrap_or_else(|| json!(["url", "admission_ref"])),
            "optional_fields": config
                .pointer("/request_contract/optional_fields")
                .cloned()
                .unwrap_or_else(|| json!([
                    "request_id",
                    "extract_mode",
                    "wait_until",
                    "wait_for_selector",
                    "timeout_ms",
                    "max_response_bytes",
                    "profile_ref",
                    "evidence_gap_reason"
                ])),
            "denied_fields": config
                .pointer("/request_contract/denied_fields")
                .cloned()
                .unwrap_or_else(|| json!([
                    "browser_args",
                    "launch_args",
                    "cdp_command",
                    "user_script",
                    "proxy",
                    "proxy_url",
                    "proxy_credentials",
                    "session_id",
                    "storage_state",
                    "local_file"
                ])),
            "caller_controlled_browser_args_allowed": false,
            "proxy_fields_allowed": false,
            "persistent_session_fields_allowed": false,
            "raw_cdp_commands_allowed": false,
            "arbitrary_user_scripts_allowed": false
        },
        "profile_contract": config
            .get("profile_contract")
            .cloned()
            .unwrap_or_else(|| json!({
                "profile_source": "tool_cd_policy",
                "default_profile": "stateless_public_materialization",
                "state_scope": "stateless",
                "proxy_capability_required": true,
                "persistent_session_capability_required": true,
                "caller_override_allowed": false,
                "denied_launch_args": [
                    "--remote-debugging-port",
                    "--disable-web-security",
                    "--ignore-certificate-errors",
                    "--allow-file-access-from-files",
                    "--load-extension"
                ],
                "telemetry_fields": [
                    "profile_ref",
                    "state_scope",
                    "effective_profile_hash",
                    "denied_option_count"
                ]
            })),
        "security_contract": config
            .get("security")
            .cloned()
            .unwrap_or_else(|| json!({
                "allowed_schemes": ["http", "https"],
                "reject_url_credentials": true,
                "block_private_network_targets": true,
                "revalidate_final_url_after_navigation": true,
                "reject_caller_supplied_browser_args": true,
                "max_redirects": 8,
                "url_safety_status_values": [
                    "allowed",
                    "scheme_blocked",
                    "private_network_blocked",
                    "redirect_target_blocked",
                    "credentials_redacted",
                    "invalid_url"
                ]
            })),
        "output_contract": config
            .get("output_contract")
            .cloned()
            .unwrap_or_else(|| json!({
                "fields": [
                    "source_url",
                    "final_url",
                    "status_code",
                    "title",
                    "main_text_or_markdown",
                    "links_summary",
                    "blocker_classification",
                    "extraction_confidence",
                    "artifact_ref"
                ],
                "chat_visible": false
            })),
        "evidence_handoff": config
            .get("evidence_handoff")
            .cloned()
            .unwrap_or_else(|| json!({
                "target_lane": "candidate_enrichment",
                "promotion_requires": [
                    "safe_final_url",
                    "substantive_main_text",
                    "query_relevance",
                    "not_blocker_shell"
                ],
                "confidence_values": ["usable", "low_confidence_raw", "rejected"],
                "raw_payload_chat_visible": false
            })),
        "blocker_taxonomy": {
            "classes": [
                "anti_bot_challenge",
                "needs_js",
                "rate_limited",
                "access_denied",
                "provider_degraded",
                "content_materialization_missing",
                "off_intent_noise",
                "low_signal"
            ],
            "decision_authority": "tool_diagnostics_and_policy",
            "chat_visibility": "telemetry_only_until_synthesized"
        },
        "non_goals": [
            "do_not_make_browser_materialization_default_search",
            "do_not_expose_raw_cdp_or_browser_traces_to_chat",
            "do_not_use_proxy_or_persistent_sessions_without_a_separate_admitted_capability"
        ]
    })
}

fn public_artifact_candidates(family: WebProviderFamily) -> &'static [&'static str] {
    match family {
        WebProviderFamily::Search => SEARCH_PUBLIC_ARTIFACT_CANDIDATES,
        WebProviderFamily::Fetch => FETCH_PUBLIC_ARTIFACT_CANDIDATES,
    }
}

fn provider_ids_for_family(family: WebProviderFamily) -> Vec<String> {
    builtin_provider_descriptors(family)
        .iter()
        .map(|descriptor| descriptor.provider.to_string())
        .collect::<Vec<_>>()
}

fn unsupported_provider_examples(family: WebProviderFamily) -> &'static [&'static str] {
    match family {
        WebProviderFamily::Search => &[
            "firecrawl",
            "google",
            "moonshot",
            "perplexity",
            "xai",
        ],
        WebProviderFamily::Fetch => &["firecrawl"],
    }
}

fn public_artifact_contract_for_family(family: WebProviderFamily) -> Value {
    json!({
        "family": provider_family_name(family),
        "runtime_mode": "built_in_only",
        "resolution_mode": "explicit_allowlist",
        "artifact_candidates": public_artifact_candidates(family),
        "allowlisted_provider_ids": provider_ids_for_family(family)
    })
}

fn runtime_resolution_contract(family: WebProviderFamily) -> Value {
    json!({
        "family": provider_family_name(family),
        "runtime_mode": "built_in_only",
        "supports_runtime_registry": false,
        "prefer_runtime_providers": false,
        "configured_provider_fallback": "auto-detect",
        "bundled_provider_precedence": true
    })
}

fn provider_capability_id(family: WebProviderFamily) -> &'static str {
    match family {
        WebProviderFamily::Search => "web.search",
        WebProviderFamily::Fetch => "web.materialize",
    }
}

fn provider_adapter_lane(provider: &str, family: WebProviderFamily) -> &'static str {
    match family {
        WebProviderFamily::Search => match provider {
            "tavily" | "exa" | "brave" | "serperdev" => "api_search_provider",
            "browser_serp" => "browser_serp_provider",
            "google_news_rss" | "bing_rss" => "rss_search_fallback",
            "duckduckgo" | "duckduckgo_lite" => "html_search_fallback",
            _ => "unknown_search_provider",
        },
        WebProviderFamily::Fetch => match provider {
            "direct_http" => "direct_http_materializer",
            _ => "unknown_materialization_provider",
        },
    }
}

fn provider_lifecycle(provider: &str, family: WebProviderFamily) -> &'static str {
    match family {
        WebProviderFamily::Search => match provider {
            "tavily" | "exa" | "brave" | "serperdev" => "admitted_when_credentialed",
            "browser_serp" => "experimental_canary_explicit_only",
            "google_news_rss" | "bing_rss" | "duckduckgo" | "duckduckgo_lite" => {
                "builtin_fallback"
            }
            _ => "unknown",
        },
        WebProviderFamily::Fetch => match provider {
            "direct_http" => "builtin_default",
            _ => "unknown",
        },
    }
}

fn provider_default_chain_recommended(provider: &str, family: WebProviderFamily) -> bool {
    match family {
        WebProviderFamily::Search => provider != "browser_serp",
        WebProviderFamily::Fetch => true,
    }
}

fn provider_capability_contract(provider: &str, family: WebProviderFamily) -> Value {
    json!({
        "version": "web_provider_capability_contract_v1",
        "capability_id": provider_capability_id(family),
        "adapter_lane": provider_adapter_lane(provider, family),
        "provider_lifecycle": provider_lifecycle(provider, family),
        "integration_boundary": "provider_adapter_registry",
        "workflow_direct_vendor_calls_allowed": false,
        "socket_boundary_role": "transport_invocation_only",
        "normalizes_to": match family {
            WebProviderFamily::Search => "web_search_result_v1",
            WebProviderFamily::Fetch => "web_materialized_evidence_v1",
        },
        "default_chain_recommended": provider_default_chain_recommended(provider, family),
        "vendor_specific_fields_scope": "provider_adapter_only",
        "cd_visible_surface": provider_capability_id(family),
    })
}

fn provider_capability_contracts_for_family(family: WebProviderFamily) -> Vec<Value> {
    provider_ids_for_family(family)
        .into_iter()
        .map(|provider| provider_capability_contract(&provider, family))
        .collect::<Vec<_>>()
}

pub(crate) fn web_capability_contract_snapshot(policy: &Value) -> Value {
    json!({
        "version": "web_capability_contracts_v1",
        "principle": "workflows call capability contracts; vendors plug in only as provider adapters",
        "socket_boundary_role": "transport_invocation_only",
        "workflow_direct_vendor_calls_allowed": false,
        "capabilities": {
            "web.search": {
                "integration_point": "search_provider_adapter_registry",
                "workflow_surface": "web.search_or_batch_query",
                "provider_families": ["api_search_provider", "browser_serp_provider", "rss_search_fallback", "html_search_fallback"],
                "default_provider_chain": resolved_search_provider_chain("", &json!({}), policy),
                "provider_contracts": provider_capability_contracts_for_family(WebProviderFamily::Search),
                "output_contract": {
                    "schema": "web_search_result_v1",
                    "required_fields": ["ok", "provider", "query", "results", "provider_raw_count", "provider_filtered_count", "diagnostics"],
                    "result_fields": ["title", "url", "snippet", "published_at", "source_class"]
                },
                "non_goals": [
                    "do_not_call_vendor_apis_from_workflow_json",
                    "do_not_expose_vendor_response_shapes_to_synthesis",
                    "do_not_make_browser_serp_default_until_it_proves_external_organic_urls"
                ]
            },
            "web.materialize": {
                "integration_point": "materialization_provider_adapter_registry",
                "workflow_surface": "web.materialize",
                "provider_families": ["direct_http_materializer", "browser_materializer", "api_scrape_provider"],
                "default_provider_chain": fetch_provider_chain_from_request("", &json!({}), policy),
                "provider_contracts": provider_capability_contracts_for_family(WebProviderFamily::Fetch),
                "output_contract": {
                    "schema": "web_materialized_evidence_v1",
                    "required_fields": ["ok", "provider", "source_url", "final_url", "main_text_or_markdown", "links_summary", "blocker_classification", "diagnostics"]
                },
                "non_goals": [
                    "do_not_use_materialization_as_search_discovery_without_a_serp_extraction_contract",
                    "do_not_promote_search_shell_dom_as_evidence"
                ]
            },
            "web.evidence": {
                "integration_point": "evidence_pack_normalizer",
                "workflow_surface": "normalized_evidence_package",
                "input_sources": ["web.search", "web.materialize"],
                "output_contract": {
                    "schema": "web_evidence_package_v1",
                    "required_fields": ["evidence_refs", "citations", "source_class_coverage", "evidence_pack_quality", "diagnostics"]
                },
                "non_goals": [
                    "do_not_let_low_signal_rows_masquerade_as_citable_evidence",
                    "do_not_grade_synthesis_failure_as_tool_failure_or_tool_failure_as_synthesis_failure"
                ]
            }
        }
    })
}

fn provider_contract_fields_snapshot(provider: &str, family: WebProviderFamily) -> Value {
    match family {
        WebProviderFamily::Search => {
            let credential_path = format!("/web_conduit/search_provider_config/{provider}/api_key");
            let env_path = format!("/web_conduit/search_provider_config/{provider}/api_key_env");
            let env_keys = provider_env_keys(provider, family);
            if env_keys.is_empty() {
                json!({
                    "inactive_secret_paths": [],
                    "credential_contract": {
                        "type": "none"
                    },
                    "configured_credential": Value::Null
                })
            } else {
                json!({
                    "inactive_secret_paths": [credential_path.clone()],
                    "credential_contract": {
                        "type": "top-level",
                        "env_keys": env_keys,
                        "inline_path": credential_path,
                        "env_path": env_path,
                        "secret_id_path": format!("/web_conduit/search_provider_config/{provider}/secret_id"),
                        "secret_ref_path": format!("/web_conduit/search_provider_config/{provider}/secret_ref"),
                        "preferred_instance_store": "secret_broker_encrypted_file"
                    },
                    "configured_credential": {
                        "provider_id": provider,
                        "field": "api_key",
                        "path": format!("/web_conduit/search_provider_config/{provider}/api_key"),
                        "env_path": format!("/web_conduit/search_provider_config/{provider}/api_key_env"),
                        "secret_id_path": format!("/web_conduit/search_provider_config/{provider}/secret_id"),
                        "secret_ref_path": format!("/web_conduit/search_provider_config/{provider}/secret_ref")
                    }
                })
            }
        }
        WebProviderFamily::Fetch => json!({
            "inactive_secret_paths": [],
            "credential_contract": {
                "type": "none"
            },
            "configured_credential": Value::Null
        }),
    }
}

fn provider_registration_contract(policy: &Value, family: WebProviderFamily) -> Value {
    let default_provider_chain = match family {
        WebProviderFamily::Search => resolved_search_provider_chain("", &json!({}), policy),
        WebProviderFamily::Fetch => fetch_provider_chain_from_request("", &json!({}), policy),
    };
    json!({
        "family": provider_family_name(family),
        "selection_policy_path": match family {
            WebProviderFamily::Search => "/web_conduit/search_provider_order",
            WebProviderFamily::Fetch => "/web_conduit/fetch_provider_order",
        },
        "default_provider_chain": default_provider_chain,
        "supported_provider_ids": provider_ids_for_family(family),
        "unsupported_provider_examples": unsupported_provider_examples(family),
        "credential_types_supported": match family {
            WebProviderFamily::Search => json!(["none", "top-level"]),
            WebProviderFamily::Fetch => json!(["none"]),
        },
        "runtime_resolution_contract": runtime_resolution_contract(family),
        "supports_configured_credential": family == WebProviderFamily::Search,
        "scoped_credentials_supported": false,
        "public_artifact_contract": public_artifact_contract_for_family(family)
    })
}

pub(crate) fn search_provider_request_contract(policy: &Value) -> Value {
    json!({
        "default_count": search_default_count(policy),
        "max_count": search_max_count(policy),
        "timeout_ms": search_default_timeout_ms(policy),
        "cache_ttl_minutes": search_default_cache_ttl_minutes(policy),
        "supports_filters": search_filter_support_matrix()
    })
}

pub(crate) fn search_provider_registration_contract(policy: &Value) -> Value {
    provider_registration_contract(policy, WebProviderFamily::Search)
}

pub(crate) fn fetch_provider_registration_contract(policy: &Value) -> Value {
    provider_registration_contract(policy, WebProviderFamily::Fetch)
}

pub(crate) fn web_provider_public_artifact_contracts() -> Value {
    json!({
        "capability_contracts": web_capability_contract_snapshot(&json!({})),
        "search": public_artifact_contract_for_family(WebProviderFamily::Search),
        "fetch": public_artifact_contract_for_family(WebProviderFamily::Fetch),
        "browser_materialization": {
            "family": "browser_materialization",
            "runtime_mode": "optional_capability",
            "resolution_mode": "tool_cd_and_gateway_admission",
            "artifact_candidates": [],
            "allowlisted_provider_ids": ["local_browser"]
        }
    })
}

pub(crate) fn web_tool_catalog_snapshot(policy: &Value) -> Value {
    let search_chain = resolved_search_provider_chain("", &json!({}), policy);
    let fetch_chain = fetch_provider_chain_from_request("", &json!({}), policy);
    let capability_contracts = web_capability_contract_snapshot(policy);
    let browser_provider_chain = policy
        .pointer("/web_conduit/browser_materialization/provider_order")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| clean_text(row, 80))
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>()
        })
        .filter(|rows| !rows.is_empty())
        .unwrap_or_else(|| vec!["local_browser".to_string()]);
    json!([
        {
            "tool": "web_search",
            "label": "Web Search",
            "family": "search",
            "enabled": policy.pointer("/web_conduit/enabled").and_then(Value::as_bool).unwrap_or(true),
            "default_provider": search_chain.first().cloned().unwrap_or_else(|| "none".to_string()),
            "default_provider_chain": search_chain,
            "request_contract": search_provider_request_contract(policy),
            "registration_contract": search_provider_registration_contract(policy),
            "capability_contract": capability_contracts.pointer("/capabilities/web.search").cloned().unwrap_or_else(|| json!({}))
        },
        {
            "tool": "web_fetch",
            "label": "Web Fetch",
            "family": "fetch",
            "enabled": policy.pointer("/web_conduit/enabled").and_then(Value::as_bool).unwrap_or(true),
            "default_provider": fetch_chain.first().cloned().unwrap_or_else(|| "none".to_string()),
            "default_provider_chain": fetch_chain,
            "request_contract": fetch_provider_request_contract(),
            "registration_contract": fetch_provider_registration_contract(policy),
            "capability_contract": capability_contracts.pointer("/capabilities/web.materialize").cloned().unwrap_or_else(|| json!({}))
        },
        {
            "tool": "web_browser_materialize_page",
            "label": "Browser Materialize Page",
            "family": "browser_materialization",
            "enabled": policy.pointer("/web_conduit/enabled").and_then(Value::as_bool).unwrap_or(true)
                && browser_materialization_enabled(policy),
            "optional_capability": true,
            "default_provider": browser_provider_chain.first().cloned().unwrap_or_else(|| "local_browser".to_string()),
            "default_provider_chain": browser_provider_chain,
            "request_contract": browser_materialization_request_contract(policy),
            "registration_contract": {
                "family": "browser_materialization",
                "selection_policy_path": "/web_conduit/browser_materialization/provider_order",
                "supported_provider_ids": ["local_browser"],
                "credential_types_supported": ["none"],
                "runtime_resolution_contract": {
                    "family": "browser_materialization",
                    "runtime_mode": "optional_capability",
                    "supports_runtime_registry": false,
                    "configured_provider_fallback": "none"
                }
            },
            "capability_contract": capability_contracts.pointer("/capabilities/web.materialize").cloned().unwrap_or_else(|| json!({}))
        }
    ])
}

pub(crate) fn provider_health_snapshot(root: &Path, providers: &[String]) -> Value {
    let state = load_provider_health(root);
    let now_ts = Utc::now().timestamp();
    let rows = providers
        .iter()
        .map(|provider| {
            let provider_id = normalize_provider_token(provider).unwrap_or_else(|| provider.clone());
            let entry = state
                .pointer(&format!("/providers/{provider_id}"))
                .cloned()
                .unwrap_or_else(|| json!({}));
            let circuit_open_until = entry
                .get("circuit_open_until")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            json!({
                "provider": provider_id,
                "consecutive_failures": entry.get("consecutive_failures").and_then(Value::as_u64).unwrap_or(0),
                "circuit_open_until": circuit_open_until,
                "circuit_open": circuit_open_until > now_ts,
                "last_success_at": entry.get("last_success_at").cloned().unwrap_or(Value::Null),
                "last_failure_at": entry.get("last_failure_at").cloned().unwrap_or(Value::Null),
                "last_error": clean_text(entry.get("last_error").and_then(Value::as_str).unwrap_or(""), 220)
            })
        })
        .collect::<Vec<_>>();
    json!(rows)
}

fn provider_catalog_snapshot_with_env_family<F>(
    root: &Path,
    policy: &Value,
    family: WebProviderFamily,
    resolve_env: F,
) -> Value
where
    F: Fn(&str) -> Option<String> + Copy,
{
    let state = if family == WebProviderFamily::Search {
        load_provider_health(root)
    } else {
        default_provider_health_state()
    };
    let now_ts = Utc::now().timestamp();
    let chain = match family {
        WebProviderFamily::Search => {
            provider_chain_from_request_with_env("", &json!({}), policy, resolve_env)
        }
        WebProviderFamily::Fetch => {
            fetch_provider_chain_from_request_with_env("", &json!({}), policy, resolve_env)
        }
    };
    let request_contract = if family == WebProviderFamily::Search {
        search_provider_request_contract(policy)
    } else {
        fetch_provider_request_contract()
    };
    let public_artifact_contract = public_artifact_contract_for_family(family);
    let rows = chain
        .iter()
        .enumerate()
        .map(|(index, provider)| {
            let entry = state
                .pointer(&format!("/providers/{provider}"))
                .cloned()
                .unwrap_or_else(|| json!({}));
            let descriptor = provider_descriptor(provider, family);
            let requires_credential = descriptor
                .map(|current| !current.env_keys.is_empty())
                .unwrap_or(false);
            let credential_present = provider_has_configured_secret_ref(policy, provider, family)
                || provider_has_runtime_credential_with(provider, family, resolve_env);
            let circuit_open_until = entry
                .get("circuit_open_until")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            json!({
                "family": provider_family_name(descriptor.map(|current| current.family).unwrap_or(family)),
                "provider": provider,
                "aliases": provider_aliases(provider, family),
                "source": provider_source_kind(provider, family),
                "requires_credential": requires_credential,
                "credential_present": credential_present,
                "credential_env_keys": provider_env_keys(provider, family),
                "credential_source": resolve_provider_credential_source_with_env(policy, provider, family, resolve_env),
                "available": !requires_credential || credential_present,
                "selected_by_default": index == 0,
                "auto_detect_rank": index + 1,
                "consecutive_failures": if family == WebProviderFamily::Search {
                    entry.get("consecutive_failures").and_then(Value::as_u64).unwrap_or(0)
                } else {
                    0
                },
                "circuit_open_until": if family == WebProviderFamily::Search && circuit_open_until > now_ts {
                    circuit_open_until
                } else {
                    0
                },
                "last_error": if family == WebProviderFamily::Search {
                    clean_text(entry.get("last_error").and_then(Value::as_str).unwrap_or(""), 220)
                } else {
                    String::new()
                },
                "request_contract": request_contract.clone(),
                "contract_fields": provider_contract_fields_snapshot(provider, family),
                "capability_contract": provider_capability_contract(provider, family),
                "public_artifact_contract": public_artifact_contract.clone()
            })
        })
        .collect::<Vec<_>>();
    json!(rows)
}

pub(crate) fn provider_catalog_snapshot(root: &Path, policy: &Value) -> Value {
    provider_catalog_snapshot_with_env_family(root, policy, WebProviderFamily::Search, |key| {
        std::env::var(key).ok()
    })
}

pub(crate) fn fetch_provider_catalog_snapshot(root: &Path, policy: &Value) -> Value {
    provider_catalog_snapshot_with_env_family(root, policy, WebProviderFamily::Fetch, |key| {
        std::env::var(key).ok()
    })
}

#[cfg(test)]
mod openclaw_provider_contract_tests {
    use super::*;

    #[test]
    fn provider_catalog_snapshot_reports_public_contract_fields_for_serper() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = json!({
            "web_conduit": {
                "search_provider_order": ["serperdev", "duckduckgo"],
                "search_default_count": 7,
                "search_cache_ttl_minutes": 13
            }
        });
        let catalog = provider_catalog_snapshot_with_env_family(
            tmp.path(),
            &policy,
            WebProviderFamily::Search,
            |key| {
                if key == "SERPER_API_KEY" {
                    Some("test-key".to_string())
                } else {
                    None
                }
            },
        );
        let rows = catalog.as_array().expect("catalog rows");
        let serper = rows
            .iter()
            .find(|row| row.get("provider").and_then(Value::as_str) == Some("serperdev"))
            .expect("serper row");
        assert_eq!(
            serper.get("credential_source").and_then(Value::as_str),
            Some("env")
        );
        assert_eq!(
            serper
                .pointer("/request_contract/default_count")
                .and_then(Value::as_u64),
            Some(7)
        );
        assert_eq!(
            serper
                .pointer("/contract_fields/credential_contract/type")
                .and_then(Value::as_str),
            Some("top-level")
        );
        assert_eq!(
            serper
                .pointer("/contract_fields/configured_credential/path")
                .and_then(Value::as_str),
            Some("/web_conduit/search_provider_config/serperdev/api_key")
        );
        assert_eq!(
            serper
                .pointer("/public_artifact_contract/artifact_candidates/0")
                .and_then(Value::as_str),
            Some("web-search-contract-api.js")
        );
        assert_eq!(
            serper
                .pointer("/capability_contract/capability_id")
                .and_then(Value::as_str),
            Some("web.search")
        );
        assert_eq!(
            serper
                .pointer("/capability_contract/adapter_lane")
                .and_then(Value::as_str),
            Some("api_search_provider")
        );
        assert_eq!(
            serper
                .pointer("/capability_contract/workflow_direct_vendor_calls_allowed")
                .and_then(Value::as_bool),
            Some(false)
        );
    }

    #[test]
    fn browser_serp_is_registered_as_explicit_canary_provider_lane() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = json!({
            "web_conduit": {
                "search_provider_order": ["browser_serp", "bing_rss"]
            }
        });
        let catalog = provider_catalog_snapshot_with_env_family(
            tmp.path(),
            &policy,
            WebProviderFamily::Search,
            |_key| None,
        );
        let rows = catalog.as_array().expect("catalog rows");
        let browser_serp = rows
            .iter()
            .find(|row| row.get("provider").and_then(Value::as_str) == Some("browser_serp"))
            .expect("browser_serp row");
        assert_eq!(
            browser_serp
                .pointer("/capability_contract/adapter_lane")
                .and_then(Value::as_str),
            Some("browser_serp_provider")
        );
        assert_eq!(
            browser_serp
                .pointer("/capability_contract/provider_lifecycle")
                .and_then(Value::as_str),
            Some("experimental_canary_explicit_only")
        );
        assert_eq!(
            browser_serp
                .pointer("/capability_contract/default_chain_recommended")
                .and_then(Value::as_bool),
            Some(false)
        );
    }

    #[test]
    fn fetch_provider_catalog_snapshot_reports_public_artifact_contract() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = json!({
            "web_conduit": {
                "fetch_provider_order": ["direct_http"]
            }
        });
        let catalog = fetch_provider_catalog_snapshot(tmp.path(), &policy);
        let rows = catalog.as_array().expect("catalog rows");
        let default = rows.first().expect("default fetch provider");
        assert_eq!(
            default
                .pointer("/contract_fields/credential_contract/type")
                .and_then(Value::as_str),
            Some("none")
        );
        assert_eq!(
            default
                .pointer("/public_artifact_contract/artifact_candidates/0")
                .and_then(Value::as_str),
            Some("web-fetch-contract-api.js")
        );
    }

    #[test]
    fn web_tool_catalog_snapshot_reports_registration_contracts() {
        let policy = json!({
            "web_conduit": {
                "search_provider_order": ["duckduckgo", "bing_rss"],
                "fetch_provider_order": ["direct_http"],
                "search_default_count": 4
            }
        });
        let catalog = web_tool_catalog_snapshot(&policy);
        let rows = catalog.as_array().expect("tool rows");
        let search = rows
            .iter()
            .find(|row| row.get("tool").and_then(Value::as_str) == Some("web_search"))
            .expect("web_search tool");
        let fetch = rows
            .iter()
            .find(|row| row.get("tool").and_then(Value::as_str) == Some("web_fetch"))
            .expect("web_fetch tool");
        assert_eq!(
            search
                .pointer("/request_contract/default_count")
                .and_then(Value::as_u64),
            Some(4)
        );
        assert_eq!(
            search
                .pointer("/registration_contract/public_artifact_contract/artifact_candidates/0")
                .and_then(Value::as_str),
            Some("web-search-contract-api.js")
        );
        assert_eq!(
            fetch
                .pointer("/registration_contract/public_artifact_contract/runtime_mode")
                .and_then(Value::as_str),
            Some("built_in_only")
        );
        assert_eq!(
            fetch
                .pointer("/registration_contract/runtime_resolution_contract/configured_provider_fallback")
                .and_then(Value::as_str),
            Some("auto-detect")
        );
        assert_eq!(
            search
                .pointer("/capability_contract/integration_point")
                .and_then(Value::as_str),
            Some("search_provider_adapter_registry")
        );
        assert_eq!(
            search
                .pointer("/capability_contract/output_contract/schema")
                .and_then(Value::as_str),
            Some("web_search_result_v1")
        );
        assert_eq!(
            fetch
                .pointer("/capability_contract/integration_point")
                .and_then(Value::as_str),
            Some("materialization_provider_adapter_registry")
        );
    }
}
