#[test]
fn query_satisfaction_accepts_explanatory_prompt_overlap() {
    let response = normalize_for_compare(
        "The Works Progress Administration shaped American public art through \
             New Deal art programs, especially the Federal Art Project, and \
             historians generally interpret the legacy as democratizing access \
             to civic art while still noting political and regional tensions.",
    );
    let satisfaction = query_satisfaction(
        &normalize_for_compare(
            "Research how the Works Progress Administration influenced American public art. Which programs mattered most?",
        ),
        &response,
        &["Works Progress Administration".to_string()],
        1.0,
        true,
        true,
        true,
        false,
    );
    assert_eq!(
        satisfaction.get("intent_answered").and_then(Value::as_bool),
        Some(true)
    );
}

#[test]
fn decision_value_accepts_informational_selection_prompts() {
    let response = normalize_for_compare(
        "The Works Progress Administration shaped American public art primarily \
             through the Federal Art Project and related initiatives. The main \
             historical interpretation is that these programs democratized civic art.",
    );
    let satisfaction = query_satisfaction(
        &normalize_for_compare(
            "Research how the Works Progress Administration influenced American public art. Which programs mattered most?",
        ),
        &response,
        &["Works Progress Administration".to_string()],
        1.0,
        true,
        true,
        true,
        false,
    );
    assert_eq!(
        satisfaction.get("decision_value").and_then(Value::as_bool),
        Some(true)
    );
}

#[test]
fn decision_value_accepts_action_oriented_practical_guidance() {
    let response = normalize_for_compare(
        "Consolidate to one central cloud storage solution, adopt a unified filing system, \
             and decide clear organization rules for the family.",
    );
    let satisfaction = query_satisfaction(
        &normalize_for_compare(
            "Research practical approaches for moving a family from scattered cloud storage to a simple organized digital filing system.",
        ),
        &response,
        &[],
        1.0,
        true,
        true,
        true,
        false,
    );

    assert_eq!(
        satisfaction.get("decision_value").and_then(Value::as_bool),
        Some(true),
        "{satisfaction:#?}"
    );
}

#[test]
fn decision_value_accepts_promising_approaches_selection_prompts() {
    let response = normalize_for_compare(
        "Progress centers on three frequently discussed approaches: lipid nanoparticles, \
             viral vectors, and virus-like particles. Lipid nanoparticles are attractive for \
             transient delivery, while viral vectors offer targeting experience. Broad use is \
             still blocked by tissue targeting, immune reactions, and manufacturing consistency.",
    );
    let satisfaction = query_satisfaction(
        &normalize_for_compare(
            "Research delivery progress. Which approaches look most promising, and what is still blocking broad use?",
        ),
        &response,
        &[],
        1.0,
        true,
        true,
        true,
        false,
    );

    assert_eq!(
        satisfaction.get("decision_value").and_then(Value::as_bool),
        Some(true),
        "{satisfaction:#?}"
    );
}

#[test]
fn recommendation_signal_accepts_shortlist_selection_language() {
    let response = normalize_for_compare(
        "Travel headphone shortlist: Sony is the strongest all-round option, \
             Bose is the top performer for comfort, and Sennheiser is the value pick.",
    );

    assert!(has_recommendation_signal(&response));
}

#[test]
fn query_satisfaction_accepts_plain_contrast_for_comparison_prompts() {
    let response = normalize_for_compare(
        "NAWSA concentrated on state campaigns and lobbying, while the National \
             Woman's Party used picketing and public pressure. Ida B. Wells \
             exposes the movement's racial exclusions, and the 19th Amendment \
             was a constitutional milestone but did not end voter suppression.",
    );
    let satisfaction = query_satisfaction(
        &normalize_for_compare(
            "Research the US women's suffrage movement. Compare NAWSA, the National Woman's Party, Ida B. Wells, and the 19th Amendment.",
        ),
        &response,
        &[
            "NAWSA".to_string(),
            "National Woman's Party".to_string(),
            "Ida B. Wells".to_string(),
            "19th Amendment".to_string(),
        ],
        entity_coverage(
            &response,
            &[
                "NAWSA".to_string(),
                "National Woman's Party".to_string(),
                "Ida B. Wells".to_string(),
                "19th Amendment".to_string(),
            ],
        ),
        true,
        true,
        true,
        false,
    );
    assert_eq!(
        satisfaction.get("intent_answered").and_then(Value::as_bool),
        Some(true)
    );
    assert!(
        satisfaction
            .get("score")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            >= 9
    );
}

#[test]
fn broad_scope_descriptors_do_not_get_derived_initialism_aliases() {
    assert_eq!(
        entity_coverage_aliases("AI agentic landscape"),
        vec!["AI agentic landscape".to_string()]
    );
    assert_eq!(
        entity_coverage_aliases("US public sector"),
        vec!["US public sector".to_string()]
    );
}
