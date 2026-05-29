fn default_workflow_tool_menu_contract() -> Value {
    default_workflow_definition()
        .map(|workflow| workflow.tool_menu_interface_contract)
        .unwrap_or_else(|| json!({}))
}

fn workflow_child_id_for_tool_family(workflow: &WorkflowDefinition, family_key: &str) -> String {
    let family_key = clean_text(family_key, 120);
    workflow
        .workflow_composition_contract
        .get("child_workflow_calls")
        .and_then(Value::as_array)
        .and_then(|calls| {
            calls.iter().find_map(|call| {
                let call_family = call
                    .get("tool_family_key")
                    .and_then(Value::as_str)
                    .map(|row| clean_text(row, 120))
                    .unwrap_or_default();
                if call_family == family_key {
                    call.get("workflow_id")
                        .and_then(Value::as_str)
                        .map(|row| clean_text(row, 120))
                        .filter(|row| !row.is_empty())
                } else {
                    None
                }
            })
        })
        .unwrap_or_default()
}

fn workflow_tool_menu_contract_for_family(family_key: &str) -> Value {
    let family_key = clean_text(family_key, 120);
    let Some(default_workflow) = default_workflow_definition() else {
        return json!({});
    };
    let child_id = workflow_child_id_for_tool_family(&default_workflow, &family_key);
    if !child_id.is_empty() {
        if let Some(child) = workflow_definition_by_name(&child_id) {
            let child_has_family = child
                .tool_menu_interface_contract
                .get("tool_menu_by_family")
                .and_then(Value::as_object)
                .and_then(|families| families.get(&family_key))
                .is_some();
            if child_has_family {
                return child.tool_menu_interface_contract;
            }
        }
    }
    default_workflow.tool_menu_interface_contract
}

fn workflow_tool_menu_contract_for_tool_key(tool_key: &str) -> Value {
    let tool_key = clean_text(tool_key, 120);
    if tool_key.is_empty() {
        return default_workflow_tool_menu_contract();
    }
    let mut contracts = default_workflow_definition()
        .map(|default_workflow| {
            let mut contracts = vec![default_workflow.tool_menu_interface_contract.clone()];
            if let Some(child_calls) = default_workflow
                .workflow_composition_contract
                .get("child_workflow_calls")
                .and_then(Value::as_array)
            {
                for call in child_calls {
                    if let Some(child_id) = call.get("workflow_id").and_then(Value::as_str) {
                        if let Some(child) = workflow_definition_by_name(child_id) {
                            contracts.push(child.tool_menu_interface_contract);
                        }
                    }
                }
            }
            contracts
        })
        .unwrap_or_default();
    contracts.reverse();
    contracts
        .into_iter()
        .find(|contract| {
            contract
                .get("tool_menu_by_family")
                .and_then(Value::as_object)
                .map(|families| {
                    families.values().any(|tools| {
                        tools.as_array().map(|tools| {
                            tools.iter().any(|tool| workflow_option_key(tool) == tool_key)
                        }) == Some(true)
                    })
                })
                .unwrap_or(false)
        })
        .unwrap_or_else(default_workflow_tool_menu_contract)
}

fn workflow_contract_gate(contract: &Value, gate_id: &str) -> Value {
    contract
        .pointer(&format!("/gates/{gate_id}"))
        .cloned()
        .unwrap_or_else(|| json!({}))
}

fn workflow_gate_options(contract: &Value, gate_id: &str) -> Vec<Value> {
    workflow_contract_gate(contract, gate_id)
        .get("options")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn workflow_contract_gate_order(contract: &Value) -> Vec<String> {
    contract
        .get("gate_order")
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

fn workflow_first_gate_id(contract: &Value) -> String {
    workflow_contract_gate_order(contract)
        .into_iter()
        .next()
        .unwrap_or_default()
}

fn workflow_final_gate_id(contract: &Value) -> String {
    let gates = contract
        .get("gates")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    workflow_contract_gate_order(contract)
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

fn workflow_post_tool_gate_id(contract: &Value) -> String {
    contract
        .get("declared_loopbacks")
        .and_then(Value::as_array)
        .and_then(|rows| {
            rows.iter()
                .find_map(|row| row.get("from").and_then(Value::as_str))
        })
        .map(|row| clean_text(row, 120))
        .filter(|row| !row.is_empty())
        .or_else(|| {
            let final_gate_id = workflow_final_gate_id(contract);
            workflow_contract_gate_order(contract)
                .into_iter()
                .rev()
                .find(|gate_id| gate_id != &final_gate_id)
        })
        .unwrap_or_default()
}

fn workflow_gate_resume_token(gate_id: &str, status: &str) -> String {
    let gate_id = clean_text(gate_id, 120);
    let status = clean_text(status, 80);
    if gate_id.is_empty() || status.is_empty() {
        String::new()
    } else {
        format!("{gate_id}.{status}")
    }
}

fn workflow_option_label(option: &Value) -> String {
    clean_text(
        option.get("label").and_then(Value::as_str).unwrap_or(""),
        120,
    )
}

fn workflow_option_key(option: &Value) -> String {
    clean_text(option.get("key").and_then(Value::as_str).unwrap_or(""), 120)
}

fn normalized_workflow_token(value: &str) -> String {
    clean_text(value, 240)
        .to_ascii_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn workflow_structured_gate_submission(response: &str) -> Option<Value> {
    let mut raw = clean_text(response, 8_000);
    let trimmed = raw.trim();
    if trimmed.starts_with("```") {
        raw = trimmed
            .trim_matches('`')
            .trim_start_matches("json")
            .trim()
            .to_string();
    }
    if !raw.trim_start().starts_with('{') && raw.trim_start().starts_with('"') {
        raw = format!("{{{}", raw.trim());
    }
    serde_json::from_str::<Value>(raw.trim())
        .ok()
        .filter(Value::is_object)
}

fn workflow_structured_gate_token_fields(contract: &Value) -> Vec<String> {
    let first_gate_id = workflow_first_gate_id(contract);
    workflow_contract_gate(contract, &first_gate_id)
        .pointer("/submission_contract/structured_token_fields")
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

fn workflow_structured_gate_token_value(token: &Value) -> Option<String> {
    token
        .as_str()
        .map(|row| clean_text(row, 120))
        .or_else(|| token.as_i64().map(|row| row.to_string()))
        .filter(|row| !row.is_empty())
}

fn workflow_structured_gate_token(contract: &Value, response: &str) -> Option<String> {
    let value = workflow_structured_gate_submission(response)?;
    let object = value.as_object()?;
    workflow_structured_gate_token_fields(contract)
        .into_iter()
        .map(|field| normalized_workflow_token(&field))
        .find_map(|field| {
            object.iter().find_map(|(key, token)| {
                (normalized_workflow_token(key) == field)
                    .then(|| workflow_structured_gate_token_value(token))
                    .flatten()
            })
        })
}

fn workflow_structured_gate_final_answer(response: &str) -> Option<String> {
    if let Some(value) = workflow_structured_gate_submission(response) {
        if let Some(answer) = value.as_object().and_then(|object| {
            object.iter().find_map(|(key, value)| {
                let normalized_key = normalized_workflow_token(key);
                (matches!(
                    normalized_key.as_str(),
                    "final answer" | "final response" | "visible response" | "message" | "answer"
                ) || normalized_key.ends_with(" final answer")
                    || normalized_key.ends_with(" final response"))
                .then(|| value.as_str())
                .flatten()
            })
        }) {
            let answer = sanitize_workflow_final_response_candidate(answer);
            if !answer.is_empty() {
                return Some(answer);
            }
        }
    }
    let raw = clean_text(response, 8_000);
    let Some((key_part, value_part)) = raw.split_once(':') else {
        return None;
    };
    let normalized_key = normalized_workflow_token(key_part);
    if !(normalized_key.ends_with("final answer")
        || normalized_key.ends_with("final response")
        || normalized_key.ends_with("visible response"))
    {
        return None;
    }
    let answer = sanitize_workflow_final_response_candidate(
        value_part
            .trim()
            .trim_matches('{')
            .trim_matches('}')
            .trim_matches('"'),
    );
    (!answer.is_empty()).then_some(answer)
}

fn normalized_workflow_option_tokens(option: &Value) -> Vec<String> {
    let mut tokens = [workflow_option_key(option), workflow_option_label(option)]
        .into_iter()
        .collect::<Vec<_>>();
    if let Some(aliases) = option.get("aliases").and_then(Value::as_array) {
        tokens.extend(
            aliases
                .iter()
                .filter_map(Value::as_str)
                .map(|alias| clean_text(alias, 120)),
        );
    }
    tokens
        .into_iter()
        .map(|row| normalized_workflow_token(&row))
        .filter(|row| !row.is_empty())
        .collect()
}

fn workflow_selection_token_variants(token: &str) -> Vec<String> {
    let normalized = normalized_workflow_token(token);
    if normalized.is_empty() {
        return Vec::new();
    }
    let mut variants = vec![normalized.clone()];
    if let Some(first) = normalized.split_whitespace().next() {
        if first.chars().all(|ch| ch.is_ascii_digit()) {
            let first = first.to_string();
            if !variants.iter().any(|candidate| candidate == &first) {
                variants.push(first);
            }
        }
    }
    for separator in [" = ", " : "] {
        if let Some((left, right)) = normalized.split_once(separator) {
            for part in [left, right] {
                let candidate = normalized_workflow_token(part);
                if !candidate.is_empty() && !variants.iter().any(|existing| existing == &candidate)
                {
                    variants.push(candidate);
                }
            }
        }
    }
    if let Some(stripped) = normalized
        .strip_prefix(|ch: char| ch.is_ascii_digit() || ch == '=' || ch == ':' || ch == '-')
        .map(str::trim)
    {
        let candidate = normalized_workflow_token(stripped);
        if !candidate.is_empty() && !variants.iter().any(|existing| existing == &candidate) {
            variants.push(candidate);
        }
    }
    variants
}

fn workflow_gate_option_labels(contract: &Value, has_tools: Option<bool>) -> Vec<String> {
    let first_gate_id = workflow_first_gate_id(contract);
    workflow_gate_options(contract, &first_gate_id)
        .into_iter()
        .filter(|option| {
            has_tools
                .map(|expected| {
                    option
                        .get("has_tools")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                        == expected
                })
                .unwrap_or(true)
        })
        .filter_map(|option| {
            let label = workflow_option_label(&option);
            if label.is_empty() {
                None
            } else {
                Some(label)
            }
        })
        .collect()
}

fn workflow_gate_option_menu_entries(contract: &Value, has_tools: Option<bool>) -> Vec<String> {
    let first_gate_id = workflow_first_gate_id(contract);
    workflow_gate_options(contract, &first_gate_id)
        .into_iter()
        .filter(|option| {
            has_tools
                .map(|expected| {
                    option
                        .get("has_tools")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                        == expected
                })
                .unwrap_or(true)
        })
        .filter_map(|option| {
            let label = workflow_option_label(&option);
            if label.is_empty() {
                return None;
            }
            let leading_alias = option
                .get("aliases")
                .and_then(Value::as_array)
                .and_then(|aliases| aliases.iter().filter_map(Value::as_str).next())
                .map(|alias| clean_text(alias, 40))
                .filter(|alias| !alias.is_empty());
            Some(match leading_alias {
                Some(alias) => format!("{alias} = {label}"),
                None => label,
            })
        })
        .collect()
}

fn workflow_gate_1_allowed_outputs(contract: &Value) -> Value {
    let first_gate_id = workflow_first_gate_id(contract);
    workflow_contract_gate(contract, &first_gate_id)
        .pointer("/submission_contract/accepted_outputs")
        .cloned()
        .unwrap_or_else(|| json!([]))
}

fn workflow_tool_family_menu(contract: &Value, selected_family: &str) -> Value {
    let selected_family = clean_text(selected_family, 120);
    let rows = contract
        .get("tool_family_menu")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    Value::Array(
        rows.into_iter()
            .map(|mut row| {
                let key = clean_text(row.get("key").and_then(Value::as_str).unwrap_or(""), 120);
                row["selected"] =
                    Value::Bool(!selected_family.is_empty() && key == selected_family);
                row
            })
            .collect(),
    )
}

fn workflow_tool_menu_by_family(contract: &Value) -> Value {
    contract
        .get("tool_menu_by_family")
        .cloned()
        .unwrap_or_else(|| json!({}))
}

fn workflow_post_tool_options(contract: &Value) -> Value {
    let post_tool_gate_id = workflow_post_tool_gate_id(contract);
    workflow_contract_gate(contract, &post_tool_gate_id)
        .get("options")
        .cloned()
        .unwrap_or_else(|| json!([]))
}

fn workflow_final_output_contract(contract: &Value) -> Value {
    let final_gate_id = workflow_final_gate_id(contract);
    workflow_contract_gate(contract, &final_gate_id)
        .get("final_output_contract")
        .cloned()
        .unwrap_or_else(|| json!({}))
}

fn workflow_final_answer_instruction(contract: &Value) -> String {
    workflow_final_output_contract(contract)
        .get("chat_requirement")
        .and_then(Value::as_str)
        .map(|row| clean_text(row, 8_000))
        .unwrap_or_default()
}

fn workflow_final_answer_prompt_context() -> String {
    workflow_final_answer_instruction(&default_workflow_tool_menu_contract())
}

fn workflow_final_answer_prompt_context_for_tools(response_tools: &[Value]) -> String {
    let tool_contract = response_tools.iter().find_map(|tool| {
        let tool_key = tool
            .get("name")
            .or_else(|| tool.get("tool_name"))
            .or_else(|| tool.get("tool"))
            .and_then(Value::as_str)
            .map(|row| clean_text(row, 120))
            .filter(|row| !row.is_empty())?;
        Some(workflow_tool_menu_contract_for_tool_key(&tool_key))
    });
    tool_contract
        .map(|contract| workflow_final_answer_instruction(&contract))
        .filter(|row| !row.is_empty())
        .unwrap_or_else(workflow_final_answer_prompt_context)
}

fn workflow_example_tool_key(contract: &Value) -> String {
    contract
        .get("tool_menu_by_family")
        .and_then(Value::as_object)
        .and_then(|families| {
            families
                .values()
                .filter_map(Value::as_array)
                .flat_map(|tools| tools.iter())
                .filter_map(|tool| tool.get("key").and_then(Value::as_str))
                .next()
        })
        .map(|key| clean_text(key, 80))
        .unwrap_or_default()
}

fn workflow_tool_submission_format(contract: &Value) -> String {
    let first_gate_id = workflow_first_gate_id(contract);
    workflow_contract_gate(contract, &first_gate_id)
        .pointer("/submission_contract/accepted_outputs")
        .and_then(Value::as_array)
        .and_then(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .find(|row| !clean_text(row, 240).is_empty())
        })
        .map(|row| clean_text(row, 240))
        .unwrap_or_default()
}

fn workflow_message_matches_contract_markers(
    contract: &Value,
    pointer: &str,
    message: &str,
) -> bool {
    let lowered = clean_text(message, 1_200).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    contract
        .pointer(pointer)
        .and_then(Value::as_array)
        .map(|markers| {
            markers
                .iter()
                .filter_map(Value::as_str)
                .map(|marker| clean_text(marker, 120).to_ascii_lowercase())
                .filter(|marker| !marker.is_empty())
                .any(|marker| lowered.contains(&marker))
        })
        .unwrap_or(false)
}

fn render_workflow_instruction_template(contract: &Value, template: &str) -> String {
    let first_gate_id = workflow_first_gate_id(contract);
    let gate_prompt = workflow_contract_gate(contract, &first_gate_id)
        .get("question")
        .and_then(Value::as_str)
        .map(|row| clean_text(row, 120))
        .unwrap_or_default();
    let category_options = workflow_gate_option_menu_entries(contract, None).join("`, `");
    let no_tool_categories = workflow_gate_option_menu_entries(contract, Some(false)).join("`, `");
    let tool_bearing_categories =
        workflow_gate_option_menu_entries(contract, Some(true)).join("`, `");
    let tool_submission_format = workflow_tool_submission_format(contract);
    let example_tool_key = workflow_example_tool_key(contract);
    clean_text(
        &template
            .replace("{gate_question}", &gate_prompt)
            .replace("{category_options}", &format!("`{category_options}`"))
            .replace("{no_tool_categories}", &format!("`{no_tool_categories}`"))
            .replace(
                "{tool_bearing_categories}",
                &format!("`{tool_bearing_categories}`"),
            )
            .replace("{tool_submission_format}", &tool_submission_format)
            .replace("{example_tool_key}", &example_tool_key),
        1_400,
    )
}

fn workflow_category_token_matches(response: &str, has_tools: Option<bool>) -> bool {
    workflow_category_selection(&default_workflow_tool_menu_contract(), response, has_tools)
        .is_some()
}

fn response_contains_no_tool_gate_token_fragment(response: &str) -> bool {
    let token = normalized_workflow_token(response);
    if token.is_empty() {
        return false;
    }
    let haystack = format!(" {token} ");
    let contract = default_workflow_tool_menu_contract();
    let first_gate_id = workflow_first_gate_id(&contract);
    workflow_gate_options(&contract, &first_gate_id)
        .into_iter()
        .filter(|option| {
            !option
                .get("has_tools")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .flat_map(|option| normalized_workflow_option_tokens(&option))
        .any(|candidate| {
            candidate.split_whitespace().count() > 1 && haystack.contains(&format!(" {candidate} "))
        })
}

fn workflow_category_selection(
    contract: &Value,
    response: &str,
    has_tools: Option<bool>,
) -> Option<(String, String)> {
    let token = workflow_structured_gate_token(contract, response)
        .map(|row| normalized_workflow_token(&row))
        .unwrap_or_else(|| normalized_workflow_token(response));
    let token_variants = workflow_selection_token_variants(&token);
    if token_variants.is_empty() {
        return None;
    }
    let first_gate_id = workflow_first_gate_id(contract);
    workflow_gate_options(contract, &first_gate_id)
        .into_iter()
        .filter(|option| {
            has_tools
                .map(|expected| {
                    option
                        .get("has_tools")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                        == expected
                })
                .unwrap_or(true)
        })
        .find_map(|option| {
            let key = workflow_option_key(&option);
            let label = workflow_option_label(&option);
            normalized_workflow_option_tokens(&option)
                .into_iter()
                .any(|candidate| token_variants.iter().any(|token| token == &candidate))
                .then_some((key, label))
        })
}

fn workflow_category_token_matches_any_declared_option(
    response: &str,
    has_tools: Option<bool>,
) -> bool {
    let token = normalized_workflow_token(response);
    if token.is_empty() {
        return false;
    }
    let contract = default_workflow_tool_menu_contract();
    let first_gate_id = workflow_first_gate_id(&contract);
    workflow_gate_options(&contract, &first_gate_id)
        .into_iter()
        .filter(|option| {
            has_tools
                .map(|expected| {
                    option
                        .get("has_tools")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                        == expected
                })
                .unwrap_or(true)
        })
        .any(|option| {
            normalized_workflow_option_tokens(&option)
                .into_iter()
                .any(|candidate| token == candidate)
        })
}

fn workflow_family_key_for_selection(contract: &Value, family: &str) -> String {
    let selected_variants = workflow_selection_token_variants(family);
    if selected_variants.is_empty() {
        return String::new();
    }
    contract
        .get("tool_family_menu")
        .and_then(Value::as_array)
        .and_then(|families| {
            families.iter().find_map(|row| {
                let tokens = normalized_workflow_option_tokens(row);
                if tokens
                    .into_iter()
                    .any(|token| selected_variants.iter().any(|selected| selected == &token))
                {
                    Some(workflow_option_key(row))
                } else {
                    None
                }
            })
        })
        .unwrap_or_default()
}

fn workflow_tool_key_for_selection(contract: &Value, family: &str, tool_label: &str) -> String {
    let selected_variants = workflow_selection_token_variants(tool_label);
    if selected_variants.is_empty() {
        return String::new();
    }
    let selected_family = workflow_family_key_for_selection(contract, family);
    let Some(tool_menus) = contract
        .get("tool_menu_by_family")
        .and_then(Value::as_object)
    else {
        return String::new();
    };
    let families = if selected_family.is_empty() {
        tool_menus.values().collect::<Vec<_>>()
    } else {
        tool_menus
            .get(&selected_family)
            .into_iter()
            .collect::<Vec<_>>()
    };
    families
        .into_iter()
        .filter_map(Value::as_array)
        .flat_map(|tools| tools.iter())
        .find_map(|tool| {
            let key = workflow_option_key(tool);
            normalized_workflow_option_tokens(tool)
                .into_iter()
                .any(|candidate| selected_variants.iter().any(|selected| selected == &candidate))
                .then_some(key)
        })
        .unwrap_or_default()
}

fn workflow_tool_request_field_labels(contract: &Value, field: &str) -> Vec<String> {
    let field = clean_text(field, 80);
    if field.is_empty() {
        return Vec::new();
    }
    let mut labels = Vec::new();
    if let Some(label) = contract
        .pointer(&format!(
            "/tool_request_submission_contract/field_labels/{field}"
        ))
        .and_then(Value::as_str)
        .map(|row| clean_text(row, 80))
        .filter(|row| !row.is_empty())
    {
        labels.push(label);
    }
    if let Some(aliases) = contract
        .pointer(&format!(
            "/tool_request_submission_contract/field_aliases/{field}"
        ))
        .and_then(Value::as_array)
    {
        labels.extend(
            aliases
                .iter()
                .filter_map(Value::as_str)
                .map(|row| clean_text(row, 80))
                .filter(|row| !row.is_empty()),
        );
    }
    labels
}

fn workflow_tool_request_all_field_labels(contract: &Value) -> Vec<String> {
    contract
        .pointer("/tool_request_submission_contract/field_order")
        .and_then(Value::as_array)
        .map(|fields| {
            fields
                .iter()
                .filter_map(Value::as_str)
                .flat_map(|field| workflow_tool_request_field_labels(contract, field))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn manual_toolbox_selection_any_field(
    response: &str,
    labels: &[String],
    end_labels: &[String],
) -> String {
    labels
        .iter()
        .map(|label| {
            manual_toolbox_selection_field(
                response,
                label,
                &end_labels.iter().map(String::as_str).collect::<Vec<_>>(),
            )
        })
        .find(|value| !value.trim().is_empty())
        .unwrap_or_default()
}

fn workflow_trace_gates_from_contract(contract: &Value, first_gate_submission: &Value) -> Value {
    let gates = contract
        .get("gates")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let first_gate_id = workflow_first_gate_id(contract);
    Value::Array(
        workflow_contract_gate_order(contract)
            .into_iter()
            .filter_map(|gate_id| {
                let gate = gates.get(&gate_id)?;
                let mut row = json!({
                    "gate_id": gate_id,
                    "input_kind": gate.get("input_kind").cloned().unwrap_or_else(|| json!("")),
                    "question": gate.get("question").cloned().unwrap_or(Value::Null),
                    "options": gate.get("options").cloned().unwrap_or_else(|| json!([])),
                    "transition": gate.get("transition").cloned().unwrap_or(Value::Null),
                    "selection_mode": gate.get("input_kind").cloned().unwrap_or_else(|| json!("")),
                    "final_output_contract": gate.get("final_output_contract").cloned().unwrap_or(Value::Null)
                });
                if row.get("gate_id").and_then(Value::as_str) == Some(first_gate_id.as_str()) {
                    row["gate_submission"] = first_gate_submission.clone();
                }
                Some(row)
            })
            .collect::<Vec<_>>(),
    )
}

fn workflow_turn_tool_decision_tree(message: &str) -> Value {
    let contract = default_workflow_tool_menu_contract();
    let first_gate_id = workflow_first_gate_id(&contract);
    let first_gate = workflow_contract_gate(&contract, &first_gate_id);
    let cleaned_message = clean_text(message, 2_200);
    let lowered_message = cleaned_message.to_ascii_lowercase();
    let strong_live_web_freshness_cue = !cleaned_message.is_empty()
        && !message_refers_to_local_workspace_or_code(&cleaned_message, &lowered_message)
        && (lowered_message.contains("latest")
            || lowered_message.contains("current")
            || lowered_message.contains("today")
            || lowered_message.contains("this week")
            || lowered_message.contains("this month")
            || lowered_message.contains("this year")
            || lowered_message.contains("news")
            || lowered_message.contains("update")
            || message_has_explicit_year_scope(&cleaned_message));
    let requires_file_mutation = false;
    let requires_local_lookup = false;
    let requires_live_web =
        message_requires_information_search(message) || strong_live_web_freshness_cue;
    let explicit_web_intent = natural_web_search_query_from_message(message).is_some();
    let has_sufficient_information = false;
    let should_call_tools = false;
    let gate_decision_mode = "manual_work_category_v1";
    let reason_code = if requires_live_web {
        "advisory_external_information_required"
    } else {
        "manual_menu_presented"
    };
    let info_source = if explicit_web_intent {
        "explicit_web_query_detector"
    } else if requires_live_web {
        "external_information_detector"
    } else {
        "menu_only"
    };
    let selected_tool_family = "unselected";
    let meta_control = false;
    let status_check = false;
    let meta_diagnostic_request = false;
    let llm_should_answer_directly = false;
    let automatic_tool_calls_allowed = false;
    let tool_selection_authority = "llm_submitted_menu_or_text_input";
    let decision_authority_mode = "llm_manual_only_v1";
    let gate_enforcement_mode = "disabled";
    let gate_is_advisory = false;
    let workflow_retry_limit = 1;
    let needs_tool_access: Option<bool> = if requires_live_web { Some(true) } else { None };
    let selected_work_category = if requires_live_web {
        Value::String("web_research".to_string())
    } else {
        Value::Null
    };
    let gate_1_llm_submission = Value::Null;
    let gate_1_allowed_outputs = workflow_gate_1_allowed_outputs(&contract);
    let gate_1_submission_status = "awaiting_llm_submission";
    let gate_1_decision_source = if requires_live_web {
        "advisory_information_search_detector"
    } else {
        "pending_llm_submission"
    };
    let gate_prompt = clean_text(
        first_gate
            .get("question")
            .and_then(Value::as_str)
            .unwrap_or(""),
        120,
    );
    let first_gate_resume_token =
        workflow_gate_resume_token(&first_gate_id, gate_1_submission_status);
    let gate_submission = json!({
        "gate_id": first_gate_id,
        "input_shape": {
            "type": first_gate.get("input_kind").and_then(Value::as_str).unwrap_or(""),
            "allowed_outputs": gate_1_allowed_outputs.clone()
        },
        "llm_submission": gate_1_llm_submission,
        "accepted": false,
        "resume_token": first_gate_resume_token
    });
    let tool_family_menu = workflow_tool_family_menu(&contract, selected_tool_family);
    let tool_menu = json!([]);
    let tool_menu_by_family = workflow_tool_menu_by_family(&contract);
    let manual_tool_selection = true;
    let auto_decisions_disabled = true;
    let manual_gate_mode = "llm_only_multiple_choice_v1";
    let gate_1_options = workflow_gate_options(&contract, &first_gate_id)
        .into_iter()
        .enumerate()
        .map(|(idx, option)| {
            json!({
                "option": idx + 1,
                "key": workflow_option_key(&option),
                "label": workflow_option_label(&option),
                "has_tools": option.get("has_tools").and_then(Value::as_bool).unwrap_or(false)
            })
        })
        .collect::<Vec<_>>();
    let gate_5_options = workflow_post_tool_options(&contract);
    let gate_6_contract = workflow_final_output_contract(&contract);
    let gates = workflow_trace_gates_from_contract(&contract, &gate_submission);
    json!({
        "contract": "manual_toolbox_gate_v1", "workflow_gate_contract": "tool_menu_interface_v1",
        "gate_decision_mode": gate_decision_mode,
        "system_may_select_tools": false, "tool_recommendations_allowed": false,
        "gate_1_question_type": "multiple_choice", "gate_1_allowed_outputs": gate_1_allowed_outputs,
        "first_gate_id": gate_submission.get("gate_id").cloned().unwrap_or(Value::Null),
        "current_gate_id": gate_submission.get("gate_id").cloned().unwrap_or(Value::Null),
        "reason_code": reason_code,
        "requires_file_mutation": requires_file_mutation,
        "requires_local_lookup": requires_local_lookup,
        "requires_live_web": requires_live_web,
        "explicit_web_intent": explicit_web_intent,
        "has_sufficient_information": has_sufficient_information,
        "llm_should_answer_directly": llm_should_answer_directly,
        "should_call_tools": should_call_tools,
        "needs_tool_access": needs_tool_access,
        "selected_work_category": selected_work_category,
        "gate_1_submission_status": gate_1_submission_status,
        "gate_1_decision_source": gate_1_decision_source,
        "gate_submission": gate_submission.clone(),
        "gate_prompt": gate_prompt,
        "info_source": info_source,
        "selected_tool_family": selected_tool_family,
        "decision_authority_mode": decision_authority_mode,
        "gate_enforcement_mode": gate_enforcement_mode,
        "gate_is_advisory": gate_is_advisory,
        "tool_family_menu": tool_family_menu,
        "tool_menu": tool_menu,
        "tool_menu_by_family": tool_menu_by_family,
        "manual_tool_selection": manual_tool_selection, "auto_decisions_disabled": auto_decisions_disabled,
        "semantic_message_detectors_active": false,
        "manual_gate_mode": manual_gate_mode, "meta_control_message": meta_control,
        "status_check_message": status_check, "meta_diagnostic_request": meta_diagnostic_request,
        "automatic_tool_calls_allowed": automatic_tool_calls_allowed,
        "tool_selection_authority": tool_selection_authority,
        "workflow_retry_limit": workflow_retry_limit,
        "gate_1_options": gate_1_options,
        "post_tool_options": gate_5_options,
        "final_output_contract": gate_6_contract,
        "gates": gates
    })
}

fn workflow_library_prompt_context(message: &str, latent_tool_candidates: &[Value]) -> String {
    let _ = latent_tool_candidates;
    let _ = message;
    let contract = default_workflow_tool_menu_contract();
    contract
        .get("llm_gate_instruction")
        .and_then(Value::as_str)
        .map(|template| render_workflow_instruction_template(&contract, template))
        .unwrap_or_default()
}

fn workflow_tool_request_prompt_context(category_key: &str, category_label: &str) -> String {
    let contract = default_workflow_tool_menu_contract();
    let category_key = clean_text(category_key, 120);
    let category_label = clean_text(category_label, 120);
    let tools = contract
        .get("tool_menu_by_family")
        .and_then(Value::as_object)
        .and_then(|families| families.get(&category_key))
        .cloned()
        .unwrap_or_else(|| json!([]));
    let tools_json = serde_json::to_string(&tools).unwrap_or_else(|_| "[]".to_string());
    contract
        .get("llm_tool_request_instruction")
        .and_then(Value::as_str)
        .map(|template| {
            clean_text(
                &template
                    .replace("{selected_category_key}", &category_key)
                    .replace("{selected_category_label}", &category_label)
                    .replace("{selected_tool_menu_json}", &tools_json),
                4_000,
            )
        })
        .unwrap_or_default()
}

fn turn_workflow_requires_final_llm(
    response_tools: &[Value],
    workflow_events: &[Value],
    draft_response: &str,
) -> bool {
    let pending_confirmation_wait = response_tools.is_empty()
        && workflow_events.iter().any(|event| {
            matches!(
                event.get("kind").and_then(Value::as_str).unwrap_or(""),
                "manual_toolbox_pending_tool_request" | "pending_confirmation_required"
            )
        });
    if pending_confirmation_wait {
        return false;
    }
    if !response_tools.is_empty() || !workflow_events.is_empty() {
        return true;
    }
    let cleaned_draft = clean_text(draft_response, 4_000);
    if cleaned_draft.is_empty() {
        return true;
    }
    if response_is_exact_no_tool_gate_submission(&cleaned_draft) {
        return true;
    }
    if response_contains_no_tool_gate_token_fragment(&cleaned_draft) {
        return true;
    }
    if response_is_visible_workflow_gate_choice(&cleaned_draft) {
        return true;
    }
    let (without_inline_calls, inline_calls) = extract_inline_tool_calls(&cleaned_draft, 6);
    if !inline_calls.is_empty()
        || without_inline_calls
            .to_ascii_lowercase()
            .contains("<function=")
    {
        return true;
    }
    if response_is_no_findings_placeholder(&cleaned_draft)
        || response_looks_like_tool_ack_without_findings(&cleaned_draft)
        || workflow_response_requests_more_tooling(&cleaned_draft)
    {
        return true;
    }
    false
}

fn turn_workflow_stage_rows(
    workflow_mode: &str,
    response_tools: &[Value],
    workflow_events: &[Value],
    draft_response: &str,
) -> Vec<Value> {
    let requires_final_llm =
        turn_workflow_requires_final_llm(response_tools, workflow_events, draft_response);
    let contract = default_workflow_tool_menu_contract();
    let first_gate_id = workflow_first_gate_id(&contract);
    let final_gate_id = workflow_final_gate_id(&contract);
    let first_gate_question = workflow_contract_gate(&contract, &first_gate_id)
        .get("question")
        .and_then(Value::as_str)
        .map(|row| clean_text(row, 120))
        .unwrap_or_default();
    let _ = workflow_mode;
    let cleaned_draft = clean_text(draft_response, 2_000);
    let final_stage_status = if requires_final_llm {
        "pending_final_llm"
    } else {
        "no_post_synthesis_required"
    };
    if !requires_final_llm && response_tools.is_empty() && workflow_events.is_empty() {
        return vec![
            json!({
                "stage": first_gate_id,
                "status": "answered_no_tool_category",
                "selection_mode": "multiple_choice",
                "question": first_gate_question
            }),
            json!({
                "stage": final_gate_id,
                "status": "skipped_not_required",
                "source": "initial_llm_answer"
            }),
        ];
    }
    vec![
        json!({
            "stage": first_gate_id,
            "status": "presented"
        }),
        json!({
            "stage": "initial_model_interpretation",
            "status": if cleaned_draft.is_empty() {
                "completed_empty"
            } else {
                "completed"
            },
            "draft_response_state": if cleaned_draft.is_empty() {
                "empty"
            } else if response_is_no_findings_placeholder(&cleaned_draft) {
                "no_findings"
            } else if response_looks_like_tool_ack_without_findings(&cleaned_draft) {
                "ack_only"
            } else {
                "present"
            }
        }),
        json!({
            "stage": "tool_and_system_collection",
            "status": if response_tools.is_empty() && workflow_events.is_empty() {
                "no_external_events"
            } else {
                "collected"
            },
            "tool_count": response_tools.len(),
            "system_event_count": workflow_events.len()
        }),
        json!({
            "stage": "final_llm_response",
            "required": requires_final_llm,
            "status": final_stage_status
        }),
    ]
}

fn workflow_trace_status_message(contract: &Value, status: &str, field: &str) -> String {
    let status = clean_text(status, 120);
    let field = clean_text(field, 80);
    if status.is_empty() || field.is_empty() {
        return String::new();
    }
    contract
        .pointer(&format!("/trace_status_messages/{status}/{field}"))
        .or_else(|| contract.pointer(&format!("/trace_status_messages/default/{field}")))
        .and_then(Value::as_str)
        .map(|row| clean_text(row, 320))
        .unwrap_or_default()
}

fn workflow_trace_status_key(status: &str) -> String {
    match clean_text(status, 80).as_str() {
        "skipped_not_required" | "skipped_test" | "no_post_synthesis_required" => {
            "no_post_synthesis_required".to_string()
        }
        "invoke_failed" | "synthesis_failed" => "synthesis_failed".to_string(),
        other => clean_text(other, 80),
    }
}

fn turn_workflow_visibility(final_stage_status: &str) -> Value {
    let status = clean_text(final_stage_status, 80);
    let contract = default_workflow_tool_menu_contract();
    let first_gate_id = workflow_first_gate_id(&contract);
    let final_gate_id = workflow_final_gate_id(&contract);
    let first_gate_direct_status = workflow_gate_resume_token(&first_gate_id, "no_tool_category");
    let final_pending_status = workflow_gate_resume_token(&final_gate_id, "pending_final_llm");
    let final_synthesized_status = workflow_gate_resume_token(&final_gate_id, "synthesized");
    let final_unavailable_status =
        workflow_gate_resume_token(&final_gate_id, "skipped_missing_model");
    let final_diagnostic_status = workflow_gate_resume_token(&final_gate_id, "diagnostic_only");
    let final_failed_status = workflow_gate_resume_token(&final_gate_id, "final_synthesis_failed");
    let status_key = workflow_trace_status_key(&status);
    let ui_status = workflow_trace_status_message(&contract, &status_key, "ui");
    let agent_process_status =
        workflow_trace_status_message(&contract, &status_key, "agent_process");
    let debug_status = match status.as_str() {
        "pending_final_llm" => final_pending_status.as_str(),
        "synthesized" => final_synthesized_status.as_str(),
        "skipped_not_required" | "skipped_test" | "no_post_synthesis_required" => {
            first_gate_direct_status.as_str()
        }
        "skipped_missing_model" => final_unavailable_status.as_str(),
        "diagnostic_failure_pass_through" | "withheld_non_llm_fallback_response" => {
            final_diagnostic_status.as_str()
        }
        "synthesis_failed" | "invoke_failed" => final_failed_status.as_str(),
        _ => "",
    };
    json!({
        "current_stage": final_gate_id,
        "current_stage_status": status,
        "ui_status": ui_status,
        "agent_process_status": agent_process_status,
        "debug_status": debug_status,
        "final_chat_authority": "llm_only",
        "system_injected_chat_text_allowed": false,
        "formats": {
            "ui": ui_status,
            "agent_process": agent_process_status,
            "debug": debug_status
        }
    })
}

fn turn_workflow_direct_response_path(
    workflow_mode: &str,
    workflow_events: &[Value],
) -> &'static str {
    let _ = workflow_mode;
    let has_pending = workflow_events.iter().any(|event| {
        matches!(
            event.get("kind").and_then(Value::as_str).unwrap_or(""),
            "manual_toolbox_pending_tool_request" | "pending_confirmation_required"
        )
    });
    if has_pending {
        return "first_gate_pending_tool_confirmation";
    }
    let has_manual_toolbox_menu = workflow_events.iter().any(|event| {
        matches!(
            event.get("kind").and_then(Value::as_str).unwrap_or(""),
            "manual_toolbox_candidate_menu"
        )
    });
    if has_manual_toolbox_menu {
        return "first_gate_pending_llm_tool_choice";
    }
    "first_gate_unresolved"
}

fn workflow_selected_name(selected_workflow: &Value) -> String {
    clean_text(
        selected_workflow
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or(""),
        120,
    )
}

fn workflow_child_definition_from_selected(selected_workflow: &Value, workflow_id: &str) -> Value {
    let workflow_id = clean_text(workflow_id, 120);
    if workflow_id.is_empty() {
        return json!({});
    }
    selected_workflow
        .pointer(&format!("/child_workflow_definitions/{workflow_id}"))
        .filter(|value| value.is_object())
        .cloned()
        .unwrap_or_else(|| json!({}))
}

fn workflow_active_research_child_id(
    selected_workflow: &Value,
    response_tools: &[Value],
    requires_final_llm: bool,
    final_stage_status: &str,
    direct_response_path: &str,
) -> String {
    let selected_name = workflow_selected_name(selected_workflow);
    if selected_name != "research_synthesize_verify_v1" {
        return String::new();
    }
    let status = clean_text(final_stage_status, 120);
    if status == "pending_final_llm" || requires_final_llm {
        return "research_answer_synthesis".to_string();
    }
    if !status.is_empty()
        && status != "no_post_synthesis_required"
        && status != "presented"
        && status != "initial"
    {
        return "research_answer_verification".to_string();
    }
    if !response_tools.is_empty()
        || direct_response_path.contains("pending_tool")
        || direct_response_path.contains("pending_llm_tool_choice")
    {
        return "research_evidence_acquisition".to_string();
    }
    "research_objective_normalization".to_string()
}

fn workflow_active_child_runtime_payload(
    selected_workflow: &Value,
    response_tools: &[Value],
    requires_final_llm: bool,
    final_stage_status: &str,
    direct_response_path: &str,
) -> Value {
    let active_id = workflow_active_research_child_id(
        selected_workflow,
        response_tools,
        requires_final_llm,
        final_stage_status,
        direct_response_path,
    );
    if active_id.is_empty() {
        return json!({});
    }
    let child = workflow_child_definition_from_selected(selected_workflow, &active_id);
    if !child.is_object() {
        return json!({});
    }
    let phase_reason = if active_id == "research_objective_normalization" {
        "pre_tool_goal_normalization"
    } else if active_id == "research_evidence_acquisition" {
        "tool_execution_and_evidence_packaging"
    } else if active_id == "research_answer_synthesis" {
        "evidence_to_answer_draft"
    } else {
        "answer_verification_and_terminal_projection"
    };
    json!({
        "name": active_id,
        "description": child.get("description").cloned().unwrap_or_else(|| json!("")),
        "workflow_type": child.get("workflow_type").cloned().unwrap_or_else(|| json!("")),
        "source_path": child.get("source_path").cloned().unwrap_or_else(|| json!("")),
        "phase_reason": phase_reason,
        "final_output_contract": child.get("final_output_contract").cloned().unwrap_or_else(|| json!({})),
        "workflow_source_of_truth_contract": child.get("workflow_source_of_truth_contract").cloned().unwrap_or_else(|| json!({})),
        "workflow_composition_contract": child.get("workflow_composition_contract").cloned().unwrap_or_else(|| json!({}))
    })
}

fn turn_workflow_metadata(
    workflow_mode: &str,
    response_tools: &[Value],
    workflow_events: &[Value],
    draft_response: &str,
    message: &str,
) -> Value {
    let cleaned_draft = clean_text(draft_response, 4_000);
    let draft_response_state = if cleaned_draft.is_empty() {
        "empty"
    } else if response_is_no_findings_placeholder(&cleaned_draft) {
        "no_findings"
    } else if response_looks_like_tool_ack_without_findings(&cleaned_draft) {
        "ack_only"
    } else {
        "present"
    };
    let requires_final_llm =
        turn_workflow_requires_final_llm(response_tools, workflow_events, draft_response);
    let tool_gate = workflow_turn_tool_decision_tree(message);
    let contract = default_workflow_tool_menu_contract();
    let final_gate_id = workflow_final_gate_id(&contract);
    let final_stage_status = if requires_final_llm {
        "pending_final_llm"
    } else {
        "no_post_synthesis_required"
    };
    let visibility = turn_workflow_visibility(final_stage_status);
    let direct_response_path = turn_workflow_direct_response_path(workflow_mode, workflow_events);
    let mut selected_workflow = selected_turn_workflow(workflow_mode);
    let active_child_workflow = workflow_active_child_runtime_payload(
        &selected_workflow,
        response_tools,
        requires_final_llm,
        final_stage_status,
        direct_response_path,
    );
    if active_child_workflow.is_object() && !active_child_workflow.as_object().unwrap().is_empty() {
        selected_workflow["active_child_workflow"] = active_child_workflow.clone();
    }
    let synthesis_input =
        workflow_synthesis_input_for_final_response(message, response_tools, &selected_workflow);
    json!({
        "contract": "agent_workflow_library_v1",
        "current_stage": visibility
            .get("current_stage")
            .cloned()
            .unwrap_or_else(|| json!(final_gate_id)),
        "current_stage_status": visibility
            .get("current_stage_status")
            .cloned()
            .unwrap_or_else(|| json!(final_stage_status)),
        "ui_status": visibility
            .get("ui_status")
            .cloned()
            .unwrap_or_else(|| json!("")),
        "agent_process_status": visibility
            .get("agent_process_status")
            .cloned()
            .unwrap_or_else(|| json!("")),
        "debug_status": visibility
            .get("debug_status")
            .cloned()
            .unwrap_or_else(|| json!("")),
        "visibility": visibility,
        "workflow_gate": {
            "required": false,
            "status": "presented"
        },
        "tool_gate": tool_gate,
        "library": {
            "default_workflow": default_turn_workflow_name(),
            "available_workflows": turn_workflow_library_catalog()
        },
        "selected_workflow": selected_workflow,
        "synthesis_input": synthesis_input,
        "tool_count": response_tools.len(),
        "system_event_count": workflow_events.len(),
        "draft_response_state": draft_response_state,
        "findings_summary": clean_text(&response_tools_summary_for_user(response_tools, 4), 2_000),
        "failure_summary": clean_text(&response_tools_failure_reason_for_user(response_tools, 4), 2_000),
        "workflow_control": {
            "mode": "tool_menu_interface_v1",
            "direct_response_path": direct_response_path,
            "active_child_workflow_id": clean_text(
                active_child_workflow.get("name").and_then(Value::as_str).unwrap_or(""),
                120,
            ),
            "active_child_phase_reason": clean_text(
                active_child_workflow
                    .get("phase_reason")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                160,
            )
        },
        "system_events": workflow_events,
        "stage_statuses": turn_workflow_stage_rows(workflow_mode, response_tools, workflow_events, draft_response),
        "final_llm_response": {
            "required": requires_final_llm,
            "source": "workflow_post_synthesis"
        }
    })
}

fn set_turn_workflow_final_stage_status(workflow: &mut Value, status: &str) {
    let visibility = turn_workflow_visibility(status);
    let contract = default_workflow_tool_menu_contract();
    let final_gate_id = workflow_final_gate_id(&contract);
    workflow["current_stage"] = visibility
        .get("current_stage")
        .cloned()
        .unwrap_or_else(|| json!(final_gate_id.clone()));
    workflow["current_stage_status"] = visibility
        .get("current_stage_status")
        .cloned()
        .unwrap_or_else(|| json!(clean_text(status, 80)));
    workflow["ui_status"] = visibility
        .get("ui_status")
        .cloned()
        .unwrap_or_else(|| json!(""));
    workflow["agent_process_status"] = visibility
        .get("agent_process_status")
        .cloned()
        .unwrap_or_else(|| json!(""));
    workflow["debug_status"] = visibility
        .get("debug_status")
        .cloned()
        .unwrap_or_else(|| json!(""));
    workflow["visibility"] = visibility;
    if let Some(rows) = workflow
        .get_mut("stage_statuses")
        .and_then(Value::as_array_mut)
    {
        for row in rows.iter_mut() {
            if row
                .get("stage")
                .and_then(Value::as_str)
                .map(|value| value == "final_llm_response" || value == final_gate_id)
                .unwrap_or(false)
            {
                row["status"] = Value::String(clean_text(status, 80));
            }
        }
    }
    let selected_workflow = workflow
        .get("selected_workflow")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let active_child = workflow_active_child_runtime_payload(
        &selected_workflow,
        &[],
        matches!(clean_text(status, 120).as_str(), "pending_final_llm"),
        status,
        workflow
            .pointer("/workflow_control/direct_response_path")
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    if active_child.is_object() && !active_child.as_object().unwrap().is_empty() {
        workflow["selected_workflow"]["active_child_workflow"] = active_child.clone();
        workflow["workflow_control"]["active_child_workflow_id"] = Value::String(clean_text(
            active_child.get("name").and_then(Value::as_str).unwrap_or(""),
            120,
        ));
        workflow["workflow_control"]["active_child_phase_reason"] = Value::String(clean_text(
            active_child
                .get("phase_reason")
                .and_then(Value::as_str)
                .unwrap_or(""),
            160,
        ));
    }
}

fn workflow_response_requests_more_tooling(response: &str) -> bool {
    workflow_message_matches_contract_markers(
        &default_workflow_tool_menu_contract(),
        "/diagnostic_markers/deferred_tool_request_phrases",
        response,
    )
}

fn manual_toolbox_response_exposes_unresolved_tool_need(response: &str) -> bool {
    workflow_message_matches_contract_markers(
        &default_workflow_tool_menu_contract(),
        "/diagnostic_markers/unresolved_tool_need_phrases",
        response,
    )
}

fn response_is_no_tool_category_gate_submission(response: &str) -> bool {
    workflow_category_token_matches(response, Some(false))
}

fn response_is_tool_bearing_category_gate_submission(response: &str) -> bool {
    workflow_category_token_matches(response, Some(true))
}

fn response_is_manual_toolbox_gate_choice(response: &str) -> bool {
    let contract = default_workflow_tool_menu_contract();
    if let Some(request) = workflow_structured_gate_submission(response) {
        let has_family = workflow_tool_request_string_field(&request, &contract, "tool_family")
            .or_else(|| workflow_tool_request_string_field(&request, &contract, "family"))
            .is_some();
        let has_tool = workflow_tool_request_string_field(&request, &contract, "tool")
            .or_else(|| workflow_tool_request_string_field(&request, &contract, "tool_key"))
            .is_some();
        let has_payload = workflow_tool_request_object_field(&request, &contract, "request_payload")
            .and_then(|value| workflow_tool_request_payload_from_json_value(&value))
            .map(|payload| payload.is_object())
            .unwrap_or(false);
        if has_family && has_tool && has_payload {
            return true;
        }
    }
    let lowered = clean_text(response, 2_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let labels = workflow_tool_request_all_field_labels(&contract)
        .into_iter()
        .map(|label| label.to_ascii_lowercase())
        .collect::<Vec<_>>();
    !labels.is_empty()
        && labels
            .iter()
            .filter(|label| lowered.contains(label.as_str()))
            .count()
            >= 3
}

fn response_is_exact_no_tool_gate_submission(response: &str) -> bool {
    response_is_no_tool_category_gate_submission(response)
}

fn workflow_tool_request_object_field(
    request: &Value,
    contract: &Value,
    field: &str,
) -> Option<Value> {
    let mut candidate_keys = vec![field.to_string()];
    candidate_keys.extend(workflow_tool_request_field_labels(contract, field));
    let fallback_aliases = match field {
        "category" => vec![
            "selected_category_label",
            "selected_category",
            "category_label",
        ],
        "tool_family" => vec!["selected_tool_family", "tool family", "family"],
        "tool" => vec![
            "tool_name",
            "tool name",
            "selected_tool",
            "selected_tool_name",
            "selected_tool_key",
        ],
        "request_payload" => vec![
            "payload",
            "input",
            "arguments",
            "args",
            "parameters",
            "params",
        ],
        _ => Vec::new(),
    };
    candidate_keys.extend(fallback_aliases.into_iter().map(str::to_string));
    let normalized_candidates = candidate_keys
        .into_iter()
        .map(|row| normalized_workflow_token(&row))
        .collect::<Vec<_>>();
    request.as_object().and_then(|object| {
        object.iter().find_map(|(key, value)| {
            let normalized_key = normalized_workflow_token(key);
            normalized_candidates
                .iter()
                .any(|candidate| candidate == &normalized_key)
                .then_some(value.clone())
        })
    })
}

fn workflow_tool_request_payload_from_json_value(value: &Value) -> Option<Value> {
    if let Some(payload) = value.as_object() {
        return Some(Value::Object(payload.clone()));
    }
    if let Some(raw) = value.as_str() {
        return manual_toolbox_payload_json(raw);
    }
    None
}

fn workflow_tool_request_string_field(
    request: &Value,
    contract: &Value,
    field: &str,
) -> Option<String> {
    workflow_tool_request_object_field(request, contract, field).and_then(|value| {
        value
            .as_str()
            .map(|value| clean_text(value, 120))
            .filter(|value| !value.is_empty())
    })
}

fn manual_toolbox_pending_request_from_response(response: &str, message: &str) -> Option<Value> {
    if let Some(request) = workflow_structured_gate_submission(response) {
        let contract = default_workflow_tool_menu_contract();
        let family = workflow_tool_request_string_field(&request, &contract, "tool_family")
            .or_else(|| workflow_tool_request_string_field(&request, &contract, "category"))
            .unwrap_or_default();
        let tool_label =
            workflow_tool_request_string_field(&request, &contract, "tool").unwrap_or_default();
        let payload = workflow_tool_request_object_field(&request, &contract, "request_payload")
            .and_then(|value| workflow_tool_request_payload_from_json_value(&value));
        if !family.is_empty() && !tool_label.is_empty() && payload.is_some() {
            let tool_name = canonical_manual_toolbox_tool_name(&family, &tool_label);
            if !tool_name.is_empty() {
                let input = workflow_repair_recovered_request_payload(
                    &family,
                    &tool_label,
                    payload.unwrap_or_else(|| json!({})),
                    message,
                );
                let receipt_binding = crate::deterministic_receipt_hash(&json!({
                    "type": "manual_toolbox_pending_tool_request",
                    "tool_name": tool_name,
                    "input": input,
                    "message": clean_text(message, 600)
                }));
                return Some(json!({
                    "status": "pending_confirmation",
                    "source": request
                        .get("source")
                        .or_else(|| request.get("selection_source"))
                        .and_then(|value| value.as_str())
                        .map(|value| clean_text(value, 120))
                        .filter(|value| !value.is_empty())
                        .unwrap_or_else(|| "manual_toolbox_gate".to_string()),
                    "tool_name": tool_name,
                    "selected_tool_family": family,
                    "selected_tool_label": tool_label,
                    "input": input,
                    "receipt_binding": receipt_binding,
                    "chat_injection_allowed": false,
                    "execution_claim_allowed": false
                }));
            }
        }
    }

    if !response_is_manual_toolbox_gate_choice(response) {
        return None;
    }
    let contract = default_workflow_tool_menu_contract();
    let category_labels = workflow_tool_request_field_labels(&contract, "category");
    let family_labels = workflow_tool_request_field_labels(&contract, "tool_family");
    let tool_labels = workflow_tool_request_field_labels(&contract, "tool");
    let payload_labels = workflow_tool_request_field_labels(&contract, "request_payload");
    let after_category = family_labels
        .iter()
        .chain(tool_labels.iter())
        .chain(payload_labels.iter())
        .cloned()
        .collect::<Vec<_>>();
    let after_family = tool_labels
        .iter()
        .chain(payload_labels.iter())
        .cloned()
        .collect::<Vec<_>>();
    let family = manual_toolbox_selection_any_field(response, &family_labels, &after_family)
        .if_empty_then(|| {
            manual_toolbox_selection_any_field(response, &category_labels, &after_category)
        });
    let tool_label = manual_toolbox_selection_any_field(response, &tool_labels, &payload_labels);
    let payload_text = manual_toolbox_selection_any_field(response, &payload_labels, &[]);
    if family.is_empty() || tool_label.is_empty() || payload_text.trim().is_empty() {
        return None;
    }
    let tool_name = canonical_manual_toolbox_tool_name(&family, &tool_label);
    if tool_name.is_empty() {
        return None;
    }
    let input = manual_toolbox_payload_json(&payload_text)?;
    if !input.is_object() {
        return None;
    }
    let input = workflow_repair_recovered_request_payload(&family, &tool_label, input, message);
    let receipt_binding = crate::deterministic_receipt_hash(&json!({
        "type": "manual_toolbox_pending_tool_request",
        "tool_name": tool_name,
        "input": input,
        "message": clean_text(message, 600)
    }));
    Some(json!({
        "status": "pending_confirmation",
        "source": "manual_toolbox_gate",
        "tool_name": tool_name,
        "selected_tool_family": family,
        "selected_tool_label": tool_label,
        "input": input,
        "receipt_binding": receipt_binding,
        "chat_injection_allowed": false,
        "execution_claim_allowed": false
    }))
}

fn manual_toolbox_selection_field(response: &str, label: &str, end_labels: &[&str]) -> String {
    let lowered = response.to_ascii_lowercase();
    let normalized_label = label.to_ascii_lowercase();
    let Some(start) = lowered.find(&normalized_label) else {
        return String::new();
    };
    let value_start = start + normalized_label.len();
    let mut value_end = response.len();
    for end_label in end_labels {
        let normalized_end_label = end_label.to_ascii_lowercase();
        if let Some(end) = lowered[value_start..].find(&normalized_end_label) {
            value_end = value_end.min(value_start + end);
        }
    }
    clean_text(
        response
            .get(value_start..value_end)
            .unwrap_or("")
            .trim_matches([' ', '.', '\n', '\r']),
        2_000,
    )
}

trait EmptyStringExt {
    fn if_empty_then<F: FnOnce() -> String>(self, fallback: F) -> String;
}

impl EmptyStringExt for String {
    fn if_empty_then<F: FnOnce() -> String>(self, fallback: F) -> String {
        if self.trim().is_empty() {
            fallback()
        } else {
            self
        }
    }
}

fn manual_toolbox_payload_json(payload_text: &str) -> Option<Value> {
    let start = payload_text.find('{')?;
    let end = payload_text.rfind('}')?;
    if end < start {
        return None;
    }
    serde_json::from_str(payload_text.get(start..=end)?).ok()
}

fn canonical_manual_toolbox_tool_name(family: &str, tool_label: &str) -> String {
    if normalized_workflow_token(family) == "workspace files"
        && normalized_workflow_token(tool_label) == "workspace search"
    {
        return "workspace_search".to_string();
    }
    let default_contract = default_workflow_tool_menu_contract();
    let family_key = workflow_family_key_for_selection(&default_contract, family);
    let family_key = if family_key.is_empty() {
        clean_text(family, 120)
    } else {
        family_key
    };
    workflow_tool_key_for_selection(
        &workflow_tool_menu_contract_for_family(&family_key),
        &family_key,
        tool_label,
    )
}

fn response_is_visible_workflow_gate_choice(response: &str) -> bool {
    let lowered = clean_text(response, 2_000).to_ascii_lowercase();
    let trimmed = lowered.trim();
    if trimmed.is_empty() {
        return false;
    }
    let compact = trimmed
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch.is_whitespace() {
                ch
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let contract = default_workflow_tool_menu_contract();
    let tool_request_labels = workflow_tool_request_all_field_labels(&contract)
        .into_iter()
        .map(|label| label.to_ascii_lowercase())
        .collect::<Vec<_>>();
    let has_tool_request_labels = !tool_request_labels.is_empty()
        && tool_request_labels
            .iter()
            .filter(|label| trimmed.contains(label.as_str()))
            .count()
            >= 3;
    response_is_manual_toolbox_gate_choice(trimmed)
        || response_is_no_tool_category_gate_submission(trimmed)
        || response_is_tool_bearing_category_gate_submission(trimmed)
        || workflow_category_token_matches_any_declared_option(&compact, None)
        || has_tool_request_labels
}

fn strip_dangling_inline_tool_markup(text: &str) -> String {
    let mut cleaned = text.to_string();
    loop {
        let lowered = cleaned.to_ascii_lowercase();
        let Some(start) = lowered.find("<function=") else {
            break;
        };
        let tail = &cleaned[start..];
        let end_rel = tail
            .find("</function>")
            .map(|idx| idx + "</function>".len())
            .or_else(|| tail.find('\n'))
            .unwrap_or(tail.len());
        let end = start.saturating_add(end_rel).min(cleaned.len());
        if end <= start {
            break;
        }
        cleaned.replace_range(start..end, "");
    }
    cleaned.replace("</function>", "")
}

fn sanitize_workflow_final_response_candidate(response: &str) -> String {
    let (without_inline_calls, inline_calls) = extract_inline_tool_calls(response, 6);
    let candidate = if inline_calls.is_empty() {
        response
    } else {
        without_inline_calls.trim()
    };
    let cleaned = clean_chat_text(strip_dangling_inline_tool_markup(candidate).trim(), 32_000);
    normalize_response_field_json_wrapper(&cleaned).unwrap_or(cleaned)
}

#[cfg(test)]
mod workflow_control_tests {
    use super::*;

    #[test]
    fn visible_workflow_gate_choice_uses_json_declared_tokens() {
        assert!(response_is_visible_workflow_gate_choice("Respond directly"));
        assert!(response_is_visible_workflow_gate_choice(
            "Respond directly. Category: Respond directly. Tool family: None. Request payload: {}"
        ));
        assert!(!response_is_visible_workflow_gate_choice(
            "This kind of work is `Respond directly`."
        ));
        assert!(!response_is_visible_workflow_gate_choice(
            "Infring centers workflow-gated synthesis while OpenClaw emphasizes governed web/media tooling."
        ));
        let web_category =
            workflow_gate_option_labels(&default_workflow_tool_menu_contract(), Some(true))
                .into_iter()
                .find(|label| label == "Web research")
                .expect("web research option comes from workflow JSON");
        assert!(response_is_visible_workflow_gate_choice(&format!(
            "Category: {web_category}. Tool family: {web_category}. Tool: web_search. Request payload: {{\"source\":\"web\",\"query\":\"x\"}}."
        )));
        assert!(!response_is_visible_workflow_gate_choice("Need tools? Yes"));
        assert!(!response_is_visible_workflow_gate_choice(
            "I need tools to answer this well, but I have not run them yet."
        ));
    }

    #[test]
    fn workflow_prompt_contract_requires_private_exact_gate_submission() {
        let prompt =
            workflow_library_prompt_context("Use web search to compare agent frameworks.", &[]);
        assert!(prompt.contains("INTERNAL GATE"));
        assert!(prompt.contains("MUST output ONLY a JSON object"));
        assert!(prompt.contains(r#""gate": "<value>""#));
        assert!(prompt.contains("open the tool menu"));
        assert!(prompt.contains("A natural-language answer at this gate is invalid"));
        assert!(!prompt.contains("present exactly one gate"));
        assert!(!prompt.contains("If Yes, continue"));
    }

    #[test]
    fn research_workflow_projects_active_child_synthesis_contract() {
        let workflow = turn_workflow_metadata(
            "workflow=research_synthesize_verify_v1",
            &[json!({
                "name": "batch_query",
                "status": "ok",
                "result": "Battery chemistry and protein design examples were retrieved."
            })],
            &[],
            "",
            "What are some scientific breakthroughs in 2026?",
        );
        assert_eq!(
            workflow
                .pointer("/selected_workflow/active_child_workflow/name")
                .and_then(Value::as_str),
            Some("research_answer_synthesis")
        );
        assert_eq!(
            workflow
                .pointer("/workflow_control/active_child_workflow_id")
                .and_then(Value::as_str),
            Some("research_answer_synthesis")
        );
        assert_eq!(
            workflow
                .pointer("/synthesis_input/final_output_contract/schema_version")
                .and_then(Value::as_str),
            Some("research_answer_synthesis_final_output_contract_v1")
        );
    }

    #[test]
    fn research_workflow_final_stage_promotes_verification_child() {
        let mut workflow = turn_workflow_metadata(
            "workflow=research_synthesize_verify_v1",
            &[json!({
                "name": "batch_query",
                "status": "ok",
                "result": "Battery chemistry and protein design examples were retrieved."
            })],
            &[],
            "",
            "What are some scientific breakthroughs in 2026?",
        );
        set_turn_workflow_final_stage_status(&mut workflow, "synthesized");
        assert_eq!(
            workflow
                .pointer("/selected_workflow/active_child_workflow/name")
                .and_then(Value::as_str),
            Some("research_answer_verification")
        );
        assert_eq!(
            workflow
                .pointer("/workflow_control/active_child_workflow_id")
                .and_then(Value::as_str),
            Some("research_answer_verification")
        );
    }

    #[test]
    fn workflow_gate_prompt_does_not_embed_latent_tool_candidates() {
        let latent = json!([
            {"tool": "web_search", "reason": "hidden suggestion"},
            {"tool": "terminal_exec", "reason": "hidden suggestion"}
        ]);
        let prompt = workflow_library_prompt_context(
            "compare agent frameworks",
            latent.as_array().map(Vec::as_slice).unwrap_or(&[]),
        );
        assert!(!prompt.contains("hidden suggestion"));
        assert!(!prompt.contains("web_search"));
        assert!(!prompt.contains("terminal_exec"));
    }

    #[test]
    fn workflow_tool_request_prompt_comes_from_json_contract() {
        let prompt = workflow_tool_request_prompt_context("web_research", "Web research");
        assert!(prompt.contains("LEGACY INTERNAL GATE"));
        assert!(prompt.contains("output ONLY a JSON object"));
        assert!(prompt.contains("Available tools (JSON)"));
        assert!(prompt.contains("web_search"));
        assert!(default_workflow_tool_menu_contract()
            .get("llm_tool_request_instruction")
            .and_then(Value::as_str)
            .map(|template| template.contains("{selected_tool_menu_json}"))
            .unwrap_or(false));
    }

    #[test]
    fn workflow_final_answer_prompt_keeps_cd_synthesis_requirements() {
        let prompt = workflow_final_answer_prompt_context_for_tools(&[json!({
            "name": "web_search"
        })]);
        assert!(prompt.contains("User-visible reply"));
        assert!(prompt.contains("do not echo menu options"));
        assert!(prompt.contains("source-backed"));
        assert!(prompt.contains("best bounded answer"));
        assert!(prompt.contains("Never use training"));
        assert!(prompt.contains("do not let it carry the conclusion"));
        assert!(prompt.contains("did not use outside evidence as the decision basis"));
    }

    #[test]
    fn no_tool_gate_submission_is_exact_private_token() {
        assert!(response_is_exact_no_tool_gate_submission(
            "Respond directly"
        ));
        assert!(!response_is_exact_no_tool_gate_submission(
            "No, I can answer directly."
        ));
        assert!(!response_is_exact_no_tool_gate_submission(
            r#"{"final_answer":"I can answer this from memory."}"#
        ));
    }

    #[test]
    fn embedded_no_tool_gate_token_fragment_requires_final_llm() {
        let draft = "I will answer now. 1 = Respond directly";
        assert!(response_contains_no_tool_gate_token_fragment(draft));
        assert!(turn_workflow_requires_final_llm(&[], &[], draft));
    }

    #[test]
    fn manual_toolbox_tool_names_come_from_workflow_json_catalog() {
        assert_eq!(
            canonical_manual_toolbox_tool_name("Web research", "web_search"),
            "web_search"
        );
        assert_eq!(
            canonical_manual_toolbox_tool_name("Web research", "Search web"),
            "web_search"
        );
        assert_eq!(
            canonical_manual_toolbox_tool_name("Web research", "I would choose a menu item"),
            ""
        );
    }

    #[test]
    fn workflow_decision_tree_marks_broad_current_info_queries_as_web_research_advisory() {
        let decision =
            workflow_turn_tool_decision_tree("give me an update on the AI agentic landscape in may 2026");
        assert_eq!(
            decision.get("gate_decision_mode").and_then(Value::as_str),
            Some("manual_work_category_v1")
        );
        assert_eq!(
            decision.get("should_call_tools").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            decision.get("requires_live_web").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            decision
                .get("selected_work_category")
                .and_then(Value::as_str),
            Some("web_research")
        );
        assert_eq!(
            decision
                .get("gate_1_submission_status")
                .and_then(Value::as_str),
            Some("awaiting_llm_submission")
        );
        assert!(
            decision
                .pointer("/gate_submission/llm_submission")
                .is_some_and(Value::is_null)
        );
    }

    #[test]
    fn workflow_decision_tree_marks_year_scoped_external_questions_as_web_research_advisory() {
        let decision =
            workflow_turn_tool_decision_tree("what are some scientific breakthroughs 2026?");
        assert_eq!(
            decision.get("requires_live_web").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            decision
                .get("selected_work_category")
                .and_then(Value::as_str),
            Some("web_research")
        );
        assert_eq!(
            decision
                .get("selected_tool_family")
                .and_then(Value::as_str),
            Some("unselected")
        );
    }

    #[test]
    fn workflow_decision_tree_marks_latest_this_week_queries_as_web_research_advisory() {
        let prompt = "what are the latest agent framework changes this week?";
        let lowered = clean_text(prompt, 2_200).to_ascii_lowercase();
        assert_eq!(
            message_refers_to_local_workspace_or_code(prompt, &lowered),
            false
        );
        assert_eq!(message_requires_information_search(prompt), true);

        let decision = workflow_turn_tool_decision_tree(prompt);
        assert_eq!(
            decision.get("requires_live_web").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            decision
                .get("selected_work_category")
                .and_then(Value::as_str),
            Some("web_research")
        );
    }

    #[test]
    fn workflow_decision_tree_keeps_general_coding_questions_off_web_advisory() {
        let decision =
            workflow_turn_tool_decision_tree("what is the best way to structure this Rust module?");
        assert_eq!(
            decision.get("requires_live_web").and_then(Value::as_bool),
            Some(false)
        );
        assert!(decision.get("selected_work_category").is_some_and(Value::is_null));
    }

    #[test]
    fn structured_manual_toolbox_gate_submission_counts_as_gate_choice() {
        assert!(response_is_manual_toolbox_gate_choice(
            r#"{"tool_family":"web_research","tool":"batch_query","request_payload":{"query":"best agentic framework","queries":["best agentic framework independent comparison"],"aperture":"medium"}}"#
        ));
    }
}
