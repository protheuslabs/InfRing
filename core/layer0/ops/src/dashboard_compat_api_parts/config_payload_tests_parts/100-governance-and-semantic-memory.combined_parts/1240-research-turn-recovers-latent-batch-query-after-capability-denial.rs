#[test]
fn research_turn_recovers_latent_batch_query_after_capability_denial() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"latent-research-recovery-agent","role":"researcher"}"#,
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
                    "response": "I don't have access to current news sources or browsing tools to retrieve real-time global news from May 2026. My knowledge cutoff limits me from accessing events that have occurred after my training data."
                },
                {
                    "response": "In May 2026, the retrieved global coverage centered on three themes: trade and labor disruptions, major AI platform releases, and energy-market volatility. The safest source-backed summary is that those issues dominated the international news cycle across the recorded results."
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
                        "summary": "Reuters, AP, and BBC coverage from May 2026 highlighted global labor actions, major AI platform announcements, and energy-market volatility."
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
        br#"{"message":"tell me some global news from may 2026"}"#,
        &snapshot,
    )
    .expect("message response");
    assert_eq!(response.status, 200);
    let tools = response
        .payload
        .get("tools")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(
        tools.iter()
            .any(|row| row.get("name").and_then(Value::as_str) == Some("batch_query")),
        "{tools:?}"
    );
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(
        response_text.contains("May 2026"),
        "{response_text}"
    );
    assert!(
        response_text.contains("Reuters")
            || response_text.contains("AP")
            || response_text.contains("BBC")
            || response_text.contains("labor")
            || response_text.contains("energy-market"),
        "{response_text}"
    );
    let lowered = response_text.to_ascii_lowercase();
    assert!(!lowered.contains("knowledge cutoff"), "{response_text}");
    assert!(
        !lowered.contains("don't have access to current news sources or browsing tools"),
        "{response_text}"
    );
    assert!(
        !lowered.contains("do not have enough reliable information from this turn"),
        "{response_text}"
    );
    assert!(
        matches!(
            response
                .payload
                .pointer("/response_workflow/final_llm_response/status")
                .and_then(Value::as_str),
            Some("synthesized") | Some("tool_evidence_fallback_used")
        ),
        "{:?}",
        response.payload.pointer("/response_workflow/final_llm_response/status")
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_finalization/workflow_system_fallback_used")
            .and_then(Value::as_bool),
        Some(false)
    );
}
