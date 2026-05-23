// Layer ownership: orchestration (non-canonical orchestration coordination only).

#[derive(Debug, Clone)]
pub enum WorkflowInput {
    GateText {
        stage: &'static str,
        text: &'static str,
    },
    ToolObservation {
        ok: bool,
        summary: &'static str,
    },
    SynthesizeFromLatestToolResult,
    FinalAnswer(&'static str),
    Abort,
}

#[derive(Debug, Clone)]
pub struct WorkflowReplayFixture {
    pub id: &'static str,
    pub workflow_id: &'static str,
    pub user_input: &'static str,
    pub inputs: Vec<WorkflowInput>,
}

pub fn workflow_replay_fixtures() -> Vec<WorkflowReplayFixture> {
    vec![
        WorkflowReplayFixture {
            id: "direct_conversation",
            workflow_id: "plan_execute_review",
            user_input: "hey",
            inputs: vec![
                WorkflowInput::GateText {
                    stage: "gate_1_need_tool_access_menu",
                    text: "No",
                },
                WorkflowInput::FinalAnswer("Hey. I can answer directly without tools."),
            ],
        },
        WorkflowReplayFixture {
            id: "workspace_search",
            workflow_id: "plan_execute_review",
            user_input: "Find the workflow policy file in the workspace.",
            inputs: vec![
                WorkflowInput::GateText {
                    stage: "gate_1_need_tool_access_menu",
                    text: "Yes",
                },
                WorkflowInput::GateText {
                    stage: "gate_2_tool_family_menu",
                    text: "1",
                },
                WorkflowInput::GateText {
                    stage: "gate_3_tool_menu",
                    text: "search_workspace",
                },
                WorkflowInput::GateText {
                    stage: "gate_4_request_payload_input",
                    text: "workflow_json_format_policy.md",
                },
                WorkflowInput::ToolObservation {
                    ok: true,
                    summary: "workspace result found",
                },
                WorkflowInput::FinalAnswer("Found the workspace policy file."),
            ],
        },
        WorkflowReplayFixture {
            id: "web_search",
            workflow_id: "research_synthesize_verify",
            user_input: "Search the web for current agent framework comparisons.",
            inputs: vec![
                WorkflowInput::GateText {
                    stage: "gate_1_need_tool_access_menu",
                    text: "Yes",
                },
                WorkflowInput::GateText {
                    stage: "gate_2_tool_family_menu",
                    text: "2",
                },
                WorkflowInput::GateText {
                    stage: "gate_3_tool_menu",
                    text: "web_search",
                },
                WorkflowInput::GateText {
                    stage: "gate_4_request_payload_input",
                    text: "agentic framework comparison April 2026",
                },
                WorkflowInput::ToolObservation {
                    ok: true,
                    summary: "web results returned",
                },
                WorkflowInput::FinalAnswer("Synthesized web findings."),
            ],
        },
        WorkflowReplayFixture {
            id: "failed_tool_retry",
            workflow_id: "diagnose_retry_escalate",
            user_input: "Try a web search and recover if it fails.",
            inputs: vec![
                WorkflowInput::GateText {
                    stage: "gate_1_need_tool_access_menu",
                    text: "Yes",
                },
                WorkflowInput::GateText {
                    stage: "gate_2_tool_family_menu",
                    text: "web",
                },
                WorkflowInput::GateText {
                    stage: "gate_3_tool_menu",
                    text: "web_search",
                },
                WorkflowInput::GateText {
                    stage: "gate_4_request_payload_input",
                    text: "recoverable search",
                },
                WorkflowInput::ToolObservation {
                    ok: false,
                    summary: "temporary provider failure",
                },
                WorkflowInput::ToolObservation {
                    ok: true,
                    summary: "retry returned results",
                },
                WorkflowInput::FinalAnswer("Recovered after one retry."),
            ],
        },
        WorkflowReplayFixture {
            id: "user_aborted",
            workflow_id: "plan_execute_review",
            user_input: "Cancel the tool workflow.",
            inputs: vec![WorkflowInput::Abort],
        },
    ]
}
