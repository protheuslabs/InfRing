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
        Some(true)
    );
    assert_eq!(
        decision.get("should_call_tools").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        decision
            .get("selected_work_category")
            .and_then(Value::as_str),
        Some("web_research")
    );
}

#[test]
