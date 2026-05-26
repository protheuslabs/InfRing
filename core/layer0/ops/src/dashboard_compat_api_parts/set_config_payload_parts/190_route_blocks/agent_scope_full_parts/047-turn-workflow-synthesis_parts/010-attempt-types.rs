struct WorkflowFinalResponseAttemptContext<'a> {
    message: &'a str,
    response_tools: &'a [Value],
    enriched_workflow_events: &'a [Value],
    final_context_messages: &'a [Value],
    cleaned_provider: &'a str,
    cleaned_model: &'a str,
    system_prompt: &'a str,
    user_prompt: &'a str,
    final_answer_instruction: &'a str,
    tool_state_summary: &'a str,
    answer_unit_synthesis_block: &'a str,
    missing_turn_tool_context_block: &'a str,
    synthesis_input_json: &'a str,
    tool_rows_json: &'a str,
    recent_context: &'a str,
    template_label: &'a str,
    detail_style: &'a str,
    max_attempts: u64,
    manual_toolbox_gate_turn: bool,
    direct_gate_recovery_turn: bool,
    missing_turn_tool_context_recovery: bool,
    has_structured_block_evidence: bool,
}

#[derive(Default)]
struct WorkflowFinalResponseGateState {
    manual_toolbox_no_selected: bool,
    manual_toolbox_selected_category_key: String,
    manual_toolbox_selected_category_label: String,
    manual_toolbox_selected_family_key: String,
    manual_toolbox_selected_family_label: String,
    manual_toolbox_selected_tool_key: String,
    manual_toolbox_selected_tool_label: String,
}

#[derive(Default)]
struct WorkflowFinalResponseDiagnostics {
    last_error: String,
    last_invalid_response_text: String,
    last_invalid_excerpt: String,
    last_reject_reason: String,
    synthesis_attempt_count: u64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum WorkflowFinalResponseAttemptResult {
    Finalize,
    Exhausted,
}

impl WorkflowFinalResponseAttemptResult {
    fn should_finalize(self) -> bool {
        matches!(self, Self::Finalize)
    }
}
