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
        truncate_to_char_boundary(&mut out, max_len);
        out = out.trim_end().to_string();
    }
    out
}

fn truncate_to_char_boundary(text: &mut String, max_len: usize) {
    if text.len() <= max_len {
        return;
    }

    let mut end = max_len.min(text.len());
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    text.truncate(end);
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

#[cfg(test)]
mod tests {
    use super::clean_text;

    #[test]
    fn clean_text_truncates_multibyte_without_panic() {
        let out = clean_text(Some("abcdé f"), 5);
        assert_eq!(out, "abcd");
    }

    #[test]
    fn clean_text_handles_zero_length_limit() {
        let out = clean_text(Some("é"), 0);
        assert_eq!(out, "");
    }

    #[test]
    fn clean_text_collapses_whitespace_and_removes_nuls() {
        let out = clean_text(Some("alpha\u{0000}\n\n beta\tgamma"), 64);
        assert_eq!(out, "alpha beta gamma");
    }
}
