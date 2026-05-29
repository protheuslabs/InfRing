fn answer_unit_evidence_alignment(
    payload: &Value,
    response_text: &str,
    retrieval_quality: &Value,
) -> Value {
    let usable_evidence = retrieval_quality
        .get("usable_evidence")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let evidence_texts = evidence_alignment_texts(payload);
    let scope_texts = answer_alignment_scope_texts(payload);
    let units = answer_text_units(response_text);
    let mut checked_units = Vec::<Value>::new();
    let mut unsupported_units = Vec::<Value>::new();
    let mut high_specificity_units = 0_u64;
    let mut total_terms = 0_u64;
    let mut supported_terms_total = 0_u64;

    for unit in units.iter().take(18) {
        let terms = answer_unit_specific_terms(unit);
        if terms.is_empty() {
            continue;
        }
        high_specificity_units += 1;
        let normalized_unit = normalize_for_compare(unit);
        let hedged = answer_unit_is_hedged_or_gap(&normalized_unit);
        let mut supported_terms = Vec::<String>::new();
        let mut scope_supported_terms = Vec::<String>::new();
        let mut unsupported_terms = Vec::<String>::new();
        for term in terms {
            total_terms += 1;
            if evidence_texts_support_term(&evidence_texts, &term) {
                supported_terms_total += 1;
                supported_terms.push(term);
            } else if evidence_texts_support_term(&scope_texts, &term) {
                supported_terms_total += 1;
                scope_supported_terms.push(term);
            } else {
                unsupported_terms.push(term);
            }
        }
        let unsupported_is_significant = answer_unit_unsupported_is_significant(
            &normalized_unit,
            &supported_terms,
            &scope_supported_terms,
            &unsupported_terms,
        );
        let unit_row = json!({
            "unit_preview": clean_text(unit, 300),
            "hedged_or_gap_labeled": hedged,
            "supported_terms": supported_terms,
            "scope_supported_terms": scope_supported_terms,
            "unsupported_terms": unsupported_terms,
            "unsupported_is_significant": unsupported_is_significant,
        });
        if !unsupported_terms.is_empty() && !hedged && unsupported_is_significant {
            unsupported_units.push(unit_row.clone());
        }
        if checked_units.len() < 12 {
            checked_units.push(unit_row);
        }
    }

    let evaluated = !evidence_texts.is_empty() && high_specificity_units > 0;
    let support_rate = ratio(supported_terms_total, total_terms);
    let blockers = if evaluated && !unsupported_units.is_empty() {
        vec!["unsupported_answer_units".to_string()]
    } else {
        Vec::new()
    };
    json!({
        "schema_version": 1,
        "lane_id": "answer_unit_evidence_alignment_v1",
        "pass": blockers.is_empty(),
        "evaluated": evaluated,
        "usable_evidence": usable_evidence,
        "evidence_text_count": evidence_texts.len() as u64,
        "scope_text_count": scope_texts.len() as u64,
        "units_checked": checked_units.len() as u64,
        "high_specificity_units": high_specificity_units,
        "term_support_rate": support_rate,
        "unsupported_unit_count": unsupported_units.len() as u64,
        "checked_units": checked_units,
        "unsupported_units": unsupported_units,
        "blockers": blockers,
        "top_blocker": blockers.first().cloned().unwrap_or_else(|| "none".to_string()),
        "note": "Soft generic smoke lane. It extracts high-specificity answer units from the final answer and checks whether their concrete terms appear in retrieved evidence/citation artifacts; hedged uncertainty and evidence-gap statements are allowed. Retrieval quality is reported separately; weak retrieval does not permit unsupported concrete answer units."
    })
}

fn answer_unit_usefulness_for_prompt(
    normalized_prompt: &str,
    response_text: &str,
    retrieval_quality: &Value,
) -> Value {
    let usable_evidence = retrieval_quality
        .get("usable_evidence")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let prompt_terms = answer_unit_usefulness_prompt_terms(normalized_prompt);
    let prompt_asks_process = prompt_asks_for_process_or_schedule(normalized_prompt);
    let mut checked_units = Vec::<Value>::new();
    let mut low_usefulness_units = Vec::<Value>::new();
    let mut process_metadata_units = 0_u64;
    let mut substantive_units = 0_u64;
    let mut direct_useful_units = 0_u64;

    for unit in answer_text_units(response_text).iter().take(18) {
        let normalized_unit = normalize_for_compare(unit);
        if answer_unit_is_hedged_or_gap(&normalized_unit) {
            continue;
        }
        if normalized_unit.split_whitespace().count() < 5 {
            continue;
        }
        substantive_units += 1;
        let overlap = answer_unit_prompt_overlap(&prompt_terms, &normalized_unit);
        let process_metadata =
            !prompt_asks_process && answer_unit_is_process_or_metadata_fact(&normalized_unit);
        if process_metadata {
            process_metadata_units += 1;
        }
        let direct_useful = !process_metadata
            && (prompt_terms.is_empty()
                || overlap > 0
                || answer_unit_has_high_commitment_claim(&normalized_unit)
                || answer_unit_has_substantive_development_signal(&normalized_unit));
        if direct_useful {
            direct_useful_units += 1;
        }
        let row = json!({
            "unit_preview": clean_text(unit, 300),
            "prompt_overlap": overlap as u64,
            "process_or_metadata_fact": process_metadata,
            "directly_useful_for_prompt": direct_useful,
        });
        if (!direct_useful || process_metadata) && checked_units.len() < 12 {
            low_usefulness_units.push(row.clone());
        }
        if checked_units.len() < 12 {
            checked_units.push(row);
        }
    }

    let evaluated = substantive_units > 0;
    let process_overrepresented = process_metadata_units >= 2
        || (process_metadata_units > 0 && process_metadata_units * 2 >= substantive_units.max(1));
    let direct_answer_units_missing = evaluated && direct_useful_units == 0;
    let mut blockers = Vec::<String>::new();
    if process_overrepresented {
        blockers.push("process_metadata_units_overrepresented".to_string());
    }
    if direct_answer_units_missing {
        blockers.push("direct_answer_units_missing".to_string());
    }
    json!({
        "schema_version": 1,
        "lane_id": "answer_unit_prompt_usefulness_v1",
        "pass": blockers.is_empty(),
        "evaluated": evaluated,
        "usable_evidence": usable_evidence,
        "prompt_terms": prompt_terms,
        "prompt_asks_process_or_schedule": prompt_asks_process,
        "substantive_units": substantive_units,
        "direct_useful_units": direct_useful_units,
        "process_metadata_units": process_metadata_units,
        "checked_units": checked_units,
        "low_usefulness_units": low_usefulness_units,
        "blockers": blockers,
        "top_blocker": blockers.first().cloned().unwrap_or_else(|| "none".to_string()),
        "note": "Generic prompt-usefulness smoke lane. Evidence-backed facts only count as answer units when they directly answer the user's requested semantic object; administrative or source-metadata facts are only useful when the prompt asks for them."
    })
}

fn answer_unit_usefulness_prompt_terms(normalized_prompt: &str) -> Vec<String> {
    research_prompt_topic_terms(normalized_prompt, 12)
        .into_iter()
        .filter(|term| !answer_unit_prompt_term_is_temporal_or_generic(term))
        .collect()
}

fn answer_unit_prompt_term_is_temporal_or_generic(term: &str) -> bool {
    if term.chars().all(|ch| ch.is_ascii_digit()) {
        return true;
    }
    matches!(
        term,
        "today"
            | "week"
            | "month"
            | "year"
            | "current"
            | "latest"
            | "recent"
            | "update"
            | "news"
    )
}

fn answer_unit_prompt_overlap(prompt_terms: &[String], normalized_unit: &str) -> usize {
    prompt_terms
        .iter()
        .filter(|term| normalized_unit.contains(term.as_str()))
        .count()
}

fn prompt_asks_for_process_or_schedule(normalized_prompt: &str) -> bool {
    contains_any(
        normalized_prompt,
        &[
            " schedule",
            " scheduled",
            " deadline",
            " nomination",
            " nominations",
            " application",
            " registration",
            " calendar",
            " when ",
            " date",
            " dates",
            " announce",
            " announcement",
            " announcements",
        ],
    )
}

fn answer_unit_is_process_or_metadata_fact(normalized_unit: &str) -> bool {
    answer_unit_is_followup_search_suggestion(normalized_unit)
        || contains_any(
            normalized_unit,
            &[
                "here s what i found",
                "heres what i found",
                "web search",
                "web search:",
                "web search returned",
                "search returned",
                "search surfaced",
                "from web retrieval",
                "announcements are scheduled",
                "announcement is scheduled",
                "is scheduled for",
                "are scheduled for",
                "nominations closed",
                "nomination period",
                "nominations are open",
                "deadline",
                "registration",
                "application window",
                "calendar",
                "press release",
                "coverage state",
                "usable evidence is present for",
                "coverage gaps still matter",
                "web result from",
                "source: web result",
            ],
        )
}

fn answer_unit_has_substantive_development_signal(normalized_unit: &str) -> bool {
    contains_any(
        normalized_unit,
        &[
            " first ",
            " new ",
            " discovered",
            " discovery",
            " breakthrough",
            " milestone",
            " improved",
            " launched",
            " released",
            " approved",
            " observed",
            " demonstrated",
            " achieved",
        ],
    )
}

fn evidence_alignment_texts(payload: &Value) -> Vec<String> {
    let mut texts = evidence_relevance_texts(payload);
    let artifacts = citation_artifact_summary(payload);
    if let Some(items) = artifacts.get("items").and_then(Value::as_array) {
        for item in items {
            let parts = [
                str_at(item, &["title"], ""),
                str_at(item, &["locator"], ""),
                str_at(item, &["source_domain"], ""),
                str_at(item, &["snippet"], ""),
            ]
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>();
            if parts.is_empty() {
                continue;
            }
            let combined = normalize_for_compare(&parts.join(" "));
            if combined.split_whitespace().count() >= 2 {
                texts.push(combined);
            }
        }
    }
    texts.sort();
    texts.dedup();
    texts
}

fn answer_alignment_scope_texts(payload: &Value) -> Vec<String> {
    let mut texts = Vec::<String>::new();
    for path in [
        &["pending_tool_request", "input"][..],
        &["response_workflow", "pending_tool_request", "input"][..],
        &[
            "response_workflow",
            "manual_toolbox_pending_tool_request",
            "input",
        ][..],
        &["response_finalization", "pending_tool_request", "input"][..],
        &[
            "response_finalization",
            "tool_completion",
            "pending_tool_request",
            "input",
        ][..],
    ] {
        let mut cursor = payload;
        let mut found = true;
        for segment in path {
            if let Some(next) = cursor.get(*segment) {
                cursor = next;
            } else {
                found = false;
                break;
            }
        }
        if found {
            collect_answer_alignment_scope_texts(cursor, &mut texts);
        }
    }
    texts.sort();
    texts.dedup();
    texts
}

fn collect_answer_alignment_scope_texts(value: &Value, texts: &mut Vec<String>) {
    match value {
        Value::String(raw) => {
            let normalized = normalize_for_compare(raw);
            if normalized.split_whitespace().count() >= 1 {
                texts.push(normalized);
            }
        }
        Value::Array(rows) => {
            for row in rows {
                collect_answer_alignment_scope_texts(row, texts);
            }
        }
        Value::Object(map) => {
            for key in [
                "query",
                "keywords",
                "aliases",
                "entities",
                "facets",
                "required_coverage",
                "negative_terms",
            ] {
                if let Some(child) = map.get(key) {
                    collect_answer_alignment_scope_texts(child, texts);
                }
            }
        }
        _ => {}
    }
}

fn answer_text_units(response_text: &str) -> Vec<String> {
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
        let line = strip_markdown_link_targets(line);
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut start = 0;
        for (idx, ch) in line.char_indices() {
            if matches!(ch, '.' | '!' | '?') {
                if ch == '.' {
                    let prev = line[..idx]
                        .chars()
                        .rev()
                        .find(|candidate| !candidate.is_ascii_whitespace());
                    let next = line[idx + ch.len_utf8()..]
                        .chars()
                        .find(|candidate| !candidate.is_ascii_whitespace());
                    if prev.map(|candidate| candidate.is_ascii_digit()).unwrap_or(false)
                        && next.map(|candidate| candidate.is_ascii_digit()).unwrap_or(false)
                    {
                        continue;
                    }
                }
                push_answer_unit(&mut units, &line[start..idx + ch.len_utf8()]);
                start = idx + ch.len_utf8();
            }
        }
        if start < line.len() {
            push_answer_unit(&mut units, &line[start..]);
        }
        if units.len() >= 18 {
            break;
        }
    }
    units
}

fn strip_markdown_link_targets(raw: &str) -> String {
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

fn push_answer_unit(units: &mut Vec<String>, raw: &str) {
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

fn answer_unit_specific_terms(unit: &str) -> Vec<String> {
    let mut seen = BTreeSet::<String>::new();
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
        for piece in answer_specific_term_pieces(cleaned) {
            let normalized = normalize_research_token(piece);
            if normalized.len() < 3
                && normalized != "ai"
                && !normalized.chars().any(|ch| ch.is_ascii_digit())
            {
                continue;
            }
            if answer_specific_stop_term(&normalized) {
                continue;
            }
            let letters = piece
                .chars()
                .filter(|ch| ch.is_ascii_alphabetic())
                .collect::<Vec<_>>();
            let uppercase_letters = letters.iter().filter(|ch| ch.is_ascii_uppercase()).count();
            let has_digit = piece.chars().any(|ch| ch.is_ascii_digit());
            let is_acronym = letters.len() >= 2
                && uppercase_letters >= 2
                && uppercase_letters * 2 >= letters.len();
            let has_internal_capital = letters.iter().skip(1).any(|ch| ch.is_ascii_uppercase());
            let is_capitalized = piece
                .chars()
                .next()
                .map(|ch| ch.is_ascii_uppercase())
                .unwrap_or(false);
            let domain_like = token_looks_domain_like(piece);
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
        if terms.len() >= 12 {
            break;
        }
    }
    terms
}

fn answer_specific_term_pieces(token: &str) -> Vec<&str> {
    if token_looks_domain_like(token) {
        return vec![token];
    }
    let pieces = token
        .split(|ch| matches!(ch, '/' | '-' | '_' | '+'))
        .filter(|piece| !piece.is_empty())
        .collect::<Vec<_>>();
    if pieces.len() <= 1 {
        vec![token]
    } else {
        pieces
    }
}

fn token_looks_domain_like(token: &str) -> bool {
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

fn answer_specific_stop_term(token: &str) -> bool {
    matches!(
        token,
        "about"
            | "according"
            | "across"
            | "also"
            | "answer"
            | "area"
            | "areas"
            | "activity"
            | "apis"
            | "based"
            | "bestsupported"
            | "because"
            | "between"
            | "bottom"
            | "boundary"
            | "case"
            | "caveat"
            | "caveats"
            | "comparative"
            | "coverage"
            | "critical"
            | "core"
            | "current"
            | "currently"
            | "dimension"
            | "does"
            | "ease"
            | "evidence"
            | "example"
            | "explicitly"
            | "final"
            | "first"
            | "for"
            | "from"
            | "gap"
            | "gaps"
            | "general"
            | "given"
            | "here"
            | "however"
            | "important"
            | "include"
            | "included"
            | "includes"
            | "including"
            | "integrated"
            | "instead"
            | "key"
            | "known"
            | "main"
            | "more"
            | "most"
            | "one"
            | "officer"
            | "overall"
            | "parliamentary"
            | "positioning"
            | "probably"
            | "recent"
            | "retrieved"
            | "safest"
            | "second"
            | "source"
            | "sources"
            | "strong"
            | "stronger"
            | "summary"
            | "than"
            | "that"
            | "the"
            | "their"
            | "there"
            | "these"
            | "third"
            | "this"
            | "those"
            | "through"
            | "what"
            | "while"
            | "with"
            | "within"
            | "without"
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
