fn tool_rows_for_llm_recovery(response_tools: &[Value], limit: usize) -> Value {
    let mut rows = Vec::<Value>::new();
    for tool in response_tools.iter().take(limit.clamp(1, 8)) {
        let name = normalize_tool_name(tool.get("name").and_then(Value::as_str).unwrap_or("tool"));
        let display_name = if name == "batch_query" || name == "batch-query" {
            "web_search"
        } else if name.is_empty() {
            "tool"
        } else {
            name.as_str()
        };
        let input = clean_text(tool.get("input").and_then(Value::as_str).unwrap_or(""), 800);
        let result = clean_text(
            tool.get("result").and_then(Value::as_str).unwrap_or(""),
            2_000,
        );
        let status = clean_text(
            tool.get("status").and_then(Value::as_str).unwrap_or(""),
            120,
        );
        let mut row = json!({
            "name": display_name,
            "input": input,
            "status": status,
            "blocked": tool.get("blocked").and_then(Value::as_bool).unwrap_or(false),
            "is_error": tool.get("is_error").and_then(Value::as_bool).unwrap_or(false),
            "result": result
        });
        if let Some(quality) = tool_quality_diagnostics_for_llm(tool) {
            row["quality_diagnostics"] = quality;
        }
        rows.push(row);
    }
    Value::Array(rows)
}

fn tool_result_quality_object(tool: &Value) -> Option<&Value> {
    tool.get("tool_result_quality")
        .or_else(|| tool.pointer("/tool_pipeline/raw_payload/tool_result_quality"))
        .filter(|value| value.is_object())
}

fn tool_quality_string_array(quality: &Value, pointer: &str, limit: usize) -> Vec<String> {
    quality
        .pointer(pointer)
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| clean_text(row, 120))
                .filter(|row| !row.is_empty())
                .take(limit)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn tool_quality_retry_recommended(tool: &Value) -> bool {
    tool_result_quality_object(tool)
        .and_then(|quality| quality.pointer("/retry/recommended"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn tool_quality_diagnostics_for_llm(tool: &Value) -> Option<Value> {
    let quality = tool_result_quality_object(tool)?;
    let candidate_quality = quality
        .get("candidate_quality")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .take(3)
                .map(|candidate| {
                    json!({
                        "title": clean_text(candidate.get("title").and_then(Value::as_str).unwrap_or(""), 160),
                        "domain": clean_text(candidate.get("domain").and_then(Value::as_str).unwrap_or(""), 160),
                        "snippet_preview": clean_text(candidate.get("snippet_preview").and_then(Value::as_str).unwrap_or(""), 320),
                        "score": candidate.get("score").cloned().unwrap_or(Value::Null),
                        "flags": tool_quality_string_array(candidate, "/flags", 6)
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Some(json!({
        "status": clean_text(quality.get("status").and_then(Value::as_str).unwrap_or(""), 80),
        "flags": tool_quality_string_array(quality, "/flags", 12),
        "evidence_count": quality.get("evidence_count").cloned().unwrap_or(Value::Null),
        "candidate_count": quality.get("candidate_count").cloned().unwrap_or(Value::Null),
        "retry": {
            "recommended": quality.pointer("/retry/recommended").and_then(Value::as_bool).unwrap_or(false),
            "reason": clean_text(quality.pointer("/retry/reason").and_then(Value::as_str).unwrap_or(""), 120),
            "next_action": clean_text(quality.pointer("/retry/next_action").and_then(Value::as_str).unwrap_or(""), 160),
            "query_strategy_hints": tool_quality_string_array(quality, "/retry/query_strategy_hints", 8)
        },
        "candidate_quality": candidate_quality
    }))
}

fn tool_hidden_array_len(tool: &Value, key: &str) -> usize {
    tool.get(key)
        .and_then(Value::as_array)
        .map(|rows| rows.len())
        .or_else(|| {
            tool.pointer(&format!("/tool_pipeline/raw_payload/{key}"))
                .and_then(Value::as_array)
                .map(|rows| rows.len())
        })
        .unwrap_or(0)
}

fn tool_hidden_array(tool: &Value, key: &str) -> Vec<Value> {
    tool.get(key)
        .and_then(Value::as_array)
        .cloned()
        .or_else(|| {
            tool.pointer(&format!("/tool_pipeline/raw_payload/{key}"))
                .and_then(Value::as_array)
                .cloned()
        })
        .unwrap_or_default()
}

fn tool_hidden_value<'a>(tool: &'a Value, key: &str) -> Option<&'a Value> {
    tool.get(key).or_else(|| {
        tool.get("tool_pipeline")
            .and_then(|pipeline| pipeline.get("raw_payload"))
            .and_then(|raw_payload| raw_payload.get(key))
    })
}

fn tool_string_array_at(
    value: Option<&Value>,
    pointer: &str,
    limit: usize,
    item_len: usize,
) -> Vec<String> {
    value
        .and_then(|row| row.pointer(pointer))
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| clean_text(row, item_len))
                .filter(|row| !row.is_empty())
                .take(limit)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn tool_query_metadata_value(tool: &Value) -> Option<Value> {
    if let Some(query_metadata) = tool_hidden_value(tool, "query_metadata") {
        return Some(query_metadata.clone());
    }
    let input = tool
        .get("input")
        .or_else(|| tool.pointer("/tool_pipeline/raw_payload/input"))
        .or_else(|| tool.get("request_payload"))
        .or_else(|| tool.pointer("/tool_pipeline/raw_payload/request_payload"))?;
    let input = match input {
        Value::String(raw) => serde_json::from_str::<Value>(raw).ok()?,
        Value::Object(_) => input.clone(),
        _ => return None,
    };
    if let Some(query_metadata) = input.get("query_metadata").filter(|row| row.is_object()) {
        return Some(query_metadata.clone());
    }
    let mut synthesized = serde_json::Map::new();
    for key in [
        "required_coverage",
        "query_metadata_policy",
        "keywords",
        "aliases",
        "negative_terms",
    ] {
        if let Some(value) = input.get(key) {
            synthesized.insert(key.to_string(), value.clone());
        }
    }
    if synthesized.is_empty() {
        None
    } else {
        Some(Value::Object(synthesized))
    }
}

fn synthesis_metadata_string_array(
    metadata: Option<&Value>,
    pointer: &str,
    limit: usize,
    item_len: usize,
) -> Vec<String> {
    metadata
        .and_then(|row| row.pointer(pointer))
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| clean_text(row, item_len))
                .filter(|row| !row.is_empty())
                .take(limit)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn synthesis_push_unique_clean_string(
    out: &mut Vec<String>,
    seen: &mut HashSet<String>,
    raw: &str,
    max_len: usize,
) {
    let cleaned = clean_text(raw, max_len);
    if cleaned.is_empty() {
        return;
    }
    let key = cleaned.to_ascii_lowercase();
    if seen.insert(key) {
        out.push(cleaned);
    }
}

fn synthesis_message_is_current_intent(message: &str) -> bool {
    let lowered = clean_text(message, 800).to_ascii_lowercase();
    [
        "today",
        "this week",
        "this month",
        "current",
        "latest",
        "recent",
        "news",
        "update",
        "updates",
        "landscape",
        "state of",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn synthesis_message_is_comparison_intent(message: &str) -> bool {
    let lowered = clean_text(message, 800).to_ascii_lowercase();
    [
        "compare",
        "comparison",
        "versus",
        " vs ",
        "tradeoff",
        "tradeoffs",
        "benchmark",
        "benchmarks",
        "best ",
        "which ",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn synthesis_answer_shape(message: &str) -> &'static str {
    if synthesis_message_is_comparison_intent(message) {
        "comparison"
    } else if synthesis_message_is_current_intent(message) {
        "update"
    } else {
        "briefing"
    }
}

fn synthesis_source_preferences(message: &str, metadata: Option<&Value>) -> Vec<String> {
    let mut out = Vec::<String>::new();
    let mut seen = HashSet::<String>::new();
    let entities = synthesis_metadata_string_array(metadata, "/required_coverage/entities", 8, 120);
    let comparison_intent = synthesis_message_is_comparison_intent(message);
    let current_intent = synthesis_message_is_current_intent(message);
    if current_intent {
        for value in [
            "announcement_or_news",
            "public_institution",
            "documentation_or_reference",
            "scholarly_or_research",
        ] {
            synthesis_push_unique_clean_string(&mut out, &mut seen, value, 80);
        }
    }
    if comparison_intent || !entities.is_empty() {
        for value in [
            "documentation_or_reference",
            "repository_or_dataset",
            "public_institution",
            "community_or_forum",
        ] {
            synthesis_push_unique_clean_string(&mut out, &mut seen, value, 80);
        }
    }
    if out.is_empty() {
        for value in [
            "documentation_or_reference",
            "public_institution",
            "scholarly_or_research",
            "repository_or_dataset",
            "community_or_forum",
        ] {
            synthesis_push_unique_clean_string(&mut out, &mut seen, value, 80);
        }
    }
    out.into_iter().take(6).collect()
}

fn synthesis_query_plan_strings(tool: &Value, limit: usize) -> Vec<String> {
    tool_hidden_array(tool, "query_plan")
        .into_iter()
        .filter_map(|row| row.as_str().map(|value| clean_text(value, 220)))
        .filter(|value| !value.is_empty())
        .take(limit)
        .collect()
}

fn synthesis_query_source(tool: &Value) -> String {
    clean_text(
        tool_hidden_value(tool, "source")
            .and_then(Value::as_str)
            .unwrap_or_else(|| tool.get("name").and_then(Value::as_str).unwrap_or("tool")),
        80,
    )
}

fn derived_research_brief_for_tools(message: &str, response_tools: &[Value]) -> Value {
    let existing = synthesis_first_hidden_object(response_tools, "research_brief");
    if existing
        .as_object()
        .map(|row| !row.is_empty())
        .unwrap_or(false)
    {
        return existing;
    }
    let Some(tool) = response_tools
        .iter()
        .find(|tool| {
            tool_query_metadata_value(tool).is_some() || tool_hidden_value(tool, "query").is_some()
        })
        .or_else(|| response_tools.first())
    else {
        return Value::Null;
    };
    let metadata = tool_query_metadata_value(tool);
    let mut open_dimensions =
        synthesis_metadata_string_array(metadata.as_ref(), "/required_coverage/facets", 8, 180);
    let mut open_seen = open_dimensions
        .iter()
        .map(|row| row.to_ascii_lowercase())
        .collect::<HashSet<_>>();
    if open_dimensions.is_empty() {
        for lane in synthesis_query_plan_strings(tool, 4)
            .into_iter()
            .skip(1)
            .take(3)
        {
            synthesis_push_unique_clean_string(&mut open_dimensions, &mut open_seen, &lane, 220);
        }
    }
    let known_constraints =
        synthesis_metadata_string_array(metadata.as_ref(), "/negative_terms", 8, 120);
    json!({
        "schema_version": "research_brief_v1",
        "original_user_goal": clean_text(message, 1_200),
        "research_objective": clean_text(message, 1_200),
        "open_dimensions": open_dimensions,
        "known_constraints": known_constraints,
        "date_context": chrono::Utc::now().date_naive().to_string(),
        "source": synthesis_query_source(tool),
        "source_preferences": synthesis_source_preferences(message, metadata.as_ref()),
        "answer_shape": synthesis_answer_shape(message),
        "metadata_authority": clean_text(
            metadata
                .as_ref()
                .and_then(|row| row.pointer("/compilation/authority"))
                .and_then(Value::as_str)
                .unwrap_or("dashboard_tool_handoff"),
            120,
        )
    })
}

fn derived_evidence_needs_for_tools(message: &str, response_tools: &[Value]) -> Value {
    let existing = synthesis_first_hidden_object(response_tools, "evidence_needs");
    if existing
        .as_object()
        .map(|row| !row.is_empty())
        .unwrap_or(false)
    {
        return existing;
    }
    let Some(tool) = response_tools
        .iter()
        .find(|tool| {
            tool_query_metadata_value(tool).is_some() || tool_hidden_value(tool, "query").is_some()
        })
        .or_else(|| response_tools.first())
    else {
        return Value::Null;
    };
    let metadata = tool_query_metadata_value(tool);
    let entities =
        synthesis_metadata_string_array(metadata.as_ref(), "/required_coverage/entities", 8, 120);
    let mut coverage_facets =
        synthesis_metadata_string_array(metadata.as_ref(), "/required_coverage/facets", 8, 180);
    let mut seen = coverage_facets
        .iter()
        .map(|row| row.to_ascii_lowercase())
        .collect::<HashSet<_>>();
    if !entities.is_empty() {
        synthesis_push_unique_clean_string(&mut coverage_facets, &mut seen, "entity_coverage", 80);
    }
    if synthesis_message_is_current_intent(message) {
        synthesis_push_unique_clean_string(&mut coverage_facets, &mut seen, "freshness", 80);
    }
    if synthesis_message_is_comparison_intent(message) {
        synthesis_push_unique_clean_string(&mut coverage_facets, &mut seen, "cross_comparison", 80);
    }
    if coverage_facets.is_empty() {
        synthesis_push_unique_clean_string(
            &mut coverage_facets,
            &mut seen,
            "overall_objective",
            80,
        );
    }
    let minimum_claims = if synthesis_message_is_comparison_intent(message)
        || synthesis_message_is_current_intent(message)
    {
        3
    } else {
        2
    };
    let minimum_distinct_domains =
        if entities.len() > 1 || synthesis_message_is_comparison_intent(message) {
            3
        } else {
            2
        };
    json!({
        "schema_version": "evidence_needs_v1",
        "coverage_facets": coverage_facets,
        "source_classes": synthesis_source_preferences(message, metadata.as_ref()),
        "required_materialization": "materialized_or_trusted_feed",
        "minimum_claims": minimum_claims,
        "minimum_distinct_domains": minimum_distinct_domains,
        "citation_expectation": "claim_backed_source_refs",
        "recency_window": if synthesis_message_is_current_intent(message) {
            json!({
                "mode": "current_context",
                "anchor_date": chrono::Utc::now().date_naive().to_string()
            })
        } else {
            Value::Null
        }
    })
}

fn synthesis_coverage_lane_key(kind: &str, requested_text: &str) -> String {
    format!(
        "{}:{}",
        clean_text(kind, 80).to_ascii_lowercase(),
        clean_text(requested_text, 240).to_ascii_lowercase()
    )
}

fn synthesis_coverage_status_rank(status: &str) -> u8 {
    match clean_text(status, 80).to_ascii_lowercase().as_str() {
        "covered" | "usable" => 3,
        "weak" | "partial" | "low_signal" | "low-signal" => 2,
        "missing" | "absent" => 1,
        _ => 0,
    }
}

fn query_metadata_required_coverage_default_status(tool: &Value) -> &'static str {
    let Some(coverage) =
        tool_result_quality_object(tool).and_then(|quality| quality.get("coverage"))
    else {
        return "missing";
    };
    let bucket_status = clean_text(
        coverage
            .get("bucket_status")
            .and_then(Value::as_str)
            .unwrap_or(""),
        80,
    )
    .to_ascii_lowercase();
    let missing_bucket_count = coverage
        .get("missing_buckets")
        .and_then(Value::as_array)
        .map(|rows| rows.iter().filter(|row| row.as_str().is_some()).count())
        .unwrap_or(0);
    match bucket_status.as_str() {
        "covered" if missing_bucket_count == 0 => "usable",
        "weak" | "partial" => "partial",
        _ => "missing",
    }
}

fn push_synthesis_coverage_lane(
    lanes: &mut Vec<Value>,
    kind: &str,
    requested_text: &str,
    status: &str,
    source: &str,
    limit: usize,
) {
    let kind = clean_text(kind, 80);
    let requested_text = clean_text(requested_text, 240);
    let status = clean_text(status, 80);
    let source = clean_text(source, 120);
    if kind.is_empty() || requested_text.is_empty() {
        return;
    }
    let key = synthesis_coverage_lane_key(&kind, &requested_text);
    if let Some(existing) = lanes.iter_mut().find(|row| {
        synthesis_coverage_lane_key(
            row.get("kind").and_then(Value::as_str).unwrap_or(""),
            row.get("requested_text")
                .and_then(Value::as_str)
                .unwrap_or(""),
        ) == key
    }) {
        let existing_rank = synthesis_coverage_status_rank(
            existing.get("status").and_then(Value::as_str).unwrap_or(""),
        );
        if synthesis_coverage_status_rank(&status) > existing_rank {
            existing["status"] = Value::String(status);
            existing["source"] = Value::String(source);
        }
        return;
    }
    if lanes.len() >= limit {
        return;
    }
    lanes.push(json!({
        "kind": kind,
        "requested_text": requested_text,
        "status": status,
        "source": source
    }));
}

fn synthesis_coverage_kind_for_requested_text(
    query_metadata: Option<&Value>,
    requested_text: &str,
) -> String {
    let requested = clean_text(requested_text, 240).to_ascii_lowercase();
    if requested.is_empty() {
        return "coverage_lane".to_string();
    }
    if tool_string_array_at(query_metadata, "/required_coverage/entities", 24, 240)
        .iter()
        .any(|row| clean_text(row, 240).to_ascii_lowercase() == requested)
    {
        return "entity".to_string();
    }
    if tool_string_array_at(query_metadata, "/required_coverage/facets", 24, 240)
        .iter()
        .any(|row| clean_text(row, 240).to_ascii_lowercase() == requested)
    {
        return "facet".to_string();
    }
    "coverage_lane".to_string()
}

fn synthesis_coverage_lanes_for_tools(response_tools: &[Value], limit: usize) -> Vec<Value> {
    let mut lanes = Vec::<Value>::new();
    let limit = limit.clamp(1, 32);
    for tool in response_tools.iter().take(8) {
        let query_metadata = tool_query_metadata_value(tool);
        let query_metadata = query_metadata.as_ref();
        let required_coverage_status = query_metadata_required_coverage_default_status(tool);
        let required_coverage_source = if required_coverage_status == "usable" {
            "query_metadata_required_coverage_tool_quality"
        } else {
            "query_metadata_required_coverage"
        };
        for entity in tool_string_array_at(query_metadata, "/required_coverage/entities", 16, 240) {
            push_synthesis_coverage_lane(
                &mut lanes,
                "entity",
                &entity,
                required_coverage_status,
                required_coverage_source,
                limit,
            );
        }
        for facet in tool_string_array_at(query_metadata, "/required_coverage/facets", 16, 240) {
            push_synthesis_coverage_lane(
                &mut lanes,
                "facet",
                &facet,
                required_coverage_status,
                required_coverage_source,
                limit,
            );
        }
        for row in tool_hidden_array(tool, "evidence_coverage") {
            let requested_text = clean_text(
                row.get("requested_text")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                240,
            );
            if requested_text.is_empty() {
                continue;
            }
            let kind = clean_text(
                row.get("facet_kind")
                    .or_else(|| row.get("coverage_kind"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                80,
            );
            let kind = if kind.is_empty() {
                synthesis_coverage_kind_for_requested_text(query_metadata, &requested_text)
            } else {
                kind
            };
            let status = clean_text(
                row.get("status")
                    .and_then(Value::as_str)
                    .unwrap_or("missing"),
                80,
            );
            push_synthesis_coverage_lane(
                &mut lanes,
                &kind,
                &requested_text,
                &status,
                "evidence_coverage",
                limit,
            );
        }
        if lanes.len() >= limit {
            break;
        }
    }
    lanes
}

fn compact_tool_evidence_item(source: &str, row: &Value) -> Value {
    if let Some(raw) = row.as_str() {
        return json!({
            "source": source,
            "ref": clean_text(raw, 240)
        });
    }
    let source_kind = clean_text(
        row.get("source_kind")
            .or_else(|| row.get("source_type"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        80,
    );
    let source_class = clean_text(
        row.get("source_class")
            .or_else(|| row.get("source_type"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        80,
    );
    let source_type = clean_text(
        row.get("source_type")
            .or_else(|| row.get("source_kind"))
            .or_else(|| row.get("source_class"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        80,
    );
    let relevant_extract = clean_text(
        row.get("relevant_extract")
            .or_else(|| row.get("extract"))
            .or_else(|| row.get("support_snippet"))
            .or_else(|| row.get("raw_content_excerpt"))
            .or_else(|| row.get("snippet"))
            .or_else(|| row.get("snippet_preview"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        700,
    );
    let why_relevant_to_query = clean_text(
        row.get("why_relevant_to_query")
            .or_else(|| row.get("relevance_reason"))
            .or_else(|| row.get("selection_reason"))
            .or_else(|| row.get("coverage_reason"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        360,
    );
    let published_or_observed_date = clean_text(
        row.get("published_or_observed_date")
            .or_else(|| row.get("published_date"))
            .or_else(|| row.get("published_at"))
            .or_else(|| row.get("observed_at"))
            .or_else(|| row.get("timestamp"))
            .or_else(|| row.get("date"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        80,
    );
    json!({
        "source": source,
        "evidence_packet_version": "evidence_packet_v1",
        "pack_version": clean_text(row.get("pack_version").and_then(Value::as_str).unwrap_or(""), 80),
        "source_type": source_type,
        "source_kind": source_kind,
        "source_class": source_class,
        "title": clean_text(row.get("title").and_then(Value::as_str).unwrap_or(""), 180),
        "locator": clean_text(row.get("locator").or_else(|| row.get("url")).and_then(Value::as_str).unwrap_or(""), 260),
        "source_domain": clean_text(row.get("source_domain").or_else(|| row.get("domain")).and_then(Value::as_str).unwrap_or(""), 120),
        "snippet": clean_text(row.get("snippet").or_else(|| row.get("snippet_preview")).and_then(Value::as_str).unwrap_or(""), 420),
        "relevant_extract": relevant_extract,
        "why_relevant_to_query": why_relevant_to_query,
        "claim_hints": compact_string_array(row.get("claim_hints"), 6, 180),
        "term_hints": compact_string_array(row.get("term_hints"), 8, 80),
        "score": row.get("score").cloned().unwrap_or(Value::Null),
        "confidence": row.get("confidence").cloned().unwrap_or(Value::Null),
        "materialization_quality": clean_text(row.get("materialization_quality").and_then(Value::as_str).unwrap_or(""), 80),
        "counts_as_usable_evidence": row.get("counts_as_usable_evidence").cloned().unwrap_or(Value::Null),
        "quality_flags": compact_string_array(row.get("quality_flags").or_else(|| row.get("flags")), 8, 80),
        "coverage_facets": row.get("coverage_facets").cloned().unwrap_or_else(|| json!([])),
        "freshness": row.get("freshness").cloned().unwrap_or(Value::Null),
        "published_or_observed_date": published_or_observed_date,
        "evidence_claims": compact_claim_array(row.get("evidence_claims"), 4)
    })
}

fn tool_evidence_row_counts_as_synthesis_evidence(row: &Value) -> bool {
    if row
        .get("counts_as_usable_evidence")
        .and_then(Value::as_bool)
        == Some(false)
    {
        return false;
    }
    let confidence = clean_text(
        row.get("confidence").and_then(Value::as_str).unwrap_or(""),
        80,
    )
    .to_ascii_lowercase();
    if matches!(
        confidence.as_str(),
        "candidate_only" | "low_confidence_raw" | "rejected"
    ) {
        return false;
    }
    let flags = compact_string_array(
        row.get("quality_flags").or_else(|| row.get("flags")),
        12,
        80,
    );
    if flags
        .as_array()
        .map(|rows| {
            rows.iter().filter_map(Value::as_str).any(|flag| {
                matches!(
                    flag,
                    "junk_marker"
                        | "style_or_script_dump"
                        | "social_video_shell"
                        | "weak_query_overlap_only"
                        | "low_confidence_raw"
                        | "low_score"
                )
            })
        })
        .unwrap_or(false)
    {
        return false;
    }
    true
}

fn compact_string_array(value: Option<&Value>, limit: usize, item_len: usize) -> Value {
    value
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| clean_text(row, item_len))
                .filter(|row| !row.is_empty())
                .take(limit)
                .map(Value::String)
                .collect::<Vec<_>>()
        })
        .map(Value::Array)
        .unwrap_or_else(|| json!([]))
}

fn compact_claim_array(value: Option<&Value>, limit: usize) -> Value {
    value
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .take(limit)
                .map(|row| {
                    json!({
                        "claim": clean_text(row.get("claim").and_then(Value::as_str).unwrap_or(""), 220),
                        "support_snippet": clean_text(row.get("support_snippet").and_then(Value::as_str).unwrap_or(""), 260),
                        "confidence": row.get("confidence").cloned().unwrap_or(Value::Null),
                        "entities": compact_string_array(row.get("entities"), 6, 120),
                        "coverage_facets": row.get("coverage_facets").cloned().unwrap_or_else(|| json!([])),
                        "limitations": compact_string_array(row.get("limitations"), 6, 120),
                        "source_ref": json!({
                            "title": clean_text(row.pointer("/source_ref/title").and_then(Value::as_str).unwrap_or(""), 180),
                            "locator": clean_text(row.pointer("/source_ref/locator").and_then(Value::as_str).unwrap_or(""), 240),
                            "source_domain": clean_text(row.pointer("/source_ref/source_domain").and_then(Value::as_str).unwrap_or(""), 120),
                            "source_kind": clean_text(row.pointer("/source_ref/source_kind").and_then(Value::as_str).unwrap_or(""), 80),
                            "materialization_quality": clean_text(row.pointer("/source_ref/materialization_quality").and_then(Value::as_str).unwrap_or(""), 80)
                        })
                    })
                })
                .collect::<Vec<_>>()
        })
        .map(Value::Array)
        .unwrap_or_else(|| json!([]))
}

fn synthesis_first_hidden_object(response_tools: &[Value], key: &str) -> Value {
    response_tools
        .iter()
        .filter_map(|tool| tool_hidden_value(tool, key))
        .find(|value| value.is_object())
        .cloned()
        .unwrap_or_else(|| json!({}))
}

fn synthesis_evidence_pack_for_tools(response_tools: &[Value], limit: usize) -> Value {
    let mut items = Vec::<Value>::new();
    for tool in response_tools {
        for row in tool_hidden_array(tool, "evidence_pack") {
            if !tool_evidence_row_counts_as_synthesis_evidence(&row) {
                continue;
            }
            if items.len() >= limit {
                return Value::Array(items);
            }
            items.push(compact_tool_evidence_item("evidence_pack", &row));
        }
        for row in tool_hidden_array(tool, "evidence_pack_candidates") {
            if !tool_evidence_row_counts_as_synthesis_evidence(&row) {
                continue;
            }
            if items.len() >= limit {
                return Value::Array(items);
            }
            items.push(compact_tool_evidence_item("evidence_pack_candidate", &row));
        }
        for row in tool_hidden_array(tool, "evidence_refs") {
            if items.len() >= limit {
                return Value::Array(items);
            }
            items.push(compact_tool_evidence_item("evidence_ref", &row));
        }
    }
    if items.is_empty() {
        for tool in response_tools.iter().take(limit) {
            let result = clean_text(
                tool.get("result").and_then(Value::as_str).unwrap_or(""),
                420,
            );
            if result.is_empty() {
                continue;
            }
            items.push(json!({
                "source": "tool_result_summary",
                "tool_name": normalize_tool_name(tool.get("name").and_then(Value::as_str).unwrap_or("tool")),
                "snippet": result
            }));
        }
    }
    Value::Array(items)
}

fn synthesis_evidence_claims_for_tools(response_tools: &[Value], limit: usize) -> Value {
    let mut items = Vec::<Value>::new();
    for tool in response_tools.iter().take(8) {
        for row in tool_hidden_array(tool, "evidence_claims") {
            if items.len() >= limit {
                return Value::Array(items);
            }
            items.push(json!({
                "claim": clean_text(row.get("claim").and_then(Value::as_str).unwrap_or(""), 220),
                "support_snippet": clean_text(row.get("support_snippet").and_then(Value::as_str).unwrap_or(""), 260),
                "confidence": row.get("confidence").cloned().unwrap_or(Value::Null),
                "entities": compact_string_array(row.get("entities"), 6, 120),
                "coverage_facets": row.get("coverage_facets").cloned().unwrap_or_else(|| json!([])),
                "limitations": compact_string_array(row.get("limitations"), 6, 120),
                "source_ref": json!({
                    "title": clean_text(row.pointer("/source_ref/title").and_then(Value::as_str).unwrap_or(""), 180),
                    "locator": clean_text(row.pointer("/source_ref/locator").and_then(Value::as_str).unwrap_or(""), 240),
                    "source_domain": clean_text(row.pointer("/source_ref/source_domain").and_then(Value::as_str).unwrap_or(""), 120),
                    "source_kind": clean_text(row.pointer("/source_ref/source_kind").and_then(Value::as_str).unwrap_or(""), 80),
                    "materialization_quality": clean_text(row.pointer("/source_ref/materialization_quality").and_then(Value::as_str).unwrap_or(""), 80)
                })
            }));
        }
    }
    Value::Array(items)
}

fn synthesis_answer_units_for_tools(
    message: &str,
    response_tools: &[Value],
    limit: usize,
) -> Value {
    Value::Array(
        evidence_packet_answer_units_for_goal(message, response_tools, limit)
            .into_iter()
            .map(Value::String)
            .collect(),
    )
}

fn synthesis_tool_receipt_refs(response_tools: &[Value]) -> Value {
    Value::Array(
        response_tools
            .iter()
            .take(8)
            .enumerate()
            .map(|(idx, tool)| {
                let receipt = response_tool_receipt_id(tool);
                if receipt.is_empty() {
                    format!(
                        "tool_observation:{}:{}",
                        idx,
                        normalize_tool_name(
                            tool.get("name").and_then(Value::as_str).unwrap_or("tool")
                        )
                    )
                } else {
                    receipt
                }
            })
            .map(Value::String)
            .collect(),
    )
}

fn synthesis_tool_result_quality(response_tools: &[Value]) -> String {
    if response_tools.is_empty() {
        return "absent".to_string();
    }
    let search_result_count = response_tools
        .iter()
        .map(|tool| tool_hidden_array_len(tool, "search_results"))
        .sum::<usize>();
    let provider_result_count = response_tools
        .iter()
        .map(|tool| tool_hidden_array_len(tool, "provider_results"))
        .sum::<usize>();
    let evidence_ref_count = response_tools
        .iter()
        .map(|tool| {
            tool_hidden_array_len(tool, "evidence_refs")
                + tool_hidden_array_len(tool, "evidence_pack")
                + tool_hidden_array_len(tool, "evidence_pack_candidates")
                + tool_hidden_array_len(tool, "evidence_claims")
        })
        .sum::<usize>();
    let has_evidence =
        search_result_count > 0 || provider_result_count > 0 || evidence_ref_count > 0;
    let has_error = !response_tools_failure_reason_for_user(response_tools, 4)
        .trim()
        .is_empty();
    let low_signal = response_tools_any_low_signal(response_tools)
        || response_tools.iter().any(tool_quality_retry_recommended);
    if has_evidence && !has_error && !low_signal {
        "usable".to_string()
    } else if has_evidence {
        "partial_or_low_signal".to_string()
    } else if low_signal {
        "low_signal".to_string()
    } else if has_error {
        "error".to_string()
    } else {
        "no_evidence".to_string()
    }
}

fn synthesis_coverage_gaps(response_tools: &[Value]) -> Value {
    let mut gaps = synthesis_coverage_lanes_for_tools(response_tools, 16);
    for tool in response_tools.iter().take(6) {
        if let Some(quality) = tool_result_quality_object(tool) {
            for flag in tool_quality_string_array(quality, "/flags", 8) {
                gaps.push(json!({"kind": "quality_flag", "detail": flag}));
            }
            let retry_reason = clean_text(
                quality
                    .pointer("/retry/reason")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                160,
            );
            if !retry_reason.is_empty() && retry_reason != "none" {
                gaps.push(json!({"kind": "retry_reason", "detail": retry_reason}));
            }
        }
    }
    Value::Array(gaps.into_iter().take(12).collect())
}

fn workflow_synthesis_input_for_final_response(
    message: &str,
    response_tools: &[Value],
    selected_workflow: &Value,
) -> Value {
    let final_output_contract = selected_workflow
        .get("final_output_contract")
        .cloned()
        .unwrap_or_else(|| json!({}));
    json!({
        "schema_version": "live_synthesis_input_v1",
        "source": "dashboard_tool_observation_handoff",
        "user_goal": clean_text(message, 1_200),
        "research_brief": derived_research_brief_for_tools(message, response_tools),
        "evidence_needs": derived_evidence_needs_for_tools(message, response_tools),
        "tool_result_quality": synthesis_tool_result_quality(response_tools),
        "tool_receipt_refs": synthesis_tool_receipt_refs(response_tools),
        "evidence_pack": synthesis_evidence_pack_for_tools(response_tools, 8),
        "evidence_claims": synthesis_evidence_claims_for_tools(response_tools, 12),
        "answer_units": synthesis_answer_units_for_tools(message, response_tools, 8),
        "coverage_gaps": synthesis_coverage_gaps(response_tools),
        "final_output_contract": final_output_contract
    })
}

fn workflow_tool_state_prompt_context(response_tools: &[Value]) -> String {
    let limited = response_tools.iter().take(6).collect::<Vec<_>>();
    let tool_count = limited.len();
    let tool_names = limited
        .iter()
        .map(|tool| normalize_tool_name(tool.get("name").and_then(Value::as_str).unwrap_or("tool")))
        .filter(|name| !name.is_empty())
        .collect::<Vec<_>>();
    let search_result_count = limited
        .iter()
        .map(|tool| tool_hidden_array_len(tool, "search_results"))
        .sum::<usize>();
    let provider_result_count = limited
        .iter()
        .map(|tool| tool_hidden_array_len(tool, "provider_results"))
        .sum::<usize>();
    let evidence_ref_count = limited
        .iter()
        .map(|tool| {
            tool_hidden_array_len(tool, "evidence_refs")
                + tool_hidden_array_len(tool, "evidence_pack")
                + tool_hidden_array_len(tool, "evidence_pack_candidates")
                + tool_hidden_array_len(tool, "evidence_claims")
        })
        .sum::<usize>();
    let evidence_claim_count = limited
        .iter()
        .map(|tool| tool_hidden_array_len(tool, "evidence_claims"))
        .sum::<usize>();
    let answer_unit_count = evidence_packet_answer_units(response_tools, 8).len();
    let tool_statuses = limited
        .iter()
        .filter_map(|tool| tool.get("status").and_then(Value::as_str))
        .map(|status| clean_text(status, 80))
        .filter(|status| !status.is_empty())
        .collect::<Vec<_>>();
    let tool_error_count = limited
        .iter()
        .filter(|tool| {
            let status = tool.get("status").and_then(Value::as_str).unwrap_or("");
            tool.get("is_error")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                || matches!(status, "error" | "failed" | "timeout" | "blocked")
        })
        .count();
    let low_signal_count = limited
        .iter()
        .filter(|tool| {
            let status = tool.get("status").and_then(Value::as_str).unwrap_or("");
            let quality_flags = tool_result_quality_object(tool)
                .map(|quality| tool_quality_string_array(quality, "/flags", 16))
                .unwrap_or_default();
            tool.get("low_signal")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                || matches!(status, "low_signal" | "no_results")
                || tool_quality_retry_recommended(tool)
                || quality_flags.iter().any(|flag| {
                    matches!(
                        flag.as_str(),
                        "insufficient_evidence"
                            | "low_signal"
                            | "low_relevance_filtered"
                            | "comparison_evidence_insufficient"
                            | "weak_single_source"
                    )
                })
        })
        .count();
    let retry_recommended_count = limited
        .iter()
        .filter(|tool| tool_quality_retry_recommended(tool))
        .count();
    let quality_flags = limited
        .iter()
        .filter_map(|tool| tool_result_quality_object(tool))
        .flat_map(|quality| tool_quality_string_array(quality, "/flags", 16))
        .take(16)
        .collect::<Vec<_>>();
    let retry_reasons = limited
        .iter()
        .filter_map(|tool| {
            tool_result_quality_object(tool)
                .and_then(|quality| quality.pointer("/retry/reason"))
                .and_then(Value::as_str)
                .map(|reason| clean_text(reason, 120))
                .filter(|reason| !reason.is_empty() && reason != "none")
        })
        .take(8)
        .collect::<Vec<_>>();
    let recorded_evidence_available =
        search_result_count > 0 || provider_result_count > 0 || evidence_ref_count > 0;
    let tool_result_quality = if tool_count == 0 {
        "none"
    } else if recorded_evidence_available
        && low_signal_count == 0
        && tool_error_count == 0
        && retry_recommended_count == 0
    {
        "evidence_available"
    } else if recorded_evidence_available {
        "partial_or_low_signal_evidence"
    } else if low_signal_count > 0 {
        "low_signal"
    } else if tool_error_count > 0 {
        "error"
    } else {
        "no_evidence"
    };
    let summary = json!({
        "recorded_tool_outcome_count": tool_count,
        "recorded_tool_names": tool_names,
        "recorded_tool_statuses": tool_statuses,
        "recorded_tool_error_count": tool_error_count,
        "recorded_low_signal_count": low_signal_count,
        "recorded_retry_recommended_count": retry_recommended_count,
        "recorded_quality_flags": quality_flags,
        "recorded_retry_reasons": retry_reasons,
        "recorded_tool_result_quality": tool_result_quality,
        "recorded_search_results": search_result_count,
        "recorded_provider_results": provider_result_count,
        "recorded_evidence_refs": evidence_ref_count,
        "recorded_evidence_claims": evidence_claim_count,
        "recorded_answer_units": answer_unit_count,
        "recorded_tool_result_available": tool_count > 0,
        "recorded_evidence_available": recorded_evidence_available
    });
    let summary_json = serde_json::to_string(&summary)
        .unwrap_or_else(|_| "{\"recorded_tool_outcome_count\":0}".to_string());
    clean_text(
        &format!(
            "Recorded tool/evidence state for this turn:\n{summary_json}\n\nThis block is private synthesis context. Do not quote, summarize, enumerate, or expose its JSON keys, `recorded_*` fields, quality flags, tool boundary signals, or internal outcome posture in the user-facing answer. The user-facing answer must still be useful and directly answer the goal if formatting is stripped away; any answer shape is acceptable if the plain text stands on its own. Only use tool or evidence details that are explicitly present in this recorded state and the recorded tool outcomes below. Treat the recorded quality, retry, claim, and answer-unit fields as private boundary signals: retry recommendations, low-signal flags, missing claim-backed evidence, missing answer units, errors, or no evidence mean the workflow ran but evidence may be insufficient. Prefer reusing recorded claim, extract, and source wording for concrete facts. Do not add unsupported topical labels, exact dates, ages, numbers, or categorical gloss unless those details appear in the recorded evidence or you clearly mark them as inference. Choose exactly one internal outcome posture before answering, then convert it into plain user-facing prose without exposing the posture label itself. If evidence counts are zero, do not claim returned snippets, evidence refs, or source-backed findings for this turn. Evidence being insufficient does not prove that the underlying fact, source surface, or entity does not exist."
        ),
        2_000,
    )
}

fn workflow_missing_turn_tool_context_prompt(message: &str, response_tools: &[Value]) -> String {
    if !response_tools.is_empty() {
        return String::new();
    }
    let lowered = clean_text(message, 800).to_ascii_lowercase();
    if lowered.is_empty() {
        return String::new();
    }
    let generic_tool_turn_reference = lowered.contains("tool")
        && (lowered.contains(" return")
            || lowered.contains(" returns")
            || lowered.contains(" returned")
            || lowered.contains(" came back")
            || lowered.contains(" comes back"));
    let references_absent_tool_state = lowered.contains("tool result")
        || lowered.contains("tool results")
        || lowered.contains("search result")
        || lowered.contains("search results")
        || lowered.contains("source snippet")
        || lowered.contains("source snippets")
        || lowered.contains("evidence ref")
        || lowered.contains("evidence refs")
        || lowered.contains("returned result")
        || lowered.contains("returned results")
        || lowered.contains("returned snippets")
        || generic_tool_turn_reference;
    let asks_to_use_referenced_state = lowered.contains("synthesize")
        || lowered.contains("summary")
        || lowered.contains("summarize")
        || lowered.contains("compare")
        || lowered.contains("comparison")
        || lowered.contains("cite")
        || lowered.contains("evidence")
        || lowered.contains("what we know")
        || lowered.contains("what we do not know");
    let post_tool_shape = references_absent_tool_state && asks_to_use_referenced_state;
    if !post_tool_shape {
        return String::new();
    }
    clean_text(
        "No returned tool result is available in this turn, so no source-backed synthesis is available yet. Begin the answer with that exact sentence. Then answer briefly using only the available limits, current uncertainty, and one bounded next search step. Any clear format is acceptable. Example formats include a short paragraph, brief bullets, or a compact mixed structure, but none is required. Do not claim returned snippets, evidence refs, low-signal results, or source-backed findings for this turn. Do not treat this missing turn state as proof that the underlying information or source surface does not exist.",
        800,
    )
}

fn workflow_missing_turn_tool_context_fallback(message: &str, response_tools: &[Value]) -> String {
    if workflow_missing_turn_tool_context_prompt(message, response_tools).is_empty() {
        return String::new();
    }
    let request_summary = first_sentence(&clean_text(message, 1_200), 180);
    let knowns = if request_summary.is_empty() {
        "What we know is that this request expects a post-tool synthesis step, but no returned tool result, snippets, or evidence refs are present in this turn.".to_string()
    } else {
        format!(
            "What we know is only the user's requested synthesis shape: {}. No returned tool result, snippets, or evidence refs are present in this turn.",
            request_summary
        )
    };
    let unknowns = "What we do not know is which source-backed findings, low-signal results, tradeoffs, or evidence refs the missing tool result would have supported, so no source-backed conclusion is justified yet. This missing turn state does not establish that the underlying information, source surface, or entity is absent.";
    clean_text(
        &format!(
            "No returned tool result is available in this turn, so no source-backed synthesis is available yet. {knowns} {unknowns} My recommendation is to rerun one focused source-evidence query before making a conclusion. The next best search query is `focused query for the requested topic with one named entity, one source family, and one time window`."
        ),
        2_400,
    )
}

fn workflow_pending_tool_confirmation_fallback(
    pending_request: &Value,
    response_tools: &[Value],
) -> String {
    if !response_tools.is_empty()
        || pending_request.get("status").and_then(Value::as_str) != Some("pending_confirmation")
    {
        return String::new();
    }
    let family = clean_text(
        pending_request
            .get("selected_tool_family")
            .or_else(|| pending_request.get("tool_family"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        120,
    )
    .replace(['_', '-'], " ")
    .to_ascii_lowercase();
    let tool_name = normalize_tool_name(
        pending_request
            .get("tool_name")
            .or_else(|| pending_request.get("tool"))
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    let fallback = if family.contains("web")
        || matches!(
            tool_name.as_str(),
            "web_search" | "batch_query" | "web_fetch"
        ) {
        "The permission policy requires your confirmation before I run that web research step. Say `confirm` and I'll run the search, then synthesize what comes back."
    } else if family.contains("workspace")
        || matches!(
            tool_name.as_str(),
            "workspace_search" | "file_read" | "parse_workspace" | "apply_patch"
        )
    {
        "The permission policy requires your confirmation before I inspect the workspace for that. Say `confirm` and I'll run the relevant workspace step first."
    } else if family.contains("browser") || tool_name.contains("browser") {
        "The permission policy requires your confirmation before I inspect that in the browser. Say `confirm` and I'll run the browser step first."
    } else if family.contains("shell")
        || tool_name.contains("command")
        || tool_name.contains("exec")
        || tool_name.contains("terminal")
    {
        "The permission policy requires your confirmation before I run that terminal action. Say `confirm` and I'll execute it."
    } else {
        "The permission policy requires your confirmation before I continue that action. Say `confirm` and I'll continue."
    };
    clean_text(fallback, 320)
}

fn workflow_missing_turn_tool_context_response_contract_satisfied(response_text: &str) -> bool {
    let normalized = clean_text(response_text, 3_200).to_ascii_lowercase();
    if !normalized.starts_with(
        "no returned tool result is available in this turn, so no source-backed synthesis is available yet.",
    ) {
        return false;
    }
    let has_limits = normalized.contains("what we know")
        || normalized.contains("only grounded detail")
        || normalized.contains("available detail")
        || normalized.contains("no returned tool result");
    let has_uncertainty = normalized.contains("what we do not know")
        || normalized.contains("do not know")
        || normalized.contains("is unavailable")
        || normalized.contains("no source-backed conclusion");
    let has_next_query = normalized.contains("next best search query")
        || normalized.contains("next useful action")
        || normalized.contains("next query")
        || normalized.contains("search query");
    let has_source_signal = normalized.contains("source") || normalized.contains("evidence");
    let has_recommendation = normalized.contains("next useful move")
        || normalized.contains("i recommend")
        || normalized.contains("recommend");
    has_limits && has_uncertainty && has_next_query && has_source_signal && has_recommendation
}

fn workflow_missing_turn_tool_context_repaired_response(
    message: &str,
    response_tools: &[Value],
    response_text: &str,
) -> String {
    let cleaned = clean_chat_text(response_text, 32_000);
    if workflow_missing_turn_tool_context_prompt(message, response_tools).is_empty() {
        return cleaned;
    }
    if workflow_missing_turn_tool_context_response_contract_satisfied(&cleaned) {
        return cleaned;
    }
    workflow_missing_turn_tool_context_fallback(message, response_tools)
}

fn ensure_tool_turn_response_text(response_text: &str, response_tools: &[Value]) -> String {
    let cleaned = clean_chat_text(response_text, 32_000);
    if !cleaned.is_empty() || response_tools.is_empty() {
        return cleaned;
    }
    String::new()
}

fn persist_last_assistant_turn_metadata(
    root: &Path,
    agent_id: &str,
    assistant_text: &str,
    metadata: &Value,
) -> Value {
    let id = clean_agent_id(agent_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "agent_id_required"});
    }
    let mut state = load_session_state(root, &id);
    let active_id = clean_text(
        state
            .get("active_session_id")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    );
    let assistant = clean_chat_text(assistant_text, 64_000);
    let mut updated = false;
    if let Some(sessions) = state.get_mut("sessions").and_then(Value::as_array_mut) {
        for session in sessions.iter_mut() {
            let sid = clean_text(
                session
                    .get("session_id")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                120,
            );
            if sid != active_id {
                continue;
            }
            if !session
                .get("messages")
                .map(Value::is_array)
                .unwrap_or(false)
            {
                session["messages"] = Value::Array(Vec::new());
            }
            let messages = session
                .get_mut("messages")
                .and_then(Value::as_array_mut)
                .expect("messages");
            let target_idx = messages.iter().rposition(|row| {
                clean_text(row.get("role").and_then(Value::as_str).unwrap_or(""), 24)
                    .eq_ignore_ascii_case("assistant")
            });
            let idx = if let Some(found) = target_idx {
                found
            } else {
                messages
                    .push(json!({"role": "assistant", "text": assistant, "ts": crate::now_iso()}));
                messages.len().saturating_sub(1)
            };
            if let Some(target) = messages.get_mut(idx) {
                if !assistant.is_empty() {
                    target["text"] = Value::String(assistant.clone());
                }
                let safe_metadata = session_safe_turn_metadata(root, &id, metadata);
                if let Some(object) = safe_metadata.as_object() {
                    for (key, value) in object {
                        target[key] = value.clone();
                    }
                }
                if target
                    .get("ts")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .is_empty()
                {
                    target["ts"] = Value::String(crate::now_iso());
                }
            }
            session["updated_at"] = Value::String(crate::now_iso());
            updated = true;
            break;
        }
    }
    save_session_state(root, &id, &state);
    json!({"ok": true, "updated": updated, "agent_id": id})
}

#[cfg(test)]
mod tool_turn_response_text_tests {
    use super::*;

    #[test]
    fn tool_turn_response_text_withholds_non_llm_failure_fallback_copy() {
        let response = ensure_tool_turn_response_text(
            "",
            &[
                json!({"name":"batch_query","status":"failed","is_error":true,"blocked":false,"result":"query_result_mismatch"}),
            ],
        );
        assert!(response.trim().is_empty(), "{response}");
    }

    #[test]
    fn tool_turn_response_text_withholds_non_llm_findings_fallback_copy() {
        let response = ensure_tool_turn_response_text(
            "",
            &[
                json!({"name":"batch_query","status":"ok","is_error":false,"blocked":false,"result":"Key findings: OpenHands is an open-source AI software development agent platform."}),
            ],
        );
        assert!(response.trim().is_empty(), "{response}");
    }

    #[test]
    fn workflow_tool_state_prompt_context_exposes_boundary_quality() {
        let summary = workflow_tool_state_prompt_context(&[]);
        for needle in [
            "\"recorded_tool_outcome_count\":0",
            "\"recorded_tool_result_available\":false",
            "\"recorded_evidence_available\":false",
            "\"recorded_tool_result_quality\":\"none\"",
        ] {
            assert!(summary.contains(needle), "{summary}");
        }
        let summary = workflow_tool_state_prompt_context(&[json!({
            "name": "web_search",
            "status": "no_results",
            "provider_results": [{"provider": "direct_http"}],
            "evidence_refs": [{"source": "provider_result"}],
            "tool_result_quality": {
                "status": "no_results",
                "flags": ["insufficient_evidence", "low_relevance_filtered"],
                "evidence_count": 0,
                "candidate_count": 1,
                "retry": {
                    "recommended": true,
                    "reason": "insufficient_evidence",
                    "next_action": "agent_refine_query_pack_and_retry_if_budget_remains",
                    "query_strategy_hints": ["target primary or official sources"]
                }
            },
            "tool_pipeline": {
                "raw_payload": {
                    "search_results": [{"title": "Example result"}]
                }
            }
        })]);
        for needle in [
            "\"recorded_tool_outcome_count\":1",
            "\"recorded_tool_names\":[\"web_search\"]",
            "\"recorded_low_signal_count\":1",
            "\"recorded_search_results\":1",
            "\"recorded_provider_results\":1",
            "\"recorded_evidence_refs\":1",
            "\"recorded_tool_result_quality\":\"partial_or_low_signal_evidence\"",
            "\"recorded_retry_recommended_count\":1",
            "insufficient_evidence",
            "low_relevance_filtered",
            "\"recorded_evidence_available\":true",
            "tool boundary signals",
        ] {
            assert!(summary.contains(needle), "{summary}");
        }
    }

    #[test]
    fn tool_rows_for_llm_recovery_expose_quality_diagnostics_without_raw_payloads() {
        let rows = tool_rows_for_llm_recovery(
            &[json!({
                "name": "batch_query",
                "status": "no_results",
                "input": "{\"query\":\"scientific breakthroughs 2026\"}",
                "result": "Search providers ran but no usable findings were extracted.",
                "tool_result_quality": {
                    "status": "no_results",
                    "flags": ["insufficient_evidence"],
                    "evidence_count": 0,
                    "candidate_count": 0,
                    "retry": {
                        "recommended": true,
                        "reason": "insufficient_evidence",
                        "next_action": "agent_refine_query_pack_and_retry_if_budget_remains",
                        "query_strategy_hints": ["split the ask into specific entities or source types"]
                    },
                    "candidate_quality": [{
                        "title": "Search page",
                        "domain": "search.example",
                        "locator": "https://search.example/?q=x",
                        "snippet_preview": "No usable result",
                        "score": 0.12,
                        "flags": ["low_score"]
                    }]
                }
            })],
            6,
        );
        assert_eq!(
            rows.pointer("/0/quality_diagnostics/retry/recommended")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            rows.pointer("/0/quality_diagnostics/retry/next_action")
                .and_then(Value::as_str),
            Some("agent_refine_query_pack_and_retry_if_budget_remains")
        );
        assert!(
            rows.pointer("/0/quality_diagnostics/candidate_quality/0/snippet_preview")
                .and_then(Value::as_str)
                .unwrap_or("")
                .contains("No usable result"),
            "{rows}"
        );
        assert!(
            rows.pointer("/0/quality_diagnostics/candidate_quality/0/locator")
                .is_none(),
            "{rows}"
        );
    }

    #[test]
    fn workflow_synthesis_input_carries_evidence_pack_and_final_contract() {
        let selected_workflow = json!({
            "name": "research_synthesize_verify",
            "final_output_contract": {
                "visible_chat_source": "llm_final_answer_only",
                "chat_requirement": "Answer from available evidence without exposing traces."
            }
        });
        let input = workflow_synthesis_input_for_final_response(
            "What are some scientific breakthroughs in 2026?",
            &[json!({
                "name": "batch_query",
                "status": "ok",
                "result": "Battery chemistry and protein design examples were retrieved.",
                "tool_attempt_receipt": {"receipt_id": "receipt-123"},
                "research_brief": {
                    "schema_version": "research_brief_v1",
                    "research_objective": "What are some scientific breakthroughs in 2026?",
                    "answer_shape": "briefing"
                },
                "evidence_needs": {
                    "schema_version": "evidence_needs_v1",
                    "minimum_claims": 3,
                    "required_materialization": "materialized_or_trusted_feed"
                },
                "evidence_pack": [{
                    "pack_version": "evidence_pack_v1",
                    "source_kind": "browser_materialized_page",
                    "source_class": "news",
                    "title": "Battery milestone",
                    "locator": "https://example.test/battery",
                    "source_domain": "example.test",
                    "snippet": "A lab reported a battery chemistry milestone.",
                    "relevant_extract": "A lab reported a battery chemistry milestone that may matter for a 2026 breakthroughs briefing.",
                    "why_relevant_to_query": "It provides a source-backed example for the user's request about scientific breakthroughs in 2026.",
                    "published_or_observed_date": "2026-05-01",
                    "claim_hints": ["battery chemistry milestone"],
                    "quality_flags": ["primary_source_needed"],
                    "score": 0.84,
                    "confidence": "medium",
                    "materialization_quality": "full_materialized",
                    "counts_as_usable_evidence": true
                }],
                "evidence_claims": [{
                    "claim": "A lab reported a battery chemistry milestone.",
                    "support_snippet": "A lab reported a battery chemistry milestone.",
                    "confidence": "medium",
                    "entities": ["battery chemistry"],
                    "limitations": ["primary_source_needed"],
                    "source_ref": {
                        "title": "Battery milestone",
                        "locator": "https://example.test/battery",
                        "source_domain": "example.test",
                        "source_kind": "browser_materialized_page",
                        "materialization_quality": "full_materialized"
                    }
                }],
                "tool_result_quality": {
                    "status": "ok",
                    "flags": [],
                    "evidence_count": 1,
                    "candidate_count": 1,
                    "retry": {"recommended": false, "reason": "none"}
                }
            })],
            &selected_workflow,
        );

        assert_eq!(
            input.get("schema_version").and_then(Value::as_str),
            Some("live_synthesis_input_v1")
        );
        assert_eq!(
            input.get("tool_result_quality").and_then(Value::as_str),
            Some("usable")
        );
        assert_eq!(
            input
                .pointer("/tool_receipt_refs/0")
                .and_then(Value::as_str),
            Some("receipt-123")
        );
        assert_eq!(
            input
                .pointer("/evidence_pack/0/title")
                .and_then(Value::as_str),
            Some("Battery milestone")
        );
        assert_eq!(
            input
                .pointer("/evidence_pack/0/evidence_packet_version")
                .and_then(Value::as_str),
            Some("evidence_packet_v1")
        );
        assert_eq!(
            input
                .pointer("/evidence_pack/0/source_type")
                .and_then(Value::as_str),
            Some("browser_materialized_page")
        );
        assert_eq!(
            input
                .pointer("/evidence_pack/0/relevant_extract")
                .and_then(Value::as_str),
            Some("A lab reported a battery chemistry milestone that may matter for a 2026 breakthroughs briefing.")
        );
        assert_eq!(
            input
                .pointer("/evidence_pack/0/why_relevant_to_query")
                .and_then(Value::as_str),
            Some("It provides a source-backed example for the user's request about scientific breakthroughs in 2026.")
        );
        assert_eq!(
            input
                .pointer("/evidence_pack/0/published_or_observed_date")
                .and_then(Value::as_str),
            Some("2026-05-01")
        );
        assert_eq!(
            input
                .pointer("/research_brief/answer_shape")
                .and_then(Value::as_str),
            Some("briefing")
        );
        assert_eq!(
            input
                .pointer("/evidence_needs/required_materialization")
                .and_then(Value::as_str),
            Some("materialized_or_trusted_feed")
        );
        assert_eq!(
            input
                .pointer("/evidence_claims/0/claim")
                .and_then(Value::as_str),
            Some("A lab reported a battery chemistry milestone.")
        );
        assert_eq!(
            input
                .pointer("/evidence_claims/0/source_ref/materialization_quality")
                .and_then(Value::as_str),
            Some("full_materialized")
        );
        assert_eq!(
            input.pointer("/answer_units/0").and_then(Value::as_str),
            Some("battery chemistry milestone Source: Battery milestone, example.test.")
        );
        assert_eq!(
            input
                .pointer("/final_output_contract/visible_chat_source")
                .and_then(Value::as_str),
            Some("llm_final_answer_only")
        );
    }

    #[test]
    fn workflow_synthesis_input_filters_candidate_only_and_provider_payload_rows() {
        let input = workflow_synthesis_input_for_final_response(
            "Compare AlphaVac and BetaBot",
            &[json!({
                "name": "batch_query",
                "status": "ok",
                "provider_results": [{
                    "provider": "tavily",
                    "summary": "Raw provider payload should remain telemetry, not synthesis evidence."
                }],
                "evidence_pack": [
                    {
                        "pack_version": "evidence_pack_v1",
                        "source_kind": "web_page_enriched",
                        "title": "Candidate-only shell",
                        "locator": "https://candidate.example/shell",
                        "source_domain": "candidate.example",
                        "snippet": "This candidate-only row should not be handed to synthesis as evidence.",
                        "claim_hints": ["candidate-only row"],
                        "confidence": "candidate_only",
                        "materialization_quality": "candidate_only",
                        "counts_as_usable_evidence": false
                    },
                    {
                        "pack_version": "evidence_pack_v1",
                        "source_kind": "browser_materialized_page",
                        "title": "Usable source",
                        "locator": "https://evidence.example/source",
                        "source_domain": "evidence.example",
                        "snippet": "The usable source compares AlphaVac and BetaBot with direct evidence.",
                        "claim_hints": ["The usable source compares AlphaVac and BetaBot."],
                        "confidence": "usable",
                        "materialization_quality": "full_materialized",
                        "counts_as_usable_evidence": true
                    }
                ]
            })],
            &json!({}),
        );
        let rows = input
            .get("evidence_pack")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(rows.len(), 1, "{input:#?}");
        assert_eq!(
            rows.first()
                .and_then(|row| row.get("title"))
                .and_then(Value::as_str),
            Some("Usable source"),
            "{input:#?}"
        );
        assert!(
            !rows
                .iter()
                .any(|row| row.get("source").and_then(Value::as_str) == Some("provider_result")),
            "{input:#?}"
        );
    }

    #[test]
    fn workflow_synthesis_input_derives_research_brief_when_tool_omits_it() {
        let selected_workflow = json!({
            "name": "research_synthesize_verify",
            "final_output_contract": {}
        });
        let input = workflow_synthesis_input_for_final_response(
            "Give me news from this week about AI agents.",
            &[json!({
                "name": "batch_query",
                "status": "ok",
                "tool_attempt_receipt": {"receipt_id": "receipt-456"},
                "query_plan": [
                    "Give me news from this week about AI agents.",
                    "AI agents this week official sources",
                    "AI agents this week announcements"
                ],
                "query_metadata": {
                    "keywords": ["AI agents", "this week"],
                    "required_coverage": {
                        "entities": ["AI agents"],
                        "facets": ["news"]
                    },
                    "aliases": [],
                    "negative_terms": ["sports"],
                    "compilation": {
                        "authority": "agent_submitted_request_metadata"
                    }
                },
                "evidence_pack": [{
                    "pack_version": "evidence_pack_v1",
                    "source_kind": "browser_materialized_page",
                    "source_class": "news_or_current",
                    "title": "Agent platform update",
                    "locator": "https://example.test/agents",
                    "source_domain": "example.test",
                    "snippet": "A major agent platform shipped updates this week.",
                    "claim_hints": ["agent platform shipped updates this week"],
                    "quality_flags": [],
                    "score": 0.91,
                    "confidence": "medium",
                    "materialization_quality": "full_materialized",
                    "counts_as_usable_evidence": true
                }],
                "tool_result_quality": {
                    "status": "ok",
                    "flags": [],
                    "evidence_count": 1,
                    "candidate_count": 1,
                    "retry": {"recommended": false, "reason": "none"}
                }
            })],
            &selected_workflow,
        );

        assert_eq!(
            input
                .pointer("/research_brief/schema_version")
                .and_then(Value::as_str),
            Some("research_brief_v1")
        );
        assert_eq!(
            input
                .pointer("/research_brief/original_user_goal")
                .and_then(Value::as_str),
            Some("Give me news from this week about AI agents.")
        );
        assert_eq!(
            input
                .pointer("/research_brief/answer_shape")
                .and_then(Value::as_str),
            Some("update")
        );
        assert_eq!(
            input
                .pointer("/evidence_needs/schema_version")
                .and_then(Value::as_str),
            Some("evidence_needs_v1")
        );
        assert_eq!(
            input
                .pointer("/evidence_needs/required_materialization")
                .and_then(Value::as_str),
            Some("materialized_or_trusted_feed")
        );
        assert_eq!(
            input
                .pointer("/evidence_needs/citation_expectation")
                .and_then(Value::as_str),
            Some("claim_backed_source_refs")
        );
    }

    #[test]
    fn workflow_missing_turn_tool_context_prompt_activates_for_post_tool_synthesis_without_state() {
        let prompt = workflow_missing_turn_tool_context_prompt(
            "Use the returned source snippets to synthesize the tradeoffs and cite evidence refs.",
            &[],
        );
        assert!(
            prompt.starts_with(
                "No returned tool result is available in this turn, so no source-backed synthesis is available yet."
            ),
            "{prompt}"
        );
        assert!(prompt.contains("does not exist"), "{prompt}");
    }

    #[test]
    fn workflow_missing_turn_tool_context_fallback_is_general_and_shape_aware() {
        let response = workflow_missing_turn_tool_context_fallback(
            "Use the returned search results to synthesize a useful answer anyway. Tell me what we know, what we do not know, and the next best search query.",
            &[],
        );
        assert!(response.starts_with("No returned tool result is available in this turn, so no source-backed synthesis is available yet."), "{response}");
        for needle in [
            "What we know",
            "What we do not know",
            "My recommendation",
            "next best search query",
            "does not establish",
        ] {
            assert!(response.contains(needle), "{response}");
        }
        assert!(
            !response.contains("Infring") && !response.contains("agent frameworks"),
            "{response}"
        );
        assert!(
            workflow_missing_turn_tool_context_response_contract_satisfied(&response),
            "{response}"
        );
        let response = workflow_missing_turn_tool_context_fallback(
            "Use the returned source snippets to synthesize the tradeoffs and cite evidence refs without dumping the raw payload.",
            &[],
        );
        for needle in [
            "tradeoff",
            "evidence refs",
            "source-backed",
            "What we know",
            "My recommendation",
            "does not establish",
        ] {
            assert!(response.contains(needle), "{response}");
        }
        assert!(!response.contains("agent frameworks"), "{response}");
        assert!(
            workflow_missing_turn_tool_context_response_contract_satisfied(&response),
            "{response}"
        );
    }

    #[test]
    fn workflow_missing_turn_tool_context_repaired_response_replaces_telemetry_dump() {
        let repaired = workflow_missing_turn_tool_context_repaired_response(
            "After the web tool returns low-signal results for Infring, synthesize a useful answer anyway. Tell me what we know, what we do not know, and the next best search query.",
            &[],
            "Based on the recorded state for this turn, here is what we actually have: recorded_evidence_available is false and recorded_tool_result_quality is none.",
        );
        assert!(repaired.starts_with("No returned tool result is available in this turn, so no source-backed synthesis is available yet."), "{repaired}");
        assert!(repaired.contains("What we know"), "{repaired}");
        assert!(repaired.contains("What we do not know"), "{repaired}");
        assert!(repaired.contains("next best search query"), "{repaired}");
        assert!(
            workflow_missing_turn_tool_context_response_contract_satisfied(&repaired),
            "{repaired}"
        );
    }

    #[test]
    fn workflow_pending_tool_confirmation_fallback_covers_web_research_and_execution() {
        let response = workflow_pending_tool_confirmation_fallback(
            &json!({
                "status": "pending_confirmation",
                "selected_tool_family": "web_research",
                "tool_name": "batch_query"
            }),
            &[],
        );
        assert!(response.contains("permission"), "{response}");
        assert!(response.contains("research"), "{response}");
        assert!(response.contains("confirm"), "{response}");
        assert!(response.contains("synthesize"), "{response}");
        let response = workflow_pending_tool_confirmation_fallback(
            &json!({
                "status": "pending_confirmation",
                "selected_tool_family": "web_research",
                "tool_name": "batch_query"
            }),
            &[json!({"name": "batch_query", "status": "ok"})],
        );
        assert!(response.is_empty(), "{response}");
    }

    #[test]
    fn workflow_auto_executes_permitted_web_request_without_confirmation() {
        let root = tempfile::tempdir().expect("tempdir");
        let script_path = scripted_tool_harness_path(root.path());
        std::fs::create_dir_all(script_path.parent().expect("script parent")).expect("mkdir");
        write_json(
            &script_path,
            &json!({
                "queue": [
                    {
                        "tool": "web_search",
                        "payload": {
                            "ok": true,
                            "status": "ok",
                            "summary": "Recent evidence suggests Playwright remains the strongest default for browser automation testing.",
                            "links": ["https://example.com/playwright"]
                        }
                    }
                ]
            }),
        );
        let gate_payload = "{\"tool_family\":\"web_research\",\"tool\":\"web_search\",\"request_payload\":{\"query\":\"compare browser automation tools for testing agents\",\"aperture\":\"medium\"}}";
        let response = finalize_message_finalization_and_payload(
            root.path(),
            "agent-test",
            &json!({}),
            &json!({
                "model_provider": "ollama",
                "model_name": "kimi-k2.6:cloud",
                "runtime_model": "kimi-k2.6:cloud"
            }),
            "Compare browser automation tools for testing agents.",
            &json!({
                "provider": "ollama",
                "model": "kimi-k2.6:cloud",
                "runtime_model": "kimi-k2.6:cloud",
                "response": gate_payload,
                "context_window": 262144
            }),
            gate_payload.to_string(),
            Vec::new(),
            "model_direct_answer".to_string(),
            Vec::new(),
            json!({}),
            json!({}),
            Vec::new(),
            Vec::new(),
            "ollama".to_string(),
            "kimi-k2.6:cloud".to_string(),
            "ollama".to_string(),
            "kimi-k2.6:cloud".to_string(),
            None,
            String::new(),
            json!({}),
            262144,
            0,
            0.0,
            "low".to_string(),
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            false,
            false,
            false,
            0,
            false,
            json!({}),
            json!([]),
            json!([]),
            false,
        );

        assert_eq!(
            response
                .payload
                .pointer("/tools/0/name")
                .and_then(Value::as_str),
            Some("web_search")
        );
        assert_eq!(
            response
                .payload
                .pointer("/pending_tool_request/status")
                .and_then(Value::as_str),
            Some("executed")
        );
        assert!(
            !response
                .payload
                .get("response")
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim()
                .is_empty(),
            "auto-executed tool turns should still surface a user-facing response"
        );
        assert!(!response
            .payload
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("Say `confirm`"));

        let script = read_json(&script_path).expect("script file");
        assert_eq!(
            script
                .get("calls")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(1)
        );
    }
}
