#!/usr/bin/env python3
"""Run native Infring Level 1 live coding attempts.

This is the honest Level 1 scoreboard path for Infring itself. The older
level1_reference_runtime_trace.py remains an observational reference-system
trace harness; this script actually asks the native Infring workflow to mutate
a local temp project, then independently checks the result.
"""

from __future__ import annotations

import argparse
import datetime as dt
import importlib.util
import json
from pathlib import Path
from typing import Any


THIS_DIR = Path(__file__).resolve().parent
DEFAULT_MODEL = "qwen2.5-coder:7b"

_LEVEL_LIVE_SPEC = importlib.util.spec_from_file_location(
    "level_live_baseline", THIS_DIR / "level3_level4_live_baseline.py"
)
level_live = importlib.util.module_from_spec(_LEVEL_LIVE_SPEC)
assert _LEVEL_LIVE_SPEC.loader is not None
_LEVEL_LIVE_SPEC.loader.exec_module(level_live)


def utc_now() -> str:
    return dt.datetime.now(dt.timezone.utc).isoformat()


def level1_case() -> dict[str, Any]:
    return {
        "id": "level1_create_single_file",
        "level": 1,
        "initial_files": {},
        "validation_command": (
            "PYTHONPATH=. python3 - <<'PY'\n"
            "from hello import greet\n"
            "assert greet('Ada') == 'Hello, Ada!'\n"
            "PY"
        ),
        "semantic_probe": (
            "from hello import greet\n"
            "assert greet('Grace') == 'Hello, Grace!'\n"
        ),
        "prompt": lambda root: (
            f"Project root: {root}\n"
            "Create a hello.py file with a greet(name: str) -> str function that returns exactly "
            "\"Hello, {name}!\" for the provided name. Run this validation command from project root: "
            "PYTHONPATH=. python3 - <<'PY'\n"
            "from hello import greet\n"
            "assert greet('Ada') == 'Hello, Ada!'\n"
            "PY\n"
            "Then run this semantic probe command from project root: "
            "PYTHONPATH=. python3 .infring/semantic_probe.py. Do not commit."
        ),
        "expected_paths": ["hello.py"],
        "expected_markers": ["def greet", "Hello"],
    }


def run_system(system: str, job: dict[str, Any], model: str) -> dict[str, Any]:
    if system == "infring":
        return level_live.run_infring(job, model)
    return {"ok": False, "blocked": "level1_native_live_adapter_not_implemented", "wall_time_ms": None}


def build_report(attempts: list[dict[str, Any]], model: str, systems: list[str], timeout_seconds: int) -> dict[str, Any]:
    return {
        "harness_kind": "level1_native_live_baseline_v1",
        "generated_at": utc_now(),
        "status": "complete",
        "model": model,
        "systems": systems,
        "levels": [1],
        "attempt_timeout_seconds": timeout_seconds,
        "summary": level_live.summarize(attempts),
        "attempts": attempts,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--attempts", type=int, default=1)
    parser.add_argument("--model", default=DEFAULT_MODEL)
    parser.add_argument("--systems", default="infring")
    parser.add_argument("--attempt-timeout-seconds", type=int, default=75)
    parser.add_argument("--out", default="")
    args = parser.parse_args()

    systems = [system.strip() for system in args.systems.split(",") if system.strip()]
    level_live.RUN_TIMEOUT_OVERRIDE_SECONDS = args.attempt_timeout_seconds

    attempts: list[dict[str, Any]] = []
    for attempt_idx in range(args.attempts):
        for system in systems:
            job = level_live.seed_case(level1_case(), system)
            case_id = f"{job['case_id']}_{attempt_idx + 1:02d}"
            job["case_id"] = case_id
            print(
                json.dumps(
                    {
                        "event": "attempt_start",
                        "system": system,
                        "level": 1,
                        "case_id": case_id,
                        "attempt_index": attempt_idx + 1,
                        "at": utc_now(),
                    }
                ),
                flush=True,
            )
            run_result = run_system(system, job, args.model)
            attempt = level_live.judge(system, job, run_result)
            attempts.append(attempt)
            print(
                json.dumps(
                    {
                        "event": "attempt_end",
                        "system": system,
                        "level": 1,
                        "case_id": case_id,
                        "attempt_index": attempt_idx + 1,
                        "ok": attempt["ok"],
                        "failure_class": attempt["failure_class"],
                        "wall_time_ms": attempt["wall_time_ms"],
                        "time_to_first_mutation_ms": attempt["time_to_first_mutation_ms"],
                        "at": utc_now(),
                    }
                ),
                flush=True,
            )

    report = build_report(attempts, args.model, systems, args.attempt_timeout_seconds)
    if args.out:
        out_path = Path(args.out)
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0 if all(attempt["ok"] for attempt in attempts) else 1


if __name__ == "__main__":
    raise SystemExit(main())
