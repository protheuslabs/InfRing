#[path = "../eval_local_coding_phase1_mutation_spine.rs"]
mod eval_local_coding_phase1_mutation_spine;

use eval_local_coding_phase1_mutation_spine::phase1_mutation_spine_level1_report;
use std::process::ExitCode;

fn main() -> ExitCode {
    let runs = std::env::args()
        .find_map(|arg| arg.strip_prefix("--runs=").and_then(|raw| raw.parse().ok()))
        .unwrap_or(5);
    let report = phase1_mutation_spine_level1_report(runs);
    println!(
        "{}",
        serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
    );
    ExitCode::from((!report.ok) as u8)
}
