// Layer ownership: Core Layer 2 (Scheduling + Execution) - agent runtime surface coordination.
pub mod agent;
pub mod capability_pack;
pub mod mcp;
pub mod merkle_receipt;
mod native_evidence;
mod native_prompt_policy;
mod native_synthetic_artifact;
pub mod native_tools;
mod native_workflow_artifact;
pub mod provider;
pub mod rbac_memory;
pub mod realtime_voice;
pub mod runtime_lane;
pub mod runtime_state;
pub mod scheduler;
pub mod telemetry;
pub mod template;
pub mod wasm_sandbox;

pub use agent::{
    AgentBuildError, AgentBuilder, AgentContract, AgentExecutionContext, AgentRunResult,
};
pub use capability_pack::{
    CapabilityPackCatalog, CapabilityPackSpec, IssueOpsCapabilityPack, LeadGenCapabilityPack,
    LocalCodingFilesCapabilityPack, ResearchCapabilityPack, SocialSignalCapabilityPack,
    WebOpsCapabilityPack,
};
pub use infring_agent_derive::{infring_agent, infring_tool};
pub use mcp::{mcp_handshake_receipt, McpBridge, McpServerConfig, McpTool};
pub use merkle_receipt::{
    merkle_receipt_options_from_value, merkle_receipt_payload, MerkleReceiptOptions,
};
pub use native_tools::{
    native_tool_observation_prompt, parse_native_tool_calls, NativeToolCall, NativeToolDispatcher,
    NativeToolReceipt,
};
pub use provider::{
    LocalEchoProvider, ProviderClient, ProviderClientRegistry, ProviderError, ProviderErrorCode,
    ProviderRequest, ProviderResponse,
};
pub use rbac_memory::{
    memory_read_allowed, memory_write_allowed, permission_manifest_from_value,
    permission_manifest_from_value_with_inheritance, permission_template_manifest,
    PermissionManifest, PermissionTrit,
};
pub use realtime_voice::{
    normalize_voice_session_request, voice_session_contract, VoiceSessionRequest,
};
pub use runtime_lane::{
    run_runtime_lane, run_runtime_lane_with_registry, RuntimeLaneRequest, RuntimeLaneResponse,
};
pub use runtime_state::{
    runtime_lane_state_load, runtime_lane_state_path, runtime_lane_state_release_gate_counters,
    runtime_lane_state_save, RuntimeLaneDurableState, RuntimeReleaseGateCounters,
};
pub use scheduler::{ScheduleEntry, SchedulePlan, Scheduler};
pub use telemetry::{ReceiptEvent, ReceiptSpan, ReceiptTraceSink, ReceiptVisualizer};
pub use template::{
    default_template_dir, scaffold_template, TemplateKind, TemplateScaffoldOptions,
    TemplateScaffoldResult,
};
pub use wasm_sandbox::{
    evaluate_wasm_execution_boundary, evaluate_wasm_policy, wasm_policy_from_value,
    wasm_policy_snapshot, WasmPolicyDecision, WasmSandboxPolicy,
};

#[macro_export]
macro_rules! agent {
    ($name:expr) => {{
        $crate::AgentBuilder::new($name)
    }};
    (
        $name:expr,
        preamble = $preamble:expr,
        provider = $provider:expr,
        tools = [$($tool:expr),* $(,)?]
    ) => {{
        let mut builder = $crate::AgentBuilder::new($name)
            .preamble($preamble)
            .provider($provider);
        $(
            builder = builder.tool($tool);
        )*
        builder
    }};
}

#[cfg(test)]
mod runtime_lane_integration_tests;
#[cfg(test)]
mod runtime_lane_unit_tests;
