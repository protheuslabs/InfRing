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
pub struct LiveLevel10SeedBatchReport {
    pub harness_kind: String,
    pub ok: bool,
    pub batch_root: String,
    pub seed_started_at_unix_ms: Option<u128>,
    pub attempt_count: usize,
    pub jobs: Vec<LiveLevel10Job>,
    pub failures: Vec<String>,
    pub operator_next_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveLevel10Job {
    pub attempt_id: String,
    pub package: String,
    pub run_root: String,
    pub project_root: String,
    pub receipts_root: String,
    pub prompt_path: String,
    pub memory_db_path: String,
    pub resume_token: String,
    pub prior_memory_row_id: String,
    pub expected_checkpoint4_memory_row_id: String,
    pub expected_checkpoint5_memory_row_id: String,
    pub project_fingerprint: String,
    pub architecture_hash: String,
    pub validation_command: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LiveLevel10JudgeReport {
    pub harness_kind: &'static str,
    pub ok: bool,
    pub batch_root: String,
    pub attempt_count: usize,
    pub pass_count: usize,
    pub fail_count: usize,
    pub timing: LiveLevel10TimingSummary,
    pub attempts: Vec<LiveLevel10AttemptJudge>,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LiveLevel10AttemptJudge {
    pub attempt_id: String,
    pub ok: bool,
    pub timing: LiveLevel10AttemptTiming,
    pub checks: Vec<LiveLevel10Check>,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LiveLevel10TimingSummary {
    pub batch_started_at_unix_ms: Option<u128>,
    pub judged_at_unix_ms: u128,
    pub batch_elapsed_ms: Option<u128>,
    pub first_attempt_completed_at_unix_ms: Option<u128>,
    pub last_attempt_completed_at_unix_ms: Option<u128>,
    pub completion_span_ms: Option<u128>,
    pub average_attempt_elapsed_ms: Option<u128>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LiveLevel10AttemptTiming {
    pub first_receipt_unix_ms: Option<u128>,
    pub completed_at_unix_ms: Option<u128>,
    pub elapsed_ms_since_batch_start: Option<u128>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LiveLevel10Check {
    pub id: &'static str,
    pub ok: bool,
    pub detail: String,
}

#[derive(Debug, Clone)]
struct DomainSpec {
    id: &'static str,
    package: &'static str,
    architecture_name: &'static str,
    primary_kind: &'static str,
    primary_destination: &'static str,
    secondary_prefix: &'static str,
    secondary_destination: &'static str,
}

const DOMAIN_SPECS: &[DomainSpec] = &[
    DomainSpec {
        id: "clinic_operator_ops",
        package: "clinic_operator_ops",
        architecture_name: "Clinic Operator Ops",
        primary_kind: "clinic.visit.completed",
        primary_destination: "billing",
        secondary_prefix: "patient",
        secondary_destination: "care-team",
    },
    DomainSpec {
        id: "warehouse_operator_ops",
        package: "warehouse_operator_ops",
        architecture_name: "Warehouse Operator Ops",
        primary_kind: "warehouse.pick.completed",
        primary_destination: "fulfillment",
        secondary_prefix: "inventory",
        secondary_destination: "stock-control",
    },
    DomainSpec {
        id: "incident_operator_ops",
        package: "incident_operator_ops",
        architecture_name: "Incident Operator Ops",
        primary_kind: "incident.restored",
        primary_destination: "postmortem",
        secondary_prefix: "oncall",
        secondary_destination: "incident-command",
    },
];

fn spec_for_package(package: &str) -> Option<&'static DomainSpec> {
    DOMAIN_SPECS.iter().find(|spec| spec.package == package)
}

pub fn seed_live_level10_batch(attempt_count: usize) -> LiveLevel10SeedBatchReport {
    let count = attempt_count.max(1);
    let seed_started_at_unix_ms = millis_now();
    let batch_root = std::env::temp_dir().join(format!(
        "coding-memory-live-level10-batch-{}-{}",
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

    let report = LiveLevel10SeedBatchReport {
        harness_kind: "coding_memory_live_level10_seed_v1".to_string(),
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

pub fn judge_live_level10_batch(batch_root: &Path) -> LiveLevel10JudgeReport {
    let mut failures = Vec::new();
    let jobs_path = batch_root.join("jobs.json");
    let seed_report = fs::read_to_string(&jobs_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<LiveLevel10SeedBatchReport>(&raw).ok());
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
    let timing = summarize_level10_timing(batch_started_at_unix_ms, judged_at_unix_ms, &attempts);
    LiveLevel10JudgeReport {
        harness_kind: "coding_memory_live_level10_judge_v1",
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
) -> Result<LiveLevel10Job, String> {
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
    let resume_token = format!("live_level10_resume_{}_{}", attempt_id, millis_now());
    let prior_memory_row_id = format!(
        "coding_memory::{}::checkpoint::checkpoint_003",
        snapshot.project_fingerprint
    );
    let expected_checkpoint4_memory_row_id = format!(
        "coding_memory::{}::checkpoint::checkpoint_004",
        snapshot.project_fingerprint
    );
    let expected_checkpoint5_memory_row_id = format!(
        "coding_memory::{}::checkpoint::checkpoint_005",
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
        "completed_checkpoint": "checkpoint_003_delivery_schema_migration",
        "changed_files": [
            "ARCHITECTURE.md",
            "PROJECT_MANIFEST.txt",
            &format!("src/{}/models.py", spec.package),
            &format!("src/{}/router.py", spec.package),
            &format!("src/{}/repository.py", spec.package),
            &format!("src/{}/service.py", spec.package),
            &format!("src/{}/cli.py", spec.package),
            "tests/test_delivery_baseline.py"
        ],
        "validation_results": {
            "status": "pass",
            "command": validation_command,
            "exit_code": 0
        },
        "recommended_next_checkpoint": "checkpoint_004_operator_audit_workbench",
        "next_slice_goal": "Promote this project from delivery-attempt persistence into an operator workbench with durable audit/task records, then continue to SLO escalation and snapshot export only if validation and risk gates are clean.",
        "constraints": [
            "read current files before planning",
            "current workspace files remain authoritative over memory",
            "use Python stdlib only",
            "preserve route, attempt-record, attempts-report, and migrate-attempts CLI behavior",
            "write a roadmap before continuing beyond the first checkpoint",
            "stop rather than continue if architecture, persistence, or product decisions become ambiguous"
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
    let job = LiveLevel10Job {
        attempt_id: attempt_id.clone(),
        package: spec.package.to_string(),
        run_root: run_root.display().to_string(),
        project_root: project_root.display().to_string(),
        receipts_root: receipts_root.display().to_string(),
        prompt_path: prompt_path.display().to_string(),
        memory_db_path: memory_db_path.display().to_string(),
        resume_token,
        prior_memory_row_id,
        expected_checkpoint4_memory_row_id,
        expected_checkpoint5_memory_row_id,
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
            "# {} Architecture\n\nThis existing project is a Python stdlib routing and delivery-operations package. It already has schema-versioned delivery attempts, v1-to-v2 JSONL migration, route/report CLI behavior, and regression tests. Level 10 is not a single slice: the next work should run in long-run project-operator mode with a roadmap, checkpoint queue, risk register, milestone receipts, durable memory writes, and safe continuation between checkpoints.\n",
            spec.architecture_name
        ),
    )?;
    write_file(
        &root.join("PROJECT_MANIFEST.txt"),
        &format!(
            "python-stdlib unittest {} existing-project live-level10 long-run operator checkpoints memory\n",
            spec.id
        ),
    )?;
    write_file(
        &root.join("fixtures/delivery_attempts_v1.jsonl"),
        &format!(
            "{{\"attempt_id\":\"old-1\",\"event_id\":\"evt-old-1\",\"destination\":\"{}\",\"status\":\"delivered\",\"error_code\":\"\",\"created_at\":\"2026-01-01T00:00:00Z\"}}\n{{\"attempt_id\":\"old-2\",\"event_id\":\"evt-old-2\",\"destination\":\"default\",\"status\":\"timeout\",\"error_code\":\"timeout\",\"created_at\":\"2026-01-01T00:01:00Z\"}}\nmalformed-line\n",
            spec.primary_destination
        ),
    )?;
    write_file(
        &root.join(format!("src/{}/__init__.py", spec.package)),
        &format!(
            "\"\"\"{} live Level 10 probe package.\"\"\"\n",
            spec.architecture_name
        ),
    )?;
    write_file(
        &root.join(format!("src/{}/models.py", spec.package)),
        r#"from dataclasses import dataclass
from typing import Mapping


@dataclass(frozen=True)
class Event:
    event_id: str
    kind: str
    payload: Mapping[str, str]


@dataclass(frozen=True)
class RouteDecision:
    event_id: str
    destination: str
    reason: str
"#,
    )?;
    write_file(
        &root.join(format!("src/{}/router.py", spec.package)),
        &format!(
            r#"from {package}.models import Event, RouteDecision


def route_event(event: Event) -> RouteDecision:
    if event.kind == "{primary_kind}":
        return RouteDecision(event.event_id, "{primary_destination}", "primary-kind")
    if event.kind.startswith("{secondary_prefix}."):
        return RouteDecision(event.event_id, "{secondary_destination}", "secondary-family")
    return RouteDecision(event.event_id, "default", "fallback")
"#,
            package = spec.package,
            primary_kind = spec.primary_kind,
            primary_destination = spec.primary_destination,
            secondary_prefix = spec.secondary_prefix,
            secondary_destination = spec.secondary_destination
        ),
    )?;
    write_file(
        &root.join(format!("src/{}/repository.py", spec.package)),
        r#"import json
import os
import tempfile
from pathlib import Path


RETRYABLE_STATUSES = {"timeout", "temporary_failure", "rate_limited"}


def normalize_attempt(raw):
    event_id = raw["event_id"]
    attempt_id = raw.get("attempt_id") or f"{event_id}:1"
    return {
        "schema_version": 2,
        "attempt_id": attempt_id,
        "event_id": event_id,
        "destination": raw["destination"],
        "status": raw.get("status", raw.get("outcome", "unknown")),
        "error_code": raw.get("error_code", ""),
        "retryable": raw.get("status") in RETRYABLE_STATUSES or raw.get("error_code") in RETRYABLE_STATUSES,
        "created_at": raw["created_at"],
    }


class DeliveryAttemptStore:
    def __init__(self, path):
        self.path = Path(path)

    def append(self, attempt):
        self.path.parent.mkdir(parents=True, exist_ok=True)
        with self.path.open("a", encoding="utf-8") as handle:
            handle.write(json.dumps(normalize_attempt(attempt), sort_keys=True) + "\n")

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
                    records.append(normalize_attempt(json.loads(line)))
                except (json.JSONDecodeError, KeyError, TypeError, ValueError) as exc:
                    malformed.append({"line": number, "content": line.rstrip(), "error": str(exc)})
        return records if not include_malformed else (records, malformed)

    def replace_all(self, attempts):
        self.path.parent.mkdir(parents=True, exist_ok=True)
        with self.path.open("w", encoding="utf-8") as handle:
            for attempt in attempts:
                handle.write(json.dumps(normalize_attempt(attempt), sort_keys=True) + "\n")

    def migrate_v1_to_v2_atomic(self, quarantine_path=None):
        records, malformed = self.load_all(include_malformed=True)
        seen = set()
        deduped = []
        for record in records:
            key = record["attempt_id"]
            if key in seen:
                continue
            seen.add(key)
            deduped.append(record)
        self.path.parent.mkdir(parents=True, exist_ok=True)
        fd, tmp_name = tempfile.mkstemp(prefix=f"{self.path.name}.", suffix=".tmp", dir=self.path.parent)
        with os.fdopen(fd, "w", encoding="utf-8") as handle:
            for record in deduped:
                handle.write(json.dumps(record, sort_keys=True) + "\n")
        os.replace(tmp_name, self.path)
        if quarantine_path and malformed:
            qpath = Path(quarantine_path)
            qpath.parent.mkdir(parents=True, exist_ok=True)
            qpath.write_text(json.dumps(malformed, sort_keys=True), encoding="utf-8")
        return {"migrated": len(deduped), "malformed": len(malformed), "deduped": len(records) - len(deduped)}

    def report(self):
        by_destination = {}
        retryable_failures = []
        for attempt in self.load_all():
            destination = attempt["destination"]
            by_destination[destination] = by_destination.get(destination, 0) + 1
            if attempt.get("retryable"):
                retryable_failures.append(attempt["attempt_id"])
        return {
            "total_attempts": sum(by_destination.values()),
            "by_destination": by_destination,
            "retryable_failures": retryable_failures,
        }
"#,
    )?;
    write_file(
        &root.join(format!("src/{}/service.py", spec.package)),
        &format!(
            r#"from datetime import datetime, timezone

from {package}.models import Event
from {package}.repository import DeliveryAttemptStore
from {package}.router import route_event


def utc_now():
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


class DeliveryService:
    def __init__(self, store: DeliveryAttemptStore):
        self.store = store

    def record_attempt(self, event: Event, status="delivered", error_code=""):
        decision = route_event(event)
        existing = [attempt for attempt in self.store.load_all() if attempt.get("event_id") == event.event_id]
        attempt_number = len(existing) + 1
        attempt = {{
            "schema_version": 2,
            "attempt_id": f"{{event.event_id}}:{{attempt_number}}",
            "event_id": event.event_id,
            "destination": decision.destination,
            "status": status,
            "error_code": error_code,
            "created_at": utc_now(),
        }}
        self.store.append(attempt)
        return attempt

    def report(self):
        return self.store.report()
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

from {package}.models import Event
from {package}.repository import DeliveryAttemptStore
from {package}.router import route_event
from {package}.service import DeliveryService


def build_parser():
    parser = argparse.ArgumentParser(prog="{package}")
    subcommands = parser.add_subparsers(dest="command", required=True)
    route = subcommands.add_parser("route")
    route.add_argument("event_id")
    route.add_argument("kind")
    record = subcommands.add_parser("attempt-record")
    record.add_argument("store")
    record.add_argument("event_id")
    record.add_argument("kind")
    record.add_argument("--status", default="delivered")
    record.add_argument("--error-code", default="")
    report = subcommands.add_parser("attempts-report")
    report.add_argument("store")
    migrate = subcommands.add_parser("migrate-attempts")
    migrate.add_argument("store")
    migrate.add_argument("--quarantine")
    return parser


def main(argv=None, stdout=None):
    stdout = stdout or sys.stdout
    args = build_parser().parse_args(argv)
    if args.command == "route":
        decision = route_event(Event(args.event_id, args.kind, {{}}))
        stdout.write(f"{{decision.destination}}\n")
        return 0
    if args.command == "attempt-record":
        service = DeliveryService(DeliveryAttemptStore(args.store))
        attempt = service.record_attempt(Event(args.event_id, args.kind, {{}}), args.status, args.error_code)
        stdout.write(json.dumps(attempt, sort_keys=True) + "\n")
        return 0
    if args.command == "attempts-report":
        store = DeliveryAttemptStore(args.store)
        stdout.write(json.dumps(store.report(), sort_keys=True) + "\n")
        return 0
    if args.command == "migrate-attempts":
        store = DeliveryAttemptStore(args.store)
        stdout.write(json.dumps(store.migrate_v1_to_v2_atomic(args.quarantine), sort_keys=True) + "\n")
        return 0
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
"#,
            package = spec.package
        ),
    )?;
    write_file(
        &root.join("tests/test_delivery_baseline.py"),
        &format!(
            r#"import io
import json
import shutil
import tempfile
import unittest
from pathlib import Path

from {package}.cli import main
from {package}.models import Event
from {package}.repository import DeliveryAttemptStore
from {package}.service import DeliveryService


class DeliveryBaselineTest(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp())
        self.store_path = self.tmp / "attempts.jsonl"

    def tearDown(self):
        shutil.rmtree(self.tmp)

    def test_route_cli_preserves_existing_behavior(self):
        output = io.StringIO()
        exit_code = main(["route", "evt-cli", "{primary_kind}"], stdout=output)
        self.assertEqual(exit_code, 0)
        self.assertEqual(output.getvalue().strip(), "{primary_destination}")

    def test_v1_fixture_report_still_loads(self):
        fixture = Path("fixtures/delivery_attempts_v1.jsonl")
        report = DeliveryAttemptStore(fixture).report()
        self.assertEqual(report["by_destination"]["{primary_destination}"], 1)
        self.assertEqual(report["by_destination"]["default"], 1)
        self.assertEqual(report["retryable_failures"], ["old-2"])

    def test_service_records_schema_versioned_retry_sequence(self):
        service = DeliveryService(DeliveryAttemptStore(self.store_path))
        first = service.record_attempt(Event("evt-new", "{primary_kind}", {{}}))
        second = service.record_attempt(Event("evt-new", "{primary_kind}", {{}}), status="timeout", error_code="timeout")
        self.assertEqual(first["attempt_id"], "evt-new:1")
        self.assertEqual(second["attempt_id"], "evt-new:2")
        self.assertEqual(DeliveryAttemptStore(self.store_path).load_all()[0]["schema_version"], 2)

    def test_migration_cli_is_atomic_and_quarantines_malformed(self):
        shutil.copyfile(Path("fixtures/delivery_attempts_v1.jsonl"), self.store_path)
        quarantine = self.tmp / "quarantine.json"
        output = io.StringIO()
        exit_code = main(["migrate-attempts", str(self.store_path), "--quarantine", str(quarantine)], stdout=output)
        self.assertEqual(exit_code, 0)
        result = json.loads(output.getvalue())
        self.assertEqual(result["migrated"], 2)
        self.assertEqual(result["malformed"], 1)
        self.assertTrue(quarantine.exists())
        self.assertEqual(DeliveryAttemptStore(self.store_path).load_all()[0]["schema_version"], 2)


if __name__ == "__main__":
    unittest.main()
"#,
            package = spec.package,
            primary_kind = spec.primary_kind,
            primary_destination = spec.primary_destination
        ),
    )?;
    Ok(())
}

fn worker_prompt(job: &LiveLevel10Job) -> String {
    format!(
        "You are running a live Level 10 long-run coding workflow probe. You are not alone in the broader codebase: do not revert or modify anything outside the assigned temp run directory. Your write ownership is limited to {project_root} and {receipts_root}.\n\nGoal: use the local_coding_program_builder long-run project-operator mode to continue this existing local Python project beyond a single checkpoint. Current files are authoritative; stored memory is only resume context. Do not ask follow-up questions unless the project is genuinely blocked by a user-owned product or architecture decision.\n\nEnvironment:\n- Project root: {project_root}\n- Python package: {package}\n- Receipts root: {receipts_root}\n- Isolated memory DB: {memory_db_path}\n- Resume token: {resume_token}\n- Prior memory row id: {prior_memory_row_id}\n- Expected checkpoint 004 memory row id: {checkpoint4_memory_row_id}\n- Expected checkpoint 005 memory row id: {checkpoint5_memory_row_id}\n- Memory CLI command pattern: INFRING_MEMORY_DB_PATH={memory_db_path} cargo run --quiet --manifest-path /Users/jay/.openclaw/workspace/core/layer0/memory/Cargo.toml --bin memory-cli -- <command>\n- Validation command from project root: {validation_command}\n\nWorkflow requirements:\n1. Read the local project files first. Current files override memory.\n2. Retrieve checkpoint memory using the resume token and/or prior row id.\n3. Write {receipts_root}/project_operator_roadmap.json before implementation. It must include product_goal, architecture_intent, stack_and_runtime, milestones, checkpoint_queue, non_goals, risk_register, validation_strategy, and stop_conditions.\n4. Implement checkpoint_004_operator_audit_workbench as a coherent milestone. Add durable operator/audit/task workbench behavior in multiple files, with JSONL or other stdlib persistence, CLI surface, and regression tests. Preserve route, attempt-record, attempts-report, and migrate-attempts behavior.\n5. Implement this exact new CLI contract in addition to the existing commands:\n   - operator-open <workbench_store> <task_id> --title <title> --owner <owner> --source-attempt <attempt_id>\n   - operator-transition <workbench_store> <task_id> closed --note <note>\n   - operator-report <workbench_store>\n   - slo-report <workbench_store> <attempt_store> --max-open-tasks <n> --max-retryable-failures <n>\n   - snapshot-export <snapshot_path> --attempt-store <attempt_store> --workbench-store <workbench_store>\n   - snapshot-import <snapshot_path> --attempt-store <attempt_store> --workbench-store <workbench_store>\n   All new commands should return JSON to stdout and should work through python3 -m {package}.cli from the project root. The status closed must be accepted by operator-transition and represented as a terminal/non-open task status.\n6. Run validation after checkpoint 004. Write {receipts_root}/checkpoint_004_handoff.json with completed_checkpoint, validation_summary, changed_files, architecture_decisions, risk_register_updates, memory_written, and next_checkpoint_decision.\n7. Write memory row {checkpoint4_memory_row_id} with tags coding,checkpoint,resume,project_context. Include changed files, validation result, known risks, and recommended next checkpoint.\n8. Continue to checkpoint_005 only if validation passed, risk is bounded, and the roadmap still applies. If you stop, the stop reason must be specific and valid.\n9. Implement checkpoint_005_slo_escalation_and_snapshot_export. Add SLO or policy evaluation for operator work, escalation/reporting behavior, snapshot export/import or equivalent resumable project state surface, CLI coverage, and regression tests.\n10. Run validation after checkpoint 005. Write {receipts_root}/checkpoint_005_handoff.json with the same fields as checkpoint 004 plus the continuation decision.\n11. Write memory row {checkpoint5_memory_row_id} with tags coding,checkpoint,resume,project_context.\n12. Final response should include pass/fail, changed file paths, validation command/result, memory row ids written, and caveats. Do not commit anything.\n",
        project_root = job.project_root,
        package = job.package,
        receipts_root = job.receipts_root,
        memory_db_path = job.memory_db_path,
        resume_token = job.resume_token,
        prior_memory_row_id = job.prior_memory_row_id,
        checkpoint4_memory_row_id = job.expected_checkpoint4_memory_row_id,
        checkpoint5_memory_row_id = job.expected_checkpoint5_memory_row_id,
        validation_command = job.validation_command
    )
}

fn judge_live_attempt(
    job: &LiveLevel10Job,
    batch_started_at_unix_ms: Option<u128>,
) -> LiveLevel10AttemptJudge {
    let mut checks = Vec::new();
    let mut failures = Vec::new();
    let project_root = PathBuf::from(&job.project_root);
    let receipts_root = PathBuf::from(&job.receipts_root);
    let roadmap_path = receipts_root.join("project_operator_roadmap.json");
    let checkpoint4_path = receipts_root.join("checkpoint_004_handoff.json");
    let checkpoint5_path = receipts_root.join("checkpoint_005_handoff.json");
    let timing = attempt_timing(
        &[
            roadmap_path.as_path(),
            checkpoint4_path.as_path(),
            checkpoint5_path.as_path(),
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
        &checkpoint4_path,
        "checkpoint_004_operator_audit_workbench",
        "checkpoint_004_receipt_written",
        "checkpoint_004_receipt_declares_operator_workbench",
    );
    judge_checkpoint_receipt(
        &mut checks,
        &mut failures,
        &checkpoint5_path,
        "checkpoint_005_slo_escalation_and_snapshot_export",
        "checkpoint_005_receipt_written",
        "checkpoint_005_receipt_declares_slo_export",
    );

    let bridge = CodingMemoryRuntimeBridge {
        workspace_root: workspace_root(),
        memory_db_path: PathBuf::from(&job.memory_db_path),
        session_id: job.attempt_id.clone(),
    };
    let checkpoint4_memory = bridge.get(&job.expected_checkpoint4_memory_row_id);
    push_check(
        &mut checks,
        &mut failures,
        "checkpoint_004_memory_written",
        checkpoint4_memory.ok,
        checkpoint4_memory.payload.to_string(),
    );
    let checkpoint5_memory = bridge.get(&job.expected_checkpoint5_memory_row_id);
    push_check(
        &mut checks,
        &mut failures,
        "checkpoint_005_memory_written",
        checkpoint5_memory.ok,
        checkpoint5_memory.payload.to_string(),
    );

    let evidence = collect_project_text(&project_root);
    let lower = evidence.to_lowercase();
    push_check(
        &mut checks,
        &mut failures,
        "operator_audit_workbench_implemented",
        (lower.contains("operator") || lower.contains("workbench"))
            && lower.contains("task")
            && (lower.contains("jsonl") || lower.contains("store") || lower.contains("snapshot")),
        "source contains operator/task durable workbench signals".to_string(),
    );
    push_check(
        &mut checks,
        &mut failures,
        "slo_escalation_implemented",
        (lower.contains("slo") || lower.contains("policy"))
            && (lower.contains("escalat")
                || lower.contains("violation")
                || lower.contains("threshold")),
        "source contains SLO/policy escalation or violation signals".to_string(),
    );
    push_check(
        &mut checks,
        &mut failures,
        "snapshot_export_or_import_implemented",
        lower.contains("snapshot") && lower.contains("export") && lower.contains("import"),
        "source contains snapshot export/import signals".to_string(),
    );
    push_check(
        &mut checks,
        &mut failures,
        "cli_surfaces_new_operator_commands",
        lower.contains("argparse")
            && lower.contains("operator")
            && lower.contains("slo")
            && lower.contains("snapshot"),
        "CLI source exposes operator, SLO, and snapshot commands".to_string(),
    );
    push_check(
        &mut checks,
        &mut failures,
        "regression_tests_cover_new_checkpoints",
        lower.matches("unittest").count() >= 2
            && lower.contains("operator")
            && lower.contains("slo")
            && lower.contains("snapshot"),
        "tests mention operator, SLO, and snapshot behavior".to_string(),
    );
    push_check(
        &mut checks,
        &mut failures,
        "baseline_delivery_behavior_preserved",
        lower.contains("migrate-attempts")
            && lower.contains("attempts-report")
            && lower.contains("route")
            && lower.contains("schema_version"),
        "baseline route/report/migration schema behavior still present".to_string(),
    );
    let semantic_probe = run_level10_cli_semantic_probe(&project_root, &job.package);
    push_check(
        &mut checks,
        &mut failures,
        "strict_cli_semantic_probe_passes",
        semantic_probe.ok,
        semantic_probe.detail,
    );

    LiveLevel10AttemptJudge {
        attempt_id: job.attempt_id.clone(),
        ok: failures.is_empty(),
        timing,
        checks,
        failures,
    }
}

fn judge_checkpoint_receipt(
    checks: &mut Vec<LiveLevel10Check>,
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
            changed_file_count >= 3,
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
    checks: &mut Vec<LiveLevel10Check>,
    failures: &mut Vec<String>,
    id: &'static str,
    ok: bool,
    detail: String,
) {
    checks.push(LiveLevel10Check { id, ok, detail });
    if !ok {
        let check = checks.last().expect("just pushed");
        failures.push(format!("{}:{}", check.id, check.detail));
    }
}

#[derive(Debug)]
struct CommandResult {
    ok: bool,
    detail: String,
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
        },
        Err(error) => CommandResult {
            ok: false,
            detail: format!("spawn_failed:{error}"),
        },
    }
}

fn run_level10_cli_semantic_probe(project_root: &Path, package: &str) -> CommandResult {
    let Some(spec) = spec_for_package(package) else {
        return CommandResult {
            ok: false,
            detail: format!("unknown_package:{package}"),
        };
    };
    let probe_root = PathBuf::from(project_root).join(".level10_strict_judge");
    if probe_root.exists() {
        if let Err(error) = fs::remove_dir_all(&probe_root) {
            return CommandResult {
                ok: false,
                detail: format!("clear_probe_root_failed:{}:{error}", probe_root.display()),
            };
        }
    }
    if let Err(error) = fs::create_dir_all(&probe_root) {
        return CommandResult {
            ok: false,
            detail: format!("create_probe_root_failed:{}:{error}", probe_root.display()),
        };
    }
    let attempt_store = probe_root.join("attempts.jsonl");
    let migrated_store = probe_root.join("migrated_attempts.jsonl");
    let quarantine_path = probe_root.join("quarantine.json");
    let workbench_store = probe_root.join("workbench.jsonl");
    let snapshot_path = probe_root.join("snapshot.json");
    let restored_attempt_store = probe_root.join("restored_attempts.jsonl");
    let restored_workbench_store = probe_root.join("restored_workbench.jsonl");

    let route = run_cli(
        project_root,
        package,
        &["route", "evt-judge", spec.primary_kind],
    );
    if !route.ok || route.stdout.trim() != spec.primary_destination {
        let route_stdout = route.stdout.trim().to_string();
        return route.fail_with(format!(
            "route_expected={};stdout={}",
            spec.primary_destination, route_stdout
        ));
    }

    let record = run_cli(
        project_root,
        package,
        &[
            "attempt-record",
            &attempt_store.display().to_string(),
            "evt-judge",
            spec.primary_kind,
            "--status",
            "timeout",
            "--error-code",
            "timeout",
        ],
    );
    if !record.ok || parse_json(&record.stdout).is_none() {
        return record.fail_with("attempt-record_failed_or_non_json".to_string());
    }

    let report = run_cli(
        project_root,
        package,
        &["attempts-report", &attempt_store.display().to_string()],
    );
    let Some(report_json) = parse_json(&report.stdout) else {
        return report.fail_with("attempts-report_non_json".to_string());
    };
    if !report.ok
        || report_json
            .get("total_attempts")
            .and_then(Value::as_u64)
            .unwrap_or_default()
            < 1
    {
        return report.fail_with("attempts-report_missing_total_attempts".to_string());
    }

    let fixture = project_root.join("fixtures/delivery_attempts_v1.jsonl");
    if let Err(error) = fs::copy(&fixture, &migrated_store) {
        return CommandResult {
            ok: false,
            detail: format!(
                "copy_migration_fixture_failed:{}->{}:{error}",
                fixture.display(),
                migrated_store.display()
            ),
        };
    }
    let migration = run_cli(
        project_root,
        package,
        &[
            "migrate-attempts",
            &migrated_store.display().to_string(),
            "--quarantine",
            &quarantine_path.display().to_string(),
        ],
    );
    if !migration.ok || !quarantine_path.exists() {
        return migration.fail_with(format!(
            "migration_or_quarantine_failed:{}",
            quarantine_path.display()
        ));
    }

    let open = run_cli(
        project_root,
        package,
        &[
            "operator-open",
            &workbench_store.display().to_string(),
            "task-judge",
            "--title",
            "Investigate retry spike",
            "--owner",
            "judge",
            "--source-attempt",
            "evt-judge:1",
        ],
    );
    if !open.ok || parse_json(&open.stdout).is_none() {
        return open.fail_with("operator-open_failed_or_non_json".to_string());
    }

    let transition = run_cli(
        project_root,
        package,
        &[
            "operator-transition",
            &workbench_store.display().to_string(),
            "task-judge",
            "closed",
            "--note",
            "validated by strict judge",
        ],
    );
    if !transition.ok || parse_json(&transition.stdout).is_none() {
        return transition.fail_with("operator-transition_failed_or_non_json".to_string());
    }

    let operator_report = run_cli(
        project_root,
        package,
        &["operator-report", &workbench_store.display().to_string()],
    );
    let Some(operator_report_json) = parse_json(&operator_report.stdout) else {
        return operator_report.fail_with("operator-report_non_json".to_string());
    };
    if !operator_report.ok
        || !(json_contains_text(&operator_report_json, "task-judge")
            || json_u64_field(&operator_report_json, "total_tasks") >= 1)
    {
        return operator_report.fail_with("operator-report_missing_task".to_string());
    }

    let slo = run_cli(
        project_root,
        package,
        &[
            "slo-report",
            &workbench_store.display().to_string(),
            &attempt_store.display().to_string(),
            "--max-open-tasks",
            "0",
            "--max-retryable-failures",
            "0",
        ],
    );
    let Some(slo_json) = parse_json(&slo.stdout) else {
        return slo.fail_with("slo-report_non_json".to_string());
    };
    if !slo.ok || !slo_report_has_escalation_signal(&slo_json) {
        return slo.fail_with("slo-report_missing_slo_or_breach_signal".to_string());
    }

    let export = run_cli(
        project_root,
        package,
        &[
            "snapshot-export",
            &snapshot_path.display().to_string(),
            "--attempt-store",
            &attempt_store.display().to_string(),
            "--workbench-store",
            &workbench_store.display().to_string(),
        ],
    );
    if !export.ok || !snapshot_path.exists() {
        return export.fail_with(format!(
            "snapshot-export_failed:{}",
            snapshot_path.display()
        ));
    }

    let import = run_cli(
        project_root,
        package,
        &[
            "snapshot-import",
            &snapshot_path.display().to_string(),
            "--attempt-store",
            &restored_attempt_store.display().to_string(),
            "--workbench-store",
            &restored_workbench_store.display().to_string(),
        ],
    );
    if !import.ok || !restored_attempt_store.exists() || !restored_workbench_store.exists() {
        return import.fail_with("snapshot-import_failed_or_missing_restored_files".to_string());
    }

    let restored_report = run_cli(
        project_root,
        package,
        &[
            "operator-report",
            &restored_workbench_store.display().to_string(),
        ],
    );
    let Some(restored_json) = parse_json(&restored_report.stdout) else {
        return restored_report.fail_with("restored_operator_report_non_json".to_string());
    };
    if !restored_report.ok
        || !(json_contains_text(&restored_json, "task-judge")
            || json_u64_field(&restored_json, "total_tasks") >= 1)
    {
        return restored_report.fail_with("snapshot_roundtrip_missing_task".to_string());
    }

    CommandResult {
        ok: true,
        detail: format!(
            "strict CLI semantic probe passed for package={} probe_root={}",
            package,
            probe_root.display()
        ),
    }
}

#[derive(Debug)]
struct CliResult {
    ok: bool,
    stdout: String,
    stderr: String,
    status: Option<i32>,
    command: String,
}

impl CliResult {
    fn fail_with(self, reason: String) -> CommandResult {
        CommandResult {
            ok: false,
            detail: format!(
                "{reason}; command={}; status={:?}; stdout={}; stderr={}",
                self.command, self.status, self.stdout, self.stderr
            ),
        }
    }
}

fn run_cli(project_root: &Path, package: &str, args: &[&str]) -> CliResult {
    let output = Command::new("python3")
        .arg("-m")
        .arg(format!("{package}.cli"))
        .args(args)
        .env("PYTHONPATH", "src")
        .current_dir(project_root)
        .output();
    let command = format!("python3 -m {package}.cli {}", args.join(" "));
    match output {
        Ok(output) => CliResult {
            ok: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            status: output.status.code(),
            command,
        },
        Err(error) => CliResult {
            ok: false,
            stdout: String::new(),
            stderr: format!("spawn_failed:{error}"),
            status: None,
            command,
        },
    }
}

fn parse_json(raw: &str) -> Option<Value> {
    serde_json::from_str::<Value>(raw.trim()).ok()
}

fn json_contains_text(value: &Value, needle: &str) -> bool {
    value
        .to_string()
        .to_lowercase()
        .contains(&needle.to_lowercase())
}

fn json_u64_field(value: &Value, field: &str) -> u64 {
    value.get(field).and_then(Value::as_u64).unwrap_or_default()
}

fn json_bool_field(value: &Value, field: &str) -> bool {
    value
        .get(field)
        .and_then(Value::as_bool)
        .unwrap_or_default()
}

fn json_array_len(value: &Value, field: &str) -> usize {
    value
        .get(field)
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or_default()
}

fn json_string_field(value: &Value, field: &str) -> String {
    value
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_lowercase()
}

fn nested_u64_field(value: &Value, object: &str, field: &str) -> u64 {
    value
        .get(object)
        .and_then(|nested| nested.get(field))
        .and_then(Value::as_u64)
        .unwrap_or_default()
}

fn slo_report_has_escalation_signal(value: &Value) -> bool {
    json_contains_text(value, "slo")
        || json_contains_text(value, "breach")
        || json_contains_text(value, "escalation")
        || json_contains_text(value, "violation")
        || json_contains_text(value, "policy")
        || json_contains_text(value, "retryable")
        || json_bool_field(value, "escalation_required")
        || json_array_len(value, "violations") > 0
        || json_array_len(value, "escalations") > 0
        || matches!(
            json_string_field(value, "status").as_str(),
            "fail" | "violation" | "escalate"
        )
        || json_u64_field(value, "retryable_failure_count") > 0
        || nested_u64_field(value, "metrics", "retryable_failures") > 0
        || nested_u64_field(value, "observed", "retryable_failures") > 0
}

fn summarize_level10_timing(
    batch_started_at_unix_ms: Option<u128>,
    judged_at_unix_ms: u128,
    attempts: &[LiveLevel10AttemptJudge],
) -> LiveLevel10TimingSummary {
    let completed = attempts
        .iter()
        .filter_map(|attempt| attempt.timing.completed_at_unix_ms)
        .collect::<Vec<_>>();
    let first_attempt_completed_at_unix_ms = completed.iter().copied().min();
    let last_attempt_completed_at_unix_ms = completed.iter().copied().max();
    let elapsed = attempts
        .iter()
        .filter_map(|attempt| attempt.timing.elapsed_ms_since_batch_start)
        .collect::<Vec<_>>();
    let average_attempt_elapsed_ms = if elapsed.is_empty() {
        None
    } else {
        Some(elapsed.iter().sum::<u128>() / elapsed.len() as u128)
    };
    LiveLevel10TimingSummary {
        batch_started_at_unix_ms,
        judged_at_unix_ms,
        batch_elapsed_ms: batch_started_at_unix_ms
            .and_then(|started| judged_at_unix_ms.checked_sub(started)),
        first_attempt_completed_at_unix_ms,
        last_attempt_completed_at_unix_ms,
        completion_span_ms: first_attempt_completed_at_unix_ms.and_then(|first| {
            last_attempt_completed_at_unix_ms.and_then(|last| last.checked_sub(first))
        }),
        average_attempt_elapsed_ms,
    }
}

fn attempt_timing(
    receipt_paths: &[&Path],
    batch_started_at_unix_ms: Option<u128>,
) -> LiveLevel10AttemptTiming {
    let receipt_times = receipt_paths
        .iter()
        .filter_map(|path| file_modified_unix_ms(path))
        .collect::<Vec<_>>();
    let first_receipt_unix_ms = receipt_times.iter().copied().min();
    let completed_at_unix_ms = receipt_times.iter().copied().max();
    LiveLevel10AttemptTiming {
        first_receipt_unix_ms,
        completed_at_unix_ms,
        elapsed_ms_since_batch_start: batch_started_at_unix_ms.and_then(|started| {
            completed_at_unix_ms.and_then(|completed| completed.checked_sub(started))
        }),
    }
}

fn file_modified_unix_ms(path: &Path) -> Option<u128> {
    fs::metadata(path)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis())
}

fn read_json_file(path: &Path) -> Option<Value> {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
}

fn collect_project_text(root: &Path) -> String {
    let mut output = String::new();
    collect_project_text_inner(root, &mut output);
    output
}

fn collect_project_text_inner(path: &Path, output: &mut String) {
    if path.is_dir() {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                collect_project_text_inner(&entry.path(), output);
            }
        }
        return;
    }
    let Some(extension) = path.extension().and_then(|raw| raw.to_str()) else {
        return;
    };
    if !matches!(extension, "py" | "md" | "txt" | "json") {
        return;
    }
    if let Ok(raw) = fs::read_to_string(path) {
        output.push_str("\n--- ");
        output.push_str(&path.display().to_string());
        output.push_str(" ---\n");
        output.push_str(&raw);
    }
}

fn read_to_string(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

fn write_file(path: &Path, content: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create_parent_failed:{}:{error}", parent.display()))?;
    }
    fs::write(path, content).map_err(|error| format!("write_failed:{}:{error}", path.display()))
}
