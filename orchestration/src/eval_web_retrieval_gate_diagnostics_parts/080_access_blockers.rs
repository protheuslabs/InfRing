fn web_access_blocker_diagnostics(payload: &Value, retrieval_quality: &Value) -> Value {
    let mut signals = Vec::<String>::new();
    let mut refs = Vec::<String>::new();
    scan_access_blocker_signals(payload, "payload", &mut signals, &mut refs);
    scan_access_blocker_signals(
        retrieval_quality,
        "retrieval_quality",
        &mut signals,
        &mut refs,
    );
    signals.sort_unstable();
    signals.dedup();
    refs.sort_unstable();
    refs.dedup();

    let has_throttle = signals.iter().any(|signal| {
        matches!(
            signal.as_str(),
            "http_status_429"
                | "too_many_requests"
                | "rate_limit"
                | "retry_after"
                | "quota_exceeded"
                | "throttled"
        )
    });
    let has_bot_challenge = signals.iter().any(|signal| {
        matches!(
            signal.as_str(),
            "captcha_challenge"
                | "cloudflare_challenge"
                | "bot_detection"
                | "human_verification"
                | "waf_or_bot_wall"
        )
    });
    let has_auth = signals.iter().any(|signal| {
        matches!(
            signal.as_str(),
            "http_status_401" | "auth_required" | "login_required"
        )
    });
    let has_access_block = signals.iter().any(|signal| {
        matches!(
            signal.as_str(),
            "http_status_403" | "access_denied" | "request_blocked"
        )
    });
    let has_provider_config_missing = signals.iter().any(|signal| {
        matches!(
            signal.as_str(),
            "missing_api_key"
                | "invalid_api_key"
                | "missing_provider_credentials"
                | "provider_not_configured"
                | "strong_provider_missing"
        )
    });

    let kind = if has_provider_config_missing {
        "provider_configuration_missing"
    } else if has_throttle && has_bot_challenge {
        "anti_bot_or_throttle"
    } else if has_throttle {
        "throttle_or_rate_limit"
    } else if has_bot_challenge {
        "anti_bot_challenge"
    } else if has_access_block && !has_auth {
        "access_blocked"
    } else if has_auth {
        "permission_or_auth"
    } else {
        "none"
    };
    json!({
        "detected": kind != "none",
        "kind": kind,
        "classes": {
            "rate_limit_or_quota": has_throttle,
            "anti_bot_challenge": has_bot_challenge,
            "permission_or_auth": has_auth,
            "access_denied_or_forbidden": has_access_block,
            "provider_configuration_missing": has_provider_config_missing
        },
        "signals": signals,
        "artifact_refs": refs,
        "note": "General web-access blocker detection based on status/error/body markers such as 429, 403, CAPTCHA, bot-wall, WAF, Cloudflare challenge, rate limit, Retry-After, auth-required, or provider-configuration signals."
    })
}

fn scan_access_blocker_signals(
    value: &Value,
    path: &str,
    signals: &mut Vec<String>,
    refs: &mut Vec<String>,
) {
    if access_blocker_declarative_path(path) {
        return;
    }
    match value {
        Value::Null | Value::Bool(_) => {}
        Value::Number(raw) => {
            if let Some(code) = raw.as_u64().filter(|_| access_status_path(path)) {
                push_status_signal(code, path, signals, refs);
            }
        }
        Value::String(raw) => scan_access_blocker_text(raw, path, signals, refs),
        Value::Array(rows) => {
            for (index, row) in rows.iter().enumerate() {
                scan_access_blocker_signals(row, &format!("{path}.{index}"), signals, refs);
            }
        }
        Value::Object(map) => {
            for (key, child) in map {
                scan_access_blocker_signals(child, &format!("{path}.{key}"), signals, refs);
            }
        }
    }
}

fn access_blocker_declarative_path(path: &str) -> bool {
    let normalized = normalize_for_compare(&path.replace(['.', '_', '-'], " "));
    [
        "blocker taxonomy",
        "browser materialization profile compilation",
        "browser materialization readiness lifecycle",
        "browser materialization url safety",
        "browser materialization non goals",
        "source pattern",
        "tool cd",
        "tooling cd",
        "capability contract",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn scan_access_blocker_text(
    raw: &str,
    path: &str,
    signals: &mut Vec<String>,
    refs: &mut Vec<String>,
) {
    let normalized = normalize_for_compare(raw);
    let explicit_challenge_markers = [
        ("captcha", "captcha_challenge"),
        ("recaptcha", "captcha_challenge"),
        ("hcaptcha", "captcha_challenge"),
        ("cf-chl", "cloudflare_challenge"),
        ("cf-ray", "cloudflare_challenge"),
        ("checking your browser", "cloudflare_challenge"),
        ("verify you are human", "human_verification"),
        ("human verification", "human_verification"),
        (
            "please complete the following challenge",
            "human_verification",
        ),
        (
            "unfortunately bots use duckduckgo too",
            "human_verification",
        ),
        ("select all squares containing a duck", "human_verification"),
        ("unusual traffic", "bot_detection"),
        ("automated queries", "bot_detection"),
    ];
    let mut explicit_challenge_detected = false;
    for (needle, signal) in explicit_challenge_markers {
        if normalized.contains(needle) {
            explicit_challenge_detected = true;
            push_access_signal(signal, path, signals, refs);
        }
    }

    let contextual_bot_markers = [
        ("cloudflare", "cloudflare_challenge"),
        ("bot detection", "bot_detection"),
        ("anti-bot", "bot_detection"),
        ("anti bot", "bot_detection"),
        ("bot wall", "waf_or_bot_wall"),
        ("waf", "waf_or_bot_wall"),
        ("datadome", "waf_or_bot_wall"),
        ("perimeterx", "waf_or_bot_wall"),
        ("imperva", "waf_or_bot_wall"),
        ("incapsula", "waf_or_bot_wall"),
        ("distil networks", "waf_or_bot_wall"),
        ("ddos-guard", "waf_or_bot_wall"),
    ];
    if access_status_path(path) || explicit_challenge_detected {
        for (needle, signal) in contextual_bot_markers {
            if normalized.contains(needle) {
                push_access_signal(signal, path, signals, refs);
            }
        }
    }

    if !access_status_path(path) {
        return;
    }

    let status_markers = [
        ("429", "http_status_429"),
        ("too many requests", "too_many_requests"),
        ("rate limit", "rate_limit"),
        ("rate-limit", "rate_limit"),
        ("rate_limited", "rate_limit"),
        ("ratelimit", "rate_limit"),
        ("retry-after", "retry_after"),
        ("quota exceeded", "quota_exceeded"),
        ("throttled", "throttled"),
        ("throttle", "throttled"),
        ("missing api key", "missing_api_key"),
        ("api key missing", "missing_api_key"),
        ("invalid api key", "invalid_api_key"),
        (
            "missing provider credentials",
            "missing_provider_credentials",
        ),
        (
            "provider credentials missing",
            "missing_provider_credentials",
        ),
        ("provider not configured", "provider_not_configured"),
        ("strong search provider missing", "strong_provider_missing"),
        ("strong_provider_missing", "strong_provider_missing"),
        ("403", "http_status_403"),
        ("forbidden", "access_denied"),
        ("access denied", "access_denied"),
        ("request blocked", "request_blocked"),
        ("blocked by", "request_blocked"),
        ("401", "http_status_401"),
        ("unauthorized", "auth_required"),
        ("authentication required", "auth_required"),
        ("login required", "login_required"),
        ("sign in required", "login_required"),
    ];
    for (needle, signal) in status_markers {
        if normalized.contains(needle) {
            push_access_signal(signal, path, signals, refs);
        }
    }
}

fn access_status_path(path: &str) -> bool {
    let normalized = normalize_for_compare(&path.replace(['.', '_', '-'], " "));
    [
        "status",
        "status code",
        "http status",
        "http code",
        "error",
        "failure",
        "exception",
        "headers",
        "header",
        "retry after",
        "retry_after",
        "rate limit",
        "rate_limit",
        "access blocker",
        "blocker",
        "blocked",
        "provider config",
        "provider configured",
        "provider not configured",
        "api key",
        "credentials",
        "credential",
        "strong provider",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn push_status_signal(code: u64, path: &str, signals: &mut Vec<String>, refs: &mut Vec<String>) {
    match code {
        401 => push_access_signal("http_status_401", path, signals, refs),
        403 => push_access_signal("http_status_403", path, signals, refs),
        429 => push_access_signal("http_status_429", path, signals, refs),
        503 => push_access_signal("waf_or_bot_wall", path, signals, refs),
        _ => {}
    }
}

fn push_access_signal(signal: &str, path: &str, signals: &mut Vec<String>, refs: &mut Vec<String>) {
    signals.push(signal.to_string());
    refs.push(path.to_string());
}
