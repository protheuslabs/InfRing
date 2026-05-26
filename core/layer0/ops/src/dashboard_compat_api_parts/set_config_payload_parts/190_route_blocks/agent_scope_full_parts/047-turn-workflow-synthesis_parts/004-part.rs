fn workflow_compact_source_refs(response_tools: &[Value], limit: usize) -> Vec<Value> {
    let mut refs = Vec::<Value>::new();
    let mut seen = std::collections::HashSet::<String>::new();
    let limit = limit.clamp(1, 12);
    for tool in response_tools {
        for key in ["evidence_pack", "evidence_refs", "evidence_pack_candidates"] {
            for row in tool_hidden_array(tool, key) {
                if key == "evidence_pack" && !evidence_packet_counts_as_usable(&row) {
                    continue;
                }
                let title =
                    evidence_packet_text_field(&row, &["title", "source_title", "source_ref"], 160);
                let locator = evidence_packet_text_field(&row, &["locator", "url", "link"], 240);
                let source_domain =
                    evidence_packet_text_field(&row, &["source_domain", "domain"], 120);
                let source_kind =
                    evidence_packet_text_field(&row, &["source_kind", "kind", "type"], 80);
                if locator.starts_with("tool:no-results") || locator.starts_with("tool:low-signal")
                {
                    continue;
                }
                if title.is_empty() && locator.is_empty() && source_domain.is_empty() {
                    continue;
                }
                let dedupe_key = format!(
                    "{}|{}|{}",
                    title.to_ascii_lowercase(),
                    locator.to_ascii_lowercase(),
                    source_domain.to_ascii_lowercase()
                );
                if !seen.insert(dedupe_key) {
                    continue;
                }
                refs.push(json!({
                    "title": title,
                    "locator": locator,
                    "source_domain": source_domain,
                    "source_kind": source_kind,
                }));
                if refs.len() >= limit {
                    return refs;
                }
            }
        }
    }
    refs
}

fn persist_workflow_compact_source_refs(workflow: &mut Value, response_tools: &[Value]) {
    let source_refs = workflow_compact_source_refs(response_tools, 6);
    if source_refs.is_empty() {
        return;
    }
    workflow["source_refs"] = Value::Array(source_refs.clone());
    workflow["response_workflow"]["final_llm_response"]["source_refs"] =
        Value::Array(source_refs.clone());
    workflow["response_finalization"]["final_response"]["source_refs"] = Value::Array(source_refs);
}

fn workflow_push_evidence_alignment_text(
    out: &mut Vec<String>,
    seen: &mut std::collections::HashSet<String>,
    raw: &str,
    max_len: usize,
) {
    let cleaned = clean_text(raw, max_len);
    if cleaned.is_empty() {
        return;
    }
    let normalized = normalize_coverage_lane_text(&cleaned);
    if normalized.split_whitespace().count() < 2 {
        return;
    }
    if seen.insert(normalized.clone()) {
        out.push(normalized);
    }
}

fn workflow_collect_evidence_alignment_array_texts(
    row: &Value,
    key: &str,
    out: &mut Vec<String>,
    seen: &mut std::collections::HashSet<String>,
) {
    for item in row.get(key).and_then(Value::as_array).into_iter().flatten() {
        let text = clean_text(item.as_str().unwrap_or(""), 420);
        if !text.is_empty() {
            workflow_push_evidence_alignment_text(out, seen, &text, 420);
        }
    }
}

fn workflow_evidence_alignment_texts(response_tools: &[Value]) -> Vec<String> {
    let mut texts = Vec::<String>::new();
    let mut seen = std::collections::HashSet::<String>::new();
    for tool in response_tools {
        for field in ["result", "summary", "text"] {
            let text = clean_text(tool.get(field).and_then(Value::as_str).unwrap_or(""), 1_200);
            if !text.is_empty() {
                workflow_push_evidence_alignment_text(&mut texts, &mut seen, &text, 1_200);
            }
        }
        if let Some(quality) = tool_result_quality_object(tool) {
            for candidate in quality
                .get("candidate_quality")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                for field in ["title", "snippet_preview", "snippet", "summary"] {
                    let text = clean_text(
                        candidate.get(field).and_then(Value::as_str).unwrap_or(""),
                        420,
                    );
                    if !text.is_empty() {
                        workflow_push_evidence_alignment_text(
                            &mut texts, &mut seen, &text, 420,
                        );
                    }
                }
            }
        }
        for key in ["evidence_pack", "evidence_refs", "evidence_pack_candidates"] {
            for row in tool_hidden_array(tool, key) {
                workflow_collect_evidence_alignment_array_texts(
                    &row,
                    "claim_hints",
                    &mut texts,
                    &mut seen,
                );
                workflow_collect_evidence_alignment_array_texts(
                    &row,
                    "evidence_claims",
                    &mut texts,
                    &mut seen,
                );
                for field in [
                    "claim",
                    "finding",
                    "summary",
                    "relevant_extract",
                    "support_snippet",
                    "snippet",
                    "content",
                    "title",
                    "source_title",
                    "source_ref",
                    "why_relevant_to_query",
                ] {
                    let text = evidence_packet_text_field(&row, &[field], 420);
                    if !text.is_empty() {
                        workflow_push_evidence_alignment_text(&mut texts, &mut seen, &text, 420);
                    }
                }
                if let Some(unit) = evidence_packet_answer_unit(&row) {
                    workflow_push_evidence_alignment_text(&mut texts, &mut seen, &unit, 520);
                }
            }
        }
    }
    texts
}

fn workflow_strip_markdown_link_targets(raw: &str) -> String {
    let mut out = String::new();
    let mut rest = raw;
    while let Some(start) = rest.find("](") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        if let Some(end) = after.find(')') {
            rest = &after[end + 1..];
        } else {
            rest = after;
            break;
        }
    }
    out.push_str(rest);
    out
}

fn workflow_push_answer_text_unit(units: &mut Vec<String>, raw: &str) {
    let unit = clean_text(
        raw.trim_matches(|ch: char| ch.is_ascii_whitespace() || ch == '-' || ch == '*'),
        700,
    );
    if unit.split_whitespace().count() >= 5
        && !unit.ends_with(':')
        && !units.iter().any(|existing| existing == &unit)
    {
        units.push(unit);
    }
}

fn workflow_answer_text_units(response_text: &str) -> Vec<String> {
    let mut units = Vec::<String>::new();
    for line in response_text.lines() {
        let line = line
            .trim()
            .trim_start_matches(|ch: char| {
                ch.is_ascii_whitespace() || ch == '-' || ch == '*' || ch == ':' || ch == ')'
            })
            .trim();
        if line.is_empty() {
            continue;
        }
        let line = workflow_strip_markdown_link_targets(line);
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut start = 0;
        for (idx, ch) in line.char_indices() {
            if matches!(ch, '.' | '!' | '?') {
                workflow_push_answer_text_unit(&mut units, &line[start..idx + ch.len_utf8()]);
                start = idx + ch.len_utf8();
            }
        }
        if start < line.len() {
            workflow_push_answer_text_unit(&mut units, &line[start..]);
        }
        if units.len() >= 18 {
            break;
        }
    }
    units
}

fn workflow_normalize_specific_term(token: &str) -> String {
    token
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase()
}

fn workflow_specific_term_stem(token: &str) -> String {
    let mut value = workflow_normalize_specific_term(token);
    for suffix in ["ing", "ed", "es", "s"] {
        if value.len() > suffix.len() + 3 && value.ends_with(suffix) {
            value.truncate(value.len() - suffix.len());
            break;
        }
    }
    value
}

fn workflow_specific_token_looks_domain_like(token: &str) -> bool {
    let host = token
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .trim_start_matches("www.")
        .split('/')
        .next()
        .unwrap_or("");
    let labels = host
        .split('.')
        .filter(|label| !label.is_empty())
        .collect::<Vec<_>>();
    if labels.len() < 2 {
        return false;
    }
    let tld = labels.last().copied().unwrap_or("");
    (2..=24).contains(&tld.len())
        && tld.chars().all(|ch| ch.is_ascii_alphabetic())
        && labels
            .iter()
            .any(|label| label.chars().any(|ch| ch.is_ascii_alphabetic()))
}

fn workflow_specific_stop_term(token: &str) -> bool {
    matches!(
        token,
        "about"
            | "according"
            | "across"
            | "also"
            | "answer"
            | "breakthrough"
            | "breakthroughs"
            | "development"
            | "developments"
            | "evidence"
            | "finding"
            | "findings"
            | "here"
            | "notable"
            | "published"
            | "reported"
            | "research"
            | "science"
            | "source"
            | "sources"
            | "support"
            | "supported"
            | "using"
            | "with"
            | "year"
    )
}

fn workflow_answer_unit_specific_terms(unit: &str) -> Vec<String> {
    let mut seen = std::collections::BTreeSet::<String>::new();
    let mut terms = Vec::<String>::new();
    for raw in unit.split_whitespace() {
        let mut cleaned = raw.trim_matches(|ch: char| {
            !ch.is_ascii_alphanumeric() && ch != '-' && ch != '.' && ch != '/'
        });
        cleaned = cleaned
            .trim_end_matches("'s")
            .trim_end_matches("'S")
            .trim_end_matches("’s")
            .trim_end_matches("’S");
        if cleaned.is_empty() {
            continue;
        }
        let normalized = workflow_normalize_specific_term(cleaned);
        if normalized.len() < 3
            && normalized != "ai"
            && !normalized.chars().any(|ch| ch.is_ascii_digit())
        {
            continue;
        }
        if workflow_specific_stop_term(&normalized) {
            continue;
        }
        let letters = cleaned
            .chars()
            .filter(|ch| ch.is_ascii_alphabetic())
            .collect::<Vec<_>>();
        let uppercase_letters = letters.iter().filter(|ch| ch.is_ascii_uppercase()).count();
        let has_digit = cleaned.chars().any(|ch| ch.is_ascii_digit());
        let is_acronym =
            letters.len() >= 2 && uppercase_letters >= 2 && uppercase_letters * 2 >= letters.len();
        let has_internal_capital = letters.iter().skip(1).any(|ch| ch.is_ascii_uppercase());
        let is_capitalized = cleaned
            .chars()
            .next()
            .map(|ch| ch.is_ascii_uppercase())
            .unwrap_or(false);
        let domain_like = workflow_specific_token_looks_domain_like(cleaned);
        let specific = has_digit
            || is_acronym
            || has_internal_capital
            || domain_like
            || (is_capitalized && normalized.len() >= 3);
        if specific && seen.insert(normalized.clone()) {
            terms.push(normalized);
        }
        if terms.len() >= 12 {
            break;
        }
    }
    terms
}

fn workflow_answer_unit_precision_terms(unit: &str) -> Vec<String> {
    let mut seen = std::collections::BTreeSet::<String>::new();
    let mut terms = Vec::<String>::new();
    for raw in unit.split_whitespace() {
        let mut cleaned = raw.trim_matches(|ch: char| {
            !ch.is_ascii_alphanumeric() && ch != '-' && ch != '.' && ch != '/'
        });
        cleaned = cleaned
            .trim_end_matches("'s")
            .trim_end_matches("'S")
            .trim_end_matches("’s")
            .trim_end_matches("’S");
        if cleaned.is_empty() {
            continue;
        }
        let normalized = workflow_normalize_specific_term(cleaned);
        if normalized.len() < 2 && !normalized.chars().any(|ch| ch.is_ascii_digit()) {
            continue;
        }
        if workflow_specific_stop_term(&normalized) {
            continue;
        }
        let letters = cleaned
            .chars()
            .filter(|ch| ch.is_ascii_alphabetic())
            .collect::<Vec<_>>();
        let uppercase_letters = letters.iter().filter(|ch| ch.is_ascii_uppercase()).count();
        let has_digit = cleaned.chars().any(|ch| ch.is_ascii_digit());
        let is_acronym =
            letters.len() >= 2 && uppercase_letters >= 2 && uppercase_letters * 2 >= letters.len();
        let domain_like = workflow_specific_token_looks_domain_like(cleaned);
        let precision_marker = has_digit || is_acronym || domain_like;
        if precision_marker && seen.insert(normalized.clone()) {
            terms.push(normalized);
        }
        if terms.len() >= 12 {
            break;
        }
    }
    terms
}

fn workflow_answer_unit_is_hedged_or_gap(normalized_unit: &str) -> bool {
    let padded = format!(" {normalized_unit} ");
    workflow_answer_unit_contains_any(
        &padded,
        &[
            " may ",
            " might ",
            " could ",
            " appears ",
            " suggests ",
            " uncertain",
            " not clear",
            " not enough",
            " does not confirm",
            " doesn't confirm",
            " current evidence does not",
            " evidence does not",
            " not retrieved",
            " no source-backed",
            " limited evidence",
            " available evidence",
            " weakly covered",
            " weak coverage",
            " coverage gap",
            " coverage gaps",
            " verify ",
            " need to verify ",
            " tentative",
            " unsettled",
            " sparse result",
            " critical limitation",
            " what remains ",
            " unknown",
            " unverified",
            " inference",
            " partial",
        ],
    )
}

fn workflow_answer_unit_has_high_commitment_claim(normalized_unit: &str) -> bool {
    workflow_answer_unit_contains_any(
        normalized_unit,
        &[
            " launched ",
            " released ",
            " announced ",
            " approved ",
            " won ",
            " raised ",
            " reported ",
            " published ",
            " disproved ",
            " discovered ",
            " identified ",
        ],
    )
}

fn workflow_evidence_texts_support_term(evidence_texts: &[String], term: &str) -> bool {
    if term.is_empty() {
        return true;
    }
    let stem = workflow_specific_term_stem(term);
    evidence_texts.iter().any(|text| {
        (term.len() > 2 && text.contains(term))
            || text.split_whitespace().any(|token| {
                let normalized = workflow_normalize_specific_term(token);
                normalized == term
                    || (!stem.is_empty() && workflow_specific_term_stem(&normalized) == stem)
            })
    })
}

fn workflow_answer_unit_unsupported_is_significant(
    normalized_unit: &str,
    supported_terms: &[String],
    unsupported_terms: &[String],
) -> bool {
    if unsupported_terms.is_empty() {
        return false;
    }
    if supported_terms.is_empty() {
        return true;
    }
    if workflow_answer_unit_has_high_commitment_claim(normalized_unit) {
        return true;
    }
    let total_terms = supported_terms.len() + unsupported_terms.len();
    unsupported_terms.len() >= 2 && unsupported_terms.len() * 2 >= total_terms.max(1)
}

fn response_has_answer_unit_precision_traceability_violation(
    response_text: &str,
    response_tools: &[Value],
) -> bool {
    let evidence_texts = workflow_evidence_alignment_texts(response_tools);
    if evidence_texts.is_empty() {
        return false;
    }
    for unit in workflow_answer_text_units(response_text).iter().take(18) {
        let normalized_unit = normalize_coverage_lane_text(unit);
        if workflow_answer_unit_is_hedged_or_gap(&normalized_unit) {
            continue;
        }
        let precision_terms = workflow_answer_unit_precision_terms(unit);
        if precision_terms.is_empty() {
            continue;
        }
        let unsupported_precision = precision_terms
            .into_iter()
            .filter(|term| !workflow_evidence_texts_support_term(&evidence_texts, term))
            .collect::<Vec<_>>();
        if unsupported_precision.is_empty() {
            continue;
        }
        if workflow_answer_unit_has_high_commitment_claim(&normalized_unit)
            || unsupported_precision.len() >= 2
        {
            return true;
        }
    }
    false
}
