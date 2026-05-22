
fn runtime_web_family_metadata(root: &Path, policy: &Value, family: WebProviderFamily) -> Value {
    let configured_path = match family {
        WebProviderFamily::Search => "/web_conduit/search_provider_order",
        WebProviderFamily::Fetch => "/web_conduit/fetch_provider_order",
    };
    let configured_provider_input = configured_provider_input_from_policy(policy, family);
    let explicit_provider_input = explicit_provider_input_from_policy(policy, family);
    let configured_provider_is_explicit = explicit_provider_input.is_some();
    let configured_provider = configured_provider_input
        .as_ref()
        .and_then(|raw| normalize_provider_token_for_family(raw, family));
    let selected_provider = match family {
        WebProviderFamily::Search => resolved_search_provider_chain("", &json!({}), policy)
            .first()
            .cloned(),
        WebProviderFamily::Fetch => fetch_provider_chain_from_request("", &json!({}), policy)
            .first()
            .cloned(),
    };
    let mut diagnostics = Vec::<Value>::new();
    if let Some(raw) = configured_provider_input.as_ref() {
        if configured_provider.is_none() {
            diagnostics.push(runtime_diagnostic(
                invalid_provider_code(family),
                format!(
                    "{configured_path} contains unsupported provider token \"{raw}\"; falling back to auto-detect precedence."
                ),
                configured_path,
            ));
        }
    }
    for raw in raw_provider_tokens_from_policy(policy, family) {
        if normalize_provider_token_for_family(&raw, family).is_none()
            && configured_provider_input.as_deref() != Some(raw.as_str())
        {
            diagnostics.push(runtime_diagnostic(
                invalid_provider_code(family),
                format!(
                    "{configured_path} contains unsupported provider token \"{raw}\"; falling back to auto-detect precedence."
                ),
                configured_path,
            ));
        }
    }
    let provider_source = if let Some(configured) = configured_provider.as_ref() {
        if selected_provider.as_ref() == Some(configured) {
            if configured_provider_is_explicit {
                "configured"
            } else {
                "policy_order"
            }
        } else if selected_provider.is_some() {
            let missing_explicit_credential =
                configured_provider_is_explicit
                    && !provider_has_configured_secret_ref(policy, configured, family)
                    && !provider_has_runtime_credential_with(configured, family, |key| {
                        std::env::var(key).ok()
                    }) && provider_descriptor(configured, family)
                        .map(|descriptor| !descriptor.env_keys.is_empty())
                        .unwrap_or(false);
            if missing_explicit_credential {
                if let Some(selected) = selected_provider.as_ref() {
                    diagnostics.push(runtime_diagnostic(
                        fallback_used_code(family),
                        format!(
                            "{configured_path} prefers \"{configured}\", but its credential is unresolved; falling back to \"{selected}\"."
                        ),
                        &configured_scope_path(configured, family),
                    ));
                } else {
                    diagnostics.push(runtime_diagnostic(
                        no_fallback_code(family),
                        format!(
                            "{configured_path} prefers \"{configured}\", but no credential-backed or keyless fallback provider is available."
                        ),
                        &configured_scope_path(configured, family),
                    ));
                }
            }
            "auto-detect"
        } else {
            "none"
        }
    } else if let Some(selected) = selected_provider.as_ref() {
        diagnostics.push(runtime_diagnostic(
            auto_detect_code(family),
            format!(
                "{} auto-detected provider \"{selected}\".",
                provider_family_name(family)
            ),
            configured_path,
        ));
        "auto-detect"
    } else {
        "none"
    };
    let selection_fallback_reason = if explicit_provider_input.is_some()
        && configured_provider.is_none()
        && selected_provider.is_some()
    {
        Some("invalid_configured_provider")
    } else if configured_provider_is_explicit
        && configured_provider.is_some()
        && selected_provider.is_some()
        && selected_provider != configured_provider
    {
        Some("credential_unresolved")
    } else {
        None
    };
    let owner_provider = selected_provider
        .as_deref()
        .or(configured_provider.as_deref());
    let selected_provider_key_source = selected_provider_key_source(policy, owner_provider, family);
    let tool_surface_health = runtime_web_family_health(
        family,
        selected_provider.as_deref(),
        &selected_provider_key_source,
        selection_fallback_reason,
        &diagnostics,
    );
    let allow_fallback_hint = provider_source != "configured";
    let execution_gate = runtime_web_execution_gate(
        tool_surface_health
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("unavailable"),
        tool_surface_health
            .get("selected_provider_ready")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        allow_fallback_hint,
        tool_surface_health
            .get("blocking_reason")
            .and_then(Value::as_str)
            .unwrap_or("none"),
    );
    json!({
        "configured_provider_input": configured_provider_input,
        "explicit_provider_input": explicit_provider_input,
        "provider_configured": configured_provider,
        "provider_source": provider_source,
        "selected_provider": selected_provider,
        "selected_provider_key_source": selected_provider_key_source,
        "selection_fallback_reason": selection_fallback_reason,
        "configured_surface_path": configured_provider
            .as_deref()
            .map(|provider| configured_scope_path(provider, family)),
        "config_surface": config_surface_snapshot(policy, owner_provider, family),
        "manifest_contract_owner": manifest_contract_owner(owner_provider, family),
        "public_artifact_runtime": public_artifact_contract_for_family(family),
        "tool_surface_health": tool_surface_health,
        "execution_gate": execution_gate,
        "resolution_contract": runtime_resolution_contract(family),
        "state_path": runtime_web_tools_state_path(root).display().to_string(),
        "diagnostics": diagnostics
    })
}

fn browser_materialization_array_field(
    source: &Value,
    field: &str,
    fallback: &[&str],
) -> Value {
    source
        .get(field)
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| json!(clean_text(row, 160)))
                .collect::<Vec<_>>()
        })
        .filter(|rows| !rows.is_empty())
        .map(Value::Array)
        .unwrap_or_else(|| Value::Array(fallback.iter().map(|row| json!(row)).collect()))
}

fn browser_materialization_profile_compilation_contract(
    config: &Value,
    enabled: bool,
    adapter_ready: bool,
) -> Value {
    let profile_contract = config
        .get("profile_contract")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let request_contract = config
        .get("request_contract")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let denied_fields = browser_materialization_array_field(
        &request_contract,
        "denied_fields",
        &[
            "browser_args",
            "launch_args",
            "args",
            "stealthArgs",
            "stealth_args",
            "extra_args",
            "_strategy_args",
            "ignoreDefaultArgs",
            "ignore_default_args",
            "binary_path",
            "binaryPath",
            "download_url",
            "downloadUrl",
            "cache_dir",
            "cacheDir",
            "chromium_version",
            "chromiumVersion",
            "fingerprint_seed",
            "fingerprintSeed",
            "backend",
            "browser_backend",
            "browserBackend",
            "adapter",
            "adapter_kind",
            "launchOptions",
            "contextOptions",
            "context_options",
            "headless",
            "viewport",
            "userAgent",
            "user_agent",
            "timezone",
            "timezoneId",
            "timezone_id",
            "locale",
            "colorScheme",
            "humanize",
            "humanPreset",
            "humanConfig",
            "geoip",
            "cdp_command",
            "user_script",
            "proxy",
            "proxy_url",
            "proxy_credentials",
            "proxyUrl",
            "proxyCredentials",
            "session_id",
            "sessionId",
            "storage_state",
            "storageState",
            "local_file",
            "localFile",
            "userDataDir",
        ],
    );
    let denied_launch_args = browser_materialization_array_field(
        &profile_contract,
        "denied_launch_args",
        &[
            "--remote-debugging-port",
            "--disable-web-security",
            "--ignore-certificate-errors",
            "--allow-file-access-from-files",
            "--load-extension",
        ],
    );
    let telemetry_fields = browser_materialization_array_field(
        &profile_contract,
        "telemetry_fields",
        &[
            "profile_ref",
            "state_scope",
            "effective_profile_hash",
            "denied_option_count",
        ],
    );
    json!({
        "version": "browser_profile_compilation_v1",
        "source_pattern": "cloakbrowser_launch_profile_compiler",
        "compile_before_adapter_launch": true,
        "status": if !enabled {
            "prepared_capability_disabled"
        } else if adapter_ready {
            "ready_for_adapter"
        } else {
            "blocked_adapter_not_ready"
        },
        "profile_source": profile_contract
            .get("profile_source")
            .and_then(Value::as_str)
            .unwrap_or("tool_cd_policy"),
        "default_profile": profile_contract
            .get("default_profile")
            .and_then(Value::as_str)
            .unwrap_or("stateless_public_materialization"),
        "effective_profile_ref": profile_contract
            .get("default_profile")
            .and_then(Value::as_str)
            .unwrap_or("stateless_public_materialization"),
        "state_scope": profile_contract
            .get("state_scope")
            .and_then(Value::as_str)
            .unwrap_or("stateless"),
        "caller_override_allowed": profile_contract
            .get("caller_override_allowed")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        "normalize_or_reject_context_conflicts": profile_contract
            .get("normalize_or_reject_context_conflicts")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        "context_options_allowed_from_caller": profile_contract
            .get("context_options_allowed_from_caller")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        "context_conflict_fields": profile_contract
            .get("context_conflict_fields")
            .cloned()
            .unwrap_or_else(|| json!([
                "locale",
                "timezone",
                "timezoneId",
                "timezone_id",
                "viewport",
                "userAgent",
                "user_agent",
                "colorScheme"
            ])),
        "close_browser_on_context_creation_failure": profile_contract
            .get("close_browser_on_context_creation_failure")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        "context_close_closes_browser": profile_contract
            .get("context_close_closes_browser")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        "default_viewport_policy_owned": profile_contract
            .get("default_viewport_policy_owned")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        "caller_viewport_allowed": profile_contract
            .get("caller_viewport_allowed")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        "caller_user_agent_allowed": profile_contract
            .get("caller_user_agent_allowed")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        "caller_color_scheme_allowed": profile_contract
            .get("caller_color_scheme_allowed")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        "locale_timezone_cdp_emulation_allowed": profile_contract
            .get("locale_timezone_cdp_emulation_allowed")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        "locale_timezone_binary_profile_fields_policy_owned": profile_contract
            .get("locale_timezone_binary_profile_fields_policy_owned")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        "geoip_filled_profile_fields_require_proxy_capability": profile_contract
            .get("geoip_filled_profile_fields_require_proxy_capability")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        "generic_context_kwargs_allowed_from_workflow": profile_contract
            .get("generic_context_kwargs_allowed_from_workflow")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        "storage_state_requires_session_capability": profile_contract
            .get("storage_state_requires_session_capability")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        "persistent_context_allowed": profile_contract
            .get("persistent_context_allowed")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        "persistent_session_contract": profile_contract
            .get("persistent_session_contract")
            .cloned()
            .unwrap_or_else(|| json!({
                "version": "browser_persistent_session_contract_v1",
                "source_pattern": "cloakbrowser_persistent_context_tests",
                "default_admitted": false,
                "separate_capability_required": true,
                "direct_user_data_dir_allowed": false,
                "session_profile_ref_broker_owned": true,
                "caller_supplied_args_allowed": false,
                "same_argument_compiler_required": true,
                "default_viewport_policy_owned": true,
                "admitted_profile_overrides_must_be_policy_fields": true,
                "locale_timezone_binary_profile_fields_only": true,
                "locale_timezone_context_kwargs_allowed": false,
                "geoip_fill_requires_proxy_capability": true,
                "timezone_id_alias_normalized_by_policy": true,
                "proxy_requires_proxy_capability": true,
                "sync_async_close_stops_driver_required": true,
                "context_close_persists_only_admitted_session_state": true,
                "raw_user_data_dir_chat_visible": false,
                "raw_storage_state_chat_visible": false,
                "raw_persistent_profile_path_chat_visible": false
            })),
        "humanized_interaction_allowed": profile_contract
            .get("humanized_interaction_allowed")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        "human_interaction_contract": profile_contract
            .get("human_interaction_contract")
            .cloned()
            .unwrap_or_else(|| json!({
                "version": "browser_human_interaction_contract_v1",
                "source_pattern": "cloakbrowser_human_config_presets",
                "default_admitted": false,
                "separate_capability_required": true,
                "direct_request_human_fields_allowed": false,
                "presets_policy_owned": true,
                "allowed_presets": ["default", "careful"],
                "numeric_action_budget_schema_required": true,
                "per_call_overrides_require_capability": true,
                "config_merge_must_not_mutate_base": true,
                "randomization_budget_policy_owned": true,
                "typing_mouse_scroll_controls_not_user_request_fields": true,
                "idle_between_actions_default_off": true,
                "page_frame_element_handle_patching_requires_capability": true,
                "cursor_state_capability_local_only": true,
                "trusted_key_dispatch_requires_human_capability": true,
                "raw_mouse_interface_exposed_to_workflow": false,
                "bezier_mouse_path_policy_owned": true,
                "movement_randomization_budget_bounded": true,
                "click_targeting_requires_element_box": true,
                "input_click_bias_policy_owned": true,
                "idle_mouse_drift_capability_only": true,
                "raw_mouse_coordinates_chat_visible": false,
                "raw_keyboard_interface_exposed_to_workflow": false,
                "typing_cadence_policy_owned": true,
                "mistype_simulation_budget_bounded": true,
                "nearby_key_error_map_policy_owned": true,
                "shift_symbol_dispatch_requires_cdp_session": true,
                "insert_text_fallback_capability_only": true,
                "synthetic_keyboard_events_telemetry_only": true,
                "raw_keyboard_text_chat_visible": false,
                "raw_scroll_interface_exposed_to_workflow": false,
                "human_scroll_motion_policy_owned": true,
                "wheel_delta_randomization_budget_bounded": true,
                "scroll_to_selector_requires_interaction_capability": true,
                "scroll_target_zone_policy_owned": true,
                "raw_scroll_coordinates_chat_visible": false,
                "original_method_handles_chat_visible": false,
                "raw_behavior_parameters_chat_visible": false
            })),
        "read_only_dom_probe_contract": profile_contract
            .get("read_only_dom_probe_contract")
            .cloned()
            .unwrap_or_else(|| json!({
                "version": "browser_read_only_dom_probe_contract_v1",
                "source_pattern": "cloakbrowser_isolated_world_dom_reads",
                "isolated_world_dom_reads_policy_owned": true,
                "isolated_world_context_recreated_after_navigation": true,
                "caller_supplied_probe_scripts_allowed": false,
                "probe_predicates_must_be_runtime_owned": true,
                "main_world_evaluate_fallback_telemetry_only": true,
                "read_only_probe_may_inform_extraction": true,
                "page_settling_probe_may_detect_scroll_required": true,
                "virtual_scroll_diagnostics_telemetry_only": true,
                "selector_scroll_requires_interaction_capability": true,
                "raw_probe_expression_chat_visible": false
            })),
        "launch_execution_contract": profile_contract
            .get("launch_execution_contract")
            .cloned()
            .unwrap_or_else(|| json!({
                "version": "browser_launch_execution_contract_v1",
                "source_pattern": "cloakbrowser_basic_launch_contract_tests",
                "launch_requires_admitted_adapter": true,
                "launch_success_requires_connected_browser": true,
                "close_required_after_capture": true,
                "sync_async_launch_semantic_parity_required": true,
                "page_navigation_success_is_not_evidence_without_packaging": true,
                "page_title_is_candidate_metadata_only": true,
                "raw_browser_handle_chat_visible": false,
                "raw_page_handle_chat_visible": false,
                "binary_info_telemetry_only": true,
                "fingerprint_probe_results_telemetry_only": true,
                "stealth_patch_requires_separate_capability": true
            })),
        "stealth_unit_test_contract": profile_contract
            .get("stealth_unit_test_contract")
            .cloned()
            .unwrap_or_else(|| json!({
                "version": "browser_stealth_unit_test_contract_v1",
                "source_pattern": "cloakbrowser_mock_fast_stealth_unit_tests",
                "mock_fast_tests_required": true,
                "isolated_world_lifecycle_tests_required": true,
                "isolated_world_retry_and_invalidation_tests_required": true,
                "selector_expression_escaping_tests_required": true,
                "shift_symbol_keymap_completeness_tests_required": true,
                "trusted_key_dispatch_tests_required": true,
                "page_evaluate_leak_tests_required_for_interaction_capability": true,
                "live_browser_tests_quarantined": true,
                "slow_external_site_tests_not_release_gate_by_default": true,
                "anti_bot_claims_not_inferred_from_unit_tests": true,
                "raw_detection_hooks_chat_visible": false
            })),
        "humanize_unit_test_contract": profile_contract
            .get("humanize_unit_test_contract")
            .cloned()
            .unwrap_or_else(|| json!({
                "version": "browser_humanize_unit_test_contract_v1",
                "source_pattern": "cloakbrowser_humanize_unit_tests",
                "config_resolution_tests_required": true,
                "motion_math_tests_required": true,
                "sync_async_patch_parity_tests_required": true,
                "focus_state_checks_required": true,
                "frame_locator_element_handle_patch_coverage_required": true,
                "no_double_patch_guard_required": true,
                "per_call_timeout_forwarding_tests_required": true,
                "per_call_config_override_containment_required": true,
                "non_ascii_insert_text_tests_required": true,
                "slow_behavioral_detection_tests_quarantined": true,
                "bot_detection_results_telemetry_only": true,
                "raw_behavior_detection_results_chat_visible": false
            })),
        "proxy_contract": profile_contract
            .get("proxy_contract")
            .cloned()
            .unwrap_or_else(|| json!({
                "version": "browser_proxy_capability_contract_v1",
                "source_pattern": "cloakbrowser_proxy_url_resolution",
                "default_admitted": false,
                "separate_capability_required": true,
                "direct_request_proxy_fields_allowed": false,
                "credentials_separated_from_server": true,
                "credentials_stored_by_gateway_secret_broker": true,
                "raw_proxy_url_chat_visible": false,
                "raw_proxy_credentials_chat_visible": false,
                "scheme_normalization_allowed_after_admission": true,
                "schemeless_proxy_normalized_after_admission": true,
                "credentials_removed_from_server_url": true,
                "username_only_proxy_credentials_supported": true,
                "socks_proxy_requires_adapter_arg_lane": true,
                "socks5h_scheme_supported_after_admission": true,
                "credential_percent_encoding_internal": true,
                "socks_credential_encoding_idempotent": true,
                "proxy_encoding_notice_redacts_credentials": true,
                "credential_encoding_notice_chat_visible": false,
                "proxy_bypass_list_policy_owned": true,
                "malformed_proxy_config_rejected_before_adapter": true,
                "nonstandard_socks_path_query_rejected_before_adapter": true,
                "ipv6_proxy_host_brackets_preserved_after_admission": true,
                "port_zero_proxy_requires_policy_admission": true
            })),
        "geo_consistency_contract": profile_contract
            .get("geo_consistency_contract")
            .cloned()
            .unwrap_or_else(|| json!({
                "version": "browser_geo_consistency_contract_v1",
                "source_pattern": "cloakbrowser_geoip_exit_ip_consistency",
                "default_admitted": false,
                "separate_capability_required": true,
                "depends_on_proxy_capability": true,
                "direct_request_geo_fields_allowed": false,
                "optional_geoip_dependency_required": true,
                "missing_geoip_dependency_status": "capability_dependency_missing",
                "exit_ip_resolution_timeout_bounded": true,
                "exit_ip_resolution_timeout_ms": 5000,
                "exit_ip_echo_provider_order_policy_owned": true,
                "literal_proxy_ip_extraction_telemetry_only": true,
                "invalid_proxy_url_geo_resolution_nonfatal": true,
                "exit_ip_lookup_fallback_to_proxy_host_dns_allowed": true,
                "private_proxy_host_ip_not_synthesis_evidence": true,
                "external_geo_db_download_allowed_during_research": false,
                "geo_db_first_use_download_allowed_during_research": false,
                "geo_db_download_requires_operator_readiness": true,
                "geo_db_source_admission_required": true,
                "geo_db_large_artifact_lifecycle_required": true,
                "geo_db_atomic_temp_rename_required": true,
                "geo_db_background_refresh_allowed_during_research": false,
                "geo_db_cache_lifecycle_policy_owned": true,
                "geo_db_update_interval_days": 30,
                "country_locale_map_policy_owned": true,
                "country_locale_map_bcp47_required": true,
                "locale_timezone_fill_allowed_after_admission": true,
                "fill_only_missing_locale_or_timezone": true,
                "explicit_profile_fields_take_precedence": true,
                "exit_ip_may_resolve_when_profile_fields_complete": true,
                "geo_timeout_preserves_existing_profile_fields": true,
                "webrtc_ip_auto_requires_proxy_exit_ip": true,
                "unresolved_webrtc_auto_removed_before_adapter": true,
                "private_exit_or_proxy_ip_not_claim_evidence": true,
                "raw_exit_ip_chat_visible": false,
                "raw_proxy_host_ip_chat_visible": false,
                "raw_geo_db_path_chat_visible": false,
                "socksio_missing_nonfatal": true,
                "geo_resolution_failure_nonfatal": true
            })),
        "adapter_parity_contract": profile_contract
            .get("adapter_parity_contract")
            .cloned()
            .unwrap_or_else(|| json!({
                "version": "browser_adapter_parity_contract_v1",
                "source_pattern": "cloakbrowser_puppeteer_playwright_parity",
                "policy_selects_adapter": true,
                "direct_backend_selection_allowed": false,
                "cross_adapter_semantics_required": true,
                "same_argument_compiler_required": true,
                "same_proxy_contract_required": true,
                "same_geo_consistency_contract_required": true,
                "same_humanized_interaction_gate_required": true,
                "adapter_specific_launch_options_allowed_from_caller": false,
                "adapter_specific_options_chat_visible": false,
                "proxy_auth_page_patch_requires_proxy_capability": true,
                "raw_page_patch_details_chat_visible": false
            })),
        "service_pool_contract": profile_contract
            .get("service_pool_contract")
            .cloned()
            .unwrap_or_else(|| json!({
                "version": "browser_service_pool_contract_v1",
                "source_pattern": "cloakbrowser_cdp_multiplexer_pool",
                "default_admitted": false,
                "separate_capability_required": true,
                "workflow_raw_cdp_authority_allowed": false,
                "raw_cdp_endpoint_chat_visible": false,
                "raw_remote_debugging_port_chat_visible": false,
                "public_bind_requires_gateway_admission": true,
                "host_binding_policy_owned": true,
                "per_session_identity_seed_requires_capability": true,
                "direct_fingerprint_seed_request_allowed": false,
                "seed_validation_required": true,
                "reserved_seed_blocklist_required": true,
                "per_seed_locking_required": true,
                "per_seed_process_isolation_required": true,
                "connection_refcount_required": true,
                "cleanup_confined_to_service_data_dir": true,
                "port_allocation_localhost_only": true,
                "cdp_ready_poll_timeout_ms": 10000,
                "first_launch_wins_for_session_profile": true,
                "query_param_profile_overrides_allowed": false,
                "generic_fingerprint_query_params_allowed_from_workflow": false,
                "repeated_query_param_policy_required": true,
                "service_cli_flags_policy_owned": true,
                "service_data_dir_policy_owned": true,
                "remote_debugging_cli_flags_policy_owned": true,
                "remote_debugging_cli_flags_stripped_from_passthrough": true,
                "headless_mode_policy_owned": true,
                "passthrough_browser_args_allowed_from_workflow": false,
                "ws_scheme_resolution_gateway_owned": true,
                "ws_url_rewrite_telemetry_only": true,
                "shutdown_terminates_children": true,
                "external_adapter_handoff_contract": {
                    "version": "browser_external_adapter_handoff_contract_v1",
                    "source_pattern": "cloakbrowser_cdp_integration_examples",
                    "capability_endpoint_ref_required": true,
                    "raw_cdp_http_url_from_workflow_allowed": false,
                    "raw_cdp_ws_url_from_workflow_allowed": false,
                    "remote_debugging_port_policy_owned": true,
                    "loopback_binding_required": true,
                    "adapter_kind_selected_by_tool_cd": true,
                    "agent_browser_adapter_not_core_retrieval_primitive": true,
                    "markdown_extraction_adapter_may_run_after_materialization": true,
                    "selector_fetch_adapter_may_run_after_materialization": true,
                    "adapter_output_must_reenter_evidence_pack": true,
                    "third_party_adapter_credentials_from_workflow_allowed": false,
                    "raw_adapter_trace_chat_visible": false,
                    "raw_cdp_version_response_chat_visible": false
                }
            })),
        "wrapper_lifecycle_contract": profile_contract
            .get("wrapper_lifecycle_contract")
            .cloned()
            .unwrap_or_else(|| json!({
                "version": "browser_wrapper_lifecycle_contract_v1",
                "source_pattern": "cloakbrowser_python_wrapper_lifecycle",
                "sync_async_semantic_parity_required": true,
                "close_stops_driver_instance": true,
                "context_creation_failure_closes_browser": true,
                "async_cancellation_closes_browser": true,
                "persistent_context_requires_separate_capability": true,
                "direct_persistent_user_data_dir_allowed": false,
                "backend_env_selection_policy_owned": true,
                "timezone_alias_normalized_by_policy": true,
                "direct_timezone_alias_fields_allowed": false,
                "raw_driver_instance_chat_visible": false,
                "raw_profile_path_chat_visible": false
            })),
        "default_config_contract": profile_contract
            .get("default_config_contract")
            .cloned()
            .unwrap_or_else(|| json!({
                "version": "browser_default_config_contract_v1",
                "source_pattern": "cloakbrowser_python_config_defaults",
                "platform_version_map_policy_owned": true,
                "per_platform_version_selection_runtime_owned": true,
                "platform_binary_path_template_policy_owned": true,
                "archive_extension_policy_owned": true,
                "archive_name_policy_owned": true,
                "ignored_default_args_policy_owned": true,
                "caller_ignore_default_args_allowed": false,
                "default_viewport_policy_owned": true,
                "download_base_url_policy_owned": true,
                "fallback_download_url_operator_only": true,
                "cache_dir_env_override_allowed_only_operator_readiness": true,
                "local_binary_override_env_allowed_only_operator_readiness": true,
                "stealth_default_args_separate_capability_required": true,
                "random_fingerprint_seed_not_ordinary_research": true,
                "random_seed_generation_operator_profile_only": true,
                "platform_spoofing_not_ordinary_research": true,
                "gpu_fingerprint_flags_not_policy_surface": true,
                "unsupported_platform_fails_closed": true,
                "latest_version_marker_platform_scoped": true,
                "raw_version_marker_chat_visible": false,
                "raw_cache_dir_chat_visible": false,
                "raw_download_url_chat_visible": false
            })),
        "argument_compiler": profile_contract
            .get("argument_compiler")
            .cloned()
            .unwrap_or_else(|| json!({
                "version": "browser_argument_compiler_contract_v1",
                "source_pattern": "cloakbrowser_build_args",
                "dedupe_key": "chromium_flag_name_before_equals",
                "priority_order": [
                    "policy_default_args",
                    "policy_profile_args",
                    "admitted_profile_fields"
                ],
                "caller_supplied_args_allowed": false,
                "dedicated_profile_fields_override_default_args": true,
                "single_effective_flag_per_key_required": true,
                "dedicated_locale_timezone_fields_override_compiled_args": true,
                "non_value_flags_must_be_policy_admitted": true,
                "timezone_alias_consumed_before_context_kwargs": true,
                "raw_fingerprint_args_allowed_from_workflow": false,
                "raw_webrtc_ip_arg_allowed_from_workflow": false,
                "webrtc_auto_resolution_requires_admitted_proxy_exit_ip": true,
                "unresolved_webrtc_auto_removed_before_adapter": true,
                "raw_webrtc_ip_chat_visible": false,
                "debug_override_logs_chat_visible": false
            })),
        "proxy_capability_required": profile_contract
            .get("proxy_capability_required")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        "persistent_session_capability_required": profile_contract
            .get("persistent_session_capability_required")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        "separate_admission_required_for": [
            "proxy",
            "persistent_session",
            "caller_controlled_launch_args",
            "raw_cdp_commands",
            "arbitrary_user_scripts"
        ],
        "denied_caller_fields": denied_fields,
        "denied_launch_args": denied_launch_args,
        "telemetry_fields": telemetry_fields,
        "raw_launch_args_chat_visible": false,
        "raw_browser_trace_chat_visible": false
    })
}

fn runtime_browser_materialization_metadata(root: &Path, policy: &Value) -> Value {
    let config = policy
        .pointer("/web_conduit/browser_materialization")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let dependency_lifecycle =
        config
            .get("dependency_lifecycle")
            .cloned()
            .unwrap_or_else(|| {
                json!({
                    "version": "browser_dependency_lifecycle_contract_v1",
                    "source_pattern": "cloakbrowser_platform_version_cache_contract",
                    "platform_detection": "runtime_owned",
                    "platform_tag_chat_visible": false,
                    "binary_version_source": "provider_readiness_manifest",
                    "local_binary_override_allowed": false,
                    "surprise_download_allowed": false,
                    "ordinary_research_may_install_dependency": false,
                    "install_requires_explicit_operator_action": true,
                    "cache_root_policy_owned": true,
                    "cache_cleanup_tied_to_system_cleanup": true,
                    "version_marker_may_upgrade_only_if_binary_exists": true,
                    "download_install_contract": {
                        "source_pattern": "cloakbrowser_atomic_download_checksum_extract",
                        "temp_download_required": true,
                        "partial_download_cleanup_required": true,
                        "checksum_verification_required": true,
                        "checksum_skip_allowed_for_ordinary_research": false,
                        "checksum_manifest_lookup_provider_order_policy_owned": true,
                        "checksum_manifest_missing_blocks_admitted_install": true,
                        "checksum_manifest_standard_and_binary_mode_supported": true,
                        "checksum_manifest_hashes_normalized_lowercase": true,
                        "checksum_mismatch_blocks_install": true,
                        "tar_gz_archive_supported": true,
                        "zip_archive_supported": true,
                        "primary_fallback_download_allowed_only_in_operator_install": true,
                        "custom_download_url_requires_operator_action": true,
                        "custom_download_url_disables_public_fallback": true,
                        "archive_path_traversal_rejected": true,
                        "absolute_symlink_targets_skipped": true,
                        "single_root_flatten_allowed": true,
                        "macos_app_bundle_flattening_denied": true,
                        "multiple_top_level_entries_not_flattened": true,
                        "extract_sets_executable_permissions": true,
                        "executable_permission_check_required": true,
                        "quarantine_removal_allowed_only_for_operator_install": true
                    },
                    "update_contract": {
                        "source_pattern": "cloakbrowser_rate_limited_background_update",
                        "background_update_during_ordinary_research_allowed": false,
                        "rate_limited_update_checks_required": true,
                        "update_checks_disabled_by_auto_update_env": true,
                        "update_checks_disabled_by_local_override": true,
                        "update_checks_disabled_by_custom_download_url": true,
                        "semantic_version_tuple_variable_length_allowed": true,
                        "draft_releases_ignored": true,
                        "non_chromium_release_tags_ignored": true,
                        "platform_release_asset_match_required": true,
                        "no_platform_release_asset_is_nonfatal_unavailable": true,
                        "update_check_timestamp_recorded_before_network_attempt": true,
                        "binary_update_downloaded_for_next_launch_only": true,
                        "update_marker_write_atomic_required": true,
                        "wrapper_update_check_once_per_process": true,
                        "wrapper_update_notice_chat_visible": false,
                        "update_failure_non_fatal_telemetry_only": true
                    },
                    "binary_info_contract": {
                        "source_pattern": "cloakbrowser_binary_info_projection",
                        "installed_status_may_be_telemetry": true,
                        "cached_binary_reused_when_available": true,
                        "local_binary_override_missing_fails_closed": true,
                        "install_result_revalidated_before_ready": true,
                        "raw_binary_path_chat_visible": false,
                        "raw_cache_dir_chat_visible": false,
                        "raw_download_url_chat_visible": false
                    },
                    "raw_binary_path_chat_visible": false,
                    "download_url_chat_visible": false,
                    "unsupported_platform_status": "dependency_unavailable"
                })
            });
    let enabled = config
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let selected_provider = config
        .get("provider_order")
        .and_then(Value::as_array)
        .and_then(|rows| rows.first())
        .and_then(Value::as_str)
        .map(|raw| clean_text(raw, 80))
        .filter(|raw| !raw.is_empty())
        .unwrap_or_else(|| "local_browser".to_string());
    let adapter_ready = enabled
        && config
            .get("adapter_ready")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    let status = if !enabled {
        "unavailable"
    } else if adapter_ready {
        "ready"
    } else {
        "degraded"
    };
    let blocking_reason = if !enabled {
        "capability_not_enabled"
    } else if adapter_ready {
        "none"
    } else {
        "adapter_not_ready"
    };
    let execution_gate = if !enabled {
        json!({
            "should_execute": false,
            "mode": "blocked",
            "reason": "capability_not_enabled",
            "retry_recommended": false,
            "retry_lane": "none"
        })
    } else {
        runtime_web_execution_gate(status, adapter_ready, false, blocking_reason)
    };
    let diagnostics = if enabled && !adapter_ready {
        vec![runtime_diagnostic(
            "WEB_BROWSER_MATERIALIZATION_ADAPTER_NOT_READY",
            "browser materialization is policy-enabled but no admitted browser adapter is ready; search/fetch providers remain the default path.".to_string(),
            "/web_conduit/browser_materialization",
        )]
    } else {
        Vec::new()
    };
    json!({
        "enabled": enabled,
        "selected_provider": selected_provider,
        "provider_source": "policy",
        "permission_class": config
            .get("permission_class")
            .and_then(Value::as_str)
            .unwrap_or("public_web_dynamic_read"),
        "requires_explicit_admission": config
            .get("requires_explicit_admission")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        "tool_surface_health": {
            "status": status,
            "selected_provider_ready": adapter_ready,
            "selected_provider_requires_credential": false,
            "selected_provider_credential_state": "not_required",
            "blocking_reason": blocking_reason,
            "available_provider_count": 1,
            "diagnostic_count": diagnostics.len()
        },
        "execution_gate": execution_gate,
        "capability_contract": {
            "capability_id": "browser_materialize_page",
            "optional_capability": true,
            "chat_visibility": "hidden_until_synthesized",
            "security": config.get("security").cloned().unwrap_or_else(|| json!({})),
            "output_contract": config.get("output_contract").cloned().unwrap_or_else(|| json!({})),
            "request_contract": config.get("request_contract").cloned().unwrap_or_else(|| json!({})),
            "profile_contract": config.get("profile_contract").cloned().unwrap_or_else(|| json!({})),
            "evidence_handoff": config.get("evidence_handoff").cloned().unwrap_or_else(|| json!({})),
            "readiness_lifecycle": {
                "state": if !enabled {
                    "not_configured"
                } else if adapter_ready {
                    "ready"
                } else {
                    "not_installed"
                },
                "ordinary_research_may_install_dependency": false,
                "cleanup_tied_to_system_cleanup": true,
                "dependency_lifecycle": dependency_lifecycle
            },
            "state_path": runtime_web_tools_state_path(root).display().to_string()
        },
        "profile_compilation": browser_materialization_profile_compilation_contract(
            &config,
            enabled,
            adapter_ready,
        ),
        "diagnostics": diagnostics
    })
}

pub(crate) fn runtime_web_tools_snapshot(root: &Path, policy: &Value) -> Value {
    let search = runtime_web_family_metadata(root, policy, WebProviderFamily::Search);
    let fetch = runtime_web_family_metadata(root, policy, WebProviderFamily::Fetch);
    let browser_materialization = runtime_browser_materialization_metadata(root, policy);
    let image_tool = image_tool_runtime_resolution_snapshot(root, policy, &json!({}));
    let search_status = search
        .pointer("/tool_surface_health/status")
        .and_then(Value::as_str)
        .unwrap_or("unavailable");
    let fetch_status = fetch
        .pointer("/tool_surface_health/status")
        .and_then(Value::as_str)
        .unwrap_or("unavailable");
    let search_ready = search
        .pointer("/tool_surface_health/selected_provider_ready")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let fetch_ready = fetch
        .pointer("/tool_surface_health/selected_provider_ready")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let browser_materialization_status = browser_materialization
        .pointer("/tool_surface_health/status")
        .and_then(Value::as_str)
        .unwrap_or("unavailable");
    let browser_materialization_ready = browser_materialization
        .pointer("/tool_surface_health/selected_provider_ready")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let browser_materialization_enabled = browser_materialization
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let search_execution_gate = search
        .get("execution_gate")
        .cloned()
        .unwrap_or_else(default_runtime_web_execution_gate);
    let fetch_execution_gate = fetch
        .get("execution_gate")
        .cloned()
        .unwrap_or_else(default_runtime_web_execution_gate);
    let browser_materialization_execution_gate = browser_materialization
        .get("execution_gate")
        .cloned()
        .unwrap_or_else(default_runtime_web_execution_gate);
    let overall_should_execute = search_execution_gate
        .get("should_execute")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || fetch_execution_gate
            .get("should_execute")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        || browser_materialization_execution_gate
            .get("should_execute")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    let overall_status = if search_status == "unavailable" || fetch_status == "unavailable" {
        "unavailable"
    } else if browser_materialization_enabled && browser_materialization_status == "unavailable" {
        "unavailable"
    } else if search_status == "degraded" || fetch_status == "degraded" {
        "degraded"
    } else if browser_materialization_enabled && browser_materialization_status == "degraded" {
        "degraded"
    } else {
        "ready"
    };
    let diagnostics = search
        .get("diagnostics")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .cloned()
        .chain(
            fetch
                .get("diagnostics")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .cloned(),
        )
        .chain(
            browser_materialization
                .get("diagnostics")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .cloned(),
        )
        .chain(
            image_tool
                .get("diagnostics")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .cloned(),
        )
        .collect::<Vec<_>>();
    let metadata = json!({
        "search": search,
        "fetch": fetch,
        "browser_materialization": browser_materialization,
        "image_tool": image_tool,
        "openclaw_web_tools_contract": {
            "exports": runtime_web_tools_exports_contract(),
            "default_enablement": runtime_web_tools_default_enablement_contract(),
            "fetch_unit_test_harness": runtime_web_fetch_unit_test_harness_contract()
        },
        "tool_surface_health": {
            "status": overall_status,
            "search_status": search_status,
            "fetch_status": fetch_status,
            "browser_materialization_status": browser_materialization_status,
            "search_ready": search_ready,
            "fetch_ready": fetch_ready,
            "browser_materialization_ready": browser_materialization_ready
        },
        "tool_execution_gate": {
            "search": search_execution_gate,
            "fetch": fetch_execution_gate,
            "browser_materialization": browser_materialization_execution_gate,
            "overall_should_execute": overall_should_execute,
            "overall_mode": if overall_should_execute { "allow_any" } else { "blocked_all" }
        },
        "diagnostics": diagnostics
    });
    store_active_runtime_web_tools_metadata(root, &metadata);
    metadata
}
