use crate::coding_memory_runtime_bridge::{
    millis_now, stable_hash, workspace_root, CodingMemoryRuntimeBridge,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPackSeedBatchReport {
    pub harness_kind: String,
    pub ok: bool,
    pub batch_root: String,
    pub attempt_count: usize,
    pub jobs: Vec<ContextPackJob>,
    pub failures: Vec<String>,
    pub operator_next_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPackJob {
    pub attempt_id: String,
    pub project_root: String,
    pub receipts_root: String,
    pub prompt_path: String,
    pub memory_db_path: String,
    pub memory_row_id: String,
    pub resume_token: String,
    pub task_goal: String,
    pub expected_selected_files: Vec<String>,
    pub forbidden_context_files: Vec<String>,
    pub expected_validation_fragments: Vec<String>,
    pub initial_file_hashes: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContextPackJudgeReport {
    pub harness_kind: &'static str,
    pub ok: bool,
    pub batch_root: String,
    pub attempt_count: usize,
    pub pass_count: usize,
    pub fail_count: usize,
    pub attempts: Vec<ContextPackAttemptJudge>,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContextPackAttemptJudge {
    pub attempt_id: String,
    pub ok: bool,
    pub checks: Vec<ContextPackCheck>,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContextPackCheck {
    pub id: &'static str,
    pub ok: bool,
    pub detail: String,
}

#[derive(Debug, Clone)]
struct ContextPackSpec {
    id: &'static str,
    task_goal: &'static str,
    files: &'static [(&'static str, &'static str)],
    expected_selected_files: &'static [&'static str],
    forbidden_context_files: &'static [&'static str],
    expected_validation_fragments: &'static [&'static str],
}

const SPECS: &[ContextPackSpec] = &[
    ContextPackSpec {
        id: "python_billing_tax",
        task_goal: "Add a tax-inclusive invoice total helper while preserving existing invoice formatting behavior.",
        files: &[
            ("README.md", "# Billing Service\n\nUse Python stdlib. Validate with unittest.\n"),
            ("pyproject.toml", "[tool.pytest.ini_options]\npythonpath = [\"src\"]\n"),
            ("src/billing/invoice.py", "from dataclasses import dataclass\n\n\n@dataclass(frozen=True)\nclass InvoiceLine:\n    sku: str\n    quantity: int\n    unit_price_cents: int\n\n\ndef subtotal_cents(lines: list[InvoiceLine]) -> int:\n    return sum(line.quantity * line.unit_price_cents for line in lines)\n\n\ndef format_invoice_total(lines: list[InvoiceLine]) -> str:\n    return f\"${subtotal_cents(lines) / 100:.2f}\"\n"),
            ("src/billing/__init__.py", "from billing.invoice import InvoiceLine, format_invoice_total, subtotal_cents\n"),
            ("tests/test_invoice.py", "import unittest\n\nfrom billing.invoice import InvoiceLine, format_invoice_total, subtotal_cents\n\n\nclass InvoiceTest(unittest.TestCase):\n    def test_subtotal(self):\n        self.assertEqual(subtotal_cents([InvoiceLine(\"a\", 2, 125)]), 250)\n\n    def test_format_total(self):\n        self.assertEqual(format_invoice_total([InvoiceLine(\"a\", 1, 250)]), \"$2.50\")\n\n\nif __name__ == \"__main__\":\n    unittest.main()\n"),
            ("docs/old_billing_plan.md", "Deprecated plan. Do not use for current invoice behavior.\n"),
            ("src/marketing/campaign.py", "def campaign_name():\n    return \"spring\"\n"),
        ],
        expected_selected_files: &["src/billing/invoice.py", "tests/test_invoice.py", "pyproject.toml"],
        forbidden_context_files: &["docs/old_billing_plan.md", "src/marketing/campaign.py"],
        expected_validation_fragments: &["PYTHONPATH=src", "unittest"],
    },
    ContextPackSpec {
        id: "typescript_cart_discount",
        task_goal: "Add a capped cart discount calculation while preserving existing cart subtotal behavior.",
        files: &[
            ("package.json", "{\"scripts\":{\"test\":\"node --test tests/cart.test.mjs\"},\"type\":\"module\"}\n"),
            ("src/cart.js", "export function subtotalCents(items) {\n  return items.reduce((sum, item) => sum + item.quantity * item.unitPriceCents, 0);\n}\n\nexport function describeCart(items) {\n  return `${items.length} items / ${subtotalCents(items)} cents`;\n}\n"),
            ("tests/cart.test.mjs", "import test from 'node:test';\nimport assert from 'node:assert/strict';\nimport { describeCart, subtotalCents } from '../src/cart.js';\n\ntest('subtotal cents', () => {\n  assert.equal(subtotalCents([{ quantity: 2, unitPriceCents: 150 }]), 300);\n});\n\ntest('describe cart', () => {\n  assert.equal(describeCart([{ quantity: 1, unitPriceCents: 125 }]), '1 items / 125 cents');\n});\n"),
            ("src/analytics.js", "export function track() { return 'noise'; }\n"),
            ("docs/roadmap.md", "Future ecommerce roadmap. Not relevant to current cart discount slice.\n"),
        ],
        expected_selected_files: &["src/cart.js", "tests/cart.test.mjs", "package.json"],
        forbidden_context_files: &["src/analytics.js", "docs/roadmap.md"],
        expected_validation_fragments: &["npm test", "node --test"],
    },
    ContextPackSpec {
        id: "rust_rate_limit",
        task_goal: "Add a burst allowance helper to the rate limiter while preserving allow_request behavior.",
        files: &[
            ("Cargo.toml", "[package]\nname = \"rate-limit-probe\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
            ("src/lib.rs", "#[derive(Debug, Clone, Copy)]\npub struct RateLimit {\n    pub max_per_minute: u32,\n}\n\nimpl RateLimit {\n    pub fn allow_request(&self, used_this_minute: u32) -> bool {\n        used_this_minute < self.max_per_minute\n    }\n}\n"),
            ("tests/rate_limit.rs", "use rate_limit_probe::RateLimit;\n\n#[test]\nfn allows_under_limit() {\n    assert!(RateLimit { max_per_minute: 3 }.allow_request(2));\n}\n\n#[test]\nfn rejects_at_limit() {\n    assert!(!RateLimit { max_per_minute: 3 }.allow_request(3));\n}\n"),
            ("benches/noise.txt", "Old benchmark notes.\n"),
            ("examples/demo.rs", "fn main() { println!(\"demo\"); }\n"),
        ],
        expected_selected_files: &["src/lib.rs", "tests/rate_limit.rs", "Cargo.toml"],
        forbidden_context_files: &["benches/noise.txt", "examples/demo.rs"],
        expected_validation_fragments: &["cargo test"],
    },
    ContextPackSpec {
        id: "go_incident_priority",
        task_goal: "Add high-priority incident detection while preserving existing destination routing.",
        files: &[
            ("go.mod", "module incidentprobe\n\ngo 1.22\n"),
            ("router/router.go", "package router\n\nfunc Destination(kind string) string {\n\tif kind == \"incident.created\" {\n\t\treturn \"incident\"\n\t}\n\tif len(kind) >= 7 && kind[:7] == \"oncall.\" {\n\t\treturn \"oncall\"\n\t}\n\treturn \"default\"\n}\n"),
            ("router/router_test.go", "package router\n\nimport \"testing\"\n\nfunc TestDestination(t *testing.T) {\n\tif Destination(\"incident.created\") != \"incident\" {\n\t\tt.Fatal(\"expected incident\")\n\t}\n\tif Destination(\"oncall.page\") != \"oncall\" {\n\t\tt.Fatal(\"expected oncall\")\n\t}\n}\n"),
            ("docs/escalation_archive.md", "Old escalation archive. Do not select for implementation context.\n"),
            ("cmd/demo/main.go", "package main\n\nfunc main() {}\n"),
        ],
        expected_selected_files: &["router/router.go", "router/router_test.go", "go.mod"],
        forbidden_context_files: &["docs/escalation_archive.md", "cmd/demo/main.go"],
        expected_validation_fragments: &["go test ./..."],
    },
    ContextPackSpec {
        id: "python_support_sla",
        task_goal: "Add SLA breach classification for support tickets while preserving queue assignment.",
        files: &[
            ("README.md", "# Support Queue\n\nStdlib Python package. Use unittest discovery.\n"),
            ("src/support_queue/tickets.py", "from dataclasses import dataclass\n\n\n@dataclass(frozen=True)\nclass Ticket:\n    ticket_id: str\n    priority: str\n    age_minutes: int\n\n\ndef assign_queue(ticket: Ticket) -> str:\n    if ticket.priority == \"urgent\":\n        return \"hotline\"\n    return \"standard\"\n"),
            ("tests/test_tickets.py", "import unittest\n\nfrom support_queue.tickets import Ticket, assign_queue\n\n\nclass TicketTest(unittest.TestCase):\n    def test_urgent_queue(self):\n        self.assertEqual(assign_queue(Ticket(\"t1\", \"urgent\", 5)), \"hotline\")\n\n    def test_standard_queue(self):\n        self.assertEqual(assign_queue(Ticket(\"t2\", \"normal\", 5)), \"standard\")\n\n\nif __name__ == \"__main__\":\n    unittest.main()\n"),
            ("src/support_queue/__init__.py", "from support_queue.tickets import Ticket, assign_queue\n"),
            ("docs/old_sla_matrix.md", "Deprecated SLA thresholds from a retired product.\n"),
            ("scripts/generate_fake_data.py", "print('fake')\n"),
        ],
        expected_selected_files: &["src/support_queue/tickets.py", "tests/test_tickets.py", "README.md"],
        forbidden_context_files: &["docs/old_sla_matrix.md", "scripts/generate_fake_data.py"],
        expected_validation_fragments: &["PYTHONPATH=src", "unittest"],
    },
];

pub fn seed_context_pack_batch(attempt_count: usize) -> ContextPackSeedBatchReport {
    let count = attempt_count.max(1);
    let batch_root = std::env::temp_dir().join(format!(
        "local-context-pack-builder-batch-{}-{}",
        std::process::id(),
        millis_now()
    ));
    let prompts_root = batch_root.join("prompts");
    let mut jobs = Vec::new();
    let mut failures = Vec::new();
    if let Err(error) = fs::create_dir_all(&prompts_root) {
        failures.push(format!("create_prompts_root_failed:{error}"));
    }
    for index in 0..count {
        let spec = &SPECS[index % SPECS.len()];
        match seed_context_pack_attempt(index + 1, spec, &batch_root, &prompts_root) {
            Ok(job) => jobs.push(job),
            Err(error) => failures.push(error),
        }
    }
    let report = ContextPackSeedBatchReport {
        harness_kind: "local_context_pack_builder_seed_v1".to_string(),
        ok: failures.is_empty() && jobs.len() == count,
        batch_root: batch_root.display().to_string(),
        attempt_count: jobs.len(),
        jobs,
        failures,
        operator_next_action: "spawn_one_worker_per_prompt_then_run_context_pack_judge".to_string(),
    };
    let _ = write_file(
        &batch_root.join("jobs.json"),
        &serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string()),
    );
    report
}

pub fn judge_context_pack_batch(batch_root: &Path) -> ContextPackJudgeReport {
    let mut failures = Vec::new();
    let jobs_path = batch_root.join("jobs.json");
    let jobs = fs::read_to_string(&jobs_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<ContextPackSeedBatchReport>(&raw).ok())
        .map(|report| report.jobs)
        .unwrap_or_else(|| {
            failures.push(format!("jobs_json_unreadable:{}", jobs_path.display()));
            Vec::new()
        });
    let attempts = jobs
        .iter()
        .map(judge_context_pack_attempt)
        .collect::<Vec<_>>();
    for attempt in &attempts {
        if !attempt.ok {
            failures.extend(
                attempt
                    .failures
                    .iter()
                    .map(|failure| format!("{}:{failure}", attempt.attempt_id)),
            );
        }
    }
    let pass_count = attempts.iter().filter(|attempt| attempt.ok).count();
    let fail_count = attempts.len().saturating_sub(pass_count);
    ContextPackJudgeReport {
        harness_kind: "local_context_pack_builder_judge_v1",
        ok: failures.is_empty() && !attempts.is_empty(),
        batch_root: batch_root.display().to_string(),
        attempt_count: attempts.len(),
        pass_count,
        fail_count,
        attempts,
        failures,
    }
}

fn seed_context_pack_attempt(
    ordinal: usize,
    spec: &ContextPackSpec,
    batch_root: &Path,
    prompts_root: &Path,
) -> Result<ContextPackJob, String> {
    let attempt_id = format!("attempt_{ordinal:02}_{}", spec.id);
    let run_root = batch_root.join(&attempt_id);
    let project_root = run_root.join("project");
    let receipts_root = run_root.join("receipts");
    let memory_db_path = run_root.join("runtime_memory.sqlite");
    fs::create_dir_all(&receipts_root)
        .map_err(|error| format!("{attempt_id}:create_receipts_failed:{error}"))?;
    for (path, content) in spec.files {
        write_file(&project_root.join(path), content)?;
    }
    let initial_file_hashes = project_file_hashes(&project_root)?;
    let resume_token = format!("context_pack_resume_{}_{}", attempt_id, millis_now());
    let memory_row_id = format!(
        "coding_memory::{}::context_hint::latest",
        stable_hash(&[&attempt_id, spec.task_goal])
    );
    let bridge = CodingMemoryRuntimeBridge {
        workspace_root: workspace_root(),
        memory_db_path: memory_db_path.clone(),
        session_id: attempt_id.clone(),
    };
    let payload = serde_json::to_string(&json!({
        "schema_version": "context_pack_memory_hint_v1",
        "attempt_id": attempt_id,
        "task_goal": spec.task_goal,
        "resume_token": resume_token,
        "hint": "Use current files as authority. Prefer source, tests, and manifest files directly tied to the task. Distractor docs and unrelated modules are stale hints only.",
        "expected_current_files": spec.expected_selected_files,
        "known_distractors": spec.forbidden_context_files
    }))
    .map_err(|error| format!("{attempt_id}:memory_payload_json_failed:{error}"))?;
    let ingest = bridge.ingest(
        &memory_row_id,
        &payload,
        &["coding", "context_pack", "resume", "project_context"],
    );
    if !ingest.ok {
        return Err(format!(
            "{attempt_id}:memory_ingest_failed:{}",
            ingest.payload
        ));
    }
    let prompt_path = prompts_root.join(format!("{attempt_id}.txt"));
    let job = ContextPackJob {
        attempt_id: attempt_id.clone(),
        project_root: project_root.display().to_string(),
        receipts_root: receipts_root.display().to_string(),
        prompt_path: prompt_path.display().to_string(),
        memory_db_path: memory_db_path.display().to_string(),
        memory_row_id,
        resume_token,
        task_goal: spec.task_goal.to_string(),
        expected_selected_files: spec
            .expected_selected_files
            .iter()
            .map(|value| value.to_string())
            .collect(),
        forbidden_context_files: spec
            .forbidden_context_files
            .iter()
            .map(|value| value.to_string())
            .collect(),
        expected_validation_fragments: spec
            .expected_validation_fragments
            .iter()
            .map(|value| value.to_string())
            .collect(),
        initial_file_hashes,
    };
    write_file(&prompt_path, &context_pack_worker_prompt(&job))?;
    Ok(job)
}

fn context_pack_worker_prompt(job: &ContextPackJob) -> String {
    format!(
        "You are executing the local_context_pack_builder workflow live eval. You are not alone in the broader codebase: do not modify anything outside {run_scope}.\n\nGoal: build a bounded local context pack for a future coding executor. Do not write code and do not run validation. Read current local files first; current files are authoritative over memory.\n\nEnvironment:\n- Project root: {project_root}\n- Receipt path: {receipt_path}\n- Isolated memory DB: {memory_db_path}\n- Resume token: {resume_token}\n- Memory row id: {memory_row_id}\n- Memory CLI pattern: INFRING_MEMORY_DB_PATH={memory_db_path} cargo run --quiet --manifest-path /Users/jay/.openclaw/workspace/core/layer0/memory/Cargo.toml --bin memory-cli -- <command>\n\nTask goal:\n{task_goal}\n\nWorkflow requirements:\n1. Read only the current local files needed to understand this coding slice: likely source, tests, manifests/config, and brief architecture/readme files.\n2. Retrieve the memory hint using the resume token and/or memory row id, but treat memory as hints only.\n3. Extract known distractor paths from the memory hint and prompt, then copy every known distractor path into the top-level excluded_files array without reading them. Do not hide distractors in memory_hint_refs, risks, notes, or another field. Do not include known distractors in read_files unless a selected current file explicitly proves they are required.\n4. Avoid irrelevant distractor files unless a current file directly proves they matter.\n5. Do not edit source files. Do not run tests or validation.\n6. Project a validation command without running it. Use the manifest/readme/test convention: Python unittest projects should include PYTHONPATH=src python -m unittest discover or equivalent; package.json scripts should include npm test; Cargo.toml projects should include cargo test; go.mod projects should include go test ./....\n7. Write JSON to {receipt_path} with these top-level fields exactly: schema_version, workflow_id, task_goal, project_root, source_of_truth_status, read_files, selected_files, selection_rationale_by_file, excluded_files, validation_commands, memory_hint_refs, open_questions, risks, confidence_score, validation_executed.\n8. Set source_of_truth_status to a sentence containing the exact words current and authoritative, for example: current local files are authoritative; memory is only a hint.\n9. Use workflow_id exactly \"local_context_pack_builder\" and schema_version exactly \"local_context_pack_builder_result_v1\".\n\nFinal response should summarize whether the context pack was written and list the selected files. Do not commit anything.\n",
        run_scope = Path::new(&job.project_root)
            .parent()
            .map(Path::display)
            .map(|value| value.to_string())
            .unwrap_or_else(|| job.project_root.clone()),
        project_root = job.project_root,
        receipt_path = Path::new(&job.receipts_root)
            .join("context_pack.json")
            .display(),
        memory_db_path = job.memory_db_path,
        resume_token = job.resume_token,
        memory_row_id = job.memory_row_id,
        task_goal = job.task_goal
    )
}

fn judge_context_pack_attempt(job: &ContextPackJob) -> ContextPackAttemptJudge {
    let mut checks = Vec::new();
    let mut failures = Vec::new();
    let project_root = PathBuf::from(&job.project_root);
    let receipt_path = PathBuf::from(&job.receipts_root).join("context_pack.json");
    let receipt = read_json_file(&receipt_path);
    push_check(
        &mut checks,
        &mut failures,
        "context_pack_receipt_written",
        receipt.is_some(),
        receipt_path.display().to_string(),
    );
    let current_hashes = project_file_hashes(&project_root).unwrap_or_default();
    push_check(
        &mut checks,
        &mut failures,
        "project_files_not_modified",
        current_hashes == job.initial_file_hashes,
        format!(
            "initial_files={} current_files={}",
            job.initial_file_hashes.len(),
            current_hashes.len()
        ),
    );
    if let Some(receipt) = &receipt {
        push_check(
            &mut checks,
            &mut failures,
            "declares_context_pack_workflow",
            receipt.get("workflow_id").and_then(Value::as_str)
                == Some("local_context_pack_builder"),
            receipt
                .get("workflow_id")
                .and_then(Value::as_str)
                .unwrap_or("missing_workflow_id")
                .to_string(),
        );
        let selected = receipt_paths(receipt, "selected_files", &project_root);
        let read = receipt_paths(receipt, "read_files", &project_root);
        let excluded = receipt_paths(receipt, "excluded_files", &project_root);
        let selected_has_expected = job
            .expected_selected_files
            .iter()
            .all(|expected| selected.iter().any(|actual| actual == expected));
        push_check(
            &mut checks,
            &mut failures,
            "selected_expected_context_files",
            selected_has_expected,
            format!("selected={}", selected.join(",")),
        );
        let forbidden_selected = job
            .forbidden_context_files
            .iter()
            .any(|forbidden| selected.iter().any(|actual| actual == forbidden));
        let forbidden_read = job
            .forbidden_context_files
            .iter()
            .any(|forbidden| read.iter().any(|actual| actual == forbidden));
        push_check(
            &mut checks,
            &mut failures,
            "avoids_forbidden_context_files",
            !forbidden_selected && !forbidden_read,
            format!("read={} selected={}", read.join(","), selected.join(",")),
        );
        let excluded_mentions_forbidden = job
            .forbidden_context_files
            .iter()
            .any(|forbidden| excluded.iter().any(|actual| actual == forbidden));
        push_check(
            &mut checks,
            &mut failures,
            "records_excluded_distractors",
            excluded_mentions_forbidden,
            format!("excluded={}", excluded.join(",")),
        );
        let validation_blob = receipt
            .get("validation_commands")
            .cloned()
            .unwrap_or(Value::Null);
        let validation_text = validation_blob.to_string();
        let validation_ok = job
            .expected_validation_fragments
            .iter()
            .any(|fragment| validation_text.contains(fragment));
        push_check(
            &mut checks,
            &mut failures,
            "projects_validation_command_without_running",
            validation_ok
                && receipt.get("validation_executed").and_then(Value::as_bool) == Some(false),
            validation_text,
        );
        let memory_text = receipt
            .get("memory_hint_refs")
            .cloned()
            .unwrap_or(Value::Null)
            .to_string();
        push_check(
            &mut checks,
            &mut failures,
            "records_memory_hint_refs",
            memory_text.contains(&job.memory_row_id) || memory_text.contains(&job.resume_token),
            memory_text,
        );
        let source_of_truth = receipt
            .get("source_of_truth_status")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_ascii_lowercase();
        push_check(
            &mut checks,
            &mut failures,
            "current_files_are_authoritative",
            source_of_truth.contains("current") && source_of_truth.contains("authoritative"),
            source_of_truth,
        );
        let confidence = receipt
            .get("confidence_score")
            .and_then(Value::as_f64)
            .unwrap_or(-1.0);
        push_check(
            &mut checks,
            &mut failures,
            "confidence_score_present",
            (0.0..=1.0).contains(&confidence),
            format!("confidence_score={confidence}"),
        );
        let rationale_ok = receipt
            .get("selection_rationale_by_file")
            .and_then(Value::as_object)
            .map(|rationale| rationale.len() >= job.expected_selected_files.len())
            .unwrap_or(false);
        push_check(
            &mut checks,
            &mut failures,
            "rationale_by_file_present",
            rationale_ok,
            "selection_rationale_by_file".to_string(),
        );
    }
    ContextPackAttemptJudge {
        attempt_id: job.attempt_id.clone(),
        ok: failures.is_empty(),
        checks,
        failures,
    }
}

fn receipt_paths(receipt: &Value, key: &str, project_root: &Path) -> Vec<String> {
    receipt
        .get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    item.as_str().or_else(|| {
                        item.get("path")
                            .or_else(|| item.get("file"))
                            .or_else(|| item.get("filepath"))
                            .and_then(Value::as_str)
                    })
                })
                .map(|path| normalize_receipt_path(path, project_root))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn normalize_receipt_path(path: &str, project_root: &Path) -> String {
    let raw = Path::new(path);
    if raw.is_absolute() {
        raw.strip_prefix(project_root)
            .map(|path| path.display().to_string())
            .unwrap_or_else(|_| raw.display().to_string())
    } else {
        raw.display().to_string()
    }
}

fn push_check(
    checks: &mut Vec<ContextPackCheck>,
    failures: &mut Vec<String>,
    id: &'static str,
    ok: bool,
    detail: String,
) {
    if !ok {
        failures.push(id.to_string());
    }
    checks.push(ContextPackCheck { id, ok, detail });
}

fn project_file_hashes(root: &Path) -> Result<BTreeMap<String, String>, String> {
    let mut out = BTreeMap::new();
    collect_file_hashes(root, root, &mut out)?;
    Ok(out)
}

fn collect_file_hashes(
    root: &Path,
    current: &Path,
    out: &mut BTreeMap<String, String>,
) -> Result<(), String> {
    for entry in fs::read_dir(current)
        .map_err(|error| format!("read_dir_failed:{}:{error}", current.display()))?
    {
        let entry = entry.map_err(|error| format!("read_dir_entry_failed:{error}"))?;
        let path = entry.path();
        if path.is_dir() {
            collect_file_hashes(root, &path, out)?;
        } else {
            let relative = path
                .strip_prefix(root)
                .map_err(|error| format!("strip_prefix_failed:{}:{error}", path.display()))?
                .display()
                .to_string();
            let content = fs::read_to_string(&path).unwrap_or_else(|_| String::new());
            out.insert(relative, stable_hash(&[&content]));
        }
    }
    Ok(())
}

fn read_json_file(path: &Path) -> Option<Value> {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
}

fn write_file(path: &Path, content: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create_parent_failed:{}:{error}", parent.display()))?;
    }
    fs::write(path, content)
        .map_err(|error| format!("write_file_failed:{}:{error}", path.display()))
}
