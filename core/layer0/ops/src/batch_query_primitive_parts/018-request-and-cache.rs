const CACHE_REL: &str = "client/runtime/local/state/batch_query/cache.json";
const CACHE_MAX_ENTRIES: usize = 240;
const CACHE_TTL_SUCCESS_SECS: i64 = 30 * 60;
const CACHE_TTL_NO_RESULTS_SECS: i64 = 2 * 60;
const CACHE_MODE_ENV: &str = "INFRING_BATCH_QUERY_CACHE_MODE";
const CACHE_TTL_SUCCESS_ENV: &str = "INFRING_BATCH_QUERY_CACHE_TTL_SUCCESS_SECONDS";
const CACHE_TTL_NO_RESULTS_ENV: &str = "INFRING_BATCH_QUERY_CACHE_TTL_NO_RESULTS_SECONDS";
const CACHE_MAX_ENTRIES_ENV: &str = "INFRING_BATCH_QUERY_CACHE_MAX_ENTRIES";

#[derive(Clone, Debug)]
struct QueryPlanSelection {
    queries: Vec<String>,
    rewrite_set: Vec<String>,
    rewrite_applied: bool,
    rerank_query: String,
    query_plan_source: &'static str,
    query_metadata: BatchQueryKeywordPack,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct BatchQueryKeywordPack {
    keywords: Vec<String>,
    entities: Vec<String>,
    facets: Vec<String>,
    aliases: Vec<String>,
    negative_terms: Vec<String>,
    metadata_authority: String,
}

impl BatchQueryKeywordPack {
    fn is_empty(&self) -> bool {
        self.keywords.is_empty()
            && self.entities.is_empty()
            && self.facets.is_empty()
            && self.aliases.is_empty()
            && self.negative_terms.is_empty()
    }

    fn has_positive_terms(&self) -> bool {
        !self.keywords.is_empty()
            || !self.entities.is_empty()
            || !self.facets.is_empty()
            || !self.aliases.is_empty()
    }

    fn to_value(&self) -> Value {
        let authority = if self.metadata_authority.is_empty() {
            "agent_submitted_request_metadata"
        } else {
            self.metadata_authority.as_str()
        };
        json!({
            "keywords": self.keywords.clone(),
            "required_coverage": {
                "entities": self.entities.clone(),
                "facets": self.facets.clone()
            },
            "aliases": self.aliases.clone(),
            "negative_terms": self.negative_terms.clone(),
            "compilation": {
                "authority": authority,
                "hidden_query_expansion": false,
                "quote_policy": "quote_exact_entity_or_alias_phrases_only",
                "negative_term_policy": "append_safe_negative_filters_to_compiled_lanes"
            }
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct BatchQuerySearchScope {
    allowed_domains: Vec<String>,
    exclude_subdomains: bool,
}

impl BatchQuerySearchScope {
    fn is_empty(&self) -> bool {
        self.allowed_domains.is_empty()
    }

    fn to_value(&self) -> Value {
        json!({
            "allowed_domains": self.allowed_domains.clone(),
            "exclude_subdomains": self.exclude_subdomains
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BatchQueryCacheControl {
    mode: String,
    ttl_success_secs: i64,
    ttl_no_results_secs: i64,
    max_entries: usize,
}

impl BatchQueryCacheControl {
    fn read_enabled(&self) -> bool {
        self.mode == "enabled"
    }

    fn write_enabled(&self) -> bool {
        self.mode == "enabled" || self.mode == "refresh"
    }

    fn fresh_status(&self) -> &'static str {
        match self.mode.as_str() {
            "enabled" => "miss",
            "refresh" => "refresh",
            _ => "disabled",
        }
    }
}

fn contains_antibot_marker(text: &str) -> bool {
    let lowered = clean_text(text, 4_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    [
        "unfortunately, bots use duckduckgo too",
        "please complete the following challenge",
        "select all squares containing",
        "error-lite@duckduckgo.com",
        "anomaly-modal",
        "captcha",
        "access to this page has been denied",
        "are you a robot",
        "confirm you are a human",
        "press & hold to confirm you are a human",
        "press and hold to confirm you are a human",
        "not a bot",
        "not a robot",
        "robot check",
        "detected unusual activity",
        "unusual activity from your computer network",
        "click the box below",
        "verify you are human",
        "checking your browser before accessing",
        "cf-challenge",
        "cloudflare ray id",
        "just a moment...",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn contains_web_junk_marker(text: &str) -> bool {
    let lowered = clean_text(text, 4_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    if contains_antibot_marker(&lowered) {
        return true;
    }
    [
        "please enable javascript",
        "enable javascript and cookies",
        "access denied",
        "403 forbidden",
        "login required",
        "subscribe to continue",
        "please log in to continue",
        "this content is not available in your region",
        "we use cookies to improve your experience",
        "manage your cookie preferences",
        "copyright zendesk, inc",
        "use of this source code is governed",
        ":root { --",
        "window.__",
        "document.addeventlistener",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn looks_like_internal_route_query(query: &str) -> bool {
    let lowered = clean_text(query, 600).to_ascii_lowercase();
    lowered.contains("tool::")
        || lowered.contains("map `tool::")
        || lowered.contains("supported route")
        || lowered.contains("command-to-route")
}

fn looks_like_domain_list_noise(text: &str) -> bool {
    let cleaned = clean_text(text, 1_600);
    if cleaned.is_empty() {
        return false;
    }
    let domains = extract_domains_from_text(&cleaned, 16);
    if domains.len() < 3 {
        return false;
    }
    let words = cleaned.split_whitespace().count();
    words <= (domains.len() * 3 + 10)
}

fn cache_path(root: &Path) -> PathBuf {
    root.join(CACHE_REL)
}

fn cache_identity_policy(policy: &Value) -> Value {
    let mut batch_query = policy
        .get("batch_query")
        .cloned()
        .unwrap_or(Value::Null);
    if let Some(obj) = batch_query.as_object_mut() {
        obj.remove("cache");
    }
    batch_query
}

fn cache_key(source: &str, query: &str, aperture: &str, policy: &Value) -> String {
    crate::deterministic_receipt_hash(&json!({
        "version": 1,
        "source": source,
        "query": query,
        "aperture": aperture,
        "policy": cache_identity_policy(policy),
    }))
}

fn cache_key_with_query_plan(
    source: &str,
    query: &str,
    aperture: &str,
    policy: &Value,
    query_plan: &[String],
) -> String {
    let normalized_plan = cache_identity_query_plan(query, query_plan);
    if normalized_plan.len() <= 1 {
        return cache_key(source, query, aperture, policy);
    }
    crate::deterministic_receipt_hash(&json!({
        "version": 4,
        "source": source,
        "query": query,
        "aperture": aperture,
        "query_plan": normalized_plan,
        "policy": cache_identity_policy(policy),
    }))
}

fn cache_key_with_query_plan_and_scope(
    source: &str,
    query: &str,
    aperture: &str,
    policy: &Value,
    query_plan: &[String],
    search_scope: &BatchQuerySearchScope,
) -> String {
    let normalized_plan = cache_identity_query_plan(query, query_plan);
    if search_scope.is_empty() && normalized_plan.len() <= 1 {
        return cache_key(source, query, aperture, policy);
    }
    if search_scope.is_empty() {
        return cache_key_with_query_plan(source, query, aperture, policy, query_plan);
    }
    crate::deterministic_receipt_hash(&json!({
        "version": 5,
        "source": source,
        "query": query,
        "aperture": aperture,
        "query_plan": normalized_plan,
        "search_scope": search_scope.to_value(),
        "policy": cache_identity_policy(policy),
    }))
}

fn cache_identity_query_plan(query: &str, query_plan: &[String]) -> Vec<String> {
    let mut dedup = HashSet::<String>::new();
    let mut normalized = Vec::<String>::new();
    for value in query_plan {
        let cleaned = clean_text(value, 600);
        if cleaned.is_empty() {
            continue;
        }
        let key = cleaned.to_ascii_lowercase();
        if dedup.insert(key) {
            normalized.push(cleaned);
        }
    }
    if normalized.is_empty() {
        let cleaned_query = clean_text(query, 600);
        if !cleaned_query.is_empty() {
            normalized.push(cleaned_query);
        }
    }
    normalized
}

fn strip_batch_query_terminal_response_suffix(raw: &str, max_chars: usize) -> String {
    let mut cleaned = clean_text(raw, max_chars);
    loop {
        let lowered = cleaned.to_ascii_lowercase();
        let mut stripped = false;
        for suffix in [
            " and return the results",
            " and return results",
            " and return the result",
            " and return the answer",
            " and return the findings",
            " and give me the results",
            " and give the results",
            " and show me the results",
            " and tell me the results",
            " and tell me what you find",
            " and tell me what you found",
            " and summarize the results",
            " and summarize the answer",
            " and summarize",
        ] {
            if lowered.ends_with(suffix) && cleaned.len() > suffix.len() {
                cleaned = clean_text(&cleaned[..cleaned.len() - suffix.len()], max_chars);
                stripped = true;
                break;
            }
        }
        if stripped {
            continue;
        }
        if matches!(cleaned.chars().last(), Some('.' | '!' | '?' | ';' | ':')) {
            cleaned.pop();
            cleaned = cleaned.trim_end().to_string();
            continue;
        }
        break;
    }
    clean_text(&cleaned, max_chars)
}

fn strip_batch_query_leading_search_control(raw: &str, max_chars: usize) -> String {
    let mut cleaned = clean_text(raw, max_chars);
    for _ in 0..4 {
        let lowered = cleaned.to_ascii_lowercase();
        let mut stripped = false;
        for prefix in [
            "please ",
            "can you ",
            "could you ",
            "would you ",
            "will you ",
            "just ",
            "try to ",
        ] {
            if lowered.starts_with(prefix) && cleaned.len() > prefix.len() {
                cleaned = clean_text(&cleaned[prefix.len()..], max_chars);
                stripped = true;
                break;
            }
        }
        if stripped {
            continue;
        }
        for prefix in [
            "give me ",
            "tell me about ",
            "tell me ",
            "show me ",
            "find me ",
            "find information about ",
            "find info about ",
            "find out about ",
            "find ",
            "look up ",
            "search online for ",
            "search the web for ",
            "search web for ",
            "search for ",
            "search ",
            "research ",
            "what are some ",
            "what are the ",
            "what are ",
            "what is the ",
            "what is ",
        ] {
            if lowered.starts_with(prefix) && cleaned.len() > prefix.len() {
                let candidate = clean_text(&cleaned[prefix.len()..], max_chars);
                if candidate.chars().any(|ch| ch.is_ascii_alphanumeric()) {
                    cleaned = candidate;
                    stripped = true;
                    break;
                }
            }
        }
        if !stripped {
            break;
        }
    }
    clean_text(&cleaned, max_chars)
}

fn canonical_batch_web_search_query(raw: &str) -> String {
    let cleaned = strip_batch_query_terminal_response_suffix(raw, 600);
    if cleaned.is_empty() {
        return cleaned;
    }
    if cleaned.to_ascii_lowercase().contains("site:") {
        return cleaned;
    }
    let stripped = strip_batch_query_leading_search_control(&cleaned, 600);
    let stripped = if stripped.is_empty() {
        cleaned
    } else {
        stripped
    };
    if let Some(focused) = instruction_focused_search_query(&stripped) {
        focused
    } else {
        stripped
    }
}

fn subject_preserving_batch_web_search_query(raw: &str) -> String {
    let cleaned = strip_batch_query_terminal_response_suffix(raw, 600);
    if cleaned.is_empty() {
        return cleaned;
    }
    let stripped = strip_batch_query_leading_search_control(&cleaned, 600);
    if stripped.is_empty() {
        cleaned
    } else {
        stripped
    }
}

fn provider_result_identity(value: &Value) -> String {
    let provider = clean_text(
        value.get("provider").and_then(Value::as_str).unwrap_or(""),
        120,
    )
    .to_ascii_lowercase();
    let status = clean_text(value.get("status").and_then(Value::as_str).unwrap_or(""), 80)
        .to_ascii_lowercase();
    let error = clean_text(value.get("error").and_then(Value::as_str).unwrap_or(""), 160)
        .to_ascii_lowercase();
    if !provider.is_empty() && !error.is_empty() {
        return crate::deterministic_receipt_hash(&json!({
            "kind": "provider_error",
            "provider": provider,
            "status": status,
            "error": error
        }));
    }

    let content = value
        .get("content_preview")
        .or_else(|| value.get("summary"))
        .and_then(Value::as_str)
        .map(|raw| clean_text(raw, 1_200).to_ascii_lowercase())
        .unwrap_or_default();
    if content.split_whitespace().count() >= 8 {
        return crate::deterministic_receipt_hash(&json!({
            "kind": "provider_content",
            "provider": provider,
            "status": status,
            "content": content
        }));
    }

    let stage = clean_text(value.get("stage").and_then(Value::as_str).unwrap_or(""), 120)
        .to_ascii_lowercase();
    let locator = clean_text(
        value.get("locator").and_then(Value::as_str).unwrap_or(""),
        600,
    )
    .to_ascii_lowercase();
    crate::deterministic_receipt_hash(&json!({
        "kind": "provider_locator",
        "provider": provider,
        "stage": stage,
        "status": status,
        "locator": locator
    }))
}

fn dedup_provider_results(provider_results: Vec<Value>) -> (Vec<Value>, usize) {
    let before = provider_results.len();
    let mut seen = HashSet::<String>::new();
    let mut deduped = Vec::<Value>::new();
    for value in provider_results {
        let key = provider_result_identity(&value);
        if seen.insert(key) {
            deduped.push(value);
        }
    }
    let removed = before.saturating_sub(deduped.len());
    (deduped, removed)
}

fn cache_policy_i64(policy: &Value, pointer: &str, default_value: i64, min: i64, max: i64) -> i64 {
    policy
        .pointer(pointer)
        .and_then(Value::as_i64)
        .unwrap_or(default_value)
        .clamp(min, max)
}

fn cache_policy_usize(
    policy: &Value,
    pointer: &str,
    default_value: usize,
    min: usize,
    max: usize,
) -> usize {
    policy
        .pointer(pointer)
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or(default_value)
        .clamp(min, max)
}

fn env_i64(key: &str, fallback: i64, min: i64, max: i64) -> i64 {
    std::env::var(key)
        .ok()
        .and_then(|raw| raw.parse::<i64>().ok())
        .unwrap_or(fallback)
        .clamp(min, max)
}

fn env_usize(key: &str, fallback: usize, min: usize, max: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .unwrap_or(fallback)
        .clamp(min, max)
}

fn normalize_cache_mode(raw: &str) -> String {
    match clean_text(raw, 40).to_ascii_lowercase().as_str() {
        "refresh" | "fresh" | "write_only" | "write-only" => "refresh".to_string(),
        "disabled" | "disable" | "off" | "none" | "bypass" | "no_cache" | "no-cache" => {
            "disabled".to_string()
        }
        _ => "enabled".to_string(),
    }
}

fn requested_cache_mode(request: &Value) -> Option<String> {
    request
        .get("cache_mode")
        .or_else(|| request.pointer("/cache/mode"))
        .or_else(|| request.pointer("/cache_policy/mode"))
        .and_then(Value::as_str)
        .map(normalize_cache_mode)
}

fn batch_query_cache_control(policy: &Value, request: &Value) -> BatchQueryCacheControl {
    let ttl_success = cache_policy_i64(
        policy,
        "/batch_query/cache/ttl_success_seconds",
        CACHE_TTL_SUCCESS_SECS,
        30,
        7 * 24 * 60 * 60,
    );
    let ttl_no_results = cache_policy_i64(
        policy,
        "/batch_query/cache/ttl_no_results_seconds",
        CACHE_TTL_NO_RESULTS_SECS,
        30,
        24 * 60 * 60,
    );
    let max_entries = cache_policy_usize(
        policy,
        "/batch_query/cache/max_entries",
        CACHE_MAX_ENTRIES,
        1,
        10_000,
    );
    let mode = std::env::var(CACHE_MODE_ENV)
        .ok()
        .map(|raw| normalize_cache_mode(&raw))
        .or_else(|| requested_cache_mode(request))
        .or_else(|| {
            policy
                .pointer("/batch_query/cache/mode")
                .and_then(Value::as_str)
                .map(normalize_cache_mode)
        })
        .unwrap_or_else(|| "enabled".to_string());
    BatchQueryCacheControl {
        mode,
        ttl_success_secs: env_i64(CACHE_TTL_SUCCESS_ENV, ttl_success, 30, 7 * 24 * 60 * 60),
        ttl_no_results_secs: env_i64(CACHE_TTL_NO_RESULTS_ENV, ttl_no_results, 30, 24 * 60 * 60),
        max_entries: env_usize(CACHE_MAX_ENTRIES_ENV, max_entries, 1, 10_000),
    }
}

fn cache_ttl_for_status(status: &str, control: &BatchQueryCacheControl) -> i64 {
    if status == "ok" || status == "partial" {
        control.ttl_success_secs
    } else {
        control.ttl_no_results_secs
    }
}

fn prune_cache_entries(
    entries: &mut Map<String, Value>,
    now_ts: i64,
    max_entries: usize,
) -> usize {
    let before = entries.len();
    entries.retain(|_, entry| entry.get("expires_at").and_then(Value::as_i64).unwrap_or(0) > now_ts);
    if entries.len() > max_entries {
        let mut order = entries
            .iter()
            .map(|(entry_key, entry)| {
                (
                    entry_key.clone(),
                    entry.get("stored_at").and_then(Value::as_i64).unwrap_or(0),
                )
            })
            .collect::<Vec<_>>();
        order.sort_by_key(|(_, stored_at)| *stored_at);
        let drop_count = entries.len().saturating_sub(max_entries);
        for (entry_key, _) in order.into_iter().take(drop_count) {
            entries.remove(&entry_key);
        }
    }
    before.saturating_sub(entries.len())
}

fn load_cached_response(root: &Path, key: &str, control: &BatchQueryCacheControl) -> Option<Value> {
    if !control.read_enabled() {
        return None;
    }
    let path = cache_path(root);
    let mut cache = read_json_or(&path, json!({"version": 1, "entries": {}}));
    let now_ts = chrono::Utc::now().timestamp();
    let mut mutated = false;
    let mut hit = None::<Value>;
    if let Some(entries) = cache.get_mut("entries").and_then(Value::as_object_mut) {
        mutated = prune_cache_entries(entries, now_ts, control.max_entries) > 0;
        if let Some(entry) = entries.get(key) {
            if let Some(response) = entry.get("response") {
                hit = Some(response.clone());
            }
        }
    }
    if mutated {
        let _ = write_json_atomic(&path, &cache);
    }
    hit
}

fn store_cached_response(
    root: &Path,
    key: &str,
    response: &Value,
    status: &str,
    control: &BatchQueryCacheControl,
) {
    if !control.write_enabled() {
        return;
    }
    let path = cache_path(root);
    let mut cache = read_json_or(&path, json!({"version": 1, "entries": {}}));
    let now_ts = chrono::Utc::now().timestamp();
    let mut entries = cache
        .get("entries")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    prune_cache_entries(&mut entries, now_ts, control.max_entries);
    let ttl = cache_ttl_for_status(status, control).max(30);
    entries.insert(
        key.to_string(),
        json!({
            "stored_at": now_ts,
            "expires_at": now_ts + ttl,
            "status": status,
            "response": response
        }),
    );
    prune_cache_entries(&mut entries, now_ts, control.max_entries);
    cache["version"] = json!(1);
    cache["entries"] = Value::Object(entries);
    let _ = write_json_atomic(&path, &cache);
}

fn prune_batch_query_cache(root: &Path, control: &BatchQueryCacheControl) -> Value {
    let path = cache_path(root);
    if !path.exists() {
        return json!({
            "ok": true,
            "type": "batch_query_cache_cleanup",
            "cache_path": path.to_string_lossy().to_string(),
            "cache_written": false,
            "before_entries": 0,
            "after_entries": 0,
            "removed_entries": 0,
            "max_entries": control.max_entries,
            "ttl_success_seconds": control.ttl_success_secs,
            "ttl_no_results_seconds": control.ttl_no_results_secs
        });
    }
    let mut cache = read_json_or(&path, json!({"version": 1, "entries": {}}));
    let now_ts = chrono::Utc::now().timestamp();
    let before = cache
        .get("entries")
        .and_then(Value::as_object)
        .map(|entries| entries.len())
        .unwrap_or(0);
    let mut entries = cache
        .get("entries")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let removed = prune_cache_entries(&mut entries, now_ts, control.max_entries);
    let after = entries.len();
    cache["version"] = json!(1);
    cache["entries"] = Value::Object(entries);
    let cache_written = removed > 0;
    let write_ok = if cache_written {
        write_json_atomic(&path, &cache).is_ok()
    } else {
        true
    };
    json!({
        "ok": write_ok,
        "type": "batch_query_cache_cleanup",
        "cache_path": path.to_string_lossy().to_string(),
        "cache_written": cache_written,
        "before_entries": before,
        "after_entries": after,
        "removed_entries": removed,
        "max_entries": control.max_entries,
        "ttl_success_seconds": control.ttl_success_secs,
        "ttl_no_results_seconds": control.ttl_no_results_secs
    })
}

pub(crate) fn cleanup_cache(root: &Path) -> Value {
    let policy = load_policy(root);
    let control = batch_query_cache_control(&policy, &json!({}));
    prune_batch_query_cache(root, &control)
}

fn request_query_text(request: &Value, max_len: usize) -> String {
    let direct = clean_text(
        request
            .get("query")
            .or_else(|| request.get("q"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        max_len,
    );
    if !direct.is_empty() {
        return direct;
    }
    request
        .get("queries")
        .and_then(Value::as_array)
        .and_then(|rows| rows.iter().find_map(|row| extract_request_query_row(row, max_len)))
        .unwrap_or_default()
}

fn request_bool_alias(request: &Value, names: &[&str]) -> bool {
    for name in names {
        if let Some(value) = request.get(*name) {
            if let Some(flag) = value.as_bool() {
                return flag;
            }
            if let Some(raw) = value.as_str() {
                let lowered = clean_text(raw, 20).to_ascii_lowercase();
                if matches!(lowered.as_str(), "1" | "true" | "yes" | "on") {
                    return true;
                }
                if matches!(lowered.as_str(), "0" | "false" | "no" | "off") {
                    return false;
                }
            }
        }
    }
    false
}

fn normalize_batch_query_scope_domain(raw: &str) -> Option<String> {
    let mut value = clean_text(raw, 260).to_ascii_lowercase();
    if value.is_empty() {
        return None;
    }
    for prefix in ["https://", "http://"] {
        if let Some(stripped) = value.strip_prefix(prefix) {
            value = stripped.to_string();
            break;
        }
    }
    if let Some(stripped) = value.strip_prefix("*.") {
        value = stripped.to_string();
    }
    if let Some(stripped) = value.strip_prefix("www.") {
        value = stripped.to_string();
    }
    value = value
        .split(['/', '?', '#'])
        .next()
        .unwrap_or("")
        .trim_matches('.')
        .to_string();
    if value.is_empty() || !value.contains('.') {
        return None;
    }
    if !value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '.' || ch == '-')
    {
        return None;
    }
    Some(value)
}

fn push_normalized_scope_domain(raw: &str, seen: &mut HashSet<String>, out: &mut Vec<String>) {
    if let Some(domain) = normalize_batch_query_scope_domain(raw) {
        if seen.insert(domain.clone()) {
            out.push(domain);
        }
    }
}

fn collect_scope_domains_from_value(
    value: &Value,
    seen: &mut HashSet<String>,
    out: &mut Vec<String>,
) {
    match value {
        Value::Array(rows) => {
            for row in rows {
                collect_scope_domains_from_value(row, seen, out);
            }
        }
        Value::String(raw) => {
            for part in raw.split([',', ';', '\n', '\t', ' ']) {
                push_normalized_scope_domain(part, seen, out);
            }
        }
        Value::Object(map) => {
            for key in ["domain", "host", "url", "origin"] {
                if let Some(raw) = map.get(key).and_then(Value::as_str) {
                    push_normalized_scope_domain(raw, seen, out);
                }
            }
        }
        _ => {}
    }
}

fn batch_query_search_scope(request: &Value) -> BatchQuerySearchScope {
    let mut seen = HashSet::<String>::new();
    let mut allowed_domains = Vec::<String>::new();
    for key in [
        "allowed_domains",
        "include_domains",
        "allowedDomains",
        "includeDomains",
    ] {
        if let Some(value) = request.get(key) {
            collect_scope_domains_from_value(value, &mut seen, &mut allowed_domains);
            if !allowed_domains.is_empty() {
                break;
            }
        }
    }
    let exclude_subdomains = !allowed_domains.is_empty()
        && request_bool_alias(
            request,
            &[
                "exclude_subdomains",
                "exact_domain_only",
                "excludeSubdomains",
                "exactDomainOnly",
            ],
        );
    BatchQuerySearchScope {
        allowed_domains,
        exclude_subdomains,
    }
}

fn extract_request_query_row(row: &Value, max_len: usize) -> Option<String> {
    let raw = if let Some(value) = row.as_str() {
        Some(value)
    } else {
        row.get("query")
            .or_else(|| row.get("q"))
            .and_then(Value::as_str)
    }?;
    let cleaned = clean_text(raw, max_len);
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

fn push_query_dedup(value: String, dedup: &mut HashSet<String>, queries: &mut Vec<String>) {
    if value.is_empty() {
        return;
    }
    let key = query_plan_dedup_key(&value);
    if dedup.insert(key) {
        queries.push(value);
    }
}

fn query_plan_dedup_key(value: &str) -> String {
    let canonical = canonical_batch_web_search_query(value);
    let canonical = if canonical.is_empty() {
        clean_text(value, 600)
    } else {
        canonical
    };
    let mut key = clean_text(&canonical, 600).to_ascii_lowercase();
    while matches!(key.chars().last(), Some('.' | '!' | '?' | ';' | ':')) {
        key.pop();
        key = key.trim_end().to_string();
    }
    key
}

fn current_month_number() -> String {
    chrono::Local::now().format("%m").to_string()
}

fn current_month_name() -> &'static str {
    match current_month_number().as_str() {
        "01" => "january",
        "02" => "february",
        "03" => "march",
        "04" => "april",
        "05" => "may",
        "06" => "june",
        "07" => "july",
        "08" => "august",
        "09" => "september",
        "10" => "october",
        "11" => "november",
        "12" => "december",
        _ => "",
    }
}

fn current_day_number() -> String {
    chrono::Local::now().format("%-d").to_string()
}

fn current_date_iso() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

fn query_uses_relative_current_window(query: &str) -> bool {
    let lowered = clean_text(query, 600).to_ascii_lowercase();
    [
        "today",
        "this week",
        "this month",
        "yesterday",
        "hours ago",
        "hour ago",
        "minutes ago",
        "minute ago",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn query_uses_week_or_day_current_window(query: &str) -> bool {
    let lowered = clean_text(query, 600).to_ascii_lowercase();
    [
        "today",
        "this week",
        "yesterday",
        "hours ago",
        "hour ago",
        "minutes ago",
        "minute ago",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn query_already_has_current_month_scope(query: &str) -> bool {
    let lowered = clean_text(query, 600).to_ascii_lowercase();
    let year = current_year();
    let month_number = current_month_number();
    let month_name = current_month_name();
    (!month_name.is_empty() && lowered.contains(month_name) && lowered.contains(&year))
        || (!month_number.is_empty() && lowered.contains(&format!("{year}-{month_number}")))
        || (!month_number.is_empty() && lowered.contains(&format!("{year}/{month_number}")))
        || (!month_number.is_empty() && lowered.contains(&format!("{month_number}/{year}")))
}

fn query_already_has_current_date_scope(query: &str) -> bool {
    let lowered = clean_text(query, 600).to_ascii_lowercase();
    let year = current_year();
    let month_number = current_month_number();
    let month_name = current_month_name();
    let day = current_day_number();
    let day_padded = chrono::Local::now().format("%d").to_string();
    lowered.contains(&current_date_iso())
        || (!month_name.is_empty()
            && lowered.contains(month_name)
            && lowered.contains(&day)
            && lowered.contains(&year))
        || (!month_number.is_empty()
            && lowered.contains(&format!("{year}-{month_number}-{day_padded}")))
        || (!month_number.is_empty()
            && lowered.contains(&format!("{month_number}/{day_padded}/{year}")))
        || (!month_number.is_empty()
            && lowered.contains(&format!("{month_number}/{day}/{year}")))
}

fn temporal_scope_query_lanes(query: &str) -> Vec<String> {
    let cleaned = clean_text(query, 520);
    if cleaned.is_empty() || !query_uses_relative_current_window(&cleaned) {
        return Vec::new();
    }
    let year = current_year();
    let month_number = current_month_number();
    let month_name = current_month_name();
    let day = current_day_number();
    let mut lanes = Vec::<String>::new();

    if query_uses_week_or_day_current_window(&cleaned)
        && !query_already_has_current_date_scope(&cleaned)
    {
        lanes.push(clean_text(&format!("{cleaned} {}", current_date_iso()), 600));
        if !month_name.is_empty() && !day.is_empty() {
            lanes.push(clean_text(
                &format!("{cleaned} week of {month_name} {day} {year}"),
                600,
            ));
        }
    }

    if !query_already_has_current_month_scope(&cleaned) && !month_name.is_empty() {
        lanes.push(clean_text(&format!("{cleaned} {month_name} {year}"), 600));
    }
    if !query_already_has_current_month_scope(&cleaned) && !month_number.is_empty() {
        lanes.push(clean_text(&format!("{cleaned} {year}-{month_number}"), 600));
    }
    lanes
        .into_iter()
        .filter(|lane| !lane.is_empty())
        .collect::<Vec<_>>()
}

fn push_temporal_scope_query_lanes(
    primary_query: &str,
    dedup: &mut HashSet<String>,
    queries: &mut Vec<String>,
    max_queries: usize,
) {
    for lane in temporal_scope_query_lanes(primary_query) {
        if queries.len() >= max_queries {
            break;
        }
        push_query_dedup(lane, dedup, queries);
    }
}

fn push_recovery_temporal_scope_query_lanes(
    query: &str,
    dedup: &mut HashSet<String>,
    queries: &mut Vec<String>,
    max_queries: usize,
) {
    if !query_uses_week_or_day_current_window(query) {
        return;
    }
    push_temporal_scope_query_lanes(query, dedup, queries, max_queries);
}

fn push_metadata_term(raw: &str, seen: &mut HashSet<String>, out: &mut Vec<String>, max: usize) {
    if out.len() >= max {
        return;
    }
    let mut value = clean_text(raw, 160);
    value = value.trim_matches(|ch| matches!(ch, '"' | '\'' | '`')).to_string();
    if value.is_empty() {
        return;
    }
    let key = value.to_ascii_lowercase();
    if seen.insert(key) {
        out.push(value);
    }
}

fn collect_metadata_terms_from_value(
    value: &Value,
    seen: &mut HashSet<String>,
    out: &mut Vec<String>,
    max: usize,
) {
    if out.len() >= max {
        return;
    }
    match value {
        Value::Array(rows) => {
            for row in rows {
                collect_metadata_terms_from_value(row, seen, out, max);
                if out.len() >= max {
                    break;
                }
            }
        }
        Value::String(raw) => {
            for part in raw.split([',', ';', '\n', '\t']) {
                push_metadata_term(part, seen, out, max);
                if out.len() >= max {
                    break;
                }
            }
        }
        Value::Object(map) => {
            for key in ["term", "keyword", "query", "text", "name", "label", "value"] {
                if let Some(raw) = map.get(key).and_then(Value::as_str) {
                    push_metadata_term(raw, seen, out, max);
                }
                if out.len() >= max {
                    break;
                }
            }
        }
        _ => {}
    }
}

fn collect_request_terms(
    request: &Value,
    keys: &[&str],
    seen: &mut HashSet<String>,
    out: &mut Vec<String>,
    max: usize,
) {
    for key in keys {
        if let Some(value) = request.get(*key) {
            collect_metadata_terms_from_value(value, seen, out, max);
        }
        if out.len() >= max {
            break;
        }
    }
}

fn collect_coverage_terms(
    coverage: Option<&Value>,
    keys: &[&str],
    seen: &mut HashSet<String>,
    out: &mut Vec<String>,
    max: usize,
) {
    let Some(Value::Object(map)) = coverage else {
        return;
    };
    for key in keys {
        if let Some(value) = map.get(*key) {
            collect_metadata_terms_from_value(value, seen, out, max);
        }
        if out.len() >= max {
            break;
        }
    }
}

fn batch_query_keyword_pack(request: &Value, budget: ApertureBudget) -> BatchQueryKeywordPack {
    let max_terms = budget.max_candidates.clamp(4, 12);
    let coverage = request
        .get("required_coverage")
        .or_else(|| request.get("coverage_targets"))
        .or_else(|| request.get("coverage"));

    let mut pack = BatchQueryKeywordPack::default();
    let mut seen = HashSet::<String>::new();
    collect_request_terms(
        request,
        &["keywords", "keyword_pack", "key_terms", "terms"],
        &mut seen,
        &mut pack.keywords,
        max_terms,
    );

    seen.clear();
    collect_coverage_terms(
        coverage,
        &["entities", "entity", "candidates", "named_entities", "subjects"],
        &mut seen,
        &mut pack.entities,
        max_terms,
    );
    collect_request_terms(
        request,
        &["entities", "candidates", "named_entities", "subjects"],
        &mut seen,
        &mut pack.entities,
        max_terms,
    );

    seen.clear();
    collect_coverage_terms(
        coverage,
        &[
            "facets",
            "facet",
            "aspects",
            "criteria",
            "source_classes",
            "time_windows",
            "coverage_lanes",
        ],
        &mut seen,
        &mut pack.facets,
        max_terms,
    );
    collect_request_terms(
        request,
        &["facets", "aspects", "criteria", "coverage_lanes"],
        &mut seen,
        &mut pack.facets,
        max_terms,
    );

    seen.clear();
    collect_request_terms(
        request,
        &["aliases", "alias_terms", "alternate_names"],
        &mut seen,
        &mut pack.aliases,
        max_terms,
    );

    seen.clear();
    collect_request_terms(
        request,
        &["negative_terms", "exclude_terms", "ambiguity_filters"],
        &mut seen,
        &mut pack.negative_terms,
        max_terms.min(6),
    );
    pack
}

fn comparison_marker_token(raw: &str) -> bool {
    matches!(
        raw.to_ascii_lowercase().as_str(),
        "vs" | "v" | "versus" | "against"
    )
}

fn leading_comparison_separator_token(raw: &str) -> bool {
    matches!(
        raw.to_ascii_lowercase().as_str(),
        "vs" | "v" | "versus" | "against" | "with" | "to"
    )
}

fn comparison_lead_token(raw: &str) -> bool {
    matches!(
        raw.to_ascii_lowercase().as_str(),
        "compare" | "compared" | "comparing" | "research" | "evaluate" | "assess"
    )
}

fn comparison_tail_boundary_token(raw: &str) -> bool {
    matches!(
        raw.to_ascii_lowercase().as_str(),
        "across"
            | "as"
            | "by"
            | "for"
            | "focus"
            | "focused"
            | "focusing"
            | "give"
            | "include"
            | "including"
            | "against"
            | "about"
            | "on"
            | "regarding"
            | "when"
            | "where"
            | "while"
            | "because"
    )
}

fn comparison_entity_stop_token(raw: &str) -> bool {
    matches!(
        raw.to_ascii_lowercase().as_str(),
        "a" | "an"
            | "and"
            | "agent"
            | "agents"
            | "are"
            | "as"
            | "best"
            | "between"
            | "compare"
            | "compared"
            | "comparison"
            | "current"
            | "docs"
            | "documentation"
            | "evidence"
            | "for"
            | "framework"
            | "frameworks"
            | "guide"
            | "in"
            | "is"
            | "library"
            | "libraries"
            | "latest"
            | "of"
            | "official"
            | "on"
            | "or"
            | "pricing"
            | "research"
            | "review"
            | "reviews"
            | "security"
            | "software"
            | "the"
            | "to"
            | "tool"
            | "tools"
            | "workflow"
            | "workflows"
            | "with"
    )
}

fn query_metadata_token(raw: &str) -> String {
    clean_text(
        raw.trim_matches(|ch: char| {
            ch.is_ascii_punctuation()
                && ch != '-'
                && ch != '_'
                && ch != '+'
                && ch != '#'
        }),
        80,
    )
}

fn query_metadata_tokens(raw: &str) -> Vec<String> {
    raw.split_whitespace()
        .map(query_metadata_token)
        .filter(|token| !token.is_empty())
        .collect()
}

fn query_metadata_token_has_entity_boundary(raw: &str) -> bool {
    raw.chars()
        .any(|ch| matches!(ch, ',' | ';' | ':' | '|' | '(' | ')' | '[' | ']' | '.' | '?' | '!'))
}

fn query_metadata_keyword_tokens(raw: &str) -> Vec<String> {
    raw.split_whitespace()
        .flat_map(|token| token.split('/'))
        .map(query_metadata_token)
        .filter(|token| !token.is_empty())
        .collect()
}

fn token_has_letter_or_number(raw: &str) -> bool {
    raw.chars().any(|ch| ch.is_ascii_alphanumeric())
}

fn collect_entity_tokens_backward(tokens: &[String], marker_index: usize) -> Vec<String> {
    let mut out = Vec::<String>::new();
    for token in tokens[..marker_index].iter().rev() {
        if token.is_empty()
            || comparison_marker_token(token)
            || comparison_entity_stop_token(token)
            || !token_has_letter_or_number(token)
        {
            if !out.is_empty() {
                break;
            }
            continue;
        }
        out.push(token.clone());
        if out.len() >= 4 {
            break;
        }
    }
    out.reverse();
    out
}

fn collect_entity_tokens_forward(tokens: &[String], marker_index: usize) -> Vec<String> {
    let mut out = Vec::<String>::new();
    for token in tokens.iter().skip(marker_index + 1) {
        if token.is_empty()
            || comparison_marker_token(token)
            || comparison_entity_stop_token(token)
            || !token_has_letter_or_number(token)
        {
            if !out.is_empty() {
                break;
            }
            continue;
        }
        out.push(token.clone());
        if out.len() >= 4 {
            break;
        }
    }
    out
}

fn push_unique_clean(out: &mut Vec<String>, seen: &mut HashSet<String>, value: String, max: usize) {
    if out.len() >= max {
        return;
    }
    let cleaned = clean_text(&value, 120);
    let key = cleaned.to_ascii_lowercase();
    if !cleaned.is_empty() && seen.insert(key) {
        out.push(cleaned);
    }
}

fn entity_phrase_stop_edge(raw: &str) -> bool {
    matches!(
        raw.to_ascii_lowercase().as_str(),
        "a" | "an"
            | "and"
            | "are"
            | "as"
            | "assistant"
            | "assistants"
            | "automation"
            | "current"
            | "docs"
            | "documentation"
            | "latest"
            | "new"
            | "option"
            | "options"
            | "recent"
            | "source"
            | "sources"
            | "task"
            | "tasks"
            | "the"
            | "this"
            | "that"
            | "these"
            | "those"
            | "for"
            | "of"
            | "on"
            | "in"
            | "with"
            | "to"
            | "from"
            | "versus"
            | "vs"
            | "compare"
            | "find"
            | "focus"
            | "look"
            | "research"
            | "search"
            | "summarize"
            | "use"
            | "evaluate"
            | "assess"
            | "workflow"
            | "workflows"
    )
}

fn query_entity_noise_token(raw: &str) -> bool {
    matches!(
        raw.to_ascii_lowercase().as_str(),
        "after" | "april"
            | "august"
            | "before"
            | "clearly"
            | "december"
            | "explain"
            | "february"
            | "find"
            | "first"
            | "focus"
            | "focused"
            | "give"
            | "if"
            | "january"
            | "july"
            | "june"
            | "look"
            | "looking"
            | "march"
            | "may"
            | "need"
            | "november"
            | "october"
            | "research"
            | "right"
            | "say"
            | "search"
            | "september"
            | "summarize"
            | "tell"
            | "use"
            | "using"
            | "where"
            | "whether"
            | "which"
    )
}

fn normalized_named_entity_token_piece(raw: &str) -> Option<String> {
    let mut cleaned = query_metadata_token(raw);
    if cleaned.is_empty() {
        return None;
    }
    let lowered = cleaned.to_ascii_lowercase();
    if lowered.ends_with("-style") {
        return None;
    }
    if lowered.ends_with("-based") && cleaned.len() > "-based".len() + 1 {
        cleaned.truncate(cleaned.len() - "-based".len());
    }
    let cleaned = clean_text(&cleaned, 80);
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

fn normalized_entity_phrase_from_tokens(tokens: &[String]) -> Option<String> {
    let mut start = 0usize;
    let mut end = tokens.len();
    while start < end && entity_phrase_stop_edge(&tokens[start]) {
        start += 1;
    }
    while end > start && entity_phrase_stop_edge(&tokens[end - 1]) {
        end -= 1;
    }
    if start >= end {
        return None;
    }
    let phrase = tokens[start..end]
        .iter()
        .filter_map(|token| normalized_named_entity_token_piece(token))
        .filter(|token| token_has_letter_or_number(token))
        .take(5)
        .collect::<Vec<_>>()
        .join(" ");
    let phrase = clean_text(&phrase, 160);
    if phrase.is_empty() {
        None
    } else {
        Some(phrase)
    }
}

fn token_looks_like_named_entity_piece(raw: &str) -> bool {
    let Some(cleaned) = normalized_named_entity_token_piece(raw) else {
        return false;
    };
    if cleaned.is_empty() || entity_phrase_stop_edge(&cleaned) || query_entity_noise_token(&cleaned) {
        return false;
    }
    let letters = cleaned
        .chars()
        .filter(|ch| ch.is_ascii_alphabetic())
        .collect::<Vec<_>>();
    if letters.is_empty() {
        return false;
    }
    let uppercase_count = letters.iter().filter(|ch| ch.is_ascii_uppercase()).count();
    let has_project_punct = cleaned.contains('-')
        || cleaned.contains('_')
        || cleaned.contains('+')
        || cleaned.contains('#');
    uppercase_count >= 2
        || (has_project_punct && letters.len() >= 3)
        || cleaned.chars().next().map(|ch| ch.is_ascii_uppercase()).unwrap_or(false)
            && !raw_query_metadata_stop_token(&cleaned.to_ascii_lowercase())
}

fn split_entity_phrase_variants(raw: &str) -> Vec<String> {
    let cleaned = clean_text(raw, 240);
    if cleaned.is_empty() {
        return Vec::new();
    }
    let pieces = cleaned
        .split([',', ';', '|'])
        .flat_map(|piece| piece.split(" and "))
        .flat_map(|piece| piece.split('/'))
        .map(|piece| {
            piece
                .split_whitespace()
                .map(query_metadata_token)
                .filter(|token| !token.is_empty())
                .collect::<Vec<_>>()
        })
        .filter_map(|tokens| normalized_entity_phrase_from_tokens(&tokens))
        .collect::<Vec<_>>();
    if pieces.is_empty() {
        normalized_entity_phrase_from_tokens(&query_metadata_tokens(&cleaned))
            .into_iter()
            .collect()
    } else {
        pieces
    }
}

fn token_looks_like_unseparated_entity_item(raw: &str) -> bool {
    let token = query_metadata_token(raw);
    let lowered = token.to_ascii_lowercase();
    !token.is_empty()
        && token_has_letter_or_number(&token)
        && !entity_phrase_stop_edge(&token)
        && !query_entity_noise_token(&token)
        && !raw_query_metadata_stop_token(&lowered)
        && (token
            .chars()
            .next()
            .map(|ch| ch.is_ascii_uppercase())
            .unwrap_or(false)
            || token
                .chars()
                .skip(1)
                .any(|ch| ch.is_ascii_uppercase())
            || token.contains('-')
            || token.contains('_')
            || token.contains('+')
            || token.contains('#'))
}

fn split_unseparated_comparison_entity_variants(raw: &str) -> Vec<String> {
    let regular = split_entity_phrase_variants(raw);
    if regular.len() != 1 {
        return regular;
    }
    let tokens = query_metadata_tokens(&regular[0]);
    if tokens.len() == 2
        && tokens
            .iter()
            .all(|token| token_looks_like_unseparated_entity_item(token))
    {
        return tokens;
    }
    regular
}

fn push_entity_phrase_variants(
    entities: &mut Vec<String>,
    seen: &mut HashSet<String>,
    tokens: &[String],
    max_terms: usize,
) {
    let Some(entity) = normalized_entity_phrase_from_tokens(tokens) else {
        return;
    };
    for piece in split_entity_phrase_variants(&entity) {
        if entities.len() >= max_terms {
            break;
        }
        push_unique_clean(entities, seen, piece, max_terms);
    }
}

fn infer_leading_comparison_entities(tokens: &[String]) -> Vec<String> {
    if tokens.len() < 4 || !comparison_lead_token(&tokens[0]) {
        return Vec::new();
    }
    let Some(separator_index) = tokens
        .iter()
        .enumerate()
        .skip(1)
        .take(10)
        .find_map(|(index, token)| leading_comparison_separator_token(token).then_some(index))
    else {
        return Vec::new();
    };
    if separator_index <= 1 {
        return Vec::new();
    }
    let left_tokens = tokens[1..separator_index].to_vec();
    let mut right_tokens = Vec::<String>::new();
    for token in tokens.iter().skip(separator_index + 1) {
        if comparison_tail_boundary_token(token) {
            break;
        }
        right_tokens.push(token.clone());
        if right_tokens.len() >= 6 {
            break;
        }
    }
    let mut out = Vec::<String>::new();
    if let Some(left) = normalized_entity_phrase_from_tokens(&left_tokens) {
        out.push(left);
    }
    if let Some(right) = normalized_entity_phrase_from_tokens(&right_tokens) {
        for piece in split_entity_phrase_variants(&right) {
            out.push(piece);
        }
    }
    out
}

fn leading_comparison_query_tail(query: &str) -> Option<String> {
    let cleaned = clean_text(query, 600);
    let mut pieces = cleaned.splitn(2, char::is_whitespace);
    let lead = pieces.next().unwrap_or("");
    if !comparison_lead_token(lead) {
        return None;
    }
    let tail = pieces.next().unwrap_or("").trim();
    if tail.is_empty() {
        None
    } else {
        Some(clean_text(tail, 420))
    }
}

fn leading_comparison_entity_segment(query: &str) -> Option<String> {
    let tail = leading_comparison_query_tail(query)?;
    let tokens = tail.split_whitespace().collect::<Vec<_>>();
    let end = tokens
        .iter()
        .position(|token| comparison_tail_boundary_token(&query_metadata_token(token)))
        .unwrap_or(tokens.len());
    if end == 0 {
        return None;
    }
    let segment = clean_text(&tokens[..end].join(" "), 320);
    if segment.is_empty() {
        None
    } else {
        Some(segment)
    }
}

fn infer_leading_comparison_entities_from_query(query: &str, max_terms: usize) -> Vec<String> {
    let Some(segment) = leading_comparison_entity_segment(query) else {
        return Vec::new();
    };
    let separator_re = Regex::new(r"(?i)\s+(?:vs\.?|v\.?|versus|against|with|to|and)\s+")
        .expect("leading comparison separator regex");
    let normalized = separator_re.replace_all(&segment, "|");
    let pieces = normalized
        .split('|')
        .flat_map(|piece| piece.split(','))
        .flat_map(|piece| piece.split(';'))
        .flat_map(|piece| piece.split('/'))
        .map(|piece| clean_text(piece, 160))
        .filter(|piece| !piece.is_empty())
        .collect::<Vec<_>>();
    let split_unseparated_pairs = pieces.len() >= 2;
    let mut out = Vec::<String>::new();
    let mut seen = HashSet::<String>::new();
    for piece in pieces {
        if out.len() >= max_terms {
            break;
        }
        let tokens = query_metadata_tokens(&piece);
        let Some(entity) = normalized_entity_phrase_from_tokens(&tokens) else {
            continue;
        };
        let variants = if split_unseparated_pairs {
            split_unseparated_comparison_entity_variants(&entity)
        } else {
            split_entity_phrase_variants(&entity)
        };
        for variant in variants {
            if out.len() >= max_terms {
                break;
            }
            if entity_phrase_is_too_generic(&variant) {
                continue;
            }
            push_unique_clean(&mut out, &mut seen, variant, max_terms);
        }
    }
    out
}

fn infer_named_entity_terms_from_query(query: &str, max_terms: usize) -> Vec<String> {
    let mut entities = Vec::<String>::new();
    let mut seen = HashSet::<String>::new();
    let mut current = Vec::<String>::new();
    for raw in query.split_whitespace() {
        let token = normalized_named_entity_token_piece(raw).unwrap_or_default();
        let boundary = query_metadata_token_has_entity_boundary(raw);
        if token_looks_like_named_entity_piece(&token) {
            current.push(token);
            if current.len() >= 5 {
                push_entity_phrase_variants(&mut entities, &mut seen, &current, max_terms);
                current.clear();
            } else if boundary {
                push_entity_phrase_variants(&mut entities, &mut seen, &current, max_terms);
                current.clear();
            }
            continue;
        }
        push_entity_phrase_variants(&mut entities, &mut seen, &current, max_terms);
        current.clear();
        if entities.len() >= max_terms {
            break;
        }
    }
    push_entity_phrase_variants(&mut entities, &mut seen, &current, max_terms);
    entities
}

fn token_matches_entity_terms(token: &str, entity_terms: &HashSet<String>) -> bool {
    let lowered = token.to_ascii_lowercase();
    entity_terms.contains(&lowered)
        || lowered
            .split('/')
            .filter(|piece| !piece.is_empty())
            .any(|piece| entity_terms.contains(piece))
}

fn raw_query_metadata_stop_token(raw: &str) -> bool {
    matches!(
        raw.to_ascii_lowercase().as_str(),
        "a" | "about"
            | "an"
            | "and"
            | "any"
            | "are"
            | "as"
            | "at"
            | "best"
            | "can"
            | "could"
            | "current"
            | "do"
            | "does"
            | "focused"
            | "focusing"
            | "focus"
            | "for"
            | "from"
            | "give"
            | "how"
            | "i"
            | "in"
            | "is"
            | "it"
            | "latest"
            | "list"
            | "me"
            | "need"
            | "new"
            | "now"
            | "of"
            | "on"
            | "or"
            | "our"
            | "please"
            | "recent"
            | "right"
            | "should"
            | "some"
            | "tell"
            | "that"
            | "the"
            | "these"
            | "this"
            | "those"
            | "to"
            | "top"
            | "up"
            | "us"
            | "use"
            | "using"
            | "want"
            | "we"
            | "what"
            | "whats"
            | "which"
            | "with"
            | "you"
            | "your"
    )
}

fn query_metadata_keyword_noise_token(raw: &str) -> bool {
    matches!(
        raw.to_ascii_lowercase().as_str(),
        "assess"
            | "compare"
            | "compared"
            | "comparing"
            | "evaluate"
            | "find"
            | "look"
            | "looking"
            | "research"
            | "summarize"
    ) || raw_query_metadata_stop_token(raw)
}

fn standalone_entity_noise_token(raw: &str) -> bool {
    matches!(
        raw.to_ascii_lowercase().as_str(),
        "ai" | "api" | "apis" | "llm" | "llms" | "ml" | "sdk" | "sdks"
    )
}

fn entity_phrase_is_too_generic(raw: &str) -> bool {
    let tokens = query_metadata_tokens(raw);
    tokens.len() == 1
        && tokens
            .first()
            .map(|token| standalone_entity_noise_token(token))
            .unwrap_or(false)
}

fn query_clause_after_marker(query: &str, marker: &str) -> Option<String> {
    let lowered = query.to_ascii_lowercase();
    let start = lowered.find(marker)? + marker.len();
    let tail = query.get(start..).unwrap_or("").trim();
    let end = tail
        .find(['.', '?', '!', ';'])
        .unwrap_or_else(|| tail.len());
    let clause = clean_text(tail.get(..end).unwrap_or(""), 260);
    if clause.is_empty() {
        None
    } else {
        Some(clause)
    }
}

fn query_metadata_facet_piece(raw: &str) -> Option<String> {
    let tokens = query_metadata_keyword_tokens(raw)
        .into_iter()
        .map(|token| token.to_ascii_lowercase())
        .filter(|token| {
            !query_metadata_keyword_noise_token(token)
                && !comparison_marker_token(token)
                && token_has_letter_or_number(token)
        })
        .take(4)
        .collect::<Vec<_>>();
    let facet = clean_text(&tokens.join(" "), 120);
    if facet.is_empty() {
        None
    } else {
        Some(facet)
    }
}

fn inferred_query_facets(query: &str, max_terms: usize) -> Vec<String> {
    let mut out = Vec::<String>::new();
    let mut seen = HashSet::<String>::new();
    for marker in [
        "focus on",
        "focused on",
        "focusing on",
        "use for",
        "used for",
        "choose for",
        "select for",
    ] {
        let Some(clause) = query_clause_after_marker(query, marker) else {
            continue;
        };
        for piece in clause
            .replace(" and ", ",")
            .split(',')
            .flat_map(|piece| piece.split('/'))
        {
            let Some(facet) = query_metadata_facet_piece(piece) else {
                continue;
            };
            push_unique_clean(&mut out, &mut seen, facet, max_terms);
            if out.len() >= max_terms {
                return out;
            }
        }
    }
    out
}

fn inferred_raw_query_term_pack(query: &str, budget: ApertureBudget) -> Option<BatchQueryKeywordPack> {
    let cleaned = clean_text(query, 600);
    if cleaned.is_empty() {
        return None;
    }
    let max_terms = budget.max_candidates.clamp(4, 12);
    let mut pack = BatchQueryKeywordPack {
        metadata_authority: "tool_structured_from_user_query_terms".to_string(),
        ..BatchQueryKeywordPack::default()
    };
    let mut seen = HashSet::<String>::new();
    for token in query_metadata_keyword_tokens(&cleaned) {
        let lowered = token.to_ascii_lowercase();
        if query_metadata_keyword_noise_token(&lowered) || !token_has_letter_or_number(&lowered) {
            continue;
        }
        push_unique_clean(&mut pack.keywords, &mut seen, lowered, max_terms);
    }
    let mut facet_seen = HashSet::<String>::new();
    for facet in inferred_query_facets(&cleaned, 6) {
        push_unique_clean(&mut pack.facets, &mut facet_seen, facet, 6);
    }
    if pack.has_positive_terms() {
        Some(pack)
    } else {
        None
    }
}

fn inferred_comparison_query_pack(query: &str, budget: ApertureBudget) -> Option<BatchQueryKeywordPack> {
    let cleaned = clean_text(query, 600);
    if cleaned.is_empty() {
        return None;
    }
    let tokens = query_metadata_tokens(&cleaned);
    if tokens.len() < 3 {
        return None;
    }
    let max_terms = budget.max_candidates.clamp(4, 12);
    let mut pack = BatchQueryKeywordPack {
        metadata_authority: "tool_inferred_from_user_query_shape".to_string(),
        ..BatchQueryKeywordPack::default()
    };
    let mut seen = HashSet::<String>::new();
    let list_entities = infer_leading_comparison_entities_from_query(&cleaned, max_terms);
    let leading_entities = if list_entities.len() >= 2 {
        list_entities
    } else {
        infer_leading_comparison_entities(&tokens)
    };
    if leading_entities.len() >= 2 {
        for entity in leading_entities {
            push_unique_clean(&mut pack.entities, &mut seen, entity, max_terms);
        }
    } else if let Some(marker_index) = tokens.iter().position(|token| comparison_marker_token(token))
    {
        let left = collect_entity_tokens_backward(&tokens, marker_index);
        let right = collect_entity_tokens_forward(&tokens, marker_index);
        if left.is_empty() || right.is_empty() {
            return None;
        }
        push_unique_clean(&mut pack.entities, &mut seen, left.join(" "), max_terms);
        push_unique_clean(&mut pack.entities, &mut seen, right.join(" "), max_terms);
    } else {
        return None;
    }
    if pack.entities.len() < 2 {
        return None;
    }

    let entity_terms = pack
        .entities
        .iter()
        .flat_map(|entity| {
            entity
                .split_whitespace()
                .map(|token| token.to_ascii_lowercase())
                .collect::<Vec<_>>()
        })
        .collect::<HashSet<_>>();
    let mut keyword_seen = HashSet::<String>::new();
    for token in tokens {
        let lowered = token.to_ascii_lowercase();
        if token_matches_entity_terms(&lowered, &entity_terms)
            || comparison_marker_token(&lowered)
            || query_metadata_keyword_noise_token(&lowered)
        {
            continue;
        }
        push_unique_clean(&mut pack.keywords, &mut keyword_seen, lowered, 4);
    }
    let mut facet_seen = HashSet::<String>::new();
    for facet in inferred_query_facets(&cleaned, 6) {
        push_unique_clean(&mut pack.facets, &mut facet_seen, facet, 6);
    }

    Some(pack)
}

fn inferred_named_entity_query_pack(
    query: &str,
    budget: ApertureBudget,
) -> Option<BatchQueryKeywordPack> {
    let cleaned = clean_text(query, 600);
    if cleaned.is_empty() {
        return None;
    }
    let max_terms = budget.max_candidates.clamp(4, 12);
    let entities = infer_named_entity_terms_from_query(&cleaned, max_terms);
    if entities.is_empty() {
        return None;
    }
    let mut pack = BatchQueryKeywordPack {
        metadata_authority: "tool_inferred_from_user_query_shape".to_string(),
        ..BatchQueryKeywordPack::default()
    };
    let mut seen = HashSet::<String>::new();
    for entity in entities {
        if entity_phrase_is_too_generic(&entity) {
            continue;
        }
        push_unique_clean(&mut pack.entities, &mut seen, entity, max_terms);
    }
    if pack.entities.is_empty() {
        return None;
    }
    let entity_terms = pack
        .entities
        .iter()
        .flat_map(|entity| {
            entity
                .split_whitespace()
                .map(|token| token.to_ascii_lowercase())
                .collect::<Vec<_>>()
        })
        .collect::<HashSet<_>>();
    let mut keyword_seen = HashSet::<String>::new();
    for token in query_metadata_tokens(&cleaned) {
        let lowered = token.to_ascii_lowercase();
        if token_matches_entity_terms(&lowered, &entity_terms)
            || query_metadata_keyword_noise_token(&lowered)
            || comparison_entity_stop_token(&lowered)
        {
            continue;
        }
        push_unique_clean(&mut pack.keywords, &mut keyword_seen, lowered, 4);
    }
    let mut facet_seen = HashSet::<String>::new();
    for facet in inferred_query_facets(&cleaned, 6) {
        push_unique_clean(&mut pack.facets, &mut facet_seen, facet, 6);
    }
    Some(pack)
}

fn quote_exact_query_term(raw: &str) -> Option<String> {
    let cleaned = clean_text(raw, 160);
    if cleaned.is_empty() {
        return None;
    }
    let unquoted = cleaned.replace('"', "");
    if unquoted.is_empty() {
        return None;
    }
    if unquoted.split_whitespace().count() > 1 {
        Some(format!("\"{unquoted}\""))
    } else {
        Some(unquoted)
    }
}

fn query_prefix_before_focus_clause(query: &str) -> String {
    let lowered = query.to_ascii_lowercase();
    let cut = [
        " focus on ",
        " focused on ",
        " focusing on ",
        " include ",
        " including ",
    ]
    .iter()
    .filter_map(|marker| lowered.find(marker))
    .min()
    .unwrap_or(query.len());
    clean_text(query.get(..cut).unwrap_or(query), 360)
}

fn query_context_subjects(query: &str, max_terms: usize) -> Vec<String> {
    let prefix = query_prefix_before_focus_clause(query);
    if prefix.is_empty() {
        return Vec::new();
    }
    let lowered = prefix.to_ascii_lowercase();
    let mut out = Vec::<String>::new();
    let mut seen = HashSet::<String>::new();
    for marker in [" around ", " about ", " regarding ", " into "] {
        let Some(index) = lowered.rfind(marker) else {
            continue;
        };
        let tail = prefix.get(index + marker.len()..).unwrap_or("");
        let subject = tail
            .split(['.', '?', '!', ';', ':', ','])
            .next()
            .unwrap_or("")
            .split_whitespace()
            .map(query_metadata_token)
            .filter(|token| {
                let lowered = token.to_ascii_lowercase();
                !lowered.is_empty()
                    && token_has_letter_or_number(&lowered)
                    && !raw_query_metadata_stop_token(&lowered)
                    && !query_metadata_keyword_noise_token(&lowered)
            })
            .take(5)
            .collect::<Vec<_>>()
            .join(" ");
        let subject = clean_text(&subject, 160);
        if subject.is_empty() || entity_phrase_is_too_generic(&subject) {
            continue;
        }
        if seen.insert(subject.to_ascii_lowercase()) {
            out.push(subject);
        }
        if out.len() >= max_terms {
            break;
        }
    }
    out
}

fn plain_query_term(raw: &str) -> Option<String> {
    let cleaned = clean_text(raw, 120).replace('"', "");
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

fn negative_query_filter(raw: &str) -> Option<String> {
    let cleaned = clean_text(raw, 80).replace('"', "");
    if cleaned.is_empty() || cleaned.starts_with('-') {
        return None;
    }
    if cleaned.split_whitespace().count() > 1 {
        Some(format!("-\"{cleaned}\""))
    } else {
        Some(format!("-{cleaned}"))
    }
}

fn append_negative_filters(candidate: &str, pack: &BatchQueryKeywordPack) -> String {
    let filters = pack
        .negative_terms
        .iter()
        .filter_map(|term| negative_query_filter(term))
        .take(2)
        .collect::<Vec<_>>();
    if filters.is_empty() {
        clean_text(candidate, 600)
    } else {
        clean_text(&format!("{candidate} {}", filters.join(" ")), 600)
    }
}

fn push_compiled_metadata_query(
    candidate: String,
    pack: &BatchQueryKeywordPack,
    dedup: &mut HashSet<String>,
    queries: &mut Vec<String>,
    max_queries: usize,
) {
    if queries.len() >= max_queries {
        return;
    }
    let candidate = append_negative_filters(&candidate, pack);
    push_query_dedup(candidate, dedup, queries);
}

fn push_subject_facet_lanes(
    subjects: &[String],
    facets: &[String],
    lane_suffix: Option<&str>,
    pack: &BatchQueryKeywordPack,
    dedup: &mut HashSet<String>,
    queries: &mut Vec<String>,
    max_queries: usize,
) {
    for facet in facets {
        for subject in subjects {
            let mut candidate = format!("{subject} {facet}");
            if let Some(suffix) = lane_suffix {
                candidate.push(' ');
                candidate.push_str(suffix);
            }
            push_compiled_metadata_query(
                clean_text(&candidate, 600),
                pack,
                dedup,
                queries,
                max_queries,
            );
            if queries.len() >= max_queries {
                return;
            }
        }
    }
}

fn push_subject_discovery_lanes(
    subjects: &[String],
    discovery_suffixes: &[&str],
    pack: &BatchQueryKeywordPack,
    dedup: &mut HashSet<String>,
    queries: &mut Vec<String>,
    max_queries: usize,
) {
    for suffix in discovery_suffixes {
        for subject in subjects {
            let candidate = clean_text(&format!("{subject} {suffix}"), 600);
            push_compiled_metadata_query(candidate, pack, dedup, queries, max_queries);
            if queries.len() >= max_queries {
                return;
            }
        }
    }
}

fn text_contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

fn keyword_pack_looks_like_local_stay_research(pack: &BatchQueryKeywordPack) -> bool {
    let haystack = pack
        .keywords
        .iter()
        .chain(pack.facets.iter())
        .map(|term| term.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join(" ");
    let local_stay_signal = text_contains_any(
        &haystack,
        &[
            "neighborhood",
            "neighborhoods",
            "where to stay",
            "stay",
            "walkability",
            "transit",
            "museum",
            "museums",
        ],
    );
    let travel_signal = text_contains_any(
        &haystack,
        &[
            "family",
            "friendly",
            "visitor",
            "visit",
            "travel",
            "hotel",
            "hotels",
            "food",
            "restaurant",
            "restaurants",
        ],
    );
    local_stay_signal && travel_signal
}

fn subject_keyword_lane_suffix(pack: &BatchQueryKeywordPack) -> &'static str {
    if keyword_pack_looks_like_local_stay_research(pack) {
        "travel guide comparison"
    } else {
        "official"
    }
}

fn subject_discovery_lane_suffixes(pack: &BatchQueryKeywordPack) -> &'static [&'static str] {
    if keyword_pack_looks_like_local_stay_research(pack) {
        &["where to stay guide", "neighborhood guide"]
    } else {
        &["official site"]
    }
}

fn push_subject_keyword_lanes(
    subjects: &[String],
    keywords: &[String],
    pack: &BatchQueryKeywordPack,
    dedup: &mut HashSet<String>,
    queries: &mut Vec<String>,
    max_queries: usize,
) {
    let subject_terms = subjects
        .iter()
        .flat_map(|subject| query_metadata_tokens(&subject.replace('"', "")))
        .map(|token| token.to_ascii_lowercase())
        .collect::<HashSet<_>>();
    let tail = keywords
        .iter()
        .filter(|keyword| {
            let tokens = query_metadata_tokens(keyword);
            !tokens.is_empty()
                && !tokens
                    .iter()
                    .all(|token| token_matches_entity_terms(token, &subject_terms))
        })
        .take(4)
        .cloned()
        .collect::<Vec<_>>()
        .join(" ");
    if tail.is_empty() {
        return;
    }
    let suffix = subject_keyword_lane_suffix(pack);
    for subject in subjects {
        let candidate = clean_text(&format!("{subject} {tail} {suffix}"), 600);
        push_compiled_metadata_query(candidate, pack, dedup, queries, max_queries);
        if queries.len() >= max_queries {
            return;
        }
    }
}

fn local_stay_research_topic(primary_query: &str, keywords: &[String]) -> String {
    let keyword_topic = clean_text(
        &keywords
            .iter()
            .take(8)
            .cloned()
            .collect::<Vec<_>>()
            .join(" "),
        300,
    );
    if !keyword_topic.is_empty() {
        return keyword_topic;
    }
    clean_text(primary_query, 300)
}

fn push_local_stay_keyword_lanes(
    primary_query: &str,
    keywords: &[String],
    pack: &BatchQueryKeywordPack,
    dedup: &mut HashSet<String>,
    queries: &mut Vec<String>,
    max_queries: usize,
) {
    if !keyword_pack_looks_like_local_stay_research(pack) {
        return;
    }
    let topic = local_stay_research_topic(primary_query, keywords);
    if topic.is_empty() {
        return;
    }
    for suffix in [
        "travel guide comparison",
        "where to stay guide",
        "neighborhood guide",
    ] {
        push_compiled_metadata_query(
            clean_text(&format!("{topic} {suffix}"), 600),
            pack,
            dedup,
            queries,
            max_queries,
        );
        if queries.len() >= max_queries {
            return;
        }
    }
}

fn push_multi_subject_combined_lanes(
    subjects: &[String],
    facets: &[String],
    comparison_intent: bool,
    pack: &BatchQueryKeywordPack,
    dedup: &mut HashSet<String>,
    queries: &mut Vec<String>,
    max_queries: usize,
) {
    if subjects.len() < 2 {
        return;
    }
    let subject_terms = subjects.iter().take(4).cloned().collect::<Vec<_>>();
    let facet_terms = facets.iter().take(2).cloned().collect::<Vec<_>>();
    let mut base_pieces = subject_terms.clone();
    base_pieces.extend(facet_terms.clone());
    if base_pieces.is_empty() {
        return;
    }
    let suffixes: &[&str] = if comparison_intent {
        &["comparison", "independent comparison", "reviews"]
    } else {
        &[
            "source-backed evidence",
            "primary source evidence",
            "recent developments",
        ]
    };
    for suffix in suffixes {
        let mut pieces = base_pieces.clone();
        pieces.push(suffix.to_string());
        push_compiled_metadata_query(
            clean_text(&pieces.join(" "), 600),
            pack,
            dedup,
            queries,
            max_queries,
        );
        if queries.len() >= max_queries {
            return;
        }
    }
    if !facet_terms.is_empty() {
        let mut pieces = subject_terms;
        pieces.extend(facet_terms);
        push_compiled_metadata_query(
            clean_text(&pieces.join(" "), 600),
            pack,
            dedup,
            queries,
            max_queries,
        );
    }
}

fn keyword_pack_requests_comparison(primary_query: &str, pack: &BatchQueryKeywordPack) -> bool {
    if is_benchmark_or_comparison_intent(primary_query) {
        return true;
    }
    let mut metadata_text = String::new();
    for value in pack
        .keywords
        .iter()
        .chain(pack.facets.iter())
        .chain(pack.aliases.iter())
        .chain(pack.entities.iter())
    {
        metadata_text.push(' ');
        metadata_text.push_str(value);
    }
    is_benchmark_or_comparison_intent(&metadata_text)
}

fn broad_facet_topic(primary_query: &str, facets: &[String], keywords: &[String]) -> String {
    let topical_facets = facets
        .iter()
        .filter(|facet| !looks_like_temporal_or_scope_facet(facet))
        .take(3)
        .cloned()
        .collect::<Vec<_>>();
    let temporal_facets = facets
        .iter()
        .filter(|facet| looks_like_temporal_or_scope_facet(facet))
        .take(1)
        .cloned()
        .collect::<Vec<_>>();
    let mut pieces = Vec::<String>::new();
    pieces.extend(topical_facets);
    pieces.extend(temporal_facets);
    if pieces.is_empty() {
        pieces.extend(keywords.iter().take(4).cloned());
    }
    let topic = clean_text(&pieces.join(" "), 240);
    if topic.is_empty() {
        clean_text(primary_query, 240)
    } else {
        topic
    }
}

fn looks_like_temporal_or_scope_facet(raw: &str) -> bool {
    let lowered = clean_text(raw, 240).to_ascii_lowercase();
    [
        "this week",
        "this month",
        "this year",
        "last week",
        "last month",
        "today",
        "current",
        "recent",
        "latest",
        "2025",
        "2026",
        "2027",
    ]
    .iter()
    .any(|needle| lowered.contains(needle))
}

fn broad_facet_sentiment_intent(primary_query: &str, facets: &[String], keywords: &[String]) -> bool {
    let mut text = clean_text(primary_query, 600).to_ascii_lowercase();
    for value in facets.iter().chain(keywords.iter()) {
        text.push(' ');
        text.push_str(&clean_text(value, 160).to_ascii_lowercase());
    }
    ["sentiment", "saying", "reviews", "complaints", "praise"]
        .iter()
        .any(|needle| text.contains(needle))
}

fn push_broad_facet_lanes(
    primary_query: &str,
    facets: &[String],
    keywords: &[String],
    pack: &BatchQueryKeywordPack,
    dedup: &mut HashSet<String>,
    queries: &mut Vec<String>,
    max_queries: usize,
) {
    let topical_facets = facets
        .iter()
        .filter(|facet| !looks_like_temporal_or_scope_facet(facet))
        .map(|facet| clean_text(facet, 160))
        .filter(|facet| !facet.is_empty())
        .collect::<Vec<_>>();
    let topic = broad_facet_topic(primary_query, facets, keywords);
    if !topic.is_empty() {
        push_compiled_metadata_query(
            clean_text(&format!("{topic} latest developments"), 600),
            pack,
            dedup,
            queries,
            max_queries,
        );
        if queries.len() >= max_queries {
            return;
        }
    }

    if topical_facets.len() >= 2 {
        for facet in &topical_facets {
            push_compiled_metadata_query(
                clean_text(&format!("{facet} source-backed evidence"), 600),
                pack,
                dedup,
                queries,
                max_queries,
            );
            if queries.len() >= max_queries {
                return;
            }
        }
    }

    if !topic.is_empty() {
        for suffix in [
            "primary source evidence",
            "official reports announcements",
            "independent analysis",
        ] {
            push_compiled_metadata_query(
                clean_text(&format!("{topic} {suffix}"), 600),
                pack,
                dedup,
                queries,
                max_queries,
            );
            if queries.len() >= max_queries {
                return;
            }
        }
        if broad_facet_sentiment_intent(primary_query, facets, keywords) {
            for suffix in ["public sentiment user reports", "reviews complaints praise"] {
                push_compiled_metadata_query(
                    clean_text(&format!("{topic} {suffix}"), 600),
                    pack,
                    dedup,
                    queries,
                    max_queries,
                );
                if queries.len() >= max_queries {
                    return;
                }
            }
        }
    }

    let has_topical_facet = facets
        .iter()
        .any(|facet| !looks_like_temporal_or_scope_facet(facet));
    for facet in facets {
        let facet = clean_text(facet, 160);
        if facet.is_empty() {
            continue;
        }
        if has_topical_facet && looks_like_temporal_or_scope_facet(&facet) {
            continue;
        }
        for suffix in ["source-backed evidence", "recent developments"] {
            push_compiled_metadata_query(
                clean_text(&format!("{facet} {suffix}"), 600),
                pack,
                dedup,
                queries,
                max_queries,
            );
            if queries.len() >= max_queries {
                return;
            }
        }
    }
}

fn compile_keyword_pack_queries(
    primary_query: &str,
    pack: &BatchQueryKeywordPack,
    budget: ApertureBudget,
) -> Vec<String> {
    if !pack.has_positive_terms() {
        return Vec::new();
    }
    let max_queries = max_explicit_queries_for_budget(primary_query, budget);
    let mut dedup = HashSet::<String>::new();
    let mut queries = Vec::<String>::new();

    let exact_subjects = pack
        .entities
        .iter()
        .chain(pack.aliases.iter())
        .filter_map(|term| quote_exact_query_term(term))
        .collect::<Vec<_>>();
    let context_subjects = if exact_subjects.is_empty() {
        query_context_subjects(primary_query, 2)
            .iter()
            .filter_map(|term| quote_exact_query_term(term))
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let facets = pack
        .facets
        .iter()
        .filter_map(|term| plain_query_term(term))
        .collect::<Vec<_>>();
    let exact_facets = pack
        .facets
        .iter()
        .filter_map(|term| quote_exact_query_term(term))
        .collect::<Vec<_>>();
    let keywords = pack
        .keywords
        .iter()
        .filter_map(|term| plain_query_term(term))
        .collect::<Vec<_>>();
    let comparison_intent = keyword_pack_requests_comparison(primary_query, pack);

    if !exact_subjects.is_empty() {
        push_multi_subject_combined_lanes(
            &exact_subjects,
            &facets,
            comparison_intent,
            pack,
            &mut dedup,
            &mut queries,
            max_queries,
        );
        if queries.len() >= max_queries {
            return queries;
        }
        push_subject_keyword_lanes(
            &exact_subjects,
            &keywords,
            pack,
            &mut dedup,
            &mut queries,
            max_queries,
        );
        if queries.len() >= max_queries {
            return queries;
        }
        if facets.is_empty() {
            push_subject_discovery_lanes(
                &exact_subjects,
                subject_discovery_lane_suffixes(pack),
                pack,
                &mut dedup,
                &mut queries,
                max_queries,
            );
            if queries.len() >= max_queries {
                return queries;
            }
        }
    }

    if !exact_subjects.is_empty() && !facets.is_empty() {
        let prioritized_facets = facets.iter().take(4).cloned().collect::<Vec<_>>();
        for suffix in [
            Some("official documentation"),
            Some("primary source evidence"),
            None,
        ] {
            push_subject_facet_lanes(
                &exact_subjects,
                &prioritized_facets,
                suffix,
                pack,
                &mut dedup,
                &mut queries,
                max_queries,
            );
            if queries.len() >= max_queries {
                return queries;
            }
        }
    } else if !context_subjects.is_empty() && !exact_facets.is_empty() {
        let prioritized_facets = exact_facets.iter().take(4).cloned().collect::<Vec<_>>();
        for suffix in [
            Some("official documentation"),
            Some("primary source evidence"),
            None,
        ] {
            push_subject_facet_lanes(
                &context_subjects,
                &prioritized_facets,
                suffix,
                pack,
                &mut dedup,
                &mut queries,
                max_queries,
            );
            if queries.len() >= max_queries {
                return queries;
            }
        }
    } else {
        for subject in &exact_subjects {
            let tail = keywords.iter().take(3).cloned().collect::<Vec<_>>().join(" ");
            if !tail.is_empty() {
                let candidate = clean_text(&format!("{subject} {tail}"), 600);
                push_compiled_metadata_query(
                    candidate,
                    pack,
                    &mut dedup,
                    &mut queries,
                    max_queries,
                );
                if queries.len() >= max_queries {
                    return queries;
                }
            }
            for source_lane in ["official source", "primary source evidence"] {
                let candidate = clean_text(&format!("{subject} {source_lane}"), 600);
                push_compiled_metadata_query(
                    candidate,
                    pack,
                    &mut dedup,
                    &mut queries,
                    max_queries,
                );
                if queries.len() >= max_queries {
                    return queries;
                }
            }
            if queries.len() >= max_queries {
                return queries;
            }
        }
    }

    if !exact_subjects.is_empty() && queries.len() < max_queries {
        push_subject_discovery_lanes(
            &exact_subjects,
            &[
                "official documentation",
                "official source",
                "primary source evidence",
            ],
            pack,
            &mut dedup,
            &mut queries,
            max_queries,
        );
        if queries.len() >= max_queries {
            return queries;
        }
    }

    if comparison_intent && exact_subjects.len() >= 2 && queries.len() < max_queries {
        let subject_terms = exact_subjects.iter().take(3).cloned().collect::<Vec<_>>();
        let mut pieces = subject_terms;
        pieces.push("comparison".to_string());
        if let Some(facet) = facets.first() {
            pieces.push(facet.clone());
        }
        push_compiled_metadata_query(
            pieces.join(" "),
            pack,
            &mut dedup,
            &mut queries,
            max_queries,
        );
    }

    if exact_subjects.is_empty() && queries.len() < max_queries && !facets.is_empty() {
        push_broad_facet_lanes(
            primary_query,
            &facets,
            &keywords,
            pack,
            &mut dedup,
            &mut queries,
            max_queries,
        );
    }

    if exact_subjects.is_empty() && facets.is_empty() && queries.len() < max_queries {
        push_local_stay_keyword_lanes(
            primary_query,
            &keywords,
            pack,
            &mut dedup,
            &mut queries,
            max_queries,
        );
    }

    if queries.len() < max_queries && !keywords.is_empty() {
        let mut pieces = Vec::<String>::new();
        if let Some(subject) = exact_subjects.first() {
            pieces.push(subject.clone());
        } else {
            let primary = clean_text(primary_query, 180);
            if !primary.is_empty() {
                pieces.push(primary);
            }
        }
        pieces.extend(keywords.iter().take(4).cloned());
        push_compiled_metadata_query(
            pieces.join(" "),
            pack,
            &mut dedup,
            &mut queries,
            max_queries,
        );
    }

    queries
}

fn max_explicit_queries_for_budget(primary_query: &str, budget: ApertureBudget) -> usize {
    let _ = primary_query;
    budget.max_candidates.clamp(2, 12)
}

fn initial_query_execution_max_lanes(policy: &Value, budget: ApertureBudget) -> usize {
    policy
        .pointer("/batch_query/query_execution_budget/max_initial_lanes")
        .or_else(|| {
            policy.pointer("/batch_query/coverage_aware_query_planning/budget/initial_execution_lanes")
        })
        .and_then(Value::as_u64)
        .unwrap_or(3)
        .clamp(1, budget.max_candidates.clamp(1, 12) as u64) as usize
}

fn execution_limited_initial_queries(
    policy: &Value,
    budget: ApertureBudget,
    submitted_queries: &[String],
) -> Vec<String> {
    let max_lanes = initial_query_execution_max_lanes(policy, budget);
    if submitted_queries.len() <= max_lanes {
        return submitted_queries.to_vec();
    }
    submitted_queries.iter().take(max_lanes).cloned().collect()
}

fn deferred_query_recovery_enabled(policy: &Value) -> bool {
    policy
        .pointer("/batch_query/query_execution_budget/deferred_recovery/enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

fn deferred_query_recovery_max_lanes(policy: &Value, budget: ApertureBudget) -> usize {
    policy
        .pointer("/batch_query/query_execution_budget/deferred_recovery/max_lanes")
        .and_then(Value::as_u64)
        .unwrap_or(2)
        .clamp(1, second_pass_recovery_max_queries(policy, budget).max(1) as u64) as usize
}

fn deferred_query_recovery_queries(
    policy: &Value,
    budget: ApertureBudget,
    submitted_queries: &[String],
    executed_queries: &[String],
) -> Vec<String> {
    if !deferred_query_recovery_enabled(policy) {
        return Vec::new();
    }
    let executed = executed_queries
        .iter()
        .map(|row| clean_text(row, 600).to_ascii_lowercase())
        .collect::<HashSet<_>>();
    let max_lanes = deferred_query_recovery_max_lanes(policy, budget);
    let mut seen = executed.clone();
    submitted_queries
        .iter()
        .filter_map(|query| {
            let cleaned = clean_text(query, 600);
            if cleaned.is_empty() {
                return None;
            }
            let key = cleaned.to_ascii_lowercase();
            if executed.contains(&key) || !seen.insert(key) {
                return None;
            }
            Some(cleaned)
        })
        .take(max_lanes)
        .collect()
}

fn metadata_expansion_budget(explicit_query_count: usize, max_queries: usize) -> usize {
    let remaining = max_queries.saturating_sub(explicit_query_count);
    if explicit_query_count >= 4 {
        remaining.min(3)
    } else if explicit_query_count >= 2 {
        remaining.min(5)
    } else {
        remaining
    }
}

fn normalize_requested_queries(
    request: &Value,
    primary_query: &str,
    budget: ApertureBudget,
    keyword_pack: &BatchQueryKeywordPack,
) -> Vec<String> {
    let mut dedup = HashSet::<String>::new();
    let mut queries = Vec::<String>::new();
    let max_queries = max_explicit_queries_for_budget(primary_query, budget);
    let normalized_primary = clean_text(primary_query, 600);
    if !normalized_primary.is_empty() {
        push_query_dedup(normalized_primary, &mut dedup, &mut queries);
        push_temporal_scope_query_lanes(primary_query, &mut dedup, &mut queries, max_queries);
    }
    let compiled_metadata = compile_keyword_pack_queries(primary_query, keyword_pack, budget);
    let explicit_queries = request
        .get("queries")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|row| extract_request_query_row(row, 600))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let has_explicit_queries = !explicit_queries.is_empty();
    let explicit_head_count = if has_explicit_queries {
        if query_pack_has_coverage_terms(keyword_pack) {
            1.min(explicit_queries.len())
        } else {
            2.min(explicit_queries.len())
        }
    } else {
        0
    };
    for value in explicit_queries.iter().take(explicit_head_count).cloned() {
        if queries.len() >= max_queries {
            break;
        }
        push_query_dedup(value, &mut dedup, &mut queries);
    }
    let top_metadata_budget = if has_explicit_queries
        && query_pack_has_coverage_terms(keyword_pack)
    {
        let subject_count = keyword_pack.entities.len() + keyword_pack.aliases.len();
        let topical_facet_count = keyword_pack
            .facets
            .iter()
            .filter_map(|term| plain_query_term(term))
            .filter(|term| !looks_like_temporal_or_scope_facet(term))
            .count()
            .min(4);
        let facet_frontload_budget = if topical_facet_count >= 2 {
            1 + topical_facet_count
        } else {
            topical_facet_count
        };
        if subject_count >= 2 && !keyword_pack.facets.is_empty() {
            (4 + subject_count.min(4)).max(facet_frontload_budget)
        } else if subject_count >= 2 || !keyword_pack.facets.is_empty() {
            3.max(facet_frontload_budget)
        } else {
            1
        }
    } else {
        0
    };
    for value in compiled_metadata
        .iter()
        .take(top_metadata_budget)
        .cloned()
    {
        if queries.len() >= max_queries {
            break;
        }
        push_query_dedup(value, &mut dedup, &mut queries);
    }
    for value in explicit_queries.into_iter().skip(explicit_head_count) {
        if queries.len() >= max_queries {
            break;
        }
        push_query_dedup(value, &mut dedup, &mut queries);
    }
    let metadata_budget = if top_metadata_budget >= 6 {
        0
    } else {
        metadata_expansion_budget(queries.len(), max_queries)
    };
    for value in compiled_metadata
        .into_iter()
        .skip(top_metadata_budget)
        .take(metadata_budget)
    {
        if queries.len() >= max_queries {
            break;
        }
        push_query_dedup(value, &mut dedup, &mut queries);
    }
    queries
}

fn query_pack_has_coverage_terms(pack: &BatchQueryKeywordPack) -> bool {
    !pack.entities.is_empty() || !pack.aliases.is_empty() || !pack.facets.is_empty()
}

fn query_pack_has_recovery_merge_terms(pack: &BatchQueryKeywordPack) -> bool {
    query_pack_has_coverage_terms(pack) || keyword_pack_looks_like_local_stay_research(pack)
}

fn query_pack_has_facet_terms(pack: &BatchQueryKeywordPack) -> bool {
    !pack.facets.is_empty()
}

fn merge_recovery_queries_with_metadata(
    primary_query: &str,
    recovery_queries: &[String],
    keyword_pack: &BatchQueryKeywordPack,
    budget: ApertureBudget,
) -> Vec<String> {
    if recovery_queries.is_empty() || !query_pack_has_recovery_merge_terms(keyword_pack) {
        return recovery_queries.to_vec();
    }
    let max_queries = max_explicit_queries_for_budget(primary_query, budget);
    let mut dedup = HashSet::<String>::new();
    let mut queries = Vec::<String>::new();
    if let Some(primary) = recovery_queries.first() {
        push_query_dedup(primary.clone(), &mut dedup, &mut queries);
    }

    let compiled_metadata = compile_keyword_pack_queries(primary_query, keyword_pack, budget);
    if query_pack_has_recovery_merge_terms(keyword_pack) {
        for value in &compiled_metadata {
            if queries.len() >= max_queries {
                return queries;
            }
            push_query_dedup(value.clone(), &mut dedup, &mut queries);
        }
    }

    for value in recovery_queries.iter().skip(1) {
        if queries.len() >= max_queries {
            break;
        }
        push_query_dedup(value.clone(), &mut dedup, &mut queries);
    }

    if !query_pack_has_facet_terms(keyword_pack) {
        for value in compiled_metadata {
            if queries.len() >= max_queries {
                break;
            }
            push_query_dedup(value, &mut dedup, &mut queries);
        }
    }
    queries
}

fn broad_current_research_recovery_enabled(policy: &Value) -> bool {
    policy
        .pointer("/batch_query/query_recovery/broad_current_research/enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

fn broad_current_research_recovery_max_queries(policy: &Value, budget: ApertureBudget) -> usize {
    policy
        .pointer("/batch_query/query_recovery/broad_current_research/max_queries")
        .and_then(Value::as_u64)
        .unwrap_or(4)
        .clamp(1, max_explicit_queries_for_budget("", budget) as u64) as usize
}

fn query_recovery_policy_strings(policy: &Value, pointer: &str, max_len: usize) -> Vec<String> {
    policy
        .pointer(pointer)
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| clean_text(row, max_len))
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn query_recovery_policy_strings_with_default(
    policy: &Value,
    pointer: &str,
    max_len: usize,
) -> Vec<String> {
    let configured = query_recovery_policy_strings(policy, pointer, max_len);
    if configured.is_empty() {
        query_recovery_policy_strings(&default_policy(), pointer, max_len)
    } else {
        configured
    }
}

fn broad_current_research_recovery_templates(policy: &Value) -> Vec<String> {
    query_recovery_policy_strings_with_default(
        policy,
        "/batch_query/query_recovery/broad_current_research/templates",
        600,
    )
}

fn broad_current_research_recovery_markers(policy: &Value) -> Vec<String> {
    query_recovery_policy_strings_with_default(
        policy,
        "/batch_query/query_recovery/broad_current_research/intent_markers",
        80,
    )
    .into_iter()
    .map(|row| row.to_ascii_lowercase())
    .collect()
}

fn query_looks_like_broad_current_research(policy: &Value, query: &str) -> bool {
    let cleaned = clean_text(query, 600);
    if cleaned.is_empty() || !current_web_intent(&cleaned) {
        return false;
    }
    let lowered = cleaned.to_ascii_lowercase();
    if lowered.contains("http://")
        || lowered.contains("https://")
        || lowered.contains('"')
        || lowered.contains('`')
    {
        return false;
    }
    let word_count = cleaned.split_whitespace().count();
    let broad_marker = broad_current_research_recovery_markers(policy)
        .iter()
        .any(|marker| lowered.contains(marker));
    broad_marker || word_count <= 8
}

fn expand_query_recovery_template(template: &str, query: &str) -> Option<String> {
    let year = current_year();
    if template.contains("{current_year}") && query.to_ascii_lowercase().contains(&year) {
        return None;
    }
    let month_name = current_month_name();
    if template.contains("{current_month_name}")
        && (!query_uses_relative_current_window(query) || query_already_has_current_month_scope(query))
    {
        return None;
    }
    let expanded = template
        .replace("{query}", query)
        .replace("{current_year}", &year)
        .replace("{current_month_name}", month_name);
    let cleaned = clean_text(&expanded, 600);
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

fn broad_current_research_recovery_queries(
    policy: &Value,
    query: &str,
    budget: ApertureBudget,
) -> Vec<String> {
    if !broad_current_research_recovery_enabled(policy)
        || !query_looks_like_broad_current_research(policy, query)
    {
        return Vec::new();
    }
    let max_queries = broad_current_research_recovery_max_queries(policy, budget);
    let mut dedup = HashSet::<String>::new();
    let mut queries = Vec::<String>::new();
    let cleaned_query = clean_text(query, 600);
    if !cleaned_query.is_empty() {
        push_query_dedup(cleaned_query, &mut dedup, &mut queries);
        push_recovery_temporal_scope_query_lanes(query, &mut dedup, &mut queries, max_queries);
    }
    for template in broad_current_research_recovery_templates(policy) {
        if queries.len() >= max_queries {
            break;
        }
        if let Some(value) = expand_query_recovery_template(&template, query) {
            let key = value.to_ascii_lowercase();
            if dedup.insert(key) {
                queries.push(value);
            }
        }
    }
    if queries.len() <= 1 {
        Vec::new()
    } else {
        queries
    }
}

fn general_research_recovery_enabled(policy: &Value) -> bool {
    policy
        .pointer("/batch_query/query_recovery/general_research/enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

fn general_research_recovery_max_queries(policy: &Value, budget: ApertureBudget) -> usize {
    policy
        .pointer("/batch_query/query_recovery/general_research/max_queries")
        .and_then(Value::as_u64)
        .unwrap_or(6)
        .clamp(1, max_explicit_queries_for_budget("", budget) as u64) as usize
}

fn general_research_recovery_templates(policy: &Value) -> Vec<String> {
    query_recovery_policy_strings_with_default(
        policy,
        "/batch_query/query_recovery/general_research/templates",
        600,
    )
}

fn general_research_recovery_markers(policy: &Value) -> Vec<String> {
    query_recovery_policy_strings_with_default(
        policy,
        "/batch_query/query_recovery/general_research/intent_markers",
        80,
    )
    .into_iter()
    .map(|row| row.to_ascii_lowercase())
    .collect()
}

fn query_looks_like_general_research(policy: &Value, query: &str) -> bool {
    let cleaned = clean_text(query, 600);
    if cleaned.is_empty() {
        return false;
    }
    let lowered = cleaned.to_ascii_lowercase();
    if lowered.contains("http://")
        || lowered.contains("https://")
        || lowered.contains('"')
        || lowered.contains('`')
        || looks_like_internal_route_query(&cleaned)
    {
        return false;
    }
    if is_framework_catalog_intent(&cleaned) {
        return false;
    }
    general_research_recovery_markers(policy)
        .iter()
        .any(|marker| lowered.contains(marker))
}

fn general_research_recovery_queries(
    policy: &Value,
    query: &str,
    budget: ApertureBudget,
) -> Vec<String> {
    if !general_research_recovery_enabled(policy)
        || !query_looks_like_general_research(policy, query)
    {
        return Vec::new();
    }
    let max_queries = general_research_recovery_max_queries(policy, budget);
    let mut dedup = HashSet::<String>::new();
    let mut queries = Vec::<String>::new();
    let cleaned_query = clean_text(query, 600);
    if !cleaned_query.is_empty() {
        push_query_dedup(cleaned_query, &mut dedup, &mut queries);
        push_recovery_temporal_scope_query_lanes(query, &mut dedup, &mut queries, max_queries);
    }
    for template in general_research_recovery_templates(policy) {
        if queries.len() >= max_queries {
            break;
        }
        if let Some(value) = expand_query_recovery_template(&template, query) {
            let key = value.to_ascii_lowercase();
            if dedup.insert(key) {
                queries.push(value);
            }
        }
    }
    if queries.len() <= 1 {
        Vec::new()
    } else {
        queries
    }
}

fn quality_gate_enabled(policy: &Value) -> bool {
    policy
        .pointer("/batch_query/quality_gate/enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

fn provider_recovery_enabled(policy: &Value) -> bool {
    quality_gate_enabled(policy)
        && policy
            .pointer("/batch_query/quality_gate/provider_recovery/enabled")
            .and_then(Value::as_bool)
            .unwrap_or(false)
}

fn provider_recovery_max_providers(policy: &Value) -> usize {
    policy
        .pointer("/batch_query/quality_gate/provider_recovery/max_providers")
        .and_then(Value::as_u64)
        .unwrap_or(1)
        .clamp(1, 6) as usize
}

fn provider_recovery_list(policy: &Value, pointer: &str) -> Vec<String> {
    policy
        .pointer(pointer)
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| clean_text(row, 80).to_ascii_lowercase())
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn official_source_provider_recovery_providers(policy: &Value) -> Vec<String> {
    let configured = provider_recovery_list(
        policy,
        "/batch_query/quality_gate/provider_recovery/official_source_providers",
    );
    let base = if configured.is_empty() {
        vec![
            "tavily".to_string(),
            "exa".to_string(),
            "brave".to_string(),
            "serperdev".to_string(),
            "browser_serp".to_string(),
            "duckduckgo_lite".to_string(),
            "duckduckgo".to_string(),
        ]
    } else {
        configured
    };
    base.into_iter()
        .filter(|provider| {
            matches!(
                provider.as_str(),
                "tavily"
                    | "exa"
                    | "brave"
                    | "serperdev"
                    | "browser_serp"
                    | "duckduckgo_lite"
                    | "duckduckgo"
            )
        })
        .collect()
}

fn provider_recovery_providers(policy: &Value, query: &str) -> Vec<String> {
    if !provider_recovery_enabled(policy) {
        return Vec::new();
    }
    let mut dedup = HashSet::<String>::new();
    let mut providers = Vec::<String>::new();
    let mut push_provider = |provider: String| {
        if provider.is_empty() {
            return;
        }
        if dedup.insert(provider.clone()) {
            providers.push(provider);
        }
    };
    if is_official_source_query_lane(query) {
        for provider in official_source_provider_recovery_providers(policy) {
            push_provider(provider);
        }
        providers.truncate(provider_recovery_max_providers(policy));
        return providers;
    }
    if current_web_intent(query) {
        for provider in provider_recovery_list(
            policy,
            "/batch_query/quality_gate/provider_recovery/current_intent_providers",
        ) {
            push_provider(provider);
        }
    }
    for provider in provider_recovery_list(
        policy,
        "/batch_query/quality_gate/provider_recovery/providers",
    ) {
        push_provider(provider);
    }
    providers.truncate(provider_recovery_max_providers(policy));
    providers
}

fn low_confidence_retention_enabled(policy: &Value) -> bool {
    policy
        .pointer("/batch_query/result_retention/retain_low_confidence_raw_results")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn low_confidence_retention_max_items(policy: &Value, budget: ApertureBudget) -> usize {
    policy
        .pointer("/batch_query/result_retention/max_low_confidence_items")
        .and_then(Value::as_u64)
        .unwrap_or(budget.max_evidence as u64)
        .clamp(1, budget.max_candidates.max(1) as u64) as usize
}

fn facet_aware_evidence_enabled(policy: &Value) -> bool {
    policy
        .pointer("/batch_query/coverage_aware_evidence/enabled")
        .and_then(Value::as_bool)
        .or_else(|| {
            policy
                .pointer("/batch_query/coverage_aware_query_planning/coverage_buckets/enabled")
                .and_then(Value::as_bool)
        })
        .unwrap_or(false)
}

fn facet_aware_max_facets(policy: &Value, budget: ApertureBudget) -> usize {
    policy
        .pointer("/batch_query/coverage_aware_evidence/max_facets")
        .or_else(|| policy.pointer("/batch_query/coverage_aware_query_planning/budget/default_max_lanes"))
        .and_then(Value::as_u64)
        .unwrap_or(8)
        .clamp(1, budget.max_candidates.clamp(1, 16) as u64) as usize
}

fn facet_aware_min_terms(policy: &Value) -> usize {
    policy
        .pointer("/batch_query/coverage_aware_evidence/min_facet_terms")
        .and_then(Value::as_u64)
        .unwrap_or(2)
        .clamp(1, 6) as usize
}

fn second_pass_recovery_enabled(policy: &Value) -> bool {
    policy
        .pointer("/batch_query/second_pass_recovery/enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn second_pass_recovery_max_queries(policy: &Value, budget: ApertureBudget) -> usize {
    policy
        .pointer("/batch_query/second_pass_recovery/max_queries")
        .and_then(Value::as_u64)
        .unwrap_or(3)
        .clamp(1, budget.max_candidates.clamp(1, 8) as u64) as usize
}

fn second_pass_recovery_templates(policy: &Value) -> Vec<String> {
    policy
        .pointer("/batch_query/second_pass_recovery/templates")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| clean_text(row, 240))
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| {
            vec![
                "{query} source-backed evidence".to_string(),
                "{query} primary or official source".to_string(),
                "{query} independent evaluation or source-backed analysis".to_string(),
            ]
        })
}

fn coverage_gap_recovery_enabled(policy: &Value) -> bool {
    policy
        .pointer("/batch_query/coverage_gap_recovery/enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn coverage_gap_recovery_min_usable_evidence(policy: &Value, budget: ApertureBudget) -> usize {
    policy
        .pointer("/batch_query/coverage_gap_recovery/min_usable_evidence")
        .and_then(Value::as_u64)
        .unwrap_or(3)
        .clamp(1, budget.max_evidence.max(1) as u64) as usize
}

fn coverage_gap_recovery_min_covered_facets(policy: &Value, facet_count: usize, budget: ApertureBudget) -> usize {
    if facet_count == 0 {
        return 0;
    }
    let configured_ratio = policy
        .pointer("/batch_query/coverage_gap_recovery/min_covered_facet_ratio")
        .and_then(Value::as_f64)
        .unwrap_or(0.5)
        .clamp(0.0, 1.0);
    let ratio_target = ((facet_count as f64) * configured_ratio).ceil() as usize;
    let configured_min = policy
        .pointer("/batch_query/coverage_gap_recovery/min_covered_facets")
        .and_then(Value::as_u64)
        .unwrap_or(2) as usize;
    ratio_target
        .max(configured_min)
        .min(facet_count)
        .min(budget.max_evidence.max(1))
}

fn coverage_gap_recovery_max_queries(policy: &Value, budget: ApertureBudget) -> usize {
    policy
        .pointer("/batch_query/coverage_gap_recovery/max_queries")
        .and_then(Value::as_u64)
        .unwrap_or_else(|| second_pass_recovery_max_queries(policy, budget) as u64)
        .clamp(1, budget.max_candidates.clamp(1, 8) as u64) as usize
}

fn coverage_gap_recovery_templates(policy: &Value) -> Vec<String> {
    policy
        .pointer("/batch_query/coverage_gap_recovery/templates")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| clean_text(row, 320))
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>()
        })
        .filter(|rows| !rows.is_empty())
        .unwrap_or_else(|| {
            vec![
                "{facet} source-backed evidence".to_string(),
                "{facet} primary or official source".to_string(),
                "{facet} independent analysis evidence".to_string(),
                "{facet} examples reports data".to_string(),
            ]
        })
}

fn claim_gap_recovery_enabled(policy: &Value) -> bool {
    policy
        .pointer("/batch_query/claim_gap_recovery/enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn claim_gap_recovery_min_materialized_evidence(
    policy: &Value,
    budget: ApertureBudget,
) -> usize {
    policy
        .pointer("/batch_query/claim_gap_recovery/min_materialized_evidence")
        .and_then(Value::as_u64)
        .unwrap_or(2)
        .clamp(1, budget.max_evidence.max(1) as u64) as usize
}

fn claim_gap_recovery_min_claim_hints(policy: &Value, budget: ApertureBudget) -> usize {
    policy
        .pointer("/batch_query/claim_gap_recovery/min_claim_hints")
        .and_then(Value::as_u64)
        .unwrap_or(3)
        .clamp(1, (budget.max_evidence.max(1) * 2) as u64) as usize
}

fn claim_gap_recovery_max_queries(policy: &Value, budget: ApertureBudget) -> usize {
    policy
        .pointer("/batch_query/claim_gap_recovery/max_queries")
        .and_then(Value::as_u64)
        .unwrap_or_else(|| second_pass_recovery_max_queries(policy, budget) as u64)
        .clamp(1, budget.max_candidates.clamp(1, 8) as u64) as usize
}

fn claim_gap_recovery_templates(policy: &Value) -> Vec<String> {
    policy
        .pointer("/batch_query/claim_gap_recovery/templates")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| clean_text(row, 320))
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>()
        })
        .filter(|rows| !rows.is_empty())
        .unwrap_or_else(|| {
            vec![
                "{query} detailed findings".to_string(),
                "{query} source-backed evidence".to_string(),
            ]
        })
}

fn second_pass_recovery_queries(
    policy: &Value,
    query: &str,
    existing_queries: &[String],
    budget: ApertureBudget,
) -> Vec<String> {
    if !second_pass_recovery_enabled(policy) {
        return Vec::new();
    }
    let base = clean_text(query, 600);
    if base.is_empty() {
        return Vec::new();
    }
    let mut seen = existing_queries
        .iter()
        .map(|row| clean_text(row, 600).to_ascii_lowercase())
        .collect::<HashSet<_>>();
    let mut out = Vec::<String>::new();
    for template in second_pass_recovery_templates(policy) {
        let candidate = clean_text(&template.replace("{query}", &base), 600);
        if candidate.is_empty() {
            continue;
        }
        if seen.insert(candidate.to_ascii_lowercase()) {
            out.push(candidate);
        }
        if out.len() >= second_pass_recovery_max_queries(policy, budget) {
            break;
        }
    }
    out
}

fn retrieval_telemetry_enabled(policy: &Value) -> bool {
    policy
        .pointer("/batch_query/retrieval_telemetry/enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn provider_artifact_count_field(artifact: &Value, key: &str) -> usize {
    artifact.get(key).and_then(Value::as_u64).unwrap_or(0) as usize
}

fn retrieval_telemetry_row(
    query: &str,
    phase: &str,
    rows: &[Candidate],
    issues: &[String],
    artifacts: &[Value],
) -> Value {
    let provider_raw_rows = artifacts
        .iter()
        .map(|artifact| {
            provider_artifact_count_field(artifact, "provider_raw_count").max(
                provider_artifact_count_field(artifact, "provider_candidate_count")
                    .max(provider_artifact_count_field(artifact, "synthesis_candidate_count")),
            )
        })
        .sum::<usize>();
    let synthesis_rows = rows
        .iter()
        .filter(|row| !candidate_is_low_confidence_retained(row))
        .count();
    let low_confidence_rows = rows.len().saturating_sub(synthesis_rows);
    let failure_reasons = issues
        .iter()
        .map(|issue| clean_text(issue, 180))
        .filter(|issue| !issue.is_empty())
        .take(12)
        .collect::<Vec<_>>();
    json!({
        "query": clean_text(query, 600),
        "phase": clean_text(phase, 80),
        "provider_count": artifacts.len(),
        "provider_raw_rows": provider_raw_rows,
        "candidate_rows": rows.len(),
        "synthesis_candidate_rows": synthesis_rows,
        "low_confidence_raw_rows": low_confidence_rows,
        "filtered_or_rejected_rows": issues.len(),
        "failure_reasons": failure_reasons
    })
}

#[derive(Clone, Debug)]
struct QueryLaneSource {
    query: String,
    phase: String,
    candidates: Vec<Candidate>,
    issues: Vec<String>,
    artifacts: Vec<Value>,
}

fn query_lane_source(
    query: &str,
    phase: &str,
    rows: &[Candidate],
    issues: &[String],
    artifacts: &[Value],
) -> QueryLaneSource {
    QueryLaneSource {
        query: clean_text(query, 600),
        phase: clean_text(phase, 80),
        candidates: rows.to_vec(),
        issues: issues.to_vec(),
        artifacts: artifacts.to_vec(),
    }
}

fn preserve_trusted_primary_lane_candidates(
    query: &str,
    selected: &mut Vec<(Candidate, f64)>,
    supplemental_pool: &[(Candidate, f64)],
    lane_sources: &[QueryLaneSource],
    facets: &[ResearchFacet],
    max_evidence: usize,
    min_terms: usize,
) -> usize {
    if max_evidence == 0 || supplemental_pool.is_empty() || lane_sources.is_empty() {
        return 0;
    }
    let mut selected_keys = selected
        .iter()
        .map(|(candidate, _)| candidate_identity_key(candidate))
        .collect::<HashSet<_>>();
    let mut pending = Vec::<(Candidate, f64)>::new();
    let mut pending_keys = HashSet::<String>::new();
    let mut covered_facet_ids = selected
        .iter()
        .flat_map(|(candidate, _)| selected_candidate_coverage_ids(facets, candidate, min_terms))
        .collect::<HashSet<_>>();

    for source in lane_sources {
        if !is_official_source_query_lane(&source.query) || source.candidates.is_empty() {
            continue;
        }
        let uncovered_lane_facet_ids = source
            .candidates
            .iter()
            .flat_map(|candidate| selected_candidate_coverage_ids(facets, candidate, min_terms))
            .filter(|facet_id| !covered_facet_ids.contains(facet_id))
            .collect::<HashSet<_>>();
        let lane_keys = source
            .candidates
            .iter()
            .map(candidate_identity_key)
            .collect::<HashSet<_>>();
        if lane_keys.iter().any(|key| selected_keys.contains(key)) {
            continue;
        }
        let mut ranked = supplemental_pool
            .iter()
            .filter(|(candidate, score)| {
                if !lane_keys.contains(&candidate_identity_key(candidate))
                    || candidate_is_low_confidence_retained(candidate)
                    || citation_wrapper_link(&candidate.locator)
                {
                    return false;
                }
                let trusted_primary =
                    candidate_has_trusted_primary_source_signal(&source.query, candidate);
                if !trusted_primary {
                    return false;
                }
                let preview_ok = candidate_retention_preview_eligible(query, candidate, *score);
                let covers_uncovered_lane_facet = !uncovered_lane_facet_ids.is_empty()
                    && selected_candidate_coverage_ids(facets, candidate, min_terms)
                        .iter()
                        .any(|facet_id| uncovered_lane_facet_ids.contains(facet_id));
                preview_ok || covers_uncovered_lane_facet
            })
            .cloned()
            .collect::<Vec<_>>();
        if ranked.is_empty() {
            continue;
        }
        ranked.sort_by(|left, right| {
            let left_cov = selected_candidate_coverage_ids(facets, &left.0, min_terms)
                .iter()
                .any(|id| !covered_facet_ids.contains(id));
            let right_cov = selected_candidate_coverage_ids(facets, &right.0, min_terms)
                .iter()
                .any(|id| !covered_facet_ids.contains(id));
            right_cov
                .cmp(&left_cov)
                .then_with(|| content_rich_text(&right.0.snippet).cmp(&content_rich_text(&left.0.snippet)))
                .then_with(|| right.1.total_cmp(&left.1))
        });
        let best = ranked.remove(0);
        let best_key = candidate_identity_key(&best.0);
        if pending_keys.insert(best_key.clone()) {
            for facet_id in selected_candidate_coverage_ids(facets, &best.0, min_terms) {
                covered_facet_ids.insert(facet_id);
            }
            pending.push(best);
        }
    }

    let preserve_cap = max_evidence.min(3);
    let mut added = 0usize;
    for row in pending.into_iter().take(preserve_cap) {
        let key = candidate_identity_key(&row.0);
        if selected_keys.contains(&key) {
            continue;
        }
        if selected.len() < max_evidence {
            selected.push(row);
            selected_keys.insert(key);
            added += 1;
            continue;
        }
        let replacement = redundant_facet_backfill_replacement_index(selected, facets, min_terms)
            .or_else(|| {
                selected
                    .iter()
                    .enumerate()
                    .filter(|(_, (candidate, _))| source_trust_adjustment(candidate) < 0.15)
                    .min_by(|(_, left), (_, right)| left.1.total_cmp(&right.1))
                    .map(|(idx, _)| idx)
            });
        let Some(replace_idx) = replacement else {
            continue;
        };
        let replacement_score = selected.get(replace_idx).map(|(_, score)| *score).unwrap_or(0.0);
        let adds_new_coverage = selected_candidate_coverage_ids(facets, &row.0, min_terms)
            .iter()
            .any(|id| {
                !selected.iter().any(|(candidate, _)| {
                    selected_candidate_coverage_ids(facets, candidate, min_terms)
                        .iter()
                        .any(|existing| existing == id)
                })
            });
        if !adds_new_coverage && row.1 + 0.08 < replacement_score {
            continue;
        }
        if let Some((replaced_candidate, _)) = selected.get(replace_idx) {
            selected_keys.remove(&candidate_identity_key(replaced_candidate));
        }
        selected[replace_idx] = row;
        selected_keys.insert(key);
        added += 1;
    }

    if added > 0 {
        selected.sort_by(|left, right| right.1.total_cmp(&left.1));
    }
    added
}

fn provider_artifact_summaries(artifacts: &[Value]) -> Vec<Value> {
    artifacts
        .iter()
        .map(|artifact| {
            json!({
                "provider": artifact.get("provider").cloned().unwrap_or_else(|| json!("unknown")),
                "stage": artifact.get("stage").cloned().unwrap_or_else(|| json!("unknown")),
                "transport_ok": artifact
                    .get("provider_transport_ok")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                "result_quality": artifact
                    .get("result_quality")
                    .cloned()
                    .unwrap_or_else(|| json!("unknown")),
                "provider_raw_rows": provider_artifact_count_field(artifact, "provider_raw_count"),
                "synthesis_candidate_rows": provider_artifact_count_field(
                    artifact,
                    "synthesis_candidate_count",
                ),
                "filtered_or_rejected_rows": provider_artifact_count_field(
                    artifact,
                    "provider_filtered_count",
                ),
                "failure_reasons": artifact
                    .get("failure_reasons")
                    .cloned()
                    .unwrap_or_else(|| json!([]))
            })
        })
        .collect()
}

fn query_lane_attribution_status(
    provider_count: usize,
    provider_raw_rows: usize,
    candidate_rows: usize,
    synthesis_rows: usize,
    selected_evidence_count: usize,
    selected_covered_facet_count: usize,
    issue_count: usize,
) -> &'static str {
    if selected_evidence_count > 0 && selected_covered_facet_count > 0 {
        "selected_covered"
    } else if selected_evidence_count > 0 {
        "selected_without_requested_facet"
    } else if synthesis_rows > 0 {
        "candidates_not_selected_after_rerank"
    } else if candidate_rows > 0 {
        "candidates_filtered_or_low_confidence"
    } else if provider_raw_rows > 0 {
        "provider_raw_rows_filtered"
    } else if provider_count > 0 || issue_count > 0 {
        "provider_empty_or_failed"
    } else {
        "not_recorded"
    }
}

fn query_lane_attribution_report(
    lane_sources: &[QueryLaneSource],
    evidence_ranked: &[(Candidate, f64)],
    facets: &[ResearchFacet],
    min_terms: usize,
) -> Value {
    let mut selected_keys = HashSet::<String>::new();
    let mut selected_coverage = HashMap::<String, Vec<String>>::new();
    let mut selected_low_confidence = HashSet::<String>::new();
    for (candidate, _) in evidence_ranked {
        let key = candidate_identity_key(candidate);
        selected_keys.insert(key.clone());
        selected_coverage.insert(key.clone(), candidate_coverage_facets(facets, candidate, min_terms));
        if candidate_is_low_confidence_retained(candidate) {
            selected_low_confidence.insert(key);
        }
    }

    let requested_text_by_facet = facets
        .iter()
        .map(|facet| (facet.id.clone(), facet.requested_text.clone()))
        .collect::<HashMap<_, _>>();

    let mut selected_lane_count = 0usize;
    let mut provider_empty_or_failed_count = 0usize;
    let mut candidates_not_selected_count = 0usize;
    let mut candidates_filtered_or_low_confidence_count = 0usize;
    let mut provider_raw_rows_filtered_count = 0usize;
    let mut rows = Vec::<Value>::new();

    for source in lane_sources {
        let telemetry = retrieval_telemetry_row(
            &source.query,
            &source.phase,
            &source.candidates,
            &source.issues,
            &source.artifacts,
        );
        let provider_count = source.artifacts.len();
        let provider_raw_rows = telemetry
            .get("provider_raw_rows")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize;
        let candidate_rows = source.candidates.len();
        let synthesis_rows = source
            .candidates
            .iter()
            .filter(|candidate| !candidate_is_low_confidence_retained(candidate))
            .count();
        let issue_count = source.issues.len();
        let mut selected_evidence_count = 0usize;
        let mut selected_low_confidence_count = 0usize;
        let mut covered_facet_ids = HashSet::<String>::new();

        for candidate in &source.candidates {
            let key = candidate_identity_key(candidate);
            if !selected_keys.contains(&key) {
                continue;
            }
            selected_evidence_count += 1;
            if selected_low_confidence.contains(&key) {
                selected_low_confidence_count += 1;
            }
            if let Some(facet_ids) = selected_coverage.get(&key) {
                for facet_id in facet_ids {
                    covered_facet_ids.insert(facet_id.clone());
                }
            }
        }

        let mut covered_facet_ids = covered_facet_ids.into_iter().collect::<Vec<_>>();
        covered_facet_ids.sort();
        let covered_requested_texts = covered_facet_ids
            .iter()
            .filter_map(|facet_id| requested_text_by_facet.get(facet_id).cloned())
            .collect::<Vec<_>>();
        let status = query_lane_attribution_status(
            provider_count,
            provider_raw_rows,
            candidate_rows,
            synthesis_rows,
            selected_evidence_count,
            covered_facet_ids.len(),
            issue_count,
        );
        if selected_evidence_count > 0 {
            selected_lane_count += 1;
        }
        if status == "provider_empty_or_failed" {
            provider_empty_or_failed_count += 1;
        }
        if status == "candidates_not_selected_after_rerank" {
            candidates_not_selected_count += 1;
        }
        if status == "candidates_filtered_or_low_confidence" {
            candidates_filtered_or_low_confidence_count += 1;
        }
        if status == "provider_raw_rows_filtered" {
            provider_raw_rows_filtered_count += 1;
        }

        rows.push(json!({
            "query": source.query,
            "phase": source.phase,
            "status": status,
            "provider_count": provider_count,
            "provider_raw_rows": provider_raw_rows,
            "candidate_rows": candidate_rows,
            "synthesis_candidate_rows": synthesis_rows,
            "low_confidence_raw_rows": candidate_rows.saturating_sub(synthesis_rows),
            "filtered_or_rejected_rows": issue_count,
            "selected_evidence_count": selected_evidence_count,
            "selected_usable_evidence_count": selected_evidence_count
                .saturating_sub(selected_low_confidence_count),
            "selected_low_confidence_count": selected_low_confidence_count,
            "covered_facet_ids": covered_facet_ids,
            "covered_requested_texts": covered_requested_texts,
            "provider_results": provider_artifact_summaries(&source.artifacts),
            "failure_reasons": source
                .issues
                .iter()
                .map(|issue| clean_text(issue, 180))
                .filter(|issue| !issue.is_empty())
                .take(12)
                .collect::<Vec<_>>()
        }));
    }

    let status = if rows.is_empty() {
        "not_recorded"
    } else if selected_lane_count == 0 {
        "no_selected_evidence"
    } else if selected_lane_count < rows.len()
        || provider_empty_or_failed_count > 0
        || candidates_not_selected_count > 0
        || candidates_filtered_or_low_confidence_count > 0
        || provider_raw_rows_filtered_count > 0
    {
        "mixed"
    } else {
        "attributed"
    };

    json!({
        "version": "query_lane_attribution_v1",
        "status": status,
        "lane_count": rows.len(),
        "selected_lane_count": selected_lane_count,
        "unselected_lane_count": rows.len().saturating_sub(selected_lane_count),
        "provider_empty_or_failed_count": provider_empty_or_failed_count,
        "candidates_not_selected_after_rerank_count": candidates_not_selected_count,
        "candidates_filtered_or_low_confidence_count": candidates_filtered_or_low_confidence_count,
        "provider_raw_rows_filtered_count": provider_raw_rows_filtered_count,
        "rows": rows,
        "diagnostic_use": "telemetry_only",
        "non_goals": [
            "do_not_use_as_final_answer_text",
            "do_not_treat_query_lane_success_as_truth",
            "do_not_expose_raw_provider_payloads_to_chat"
        ]
    })
}

fn resolve_query_plan(
    policy: &Value,
    request: &Value,
    query: &str,
    budget: ApertureBudget,
) -> QueryPlanSelection {
    let mut query_metadata = batch_query_keyword_pack(request, budget);
    let explicit_metadata_supplied = !query_metadata.is_empty();
    let explicit_queries_supplied = request
        .get("queries")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .any(|row| extract_request_query_row(row, 600).is_some())
        })
        .unwrap_or(false);
    let search_query = if explicit_queries_supplied || explicit_metadata_supplied {
        subject_preserving_batch_web_search_query(query)
    } else {
        canonical_batch_web_search_query(query)
    };
    let planning_query = if search_query.is_empty() {
        clean_text(query, 600)
    } else {
        search_query
    };
    if query_metadata.is_empty() {
        if let Some(inferred) = inferred_comparison_query_pack(&planning_query, budget) {
            query_metadata = inferred;
        } else if let Some(inferred) = inferred_named_entity_query_pack(&planning_query, budget) {
            query_metadata = inferred;
        }
    }
    let explicit_queries = normalize_requested_queries(request, &planning_query, budget, &query_metadata);
    let explicit_query_pack_used = !explicit_queries.is_empty()
        && (explicit_queries_supplied || explicit_metadata_supplied)
        && (query.is_empty()
            || explicit_queries.len() > 1
            || explicit_queries
                .first()
                .map(|value| !value.eq_ignore_ascii_case(query))
                .unwrap_or(false));
    if explicit_query_pack_used {
        let rerank_query = clean_text(&planning_query, 600);
        let rerank_query = if rerank_query.is_empty() {
            explicit_queries
                .first()
                .cloned()
                .unwrap_or_else(|| planning_query.clone())
        } else {
            rerank_query
        };
        let rewrite_set = explicit_queries.iter().skip(1).cloned().collect::<Vec<_>>();
        return QueryPlanSelection {
            rewrite_applied: explicit_queries.len() > 1,
            queries: explicit_queries,
            rewrite_set,
            rerank_query,
            query_plan_source: if query_metadata.metadata_authority == "tool_inferred_from_user_query_shape" {
                "tool_inferred_query_pack_from_user_query"
            } else if query_metadata.is_empty() {
                "explicit_request_pack"
            } else {
                "explicit_request_pack_with_metadata"
            },
            query_metadata,
        };
    }
    let recovery_queries = general_research_recovery_queries(policy, &planning_query, budget);
    if !recovery_queries.is_empty() {
        if query_metadata.is_empty() {
            if let Some(inferred) = inferred_raw_query_term_pack(&planning_query, budget) {
                query_metadata = inferred;
            } else if let Some(inferred) = inferred_named_entity_query_pack(&planning_query, budget) {
                query_metadata = inferred;
            }
        }
        let recovery_queries =
            merge_recovery_queries_with_metadata(&planning_query, &recovery_queries, &query_metadata, budget);
        let rerank_query = recovery_queries
            .first()
            .cloned()
                .unwrap_or_else(|| planning_query.clone());
        let rewrite_set = recovery_queries.iter().skip(1).cloned().collect::<Vec<_>>();
        return QueryPlanSelection {
            rewrite_applied: recovery_queries.len() > 1,
            queries: recovery_queries,
            rewrite_set,
            rerank_query,
            query_plan_source: "policy_general_research_recovery",
            query_metadata,
        };
    }
    let recovery_queries = broad_current_research_recovery_queries(policy, &planning_query, budget);
    if !recovery_queries.is_empty() {
        if query_metadata.is_empty() {
            if let Some(inferred) = inferred_raw_query_term_pack(&planning_query, budget) {
                query_metadata = inferred;
            } else if let Some(inferred) = inferred_named_entity_query_pack(&planning_query, budget) {
                query_metadata = inferred;
            }
        }
        let recovery_queries =
            merge_recovery_queries_with_metadata(&planning_query, &recovery_queries, &query_metadata, budget);
        let rerank_query = recovery_queries
            .first()
            .cloned()
                .unwrap_or_else(|| planning_query.clone());
        let rewrite_set = recovery_queries.iter().skip(1).cloned().collect::<Vec<_>>();
        return QueryPlanSelection {
            rewrite_applied: recovery_queries.len() > 1,
            queries: recovery_queries,
            rewrite_set,
            rerank_query,
            query_plan_source: "policy_broad_current_research_recovery",
            query_metadata,
        };
    }
    let queries = cache_identity_query_plan(&planning_query, &explicit_queries);
    if query_metadata.is_empty() {
        if let Some(inferred) = inferred_raw_query_term_pack(&planning_query, budget) {
            query_metadata = inferred;
        } else if let Some(inferred) = inferred_named_entity_query_pack(&planning_query, budget) {
            query_metadata = inferred;
        }
    }
    let rerank_query = queries
        .first()
        .cloned()
        .unwrap_or_else(|| planning_query.clone());
    QueryPlanSelection {
        queries,
        rewrite_set: Vec::new(),
        rewrite_applied: false,
        rerank_query,
        query_plan_source: "agent_submitted_single_query",
        query_metadata,
    }
}
