#!/usr/bin/env python3
"""Run Level 3/4/5 coding baseline attempts across local coding-agent systems.

This harness is for comparison and behavioral assimilation. It must not patch
or improve the reference systems; it only drives them against the same local
fixtures and records runtime data.
"""

from __future__ import annotations

import argparse
import datetime as dt
import importlib.util
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from typing import Any

from command_resolution import command_execution_policy, resolve_forge_command, resolve_xtask_command


THIS_DIR = Path(__file__).resolve().parent
REPO_ROOT = THIS_DIR.parents[2]
DEFAULT_MODEL = "qwen2.5-coder:7b"

_LEVEL2_SPEC = importlib.util.spec_from_file_location(
    "level2_baseline", THIS_DIR / "level2_weak_model_live_baseline.py"
)
level2 = importlib.util.module_from_spec(_LEVEL2_SPEC)
assert _LEVEL2_SPEC.loader is not None
_LEVEL2_SPEC.loader.exec_module(level2)


def utc_now() -> str:
    return dt.datetime.now(dt.timezone.utc).isoformat()


def run_cmd(
    command: list[str],
    *,
    cwd: Path,
    timeout: int = 180,
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
            "stdout_tail": completed.stdout[-5000:],
            "stderr_tail": completed.stderr[-5000:],
            "timed_out": False,
            "wall_time_ms": round((time.monotonic() - started) * 1000),
        }
    except subprocess.TimeoutExpired as exc:
        stdout = exc.stdout if isinstance(exc.stdout, str) else ""
        stderr = exc.stderr if isinstance(exc.stderr, str) else ""
        return {
            "ok": False,
            "returncode": None,
            "stdout": stdout,
            "stderr": stderr,
            "stdout_tail": stdout[-5000:],
            "stderr_tail": stderr[-5000:],
            "timed_out": True,
            "wall_time_ms": round((time.monotonic() - started) * 1000),
        }


def seed_case(case: dict[str, Any], system: str) -> dict[str, Any]:
    root = Path(tempfile.mkdtemp(prefix=f"infring-{case['id']}-{system}-"))
    for relative_path, content in case["initial_files"].items():
        target = root / relative_path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(content, encoding="utf-8")
    probe_path = root / ".infring" / "semantic_probe.py"
    probe_path.parent.mkdir(parents=True, exist_ok=True)
    probe_path.write_text(case["semantic_probe"], encoding="utf-8")
    seed_ms = round(time.time() * 1000)
    prompt = case["prompt"](root)
    prompt_path = root / ".infring" / "prompt.txt"
    prompt_path.write_text(prompt, encoding="utf-8")
    return {
        "case": case,
        "case_id": case["id"],
        "level": case["level"],
        "project_root": root,
        "prompt": prompt,
        "prompt_path": prompt_path,
        "seed_ms": seed_ms,
        "validation_command": case["validation_command"],
        "semantic_probe_command": "PYTHONPATH=. python3 .infring/semantic_probe.py",
    }


def cases() -> list[dict[str, Any]]:
    return [
        {
            "id": "level3_existing_project_edit",
            "level": 3,
            "initial_files": {
                "math_tools.py": "def add(a, b):\n    return a + b\n",
                "test_math_tools.py": (
                    "import unittest\n"
                    "from math_tools import add\n\n"
                    "class MathToolsTests(unittest.TestCase):\n"
                    "    def test_add(self):\n"
                    "        self.assertEqual(add(2, 3), 5)\n\n"
                    "if __name__ == \"__main__\":\n"
                    "    unittest.main()\n"
                ),
            },
            "validation_command": "PYTHONPATH=. python3 -m unittest discover -s . -p 'test_*.py'",
            "semantic_probe": (
                "from math_tools import subtract\n"
                "assert subtract(7, 2) == 5\n"
            ),
            "prompt": lambda root: (
                f"Project root: {root}\n"
                "This is an existing Python project. Inspect the local files and add a subtract(a, b) "
                "function to math_tools.py, then add a unittest for it in test_math_tools.py. "
                "Run this validation command from project root: "
                "PYTHONPATH=. python3 -m unittest discover -s . -p 'test_*.py'. "
                "Run this semantic probe command from project root after validation: "
                "PYTHONPATH=. python3 .infring/semantic_probe.py. "
                "Preserve add behavior. Do not commit."
            ),
            "expected_paths": ["math_tools.py", "test_math_tools.py"],
            "expected_markers": ["def subtract", "test_subtract"],
        },
        {
            "id": "level4_validation_guided_repair",
            "level": 4,
            "initial_files": {
                "slug_tools.py": (
                    "def slugify(value: str) -> str:\n"
                    "    return value.lower().replace(\" \", \"-\")\n"
                ),
                "test_slug_tools.py": (
                    "import unittest\n"
                    "from slug_tools import slugify\n\n"
                    "class SlugToolsTests(unittest.TestCase):\n"
                    "    def test_removes_punctuation(self):\n"
                    "        self.assertEqual(slugify(\"Hello, World!\"), \"hello-world\")\n\n"
                    "    def test_collapses_spaces(self):\n"
                    "        self.assertEqual(slugify(\"multi   space\"), \"multi-space\")\n\n"
                    "    def test_preserves_existing_slug_shape(self):\n"
                    "        self.assertEqual(slugify(\"Already-Slug\"), \"already-slug\")\n\n"
                    "if __name__ == \"__main__\":\n"
                    "    unittest.main()\n"
                ),
            },
            "validation_command": "PYTHONPATH=. python3 -m unittest discover -s . -p 'test_*.py'",
            "semantic_probe": (
                "from slug_tools import slugify\n"
                "assert slugify('Hello, World!') == 'hello-world'\n"
                "assert slugify('multi   space') == 'multi-space'\n"
            ),
            "prompt": lambda root: (
                f"Project root: {root}\n"
                "This is an existing Python project with failing tests. First run this validation command "
                "from project root to observe the failure: PYTHONPATH=. python3 -m unittest discover -s . -p 'test_*.py'. "
                "Then inspect the relevant files, repair slugify, rerun validation until tests pass, and run "
                "this semantic probe: PYTHONPATH=. python3 .infring/semantic_probe.py. Do not commit."
            ),
            "expected_paths": ["slug_tools.py", "test_slug_tools.py"],
            "expected_markers": ["def slugify", "test_removes_punctuation", "test_collapses_spaces"],
        },
        {
            "id": "level5_public_interface_repair",
            "level": 5,
            "initial_files": {
                "calcpack/__init__.py": (
                    "from .arithmetic import add\n\n"
                    "__all__ = [\"add\"]\n"
                ),
                "calcpack/arithmetic.py": (
                    "def add(a, b):\n"
                    "    return a + b\n"
                ),
                "tests/test_public_api.py": (
                    "import unittest\n"
                    "import calcpack\n"
                    "from calcpack import add, multiply\n\n"
                    "class PublicApiTests(unittest.TestCase):\n"
                    "    def test_add_still_exported(self):\n"
                    "        self.assertEqual(add(2, 3), 5)\n\n"
                    "    def test_multiply_exported_from_package(self):\n"
                    "        self.assertEqual(multiply(4, 5), 20)\n\n"
                    "    def test_public_all_declares_multiply(self):\n"
                    "        self.assertIn(\"multiply\", calcpack.__all__)\n\n"
                    "if __name__ == \"__main__\":\n"
                    "    unittest.main()\n"
                ),
            },
            "validation_command": "PYTHONPATH=. python3 -m unittest discover -s tests -p 'test_*.py'",
            "semantic_probe": (
                "import calcpack\n"
                "from calcpack import add, multiply\n"
                "assert add(2, 3) == 5\n"
                "assert multiply(6, 7) == 42\n"
                "assert \"multiply\" in calcpack.__all__\n"
            ),
            "prompt": lambda root: (
                f"Project root: {root}\n"
                "This is an existing Python package with a failing public API test. First run this validation "
                "command from project root to observe the failure: PYTHONPATH=. python3 -m unittest discover -s tests -p 'test_*.py'. "
                "Then inspect the package files, implement multiply(a, b), expose it from the calcpack public API, "
                "preserve add behavior, rerun validation until it passes, and run this semantic probe: "
                "PYTHONPATH=. python3 .infring/semantic_probe.py. Do not commit."
            ),
            "expected_paths": ["calcpack/__init__.py", "calcpack/arithmetic.py", "tests/test_public_api.py"],
            "expected_markers": ["def multiply", "\"multiply\"", "test_multiply_exported_from_package"],
        },
        {
            "id": "level6_public_persistence_slice",
            "level": 6,
            "initial_files": {
                "orderflow/__init__.py": (
                    "from .attempts import DeliveryAttempt, normalize_status\n\n"
                    "__all__ = [\"DeliveryAttempt\", \"normalize_status\"]\n"
                ),
                "orderflow/attempts.py": (
                    "from dataclasses import dataclass\n\n"
                    "@dataclass\n"
                    "class DeliveryAttempt:\n"
                    "    order_id: str\n"
                    "    status: str\n\n"
                    "def normalize_status(value: str) -> str:\n"
                    "    return value.strip().lower().replace(\" \", \"_\")\n"
                ),
                "tests/test_delivery_attempt_ledger.py": (
                    "import json\n"
                    "import tempfile\n"
                    "import unittest\n"
                    "from pathlib import Path\n\n"
                    "import orderflow\n"
                    "from orderflow import DeliveryAttempt, DeliveryAttemptLedger, summarize_attempts\n\n"
                    "class DeliveryAttemptLedgerTests(unittest.TestCase):\n"
                    "    def test_public_exports_include_ledger_api(self):\n"
                    "        self.assertIn(\"DeliveryAttemptLedger\", orderflow.__all__)\n"
                    "        self.assertIn(\"summarize_attempts\", orderflow.__all__)\n\n"
                    "    def test_records_and_summarizes_statuses(self):\n"
                    "        ledger = DeliveryAttemptLedger()\n"
                    "        ledger.record(DeliveryAttempt(\"A-1\", \"Delivered\"))\n"
                    "        ledger.record(DeliveryAttempt(\"A-2\", \"failed delivery\"))\n"
                    "        self.assertEqual(ledger.count_by_status(), {\"delivered\": 1, \"failed_delivery\": 1})\n"
                    "        self.assertEqual(summarize_attempts(ledger.attempts), {\n"
                    "            \"total\": 2,\n"
                    "            \"by_status\": {\"delivered\": 1, \"failed_delivery\": 1},\n"
                    "        })\n\n"
                    "    def test_jsonl_round_trip(self):\n"
                    "        ledger = DeliveryAttemptLedger()\n"
                    "        ledger.record(DeliveryAttempt(\"A-3\", \"delivered\"))\n"
                    "        with tempfile.TemporaryDirectory() as tmp:\n"
                    "            path = Path(tmp) / \"attempts.jsonl\"\n"
                    "            ledger.save(path)\n"
                    "            raw = path.read_text(encoding=\"utf-8\").strip().splitlines()\n"
                    "            self.assertEqual(json.loads(raw[0]), {\"order_id\": \"A-3\", \"status\": \"delivered\"})\n"
                    "            restored = DeliveryAttemptLedger.load(path)\n"
                    "        self.assertEqual(restored.count_by_status(), {\"delivered\": 1})\n\n"
                    "if __name__ == \"__main__\":\n"
                    "    unittest.main()\n"
                ),
            },
            "validation_command": "PYTHONPATH=. python3 -m unittest discover -s tests -p 'test_*.py'",
            "semantic_probe": (
                "from pathlib import Path\n"
                "import tempfile\n"
                "from orderflow import DeliveryAttempt, DeliveryAttemptLedger, summarize_attempts\n"
                "ledger = DeliveryAttemptLedger()\n"
                "ledger.record(DeliveryAttempt('B-1', 'Delivered'))\n"
                "ledger.record(DeliveryAttempt('B-2', 'failed delivery'))\n"
                "assert ledger.count_by_status() == {'delivered': 1, 'failed_delivery': 1}\n"
                "assert summarize_attempts(ledger.attempts)['total'] == 2\n"
                "with tempfile.TemporaryDirectory() as tmp:\n"
                "    path = Path(tmp) / 'attempts.jsonl'\n"
                "    ledger.save(path)\n"
                "    restored = DeliveryAttemptLedger.load(path)\n"
                "assert restored.count_by_status() == {'delivered': 1, 'failed_delivery': 1}\n"
            ),
            "prompt": lambda root: (
                f"Project root: {root}\n"
                "This is an existing Python package with failing tests for a delivery attempt ledger. "
                "First run this validation command from project root to observe the failure: "
                "PYTHONPATH=. python3 -m unittest discover -s tests -p 'test_*.py'. "
                "Then inspect the package files, implement DeliveryAttemptLedger with record(), attempts, "
                "count_by_status(), save(path), and load(path), implement summarize_attempts(attempts), "
                "store JSONL rows with order_id and normalized status, expose the new public API from orderflow, "
                "preserve DeliveryAttempt and normalize_status behavior, rerun validation until it passes, and run "
                "this semantic probe: PYTHONPATH=. python3 .infring/semantic_probe.py. Do not commit."
            ),
            "expected_paths": [
                "orderflow/__init__.py",
                "orderflow/attempts.py",
                "tests/test_delivery_attempt_ledger.py",
            ],
            "expected_markers": [
                "class DeliveryAttemptLedger",
                "def summarize_attempts",
                "test_jsonl_round_trip",
            ],
        },
        {
            "id": "level7_multi_module_reporting_slice",
            "level": 7,
            "initial_files": {
                "warehouse/__init__.py": (
                    "from .items import StockItem, normalize_location\n\n"
                    "__all__ = [\"StockItem\", \"normalize_location\"]\n"
                ),
                "warehouse/items.py": (
                    "from dataclasses import dataclass\n\n"
                    "@dataclass\n"
                    "class StockItem:\n"
                    "    sku: str\n"
                    "    quantity: int\n"
                    "    location: str\n\n"
                    "def normalize_location(value: str) -> str:\n"
                    "    return value.strip().lower().replace(\" \", \"_\")\n"
                ),
                "tests/test_inventory_reporting.py": (
                    "import csv\n"
                    "import tempfile\n"
                    "import unittest\n"
                    "from pathlib import Path\n\n"
                    "import warehouse\n"
                    "from warehouse import InventoryCatalog, StockItem, summarize_inventory, write_reorder_report\n\n"
                    "class InventoryReportingTests(unittest.TestCase):\n"
                    "    def test_public_exports_include_reporting_api(self):\n"
                    "        self.assertIn(\"InventoryCatalog\", warehouse.__all__)\n"
                    "        self.assertIn(\"summarize_inventory\", warehouse.__all__)\n"
                    "        self.assertIn(\"write_reorder_report\", warehouse.__all__)\n\n"
                    "    def test_catalog_records_and_summarizes_locations(self):\n"
                    "        catalog = InventoryCatalog()\n"
                    "        catalog.add(StockItem(\"SKU-1\", 10, \"Front Room\"))\n"
                    "        catalog.add(StockItem(\"SKU-2\", 3, \"front room\"))\n"
                    "        catalog.add(StockItem(\"SKU-3\", 8, \"Back Room\"))\n"
                    "        self.assertEqual(catalog.count_by_location(), {\"front_room\": 2, \"back_room\": 1})\n"
                    "        self.assertEqual(summarize_inventory(catalog.items, low_stock_threshold=5), {\n"
                    "            \"total_skus\": 3,\n"
                    "            \"total_quantity\": 21,\n"
                    "            \"by_location\": {\"front_room\": 2, \"back_room\": 1},\n"
                    "            \"low_stock_skus\": [\"SKU-2\"],\n"
                    "        })\n\n"
                    "    def test_reorder_report_csv_contains_low_stock_rows(self):\n"
                    "        items = [\n"
                    "            StockItem(\"SKU-9\", 1, \"Remote Shelf\"),\n"
                    "            StockItem(\"SKU-8\", 9, \"Remote Shelf\"),\n"
                    "            StockItem(\"SKU-7\", 2, \"Main Shelf\"),\n"
                    "        ]\n"
                    "        with tempfile.TemporaryDirectory() as tmp:\n"
                    "            path = Path(tmp) / \"reorder.csv\"\n"
                    "            write_reorder_report(items, path, low_stock_threshold=5)\n"
                    "            rows = list(csv.DictReader(path.read_text(encoding=\"utf-8\").splitlines()))\n"
                    "        self.assertEqual(rows, [\n"
                    "            {\"sku\": \"SKU-7\", \"quantity\": \"2\", \"location\": \"main_shelf\"},\n"
                    "            {\"sku\": \"SKU-9\", \"quantity\": \"1\", \"location\": \"remote_shelf\"},\n"
                    "        ])\n\n"
                    "if __name__ == \"__main__\":\n"
                    "    unittest.main()\n"
                ),
            },
            "validation_command": "PYTHONPATH=. python3 -m unittest discover -s tests -p 'test_*.py'",
            "semantic_probe": (
                "from pathlib import Path\n"
                "import csv\n"
                "import tempfile\n"
                "from warehouse import InventoryCatalog, StockItem, summarize_inventory, write_reorder_report\n"
                "catalog = InventoryCatalog()\n"
                "catalog.add(StockItem('A-1', 2, 'Floor Bin'))\n"
                "catalog.add(StockItem('A-2', 7, 'Floor Bin'))\n"
                "catalog.add(StockItem('A-3', 4, 'Overstock'))\n"
                "summary = summarize_inventory(catalog.items, low_stock_threshold=5)\n"
                "assert summary['total_skus'] == 3\n"
                "assert summary['total_quantity'] == 13\n"
                "assert summary['by_location'] == {'floor_bin': 2, 'overstock': 1}\n"
                "assert summary['low_stock_skus'] == ['A-1', 'A-3']\n"
                "with tempfile.TemporaryDirectory() as tmp:\n"
                "    path = Path(tmp) / 'reorder.csv'\n"
                "    write_reorder_report(catalog.items, path, low_stock_threshold=5)\n"
                "    rows = list(csv.DictReader(path.read_text(encoding='utf-8').splitlines()))\n"
                "assert rows == [\n"
                "    {'sku': 'A-1', 'quantity': '2', 'location': 'floor_bin'},\n"
                "    {'sku': 'A-3', 'quantity': '4', 'location': 'overstock'},\n"
                "]\n"
            ),
            "prompt": lambda root: (
                f"Project root: {root}\n"
                "This is an existing Python package with failing inventory reporting tests. "
                "First run this validation command from project root to observe the failure: "
                "PYTHONPATH=. python3 -m unittest discover -s tests -p 'test_*.py'. "
                "Then inspect the package files, implement InventoryCatalog with add(), items, and "
                "count_by_location(), implement summarize_inventory(items, low_stock_threshold=5), "
                "implement write_reorder_report(items, path, low_stock_threshold=5) as a CSV writer for "
                "low-stock rows sorted by sku, expose the new public API from warehouse, preserve StockItem "
                "and normalize_location behavior, rerun validation until it passes, and run this semantic "
                "probe: PYTHONPATH=. python3 .infring/semantic_probe.py. Do not commit."
            ),
            "expected_paths": [
                "warehouse/__init__.py",
                "warehouse/items.py",
                "tests/test_inventory_reporting.py",
            ],
            "expected_markers": [
                "class InventoryCatalog",
                "def summarize_inventory",
                "def write_reorder_report",
                "test_reorder_report_csv_contains_low_stock_rows",
            ],
        },
    ]


def changed_py_files(job: dict[str, Any]) -> list[str]:
    root = job["project_root"]
    seed_ms = job["seed_ms"]
    changed: list[str] = []
    for path in sorted(root.rglob("*.py")):
        if ".infring" in path.parts:
            continue
        if round(path.stat().st_mtime * 1000) > seed_ms:
            changed.append(str(path))
    return changed


def time_to_first_mutation_ms(job: dict[str, Any]) -> int | None:
    root = job["project_root"]
    seed_ms = job["seed_ms"]
    first: int | None = None
    for path in sorted(root.rglob("*.py")):
        if ".infring" in path.parts:
            continue
        mtime = round(path.stat().st_mtime * 1000)
        if mtime > seed_ms and (first is None or mtime < first):
            first = mtime
    if first is None:
        return None
    return max(0, first - seed_ms)


def run_validation(job: dict[str, Any]) -> dict[str, Any]:
    return run_cmd(
        ["sh", "-lc", job["validation_command"]],
        cwd=job["project_root"],
        timeout=30,
        env=os.environ | {"PYTHONPATH": "."},
    )


def run_semantic_probe(job: dict[str, Any]) -> dict[str, Any]:
    return run_cmd(
        ["sh", "-lc", job["semantic_probe_command"]],
        cwd=job["project_root"],
        timeout=30,
        env=os.environ | {"PYTHONPATH": "."},
    )


def content_markers_present(job: dict[str, Any]) -> bool:
    text = ""
    for path in job["case"]["expected_paths"]:
        target = job["project_root"] / path
        if target.exists():
            text += target.read_text(encoding="utf-8", errors="replace") + "\n"
    return all(marker in text for marker in job["case"]["expected_markers"])


def judge(system: str, job: dict[str, Any], run_result: dict[str, Any]) -> dict[str, Any]:
    validation = run_validation(job)
    semantic = run_semantic_probe(job)
    changed = changed_py_files(job)
    markers = content_markers_present(job)
    checks = [
        {"id": "agent_run_completed", "ok": run_result.get("ok") is True, "detail": run_result.get("error")},
        {"id": "source_or_test_mutated_after_seed", "ok": bool(changed), "detail": {"changed_files": changed}},
        {"id": "validation_passes_after_worker", "ok": validation["ok"], "detail": validation},
        {"id": "semantic_probe_passes", "ok": semantic["ok"], "detail": semantic},
        {"id": "expected_markers_present", "ok": markers, "detail": {"markers": job["case"]["expected_markers"]}},
    ]
    failures = [check["id"] for check in checks if not check["ok"]]
    return {
        "system": system,
        "case_id": job["case_id"],
        "level": job["level"],
        "ok": not failures,
        "failure_class": classify_failure(failures, run_result, checks),
        "wall_time_ms": run_result.get("wall_time_ms"),
        "time_to_first_mutation_ms": time_to_first_mutation_ms(job),
        "changed_files": changed,
        "checks": checks,
        "failures": failures,
        "run_result": run_result,
    }


def classify_failure(
    failures: list[str],
    run_result: dict[str, Any],
    checks: list[dict[str, Any]],
) -> str | None:
    if not failures:
        return None
    actionable = runtime_actionable_repair_class(run_result)
    seeded = runtime_seeded_import_surface(run_result)
    validation_class = validation_failure_class_from_checks(checks)
    if run_result.get("blocked"):
        return str(run_result["blocked"])
    if run_result.get("timed_out"):
        if actionable:
            return actionable
        if seeded:
            return "seeded_repair_timeout"
        return "runtime_timeout"
    if "source_or_test_mutated_after_seed" in failures:
        return "no_successful_mutation"
    if "validation_passes_after_worker" in failures:
        if actionable:
            return actionable
        if validation_class and seeded:
            return f"seeded_repair_{validation_class}"
        if validation_class:
            return validation_class
        return "validation_failed"
    if "semantic_probe_passes" in failures:
        if actionable:
            return actionable
        if validation_class and seeded:
            return f"seeded_repair_{validation_class}"
        if validation_class:
            return validation_class
        return "semantic_probe_failed"
    if "expected_markers_present" in failures:
        return "missing_expected_markers"
    return "agent_run_failed"


def runtime_actionable_repair_class(run_result: dict[str, Any]) -> str | None:
    analysis = run_result.get("runtime_failure_analysis")
    if isinstance(analysis, dict):
        value = analysis.get("actionable_repair_class")
        if isinstance(value, str) and value:
            return value
    return None


def runtime_seeded_import_surface(run_result: dict[str, Any]) -> bool:
    analysis = run_result.get("runtime_failure_analysis")
    if isinstance(analysis, dict) and analysis.get("seeded_python_import_surface") is True:
        return True
    for receipt in run_result.get("native_tool_receipt_summary") or []:
        if not isinstance(receipt, dict):
            continue
        call_id = str(receipt.get("call_id") or "")
        status = str(receipt.get("status") or "")
        if "runtime_python_import_surface_seed" in call_id and status == "ok":
            return True
    return False


def validation_failure_class_from_checks(checks: list[dict[str, Any]]) -> str | None:
    text_parts: list[str] = []
    for check in checks:
        if not isinstance(check, dict) or check.get("ok") is True:
            continue
        detail = check.get("detail")
        if isinstance(detail, (dict, list)):
            text_parts.append(json.dumps(detail, sort_keys=True))
        elif detail is not None:
            text_parts.append(str(detail))
    text = "\n".join(text_parts).lower()
    if not text:
        return None
    if "cannot import name" in text or "modulenotfounderror" in text:
        return "import_surface_missing"
    if "attributeerror" in text or "has no attribute" in text:
        return "attribute_missing"
    if "typeerror" in text:
        return "type_error"
    if "filenotfounderror" in text or "no such file or directory" in text:
        return "file_not_found"
    if "assertionerror" in text or "assert" in text:
        return "assertion_mismatch"
    if "timed out" in text or "timeout" in text:
        return "command_timeout"
    if "syntaxerror" in text or "indentationerror" in text:
        return "syntax_error"
    return "unknown_validation_failure"


def summarize_native_provider_turn_timing(project_root: Path) -> dict[str, Any] | None:
    path = project_root / ".infring" / "native_provider_turn_timing.jsonl"
    if not path.exists():
        return None
    rows: list[dict[str, Any]] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            value = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(value, dict):
            rows.append(value)
    if not rows:
        return None
    latencies = [
        row.get("provider_latency_ms")
        for row in rows
        if isinstance(row.get("provider_latency_ms"), int)
    ]
    return {
        "source": "native_provider_turn_timing_probe_v1",
        "path": str(path),
        "turn_count": len(rows),
        "total_provider_latency_ms": sum(latencies),
        "first_turn": rows[0],
        "last_turn": rows[-1],
        "turns": rows[-8:],
    }


def summarize_native_runtime_timeline(project_root: Path) -> dict[str, Any] | None:
    path = project_root / ".infring" / "native_runtime_timeline.jsonl"
    if not path.exists():
        return None
    rows: list[dict[str, Any]] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            value = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(value, dict):
            rows.append(value)
    if not rows:
        return None
    return {
        "source": "native_runtime_timeline_probe_v1",
        "path": str(path),
        "event_count": len(rows),
        "first_event": rows[0],
        "last_event": rows[-1],
        "events": rows[-16:],
    }


def run_infring(job: dict[str, Any], model: str) -> dict[str, Any]:
    started = time.monotonic()
    resolution = resolve_xtask_command(REPO_ROOT, policy=command_execution_policy())
    if not resolution["ok"]:
        return {
            "ok": False,
            "blocked": resolution.get("blocked"),
            "wall_time_ms": round((time.monotonic() - started) * 1000),
            "model": model,
            "raw_command_ok": False,
            "error": resolution.get("blocked"),
            "command_resolution": resolution["receipt"],
            "execution_mode": resolution["receipt"]["execution_mode"],
            "timing_comparable": resolution["receipt"]["timing_comparable"],
        }
    command = list(resolution["command"])
    command.extend(
        [
            "--workflow=local_coding_phase1_mutation_spine",
            f"--name=baseline-{job['case_id']}",
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
        cwd=REPO_ROOT,
        timeout=240,
        env=os.environ | {"INFRING_RUNTIME_LANE_MODEL_LOCK": model},
    )
    parsed: dict[str, Any] = {}
    try:
        parsed = json.loads(result.get("stdout") or result.get("stdout_tail") or "{}")
    except json.JSONDecodeError:
        start = (result.get("stdout") or "").find("{")
        if start >= 0:
            try:
                parsed = json.loads(result["stdout"][start:])
            except json.JSONDecodeError:
                parsed = {}
    receipt = parsed.get("receipt") if isinstance(parsed.get("receipt"), dict) else {}
    runtime_failure_analysis = (
        receipt.get("runtime_failure_analysis")
        if isinstance(receipt.get("runtime_failure_analysis"), dict)
        else None
    )
    trace_summary = parsed.get("trace_summary") if isinstance(parsed.get("trace_summary"), dict) else {}
    xtask_outer_timing_ms = parsed.get("xtask_outer_timing_ms") if isinstance(parsed.get("xtask_outer_timing_ms"), dict) else None
    agent_runtime_phase_latency_ms = receipt.get("agent_runtime_phase_latency_ms") if isinstance(receipt.get("agent_runtime_phase_latency_ms"), dict) else None
    native_tool_phase_latency_ms = receipt.get("native_tool_phase_latency_ms") if isinstance(receipt.get("native_tool_phase_latency_ms"), dict) else None
    runtime_lane_phase_latency_ms = trace_summary.get("runtime_lane_phase_latency_ms") if isinstance(trace_summary.get("runtime_lane_phase_latency_ms"), dict) else None
    coding_runtime_probe = receipt.get("coding_runtime_probe") if isinstance(receipt.get("coding_runtime_probe"), dict) else None
    if coding_runtime_probe is None and isinstance(trace_summary.get("coding_runtime_probe"), dict):
        coding_runtime_probe = trace_summary["coding_runtime_probe"]
    native_provider_turn_timing_probe = summarize_native_provider_turn_timing(job["project_root"])
    if native_provider_turn_timing_probe is not None:
        if coding_runtime_probe is None:
            coding_runtime_probe = native_provider_turn_timing_probe
        else:
            coding_runtime_probe = {
                "runtime_probe": coding_runtime_probe,
                "native_provider_turn_timing": native_provider_turn_timing_probe,
            }
    native_runtime_timeline_probe = summarize_native_runtime_timeline(job["project_root"])
    native_receipts = receipt.get("native_tool_receipts") if isinstance(receipt.get("native_tool_receipts"), list) else []
    response = parsed.get("response") if isinstance(parsed.get("response"), dict) else {}
    response_raw = response.get("raw") if isinstance(response.get("raw"), dict) else {}
    native_lane = response_raw.get("native_bounded_patch_artifact_lane") if isinstance(response_raw.get("native_bounded_patch_artifact_lane"), dict) else {}
    lane_phase_latency_ms = native_lane.get("phase_latency_ms") if isinstance(native_lane.get("phase_latency_ms"), dict) else None
    lane_artifact_profile = native_lane.get("artifact_profile")
    native_tool_receipt_summary = []
    for item in native_receipts:
        if not isinstance(item, dict):
            continue
        result_payload = item.get("result") if isinstance(item.get("result"), dict) else {}
        details = result_payload.get("details") if isinstance(result_payload.get("details"), dict) else {}
        if lane_phase_latency_ms is None and isinstance(details.get("phase_latency_ms"), dict):
            lane_phase_latency_ms = details["phase_latency_ms"]
        if lane_artifact_profile is None and isinstance(details.get("artifact_profile"), str):
            lane_artifact_profile = details["artifact_profile"]
        native_tool_receipt_summary.append({
            "call_id": item.get("call_id"),
            "tool_name": item.get("tool_name"),
            "status": item.get("status"),
            "error": item.get("error"),
            "path": result_payload.get("path"),
            "terminal_status": result_payload.get("terminal_status"),
            "reason": result_payload.get("reason"),
            "details": result_payload.get("details"),
        })
    return {
        "ok": result["ok"] and parsed.get("ok") is True,
        "wall_time_ms": round((time.monotonic() - started) * 1000),
        "model": model,
        "raw_command_ok": result["ok"],
        "error": parsed.get("error") or result.get("stderr_tail"),
        "native_tool_names": [item.get("tool_name") for item in native_receipts if isinstance(item, dict)],
        "native_tool_call_count": len(native_receipts),
        "native_tool_receipt_summary": native_tool_receipt_summary,
        "native_patch_artifact_profile": lane_artifact_profile,
        "native_patch_lane_phase_latency_ms": lane_phase_latency_ms,
        "agent_runtime_phase_latency_ms": agent_runtime_phase_latency_ms,
        "native_tool_phase_latency_ms": native_tool_phase_latency_ms,
        "runtime_lane_phase_latency_ms": runtime_lane_phase_latency_ms,
        "runtime_failure_analysis": runtime_failure_analysis,
        "coding_runtime_probe": coding_runtime_probe,
        "native_runtime_timeline_probe": native_runtime_timeline_probe,
        "xtask_outer_timing_ms": xtask_outer_timing_ms,
        "command_resolution": resolution["receipt"],
        "execution_mode": resolution["receipt"]["execution_mode"],
        "timing_comparable": resolution["receipt"]["timing_comparable"],
        "timed_out": result.get("timed_out"),
    }


def run_mini_swe_agent(job: dict[str, Any], model: str) -> dict[str, Any]:
    mini_root = REPO_ROOT / "references/coding-agent-systems/mini-swe-agent"
    if not mini_root.exists():
        return {"ok": False, "blocked": "repo_missing", "wall_time_ms": None}
    sys.path.insert(0, str(mini_root / "src"))
    try:
        from minisweagent.agents.default import DefaultAgent
        from minisweagent.environments.local import LocalEnvironment
    except Exception as exc:  # noqa: BLE001
        return {"ok": False, "blocked": f"import_failed:{type(exc).__name__}:{exc}", "wall_time_ms": None}

    outputs = job["project_root"] / ".infring" / "system_outputs" / "mini-swe-agent"
    outputs.mkdir(parents=True, exist_ok=True)
    trajectory_path = outputs / "trajectory.json"
    model_adapter = level2.OllamaJsonModel(model)
    env = LocalEnvironment(cwd=str(job["project_root"]), timeout=30)
    agent = DefaultAgent(
        model_adapter,
        env,
        system_template=(
            "You are a local coding agent. Return only JSON.\n"
            "Use shell commands to inspect, edit, and validate the project.\n"
            "JSON schema: {\"actions\":[{\"command\":\"shell command\"}]} or {\"finish\":\"summary\"}.\n"
            "Do one or two useful commands per turn. Read files before editing existing behavior.\n"
            "Use python heredocs for multi-line writes. Do not commit. Stop after validation and semantic probe pass.\n"
        ),
        instance_template="{{task}}",
        step_limit=8,
        cost_limit=0,
        output_path=trajectory_path,
    )
    started = time.monotonic()
    try:
        result = agent.run(job["prompt"])
        ok = result.get("exit_status") == "Submitted"
        error = None
    except Exception as exc:  # noqa: BLE001
        ok = False
        error = f"{type(exc).__name__}:{exc}"
    return {
        "ok": ok,
        "wall_time_ms": round((time.monotonic() - started) * 1000),
        "model": model,
        "error": error,
        "trajectory_path": str(trajectory_path),
    }


def run_aider(job: dict[str, Any], model: str) -> dict[str, Any]:
    aider_bin = Path("/tmp/infring-baselines-aider/bin/aider")
    if not aider_bin.exists():
        return {"ok": False, "blocked": "temp_venv_missing:/tmp/infring-baselines-aider", "wall_time_ms": None}
    outputs = job["project_root"] / ".infring" / "system_outputs" / "aider"
    outputs.mkdir(parents=True, exist_ok=True)
    prompt_path = outputs / "prompt.txt"
    stdout_path = outputs / "stdout.txt"
    prompt_path.write_text(
        job["prompt"]
        + "\n\nUse the existing project files in the current working directory."
        + f"\nRun this validation command before final response: {job['validation_command']}"
        + "\nDo not commit.",
        encoding="utf-8",
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
    for path in sorted(job["project_root"].rglob("*.py")):
        if ".infring" in path.parts:
            continue
        command.extend(["--file", str(path.relative_to(job["project_root"]))])
    result = run_cmd(command, cwd=job["project_root"], timeout=240, env=os.environ | {"PYTHONPATH": "."})
    stdout_path.write_text((result.get("stdout") or "") + "\n\nSTDERR:\n" + (result.get("stderr") or ""), encoding="utf-8")
    return {
        "ok": result["ok"],
        "wall_time_ms": result["wall_time_ms"],
        "model": model,
        "error": result.get("stderr_tail") if not result["ok"] else None,
        "stdout_path": str(stdout_path),
        "timed_out": result.get("timed_out"),
    }


def run_swe_agent(job: dict[str, Any], model: str) -> dict[str, Any]:
    sweagent_bin = Path("/tmp/infring-baselines-sweagent/bin/sweagent")
    if not sweagent_bin.exists():
        return {"ok": False, "blocked": "temp_venv_missing:/tmp/infring-baselines-sweagent", "wall_time_ms": None}
    outputs = job["project_root"] / ".infring" / "system_outputs" / "swe-agent"
    outputs.mkdir(parents=True, exist_ok=True)
    config_path = REPO_ROOT / "references/coding-agent-systems/swe-agent/config/coding_challenge.yaml"
    problem_path = outputs / "problem.md"
    output_dir = outputs / "run"
    stdout_path = outputs / "stdout.txt"
    local_root = outputs / "swe-agent-local-root"
    shim_path = REPO_ROOT / "references/coding-agent-systems/runtime_trace_harness/swe_agent_local_root_sitecustomize"
    level2.ensure_temp_git_repo(job["project_root"])
    problem_path.write_text(job["prompt"] + "\n\nWork in the existing local repository. Do not commit.", encoding="utf-8")
    result = run_cmd(
        [
            str(sweagent_bin),
            "run",
            "--config",
            str(config_path),
            f"--problem_statement.path={problem_path}",
            "--env.deployment.type=local",
            "--env.repo.type=preexisting",
            f"--env.repo.repo_name={str(job['project_root']).lstrip('/')}",
            f"--agent.model.name=ollama/{model}",
            "--agent.model.api_base=http://localhost:11434",
            "--agent.model.per_instance_cost_limit=0",
            "--agent.model.total_cost_limit=0",
            "--agent.model.per_instance_call_limit=8",
            "--actions.apply_patch_locally=True",
            f"--output_dir={output_dir}",
        ],
        cwd=REPO_ROOT / "references/coding-agent-systems/swe-agent",
        timeout=300,
        env=os.environ | {"PYTHONPATH": f"{shim_path}:src", "SWE_AGENT_LOCAL_ROOT": str(local_root)},
    )
    stdout_path.write_text((result.get("stdout") or "") + "\n\nSTDERR:\n" + (result.get("stderr") or ""), encoding="utf-8")
    return {
        "ok": result["ok"],
        "wall_time_ms": result["wall_time_ms"],
        "model": model,
        "error": result.get("stderr_tail") if not result["ok"] else None,
        "stdout_path": str(stdout_path),
        "output_dir": str(output_dir),
        "timed_out": result.get("timed_out"),
    }


def run_forgecode(job: dict[str, Any], model: str) -> dict[str, Any]:
    started = time.monotonic()
    forge_root = REPO_ROOT / "references/coding-agent-systems/forgecode"
    if not (forge_root / "Cargo.toml").exists():
        return {"ok": False, "blocked": "repo_missing:references/coding-agent-systems/forgecode", "wall_time_ms": None}
    outputs = job["project_root"] / ".infring" / "system_outputs" / "forgecode"
    outputs.mkdir(parents=True, exist_ok=True)
    prompt_path = outputs / "prompt.txt"
    stdout_path = outputs / "stdout.txt"
    config_root = outputs / "forge-config"
    debug_requests = outputs / "forge-debug-requests.json"
    config_root.mkdir(parents=True, exist_ok=True)
    (config_root / ".forge.toml").write_text(
        "\n".join([
            "[session]",
            "provider_id = \"openai_compatible\"",
            f"model_id = \"{model}\"",
            "",
            "[reasoning]",
            "enabled = false",
            "",
        ]),
        encoding="utf-8",
    )
    prompt_path.write_text(
        job["prompt"]
        + "\n\nUse ForgeCode one-shot local coding mode in the current project."
        + f"\nRun this validation command before final response: {job['validation_command']}"
        + "\nDo not commit.",
        encoding="utf-8",
    )
    resolution = resolve_forge_command(REPO_ROOT, forge_root, policy=command_execution_policy())
    if not resolution["ok"]:
        return {
            "ok": False,
            "blocked": resolution.get("blocked"),
            "wall_time_ms": round((time.monotonic() - started) * 1000),
            "model": model,
            "provider": "openai_compatible",
            "error": resolution.get("blocked"),
            "stdout_path": str(stdout_path),
            "debug_requests_path": str(debug_requests),
            "debug_request_file_count": 0,
            "command_resolution": resolution["receipt"],
            "execution_mode": resolution["receipt"]["execution_mode"],
            "timing_comparable": resolution["receipt"]["timing_comparable"],
        }
    result = run_cmd(
        [
            *resolution["command"],
            "-C",
            str(job["project_root"]),
            "-p",
            prompt_path.read_text(encoding="utf-8"),
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
            "PYTHONPATH": ".",
        },
    )
    stdout_path.write_text((result.get("stdout") or "") + "\n\nSTDERR:\n" + (result.get("stderr") or ""), encoding="utf-8")
    return {
        "ok": result["ok"],
        "wall_time_ms": result["wall_time_ms"],
        "model": model,
        "error": result.get("stderr_tail") if not result["ok"] else None,
        "stdout_path": str(stdout_path),
        "debug_requests_path": str(debug_requests),
        "debug_request_file_count": 1 if debug_requests.exists() else 0,
        "command_resolution": resolution["receipt"],
        "execution_mode": resolution["receipt"]["execution_mode"],
        "timing_comparable": resolution["receipt"]["timing_comparable"],
        "timed_out": result.get("timed_out"),
    }


def _collect_claude_code_tool_names(value: Any) -> list[str]:
    names: list[str] = []
    if isinstance(value, dict):
        if value.get("type") == "tool_use" and isinstance(value.get("name"), str):
            names.append(value["name"])
        for child in value.values():
            names.extend(_collect_claude_code_tool_names(child))
    elif isinstance(value, list):
        for child in value:
            names.extend(_collect_claude_code_tool_names(child))
    return names


def _summarize_claude_code_stream(stdout: str) -> dict[str, Any]:
    event_types: dict[str, int] = {}
    tool_names: list[str] = []
    parse_errors = 0
    event_count = 0
    result_is_error = False
    api_error_status = None
    result_text = ""
    for line in stdout.splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            parse_errors += 1
            continue
        event_count += 1
        event_type = str(event.get("type") or event.get("event") or "unknown")
        event_types[event_type] = event_types.get(event_type, 0) + 1
        if event_type == "result":
            result_is_error = bool(event.get("is_error"))
            api_error_status = event.get("api_error_status")
            if isinstance(event.get("result"), str):
                result_text = event["result"]
        tool_names.extend(_collect_claude_code_tool_names(event))
    return {
        "event_count": event_count,
        "event_types": event_types,
        "tool_use_names": tool_names,
        "tool_use_count": len(tool_names),
        "json_parse_error_count": parse_errors,
        "result_is_error": result_is_error,
        "api_error_status": api_error_status,
        "result_text": result_text,
    }


def run_claude_code(job: dict[str, Any], model: str) -> dict[str, Any]:
    claude_bin = shutil.which("claude")
    ollama_bin = shutil.which("ollama")
    if not claude_bin and not ollama_bin:
        return {"ok": False, "blocked": "cli_missing:claude_and_ollama", "wall_time_ms": None}
    requested_model = model
    claude_model = os.environ.get("INFRING_CLAUDE_CODE_MODEL", model)
    if claude_model != requested_model:
        return {
            "ok": False,
            "blocked": "claude_code_control_model_mismatch",
            "wall_time_ms": None,
            "model": claude_model,
            "requested_model": requested_model,
            "model_controlled": False,
            "model_routing_note": "Claude Code comparison requires the requested harness model. Set INFRING_CLAUDE_CODE_MODEL equal to --model or unset it.",
        }
    outputs = job["project_root"] / ".infring" / "system_outputs" / "claude-code"
    outputs.mkdir(parents=True, exist_ok=True)
    prompt_path = outputs / "prompt.txt"
    stdout_path = outputs / "stdout.stream-jsonl"
    stderr_path = outputs / "stderr.txt"
    debug_path = outputs / "debug.log"
    prompt_path.write_text(
        job["prompt"]
        + "\n\nUse the existing project files in the current working directory."
        + f"\nRun this validation command before final response: {job['validation_command']}"
        + "\nRun the semantic probe too. After validation and semantic probe pass, print a concise final summary and stop immediately. Do not commit.",
        encoding="utf-8",
    )
    bridge_mode = os.environ.get("INFRING_CLAUDE_CODE_BRIDGE", "ollama-launch").strip().lower()
    if bridge_mode not in {"ollama-launch", "anthropic-base-url", "direct"}:
        bridge_mode = "ollama-launch"
    if bridge_mode == "ollama-launch" and not ollama_bin:
        return {
            "ok": False,
            "blocked": "claude_code_ollama_launch_unavailable",
            "wall_time_ms": None,
            "model": claude_model,
            "requested_model": requested_model,
            "model_controlled": True,
            "model_routing_note": "Claude Code control run requires ollama launch for Ollama model routing, but ollama was not found.",
        }
    if bridge_mode != "ollama-launch" and not claude_bin:
        return {"ok": False, "blocked": "cli_missing:claude", "wall_time_ms": None}
    common_args = [
        "--print",
        "--output-format=stream-json",
        "--verbose",
        "--include-partial-messages",
        "--no-session-persistence",
        "--permission-mode=bypassPermissions",
        "--tools=Read,Edit,MultiEdit,Write,Bash",
        f"--allowedTools=Read,Edit,MultiEdit,Write,Bash",
        f"--debug-file={debug_path}",
        f"--model={claude_model}",
        f"--add-dir={job['project_root']}",
        prompt_path.read_text(encoding="utf-8"),
    ]
    if bridge_mode == "ollama-launch":
        command = [
            ollama_bin,
            "launch",
            "claude",
            "--model",
            claude_model,
            "--yes",
            "--",
            *common_args,
        ]
        env = os.environ | {"PYTHONPATH": "."}
    elif bridge_mode == "anthropic-base-url":
        command = [
            claude_bin,
            *common_args,
        ]
        env = os.environ | {
            "PYTHONPATH": ".",
            "ANTHROPIC_AUTH_TOKEN": "ollama",
            "ANTHROPIC_API_KEY": "",
            "ANTHROPIC_BASE_URL": os.environ.get("ANTHROPIC_BASE_URL", "http://localhost:11434"),
        }
    else:
        command = [
            claude_bin,
            *common_args,
        ]
        env = os.environ | {"PYTHONPATH": "."}
    result = run_cmd(
        command,
        cwd=job["project_root"],
        timeout=300,
        env=env,
    )
    stdout_path.write_text(result.get("stdout") or "", encoding="utf-8")
    stderr_path.write_text(result.get("stderr") or "", encoding="utf-8")
    stream_summary = _summarize_claude_code_stream(result.get("stdout") or "")
    control_model_unavailable = (
        stream_summary["result_is_error"]
        and stream_summary["api_error_status"] in {400, 404}
        and requested_model in stream_summary["result_text"]
    )
    bridge_unavailable = stream_summary["result_is_error"] and (
        "ollama" in stream_summary["result_text"].lower()
        or "base url" in stream_summary["result_text"].lower()
        or "api key" in stream_summary["result_text"].lower()
    )
    return {
        "ok": result["ok"] and not stream_summary["result_is_error"],
        "blocked": "claude_code_control_model_unavailable"
        if control_model_unavailable
        else "claude_code_ollama_bridge_unavailable"
        if bridge_unavailable
        else None,
        "wall_time_ms": result["wall_time_ms"],
        "model": claude_model,
        "requested_model": requested_model,
        "model_controlled": claude_model == requested_model,
        "model_routing_note": f"Claude Code was invoked with the requested harness control model through bridge={bridge_mode}.",
        "bridge_mode": bridge_mode,
        "cli_path": claude_bin,
        "ollama_path": ollama_bin,
        "error": stream_summary["result_text"]
        if stream_summary["result_is_error"]
        else result.get("stderr_tail")
        if not result["ok"]
        else None,
        "stdout_path": str(stdout_path),
        "stderr_path": str(stderr_path),
        "debug_path": str(debug_path),
        "timed_out": result.get("timed_out"),
        "stream_event_count": stream_summary["event_count"],
        "stream_event_types": stream_summary["event_types"],
        "tool_use_names": stream_summary["tool_use_names"],
        "tool_use_count": stream_summary["tool_use_count"],
        "stream_json_parse_error_count": stream_summary["json_parse_error_count"],
        "stream_result_is_error": stream_summary["result_is_error"],
        "stream_api_error_status": stream_summary["api_error_status"],
    }


def _collect_grok_tool_names(value: Any) -> list[str]:
    names: list[str] = []
    if isinstance(value, dict):
        for key in ("tool_name", "tool", "name"):
            raw = value.get(key)
            if isinstance(raw, str) and raw:
                if raw in {"Read", "Edit", "MultiEdit", "Write", "Bash", "Glob", "Grep"}:
                    names.append(raw)
                elif "tool" in raw.lower() or "bash" in raw.lower() or "edit" in raw.lower():
                    names.append(raw)
        event_type = value.get("type") or value.get("event")
        if isinstance(event_type, str) and ("tool" in event_type.lower() or "bash" in event_type.lower()):
            names.append(event_type)
        for child in value.values():
            names.extend(_collect_grok_tool_names(child))
    elif isinstance(value, list):
        for child in value:
            names.extend(_collect_grok_tool_names(child))
    return names


def _collect_grok_session_ids(value: Any) -> list[str]:
    ids: list[str] = []
    if isinstance(value, dict):
        for key in ("session_id", "sessionId", "conversation_id", "conversationId"):
            raw = value.get(key)
            if isinstance(raw, str) and raw:
                ids.append(raw)
        for child in value.values():
            ids.extend(_collect_grok_session_ids(child))
    elif isinstance(value, list):
        for child in value:
            ids.extend(_collect_grok_session_ids(child))
    return ids


def _summarize_grok_stream(stdout: str) -> dict[str, Any]:
    event_types: dict[str, int] = {}
    tool_names: list[str] = []
    session_ids: list[str] = []
    parse_errors = 0
    event_count = 0
    result_text = ""
    result_is_error = False
    for line in stdout.splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            parse_errors += 1
            continue
        event_count += 1
        event_type = str(event.get("type") or event.get("event") or "unknown")
        event_types[event_type] = event_types.get(event_type, 0) + 1
        tool_names.extend(_collect_grok_tool_names(event))
        session_ids.extend(_collect_grok_session_ids(event))
        if isinstance(event.get("result"), str):
            result_text = event["result"]
        if isinstance(event.get("message"), str):
            result_text = event["message"]
        if event.get("is_error") is True or event.get("error"):
            result_is_error = True
    if event_count == 0 and stdout.strip():
        result_text = stdout[-4000:]
        lower = result_text.lower()
        result_is_error = "error" in lower or "not authenticated" in lower
    return {
        "event_count": event_count,
        "event_types": event_types,
        "tool_use_names": tool_names,
        "tool_use_count": len(tool_names),
        "session_ids": sorted(set(session_ids)),
        "json_parse_error_count": parse_errors,
        "result_is_error": result_is_error,
        "result_text": result_text,
    }


def run_grok(job: dict[str, Any], model: str) -> dict[str, Any]:
    default_grok_bin = Path("/Users/jay/.grok/bin/grok")
    grok_bin = os.environ.get("INFRING_GROK_BIN") or (
        str(default_grok_bin) if default_grok_bin.exists() else shutil.which("grok")
    )
    if not grok_bin or not Path(grok_bin).exists():
        return {"ok": False, "blocked": "cli_missing:grok", "wall_time_ms": None}
    requested_model = model
    grok_model = os.environ.get("INFRING_GROK_MODEL", model)
    use_default_model = os.environ.get("INFRING_GROK_USE_DEFAULT_MODEL", "").strip().lower() in {
        "1",
        "true",
        "yes",
    }
    if grok_model != requested_model and not use_default_model:
        return {
            "ok": False,
            "blocked": "grok_control_model_mismatch",
            "wall_time_ms": None,
            "model": grok_model,
            "requested_model": requested_model,
            "model_controlled": False,
            "model_routing_note": "Grok comparison requires the requested harness model. Set INFRING_GROK_MODEL equal to --model or unset it.",
        }
    unsupported_bridge = {
        "INFRING_GROK_CLI_CHAT_PROXY_BASE_URL": os.environ.get("INFRING_GROK_CLI_CHAT_PROXY_BASE_URL", "").strip(),
        "INFRING_GROK_XAI_API_BASE_URL": os.environ.get("INFRING_GROK_XAI_API_BASE_URL", "").strip(),
    }
    requested_bridge_keys = [key for key, value in unsupported_bridge.items() if value]
    if requested_bridge_keys:
        return {
            "ok": False,
            "blocked": "grok_single_turn_base_url_bridge_unsupported",
            "wall_time_ms": None,
            "model": grok_model,
            "requested_model": requested_model,
            "model_controlled": False,
            "model_routing_note": "Grok exposes base-url overrides on the agent subcommand, but the harness uses top-level --single for deterministic fixture runs; an agent stdio/headless bridge is needed before these env vars can be used.",
            "unsupported_bridge_env": requested_bridge_keys,
        }

    level2.ensure_temp_git_repo(job["project_root"])
    outputs = job["project_root"] / ".infring" / "system_outputs" / "grok"
    outputs.mkdir(parents=True, exist_ok=True)
    prompt_path = outputs / "prompt.txt"
    stdout_path = outputs / "stdout.streaming-jsonl"
    stderr_path = outputs / "stderr.txt"
    trace_export_path = outputs / "trace.tar.gz"
    prompt_path.write_text(
        job["prompt"]
        + "\n\nUse the existing project files in the current working directory."
        + f"\nRun this validation command before final response: {job['validation_command']}"
        + "\nRun the semantic probe too. Do not commit.",
        encoding="utf-8",
    )
    command = [
        grok_bin,
        "--cwd",
        str(job["project_root"]),
        "--output-format",
        "streaming-json",
        "--always-approve",
        "--permission-mode",
        "bypassPermissions",
        "--max-turns",
        os.environ.get("INFRING_GROK_MAX_TURNS", "128"),
        "--no-memory",
        "--no-plan",
        "--disable-web-search",
    ]
    if not use_default_model:
        command.extend(["--model", grok_model])
    sandbox = os.environ.get("INFRING_GROK_SANDBOX", "").strip()
    if sandbox:
        command.extend(["--sandbox", sandbox])
    command.extend(["--single", prompt_path.read_text(encoding="utf-8")])

    result = run_cmd(command, cwd=job["project_root"], timeout=300, env=os.environ | {"PYTHONPATH": "."})
    stdout_path.write_text(result.get("stdout") or "", encoding="utf-8")
    stderr_path.write_text(result.get("stderr") or "", encoding="utf-8")
    stream_summary = _summarize_grok_stream(result.get("stdout") or "")
    combined_tail = ((result.get("stdout_tail") or "") + "\n" + (result.get("stderr_tail") or "")).lower()
    auth_blocked = "not authenticated" in combined_tail or "sign in" in combined_tail
    model_unavailable = "unknown model" in combined_tail or (
        "model" in combined_tail and "not found" in combined_tail
    )
    blocked = None
    if auth_blocked and model_unavailable:
        blocked = "grok_not_authenticated_and_control_model_unavailable"
    elif auth_blocked:
        blocked = "grok_not_authenticated"
    elif model_unavailable:
        blocked = "grok_control_model_unavailable"

    trace_export_result: dict[str, Any] | None = None
    session_ids = stream_summary["session_ids"]
    if session_ids:
        trace_export_result = run_cmd(
            [
                grok_bin,
                "trace",
                "--local",
                "--json",
                "--output",
                str(trace_export_path),
                session_ids[-1],
            ],
            cwd=job["project_root"],
            timeout=60,
            env=os.environ,
        )

    return {
        "ok": result["ok"] and blocked is None and not stream_summary["result_is_error"],
        "blocked": blocked,
        "wall_time_ms": result["wall_time_ms"],
        "model": "grok-default" if use_default_model else grok_model,
        "requested_model": requested_model,
        "model_controlled": (grok_model == requested_model) and not use_default_model,
        "model_routing_note": "Grok was invoked without --model and used its configured default model; this is useful for runtime behavior tracing but not model-controlled."
        if use_default_model
        else "Grok was invoked with the requested harness control model through top-level --model. The top-level --single path does not accept Grok's agent-subcommand base-url overrides.",
        "auth_blocked": auth_blocked,
        "control_model_unavailable": model_unavailable,
        "cli_path": grok_bin,
        "error": stream_summary["result_text"]
        if stream_summary["result_is_error"]
        else result.get("stderr_tail")
        if not result["ok"]
        else None,
        "stdout_path": str(stdout_path),
        "stderr_path": str(stderr_path),
        "trace_export_path": str(trace_export_path) if trace_export_result and trace_export_path.exists() else None,
        "trace_export_result": trace_export_result,
        "timed_out": result.get("timed_out"),
        "stream_event_count": stream_summary["event_count"],
        "stream_event_types": stream_summary["event_types"],
        "tool_use_names": stream_summary["tool_use_names"],
        "tool_use_count": stream_summary["tool_use_count"],
        "stream_session_ids": session_ids,
        "stream_json_parse_error_count": stream_summary["json_parse_error_count"],
        "stream_result_is_error": stream_summary["result_is_error"],
    }


def _collect_codex_tool_names(value: Any) -> list[str]:
    names: list[str] = []
    if isinstance(value, dict):
        for key in ("tool_name", "tool", "name"):
            raw = value.get(key)
            if isinstance(raw, str) and raw in {
                "exec_command",
                "apply_patch",
                "file_read",
                "file_write",
                "file_patch",
                "shell",
                "command",
            }:
                names.append(raw)
        event_type = value.get("type")
        if isinstance(event_type, str) and (
            "exec" in event_type or "patch" in event_type or "tool" in event_type
        ):
            names.append(event_type)
        for child in value.values():
            names.extend(_collect_codex_tool_names(child))
    elif isinstance(value, list):
        for child in value:
            names.extend(_collect_codex_tool_names(child))
    return names


def _summarize_codex_jsonl(stdout: str) -> dict[str, Any]:
    event_types: dict[str, int] = {}
    tool_names: list[str] = []
    parse_errors = 0
    event_count = 0
    for line in stdout.splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            parse_errors += 1
            continue
        event_count += 1
        event_type = str(event.get("type") or event.get("event") or "unknown")
        event_types[event_type] = event_types.get(event_type, 0) + 1
        tool_names.extend(_collect_codex_tool_names(event))
    return {
        "event_count": event_count,
        "event_types": event_types,
        "tool_use_names": tool_names,
        "tool_use_count": len(tool_names),
        "json_parse_error_count": parse_errors,
    }


def _codex_oss_provider_for_model(model: str) -> str | None:
    forced = os.environ.get("INFRING_CODEX_LOCAL_PROVIDER", "").strip()
    if forced:
        return forced
    lower = model.lower()
    local_markers = (
        ":",
        "kimi",
        "qwen",
        "llama",
        "mistral",
        "mixtral",
        "deepseek",
        "codellama",
        "gemma",
        "starcoder",
    )
    hosted_prefixes = ("gpt-", "o1", "o3", "o4", "o5")
    if lower.startswith(hosted_prefixes):
        return None
    if any(marker in lower for marker in local_markers):
        return "ollama"
    return None


def run_codex(job: dict[str, Any], model: str) -> dict[str, Any]:
    codex_bin = shutil.which("codex")
    if not codex_bin:
        fallback = Path("/Applications/Codex.app/Contents/Resources/codex")
        codex_bin = str(fallback) if fallback.exists() else ""
    if not codex_bin:
        return {"ok": False, "blocked": "cli_missing:codex", "wall_time_ms": None}
    requested_model = model
    codex_model = os.environ.get("INFRING_CODEX_MODEL", model)
    outputs = job["project_root"] / ".infring" / "system_outputs" / "codex"
    outputs.mkdir(parents=True, exist_ok=True)
    prompt_path = outputs / "prompt.txt"
    stdout_path = outputs / "stdout.jsonl"
    stderr_path = outputs / "stderr.txt"
    last_message_path = outputs / "last-message.txt"
    prompt_path.write_text(
        job["prompt"]
        + "\n\nUse the existing project files in the current working directory."
        + f"\nRun this validation command before final response: {job['validation_command']}"
        + "\nRun the semantic probe too. Do not commit.",
        encoding="utf-8",
    )
    local_provider = _codex_oss_provider_for_model(codex_model)
    provider_args = ["--oss", f"--local-provider={local_provider}"] if local_provider else []
    result = run_cmd(
        [
            codex_bin,
            "exec",
            "--json",
            "--ephemeral",
            "--skip-git-repo-check",
            "--dangerously-bypass-approvals-and-sandbox",
            "--sandbox=danger-full-access",
            *provider_args,
            f"--model={codex_model}",
            f"--cd={job['project_root']}",
            f"--output-last-message={last_message_path}",
            prompt_path.read_text(encoding="utf-8"),
        ],
        cwd=job["project_root"],
        timeout=300,
        env=os.environ | {"PYTHONPATH": "."},
    )
    stdout_path.write_text(result.get("stdout") or "", encoding="utf-8")
    stderr_path.write_text(result.get("stderr") or "", encoding="utf-8")
    stream_summary = _summarize_codex_jsonl(result.get("stdout") or "")
    return {
        "ok": result["ok"],
        "wall_time_ms": result["wall_time_ms"],
        "model": codex_model,
        "requested_model": requested_model,
        "codex_local_provider": local_provider,
        "model_routing_note": None
        if codex_model == requested_model
        else "Codex model routed through INFRING_CODEX_MODEL.",
        "cli_path": codex_bin,
        "error": result.get("stderr_tail") if not result["ok"] else None,
        "stdout_path": str(stdout_path),
        "stderr_path": str(stderr_path),
        "last_message_path": str(last_message_path),
        "timed_out": result.get("timed_out"),
        "stream_event_count": stream_summary["event_count"],
        "stream_event_types": stream_summary["event_types"],
        "tool_use_names": stream_summary["tool_use_names"],
        "tool_use_count": stream_summary["tool_use_count"],
        "stream_json_parse_error_count": stream_summary["json_parse_error_count"],
    }


RUNNERS = {
    "infring": run_infring,
    "mini-swe-agent": run_mini_swe_agent,
    "aider": run_aider,
    "swe-agent": run_swe_agent,
    "forgecode": run_forgecode,
    "claude-code": run_claude_code,
    "claude": run_claude_code,
    "codex": run_codex,
    "codex-cli": run_codex,
    "grok": run_grok,
    "grok-build": run_grok,
}


def summarize(attempts: list[dict[str, Any]]) -> list[dict[str, Any]]:
    out: list[dict[str, Any]] = []
    for system in sorted({attempt["system"] for attempt in attempts}):
        for level in sorted({attempt["level"] for attempt in attempts if attempt["system"] == system}):
            subset = [attempt for attempt in attempts if attempt["system"] == system and attempt["level"] == level]
            times = [attempt["wall_time_ms"] for attempt in subset if isinstance(attempt.get("wall_time_ms"), int)]
            first_mutation_times = [
                attempt["time_to_first_mutation_ms"]
                for attempt in subset
                if isinstance(attempt.get("time_to_first_mutation_ms"), int)
            ]
            out.append({
                "system": system,
                "level": level,
                "attempt_count": len(subset),
                "pass_count": sum(1 for attempt in subset if attempt["ok"]),
                "fail_count": sum(1 for attempt in subset if not attempt["ok"]),
                "average_wall_time_ms": round(sum(times) / len(times)) if times else None,
                "median_wall_time_ms": percentile(times, 0.5),
                "p90_wall_time_ms": percentile(times, 0.9),
                "average_time_to_first_mutation_ms": round(sum(first_mutation_times) / len(first_mutation_times)) if first_mutation_times else None,
                "median_time_to_first_mutation_ms": percentile(first_mutation_times, 0.5),
                "p90_time_to_first_mutation_ms": percentile(first_mutation_times, 0.9),
                "failure_classes": failure_counts(subset),
            })
    return out


def percentile(values: list[int], fraction: float) -> int | None:
    if not values:
        return None
    ordered = sorted(values)
    index = round((len(ordered) - 1) * fraction)
    return ordered[index]


def failure_counts(attempts: list[dict[str, Any]]) -> dict[str, int]:
    counts: dict[str, int] = {}
    for attempt in attempts:
        key = attempt.get("failure_class") or "pass"
        counts[key] = counts.get(key, 0) + 1
    return counts


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--model", default=DEFAULT_MODEL)
    parser.add_argument("--systems", default="infring,mini-swe-agent,aider,swe-agent,forgecode")
    parser.add_argument("--levels", default="3,4")
    parser.add_argument("--out", default="")
    args = parser.parse_args()

    requested_systems = [item.strip() for item in args.systems.split(",") if item.strip()]
    requested_levels = {int(item.strip()) for item in args.levels.split(",") if item.strip()}
    attempts: list[dict[str, Any]] = []
    for case in [case for case in cases() if case["level"] in requested_levels]:
        for system in requested_systems:
            job = seed_case(case, system.replace("/", "_"))
            runner = RUNNERS.get(system)
            if runner is None:
                run_result = {"ok": False, "blocked": "unknown_system", "wall_time_ms": None}
            else:
                run_result = runner(job, args.model)
            attempts.append(judge(system, job, run_result))

    report = {
        "harness_kind": "level3_level4_live_baseline_v1",
        "generated_at": utc_now(),
        "model": args.model,
        "systems": requested_systems,
        "levels": sorted(requested_levels),
        "summary": summarize(attempts),
        "attempts": attempts,
    }
    out_path = Path(args.out) if args.out else Path(tempfile.mkdtemp(prefix="level3-level4-baseline-")) / "report.json"
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
