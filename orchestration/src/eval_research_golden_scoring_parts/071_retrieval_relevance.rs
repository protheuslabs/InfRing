
fn evidence_prompt_relevance(payload: &Value, normalized_prompt: &str) -> Value {
    let evidence_texts = evidence_relevance_texts(payload);
    evidence_prompt_relevance_from_texts(
        normalized_prompt,
        evidence_texts,
        "Checks whether at least one evidence item overlaps the user's durable topic terms, so unrelated source rows do not count as usable research evidence.",
        false,
    )
}

fn evidence_prompt_relevance_from_texts(
    normalized_prompt: &str,
    evidence_texts: Vec<String>,
    note: &str,
    strict_claim_overlap: bool,
) -> Value {
    let prompt_terms = research_prompt_topic_terms(normalized_prompt, 12);
    if strict_claim_overlap
        && prompt_terms.is_empty()
        && strict_broad_current_event_prompt(normalized_prompt)
        && !evidence_texts.is_empty()
    {
        let relevant_evidence_count = evidence_texts
            .iter()
            .filter(|text| evidence_text_has_concrete_current_event_signal(text))
            .count() as u64;
        return json!({
            "schema_version": 1,
            "topic_relevant_evidence": relevant_evidence_count > 0,
            "prompt_terms": prompt_terms,
            "evidence_text_count": evidence_texts.len(),
            "relevant_evidence_count": relevant_evidence_count,
            "min_overlap_terms": 0,
            "broad_current_event_claim_check": true,
            "note": "Broad current-event prompts may have no durable topic terms, so strict evidence-claim relevance falls back to concrete event/action signal instead of accepting evergreen source metadata."
        });
    }
    if prompt_terms.len() < 2 || evidence_texts.is_empty() {
        return json!({
            "schema_version": 1,
            "topic_relevant_evidence": true,
            "prompt_terms": prompt_terms,
            "evidence_text_count": evidence_texts.len(),
            "relevant_evidence_count": 0,
            "min_overlap_terms": 0,
            "note": "Prompt relevance was not enforced because the prompt had too few durable topic terms or no evidence text was available."
        });
    }
    let min_overlap = if strict_claim_overlap && prompt_terms.len() >= 2 {
        2
    } else {
        1
    };
    let relevant_evidence_count = evidence_texts
        .iter()
        .filter(|text| prompt_term_overlap_count(&prompt_terms, text) >= min_overlap)
        .count() as u64;
    json!({
        "schema_version": 1,
        "topic_relevant_evidence": relevant_evidence_count > 0,
        "prompt_terms": prompt_terms,
        "evidence_text_count": evidence_texts.len(),
        "relevant_evidence_count": relevant_evidence_count,
        "min_overlap_terms": min_overlap,
        "note": note
    })
}

fn strict_broad_current_event_prompt(normalized_prompt: &str) -> bool {
    contains_any(
        normalized_prompt,
        &[
            " news",
            "headline",
            "headlines",
            "current event",
            "current events",
            "world news",
            "this week",
            "today",
        ],
    )
}

fn evidence_text_has_concrete_current_event_signal(normalized_text: &str) -> bool {
    if contains_any(
        normalized_text,
        &[
            "news sources are everywhere",
            "latest news photos videos",
            "current news latest news",
            "top headlines on",
            "section index",
            "homepage",
            "landing page",
        ],
    ) {
        return false;
    }
    contains_any(
        normalized_text,
        &[
            "announced",
            "approved",
            "attacked",
            "canceled",
            "cancelled",
            "charged",
            "died",
            "killed",
            "launched",
            "passed",
            "released",
            "reported",
            "responded",
            "recaptured",
            "resigned",
            "raised fears",
            "shift",
            "sued",
            "voted",
            "warned",
        ],
    )
}

fn direct_evidence_claim_count(payload: &Value) -> u64 {
    payload
        .get("evidence_claims")
        .and_then(Value::as_array)
        .map(|rows| rows.len() as u64)
        .unwrap_or(0)
}

fn direct_evidence_claim_texts(payload: &Value) -> Vec<String> {
    let mut out = payload
        .get("evidence_claims")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    let mut parts = Vec::<String>::new();
                    for key in [
                        "claim",
                        "support_snippet",
                        "source_title",
                        "title",
                        "source_domain",
                    ] {
                        if let Some(raw) = row.get(key).and_then(Value::as_str) {
                            let cleaned = clean_text(raw, 700);
                            if !cleaned.is_empty() {
                                parts.push(cleaned);
                            }
                        }
                    }
                    if let Some(source_ref) = row.get("source_ref").and_then(Value::as_object) {
                        for key in ["title", "source_domain"] {
                            if let Some(raw) = source_ref.get(key).and_then(Value::as_str) {
                                let cleaned = clean_text(raw, 700);
                                if !cleaned.is_empty() {
                                    parts.push(cleaned);
                                }
                            }
                        }
                    }
                    if parts.is_empty() {
                        None
                    } else {
                        Some(normalize_for_compare(&parts.join(" ")))
                    }
                })
                .filter(|text| text.split_whitespace().count() >= 3)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    out.sort();
    out.dedup();
    out
}

fn direct_tool_quality_flags(payload: &Value) -> Vec<String> {
    let mut flags = Vec::<String>::new();
    for pointer in ["/tool_result_quality/flags", "/evidence_pack_quality/flags"] {
        if let Some(rows) = payload.pointer(pointer).and_then(Value::as_array) {
            for row in rows {
                if let Some(raw) = row.as_str() {
                    let flag = normalize_for_compare(raw);
                    if !flag.is_empty() {
                        flags.push(flag);
                    }
                }
            }
        }
    }
    flags.sort();
    flags.dedup();
    flags
}

fn evidence_relevance_texts(payload: &Value) -> Vec<String> {
    let mut out = Vec::<String>::new();
    for row in selected_tool_contexts(payload) {
        collect_evidence_relevance_texts(row, 0, &mut out);
    }
    out.sort();
    out.dedup();
    out
}

fn collect_evidence_relevance_texts(value: &Value, depth: usize, out: &mut Vec<String>) {
    if depth > 7 || out.len() >= 80 {
        return;
    }
    match value {
        Value::Array(rows) => {
            for row in rows {
                collect_evidence_relevance_texts(row, depth + 1, out);
            }
        }
        Value::Object(map) => {
            let mut doc_parts = Vec::<String>::new();
            for key in [
                "title",
                "source_domain",
                "snippet",
                "summary",
                "content",
                "markdown",
                "text",
                "body",
                "description",
                "abstract",
                "claim_hints",
                "claims",
                "extracted_claims",
                "claim_candidates",
                "key_findings",
                "findings",
            ] {
                if let Some(child) = map.get(key) {
                    collect_relevance_doc_parts(child, depth + 1, &mut doc_parts);
                }
            }
            if !doc_parts.is_empty() {
                let combined = normalize_for_compare(&doc_parts.join(" "));
                if combined.split_whitespace().count() >= 3 {
                    out.push(combined);
                }
            }
            for key in [
                "evidence",
                "evidence_refs",
                "evidence_pack",
                "evidence_pack_candidates",
                "sources",
                "citations",
                "search_results",
                "provider_results",
            ] {
                if let Some(child) = map.get(key) {
                    collect_evidence_relevance_texts(child, depth + 1, out);
                }
            }
        }
        Value::String(raw) => {
            let cleaned = clean_text(raw, 1_000);
            if cleaned.split_whitespace().count() >= 3 {
                out.push(normalize_for_compare(&cleaned));
            }
        }
        _ => {}
    }
}

fn collect_relevance_doc_parts(value: &Value, depth: usize, out: &mut Vec<String>) {
    if depth > 7 || out.len() >= 32 {
        return;
    }
    match value {
        Value::Array(rows) => {
            for row in rows {
                collect_relevance_doc_parts(row, depth + 1, out);
            }
        }
        Value::Object(map) => {
            for key in [
                "text",
                "snippet",
                "summary",
                "title",
                "content",
                "markdown",
                "body",
                "description",
                "abstract",
            ] {
                if let Some(child) = map.get(key) {
                    collect_relevance_doc_parts(child, depth + 1, out);
                }
            }
        }
        Value::String(raw) => {
            let cleaned = clean_text(raw, 500);
            if cleaned.split_whitespace().count() >= 2 {
                out.push(cleaned);
            }
        }
        _ => {}
    }
}

fn research_prompt_topic_terms(normalized_prompt: &str, limit: usize) -> Vec<String> {
    let mut terms = Vec::<String>::new();
    for token in normalized_prompt.split_whitespace() {
        let token = normalize_research_token(token);
        if token.len() < 3 && token != "ai" {
            continue;
        }
        if research_prompt_stop_term(&token) {
            continue;
        }
        let stem = research_term_stem(&token);
        if stem.len() < 3 && stem != "ai" {
            continue;
        }
        if research_prompt_stop_term(&stem) {
            continue;
        }
        if !terms.iter().any(|existing| existing == &stem) {
            terms.push(stem);
        }
        if terms.len() >= limit {
            break;
        }
    }
    terms
}

fn normalize_research_token(token: &str) -> String {
    token
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase()
}

fn research_prompt_stop_term(token: &str) -> bool {
    matches!(
        token,
        "about"
            | "after"
            | "against"
            | "also"
            | "answer"
            | "anything"
            | "around"
            | "broad"
            | "broadly"
            | "and"
            | "are"
            | "before"
            | "best"
            | "biggest"
            | "brief"
            | "briefly"
            | "but"
            | "buying"
            | "between"
            | "blindly"
            | "browse"
            | "caveat"
            | "caveats"
            | "cite"
            | "cited"
            | "citable"
            | "compare"
            | "concise"
            | "citation"
            | "citations"
            | "current"
            | "currently"
            | "defensible"
            | "doc"
            | "docs"
            | "documentation"
            | "does"
            | "different"
            | "explain"
            | "example"
            | "examples"
            | "find"
            | "first"
            | "field"
            | "fields"
            | "for"
            | "from"
            | "global"
            | "give"
            | "group"
            | "grouped"
            | "how"
            | "headline"
            | "headlines"
            | "important"
            | "into"
            | "landscape"
            | "latest"
            | "look"
            | "looking"
            | "make"
            | "major"
            | "marketing"
            | "more"
            | "most"
            | "month"
            | "monthly"
            | "need"
            | "news"
            | "not"
            | "official"
            | "overview"
            | "page"
            | "pages"
            | "primary"
            | "practical"
            | "prioritize"
            | "prioritise"
            | "research"
            | "recommend"
            | "recommendation"
            | "recommendations"
            | "recent"
            | "report"
            | "reported"
            | "reporting"
            | "release"
            | "releases"
            | "result"
            | "results"
            | "right"
            | "search"
            | "some"
            | "far"
            | "source"
            | "sources"
            | "stories"
            | "story"
            | "summarize"
            | "tell"
            | "that"
            | "the"
            | "their"
            | "there"
            | "these"
            | "this"
            | "theme"
            | "themes"
            | "today"
            | "tomorrow"
            | "trust"
            | "update"
            | "using"
            | "web"
            | "what"
            | "when"
            | "where"
            | "which"
            | "why"
            | "while"
            | "with"
            | "would"
            | "week"
            | "weekly"
            | "world"
            | "year"
            | "years"
            | "yesterday"
            | "january"
            | "february"
            | "march"
            | "april"
            | "may"
            | "june"
            | "july"
            | "august"
            | "september"
            | "october"
            | "november"
            | "december"
    )
}

fn research_term_stem(token: &str) -> String {
    let mut value = normalize_research_token(token);
    for suffix in ["ing", "ed", "es", "s"] {
        if value.len() > suffix.len() + 3 && value.ends_with(suffix) {
            value.truncate(value.len() - suffix.len());
            break;
        }
    }
    value
}

fn prompt_term_overlap_count(prompt_terms: &[String], normalized_text: &str) -> usize {
    let text_terms = normalized_text
        .split_whitespace()
        .map(research_term_stem)
        .filter(|term| !term.is_empty())
        .collect::<Vec<_>>();
    prompt_terms
        .iter()
        .filter(|term| text_terms.iter().any(|text_term| text_term == *term))
        .count()
}
