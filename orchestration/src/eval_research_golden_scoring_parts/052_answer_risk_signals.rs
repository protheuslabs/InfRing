fn answer_unit_is_hedged_or_gap(normalized_unit: &str) -> bool {
    let padded = format!(" {normalized_unit} ");
    contains_any(
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
            " wasn't materialized",
            " wasnt materialized",
            " not materialized",
            " not retrieved",
            " can't give ",
            " cannot give ",
            " source-backed comparison",
            " source backed comparison",
            " search returned only",
            " returned only headline",
            " headline-level",
            " coverage gaps",
            " missing entity",
            " missing facet",
            " lacked direct",
            " lacks direct",
            " no source-backed",
            " limited evidence",
            " available evidence",
            " available snippet",
            " available snippets",
            " coverage gap",
            " safe boundary",
            " do not choose",
            " dont choose",
            " more targeted search",
            " targeted search",
            " would likely yield",
            " verify ",
            " next search direction",
            " needed to choose",
            " unknown",
            " unverified",
            " inference",
            " partial",
        ],
    )
}

fn answer_unit_unsupported_is_significant(
    normalized_unit: &str,
    supported_terms: &[String],
    scope_supported_terms: &[String],
    unsupported_terms: &[String],
) -> bool {
    if unsupported_terms.is_empty() {
        return false;
    }
    if supported_terms.is_empty() && scope_supported_terms.is_empty() {
        return true;
    }
    if answer_unit_has_high_commitment_claim(normalized_unit) {
        return true;
    }
    let total_terms = supported_terms.len() + scope_supported_terms.len() + unsupported_terms.len();
    unsupported_terms.len() >= 2 && unsupported_terms.len() * 2 >= total_terms.max(1)
}

fn answer_unit_has_high_commitment_claim(normalized_unit: &str) -> bool {
    contains_any(
        normalized_unit,
        &[
            " launched ",
            " released ",
            " announced ",
            " acquired ",
            " approved ",
            " indicted ",
            " sued ",
            " won ",
            " raised ",
            " claims ",
            " claimed ",
            " reports ",
            " reported ",
            " published ",
        ],
    )
}

fn answer_unit_alignment_hard_failure(alignment: &Value) -> bool {
    if !alignment
        .get("evaluated")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return false;
    }
    if alignment
        .get("pass")
        .and_then(Value::as_bool)
        .unwrap_or(true)
    {
        return false;
    }
    let unsupported_units = alignment
        .get("unsupported_unit_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let support_rate = alignment
        .get("term_support_rate")
        .and_then(Value::as_f64)
        .unwrap_or(1.0);
    unsupported_units >= 2 || support_rate < 0.75
}

fn evidence_texts_support_term(evidence_texts: &[String], term: &str) -> bool {
    if term.is_empty() {
        return true;
    }
    let stem = research_term_stem(term);
    evidence_texts.iter().any(|text| {
        (term.len() > 2 && text.contains(term))
            || text.split_whitespace().any(|token| {
                let normalized = normalize_research_token(token);
                normalized == term || (!stem.is_empty() && research_term_stem(&normalized) == stem)
            })
    })
}

fn response_has_meta_process_talk(normalized_response: &str) -> bool {
    contains_any(
        normalized_response,
        &[
            "recorded evidence so far",
            "the current turn does not yet support",
            "the current turn",
            "recorded state",
            "research workflow",
            "structured workflow",
            "prompt chain",
            "i m operating within",
            "i am operating within",
            "tools actually executed",
            "no tools actually executed",
            "tool trace complete",
        ],
    )
}

fn response_delegates_research_back_to_user(normalized_response: &str) -> bool {
    contains_any(
        normalized_response,
        &[
            "try a narrower query",
            "retry with a narrower query",
            "check directly",
            "provide sources directly",
            "you can attempt the search again",
            "you could try again",
            "narrow the query",
        ],
    )
}

fn response_explicitly_cannot_answer_goal_from_current_evidence(normalized_response: &str) -> bool {
    if normalized_response.is_empty() {
        return false;
    }
    let explicit_goal_gap = contains_any(
        normalized_response,
        &[
            "i dont have usable source backed evidence",
            "i do not have usable source backed evidence",
            "i dont have usable evidence about",
            "i do not have usable evidence about",
            "i cant provide a source backed",
            "i cannot provide a source backed",
            "no source backed basis to compare",
            "no source backed basis to choose",
            "none of the required facets",
            "everything specific to your research goal",
            "search missed the entity entirely",
        ],
    );
    let off_topic_or_missing_coverage = contains_any(
        normalized_response,
        &[
            "largely off topic snippets",
            "largely off-topic snippets",
            "do not cover the actual",
            "does not cover the actual",
            "doesnt cover the actual",
            "what the evidence covers none",
            "what the evidence misses everything specific",
        ],
    );
    explicit_goal_gap
        || (off_topic_or_missing_coverage
            && response_delegates_research_back_to_user(normalized_response))
}

fn response_denies_recorded_evidence(normalized_response: &str, evidence_count: u64) -> bool {
    if evidence_count == 0 {
        return false;
    }
    let qualified_relevance_denial = contains_any(
        normalized_response,
        &[
            "no relevant evidence",
            "no relevant source",
            "does not cover",
            "doesn't cover",
            "not cover",
            "false positive",
            "off topic",
            "off-topic",
            "not relevant",
            "not about",
            "does not establish",
            "doesn't establish",
            "no source-backed basis to",
            "no source backed basis to",
        ],
    );
    if qualified_relevance_denial {
        return false;
    }
    let denies_source_backed = contains_any(
        normalized_response,
        &[
            "no source backed findings are available",
            "no source-backed findings are available",
            "no source backed synthesis is available",
            "no source-backed synthesis is available",
            "no source backed evidence is available",
            "no source-backed evidence is available",
        ],
    );
    denies_source_backed
        || contains_any(
            normalized_response,
            &[
                "no evidence was found",
                "no evidence is available",
                "no tool result is available",
            ],
        )
}

fn source_summary_without_answer_signal(normalized_response: &str) -> bool {
    if normalized_response.is_empty() {
        return false;
    }
    let generic_bounded_template = normalized_response.contains("the safest bounded answer")
        && normalized_response.contains("recorded evidence so far");
    let raw_retrieval_summary = normalized_response.contains("recorded evidence so far")
        && normalized_response.contains("from web retrieval")
        && (normalized_response.contains("here s what i found")
            || normalized_response.contains("heres what i found"));
    let unanswered_retry_template = normalized_response
        .contains("current turn does not yet support a complete answer")
        && (normalized_response.contains("current tradeoff is breadth versus confidence")
            || normalized_response.contains("treat this as a partial answer"));
    let retrieval_status_dump = contains_any(
        normalized_response,
        &[
            "this retrieval attempt did not produce enough",
            "retrieval attempt did not produce enough",
            "web retrieval ran, but only",
            "only low signal snippets were available",
            "only low-signal snippets were available",
        ],
    ) && contains_any(
        normalized_response,
        &[
            "recorded evidence so far",
            "here s what i found",
            "heres what i found",
            "retry with a narrower query",
            "narrower query",
        ],
    );
    let broken_prompt_echo = normalized_response.contains("complete answer to ?");
    generic_bounded_template
        || raw_retrieval_summary
        || unanswered_retry_template
        || retrieval_status_dump
        || broken_prompt_echo
}

fn excellent_insufficiency_marker_count(normalized_response: &str) -> usize {
    [
        "very limited evidence",
        "limited evidence",
        "insufficient evidence",
        "evidence is insufficient",
        "low confidence snippets",
        "low-confidence snippets",
        "off topic snippets",
        "off-topic snippets",
        "missing entity",
        "missing entities",
        "no source backed",
        "no source-backed",
        "no returned tool result",
        "comparison evidence is insufficient",
        "cannot answer from current evidence",
        "cannot provide a source backed",
        "cannot provide a source-backed",
        "do not have usable source backed evidence",
        "do not have usable source-backed evidence",
        "search missed the entity entirely",
    ]
    .iter()
    .filter(|needle| normalized_response.contains(**needle))
    .count()
}

fn opening_limitation_preface_for_excellent(normalized_response: &str) -> bool {
    let opening = normalized_response
        .split_whitespace()
        .take(60)
        .collect::<Vec<_>>()
        .join(" ");
    if opening.is_empty() {
        return false;
    }
    excellent_insufficiency_marker_count(&opening) >= 1
        || response_has_meta_process_talk(&opening)
        || contains_any(
            &opening,
            &[
                "what the recorded evidence actually shows",
                "what we know",
                "what we do not know",
                "recorded evidence so far",
                "the current turn does not yet support",
            ],
        )
}

fn limitation_heavy_for_excellent(normalized_response: &str) -> bool {
    let insufficiency_marker_count = excellent_insufficiency_marker_count(normalized_response);
    let limitation_preface = opening_limitation_preface_for_excellent(normalized_response);
    let recommendation_signal = has_recommendation_signal(normalized_response);
    let structure_signal = has_tradeoff_or_structure(normalized_response);
    let explicit_goal_gap =
        response_explicitly_cannot_answer_goal_from_current_evidence(normalized_response);
    let source_summary_without_answer = source_summary_without_answer_signal(normalized_response);
    explicit_goal_gap
        || source_summary_without_answer
        || (limitation_preface && insufficiency_marker_count >= 2)
        || (limitation_preface && !recommendation_signal && !structure_signal)
}
