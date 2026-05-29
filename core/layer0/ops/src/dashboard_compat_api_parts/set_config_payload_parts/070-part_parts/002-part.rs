fn latent_tool_candidates_for_message(message: &str, workspace_hints: &[Value]) -> Vec<Value> {
    let cleaned = clean_text(message, 2_200);
    if cleaned.is_empty()
        || cleaned.starts_with('/')
        || cleaned.contains("tool::")
        || message_explicitly_disallows_tool_calls(&cleaned)
        || message_is_affirmative_confirmation(&cleaned)
        || message_is_negative_confirmation(&cleaned)
        || message_is_tooling_status_check(&cleaned)
        || !workspace_hints.is_empty()
    {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut seen = HashSet::<String>::new();
    if let Some(query) = latent_batch_query_recovery_query_from_message(&cleaned) {
        let proposed_input = json!({
            "source": "web",
            "query": query,
            "aperture": "medium"
        });
        push_latent_tool_candidate(
            &mut out,
            &mut seen,
            &cleaned.to_ascii_lowercase(),
            "batch_query",
            "Research query pack",
            "High-confidence external research turn should recover into web retrieval before final answer.",
            proposed_input.clone(),
        );
        if let Some(candidate) = out.last_mut().and_then(Value::as_object_mut) {
            candidate.insert("workflow_only".to_string(), Value::Bool(true));
            candidate.insert(
                "selected_tool_family".to_string(),
                Value::String("web_research".to_string()),
            );
            candidate.insert(
                "selected_tool_key".to_string(),
                Value::String("batch_query".to_string()),
            );
            candidate.insert(
                "selected_tool_label".to_string(),
                Value::String("Research query pack".to_string()),
            );
            candidate.insert(
                "requires_tool_attempt_before_final_answer".to_string(),
                Value::Bool(true),
            );
            candidate.insert("input".to_string(), proposed_input);
            candidate.insert(
                "latent_reason_contract".to_string(),
                Value::String("high_confidence_external_information_recovery".to_string()),
            );
        }
    }
    out
}

fn latent_batch_query_recovery_query_from_message(message: &str) -> Option<String> {
    if let Some(query) = natural_web_search_query_from_message(message) {
        return Some(clean_text(&query, 1_200)).filter(|value| !value.is_empty());
    }
    let cleaned = clean_text(message, 1_200);
    if cleaned.is_empty() {
        return None;
    }
    let lowered = cleaned.to_ascii_lowercase();
    if latent_batch_query_recovery_looks_internal(&lowered) {
        return None;
    }
    let explicit_research = lowered.starts_with("research ")
        || lowered.contains(" research ")
        || lowered.contains("web research")
        || lowered.contains("source-backed")
        || lowered.contains("public evidence")
        || lowered.contains("public sentiment")
        || lowered.contains("research tooling")
        || lowered.contains("research workflow");
    let fresh_or_external = lowered.contains("news")
        || lowered.contains("headline")
        || lowered.contains("current")
        || lowered.contains("latest")
        || lowered.contains("recent")
        || lowered.contains("today")
        || lowered.contains("this week")
        || lowered.contains("this month")
        || lowered.contains("this year")
        || lowered.contains("right now")
        || lowered.contains("as of ")
        || lowered.contains("update on")
        || lowered.contains("landscape")
        || lowered.contains("state of")
        || lowered.contains("overview of")
        || lowered.contains("brief me on")
        || lowered.contains("what's happening")
        || lowered.contains("whats happening");
    let comparison_or_ranking = lowered.contains("compare ")
        || lowered.contains(" comparison")
        || lowered.contains(" vs ")
        || lowered.contains(" versus ")
        || lowered.contains("best ")
        || lowered.contains("top ")
        || lowered.contains("rank ")
        || lowered.contains("which is better");
    if !(explicit_research || fresh_or_external || comparison_or_ranking) {
        return None;
    }
    Some(cleaned)
}

fn latent_batch_query_recovery_looks_internal(lowered: &str) -> bool {
    message_mentions_host_project(lowered)
        || lowered.contains("this system")
        || lowered.contains("this workspace")
        || lowered.contains("workspace ")
        || lowered.contains(" codebase")
        || lowered.contains(" repo")
        || lowered.contains(" repository")
        || lowered.contains(" local files")
        || lowered.contains(" file ")
}
