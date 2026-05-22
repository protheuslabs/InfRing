use infring_orchestration_v1::eval_forge_software_complexity_ladder::forge_software_complexity_ladder_lab_report;
use std::process::ExitCode;

fn main() -> ExitCode {
    let report = forge_software_complexity_ladder_lab_report();
    println!(
        "{}",
        serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
    );
    ExitCode::from((!report.ok) as u8)
}
