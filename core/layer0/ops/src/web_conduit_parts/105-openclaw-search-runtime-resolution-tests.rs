#[cfg(test)]
mod openclaw_search_runtime_resolution_tests {
    use super::*;

    #[test]
    fn openclaw_search_runtime_resolution_flags_invalid_top_level_search_provider() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_json_atomic(
            &policy_path(tmp.path()),
            &json!({
                "web_conduit": {
                    "enabled": true,
                    "search_provider": "firecrawl",
                    "search_provider_order": ["serperdev", "duckduckgo", "bing_rss"]
                }
            }),
        )
        .expect("write policy");

        let out = api_providers(tmp.path());
        assert_eq!(
            out.pointer("/runtime_web_tools_metadata/search/configured_provider_input")
                .and_then(Value::as_str),
            Some("firecrawl")
        );
        assert_eq!(
            out.pointer("/runtime_web_tools_metadata/search/selected_provider")
                .and_then(Value::as_str),
            Some("duckduckgo")
        );
        assert_eq!(
            out.pointer("/runtime_web_tools_metadata/search/selection_fallback_reason")
                .and_then(Value::as_str),
            Some("invalid_configured_provider")
        );
        assert_eq!(
            out.pointer("/default_search_provider_chain/0")
                .and_then(Value::as_str),
            Some("duckduckgo")
        );
        assert!(out
            .pointer("/runtime_web_tools_metadata/search/diagnostics")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().any(|row| {
                row.get("code").and_then(Value::as_str)
                    == Some("WEB_SEARCH_PROVIDER_INVALID_AUTODETECT")
            }))
            .unwrap_or(false));
    }

    #[test]
    fn openclaw_search_runtime_resolution_prefers_valid_top_level_provider_in_default_chain() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_json_atomic(
            &policy_path(tmp.path()),
            &json!({
                "web_conduit": {
                    "enabled": true,
                    "search_provider": "bing_rss",
                    "search_provider_order": ["duckduckgo", "duckduckgo_lite", "bing_rss"]
                }
            }),
        )
        .expect("write policy");

        let out = api_providers(tmp.path());
        assert_eq!(
            out.pointer("/default_search_provider_chain/0")
                .and_then(Value::as_str),
            Some("bing_rss")
        );
        assert_eq!(
            out.pointer("/runtime_web_tools_metadata/search/provider_source")
                .and_then(Value::as_str),
            Some("configured")
        );
        assert_eq!(
            out.pointer("/runtime_web_tools_metadata/search/selected_provider")
                .and_then(Value::as_str),
            Some("bing_rss")
        );
        assert_eq!(
            out.pointer("/tool_catalog/0/default_provider")
                .and_then(Value::as_str),
            Some("bing_rss")
        );
    }

    #[test]
    fn openclaw_search_runtime_resolution_keeps_secret_ref_providers_ahead_of_keyless_fallbacks() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_json_atomic(
            &policy_path(tmp.path()),
            &json!({
                "web_conduit": {
                    "enabled": true,
                    "search_provider_config": {
                        "tavily": {
                            "secret_id": "web_search_tavily_api_key"
                        },
                        "exa": {
                            "secret_id": "web_search_exa_api_key"
                        }
                    },
                    "search_provider_order": [
                        "tavily",
                        "exa",
                        "google_news_rss",
                        "bing_rss",
                        "duckduckgo"
                    ]
                }
            }),
        )
        .expect("write policy");

        let out = api_providers(tmp.path());
        assert_eq!(
            out.pointer("/default_search_provider_chain/0")
                .and_then(Value::as_str),
            Some("tavily")
        );
        assert_eq!(
            out.pointer("/default_search_provider_chain/1")
                .and_then(Value::as_str),
            Some("exa")
        );
        assert_eq!(
            out.pointer("/runtime_web_tools_metadata/search/selected_provider")
                .and_then(Value::as_str),
            Some("tavily")
        );
        assert_eq!(
            out.pointer("/runtime_web_tools_metadata/search/selected_provider_key_source")
                .and_then(Value::as_str),
            Some("secret_broker")
        );
    }

    #[test]
    fn openclaw_search_runtime_resolution_snapshot_prefers_request_hint_scope() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = json!({
            "web_conduit": {
                "enabled": true,
                "search_provider_order": ["duckduckgo", "bing_rss"]
            }
        });

        let out = crate::web_conduit_provider_runtime::search_provider_resolution_snapshot(
            tmp.path(),
            &policy,
            &json!({"provider": "ddg"}),
            "ddg",
        );
        assert_eq!(
            out.pointer("/selection_scope").and_then(Value::as_str),
            Some("request_provider_hint")
        );
        assert_eq!(
            out.pointer("/requested_provider_hint")
                .and_then(Value::as_str),
            Some("ddg")
        );
        assert_eq!(
            out.pointer("/provider_chain/0").and_then(Value::as_str),
            Some("duckduckgo")
        );
        assert_eq!(
            out.pointer("/allow_fallback").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/resolution_contract/prefer_runtime_providers")
                .and_then(Value::as_bool),
            Some(false)
        );
        let expect_str = |pointer: &str, expected: &str| {
            assert_eq!(out.pointer(pointer).and_then(Value::as_str), Some(expected));
        };
        let expect_bool = |pointer: &str, expected: bool| {
            assert_eq!(out.pointer(pointer).and_then(Value::as_bool), Some(expected));
        };
        let expect_u64 = |pointer: &str, expected: u64| {
            assert_eq!(out.pointer(pointer).and_then(Value::as_u64), Some(expected));
        };

        for (pointer, expected) in [
            (
                "/openclaw_runtime_contract/fallback_runtime_resolver",
                "resolvePluginWebSearchProviders",
            ),
            (
                "/openclaw_runtime_contract/public_artifact_runtime_resolver",
                "resolveBundledWebSearchProvidersFromPublicArtifacts",
            ),
            (
                "/openclaw_runtime_contract/manifest_contract_owner_resolver",
                "resolveManifestContractOwnerPluginId",
            ),
            (
                "/openclaw_runtime_contract/runtime_registry_resolver",
                "resolveRuntimeWebSearchProviders",
            ),
            (
                "/openclaw_runtime_contract/candidate_plugin_contract/contract",
                "webSearchProviders",
            ),
            (
                "/openclaw_runtime_contract/provider_sort_contract/auto_detect_sorter",
                "sortPluginProvidersForAutoDetect",
            ),
            (
                "/openclaw_runtime_contract/provider_type_contract/provider_context_type",
                "WebSearchProviderContext",
            ),
            (
                "/openclaw_runtime_contract/credential_presence_contract/resolver",
                "hasConfiguredWebSearchCredential",
            ),
            (
                "/openclaw_runtime_contract/provider_contract_suite_contract/suite_helper_module",
                "test/helpers/plugins/web-search-provider-contract.ts",
            ),
            (
                "/openclaw_runtime_contract/provider_contract_suite_contract/provider_specific_contract_invocation",
                "describeWebSearchProviderContracts(providerId)",
            ),
            (
                "/openclaw_runtime_contract/provider_contract_suite_contract/base_provider_contract/provider_id_regex",
                "^[a-z0-9][a-z0-9-]*$",
            ),
            (
                "/openclaw_runtime_contract/provider_registry_contract/registry_contract_test_file",
                "src/plugins/contracts/registry.contract.test.ts",
            ),
            (
                "/openclaw_runtime_contract/provider_runtime_contract/runtime_contract_test_file",
                "src/plugins/contracts/provider-runtime.contract.test.ts",
            ),
            (
                "/openclaw_runtime_contract/provider_runtime_contract/runtime_contract_helper_module",
                "test/helpers/plugins/provider-runtime-contract.ts",
            ),
            (
                "/openclaw_runtime_contract/provider_runtime_contract/runtime_contract_provider_targets/3",
                "openai",
            ),
            (
                "/openclaw_runtime_contract/provider_runtime_contract/runtime_contract_invariants/2",
                "auth_doctor_hint_generation_parity",
            ),
            (
                "/openclaw_runtime_contract/provider_runtime_module_contract/runtime_module_targets/0",
                "src/plugins/provider-runtime.test.ts",
            ),
            (
                "/openclaw_runtime_contract/provider_runtime_module_contract/runtime_module_targets/4",
                "src/plugins/provider-runtime.test-support.ts",
            ),
            (
                "/openclaw_runtime_contract/provider_runtime_module_contract/module_runtime_entrypoints/1",
                "resolveProviderRuntimeModels",
            ),
            (
                "/openclaw_runtime_contract/provider_family_contract_suite_contract/contract_targets/0",
                "src/plugins/contracts/memory-embedding-provider.contract.test.ts",
            ),
            (
                "/openclaw_runtime_contract/provider_family_contract_suite_contract/contract_targets/4",
                "src/plugins/contracts/provider.minimax.contract.test.ts",
            ),
            (
                "/openclaw_runtime_contract/provider_family_contract_suite_contract/contract_targets/7",
                "src/plugins/contracts/provider.openrouter.contract.test.ts",
            ),
            (
                "/openclaw_runtime_contract/provider_family_contract_suite_contract/contract_targets/8",
                "src/plugins/contracts/provider.xai.contract.test.ts",
            ),
            (
                "/openclaw_runtime_contract/provider_family_contract_suite_contract/suite_validation_tests/0",
                "src/plugins/contracts/provider-family-plugin-tests.test.ts",
            ),
            (
                "/openclaw_runtime_contract/provider_family_contract_suite_contract/runtime_invariants/1",
                "model_catalog_compatibility_contract",
            ),
            (
                "/openclaw_runtime_contract/provider_auth_contract/auth_contract_test_file",
                "src/plugins/contracts/provider-auth.contract.test.ts",
            ),
            (
                "/openclaw_runtime_contract/provider_auth_contract/auth_contract_helper_module",
                "test/helpers/plugins/provider-auth-contract.ts",
            ),
            (
                "/openclaw_runtime_contract/provider_auth_contract/auth_contract_provider_targets/0",
                "openai-codex",
            ),
            (
                "/openclaw_runtime_contract/provider_auth_contract/auth_contract_provider_targets/1",
                "github-copilot",
            ),
            (
                "/openclaw_runtime_contract/provider_config_contract/forced_provider_wrapper",
                "withForcedProvider",
            ),
            (
                "/openclaw_runtime_contract/provider_config_contract/scoped_credential_accessors/0",
                "getScopedCredentialValue",
            ),
            (
                "/openclaw_runtime_contract/provider_credential_resolution_contract/resolver",
                "resolveWebSearchProviderCredential",
            ),
            (
                "/openclaw_runtime_contract/provider_credential_resolution_contract/resolution_order/1",
                "config_secret_ref_env_value",
            ),
            (
                "/openclaw_runtime_contract/provider_common_runtime_contract/cache_write_helper",
                "writeCache",
            ),
            (
                "/openclaw_runtime_contract/provider_common_runtime_contract/trusted_json_post_wrapper",
                "postTrustedWebToolsJson",
            ),
            (
                "/openclaw_runtime_contract/provider_common_runtime_contract/freshness_cross_provider_mapping_supported/brave_shortcuts/1",
                "pw",
            ),
            (
                "/openclaw_runtime_contract/provider_common_runtime_contract/date_range_contract/perplexity_date_converter",
                "isoToPerplexityDate",
            ),
            (
                "/openclaw_runtime_contract/provider_common_runtime_contract/unsupported_filter_contract/date_filter_error_code",
                "unsupported_date_filter",
            ),
            (
                "/openclaw_runtime_contract/citation_redirect_contract/resolver_entrypoint",
                "resolveCitationRedirectUrl",
            ),
            (
                "/openclaw_runtime_contract/citation_redirect_contract/failure_fallback",
                "returns_original_url",
            ),
            (
                "/openclaw_runtime_contract/redirect_hardening_contract/guarded_endpoint_entrypoint",
                "withStrictWebToolsEndpoint",
            ),
            (
                "/openclaw_runtime_contract/redirect_hardening_contract/failure_mode_contract",
                "never_throws_returns_original_url",
            ),
            (
                "/openclaw_runtime_contract/snapshot_cache_contract/cache_key_builder",
                "buildWebProviderSnapshotCacheKey",
            ),
            (
                "/openclaw_runtime_contract/snapshot_cache_contract/in_flight_registry_load_guard",
                "does_not_force_fresh_snapshot_load",
            ),
            (
                "/openclaw_runtime_contract/snapshot_cache_contract/cache_key_dimensions/2",
                "workspace_dir",
            ),
            (
                "/openclaw_runtime_contract/provider_sort_contract/shared_sort_entrypoint",
                "sortWebSearchProviders",
            ),
            (
                "/openclaw_runtime_contract/public_artifact_resolution_contract/bundled_resolution_config_resolver",
                "resolveBundledWebSearchResolutionConfig",
            ),
            (
                "/openclaw_runtime_contract/bundled_fast_path_contract_suite_contract/suite_entrypoint",
                "describeBundledWebSearchFastPathContract",
            ),
            (
                "/openclaw_runtime_contract/bundled_fast_path_contract_suite_contract/runtime_registry_loader",
                "loadBundledCapabilityRuntimeRegistry",
            ),
            (
                "/openclaw_runtime_contract/bundled_fast_path_contract_suite_contract/provider_metadata_parity_required_fields/10",
                "inactiveSecretPaths",
            ),
            (
                "/openclaw_runtime_contract/bundled_fast_path_contract_suite_contract/runtime_metadata_parity_case_matrix/2",
                "provider_specific_model_override",
            ),
        ] {
            expect_str(pointer, expected);
        }
        for (idx, expected) in [
            (0usize, "brave"),
            (1usize, "duckduckgo"),
            (2usize, "exa"),
            (6usize, "tavily"),
            (7usize, "moonshot"),
            (8usize, "xai"),
        ] {
            let ptr = format!(
                "/openclaw_runtime_contract/provider_contract_suite_contract/provider_specific_contract_targets/{idx}"
            );
            assert_eq!(out.pointer(&ptr).and_then(Value::as_str), Some(expected));
        }
        for (idx, expected) in [
            (4usize, "web-search-provider.google.contract.test.ts"),
            (7usize, "web-search-provider.moonshot.contract.test.ts"),
        ] {
            let ptr = format!(
                "/openclaw_runtime_contract/provider_contract_suite_contract/registry_contract_test_files/{idx}"
            );
            assert_eq!(out.pointer(&ptr).and_then(Value::as_str), Some(expected));
        }
        for (pointer, expected) in [
            (
                "/openclaw_runtime_contract/provider_contract_suite_contract/tool_definition_contract/run_setup_optional_function",
                true,
            ),
            (
                "/openclaw_runtime_contract/provider_registry_contract/shared_resolver_coverage_invariant",
                true,
            ),
            (
                "/openclaw_runtime_contract/provider_common_runtime_contract/module_local_cache_required",
                true,
            ),
            (
                "/openclaw_runtime_contract/redirect_hardening_contract/guarded_head_resolution_required",
                true,
            ),
            (
                "/openclaw_runtime_contract/snapshot_cache_contract/active_registry_workspace_inheritance",
                true,
            ),
            (
                "/openclaw_runtime_contract/snapshot_cache_contract/workspace_change_invalidation",
                true,
            ),
            (
                "/openclaw_runtime_contract/public_artifact_resolution_contract/fast_path_skips_manifest_scans_when_only_plugin_ids",
                true,
            ),
            (
                "/openclaw_runtime_contract/public_artifact_resolution_contract/requires_public_artifact_for_each_bundled_manifest_contract_provider",
                true,
            ),
            (
                "/openclaw_runtime_contract/bundled_fast_path_contract_suite_contract/runtime_metadata_parity_optional",
                true,
            ),
        ] {
            expect_bool(pointer, expected);
        }
        for (pointer, expected) in [
            (
                "/openclaw_runtime_contract/provider_common_runtime_contract/default_search_count",
                5u64,
            ),
            (
                "/openclaw_runtime_contract/provider_common_runtime_contract/max_search_count",
                10u64,
            ),
            (
                "/openclaw_runtime_contract/provider_common_runtime_contract/timeout_default_seconds",
                30u64,
            ),
            (
                "/openclaw_runtime_contract/provider_common_runtime_contract/cache_ttl_default_minutes",
                15u64,
            ),
            (
                "/openclaw_runtime_contract/citation_redirect_contract/timeout_ms",
                5000u64,
            ),
        ] {
            expect_u64(pointer, expected);
        }
        for (idx, expected) in [
            (1usize, "xai"),
            (3usize, "brave"),
            (7usize, "google"),
            (8usize, "minimax"),
            (9usize, "perplexity"),
            (10usize, "tavily"),
        ] {
            let ptr = format!(
                "/openclaw_runtime_contract/bundled_fast_path_contract_suite_contract/suite_target_plugin_ids/{idx}"
            );
            assert_eq!(out.pointer(&ptr).and_then(Value::as_str), Some(expected));
        }
        for (idx, expected) in [
            (2usize, "bundled-web-search.searxng.contract.test.ts"),
            (3usize, "bundled-web-search.brave.contract.test.ts"),
            (7usize, "bundled-web-search.google.contract.test.ts"),
            (8usize, "bundled-web-search.minimax.contract.test.ts"),
            (9usize, "bundled-web-search.perplexity.contract.test.ts"),
            (10usize, "bundled-web-search.tavily.contract.test.ts"),
        ] {
            let ptr = format!(
                "/openclaw_runtime_contract/bundled_fast_path_contract_suite_contract/suite_contract_test_files/{idx}"
            );
            assert_eq!(out.pointer(&ptr).and_then(Value::as_str), Some(expected));
        }
        assert!(out
            .pointer("/openclaw_runtime_contract/diagnostic_code_contract")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().any(|row| {
                row.as_str() == Some("WEB_SEARCH_KEY_UNRESOLVED_NO_FALLBACK")
            }))
            .unwrap_or(false));
    }

    #[test]
    fn openclaw_search_runtime_resolution_prefers_runtime_metadata_provider_when_present() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = json!({
            "web_conduit": {
                "enabled": true,
                "search_provider_order": ["duckduckgo", "bing_rss"]
            }
        });

        let out = crate::web_conduit_provider_runtime::search_provider_resolution_snapshot(
            tmp.path(),
            &policy,
            &json!({
                "runtimeWebSearch": {
                    "selectedProvider": "bing_rss"
                }
            }),
            "auto",
        );
        assert_eq!(
            out.pointer("/selection_scope").and_then(Value::as_str),
            Some("runtime_metadata")
        );
        assert_eq!(
            out.pointer("/runtime_selected_provider")
                .and_then(Value::as_str),
            Some("bing_rss")
        );
        assert_eq!(
            out.pointer("/runtime_provider_preferred")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/provider_chain/0").and_then(Value::as_str),
            Some("bing_rss")
        );
        assert_eq!(
            out.pointer("/allow_fallback").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/tool_surface_status")
                .and_then(Value::as_str),
            Some("ready")
        );
        assert_eq!(
            out.pointer("/tool_surface_ready")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn openclaw_search_runtime_resolution_allows_secret_broker_request_hint() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = json!({
            "web_conduit": {
                "enabled": true,
                "search_provider_config": {
                    "tavily": {
                        "secret_id": "web_search_tavily_api_key"
                    }
                },
                "search_provider_order": ["duckduckgo_lite", "bing_rss"]
            }
        });

        let out = crate::web_conduit_provider_runtime::search_provider_resolution_snapshot(
            tmp.path(),
            &policy,
            &json!({"provider": "tavily"}),
            "tavily",
        );
        assert_eq!(
            out.pointer("/selection_scope").and_then(Value::as_str),
            Some("request_provider_hint")
        );
        assert_eq!(
            out.pointer("/selected_provider").and_then(Value::as_str),
            Some("tavily")
        );
        assert_eq!(
            out.pointer("/tool_surface_health/selected_provider_credential_state")
                .and_then(Value::as_str),
            Some("resolved")
        );
        assert_eq!(
            out.pointer("/tool_surface_ready").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/tool_execution_gate/should_execute")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn openclaw_search_runtime_resolution_accepts_camel_case_provider_chain_alias() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = json!({
            "web_conduit": {
                "enabled": true,
                "search_provider_order": ["duckduckgo", "bing_rss"]
            }
        });

        let out = crate::web_conduit_provider_runtime::search_provider_resolution_snapshot(
            tmp.path(),
            &policy,
            &json!({
                "providerChain": ["duckduckgo_lite", "bing_rss"]
            }),
            "auto",
        );
        assert_eq!(
            out.pointer("/selection_scope").and_then(Value::as_str),
            Some("request_provider_chain")
        );
        assert_eq!(
            out.pointer("/provider_chain/0").and_then(Value::as_str),
            Some("duckduckgo_lite")
        );
    }

    #[test]
    fn api_search_does_not_fallback_when_policy_provider_is_explicit_and_circuit_open() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_json_atomic(
            &policy_path(tmp.path()),
            &json!({
                "web_conduit": {
                    "enabled": true,
                    "search_provider": "bing_rss",
                    "search_provider_order": ["duckduckgo", "duckduckgo_lite", "bing_rss"],
                    "provider_circuit_breaker": {
                        "enabled": true,
                        "failure_threshold": 1,
                        "open_for_secs": 60
                    }
                }
            }),
        )
        .expect("write policy");
        let (policy, _) = load_policy(tmp.path());
        record_provider_attempt(tmp.path(), "bing_rss", false, "timeout", &policy);

        let out = api_search(tmp.path(), &json!({"query": "agent reliability"}));
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.get("error").and_then(Value::as_str),
            Some("provider_circuit_open")
        );
        assert_eq!(
            out.get("provider").and_then(Value::as_str),
            Some("bing_rss")
        );
        assert_eq!(
            out.pointer("/provider_resolution/selection_scope")
                .and_then(Value::as_str),
            Some("policy_configured")
        );
        assert_eq!(
            out.pointer("/provider_resolution/allow_fallback")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/provider_resolution/selected_provider")
                .and_then(Value::as_str),
            Some("bing_rss")
        );
        assert_eq!(
            out.pointer("/providers_skipped/0/provider")
                .and_then(Value::as_str),
            Some("bing_rss")
        );
        assert_eq!(
            out.pointer("/providers_skipped/0/reason")
                .and_then(Value::as_str),
            Some("circuit_open")
        );
        assert_eq!(
            out.get("tool_surface_status").and_then(Value::as_str),
            Some("ready")
        );
        assert_eq!(
            out.get("tool_surface_ready").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.get("tool_surface_blocking_reason")
                .and_then(Value::as_str),
            Some("none")
        );
        assert!(out
            .get("providers_attempted")
            .and_then(Value::as_array)
            .map(|rows| rows.is_empty())
            .unwrap_or(false));
    }

    #[test]
    fn empty_duckduckgo_metadata_shell_is_treated_as_low_signal_search_payload() {
        let payload = json!({
            "ok": true,
            "summary": "Key findings: {\"Abstract\":\"\",\"AbstractSource\":\"\",\"AbstractText\":\"\",\"AbstractURL\":\"\",\"Answer\":\"\",\"AnswerType\":\"\",\"Definition\":\"\",\"DefinitionSource\":\"\",\"DefinitionURL\":\"\",\"Entity\":\"\",\"Heading\":\"\",\"RelatedTopics\":[],\"Results\":[],\"Type\":\"\",\"url\":\"https://duck.",
            "content": "{\"Abstract\":\"\",\"AbstractSource\":\"\",\"AbstractText\":\"\",\"AbstractURL\":\"\",\"Answer\":\"\",\"AnswerType\":\"\",\"Definition\":\"\",\"DefinitionSource\":\"\",\"DefinitionURL\":\"\",\"Entity\":\"\",\"Heading\":\"\",\"RelatedTopics\":[],\"Results\":[],\"Type\":\"\"}"
        });
        assert!(payload_looks_low_signal_search(&payload));
        assert!(!search_payload_usable(&payload));
        assert_eq!(search_payload_error(&payload), "low_signal_search_payload");
    }
}
