// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)
// WEB CONDUIT + SAFETY: fail-closed routed fetch with deterministic receipts.

use chrono::{DateTime, Utc};
use regex::Regex;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use crate::parse_args;
use crate::web_conduit_provider_runtime::{
    fetch_cache_key, fetch_provider_catalog_snapshot, fetch_provider_chain_from_request,
    fetch_provider_registration_contract, load_fetch_cache, load_search_cache,
    normalized_search_filters, provider_catalog_snapshot, provider_chain_from_request,
    provider_circuit_open_until, provider_health_snapshot, provider_requires_credential,
    recent_tool_attempt_replay_guard,
    record_provider_attempt, resolve_provider_credential_source_with_env,
    resolve_search_cache_ttl_seconds, resolve_search_count, resolve_search_provider_credential,
    resolve_search_timeout_ms, runtime_web_execution_gate, runtime_web_process_summary,
    runtime_web_replay_bypass, runtime_web_replay_guard_passthrough,
    runtime_web_replay_policy, runtime_web_request_flag, runtime_web_tools_snapshot,
    runtime_web_tools_state_path, runtime_web_truthy_flag, search_cache_key,
    search_default_timeout_ms, search_provider_registration_contract,
    search_provider_request_contract, store_fetch_cache, store_search_cache,
    unsupported_search_filter_response, validate_explicit_fetch_provider_hint,
    validate_explicit_provider_hint, web_provider_public_artifact_contracts,
    web_tool_catalog_snapshot, WebProviderFamily,
};

const POLICY_REL: &str = "core/layer0/ops/config/web_conduit_policy.json";
const LEGACY_POLICY_REL: &str = "client/runtime/config/web_conduit_policy.json";
const RECEIPTS_REL: &str = "client/runtime/local/state/web_conduit/receipts.jsonl";
const APPROVALS_REL: &str = "client/runtime/local/state/ui/infring_dashboard/approvals.json";
const ARTIFACTS_DIR_REL: &str = "client/runtime/local/state/web_conduit/artifacts";
const DEFAULT_ACCEPT_LANGUAGE: &str = "en-US,en;q=0.9";
const DEFAULT_REFERER: &str = "https://www.google.com/";
const DEFAULT_WEB_USER_AGENTS: &[&str] = &[
    "Infring-WebConduit/1.0",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_5) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36",
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:125.0) Gecko/20100101 Firefox/125.0",
];
const SERPER_SEARCH_URL: &str = "https://google.serper.dev/search";
const TAVILY_SEARCH_URL: &str = "https://api.tavily.com/search";
const EXA_SEARCH_URL: &str = "https://api.exa.ai/search";
const BRAVE_SEARCH_URL: &str = "https://api.search.brave.com/res/v1/web/search";

fn usage() {
    println!("web-conduit commands:");
    println!("  infring-ops web-conduit status");
    println!("  infring-ops web-conduit receipts [--limit=<n>]");
    println!(
        "  infring-ops web-conduit setup [--provider=<tavily|exa|brave|serperdev|google-news|duckduckgo|duckduckgo-lite|bing>] [--api-key=<key>] [--api-key-env=<ENV>] [--apply=1] [--summary-only=1]"
    );
    println!(
        "  infring-ops web-conduit migrate-legacy-config [--source-path=<path>] [--apply=1] [--summary-only=1]"
    );
    println!(
        "  infring-ops web-conduit native-codex [--model-provider=<id>] [--model-api=<id>] [--payload-json=<json>] [--summary-only=1]"
    );
    println!(
        "  infring-ops web-conduit fetch --url=<https://...> [--provider=auto|direct-http|curl] [--extract-mode=text|markdown] [--max-chars=<n>] [--cache-ttl-minutes=<n>] [--timeout-ms=<n>] [--max-response-bytes=<n>] [--resolve-citation-redirect=1] [--human-approved=1] [--approval-id=<id>] [--summary-only=1]"
    );
    println!(
        "  infring-ops web-conduit browser-materialize --url=<https://...> --admission-ref=<capability-ref> [--extract-mode=text|markdown] [--wait-until=<event>] [--wait-for-selector=<selector>] [--profile-ref=<profile>] [--timeout-ms=<n>] [--max-response-bytes=<n>] [--summary-only=1]"
    );
    println!(
        "  infring-ops web-conduit media --url=<https://...|file://...>|--path=<local-path> [--workspace-dir=<path>] [--local-roots=<path,...>|any] [--host-read-capability=1] [--max-bytes=<n>] [--optimize-images=1] [--raw=1] [--resolve-citation-redirect=1] [--human-approved=1] [--approval-id=<id>] [--summary-only=1]"
    );
    println!(
        "  infring-ops web-conduit audio-probe --url=<https://...|file://...>|--path=<local-path> [--workspace-dir=<path>] [--local-roots=<path,...>|any] [--host-read-capability=1] [--max-bytes=<n>]"
    );
    println!(
        "  infring-ops web-conduit pdf-extract --url=<https://...|file://...>|--path=<local-path> [--max-pages=<n>] [--page-numbers=1,2] [--min-text-chars=<n>] [--extract-images=1] [--summary-only=1]"
    );
    println!(
        "  infring-ops web-conduit pdf-native-analyze --provider=<anthropic|google> --model-id=<id> --prompt='<text>' [--path=<pdf>|--url=<pdf>|--sources-json=<json>] [--api-key=<key>|--api-key-env=<ENV>] [--base-url=<url>] [--max-tokens=<n>] [--summary-only=1]"
    );
    println!(
        "  infring-ops web-conduit pdf-tool [--prompt='<text>'] [--model=<provider/model>|--provider=<provider> --model-id=<id>] [--path=<pdf>|--url=<pdf>|--pdf=<pdf>|--pdfs-json=<json>] [--pages=1-3,5] [--max-bytes-mb=<n>] [--max-pages=<n>] [--min-text-chars=<n>] [--api-key=<key>|--api-key-env=<ENV>] [--summary-only=1]"
    );
    println!(
        "  infring-ops web-conduit image-metadata --url=<https://...|file://...>|--path=<local-path> [--workspace-dir=<path>] [--local-roots=<path,...>|any] [--host-read-capability=1] [--max-bytes=<n>] [--summary-only=1]"
    );
    println!(
        "  infring-ops web-conduit image-tool-status [--provider=<id>] [--model=<provider/model|model>] [--summary-only=1]"
    );
    println!(
        "  infring-ops web-conduit image-tool [--prompt='<text>'] [--provider=<id>] [--model=<provider/model|model>] [--image=<path|url>|--images-json=<json>|--path=<path>|--url=<url>] [--max-images=<n>] [--max-bytes=<n>] [--timeout-seconds=<n>] [--max-tokens=<n>] [--summary-only=1]"
    );
    println!(
        "  infring-ops web-conduit attachments [--context-json=<json>] [--attachments-json=<json>] [--media-path=<path>|--media-url=<url>|--media-type=<mime>] [--media-paths-json=<json>] [--media-urls-json=<json>] [--media-types-json=<json>] [--already-transcribed-indices=0,2] [--capability=image|audio|video] [--prefer=first|last|path|url] [--mode=first|all] [--max-attachments=<n>] [--summary-only=1]"
    );
    println!(
        "  infring-ops web-conduit media-host --url=<https://...|file://...>|--path=<local-path> [--workspace-dir=<path>] [--local-roots=<path,...>|any] [--ttl-seconds=<n>] [--base-url=<url>] [--summary-only=1]"
    );
    println!(
        "  infring-ops web-conduit outbound-attachment --url=<https://...|file://...>|--path=<local-path> [--workspace-dir=<path>] [--local-roots=<path,...>|any] [--host-read-capability=1] [--optimize-images=1] [--raw=1] [--summary-only=1]"
    );
    println!("  infring-ops web-conduit parse-media --text='<output with MEDIA:... tokens>'");
    println!(
        "  infring-ops web-conduit qr-image --text='<text>' [--scale=<n>] [--margin-modules=<n>] [--prompt-image-order=inline|offloaded] [--summary-only=1]"
    );
    println!("  infring-ops web-conduit file-context --content='<text>' [--content-base64=<base64>] [--file-name=<name>] [--mime-type=<type>] [--fallback-name=<name>] [--compact=1]");
    println!(
        "  infring-ops web-conduit search --query=<terms> [--provider=auto|serper|browser-serp|google-news|duckduckgo|duckduckgo-lite|bing] [--top-k=8|--count=8] [--timeout-ms=<n>] [--cache-ttl-minutes=<n>] [--allowed-domains=docs.rs,github.com] [--exact-domain-only=1] [--country=<code>] [--language=<code>] [--freshness=<token>] [--date-after=<YYYY-MM-DD>] [--date-before=<YYYY-MM-DD>] [--human-approved=1] [--summary-only=1]"
    );
    println!("  infring-ops web-conduit providers");
    println!("  infring-ops browse fetch --url=<https://...>");
}

fn clean_text(raw: &str, max_len: usize) -> String {
    raw.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max_len.max(1))
        .collect::<String>()
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
            .map_err(|err| format!("web_conduit_create_policy_dir_failed:{err}"))?;
    }
    let tmp = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        Utc::now().timestamp_millis()
    ));
    let encoded = serde_json::to_vec_pretty(value)
        .map_err(|err| format!("web_conduit_encode_policy_failed:{err}"))?;
    fs::write(&tmp, encoded).map_err(|err| format!("web_conduit_write_policy_tmp_failed:{err}"))?;
    fs::rename(&tmp, path).map_err(|err| format!("web_conduit_rename_policy_failed:{err}"))?;
    Ok(())
}

fn append_jsonl(path: &Path, row: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("web_conduit_create_state_dir_failed:{err}"))?;
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| format!("web_conduit_open_receipts_failed:{err}"))?;
    let line = serde_json::to_string(row)
        .map_err(|err| format!("web_conduit_encode_receipt_failed:{err}"))?;
    writeln!(file, "{line}").map_err(|err| format!("web_conduit_append_receipt_failed:{err}"))?;
    Ok(())
}

fn parse_bool(value: Option<&String>) -> bool {
    value
        .map(|raw| {
            matches!(
                raw.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

fn parse_u64(value: Option<&String>, fallback: u64, min: u64, max: u64) -> u64 {
    value
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .unwrap_or(fallback)
        .clamp(min, max)
}

fn env_policy_path() -> Option<PathBuf> {
    if let Ok(raw) = std::env::var("INFRING_WEB_CONDUIT_POLICY_PATH") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return Some(PathBuf::from(trimmed));
        }
    }
    None
}

fn policy_path(root: &Path) -> PathBuf {
    if let Some(path) = env_policy_path() {
        return path;
    }
    root.join(POLICY_REL)
}

fn legacy_policy_path(root: &Path) -> PathBuf {
    root.join(LEGACY_POLICY_REL)
}

fn receipts_path(root: &Path) -> PathBuf {
    root.join(RECEIPTS_REL)
}

fn approvals_path(root: &Path) -> PathBuf {
    root.join(APPROVALS_REL)
}

fn artifacts_dir_path(root: &Path) -> PathBuf {
    root.join(ARTIFACTS_DIR_REL)
}

fn default_policy() -> Value {
    json!({
        "version": "v1",
        "mode": "production",
        "web_conduit": {
            "enabled": true,
            "max_response_bytes": 350000,
            "timeout_ms": 9000,
            "rate_limit_per_minute": 30,
            "allow_domains": [],
            "deny_domains": [
                "127.0.0.1",
                "localhost",
                "metadata.google.internal",
                "169.254.169.254"
            ],
            "sensitive_domains": [
                "accounts.google.com",
                "api.stripe.com",
                "paypal.com",
                "chase.com",
                "bankofamerica.com"
            ],
            "require_human_for_sensitive": true,
            "search_default_count": 8,
            "search_max_count": 12,
            "search_cache_ttl_minutes": 8,
            "search_provider_order": ["tavily", "exa", "brave", "serperdev", "google_news_rss", "bing_rss", "duckduckgo_lite", "duckduckgo"],
            "fetch_provider_order": ["direct_http"],
                "browser_materialization": {
                    "enabled": false,
                    "adapter_ready": false,
                    "live_execution_enabled": false,
                    "provider_order": ["local_browser"],
                    "requires_explicit_admission": true,
                    "permission_class": "public_web_dynamic_read",
                    "local_static_fixture": {
                        "source": "policy_owned_test_fixture",
                        "fixture_url": "",
                        "final_url": "",
                        "fixture_rel_path": "",
                        "fixture_path_chat_visible": false,
                        "raw_fixture_payload_chat_visible": false
                    },
                    "local_js_rendered_fixture": {
                        "source": "policy_owned_test_fixture",
                        "fixture_url": "",
                        "final_url": "",
                        "fixture_rel_path": "",
                        "rendered_text_rel_path": "",
                        "rendered_marker": "",
                        "fixture_path_chat_visible": false,
                        "raw_fixture_payload_chat_visible": false,
                        "raw_rendered_fixture_payload_chat_visible": false,
                        "caller_supplied_script_allowed": false
                    },
                    "default_timeout_ms": 30000,
                    "max_response_bytes": 350000,
                "request_contract": {
                    "required_fields": ["url", "admission_ref"],
                    "optional_fields": [
                        "request_id",
                        "extract_mode",
                        "wait_until",
                        "wait_for_selector",
                        "timeout_ms",
                        "max_response_bytes",
                        "profile_ref",
                        "evidence_gap_reason"
                    ],
                    "denied_fields": [
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
                        "script",
                        "scripts",
                        "javascript",
                        "evaluate",
                        "evaluate_script",
                        "wait_script",
                        "raw_wait_script",
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
                        "raw_html",
                        "rawHtml",
                        "html",
                        "raw_payload",
                        "rawPayload",
                        "screenshot",
                        "screenshot_bytes",
                        "screenshotBytes",
                        "browser_trace",
                        "browserTrace",
                        "console_logs",
                        "consoleLogs",
                        "network_logs",
                        "networkLogs",
                        "trace"
                    ]
                },
                "profile_contract": {
                    "profile_source": "tool_cd_policy",
                    "default_profile": "stateless_public_materialization",
                    "state_scope": "stateless",
                    "proxy_capability_required": true,
                    "persistent_session_capability_required": true,
                    "caller_override_allowed": false,
                    "context_options_allowed_from_caller": false,
                    "context_conflict_fields": [
                        "locale",
                        "timezone",
                        "timezoneId",
                        "timezone_id",
                        "viewport",
                        "userAgent",
                        "user_agent",
                        "colorScheme"
                    ],
                    "close_browser_on_context_creation_failure": true,
                    "context_close_closes_browser": true,
                    "default_viewport_policy_owned": true,
                    "caller_viewport_allowed": false,
                    "caller_user_agent_allowed": false,
                    "caller_color_scheme_allowed": false,
                    "locale_timezone_cdp_emulation_allowed": false,
                    "locale_timezone_binary_profile_fields_policy_owned": true,
                    "geoip_filled_profile_fields_require_proxy_capability": true,
                    "generic_context_kwargs_allowed_from_workflow": false,
                    "storage_state_requires_session_capability": true,
                    "persistent_context_allowed": false,
                    "persistent_session_contract": {
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
                    },
                    "humanized_interaction_allowed": false,
                    "human_interaction_contract": {
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
                    },
                    "read_only_dom_probe_contract": {
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
                    },
                    "launch_execution_contract": {
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
                    },
                    "stealth_unit_test_contract": {
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
                    },
                    "humanize_unit_test_contract": {
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
                    },
                    "proxy_contract": {
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
                    },
                    "geo_consistency_contract": {
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
                    },
                    "adapter_parity_contract": {
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
                    },
                    "service_pool_contract": {
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
                    },
                    "wrapper_lifecycle_contract": {
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
                    },
                    "default_config_contract": {
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
                    },
                    "argument_compiler": {
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
                    },
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
                },
                "dependency_lifecycle": {
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
                },
                "smart_wait": {
                    "enabled": true,
                    "dom_stable_ms": 1500,
                    "max_settle_ms": 15000
                },
                "security": {
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
                        "blocked_url_credentials",
                        "blocked_internal_host_hint",
                        "credentials_redacted",
                        "invalid_url"
                    ]
                },
                "output_contract": {
                    "fields": [
                        "source_url",
                        "pre_navigation_url_safety",
                        "final_url",
                        "final_url_safety",
                        "status_code",
                        "title",
                        "main_text_or_markdown",
                        "content_truncated",
                        "extractor",
                        "links_summary",
                        "blocker_classification",
                        "extraction_confidence",
                        "artifact_ref",
                        "artifact_manifest_ref",
                        "evidence_pack_candidates",
                        "evidence_refs",
                        "web_tooling_gates",
                        "readiness_strategy",
                        "cleanup_status",
                        "retry_diagnostics",
                        "local_static_fixture",
                        "local_js_rendered_fixture",
                        "js_render_proof"
                    ],
                    "chat_visible": false
                },
                "evidence_handoff": {
                    "target_lane": "candidate_enrichment",
                    "promotion_requires": [
                        "safe_final_url",
                        "substantive_main_text",
                        "query_relevance",
                        "not_blocker_shell"
                    ],
                    "confidence_values": ["usable", "low_confidence_raw", "rejected"],
                    "raw_payload_chat_visible": false
                }
            },
            "provider_circuit_breaker": {
                "enabled": true,
                "failure_threshold": 3,
                "open_for_secs": 300
            },
            "native_codex_web_search": {
                "enabled": false,
                "mode": "cached",
                "allowed_domains": []
            },
            "image_tool": {
                "enabled": true,
                "provider": "",
                "model": "",
                "default_prompt": "Describe the image.",
                "max_images": 20,
                "max_bytes": 10485760,
                "timeout_seconds": 60,
                "output_max_buffer_bytes": 5242880,
                "media_concurrency": 2
            }
        }
    })
}

fn load_policy(root: &Path) -> (Value, PathBuf) {
    if let Some(path) = env_policy_path() {
        if !path.exists() {
            let _ = write_json_atomic(&path, &default_policy());
        }
        return (read_json_or(&path, default_policy()), path);
    }
    let path = policy_path(root);
    if path.exists() {
        return (read_json_or(&path, default_policy()), path);
    }
    let legacy_path = legacy_policy_path(root);
    if legacy_path.exists() {
        return (read_json_or(&legacy_path, default_policy()), legacy_path);
    }
    let _ = write_json_atomic(&path, &default_policy());
    (read_json_or(&path, default_policy()), path)
}

fn load_approvals(root: &Path) -> Vec<Value> {
    let path = approvals_path(root);
    let raw = read_json_or(&path, json!({"approvals": []}));
    raw.get("approvals")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn save_approvals(root: &Path, approvals: &[Value]) -> Result<(), String> {
    write_json_atomic(
        &approvals_path(root),
        &json!({
            "type": "infring_dashboard_approvals",
            "updated_at": crate::now_iso(),
            "approvals": approvals
        }),
    )
}

fn approval_state_for_request(
    root: &Path,
    approval_id: &str,
    requested_url: &str,
) -> Option<String> {
    let approval_key = clean_text(approval_id, 160);
    if approval_key.is_empty() {
        return None;
    }
    let url_key = clean_text(requested_url, 2200);
    for row in load_approvals(root) {
        let row_id = clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 160);
        if row_id != approval_key {
            continue;
        }
        let row_url = clean_text(
            row.get("requested_url")
                .and_then(Value::as_str)
                .unwrap_or(""),
            2200,
        );
        if !row_url.is_empty() && !url_key.is_empty() && row_url != url_key {
            return Some("mismatched".to_string());
        }
        let state = clean_text(
            row.get("status")
                .and_then(Value::as_str)
                .unwrap_or("pending"),
            40,
        )
        .to_ascii_lowercase();
        return if state.is_empty() {
            Some("pending".to_string())
        } else {
            Some(state)
        };
    }
    None
}

fn ensure_sensitive_web_approval(
    root: &Path,
    requested_url: &str,
    policy_eval: &Value,
) -> Option<Value> {
    let requested = clean_text(requested_url, 2200);
    if requested.is_empty() {
        return None;
    }
    let domain = extract_domain(&requested);
    let approval_id = format!(
        "approval-web-{}",
        &sha256_hex(&format!("{}:{}", domain, requested))[..10]
    );
    let mut approvals = load_approvals(root);
    if let Some(existing) = approvals
        .iter()
        .find(|row| {
            clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 160) == approval_id
                && clean_text(
                    row.get("requested_url")
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                    2200,
                ) == requested
                && clean_text(
                    row.get("status")
                        .and_then(Value::as_str)
                        .unwrap_or("pending"),
                    40,
                )
                .to_ascii_lowercase()
                    == "pending"
        })
        .cloned()
    {
        return Some(existing);
    }
    let now = crate::now_iso();
    let row = json!({
        "id": approval_id,
        "action": "Web fetch approval",
        "description": format!("Approve governed web fetch for {}.", requested),
        "agent_name": "web_conduit",
        "status": "pending",
        "domain": domain,
        "requested_url": requested,
        "policy_reason": clean_text(policy_eval.get("reason").and_then(Value::as_str).unwrap_or("human_approval_required_for_sensitive_domain"), 180),
        "created_at": now,
        "updated_at": now
    });
    approvals.push(row.clone());
    let _ = save_approvals(root, &approvals);
    Some(row)
}

fn read_recent_receipts(root: &Path, limit: usize) -> Vec<Value> {
    let raw = fs::read_to_string(receipts_path(root)).unwrap_or_default();
    raw.lines()
        .rev()
        .take(limit.max(1))
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect::<Vec<_>>()
}

fn receipt_count(root: &Path) -> usize {
    fs::read_to_string(receipts_path(root))
        .ok()
        .map(|raw| raw.lines().count())
        .unwrap_or(0)
}

fn receipt_counts_against_rate_limit(row: &Value) -> bool {
    let policy_decision = clean_text(
        row.get("policy_decision")
            .and_then(Value::as_str)
            .unwrap_or(""),
        40,
    )
    .to_ascii_lowercase();
    let error = clean_text(
        row.get("error").and_then(Value::as_str).unwrap_or(""),
        80,
    )
    .to_ascii_lowercase();
    let status_code = row
        .get("status_code")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let response_hash = clean_text(
        row.get("response_hash")
            .and_then(Value::as_str)
            .unwrap_or(""),
        96,
    );

    if error == "policy_denied" {
        return false;
    }
    if status_code > 0 || !response_hash.is_empty() {
        return true;
    }
    policy_decision == "allow"
}

fn receipt_rate_limit_lane(row: &Value) -> &'static str {
    let policy_reason = clean_text(
        row.get("policy_reason")
            .and_then(Value::as_str)
            .unwrap_or(""),
        120,
    )
    .to_ascii_lowercase();
    if policy_reason == "search_provider_chain" {
        return "search";
    }

    let requested_url = clean_text(
        row.get("requested_url")
            .and_then(Value::as_str)
            .unwrap_or(""),
        2200,
    )
    .to_ascii_lowercase();
    let domain = extract_domain(&requested_url);
    let is_search_provider_url = matches!(
        domain.as_str(),
        "google.serper.dev" | "api.tavily.com" | "api.exa.ai" | "api.search.brave.com"
    ) || domain.ends_with("duckduckgo.com")
        || ((domain == "www.bing.com" || domain == "bing.com")
            && requested_url.contains("/search"))
        || (domain == "news.google.com" && requested_url.contains("/rss/search"));

    if is_search_provider_url {
        "search"
    } else {
        "fetch"
    }
}

fn requests_last_minute(root: &Path) -> u64 {
    requests_last_minute_for_lane(root, "all")
}

fn requests_last_minute_for_lane(root: &Path, lane: &str) -> u64 {
    let now = Utc::now();
    let mut count = 0u64;
    for row in read_recent_receipts(root, 400) {
        if !receipt_counts_against_rate_limit(&row) {
            continue;
        }
        if lane != "all" && receipt_rate_limit_lane(&row) != lane {
            continue;
        }
        let ts = row
            .get("timestamp")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let Ok(parsed) = DateTime::parse_from_rfc3339(ts) else {
            continue;
        };
        let age = now.signed_duration_since(parsed.with_timezone(&Utc));
        if age.num_seconds() <= 60 {
            count = count.saturating_add(1);
        }
    }
    count
}
