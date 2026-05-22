fn search_cache_path(root: &Path) -> PathBuf {
    runtime_state_path(root, SEARCH_CACHE_REL)
}

fn cache_ttl_for_status(status: &str) -> i64 {
    if status == "ok" || status == "partial" {
        SEARCH_CACHE_TTL_SUCCESS_SECS
    } else {
        SEARCH_CACHE_TTL_NO_RESULTS_SECS
    }
}

pub(crate) fn search_cache_key(
    query: &str,
    effective_query: &str,
    allowed_domains: &[String],
    exclude_subdomains: bool,
    top_k: usize,
    summary_only: bool,
    provider_chain: &[String],
) -> String {
    crate::deterministic_receipt_hash(&json!({
        "version": 2,
        "query": clean_text(query, 900),
        "effective_query": clean_text(effective_query, 900),
        "allowed_domains": allowed_domains,
        "exclude_subdomains": exclude_subdomains,
        "top_k": top_k,
        "summary_only": summary_only,
        "provider_chain": provider_chain
    }))
}

pub(crate) fn load_search_cache(root: &Path, key: &str) -> Option<Value> {
    let path = search_cache_path(root);
    let mut cache = read_json_or(&path, default_search_cache_state());
    let now_ts = Utc::now().timestamp();
    let mut mutated = false;
    let mut hit = None::<Value>;
    if let Some(entries) = cache.get_mut("entries").and_then(Value::as_object_mut) {
        let stale_keys = entries
            .iter()
            .filter_map(|(entry_key, entry)| {
                let expires_at = entry.get("expires_at").and_then(Value::as_i64).unwrap_or(0);
                if expires_at <= now_ts {
                    Some(entry_key.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        for stale_key in stale_keys {
            entries.remove(&stale_key);
            mutated = true;
        }
        if let Some(entry) = entries.get_mut(key) {
            if let Some(row) = entry.get("response") {
                hit = Some(row.clone());
            }
            if let Some(obj) = entry.as_object_mut() {
                obj.insert("last_hit_at".to_string(), json!(now_ts));
            }
            mutated = true;
        }
    }
    if mutated {
        let _ = write_json_atomic(&path, &cache);
    }
    hit
}

pub(crate) fn store_search_cache(
    root: &Path,
    key: &str,
    response: &Value,
    status: &str,
    ttl_override_seconds: Option<i64>,
) {
    let path = search_cache_path(root);
    let mut cache = read_json_or(&path, default_search_cache_state());
    let now_ts = Utc::now().timestamp();
    let mut entries = cache
        .get("entries")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    entries
        .retain(|_, entry| entry.get("expires_at").and_then(Value::as_i64).unwrap_or(0) > now_ts);
    let ttl = ttl_override_seconds
        .unwrap_or_else(|| cache_ttl_for_status(status))
        .max(30);
    entries.insert(
        key.to_string(),
        json!({
            "stored_at": now_ts,
            "last_hit_at": now_ts,
            "expires_at": now_ts + ttl,
            "status": clean_text(status, 40),
            "response": response
        }),
    );
    if entries.len() > SEARCH_CACHE_MAX_ENTRIES {
        let mut order = entries
            .iter()
            .map(|(entry_key, entry)| {
                (
                    entry_key.clone(),
                    entry
                        .get("last_hit_at")
                        .and_then(Value::as_i64)
                        .or_else(|| entry.get("stored_at").and_then(Value::as_i64))
                        .unwrap_or(0),
                )
            })
            .collect::<Vec<_>>();
        order.sort_by_key(|(_, used_at)| *used_at);
        let drop_count = entries.len().saturating_sub(SEARCH_CACHE_MAX_ENTRIES);
        for (entry_key, _) in order.into_iter().take(drop_count) {
            entries.remove(&entry_key);
        }
    }
    cache["version"] = json!(1);
    cache["entries"] = Value::Object(entries);
    let _ = write_json_atomic(&path, &cache);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_chain_prefers_hint_then_policy_then_defaults() {
        let request = json!({});
        let policy = json!({
            "web_conduit": {
                "search_provider_order": ["duckduckgo", "bing_rss"]
            }
        });
        let chain = provider_chain_from_request("serper", &request, &policy);
        assert_eq!(
            chain,
            vec![
                "serperdev".to_string(),
                "duckduckgo".to_string(),
                "bing_rss".to_string(),
                "tavily".to_string(),
                "exa".to_string(),
                "brave".to_string(),
                "google_news_rss".to_string(),
                "duckduckgo_lite".to_string()
            ]
        );
    }

    #[test]
    fn provider_chain_auto_reorders_missing_credential_providers_to_tail() {
        let request = json!({});
        let policy = json!({
            "web_conduit": {
                "search_provider_order": ["serperdev", "duckduckgo", "bing_rss"]
            }
        });
        let chain = provider_chain_from_request_with_env("", &request, &policy, |_key| None);
        assert_eq!(
            chain,
            vec![
                "duckduckgo".to_string(),
                "bing_rss".to_string(),
                "google_news_rss".to_string(),
                "duckduckgo_lite".to_string(),
                "serperdev".to_string(),
                "tavily".to_string(),
                "exa".to_string(),
                "brave".to_string()
            ]
        );
    }

    #[test]
    fn provider_chain_auto_keeps_credentialed_provider_first_when_key_present() {
        let request = json!({});
        let policy = json!({
            "web_conduit": {
                "search_provider_order": ["serperdev", "duckduckgo", "bing_rss"]
            }
        });
        let chain = provider_chain_from_request_with_env("", &request, &policy, |key| {
            if key == "SERPER_API_KEY" {
                Some("test-key".to_string())
            } else {
                None
            }
        });
        assert_eq!(chain.first().map(String::as_str), Some("serperdev"));
    }

    #[test]
    fn provider_chain_strict_request_does_not_append_default_fallbacks() {
        let request = json!({
            "search_provider_chain": ["duckduckgo_lite", "duckduckgo"],
            "search_provider_chain_strict": true
        });
        let policy = json!({
            "web_conduit": {
                "search_provider_order": ["google_news_rss", "bing_rss", "duckduckgo"]
            }
        });
        let chain = provider_chain_from_request("", &request, &policy);
        assert_eq!(
            chain,
            vec!["duckduckgo_lite".to_string(), "duckduckgo".to_string()]
        );
    }

    #[test]
    fn explicit_duckduckgo_lite_hint_stays_explicit() {
        let request = json!({});
        let policy = json!({
            "web_conduit": {
                "search_provider_order": ["google_news_rss", "bing_rss", "duckduckgo"]
            }
        });
        let chain = provider_chain_from_request("duckduckgo_lite", &request, &policy);
        assert_eq!(chain.first().map(String::as_str), Some("duckduckgo_lite"));
    }

    #[test]
    fn explicit_browser_serp_hint_stays_available_without_default_chain() {
        let request = json!({});
        let policy = json!({
            "web_conduit": {
                "search_provider_order": ["google_news_rss", "bing_rss", "duckduckgo"]
            }
        });
        let chain = provider_chain_from_request("browser_serp", &request, &policy);
        assert_eq!(chain.first().map(String::as_str), Some("browser_serp"));
    }

    #[test]
    fn explicit_provider_hint_rejects_unknown_provider() {
        assert_eq!(
            validate_explicit_provider_hint("perplexity"),
            Some("perplexity".to_string())
        );
        assert_eq!(validate_explicit_provider_hint("auto"), None);
        assert_eq!(validate_explicit_provider_hint("duckduckgo-lite"), None);
    }

    #[test]
    fn fetch_provider_chain_prefers_explicit_alias_then_policy_then_defaults() {
        let request = json!({});
        let policy = json!({
            "web_conduit": {
                "fetch_provider_order": ["direct_http"]
            }
        });
        let chain = fetch_provider_chain_from_request("curl", &request, &policy);
        assert_eq!(chain, vec!["direct_http".to_string()]);
    }

    #[test]
    fn explicit_fetch_provider_hint_rejects_unknown_provider() {
        assert_eq!(
            validate_explicit_fetch_provider_hint("firecrawl"),
            Some("firecrawl".to_string())
        );
        assert_eq!(validate_explicit_fetch_provider_hint("auto"), None);
        assert_eq!(validate_explicit_fetch_provider_hint("curl"), None);
    }

    #[test]
    fn provider_catalog_snapshot_reports_aliases_and_availability() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = json!({
            "web_conduit": {
                "search_provider_order": ["serperdev", "duckduckgo", "bing_rss"]
            }
        });
        let catalog = provider_catalog_snapshot(tmp.path(), &policy);
        let rows = catalog.as_array().expect("catalog rows");
        let default = rows
            .iter()
            .find(|row| row.get("selected_by_default").and_then(Value::as_bool) == Some(true))
            .expect("default provider");
        assert_eq!(default.get("provider").and_then(Value::as_str), Some("duckduckgo"));
        let serper = rows
            .iter()
            .find(|row| row.get("provider").and_then(Value::as_str) == Some("serperdev"))
            .expect("serper row");
        assert_eq!(
            serper.get("requires_credential").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            serper.get("credential_present").and_then(Value::as_bool),
            Some(false)
        );
        assert!(serper
            .get("aliases")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().any(|row| row.as_str() == Some("serper")))
            .unwrap_or(false));
    }

    #[test]
    fn fetch_provider_catalog_snapshot_reports_direct_http() {
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
            default.get("provider").and_then(Value::as_str),
            Some("direct_http")
        );
        assert_eq!(default.get("family").and_then(Value::as_str), Some("fetch"));
        assert_eq!(default.get("source").and_then(Value::as_str), Some("http_get"));
        assert!(default
            .get("aliases")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().any(|row| row.as_str() == Some("curl")))
            .unwrap_or(false));
    }

    #[test]
    fn circuit_breaker_opens_after_threshold_failures() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = json!({
            "web_conduit": {
                "provider_circuit_breaker": {
                    "enabled": true,
                    "failure_threshold": 2,
                    "open_for_secs": 120
                }
            }
        });
        record_provider_attempt(tmp.path(), "serperdev", false, "timeout", &policy);
        assert!(provider_circuit_open_until(tmp.path(), "serperdev", &policy).is_none());
        record_provider_attempt(tmp.path(), "serperdev", false, "timeout", &policy);
        assert!(provider_circuit_open_until(tmp.path(), "serperdev", &policy).is_some());
        record_provider_attempt(tmp.path(), "serperdev", true, "", &policy);
        assert!(provider_circuit_open_until(tmp.path(), "serperdev", &policy).is_none());
    }

    #[test]
    fn circuit_breaker_fast_opens_for_access_or_throttle_failures() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = json!({
            "web_conduit": {
                "provider_circuit_breaker": {
                    "enabled": true,
                    "failure_threshold": 6,
                    "open_for_secs": 120
                }
            }
        });
        record_provider_attempt(tmp.path(), "duckduckgo", false, "http_429 rate_limited", &policy);
        assert!(provider_circuit_open_until(tmp.path(), "duckduckgo", &policy).is_some());
    }

    #[test]
    fn circuit_breaker_ignores_query_quality_failures() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = json!({
            "web_conduit": {
                "provider_circuit_breaker": {
                    "enabled": true,
                    "failure_threshold": 2,
                    "open_for_secs": 120
                }
            }
        });
        record_provider_attempt(tmp.path(), "duckduckgo", false, "low_signal_search_payload", &policy);
        record_provider_attempt(tmp.path(), "duckduckgo", false, "query_result_mismatch", &policy);
        record_provider_attempt(tmp.path(), "duckduckgo", false, "no_usable_summary", &policy);
        assert!(provider_circuit_open_until(tmp.path(), "duckduckgo", &policy).is_none());
        let rows = provider_health_snapshot(tmp.path(), &[String::from("duckduckgo")])
            .as_array()
            .cloned()
            .unwrap_or_default();
        let row = rows.first().cloned().unwrap_or_else(|| json!({}));
        assert_eq!(row.get("circuit_open").and_then(Value::as_bool), Some(false));
        assert_eq!(row.get("consecutive_failures").and_then(Value::as_u64), Some(0));
    }

    #[test]
    fn circuit_breaker_ignores_no_relevant_results_failures() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = json!({
            "web_conduit": {
                "provider_circuit_breaker": {
                    "enabled": true,
                    "failure_threshold": 2,
                    "open_for_secs": 120
                }
            }
        });
        record_provider_attempt(tmp.path(), "google_news_rss", false, "no_relevant_results", &policy);
        record_provider_attempt(tmp.path(), "google_news_rss", false, "low_relevance", &policy);
        record_provider_attempt(tmp.path(), "google_news_rss", false, "no relevant results", &policy);
        assert!(provider_circuit_open_until(tmp.path(), "google_news_rss", &policy).is_none());
        let rows = provider_health_snapshot(tmp.path(), &[String::from("google_news_rss")])
            .as_array()
            .cloned()
            .unwrap_or_default();
        let row = rows.first().cloned().unwrap_or_else(|| json!({}));
        assert_eq!(row.get("circuit_open").and_then(Value::as_bool), Some(false));
        assert_eq!(row.get("consecutive_failures").and_then(Value::as_u64), Some(0));
        let state = load_provider_health(tmp.path());
        assert_eq!(
            state
                .pointer("/providers/google_news_rss/last_failure_class")
                .and_then(Value::as_str),
            Some("query_quality")
        );
    }

    #[test]
    fn circuit_breaker_ignores_missing_credential_failures() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = json!({
            "web_conduit": {
                "provider_circuit_breaker": {
                    "enabled": true,
                    "failure_threshold": 2,
                    "open_for_secs": 120
                }
            }
        });
        record_provider_attempt(tmp.path(), "tavily", false, "tavily_api_key_missing", &policy);
        record_provider_attempt(tmp.path(), "tavily", false, "missing api key", &policy);
        record_provider_attempt(tmp.path(), "tavily", false, "credential_unresolved", &policy);
        assert!(provider_circuit_open_until(tmp.path(), "tavily", &policy).is_none());
        let rows = provider_health_snapshot(tmp.path(), &[String::from("tavily")])
            .as_array()
            .cloned()
            .unwrap_or_default();
        let row = rows.first().cloned().unwrap_or_else(|| json!({}));
        assert_eq!(row.get("circuit_open").and_then(Value::as_bool), Some(false));
        assert_eq!(row.get("consecutive_failures").and_then(Value::as_u64), Some(0));
    }

    #[test]
    fn circuit_breaker_ignores_policy_denied_failures() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = json!({
            "web_conduit": {
                "provider_circuit_breaker": {
                    "enabled": true,
                    "failure_threshold": 2,
                    "open_for_secs": 120
                }
            }
        });
        record_provider_attempt(tmp.path(), "tavily", false, "web_conduit_policy_denied", &policy);
        record_provider_attempt(tmp.path(), "tavily", false, "policy_denied", &policy);
        record_provider_attempt(tmp.path(), "tavily", false, "provider_network_policy_blocked", &policy);
        assert!(provider_circuit_open_until(tmp.path(), "tavily", &policy).is_none());
        let rows = provider_health_snapshot(tmp.path(), &[String::from("tavily")])
            .as_array()
            .cloned()
            .unwrap_or_default();
        let row = rows.first().cloned().unwrap_or_else(|| json!({}));
        assert_eq!(row.get("circuit_open").and_then(Value::as_bool), Some(false));
        assert_eq!(row.get("consecutive_failures").and_then(Value::as_u64), Some(0));
        let state = load_provider_health(tmp.path());
        assert_eq!(
            state
                .pointer("/providers/tavily/last_failure_class")
                .and_then(Value::as_str),
            Some("configuration")
        );
    }

    #[test]
    fn circuit_breaker_clears_legacy_query_quality_opens() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = json!({
            "web_conduit": {
                "provider_circuit_breaker": {
                    "enabled": true,
                    "failure_threshold": 2,
                    "open_for_secs": 120
                }
            }
        });
        let path = provider_health_path(tmp.path());
        write_json_atomic(
            &path,
            &json!({
                "version": 1,
                "providers": {
                    "bing_rss": {
                        "consecutive_failures": 8,
                        "circuit_open_until": Utc::now().timestamp() + 120,
                        "last_error": "query_result_mismatch",
                        "last_failure_class": "provider_failure"
                    }
                }
            }),
        )
        .expect("provider health fixture");
        assert!(provider_circuit_open_until(tmp.path(), "bing_rss", &policy).is_none());
        let rows = provider_health_snapshot(tmp.path(), &[String::from("bing_rss")])
            .as_array()
            .cloned()
            .unwrap_or_default();
        let row = rows.first().cloned().unwrap_or_else(|| json!({}));
        assert_eq!(row.get("circuit_open").and_then(Value::as_bool), Some(false));
        assert_eq!(row.get("consecutive_failures").and_then(Value::as_u64), Some(0));
    }

    #[test]
    fn provider_health_snapshot_exposes_timestamps_and_circuit_state() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = json!({
            "web_conduit": {
                "provider_circuit_breaker": {
                    "enabled": true,
                    "failure_threshold": 2,
                    "open_for_secs": 120
                }
            }
        });
        record_provider_attempt(tmp.path(), "serperdev", false, "timeout", &policy);
        record_provider_attempt(tmp.path(), "serperdev", false, "timeout", &policy);
        let rows = provider_health_snapshot(tmp.path(), &[String::from("serperdev")])
            .as_array()
            .cloned()
            .unwrap_or_default();
        let row = rows.first().cloned().unwrap_or_else(|| json!({}));
        assert_eq!(
            row.get("provider").and_then(Value::as_str),
            Some("serperdev")
        );
        assert!(row
            .get("last_failure_at")
            .and_then(Value::as_str)
            .map(|value| !value.is_empty())
            .unwrap_or(false));
        assert_eq!(row.get("circuit_open").and_then(Value::as_bool), Some(true));
        record_provider_attempt(tmp.path(), "serperdev", true, "", &policy);
        let rows_after = provider_health_snapshot(tmp.path(), &[String::from("serperdev")])
            .as_array()
            .cloned()
            .unwrap_or_default();
        let row_after = rows_after.first().cloned().unwrap_or_else(|| json!({}));
        assert!(row_after
            .get("last_success_at")
            .and_then(Value::as_str)
            .map(|value| !value.is_empty())
            .unwrap_or(false));
        assert_eq!(
            row_after.get("circuit_open").and_then(Value::as_bool),
            Some(false)
        );
    }

    #[test]
    fn search_cache_roundtrip_returns_stored_payload() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let key = "cache-key";
        let response = json!({"ok": true, "summary": "cached"});
        store_search_cache(tmp.path(), key, &response, "ok", None);
        let loaded = load_search_cache(tmp.path(), key).expect("cache hit");
        assert_eq!(
            loaded.get("summary").and_then(Value::as_str),
            Some("cached")
        );
    }
}
