#[cfg(test)]
mod openclaw_search_tool_tests {
    use super::*;

    #[test]
    fn api_search_rejects_unsupported_search_filters() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_search(
            tmp.path(),
            &json!({
                "query": "agent reliability benchmarks",
                "country": "us"
            }),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.get("error").and_then(Value::as_str),
            Some("unsupported_country")
        );
        assert_eq!(
            out.get("unsupported_filter").and_then(Value::as_str),
            Some("country")
        );
        assert!(out.get("provider_catalog").is_some());
        assert!(out.get("receipt").is_some());
    }

    #[test]
    fn api_search_accepts_camel_case_search_provider_hint_alias() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_search(
            tmp.path(),
            &json!({
                "query": "agent reliability benchmarks",
                "searchProvider": "firecrawl"
            }),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.get("error").and_then(Value::as_str),
            Some("unknown_search_provider")
        );
        assert_eq!(
            out.get("requested_provider").and_then(Value::as_str),
            Some("firecrawl")
        );
    }

    #[test]
    fn api_search_count_alias_hits_cached_response() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let request = json!({
            "query": "agent reliability benchmark",
            "count": 4,
            "summary_only": true,
            "cache_ttl_minutes": 9
        });
        let query = clean_text(
            request
                .get("query")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            600,
        );
        let (policy, _) = load_policy(tmp.path());
        let allowed_domains =
            normalize_allowed_domains(request.get("allowed_domains").unwrap_or(&Value::Null));
        let exclude_subdomains = false;
        let scoped_query = scoped_search_query(&query, &allowed_domains, exclude_subdomains);
        let provider_chain = crate::web_conduit_provider_runtime::provider_chain_from_request(
            "auto", &request, &policy,
        );
        let top_k = crate::web_conduit_provider_runtime::resolve_search_count(&request, &policy);
        let key = crate::web_conduit_provider_runtime::search_cache_key(
            &query,
            &scoped_query,
            &allowed_domains,
            exclude_subdomains,
            top_k,
            true,
            &provider_chain,
            &normalized_search_filters(&request),
        );
        crate::web_conduit_provider_runtime::store_search_cache(
            tmp.path(),
            &key,
            &json!({
                "ok": true,
                "summary": "cached search summary",
                "content": "",
                "provider": "duckduckgo"
            }),
            "ok",
            Some(9 * 60),
        );

        let out = api_search(tmp.path(), &request);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("cache_status").and_then(Value::as_str), Some("hit"));
        assert_eq!(out.get("top_k").and_then(Value::as_u64), Some(4));
        assert_eq!(out.get("count").and_then(Value::as_u64), Some(4));
        assert_eq!(
            out.get("cache_ttl_minutes").and_then(Value::as_u64),
            Some(9)
        );
    }

    #[test]
    fn api_providers_reports_tool_catalog_and_search_request_contract() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_providers(tmp.path());
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.pointer("/search_request_contract/default_count")
                .and_then(Value::as_u64),
            Some(8)
        );
        let tools = out
            .get("tool_catalog")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(tools
            .iter()
            .any(|row| row.get("tool").and_then(Value::as_str) == Some("web_search")));
        assert!(tools
            .iter()
            .any(|row| row.get("tool").and_then(Value::as_str) == Some("web_fetch")));
    }

    #[test]
    fn api_providers_reports_public_artifact_contracts_and_registration_contracts() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_providers(tmp.path());
        assert_eq!(
            out.pointer("/public_artifact_contracts/search/artifact_candidates/0")
                .and_then(Value::as_str),
            Some("web-search-contract-api.js")
        );
        assert_eq!(
            out.pointer("/search_provider_registration_contract/credential_types_supported/0")
                .and_then(Value::as_str),
            Some("none")
        );
        assert_eq!(
            out.pointer("/search_provider_registration_contract/credential_types_supported/1")
                .and_then(Value::as_str),
            Some("top-level")
        );
        assert_eq!(
            out.pointer("/fetch_provider_registration_contract/public_artifact_contract/resolution_mode")
                .and_then(Value::as_str),
            Some("explicit_allowlist")
        );
    }

    #[test]
    fn api_status_and_providers_report_runtime_web_tools_metadata() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let status = api_status(tmp.path());
        let providers = api_providers(tmp.path());
        assert!(status
            .get("runtime_web_tools_state_path")
            .and_then(Value::as_str)
            .map(|path| path.ends_with("runtime_web_tools_metadata.json"))
            .unwrap_or(false));
        let default_search_provider = status
            .pointer("/default_search_provider_chain/0")
            .and_then(Value::as_str);
        assert!(default_search_provider.is_some());
        assert_eq!(
            status
                .pointer("/runtime_web_tools_metadata/search/selected_provider")
                .and_then(Value::as_str),
            default_search_provider
        );
        assert_eq!(
            providers
                .pointer("/runtime_web_tools_metadata/fetch/selected_provider")
                .and_then(Value::as_str),
            Some("direct_http")
        );
    }

    #[test]
    fn api_status_and_providers_report_native_codex_contract() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let status = api_status(tmp.path());
        let providers = api_providers(tmp.path());
        assert_eq!(
            status
                .pointer("/native_codex_web_search/eligible_model_contract/api")
                .and_then(Value::as_str),
            Some("openai-codex-responses")
        );
        assert_eq!(
            providers
                .pointer("/native_codex_web_search/auth/env_keys/0")
                .and_then(Value::as_str),
            Some("OPENAI_API_KEY")
        );
        assert_eq!(
            providers
                .pointer("/native_codex_web_search/tool_definition/type")
                .and_then(Value::as_str),
            Some("web_search")
        );
    }

    #[test]
    fn api_native_codex_requires_auth_for_direct_openai_codex_models() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_json_atomic(
            &policy_path(tmp.path()),
            &json!({
                "web_conduit": {
                    "enabled": true,
                    "search_provider_order": ["duckduckgo"],
                    "fetch_provider_order": ["direct_http"],
                    "native_codex_web_search": {
                        "enabled": true,
                        "mode": "cached"
                    }
                }
            }),
        )
        .expect("write policy");
        let out = api_native_codex(
            tmp.path(),
            &json!({
                "model_provider": "openai-codex",
                "model_api": "openai-codex-responses"
            }),
        );
        assert_eq!(
            out.pointer("/activation/state").and_then(Value::as_str),
            Some("managed_only")
        );
        assert_eq!(
            out.pointer("/activation/inactive_reason")
                .and_then(Value::as_str),
            Some("codex_auth_missing")
        );
    }

    #[test]
    fn api_native_codex_activates_and_patches_payload_when_configured() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_json_atomic(
            &policy_path(tmp.path()),
            &json!({
                "web_conduit": {
                    "enabled": true,
                    "search_provider_order": ["duckduckgo", "bing_rss"],
                    "fetch_provider_order": ["direct_http"],
                    "native_codex_web_search": {
                        "enabled": true,
                        "mode": "live",
                        "allowedDomains": [" example.com ", "example.com"],
                        "contextSize": "high",
                        "userLocation": {
                            "country": "US",
                            "city": "New York",
                            "timezone": "America/New_York"
                        }
                    }
                }
            }),
        )
        .expect("write policy");
        write_json_atomic(
            &tmp.path()
                .join("client/runtime/local/state/ui/infring_dashboard/provider_secrets.json"),
            &json!({
                "providers": {
                    "openai": {
                        "key": "sk-test-openai"
                    }
                }
            }),
        )
        .expect("write provider secrets");
        let out = api_native_codex(
            tmp.path(),
            &json!({
                "model_provider": "openai-codex",
                "model_api": "openai-codex-responses",
                "payload": {
                    "tools": [
                        { "type": "function", "name": "read" }
                    ]
                }
            }),
        );
        assert_eq!(
            out.pointer("/activation/state").and_then(Value::as_str),
            Some("native_active")
        );
        assert_eq!(
            out.pointer("/payload_patch/status").and_then(Value::as_str),
            Some("injected")
        );
        assert_eq!(
            out.pointer("/suppress_managed_web_search_tool")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/native_codex_web_search/tool_definition/external_web_access")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/native_codex_web_search/tool_definition/filters/allowed_domains/0")
                .and_then(Value::as_str),
            Some("example.com")
        );
        assert_eq!(
            out.pointer("/native_codex_web_search/tool_definition/search_context_size")
                .and_then(Value::as_str),
            Some("high")
        );
        assert_eq!(
            out.pointer("/native_codex_web_search/tool_definition/user_location/city")
                .and_then(Value::as_str),
            Some("New York")
        );
    }

    #[test]
    fn api_native_codex_keeps_api_compatible_models_active_without_direct_auth() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_json_atomic(
            &policy_path(tmp.path()),
            &json!({
                "web_conduit": {
                    "enabled": true,
                    "search_provider_order": ["duckduckgo"],
                    "fetch_provider_order": ["direct_http"],
                    "native_codex_web_search": {
                        "enabled": true,
                        "mode": "cached"
                    }
                }
            }),
        )
        .expect("write policy");
        let out = api_native_codex(
            tmp.path(),
            &json!({
                "model_provider": "gateway",
                "model_api": "openai-codex-responses",
                "payload": {
                    "tools": [{ "type": "web_search" }]
                }
            }),
        );
        assert_eq!(
            out.pointer("/activation/state").and_then(Value::as_str),
            Some("native_active")
        );
        assert_eq!(
            out.pointer("/payload_patch/status").and_then(Value::as_str),
            Some("native_tool_already_present")
        );
    }

    #[test]
    fn api_providers_fetch_contract_aligns_with_builtin_fetch_catalog() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_providers(tmp.path());
        let allowlist = out
            .pointer("/fetch_provider_registration_contract/public_artifact_contract/allowlisted_provider_ids")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let fetch_rows = out
            .get("fetch_providers")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(!fetch_rows.is_empty());
        for row in fetch_rows {
            let provider = row
                .get("provider")
                .and_then(Value::as_str)
                .expect("provider");
            assert!(allowlist
                .iter()
                .any(|entry| entry.as_str() == Some(provider)));
            assert_eq!(
                row.pointer("/contract_fields/credential_contract/type")
                    .and_then(Value::as_str),
                Some("none")
            );
            assert_eq!(
                row.pointer("/public_artifact_contract/resolution_mode")
                    .and_then(Value::as_str),
                Some("explicit_allowlist")
            );
        }
    }

    #[test]
    fn api_setup_lists_provider_options_and_defaults() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_setup(tmp.path(), &json!({}));
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.pointer("/setup_contract/default_provider")
                .and_then(Value::as_str),
            Some("tavily")
        );
        assert!(out
            .pointer("/setup_contract/provider_options")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().any(|row| row.get("provider").and_then(Value::as_str) == Some("tavily")))
            .unwrap_or(false));
    }

    #[test]
    fn api_setup_apply_preserves_disabled_state_and_sets_key_env() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = policy_path(tmp.path());
        write_json_atomic(
            &path,
            &json!({
                "web_conduit": {
                    "enabled": false,
                    "search_provider_order": ["duckduckgo", "bing_rss"],
                    "fetch_provider_order": ["direct_http"]
                }
            }),
        )
        .expect("write policy");
        let out = api_setup(
            tmp.path(),
            &json!({
                "provider": "serper",
                "api_key_env": "SERPER_API_KEY",
                "apply": true
            }),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        let updated = read_json_or(&path, json!({}));
        assert_eq!(
            updated.pointer("/web_conduit/enabled").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            updated
                .pointer("/web_conduit/search_provider_order/0")
                .and_then(Value::as_str),
            Some("serperdev")
        );
        assert_eq!(
            updated
                .pointer("/web_conduit/search_provider_config/serperdev/api_key_env")
                .and_then(Value::as_str),
            Some("SERPER_API_KEY")
        );
    }

    #[test]
    fn api_migrate_legacy_config_moves_search_and_archives_fetch() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let source = tmp.path().join("legacy-web-config.json");
        write_json_atomic(
            &source,
            &json!({
                "tools": {
                    "web": {
                        "search": {
                            "provider": "serper",
                            "enabled": false,
                            "apiKeyEnv": "SERPERDEV_API_KEY"
                        },
                        "fetch": {
                            "firecrawl": {
                                "apiKey": "fc-test",
                                "baseUrl": "https://api.firecrawl.dev"
                            }
                        }
                    }
                }
            }),
        )
        .expect("write source");
        let out = api_migrate_legacy_config(
            tmp.path(),
            &json!({
                "source_path": source.display().to_string(),
                "apply": true
            }),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        let updated = read_json_or(&source, json!({}));
        assert_eq!(
            updated
                .pointer("/web_conduit/search_provider_order/0")
                .and_then(Value::as_str),
            Some("serperdev")
        );
        assert_eq!(
            updated
                .pointer("/web_conduit/search_provider_config/serperdev/api_key_env")
                .and_then(Value::as_str),
            Some("SERPERDEV_API_KEY")
        );
        assert_eq!(
            updated.pointer("/web_conduit/enabled").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            updated
                .pointer("/web_conduit/legacy_migration_archive/fetch/firecrawl/baseUrl")
                .and_then(Value::as_str),
            Some("https://api.firecrawl.dev")
        );
        assert!(updated.pointer("/tools/web/fetch/firecrawl").is_none());
    }
}
