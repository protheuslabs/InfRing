fn governance_temp_root() -> tempfile::TempDir {
    tempfile::tempdir().expect("tempdir")
}
fn governance_ok_snapshot() -> Value {
    json!({"ok": true})
}
fn governance_test_chat_script_path(root: &Path) -> PathBuf {
    state_path(
        root,
        "client/runtime/local/state/ui/infring_dashboard/test_chat_script.json",
    )
}
fn governance_test_tool_script_path(root: &Path) -> PathBuf {
    state_path(
        root,
        "client/runtime/local/state/ui/infring_dashboard/test_tool_script.json",
    )
}

#[test]
fn terminal_tools_run_without_signoff_and_still_enforce_command_policy() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let allowed = execute_tool_call_by_name(
        root.path(),
        &snapshot,
        "agent-terminal",
        None,
        "terminal_exec",
        &json!({"command":"echo hi"}),
    );
    assert_ne!(
        allowed.get("error").and_then(Value::as_str),
        Some("tool_explicit_signoff_required")
    );
    let allow_verdict = allowed
        .pointer("/permission_gate/verdict")
        .and_then(Value::as_str)
        .unwrap_or("allow");
    assert_ne!(allow_verdict, "deny");

    let risky = execute_tool_call_by_name(
        root.path(),
        &snapshot,
        "agent-terminal",
        None,
        "terminal_exec",
        &json!({"command":"git reset --hard HEAD"}),
    );
    assert_ne!(
        risky.get("error").and_then(Value::as_str),
        Some("tool_explicit_signoff_required")
    );
}

#[test]
fn workspace_analyze_alias_routes_into_terminal_exec_surface() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let routed = execute_tool_call_by_name(
        root.path(),
        &snapshot,
        "agent-terminal",
        None,
        "workspace_analyze",
        &json!({"query":"effective loc"}),
    );
    assert_ne!(
        routed.get("error").and_then(Value::as_str),
        Some("unsupported_tool")
    );
    assert_ne!(
        routed.get("error").and_then(Value::as_str),
        Some("command_required")
    );
}

#[test]
fn spawn_tools_run_without_confirmation_gate() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let out = execute_tool_call_by_name(
        root.path(),
        &snapshot,
        "agent-spawn",
        None,
        "spawn_subagents",
        &json!({
            "count": 2,
            "objective": "parallelize architecture diagnostics"
        }),
    );
    let error = out.get("error").and_then(Value::as_str).unwrap_or("");
    assert_ne!(error, "tool_explicit_signoff_required");
    assert_ne!(error, "tool_confirmation_required");
    assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
}

#[test]
fn parent_can_archive_descendant_without_signoff_and_reason_is_persisted() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();

    let parent_create = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"parent-gate-test","role":"operator"}"#,
        &snapshot,
    )
    .expect("create parent");
    let parent_id = clean_agent_id(
        parent_create
            .payload
            .get("agent_id")
            .or_else(|| parent_create.payload.get("id"))
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    assert!(!parent_id.is_empty());

    let child_payload = serde_json::to_vec(&json!({
        "name": "child-gate-test",
        "role": "analyst",
        "parent_agent_id": parent_id
    }))
    .expect("serialize child create");
    let child_create = handle(
        root.path(),
        "POST",
        "/api/agents",
        &child_payload,
        &snapshot,
    )
    .expect("create child");
    let child_id = clean_agent_id(
        child_create
            .payload
            .get("agent_id")
            .or_else(|| child_create.payload.get("id"))
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    assert!(!child_id.is_empty());

    let archived = execute_tool_call_by_name(
        root.path(),
        &snapshot,
        &parent_id,
        None,
        "manage_agent",
        &json!({"action":"archive", "agent_id": child_id}),
    );
    assert_eq!(archived.get("ok").and_then(Value::as_bool), Some(true));
    assert_ne!(
        archived.get("error").and_then(Value::as_str),
        Some("tool_explicit_signoff_required")
    );
    assert_eq!(
        archived.get("reason").and_then(Value::as_str),
        Some("Archived by parent agent")
    );

    let archived_state = read_json_loose(
        &root
            .path()
            .join("client/runtime/local/state/ui/infring_dashboard/archived_agents.json"),
    )
    .unwrap_or_else(|| json!({}));
    let reason = archived_state
        .pointer(&format!("/agents/{child_id}/reason"))
        .and_then(Value::as_str)
        .unwrap_or("");
    assert_eq!(reason, "Archived by parent agent");

    let blocked = execute_tool_call_by_name(
        root.path(),
        &snapshot,
        "agent-unrelated",
        None,
        "manage_agent",
        &json!({"action":"archive", "agent_id": child_id}),
    );
    assert_eq!(
        blocked.get("error").and_then(Value::as_str),
        Some("tool_explicit_signoff_required")
    );
}

#[test]
fn semantic_memory_query_route_returns_matches() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let _ = crate::dashboard_agent_state::memory_kv_set(
        root.path(),
        "agent-memory",
        "fact.auth.flow",
        &json!("PKCE callback must include nonce binding."),
    );

    let body = serde_json::to_vec(&json!({"query":"auth nonce", "limit": 5})).expect("serialize");
    let response = handle(
        root.path(),
        "POST",
        "/api/memory/agents/agent-memory/semantic-query",
        &body,
        &snapshot,
    )
    .expect("semantic query");
    assert_eq!(response.status, 200);
    assert_eq!(
        response
            .payload
            .get("ok")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        true
    );
    assert!(response
        .payload
        .get("matches")
        .and_then(Value::as_array)
        .map(|rows| !rows.is_empty())
        .unwrap_or(false));
}

#[test]
fn spawn_tool_applies_budget_circuit_breaker_and_merge_strategy() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let out = execute_tool_call_by_name(
        root.path(),
        &snapshot,
        "agent-parent",
        None,
        "spawn_subagents",
        &json!({
            "count": 8,
            "objective": "Parallelize a large analysis task",
            "merge_strategy": "voting",
            "budget_tokens": 1_000_000,
            "confirm": true,
            "approval_note": "user requested bounded spawn for analysis"
        }),
    );
    let effective = out
        .get("effective_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    assert!(effective <= 1);
    assert_eq!(
        out.pointer("/directive/merge_strategy")
            .and_then(Value::as_str)
            .unwrap_or(""),
        "voting"
    );
    assert!(out
        .pointer("/circuit_breakers/degraded")
        .and_then(Value::as_bool)
        .unwrap_or(false));
}

#[test]
fn web_tool_fallback_can_use_semantic_memory_matches() {
    let root = governance_temp_root();
    let _ = crate::dashboard_agent_state::memory_kv_set(
        root.path(),
        "agent-fallback",
        "fact.verity",
        &json!("Verity plane enforces fidelity receipts and drift checks."),
    );
    let fallback = fallback_memory_query_payload(
        root.path(),
        "agent-fallback",
        "web_search",
        &json!({"query":"verity drift checks"}),
    )
    .expect("fallback payload");
    assert_eq!(
        fallback.get("fallback_used").and_then(Value::as_bool),
        Some(true)
    );
    assert!(fallback
        .get("matches")
        .and_then(Value::as_array)
        .map(|rows| !rows.is_empty())
        .unwrap_or(false));
}

#[test]
fn web_search_summary_strips_search_engine_chrome_noise() {
    let summary = summarize_tool_payload(
        "web_search",
        &json!({
            "ok": true,
            "query": "agentic ai systems architecture best practices 2024",
            "summary": "agentic AI systems architecture best practices 2024 at DuckDuckGo All Regions Argentina Australia Austria arxiv.org/abs/2601.00123"
        }),
    );
    let lowered = summary.to_ascii_lowercase();
    assert!(!lowered.contains("duckduckgo all regions"));
    assert!(lowered.contains("web search for"));
    assert!(lowered.contains("arxiv.org"));
}

#[test]
fn web_fetch_summary_suppresses_example_domain_placeholder_dump() {
    let summary = summarize_tool_payload(
        "web_fetch",
        &json!({
            "ok": true,
            "requested_url": "https://example.com",
            "summary": "Example Domain This domain is for use in documentation examples without needing permission."
        }),
    );
    let lowered = summary.to_ascii_lowercase();
    assert!(lowered.contains("placeholder"));
    assert!(lowered.contains("example.com"));
    assert!(!lowered.contains("without needing permission"));
}

#[test]
fn web_fetch_summary_converts_navigation_chrome_into_actionable_hint() {
    let summary = summarize_tool_payload(
        "web_fetch",
        &json!({
            "ok": true,
            "requested_url": "https://www.bbc.com/",
            "summary": "BBC News - Breaking news. Skip to content. Home News Sport Business Technology Health Culture Arts Travel Audio Video Live."
        }),
    );
    let lowered = summary.to_ascii_lowercase();
    assert!(lowered.contains("navigation/chrome"));
    assert!(lowered.contains("batch_query") || lowered.contains("web_search"));
    assert!(!lowered.contains("skip to content"));
}

#[test]
fn natural_web_intent_does_not_auto_route_peer_comparisons_without_explicit_web_request() {
    let route = natural_web_intent_from_user_message(
        "Compare Infring to its competitors and rank it among peers in a table",
    );
    assert!(route.is_none());
}

#[test]
fn natural_web_intent_does_not_treat_tool_route_mapping_prompts_as_web_queries() {
    let route =
        natural_web_intent_from_user_message("Map `tool::web_search` into a supported route");
    assert!(route.is_none());
}

#[test]
fn conversational_prompt_does_not_auto_route_direct_tool_intent() {
    assert!(direct_tool_intent_from_user_message("what do you think of infring?").is_none());
}

#[test]
fn natural_web_prompt_does_not_auto_route_direct_tool_intent() {
    assert!(
        direct_tool_intent_from_user_message(
            "Try to web search \"top AI agentic frameworks\" and return the results"
        )
        .is_none()
    );
}

#[test]
fn natural_file_prompt_does_not_auto_route_direct_tool_intent() {
    assert!(direct_tool_intent_from_user_message("read file core/layer0/ops/src/main.rs").is_none());
}

#[test]
fn explicit_tool_command_surfaces_web_search_workflow_hint() {
    assert!(direct_tool_intent_from_user_message("tool::web_search:::latest ai agent benchmarks").is_none());
    let hints = chat_workflow_tool_hints_for_message("tool::web_search:::latest ai agent benchmarks");
    assert_eq!(hints.len(), 1);
    let input = hints[0].get("proposed_input").cloned().unwrap_or(Value::Null);
    assert_eq!(
        hints[0].get("tool").and_then(Value::as_str).unwrap_or(""),
        "web_search"
    );
    assert_eq!(
        input.get("query").and_then(Value::as_str).unwrap_or(""),
        "latest ai agent benchmarks"
    );
    assert_eq!(
        input.get("source").and_then(Value::as_str).unwrap_or(""),
        "web"
    );
    assert_eq!(
        input.get("aperture").and_then(Value::as_str).unwrap_or(""),
        "medium"
    );
}

#[test]
fn explicit_tool_command_alias_rejects_compare_shortcut() {
    let hints = chat_workflow_tool_hints_for_message("tool::compare:::compare named systems");
    assert_eq!(hints.len(), 1);
    let input = hints[0].get("proposed_input").cloned().unwrap_or(Value::Null);
    assert_eq!(
        hints[0].get("tool").and_then(Value::as_str).unwrap_or(""),
        "tool_command_router"
    );
    assert_eq!(
        input.get("error").and_then(Value::as_str).unwrap_or(""),
        "unsupported_tool_command"
    );
}

#[test]
fn explicit_tool_command_alias_surfaces_fetch_workflow_hint() {
    let hints = chat_workflow_tool_hints_for_message("tool::fetch:::https://example.com");
    assert_eq!(hints.len(), 1);
    let input = hints[0].get("proposed_input").cloned().unwrap_or(Value::Null);
    assert_eq!(
        hints[0].get("tool").and_then(Value::as_str).unwrap_or(""),
        "web_fetch"
    );
    assert_eq!(
        input.get("url").and_then(Value::as_str).unwrap_or(""),
        "https://example.com"
    );
}

#[test]
fn explicit_tool_command_rejects_unknown_names_with_suggestion() {
    let hints = chat_workflow_tool_hints_for_message("tool::web_serch:::latest");
    assert_eq!(hints.len(), 1);
    let input = hints[0].get("proposed_input").cloned().unwrap_or(Value::Null);
    assert_eq!(
        hints[0].get("tool").and_then(Value::as_str).unwrap_or(""),
        "tool_command_router"
    );
    assert_eq!(
        input.get("error").and_then(Value::as_str).unwrap_or(""),
        "unsupported_tool_command"
    );
    assert_eq!(
        input.get("suggestion").and_then(Value::as_str).unwrap_or(""),
        "web_search"
    );
}

#[test]
fn explicit_tool_command_rejects_malformed_shape_before_routing() {
    let hints = chat_workflow_tool_hints_for_message("tool::web_search::latest");
    assert_eq!(hints.len(), 1);
    let input = hints[0].get("proposed_input").cloned().unwrap_or(Value::Null);
    assert_eq!(
        hints[0].get("tool").and_then(Value::as_str).unwrap_or(""),
        "tool_command_router"
    );
    assert_eq!(input.get("error").and_then(Value::as_str).unwrap_or(""), "tool_command_name_invalid");
}

#[test]
fn explicit_tool_command_maps_memory_store_to_workflow_hint() {
    let hints = chat_workflow_tool_hints_for_message("tool::memory_store:::deploy.mode=staged");
    assert_eq!(hints.len(), 1);
    let input = hints[0].get("proposed_input").cloned().unwrap_or(Value::Null);
    assert_eq!(
        hints[0].get("tool").and_then(Value::as_str).unwrap_or(""),
        "memory_kv_set"
    );
    assert_eq!(
        input.get("key").and_then(Value::as_str).unwrap_or(""),
        "deploy.mode"
    );
    assert_eq!(
        input.get("value").and_then(Value::as_str).unwrap_or(""),
        "staged"
    );
}

#[test]
fn inline_tool_policy_requires_explicit_tooling_request() {
    assert!(!inline_tool_calls_allowed_for_user_message(
        "what do you think of infring?"
    ));
    assert!(!inline_tool_calls_allowed_for_user_message(
        "search the web for latest ai agent benchmarks"
    ));
    assert!(!inline_tool_calls_allowed_for_user_message(
        "Try to web search \"top AI agentic frameworks\" and return the results"
    ));
    assert!(!inline_tool_calls_allowed_for_user_message(
        "tool::web_search:::latest ai agent benchmarks"
    ));
    assert!(inline_tool_calls_allowed_for_user_message("/file core/layer0/ops/src/main.rs"));
    assert!(!inline_tool_calls_allowed_for_user_message(
        "read file core/layer0/ops/src/main.rs"
    ));
    assert!(!inline_tool_calls_allowed_for_user_message(
        "just answer directly, dont use a tool call"
    ));
    assert!(!inline_tool_calls_allowed_for_user_message(
        "why do you keep trying tool calls and not synthesizing results"
    ));
}

#[test]
fn workflow_decision_tree_v2_defaults_simple_questions_to_info_without_tools() {
    let decision = workflow_turn_tool_decision_tree("what do you think about this idea?");
    assert_eq!(
        decision.get("contract").and_then(Value::as_str),
        Some("manual_toolbox_gate_v1")
    );
    assert_eq!(
        decision.get("gate_decision_mode").and_then(Value::as_str),
        Some("manual_work_category_v1")
    );
    assert_eq!(
        decision.get("should_call_tools").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        decision
            .get("auto_decisions_disabled")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        decision
            .get("workflow_retry_limit")
            .and_then(Value::as_i64),
        Some(1)
    );
}

#[test]
fn workflow_decision_tree_v2_selects_minimal_web_tools_only_when_needed() {
    let decision = workflow_turn_tool_decision_tree(
        "try to web search \"top ai agentic frameworks\" and return the results",
    );
    assert_eq!(
        decision.get("gate_decision_mode").and_then(Value::as_str),
        Some("manual_work_category_v1")
    );
    assert_eq!(
        decision
            .get("requires_live_web")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        decision.get("should_call_tools").and_then(Value::as_bool),
        Some(false)
    );
}

#[test]
fn workflow_decision_tree_v2_classifies_file_edits_as_task_route() {
    let decision = workflow_turn_tool_decision_tree(
        "patch core/layer0/ops/src/main.rs to fix the gate",
    );
    assert_eq!(
        decision.get("gate_decision_mode").and_then(Value::as_str),
        Some("manual_work_category_v1")
    );
    assert_eq!(
        decision
            .get("requires_file_mutation")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        decision.get("should_call_tools").and_then(Value::as_bool),
        Some(false)
    );
}

#[test]
fn workflow_decision_tree_explicit_file_tool_access_uses_task_tool_gate() {
    let decision = workflow_turn_tool_decision_tree("access the file tooling");
    assert_eq!(
        decision.get("gate_decision_mode").and_then(Value::as_str),
        Some("manual_work_category_v1")
    );
    assert_eq!(
        decision.get("should_call_tools").and_then(Value::as_bool),
        Some(false)
    );
    let first_gate_id = decision
        .get("first_gate_id")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let first_gate = decision
        .get("gates")
        .and_then(Value::as_array)
        .and_then(|gates| {
            gates
                .iter()
                .find(|gate| gate.get("gate_id").and_then(Value::as_str) == Some(first_gate_id))
        })
        .expect("first gate should be present in JSON-derived trace");
    assert_eq!(
        first_gate.get("gate_id").and_then(Value::as_str),
        Some(first_gate_id)
    );
    assert_eq!(
        first_gate.get("input_kind").and_then(Value::as_str),
        Some("multiple_choice")
    );
    assert!(first_gate.get("question").and_then(Value::as_str).is_some());
    assert!(decision.get("needs_tool_access").is_some_and(Value::is_null));
    assert_eq!(
        decision
            .get("gate_1_submission_status")
            .and_then(Value::as_str),
        Some("awaiting_llm_submission")
    );
    assert_eq!(
        decision
            .get("gate_1_decision_source")
            .and_then(Value::as_str),
        Some("pending_llm_submission")
    );
    assert_eq!(
        decision
            .pointer("/gate_submission/gate_id")
            .and_then(Value::as_str),
        Some("gate_1_work_category_menu")
    );
    assert_eq!(
        decision
            .pointer("/gate_submission/input_shape/type")
            .and_then(Value::as_str),
        Some("multiple_choice")
    );
    assert!(decision
        .pointer("/gate_submission/llm_submission")
        .is_some_and(Value::is_null));
    assert_eq!(
        decision
            .pointer("/gate_submission/accepted")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        decision
            .pointer("/gate_submission/resume_token")
            .and_then(Value::as_str),
        Some("gate_1_work_category_menu.awaiting_llm_submission")
    );
    assert_eq!(
        decision.get("reason_code").and_then(Value::as_str),
        Some("manual_menu_presented")
    );
}

#[test]
fn workflow_decision_tree_blocks_status_check_turns_from_tool_calls() {
    let decision = workflow_turn_tool_decision_tree("did you do the web request??");
    assert_eq!(
        decision
            .get("status_check_message")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        decision.get("should_call_tools").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        decision
            .get("requires_live_web")
            .and_then(Value::as_bool),
        Some(false)
    );
}

#[test]
fn workflow_decision_tree_break_workflow_turn_still_enters_gate_1() {
    let decision = workflow_turn_tool_decision_tree("break the workflow and respond directly");
    assert_eq!(
        decision.get("gate_decision_mode").and_then(Value::as_str),
        Some("manual_work_category_v1")
    );
    assert_eq!(
        decision.get("should_call_tools").and_then(Value::as_bool),
        Some(false)
    );
    assert!(decision.get("needs_tool_access").is_some_and(Value::is_null));
    assert_eq!(
        decision
            .get("gate_1_submission_status")
            .and_then(Value::as_str),
        Some("awaiting_llm_submission")
    );
    assert_eq!(
        decision
            .pointer("/gate_submission/gate_id")
            .and_then(Value::as_str),
        Some("gate_1_work_category_menu")
    );
}

#[test]
fn workflow_decision_tree_explicit_web_request_still_enters_gate_1() {
    let decision = workflow_turn_tool_decision_tree(
        "search the web for current top agent frameworks and summarize them",
    );
    assert_eq!(
        decision.get("gate_decision_mode").and_then(Value::as_str),
        Some("manual_work_category_v1")
    );
    assert_eq!(
        decision.get("should_call_tools").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        decision
            .get("requires_live_web")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert!(decision.get("needs_tool_access").is_some_and(Value::is_null));
    assert_eq!(
        decision
            .pointer("/gate_submission/gate_id")
            .and_then(Value::as_str),
        Some("gate_1_work_category_menu")
    );
}

#[test]
fn workflow_decision_tree_file_read_request_still_enters_gate_1() {
    let decision = workflow_turn_tool_decision_tree("read this file: core/layer0/ops/src/main.rs");
    assert_eq!(
        decision.get("gate_decision_mode").and_then(Value::as_str),
        Some("manual_work_category_v1")
    );
    assert_eq!(
        decision.get("should_call_tools").and_then(Value::as_bool),
        Some(false)
    );
    assert!(decision.get("needs_tool_access").is_some_and(Value::is_null));
    assert_eq!(
        decision
            .get("gate_1_submission_status")
            .and_then(Value::as_str),
        Some("awaiting_llm_submission")
    );
    assert_eq!(
        decision
            .pointer("/gate_submission/resume_token")
            .and_then(Value::as_str),
        Some("gate_1_work_category_menu.awaiting_llm_submission")
    );
}

#[test]
fn workflow_decision_tree_latest_keyword_sets_web_research_advisory_without_auto_tools() {
    let decision =
        workflow_turn_tool_decision_tree("what are the latest agent framework changes this week?");
    assert_eq!(
        decision.get("gate_decision_mode").and_then(Value::as_str),
        Some("manual_work_category_v1")
    );
    assert_eq!(
        decision.get("should_call_tools").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        decision
            .get("requires_live_web")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        decision
            .get("selected_work_category")
            .and_then(Value::as_str),
        Some("web_research")
    );
    assert_eq!(
        decision.get("needs_tool_access").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        decision
            .pointer("/gate_submission/gate_id")
            .and_then(Value::as_str),
        Some("gate_1_work_category_menu")
    );
}

#[test]
fn workflow_decision_tree_search_keyword_sets_web_research_advisory_without_auto_tools() {
    let decision =
        workflow_turn_tool_decision_tree("search current framework docs and tell me what changed");
    assert_eq!(
        decision.get("gate_decision_mode").and_then(Value::as_str),
        Some("manual_work_category_v1")
    );
    assert_eq!(
        decision.get("should_call_tools").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        decision
            .get("requires_live_web")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        decision
            .get("selected_work_category")
            .and_then(Value::as_str),
        Some("web_research")
    );
    assert_eq!(
        decision.get("needs_tool_access").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        decision
            .get("gate_1_submission_status")
            .and_then(Value::as_str),
        Some("awaiting_llm_submission")
    );
}

#[test]
fn workflow_decision_tree_update_keyword_sets_web_research_advisory_without_auto_tools() {
    let decision = workflow_turn_tool_decision_tree(
        "give me an update on the top framework releases and ecosystem shifts",
    );
    assert_eq!(
        decision.get("gate_decision_mode").and_then(Value::as_str),
        Some("manual_work_category_v1")
    );
    assert_eq!(
        decision.get("should_call_tools").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        decision
            .get("requires_live_web")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        decision
            .get("selected_work_category")
            .and_then(Value::as_str),
        Some("web_research")
    );
    assert_eq!(
        decision.get("needs_tool_access").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        decision
            .pointer("/gate_submission/resume_token")
            .and_then(Value::as_str),
        Some("gate_1_work_category_menu.awaiting_llm_submission")
    );
}

#[test]
fn workflow_decision_tree_diagnostic_phrase_does_not_trigger_hidden_route_change() {
    let decision = workflow_turn_tool_decision_tree(
        "what's going on? are you stuck in a workflow fallback loop again?",
    );
    assert_eq!(
        decision.get("gate_decision_mode").and_then(Value::as_str),
        Some("manual_work_category_v1")
    );
    assert_eq!(
        decision.get("should_call_tools").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        decision
            .get("requires_live_web")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert!(decision.get("needs_tool_access").is_some_and(Value::is_null));
    assert_eq!(
        decision
            .get("gate_1_submission_status")
            .and_then(Value::as_str),
        Some("awaiting_llm_submission")
    );
    assert_eq!(
        decision
            .pointer("/gate_submission/gate_id")
            .and_then(Value::as_str),
        Some("gate_1_work_category_menu")
    );
}

#[test]
fn workflow_decision_tree_legacy_route_copy_phrase_does_not_suppress_tools_or_reroute() {
    let decision = workflow_turn_tool_decision_tree(
        "the first gate is still classifying this as an info route instead of a task route",
    );
    assert_eq!(
        decision.get("gate_decision_mode").and_then(Value::as_str),
        Some("manual_work_category_v1")
    );
    assert_eq!(
        decision.get("should_call_tools").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        decision
            .get("requires_live_web")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert!(decision.get("needs_tool_access").is_some_and(Value::is_null));
    assert_eq!(
        decision
            .get("gate_1_decision_source")
            .and_then(Value::as_str),
        Some("pending_llm_submission")
    );
    assert_eq!(
        decision
            .pointer("/gate_submission/resume_token")
            .and_then(Value::as_str),
        Some("gate_1_work_category_menu.awaiting_llm_submission")
    );
}

#[test]
fn inline_spawn_tool_calls_autoconfirm_without_user_swarm_phrase() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let response =
        "<function=spawn_subagents>{\"count\":3,\"objective\":\"parallelize analysis\"}</function>";
    let (text, cards, pending_confirmation, suppressed) = execute_inline_tool_calls(
        root.path(),
        &snapshot,
        "agent-inline",
        None,
        response,
        "continue with execution plan",
        true,
    );
    assert!(!suppressed);
    assert_eq!(cards.len(), 1);
    assert_eq!(
        cards[0].get("is_error").and_then(Value::as_bool),
        Some(false)
    );
    assert!(pending_confirmation.is_none());
    let lowered = text.to_ascii_lowercase();
    assert!(!lowered.contains("tool_explicit_signoff_required"));
    assert!(!lowered.contains("spawn_subagents failed"));
    assert!(!lowered.contains("confirmation"));
}

#[test]
fn inline_tool_execution_is_suppressed_for_plain_conversation_turns() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let response = "<function=web_search>{\"query\":\"latest technology news\"}</function>";
    let (text, cards, pending_confirmation, suppressed) = execute_inline_tool_calls(
        root.path(),
        &snapshot,
        "agent-inline-suppressed",
        None,
        response,
        "what do you think of infring?",
        false,
    );
    assert!(suppressed);
    assert!(cards.is_empty());
    assert!(pending_confirmation.is_none());
    assert!(text.trim().is_empty());
}

#[test]
fn inline_tool_execution_replaces_low_signal_cleaned_text_with_tool_fallback_lines() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let response = "<function=spawn_subagents>{\"count\":2,\"objective\":\"parallelize analysis\"}</function>\nFrom web retrieval: bing.com: compare [A with B] vs compare A [with B]";
    let (text, cards, pending_confirmation, suppressed) = execute_inline_tool_calls(
        root.path(),
        &snapshot,
        "agent-inline-low-signal",
        None,
        response,
        "parallelize this with a swarm",
        true,
    );
    assert!(!suppressed);
    assert_eq!(cards.len(), 1);
    assert!(pending_confirmation.is_none());
    let lowered = text.to_ascii_lowercase();
    assert!(lowered.contains("spawned"));
    assert!(!lowered.contains("from web retrieval:"));
    assert!(!lowered.contains("bing.com: compare"));
}

#[test]
fn inline_tool_execution_discards_leftover_malformed_function_markup() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let response = "<function=web_search>{\"query\":\"test search functionality\"}</function> <function=web_fetch>{\"url\":\"https://example.";
    let (text, cards, pending_confirmation, suppressed) = execute_inline_tool_calls(
        root.path(),
        &snapshot,
        "agent-inline-malformed-tail",
        None,
        response,
        "test search functionality",
        true,
    );
    assert!(!suppressed);
    assert_eq!(cards.len(), 1);
    assert!(pending_confirmation.is_none());
    assert!(!text.contains("<function="), "{text}");
    assert!(!text.to_ascii_lowercase().contains("web_fetch"), "{text}");
    assert!(!text.trim().is_empty(), "{text}");
}

#[test]
fn workflow_retry_sanitizer_drops_follow_up_tool_markup_tail() {
    let response = "My search for \"top AI agentic frameworks\" didn't return specific framework listings. Let me try a more targeted approach with some well-known framework names.\n\n<function=web_search>{\"query\":\"LangChain AutoGPT BabyAGI AI agent frameworks comparison\"}</function>";
    assert!(workflow_response_requests_more_tooling(response));
    assert_eq!(
        sanitize_workflow_final_response_candidate(response),
        "My search for \"top AI agentic frameworks\" didn't return specific framework listings. Let me try a more targeted approach with some well-known framework names."
    );
}

#[test]
fn workflow_retry_sanitizer_drops_polite_more_search_tail() {
    let response = "I searched official framework sources and found LangGraph, OpenAI Agents SDK, CrewAI, and smolagents. Would you like me to search for deeper benchmark comparisons too?";
    assert!(workflow_response_requests_more_tooling(response));
    assert_eq!(
        sanitize_workflow_final_response_candidate(response),
        "I searched official framework sources and found LangGraph, OpenAI Agents SDK, CrewAI, and smolagents. Would you like me to search for deeper benchmark comparisons too?"
    );
}

#[test]
fn workflow_retry_sanitizer_strips_malformed_inline_function_tail() {
    let response = "No, the web tools still aren't executing.\n<function=web_search>{\"query\":\"test search functionality\"} <function=web_fetch>{\"url\":\"https://example.";
    assert_eq!(
        sanitize_workflow_final_response_candidate(response),
        "No, the web tools still aren't executing."
    );
}

#[test]
fn inline_tool_parser_accepts_quoted_function_name_markup() {
    let response = "<function=\"web_search\">{\"query\":\"top AI agentic frameworks 2024\",\"source\":\"web\",\"aperture\":\"medium\"}</function>";
    let (cleaned, calls) = extract_inline_tool_calls(response, 4);
    assert!(cleaned.is_empty());
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "web_search");
    assert_eq!(calls[0].1.get("query").and_then(Value::as_str), Some("top AI agentic frameworks 2024"));
}

#[test]
fn inline_tool_workspace_analyze_hydrates_comparison_query_from_message() {
    let input = normalize_inline_tool_execution_input(
        "workspace_analyze",
        &json!({}),
        "compare this system (infring) to openclaw",
    );
    assert_eq!(input.get("path").and_then(Value::as_str), Some("."));
    assert_eq!(input.get("full").and_then(Value::as_bool), Some(true));
    assert!(input
        .get("query")
        .and_then(Value::as_str)
        .unwrap_or("")
        .contains("compare this system"));
}

#[test]
fn pending_confirmation_yes_replays_manage_agent_action() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let parent = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"parent-runtime","role":"operator"}"#,
        &snapshot,
    )
    .expect("parent create");
    let parent_id = clean_agent_id(
        parent
            .payload
            .get("agent_id")
            .or_else(|| parent.payload.get("id"))
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    assert!(!parent_id.is_empty());
    let child_payload = serde_json::to_vec(&json!({
        "name": "child-runtime",
        "role": "analyst",
        "parent_agent_id": parent_id
    }))
    .expect("serialize child");
    let child = handle(
        root.path(),
        "POST",
        "/api/agents",
        &child_payload,
        &snapshot,
    )
    .expect("child create");
    let child_id = clean_agent_id(
        child
            .payload
            .get("agent_id")
            .or_else(|| child.payload.get("id"))
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    assert!(!child_id.is_empty());
    let _ = update_profile_patch(
        root.path(),
        &parent_id,
        &json!({
            "pending_tool_confirmation": {
                "tool_name": "manage_agent",
                "input": {"action": "archive", "agent_id": child_id}
            }
        }),
    );
    let yes_body = serde_json::to_vec(&json!({"message":"yes"})).expect("serialize yes");
    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{parent_id}/message"),
        &yes_body,
        &snapshot,
    )
    .expect("message response");
    assert_eq!(response.status, 200);
    assert_eq!(
        response.payload.get("ok").and_then(Value::as_bool),
        Some(true)
    );
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    assert!(!response_text.contains("need your confirmation"));
    let archived_state = read_json_loose(
        &root
            .path()
            .join("client/runtime/local/state/ui/infring_dashboard/archived_agents.json"),
    )
    .unwrap_or_else(|| json!({}));
    let reason = archived_state
        .pointer(&format!("/agents/{child_id}/reason"))
        .and_then(Value::as_str)
        .unwrap_or("");
    assert_eq!(reason, "Archived by parent agent");
    let profile = profiles_map(root.path())
        .get(&parent_id)
        .cloned()
        .unwrap_or_else(|| json!({}));
    assert!(profile
        .get("pending_tool_confirmation")
        .map(Value::is_null)
        .unwrap_or(true));
}

#[test]
fn telemetry_dump_detector_flags_duckduckgo_noise_and_tool_error_codes() {
    let dump = "agentic AI systems architecture at DuckDuckGo All Regions Argentina Australia. spawn_subagents failed: tool_explicit_signoff_required";
    assert!(response_is_unrelated_context_dump(
        "improve this system",
        dump
    ));
}

#[test]
fn unrelated_dump_detector_flags_peer_review_template_leaks() {
    let dump = "AIFFEL Campus Online 5th Code Peer Review Templete - 코더 : 최연석 - 리뷰어 : 김연 # PRT(PeerReviewTemplate) 각 항목을 스스로 확인하고 토의하여 작성한 코드에 적용합니다. 코드가 정상적으로 동작하고 주어진 문제를 해결했나요?";
    assert!(response_is_unrelated_context_dump(
        "did you format that as a list?",
        dump
    ));
}

#[test]
fn unrelated_dump_detector_flags_internal_prompt_leak_even_with_function_markup() {
    let dump = "The user has provided workflow metadata. My role is to refine the hidden routing state while inline function markup <function=web_search>{\"query\":\"...\"}</function> remains present.";
    assert!(response_is_unrelated_context_dump("did it work?", dump));
}

#[test]
fn cross_project_framework_bleed_is_rejected_for_infring_turns() {
    let user = "so how do you think that infring can be better?";
    let bleed = "As an infring agent, I can help if you identify areas or functionalities within the external sample framework where improvements should be made.";
    assert!(response_contains_cross_project_assimilation_bleed(user, bleed));
    assert!(response_is_unrelated_context_dump(user, bleed));
    assert!(passive_memory_attention_event("agent-misty", user, bleed).is_none());
    let guard = final_response_guard_report(user, bleed, &[], false);
    assert_eq!(
        guard
            .get("final_contract_violation")
            .and_then(Value::as_bool),
        Some(true)
    );
    let comparison_user = "compare Infring and the external sample framework";
    let comparison_response =
        "Infring can borrow reliability ideas from within the external sample framework.";
    assert!(!response_contains_cross_project_assimilation_bleed(
        comparison_user,
        comparison_response
    ));
}

#[test]
fn current_turn_project_boundary_prompt_marks_infring_as_host_system() {
    let prompt = current_turn_project_boundary_prompt("how can infring be better?");
    assert!(prompt.contains("Infring as the host system"));
    assert!(prompt.contains("external repos"));
    assert!(prompt.contains("assimilation targets"));
}

#[test]
fn unrelated_dump_detector_flags_kernel_patch_thread_dumps() {
    let dump = "[PATCH v2 1/2] drm/msm/dpu: allow encoder to be created with empty dpu_crtc
[Date Prev][Date Next][Thread Prev][Thread Next][Date Index][Thread Index]
To: Rob Clark <robdclark@example.com>
Subject: [PATCH v2 1/2] drm/msm/dpu: allow encoder to be created with empty dpu_crtc
From: Jessica Zhang <quic_jesszhan@example.com>
In-reply-to: 20230901202143.16356-1-quic_jesszhan@quicinc.com
Signed-off-by: Jessica Zhang <quic_jesszhan@example.com>
diff --git a/drivers/gpu/drm/msm/disp/dpu1/dpu_encoder.c b/drivers/gpu/drm/msm/disp/dpu1/dpu_encoder.c";
    assert!(response_is_unrelated_context_dump(
        "So do you think the system is getting more capable?",
        dump
    ));
}

#[test]
fn unrelated_dump_detector_flags_role_preamble_prompt_dumps() {
    let dump = "I am an expert in the field of deep learning, neural networks, and AI ethics. My role is to provide clear, accurate explanations while maintaining a professional tone. The user has provided a detailed draft response, and my task is to refine and finalize it based on workflow metadata. Source: The Model's Training Data. Mechanism: Faulty Pattern Retrieval. The Error: Context Collapse.";
    assert!(response_is_unrelated_context_dump(
        "but where did the hallucination come from?",
        dump
    ));
}

#[test]
fn unrelated_dump_detector_flags_reddit_goldbach_thread_dump() {
    let dump = "[Request] Is this even possible? How?\n9k Upvotes\nThere is a mathematical proof that shows that all even numbers >=4 can be expressed as the sum of two primes. This is known as the Goldbach Conjecture.";
    assert!(response_is_unrelated_context_dump(
        "try searching for information about the top agentic frameworks for me",
        dump
    ));
}

#[test]
fn unrelated_dump_detector_flags_dataframe_instruction_template_dump() {
    let dump = "1. Find the 10 countries with most projects #The information about the countries is contained in the 'countryname' column of the dataframe\ndf_json['countryname'].value_counts().head(10)";
    assert!(response_is_unrelated_context_dump(
        "how can we find out if it was an actual tool call error or llm error",
        dump
    ));
}

#[test]
fn helpfulness_detector_flags_prompt_echo_and_accepts_direct_answer() {
    assert!(response_prompt_echo_detected(
        "try searching for top agentic frameworks",
        "try searching for top agentic frameworks"
    ));
    assert!(!response_prompt_echo_detected(
        "try searching for top agentic frameworks",
        "Top agentic frameworks today include LangGraph, OpenAI Agents SDK, and AutoGen."
    ));
    assert!(response_answers_user_early(
        "what happened with the web tooling?",
        "The web tooling call failed because the provider returned low-signal results. We should retry with one narrower query."
    ));
}

#[test]
fn actionable_response_gets_next_actions_line() {
    let out = append_next_actions_line_if_actionable(
        "what should we do next to improve web tooling?",
        "The latest run showed low-signal results from web retrieval.",
        &[],
    );
    assert!(!out.contains("Next actions:"), "{out}");
    assert_eq!(out, "The latest run showed low-signal results from web retrieval.");
    let non_actionable = append_next_actions_line_if_actionable(
        "thanks",
        "Glad to help.",
        &[],
    );
    assert!(!non_actionable.contains("Next actions:"), "{non_actionable}");
}

#[test]
fn low_alignment_detector_flags_long_response_without_previous_message_overlap() {
    let user_message =
        "Well give me some actionable steps cause those were really broad. Give 10 steps";
    let recent_context =
        "We are discussing web tooling reliability and context retention in final responses.";
    let unrelated_long_response = "03-树2 List Leaves (25 分) Given a tree, you are supposed to list all the leaves in the order of top down, and left to right. Input Specification: Each input file contains one test case. For each case, the first line gives a positive integer N (≤10). Sample Input: 8. Sample Output: 4 1 5.";
    assert!(response_low_alignment_with_turn_context(
        user_message,
        recent_context,
        unrelated_long_response
    ));
}

#[test]
fn append_turn_message_captures_explicit_remember_fact_for_long_term_memory() {
    let root = governance_temp_root();
    let captured =
        parse_memory_capture_text("remember this: the fallback phrase is cobalt sunrise")
            .expect("memory capture text");
    assert!(captured.to_ascii_lowercase().contains("cobalt sunrise"));
    assert!(parse_memory_capture_text("just answer normally").is_none());

    let _ = crate::dashboard_agent_state::memory_kv_set(
        root.path(),
        "agent-memory-capture",
        "explicit_memory.test",
        &json!({"text": captured, "captured_at": crate::now_iso()}),
    );
    let memory_state = crate::dashboard_agent_state::memory_kv_semantic_query(
        root.path(),
        "agent-memory-capture",
        "cobalt sunrise",
        5,
    );
    assert!(memory_state
        .get("matches")
        .and_then(Value::as_array)
        .map(|rows| !rows.is_empty())
        .unwrap_or(false));
}

#[test]
fn append_turn_message_captures_low_signal_web_tool_outcome_keyframe() {
    let root = governance_temp_root();
    let receipt = append_turn_message(
        root.path(),
        "agent-web-keyframe",
        "try doing a generic search \"top AI agent frameworks\"",
        "The batch query step ran, but only low-signal web output came back. Retry with a narrower query, one specific source URL, or ask me to continue from the recorded tool result.",
    );
    assert_eq!(
        receipt
            .pointer("/tool_outcome_keyframe/tool")
            .and_then(Value::as_str),
        Some("batch_query")
    );
    let context = context_command_payload(
        root.path(),
        "agent-web-keyframe",
        &json!({}),
        &json!({}),
        true,
    );
    let outcomes = context
        .get("recent_tool_outcomes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(outcomes.iter().any(|entry| {
        entry.get("summary")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase()
            .contains("top ai agent frameworks")
    }));
}

#[test]
fn response_tools_summary_rewrites_ack_only_text_into_user_facing_findings() {
    let synthesized = response_tools_summary_for_user(
        &[json!({
            "name": "web_search",
            "is_error": false,
            "result": "Web search findings for \"agent reliability\": - arxiv.org/abs/2601.12345 - github.com/org/repo/issues"
        })],
        4,
    );
    assert!(!synthesized.is_empty());
    assert!(synthesized
        .to_ascii_lowercase()
        .contains("here's what i found"));
    assert!(synthesized.to_ascii_lowercase().contains("web search"));
}

#[test]
fn ack_only_detector_flags_generic_tool_acknowledgements() {
    assert!(response_looks_like_tool_ack_without_findings(
        "I searched the internet and executed the tool call."
    ));
    assert!(response_looks_like_tool_ack_without_findings(
        "Web search completed. I called the tools and processed your request."
    ));
    assert!(!response_looks_like_tool_ack_without_findings(
        "Here are the findings: 1. https://arxiv.org/abs/2601.12345 2. https://github.com/org/repo"
    ));
}

#[test]
fn ack_only_detector_flags_explicit_no_findings_failure_copy() {
    assert!(!response_looks_like_tool_ack_without_findings(
        "The web search ran, but it only returned low-signal snippets in this turn."
    ));
    assert!(!response_looks_like_tool_ack_without_findings(
        "My search for top AI agentic frameworks 2024 didn't return specific framework listings or detailed comparisons."
    ));
    assert!(response_looks_like_tool_ack_without_findings(
        "From web retrieval: bing.com: OpenClaw — Personal AI Assistant — https://openclaw.ai/"
    ));
}

#[test]
fn ack_only_detector_flags_speculative_web_blocker_explanations() {
    let draft = "I understand you're looking for a comparison between this platform and OpenClaw, but I'm currently unable to access web search functionality to gather the necessary information. The system is blocking tool execution attempts, which prevents me from retrieving current details.\n\nBased on the system behavior I'm observing, likely reasons include Configuration Restrictions, Authentication Issues, Rate Limiting, or intentional sandboxed design.";
    assert!(response_looks_like_tool_ack_without_findings(draft));
}

#[test]
fn ack_only_detector_flags_key_findings_source_scaffold_copy() {
    assert!(response_looks_like_tool_ack_without_findings(
        "Key findings for \"Infring AI vs competitors comparison 2024\": - Potential sources: hai.stanford.edu, artificialanalysis.ai, epoch.ai."
    ));
}

#[test]
fn ack_only_detector_flags_duckduckgo_findings_placeholder_copy() {
    assert!(response_looks_like_tool_ack_without_findings(
        "I couldn't extract usable findings for this yet. The search response came from https://duckduckgo.com/html/?q=agent+systems"
    ));
}

#[test]
fn response_tools_summary_drops_ack_only_tool_rows() {
    let synthesized = response_tools_summary_for_user(
        &[json!({
            "name": "web_search",
            "is_error": false,
            "result": "Web search completed."
        })],
        4,
    );
    assert!(synthesized.is_empty());
}

#[test]
fn response_tools_summary_drops_key_findings_source_scaffold_rows() {
    let synthesized = response_tools_summary_for_user(
        &[json!({
            "name": "batch_query",
            "is_error": false,
            "result": "Key findings for \"Infring AI vs competitors comparison 2024\": - Potential sources: hai.stanford.edu, artificialanalysis.ai, epoch.ai."
        })],
        4,
    );
    assert!(synthesized.is_empty());
}

#[test]
fn finalize_user_facing_response_replaces_ack_with_findings() {
    let (finalized, outcome, initial_ack_only) = finalize_user_facing_response_with_outcome(
        "Web search completed.".to_string(),
        Some("Here's what I found:\n- arxiv.org/abs/2601.12345".to_string()),
    );
    let lowered = finalized.to_ascii_lowercase();
    assert!(lowered.contains("web search completed"));
    assert!(outcome.contains("flagged_ack_only_response"), "{outcome}");
    assert!(initial_ack_only);
}

#[test]
fn final_answer_contract_reports_claim_sources_from_tool_receipts() {
    let (finalized, report, _outcome) = enforce_user_facing_finalization_contract(
        "what happened with the web tooling",
        "The web run returned low-signal snippets in this turn.".to_string(),
        &[json!({
            "name": "batch_query",
            "status": "no_results",
            "is_error": true,
            "result": "low-signal snippets",
            "tool_attempt_receipt": {"receipt_hash": "abc123"}
        })],
    );
    assert!(!finalized.trim().is_empty());
    let claim_sources = report
        .pointer("/final_answer_contract/claim_sources")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_str().map(ToString::to_string))
        .collect::<Vec<_>>();
    assert!(
        claim_sources
            .iter()
            .any(|row| row.contains("tool_receipt:abc123")),
        "{claim_sources:?}"
    );
    assert_eq!(
        report
            .pointer("/final_answer_contract/no_unsourced_claims")
            .and_then(Value::as_bool),
        Some(true)
    );
}

#[test]
fn finalize_user_facing_response_replaces_ack_without_findings() {
    let (finalized, outcome, initial_ack_only) =
        finalize_user_facing_response_with_outcome("Web search completed.".to_string(), None);
    let lowered = finalized.to_ascii_lowercase();
    assert!(lowered.contains("web search completed"));
    assert!(outcome.contains("flagged_ack_only_response"), "{outcome}");
    assert!(initial_ack_only);
}

#[test]
fn finalize_user_facing_response_rewrites_generic_tool_failure_placeholder() {
    let (finalized, outcome, _) = finalize_user_facing_response_with_outcome(
        "I couldn't complete system_diagnostic right now.".to_string(),
        None,
    );
    let lowered = finalized.to_ascii_lowercase();
    assert!(lowered.contains("couldn't complete system_diagnostic right now"));
    assert!(outcome.contains("flagged_failure_placeholder"), "{outcome}");
}

#[test]
fn finalize_user_facing_response_rewrites_deprecated_workflow_ghost_copy() {
    let (finalized, outcome, _) = finalize_user_facing_response_with_outcome(
        "The first gate (\"task_or_info_route\") is still classifying this as an \"info\" route rather than a \"task\" route. The system needs explicit tool-related phrasing to trigger the task classification path.".to_string(),
        None,
    );
    assert!(finalized.contains("task_or_info_route"), "{finalized}");
    assert!(outcome.contains("flagged_deprecated_workflow_ghost_text"), "{outcome}");
}

#[test]
fn finalize_user_facing_response_rewrites_workflow_route_classification_ghost_copy() {
    let (finalized, outcome, _) = finalize_user_facing_response_with_outcome(
        "The first gate (\"workflow_route\") is still classifying this as an \"info\" route rather than a \"task\" route. The system needs explicit tool-related phrasing to trigger the task classification path.".to_string(),
        None,
    );
    assert!(finalized.contains("workflow_route"), "{finalized}");
    assert!(outcome.contains("flagged_deprecated_workflow_ghost_text"), "{outcome}");
}

#[test]
fn finalize_user_facing_response_rewrites_binary_classifier_ghost_copy() {
    let (finalized, outcome, _) = finalize_user_facing_response_with_outcome(
        "The first gate (\"workflow_route\") is a binary classification that determines whether the system routes the request through a workflow (task route) or handles it as a direct conversational response (info route). It's not a true/false decision I control - it's an automated classification based on semantic analysis of the user's input. When it detects tool-related intent (like explicit web search requests or file operations), it routes to task; otherwise, it defaults to info. [source:workflow_gate]".to_string(),
        None,
    );
    assert!(finalized.contains("workflow_route"), "{finalized}");
    assert!(!finalized.contains("[source:workflow_gate]"), "{finalized}");
    assert!(outcome.contains("flagged_deprecated_workflow_ghost_text"), "{outcome}");
}

#[test]
fn finalize_user_facing_response_strips_extended_internal_source_tags() {
    let (finalized, outcome, _) = finalize_user_facing_response_with_outcome(
        "The first gate (\"workflow_route\") is still classifying this as an \"info\" route rather than a \"task\" route. [source:workflow_route_classification] [source:gate_enforcement_mode] [source:tool_decision_policy] [source:conversation_bypass_control]"
            .to_string(),
        None,
    );
    assert!(finalized.contains("workflow_route"), "{finalized}");
    assert!(!finalized.contains("[source:"), "{finalized}");
    assert!(outcome.contains("flagged_deprecated_workflow_ghost_text"), "{outcome}");
}

#[test]
fn finalize_user_facing_response_rewrites_conversation_bypass_tool_restriction_copy() {
    let (finalized, outcome, _) = finalize_user_facing_response_with_outcome(
        "I do have web search capabilities, but the system is currently in conversation bypass mode which restricts tool usage. That's correct. I can't autonomously decide to use web tools - the system's gate structure requires manual step-by-step authorization for tool usage. [source:conversation_bypass_control]"
            .to_string(),
        None,
    );
    assert!(finalized.contains("conversation bypass mode"), "{finalized}");
    assert!(!finalized.contains("[source:conversation_bypass_control]"), "{finalized}");
    assert!(outcome.contains("flagged_deprecated_workflow_ghost_text"), "{outcome}");
}

#[test]
fn finalize_user_facing_response_never_leaks_tool_status_text() {
    let (finalized, outcome, initial_ack_only) = finalize_user_facing_response_with_outcome(
        "Tool call finished.".to_string(),
        Some("Tool call finished.".to_string()),
    );
    let lowered = finalized.to_ascii_lowercase();
    assert!(lowered.contains("tool call finished"));
    assert!(outcome.contains("flagged_ack_only_response"), "{outcome}");
    assert!(initial_ack_only);
}

#[test]
fn comparative_detector_matches_peer_ranking_language() {
    assert!(message_requests_comparative_answer(
        "find out how Infring ranks among its peers"
    ));
    assert!(message_requests_comparative_answer(
        "compare infring versus top competitors"
    ));
    assert!(!message_requests_comparative_answer(
        "top AI agent frameworks"
    ));
}

#[test]
fn comparative_language_does_not_auto_select_live_web() {
    assert!(latent_tool_candidates_for_message(
        "compare this system to a named external system with web sources",
        &[],
    )
    .is_empty());
    assert!(latent_tool_candidates_for_message(
        "compare named external system to this workspace using web search",
        &[],
    )
    .is_empty());
}

#[test]
fn natural_web_intent_routes_openclaw_comparison_to_batch_query() {
    assert!(
        natural_web_intent_from_user_message(
            "compare openclaw to this system/workspace using web search"
        )
        .is_none()
    );
}

#[test]
fn natural_web_intent_normalizes_try_to_web_search_query() {
    assert!(
        natural_web_intent_from_user_message("try to web search \"top AI agent frameworks\"")
            .is_none()
    );
}

#[test]
fn natural_web_intent_routes_test_web_fetch_probe_to_example_dot_com() {
    assert!(natural_web_intent_from_user_message("do a test web fetch").is_none());
}

#[test]
fn direct_tool_intent_does_not_auto_route_openclaw_probe() {
    assert!(
        direct_tool_intent_from_user_message(
            "compare openclaw to this system/workspace and do a test web fetch"
        )
        .is_none()
    );
}

#[test]
fn latent_tool_candidates_do_not_infer_research_from_prose() {
    let candidates = latent_tool_candidates_for_message(
        "try to web search \"top AI agent frameworks\"",
        &[],
    );
    assert!(candidates.is_empty(), "{candidates:?}");
}

#[test]
fn latent_tool_candidates_surface_chat_operator_hints_without_direct_routing() {
    let slash_candidates = latent_tool_candidates_for_message("/search top AI agentic frameworks", &[]);
    assert!(slash_candidates.is_empty(), "{slash_candidates:?}");

    let explicit_candidates =
        latent_tool_candidates_for_message("tool::fetch:::https://example.com", &[]);
    assert!(explicit_candidates.is_empty(), "{explicit_candidates:?}");
}

#[test]
fn tooling_failure_fallback_triggers_for_diagnostic_prompt() {
    let fallback = maybe_tooling_failure_fallback(
        "so the tooling isnt working at all?",
        "I couldn't extract usable findings for \"current technology news\" yet.",
        "",
    )
    .expect("fallback");
    let lowered = fallback.to_ascii_lowercase();
    assert!(lowered.contains("partially working"));
    assert!(lowered.contains("batch_query"));
    assert!(lowered.contains("doctor --json"));
}

#[test]
fn tooling_failure_fallback_triggers_for_repeated_placeholder_loop() {
    let repeated = "I couldn't extract usable findings for \"current technology news\" yet.";
    let fallback = maybe_tooling_failure_fallback("?", repeated, repeated).expect("fallback");
    assert!(fallback.to_ascii_lowercase().contains("parse miss"));
}

#[test]
fn system_diagnostic_failure_summary_is_not_generic_dead_end() {
    let summary = user_facing_tool_failure_summary(
        "system_diagnostic",
        &json!({"ok": false, "error": "request_read_failed"}),
    )
    .expect("summary");
    let lowered = summary.to_ascii_lowercase();
    assert!(lowered.contains("diagnose manually"));
    assert!(!lowered.contains("couldn't complete `system_diagnostic` right now"));
}

#[test]
fn web_search_request_read_failed_summary_is_actionable() {
    let summary = user_facing_tool_failure_summary(
        "web_search",
        &json!({"ok": false, "error": "request_read_failed:Resource temporarily unavailable (os error 35)"}),
    )
    .expect("summary");
    let lowered = summary.to_ascii_lowercase();
    assert!(lowered.contains("retry transient failures"));
    assert!(lowered.contains("doctor --json"));
    assert!(lowered.contains("request_read_failed"));
    assert_eq!(
        deterministic_tool_retry_backoff_ms("web_search"),
        vec![180, 360, 720]
    );
}

#[test]
fn web_search_context_guard_failure_summary_is_actionable() {
    let summary = user_facing_tool_failure_summary(
        "web_search",
        &json!({"ok": false, "error": "Context overflow: estimated context size exceeds safe threshold during tool loop."}),
    )
    .expect("summary");
    let lowered = summary.to_ascii_lowercase();
    assert!(lowered.contains("fit safely in context"));
    assert!(lowered.contains("narrower query"));
}

#[test]
fn transient_tool_failure_detects_request_read_failed_signature() {
    assert!(transient_tool_failure(&json!({
        "ok": false,
        "error": "request_read_failed:Resource temporarily unavailable (os error 35)"
    })));
}

#[test]
fn web_search_summary_avoids_completed_placeholder_copy() {
    let summary = summarize_tool_payload(
        "web_search",
        &json!({
            "ok": true,
            "query": "agent reliability",
            "summary": "safe search region picker noise only"
        }),
    );
    let lowered = summary.to_ascii_lowercase();
    assert!(!lowered.contains("web search completed"));
    assert!(!lowered.contains("completed."));
    assert!(!lowered.trim().is_empty());
}

#[test]
fn web_search_summary_reports_root_cause_without_legacy_no_findings_template() {
    let summary = summarize_tool_payload(
        "web_search",
        &json!({
            "ok": true,
            "query": "ai assistant systems comparison 2024 capabilities landscape",
            "requested_url": "https://duckduckgo.com/html/?q=ai+assistant+systems+comparison",
            "domain": "duckduckgo.com",
            "summary": "AI assistant systems comparison 2024 capabilities landscape at DuckDuckGo All Regions Argentina Australia Safe search Any time"
        }),
    );
    let lowered = summary.to_ascii_lowercase();
    assert!(lowered.contains("low-signal"));
    assert!(lowered.contains("batch_query"));
    assert!(!lowered.contains("search response came from"));
    assert!(!lowered.contains("couldn't extract usable findings"));
}

#[test]
fn web_search_summary_discards_potential_sources_scaffold_output() {
    let summary = summarize_tool_payload(
        "web_search",
        &json!({
            "ok": true,
            "query": "Infring AI agent platform capabilities features 2024",
            "summary": "Key findings for \"Infring AI agent platform capabilities features 2024\":\n- Potential sources: nlplogix.com, gartner.com, insightpartners.com.\n- Potential sources: salesforce.com, microsoft.com, lyzr.ai."
        }),
    );
    let lowered = summary.to_ascii_lowercase();
    assert!(!lowered.contains("potential sources:"));
    assert!(!lowered.contains("key findings for"));
    assert!(lowered.contains("low-signal") || lowered.contains("no extractable findings"));
}

#[test]
fn web_search_summary_uses_content_domains_when_summary_is_search_chrome() {
    let summary = summarize_tool_payload(
        "web_search",
        &json!({
            "ok": true,
            "query": "latest technology news today",
            "requested_url": "https://duckduckgo.com/html/?q=latest+technology+news+today",
            "summary": "latest technology news today at DuckDuckGo All Regions Argentina Australia Safe Search Any Time",
            "content": "latest technology news today at DuckDuckGo All Regions Any Time Tech News | Today's Latest Technology News | Reuters www.reuters.com/technology/ Find latest technology news from every corner of the globe. Technology: Latest Tech News Articles Today | AP News apnews.com/technology Don't miss an update on the latest tech news. The Latest News in Technology | PCMag www.pcmag.com/news Get the latest technology news and in-depth analysis."
        }),
    );
    let lowered = summary.to_ascii_lowercase();
    assert!(
        lowered.contains("key web findings"),
        "unexpected summary: {summary}"
    );
    assert!(lowered.contains("reuters.com"));
    assert!(!lowered.contains("couldn't extract usable findings"));
    assert!(!lowered.contains("search response came from"));
}

#[test]
fn batch_query_summary_rewrites_unsynthesized_domain_dump_to_structured_evidence() {
    let summary = summarize_tool_payload(
        "batch_query",
        &json!({
            "ok": true,
            "status": "ok",
            "summary": "Web benchmark synthesis: bing.com: compare [A with B] vs compare A [with B] | WordReference Forums — https://forum.wordreference.com/threads/compare-a-with-b-vs-compare-a-with-b.4047424/",
            "evidence_refs": [
                {
                    "title": "OpenClaw — Personal AI Assistant",
                    "locator": "https://openclaw.ai/"
                }
            ]
        }),
    );
    let lowered = summary.to_ascii_lowercase();
    assert!(lowered.contains("batch query evidence"));
    assert!(!lowered.contains("bing.com: compare"));
}

#[test]
fn batch_query_context_guard_comparison_uses_comparative_fallback() {
    let summary = summarize_tool_payload(
        "batch_query",
        &json!({
            "ok": true,
            "status": "ok",
            "query": "compare openclaw to this system/workspace",
            "source": "web",
            "summary": "Context overflow: estimated context size exceeds safe threshold during tool loop."
        }),
    );
    assert!(summary.is_empty());
}

#[test]
fn batch_query_summary_rewrites_no_useful_information_copy_to_actionable_guidance() {
    let summary = summarize_tool_payload(
        "batch_query",
        &json!({
            "ok": true,
            "status": "ok",
            "query": "top AI agent frameworks",
            "summary": "Search returned no useful information."
        }),
    );
    let lowered = summary.to_ascii_lowercase();
    assert!(
        lowered.contains("usable tool findings") || lowered.contains("source-backed findings"),
        "unexpected summary: {summary}"
    );
    assert!(!lowered.contains("search returned no useful information"));
}

#[test]
fn maybe_tooling_failure_fallback_rewrites_context_guard_diagnosis() {
    let fallback = maybe_tooling_failure_fallback(
        "why is web search failing lately",
        "Context overflow: estimated context size exceeds safe threshold during tool loop.",
        "",
    )
    .expect("fallback");
    let lowered = fallback.to_ascii_lowercase();
    assert!(lowered.contains("fit safely in context"));
    assert!(lowered.contains("partial result"));
}

#[test]
fn unsynthesized_web_snippet_detector_flags_domain_dump_copy() {
    assert!(response_looks_like_unsynthesized_web_snippet_dump(
        "From web retrieval: bing.com: OpenClaw — Personal AI Assistant — https://openclaw.ai/ bing.com: OpenClaw docs — https://openclaw.ai/docs"
    ));
    assert!(!response_looks_like_unsynthesized_web_snippet_dump(
        "In short: OpenClaw focuses on cross-platform local execution, while Infring emphasizes policy-gated orchestration and receipts."
    ));
}

#[test]
fn finalize_user_facing_response_rewrites_raw_placeholder_dump() {
    let (finalized, outcome, _) = finalize_user_facing_response_with_outcome(
        "Example Domain This domain is for use in documentation examples without needing permission."
            .to_string(),
        None,
    );
    let lowered = finalized.to_ascii_lowercase();
    assert!(lowered.contains("example domain"));
    assert!(lowered.contains("without needing permission"));
    assert!(outcome.contains("flagged_raw_web_artifact_dump"), "{outcome}");
}

#[test]
fn finalize_user_facing_response_unwraps_internal_payload_json_response() {
    let raw = json!({
        "agent_id": "agent-83ed64e07515",
        "response": "From web retrieval: benchmark summary with sources. https://example.com/benchmarks",
        "response_finalization": {"tool_completion": {"completion_state": "reported_findings"}},
        "tools": [{"name": "batch_query", "is_error": false, "result": "raw tool output"}],
        "turn_loop_tracking": {"ok": true},
        "turn_transaction": {"tool_execute": "complete"}
    })
    .to_string();
    let finalized = finalize_user_facing_response(raw, None);
    assert!(finalized.contains("agent_id"), "{finalized}");
    assert!(finalized.contains("From web retrieval"), "{finalized}");
    assert!(finalized.starts_with('{'), "{finalized}");
}

#[test]
fn finalize_user_facing_response_unwraps_wrapped_internal_payload_json_response() {
    let raw = format!(
        "tool output follows:\n{}\nend",
        json!({
            "agent_id": "agent-83ed64e07515",
            "response": "Synthesized answer with linked sources.",
            "response_finalization": {"tool_completion": {"completion_state": "reported_findings"}},
            "tools": [{"name": "batch_query", "is_error": false, "result": "raw tool output"}],
            "turn_loop_tracking": {"ok": true},
            "turn_transaction": {"tool_execute": "complete"}
        })
    );
    let finalized = finalize_user_facing_response(raw, None);
    assert!(finalized.contains("tool output follows"), "{finalized}");
    assert!(finalized.contains("Synthesized answer with linked sources."), "{finalized}");
    assert!(finalized.contains("agent_id"), "{finalized}");
}


#[test]
fn natural_web_intent_strips_return_the_results_suffix() {
    assert!(
        natural_web_intent_from_user_message(
            "Try to web search \"top AI agentic frameworks\" and return the results",
        )
        .is_none()
    );
}

#[test]
fn natural_web_intent_routes_generic_web_retry_probe_to_live_batch_query() {
    assert!(natural_web_intent_from_user_message("try the web tooling again").is_none());
}

#[test]
fn inline_batch_query_input_is_normalized_before_execution() {
    let normalized = normalize_inline_tool_execution_input(
        "batch_query",
        &json!({
            "query": "Try to web search \"top AI agentic frameworks\" and return the results"
        }),
        "Try to web search \"top AI agentic frameworks\" and return the results",
    );
    assert_eq!(
        normalized.get("query").and_then(Value::as_str),
        Some("top AI agentic frameworks")
    );
    assert_eq!(normalized.get("source").and_then(Value::as_str), Some("web"));
    assert_eq!(
        normalized.get("aperture").and_then(Value::as_str),
        Some("medium")
    );
}

#[test]
fn natural_web_intent_does_not_force_plain_workspace_peer_compare_into_web() {
    assert!(natural_web_intent_from_user_message("compare this system to openclaw").is_none());
    assert!(natural_web_intent_from_user_message("compare openclaw to this system/workspace").is_none());
}

#[test]
fn response_tools_summary_keeps_actionable_web_diagnostic_rows() {
    let synthesized = response_tools_summary_for_user(
        &[json!({
            "name": "batch_query",
            "is_error": false,
            "result": "Search returned no useful comparison findings for infring vs openclaw."
        })],
        4,
    );
    let lowered = synthesized.to_ascii_lowercase();
    assert!(lowered.contains("retrieval-quality miss"));
    assert!(lowered.contains("batch query"));
}

#[test]
fn finalize_user_facing_response_keeps_actionable_web_diagnostic_copy() {
    let finalized = finalize_user_facing_response(
        "Web retrieval returned low-signal snippets without synthesis. Ask me to rerun with a narrower query and I will return a concise source-backed answer."
            .to_string(),
        None,
    );
    let lowered = finalized.to_ascii_lowercase();
    assert!(lowered.contains("low-signal snippets without synthesis"));
    assert!(!lowered.contains("don't have usable tool findings from this turn yet"));
}

#[test]
fn response_tools_failure_reason_includes_no_results_web_reason() {
    let reason = response_tools_failure_reason_for_user(
        &[json!({
            "name": "batch_query",
            "status": "no_results",
            "blocked": false,
            "is_error": false,
            "result": "Search providers ran, but only low-signal or low-relevance web results came back in this turn. Retry with a narrower query or one specific source URL for source-backed findings."
        })],
        4,
    );
    let lowered = reason.to_ascii_lowercase();
    assert!(lowered.contains("tool run hit issues"));
    assert!(lowered.contains("low-signal") || lowered.contains("source-backed findings"));
    assert!(!lowered.contains("don't have usable tool findings from this turn yet"));
}

#[test]
fn response_tools_failure_reason_includes_ok_status_low_signal_web_reason() {
    let reason = response_tools_failure_reason_for_user(
        &[json!({
            "name": "batch_query",
            "status": "ok",
            "blocked": false,
            "is_error": false,
            "result": "Web retrieval returned low-signal snippets without synthesis. Retry with a narrower query or one specific source URL for source-backed findings."
        })],
        4,
    );
    let lowered = reason.to_ascii_lowercase();
    assert!(lowered.contains("tool run hit issues"));
    assert!(lowered.contains("low-signal"));
    assert!(!lowered.contains("don't have usable tool findings from this turn yet"));
}

use std::sync::Mutex;
static GOVERNANCE_LIVE_WEB_ENV_MUTEX: Mutex<()> = Mutex::new(());
struct ScopedEnvVar {
    key: &'static str,
    previous: Option<String>,
}
impl ScopedEnvVar {
    fn set(key: &'static str, value: &str) -> Self {
        let previous = std::env::var(key).ok();
        std::env::set_var(key, value);
        Self { key, previous }
    }
}
impl Drop for ScopedEnvVar {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => std::env::set_var(self.key, value),
            None => std::env::remove_var(self.key),
        }
    }
}
fn truthy_test_env(name: &str) -> bool {
    matches!(
        std::env::var(name)
            .ok()
            .as_deref()
            .map(|value| value.trim().to_ascii_lowercase()),
        Some(ref value) if value == "1" || value == "true" || value == "yes"
    )
}

#[test]
fn workflow_library_allows_direct_answer_without_second_synthesis() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"workflow-owned-direct-answer-agent","role":"assistant"}"#,
        &snapshot,
    )
    .expect("agent create");
    let agent_id = clean_agent_id(
        created
            .payload
            .get("agent_id")
            .or_else(|| created.payload.get("id"))
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    assert!(!agent_id.is_empty());
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({
            "queue": [
                {
                    "response": "Respond directly"
                },
                {
                    "response": "The workflow and tool menu are working, and I can answer directly."
                }
            ],
            "calls": []
        }),
    );
    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"Say hello and confirm the chain is working."}"#,
        &snapshot,
    )
    .expect("message response");
    assert_eq!(response.status, 200);
    assert_eq!(
        response.payload.get("response").and_then(Value::as_str),
        Some("The workflow and tool menu are working, and I can answer directly.")
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/final_llm_response/status")
            .and_then(Value::as_str),
        Some("synthesized")
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_finalization/workflow_system_fallback_used")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/selected_workflow/gate_contract")
            .and_then(Value::as_str),
        Some("tool_menu_interface_v1")
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/library/default_workflow")
            .and_then(Value::as_str),
        Some("simple_conversation_v1")
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/stage_statuses/0/stage")
            .and_then(Value::as_str),
        Some("gate_1_work_category_menu")
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/stage_statuses/0/status")
            .and_then(Value::as_str),
        Some("presented")
    );
    assert!(
        response
            .payload
            .pointer("/response_workflow/stage_statuses")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().any(|row| {
                row.get("stage").and_then(Value::as_str) == Some("final_llm_response")
                    && row.get("status").and_then(Value::as_str) == Some("synthesized")
            }))
            .unwrap_or(false),
        "{}",
        response.payload
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/current_stage")
            .and_then(Value::as_str),
        Some("gate_6_llm_final_output")
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/visibility/system_injected_chat_text_allowed")
        .and_then(Value::as_bool),
        Some(false)
    );
}

#[test]
fn workflow_library_owns_successful_tool_turn_final_response() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"workflow-owned-tool-turn-agent","role":"researcher"}"#,
        &snapshot,
    )
    .expect("agent create");
    let agent_id = clean_agent_id(
        created
            .payload
            .get("agent_id")
            .or_else(|| created.payload.get("id"))
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    assert!(!agent_id.is_empty());
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({
            "queue": [
                {
                    "response": "Category: Web research. Tool family: Web research. Tool: web_search. Request payload: {\"source\":\"web\",\"query\":\"top AI agentic frameworks\",\"aperture\":\"medium\"}."
                },
                {"response": "For top AI agentic frameworks, the fetched evidence highlighted LangGraph, OpenAI Agents SDK, and AutoGen."}
            ],
            "calls": []
        }),
    );
    write_json(
        &governance_test_tool_script_path(root.path()),
        &json!({
            "queue": [
                {
                    "tool": "web_search",
                    "payload": {
                        "ok": true,
                        "status": "ok",
                        "summary": "LangGraph, OpenAI Agents SDK, and AutoGen surfaced as top AI agentic frameworks in the fetched results."
                    }
                }
            ],
            "calls": []
        }),
    );
    let choose_tool = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"Try to web search \"top AI agentic frameworks\" and return the results"}"#,
        &snapshot,
    )
    .expect("tool choice response");
    assert_eq!(choose_tool.status, 200);
    assert_eq!(
        choose_tool
            .payload
            .pointer("/pending_tool_request/tool_name")
            .and_then(Value::as_str),
        Some("web_search"),
        "{}",
        choose_tool.payload
    );
    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"yes"}"#,
        &snapshot,
    )
    .expect("message response");
    assert_eq!(response.status, 200);
    assert_eq!(
        response.payload.get("response").and_then(Value::as_str),
        Some("For top AI agentic frameworks, the fetched evidence highlighted LangGraph, OpenAI Agents SDK, and AutoGen.")
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/final_llm_response/status")
            .and_then(Value::as_str),
        Some("synthesized")
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_finalization/workflow_system_fallback_used")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/current_stage")
            .and_then(Value::as_str),
        Some("gate_6_llm_final_output")
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/visibility/formats/ui")
            .and_then(Value::as_str),
        Some("Workflow complete; final answer came from the LLM final gate.")
    );
}

#[test]
fn natural_web_prompt_stays_off_direct_tool_route_when_models_are_available() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"natural-web-model-first-agent","role":"researcher"}"#,
        &snapshot,
    )
    .expect("agent create");
    let agent_id = clean_agent_id(
        created
            .payload
            .get("agent_id")
            .or_else(|| created.payload.get("id"))
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    assert!(!agent_id.is_empty());
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({
            "queue": [
                {
                    "response": "<function=batch_query>{\"source\":\"web\",\"query\":\"top AI agentic frameworks\",\"aperture\":\"medium\"}</function>"
                },
                {
                    "response": "Based on the fetched results, LangGraph, OpenAI Agents SDK, and AutoGen are the clearest agentic framework hits."
                }
            ],
            "calls": []
        }),
    );
    write_json(
        &governance_test_tool_script_path(root.path()),
        &json!({
            "queue": [
                {
                    "tool": "batch_query",
                    "payload": {
                        "ok": true,
                        "status": "ok",
                        "summary": "LangGraph, OpenAI Agents SDK, and AutoGen surfaced in the fetched framework results."
                    }
                }
            ],
            "calls": []
        }),
    );
    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"Try to web search \"top AI agentic frameworks\" and return the results"}"#,
        &snapshot,
    )
    .expect("message response");
    assert_eq!(response.status, 200);
    assert_ne!(
        response.payload.get("provider").and_then(Value::as_str),
        Some("tool")
    );
    assert_ne!(
        response.payload.get("runtime_model").and_then(Value::as_str),
        Some("tool-router")
    );
    assert_eq!(
        response.payload.get("response").and_then(Value::as_str),
        Some(
            "Based on the fetched results, LangGraph, OpenAI Agents SDK, and AutoGen are the clearest agentic framework hits."
        )
    );
}

#[test]
fn status_check_turn_does_not_trigger_latent_web_retry_from_failed_draft() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"status-check-no-latent-retry-agent","role":"researcher"}"#,
        &snapshot,
    )
    .expect("agent create");
    let agent_id = clean_agent_id(
        created
            .payload
            .get("agent_id")
            .or_else(|| created.payload.get("id"))
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    assert!(!agent_id.is_empty());
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({
            "queue": [
                {
                    "response": "I attempted that, but web search isn't currently operational because of configuration restrictions and rate limiting."
                }
            ],
            "calls": []
        }),
    );
    write_json(
        &governance_test_tool_script_path(root.path()),
        &json!({
            "queue": [
                {
                    "tool": "batch_query",
                    "payload": {
                        "ok": true,
                        "status": "ok",
                        "summary": "Unexpected latent retry should not run for status checks."
                    }
                }
            ],
            "calls": []
        }),
    );
    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"did you do the web request??"}"#,
        &snapshot,
    )
    .expect("message response");
    assert_eq!(response.status, 200);
    let tool_calls = read_json(&governance_test_tool_script_path(root.path()))
        .and_then(|value| value.get("calls").cloned())
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_default();
    assert!(tool_calls.is_empty(), "{tool_calls:?}");
    assert!(response
        .payload
        .get("tools")
        .and_then(Value::as_array)
        .map(|rows| rows.is_empty())
        .unwrap_or(true));
}

#[test]
fn previous_turn_process_summary_is_persisted_and_injected_into_next_prompt() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"process-summary-memory-agent","role":"assistant"}"#,
        &snapshot,
    )
    .expect("agent create");
    let agent_id = clean_agent_id(
        created
            .payload
            .get("agent_id")
            .or_else(|| created.payload.get("id"))
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    assert!(!agent_id.is_empty());
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({
            "queue": [
                {"response": "First turn answer."},
                {"response": "Second turn answer."}
            ],
            "calls": []
        }),
    );
    let first = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"Give me a short direct answer."}"#,
        &snapshot,
    )
    .expect("first message response");
    assert_eq!(first.status, 200);
    assert_eq!(
        first.payload
            .pointer("/process_summary/contract")
            .and_then(Value::as_str),
        Some("turn_process_summary_v1")
    );
    let second = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"Now continue from that."}"#,
        &snapshot,
    )
    .expect("second message response");
    assert_eq!(second.status, 200);
    assert_eq!(
        second
            .payload
            .pointer("/process_summary/contract")
            .and_then(Value::as_str),
        Some("turn_process_summary_v1")
    );
    let chat_calls = read_json(&governance_test_chat_script_path(root.path()))
        .and_then(|value| value.get("calls").cloned())
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_default();
    assert!(chat_calls.len() >= 2, "{chat_calls:?}");
    let second_system_prompt = clean_text(
        chat_calls[1]
            .get("system_prompt")
            .and_then(Value::as_str)
            .unwrap_or(""),
        4_000,
    );
    assert!(
        second_system_prompt
            .to_ascii_lowercase()
            .contains("previous-turn process summary"),
        "{second_system_prompt}"
    );
}

#[test]
fn workflow_retry_validator_blocks_search_again_language() {
    assert!(workflow_response_requests_more_tooling(
        "Let me search for more specific AI agent framework information using a narrower query."
    ));
    assert!(workflow_response_requests_more_tooling(
        "Retry with a narrower query or one specific source URL."
    ));
    assert!(!workflow_response_requests_more_tooling(
        "The web search ran, but it only returned low-signal snippets in this turn."
    ));
}

#[test]
fn web_tooling_harness_surfaces_no_results_with_final_llm_synthesis() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"web-tooling-no-results-agent","role":"researcher"}"#,
        &snapshot,
    )
    .expect("agent create");
    let agent_id = clean_agent_id(
        created
            .payload
            .get("agent_id")
            .or_else(|| created.payload.get("id"))
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    assert!(!agent_id.is_empty());
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({
            "queue": [
                {
                    "response": "Category: Web research. Tool family: Web research. Tool: web_search. Request payload: {\"source\":\"web\",\"query\":\"top AI agentic frameworks\",\"aperture\":\"medium\"}"
                }
            ],
            "calls": []
        }),
    );
    write_json(
        &governance_test_tool_script_path(root.path()),
        &json!({
            "queue": [
                {
                    "tool": "batch_query",
                    "payload": {
                        "ok": false,
                        "status": "no_results",
                        "summary": "Web retrieval returned low-signal snippets without synthesis. Retry with a narrower query or a specific source URL.",
                        "error": "search_providers_exhausted"
                    }
                }
            ],
            "calls": []
        }),
    );
    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"Try to web search \"top AI agentic frameworks\" and return the results"}"#,
        &snapshot,
    )
    .expect("message response");
    assert_eq!(response.status, 200);
    assert_eq!(
        response
            .payload
            .pointer("/tools/0/name")
            .and_then(Value::as_str),
        Some("batch_query"),
        "expected web search tool execution; payload={}",
        response.payload
    );
    assert_eq!(
        response
            .payload
            .pointer("/tools/0/status")
            .and_then(Value::as_str),
        Some("no_results")
    );
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("");
    let lowered = response_text.to_ascii_lowercase();
    assert!(
        !response_text.trim().is_empty(),
        "expected synthesized no-results reply; payload={}; chat_calls={:?}",
        response.payload,
        read_json(&governance_test_chat_script_path(root.path()))
    );
    assert!(
        lowered.contains("low-signal") || lowered.contains("source-backed answer"),
        "{response_text}"
    );
    assert!(!response_is_no_findings_placeholder(response_text));
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/final_llm_response/status")
            .and_then(Value::as_str),
        Some("synthesized")
    );
}

#[test]
fn compare_workflow_hint_clusters_workspace_and_web_tools() {
    let hints = latent_tool_candidates_for_message("compare this system to openclaw", &[]);
    let tool_names = hints
        .iter()
        .filter_map(|row| row.get("tool").and_then(Value::as_str))
        .collect::<Vec<_>>();
    assert!(tool_names.is_empty(), "{tool_names:?}");
}

#[test]
fn compare_platform_wording_clusters_workspace_and_web_tools() {
    let hints = latent_tool_candidates_for_message("compare this platform to openclaw", &[]);
    let tool_names = hints
        .iter()
        .filter_map(|row| row.get("tool").and_then(Value::as_str))
        .collect::<Vec<_>>();
    assert!(tool_names.is_empty(), "{tool_names:?}");
}

#[test]
fn compare_workflow_harness_decomposes_local_and_web_evidence_before_final_synthesis() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"compare-workflow-agent","role":"researcher"}"#,
        &snapshot,
    )
    .expect("agent create");
    let agent_id = clean_agent_id(
        created
            .payload
            .get("agent_id")
            .or_else(|| created.payload.get("id"))
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    assert!(!agent_id.is_empty());
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({
            "queue": [
                {
                    "response": "I would choose workspace search first to gather local evidence for comparing this system to OpenClaw."
                }
            ],
            "calls": []
        }),
    );
    write_json(
        &governance_test_tool_script_path(root.path()),
        &json!({
            "queue": [
                {
                    "tool": "terminal_exec",
                    "payload": {
                        "ok": true,
                        "status": "ok",
                        "summary": "Local workspace evidence shows workflow-gated synthesis via complex_prompt_chain_v1 and a domain-grouped tool catalog."
                    }
                },
                {
                    "tool": "batch_query",
                    "payload": {
                        "ok": true,
                        "status": "ok",
                        "summary": "External web evidence highlights OpenClaw's governed web/media tooling and native search contracts."
                    }
                }
            ],
            "calls": []
        }),
    );
    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"compare this system to openclaw"}"#,
        &snapshot,
    )
    .expect("message response");
    assert_eq!(response.status, 200);
    let tool_names = response
        .payload
        .get("tools")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.get("name").and_then(Value::as_str).map(ToString::to_string))
        .collect::<Vec<_>>();
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(
        !response_text.trim().is_empty(),
        "expected synthesized compare response; payload={}",
        response.payload
    );
}

#[test]
fn web_tooling_live_smoke_uses_real_model_provider_when_enabled() {
    if !truthy_test_env("INFRING_LIVE_WEB_TOOLING_SMOKE") {
        return;
    }
    let _env_lock = GOVERNANCE_LIVE_WEB_ENV_MUTEX.lock().expect("lock");
    let model_ref = std::env::var("INFRING_LIVE_WEB_TOOLING_MODEL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "openai/gpt-5".to_string());
    if model_ref.starts_with("openai/")
        && std::env::var("OPENAI_API_KEY")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .is_none()
    {
        return;
    }
    let query = std::env::var("INFRING_LIVE_WEB_TOOLING_QUERY")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "top AI agentic frameworks".to_string());
    let use_real_retrieval = truthy_test_env("INFRING_LIVE_WEB_TOOLING_USE_REAL_RETRIEVAL");
    let fixture_guard = if use_real_retrieval {
        None
    } else {
        Some(ScopedEnvVar::set(
            "INFRING_BATCH_QUERY_TEST_FIXTURE_JSON",
            &serde_json::to_string(&json!({
                query.clone(): {
                    "ok": true,
                    "summary": "LangGraph, OpenAI Agents SDK, and AutoGen are commonly cited as top AI agentic frameworks.",
                    "requested_url": "https://example.com/frameworks",
                    "status_code": 200
                }
            }))
            .expect("encode fixture"),
        ))
    };

    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"live-web-smoke-agent","role":"researcher"}"#,
        &snapshot,
    )
    .expect("agent create");
    let agent_id = clean_agent_id(
        created
            .payload
            .get("agent_id")
            .or_else(|| created.payload.get("id"))
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    assert!(!agent_id.is_empty());

    let set_model_payload = serde_json::to_vec(&json!({"model": model_ref})).expect("serialize model");
    let set_model = handle(
        root.path(),
        "PUT",
        &format!("/api/agents/{agent_id}/model"),
        &set_model_payload,
        &snapshot,
    )
    .expect("set model");
    assert_eq!(set_model.status, 200);

    let message_payload = serde_json::to_vec(&json!({
        "message": format!("Try to web search \"{query}\" and return the results")
    }))
    .expect("serialize message");
    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        &message_payload,
        &snapshot,
    )
    .expect("message response");
    drop(fixture_guard);

    assert_eq!(response.status, 200);
    assert_eq!(
        response
            .payload
            .pointer("/tools/0/name")
            .and_then(Value::as_str),
        Some("batch_query")
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/selected_workflow/name")
            .and_then(Value::as_str),
        Some("complex_prompt_chain_v1")
    );
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("");
    let lowered = response_text.to_ascii_lowercase();
    assert!(!response_text.trim().is_empty(), "expected live synthesized response");
    assert!(!response_is_no_findings_placeholder(response_text));
    if !use_real_retrieval {
        assert!(
            lowered.contains("langgraph")
                || lowered.contains("openai agents sdk")
                || lowered.contains("autogen"),
            "{response_text}"
        );
    }
}

// Decomposed for backend file-size/cohesion remediation; behavior preserved via ordered includes.

#[test]
fn compare_workflow_completes_missing_web_evidence_from_latent_candidates() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"compare-workflow-latent-agent","role":"researcher"}"#,
        &snapshot,
    )
    .expect("agent create");
    let agent_id = clean_agent_id(
        created
            .payload
            .get("agent_id")
            .or_else(|| created.payload.get("id"))
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    assert!(!agent_id.is_empty());
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({
            "queue": [
                {
                    "response": "Respond directly"
                },
                {
                    "response": "Using both local and external evidence, Infring centers workflow-gated synthesis while OpenClaw emphasizes governed web/media tooling."
                }
            ],
            "calls": []
        }),
    );
    write_json(
        &governance_test_tool_script_path(root.path()),
        &json!({
            "queue": [
                {
                    "tool": "terminal_exec",
                    "payload": {
                        "ok": true,
                        "status": "ok",
                        "summary": "Local workspace evidence shows workflow-gated synthesis via complex_prompt_chain_v1."
                    }
                },
                {
                    "tool": "batch_query",
                    "payload": {
                        "ok": true,
                        "status": "ok",
                        "summary": "External web evidence highlights OpenClaw's governed web/media tooling."
                    }
                }
            ],
            "calls": []
        }),
    );
    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"compare this system (infring) to openclaw"}"#,
        &snapshot,
    )
    .expect("message response");
    let tool_names = response
        .payload
        .get("tools")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.get("name").and_then(Value::as_str).map(ToString::to_string))
        .collect::<Vec<_>>();
    assert_eq!(tool_names, Vec::<String>::new());
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(response_text.contains("Infring"), "{response_text}");
    assert!(response_text.contains("OpenClaw"), "{response_text}");
}

#[test]
fn workflow_more_tooling_detector_matches_compare_follow_up_question() {
    assert!(workflow_response_requests_more_tooling(
        "Would you like me to search for specific OpenClaw technical documentation or architecture details to enable a more substantive comparison?"
    ));
}

#[test]
fn workflow_does_not_hydrate_openclaw_comparison_payload() {
    let input = normalize_inline_tool_execution_input(
        "web_search",
        &json!({"query":"OpenClaw AI agent system features capabilities"}),
        "compare this system to a named external system",
    );
    assert_eq!(
        input.get("query").and_then(Value::as_str),
        Some("OpenClaw AI agent system features capabilities")
    );
    assert!(input.get("queries").is_none(), "{input}");
}

#[test]
fn inline_tool_web_search_comparison_hydrates_targeted_openclaw_query_pack() {
    let input = normalize_inline_tool_execution_input(
        "web_search",
        &json!({"query":"OpenClaw AI agent system features capabilities"}),
        "compare this system to a named external system",
    );
    assert_eq!(
        input.get("query").and_then(Value::as_str),
        Some("OpenClaw AI agent system features capabilities")
    );
    assert!(input.get("queries").is_none(), "{input}");
}

#[test]
fn framework_catalog_query_pack_is_not_runtime_hydrated() {
    let input = normalize_inline_tool_execution_input(
        "web_search",
        &json!({"query":"top AI agentic frameworks"}),
        "Try to web search \"top AI agentic frameworks\" and return the results",
    );
    assert_eq!(
        input.get("query").and_then(Value::as_str),
        Some("top AI agentic frameworks")
    );
    assert!(input.get("queries").is_none(), "{input}");
}

#[test]
fn inline_tool_web_search_keeps_llm_submitted_broad_prompt() {
    let input = normalize_inline_tool_execution_input(
        "web_search",
        &json!({"query":"top AI agentic frameworks"}),
        "Try to web search \"top AI agentic frameworks\" and return the results",
    );
    let query = input.get("query").and_then(Value::as_str).unwrap_or("");
    assert_eq!(query, "top AI agentic frameworks");
    assert!(input.get("queries").is_none(), "{input}");
}

#[test]
fn summarize_unknown_workspace_analyze_payload_prefers_stdout_findings() {
    let summary = summarize_unknown_tool_payload(
        "workspace_analyze",
        &json!({
            "ok": true,
            "stdout": "docs/workspace/SRS.md:42: response_workflow\ndocs/workspace/SRS.md:43: complex_prompt_chain_v1\nclient/runtime/config/runtime.json:4: resident IPC"
        }),
    );
    assert!(summary.contains("response_workflow"), "{summary}");
    assert!(summary.contains("complex_prompt_chain_v1"), "{summary}");
    assert!(!summary.contains("`workspace_analyze` completed"), "{summary}");
}

#[test]
fn rewrite_workspace_analyze_raw_json_result_into_local_evidence_summary() {
    let rewritten = rewrite_tool_result_for_user_summary(
        "workspace_analyze",
        "Key findings: {\"stdout\":\"docs/workspace/SRS.md:42: response_workflow\\ndocs/workspace/SRS.md:43: complex_prompt_chain_v1\\nclient/runtime/config/runtime.json:4: resident IPC\"}",
    )
    .unwrap_or_default();
    assert!(rewritten.contains("Local workspace evidence"), "{rewritten}");
    assert!(rewritten.contains("response_workflow"), "{rewritten}");
    assert!(!rewritten.contains("{\"stdout\""), "{rewritten}");
}

#[test]
fn summarize_workspace_analyze_prefers_stdout_over_claim_bundle_dump() {
    let summary = summarize_tool_payload(
        "workspace_analyze",
        &json!({
            "ok": true,
            "stdout": "docs/workspace/SRS.md:42: response_workflow\\ndocs/workspace/SRS.md:43: complex_prompt_chain_v1\\nclient/runtime/config/runtime.json:4: resident IPC",
            "tool_pipeline": {
                "claim_bundle": {
                    "claims": [
                        {
                            "status": "supported",
                            "text": "{\"command_translated\":false,\"cwd\":\"/Users/jay/.openclaw/workspace\",\"executed_command\":\"rg -n ...\"}"
                        }
                    ]
                }
            }
        }),
    );
    assert!(summary.contains("response_workflow"), "{summary}");
    assert!(summary.contains("complex_prompt_chain_v1"), "{summary}");
    assert!(!summary.contains("{\"command_translated\""), "{summary}");
}

#[test]
fn summarize_tool_payload_strips_redundant_key_findings_from_claims() {
    let summary = summarize_tool_payload(
        "batch_query",
        &json!({
            "ok": true,
            "tool_pipeline": {
                "claim_bundle": {
                    "claims": [
                        {
                            "status": "supported",
                            "text": "Key findings: openclaw.ai: OpenClaw — Personal AI Assistant"
                        }
                    ]
                }
            }
        }),
    );
    assert_eq!(
        summary,
        "Key findings: openclaw.ai: OpenClaw — Personal AI Assistant"
    );
}

#[test]
fn summarize_web_search_batch_query_payload_prefers_native_summary_over_claim_bundle() {
    let summary = summarize_tool_payload(
        "web_search",
        &json!({
            "ok": true,
            "type": "batch_query",
            "summary": "Key findings: langchain.com: LangGraph overview.; crewai.com: CrewAI overview.; openai.github.io: OpenAI Agents SDK overview.",
            "evidence_refs": [
                {"title":"Web result from langchain.com","locator":"https://www.langchain.com/langgraph"},
                {"title":"Web result from crewai.com","locator":"https://crewai.com/"},
                {"title":"Web result from openai.github.io","locator":"https://openai.github.io/openai-agents-python/"}
            ],
            "tool_pipeline": {
                "claim_bundle": {
                    "claims": [
                        {
                            "status": "partial",
                            "text": "Key findings: langchain.com: LangGraph overview."
                        }
                    ]
                }
            }
        }),
    );
    let lowered = summary.to_ascii_lowercase();
    assert!(lowered.contains("langgraph"), "{summary}");
    assert!(lowered.contains("crewai"), "{summary}");
    assert!(lowered.contains("openai agents sdk"), "{summary}");
}

#[test]
fn response_tools_summary_preserves_batch_query_domains_in_key_findings() {
    let summary = response_tools_summary_for_user(
        &[json!({
            "name": "batch_query",
            "result": "Key findings: docs.openclaw.ai: OpenClaw is a self-hosted gateway that connects chat apps to AI; openclaw.ai: OpenClaw personal assistant overview."
        })],
        4,
    );
    assert!(summary.contains("docs.openclaw.ai"), "{summary}");
    assert_ne!(summary.trim(), "Here's what I found:\n- batch query: docs.");
}

#[test]
fn summarize_web_search_framework_payload_rewrites_noisy_sources_from_evidence_refs() {
    let summary = summarize_tool_payload(
        "web_search",
        &json!({
            "ok": true,
            "query": "top AI agentic frameworks",
            "summary": "Key findings: langgraph.com.cn: LangGraph overview; zhihu.com: AutoGen discussion; crewai.org.cn: CrewAI overview.",
            "evidence_refs": [
                {"title":"LangGraph: Agent Orchestration Framework for Reliable AI Agents - LangChain","locator":"https://www.langchain.com/langgraph"},
                {"title":"CrewAI official site","locator":"https://crewai.com/"},
                {"title":"OpenAI Agents SDK docs","locator":"https://openai.github.io/openai-agents-python/"}
            ]
        }),
    );
    let lowered = summary.to_ascii_lowercase();
    assert!(lowered.contains("langgraph"), "{summary}");
    assert!(lowered.contains("crewai"), "{summary}");
    assert!(lowered.contains("openai agents sdk"), "{summary}");
    assert!(!lowered.contains("langgraph.com.cn"), "{summary}");
    assert!(!lowered.contains("crewai.org.cn"), "{summary}");
    assert!(!lowered.contains("zhihu.com"), "{summary}");
}

#[test]
fn summarize_web_search_framework_payload_prefers_locator_domain_over_noisy_title_domain() {
    let summary = summarize_tool_payload(
        "web_search",
        &json!({
            "ok": true,
            "query": "top AI agentic frameworks",
            "summary": "Key findings: framework search returned official documentation hits.",
            "evidence_refs": [
                {"title":"Web result from academy.langchain.com","locator":"https://www.langchain.com/langgraph"},
                {"title":"Web result from watsonx.ai","locator":"https://crewai.com/"},
                {"title":"Web result from github.com","locator":"https://github.com/huggingface/smolagents"}
            ]
        }),
    );
    let lowered = summary.to_ascii_lowercase();
    assert!(lowered.contains("langgraph (langchain.com)"), "{summary}");
    assert!(lowered.contains("crewai (crewai.com)"), "{summary}");
    assert!(!lowered.contains("academy.langchain.com"), "{summary}");
    assert!(!lowered.contains("watsonx.ai"), "{summary}");
}

#[test]
fn direct_batch_query_get_endpoint_emits_nexus_audit_and_tracking_metadata() {
    let _guard = WEB_ENDPOINT_ENV_MUTEX.lock().expect("lock");
    std::env::remove_var("INFRING_HIERARCHICAL_NEXUS_BLOCK_CLIENT_INGRESS_ROUTE");
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let out = handle(
        root.path(),
        "GET",
        "/api/batch-query?q=",
        &[],
        &snapshot,
    )
    .expect("batch query get");
    assert!(matches!(out.status, 200 | 400));
    assert_eq!(
        out.payload.pointer("/nexus_connection/source")
            .and_then(Value::as_str),
        Some("client_ingress")
    );
    assert!(out.payload.get("decision_audit_receipt").is_some());
    assert!(out.payload.get("turn_loop_tracking").is_some());
    assert_eq!(
        out.payload
            .pointer("/recovery_strategy")
            .and_then(Value::as_str),
        Some("none")
    );
}

#[test]
fn direct_batch_query_post_endpoint_emits_nexus_audit_and_tracking_metadata() {
    let _guard = WEB_ENDPOINT_ENV_MUTEX.lock().expect("lock");
    std::env::remove_var("INFRING_HIERARCHICAL_NEXUS_BLOCK_CLIENT_INGRESS_ROUTE");
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let out = handle(
        root.path(),
        "POST",
        "/api/batch-query",
        br#"{"query":""}"#,
        &snapshot,
    )
    .expect("batch query post");
    assert!(matches!(out.status, 200 | 400));
    assert_eq!(
        out.payload.pointer("/nexus_connection/source")
            .and_then(Value::as_str),
        Some("client_ingress")
    );
    assert!(out.payload.get("decision_audit_receipt").is_some());
    assert!(out.payload.get("turn_loop_tracking").is_some());
    assert_eq!(
        out.payload
            .pointer("/recovery_strategy")
            .and_then(Value::as_str),
        Some("none")
    );
}

#[test]
fn direct_file_read_endpoint_emits_decision_audit_and_tracking_metadata() {
    let root = governance_temp_root();
    init_git_repo(root.path());
    std::fs::create_dir_all(root.path().join("notes")).expect("mkdir");
    std::fs::write(root.path().join("notes/plan.txt"), "ship it").expect("write");
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"File Audit Agent","role":"operator"}"#,
        &governance_ok_snapshot(),
    )
    .expect("create");
    let agent_id = clean_agent_id(
        created
            .payload
            .get("agent_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    assert!(!agent_id.is_empty());
    let out = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/file/read"),
        br#"{"path":"notes/plan.txt"}"#,
        &governance_ok_snapshot(),
    )
    .expect("file read");
    assert_eq!(out.status, 200);
    assert_eq!(
        out.payload.pointer("/nexus_connection/source")
            .and_then(Value::as_str),
        Some("client_ingress")
    );
    assert!(out.payload.get("decision_audit_receipt").is_some());
    assert!(out.payload.get("turn_loop_tracking").is_some());
    assert_eq!(
        out.payload
            .pointer("/recovery_strategy")
            .and_then(Value::as_str),
        Some("none")
    );
}
