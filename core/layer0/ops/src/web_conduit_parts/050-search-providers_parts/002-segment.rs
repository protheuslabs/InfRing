fn api_search_bing_rss(
    query: &str,
    summary_only: bool,
    allowed_domains: &[String],
    exclude_subdomains: bool,
    top_k: usize,
    timeout_ms: u64,
) -> Value {
    let requested_url = web_search_bing_rss_url(query);
    let max_response_bytes = 280_000usize;
    let retry_attempts = 2usize;
    let fetched = fetch_with_curl_retry(
        &requested_url,
        timeout_ms,
        max_response_bytes,
        retry_attempts,
        false,
    );
    let status_code = fetched
        .get("status_code")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let content_type = clean_text(
        fetched
            .get("content_type")
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );
    let parsed = render_bing_rss_payload(
        fetched.get("body").and_then(Value::as_str).unwrap_or(""),
        allowed_domains,
        exclude_subdomains,
        top_k,
        max_response_bytes,
    );
    let content = clean_text(
        parsed.get("content").and_then(Value::as_str).unwrap_or(""),
        max_response_bytes,
    );
    let summary = clean_text(
        parsed.get("summary").and_then(Value::as_str).unwrap_or(""),
        900,
    );
    let fetch_ok = fetched.get("ok").and_then(Value::as_bool).unwrap_or(false)
        && parsed.get("ok").and_then(Value::as_bool).unwrap_or(false)
        && !summary.is_empty();
    let mut error_value = clean_text(
        fetched.get("stderr").and_then(Value::as_str).unwrap_or(""),
        320,
    );
    if error_value.is_empty() {
        error_value = clean_text(
            parsed.get("error").and_then(Value::as_str).unwrap_or(""),
            220,
        );
    }
    json!({
        "ok": fetch_ok,
        "requested_url": requested_url,
        "status_code": status_code,
        "content_type": if content_type.is_empty() { Value::String("application/rss+xml".to_string()) } else { Value::String(content_type) },
        "summary": summary,
        "content": if summary_only { Value::String(String::new()) } else { Value::String(content) },
        "links": parsed.get("links").cloned().unwrap_or_else(|| json!([])),
        "results": parsed.get("results").cloned().unwrap_or_else(|| json!([])),
        "content_domains": parsed.get("content_domains").cloned().unwrap_or_else(|| json!([])),
        "provider_raw_count": parsed.get("provider_raw_count").cloned().unwrap_or_else(|| json!(0)),
        "provider_filtered_count": parsed.get("provider_filtered_count").cloned().unwrap_or_else(|| json!(0)),
        "retry_attempts": fetched.get("retry_attempts").cloned().unwrap_or_else(|| json!(1)),
        "retry_used": fetched.get("retry_used").cloned().unwrap_or_else(|| json!(false)),
        "user_agent": fetched.get("user_agent").cloned().unwrap_or_else(|| json!(DEFAULT_WEB_USER_AGENTS[0])),
        "provider": "bing_rss",
        "error": if fetch_ok {
            Value::Null
        } else if error_value.is_empty() {
            Value::String("bing_rss_search_failed".to_string())
        } else {
            Value::String(error_value)
        }
    })
}

fn api_search_google_news_rss(
    query: &str,
    summary_only: bool,
    allowed_domains: &[String],
    exclude_subdomains: bool,
    top_k: usize,
    timeout_ms: u64,
) -> Value {
    let requested_url = web_search_google_news_rss_url(query);
    let max_response_bytes = 320_000usize;
    let retry_attempts = 2usize;
    let fetched = fetch_with_curl_retry(
        &requested_url,
        timeout_ms,
        max_response_bytes,
        retry_attempts,
        false,
    );
    let status_code = fetched
        .get("status_code")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let content_type = clean_text(
        fetched
            .get("content_type")
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );
    let parsed = render_google_news_rss_payload(
        fetched.get("body").and_then(Value::as_str).unwrap_or(""),
        allowed_domains,
        exclude_subdomains,
        top_k,
        max_response_bytes,
    );
    let content = clean_text(
        parsed.get("content").and_then(Value::as_str).unwrap_or(""),
        max_response_bytes,
    );
    let summary = clean_text(
        parsed.get("summary").and_then(Value::as_str).unwrap_or(""),
        900,
    );
    let fetch_ok = fetched.get("ok").and_then(Value::as_bool).unwrap_or(false)
        && parsed.get("ok").and_then(Value::as_bool).unwrap_or(false)
        && !summary.is_empty();
    let mut error_value = clean_text(
        fetched.get("stderr").and_then(Value::as_str).unwrap_or(""),
        320,
    );
    if error_value.is_empty() {
        error_value = clean_text(
            parsed.get("error").and_then(Value::as_str).unwrap_or(""),
            220,
        );
    }
    json!({
        "ok": fetch_ok,
        "requested_url": requested_url,
        "status_code": status_code,
        "content_type": if content_type.is_empty() { Value::String("application/rss+xml".to_string()) } else { Value::String(content_type) },
        "summary": summary,
        "content": if summary_only { Value::String(String::new()) } else { Value::String(content) },
        "links": parsed.get("links").cloned().unwrap_or_else(|| json!([])),
        "results": parsed.get("results").cloned().unwrap_or_else(|| json!([])),
        "content_domains": parsed.get("content_domains").cloned().unwrap_or_else(|| json!([])),
        "provider_raw_count": parsed.get("provider_raw_count").cloned().unwrap_or_else(|| json!(0)),
        "provider_filtered_count": parsed.get("provider_filtered_count").cloned().unwrap_or_else(|| json!(0)),
        "retry_attempts": fetched.get("retry_attempts").cloned().unwrap_or_else(|| json!(1)),
        "retry_used": fetched.get("retry_used").cloned().unwrap_or_else(|| json!(false)),
        "user_agent": fetched.get("user_agent").cloned().unwrap_or_else(|| json!(DEFAULT_WEB_USER_AGENTS[0])),
        "provider": "google_news_rss",
        "error": if fetch_ok {
            Value::Null
        } else if error_value.is_empty() {
            Value::String("google_news_rss_search_failed".to_string())
        } else {
            Value::String(error_value)
        }
    })
}

fn search_payload_usable(payload: &Value) -> bool {
    if !payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        return false;
    }
    if payload_looks_like_search_challenge(payload)
        || payload_looks_low_signal_search(payload)
        || search_payload_looks_competitive_programming_dump(payload)
    {
        return false;
    }
    let summary = clean_text(
        payload.get("summary").and_then(Value::as_str).unwrap_or(""),
        1_200,
    );
    if summary.is_empty() {
        return false;
    }
    !search_summary_has_low_signal_marker(&summary)
}
fn search_payload_looks_competitive_programming_dump(payload: &Value) -> bool {
    let summary = clean_text(
        payload.get("summary").and_then(Value::as_str).unwrap_or(""),
        2_400,
    );
    let content = clean_text(
        payload.get("content").and_then(Value::as_str).unwrap_or(""),
        3_200,
    );
    let combined = format!("{summary}\n{content}").to_ascii_lowercase();
    if combined.trim().is_empty() {
        return false;
    }
    let marker_hits = [
        "given a tree",
        "input specification",
        "output specification",
        "sample input",
        "sample output",
        "#include <stdio.h>",
        "int main()",
        "public class",
        "translate the following java code",
        "csdn.net",
        "acm",
    ]
    .iter()
    .filter(|marker| combined.contains(**marker))
    .count();
    marker_hits >= 3
}
fn search_query_is_meta_diagnostic(query: &str) -> bool {
    let lowered = clean_text(query, 600).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let explicit_search_intent = ["search for ", "search the web", "web search", "find information", "finding information", "look up", "compare ", "official docs", "research online", "research on web"]
        .iter()
        .any(|marker| lowered.contains(*marker));
    if explicit_search_intent {
        return false;
    }
    if [
        "that was just a test",
        "that was a test",
        "did you do the web request",
        "did you try it",
        "where did that come from",
        "where the hell did that come from",
        "you hallucinated",
        "you returned no result",
        "answer the question",
    ]
    .iter()
    .any(|marker| lowered.contains(*marker))
    {
        return true;
    }
    if lowered.contains("did you do the web request")
        || lowered.contains("did you try it")
        || lowered.contains("why did my last prompt")
        || lowered.contains("you returned no result")
        || lowered.contains("that was just a test")
        || lowered.contains("that was a test")
        || lowered.contains("where did that come from")
    {
        return true;
    }
    let signal_terms = lowered
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| token.len() >= 3)
        .count();
    let meta_hits = ["what happened", "workflow", "tool call", "web tooling", "hallucination", "hallucinated", "training data", "context issue", "answer the question", "last response", "previous response"]
    .iter()
    .filter(|marker| lowered.contains(**marker))
    .count();
    if meta_hits == 0 {
        return false;
    }
    let web_intent_hits = ["site:", "http://", "https://", "latest ", "top ", "best ", "news", "framework", "docs", "recipe", "weather", "price"]
    .iter()
    .filter(|marker| lowered.contains(**marker))
    .count();
    if web_intent_hits > 0 {
        return false;
    }
    let research_intent_hits = [
        "technique",
        "techniques",
        "mitigation",
        "how to",
        "best practice",
        "best practices",
        "guide",
        "tutorial",
        "methods",
        "strategy",
    ]
    .iter()
    .filter(|marker| lowered.contains(**marker))
    .count();
    if meta_hits == 1 && research_intent_hits > 0 {
        return false;
    }
    meta_hits >= 2 || signal_terms <= 7
}
fn search_override_flag_enabled(value: &Value) -> bool {
    runtime_web_truthy_flag(value)
}
fn search_meta_query_override(request: &Value) -> bool {
    let pointers = [
        "/allow_meta_query_search",
        "/allowMetaQuerySearch",
        "/force_web_search",
        "/forceWebSearch",
        "/force_web_lookup",
        "/forceWebLookup",
        "/search_policy/allow_meta_query_search",
        "/searchPolicy/allowMetaQuerySearch",
        "/search_policy/force_web_search",
        "/searchPolicy/forceWebSearch",
        "/search_policy/force_web_lookup",
        "/searchPolicy/forceWebLookup",
    ];
    runtime_web_request_flag(request, &pointers)
}
