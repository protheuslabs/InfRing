// Layer ownership: Core Layer 2 (Scheduling + Execution) - agent runtime surface coordination.
mod command_run;
mod dispatcher;
mod export_guard;
mod file_discovery;
mod file_patch;
mod file_read;
mod file_write;
mod hashing;
mod paths;
mod protocol;
mod receipts;

pub use dispatcher::NativeToolDispatcher;
pub use protocol::{native_tool_observation_prompt, parse_native_tool_calls, NativeToolCall};
pub use receipts::NativeToolReceipt;
