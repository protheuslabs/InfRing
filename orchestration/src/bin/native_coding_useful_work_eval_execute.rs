#[path = "../eval_native_coding_useful_work.rs"]
mod eval_native_coding_useful_work;

use eval_native_coding_useful_work::{
    judge_native_coding_useful_work_batch, seed_native_coding_useful_work_batch,
};
use std::path::PathBuf;

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    let Some(command) = args.get(1).map(String::as_str) else {
        eprintln!("usage: native_coding_useful_work_eval_execute seed --attempts=N | judge --batch-root=PATH");
        std::process::exit(2);
    };
    match command {
        "seed" => {
            let attempts = args
                .iter()
                .find_map(|arg| arg.strip_prefix("--attempts="))
                .and_then(|raw| raw.parse::<usize>().ok())
                .unwrap_or(4);
            let report = seed_native_coding_useful_work_batch(attempts);
            println!(
                "{}",
                serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
            );
            if !report.ok {
                std::process::exit(1);
            }
        }
        "judge" => {
            let Some(batch_root) = args
                .iter()
                .find_map(|arg| arg.strip_prefix("--batch-root="))
            else {
                eprintln!("missing --batch-root=PATH");
                std::process::exit(2);
            };
            let report = judge_native_coding_useful_work_batch(&PathBuf::from(batch_root));
            println!(
                "{}",
                serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
            );
            if !report.ok {
                std::process::exit(1);
            }
        }
        _ => {
            eprintln!("usage: native_coding_useful_work_eval_execute seed --attempts=N | judge --batch-root=PATH");
            std::process::exit(2);
        }
    }
}
