#!/usr/bin/env python3
"""Run a weak-model Level 2 baseline across local coding-agent systems.

This is a measurement harness, not production runtime. It uses the same seeded
Native Coding Useful-Work Level 2 fixtures and the same local model label across
systems so we can separate model weakness from workflow/runtime weakness.

Runnable adapters produce live attempts. Non-runnable systems are reported as
blocked with evidence rather than silently treated as failures or successes.
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import shutil
import subprocess
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any

from command_resolution import command_execution_policy, resolve_forge_command, resolve_xtask_command


DEFAULT_MODEL = "qwen2.5-coder:7b"
LEVEL2_BUDGET_MS = 180_000
LEVEL2_FAST_BUDGET_MS = 90_000


def utc_now() -> str:
    return dt.datetime.now(dt.timezone.utc).isoformat()


def run_cmd(
    command: list[str],
    *,
    cwd: Path,
    timeout: int = 120,
    env: dict[str, str] | None = None,
) -> dict[str, Any]:
    started = time.monotonic()
    try:
        completed = subprocess.run(
            command,
            cwd=str(cwd),
            env=env,
            text=True,
            capture_output=True,
            timeout=timeout,
            check=False,
        )
        return {
            "ok": completed.returncode == 0,
            "returncode": completed.returncode,
            "stdout": completed.stdout,
            "stderr": completed.stderr,
            "stdout_tail": completed.stdout[-4000:],
            "stderr_tail": completed.stderr[-4000:],
            "timed_out": False,
            "wall_time_ms": round((time.monotonic() - started) * 1000),
        }
    except subprocess.TimeoutExpired as exc:
        return {
            "ok": False,
            "returncode": None,
            "stdout": exc.stdout or "" if isinstance(exc.stdout, str) else "",
            "stderr": exc.stderr or "" if isinstance(exc.stderr, str) else "",
            "stdout_tail": (exc.stdout or "")[-4000:] if isinstance(exc.stdout, str) else "",
            "stderr_tail": (exc.stderr or "")[-4000:] if isinstance(exc.stderr, str) else "",
            "timed_out": True,
            "wall_time_ms": round((time.monotonic() - started) * 1000),
        }
    except OSError as exc:
        return {
            "ok": False,
            "returncode": None,
            "stdout": "",
            "stderr": str(exc),
            "stdout_tail": "",
            "stderr_tail": str(exc),
            "timed_out": False,
            "wall_time_ms": round((time.monotonic() - started) * 1000),
        }


def repo_root_from_script() -> Path:
    return Path(__file__).resolve().parents[3]


def parse_last_json_object(text: str, *, preferred_key: str | None = None) -> dict[str, Any]:
    decoder = json.JSONDecoder()
    last: dict[str, Any] | None = None
    preferred: dict[str, Any] | None = None
    for index, char in enumerate(text):
        if char != "{":
            continue
        try:
            value, _ = decoder.raw_decode(text[index:])
        except json.JSONDecodeError:
            continue
        if isinstance(value, dict):
            last = value
            if preferred_key and preferred_key in value:
                preferred = value
    if preferred is not None:
        return preferred
    if last is None:
        raise json.JSONDecodeError("no JSON object found", text, 0)
    return last


def seed_level2_batch(repo_root: Path, attempts: int) -> dict[str, Any]:
    result = run_cmd(
        [
            "cargo",
            "run",
            "--quiet",
            "--manifest-path",
            "orchestration/Cargo.toml",
            "--bin",
            "native_coding_useful_work_eval_execute",
            "seed",
            f"--attempts={attempts}",
        ],
        cwd=repo_root,
        timeout=120,
    )
    if not result["ok"]:
        raise RuntimeError(f"seed failed: {result}")
    return parse_last_json_object(result.get("stdout") or result["stdout_tail"], preferred_key="batch_root")


def load_jobs(batch_root: Path) -> list[dict[str, Any]]:
    return json.loads((batch_root / "jobs.json").read_text())["jobs"]


def copy_batch_for_system(source_batch: Path, system: str) -> Path:
    target = source_batch.parent / f"{source_batch.name}-{system}"
    if target.exists():
        shutil.rmtree(target)
    shutil.copytree(source_batch, target)
    return target


def rewrite_jobs_root(batch_root: Path) -> None:
    jobs_path = batch_root / "jobs.json"
    report = json.loads(jobs_path.read_text())
    for job in report["jobs"]:
        old_run_root = job["run_root"]
        old_project_root = job["project_root"]
        old_prompt_path = job["prompt_path"]
        old_attempt = Path(job["run_root"]).name
        run_root = batch_root / old_attempt
        project_root = run_root / "project"
        prompt_path = batch_root / "prompts" / f"{job['attempt_id']}.txt"
        job["run_root"] = str(run_root)
        job["project_root"] = str(project_root)
        job["prompt_path"] = str(prompt_path)
        if prompt_path.exists():
            prompt = prompt_path.read_text()
            prompt = prompt.replace(old_project_root, str(project_root))
            prompt = prompt.replace(old_run_root, str(run_root))
            prompt = prompt.replace(old_prompt_path, str(prompt_path))
            prompt_path.write_text(prompt)
    report["batch_root"] = str(batch_root)
    jobs_path.write_text(json.dumps(report, indent=2))


def read_text(path: Path) -> str:
    try:
        return path.read_text()
    except OSError:
        return ""


def adapted_prompt(job: dict[str, Any]) -> str:
    raw = read_text(Path(job["prompt_path"]))
    raw = raw.replace(
        "Use the Infring native coding workflow and tools, not a simulated Codex worker.",
        "Use this coding agent's local shell and file-editing capabilities.",
    )
    raw = raw.replace(
        "Final response must list changed files, validation command/result, semantic probe result, caveats, and receipt-backed evidence.",
        "Final response must list changed files, validation command/result, semantic probe result, and caveats.",
    )
    return raw


def parse_unittest_count(detail: str) -> int:
    marker = "Ran "
    idx = detail.find(marker)
    if idx < 0:
        return 0
    tail = detail[idx + len(marker) :]
    digits = []
    for ch in tail:
        if not ch.isdigit():
            break
        digits.append(ch)
    return int("".join(digits) or "0")


def run_validation(project_root: Path, command: str) -> dict[str, Any]:
    return run_cmd(["sh", "-c", command], cwd=project_root, timeout=30, env=os.environ | {"PYTHONPATH": "src"})


def run_semantic_probe(project_root: Path) -> dict[str, Any]:
    return run_cmd(
        ["sh", "-c", "PYTHONPATH=src python3 .infring/semantic_probe.py"],
        cwd=project_root,
        timeout=30,
    )


def read_project_text(project_root: Path, package: str) -> str:
    parts: list[str] = []
    for root in [project_root / "src" / package, project_root / "tests"]:
        if not root.exists():
            continue
        for path in sorted(root.rglob("*.py")):
            parts.append(read_text(path))
    return "\n".join(parts)


def modified_py_after_seed(job: dict[str, Any]) -> tuple[bool, list[str]]:
    seed_ms = int(job.get("seed_completed_at_unix_ms") or 0)
    project_root = Path(job["project_root"])
    changed: list[str] = []
    for root in [project_root / "src", project_root / "tests"]:
        if not root.exists():
            continue
        for path in sorted(root.rglob("*.py")):
            mtime_ms = round(path.stat().st_mtime * 1000)
            if mtime_ms > seed_ms:
                changed.append(str(path))
    return bool(changed), changed


def time_to_first_py_mutation_after_seed(job: dict[str, Any]) -> int | None:
    seed_ms = int(job.get("seed_completed_at_unix_ms") or 0)
    project_root = Path(job["project_root"])
    first_ms: int | None = None
    for root in [project_root / "src", project_root / "tests"]:
        if not root.exists():
            continue
        for path in sorted(root.rglob("*.py")):
            mtime_ms = round(path.stat().st_mtime * 1000)
            if mtime_ms > seed_ms and (first_ms is None or mtime_ms < first_ms):
                first_ms = mtime_ms
    if first_ms is None:
        return None
    return max(0, first_ms - seed_ms)


def extract_infring_control_metrics(parsed: dict[str, Any], wall_time_ms: int) -> dict[str, Any]:
    receipt = parsed.get("receipt") if isinstance(parsed.get("receipt"), dict) else {}
    contract = parsed.get("contract") if isinstance(parsed.get("contract"), dict) else {}
    trace_summary = parsed.get("trace_summary") if isinstance(parsed.get("trace_summary"), dict) else {}
    native_receipts = receipt.get("native_tool_receipts")
    if not isinstance(native_receipts, list):
        native_receipts = []
    phase_latency = receipt.get("phase_latency_ms")
    if not isinstance(phase_latency, dict):
        phase_latency = trace_summary.get("phase_latency_ms") if isinstance(trace_summary.get("phase_latency_ms"), dict) else {}
    receipt_call_ids = [str(item.get("call_id", "")) for item in native_receipts if isinstance(item, dict)]
    repair_receipt_count = sum(1 for call_id in receipt_call_ids if "repair" in call_id)
    validation_command_count = sum(
        1
        for item in native_receipts
        if isinstance(item, dict) and item.get("tool_name") == "command_run"
    )
    mutation_receipt_count = sum(
        1
        for item in native_receipts
        if isinstance(item, dict) and item.get("tool_name") in {"file_write", "file_patch"}
    )
    observed_models = sorted(
        {
            str(model)
            for model in [contract.get("planner_model"), receipt.get("planner_model")]
            if model
        }
    )
    model_latency_ms = phase_latency.get("model_call") if isinstance(phase_latency.get("model_call"), int) else None
    return {
        "wall_time_ms": wall_time_ms,
        "phase_latency_ms": phase_latency,
        "native_tool_call_count": receipt.get("native_tool_call_count") or len(native_receipts),
        "mutation_receipt_count": mutation_receipt_count,
        "validation_command_count": validation_command_count,
        "repair_receipt_count": repair_receipt_count,
        "repair_loop_count_estimate": 1 if repair_receipt_count else 0,
        "model_call_count_estimate": 1 + (1 if repair_receipt_count else 0),
        "time_to_first_mutation_ms_estimate": model_latency_ms if mutation_receipt_count else None,
        "planner_models_observed": observed_models,
    }


def judge_system_attempt(system: str, job: dict[str, Any], run_result: dict[str, Any]) -> dict[str, Any]:
    project_root = Path(job["project_root"])
    validation = run_validation(project_root, job["validation_command"])
    semantic = run_semantic_probe(project_root)
    observed_tests = parse_unittest_count(f"{validation.get('stdout_tail','')}\n{validation.get('stderr_tail','')}")
    combined = read_project_text(project_root, job["package"])
    missing_symbols = [symbol for symbol in job["expected_symbols"] if symbol not in combined]
    mutated, changed_files = modified_py_after_seed(job)
    strict_model_lock_model = run_result.get("strict_model_lock_model")
    control_metrics = run_result.get("control_metrics") if isinstance(run_result.get("control_metrics"), dict) else {}
    observed_models = control_metrics.get("planner_models_observed") if isinstance(control_metrics.get("planner_models_observed"), list) else []
    checks = [
        {"id": "agent_run_completed", "ok": run_result.get("ok") is True, "detail": run_result.get("error") or ""},
        {
            "id": "strict_model_lock_observed",
            "ok": (
                not strict_model_lock_model
                or all(model == strict_model_lock_model for model in observed_models)
                and (bool(observed_models) or run_result.get("ok") is not True)
            ),
            "detail": {"expected": strict_model_lock_model, "observed": observed_models},
        },
        {"id": "validation_passes_after_worker", "ok": validation["ok"], "detail": validation},
        {
            "id": "new_regression_tests_exercised",
            "ok": observed_tests > int(job["baseline_test_count"]),
            "detail": {"observed_test_count": observed_tests, "baseline_test_count": job["baseline_test_count"]},
        },
        {"id": "expected_symbols_present", "ok": not missing_symbols, "detail": {"missing_symbols": missing_symbols}},
        {"id": "semantic_probe_passes", "ok": semantic["ok"], "detail": semantic},
        {"id": "source_or_test_mutated_after_seed", "ok": mutated, "detail": {"changed_files": changed_files}},
        {
            "id": "worker_runtime_within_level2_budget",
            "ok": (run_result.get("wall_time_ms") or 10**12) <= LEVEL2_BUDGET_MS,
            "detail": {"wall_time_ms": run_result.get("wall_time_ms"), "budget_ms": LEVEL2_BUDGET_MS},
        },
        {
            "id": "worker_runtime_within_level2_fast_budget",
            "ok": (run_result.get("wall_time_ms") or 10**12) <= LEVEL2_FAST_BUDGET_MS,
            "detail": {"wall_time_ms": run_result.get("wall_time_ms"), "budget_ms": LEVEL2_FAST_BUDGET_MS},
        },
    ]
    failures = [check["id"] for check in checks if not check["ok"]]
    return {
        "system": system,
        "attempt_id": job["attempt_id"],
        "task_id": job["task_id"],
        "ok": not failures,
        "wall_time_ms": run_result.get("wall_time_ms"),
        "time_to_first_mutation_ms": run_result.get("time_to_first_mutation_ms"),
        "control_metrics": control_metrics,
        "failure_class": classify_failure(failures, run_result),
        "checks": checks,
        "failures": failures,
        "run_result": run_result,
    }


def classify_failure(failures: list[str], run_result: dict[str, Any]) -> str | None:
    if not failures:
        return None
    if run_result.get("blocked"):
        return str(run_result.get("blocked"))
    if run_result.get("timed_out"):
        return "runtime_timeout"
    if "strict_model_lock_observed" in failures:
        return "model_lock_violation"
    if "source_or_test_mutated_after_seed" in failures:
        return "no_successful_mutation"
    if "validation_passes_after_worker" in failures:
        return "validation_failed"
    if "semantic_probe_passes" in failures or "expected_symbols_present" in failures:
        return "semantic_or_public_interface_failed"
    if "new_regression_tests_exercised" in failures:
        return "missing_regression_test_evidence"
    if "worker_runtime_within_level2_fast_budget" in failures or "worker_runtime_within_level2_budget" in failures:
        return "latency_budget_exceeded"
    return "unknown_failure"


def run_infring(repo_root: Path, batch_root: Path, model: str, *, strict_model_lock: bool) -> list[dict[str, Any]]:
    jobs = load_jobs(batch_root)
    outputs = batch_root / "system_outputs" / "infring"
    outputs.mkdir(parents=True, exist_ok=True)
    attempts: list[dict[str, Any]] = []
    for job in jobs:
        started = time.monotonic()
        out_path = outputs / f"{job['attempt_id']}.json"
        env = os.environ.copy()
        if strict_model_lock:
            env["INFRING_RUNTIME_LANE_MODEL_LOCK"] = model
        resolution = resolve_xtask_command(repo_root, policy=command_execution_policy())
        if not resolution["ok"]:
            wall_ms = round((time.monotonic() - started) * 1000)
            run_result = {
                "ok": False,
                "blocked": resolution.get("blocked"),
                "wall_time_ms": wall_ms,
                "model": model,
                "strict_model_lock_model": model if strict_model_lock else None,
                "strict_model_lock": strict_model_lock,
                "fairness_note": "Strict model lock forces runtime lane planner and repair planner calls to the requested baseline model when enabled.",
                "control_metrics": {"wall_time_ms": wall_ms},
                "command_resolution": resolution["receipt"],
                "execution_mode": resolution["receipt"]["execution_mode"],
                "timing_comparable": resolution["receipt"]["timing_comparable"],
                "raw_command_ok": False,
                "error": resolution.get("blocked"),
                "output_path": str(out_path),
            }
            attempts.append(judge_system_attempt("infring", job, run_result))
            continue
        command = list(resolution["command"])
        command.extend(
            [
                "--workflow=coding_project_operator",
                f"--name=weak-baseline-{job['attempt_id']}",
                f"--prompt=@{job['prompt_path']}",
                "--provider=ollama",
                f"--model={model}",
                "--permissions-template=admin",
                "--pack=local-coding-files",
                "--tool=file_list,file_stat,file_read,file_read_many,file_write,file_patch,command_resolve,command_run",
            ]
        )
        result = run_cmd(
            command,
            cwd=repo_root,
            timeout=240,
            env=env,
        )
        out_path.write_text(result.get("stdout") or result["stdout_tail"])
        wall_ms = round((time.monotonic() - started) * 1000)
        parsed = {}
        try:
            parsed = json.loads(result.get("stdout") or result["stdout_tail"])
        except json.JSONDecodeError:
            pass
        run_result = {
            "ok": result["ok"] and parsed.get("ok") is True,
            "wall_time_ms": wall_ms,
            "model": model,
            "strict_model_lock_model": model if strict_model_lock else None,
            "strict_model_lock": strict_model_lock,
            "fairness_note": "Strict model lock forces runtime lane planner and repair planner calls to the requested baseline model when enabled.",
            "control_metrics": extract_infring_control_metrics(parsed, wall_ms),
            "command_resolution": resolution["receipt"],
            "execution_mode": resolution["receipt"]["execution_mode"],
            "timing_comparable": resolution["receipt"]["timing_comparable"],
            "raw_command_ok": result["ok"],
            "error": parsed.get("error") or result.get("stderr_tail"),
            "output_path": str(out_path),
        }
        attempts.append(judge_system_attempt("infring", job, run_result))
    return attempts


class OllamaJsonModel:
    def __init__(self, model: str, endpoint: str = "http://127.0.0.1:11434/api/chat") -> None:
        self.model = model
        self.endpoint = endpoint
        self.calls = 0

    def get_template_vars(self) -> dict[str, Any]:
        return {"model": self.model}

    def format_message(self, role: str, content: str, extra: dict[str, Any] | None = None) -> dict[str, Any]:
        return {"role": role, "content": content, "extra": extra or {}}

    def query(self, messages: list[dict[str, Any]]) -> dict[str, Any]:
        self.calls += 1
        payload = {
            "model": self.model,
            "stream": False,
            "format": "json",
            "options": {"temperature": 0},
            "messages": [
                {"role": msg.get("role", "user") if msg.get("role") in {"system", "user", "assistant"} else "user", "content": msg.get("content", "")}
                for msg in messages
                if msg.get("role") != "exit"
            ],
        }
        request = urllib.request.Request(
            self.endpoint,
            data=json.dumps(payload).encode("utf-8"),
            headers={"Content-Type": "application/json"},
            method="POST",
        )
        try:
            with urllib.request.urlopen(request, timeout=60) as response:
                raw = response.read().decode("utf-8", errors="replace")
        except (urllib.error.URLError, TimeoutError) as exc:
            return self.format_message("exit", f"provider_error:{exc}", {"exit_status": "provider_error", "submission": ""})
        try:
            content = json.loads(raw).get("message", {}).get("content", "{}")
            parsed = json.loads(content)
        except json.JSONDecodeError:
            parsed = {"finish": "invalid_json_model_output"}
            content = raw[-2000:]
        if parsed.get("finish"):
            return self.format_message(
                "exit",
                str(parsed.get("finish")),
                {"exit_status": "Submitted", "submission": str(parsed.get("finish")), "cost": 0.0},
            )
        actions = parsed.get("actions") if isinstance(parsed.get("actions"), list) else []
        normalized = [
            {"command": str(action.get("command", ""))}
            for action in actions
            if isinstance(action, dict) and action.get("command")
        ]
        return self.format_message("assistant", content, {"actions": normalized, "cost": 0.0})

    def format_observation_messages(
        self,
        message: dict[str, Any],
        outputs: list[dict[str, Any]],
        template_vars: dict[str, Any],
    ) -> list[dict[str, Any]]:
        return [self.format_message("user", "OBSERVATION:\n" + json.dumps(outputs, indent=2)[-8000:])]

    def serialize(self) -> dict[str, Any]:
        return {"model_probe": {"model": self.model, "calls": self.calls}}


def run_mini_swe_agent(repo_root: Path, batch_root: Path, model: str) -> list[dict[str, Any]]:
    mini_root = repo_root / "references/coding-agent-systems/mini-swe-agent"
    if not mini_root.exists():
        return blocked_attempts("mini-swe-agent", batch_root, "repo_missing")
    sys.path.insert(0, str(mini_root / "src"))
    try:
        from minisweagent.agents.default import DefaultAgent
        from minisweagent.environments.local import LocalEnvironment
    except Exception as exc:  # noqa: BLE001
        return blocked_attempts("mini-swe-agent", batch_root, f"import_failed:{type(exc).__name__}:{exc}")

    jobs = load_jobs(batch_root)
    outputs = batch_root / "system_outputs" / "mini-swe-agent"
    outputs.mkdir(parents=True, exist_ok=True)
    attempts: list[dict[str, Any]] = []
    system_template = (
        "You are a local coding agent. Return only JSON.\n"
        "Use shell commands to inspect, edit, and validate the project.\n"
        "JSON schema: {\"actions\":[{\"command\":\"shell command\"}]} or {\"finish\":\"summary\"}.\n"
        "Do one or two useful commands per turn. Read files before editing existing behavior.\n"
        "Use python heredocs for multi-line writes. Do not commit. Stop after validation and semantic probe pass.\n"
    )
    instance_template = "{{task}}"
    for job in jobs:
        started = time.monotonic()
        trajectory_path = outputs / f"{job['attempt_id']}.trajectory.json"
        project_root = Path(job["project_root"])
        model_adapter = OllamaJsonModel(model)
        env = LocalEnvironment(cwd=str(project_root), timeout=30)
        agent = DefaultAgent(
            model_adapter,
            env,
            system_template=system_template,
            instance_template=instance_template,
            step_limit=8,
            cost_limit=0,
            output_path=trajectory_path,
        )
        try:
            result = agent.run(adapted_prompt(job))
            ok = result.get("exit_status") == "Submitted"
            error = None
        except Exception as exc:  # noqa: BLE001
            ok = False
            error = f"{type(exc).__name__}:{exc}"
        wall_ms = round((time.monotonic() - started) * 1000)
        run_result = {
            "ok": ok,
            "wall_time_ms": wall_ms,
            "model": model,
            "error": error,
            "trajectory_path": str(trajectory_path),
        }
        attempts.append(judge_system_attempt("mini-swe-agent", job, run_result))
    return attempts


def run_aider(repo_root: Path, batch_root: Path, model: str) -> list[dict[str, Any]]:
    aider_bin = Path("/tmp/infring-baselines-aider/bin/aider")
    if not aider_bin.exists():
        return blocked_attempts("aider", batch_root, "temp_venv_missing:/tmp/infring-baselines-aider")
    jobs = load_jobs(batch_root)
    outputs = batch_root / "system_outputs" / "aider"
    outputs.mkdir(parents=True, exist_ok=True)
    attempts: list[dict[str, Any]] = []
    for job in jobs:
        started = time.monotonic()
        project_root = Path(job["project_root"])
        prompt_path = outputs / f"{job['attempt_id']}.prompt.txt"
        stdout_path = outputs / f"{job['attempt_id']}.stdout.txt"
        prompt_path.write_text(
            adapted_prompt(job)
            + "\n\nUse the existing project files in the current working directory."
            + f"\nRun this validation command before final response: {job['validation_command']}"
            + "\nDo not commit. Preserve existing public import paths and owner modules."
        )
        command = [
            str(aider_bin),
            "--model",
            f"ollama_chat/{model}",
            "--edit-format",
            "diff",
            "--message-file",
            str(prompt_path),
            "--yes-always",
            "--no-git",
            "--no-auto-commits",
            "--no-pretty",
            "--no-stream",
            "--analytics-disable",
            "--timeout",
            "60",
            "--no-show-model-warnings",
            "--no-check-model-accepts-settings",
            "--test-cmd",
            job["validation_command"],
            "--auto-test",
        ]
        for path in aider_seed_files(project_root):
            command.extend(["--file", str(path.relative_to(project_root))])
        result = run_cmd(
            command,
            cwd=project_root,
            timeout=240,
            env=os.environ | {"PYTHONPATH": "src"},
        )
        stdout_path.write_text((result.get("stdout") or "") + "\n\nSTDERR:\n" + (result.get("stderr") or ""))
        wall_ms = round((time.monotonic() - started) * 1000)
        run_result = {
            "ok": result["ok"],
            "wall_time_ms": wall_ms,
            "model": model,
            "error": result.get("stderr_tail") if not result["ok"] else None,
            "stdout_path": str(stdout_path),
        }
        attempts.append(judge_system_attempt("aider", job, run_result))
    return attempts


def aider_seed_files(project_root: Path) -> list[Path]:
    roots = [project_root / "src", project_root / "tests"]
    files: list[Path] = []
    for root in roots:
        if root.exists():
            files.extend(sorted(root.rglob("*.py")))
    return files[:24]


def run_swe_agent(repo_root: Path, batch_root: Path, model: str) -> list[dict[str, Any]]:
    sweagent_bin = Path("/tmp/infring-baselines-sweagent/bin/sweagent")
    if not sweagent_bin.exists():
        return blocked_attempts("swe-agent", batch_root, "temp_venv_missing:/tmp/infring-baselines-sweagent")
    jobs = load_jobs(batch_root)
    outputs = batch_root / "system_outputs" / "swe-agent"
    outputs.mkdir(parents=True, exist_ok=True)
    attempts: list[dict[str, Any]] = []
    config_path = repo_root / "references/coding-agent-systems/swe-agent/config/coding_challenge.yaml"
    for job in jobs:
        started = time.monotonic()
        project_root = Path(job["project_root"])
        git_init = ensure_temp_git_repo(project_root)
        problem_path = outputs / f"{job['attempt_id']}.problem.md"
        output_dir = outputs / f"{job['attempt_id']}.run"
        stdout_path = outputs / f"{job['attempt_id']}.stdout.txt"
        local_root = outputs / "swe-agent-local-root" / job["attempt_id"]
        shim_path = repo_root / "references/coding-agent-systems/runtime_trace_harness/swe_agent_local_root_sitecustomize"
        problem_path.write_text(
            adapted_prompt(job)
            + "\n\nWork in the existing local repository."
            + f"\nRun this validation command before final response: {job['validation_command']}"
            + "\nDo not commit. Preserve existing public import paths and owner modules."
        )
        result = run_cmd(
            [
                str(sweagent_bin),
                "run",
                "--config",
                str(config_path),
                f"--problem_statement.path={problem_path}",
                "--env.deployment.type=local",
                "--env.repo.type=preexisting",
                f"--env.repo.repo_name={str(project_root).lstrip('/')}",
                f"--agent.model.name=ollama/{model}",
                "--agent.model.api_base=http://localhost:11434",
                "--agent.model.per_instance_cost_limit=0",
                "--agent.model.total_cost_limit=0",
                "--agent.model.per_instance_call_limit=8",
                "--actions.apply_patch_locally=True",
                f"--output_dir={output_dir}",
            ],
            cwd=repo_root / "references/coding-agent-systems/swe-agent",
            timeout=300,
            env=os.environ
            | {
                "PYTHONPATH": f"{shim_path}:src",
                "SWE_AGENT_LOCAL_ROOT": str(local_root),
            },
        )
        stdout_path.write_text((result.get("stdout") or "") + "\n\nSTDERR:\n" + (result.get("stderr") or ""))
        wall_ms = round((time.monotonic() - started) * 1000)
        run_result = {
            "ok": result["ok"],
            "wall_time_ms": wall_ms,
            "model": model,
            "adapter_preconditions": {
                "git_repo_initialized": git_init,
                "local_root_shim": str(local_root),
            },
            "error": result.get("stderr_tail") if not result["ok"] else None,
            "stdout_path": str(stdout_path),
            "output_dir": str(output_dir),
        }
        attempts.append(judge_system_attempt("swe-agent", job, run_result))
    return attempts


def ensure_temp_git_repo(project_root: Path) -> bool:
    if (project_root / ".git").exists():
        return True
    init = run_cmd(["git", "init"], cwd=project_root, timeout=10)
    if not init["ok"]:
        return False
    run_cmd(["git", "config", "user.email", "infring-baseline@example.invalid"], cwd=project_root, timeout=10)
    run_cmd(["git", "config", "user.name", "Infring Baseline"], cwd=project_root, timeout=10)
    add = run_cmd(["git", "add", "."], cwd=project_root, timeout=20)
    if not add["ok"]:
        return False
    commit = run_cmd(["git", "commit", "-m", "seed fixture"], cwd=project_root, timeout=30)
    return bool(commit["ok"])


def run_forgecode(repo_root: Path, batch_root: Path, model: str) -> list[dict[str, Any]]:
    forge_root = repo_root / "references/coding-agent-systems/forgecode"
    if not (forge_root / "Cargo.toml").exists():
        return blocked_attempts("forgecode", batch_root, "repo_missing:references/coding-agent-systems/forgecode")
    jobs = load_jobs(batch_root)
    outputs = batch_root / "system_outputs" / "forgecode"
    outputs.mkdir(parents=True, exist_ok=True)
    attempts: list[dict[str, Any]] = []
    for job in jobs:
        started = time.monotonic()
        project_root = Path(job["project_root"])
        prompt_path = outputs / f"{job['attempt_id']}.prompt.txt"
        stdout_path = outputs / f"{job['attempt_id']}.stdout.txt"
        config_root = outputs / "forge-config" / job["attempt_id"]
        debug_requests = outputs / "forge-debug-requests" / f"{job['attempt_id']}.json"
        config_root.mkdir(parents=True, exist_ok=True)
        debug_requests.parent.mkdir(parents=True, exist_ok=True)
        (config_root / ".forge.toml").write_text(
            "\n".join(
                [
                    "[session]",
                    "provider_id = \"openai_compatible\"",
                    f"model_id = \"{model}\"",
                    "",
                    "[reasoning]",
                    "enabled = false",
                    "",
                ]
            )
        )
        prompt_path.write_text(
            adapted_prompt(job)
            + "\n\nUse ForgeCode one-shot local coding mode in the current project."
            + f"\nRun this validation command before final response: {job['validation_command']}"
            + "\nDo not commit. Preserve existing public import paths and owner modules."
        )
        resolution = resolve_forge_command(repo_root, forge_root, policy=command_execution_policy())
        if not resolution["ok"]:
            wall_ms = round((time.monotonic() - started) * 1000)
            run_result = {
                "ok": False,
                "blocked": resolution.get("blocked"),
                "wall_time_ms": wall_ms,
                "time_to_first_mutation_ms": time_to_first_py_mutation_after_seed(job),
                "model": model,
                "provider": "openai_compatible",
                "openai_url": "http://127.0.0.1:11434/v1",
                "control_metrics": {
                    "wall_time_ms": wall_ms,
                    "time_to_first_mutation_ms": time_to_first_py_mutation_after_seed(job),
                },
                "command_resolution": resolution["receipt"],
                "execution_mode": resolution["receipt"]["execution_mode"],
                "timing_comparable": resolution["receipt"]["timing_comparable"],
                "error": resolution.get("blocked"),
                "stdout_path": str(stdout_path),
                "prompt_path": str(prompt_path),
                "config_root": str(config_root),
                "debug_requests_path": str(debug_requests),
                "debug_request_files": [],
            }
            attempts.append(judge_system_attempt("forgecode", job, run_result))
            continue
        result = run_cmd(
            [
                *resolution["command"],
                "-C",
                str(project_root),
                "-p",
                prompt_path.read_text(),
            ],
            cwd=forge_root,
            timeout=420,
            env=os.environ
            | {
                "FORGE_CONFIG": str(config_root),
                "FORGE_SESSION__PROVIDER_ID": "openai_compatible",
                "FORGE_SESSION__MODEL_ID": model,
                "FORGE_REASONING__ENABLED": "false",
                "FORGE_DEBUG_REQUESTS": str(debug_requests),
                "FORGE_DUMP_AUTO_OPEN": "false",
                "OPENAI_API_KEY": "ollama",
                "OPENAI_URL": "http://127.0.0.1:11434/v1",
                "PYTHONPATH": "src",
            },
        )
        stdout_path.write_text((result.get("stdout") or "") + "\n\nSTDERR:\n" + (result.get("stderr") or ""))
        wall_ms = round((time.monotonic() - started) * 1000)
        request_files = [str(debug_requests)] if debug_requests.exists() else []
        run_result = {
            "ok": result["ok"],
            "wall_time_ms": wall_ms,
            "time_to_first_mutation_ms": time_to_first_py_mutation_after_seed(job),
            "model": model,
            "provider": "openai_compatible",
            "openai_url": "http://127.0.0.1:11434/v1",
            "control_metrics": {
                "wall_time_ms": wall_ms,
                "debug_request_file_count": len(request_files),
                "time_to_first_mutation_ms": time_to_first_py_mutation_after_seed(job),
            },
            "command_resolution": resolution["receipt"],
            "execution_mode": resolution["receipt"]["execution_mode"],
            "timing_comparable": resolution["receipt"]["timing_comparable"],
            "error": result.get("stderr_tail") if not result["ok"] else None,
            "stdout_path": str(stdout_path),
            "prompt_path": str(prompt_path),
            "config_root": str(config_root),
            "debug_requests_path": str(debug_requests),
            "debug_request_files": request_files[:24],
        }
        attempts.append(judge_system_attempt("forgecode", job, run_result))
    return attempts


def blocked_attempts(system: str, batch_root: Path, reason: str) -> list[dict[str, Any]]:
    attempts: list[dict[str, Any]] = []
    for job in load_jobs(batch_root):
        attempts.append(
            {
                "system": system,
                "attempt_id": job["attempt_id"],
                "task_id": job["task_id"],
                "ok": False,
                "wall_time_ms": None,
                "time_to_first_mutation_ms": None,
                "failure_class": reason,
                "checks": [{"id": "runnable_adapter_available", "ok": False, "detail": reason}],
                "failures": ["runnable_adapter_available"],
                "run_result": {"ok": False, "blocked": reason},
            }
        )
    return attempts


def run_external_cli_placeholder(system: str, command_name: str, batch_root: Path) -> list[dict[str, Any]]:
    if shutil.which(command_name):
        return blocked_attempts(system, batch_root, f"adapter_not_implemented_for_available_cli:{command_name}")
    return blocked_attempts(system, batch_root, f"cli_missing:{command_name}")


def summarize(system_attempts: dict[str, list[dict[str, Any]]]) -> dict[str, Any]:
    systems = []
    for system, attempts in system_attempts.items():
        pass_count = sum(1 for attempt in attempts if attempt["ok"])
        fail_count = len(attempts) - pass_count
        failure_classes: dict[str, int] = {}
        for attempt in attempts:
            key = attempt.get("failure_class") or "pass"
            failure_classes[key] = failure_classes.get(key, 0) + 1
        metric_attempts = [
            attempt.get("control_metrics")
            for attempt in attempts
            if isinstance(attempt.get("control_metrics"), dict)
        ]
        systems.append(
            {
                "system": system,
                "attempt_count": len(attempts),
                "pass_count": pass_count,
                "fail_count": fail_count,
                "failure_classes": failure_classes,
                "average_wall_time_ms": (
                    round(sum(a["wall_time_ms"] for a in attempts if a.get("wall_time_ms")) / max(1, sum(1 for a in attempts if a.get("wall_time_ms"))))
                    if any(a.get("wall_time_ms") for a in attempts)
                    else None
                ),
                "average_native_tool_call_count": average_metric(metric_attempts, "native_tool_call_count"),
                "average_model_call_count_estimate": average_metric(metric_attempts, "model_call_count_estimate"),
                "average_validation_command_count": average_metric(metric_attempts, "validation_command_count"),
                "average_repair_loop_count_estimate": average_metric(metric_attempts, "repair_loop_count_estimate"),
            }
        )
    return {"systems": systems}


def average_metric(metrics: list[dict[str, Any]], key: str) -> int | None:
    values = [item.get(key) for item in metrics if isinstance(item.get(key), int)]
    if not values:
        return None
    return round(sum(values) / len(values))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--attempts", type=int, default=4)
    parser.add_argument("--model", default=DEFAULT_MODEL)
    parser.add_argument(
        "--systems",
        default="infring,mini-swe-agent,aider,swe-agent,goose,forgecode",
        help="Comma-separated systems: infring,mini-swe-agent,aider,swe-agent,goose,forgecode",
    )
    parser.add_argument("--out", default="")
    parser.add_argument("--allow-workflow-repair-model", action="store_true")
    args = parser.parse_args()

    repo_root = repo_root_from_script()
    seed = seed_level2_batch(repo_root, args.attempts)
    source_batch = Path(seed["batch_root"])
    requested = [item.strip() for item in args.systems.split(",") if item.strip()]
    system_attempts: dict[str, list[dict[str, Any]]] = {}
    system_batches: dict[str, str] = {}
    for system in requested:
        batch = copy_batch_for_system(source_batch, system.replace("/", "_"))
        rewrite_jobs_root(batch)
        system_batches[system] = str(batch)
        if system == "infring":
            system_attempts[system] = run_infring(
                repo_root,
                batch,
                args.model,
                strict_model_lock=not args.allow_workflow_repair_model,
            )
        elif system == "mini-swe-agent":
            system_attempts[system] = run_mini_swe_agent(repo_root, batch, args.model)
        elif system == "aider":
            system_attempts[system] = run_aider(repo_root, batch, args.model)
        elif system == "swe-agent":
            system_attempts[system] = run_swe_agent(repo_root, batch, args.model)
        elif system == "goose":
            system_attempts[system] = run_external_cli_placeholder("goose", "goose", batch)
        elif system == "forgecode":
            system_attempts[system] = run_forgecode(repo_root, batch, args.model)
        else:
            system_attempts[system] = blocked_attempts(system, batch, "unknown_system")

    report = {
        "harness_kind": "level2_weak_model_live_baseline_v1",
        "generated_at": utc_now(),
        "model": args.model,
        "strict_model_lock": not args.allow_workflow_repair_model,
        "level2_fast_budget_ms": LEVEL2_FAST_BUDGET_MS,
        "source_batch_root": str(source_batch),
        "system_batches": system_batches,
        "summary": summarize(system_attempts),
        "attempts": system_attempts,
        "interpretation_rules": [
            "If all runnable systems fail with the same model, model capability is likely a primary bottleneck.",
            "If reference systems pass and Infring fails, Infring is missing runtime primitives or composition.",
            "If systems fail faster and cleaner, Infring needs better budget/failure control even if pass rate matches.",
            "Blocked systems are not counted as capability failures.",
        ],
    }
    out_path = Path(args.out) if args.out else source_batch / "level2_weak_model_live_baseline.json"
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(report, indent=2))
    print(json.dumps(report, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
