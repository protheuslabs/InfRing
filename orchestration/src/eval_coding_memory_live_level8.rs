use crate::coding_memory_runtime_bridge::{
    millis_now, project_snapshot, workspace_root, CodingMemoryRuntimeBridge,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

const LEVEL8_WORKER_TIMEOUT_SECONDS: u64 = 900;
const LEVEL8_WORKER_HEARTBEAT_SECONDS: u64 = 30;
const LEVEL8_PROVIDER_TIMEOUT_SECONDS: u64 = 300;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveLevel8SeedBatchReport {
    pub harness_kind: String,
    pub ok: bool,
    pub batch_root: String,
    pub attempt_count: usize,
    pub jobs: Vec<LiveLevel8Job>,
    pub failures: Vec<String>,
    pub operator_next_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveLevel8Job {
    pub attempt_id: String,
    pub package: String,
    pub run_root: String,
    pub project_root: String,
    pub receipts_root: String,
    pub prompt_path: String,
    pub memory_db_path: String,
    pub resume_token: String,
    pub prior_memory_row_id: String,
    pub expected_new_memory_row_id: String,
    pub project_fingerprint: String,
    pub architecture_hash: String,
    pub validation_command: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LiveLevel8JudgeReport {
    pub harness_kind: &'static str,
    pub ok: bool,
    pub batch_root: String,
    pub attempt_count: usize,
    pub scored_attempt_count: usize,
    pub pass_count: usize,
    pub fail_count: usize,
    pub infra_failure_count: usize,
    pub coding_failure_count: usize,
    pub attempts: Vec<LiveLevel8AttemptJudge>,
    pub failures: Vec<String>,
    pub infra_failures: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LiveLevel8RunReport {
    pub harness_kind: &'static str,
    pub ok: bool,
    pub batch_root: String,
    pub attempt_count: usize,
    pub jobs: Vec<LiveLevel8Job>,
    pub worker_runs: Vec<LiveLevel8WorkerRun>,
    pub judge: LiveLevel8JudgeReport,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LiveLevel8WorkerRun {
    pub attempt_id: String,
    pub ok: bool,
    pub output_path: String,
    pub timeout_seconds: u64,
    pub timeout_count: usize,
    pub duration_seconds: u64,
    pub run_count: usize,
    pub retried_infra_failure: bool,
    pub final_infra_failure: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LiveLevel8AttemptJudge {
    pub attempt_id: String,
    pub ok: bool,
    pub classification: &'static str,
    pub checks: Vec<LiveLevel8Check>,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LiveLevel8Check {
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
        id: "event_delivery_ops",
        package: "event_delivery_ops",
        architecture_name: "Event Delivery Ops",
        primary_kind: "billing.invoice.created",
        primary_destination: "billing",
        secondary_prefix: "support",
        secondary_destination: "support",
    },
    DomainSpec {
        id: "incident_delivery_ops",
        package: "incident_delivery_ops",
        architecture_name: "Incident Delivery Ops",
        primary_kind: "incident.created",
        primary_destination: "incident",
        secondary_prefix: "oncall",
        secondary_destination: "oncall",
    },
    DomainSpec {
        id: "fulfillment_delivery_ops",
        package: "fulfillment_delivery_ops",
        architecture_name: "Fulfillment Delivery Ops",
        primary_kind: "order.shipped",
        primary_destination: "fulfillment",
        secondary_prefix: "warehouse",
        secondary_destination: "warehouse",
    },
    DomainSpec {
        id: "risk_delivery_ops",
        package: "risk_delivery_ops",
        architecture_name: "Risk Delivery Ops",
        primary_kind: "risk.alert.created",
        primary_destination: "risk",
        secondary_prefix: "fraud",
        secondary_destination: "fraud",
    },
    DomainSpec {
        id: "notification_delivery_ops",
        package: "notification_delivery_ops",
        architecture_name: "Notification Delivery Ops",
        primary_kind: "notification.email.queued",
        primary_destination: "email",
        secondary_prefix: "sms",
        secondary_destination: "sms",
    },
];

pub fn seed_live_level8_batch(attempt_count: usize) -> LiveLevel8SeedBatchReport {
    let count = attempt_count.max(1);
    let batch_root = std::env::temp_dir().join(format!(
        "coding-memory-live-level8-batch-{}-{}",
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
        let spec = &DOMAIN_SPECS[index % DOMAIN_SPECS.len()];
        match seed_live_attempt(index + 1, spec, &batch_root, &prompts_root) {
            Ok(job) => jobs.push(job),
            Err(error) => failures.push(error),
        }
    }

    let report = LiveLevel8SeedBatchReport {
        harness_kind: "coding_memory_live_level8_seed_v1".to_string(),
        ok: failures.is_empty() && jobs.len() == count,
        batch_root: batch_root.display().to_string(),
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

pub fn judge_live_level8_batch(batch_root: &Path) -> LiveLevel8JudgeReport {
    let mut failures = Vec::new();
    let mut infra_failures = Vec::new();
    let jobs_path = batch_root.join("jobs.json");
    let seed_report = fs::read_to_string(&jobs_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<LiveLevel8SeedBatchReport>(&raw).ok());
    let jobs = match seed_report {
        Some(report) => report.jobs,
        None => {
            failures.push(format!("jobs_json_unreadable:{}", jobs_path.display()));
            Vec::new()
        }
    };

    let attempts = jobs.iter().map(judge_live_attempt).collect::<Vec<_>>();
    for attempt in &attempts {
        if !attempt.ok {
            let target = if attempt.classification == "infra_failure" {
                &mut infra_failures
            } else {
                &mut failures
            };
            target.extend(
                attempt
                    .failures
                    .iter()
                    .map(|failure| format!("{}:{failure}", attempt.attempt_id)),
            );
        }
    }
    let pass_count = attempts.iter().filter(|attempt| attempt.ok).count();
    let infra_failure_count = attempts
        .iter()
        .filter(|attempt| attempt.classification == "infra_failure")
        .count();
    let coding_failure_count = attempts
        .iter()
        .filter(|attempt| attempt.classification == "coding_failure")
        .count();
    let scored_attempt_count = attempts.len().saturating_sub(infra_failure_count);
    let fail_count = coding_failure_count;
    LiveLevel8JudgeReport {
        harness_kind: "coding_memory_live_level8_judge_v1",
        ok: failures.is_empty() && scored_attempt_count > 0,
        batch_root: batch_root.display().to_string(),
        attempt_count: attempts.len(),
        scored_attempt_count,
        pass_count,
        fail_count,
        infra_failure_count,
        coding_failure_count,
        attempts,
        failures,
        infra_failures,
    }
}

pub fn run_live_level8_batch(attempt_count: usize, infra_retries: usize) -> LiveLevel8RunReport {
    let seed = seed_live_level8_batch(attempt_count);
    let batch_root = PathBuf::from(&seed.batch_root);
    let outputs_root = batch_root.join("agent_outputs");
    let mut worker_runs = Vec::new();
    let mut failures = seed.failures.clone();
    if let Err(error) = fs::create_dir_all(&outputs_root) {
        failures.push(format!(
            "create_agent_outputs_failed:{}:{error}",
            outputs_root.display()
        ));
    }
    eprintln!(
        "level8_batch_start attempts={} infra_retries={} batch_root={}",
        seed.jobs.len(),
        infra_retries,
        seed.batch_root
    );
    for (index, job) in seed.jobs.iter().enumerate() {
        eprintln!(
            "level8_worker_queue ordinal={}/{} attempt={}",
            index + 1,
            seed.jobs.len(),
            job.attempt_id
        );
        let worker = run_live_level8_worker(job, &outputs_root, index + 1, infra_retries);
        if !worker.ok {
            if let Some(kind) = &worker.final_infra_failure {
                failures.push(format!("{}:infra_failure:{kind}", job.attempt_id));
            }
        }
        eprintln!(
            "level8_worker_done attempt={} ok={} runs={} duration_seconds={} timeout_count={} final_infra_failure={}",
            worker.attempt_id,
            worker.ok,
            worker.run_count,
            worker.duration_seconds,
            worker.timeout_count,
            worker
                .final_infra_failure
                .clone()
                .unwrap_or_else(|| "none".to_string())
        );
        worker_runs.push(worker);
    }
    let judge = judge_live_level8_batch(&batch_root);
    LiveLevel8RunReport {
        harness_kind: "coding_memory_live_level8_run_v1",
        ok: seed.ok && judge.ok,
        batch_root: seed.batch_root.clone(),
        attempt_count: seed.jobs.len(),
        jobs: seed.jobs,
        worker_runs,
        judge,
        failures,
    }
}

fn run_live_level8_worker(
    job: &LiveLevel8Job,
    outputs_root: &Path,
    ordinal: usize,
    infra_retries: usize,
) -> LiveLevel8WorkerRun {
    let output_path = outputs_root.join(format!("{}.json", job.attempt_id));
    let mut run_count = 0usize;
    let mut timeout_count = 0usize;
    let mut duration_seconds = 0u64;
    let mut retried_infra_failure = false;
    let mut final_infra_failure = None;
    let mut ok = false;
    let timeout_seconds = level8_worker_timeout_seconds();
    for retry_index in 0..=infra_retries {
        run_count += 1;
        let start = millis_now();
        let stdout_path =
            outputs_root.join(format!("{}.try{}.stdout.log", job.attempt_id, run_count));
        let stderr_path =
            outputs_root.join(format!("{}.try{}.stderr.log", job.attempt_id, run_count));
        eprintln!(
            "level8_worker_start attempt={} try={}/{} timeout_seconds={} stdout={} stderr={}",
            job.attempt_id,
            run_count,
            infra_retries + 1,
            timeout_seconds,
            stdout_path.display(),
            stderr_path.display()
        );
        let output = run_level8_worker_command(
            job,
            ordinal,
            run_count,
            timeout_seconds,
            &stdout_path,
            &stderr_path,
        );
        duration_seconds =
            duration_seconds.saturating_add(((millis_now().saturating_sub(start)) / 1000) as u64);
        if output.timed_out {
            timeout_count += 1;
        }
        let artifact = match output {
            Level8WorkerCommandOutput {
                status,
                stdout,
                stderr,
                timed_out,
                spawn_error: None,
            } => {
                let infra_marker = if timed_out || status.starts_with("try_wait_failed:") {
                    "worker_infra_failure=provider_timeout_or_spawn_failure\n"
                } else {
                    ""
                };
                let text = format!(
                    "status={status}\ntimed_out={timed_out}\ntimeout_seconds={timeout_seconds}\n{infra_marker}stdout_path={}\nstderr_path={}\nstdout:\n{stdout}\nstderr:\n{stderr}",
                    stdout_path.display(),
                    stderr_path.display()
                );
                ok = status.starts_with("exit status: 0") && !timed_out;
                text
            }
            Level8WorkerCommandOutput {
                status,
                stdout,
                stderr,
                timed_out,
                spawn_error: Some(error),
            } => {
                ok = false;
                format!(
                    "status={status}\ntimed_out={timed_out}\ntimeout_seconds={timeout_seconds}\nworker_infra_failure=provider_timeout_or_spawn_failure\nworker_spawn_failed:{error}\nstdout_path={}\nstderr_path={}\nstdout:\n{stdout}\nstderr:\n{stderr}",
                    stdout_path.display(),
                    stderr_path.display()
                )
            }
        };
        let _ = write_file(&output_path, &artifact);
        final_infra_failure = if artifact.contains("timed_out=true")
            || artifact.contains("worker_spawn_failed:")
            || artifact.contains("try_wait_failed:")
        {
            Some("provider_timeout_or_spawn_failure".to_string())
        } else {
            classify_worker_infra_failure_text(&artifact)
        };
        if final_infra_failure.is_some() && retry_index < infra_retries {
            retried_infra_failure = true;
            let retry_path =
                outputs_root.join(format!("{}.infra_try{}.log", job.attempt_id, run_count));
            let _ = fs::rename(&output_path, retry_path);
            eprintln!(
                "level8_worker_retry attempt={} completed_try={} reason={} next_try={}",
                job.attempt_id,
                run_count,
                final_infra_failure
                    .clone()
                    .unwrap_or_else(|| "unknown_infra_failure".to_string()),
                run_count + 1
            );
            ok = false;
            continue;
        }
        if final_infra_failure.is_some() {
            ok = false;
        }
        eprintln!(
            "level8_worker_try_done attempt={} try={} ok={} elapsed_seconds={} timed_out={} infra_failure={}",
            job.attempt_id,
            run_count,
            ok,
            ((millis_now().saturating_sub(start)) / 1000),
            artifact.contains("timed_out=true"),
            final_infra_failure
                .clone()
                .unwrap_or_else(|| "none".to_string())
        );
        break;
    }
    LiveLevel8WorkerRun {
        attempt_id: job.attempt_id.clone(),
        ok,
        output_path: output_path.display().to_string(),
        timeout_seconds,
        timeout_count,
        duration_seconds,
        run_count,
        retried_infra_failure,
        final_infra_failure,
    }
}

struct Level8WorkerCommandOutput {
    status: String,
    stdout: String,
    stderr: String,
    timed_out: bool,
    spawn_error: Option<String>,
}

fn level8_worker_timeout_seconds() -> u64 {
    std::env::var("INFRING_LEVEL8_WORKER_TIMEOUT_SECONDS")
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .filter(|seconds| *seconds > 0)
        .unwrap_or(LEVEL8_WORKER_TIMEOUT_SECONDS)
}

fn level8_worker_provider() -> String {
    std::env::var("INFRING_LEVEL8_PROVIDER").unwrap_or_else(|_| "ollama".to_string())
}

fn level8_worker_model() -> String {
    std::env::var("INFRING_LEVEL8_MODEL").unwrap_or_else(|_| "kimi-k2.6:cloud".to_string())
}

fn level8_worker_provider_timeout_seconds() -> u64 {
    std::env::var("INFRING_LEVEL8_PROVIDER_TIMEOUT_SECONDS")
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .filter(|seconds| *seconds > 0)
        .unwrap_or(LEVEL8_PROVIDER_TIMEOUT_SECONDS)
}

fn run_level8_worker_command(
    job: &LiveLevel8Job,
    ordinal: usize,
    run_count: usize,
    timeout_seconds: u64,
    stdout_path: &Path,
    stderr_path: &Path,
) -> Level8WorkerCommandOutput {
    let stdout_file = match fs::File::create(stdout_path) {
        Ok(file) => file,
        Err(error) => {
            return Level8WorkerCommandOutput {
                status: "spawn_not_attempted".to_string(),
                stdout: String::new(),
                stderr: String::new(),
                timed_out: false,
                spawn_error: Some(format!("worker_stdout_log_create_failed:{error}")),
            };
        }
    };
    let stderr_file = match fs::File::create(stderr_path) {
        Ok(file) => file,
        Err(error) => {
            return Level8WorkerCommandOutput {
                status: "spawn_not_attempted".to_string(),
                stdout: String::new(),
                stderr: String::new(),
                timed_out: false,
                spawn_error: Some(format!("worker_stderr_log_create_failed:{error}")),
            };
        }
    };
    let mut command = Command::new("cargo");
    command
        .arg("run")
        .arg("-p")
        .arg("xtask")
        .arg("--")
        .arg("infring-agent-run")
        .arg(format!("--name=level8-native-{ordinal}-try{run_count}"))
        .arg("--workflow=coding_project_operator")
        .arg(format!("--provider={}", level8_worker_provider()))
        .arg(format!("--model={}", level8_worker_model()))
        .arg(format!("--prompt=@{}", job.prompt_path))
        .current_dir(workspace_root())
        .env(
            "INFRING_PROVIDER_TIMEOUT_SECONDS",
            level8_worker_provider_timeout_seconds().to_string(),
        )
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file));
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            return Level8WorkerCommandOutput {
                status: "spawn_failed".to_string(),
                stdout: read_to_string(stdout_path),
                stderr: read_to_string(stderr_path),
                timed_out: false,
                spawn_error: Some(error.to_string()),
            };
        }
    };
    let start = millis_now();
    let mut last_heartbeat = start;
    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.to_string(),
            Ok(None) => {
                let now = millis_now();
                let elapsed_seconds = ((now.saturating_sub(start)) / 1000) as u64;
                if elapsed_seconds >= timeout_seconds {
                    timed_out = true;
                    eprintln!(
                        "level8_worker_timeout attempt={} try={} elapsed_seconds={} timeout_seconds={}",
                        job.attempt_id,
                        run_count,
                        elapsed_seconds,
                        timeout_seconds
                    );
                    let _ = child.kill();
                    break child
                        .wait()
                        .map(|status| status.to_string())
                        .unwrap_or_else(|error| format!("wait_after_kill_failed:{error}"));
                }
                if ((now.saturating_sub(last_heartbeat)) / 1000) as u64
                    >= LEVEL8_WORKER_HEARTBEAT_SECONDS
                {
                    eprintln!(
                        "level8_worker_heartbeat attempt={} try={} elapsed_seconds={} timeout_seconds={}",
                        job.attempt_id,
                        run_count,
                        elapsed_seconds,
                        timeout_seconds
                    );
                    last_heartbeat = now;
                }
                thread::sleep(Duration::from_secs(1));
            }
            Err(error) => {
                break format!("try_wait_failed:{error}");
            }
        }
    };
    let stdout = read_to_string(stdout_path);
    let stderr = read_to_string(stderr_path);
    Level8WorkerCommandOutput {
        status,
        stdout,
        stderr,
        timed_out,
        spawn_error: None,
    }
}

fn seed_live_attempt(
    ordinal: usize,
    spec: &DomainSpec,
    batch_root: &Path,
    prompts_root: &Path,
) -> Result<LiveLevel8Job, String> {
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
    let resume_token = format!("live_level8_resume_{}_{}", attempt_id, millis_now());
    let prior_memory_row_id = format!(
        "coding_memory::{}::checkpoint::checkpoint_001",
        snapshot.project_fingerprint
    );
    let expected_new_memory_row_id = format!(
        "coding_memory::{}::checkpoint::checkpoint_002",
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
        "completed_checkpoint": "checkpoint_001_existing_router_service_cli",
        "changed_files": [
            "ARCHITECTURE.md",
            "PROJECT_MANIFEST.txt",
            &format!("src/{}/models.py", spec.package),
            &format!("src/{}/router.py", spec.package),
            &format!("src/{}/repository.py", spec.package),
            &format!("src/{}/service.py", spec.package),
            &format!("src/{}/cli.py", spec.package),
            "tests/test_baseline.py"
        ],
        "validation_results": {
            "status": "pass",
            "command": validation_command,
            "exit_code": 0
        },
        "recommended_next_checkpoint": "checkpoint_002_persistent_delivery_operations",
        "next_slice_goal": "Add persistent delivery-attempt operations: JSONL storage, attempt model, service integration, destination reports, retryable failure detection, import/export or report CLI behavior, regression tests, checkpoint receipt, and checkpoint memory.",
        "constraints": [
            "read current files before planning",
            "current workspace files remain authoritative over memory",
            "use Python stdlib only",
            "preserve baseline routing and route CLI behavior",
            "do not stop at an in-memory ledger"
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
    let job = LiveLevel8Job {
        attempt_id: attempt_id.clone(),
        package: spec.package.to_string(),
        run_root: run_root.display().to_string(),
        project_root: project_root.display().to_string(),
        receipts_root: receipts_root.display().to_string(),
        prompt_path: prompt_path.display().to_string(),
        memory_db_path: memory_db_path.display().to_string(),
        resume_token,
        prior_memory_row_id,
        expected_new_memory_row_id,
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
            "# {} Architecture\n\nThis existing project is a Python stdlib routing service with a domain model, pure router, in-memory route history, service facade, and CLI. Preserve baseline routing and the existing `route` CLI command. New delivery operations should be added behind focused modules and tests. Current files are authoritative; stored checkpoint memory can guide resume planning but must not override changed files.\n\nLevel 8 target: move beyond an in-memory checkpoint by adding persistent delivery-attempt operations with a small operator-facing CLI/report surface and durable handoff memory.\n",
            spec.architecture_name
        ),
    )?;
    write_file(
        &root.join("PROJECT_MANIFEST.txt"),
        &format!("python-stdlib unittest {} existing-project live-level8 persistence cli report memory\n", spec.id),
    )?;
    write_file(
        &root.join(format!("src/{}/__init__.py", spec.package)),
        &format!(
            "\"\"\"{} live Level 8 probe package.\"\"\"\n",
            spec.architecture_name
        ),
    )?;
    write_file(
        &root.join(format!("src/{}/models.py", spec.package)),
        "from dataclasses import dataclass\nfrom typing import Mapping\n\n\n@dataclass(frozen=True)\nclass Event:\n    event_id: str\n    kind: str\n    payload: Mapping[str, str]\n\n\n@dataclass(frozen=True)\nclass RouteDecision:\n    event_id: str\n    destination: str\n    reason: str\n",
    )?;
    write_file(
        &root.join(format!("src/{}/router.py", spec.package)),
        &format!(
            "from {}.models import Event, RouteDecision\n\n\ndef route_event(event: Event) -> RouteDecision:\n    if event.kind == \"{}\":\n        return RouteDecision(event.event_id, \"{}\", \"primary-kind\")\n    if event.kind.startswith(\"{}.\"):\n        return RouteDecision(event.event_id, \"{}\", \"secondary-family\")\n    return RouteDecision(event.event_id, \"default\", \"fallback\")\n",
            spec.package,
            spec.primary_kind,
            spec.primary_destination,
            spec.secondary_prefix,
            spec.secondary_destination
        ),
    )?;
    write_file(
        &root.join(format!("src/{}/repository.py", spec.package)),
        &format!(
            "from {}.models import RouteDecision\n\n\nclass RouteHistory:\n    def __init__(self):\n        self._decisions = []\n\n    def record(self, decision: RouteDecision) -> RouteDecision:\n        self._decisions.append(decision)\n        return decision\n\n    def all(self):\n        return list(self._decisions)\n\n    def count_by_destination(self):\n        counts = {{}}\n        for decision in self._decisions:\n            counts[decision.destination] = counts.get(decision.destination, 0) + 1\n        return counts\n",
            spec.package
        ),
    )?;
    write_file(
        &root.join(format!("src/{}/service.py", spec.package)),
        &format!(
            "from {}.models import Event, RouteDecision\nfrom {}.repository import RouteHistory\nfrom {}.router import route_event\n\n\nclass RoutingService:\n    def __init__(self, history=None):\n        self.history = history or RouteHistory()\n\n    def route(self, event: Event) -> RouteDecision:\n        decision = route_event(event)\n        return self.history.record(decision)\n\n    def destination_counts(self):\n        return self.history.count_by_destination()\n",
            spec.package, spec.package, spec.package
        ),
    )?;
    write_file(
        &root.join(format!("src/{}/cli.py", spec.package)),
        &format!(
            "import argparse\nimport sys\n\nfrom {}.models import Event\nfrom {}.service import RoutingService\n\n\ndef build_parser():\n    parser = argparse.ArgumentParser(prog=\"{}\")\n    subcommands = parser.add_subparsers(dest=\"command\", required=True)\n    route = subcommands.add_parser(\"route\")\n    route.add_argument(\"event_id\")\n    route.add_argument(\"kind\")\n    return parser\n\n\ndef main(argv=None, stdout=None):\n    stdout = stdout or sys.stdout\n    args = build_parser().parse_args(argv)\n    if args.command == \"route\":\n        service = RoutingService()\n        decision = service.route(Event(args.event_id, args.kind, {{}}))\n        stdout.write(f\"{{decision.destination}}\\n\")\n        return 0\n    return 2\n\n\nif __name__ == \"__main__\":\n    raise SystemExit(main())\n",
            spec.package, spec.package, spec.package
        ),
    )?;
    write_file(
        &root.join("tests/test_baseline.py"),
        &format!(
            "import io\nimport unittest\n\nfrom {package}.cli import main\nfrom {package}.models import Event\nfrom {package}.repository import RouteHistory\nfrom {package}.router import route_event\nfrom {package}.service import RoutingService\n\n\nclass BaselineRoutingTest(unittest.TestCase):\n    def test_routes_primary_event(self):\n        event = Event(\"evt-1\", \"{primary_kind}\", {{\"source\": \"primary\"}})\n        self.assertEqual(route_event(event).destination, \"{primary_destination}\")\n\n    def test_routes_secondary_family(self):\n        event = Event(\"evt-2\", \"{secondary_prefix}.created\", {{\"source\": \"secondary\"}})\n        self.assertEqual(route_event(event).destination, \"{secondary_destination}\")\n\n    def test_routes_unknown_to_default(self):\n        event = Event(\"evt-3\", \"analytics.page.viewed\", {{}})\n        self.assertEqual(route_event(event).destination, \"default\")\n\n    def test_service_records_route_history(self):\n        history = RouteHistory()\n        service = RoutingService(history)\n        service.route(Event(\"evt-4\", \"{primary_kind}\", {{}}))\n        self.assertEqual(history.count_by_destination(), {{\"{primary_destination}\": 1}})\n\n    def test_route_cli_prints_destination(self):\n        output = io.StringIO()\n        exit_code = main([\"route\", \"evt-cli\", \"{primary_kind}\"], stdout=output)\n        self.assertEqual(exit_code, 0)\n        self.assertEqual(output.getvalue().strip(), \"{primary_destination}\")\n\n\nif __name__ == \"__main__\":\n    unittest.main()\n",
            package = spec.package,
            primary_kind = spec.primary_kind,
            primary_destination = spec.primary_destination,
            secondary_prefix = spec.secondary_prefix,
            secondary_destination = spec.secondary_destination
        ),
    )?;
    Ok(())
}

fn worker_prompt(job: &LiveLevel8Job) -> String {
    format!(
        "You are running a live Level 8 coding-memory resume probe. You are not alone in the broader codebase: do not revert or modify anything outside the assigned temp run directory. Your write ownership is limited to {project_root} and {receipts_root}.\n\nGoal: continue the existing local Python project by using current files plus stored checkpoint memory. Do not ask follow-up questions. Complete one coherent checkpoint slice that is harder than an in-memory ledger.\n\nEnvironment:\n- Project root: {project_root}\n- Python package: {package}\n- Isolated memory DB: {memory_db_path}\n- Resume token: {resume_token}\n- Prior memory row id: {prior_memory_row_id}\n- Expected new memory row id: {expected_new_memory_row_id}\n- Memory CLI command pattern: INFRING_MEMORY_DB_PATH={memory_db_path} cargo run --quiet --manifest-path /Users/jay/.openclaw/workspace/core/layer0/memory/Cargo.toml --bin memory-cli -- <command>\n- Validation command from project root: {validation_command}\n\nWorkflow requirements:\n1. Read the local project files first. Current files are authoritative.\n2. Retrieve checkpoint memory using the resume token and/or row id with the memory CLI.\n3. Decide the next checkpoint from local context plus memory.\n4. Implement checkpoint_002_persistent_delivery_operations in multiple files.\n5. The slice must add persistent delivery-attempt storage, preferably JSONL or another stdlib file format, not just an in-memory list.\n6. Add a DeliveryAttempt-style model or equivalent durable record with event id, destination, status/outcome, retryable classification, and timestamp or attempt id.\n7. Integrate the delivery attempt operations with the routing/service layer while preserving baseline route behavior and the existing `route` CLI command.\n8. Add an operator-facing CLI/report surface that can summarize attempts by destination and retryable failures, and include import/export or equivalent durable file round-trip behavior.\n9. Add regression tests for baseline preservation, persistence round trip, report summary, retryable failure detection, and CLI/report behavior.\n10. Run the validation command.\n11. Write a checkpoint receipt under {receipts_root}/checkpoint_002_handoff.json.\n12. Write a new checkpoint memory row to the isolated DB using the expected new memory row id and tags coding,checkpoint,resume,project_context. Include changed files, validation result, known risks, and recommended next checkpoint.\n\nCompletion guardrails:\n- Do not stop after only adding a model/repository or tests. That is an incomplete checkpoint.\n- Do not write a checkpoint receipt that lists pending service/router/CLI integration; continue implementing the integration instead.\n- The checkpoint is incomplete unless service integration, report summary, CLI import/export or durable round-trip behavior, regression tests, checkpoint receipt, and checkpoint memory write are all complete.\n- The checkpoint receipt must include completed_checkpoint=checkpoint_002_persistent_delivery_operations, changed_files with at least three source/test/CLI files, validation_summary or validation_results, known_risks, and recommended_next_checkpoint.\n\nFinal response should include: whether it passed, changed file paths, validation command/result, new memory row id, and any caveats. Do not commit anything.\n",
        project_root = job.project_root,
        receipts_root = job.receipts_root,
        package = job.package,
        memory_db_path = job.memory_db_path,
        resume_token = job.resume_token,
        prior_memory_row_id = job.prior_memory_row_id,
        expected_new_memory_row_id = job.expected_new_memory_row_id,
        validation_command = job.validation_command
    )
}

fn judge_live_attempt(job: &LiveLevel8Job) -> LiveLevel8AttemptJudge {
    let mut checks = Vec::new();
    let mut failures = Vec::new();
    if let Some(infra_failure) = classify_worker_infra_failure(job) {
        push_check(
            &mut checks,
            &mut failures,
            "worker_infra_failure",
            false,
            infra_failure,
        );
        return LiveLevel8AttemptJudge {
            attempt_id: job.attempt_id.clone(),
            ok: false,
            classification: "infra_failure",
            checks,
            failures,
        };
    }
    let project_root = PathBuf::from(&job.project_root);
    let receipt_path = PathBuf::from(&job.receipts_root).join("checkpoint_002_handoff.json");

    let validation = run_python_validation(&project_root);
    push_check(
        &mut checks,
        &mut failures,
        "validation_passes_after_live_worker",
        validation.ok,
        validation.detail,
    );

    let receipt = read_json_file(&receipt_path);
    push_check(
        &mut checks,
        &mut failures,
        "checkpoint_receipt_written",
        receipt.is_some(),
        receipt_path.display().to_string(),
    );
    if let Some(receipt) = &receipt {
        let completed_checkpoint = receipt
            .get("completed_checkpoint")
            .or_else(|| receipt.get("checkpoint"))
            .and_then(Value::as_str)
            .unwrap_or("missing_completed_checkpoint");
        push_check(
            &mut checks,
            &mut failures,
            "receipt_declares_level8_checkpoint",
            completed_checkpoint == "checkpoint_002_persistent_delivery_operations",
            completed_checkpoint.to_string(),
        );
        let changed_file_count = receipt
            .get("changed_files")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or_default();
        push_check(
            &mut checks,
            &mut failures,
            "receipt_declares_multi_file_subsystem_change",
            changed_file_count >= 3,
            format!("changed_file_count={changed_file_count}"),
        );
    }

    let evidence = collect_package_source(&project_root, &job.package);
    let lower = evidence.to_lowercase();
    push_check(
        &mut checks,
        &mut failures,
        "persistent_delivery_storage_present",
        lower.contains("json")
            && (lower.contains("jsonl") || lower.contains("open(") || lower.contains("write_text"))
            && (lower.contains("path") || lower.contains("file")),
        "source mentions json/jsonl plus file/path persistence".to_string(),
    );
    push_check(
        &mut checks,
        &mut failures,
        "delivery_attempt_model_present",
        lower.contains("deliveryattempt") || lower.contains("delivery_attempt"),
        "source contains delivery attempt record".to_string(),
    );
    push_check(
        &mut checks,
        &mut failures,
        "report_surface_present",
        lower.contains("report")
            && lower.contains("destination")
            && (lower.contains("retryable") || lower.contains("retry")),
        "source contains report, destination summary, and retryable signal".to_string(),
    );
    push_check(
        &mut checks,
        &mut failures,
        "cli_import_export_or_roundtrip_surface_present",
        lower.contains("argparse")
            && lower.contains("report")
            && (lower.contains("export")
                || lower.contains("import")
                || lower.contains("roundtrip")),
        "CLI source exposes report plus import/export or round-trip behavior".to_string(),
    );

    let bridge = CodingMemoryRuntimeBridge {
        workspace_root: workspace_root(),
        memory_db_path: PathBuf::from(&job.memory_db_path),
        session_id: format!("{}_judge", job.attempt_id),
    };
    let memory_get = bridge.get(&job.expected_new_memory_row_id);
    let memory_content = memory_get
        .payload
        .pointer("/row/content")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    push_check(
        &mut checks,
        &mut failures,
        "checkpoint_002_memory_row_written",
        memory_get.ok && memory_content.contains("checkpoint_002_persistent_delivery_operations"),
        format!("ok={}", memory_get.ok),
    );
    push_check(
        &mut checks,
        &mut failures,
        "memory_row_preserves_validation_result",
        memory_get.ok && memory_content.contains("\"status\"") && memory_content.contains("pass"),
        "memory row includes validation status".to_string(),
    );

    LiveLevel8AttemptJudge {
        attempt_id: job.attempt_id.clone(),
        ok: failures.is_empty(),
        classification: if failures.is_empty() {
            "pass"
        } else {
            "coding_failure"
        },
        checks,
        failures,
    }
}

fn classify_worker_infra_failure(job: &LiveLevel8Job) -> Option<String> {
    let run_root = PathBuf::from(&job.run_root);
    let batch_root = run_root.parent()?;
    let output_path = batch_root
        .join("agent_outputs")
        .join(format!("{}.json", job.attempt_id));
    let text = fs::read_to_string(&output_path).ok()?;
    classify_worker_infra_failure_text(&text)
}

fn classify_worker_infra_failure_text(text: &str) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    if lower.contains("502 bad gateway")
        || lower.contains("503 service unavailable")
        || lower.contains("504 gateway timeout")
        || lower.contains("bad gateway")
        || lower.contains("gateway timeout")
        || lower.contains("temporarily overloaded")
        || lower.contains("model is temporarily overloaded")
    {
        return Some("provider_overloaded".to_string());
    }
    if lower.contains("internal server error") {
        return Some("provider_internal_server_error".to_string());
    }
    if lower.contains("no space left on device")
        || lower.contains("failed to write")
            && (lower.contains("target/debug") || lower.contains("rustc"))
    {
        return Some("host_disk_exhausted".to_string());
    }
    if lower.contains("provider:ollama_run_timeout")
        || lower.contains("ollama_run_timeout:timeout_seconds")
        || lower.contains("provider:ollama_run_failed")
        || lower.contains("ollama_run_failed")
        || lower.contains("worker_spawn_failed")
    {
        return Some("provider_timeout_or_spawn_failure".to_string());
    }
    None
}

fn push_check(
    checks: &mut Vec<LiveLevel8Check>,
    failures: &mut Vec<String>,
    id: &'static str,
    ok: bool,
    detail: String,
) {
    if !ok {
        failures.push(id.to_string());
    }
    checks.push(LiveLevel8Check { id, ok, detail });
}

struct ValidationResult {
    ok: bool,
    detail: String,
}

fn run_python_validation(project_root: &Path) -> ValidationResult {
    let output = Command::new("python3")
        .arg("-m")
        .arg("unittest")
        .arg("discover")
        .arg("-s")
        .arg("tests")
        .env("PYTHONPATH", project_root.join("src"))
        .current_dir(project_root)
        .output();
    match output {
        Ok(output) => ValidationResult {
            ok: output.status.success(),
            detail: format!(
                "exit={:?} stdout={} stderr={}",
                output.status.code(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ),
        },
        Err(error) => ValidationResult {
            ok: false,
            detail: format!("python_validation_spawn_failed:{error}"),
        },
    }
}

fn collect_package_source(project_root: &Path, package: &str) -> String {
    let package_root = project_root.join("src").join(package);
    let mut out = String::new();
    collect_python_source(&package_root, &mut out);
    collect_python_source(&project_root.join("tests"), &mut out);
    out
}

fn collect_python_source(path: &Path, out: &mut String) {
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_python_source(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("py") {
            out.push_str(&read_to_string(&path));
            out.push('\n');
        }
    }
}

fn read_to_string(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

fn read_json_file(path: &Path) -> Option<Value> {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
}

fn write_file(path: &Path, content: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create_parent_failed:{}:{error}", parent.display()))?;
    }
    fs::write(path, content)
        .map_err(|error| format!("write_file_failed:{}:{error}", path.display()))
}
