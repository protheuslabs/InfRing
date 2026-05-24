#!/usr/bin/env python3
"""Policy-aware command resolution for coding-system trace harnesses.

The harnesses compare runtime behavior across systems, so launch mechanics are
part of the measurement. This module keeps that concern out of task fixtures:
prefer real binaries for eval/comparison runs, allow workspace binaries when
they are already built, and only use cargo-run as an explicit dev fallback.
"""

from __future__ import annotations

import os
import shutil
from pathlib import Path
from typing import Any


STRICT_POLICIES = {"ci", "comparison", "eval", "production"}
DEV_POLICIES = {"debug", "dev", "development", "local-dev"}


def command_execution_policy() -> str:
    return os.environ.get("INFRING_COMMAND_EXECUTION_POLICY", "comparison").strip().lower() or "comparison"


def _truthy_env(name: str) -> bool:
    return os.environ.get(name, "").strip().lower() in {"1", "true", "yes", "on"}


def cargo_run_allowed(policy: str) -> bool:
    if _truthy_env("INFRING_ALLOW_CARGO_RUN_FALLBACK"):
        return True
    if policy in DEV_POLICIES:
        return True
    return policy not in STRICT_POLICIES


def _mtime_ms(path: Path) -> int | None:
    try:
        return round(path.stat().st_mtime * 1000)
    except OSError:
        return None


def _is_workspace_binary(path: Path, workspace_roots: list[Path]) -> bool:
    resolved = path.resolve()
    if "target" in resolved.parts and ("debug" in resolved.parts or "release" in resolved.parts):
        return True
    for root in workspace_roots:
        try:
            resolved.relative_to(root.resolve())
            return True
        except ValueError:
            continue
    return False


def _binary_receipt(
    *,
    tool_name: str,
    policy: str,
    command: list[str],
    resolved_path: Path,
    mode: str,
    fallback_chain: list[str],
    source: str,
) -> dict[str, Any]:
    return {
        "ok": True,
        "tool_name": tool_name,
        "policy": policy,
        "execution_mode": mode,
        "resolved_executable": str(resolved_path),
        "resolved_source": source,
        "fallback_chain": fallback_chain,
        "fallback_reason": None,
        "timing_comparable": mode in {"installed_binary", "workspace_binary"},
        "binary_exists": True,
        "binary_mtime_unix_ms": _mtime_ms(resolved_path),
        "cargo_run_used": False,
        "command_preview": command[:2],
    }


def _cargo_receipt(
    *,
    tool_name: str,
    policy: str,
    command: list[str],
    fallback_chain: list[str],
    fallback_reason: str,
) -> dict[str, Any]:
    return {
        "ok": True,
        "tool_name": tool_name,
        "policy": policy,
        "execution_mode": "cargo_run_dev_fallback",
        "resolved_executable": shutil.which("cargo") or "cargo",
        "resolved_source": "cargo_run_dev_fallback",
        "fallback_chain": fallback_chain,
        "fallback_reason": fallback_reason,
        "timing_comparable": False,
        "binary_exists": bool(shutil.which("cargo")),
        "binary_mtime_unix_ms": None,
        "cargo_run_used": True,
        "command_preview": command[:6],
    }


def _blocked_receipt(
    *,
    tool_name: str,
    policy: str,
    fallback_chain: list[str],
    fallback_reason: str,
) -> dict[str, Any]:
    return {
        "ok": False,
        "tool_name": tool_name,
        "policy": policy,
        "execution_mode": "missing",
        "resolved_executable": None,
        "resolved_source": None,
        "fallback_chain": fallback_chain,
        "fallback_reason": fallback_reason,
        "timing_comparable": False,
        "binary_exists": False,
        "binary_mtime_unix_ms": None,
        "cargo_run_used": False,
        "command_preview": [],
    }


def _resolve_binary_candidates(
    *,
    tool_name: str,
    policy: str,
    candidates: list[dict[str, Any]],
    workspace_roots: list[Path],
    cargo_command: list[str] | None,
) -> dict[str, Any]:
    fallback_chain: list[str] = []
    for candidate in candidates:
        source = str(candidate["source"])
        raw_path = candidate.get("path")
        if not raw_path:
            fallback_chain.append(f"{source}:unset")
            continue
        path = Path(str(raw_path)).expanduser()
        if not path.exists():
            fallback_chain.append(f"{source}:missing:{path}")
            continue
        if not path.is_file():
            fallback_chain.append(f"{source}:not_file:{path}")
            continue
        command = [str(path), *candidate.get("suffix", [])]
        mode = "workspace_binary" if _is_workspace_binary(path, workspace_roots) else "installed_binary"
        fallback_chain.append(f"{source}:selected:{mode}")
        return {
            "ok": True,
            "command": command,
            "receipt": _binary_receipt(
                tool_name=tool_name,
                policy=policy,
                command=command,
                resolved_path=path,
                mode=mode,
                fallback_chain=fallback_chain,
                source=source,
            ),
        }

    if cargo_command and cargo_run_allowed(policy):
        reason = "binary_missing_using_dev_fallback"
        fallback_chain.append("cargo_run_dev_fallback:selected")
        return {
            "ok": True,
            "command": cargo_command,
            "receipt": _cargo_receipt(
                tool_name=tool_name,
                policy=policy,
                command=cargo_command,
                fallback_chain=fallback_chain,
                fallback_reason=reason,
            ),
        }

    reason = f"binary_missing:cargo_run_forbidden_by_policy:{policy}"
    fallback_chain.append("cargo_run_dev_fallback:forbidden")
    return {
        "ok": False,
        "command": [],
        "blocked": reason,
        "receipt": _blocked_receipt(
            tool_name=tool_name,
            policy=policy,
            fallback_chain=fallback_chain,
            fallback_reason=reason,
        ),
    }


def resolve_xtask_command(repo_root: Path, *, policy: str | None = None) -> dict[str, Any]:
    policy = policy or command_execution_policy()
    candidates: list[dict[str, Any]] = []
    direct_agent = os.environ.get("INFRING_AGENT_RUN_BIN", "").strip()
    if direct_agent:
        candidates.append({"source": "env:INFRING_AGENT_RUN_BIN", "path": direct_agent, "suffix": []})
    xtask_env = os.environ.get("INFRING_XTASK_BIN", "").strip()
    if xtask_env:
        candidates.append({"source": "env:INFRING_XTASK_BIN", "path": xtask_env, "suffix": ["infring-agent-run"]})
    xtask_on_path = shutil.which("xtask")
    if xtask_on_path:
        candidates.append({"source": "path:xtask", "path": xtask_on_path, "suffix": ["infring-agent-run"]})
    candidates.append({"source": "workspace:target/debug/xtask", "path": repo_root / "target/debug/xtask", "suffix": ["infring-agent-run"]})
    cargo_command = ["cargo", "run", "--quiet", "-p", "xtask", "--", "infring-agent-run"]
    return _resolve_binary_candidates(
        tool_name="infring_xtask",
        policy=policy,
        candidates=candidates,
        workspace_roots=[repo_root, Path("/tmp")],
        cargo_command=cargo_command,
    )


def resolve_forge_command(repo_root: Path, forge_root: Path, *, policy: str | None = None) -> dict[str, Any]:
    policy = policy or command_execution_policy()
    candidates: list[dict[str, Any]] = []
    for env_name in ("INFRING_FORGECODE_BIN", "FORGECODE_BIN", "FORGE_BIN"):
        raw = os.environ.get(env_name, "").strip()
        if raw:
            candidates.append({"source": f"env:{env_name}", "path": raw, "suffix": []})
    for binary_name in ("forge", "forgecode"):
        found = shutil.which(binary_name)
        if found:
            candidates.append({"source": f"path:{binary_name}", "path": found, "suffix": []})
    candidates.extend(
        [
            {"source": "workspace:forgecode/target/debug/forge", "path": forge_root / "target/debug/forge", "suffix": []},
            {"source": "workspace:target/debug/forge", "path": repo_root / "target/debug/forge", "suffix": []},
        ]
    )
    cargo_command = [
        "cargo",
        "run",
        "--quiet",
        "--manifest-path",
        str(forge_root / "Cargo.toml"),
        "--package",
        "forge_main",
        "--bin",
        "forge",
        "--",
    ]
    return _resolve_binary_candidates(
        tool_name="forgecode",
        policy=policy,
        candidates=candidates,
        workspace_roots=[forge_root, repo_root, Path("/tmp")],
        cargo_command=cargo_command,
    )
