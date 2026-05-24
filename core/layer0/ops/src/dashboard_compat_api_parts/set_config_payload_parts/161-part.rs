fn leading_quote_pair(raw: &str) -> Option<(char, char)> {
    let first = raw.chars().next()?;
    match first {
        '"' => Some(('"', '"')),
        '\'' => Some(('\'', '\'')),
        '`' => Some(('`', '`')),
        '“' => Some(('“', '”')),
        _ => None,
    }
}

fn trailing_web_query_instruction_tail(raw: &str) -> bool {
    let lowered = clean_text(raw, 240)
        .trim()
        .trim_matches(|ch| matches!(ch, '"' | '\'' | '`' | '“' | '”'))
        .trim()
        .to_ascii_lowercase();
    if lowered.is_empty() {
        return true;
    }
    [
        "and return the results",
        "and return results",
        "and return the result",
        "and return the answer",
        "and return the findings",
        "and give me the results",
        "and give the results",
        "and show me the results",
        "and tell me the results",
        "and tell me what you find",
        "and tell me what you found",
        "and summarize the results",
        "and summarize the answer",
        "and summarize",
    ]
    .iter()
    .any(|suffix| lowered.starts_with(suffix))
}

fn extract_leading_quoted_natural_web_query(text: &str, max_chars: usize) -> Option<String> {
    let trimmed = clean_text(text, max_chars);
    let trimmed = trimmed.trim();
    let (_, close) = leading_quote_pair(trimmed)?;
    let rest = &trimmed[trimmed.chars().next()?.len_utf8()..];
    let end_rel = rest.find(close)?;
    let inside = clean_text(&rest[..end_rel], max_chars);
    if inside.is_empty() {
        return None;
    }
    if trailing_web_query_instruction_tail(&rest[end_rel + close.len_utf8()..]) {
        return Some(inside);
    }
    None
}

fn strip_wrapped_natural_web_query(text: &str, max_chars: usize) -> String {
    let mut cleaned = clean_text(text, max_chars);
    if cleaned.is_empty() {
        return cleaned;
    }
    if let Some(quoted) = extract_leading_quoted_natural_web_query(&cleaned, max_chars) {
        cleaned = quoted;
    }
    cleaned = cleaned
        .trim()
        .trim_matches(|ch| matches!(ch, '"' | '\'' | '`' | '“' | '”'))
        .trim()
        .to_string();
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
            cleaned = cleaned.trim().to_string();
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

fn normalize_inline_tool_execution_input(
    normalized_name: &str,
    input: &Value,
    user_message: &str,
) -> Value {
    let mut normalized_input = input.clone();
    if normalized_name == "workspace_analyze" {
        let lowered = clean_text(user_message, 600).to_ascii_lowercase();
        let hydrated_query = clean_text(
            normalized_input
                .get("query")
                .and_then(Value::as_str)
                .unwrap_or(""),
            800,
        );
        let fallback_query = if hydrated_query.is_empty() {
            workspace_analyze_intent_from_message(user_message, &lowered)
                .and_then(|(_, payload)| {
                    payload
                        .get("query")
                        .and_then(Value::as_str)
                        .map(|value| clean_text(value, 800))
                })
                .unwrap_or_default()
        } else {
            hydrated_query
        };
        if !fallback_query.is_empty() {
            if !normalized_input.is_object() {
                normalized_input = json!({});
            }
            normalized_input["query"] = json!(fallback_query);
            if normalized_input
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or("")
                .is_empty()
            {
                normalized_input["path"] = json!(".");
            }
            if normalized_input.get("full").is_none() {
                normalized_input["full"] = json!(true);
            }
        }
    }
    if matches!(
        normalized_name,
        "batch_query" | "batch-query" | "web_search" | "search_web" | "search" | "web_query"
    ) {
        let raw_query = clean_text(
            normalized_input
                .get("query")
                .or_else(|| normalized_input.get("q"))
                .and_then(Value::as_str)
                .unwrap_or(user_message),
            600,
        );
        let cleaned_query = natural_web_search_query_from_message(&raw_query)
            .unwrap_or_else(|| strip_wrapped_natural_web_query(&raw_query, 600));
        if !cleaned_query.is_empty() {
            if !normalized_input.is_object() {
                normalized_input = json!({});
            }
            normalized_input["query"] = json!(cleaned_query);
            if normalized_input
                .get("source")
                .and_then(Value::as_str)
                .unwrap_or("")
                .is_empty()
            {
                normalized_input["source"] = json!("web");
            }
            if normalized_input
                .get("aperture")
                .and_then(Value::as_str)
                .unwrap_or("")
                .is_empty()
            {
                normalized_input["aperture"] = json!("medium");
            }
        }
    }
    normalized_input
}

fn tool_result_hidden_artifact_value(payload: &Value, key: &str) -> Option<Value> {
    let value = payload
        .get(key)
        .or_else(|| payload.pointer(&format!("/tool_pipeline/raw_payload/{key}")));
    match key {
        "search_results" | "provider_results" => value
            .and_then(|value| {
                value.as_array().and_then(|rows| {
                    let projected = rows
                        .iter()
                        .filter_map(project_hidden_tool_result_row)
                        .take(6)
                        .collect::<Vec<_>>();
                    (!projected.is_empty()).then(|| Value::Array(projected))
                })
            })
            .or_else(|| derive_hidden_tool_result_artifact(payload, key)),
        "evidence_refs" | "evidence_pack" | "evidence_pack_candidates" => value
            .and_then(|value| {
                value.as_array().and_then(|rows| {
                    let projected = rows.iter().take(6).cloned().collect::<Vec<_>>();
                    (!projected.is_empty()).then(|| Value::Array(projected))
                })
            })
            .or_else(|| derive_hidden_tool_result_artifact(payload, key)),
        "tool_result_quality" | "evidence_pack_quality" => {
            value.and_then(|value| value.is_object().then(|| value.clone()))
        }
        _ => None,
    }
}

fn derive_hidden_tool_result_artifact(payload: &Value, key: &str) -> Option<Value> {
    match key {
        "search_results" => derive_hidden_search_results_from_web_payload(payload),
        "provider_results" => derive_hidden_provider_results_from_web_payload(payload),
        "evidence_refs" => derive_hidden_evidence_refs_from_web_payload(payload),
        _ => None,
    }
}

fn derive_hidden_search_results_from_web_payload(payload: &Value) -> Option<Value> {
    let links = payload
        .get("links")
        .or_else(|| payload.pointer("/tool_pipeline/raw_payload/links"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let snippet = trim_text(
        payload
            .get("content")
            .or_else(|| payload.pointer("/tool_pipeline/raw_payload/content"))
            .and_then(Value::as_str)
            .filter(|raw| !raw.trim().is_empty())
            .or_else(|| {
                payload
                    .get("summary")
                    .or_else(|| payload.pointer("/tool_pipeline/raw_payload/summary"))
                    .and_then(Value::as_str)
            })
            .unwrap_or(""),
        1_200,
    );
    let provider = trim_text(
        payload
            .get("provider")
            .or_else(|| payload.pointer("/tool_pipeline/raw_payload/provider"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        120,
    );
    let projected = links
        .iter()
        .filter_map(Value::as_str)
        .map(|raw| trim_text(raw.trim(), 2_200))
        .filter(|raw| !raw.is_empty())
        .take(6)
        .map(|locator| {
            let mut row = serde_json::Map::new();
            row.insert("locator".to_string(), Value::String(locator));
            if !snippet.is_empty() {
                row.insert("snippet".to_string(), Value::String(snippet.clone()));
            }
            if !provider.is_empty() {
                row.insert("provider".to_string(), Value::String(provider.clone()));
            }
            Value::Object(row)
        })
        .collect::<Vec<_>>();
    if !projected.is_empty() {
        return Some(Value::Array(projected));
    }
    for pointer in [
        "/tool_result_quality/candidate_quality",
        "/tool_pipeline/raw_payload/tool_result_quality/candidate_quality",
        "/evidence_refs",
        "/tool_pipeline/raw_payload/evidence_refs",
    ] {
        let Some(rows) = payload.pointer(pointer).and_then(Value::as_array) else {
            continue;
        };
        let projected = rows
            .iter()
            .filter_map(project_hidden_tool_result_row)
            .take(6)
            .collect::<Vec<_>>();
        if !projected.is_empty() {
            return Some(Value::Array(projected));
        }
    }
    None
}

fn derive_hidden_provider_results_from_web_payload(payload: &Value) -> Option<Value> {
    let provider = trim_text(
        payload
            .get("provider")
            .or_else(|| payload.get("source"))
            .or_else(|| payload.pointer("/input/source"))
            .or_else(|| payload.pointer("/tool_pipeline/input/source"))
            .or_else(|| payload.pointer("/tool_pipeline/raw_payload/provider"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        120,
    );
    let summary = trim_text(
        payload
            .get("summary")
            .or_else(|| payload.pointer("/tool_pipeline/raw_payload/summary"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        1_200,
    );
    let error = trim_text(
        payload
            .get("error")
            .or_else(|| payload.get("transport_error"))
            .or_else(|| payload.pointer("/tool_pipeline/raw_payload/transport_error"))
            .or_else(|| payload.pointer("/tool_pipeline/raw_payload/error"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        240,
    );
    let status = trim_text(
        payload
            .get("status")
            .or_else(|| payload.pointer("/tool_pipeline/raw_payload/status"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        80,
    );
    let query = trim_text(
        payload
            .get("query")
            .or_else(|| payload.pointer("/input/query"))
            .or_else(|| payload.pointer("/tool_pipeline/input/query"))
            .or_else(|| payload.pointer("/tool_pipeline/raw_payload/query"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        600,
    );
    let links = payload
        .get("links")
        .or_else(|| payload.pointer("/tool_pipeline/raw_payload/links"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|value| value.as_str().map(|raw| trim_text(raw.trim(), 2_200)))
        .filter(|raw| !raw.is_empty())
        .take(4)
        .map(Value::String)
        .collect::<Vec<_>>();
    let raw_count = payload
        .get("provider_raw_count")
        .or_else(|| payload.pointer("/tool_pipeline/raw_payload/provider_raw_count"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let filtered_count = payload
        .get("provider_filtered_count")
        .or_else(|| payload.pointer("/tool_pipeline/raw_payload/provider_filtered_count"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let failure_detail = payload
        .get("partial_failure_details")
        .or_else(|| payload.pointer("/tool_pipeline/raw_payload/partial_failure_details"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(|raw| trim_text(raw.trim(), 240))
        .find(|raw| !raw.is_empty());
    if provider.is_empty()
        && summary.is_empty()
        && error.is_empty()
        && status.is_empty()
        && query.is_empty()
        && links.is_empty()
        && raw_count == 0
        && filtered_count == 0
        && failure_detail.is_none()
    {
        return None;
    }
    let mut row = serde_json::Map::new();
    if !provider.is_empty() {
        row.insert("provider".to_string(), Value::String(provider));
    }
    if !summary.is_empty() {
        row.insert("summary".to_string(), Value::String(summary));
    }
    if !error.is_empty() {
        row.insert("error".to_string(), Value::String(error));
    }
    if !status.is_empty() {
        row.insert("status".to_string(), Value::String(status));
    }
    if let Some(detail) = failure_detail {
        row.insert("failure_detail".to_string(), Value::String(detail));
    }
    if !query.is_empty() {
        row.insert("query".to_string(), Value::String(query));
    }
    if !links.is_empty() {
        row.insert("links".to_string(), Value::Array(links));
    }
    if raw_count > 0 {
        row.insert("provider_raw_count".to_string(), json!(raw_count));
    }
    if filtered_count > 0 {
        row.insert("provider_filtered_count".to_string(), json!(filtered_count));
    }
    (!row.is_empty()).then(|| Value::Array(vec![Value::Object(row)]))
}

fn derive_hidden_evidence_refs_from_web_payload(payload: &Value) -> Option<Value> {
    for pointer in [
        "/evidence_pack_candidates",
        "/tool_pipeline/raw_payload/evidence_pack_candidates",
        "/evidence_pack",
        "/tool_pipeline/raw_payload/evidence_pack",
        "/tool_result_quality/candidate_quality",
        "/tool_pipeline/raw_payload/tool_result_quality/candidate_quality",
        "/search_results",
        "/tool_pipeline/raw_payload/search_results",
        "/provider_results",
        "/tool_pipeline/raw_payload/provider_results",
    ] {
        let Some(rows) = payload.pointer(pointer).and_then(Value::as_array) else {
            continue;
        };
        let projected = rows
            .iter()
            .filter_map(project_hidden_tool_result_row)
            .take(6)
            .collect::<Vec<_>>();
        if !projected.is_empty() {
            return Some(Value::Array(projected));
        }
    }
    for derived in [
        derive_hidden_search_results_from_web_payload(payload),
        derive_hidden_provider_results_from_web_payload(payload),
    ] {
        let Some(Value::Array(rows)) = derived else {
            continue;
        };
        let projected = rows
            .iter()
            .filter_map(project_hidden_tool_result_row)
            .take(6)
            .collect::<Vec<_>>();
        if !projected.is_empty() {
            return Some(Value::Array(projected));
        }
    }
    None
}

fn project_hidden_tool_result_row(value: &Value) -> Option<Value> {
    match value {
        Value::String(raw) => {
            let snippet = trim_text(raw.trim(), 1_200);
            (!snippet.is_empty()).then(|| Value::String(snippet))
        }
        Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for key in [
                "source_kind",
                "title",
                "locator",
                "snippet",
                "snippet_preview",
                "summary",
                "content_preview",
                "score",
                "timestamp",
                "permissions",
                "status_code",
                "query",
                "stage",
                "provider",
                "status",
                "error",
                "links",
            ] {
                let Some(value) = map.get(key) else { continue };
                match value {
                    Value::String(raw) => {
                        let cleaned =
                            trim_text(raw.trim(), if key == "locator" { 2_200 } else { 1_200 });
                        if !cleaned.is_empty() {
                            out.insert(key.to_string(), Value::String(cleaned));
                        }
                    }
                    Value::Number(_) => {
                        out.insert(key.to_string(), value.clone());
                    }
                    Value::Bool(_) => {
                        out.insert(key.to_string(), value.clone());
                    }
                    Value::Array(rows) if key == "links" => {
                        let links = rows
                            .iter()
                            .filter_map(Value::as_str)
                            .map(|raw| trim_text(raw.trim(), 2_200))
                            .filter(|raw| !raw.is_empty())
                            .take(4)
                            .map(Value::String)
                            .collect::<Vec<_>>();
                        if !links.is_empty() {
                            out.insert(key.to_string(), Value::Array(links));
                        }
                    }
                    _ => {}
                }
            }
            (!out.is_empty()).then(|| Value::Object(out))
        }
        _ => None,
    }
}

fn carry_hidden_tool_result_artifacts(card: &mut Value, payload: &Value) {
    let Some(obj) = card.as_object_mut() else {
        return;
    };
    for key in [
        "search_results",
        "provider_results",
        "evidence_pack",
        "evidence_pack_candidates",
        "evidence_refs",
        "tool_result_quality",
        "evidence_pack_quality",
    ] {
        if let Some(value) = tool_result_hidden_artifact_value(payload, key) {
            obj.insert(key.to_string(), value);
        }
    }
}

fn response_tool_card(
    id: String,
    tool_name: &str,
    input: &Value,
    payload: &Value,
    is_error: bool,
    status: &str,
) -> Value {
    let mut artifact_payload = if payload.is_object() {
        payload.clone()
    } else {
        let mut obj = serde_json::Map::new();
        if !payload.is_null() {
            obj.insert("raw_result".to_string(), payload.clone());
            if let Some(raw) = payload.as_str() {
                let summary = trim_text(raw.trim(), 1_200);
                if !summary.is_empty() {
                    obj.insert("summary".to_string(), Value::String(summary));
                }
            }
        }
        Value::Object(obj)
    };
    if let Some(obj) = artifact_payload.as_object_mut() {
        for (key, value) in [
            ("query", input.get("query").or_else(|| input.get("q"))),
            ("source", input.get("source")),
        ] {
            if obj
                .get(key)
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or("")
                .is_empty()
            {
                if let Some(value) = value.cloned() {
                    obj.insert(key.to_string(), value);
                }
            }
        }
        let normalized_tool = normalize_tool_name(tool_name);
        if matches!(
            normalized_tool.as_str(),
            "batch_query" | "batch-query" | "web_search" | "search_web" | "search" | "web_query"
        ) && obj
            .get("source")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("")
            .is_empty()
        {
            obj.insert("source".to_string(), json!("web"));
        }
        if obj
            .get("status")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("")
            .is_empty()
        {
            obj.insert("status".to_string(), json!(status));
        }
        if is_error
            && obj
                .get("error")
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or("")
                .is_empty()
        {
            obj.insert("error".to_string(), json!("tool_execution_failed"));
        }
    }
    let tool_attempt_receipt = payload
        .pointer("/tool_pipeline/tool_attempt_receipt")
        .cloned()
        .or_else(|| payload.get("tool_attempt_receipt").cloned())
        .or_else(|| payload.pointer("/tool_attempt/attempt").cloned())
        .unwrap_or(Value::Null);
    let mut card = json!({
        "id": id,
        "name": normalize_tool_name(tool_name),
        "input": trim_text(&input.to_string(), 4000),
        "result": trim_text(&summarize_tool_payload(tool_name, payload), 24_000),
        "is_error": is_error,
        "blocked": status == "blocked" || status == "policy_denied",
        "status": status,
        "tool_attempt_receipt": tool_attempt_receipt
    });
    carry_hidden_tool_result_artifacts(&mut card, &artifact_payload);
    card
}

#[cfg(test)]
mod response_tool_card_tests {
    use super::*;

    #[test]
    fn response_tool_card_carries_hidden_search_results_from_tool_pipeline() {
        let payload = json!({
            "tool_pipeline": {
                "tool_attempt_receipt": {"status": "ok"},
                "raw_payload": {
                    "search_results": [
                        {
                            "title": "LangGraph docs",
                            "locator": "https://docs.langchain.com/langgraph",
                            "snippet": "LangGraph documentation covers durable execution, checkpoints, and human-in-the-loop review for reliable agents.",
                            "score": 0.91
                        }
                    ],
                    "provider_results": [
                        {
                            "query": "Compare LangGraph vs CrewAI",
                            "stage": "primary",
                            "provider": "direct_http",
                            "summary": "Web search tooling is degraded (provider readiness mismatch). Retry after credentials or provider runtime are repaired.",
                            "error": "web_search_tool_surface_degraded",
                            "links": [
                                "https://docs.langchain.com/langgraph"
                            ],
                            "ok": false
                        }
                    ],
                    "evidence_refs": [
                        {"title": "LangGraph docs", "locator": "https://docs.langchain.com/langgraph", "score": 0.91}
                    ],
                    "tool_result_quality": {"version": "v1", "flags": ["insufficient_evidence"]}
                }
            }
        });

        let card = response_tool_card(
            "tool-direct-batch_query".to_string(),
            "batch_query",
            &json!({"query": "Compare LangGraph vs CrewAI"}),
            &payload,
            false,
            "no_results",
        );

        assert_eq!(
            card.pointer("/search_results/0/title")
                .and_then(Value::as_str),
            Some("LangGraph docs")
        );
        assert_eq!(
            card.pointer("/evidence_refs/0/locator")
                .and_then(Value::as_str),
            Some("https://docs.langchain.com/langgraph")
        );
        assert_eq!(
            card.pointer("/provider_results/0/provider")
                .and_then(Value::as_str),
            Some("direct_http")
        );
        assert_eq!(
            card.pointer("/tool_result_quality/version")
                .and_then(Value::as_str),
            Some("v1")
        );
    }

    #[test]
    fn response_tool_card_carries_materialized_evidence_candidates() {
        let payload = json!({
            "tool_pipeline": {
                "raw_payload": {
                    "evidence_pack_candidates": [
                        {
                            "source_kind": "browser_materialized_page",
                            "title": "Rendered research page",
                            "locator": "https://example.test/rendered",
                            "snippet": "The rendered page contains enough extracted body text to support synthesis through the normal evidence pack lane.",
                            "claim_hints": [
                                "Rendered content can enter synthesis as packaged evidence."
                            ],
                            "term_hints": ["rendered", "evidence"],
                            "score": 76.0,
                            "confidence": "usable"
                        }
                    ]
                }
            }
        });

        let card = response_tool_card(
            "tool-direct-browser_materialize_page".to_string(),
            "browser_materialize_page",
            &json!({"url": "https://example.test/rendered"}),
            &payload,
            false,
            "ok",
        );

        assert_eq!(
            card.pointer("/evidence_pack_candidates/0/source_kind")
                .and_then(Value::as_str),
            Some("browser_materialized_page")
        );
        assert_eq!(
            card.pointer("/evidence_refs/0/locator")
                .and_then(Value::as_str),
            Some("https://example.test/rendered")
        );
        assert_eq!(
            card.pointer("/evidence_refs/0/title")
                .and_then(Value::as_str),
            Some("Rendered research page")
        );
    }

    #[test]
    fn response_tool_card_carries_evidence_pack_quality() {
        let payload = json!({
            "tool_pipeline": {
                "raw_payload": {
                    "evidence_pack_quality": {
                        "version": "evidence_pack_quality_v1",
                        "status": "usable",
                        "usable_count": 3,
                        "source_domain_count": 2
                    }
                }
            }
        });

        let card = response_tool_card(
            "tool-direct-batch_query".to_string(),
            "batch_query",
            &json!({"query": "Research current evidence"}),
            &payload,
            false,
            "ok",
        );

        assert_eq!(
            card.pointer("/evidence_pack_quality/status")
                .and_then(Value::as_str),
            Some("usable")
        );
        assert_eq!(
            card.pointer("/evidence_pack_quality/usable_count")
                .and_then(Value::as_u64),
            Some(3)
        );
    }

    #[test]
    fn response_tool_card_derives_hidden_search_results_from_web_search_payload() {
        let payload = json!({
            "provider": "bing_rss",
            "summary": "Mastra search returned limited but real source material.",
            "content": "Mastra docs describe a TypeScript agent framework with workflow primitives and deployment options.",
            "links": [
                "https://mastra.ai/docs/overview",
                "https://github.com/mastra-ai/mastra"
            ],
            "provider_raw_count": 8,
            "provider_filtered_count": 2,
            "tool_pipeline": {
                "input": {
                    "query": "Research Mastra for TypeScript agent workflows"
                }
            }
        });

        let card = response_tool_card(
            "tool-direct-web_search".to_string(),
            "web_search",
            &json!({"query": "Research Mastra for TypeScript agent workflows"}),
            &payload,
            false,
            "ok",
        );

        assert_eq!(
            card.pointer("/search_results/0/locator")
                .and_then(Value::as_str),
            Some("https://mastra.ai/docs/overview")
        );
        assert_eq!(
            card.pointer("/search_results/0/provider")
                .and_then(Value::as_str),
            Some("bing_rss")
        );
        assert_eq!(
            card.pointer("/provider_results/0/provider")
                .and_then(Value::as_str),
            Some("bing_rss")
        );
        assert_eq!(
            card.pointer("/provider_results/0/query")
                .and_then(Value::as_str),
            Some("Research Mastra for TypeScript agent workflows")
        );

        let quality_only = response_tool_card(
            "tool-direct-web_search".to_string(),
            "web_search",
            &json!({"query": "Research Mastra for TypeScript agent workflows"}),
            &json!({
                "tool_result_quality": {
                    "candidate_quality": [
                        {
                            "title": "Mastra docs",
                            "locator": "https://mastra.ai/",
                            "snippet_preview": "Mastra is a TypeScript AI agent framework.",
                            "score": 0.52
                        }
                    ]
                }
            }),
            false,
            "ok",
        );
        assert_eq!(
            quality_only
                .pointer("/search_results/0/locator")
                .and_then(Value::as_str),
            Some("https://mastra.ai/")
        );
        assert_eq!(
            quality_only
                .pointer("/evidence_refs/0/locator")
                .and_then(Value::as_str),
            Some("https://mastra.ai/")
        );
    }

    #[test]
    fn response_tool_card_derives_hidden_evidence_refs_from_provider_results() {
        let payload = json!({
            "tool_pipeline": {
                "raw_payload": {
                    "provider_results": [
                        {
                            "provider": "direct_http",
                            "stage": "duckduckgo_instant",
                            "query": "Find recent benchmarks comparing agent frameworks",
                            "locator": "https://api.duckduckgo.com/?q=agent%20framework%20benchmark",
                            "summary": "Search provider returned only low-signal instant-answer material.",
                            "status": "ok"
                        }
                    ]
                }
            }
        });

        let card = response_tool_card(
            "tool-direct-batch_query".to_string(),
            "batch_query",
            &json!({"query": "Find recent benchmarks comparing agent frameworks"}),
            &payload,
            false,
            "no_results",
        );

        assert_eq!(
            card.pointer("/provider_results/0/provider")
                .and_then(Value::as_str),
            Some("direct_http")
        );
        assert_eq!(
            card.pointer("/evidence_refs/0/locator")
                .and_then(Value::as_str),
            Some("https://api.duckduckgo.com/?q=agent%20framework%20benchmark")
        );
    }

    #[test]
    fn response_tool_card_derives_hidden_evidence_refs_when_explicit_array_is_empty() {
        let payload = json!({
            "evidence_refs": [],
            "provider_results": [
                {
                    "provider": "direct_http",
                    "stage": "duckduckgo_instant",
                    "query": "Find recent benchmarks comparing agent frameworks",
                    "locator": "https://api.duckduckgo.com/?q=agent%20framework%20benchmark",
                    "summary": "Search provider returned only low-signal instant-answer material.",
                    "status": "ok"
                }
            ]
        });

        let card = response_tool_card(
            "tool-direct-batch_query".to_string(),
            "batch_query",
            &json!({"query": "Find recent benchmarks comparing agent frameworks"}),
            &payload,
            false,
            "no_results",
        );

        assert_eq!(
            card.pointer("/evidence_refs/0/locator")
                .and_then(Value::as_str),
            Some("https://api.duckduckgo.com/?q=agent%20framework%20benchmark")
        );
    }

    #[test]
    fn response_tool_card_derives_hidden_provider_results_from_batch_query_failure_shape() {
        let payload = json!({
            "partial_failure_details": [
                "primary:search_failed"
            ]
        });

        let card = response_tool_card(
            "tool-direct-batch_query".to_string(),
            "batch_query",
            &json!({"source":"web","query":"Summarize recent changes in LangGraph, CrewAI, and AutoGen and assess their impact on production agent systems."}),
            &payload,
            false,
            "error",
        );

        assert_eq!(card.pointer("/provider_results/0/query").and_then(Value::as_str), Some("Summarize recent changes in LangGraph, CrewAI, and AutoGen and assess their impact on production agent systems."));
        assert_eq!(
            card.pointer("/provider_results/0/provider")
                .and_then(Value::as_str),
            Some("web")
        );
        assert_eq!(
            card.pointer("/provider_results/0/status")
                .and_then(Value::as_str),
            Some("error")
        );
        assert_eq!(
            card.pointer("/provider_results/0/failure_detail")
                .and_then(Value::as_str),
            Some("primary:search_failed")
        );
    }

    #[test]
    fn response_tool_card_derives_hidden_provider_results_from_receiptless_error_shape() {
        let card = response_tool_card(
            "tool-auto-batch_query".to_string(),
            "batch_query",
            &json!({
                "query": "Compare current agent frameworks on reliability and deployment.",
                "aperture": "medium"
            }),
            &Value::Null,
            true,
            "error",
        );

        assert_eq!(
            card.pointer("/provider_results/0/provider")
                .and_then(Value::as_str),
            Some("web")
        );
        assert_eq!(
            card.pointer("/provider_results/0/query")
                .and_then(Value::as_str),
            Some("Compare current agent frameworks on reliability and deployment.")
        );
        assert_eq!(
            card.pointer("/provider_results/0/status")
                .and_then(Value::as_str),
            Some("error")
        );
        assert_eq!(
            card.pointer("/provider_results/0/error")
                .and_then(Value::as_str),
            Some("tool_execution_failed")
        );
        assert_eq!(
            card.pointer("/evidence_refs/0/provider")
                .and_then(Value::as_str),
            Some("web")
        );
    }
}

fn latent_tool_candidate_completion_cards(
    root: &Path,
    snapshot: &Value,
    actor_agent_id: &str,
    existing: Option<&Value>,
    user_message: &str,
    draft_response: &str,
    allow_draft_retry_fallback: bool,
    latent_tool_candidates: &Value,
    response_tools: &[Value],
) -> Vec<Value> {
    let _ = (
        root,
        snapshot,
        actor_agent_id,
        existing,
        user_message,
        draft_response,
        allow_draft_retry_fallback,
        latent_tool_candidates,
        response_tools,
    );
    // LLM-authoritative workflow mode: never execute semantic/latent tools as
    // supplemental cards. The model must request tools through the workflow CD.
    Vec::new()
}
