fn apply_final_empty_response_diagnostic(
    workflow: &mut Value,
    message: &str,
    latest_assistant_text: &str,
    response_tools: &[Value],
) {
    let response_text = clean_text(
        workflow
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or(""),
        32_000,
    );
    if !response_text.is_empty() {
        return;
    }
    let _ = latest_assistant_text;
    let fallback_response = clean_text(
        &fallback_final_response_from_tool_evidence(message, response_tools),
        3_000,
    );
    if !fallback_response.is_empty() {
        apply_tool_evidence_fallback_response(
            workflow,
            response_tools,
            &fallback_response,
            "tool_evidence_runtime_fallback",
            "empty_response_replaced_from_tool_evidence",
            None,
            None,
            "tool_evidence_runtime_fallback_used",
            "final_presence_diagnostic",
        );
        return;
    }

    workflow["quality_telemetry"]["final_fallback_used"] = Value::Bool(false);
    workflow["final_llm_response"]["used"] = Value::Bool(false);
    workflow["final_llm_response"]["status"] = Value::String("empty_llm_response".to_string());
    workflow["final_llm_response"]["runtime_interference_disabled"] = Value::Bool(true);
    workflow["final_llm_response"]["visible_response_preserved"] = Value::Bool(false);
    workflow["final_llm_response"]["error"] = Value::String("empty_response".to_string());
    workflow["final_llm_response"]["last_reject_reason"] =
        Value::String("diagnostic_only_presence".to_string());
    record_workflow_diagnostic_event(
        workflow,
        "empty_response_presence_diagnostic",
        "final_presence_diagnostic",
    );
    set_turn_workflow_final_stage_status(workflow, "empty_llm_response");
}

fn fallback_coverage_lane_sentence(response_tools: &[Value]) -> String {
    let lanes = synthesis_coverage_lanes_for_tools(response_tools, 12);
    if lanes.is_empty() {
        return String::new();
    }
    let lane_label = |row: &Value| -> String {
        let kind = clean_text(row.get("kind").and_then(Value::as_str).unwrap_or(""), 80);
        let requested = clean_text(
            row.get("requested_text")
                .and_then(Value::as_str)
                .unwrap_or(""),
            180,
        );
        if requested.is_empty() {
            String::new()
        } else if kind == "entity" {
            requested
        } else {
            requested
        }
    };
    let covered = lanes
        .iter()
        .filter(|row| {
            matches!(
                row.get("status").and_then(Value::as_str),
                Some("covered") | Some("usable")
            )
        })
        .filter_map(|row| {
            let label = lane_label(row);
            if label.is_empty() {
                None
            } else {
                Some(label)
            }
        })
        .take(4)
        .collect::<Vec<_>>();
    let weak_or_missing = lanes
        .iter()
        .filter(|row| {
            !matches!(
                row.get("status").and_then(Value::as_str),
                Some("covered") | Some("usable")
            )
        })
        .filter_map(|row| {
            let label = lane_label(row);
            if label.is_empty() {
                None
            } else {
                Some(label)
            }
        })
        .take(8)
        .collect::<Vec<_>>();
    if !covered.is_empty() && !weak_or_missing.is_empty() {
        format!(
            "Coverage state: usable evidence is present for {}; weak or missing coverage remains for {}.",
            covered.join(", "),
            weak_or_missing.join(", ")
        )
    } else if !covered.is_empty() {
        format!(
            "Coverage state: usable evidence is present for {}.",
            covered.join(", ")
        )
    } else if !weak_or_missing.is_empty() {
        format!(
            "Coverage gaps still matter for: {}.",
            weak_or_missing.join(", ")
        )
    } else {
        String::new()
    }
}

fn response_tools_have_recorded_material(response_tools: &[Value]) -> bool {
    response_tools.iter().any(|tool| {
        !clean_text(
            tool.get("result").and_then(Value::as_str).unwrap_or(""),
            240,
        )
        .is_empty()
            || tool_hidden_array_len(tool, "search_results") > 0
            || tool_hidden_array_len(tool, "provider_results") > 0
            || tool_hidden_array_len(tool, "evidence_refs") > 0
            || tool_hidden_array_len(tool, "evidence_pack") > 0
            || tool_hidden_array_len(tool, "evidence_pack_candidates") > 0
    })
}

fn tool_evidence_outcome_posture(response_tools: &[Value]) -> &'static str {
    if response_tools.is_empty() || !response_tools_have_recorded_material(response_tools) {
        return "evidence_insufficient_answer";
    }
    let weak_or_missing_lane_count = synthesis_coverage_lanes_for_tools(response_tools, 16)
        .iter()
        .filter(|row| {
            !matches!(
                row.get("status").and_then(Value::as_str),
                Some("covered") | Some("usable")
            )
        })
        .count();
    let has_low_signal_or_failure = response_tools.iter().any(|tool| {
        let status = tool.get("status").and_then(Value::as_str).unwrap_or("");
        let quality_flags = tool_result_quality_object(tool)
            .map(|quality| tool_quality_string_array(quality, "/flags", 16))
            .unwrap_or_default();
        matches!(
            status,
            "low_signal" | "no_results" | "error" | "failed" | "timeout" | "blocked"
        ) || tool_quality_retry_recommended(tool)
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
    });
    if has_low_signal_or_failure || weak_or_missing_lane_count > 0 {
        "bounded_partial_answer"
    } else {
        "supported_answer"
    }
}

fn evidence_packet_text_field(row: &Value, keys: &[&str], max_len: usize) -> String {
    for key in keys {
        let value = clean_text(row.get(*key).and_then(Value::as_str).unwrap_or(""), max_len);
        if !value.is_empty() {
            return value;
        }
    }
    String::new()
}

fn evidence_packet_first_string(value: Option<&Value>, max_len: usize) -> String {
    match value {
        Some(Value::String(raw)) => clean_text(raw, max_len),
        Some(Value::Array(rows)) => rows
            .iter()
            .find_map(|row| {
                let value = evidence_packet_first_string(Some(row), max_len);
                (!value.is_empty()).then_some(value)
            })
            .unwrap_or_default(),
        Some(Value::Object(map)) => {
            for key in ["claim", "text", "summary", "snippet", "relevant_extract"] {
                let value = clean_text(map.get(key).and_then(Value::as_str).unwrap_or(""), max_len);
                if !value.is_empty() {
                    return value;
                }
            }
            String::new()
        }
        _ => String::new(),
    }
}

fn evidence_packet_claim_text(row: &Value) -> String {
    let claim = evidence_packet_first_answer_claim(row.get("claim_hints"), 260);
    if !claim.is_empty() {
        return claim;
    }
    let claim = evidence_packet_first_answer_claim(row.get("evidence_claims"), 260);
    if !claim.is_empty() {
        return claim;
    }
    for field in ["claim", "finding"] {
        let claim = evidence_packet_text_field(row, &[field], 260);
        if evidence_packet_text_is_answer_claim(&claim) {
            return claim;
        }
    }
    String::new()
}

fn evidence_packet_first_answer_claim(value: Option<&Value>, max_len: usize) -> String {
    match value {
        Some(Value::String(raw)) => {
            let cleaned = clean_text(raw, max_len);
            if evidence_packet_text_is_answer_claim(&cleaned) {
                cleaned
            } else {
                String::new()
            }
        }
        Some(Value::Array(rows)) => rows
            .iter()
            .find_map(|row| {
                let value = evidence_packet_first_answer_claim(Some(row), max_len);
                (!value.is_empty()).then_some(value)
            })
            .unwrap_or_default(),
        Some(Value::Object(map)) => {
            for key in ["claim", "text", "finding", "relevant_extract", "snippet"] {
                let value = evidence_packet_first_answer_claim(map.get(key), max_len);
                if !value.is_empty() {
                    return value;
                }
            }
            String::new()
        }
        _ => String::new(),
    }
}

fn evidence_packet_text_is_answer_claim(raw: &str) -> bool {
    let cleaned = clean_text(raw, 420);
    if cleaned.is_empty() {
        return false;
    }
    let normalized = cleaned.to_ascii_lowercase();
    let normalized_shell = format!(" {} ", normalize_coverage_lane_text(&cleaned));
    if normalized.starts_with("title:")
        || normalized.starts_with("description:")
        || normalized.starts_with("web result from ")
        || normalized.starts_with("source:")
        || normalized.starts_with("articles /")
        || normalized.starts_with("article /")
        || normalized.starts_with("blog /")
        || normalized.starts_with("user guide")
        || normalized.starts_with("description summary")
        || normalized.starts_with("this survey examines")
        || normalized.starts_with("this report examines")
        || normalized.starts_with("this guide examines")
        || normalized.starts_with("this overview examines")
        || normalized.starts_with("this article examines")
        || normalized.starts_with("this article explores")
        || normalized.starts_with("this paper examines")
        || normalized.starts_with("this paper explores")
        || normalized.contains(" mins read")
        || normalized.contains(" min read")
        || normalized.contains(" minute read")
        || normalized.starts_with("pt ")
        || normalized.starts_with("part ")
        || normalized.contains(" / menu ")
        || normalized.contains(" shop ")
        || normalized.contains("©")
        || cleaned.trim_end_matches('.').ends_with(':')
        || normalized_shell.contains(" affiliate disclosure ")
        || normalized_shell.contains(" reader supported ")
        || normalized_shell.contains(" if youre from the future ")
        || normalized_shell.contains(" if you re from the future ")
        || normalized_shell.contains(" other supported points ")
        || normalized_shell.contains(" important limitation ")
    {
        return false;
    }
    let word_count = normalized
        .split_whitespace()
        .filter(|word| word.chars().any(|ch| ch.is_ascii_alphanumeric()))
        .count();
    if word_count < 7 {
        return false;
    }
    if workflow_answer_unit_has_dangling_truncated_tail(&normalized) {
        return false;
    }
    let title_or_question_heading = (normalized.contains(" / ") && normalized.contains(':'))
        || (normalized.contains("what does") && normalized.contains("show"));
    if title_or_question_heading && !normalized.contains('.') {
        return false;
    }
    let has_sentence_verb = [
        " is ",
        " are ",
        " was ",
        " were ",
        " can ",
        " could ",
        " should ",
        " has ",
        " have ",
        " had ",
        " does ",
        " do ",
        " did ",
        " focuses ",
        " supports ",
        " emphasizes ",
        " provides ",
        " offers ",
        " requires ",
        " involves ",
        " differs ",
        " reported ",
        " published ",
        " found ",
        " shows ",
        " suggests ",
        " improved ",
        " continued ",
        " extends ",
        " limits ",
        " allows ",
    ]
    .iter()
    .any(|needle| normalized.contains(*needle));
    has_sentence_verb
}

fn evidence_packet_source_label(row: &Value) -> String {
    let title = evidence_packet_text_field(row, &["title", "source_title", "source_ref"], 120);
    let domain = evidence_packet_text_field(row, &["source_domain", "domain"], 80);
    let locator = evidence_packet_text_field(row, &["locator", "url", "link"], 160);
    if !title.is_empty() && !domain.is_empty() {
        format!("{title}, {domain}")
    } else if !title.is_empty() {
        title
    } else if !domain.is_empty() {
        domain
    } else {
        locator
    }
}

fn evidence_packet_counts_as_usable(row: &Value) -> bool {
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
    !matches!(
        confidence.as_str(),
        "candidate_only" | "low_confidence_raw" | "rejected"
    )
}

fn evidence_packet_answer_unit(row: &Value) -> Option<String> {
    if !evidence_packet_counts_as_usable(row) {
        return None;
    }
    let claim = evidence_packet_claim_text(row);
    let answer_text = if !claim.is_empty() {
        claim
    } else {
        ["relevant_extract", "support_snippet", "snippet", "content"]
            .iter()
            .find_map(|field| {
                let extract = evidence_packet_text_field(row, &[*field], 360);
                let sentence = first_sentence(&extract, 260);
                evidence_packet_text_is_answer_claim(&sentence).then_some(sentence)
            })
            .unwrap_or_default()
    };
    if answer_text.is_empty() {
        return None;
    }
    let source = evidence_packet_source_label(row);
    let unit = if source.is_empty() {
        answer_text
    } else {
        format!("{answer_text} Source: {source}.")
    };
    Some(clean_text(&unit, 520))
}

fn evidence_packet_answer_units(response_tools: &[Value], limit: usize) -> Vec<String> {
    let mut units = Vec::<String>::new();
    let mut seen = std::collections::HashSet::<String>::new();
    let limit = limit.clamp(1, 8);
    for tool in response_tools {
        for key in ["evidence_pack", "evidence_refs", "evidence_pack_candidates"] {
            for row in tool_hidden_array(tool, key) {
                let Some(unit) = evidence_packet_answer_unit(&row) else {
                    continue;
                };
                let dedupe_key = unit.to_ascii_lowercase();
                if seen.insert(dedupe_key) {
                    units.push(unit);
                }
                if units.len() >= limit {
                    return units;
                }
            }
        }
    }
    units
}

