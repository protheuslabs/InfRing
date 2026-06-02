#!/usr/bin/env python3
"""Run the native Infring Levels 1-7 regression sweep.

This wrapper composes the primitive live harnesses without changing their
behavior:

- Level 1: native live one-file creation harness.
- Level 2: weak-model existing-behavior patch harness.
- Levels 3-7: live coding baseline harness.

The old Level 1 reference trace is intentionally not part of this scoreboard.
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import subprocess
import sys
import time
from pathlib import Path
from typing import Any


THIS_DIR = Path(__file__).resolve().parent
REPO_ROOT = THIS_DIR.parents[2]
REPORT_DIR = THIS_DIR / "reports"
DEFAULT_MODEL = "qwen2.5-coder:7b"


def utc_now() -> str:
    return dt.datetime.now(dt.timezone.utc).isoformat()


def safe_label(value: str) -> str:
    cleaned = "".join(ch if ch.isalnum() or ch in {"-", "_"} else "_" for ch in value)
    return cleaned.strip("_") or "run"


def run_child(name: str, command: list[str], report_path: Path) -> dict[str, Any]:
    started = time.monotonic()
    completed = subprocess.run(
        command,
        cwd=str(REPO_ROOT),
        text=True,
        capture_output=True,
        check=False,
    )
    if completed.stdout:
        print(completed.stdout, end="" if completed.stdout.endswith("\n") else "\n")
    if completed.stderr:
        print(completed.stderr, end="" if completed.stderr.endswith("\n") else "\n", file=sys.stderr)
    report: dict[str, Any] | None = None
    report_error: str | None = None
    if report_path.exists():
        try:
            report = json.loads(report_path.read_text(encoding="utf-8"))
        except json.JSONDecodeError as exc:
            report_error = f"json_decode_error:{exc}"
    else:
        report_error = "report_missing"
    attempt_fail_count = child_report_fail_count(report)
    return {
        "name": name,
        "ok": completed.returncode == 0 and report_error is None and attempt_fail_count == 0,
        "returncode": completed.returncode,
        "wall_time_ms": round((time.monotonic() - started) * 1000),
        "command": command,
        "report_path": str(report_path),
        "report_error": report_error,
        "attempt_fail_count": attempt_fail_count,
        "stdout_tail": completed.stdout[-4000:],
        "stderr_tail": completed.stderr[-4000:],
        "report": report,
    }


def child_report_fail_count(report: dict[str, Any] | None) -> int:
    if not isinstance(report, dict):
        return 1
    value = report.get("fail_count")
    if isinstance(value, int):
        return value
    total = 0
    attempts = report.get("attempts")
    if isinstance(attempts, list):
        return sum(1 for attempt in attempts if isinstance(attempt, dict) and attempt.get("ok") is not True)
    if isinstance(attempts, dict):
        for system_attempts in attempts.values():
            if isinstance(system_attempts, list):
                total += sum(1 for attempt in system_attempts if isinstance(attempt, dict) and attempt.get("ok") is not True)
        return total
    summary = report.get("summary")
    if isinstance(summary, list):
        return sum(
            int(item.get("fail_count") or 0)
            for item in summary
            if isinstance(item, dict)
        )
    if isinstance(summary, dict) and isinstance(summary.get("systems"), list):
        return sum(
            int(item.get("fail_count") or 0)
            for item in summary["systems"]
            if isinstance(item, dict)
        )
    return 0


def normalize_summary(child_results: list[dict[str, Any]]) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for child in child_results:
        report = child.get("report")
        if not isinstance(report, dict):
            continue
        if child["name"] == "level2":
            summary = report.get("summary")
            systems = summary.get("systems") if isinstance(summary, dict) else None
            if not isinstance(systems, list):
                continue
            for item in systems:
                if isinstance(item, dict):
                    row = dict(item)
                    row["level"] = 2
                    rows.append(row)
            continue
        summary = report.get("summary")
        if isinstance(summary, list):
            rows.extend(dict(item) for item in summary if isinstance(item, dict))
    return rows


def flatten_attempts(child_results: list[dict[str, Any]]) -> list[dict[str, Any]]:
    attempts: list[dict[str, Any]] = []
    for child in child_results:
        report = child.get("report")
        if not isinstance(report, dict):
            continue
        value = report.get("attempts")
        if isinstance(value, list):
            attempts.extend(dict(item) for item in value if isinstance(item, dict))
        elif isinstance(value, dict):
            for system_attempts in value.values():
                if isinstance(system_attempts, list):
                    attempts.extend(dict(item) for item in system_attempts if isinstance(item, dict))
    return attempts


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--model", default=DEFAULT_MODEL)
    parser.add_argument("--systems", default="infring")
    parser.add_argument("--level1-attempts", type=int, default=1)
    parser.add_argument("--level2-attempts", type=int, default=1)
    parser.add_argument("--levels3-7", default="3,4,5,6,7")
    parser.add_argument("--attempt-timeout-seconds", type=int, default=75)
    parser.add_argument("--out", default="")
    args = parser.parse_args()

    timestamp = dt.datetime.now(dt.timezone.utc).strftime("%Y%m%d_%H%M%S")
    run_label = safe_label(f"{args.systems}_{timestamp}")
    out_path = Path(args.out) if args.out else REPORT_DIR / f"levels1_7_native_regression_{run_label}.json"
    if not out_path.is_absolute():
        out_path = REPO_ROOT / out_path
    out_path.parent.mkdir(parents=True, exist_ok=True)
    child_dir = out_path.parent / f"{out_path.stem}_children"
    child_dir.mkdir(parents=True, exist_ok=True)

    child_specs = [
        (
            "level1",
            child_dir / "level1_native_live.json",
            [
                sys.executable,
                str(THIS_DIR / "level1_native_live_baseline.py"),
                "--attempts",
                str(args.level1_attempts),
                "--model",
                args.model,
                "--systems",
                args.systems,
                "--attempt-timeout-seconds",
                str(args.attempt_timeout_seconds),
                "--out",
                str(child_dir / "level1_native_live.json"),
            ],
        ),
        (
            "level2",
            child_dir / "level2_weak_model_live.json",
            [
                sys.executable,
                str(THIS_DIR / "level2_weak_model_live_baseline.py"),
                "--attempts",
                str(args.level2_attempts),
                "--model",
                args.model,
                "--systems",
                args.systems,
                "--out",
                str(child_dir / "level2_weak_model_live.json"),
            ],
        ),
        (
            "levels3_7",
            child_dir / "levels3_7_live.json",
            [
                sys.executable,
                str(THIS_DIR / "level3_level4_live_baseline.py"),
                "--model",
                args.model,
                "--systems",
                args.systems,
                "--levels",
                args.levels3_7,
                "--attempt-timeout-seconds",
                str(args.attempt_timeout_seconds),
                "--out",
                str(child_dir / "levels3_7_live.json"),
            ],
        ),
    ]

    print(
        json.dumps(
            {
                "event": "native_regression_sweep_start",
                "model": args.model,
                "systems": args.systems,
                "levels": "1,2," + args.levels3_7,
                "out": str(out_path),
                "at": utc_now(),
            }
        ),
        flush=True,
    )
    child_results: list[dict[str, Any]] = []
    for name, report_path, command in child_specs:
        print(json.dumps({"event": "child_start", "name": name, "report": str(report_path), "at": utc_now()}), flush=True)
        child = run_child(name, command, report_path)
        child_results.append(child)
        print(
            json.dumps(
                {
                    "event": "child_end",
                    "name": name,
                    "ok": child["ok"],
                    "returncode": child["returncode"],
                    "wall_time_ms": child["wall_time_ms"],
                    "report_error": child["report_error"],
                    "at": utc_now(),
                }
            ),
            flush=True,
        )

    attempts = flatten_attempts(child_results)
    child_receipts = [
        {key: value for key, value in child.items() if key != "report"}
        for child in child_results
    ]
    failed_attempts = [attempt for attempt in attempts if attempt.get("ok") is not True]
    failed_children = [child for child in child_results if child.get("ok") is not True]
    report = {
        "harness_kind": "levels1_7_native_regression_sweep_v1",
        "generated_at": utc_now(),
        "status": "complete" if not failed_children and not failed_attempts else "failed",
        "ok": not failed_children and not failed_attempts,
        "model": args.model,
        "systems": [system.strip() for system in args.systems.split(",") if system.strip()],
        "levels": [1, 2] + [int(level.strip()) for level in args.levels3_7.split(",") if level.strip()],
        "level1_attempts": args.level1_attempts,
        "level2_attempts": args.level2_attempts,
        "attempt_timeout_seconds": args.attempt_timeout_seconds,
        "summary": normalize_summary(child_results),
        "attempt_count": len(attempts),
        "pass_count": sum(1 for attempt in attempts if attempt.get("ok") is True),
        "fail_count": len(failed_attempts),
        "child_reports": {
            child["name"]: child["report_path"]
            for child in child_results
        },
        "child_receipts": child_receipts,
        "attempts": attempts,
    }
    out_path.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0 if report["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
