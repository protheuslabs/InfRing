{
    gates.extend([
        web_gate(
            "web_1_request_shape_present",
            request.is_some() || tool_attempted,
            request_shape_present,
            if request_shape_present {
                "web request carries a query, query pack, URL, locator, or equivalent executed request shape"
            } else {
                "no query, query pack, URL, locator, or equivalent request shape was visible"
            },
            request_shape_refs(request_input),
        ),
        web_gate(
            "web_2_query_metadata_present",
            request_shape_present,
            query_metadata_present,
            if query_metadata_present {
                "request includes query metadata, expansion/narrowing marker, keywords, or required coverage"
            } else if request_shape_present {
                "request used a minimal query shape without metadata, keywords, expansion marker, or required coverage"
            } else {
                "query metadata cannot be inspected without a visible request shape"
            },
            metadata_refs(query_metadata_diagnostics),
        ),
        web_gate(
            "web_3_tool_attempt_recorded",
            request_shape_present || tool_attempted,
            tool_attempted,
            if tool_attempted {
                "web tool attempt is recorded"
            } else {
                "request shape exists but no web tool attempt is recorded"
            },
            vec![
                "tools".to_string(),
                "response_finalization.tool_completion.tool_attempts".to_string(),
            ],
        ),
        web_gate(
            "web_3b1_provider_quota_not_rate_limited",
            tool_attempted,
            !rate_limited_hard,
            if rate_limited_hard {
                "provider or retrieval lane reported rate-limit, quota, Retry-After, throttling, or HTTP 429 signals"
            } else if rate_limited {
                "rate-limit or quota signal appeared on at least one lane, but retrieval still produced candidates and evidence"
            } else if tool_attempted {
                "no provider rate-limit, quota, Retry-After, throttling, or HTTP 429 signal was detected"
            } else {
                "rate-limit signals cannot be inspected before a tool attempt"
            },
            access_blocker_refs(&access_blocker),
        ),
        web_gate(
            "web_3b2_no_bot_challenge_or_waf",
            tool_attempted,
            !anti_bot_challenge_hard,
            if anti_bot_challenge_hard {
                "tool artifacts contained CAPTCHA, human-verification, Cloudflare, WAF, or bot-wall challenge signals"
            } else if anti_bot_challenge {
                "tool artifacts contained challenge signals on at least one lane, but retrieval still produced usable evidence"
            } else if tool_attempted {
                "no CAPTCHA, human-verification, Cloudflare, WAF, or bot-wall challenge signal was detected"
            } else {
                "bot-challenge signals cannot be inspected before a tool attempt"
            },
            access_blocker_refs(&access_blocker),
        ),
        web_gate(
            "web_3b3_no_permission_or_auth_block",
            tool_attempted,
            !permission_or_auth,
            if permission_or_auth {
                "tool artifacts contained login, auth-required, unauthorized, or HTTP 401 signals"
            } else if tool_attempted {
                "no login, auth-required, unauthorized, or HTTP 401 signal was detected"
            } else {
                "auth/permission signals cannot be inspected before a tool attempt"
            },
            access_blocker_refs(&access_blocker),
        ),
        web_gate(
            "web_3b4_no_access_denied_or_forbidden",
            tool_attempted,
            !access_denied,
            if access_denied {
                "tool artifacts contained access-denied, forbidden, request-blocked, or HTTP 403 signals"
            } else if tool_attempted {
                "no access-denied, forbidden, request-blocked, or HTTP 403 signal was detected"
            } else {
                "access-denied signals cannot be inspected before a tool attempt"
            },
            access_blocker_refs(&access_blocker),
        ),
        web_gate(
            "web_3b5_provider_configuration_available",
            tool_attempted,
            !provider_config_missing_hard,
            if provider_config_missing_hard {
                "tool artifacts indicate provider credentials, provider admission, or required provider configuration is missing"
            } else if provider_config_missing {
                "provider configuration gap was detected on at least one lane, but configured provider supply continued far enough to produce candidates and evidence"
            } else if tool_attempted {
                "no missing provider credential, admission, or configuration signal was detected"
            } else {
                "provider configuration signals cannot be inspected before a tool attempt"
            },
            access_blocker_refs(&access_blocker),
        ),
        web_gate(
            "web_3b_access_not_blocked_or_throttled",
            tool_attempted,
            !access_blocked_or_throttled_hard,
            if access_blocked_or_throttled_hard {
                "tool attempt appears blocked or throttled by an access, rate-limit, CAPTCHA, bot-wall, or similar web-control signal"
            } else if access_blocked_or_throttled {
                "an access blocker signal appeared on at least one lane, but retrieval still produced usable evidence"
            } else if tool_attempted {
                "no access-block, CAPTCHA, bot-wall, or rate-limit signal was detected in the tool artifacts"
            } else {
                "access blockers cannot be inspected before a tool attempt"
            },
            access_blocker_refs(&access_blocker),
        ),
        web_gate(
            "web_3c_blocker_recovery_lane_visible",
            access_blocked_or_throttled_hard,
            !access_blocked_or_throttled_hard || blocker_recovery_lane_visible,
            if !access_blocked_or_throttled_hard {
                "no access blocker was detected, so a browser-materialization recovery lane is not required"
            } else if blocker_recovery_lane_visible {
                "access blocker was detected and the payload exposes browser-materialization recovery capability, recommendation, or attempt metadata"
            } else {
                "access blocker was detected but no browser-materialization recovery lane metadata was visible"
            },
            browser_materialization_recovery
                .get("artifact_refs")
                .and_then(Value::as_array)
                .map(|rows| {
                    rows.iter()
                        .filter_map(Value::as_str)
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                })
                .filter(|rows| !rows.is_empty())
                .unwrap_or_else(|| {
                    vec![
                        "tool_result_quality.browser_materialization".to_string(),
                        "retrieval_broker.provider_attempts".to_string(),
                        "runtime_web_tools_metadata.browser_materialization".to_string(),
                    ]
                }),
        ),
    ]);
}
