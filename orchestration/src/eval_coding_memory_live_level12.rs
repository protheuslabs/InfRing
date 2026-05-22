use crate::coding_memory_runtime_bridge::{
    millis_now, project_snapshot, workspace_root, CodingMemoryRuntimeBridge,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::UNIX_EPOCH;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveLevel12SeedBatchReport {
    pub harness_kind: String,
    pub ok: bool,
    pub batch_root: String,
    pub seed_started_at_unix_ms: Option<u128>,
    pub attempt_count: usize,
    pub jobs: Vec<LiveLevel12Job>,
    pub failures: Vec<String>,
    pub operator_next_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveLevel12Job {
    pub attempt_id: String,
    pub package: String,
    pub run_root: String,
    pub project_root: String,
    pub receipts_root: String,
    pub prompt_path: String,
    pub memory_db_path: String,
    pub resume_token: String,
    pub prior_memory_row_id: String,
    pub expected_checkpoint8_memory_row_id: String,
    pub expected_checkpoint9_memory_row_id: String,
    pub project_fingerprint: String,
    pub architecture_hash: String,
    pub validation_command: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LiveLevel12JudgeReport {
    pub harness_kind: &'static str,
    pub ok: bool,
    pub batch_root: String,
    pub attempt_count: usize,
    pub pass_count: usize,
    pub fail_count: usize,
    pub timing: LiveLevel12TimingSummary,
    pub attempts: Vec<LiveLevel12AttemptJudge>,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LiveLevel12AttemptJudge {
    pub attempt_id: String,
    pub ok: bool,
    pub timing: LiveLevel12AttemptTiming,
    pub checks: Vec<LiveLevel12Check>,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LiveLevel12TimingSummary {
    pub batch_started_at_unix_ms: Option<u128>,
    pub judged_at_unix_ms: u128,
    pub batch_elapsed_ms: Option<u128>,
    pub first_attempt_completed_at_unix_ms: Option<u128>,
    pub last_attempt_completed_at_unix_ms: Option<u128>,
    pub completion_span_ms: Option<u128>,
    pub average_attempt_elapsed_ms: Option<u128>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LiveLevel12AttemptTiming {
    pub first_receipt_unix_ms: Option<u128>,
    pub completed_at_unix_ms: Option<u128>,
    pub elapsed_ms_since_batch_start: Option<u128>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LiveLevel12Check {
    pub id: &'static str,
    pub ok: bool,
    pub detail: String,
}

#[derive(Debug, Clone)]
struct DomainSpec {
    id: &'static str,
    package: &'static str,
    architecture_name: &'static str,
    default_owner: &'static str,
    critical_item_id: &'static str,
    normal_item_id: &'static str,
}

const DOMAIN_SPECS: &[DomainSpec] = &[
    DomainSpec {
        id: "clinic_queue_ops",
        package: "clinic_queue_ops",
        architecture_name: "Clinic Queue Ops",
        default_owner: "care-team",
        critical_item_id: "visit-delay-001",
        normal_item_id: "forms-followup-001",
    },
    DomainSpec {
        id: "warehouse_queue_ops",
        package: "warehouse_queue_ops",
        architecture_name: "Warehouse Queue Ops",
        default_owner: "fulfillment",
        critical_item_id: "dock-blocked-001",
        normal_item_id: "label-review-001",
    },
    DomainSpec {
        id: "incident_queue_ops",
        package: "incident_queue_ops",
        architecture_name: "Incident Queue Ops",
        default_owner: "oncall",
        critical_item_id: "sev-followup-001",
        normal_item_id: "postmortem-draft-001",
    },
];

pub fn seed_live_level12_batch(attempt_count: usize) -> LiveLevel12SeedBatchReport {
    let count = attempt_count.max(1);
    let seed_started_at_unix_ms = millis_now();
    let batch_root = std::env::temp_dir().join(format!(
        "coding-memory-live-level12-batch-{}-{}",
        std::process::id(),
        seed_started_at_unix_ms
    ));
    let prompts_root = batch_root.join("prompts");
    let mut jobs = Vec::new();
    let mut failures = Vec::new();

    if let Err(error) = fs::create_dir_all(&prompts_root) {
        failures.push(format!("create_prompts_root_failed:{error}"));
    }

    for index in 0..count {
        let spec = &DOMAIN_SPECS[index % DOMAIN_SPECS.len()];
        match seed_live_attempt(index + 1, spec, &batch_root, &prompts_root) {
            Ok(job) => jobs.push(job),
            Err(error) => failures.push(error),
        }
    }

    let report = LiveLevel12SeedBatchReport {
        harness_kind: "coding_memory_live_level12_seed_v1".to_string(),
        ok: failures.is_empty() && jobs.len() == count,
        batch_root: batch_root.display().to_string(),
        seed_started_at_unix_ms: Some(seed_started_at_unix_ms),
        attempt_count: jobs.len(),
        jobs,
        failures,
        operator_next_action: "spawn_one_worker_per_prompt_then_run_judge".to_string(),
    };
    let _ = write_file(
        &batch_root.join("jobs.json"),
        &serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string()),
    );
    report
}

pub fn judge_live_level12_batch(batch_root: &Path) -> LiveLevel12JudgeReport {
    let mut failures = Vec::new();
    let jobs_path = batch_root.join("jobs.json");
    let seed_report = fs::read_to_string(&jobs_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<LiveLevel12SeedBatchReport>(&raw).ok());
    let batch_started_at_unix_ms = seed_report
        .as_ref()
        .and_then(|report| report.seed_started_at_unix_ms)
        .or_else(|| file_modified_unix_ms(&jobs_path));
    let jobs = match seed_report {
        Some(report) => report.jobs,
        None => {
            failures.push(format!("jobs_json_unreadable:{}", jobs_path.display()));
            Vec::new()
        }
    };

    let attempts = jobs
        .iter()
        .map(|job| judge_live_attempt(job, batch_started_at_unix_ms))
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
    let judged_at_unix_ms = millis_now();
    let timing = summarize_level12_timing(batch_started_at_unix_ms, judged_at_unix_ms, &attempts);
    LiveLevel12JudgeReport {
        harness_kind: "coding_memory_live_level12_judge_v1",
        ok: failures.is_empty() && !attempts.is_empty(),
        batch_root: batch_root.display().to_string(),
        attempt_count: attempts.len(),
        pass_count,
        fail_count,
        timing,
        attempts,
        failures,
    }
}

fn seed_live_attempt(
    ordinal: usize,
    spec: &DomainSpec,
    batch_root: &Path,
    prompts_root: &Path,
) -> Result<LiveLevel12Job, String> {
    let attempt_id = format!("attempt_{ordinal:02}_{}", spec.id);
    let run_root = batch_root.join(&attempt_id);
    let project_root = run_root.join("project");
    let receipts_root = run_root.join("receipts");
    let memory_db_path = run_root.join("runtime_memory.sqlite");
    fs::create_dir_all(&receipts_root).map_err(|error| {
        format!(
            "{attempt_id}:create_receipts_root_failed:{}:{error}",
            receipts_root.display()
        )
    })?;
    seed_python_project(spec, &project_root)?;

    let validation = run_python_validation(&project_root);
    if !validation.ok {
        return Err(format!(
            "{attempt_id}:seed_validation_failed:{}",
            validation.detail
        ));
    }

    let architecture_text = read_to_string(&project_root.join("ARCHITECTURE.md"));
    let manifest_text = read_to_string(&project_root.join("PROJECT_MANIFEST.txt"));
    let validation_command = "PYTHONPATH=src python3 -m unittest discover -s tests";
    let snapshot = project_snapshot(
        &attempt_id,
        &project_root,
        &architecture_text,
        &manifest_text,
        validation_command,
    );
    let resume_token = format!("live_level12_resume_{}_{}", attempt_id, millis_now());
    let prior_memory_row_id = format!(
        "coding_memory::{}::checkpoint::checkpoint_007",
        snapshot.project_fingerprint
    );
    let expected_checkpoint8_memory_row_id = format!(
        "coding_memory::{}::checkpoint::checkpoint_008",
        snapshot.project_fingerprint
    );
    let expected_checkpoint9_memory_row_id = format!(
        "coding_memory::{}::checkpoint::checkpoint_009",
        snapshot.project_fingerprint
    );
    let bridge = CodingMemoryRuntimeBridge {
        workspace_root: workspace_root(),
        memory_db_path: memory_db_path.clone(),
        session_id: attempt_id.clone(),
    };
    let prior_payload = serde_json::to_string(&json!({
        "schema_version": "checkpoint_memory_write_v1",
        "project_id": attempt_id,
        "project_root": project_root.display().to_string(),
        "project_fingerprint": snapshot.project_fingerprint,
        "architecture_hash": snapshot.architecture_hash,
        "completed_checkpoint": "checkpoint_007_snapshot_recovery_and_backward_compatibility",
        "changed_files": [
            "ARCHITECTURE.md",
            "PROJECT_MANIFEST.txt",
            &format!("src/{}/models.py", spec.package),
            &format!("src/{}/repository.py", spec.package),
            &format!("src/{}/policy.py", spec.package),
            &format!("src/{}/service.py", spec.package),
            &format!("src/{}/cli.py", spec.package),
            "tests/test_existing_queue_behavior.py"
        ],
        "validation_results": {
            "status": "pass",
            "command": validation_command,
            "exit_code": 0
        },
        "recommended_next_checkpoint": "checkpoint_008_existing_codebase_safe_feature_and_bugfix",
        "next_slice_goal": "Evolve the existing queue package in place. Preserve item-create, item-complete, and queue-summary while adding idempotent external bulk import, hold-aware SLA reporting, and regression tests that prove old behavior still works.",
        "constraints": [
            "read current files before planning",
            "current workspace files remain authoritative over memory",
            "use Python stdlib only",
            "modify the existing modules instead of replacing the app with a disconnected implementation",
            "preserve public CLI contracts and existing tests",
            "write a roadmap before implementation",
            "stop rather than guess if a product-owned SLA threshold or data-contract decision is ambiguous"
        ],
        "unique_probe_token": resume_token
    }))
    .map_err(|error| format!("{attempt_id}:prior_payload_json_failed:{error}"))?;
    let ingest = bridge.ingest(
        &prior_memory_row_id,
        &prior_payload,
        &["coding", "checkpoint", "resume", "project_context"],
    );
    if !ingest.ok {
        return Err(format!(
            "{attempt_id}:prior_memory_ingest_failed:{}",
            ingest.payload
        ));
    }

    let prompt_path = prompts_root.join(format!("{attempt_id}.txt"));
    let job = LiveLevel12Job {
        attempt_id: attempt_id.clone(),
        package: spec.package.to_string(),
        run_root: run_root.display().to_string(),
        project_root: project_root.display().to_string(),
        receipts_root: receipts_root.display().to_string(),
        prompt_path: prompt_path.display().to_string(),
        memory_db_path: memory_db_path.display().to_string(),
        resume_token,
        prior_memory_row_id,
        expected_checkpoint8_memory_row_id,
        expected_checkpoint9_memory_row_id,
        project_fingerprint: snapshot.project_fingerprint,
        architecture_hash: snapshot.architecture_hash,
        validation_command: validation_command.to_string(),
    };
    write_file(&prompt_path, &worker_prompt(&job)).map_err(|error| {
        format!(
            "{attempt_id}:write_worker_prompt_failed:{}:{error}",
            prompt_path.display()
        )
    })?;
    Ok(job)
}

fn seed_python_project(spec: &DomainSpec, root: &Path) -> Result<(), String> {
    write_file(
        &root.join("ARCHITECTURE.md"),
        &format!(
            "# {} Architecture\n\nThis is an existing Python stdlib queue-operations package. It already exposes item-create, item-complete, and queue-summary CLI contracts over a JSONL store, with tests that must continue to pass. Level 12 evaluates whether a coding workflow can safely evolve an existing multi-module codebase instead of creating a detached greenfield slice.\n",
            spec.architecture_name
        ),
    )?;
    write_file(
        &root.join("PROJECT_MANIFEST.txt"),
        &format!(
            "python-stdlib unittest {} existing-codebase live-level12 integration-preservation memory timing\n",
            spec.id
        ),
    )?;
    write_file(
        &root.join("fixtures/existing_items.jsonl"),
        &format!(
            "{{\"schema_version\":1,\"item_id\":\"{}\",\"title\":\"Critical existing item\",\"owner\":\"{}\",\"severity\":\"critical\",\"status\":\"open\",\"created_at\":\"2026-01-01T00:00:00Z\",\"updated_at\":\"2026-01-01T00:00:00Z\",\"completed_at\":\"\",\"external_ref\":\"seed-critical\",\"source\":\"seed\",\"history\":[{{\"event\":\"created\",\"at\":\"2026-01-01T00:00:00Z\",\"note\":\"seed\"}}]}}\n{{\"schema_version\":1,\"item_id\":\"{}\",\"title\":\"Normal existing item\",\"owner\":\"{}\",\"severity\":\"normal\",\"status\":\"completed\",\"created_at\":\"2026-01-01T00:10:00Z\",\"updated_at\":\"2026-01-01T00:45:00Z\",\"completed_at\":\"2026-01-01T00:45:00Z\",\"external_ref\":\"seed-normal\",\"source\":\"seed\",\"history\":[{{\"event\":\"created\",\"at\":\"2026-01-01T00:10:00Z\",\"note\":\"seed\"}},{{\"event\":\"completed\",\"at\":\"2026-01-01T00:45:00Z\",\"note\":\"seed\"}}]}}\n",
            spec.critical_item_id,
            spec.default_owner,
            spec.normal_item_id,
            spec.default_owner
        ),
    )?;
    write_file(
        &root.join(format!("src/{}/__init__.py", spec.package)),
        &format!(
            "\"\"\"{} live Level 12 existing-codebase probe package.\"\"\"\n",
            spec.architecture_name
        ),
    )?;
    write_file(
        &root.join(format!("src/{}/models.py", spec.package)),
        r#"VALID_SEVERITIES = {"low", "normal", "high", "critical"}
VALID_STATUSES = {"open", "completed"}


def normalize_item(raw):
    severity = raw.get("severity", "normal")
    if severity not in VALID_SEVERITIES:
        raise ValueError(f"invalid severity: {severity}")
    status = raw.get("status", "open")
    if status not in VALID_STATUSES:
        raise ValueError(f"invalid status: {status}")
    item_id = raw["item_id"]
    created_at = raw["created_at"]
    updated_at = raw.get("updated_at") or created_at
    history = list(raw.get("history") or [])
    return {
        "schema_version": 1,
        "item_id": item_id,
        "title": raw.get("title", ""),
        "owner": raw.get("owner", "unassigned"),
        "severity": severity,
        "status": status,
        "created_at": created_at,
        "updated_at": updated_at,
        "completed_at": raw.get("completed_at", ""),
        "external_ref": raw.get("external_ref", ""),
        "source": raw.get("source", "manual"),
        "history": history,
    }


def append_history(item, event, at, note=""):
    next_item = dict(item)
    history = list(next_item.get("history") or [])
    history.append({"event": event, "at": at, "note": note})
    next_item["history"] = history
    next_item["updated_at"] = at
    return next_item
"#,
    )?;
    write_file(
        &root.join(format!("src/{}/repository.py", spec.package)),
        &format!(
            r#"import json
import os
import tempfile
from pathlib import Path

from {package}.models import normalize_item


class WorkItemStore:
    def __init__(self, path):
        self.path = Path(path)

    def load_all(self, include_malformed=False):
        if not self.path.exists():
            return [] if not include_malformed else ([], [])
        records = []
        malformed = []
        with self.path.open(encoding="utf-8") as handle:
            for number, line in enumerate(handle, start=1):
                if not line.strip():
                    continue
                try:
                    records.append(normalize_item(json.loads(line)))
                except (json.JSONDecodeError, KeyError, TypeError, ValueError) as exc:
                    malformed.append({{"line": number, "content": line.rstrip(), "error": str(exc)}})
        return records if not include_malformed else (records, malformed)

    def replace_all(self, items):
        self.path.parent.mkdir(parents=True, exist_ok=True)
        fd, tmp_name = tempfile.mkstemp(prefix=f"{{self.path.name}}.", suffix=".tmp", dir=self.path.parent)
        with os.fdopen(fd, "w", encoding="utf-8") as handle:
            for item in sorted([normalize_item(item) for item in items], key=lambda row: row["item_id"]):
                handle.write(json.dumps(item, sort_keys=True) + "\n")
        os.replace(tmp_name, self.path)

    def get(self, item_id):
        for item in self.load_all():
            if item["item_id"] == item_id:
                return item
        return None

    def upsert(self, item):
        item = normalize_item(item)
        items = [existing for existing in self.load_all() if existing["item_id"] != item["item_id"]]
        items.append(item)
        self.replace_all(items)
        return item
"#,
            package = spec.package
        ),
    )?;
    write_file(
        &root.join(format!("src/{}/policy.py", spec.package)),
        r#"def severity_rank(severity):
    return {"low": 1, "normal": 2, "high": 3, "critical": 4}.get(severity, 0)


def summarize_items(items):
    by_status = {}
    by_owner = {}
    for item in items:
        by_status[item["status"]] = by_status.get(item["status"], 0) + 1
        by_owner[item["owner"]] = by_owner.get(item["owner"], 0) + 1
    return {
        "total_items": len(items),
        "open_items": by_status.get("open", 0),
        "completed_items": by_status.get("completed", 0),
        "by_status": by_status,
        "by_owner": by_owner,
    }
"#,
    )?;
    write_file(
        &root.join(format!("src/{}/service.py", spec.package)),
        &format!(
            r#"from {package}.models import append_history, normalize_item
from {package}.policy import summarize_items
from {package}.repository import WorkItemStore


class WorkQueueService:
    def __init__(self, store: WorkItemStore):
        self.store = store

    def create_item(self, item_id, title, owner, severity, created_at, external_ref="", source="manual"):
        if self.store.get(item_id):
            raise ValueError(f"item already exists: {{item_id}}")
        item = normalize_item({{
            "item_id": item_id,
            "title": title,
            "owner": owner,
            "severity": severity,
            "status": "open",
            "created_at": created_at,
            "updated_at": created_at,
            "external_ref": external_ref,
            "source": source,
            "history": [],
        }})
        item = append_history(item, "created", created_at, source)
        return self.store.upsert(item)

    def complete_item(self, item_id, completed_at):
        item = self.store.get(item_id)
        if not item:
            raise ValueError(f"missing item: {{item_id}}")
        item = dict(item)
        item["status"] = "completed"
        item["completed_at"] = completed_at
        item = append_history(item, "completed", completed_at, "completed via CLI")
        return self.store.upsert(item)

    def summary(self):
        return summarize_items(self.store.load_all())
"#,
            package = spec.package
        ),
    )?;
    write_file(
        &root.join(format!("src/{}/cli.py", spec.package)),
        &format!(
            r#"import argparse
import json
import sys

from {package}.repository import WorkItemStore
from {package}.service import WorkQueueService


def _write_json(stdout, payload):
    stdout.write(json.dumps(payload, sort_keys=True) + "\n")


def build_parser():
    parser = argparse.ArgumentParser(prog="{package}")
    sub = parser.add_subparsers(dest="command", required=True)

    create = sub.add_parser("item-create")
    create.add_argument("store")
    create.add_argument("item_id")
    create.add_argument("--title", required=True)
    create.add_argument("--owner", required=True)
    create.add_argument("--severity", required=True, choices=["low", "normal", "high", "critical"])
    create.add_argument("--created-at", required=True)
    create.add_argument("--external-ref", default="")
    create.add_argument("--source", default="manual")

    complete = sub.add_parser("item-complete")
    complete.add_argument("store")
    complete.add_argument("item_id")
    complete.add_argument("--completed-at", required=True)

    summary = sub.add_parser("queue-summary")
    summary.add_argument("store")

    return parser


def main(argv=None, stdout=None, stderr=None):
    stdout = stdout or sys.stdout
    stderr = stderr or sys.stderr
    parser = build_parser()
    args = parser.parse_args(argv)
    try:
        store = WorkItemStore(getattr(args, "store", ""))
        service = WorkQueueService(store)
        if args.command == "item-create":
            _write_json(stdout, service.create_item(
                args.item_id,
                args.title,
                args.owner,
                args.severity,
                args.created_at,
                args.external_ref,
                args.source,
            ))
            return 0
        if args.command == "item-complete":
            _write_json(stdout, service.complete_item(args.item_id, args.completed_at))
            return 0
        if args.command == "queue-summary":
            _write_json(stdout, service.summary())
            return 0
    except ValueError as exc:
        _write_json(stdout, {{"error": {{"type": "validation_error", "message": str(exc)}}}})
        return 1
    stderr.write("unknown command\n")
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
"#,
            package = spec.package
        ),
    )?;
    let baseline_test = r#"import io
import json
import shutil
import tempfile
import unittest
from pathlib import Path

from __PACKAGE__.cli import main
from __PACKAGE__.repository import WorkItemStore
from __PACKAGE__.service import WorkQueueService


class ExistingQueueBehaviorTest(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp())
        self.store_path = self.tmp / "items.jsonl"

    def tearDown(self):
        shutil.rmtree(self.tmp)

    def test_item_create_and_summary_cli_contracts(self):
        output = io.StringIO()
        exit_code = main([
            "item-create",
            str(self.store_path),
            "__CRITICAL_ITEM__",
            "--title",
            "Critical baseline",
            "--owner",
            "__OWNER__",
            "--severity",
            "critical",
            "--created-at",
            "2026-01-01T00:00:00Z",
        ], stdout=output)
        self.assertEqual(exit_code, 0)
        created = json.loads(output.getvalue())
        self.assertEqual(created["status"], "open")
        self.assertEqual(created["history"][0]["event"], "created")

        summary_output = io.StringIO()
        self.assertEqual(main(["queue-summary", str(self.store_path)], stdout=summary_output), 0)
        summary = json.loads(summary_output.getvalue())
        self.assertEqual(summary["total_items"], 1)
        self.assertEqual(summary["open_items"], 1)

    def test_item_complete_preserves_existing_contract(self):
        service = WorkQueueService(WorkItemStore(self.store_path))
        service.create_item(
            "__NORMAL_ITEM__",
            "Normal baseline",
            "__OWNER__",
            "normal",
            "2026-01-01T00:10:00Z",
        )
        output = io.StringIO()
        exit_code = main([
            "item-complete",
            str(self.store_path),
            "__NORMAL_ITEM__",
            "--completed-at",
            "2026-01-01T01:00:00Z",
        ], stdout=output)
        self.assertEqual(exit_code, 0)
        completed = json.loads(output.getvalue())
        self.assertEqual(completed["status"], "completed")
        self.assertEqual(completed["completed_at"], "2026-01-01T01:00:00Z")

    def test_existing_fixture_summary_remains_readable(self):
        fixture = Path("fixtures/existing_items.jsonl")
        summary = WorkQueueService(WorkItemStore(fixture)).summary()
        self.assertEqual(summary["total_items"], 2)
        self.assertEqual(summary["open_items"], 1)
        self.assertEqual(summary["completed_items"], 1)


if __name__ == "__main__":
    unittest.main()
"#
    .replace("__PACKAGE__", spec.package)
    .replace("__OWNER__", spec.default_owner)
    .replace("__CRITICAL_ITEM__", spec.critical_item_id)
    .replace("__NORMAL_ITEM__", spec.normal_item_id);
    write_file(
        &root.join("tests/test_existing_queue_behavior.py"),
        &baseline_test,
    )?;
    Ok(())
}

fn worker_prompt(job: &LiveLevel12Job) -> String {
    format!(
        "You are running a live Level 12 existing-codebase coding workflow probe. You are not alone in the broader codebase: do not revert or modify anything outside the assigned temp run directory. Your write ownership is limited to {project_root} and {receipts_root}.\n\nGoal: use the higher-level coding workflow plus the primitive coding workflow to evolve an existing local Python project safely. This is not greenfield. Current files are authoritative; stored memory is only resume context. Do not ask follow-up questions unless the project is genuinely blocked by a user-owned product or architecture decision.\n\nEnvironment:\n- Project root: {project_root}\n- Python package: {package}\n- Receipts root: {receipts_root}\n- Isolated memory DB: {memory_db_path}\n- Resume token: {resume_token}\n- Prior memory row id: {prior_memory_row_id}\n- Expected checkpoint 008 memory row id: {checkpoint8_memory_row_id}\n- Expected checkpoint 009 memory row id: {checkpoint9_memory_row_id}\n- Memory CLI command pattern: INFRING_MEMORY_DB_PATH={memory_db_path} cargo run --quiet --manifest-path /Users/jay/.openclaw/workspace/core/layer0/memory/Cargo.toml --bin memory-cli -- <command>\n- Validation command from project root: {validation_command}\n\nWorkflow requirements:\n1. Read the local project files first. Current files override memory.\n2. Retrieve checkpoint memory using the resume token and/or prior row id.\n3. Write {receipts_root}/project_operator_roadmap.json before implementation. It must include product_goal, architecture_intent, stack_and_runtime, existing_contracts_to_preserve, milestones, checkpoint_queue, non_goals, risk_register, validation_strategy, and stop_conditions.\n4. Implement checkpoint_008_existing_codebase_safe_feature_and_bugfix as a coherent milestone. Evolve the existing modules in place rather than replacing the app with an unrelated implementation. Preserve item-create, item-complete, and queue-summary behavior. Add idempotent external bulk import, hold-aware SLA reporting, structured validation errors, and regression tests.\n5. Implement this exact new CLI contract in addition to the existing commands:\n   - bulk-import <store> <csv_path> --source <source>\n   - item-hold <store> <item_id> --reason <reason> --until <iso8601> --created-at <iso8601>\n   - sla-report <store> --as-of <iso8601> --critical-minutes <n> --high-minutes <n> --normal-minutes <n> --low-minutes <n> [--include-held]\n   New command output must be JSON. bulk-import must be idempotent by external_ref when present, then item_id as fallback. item-hold must preserve task history and exclude held open items from breached_item_ids until the hold expires. sla-report must use the explicit --as-of value, not wall clock time.\n6. Run validation after checkpoint 008. Write {receipts_root}/checkpoint_008_handoff.json with completed_checkpoint, validation_summary, changed_files, architecture_decisions, risk_register_updates, memory_written, and next_checkpoint_decision.\n7. Write memory row {checkpoint8_memory_row_id} with tags coding,checkpoint,resume,project_context. Include changed files, validation result, known risks, and recommended next checkpoint.\n8. Continue to checkpoint_009 only if validation passed, risk is bounded, and the roadmap still applies. If you stop, the stop reason must be specific and valid.\n9. Implement checkpoint_009_contract_preserving_export_import_and_diff. Add rollback-safe state-export, state-import, and state-diff CLI coverage. Export/import must preserve item history, holds, external references, existing summary behavior, and deterministic ordering. Add regression tests proving old commands still pass after import.\n10. Implement this exact additional CLI contract:\n   - state-export <store> <snapshot_path>\n   - state-import <snapshot_path> <store> [--dry-run]\n   - state-diff <snapshot_path_a> <snapshot_path_b>\n   state-diff of identical snapshots must report no changes in JSON.\n11. Run validation after checkpoint 009. Write {receipts_root}/checkpoint_009_handoff.json with the same fields as checkpoint 008 plus continuation decision.\n12. Write memory row {checkpoint9_memory_row_id} with tags coding,checkpoint,resume,project_context.\n13. Final response should include pass/fail, changed file paths, validation command/result, memory row ids written, and caveats. Do not commit anything.\n",
        project_root = job.project_root,
        package = job.package,
        receipts_root = job.receipts_root,
        memory_db_path = job.memory_db_path,
        resume_token = job.resume_token,
        prior_memory_row_id = job.prior_memory_row_id,
        checkpoint8_memory_row_id = job.expected_checkpoint8_memory_row_id,
        checkpoint9_memory_row_id = job.expected_checkpoint9_memory_row_id,
        validation_command = job.validation_command
    )
}

fn judge_live_attempt(
    job: &LiveLevel12Job,
    batch_started_at_unix_ms: Option<u128>,
) -> LiveLevel12AttemptJudge {
    let mut checks = Vec::new();
    let mut failures = Vec::new();
    let project_root = PathBuf::from(&job.project_root);
    let receipts_root = PathBuf::from(&job.receipts_root);
    let roadmap_path = receipts_root.join("project_operator_roadmap.json");
    let checkpoint8_path = receipts_root.join("checkpoint_008_handoff.json");
    let checkpoint9_path = receipts_root.join("checkpoint_009_handoff.json");
    let timing = attempt_timing(
        &[
            roadmap_path.as_path(),
            checkpoint8_path.as_path(),
            checkpoint9_path.as_path(),
        ],
        batch_started_at_unix_ms,
    );

    let validation = run_python_validation(&project_root);
    push_check(
        &mut checks,
        &mut failures,
        "validation_passes_after_live_worker",
        validation.ok,
        validation.detail,
    );

    let roadmap = read_json_file(&roadmap_path);
    push_check(
        &mut checks,
        &mut failures,
        "project_operator_roadmap_written",
        roadmap.is_some(),
        roadmap_path.display().to_string(),
    );
    if let Some(roadmap) = &roadmap {
        for field in [
            "product_goal",
            "architecture_intent",
            "stack_and_runtime",
            "existing_contracts_to_preserve",
            "milestones",
            "checkpoint_queue",
            "non_goals",
            "risk_register",
            "validation_strategy",
            "stop_conditions",
        ] {
            push_check(
                &mut checks,
                &mut failures,
                "roadmap_required_field_present",
                roadmap.get(field).is_some(),
                field.to_string(),
            );
        }
        let queue_count = roadmap
            .get("checkpoint_queue")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or_default();
        push_check(
            &mut checks,
            &mut failures,
            "roadmap_declares_multi_checkpoint_queue",
            queue_count >= 2,
            format!("checkpoint_queue_count={queue_count}"),
        );
    }

    judge_checkpoint_receipt(
        &mut checks,
        &mut failures,
        &checkpoint8_path,
        "checkpoint_008_existing_codebase_safe_feature_and_bugfix",
        "checkpoint_008_receipt_written",
        "checkpoint_008_receipt_declares_existing_codebase_feature",
    );
    judge_checkpoint_receipt(
        &mut checks,
        &mut failures,
        &checkpoint9_path,
        "checkpoint_009_contract_preserving_export_import_and_diff",
        "checkpoint_009_receipt_written",
        "checkpoint_009_receipt_declares_export_import_diff",
    );

    let bridge = CodingMemoryRuntimeBridge {
        workspace_root: workspace_root(),
        memory_db_path: PathBuf::from(&job.memory_db_path),
        session_id: job.attempt_id.clone(),
    };
    let checkpoint8_memory = bridge.get(&job.expected_checkpoint8_memory_row_id);
    push_check(
        &mut checks,
        &mut failures,
        "checkpoint_008_memory_written",
        checkpoint8_memory.ok,
        checkpoint8_memory.payload.to_string(),
    );
    let checkpoint9_memory = bridge.get(&job.expected_checkpoint9_memory_row_id);
    push_check(
        &mut checks,
        &mut failures,
        "checkpoint_009_memory_written",
        checkpoint9_memory.ok,
        checkpoint9_memory.payload.to_string(),
    );

    let evidence = collect_project_text(&project_root);
    let lower = evidence.to_lowercase();
    push_check(
        &mut checks,
        &mut failures,
        "baseline_contracts_still_present",
        lower.contains("item-create")
            && lower.contains("item-complete")
            && lower.contains("queue-summary")
            && lower.contains("workqueueservice"),
        "source still contains baseline CLI and service contracts".to_string(),
    );
    push_check(
        &mut checks,
        &mut failures,
        "bulk_import_idempotency_implemented",
        lower.contains("bulk-import")
            && lower.contains("csv")
            && lower.contains("external_ref")
            && (lower.contains("idempot") || lower.contains("unchanged")),
        "source contains bulk import, CSV, external_ref, and idempotency signals".to_string(),
    );
    push_check(
        &mut checks,
        &mut failures,
        "hold_aware_sla_reporting_implemented",
        lower.contains("sla-report")
            && lower.contains("item-hold")
            && lower.contains("as_of")
            && lower.contains("held")
            && lower.contains("breached"),
        "source contains hold-aware explicit-as-of SLA reporting signals".to_string(),
    );
    push_check(
        &mut checks,
        &mut failures,
        "state_export_import_diff_implemented",
        lower.contains("state-export")
            && lower.contains("state-import")
            && lower.contains("state-diff")
            && lower.contains("snapshot"),
        "source contains state export/import/diff snapshot signals".to_string(),
    );
    push_check(
        &mut checks,
        &mut failures,
        "regression_tests_cover_existing_and_new_behavior",
        lower.matches("unittest").count() >= 2
            && lower.contains("queue-summary")
            && lower.contains("bulk-import")
            && lower.contains("sla-report")
            && lower.contains("state-diff"),
        "tests mention baseline summary plus bulk import, SLA, and state diff".to_string(),
    );
    let semantic_probe = run_level12_cli_semantic_probe(&project_root, &job.package);
    push_check(
        &mut checks,
        &mut failures,
        "strict_existing_codebase_semantic_probe_passes",
        semantic_probe.ok,
        semantic_probe.detail,
    );

    LiveLevel12AttemptJudge {
        attempt_id: job.attempt_id.clone(),
        ok: failures.is_empty(),
        timing,
        checks,
        failures,
    }
}

fn judge_checkpoint_receipt(
    checks: &mut Vec<LiveLevel12Check>,
    failures: &mut Vec<String>,
    receipt_path: &Path,
    expected_checkpoint: &str,
    written_check_id: &'static str,
    declared_check_id: &'static str,
) {
    let receipt = read_json_file(receipt_path);
    push_check(
        checks,
        failures,
        written_check_id,
        receipt.is_some(),
        receipt_path.display().to_string(),
    );
    if let Some(receipt) = receipt {
        let completed_checkpoint = receipt
            .get("completed_checkpoint")
            .or_else(|| receipt.get("checkpoint"))
            .and_then(Value::as_str)
            .unwrap_or("missing_completed_checkpoint");
        push_check(
            checks,
            failures,
            declared_check_id,
            completed_checkpoint == expected_checkpoint,
            completed_checkpoint.to_string(),
        );
        let changed_file_count = receipt
            .get("changed_files")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or_default();
        push_check(
            checks,
            failures,
            "checkpoint_receipt_declares_multi_file_change",
            changed_file_count >= 2,
            format!(
                "{} changed_file_count={changed_file_count}",
                receipt_path.display()
            ),
        );
        for field in [
            "validation_summary",
            "architecture_decisions",
            "risk_register_updates",
            "memory_written",
            "next_checkpoint_decision",
        ] {
            push_check(
                checks,
                failures,
                "checkpoint_receipt_required_field_present",
                receipt.get(field).is_some(),
                format!("{} {field}", receipt_path.display()),
            );
        }
    }
}

fn push_check(
    checks: &mut Vec<LiveLevel12Check>,
    failures: &mut Vec<String>,
    id: &'static str,
    ok: bool,
    detail: String,
) {
    checks.push(LiveLevel12Check { id, ok, detail });
    if !ok {
        let check = checks.last().expect("just pushed");
        failures.push(format!("{}:{}", check.id, check.detail));
    }
}

#[derive(Debug)]
struct CommandResult {
    ok: bool,
    detail: String,
    stdout: String,
}

fn run_python_validation(project_root: &Path) -> CommandResult {
    let output = Command::new("python3")
        .arg("-m")
        .arg("unittest")
        .arg("discover")
        .arg("-s")
        .arg("tests")
        .env("PYTHONPATH", "src")
        .current_dir(project_root)
        .output();
    match output {
        Ok(output) => CommandResult {
            ok: output.status.success(),
            detail: format!(
                "exit={:?};stdout={};stderr={}",
                output.status.code(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        },
        Err(error) => CommandResult {
            ok: false,
            detail: format!("spawn_failed:{error}"),
            stdout: String::new(),
        },
    }
}

fn run_level12_cli_semantic_probe(project_root: &Path, package: &str) -> CommandResult {
    let probe_root = PathBuf::from(project_root).join(".level12_strict_judge");
    if probe_root.exists() {
        if let Err(error) = fs::remove_dir_all(&probe_root) {
            return CommandResult {
                ok: false,
                detail: format!("clear_probe_root_failed:{}:{error}", probe_root.display()),
                stdout: String::new(),
            };
        }
    }
    if let Err(error) = fs::create_dir_all(&probe_root) {
        return CommandResult {
            ok: false,
            detail: format!("create_probe_root_failed:{}:{error}", probe_root.display()),
            stdout: String::new(),
        };
    }

    let store = probe_root.join("items.jsonl");
    let imported_store = probe_root.join("items_imported.jsonl");
    let csv_path = probe_root.join("bulk.csv");
    let snapshot_a = probe_root.join("snapshot_a.json");
    let snapshot_b = probe_root.join("snapshot_b.json");
    let mut failures = Vec::new();

    let create = run_module_command(
        project_root,
        package,
        vec![
            "item-create",
            store.to_string_lossy().as_ref(),
            "judge-critical",
            "--title",
            "Judge critical",
            "--owner",
            "judge",
            "--severity",
            "critical",
            "--created-at",
            "2026-01-01T00:00:00Z",
            "--external-ref",
            "judge-critical-ref",
        ],
    );
    require_command(&mut failures, "item-create", &create);

    let summary = run_module_command(
        project_root,
        package,
        vec!["queue-summary", store.to_string_lossy().as_ref()],
    );
    let summary_json = require_json_command(&mut failures, "queue-summary", &summary);
    if let Some(summary_json) = summary_json {
        let total = summary_json
            .get("total_items")
            .and_then(Value::as_i64)
            .unwrap_or(-1);
        if total != 1 {
            failures.push(format!("queue-summary_total_expected_1_got_{total}"));
        }
    }

    if let Err(error) = write_file(
        &csv_path,
        "external_ref,item_id,title,owner,severity,created_at,status\nbulk-ext-1,bulk-1,Bulk one,ops,high,2026-01-01T00:10:00Z,open\nbulk-ext-2,bulk-2,Bulk two,ops,critical,2026-01-01T00:20:00Z,open\n",
    ) {
        failures.push(format!("write_bulk_csv_failed:{error}"));
    }

    for label in ["bulk-import-first", "bulk-import-second"] {
        let result = run_module_command(
            project_root,
            package,
            vec![
                "bulk-import",
                store.to_string_lossy().as_ref(),
                csv_path.to_string_lossy().as_ref(),
                "--source",
                "judge",
            ],
        );
        let value = require_json_command(&mut failures, label, &result);
        if label == "bulk-import-second" {
            if let Some(value) = value {
                let created = value.get("created").and_then(Value::as_i64).unwrap_or(0);
                if created > 0 {
                    failures.push(format!(
                        "bulk-import_not_idempotent_created_on_second_run:{created}"
                    ));
                }
            }
        }
    }

    let summary_after_import = run_module_command(
        project_root,
        package,
        vec!["queue-summary", store.to_string_lossy().as_ref()],
    );
    let summary_after_import_json = require_json_command(
        &mut failures,
        "queue-summary-after-import",
        &summary_after_import,
    );
    if let Some(summary_after_import_json) = summary_after_import_json {
        let total = summary_after_import_json
            .get("total_items")
            .and_then(Value::as_i64)
            .unwrap_or(-1);
        if total != 3 {
            failures.push(format!("idempotent_import_total_expected_3_got_{total}"));
        }
    }

    let hold = run_module_command(
        project_root,
        package,
        vec![
            "item-hold",
            store.to_string_lossy().as_ref(),
            "bulk-2",
            "--reason",
            "waiting for vendor",
            "--until",
            "2026-01-01T03:00:00Z",
            "--created-at",
            "2026-01-01T00:30:00Z",
        ],
    );
    require_command(&mut failures, "item-hold", &hold);

    let early_sla = run_module_command(
        project_root,
        package,
        vec![
            "sla-report",
            store.to_string_lossy().as_ref(),
            "--as-of",
            "2026-01-01T02:00:00Z",
            "--critical-minutes",
            "20",
            "--high-minutes",
            "20",
            "--normal-minutes",
            "60",
            "--low-minutes",
            "120",
            "--include-held",
        ],
    );
    let early_sla_json = require_json_command(&mut failures, "sla-report-held-window", &early_sla);
    if let Some(value) = early_sla_json {
        let held = value.get("held_count").and_then(Value::as_i64).unwrap_or(0);
        if held < 1 {
            failures.push(format!(
                "sla-report_expected_held_count_at_least_1_got_{held}"
            ));
        }
        if json_array_contains_string(value.get("breached_item_ids"), "bulk-2") {
            failures.push("sla-report_included_held_bulk_2_before_hold_expiry".to_string());
        }
        if !json_array_contains_string(value.get("breached_item_ids"), "bulk-1") {
            failures.push("sla-report_missing_unheld_bulk_1_breach".to_string());
        }
    }

    let late_sla = run_module_command(
        project_root,
        package,
        vec![
            "sla-report",
            store.to_string_lossy().as_ref(),
            "--as-of",
            "2026-01-01T04:00:00Z",
            "--critical-minutes",
            "20",
            "--high-minutes",
            "20",
            "--normal-minutes",
            "60",
            "--low-minutes",
            "120",
        ],
    );
    let late_sla_json = require_json_command(&mut failures, "sla-report-after-hold", &late_sla);
    if let Some(value) = late_sla_json {
        if !json_array_contains_string(value.get("breached_item_ids"), "bulk-2") {
            failures.push("sla-report_missing_bulk_2_after_hold_expiry".to_string());
        }
    }

    let export_a = run_module_command(
        project_root,
        package,
        vec![
            "state-export",
            store.to_string_lossy().as_ref(),
            snapshot_a.to_string_lossy().as_ref(),
        ],
    );
    require_command(&mut failures, "state-export-a", &export_a);

    let import = run_module_command(
        project_root,
        package,
        vec![
            "state-import",
            snapshot_a.to_string_lossy().as_ref(),
            imported_store.to_string_lossy().as_ref(),
        ],
    );
    require_command(&mut failures, "state-import", &import);

    let imported_summary = run_module_command(
        project_root,
        package,
        vec!["queue-summary", imported_store.to_string_lossy().as_ref()],
    );
    let imported_summary_json = require_json_command(
        &mut failures,
        "queue-summary-after-state-import",
        &imported_summary,
    );
    if let Some(imported_summary_json) = imported_summary_json {
        let total = imported_summary_json
            .get("total_items")
            .and_then(Value::as_i64)
            .unwrap_or(-1);
        if total != 3 {
            failures.push(format!("state_import_total_expected_3_got_{total}"));
        }
    }

    let export_b = run_module_command(
        project_root,
        package,
        vec![
            "state-export",
            imported_store.to_string_lossy().as_ref(),
            snapshot_b.to_string_lossy().as_ref(),
        ],
    );
    require_command(&mut failures, "state-export-b", &export_b);

    let diff = run_module_command(
        project_root,
        package,
        vec![
            "state-diff",
            snapshot_a.to_string_lossy().as_ref(),
            snapshot_a.to_string_lossy().as_ref(),
        ],
    );
    let diff_json = require_json_command(&mut failures, "state-diff-identical", &diff);
    if let Some(value) = diff_json {
        let changed = value
            .get("changed_count")
            .or_else(|| value.get("changes"))
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let equal = value
            .get("equal")
            .and_then(Value::as_bool)
            .unwrap_or(changed == 0);
        if !equal || changed != 0 {
            failures.push(format!(
                "state-diff_identical_expected_equal_changed_0:{value}"
            ));
        }
    }

    if failures.is_empty() {
        CommandResult {
            ok: true,
            detail: "level12_semantic_probe_passed".to_string(),
            stdout: String::new(),
        }
    } else {
        CommandResult {
            ok: false,
            detail: failures.join(";"),
            stdout: String::new(),
        }
    }
}

fn require_command(failures: &mut Vec<String>, label: &str, result: &CommandResult) {
    if !result.ok {
        failures.push(format!("{label}_failed:{}", result.detail));
    }
}

fn require_json_command(
    failures: &mut Vec<String>,
    label: &str,
    result: &CommandResult,
) -> Option<Value> {
    require_command(failures, label, result);
    match serde_json::from_str::<Value>(&result.stdout) {
        Ok(value) => Some(value),
        Err(error) => {
            failures.push(format!(
                "{label}_stdout_not_json:{error}:stdout={}",
                result.stdout
            ));
            None
        }
    }
}

fn json_array_contains_string(value: Option<&Value>, needle: &str) -> bool {
    value
        .and_then(Value::as_array)
        .map(|items| items.iter().any(|item| item.as_str() == Some(needle)))
        .unwrap_or(false)
}

fn run_module_command(project_root: &Path, package: &str, args: Vec<&str>) -> CommandResult {
    let output = Command::new("python3")
        .arg("-m")
        .arg(format!("{package}.cli"))
        .args(args)
        .env("PYTHONPATH", "src")
        .current_dir(project_root)
        .output();
    match output {
        Ok(output) => CommandResult {
            ok: output.status.success(),
            detail: format!(
                "exit={:?};stdout={};stderr={}",
                output.status.code(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ),
            stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
        },
        Err(error) => CommandResult {
            ok: false,
            detail: format!("spawn_failed:{error}"),
            stdout: String::new(),
        },
    }
}

fn write_file(path: &Path, content: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create_parent_failed:{}:{error}", parent.display()))?;
    }
    fs::write(path, content).map_err(|error| format!("write_failed:{}:{error}", path.display()))
}

fn read_to_string(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

fn read_json_file(path: &Path) -> Option<Value> {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
}

fn collect_project_text(project_root: &Path) -> String {
    let mut text = String::new();
    collect_text_files(project_root, &mut text);
    text
}

fn collect_text_files(path: &Path, out: &mut String) {
    let Ok(metadata) = fs::metadata(path) else {
        return;
    };
    if metadata.is_dir() {
        let Ok(entries) = fs::read_dir(path) else {
            return;
        };
        for entry in entries.flatten() {
            let child = entry.path();
            if child
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| {
                    name == "__pycache__"
                        || name == ".level12_strict_judge"
                        || name.ends_with(".sqlite")
                })
                .unwrap_or(false)
            {
                continue;
            }
            collect_text_files(&child, out);
        }
        return;
    }
    let Some(extension) = path.extension().and_then(|extension| extension.to_str()) else {
        return;
    };
    if !["py", "md", "txt", "json", "jsonl", "csv"].contains(&extension) {
        return;
    }
    if let Ok(raw) = fs::read_to_string(path) {
        out.push_str("\n--- ");
        out.push_str(&path.display().to_string());
        out.push_str(" ---\n");
        out.push_str(&raw);
    }
}

fn file_modified_unix_ms(path: &Path) -> Option<u128> {
    fs::metadata(path)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis())
}

fn attempt_timing(
    paths: &[&Path],
    batch_started_at_unix_ms: Option<u128>,
) -> LiveLevel12AttemptTiming {
    let mut receipt_times = paths
        .iter()
        .filter_map(|path| file_modified_unix_ms(path))
        .collect::<Vec<_>>();
    receipt_times.sort_unstable();
    let first_receipt_unix_ms = receipt_times.first().copied();
    let completed_at_unix_ms = receipt_times.last().copied();
    let elapsed_ms_since_batch_start = batch_started_at_unix_ms
        .zip(completed_at_unix_ms)
        .and_then(|(started, completed)| completed.checked_sub(started));
    LiveLevel12AttemptTiming {
        first_receipt_unix_ms,
        completed_at_unix_ms,
        elapsed_ms_since_batch_start,
    }
}

fn summarize_level12_timing(
    batch_started_at_unix_ms: Option<u128>,
    judged_at_unix_ms: u128,
    attempts: &[LiveLevel12AttemptJudge],
) -> LiveLevel12TimingSummary {
    let mut completed_times = attempts
        .iter()
        .filter_map(|attempt| attempt.timing.completed_at_unix_ms)
        .collect::<Vec<_>>();
    completed_times.sort_unstable();
    let first_attempt_completed_at_unix_ms = completed_times.first().copied();
    let last_attempt_completed_at_unix_ms = completed_times.last().copied();
    let completion_span_ms = first_attempt_completed_at_unix_ms
        .zip(last_attempt_completed_at_unix_ms)
        .and_then(|(first, last)| last.checked_sub(first));
    let batch_elapsed_ms =
        batch_started_at_unix_ms.and_then(|started| judged_at_unix_ms.checked_sub(started));
    let elapsed_attempts = attempts
        .iter()
        .filter_map(|attempt| attempt.timing.elapsed_ms_since_batch_start)
        .collect::<Vec<_>>();
    let average_attempt_elapsed_ms = if elapsed_attempts.is_empty() {
        None
    } else {
        Some(elapsed_attempts.iter().sum::<u128>() / elapsed_attempts.len() as u128)
    };
    LiveLevel12TimingSummary {
        batch_started_at_unix_ms,
        judged_at_unix_ms,
        batch_elapsed_ms,
        first_attempt_completed_at_unix_ms,
        last_attempt_completed_at_unix_ms,
        completion_span_ms,
        average_attempt_elapsed_ms,
    }
}
