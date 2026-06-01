#!/usr/bin/env node
/* eslint-disable no-console */
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { join } from "node:path";

const exe = process.platform === "win32" ? ".exe" : "";
const separator = process.argv.indexOf("--");
const args = separator >= 0 ? process.argv.slice(separator + 1) : process.argv.slice(2);
const candidates = [
  process.env.INFRING_OPS_BIN || "",
  join(process.cwd(), "target", "release", `infring-ops${exe}`),
  join(process.cwd(), "target", "debug", `infring-ops${exe}`),
  join(process.cwd(), "core", "layer0", "ops", "target", "release", `infring-ops${exe}`),
  join(process.cwd(), "core", "layer0", "ops", "target", "debug", `infring-ops${exe}`),
].filter(Boolean);

const binary = candidates.find((candidate) => existsSync(candidate));
if (!binary) {
  console.error(
    JSON.stringify({
      ok: false,
      type: "infring_ops_binary_command",
      error: "infring_ops_binary_missing",
      candidates,
      next_action: "build infring-ops before running health checks",
    }),
  );
  process.exit(2);
}

if (args.length === 0) {
  console.error(
    JSON.stringify({
      ok: false,
      type: "infring_ops_binary_command",
      error: "missing_command_args",
    }),
  );
  process.exit(64);
}

const child = spawn(binary, args, {
  cwd: process.cwd(),
  env: process.env,
  stdio: "inherit",
});

child.on("error", (error) => {
  console.error(
    JSON.stringify({
      ok: false,
      type: "infring_ops_binary_command",
      error: String(error),
      binary,
    }),
  );
  process.exit(1);
});

child.on("exit", (code, signal) => {
  if (signal) process.exit(124);
  process.exit(code ?? 1);
});
