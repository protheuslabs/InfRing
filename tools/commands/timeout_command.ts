#!/usr/bin/env node
/* eslint-disable no-console */
import { spawn } from "node:child_process";

function readFlag(name: string, fallback = ""): string {
  const exact = `--${name}`;
  const prefix = `${exact}=`;
  for (let idx = 2; idx < process.argv.length; idx += 1) {
    const arg = process.argv[idx] || "";
    if (arg === exact) return process.argv[idx + 1] || fallback;
    if (arg.startsWith(prefix)) return arg.slice(prefix.length);
  }
  return fallback;
}

function readEnvAssignments(): Record<string, string> {
  const assignments: Record<string, string> = {};
  const separator = process.argv.indexOf("--");
  const end = separator >= 0 ? separator : process.argv.length;
  for (let idx = 2; idx < end; idx += 1) {
    const arg = process.argv[idx] || "";
    if (arg === "--env") {
      const assignment = process.argv[idx + 1] || "";
      idx += 1;
      const split = assignment.indexOf("=");
      if (split > 0) {
        assignments[assignment.slice(0, split)] = assignment.slice(split + 1);
      }
      continue;
    }
    if (arg.startsWith("--env=")) {
      const assignment = arg.slice("--env=".length);
      const split = assignment.indexOf("=");
      if (split > 0) {
        assignments[assignment.slice(0, split)] = assignment.slice(split + 1);
      }
    }
  }
  return assignments;
}

const separator = process.argv.indexOf("--");
const timeoutMs = Math.max(1000, Number(readFlag("timeout-ms", "30000")) || 30000);
const requestedCwd = readFlag("cwd", process.cwd());
const envAssignments = readEnvAssignments();
const command = separator >= 0 ? process.argv.slice(separator + 1) : [];

if (command.length === 0) {
  console.error("timeout_command_missing_command");
  process.exit(64);
}

const started = Date.now();
const child = spawn(command[0], command.slice(1), {
  cwd: requestedCwd || process.cwd(),
  detached: process.platform !== "win32",
  env: { ...process.env, ...envAssignments },
  stdio: "inherit",
});

let finished = false;
const timer = setTimeout(() => {
  if (finished) return;
  const elapsed = Date.now() - started;
  console.error(
    JSON.stringify({
      ok: false,
      type: "timeout_command",
      timed_out: true,
      timeout_ms: timeoutMs,
      elapsed_ms: elapsed,
      cwd: requestedCwd || process.cwd(),
      command: command.join(" "),
    }),
  );
  if (process.platform === "win32") {
    child.kill("SIGTERM");
  } else {
    try {
      process.kill(-child.pid, "SIGTERM");
    } catch {
      child.kill("SIGTERM");
    }
  }
  setTimeout(() => {
    if (finished) return;
    if (process.platform === "win32") {
      child.kill("SIGKILL");
    } else {
      try {
        process.kill(-child.pid, "SIGKILL");
      } catch {
        child.kill("SIGKILL");
      }
    }
  }, 2000).unref();
}, timeoutMs);

child.on("error", (error) => {
  finished = true;
  clearTimeout(timer);
  console.error(
    JSON.stringify({
      ok: false,
      type: "timeout_command",
      error: String(error),
      cwd: requestedCwd || process.cwd(),
      command: command.join(" "),
    }),
  );
  process.exit(1);
});

child.on("exit", (code, signal) => {
  finished = true;
  clearTimeout(timer);
  if (signal) {
    process.exit(124);
  }
  process.exit(code ?? 1);
});
