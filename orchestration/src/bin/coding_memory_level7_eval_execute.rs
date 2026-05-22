#[path = "../coding_memory_runtime_bridge.rs"]
mod coding_memory_runtime_bridge;
#[path = "../eval_coding_memory_level7.rs"]
mod eval_coding_memory_level7;

use eval_coding_memory_level7::coding_memory_level7_report;
use std::process::ExitCode;

fn main() -> ExitCode {
    let report = coding_memory_level7_report();
    println!(
        "{}",
        serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
    );
    ExitCode::from((!report.ok) as u8)
}
