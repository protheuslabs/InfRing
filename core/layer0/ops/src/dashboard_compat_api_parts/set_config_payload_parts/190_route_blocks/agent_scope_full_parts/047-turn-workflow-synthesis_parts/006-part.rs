fn workflow_answer_unit_has_dangling_truncated_tail(normalized: &str) -> bool {
    let tail = normalized
        .trim_matches(|ch: char| !ch.is_ascii_alphanumeric())
        .split_whitespace()
        .last()
        .unwrap_or("");
    matches!(
        tail,
        "a" | "an"
            | "and"
            | "any"
            | "by"
            | "every"
            | "for"
            | "from"
            | "in"
            | "into"
            | "of"
            | "or"
            | "that"
            | "the"
            | "their"
            | "this"
            | "to"
            | "with"
            | "within"
            | "your"
    )
}

fn workflow_text_prefix_looks_like_headline(raw: &str) -> bool {
    let cleaned = clean_text(raw, 220);
    if cleaned.is_empty() || cleaned.contains('.') {
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
    if !(4..=18).contains(&alpha_tokens.len()) {
        return false;
    }
    let title_like_words = alpha_tokens
        .iter()
        .filter(|token| workflow_answer_unit_token_looks_title_like(token))
        .count();
    let lowercase_content_words = alpha_tokens
        .iter()
        .filter(|token| workflow_answer_unit_token_is_lowercase_content_word(token))
        .count();
    title_like_words >= 3 && lowercase_content_words <= 3
}

fn workflow_answer_unit_contains_ui_or_source_shell(unit: &str) -> bool {
    let normalized = unit.to_ascii_lowercase();
    workflow_answer_unit_contains_source_shell_boilerplate(unit)
        || workflow_answer_unit_contains_any(
            &normalized,
            &[
                "copy markdown",
                "copy as markdown",
                "open in chatgpt",
                "open in claude",
                "open in cursor",
                "view as markdown",
                "web result from ",
            ],
        )
}

fn workflow_answer_unit_looks_like_source_title_fragment(unit: &str) -> bool {
    fn title_style_stats(raw: &str) -> Option<(usize, usize, usize, f64)> {
        let cleaned = clean_text(raw, 320);
        if cleaned.is_empty() {
            return None;
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
            return None;
        }
        let title_like_words = alpha_tokens
            .iter()
            .filter(|token| workflow_answer_unit_token_looks_title_like(token))
            .count();
        let lowercase_content_words = alpha_tokens
            .iter()
            .filter(|token| workflow_answer_unit_token_is_lowercase_content_word(token))
            .count();
        let title_ratio = title_like_words as f64 / alpha_tokens.len() as f64;
        Some((
            alpha_tokens.len(),
            title_like_words,
            lowercase_content_words,
            title_ratio,
        ))
    }

    let cleaned = clean_text(unit, 520);
    if cleaned.is_empty() {
        return false;
    }
    let normalized = cleaned.to_ascii_lowercase();
    let normalized_shell = format!(" {} ", normalize_coverage_lane_text(&cleaned));
    if workflow_answer_unit_contains_source_shell_boilerplate(&cleaned)
        || normalized.contains("other supported points:")
        || normalized.contains("important limitation:")
        || normalized.contains("last updated")
        || normalized.starts_with("title:")
        || normalized.starts_with("description:")
        || normalized.starts_with("pt ")
        || normalized.starts_with("part ")
        || cleaned.trim_end_matches('.').ends_with(':')
    {
        return true;
    }
    let Some((alpha_count, title_like_words, lowercase_content_words, title_ratio)) =
        title_style_stats(&cleaned)
    else {
        return false;
    };
    let contains_vs = normalized.contains(" vs ") || normalized.contains(" versus ");
    let headline_punctuation = cleaned.contains(':') || cleaned.contains(" - ");
    let question_like = cleaned.ends_with('?');
    let preview_or_review_marker = normalized_shell.contains(" needs review ")
        || normalized_shell.contains(" public preview ")
        || normalized_shell.contains(" last updated ");
    let title_like_prefix = cleaned
        .split_once(':')
        .and_then(|(prefix, _)| title_style_stats(prefix))
        .map(
            |(
                prefix_alpha_count,
                prefix_title_like_words,
                prefix_lowercase_content_words,
                prefix_title_ratio,
            )| {
                prefix_alpha_count >= 4
                    && prefix_title_like_words >= 3
                    && prefix_lowercase_content_words <= 3
                    && prefix_title_ratio >= 0.55
            },
        )
        .unwrap_or(false);

    (contains_vs && title_like_words >= 3 && lowercase_content_words <= 4)
        || (question_like && title_ratio >= 0.45 && lowercase_content_words <= 4)
        || (headline_punctuation && title_ratio >= 0.50 && lowercase_content_words <= 3)
        || (title_ratio >= 0.65 && lowercase_content_words <= 2)
        || title_like_prefix
        || (preview_or_review_marker && (headline_punctuation || title_ratio >= 0.40))
        || (cleaned.chars().take_while(|ch| ch.is_ascii_digit()).count() >= 4
            && headline_punctuation
            && alpha_count >= 6)
}

fn workflow_answer_unit_token_looks_title_like(token: &str) -> bool {
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

fn workflow_answer_unit_token_is_lowercase_content_word(token: &str) -> bool {
    let normalized = token
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-' || *ch == '/')
        .collect::<String>()
        .to_ascii_lowercase();
    !normalized.is_empty()
        && normalized
            .chars()
            .all(|ch| !ch.is_ascii_alphabetic() || ch.is_ascii_lowercase())
        && !workflow_answer_unit_source_title_style_stopword(&normalized)
}

fn workflow_answer_unit_source_title_style_stopword(token: &str) -> bool {
    matches!(
        token,
        "a" | "an"
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

fn workflow_answer_unit_has_clean_lead(answer: &str) -> bool {
    let cleaned = clean_text(answer, 520);
    let normalized = cleaned.to_ascii_lowercase();
    cleaned
        .chars()
        .next()
        .map(|ch| ch.is_ascii_uppercase())
        .unwrap_or(false)
        && !normalized.starts_with("and ")
        && !normalized.starts_with("or ")
        && !normalized.starts_with("so ")
        && !normalized.starts_with("so that ")
        && !normalized.starts_with("to ")
        && !normalized.starts_with("for ")
}

fn workflow_answer_unit_goal_overlap_count(unit: &str, goal_terms: &[String]) -> usize {
    if goal_terms.is_empty() {
        return 0;
    }
    let normalized = unit.to_ascii_lowercase();
    goal_terms
        .iter()
        .filter(|term| normalized.contains(term.as_str()))
        .count()
}

fn workflow_answer_unit_rank(
    unit: &str,
    goal_terms: &[String],
) -> (usize, usize, usize, usize, usize, usize) {
    let (answer, source) = fallback_answer_unit_text_and_source(unit);
    (
        workflow_answer_unit_goal_overlap_count(&answer, goal_terms),
        usize::from(!workflow_answer_unit_is_low_information_profile_or_overview(
            &answer,
        )),
        usize::from(evidence_packet_text_is_answer_claim(&answer)),
        usize::from(!workflow_answer_unit_looks_like_source_title_fragment(
            &answer,
        )),
        usize::from(workflow_answer_unit_has_clean_lead(&answer)),
        usize::from(
            !source.is_empty() && !source.to_ascii_lowercase().contains("web result from "),
        ),
    )
}

fn fallback_required_lane_intro(required_entity_lanes: &[String]) -> Option<String> {
    let lanes = required_entity_lanes
        .iter()
        .map(|lane| clean_text(lane, 120))
        .filter(|lane| !lane.is_empty())
        .collect::<Vec<_>>();
    match lanes.as_slice() {
        [] => None,
        [lane] => Some(lane.clone()),
        [left, right] => {
            if left.eq_ignore_ascii_case("us") {
                Some(format!("{right} in the US"))
            } else if right.eq_ignore_ascii_case("us") {
                Some(format!("{left} in the US"))
            } else {
                Some(format!("{left} and {right}"))
            }
        }
        _ => None,
    }
}

fn minimum_required_entity_lane_coverage(required_entity_lanes: &[String]) -> usize {
    match required_entity_lanes.len() {
        0 => 0,
        1 => 1,
        _ => 2,
    }
}

fn workflow_finish_visible_sentence(raw: &str) -> String {
    let cleaned = clean_text(raw, 900);
    if cleaned.is_empty() {
        return String::new();
    }
    if cleaned.ends_with('.') || cleaned.ends_with('!') || cleaned.ends_with('?') {
        cleaned
    } else {
        format!("{cleaned}.")
    }
}

fn hard_required_entity_lanes_for_tools(response_tools: &[Value], limit: usize) -> Vec<String> {
    let mut lanes = Vec::<String>::new();
    let limit = limit.clamp(1, 12);
    for lane in synthesis_coverage_lanes_for_tools(response_tools, limit.saturating_mul(3)) {
        let kind = clean_text(lane.get("kind").and_then(Value::as_str).unwrap_or(""), 80)
            .to_ascii_lowercase();
        if kind != "entity" {
            continue;
        }
        let requested = clean_text(
            lane.get("requested_text")
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        );
        if requested.is_empty()
            || lanes
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(&requested))
            || !coverage_lane_should_be_hard_required(&requested, response_tools)
        {
            continue;
        }
        lanes.push(requested);
        if lanes.len() >= limit {
            break;
        }
    }
    lanes
}

fn text_matches_required_entity_lanes(text: &str, required_entity_lanes: &[String]) -> Vec<String> {
    let normalized = normalize_coverage_lane_text(text);
    if normalized.is_empty() {
        return Vec::new();
    }
    required_entity_lanes
        .iter()
        .filter(|lane| normalized_response_covers_coverage_lane(&normalized, lane))
        .cloned()
        .collect()
}

fn fallback_visible_answer_for_required_lanes(
    unit: &str,
    required_entity_lanes: &[String],
    goal_terms: &[String],
) -> (String, Vec<String>) {
    let (answer, source) = fallback_answer_unit_text_and_source(unit);
    if answer.is_empty() {
        return (String::new(), Vec::new());
    }
    let matched_in_answer = text_matches_required_entity_lanes(&answer, required_entity_lanes);
    if required_entity_lanes.is_empty() || !matched_in_answer.is_empty() {
        return (answer, matched_in_answer);
    }
    let matched_in_source = text_matches_required_entity_lanes(&source, required_entity_lanes);
    if matched_in_source.len() == 1 {
        let lane = clean_text(&matched_in_source[0], 120);
        return (
            clean_text(&format!("For {lane}, {answer}"), 520),
            matched_in_source,
        );
    }
    if required_entity_lanes.len() == 1 {
        if let Some(intro) = fallback_required_lane_intro(required_entity_lanes) {
            let synthetic_coverage_is_supported =
                workflow_answer_unit_goal_overlap_count(&answer, goal_terms) >= 2
                    || workflow_answer_unit_goal_overlap_count(&source, goal_terms) >= 2;
            return (
                clean_text(&format!("For {intro}, {answer}"), 520),
                if synthetic_coverage_is_supported {
                    required_entity_lanes
                        .iter()
                        .map(|lane| clean_text(lane, 120))
                        .filter(|lane| !lane.is_empty())
                        .collect()
                } else {
                    Vec::new()
                },
            );
        }
    }
    (answer, matched_in_source)
}

fn fallback_user_visible_coverage_note(response_tools: &[Value]) -> String {
    let coverage = clean_text(
        &first_sentence(&fallback_coverage_lane_sentence(response_tools), 280),
        320,
    );
    if coverage.is_empty() {
        return String::new();
    }
    let lower = coverage.to_ascii_lowercase();
    if lower.contains("usable evidence is present") && !lower.contains("weak or missing") {
        return String::new();
    }
    coverage.replace("Coverage state: ", "").replace(
        "Coverage gaps still matter for:",
        "coverage gaps remain for",
    )
}

fn workflow_join_visible_list(items: &[String]) -> String {
    let cleaned = items
        .iter()
        .map(|item| clean_text(item, 120))
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>();
    match cleaned.as_slice() {
        [] => String::new(),
        [only] => only.clone(),
        [left, right] => format!("{left} and {right}"),
        _ => {
            let mut parts = cleaned.clone();
            let last = parts.pop().unwrap_or_default();
            format!("{}, and {}", parts.join(", "), last)
        }
    }
}

fn synthesis_partial_comparison_decision_hint(
    message: &str,
    response_tools: &[Value],
) -> String {
    if !workflow_prompt_needs_decision_bearing_evidence(message) {
        return String::new();
    }
    let required_entity_lanes = hard_required_entity_lanes_for_tools(response_tools, 8);
    if required_entity_lanes.len() < 2 {
        return String::new();
    }
    let goal_terms = workflow_answer_unit_goal_terms(message);
    let answer_units = evidence_packet_answer_units_for_goal(message, response_tools, 6);
    let mut covered_lanes = synthesis_coverage_lanes_for_tools(response_tools, 24)
        .into_iter()
        .filter(|row| row.get("kind").and_then(Value::as_str) == Some("entity"))
        .filter(|row| {
            matches!(
                row.get("status").and_then(Value::as_str),
                Some("covered") | Some("usable")
            )
        })
        .filter_map(|row| {
            let requested = clean_text(
                row.get("requested_text")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                120,
            );
            (!requested.is_empty()).then_some(requested)
        })
        .filter(|lane| {
            required_entity_lanes
                .iter()
                .any(|required| required.eq_ignore_ascii_case(lane))
        })
        .fold(Vec::<String>::new(), |mut out, lane| {
            if !out.iter().any(|existing| existing.eq_ignore_ascii_case(&lane)) {
                out.push(lane);
            }
            out
        });
    if covered_lanes.is_empty() {
        for unit in answer_units.iter() {
            let (_, matched_lanes) =
                fallback_visible_answer_for_required_lanes(unit, &required_entity_lanes, &goal_terms);
            for lane in matched_lanes {
                let cleaned = clean_text(&lane, 120);
                if cleaned.is_empty()
                    || covered_lanes
                        .iter()
                        .any(|existing| existing.eq_ignore_ascii_case(&cleaned))
                {
                    continue;
                }
                covered_lanes.push(cleaned);
            }
        }
    }
    if covered_lanes.is_empty() || covered_lanes.len() >= required_entity_lanes.len() {
        return String::new();
    }
    let missing_lanes = required_entity_lanes
        .iter()
        .filter(|lane| !covered_lanes.iter().any(|covered| covered.eq_ignore_ascii_case(lane)))
        .map(|lane| clean_text(lane, 120))
        .filter(|lane| !lane.is_empty())
        .collect::<Vec<_>>();
    let best_supported_lane = answer_units
        .into_iter()
        .filter_map(|unit| {
            let (answer, matched_lanes) =
                fallback_visible_answer_for_required_lanes(&unit, &required_entity_lanes, &goal_terms);
            if answer.is_empty() || matched_lanes.len() != 1 {
                return None;
            }
            let lane = clean_text(&matched_lanes[0], 120);
            covered_lanes
                .iter()
                .any(|covered| covered.eq_ignore_ascii_case(&lane))
                .then_some((lane, answer))
        })
        .max_by_key(|(_, answer)| workflow_answer_unit_rank(answer, &goal_terms))
        .map(|(lane, _)| lane);
    if covered_lanes.len() == 1 {
        let covered = best_supported_lane.unwrap_or_else(|| covered_lanes[0].clone());
        let missing = workflow_join_visible_list(&missing_lanes);
        if missing.is_empty() {
            return String::new();
        }
        return clean_text(
            &format!(
                "If a bounded recommendation is needed, treat {covered} as the best-supported option in this evidence set so far, and explicitly mark {missing} as still weakly covered or unverified in this turn."
            ),
            420,
        );
    }
    let covered = workflow_join_visible_list(&covered_lanes);
    let missing = workflow_join_visible_list(&missing_lanes);
    if covered.is_empty() || missing.is_empty() {
        return String::new();
    }
    clean_text(
        &format!(
            "If a bounded recommendation is needed, limit it to the covered set ({covered}) and explicitly mark {missing} as still weakly covered or unverified in this turn."
        ),
        420,
    )
}

fn annotate_final_evidence_outcome_posture(workflow: &mut Value, response_tools: &[Value]) {
    let posture = tool_evidence_outcome_posture(response_tools);
    workflow["final_llm_response"]["evidence_outcome_posture"] = Value::String(posture.to_string());
    workflow["quality_telemetry"]["evidence_outcome_posture"] = Value::String(posture.to_string());
}
