fn answer_unit_is_hedged_or_gap(normalized_unit: &str) -> bool {
    let padded = format!(" {normalized_unit} ");
    answer_unit_contains_modal_may(normalized_unit)
        || contains_any(
            &padded,
            &[
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
                " weren t retrievable",
                " weren't retrievable",
                " were not retrievable",
                " not retrievable",
                " i dont have usable evidence",
                " i do not have usable evidence",
                " i don t have usable evidence",
                " i don't have usable evidence",
                " i dont have usable source backed evidence",
                " i do not have usable source backed evidence",
                " i don t have usable source backed evidence",
                " i don't have usable source backed evidence",
                " can't give ",
                " cannot give ",
                " doesnt address ",
                " doesn t address ",
                " doesn't address ",
                " does not address ",
                " source-backed comparison",
                " source backed comparison",
                " search returned only",
                " the only material returned",
                " returned only headline",
                " headline-level",
                " coverage gaps",
                " missing entity",
                " missing facet",
                " lacked direct",
                " lacks direct",
                " no source-backed",
                " insufficient for a direct",
                " insufficient for a fully source backed",
                " insufficient for a fully source-backed",
                " evidence is insufficient",
                " evidence was insufficient",
                " evidence is too thin",
                " evidence was too thin",
                " too thin to deliver",
                " too thin to provide",
                " too thin to give",
                " limited evidence",
                " available evidence",
                " available snippet",
                " available snippets",
                " without retrieved specifics",
                " retrieved specifics on",
                " retrieved material doesn t specify",
                " retrieved material doesn't specify",
                " retrieved material does not specify",
                " general references rather than substantive",
                " general state references rather than substantive",
                " remain unaddressed in the available material",
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
                " did not return enough evaluative",
                " did not return enough recent scholarship",
                " to answer your question properly i would need",
                " to answer the question properly i would need",
                " would need source backed information",
                " would need source-backed information",
                " none of which appear in the evidence",
                " cant rank ",
                " can t rank ",
                " cannot rank ",
                " cant compare ",
                " can t compare ",
                " cannot compare ",
                " unknown",
                " unverified",
                " inference",
                " partial",
            ],
        )
}

fn answer_unit_contains_modal_may(normalized_unit: &str) -> bool {
    let tokens = normalized_unit
        .split_whitespace()
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    for (idx, token) in tokens.iter().enumerate() {
        if *token != "may" {
            continue;
        }
        let prev = tokens.get(idx.saturating_sub(1)).copied().unwrap_or("");
        let next = tokens.get(idx + 1).copied().unwrap_or("");
        if answer_unit_may_looks_temporal(prev, next) {
            continue;
        }
        return true;
    }
    false
}

fn answer_unit_may_looks_temporal(prev: &str, next: &str) -> bool {
    matches!(
        prev,
        "late" | "early" | "mid" | "in" | "by" | "during" | "through" | "throughout" | "from"
    ) || next.chars().all(|ch| ch.is_ascii_digit())
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
    if !alignment
        .get("usable_evidence")
        .and_then(Value::as_bool)
        .unwrap_or(false)
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

fn response_has_source_title_fragment_contamination(response_text: &str) -> bool {
    let normalized_response = normalize_for_compare(response_text);
    let title_list_marker = contains_any(
        &normalized_response,
        &[
            "other supported points",
            "supported points",
            "related sources",
            "source titles",
            "titles include",
            "headlines include",
            "additional sources",
        ],
    );
    let title_list_comparison_dump =
        title_list_marker
            && contains_any(&normalized_response, &[" vs ", " versus ", " which framework "]);
    let units = answer_text_units(response_text);
    let suspicious_units = units
        .iter()
        .filter(|unit| answer_unit_looks_like_source_title_fragment(unit))
        .count();
    title_list_comparison_dump
        || suspicious_units >= 2
        || (title_list_marker && suspicious_units >= 1)
        || (units.len() == 1 && suspicious_units == 1)
}

fn answer_unit_looks_like_source_title_fragment(unit: &str) -> bool {
    let stripped = strip_markdown_link_targets(unit);
    let cleaned = stripped
        .trim()
        .trim_matches(|ch: char| ch == '"' || ch == '\'' || ch == '[' || ch == ']')
        .trim();
    let word_count = cleaned.split_whitespace().count();
    if !(5..=24).contains(&word_count) {
        return false;
    }

    let normalized = normalize_for_compare(cleaned);
    if contains_any(
        &normalized,
        &[
            "my recommendation",
            "i recommend",
            "the practical takeaway",
            "the practical split",
            "the tradeoff",
            "the trade off",
            "what this means",
            "so i would",
            "i would choose",
            "choose alpha",
            "choose beta",
            "better for production",
            "safer production default",
        ],
    ) {
        return false;
    }

    let tokens = cleaned
        .split_whitespace()
        .map(|token| {
            token.trim_matches(|ch: char| {
                !ch.is_ascii_alphanumeric() && ch != '-' && ch != '.' && ch != '/'
            })
        })
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    let alpha_tokens = tokens
        .iter()
        .copied()
        .filter(|token| token.chars().any(|ch| ch.is_ascii_alphabetic()))
        .collect::<Vec<_>>();
    if alpha_tokens.len() < 4 {
        return false;
    }

    let title_like_words = alpha_tokens
        .iter()
        .filter(|token| token_looks_title_like(token))
        .count();
    let lowercase_content_words = alpha_tokens
        .iter()
        .filter(|token| token_is_lowercase_content_word(token))
        .count();
    let contains_vs = normalized.contains(" vs ") || normalized.contains(" versus ");
    let headline_punctuation = cleaned.contains(':') || cleaned.contains(" - ");
    let question_like = cleaned.ends_with('?');
    let title_ratio = title_like_words as f64 / alpha_tokens.len() as f64;
    let byline_or_source_path_shell = normalized.contains("/author/")
        || normalized.contains("/authors/")
        || normalized.contains(" /author/")
        || normalized.contains(" /authors/")
        || normalized.contains("&bull")
        || cleaned.contains('•');

    (contains_vs && title_like_words >= 3 && lowercase_content_words <= 4)
        || (question_like && title_ratio >= 0.45 && lowercase_content_words <= 4)
        || (headline_punctuation && title_ratio >= 0.50 && lowercase_content_words <= 3)
        || (byline_or_source_path_shell && (headline_punctuation || title_ratio >= 0.40))
        || (title_ratio >= 0.65 && lowercase_content_words <= 2)
}

fn token_looks_title_like(token: &str) -> bool {
    let letters = token
        .chars()
        .filter(|ch| ch.is_ascii_alphabetic())
        .collect::<Vec<_>>();
    if letters.is_empty() {
        return false;
    }
    let uppercase_letters = letters.iter().filter(|ch| ch.is_ascii_uppercase()).count();
    if uppercase_letters == letters.len() {
        return true;
    }
    let first_is_uppercase = token
        .chars()
        .next()
        .map(|ch| ch.is_ascii_uppercase())
        .unwrap_or(false);
    let has_internal_capital = letters.iter().skip(1).any(|ch| ch.is_ascii_uppercase());
    first_is_uppercase || has_internal_capital
}

fn token_is_lowercase_content_word(token: &str) -> bool {
    let normalized = normalize_research_token(token);
    !normalized.is_empty()
        && normalized.chars().all(|ch| !ch.is_ascii_alphabetic() || ch.is_ascii_lowercase())
        && !source_title_style_stopword(&normalized)
}

fn source_title_style_stopword(token: &str) -> bool {
    matches!(
        token,
        "a"
            | "an"
            | "and"
            | "as"
            | "at"
            | "by"
            | "for"
            | "from"
            | "in"
            | "into"
            | "is"
            | "of"
            | "on"
            | "or"
            | "the"
            | "to"
            | "vs"
            | "versus"
            | "with"
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
            "i don t have usable source backed evidence",
            "i don't have usable source backed evidence",
            "i dont have usable evidence about",
            "i do not have usable evidence about",
            "i don t have usable evidence about",
            "i don't have usable evidence about",
            "i dont have enough usable evidence",
            "i do not have enough usable evidence",
            "i don t have enough usable evidence",
            "i don't have enough usable evidence",
            "not enough usable evidence to",
            "current evidence is insufficient for",
            "recorded evidence is insufficient for",
            "evidence is insufficient for",
            "too thin to deliver a source backed",
            "too thin to deliver a source-backed",
            "too thin to provide a source backed",
            "too thin to provide a source-backed",
            "too thin to give a source backed",
            "too thin to give a source-backed",
            "i cant provide a source backed",
            "i cannot provide a source backed",
            "i am unable to provide source backed",
            "i m unable to provide source backed",
            "unable to provide source backed",
            "unable to provide source-backed",
            "i cant rank",
            "i can t rank",
            "i cannot rank",
            "cant rank them directly",
            "can t rank them directly",
            "cannot rank them directly",
            "cant compare them directly",
            "can t compare them directly",
            "cannot compare them directly",
            "no source backed basis to compare",
            "no source backed basis to choose",
            "no directly citable",
            "no source backed claims to cite",
            "no source-backed claims to cite",
            "retrieval did not surface any directly citable",
            "did not surface any directly citable",
            "none of the required facets",
            "the only material returned",
            "to answer your question properly i would need",
            "to answer the question properly i would need",
            "would need source backed information",
            "would need source-backed information",
            "none of which appear in the evidence",
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
    let synthesis_marker_present = contains_any(
        normalized_response,
        &[
            "my recommendation",
            "i recommend",
            "the practical takeaway",
            "the tradeoff",
            "the trade-off",
            "what this means",
            "so the best",
            "so i would",
            "practical evaluation plan",
            "evaluation plan",
            "next step",
            "next steps",
        ],
    );
    let generic_bounded_template = normalized_response.contains("the safest bounded answer")
        && normalized_response.contains("recorded evidence so far");
    let raw_retrieval_summary = normalized_response.contains("recorded evidence so far")
        && normalized_response.contains("from web retrieval")
        && (normalized_response.contains("here s what i found")
            || normalized_response.contains("heres what i found"));
    let thin_source_inventory_after_answer_frame =
        (normalized_response.contains("here s what i found")
            || normalized_response.contains("heres what i found"))
            && contains_any(
                normalized_response,
                &[
                    "web search",
                    "web search:",
                    "web search returned",
                    "from web retrieval",
                    "source:",
                    " source ",
                    "source web result",
                    "description summary",
                ],
            )
            && !synthesis_marker_present;
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
            "what the search returned",
            "quality flags from the search",
            "current retrieval did not surface",
        ],
    ) && contains_any(
        normalized_response,
        &[
            "recorded evidence so far",
            "here s what i found",
            "heres what i found",
            "retry with a narrower query",
            "narrower query",
            "no directly citable",
            "no source backed claims",
            "no source-backed claims",
            "did not surface any directly citable",
        ],
    );
    let fallback_source_fragment_dump =
        normalized_response.contains("based on the retrieved evidence")
            && normalized_response.contains("strongest supported answer")
            && (normalized_response.contains("source web result from")
                || normalized_response.contains("source: web result from")
                || normalized_response.contains(" source:")
                || normalized_response.contains("coverage state usable evidence is present")
                || normalized_response.contains("description summary")
                || normalized_response.contains("user guide"))
            && !synthesis_marker_present;
    let broken_prompt_echo = normalized_response.contains("complete answer to ?");
    generic_bounded_template
        || raw_retrieval_summary
        || thin_source_inventory_after_answer_frame
        || unanswered_retry_template
        || retrieval_status_dump
        || fallback_source_fragment_dump
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
        "not enough usable evidence",
        "dont have enough usable evidence",
        "do not have enough usable evidence",
        "comparison evidence is insufficient",
        "cannot answer from current evidence",
        "cannot provide a source backed",
        "cannot provide a source-backed",
        "unable to provide source backed",
        "unable to provide source-backed",
        "do not have usable source backed evidence",
        "do not have usable source-backed evidence",
        "no directly citable",
        "no source backed claims to cite",
        "no source-backed claims to cite",
        "retrieval did not surface any directly citable",
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
