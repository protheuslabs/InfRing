#[derive(Clone, Debug)]
struct WorkflowDefinition {
    name: String,
    workflow_type: String,
    default_workflow: bool,
    description: String,
    stages: Vec<String>,
    final_response_policy: String,
    gate_contract: String,
    workflow_composition_contract: Value,
    workflow_source_of_truth_contract: Value,
    tool_menu_interface_contract: Value,
    final_output_contract: Value,
    source_path: String,
}

#[derive(Clone, Debug)]
struct AuxiliaryWorkflowDefinition {
    name: String,
    workflow_type: String,
    description: String,
    workflow_composition_contract: Value,
    workflow_source_of_truth_contract: Value,
    final_output_contract: Value,
    source_path: String,
}


fn workflow_definition_to_json(definition: &WorkflowDefinition) -> Value {
    json!({
        "name": definition.name,
        "workflow_type": definition.workflow_type,
        "default": definition.default_workflow,
        "description": definition.description,
        "stages": definition.stages,
        "final_response_policy": definition.final_response_policy,
        "gate_contract": definition.gate_contract,
        "workflow_composition_contract": definition.workflow_composition_contract,
        "workflow_source_of_truth_contract": definition.workflow_source_of_truth_contract,
        "tool_menu_interface_contract": definition.tool_menu_interface_contract,
        "final_output_contract": definition.final_output_contract,
        "source_path": definition.source_path
    })
}

fn auxiliary_workflow_definition_to_json(definition: &AuxiliaryWorkflowDefinition) -> Value {
    json!({
        "name": definition.name,
        "workflow_type": definition.workflow_type,
        "description": definition.description,
        "workflow_composition_contract": definition.workflow_composition_contract,
        "workflow_source_of_truth_contract": definition.workflow_source_of_truth_contract,
        "final_output_contract": definition.final_output_contract,
        "source_path": definition.source_path
    })
}

fn workflow_contract_string_at(value: &Value, pointer: &str, max_len: usize) -> Option<String> {
    let row = clean_text(value.pointer(pointer).and_then(Value::as_str)?, max_len);
    (!row.is_empty()).then_some(row)
}

fn workflow_contract_array_at(value: &Value, pointer: &str) -> Option<Vec<Value>> {
    let rows = value.pointer(pointer).and_then(Value::as_array)?.clone();
    (!rows.is_empty()).then_some(rows)
}

fn workflow_contract_object_at(value: &Value, pointer: &str) -> Option<Value> {
    value
        .pointer(pointer)
        .filter(|row| row.is_object())
        .cloned()
}

fn workflow_contract_array_strings_at(value: &Value, pointer: &str) -> Vec<String> {
    value
        .pointer(pointer)
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| clean_text(row, 120))
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn workflow_contract_array_contains(value: &Value, pointer: &str, expected: &str) -> bool {
    let expected = clean_text(expected, 120);
    !expected.is_empty()
        && workflow_contract_array_strings_at(value, pointer)
            .iter()
            .any(|row| row == &expected)
}

fn workflow_trace_status_message_pair_is_complete(contract: &Value, status: &str) -> bool {
    let status = clean_text(status, 120);
    !status.is_empty()
        && workflow_contract_string_at(
            contract,
            &format!("/trace_status_messages/{status}/ui"),
            240,
        )
        .is_some()
        && workflow_contract_string_at(
            contract,
            &format!("/trace_status_messages/{status}/agent_process"),
            240,
        )
        .is_some()
}

fn workflow_tool_menu_contract_is_complete(contract: &Value) -> bool {
    if workflow_contract_string_at(contract, "/version", 80).is_none()
        || workflow_contract_string_at(contract, "/llm_gate_instruction", 1_400).is_none()
        || workflow_contract_string_at(contract, "/llm_tool_request_instruction", 4_000).is_none()
        || contract
            .get("system_injected_chat_text_allowed")
            .and_then(Value::as_bool)
            != Some(false)
    {
        return false;
    }
    let gate_order = workflow_contract_array_strings_at(contract, "/gate_order");
    let allowed_shapes = workflow_contract_array_strings_at(contract, "/gate_shapes_allowed");
    let Some(gates) = contract.get("gates").and_then(Value::as_object) else {
        return false;
    };
    if gate_order.is_empty() || allowed_shapes.is_empty() || gates.is_empty() {
        return false;
    }
    let first_gate_id = gate_order.first().cloned().unwrap_or_default();
    let first_gate_submission_complete = !first_gate_id.is_empty()
        && !workflow_contract_array_strings_at(
            contract,
            &format!("/gates/{first_gate_id}/submission_contract/accepted_outputs"),
        )
        .is_empty()
        && !workflow_contract_array_strings_at(
            contract,
            &format!("/gates/{first_gate_id}/submission_contract/structured_token_fields"),
        )
        .is_empty()
        && contract
            .pointer(&format!(
                "/gates/{first_gate_id}/submission_contract/chat_injection_allowed"
            ))
            .and_then(Value::as_bool)
            == Some(false);
    let tool_request_fields =
        workflow_contract_array_strings_at(contract, "/tool_request_submission_contract/field_order");
    let tool_request_contract_complete = !tool_request_fields.is_empty()
        && tool_request_fields.iter().all(|field| {
            workflow_contract_string_at(
                contract,
                &format!("/tool_request_submission_contract/field_labels/{field}"),
                80,
            )
            .is_some()
        })
        && contract
            .pointer("/tool_request_submission_contract/chat_injection_allowed")
            .and_then(Value::as_bool)
            == Some(false)
        && contract
            .pointer("/tool_request_submission_contract/system_may_infer_missing_fields")
            .and_then(Value::as_bool)
            == Some(false);
    let trace_status_messages_complete = [
        "default",
        "pending_final_llm",
        "synthesized",
        "no_post_synthesis_required",
        "skipped_missing_model",
        "diagnostic_failure_pass_through",
        "synthesis_failed",
        "guard_violation_pass_through",
        "empty_llm_response",
    ]
    .iter()
    .all(|status| workflow_trace_status_message_pair_is_complete(contract, status));
    let diagnostic_markers_complete = workflow_contract_array_at(
        contract,
        "/diagnostic_markers/legacy_retry_templates",
    )
    .is_some()
        && workflow_contract_array_at(
            contract,
            "/diagnostic_markers/deferred_tool_request_phrases",
        )
        .is_some()
        && workflow_contract_array_at(
            contract,
            "/diagnostic_markers/unresolved_tool_need_phrases",
        )
        .is_some()
        && workflow_contract_array_at(
            contract,
            "/diagnostic_markers/gate_choice_prefix_leakage_phrases",
        )
        .is_some()
        && workflow_contract_array_at(
            contract,
            "/diagnostic_markers/prompt_analysis_leak_phrases",
        )
        .is_some()
        && [
            "/diagnostic_markers/unsupported_tool_claim/tool_surface_terms",
            "/diagnostic_markers/unsupported_tool_claim/execution_claim_phrases",
            "/diagnostic_markers/unsupported_tool_claim/empty_result_claim_phrases",
            "/diagnostic_markers/unsupported_tool_claim/result_context_terms",
            "/diagnostic_markers/unsupported_tool_claim/listing_claim_phrases",
            "/diagnostic_markers/unsupported_tool_claim/hypothetical_phrases",
            "/diagnostic_markers/recorded_tool_result_answer/tool_result_terms",
            "/diagnostic_markers/recorded_tool_result_answer/no_result_explanation_phrases",
        ]
        .iter()
        .all(|pointer| workflow_contract_array_at(contract, pointer).is_some());
    let declared_gates_are_valid = gate_order.iter().all(|gate_id| {
        gates
            .get(gate_id)
            .and_then(Value::as_object)
            .and_then(|gate| gate.get("input_kind").and_then(Value::as_str))
            .map(|input_kind| allowed_shapes.iter().any(|shape| shape == input_kind))
            .unwrap_or(false)
    });
    let declares_final_output = gates.values().any(|gate| {
        gate.get("final_output_contract")
            .filter(|contract| {
                contract.is_object()
                    && workflow_contract_string_at(
                        contract,
                        "/chat_requirement",
                        400,
                    )
                    .is_some()
            })
            .is_some()
    });
    let first_gate_options = workflow_gate_options_from_contract(contract, &first_gate_id);
    let final_gate_id = workflow_final_gate_id_from_contract(contract);
    let has_no_tool_option = first_gate_options.iter().any(|option| {
        option.get("has_tools").and_then(Value::as_bool) == Some(false)
            && workflow_contract_string_at(option, "/key", 120).is_some()
            && workflow_contract_string_at(option, "/label", 120).is_some()
            && workflow_contract_string_at(option, "/transition", 120)
                .map(|transition| transition == final_gate_id)
                .unwrap_or(false)
    });
    let tool_family_menu = workflow_contract_array_at(contract, "/tool_family_menu")
        .unwrap_or_default();
    let tool_menu_by_family = workflow_contract_object_at(contract, "/tool_menu_by_family")
        .unwrap_or_else(|| json!({}));
    let has_tool_option = first_gate_options.iter().any(|option| {
        let key = workflow_contract_string_at(option, "/key", 120).unwrap_or_default();
        option.get("has_tools").and_then(Value::as_bool) == Some(true)
            && !key.is_empty()
            && workflow_contract_string_at(option, "/label", 120).is_some()
            && workflow_contract_string_at(option, "/transition", 120).is_some()
            && tool_menu_by_family.get(&key).is_some()
    });
    let tool_family_menu_complete = !tool_family_menu.is_empty()
        && !tool_menu_by_family.as_object().map(|rows| rows.is_empty()).unwrap_or(true);
    let confirmation_gate_complete = gates.values().any(|gate| {
        gate.get("options")
            .and_then(Value::as_array)
            .map(|options| {
                let has_confirm = options.iter().any(|option| {
                    workflow_contract_string_at(option, "/key", 80)
                        .map(|key| key == "confirm")
                        .unwrap_or(false)
                        && workflow_contract_string_at(option, "/transition", 120)
                            .map(|transition| transition == "execute_tool")
                            .unwrap_or(false)
                });
                let has_cancel = options.iter().any(|option| {
                    workflow_contract_string_at(option, "/key", 80)
                        .map(|key| key == "cancel")
                        .unwrap_or(false)
                        && workflow_contract_string_at(option, "/terminal_state", 120)
                            .map(|state| state == "cancelled")
                            .unwrap_or(false)
                });
                has_confirm && has_cancel
            })
            .unwrap_or(false)
    });
    let loopback_declared = contract
        .get("declared_loopbacks")
        .and_then(Value::as_array)
        .map(|rows| !rows.is_empty())
        .unwrap_or(false);
    first_gate_submission_complete
        && tool_request_contract_complete
        && trace_status_messages_complete
        && diagnostic_markers_complete
        && declared_gates_are_valid
        && declares_final_output
        && has_no_tool_option
        && has_tool_option
        && tool_family_menu_complete
        && confirmation_gate_complete
        && loopback_declared
}

fn workflow_gate_options_from_contract(contract: &Value, gate_id: &str) -> Vec<Value> {
    contract
        .pointer(&format!("/gates/{gate_id}/options"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn workflow_final_gate_id_from_contract(contract: &Value) -> String {
    let gates = contract
        .get("gates")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    workflow_contract_array_strings_at(contract, "/gate_order")
        .into_iter()
        .find(|gate_id| {
            gates
                .get(gate_id)
                .and_then(|gate| gate.get("final_output_contract"))
                .filter(|value| value.is_object())
                .is_some()
        })
        .unwrap_or_default()
}

fn workflow_source_of_truth_contract_is_complete(contract: &Value) -> bool {
    workflow_contract_string_at(contract, "/interaction_source", 120).is_some()
        && workflow_contract_string_at(contract, "/rust_reader_role", 240).is_some()
        && contract
            .get("hardcoded_interaction_behavior_allowed")
            .and_then(Value::as_bool)
            == Some(false)
        && workflow_contract_array_at(contract, "/json_owns").is_some()
        && workflow_contract_array_at(contract, "/rust_owns").is_some()
        && workflow_contract_array_contains(contract, "/json_owns", "interaction_gates")
        && workflow_contract_array_contains(contract, "/json_owns", "gate_options")
        && workflow_contract_array_contains(contract, "/json_owns", "gate_transitions")
        && workflow_contract_array_contains(contract, "/json_owns", "tool_family_menus")
        && workflow_contract_array_contains(contract, "/json_owns", "tool_input_schemas")
        && workflow_contract_array_contains(contract, "/json_owns", "confirmation_states")
        && workflow_contract_array_contains(contract, "/json_owns", "loopbacks")
        && workflow_contract_array_contains(contract, "/json_owns", "final_output_contract")
        && workflow_contract_array_contains(contract, "/json_owns", "trace_status_messages")
        && workflow_contract_array_contains(contract, "/json_owns", "diagnostic_markers")
}

fn parse_workflow_definition(source_path: &str, raw_spec: &str) -> Option<WorkflowDefinition> {
    let parsed: Value = serde_json::from_str(raw_spec).ok()?;
    let name = clean_text(parsed.get("name").and_then(Value::as_str).unwrap_or(""), 80);
    if name.is_empty() {
        return None;
    }
    let workflow_type = clean_text(parsed.get("workflow_type").and_then(Value::as_str)?, 80);
    if workflow_type.is_empty() {
        return None;
    }
    let description = clean_text(
        parsed
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or(""),
        600,
    );
    let final_response_policy = clean_text(
        parsed
            .get("final_response_policy")
            .and_then(Value::as_str)?,
        120,
    );
    if final_response_policy.is_empty() {
        return None;
    }
    let gate_contract = clean_text(parsed.get("gate_contract").and_then(Value::as_str)?, 80);
    if gate_contract.is_empty() {
        return None;
    }
    let stages = parsed
        .get("stages")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| clean_text(row, 120))
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if stages.is_empty() {
        return None;
    }
    let workflow_composition_contract = parsed
        .get("workflow_composition_contract")
        .filter(|value| value.is_object())
        .cloned()
        .unwrap_or_else(|| json!({}));
    let workflow_source_of_truth_contract = parsed
        .get("workflow_source_of_truth_contract")
        .filter(|value| value.is_object())
        .cloned()
        .unwrap_or_else(|| json!({}));
    let tool_menu_interface_contract = parsed
        .get("tool_menu_interface_contract")
        .filter(|value| value.is_object())
        .cloned()?;
    if !workflow_tool_menu_contract_is_complete(&tool_menu_interface_contract) {
        return None;
    }
    let final_output_contract = tool_menu_interface_contract
        .get("gates")
        .and_then(Value::as_object)?
        .values()
        .find_map(|gate| {
            gate.get("final_output_contract")
                .filter(|value| value.is_object())
                .cloned()
        })?;
    Some(WorkflowDefinition {
        name,
        workflow_type,
        default_workflow: parsed
            .get("default")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        description,
        stages,
        final_response_policy,
        gate_contract,
        workflow_composition_contract,
        workflow_source_of_truth_contract,
        tool_menu_interface_contract,
        final_output_contract,
        source_path: source_path.to_string(),
    })
}

fn workflow_library_has_exactly_one_json_default(workflows: &[WorkflowDefinition]) -> bool {
    workflows.iter().filter(|row| row.default_workflow).count() == 1
}

fn workflow_spec_directory_candidates() -> Vec<std::path::PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(dir) = std::env::var("INFRING_WORKFLOW_DIR") {
        candidates.push(std::path::PathBuf::from(dir));
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("core/layer0/ops/src/dashboard_compat_api_parts/set_config_payload_parts/190_route_blocks/agent_scope_full_parts/workflows"));
    }
    candidates.push(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "src/dashboard_compat_api_parts/set_config_payload_parts/190_route_blocks/agent_scope_full_parts/workflows",
    ));
    candidates
}

fn workflow_spec_sources_from_disk() -> Vec<(String, String)> {
    for dir in workflow_spec_directory_candidates() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut paths = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| name.ends_with(".workflow.json"))
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        paths.sort();
        let sources = paths
            .into_iter()
            .filter_map(|path| {
                let raw = std::fs::read_to_string(&path).ok()?;
                Some((path.to_string_lossy().to_string(), raw))
            })
            .collect::<Vec<_>>();
        if !sources.is_empty() {
            return sources;
        }
    }
    Vec::new()
}

fn auxiliary_workflow_spec_directory_candidates() -> Vec<std::path::PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(dir) = std::env::var("INFRING_AUX_WORKFLOW_DIR") {
        candidates.push(std::path::PathBuf::from(dir));
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("orchestration/src/control_plane/workflows"));
    }
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    if let Some(repo_root) = manifest_dir.ancestors().nth(3) {
        candidates.push(repo_root.join("orchestration/src/control_plane/workflows"));
    }
    candidates
}

fn collect_recursive_workflow_specs(
    dir: &std::path::Path,
    out: &mut Vec<(String, String)>,
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut paths = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            collect_recursive_workflow_specs(&path, out);
            continue;
        }
        let is_workflow_json = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.ends_with(".workflow.json"))
            .unwrap_or(false);
        if !is_workflow_json {
            continue;
        }
        let Some(raw) = std::fs::read_to_string(&path).ok() else {
            continue;
        };
        out.push((path.to_string_lossy().to_string(), raw));
    }
}

fn auxiliary_workflow_spec_sources_from_disk() -> Vec<(String, String)> {
    for dir in auxiliary_workflow_spec_directory_candidates() {
        let mut sources = Vec::new();
        collect_recursive_workflow_specs(&dir, &mut sources);
        if !sources.is_empty() {
            return sources;
        }
    }
    Vec::new()
}

fn parse_auxiliary_workflow_definition(
    source_path: &str,
    raw_spec: &str,
) -> Option<AuxiliaryWorkflowDefinition> {
    let parsed: Value = serde_json::from_str(raw_spec).ok()?;
    let name = clean_text(parsed.get("name").and_then(Value::as_str).unwrap_or(""), 80);
    if name.is_empty() {
        return None;
    }
    let workflow_type = clean_text(parsed.get("workflow_type").and_then(Value::as_str)?, 80);
    if workflow_type.is_empty() {
        return None;
    }
    let description = clean_text(
        parsed
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or(""),
        600,
    );
    let workflow_composition_contract = parsed
        .get("workflow_composition_contract")
        .filter(|value| value.is_object())
        .cloned()
        .unwrap_or_else(|| json!({}));
    let workflow_source_of_truth_contract = parsed
        .get("workflow_source_of_truth_contract")
        .filter(|value| value.is_object())
        .cloned()
        .unwrap_or_else(|| json!({}));
    let final_output_contract = parsed
        .get("final_output_contract")
        .filter(|value| value.is_object())
        .cloned()
        .or_else(|| {
            parsed
                .get("tool_menu_interface_contract")
                .and_then(Value::as_object)
                .and_then(|contract| contract.get("gates"))
                .and_then(Value::as_object)
                .and_then(|gates| {
                    gates.values().find_map(|gate| {
                        gate.get("final_output_contract")
                            .filter(|value| value.is_object())
                            .cloned()
                    })
                })
        })?;
    Some(AuxiliaryWorkflowDefinition {
        name,
        workflow_type,
        description,
        workflow_composition_contract,
        workflow_source_of_truth_contract,
        final_output_contract,
        source_path: source_path.to_string(),
    })
}

fn load_workflow_library() -> Vec<WorkflowDefinition> {
    let parsed = workflow_spec_sources_from_disk()
        .into_iter()
        .filter_map(|(source_path, raw_spec)| parse_workflow_definition(&source_path, &raw_spec))
        .collect::<Vec<_>>();
    if parsed.is_empty() || !workflow_library_has_exactly_one_json_default(&parsed) {
        return Vec::new();
    }
    parsed
}

fn workflow_library_registry() -> Vec<WorkflowDefinition> {
    load_workflow_library()
}

fn auxiliary_workflow_registry() -> Vec<AuxiliaryWorkflowDefinition> {
    auxiliary_workflow_spec_sources_from_disk()
        .into_iter()
        .filter_map(|(source_path, raw_spec)| {
            parse_auxiliary_workflow_definition(&source_path, &raw_spec)
        })
        .collect::<Vec<_>>()
}

fn workflow_definition_by_name(name: &str) -> Option<WorkflowDefinition> {
    let cleaned = clean_text(name, 80);
    if cleaned.is_empty() {
        return None;
    }
    workflow_library_registry()
        .into_iter()
        .find(|row| row.name.eq_ignore_ascii_case(&cleaned))
}

fn default_workflow_definition() -> Option<WorkflowDefinition> {
    workflow_library_registry()
        .into_iter()
        .find(|row| row.default_workflow)
}

fn auxiliary_workflow_definition_by_name(name: &str) -> Option<AuxiliaryWorkflowDefinition> {
    let cleaned = clean_text(name, 80);
    if cleaned.is_empty() {
        return None;
    }
    auxiliary_workflow_registry()
        .into_iter()
        .find(|row| row.name.eq_ignore_ascii_case(&cleaned))
}

fn child_workflow_definition_json_by_name(name: &str) -> Option<Value> {
    workflow_definition_by_name(name)
        .map(|definition| workflow_definition_to_json(&definition))
        .or_else(|| {
            auxiliary_workflow_definition_by_name(name)
                .map(|definition| auxiliary_workflow_definition_to_json(&definition))
        })
}

fn workflow_child_workflow_definitions(contract: &Value) -> Value {
    let mut definitions = serde_json::Map::<String, Value>::new();
    let Some(child_calls) = contract.get("child_workflow_calls").and_then(Value::as_array) else {
        return Value::Object(definitions);
    };
    for call in child_calls {
        let child_id = clean_text(
            call.get("workflow_id").and_then(Value::as_str).unwrap_or(""),
            120,
        );
        if child_id.is_empty() || definitions.contains_key(&child_id) {
            continue;
        }
        if let Some(definition) = child_workflow_definition_json_by_name(&child_id) {
            definitions.insert(child_id, definition);
        }
    }
    Value::Object(definitions)
}

fn turn_workflow_library_catalog() -> Vec<Value> {
    workflow_library_registry()
        .into_iter()
        .map(|row| workflow_definition_to_json(&row))
        .collect::<Vec<_>>()
}

fn default_turn_workflow_name() -> String {
    default_workflow_definition()
        .map(|workflow| workflow.name)
        .unwrap_or_default()
}

fn workflow_name_hint_from_mode(workflow_mode: &str) -> String {
    let cleaned = clean_text(workflow_mode, 120);
    if cleaned.is_empty() {
        return String::new();
    }
    let lowered = cleaned.to_ascii_lowercase();
    for marker in ["workflow=", "workflow:", "workflow/"] {
        if let Some(idx) = lowered.find(marker) {
            let start = idx + marker.len();
            if start >= cleaned.len() {
                continue;
            }
            let tail = clean_text(&cleaned[start..], 80);
            if tail.is_empty() {
                continue;
            }
            let token = tail
                .split(|ch: char| ch.is_whitespace() || ch == ',' || ch == ';' || ch == '|')
                .next()
                .unwrap_or("")
                .to_string();
            if !token.is_empty() {
                return token;
            }
        }
    }
    String::new()
}

fn selected_turn_workflow(workflow_mode: &str) -> Value {
    let hint = workflow_name_hint_from_mode(workflow_mode);
    if !hint.is_empty() && workflow_definition_by_name(&hint).is_none() {
        return json!({
            "workflow_loaded": false,
            "mode": clean_text(workflow_mode, 80),
            "selection_reason": "explicit_workflow_hint_not_found",
            "requested_workflow": hint
        });
    }
    let Some(selected) = default_workflow_definition()
        .filter(|_| hint.is_empty())
        .or_else(|| workflow_definition_by_name(&hint))
    else {
        return json!({
            "workflow_loaded": false,
            "mode": clean_text(workflow_mode, 80),
            "selection_reason": "no_json_workflow_loaded"
        });
    };
    let selection_reason = if hint.is_empty() {
        "default_library_workflow".to_string()
    } else {
        "explicit_workflow_hint".to_string()
    };
    json!({
        "name": selected.name,
        "workflow_type": selected.workflow_type,
        "mode": clean_text(workflow_mode, 80),
        "selection_reason": selection_reason,
        "final_response_policy": selected.final_response_policy,
        "gate_contract": selected.gate_contract,
        "workflow_composition_contract": selected.workflow_composition_contract,
        "workflow_source_of_truth_contract": selected.workflow_source_of_truth_contract,
        "tool_menu_interface_contract": selected.tool_menu_interface_contract,
        "final_output_contract": selected.final_output_contract,
        "child_workflow_definitions": workflow_child_workflow_definitions(&selected.workflow_composition_contract),
        "source_path": selected.source_path
    })
}

#[cfg(test)]
mod workflow_reader_tests {
    use super::*;

    #[test]
    fn workflow_reader_loads_external_specs() {
        let catalog = turn_workflow_library_catalog();
        assert!(!catalog.is_empty());
        assert!(catalog.iter().any(|row| {
            row.get("name")
                .and_then(Value::as_str)
                .map(|name| name == "complex_prompt_chain_v1")
                .unwrap_or(false)
        }));
    }

    #[test]
    fn workflow_reader_enforces_single_default() {
        let registry = workflow_library_registry();
        let defaults = registry.iter().filter(|row| row.default_workflow).count();
        assert_eq!(defaults, 1);
        assert!(workflow_library_has_exactly_one_json_default(&registry));
    }

    #[test]
    fn workflow_reader_sources_current_workflows_from_json_specs() {
        let catalog = turn_workflow_library_catalog();
        assert!(!catalog.is_empty());
        for row in catalog {
            let source = row
                .get("source_path")
                .and_then(Value::as_str)
                .unwrap_or("");
            assert!(
                source.ends_with(".workflow.json") && !source.starts_with("builtin:"),
                "workflow not sourced from disk JSON spec: {source}"
            );
        }
    }

    #[test]
    fn workflow_reader_projects_final_output_contract_from_json_spec() {
        let selected = selected_turn_workflow("workflow=research_synthesize_verify_v1");
        assert_eq!(
            selected
                .pointer("/final_output_contract/visible_chat_source")
                .and_then(Value::as_str),
            Some("llm_final_answer_only")
        );
        let chat_excludes = selected
            .pointer("/final_output_contract/chat_excludes")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(
            chat_excludes
                .iter()
                .any(|value| value.as_str() == Some("agent_internal_notes")),
            "{}",
            selected
        );
        assert!(
            chat_excludes
                .iter()
                .any(|value| value.as_str() == Some("prompt_analysis")),
            "{}",
            selected
        );
        let chat_requirement = selected
            .pointer("/final_output_contract/chat_requirement")
            .and_then(Value::as_str)
            .expect("chat requirement");
        let lowered_chat_requirement = chat_requirement.to_ascii_lowercase();
        for needle in [
            "match the semantic shape of the request",
            "for lookup or current-state research",
            "for comparisons",
            "for rankings, selections, or tool-choice questions",
            "for low_signal evidence",
            "concrete answer units",
            "do not treat source titles",
            "do not make the whole answer a request for the user to narrow",
            "do not end with a follow-up question",
            "there is no required output format",
        ] {
            assert!(lowered_chat_requirement.contains(needle), "{chat_requirement}");
        }
        assert!(
            lowered_chat_requirement.contains("no recorded tool outcomes")
                && lowered_chat_requirement.contains("never claim no tool result is available")
                && !chat_requirement.contains("MUST begin with the exact sentence"),
            "{chat_requirement}"
        );
        assert!(
            lowered_chat_requirement.contains("do not substitute system instructions"),
            "{chat_requirement}"
        );
        assert!(
            lowered_chat_requirement
                .contains("never use training, prior, existing, or general knowledge"),
            "{chat_requirement}"
        );
        assert_eq!(
            selected
                .pointer("/tool_menu_interface_contract/final_synthesis_attempt_limit")
                .and_then(Value::as_u64),
            Some(3)
        );
        assert_eq!(
            selected
                .pointer("/tool_menu_interface_contract/final_synthesis_retry_contract/authority")
                .and_then(Value::as_str),
            Some("workflow_cd_declares_attempt_budget_runtime_executes")
        );
        assert_eq!(
            selected
                .pointer("/final_output_contract/final_response_verifier_contract/authority")
                .and_then(Value::as_str),
            Some("workflow_cd_declares_semantic_checks_runtime_may_retry_or_fail_closed")
        );
        let verifier_checks = selected
            .pointer("/final_output_contract/final_response_verifier_contract/checks")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for check in [
            "answers_the_user_goal",
            "uses_recorded_evidence_when_tool_results_exist",
            "does_not_claim_no_evidence_when_recorded_evidence_exists",
            "does_not_force_or_depend_on_a_specific_output_format",
        ] {
            assert!(
                verifier_checks
                    .iter()
                    .any(|value| value.as_str() == Some(check)),
                "{selected}"
            );
        }
        assert_eq!(
            selected
                .pointer(
                    "/tool_menu_interface_contract/latent_candidate_recovery_contract/promotion_scope"
                )
                .and_then(Value::as_str),
            Some("single_valid_workflow_only_candidate_after_private_gate_failure_or_terminal_invariant_recovery")
        );
        assert_eq!(
            selected
                .pointer(
                    "/tool_menu_interface_contract/terminal_invariant_contract/valid_latent_candidate_without_tool_attempt_policy"
                )
                .and_then(Value::as_str),
            Some("promote_single_required_candidate_or_structured_failure_before_final_answer")
        );
        assert_eq!(
            selected
                .pointer(
                    "/tool_menu_interface_contract/terminal_invariant_contract/required_latent_candidate_flag"
                )
                .and_then(Value::as_str),
            Some("requires_tool_attempt_before_final_answer")
        );
    }

    #[test]
    fn workflow_reader_projects_trace_status_messages_from_json_spec() {
        let selected = selected_turn_workflow("");
        assert_eq!(
            selected
                .pointer("/tool_menu_interface_contract/trace_status_messages/synthesized/ui")
                .and_then(Value::as_str),
            Some("Workflow complete; final answer came from the LLM final gate.")
        );
        for status in [
            "default",
            "pending_final_llm",
            "synthesized",
            "no_post_synthesis_required",
            "skipped_missing_model",
            "diagnostic_failure_pass_through",
            "synthesis_failed",
            "guard_violation_pass_through",
            "empty_llm_response",
        ] {
            assert!(
                selected
                    .pointer(&format!(
                        "/tool_menu_interface_contract/trace_status_messages/{status}/ui"
                    ))
                    .and_then(Value::as_str)
                    .map(|row| !row.trim().is_empty())
                    .unwrap_or(false),
                "missing ui trace status for {status}"
            );
            assert!(
                selected
                    .pointer(&format!(
                        "/tool_menu_interface_contract/trace_status_messages/{status}/agent_process"
                    ))
                    .and_then(Value::as_str)
                    .map(|row| !row.trim().is_empty())
                    .unwrap_or(false),
                "missing agent_process trace status for {status}"
            );
        }
        assert_eq!(
            selected
                .pointer("/workflow_source_of_truth_contract/json_owns")
                .and_then(Value::as_array)
                .map(|rows| rows.iter().any(|row| row.as_str() == Some("trace_status_messages"))),
            Some(true)
        );
    }

    #[test]
    fn workflow_reader_default_composite_delegates_web_research_to_child_cd() {
        let selected = selected_turn_workflow("");
        assert_eq!(
            selected
                .pointer("/workflow_composition_contract/cd_kind")
                .and_then(Value::as_str),
            Some("composite")
        );
        assert_eq!(
            selected
                .pointer(
                    "/workflow_composition_contract/child_workflow_calls/0/workflow_id"
                )
                .and_then(Value::as_str),
            Some("research_synthesize_verify_v1")
        );
        assert_eq!(
            selected
                .pointer(
                    "/workflow_composition_contract/child_workflow_calls/0/tool_family_key"
                )
                .and_then(Value::as_str),
            Some("web_research")
        );
        assert_eq!(
            selected
                .pointer(
                    "/tool_menu_interface_contract/delegated_capability_contract_refs/web_research/workflow_id"
                )
                .and_then(Value::as_str),
            Some("research_synthesize_verify_v1")
        );
        let projected = workflow_tool_menu_contract_for_family("web_research");
        assert!(projected
            .pointer("/retrieval_recovery_contract/version")
            .and_then(Value::as_str)
            .is_some());
        assert!(projected
            .pointer("/tool_menu_by_family/web_research")
            .and_then(Value::as_array)
            .map(|tools| tools.iter().any(|tool| {
                tool.get("key").and_then(Value::as_str) == Some("batch_query")
            }))
            .unwrap_or(false));
    }

    #[test]
    fn workflow_reader_research_cd_does_not_expose_thin_web_search() {
        let selected = selected_turn_workflow("workflow=research_synthesize_verify_v1");
        let has_web_search = selected
            .pointer("/tool_menu_interface_contract/tool_menu_by_family/web_research")
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter().any(|row| {
                    row.get("key").and_then(Value::as_str) == Some("web_search")
                })
            })
            .unwrap_or(false);

        assert!(
            !has_web_search,
            "research CD should use evidence-packet batch_query or URL-specific web_fetch"
        );
    }

    #[test]
    fn auxiliary_workflow_registry_includes_research_synthesis_child() {
        let child = auxiliary_workflow_definition_by_name("research_answer_synthesis");
        assert!(
            child.is_some(),
            "expected research child workflow in auxiliary registry; directories={:?}",
            auxiliary_workflow_spec_directory_candidates()
        );
    }

    #[test]
    fn auxiliary_workflow_parser_accepts_research_synthesis_file() {
        let (source_path, raw_spec) = auxiliary_workflow_spec_sources_from_disk()
            .into_iter()
            .find(|(source_path, _)| source_path.ends_with("research_answer_synthesis.workflow.json"))
            .expect("research synthesis workflow spec on disk");
        let parsed = parse_auxiliary_workflow_definition(&source_path, &raw_spec);
        assert!(
            parsed.is_some(),
            "failed to parse auxiliary research child workflow from {source_path}"
        );
    }

    #[test]
    fn workflow_reader_loads_auxiliary_research_child_workflow_contracts() {
        let selected = selected_turn_workflow("workflow=research_synthesize_verify_v1");
        assert_eq!(
            selected
                .pointer("/child_workflow_definitions/research_answer_synthesis/name")
                .and_then(Value::as_str),
            Some("research_answer_synthesis")
        );
        assert_eq!(
            selected
                .pointer(
                    "/child_workflow_definitions/research_answer_synthesis/final_output_contract/schema_version"
                )
                .and_then(Value::as_str),
            Some("research_answer_synthesis_final_output_contract_v1")
        );
        assert_eq!(
            selected
                .pointer(
                    "/child_workflow_definitions/research_answer_verification/final_output_contract/schema_version"
                )
                .and_then(Value::as_str),
            Some("research_answer_verification_final_output_contract_v1")
        );
        assert_eq!(
            selected
                .pointer(
                    "/child_workflow_definitions/research_evidence_acquisition/final_output_contract/schema_version"
                )
                .and_then(Value::as_str),
            Some("research_evidence_acquisition_final_output_contract_v1")
        );
    }

    #[test]
    fn workflow_reader_web_research_menu_exposes_batch_query_pack_contract() {
        let selected = selected_turn_workflow("workflow=research_synthesize_verify_v1");
        let batch_query = selected
            .pointer("/tool_menu_interface_contract/tool_menu_by_family/web_research")
            .and_then(Value::as_array)
            .and_then(|rows| {
                rows.iter().find(|row| {
                    row.get("key").and_then(Value::as_str) == Some("batch_query")
                })
            })
            .expect("batch_query tool");

        assert_eq!(
            batch_query
                .pointer("/request_format/source")
                .and_then(Value::as_str),
            Some("web")
        );
        assert_eq!(
            batch_query
                .pointer("/request_format/query")
                .and_then(Value::as_str),
            Some("<overall research question>")
        );
        assert!(
            batch_query
                .pointer("/request_format/queries")
                .and_then(Value::as_array)
                .map(|rows| rows.len() >= 2)
                .unwrap_or(false),
            "{batch_query}"
        );
        assert!(
            batch_query
                .pointer("/request_example/queries")
                .and_then(Value::as_array)
                .map(|rows| rows.len() >= 4)
                .unwrap_or(false),
            "{batch_query}"
        );
        assert!(
            batch_query
                .pointer("/request_format/keywords")
                .and_then(Value::as_array)
                .map(|rows| !rows.is_empty())
                .unwrap_or(false),
            "{batch_query}"
        );
        assert!(
            batch_query
                .pointer("/request_format/required_coverage/entities")
                .and_then(Value::as_array)
                .map(|rows| !rows.is_empty())
                .unwrap_or(false),
            "{batch_query}"
        );
        assert_eq!(
            batch_query
                .pointer("/recovery_default_for_raw_message")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            batch_query
                .pointer("/raw_message_fallback_contract/bind_user_message_to")
                .and_then(Value::as_str),
            Some("query")
        );
    }

    #[test]
    fn workflow_reader_web_research_menu_preserves_tool_cd_refs() {
        let selected = selected_turn_workflow("workflow=research_synthesize_verify_v1");
        let web_tools = selected
            .pointer("/tool_menu_interface_contract/tool_menu_by_family/web_research")
            .and_then(Value::as_array)
            .expect("web research tools");

        for tool_id in ["batch_query", "web_fetch"] {
            let tool = web_tools
                .iter()
                .find(|row| {
                    row.get("key").and_then(Value::as_str) == Some(tool_id)
                })
                .unwrap_or_else(|| panic!("missing {tool_id} tool"));
            assert_eq!(tool.get("tool_id").and_then(Value::as_str), Some(tool_id));
            let expected_cd_ref =
                format!("core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json#{tool_id}");
            assert_eq!(
                tool.get("tool_cd_ref").and_then(Value::as_str),
                Some(expected_cd_ref.as_str())
            );
        }
    }

    #[test]
    fn workflow_reader_gate_instruction_keeps_research_routing_general() {
        let selected = selected_turn_workflow("workflow=research_synthesize_verify_v1");
        let gate_instruction = selected
            .pointer("/tool_menu_interface_contract/llm_gate_instruction")
            .and_then(Value::as_str)
            .expect("gate instruction");

        assert!(
            gate_instruction.contains("Output ONLY JSON"),
            "{gate_instruction}"
        );
        assert!(
            gate_instruction.contains("research_synthesis_from_recorded_state"),
            "{gate_instruction}"
        );
        assert!(
            gate_instruction.contains("web_research"),
            "{gate_instruction}"
        );
        assert!(
            gate_instruction.contains("recorded tool outcomes, evidence refs, snippets"),
            "{gate_instruction}"
        );
        assert!(
            gate_instruction.contains("public evidence, current information, source-backed comparison"),
            "{gate_instruction}"
        );
        assert!(
            gate_instruction.contains("Do not answer the user directly at this gate"),
            "{gate_instruction}"
        );
        assert!(!gate_instruction.contains("Infring"), "{gate_instruction}");
        assert!(!gate_instruction.contains("Semantic Kernel"), "{gate_instruction}");
    }

    #[test]
    fn workflow_reader_declares_private_gate_empty_retry_contract() {
        let selected = selected_turn_workflow("");
        let retry_instruction = selected
            .pointer("/tool_menu_interface_contract/private_gate_retry_instruction")
            .and_then(Value::as_str)
            .expect("private gate retry instruction");

        assert!(
            retry_instruction.contains("If the excerpt is empty, treat it as an empty response."),
            "{retry_instruction}"
        );
        assert!(
            retry_instruction.contains("output only the exact JSON artifact required by that gate"),
            "{retry_instruction}"
        );
        assert!(
            retry_instruction.contains("{current_gate_id}")
                && retry_instruction.contains("{last_reject_reason}")
                && retry_instruction.contains("{last_invalid_excerpt}"),
            "{retry_instruction}"
        );
        assert!(!retry_instruction.contains("Infring"), "{retry_instruction}");
    }

    #[test]
    fn workflow_reader_final_answer_contract_uses_general_research_shapes() {
        let selected = selected_turn_workflow("workflow=research_synthesize_verify_v1");
        let chat_requirement = selected
            .pointer("/tool_menu_interface_contract/gates/gate_6_llm_final_output/final_output_contract/chat_requirement")
            .and_then(Value::as_str)
            .expect("chat requirement");

        assert!(
            chat_requirement.contains("Match the semantic shape of the request rather than forcing a canned format."),
            "{chat_requirement}"
        );
        assert!(
            chat_requirement.contains("For lookup or current-state research"),
            "{chat_requirement}"
        );
        assert!(
            chat_requirement.contains("For comparisons"),
            "{chat_requirement}"
        );
        assert!(
            chat_requirement.contains("For rankings, selections, or tool-choice questions"),
            "{chat_requirement}"
        );
        assert!(
            chat_requirement.contains("For low_signal evidence"),
            "{chat_requirement}"
        );
        assert!(
            chat_requirement.contains("do not fill them in from general knowledge"),
            "{chat_requirement}"
        );
        assert!(
            chat_requirement.contains("concrete answer units before any limitations")
                && chat_requirement.contains("Do not treat source titles, snippets, retrieval status, provider status, or coverage gaps as the answer"),
            "{chat_requirement}"
        );
        assert!(
            chat_requirement.contains("There is no required output format."),
            "{chat_requirement}"
        );
        assert!(
            chat_requirement.contains("Example formats include a short paragraph, brief bullets, a compact comparison table, or a mixed structure"),
            "{chat_requirement}"
        );
        assert!(!chat_requirement.contains("agentic framework"), "{chat_requirement}");
    }

    #[test]
    fn workflow_reader_projects_retrieval_recovery_contract_from_json_spec() {
        let selected = selected_turn_workflow("workflow=research_synthesize_verify_v1");
        let recovery = selected
            .pointer("/tool_menu_interface_contract/retrieval_recovery_contract")
            .expect("retrieval recovery contract");
        assert_eq!(
            recovery.get("authority").and_then(Value::as_str),
            Some("agent_submitted_tool_inputs")
        );
        assert_eq!(
            recovery
                .get("default_recovery_budget")
                .and_then(Value::as_u64),
            Some(2)
        );
        let behavior = recovery
            .get("recovery_behavior")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(
            behavior.contains("submit a more specific query or query pack before final synthesis"),
            "{behavior}"
        );
        assert!(
            behavior.contains("Do not ask the user to narrow"),
            "{behavior}"
        );
        let coverage_behavior = recovery
            .get("coverage_behavior")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(
            coverage_behavior.contains("retrieval coverage lane"),
            "{coverage_behavior}"
        );
        let two_phase_behavior = recovery
            .get("two_phase_retrieval_behavior")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(
            two_phase_behavior.contains("initial discovery pass"),
            "{two_phase_behavior}"
        );
        assert!(
            two_phase_behavior.contains("targeted follow-up queries"),
            "{two_phase_behavior}"
        );
        assert!(
            recovery
                .get("query_refinement_axes")
                .and_then(Value::as_array)
                .map(|rows| rows.iter().any(|row| {
                    row.as_str()
                        .map(|value| value.contains("avoid hidden query expansion"))
                        .unwrap_or(false)
                }))
                .unwrap_or(false),
            "{recovery}"
        );
    }

    #[test]
    fn workflow_reader_payload_instruction_requires_batch_query_top_level_query() {
        let selected = selected_turn_workflow("workflow=research_synthesize_verify_v1");
        let payload_instruction = selected
            .pointer("/tool_menu_interface_contract/llm_tool_payload_instruction")
            .and_then(Value::as_str)
            .expect("payload instruction");

        assert!(
            payload_instruction.contains("top-level `query` field is mandatory"),
            "{payload_instruction}"
        );
        assert!(
            payload_instruction.contains("A single broad query may be used as an initial discovery pass"),
            "{payload_instruction}"
        );
        assert!(
            payload_instruction.contains("must not finish from that pass unless the returned evidence already covers the request"),
            "{payload_instruction}"
        );
        assert!(
            payload_instruction.contains("4-8 concrete follow-up searches"),
            "{payload_instruction}"
        );
        assert!(
            payload_instruction.contains("omitting `queries` is also invalid unless this is explicitly a narrow lookup or an initial discovery pass"),
            "{payload_instruction}"
        );
        assert!(
            payload_instruction.contains("submit targeted follow-up queries rather than finalizing"),
            "{payload_instruction}"
        );
        assert!(
            payload_instruction.contains("Keep exact entity names in the query pack"),
            "{payload_instruction}"
        );
        assert!(
            payload_instruction.contains("allocate query coverage fairly"),
            "{payload_instruction}"
        );
        assert!(
            payload_instruction.contains("If a prior tool result quality diagnostic is available and recommends retry"),
            "{payload_instruction}"
        );
        assert!(
            payload_instruction.contains("without asking the user to narrow while internal recovery budget remains"),
            "{payload_instruction}"
        );
        assert!(
            payload_instruction.contains("payload that contains only `queries` and `aperture` is invalid"),
            "{payload_instruction}"
        );
    }

    #[test]
    fn workflow_reader_tool_selection_uses_batch_query_for_evidence_backed_research() {
        let selected = selected_turn_workflow("workflow=research_synthesize_verify_v1");
        let selection_instruction = selected
            .pointer("/tool_menu_interface_contract/llm_tool_selection_instruction")
            .and_then(Value::as_str)
            .expect("tool selection instruction");

        assert!(
            selection_instruction.contains("Choose `batch_query` for source-backed web research"),
            "{selection_instruction}"
        );
        assert!(
            selection_instruction.contains("any request whose final answer needs cited evidence or an evidence packet"),
            "{selection_instruction}"
        );
        assert!(
            selection_instruction.contains("{\"tool\":\"batch_query\"}"),
            "{selection_instruction}"
        );
        assert!(
            !selection_instruction.contains("Between `web_search` and `batch_query`"),
            "{selection_instruction}"
        );
        assert!(!selection_instruction.contains("Mastra"), "{selection_instruction}");
    }

    #[test]
    fn workflow_reader_rejects_specs_missing_json_authority_contract() {
        let raw_spec = json!({
            "name": "missing_authority_contract_v1",
            "workflow_type": "hard_agent_workflow",
            "default": false,
            "description": "Invalid spec: Rust must not invent the workflow contract.",
            "stages": ["gate_1_work_category_menu"],
            "final_response_policy": "llm_authored_only_no_system_injection",
            "gate_contract": "tool_menu_interface_v1",
            "tool_menu_interface_contract": {
                "version": "tool_menu_interface_v1",
                "gates": {
                    "gate_6_llm_final_output": {
                        "final_output_contract": {
                            "visible_chat_source": "llm_final_answer_only"
                        }
                    }
                }
            }
        })
        .to_string();

        assert!(parse_workflow_definition("test.workflow.json", &raw_spec).is_none());
    }

    #[test]
    fn workflow_reader_rejects_specs_when_json_permits_hardcoded_interaction_behavior() {
        let raw_spec = json!({
            "name": "bad_authority_escape_hatch_v1",
            "workflow_type": "hard_agent_workflow",
            "default": false,
            "description": "Invalid spec: JSON may not authorize Rust-authored interaction gates.",
            "stages": ["gate_1_work_category_menu"],
            "final_response_policy": "llm_authored_only_no_system_injection",
            "gate_contract": "tool_menu_interface_v1",
            "workflow_source_of_truth_contract": {
                "interaction_source": "json_workflow_spec",
                "rust_reader_role": "validate_execute_trace_only",
                "hardcoded_interaction_behavior_allowed": true,
                "json_owns": ["interaction_gates"],
                "rust_owns": ["json_loading"]
            },
            "tool_menu_interface_contract": {
                "version": "tool_menu_interface_v1",
                "system_injected_chat_text_allowed": false,
                "llm_gate_instruction": "Template.",
                "gate_order": ["gate_1_work_category_menu"],
                "gate_shapes_allowed": ["multiple_choice"],
                "terminal_states": ["completed"],
                "declared_loopbacks": [{"from": "gate_5_post_tool_menu", "on": "another_tool", "to": "gate_2_tool_family_menu"}],
                "tool_family_menu": [{"key": "respond_directly"}],
                "tool_menu_by_family": {"none": []},
                "gates": {
                    "gate_1_work_category_menu": {
                        "input_kind": "multiple_choice",
                        "question": "Question",
                        "submission_contract": {"accepted_outputs": ["Respond directly"]},
                        "options": [{"key": "respond_directly", "label": "Respond directly"}]
                    },
                    "gate_2_tool_family_menu": {"input_kind": "multiple_choice"},
                    "gate_3_tool_menu": {"input_kind": "multiple_choice"},
                    "gate_4_request_payload_input": {"input_kind": "text_input"},
                    "gate_4b_tool_confirmation_menu": {"input_kind": "multiple_choice"},
                    "gate_5_post_tool_menu": {"input_kind": "multiple_choice"},
                    "gate_6_llm_final_output": {
                        "input_kind": "text_input",
                        "final_output_contract": {"visible_chat_source": "llm_final_answer_only"}
                    }
                }
            }
        })
        .to_string();

        assert!(parse_workflow_definition("test.workflow.json", &raw_spec).is_none());
    }

    #[test]
    fn workflow_reader_rejects_specs_missing_final_output_contract() {
        let raw_spec = json!({
            "name": "missing_final_contract_v1",
            "workflow_type": "hard_agent_workflow",
            "default": false,
            "description": "Invalid spec: final-output separation must come from JSON.",
            "stages": ["gate_1_work_category_menu"],
            "final_response_policy": "llm_authored_only_no_system_injection",
            "gate_contract": "tool_menu_interface_v1",
            "workflow_source_of_truth_contract": {
                "interaction_source": "json_workflow_spec",
                "rust_reader_role": "validate_execute_trace_only",
                "hardcoded_interaction_behavior_allowed": false
            },
            "tool_menu_interface_contract": {
                "version": "tool_menu_interface_v1",
                "gates": {}
            }
        })
        .to_string();

        assert!(parse_workflow_definition("test.workflow.json", &raw_spec).is_none());
    }

    #[test]
    fn workflow_reader_rejects_specs_missing_tool_menu_contract_details() {
        let raw_spec = json!({
            "name": "missing_tool_menu_details_v1",
            "workflow_type": "hard_agent_workflow",
            "default": false,
            "description": "Invalid spec: Rust must not synthesize missing menus.",
            "stages": ["gate_1_work_category_menu"],
            "final_response_policy": "llm_authored_only_no_system_injection",
            "gate_contract": "tool_menu_interface_v1",
            "workflow_source_of_truth_contract": {
                "interaction_source": "json_workflow_spec",
                "rust_reader_role": "validate_execute_trace_only",
                "hardcoded_interaction_behavior_allowed": false,
                "json_owns": ["interaction_gates"],
                "rust_owns": ["json_loading"]
            },
            "tool_menu_interface_contract": {
                "version": "tool_menu_interface_v1",
                "system_injected_chat_text_allowed": false,
                "llm_gate_instruction": "Template.",
                "gates": {
                    "gate_6_llm_final_output": {
                        "input_kind": "text_input",
                        "final_output_contract": {"visible_chat_source": "llm_final_answer_only"}
                    }
                }
            }
        })
        .to_string();

        assert!(parse_workflow_definition("test.workflow.json", &raw_spec).is_none());
    }

    #[test]
    fn workflow_reader_does_not_invent_default_when_json_omits_it() {
        let mut workflows = workflow_library_registry();
        for workflow in workflows.iter_mut() {
            workflow.default_workflow = false;
        }

        assert!(!workflow_library_has_exactly_one_json_default(&workflows));
    }

    #[test]
    fn workflow_reader_does_not_map_runtime_modes_to_hidden_workflows() {
        assert_eq!(workflow_name_hint_from_mode("model_direct_answer"), "");
        assert_eq!(workflow_name_hint_from_mode("model_inline_tool_execution"), "");
        assert_eq!(
            workflow_name_hint_from_mode("workflow=complex_prompt_chain_v1"),
            "complex_prompt_chain_v1"
        );
    }

    #[test]
    fn workflow_reader_does_not_replace_unknown_explicit_workflow_with_default() {
        let selected = selected_turn_workflow("workflow=missing_cd_v1");
        assert_eq!(
            selected.get("workflow_loaded").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            selected.get("selection_reason").and_then(Value::as_str),
            Some("explicit_workflow_hint_not_found")
        );
        assert_eq!(
            selected.get("requested_workflow").and_then(Value::as_str),
            Some("missing_cd_v1")
        );
    }
}
