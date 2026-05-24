// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/ops (retrieval policy compatibility contracts).

use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::Path;

pub fn clean_text(raw: Option<&str>, max_len: usize) -> String {
    let mut out = raw.unwrap_or_default().replace('\u{0000}', " ");
    out = out.split_whitespace().collect::<Vec<_>>().join(" ");
    if out.len() > max_len {
        out.truncate(max_len);
        out = out.trim_end().to_string();
    }
    out
}

pub fn read_json(path: &Path) -> Result<Value, String> {
    let raw = fs::read_to_string(path).map_err(|err| format!("read_json_failed:{err}"))?;
    serde_json::from_str(&raw).map_err(|err| format!("parse_json_failed:{err}"))
}

pub fn append_jsonl(path: &Path, row: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("append_jsonl_parent_failed:{err}"))?;
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| format!("append_jsonl_open_failed:{err}"))?;
    let encoded =
        serde_json::to_string(row).map_err(|err| format!("append_jsonl_encode_failed:{err}"))?;
    writeln!(file, "{encoded}").map_err(|err| format!("append_jsonl_write_failed:{err}"))
}
