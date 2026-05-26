fn mark_workflow_direct_llm_no_tool_answer(workflow: &mut Value) {
    let contract = default_workflow_tool_menu_contract();
    let first_gate_id = workflow_first_gate_id(&contract);
    let Some(direct_option) = workflow_gate_options(&contract, &first_gate_id)
        .into_iter()
        .find(|option| option.get("has_tools").and_then(Value::as_bool) == Some(false))
    else {
        workflow["final_llm_response"]["direct_answer_marker_error"] =
            Value::String("workflow_cd_missing_no_tool_option".to_string());
        return;
    };
    let direct_key = workflow_option_key(&direct_option);
    let direct_label = workflow_option_label(&direct_option);
    let gate_submission = json!({
        "accepted": true,
        "gate_id": first_gate_id.clone(),
        "llm_submission": direct_label,
        "resume_token": workflow_gate_resume_token(&first_gate_id, "submitted"),
        "decision_source": "llm_direct_answer"
    });
    workflow["workflow_control"]["direct_response_path"] =
        Value::String("first_gate_no_tool_category".to_string());
    workflow["tool_gate"]["selected_work_category"] = Value::String(direct_key);
    workflow["tool_gate"]["selected_tool_family"] = Value::String("none".to_string());
    workflow["tool_gate"]["gate_1_submission_status"] = Value::String("submitted".to_string());
    workflow["tool_gate"]["gate_1_decision_source"] =
        Value::String("llm_direct_answer".to_string());
    workflow["tool_gate"]["gate_submission"] = gate_submission.clone();
    mark_workflow_gate_row_submission(
        workflow,
        &first_gate_id,
        "submitted",
        "llm_direct_answer",
        gate_submission,
    );
    workflow["tool_gate"]["info_source"] = Value::String("llm_direct_answer".to_string());
    if let Some(rows) = workflow
        .get_mut("stage_statuses")
        .and_then(Value::as_array_mut)
    {
        for row in rows.iter_mut() {
            if row
                .get("stage")
                .and_then(Value::as_str)
                .map(|stage| stage == "gate_1_work_category_menu")
                .unwrap_or(false)
            {
                row["status"] = Value::String("answered_no_tool_category".to_string());
                row["decision_source"] = Value::String("llm_direct_answer".to_string());
            }
        }
    }
}

fn mark_workflow_gate_row_submission(
    workflow: &mut Value,
    gate_id: &str,
    submission_status: &str,
    decision_source: &str,
    gate_submission: Value,
) {
    let Some(gates) = workflow
        .get_mut("tool_gate")
        .and_then(|tool_gate| tool_gate.get_mut("gates"))
    else {
        return;
    };
    if let Some(gate_map) = gates.as_object_mut() {
        let gate_row = gate_map
            .entry(gate_id.to_string())
            .or_insert_with(|| json!({}));
        gate_row["submission_status"] = Value::String(submission_status.to_string());
        gate_row["decision_source"] = Value::String(decision_source.to_string());
        gate_row["gate_submission"] = gate_submission;
        return;
    }
    if let Some(gate_rows) = gates.as_array_mut() {
        let mut updated = false;
        for row in gate_rows.iter_mut() {
            let row_gate_id = row
                .get("gate_id")
                .or_else(|| row.get("id"))
                .and_then(Value::as_str)
                .unwrap_or("");
            if row_gate_id == gate_id {
                row["submission_status"] = Value::String(submission_status.to_string());
                row["decision_source"] = Value::String(decision_source.to_string());
                row["gate_submission"] = gate_submission.clone();
                updated = true;
            }
        }
        if !updated {
            gate_rows.push(json!({
                "gate_id": gate_id,
                "submission_status": submission_status,
                "decision_source": decision_source,
                "gate_submission": gate_submission
            }));
        }
    }
}

