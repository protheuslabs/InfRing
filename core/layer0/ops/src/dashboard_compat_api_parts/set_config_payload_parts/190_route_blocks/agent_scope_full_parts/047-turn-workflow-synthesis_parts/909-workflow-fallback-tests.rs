    #[test]
    fn structured_no_tool_gate_submission_preserves_llm_final_answer() {
        let response = r#"{"gate":1,"token":"1","final_answer":"Hey there - I'm here and ready."}"#;

        assert!(response_is_exact_no_tool_gate_submission(response));
        assert_eq!(
            workflow_structured_gate_final_answer(response),
            Some("Hey there - I'm here and ready.".to_string())
        );
        assert_eq!(
            workflow_structured_gate_final_answer(r#""gate_6_final_answer": "Hey, I'm here!""#),
            Some("Hey, I'm here!".to_string())
        );
    }

    #[test]
    fn compact_source_refs_are_persisted_from_usable_tool_evidence() {
        let tools = vec![json!({
            "name": "batch_query",
            "status": "ok",
            "evidence_pack": [
                {
                    "title": "Enterprise agent platform report",
                    "locator": "https://market.example.test/agent-platform-report-2026",
                    "source_domain": "market.example.test",
                    "source_kind": "industry_report",
                    "counts_as_usable_evidence": true,
                    "confidence": "usable"
                },
                {
                    "title": "Low signal placeholder",
                    "locator": "tool:low-signal",
                    "source_domain": "",
                    "counts_as_usable_evidence": true,
                    "confidence": "usable"
                }
            ],
            "evidence_refs": [
                {
                    "title": "Runtime orchestration release notes",
                    "locator": "https://runtime.example.test/release-notes-2026",
                    "source_domain": "runtime.example.test",
                    "source_kind": "release_notes"
                }
            ]
        })];
        let mut workflow = json!({});

        persist_workflow_compact_source_refs(&mut workflow, &tools);

        let root_refs = workflow
            .get("source_refs")
            .and_then(Value::as_array)
            .expect("root source refs");
        assert_eq!(root_refs.len(), 2, "{root_refs:#?}");
        assert_eq!(
            workflow
                .pointer("/response_workflow/final_llm_response/source_refs")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(2)
        );
        assert_eq!(
            workflow
                .pointer("/response_finalization/final_response/source_refs")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(2)
        );
        assert_eq!(
            root_refs[0].get("title").and_then(Value::as_str),
            Some("Enterprise agent platform report")
        );
        assert_eq!(
            root_refs[1].get("title").and_then(Value::as_str),
            Some("Runtime orchestration release notes")
        );
    }
