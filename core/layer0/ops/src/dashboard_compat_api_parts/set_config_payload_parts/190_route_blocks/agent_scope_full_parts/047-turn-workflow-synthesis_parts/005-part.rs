fn response_has_answer_unit_traceability_violation(
    response_text: &str,
    response_tools: &[Value],
) -> bool {
    let evidence_texts = workflow_evidence_alignment_texts(response_tools);
    if evidence_texts.is_empty() {
        return false;
    }
    for unit in workflow_answer_text_units(response_text).iter().take(18) {
        let terms = workflow_answer_unit_specific_terms(unit);
        if terms.is_empty() {
            continue;
        }
        let normalized_unit = normalize_coverage_lane_text(unit);
        if workflow_answer_unit_is_hedged_or_gap(&normalized_unit) {
            continue;
        }
        let mut supported_terms = Vec::<String>::new();
        let mut unsupported_terms = Vec::<String>::new();
        for term in terms {
            if workflow_evidence_texts_support_term(&evidence_texts, &term) {
                supported_terms.push(term);
            } else {
                unsupported_terms.push(term);
            }
        }
        if workflow_answer_unit_unsupported_is_significant(
            &normalized_unit,
            &supported_terms,
            &unsupported_terms,
        ) {
            return true;
        }
    }
    false
}

fn workflow_answer_unit_contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(*needle))
}

fn workflow_answer_unit_contains_source_shell_boilerplate(unit: &str) -> bool {
    let normalized = format!(" {} ", normalize_coverage_lane_text(unit));
    workflow_answer_unit_contains_any(
        &normalized,
        &[
            " title ",
            " description ",
            " affiliate disclosure ",
            " reader supported ",
            " if youre from the future ",
            " if you re from the future ",
            " other supported points ",
            " important limitation ",
            " mins read ",
            " min read ",
            " minute read ",
            " this survey examines ",
            " this report examines ",
            " this guide examines ",
            " this overview examines ",
            " this article examines ",
            " this article explores ",
            " this paper examines ",
            " this paper explores ",
        ],
    )
}

fn workflow_answer_unit_prompt_asks_for_process_or_schedule(message: &str) -> bool {
    let normalized = message.to_ascii_lowercase();
    workflow_answer_unit_contains_any(
        &normalized,
        &[
            "schedule",
            "scheduled",
            "deadline",
            "nomination",
            "nominations",
            "application",
            "registration",
            "calendar",
            "when",
            "date",
            "dates",
            "announce",
            "announcement",
            "announcements",
        ],
    )
}

fn workflow_prompt_needs_decision_bearing_evidence(message: &str) -> bool {
    let normalized = format!(" {} ", normalize_coverage_lane_text(message));
    workflow_answer_unit_contains_any(
        &normalized,
        &[
            " assess ",
            " compare ",
            " comparison ",
            " decide ",
            " decision ",
            " choose ",
            " evaluate ",
            " evaluation ",
            " practical ",
            " recommend ",
            " recommendation ",
            " should ",
            " tradeoff ",
            " tradeoffs ",
            " versus ",
            " vs ",
            " which ",
        ],
    )
}

fn workflow_answer_unit_is_process_or_metadata_fact(unit: &str) -> bool {
    let normalized = unit.to_ascii_lowercase();
    workflow_answer_unit_contains_source_shell_boilerplate(unit)
        || workflow_answer_unit_contains_any(
            &normalized,
            &[
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
                "event calendar",
                "calendar of events",
                "calendar schedule",
                "press release",
            ],
        )
}

fn workflow_answer_unit_has_concrete_decision_signal(unit: &str) -> bool {
    let normalized = format!(" {} ", normalize_coverage_lane_text(unit));
    workflow_answer_unit_contains_any(
        &normalized,
        &[
            " approval ",
            " benchmark ",
            " capacity ",
            " certification ",
            " compliant ",
            " control ",
            " controls ",
            " cost ",
            " deployment ",
            " deployments ",
            " evidence ",
            " implementation ",
            " integration ",
            " integrations ",
            " limit ",
            " limits ",
            " limitation ",
            " metric ",
            " metrics ",
            " outcome ",
            " outcomes ",
            " performance ",
            " pricing ",
            " requirement ",
            " requirements ",
            " risk ",
            " risks ",
        ],
    )
}

fn workflow_answer_unit_is_low_information_profile_or_overview(unit: &str) -> bool {
    let normalized = format!(" {} ", normalize_coverage_lane_text(unit));
    if workflow_answer_unit_has_concrete_decision_signal(unit) {
        return false;
    }
    workflow_answer_unit_contains_any(
        &normalized,
        &[
            " clear leaders emerging ",
            " enables teams to ",
            " helps teams ",
            " is a modernized version ",
            " is a platform ",
            " is a solution ",
            " is a tool ",
            " is an ai powered ",
            " is an ai-powered ",
            " market has matured ",
            " markets have matured ",
            " purpose built ",
            " purpose-built ",
            " designed to ",
            " provides an overview ",
            " this guide covers ",
            " this overview covers ",
            " this report covers ",
        ],
    )
}

fn workflow_answer_unit_goal_terms(message: &str) -> Vec<String> {
    let mut terms = Vec::<String>::new();
    for raw in message.split_whitespace() {
        let token = raw
            .chars()
            .filter(|ch| ch.is_ascii_alphanumeric())
            .collect::<String>()
            .to_ascii_lowercase();
        if token.len() < 4 {
            continue;
        }
        if matches!(
            token.as_str(),
            "what"
                | "some"
                | "give"
                | "from"
                | "this"
                | "that"
                | "with"
                | "about"
                | "current"
                | "latest"
                | "news"
                | "week"
                | "month"
                | "year"
                | "update"
                | "overview"
                | "landscape"
                | "compare"
                | "best"
        ) {
            continue;
        }
        if token.chars().all(|ch| ch.is_ascii_digit()) {
            continue;
        }
        let term = if token.len() > 5 && token.ends_with('s') {
            token.trim_end_matches('s').to_string()
        } else {
            token
        };
        if !terms.iter().any(|existing| existing == &term) {
            terms.push(term);
        }
        if terms.len() >= 8 {
            break;
        }
    }
    terms
}

fn workflow_answer_unit_matches_goal(unit: &str, goal_terms: &[String]) -> bool {
    if goal_terms.is_empty() {
        return true;
    }
    let normalized = unit.to_ascii_lowercase();
    goal_terms.iter().any(|term| normalized.contains(term))
        || workflow_answer_unit_contains_any(
            &normalized,
            &[
                "reported",
                "observed",
                "discovered",
                "launched",
                "released",
                "approved",
                "published",
                "improved",
                "milestone",
                "first ",
                "new ",
            ],
        )
}

fn evidence_packet_answer_units_for_goal(
    message: &str,
    response_tools: &[Value],
    limit: usize,
) -> Vec<String> {
    let requested_limit = limit.clamp(1, 8);
    let internal_limit = requested_limit.saturating_mul(3).clamp(requested_limit, 8);
    let units = evidence_packet_answer_units(response_tools, internal_limit);
    if units.is_empty() || workflow_answer_unit_prompt_asks_for_process_or_schedule(message) {
        return units.into_iter().take(requested_limit).collect();
    }
    let goal_terms = workflow_answer_unit_goal_terms(message);
    let needs_decision_bearing_evidence = workflow_prompt_needs_decision_bearing_evidence(message);
    let mut filtered = units
        .iter()
        .filter(|unit| {
            let (answer, _) = fallback_answer_unit_text_and_source(unit);
            !answer.is_empty()
                && !workflow_answer_unit_is_process_or_metadata_fact(&answer)
                && !workflow_answer_unit_contains_ui_or_source_shell(&answer)
                && !workflow_answer_unit_looks_like_source_title_fragment(&answer)
                && workflow_answer_unit_matches_goal(&answer, &goal_terms)
                && evidence_packet_text_is_answer_claim(&answer)
                && !(needs_decision_bearing_evidence
                    && workflow_answer_unit_is_low_information_profile_or_overview(&answer))
        })
        .cloned()
        .collect::<Vec<_>>();
    filtered.sort_by(|left, right| {
        workflow_answer_unit_rank(right, &goal_terms)
            .cmp(&workflow_answer_unit_rank(left, &goal_terms))
    });
    if filtered.is_empty() {
        let mut fallback = units
            .iter()
            .filter(|unit| {
                let (answer, _) = fallback_answer_unit_text_and_source(unit);
                !answer.is_empty()
                    && !workflow_answer_unit_contains_ui_or_source_shell(&answer)
                    && !workflow_answer_unit_looks_like_source_title_fragment(&answer)
                    && evidence_packet_text_is_answer_claim(&answer)
                    && !(needs_decision_bearing_evidence
                        && workflow_answer_unit_is_low_information_profile_or_overview(&answer))
            })
            .cloned()
            .collect::<Vec<_>>();
        if fallback.is_empty() {
            fallback = units
                .iter()
                .filter(|unit| {
                    let (answer, _) = fallback_answer_unit_text_and_source(unit);
                    !answer.is_empty()
                        && !workflow_answer_unit_contains_ui_or_source_shell(&answer)
                        && !workflow_answer_unit_looks_like_datestamped_headline_shell(&answer)
                        && evidence_packet_text_is_answer_claim(&answer)
                        && !(needs_decision_bearing_evidence
                            && workflow_answer_unit_is_low_information_profile_or_overview(
                                &answer,
                            ))
                })
                .cloned()
                .collect::<Vec<_>>();
        }
        fallback.sort_by(|left, right| {
            workflow_answer_unit_rank(right, &goal_terms)
                .cmp(&workflow_answer_unit_rank(left, &goal_terms))
        });
        fallback.truncate(requested_limit);
        fallback
    } else {
        filtered.truncate(requested_limit);
        filtered
    }
}

fn response_tools_have_answer_ready_evidence_packets(response_tools: &[Value]) -> bool {
    !evidence_packet_answer_units(response_tools, 1).is_empty()
}

fn fallback_answer_unit_text_and_source(unit: &str) -> (String, String) {
    fn trim_answer_front_matter_prefix(raw: &str) -> String {
        let cleaned = clean_text(raw, 520);
        let lowered = cleaned.to_ascii_lowercase();
        for marker in [
            " abstract this ",
            " abstract the ",
            " introduction this ",
            " introduction the ",
            " summary this ",
            " summary the ",
        ] {
            if let Some(index) = lowered.find(marker) {
                let start = index + marker.find(|ch: char| ch.is_ascii_alphabetic()).unwrap_or(0);
                let after_heading = cleaned[start..]
                    .split_once(' ')
                    .map(|(_, rest)| rest)
                    .unwrap_or(&cleaned[start..]);
                return clean_text(after_heading, 520);
            }
        }
        for marker in [" after ", " when ", " if ", " for ", " to "] {
            if let Some(index) = lowered.find(marker) {
                let prefix = &cleaned[..index];
                let suffix = &cleaned[index + 1..];
                if workflow_text_prefix_looks_like_headline(prefix)
                    && evidence_packet_text_is_answer_claim(suffix)
                {
                    return clean_text(suffix, 520);
                }
            }
        }
        cleaned
    }

    fn trim_answer_tail(raw: &str) -> String {
        let cleaned = clean_text(raw, 520);
        let lowered = cleaned.to_ascii_lowercase();
        let mut cut_at = cleaned.len();
        for marker in [
            " copy markdown",
            " copy as markdown",
            " open in chatgpt",
            " open in claude",
            " open in cursor",
            " view as markdown",
        ] {
            if let Some(index) = lowered.find(marker) {
                cut_at = cut_at.min(index);
            }
        }
        trim_answer_front_matter_prefix(&cleaned[..cut_at])
    }
    if let Some((answer, source)) = unit.split_once(" Source: ") {
        (
            trim_answer_tail(answer),
            clean_text(source.trim_end_matches('.'), 220),
        )
    } else {
        (trim_answer_tail(unit), String::new())
    }
}
