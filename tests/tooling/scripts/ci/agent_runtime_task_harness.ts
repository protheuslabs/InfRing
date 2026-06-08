#!/usr/bin/env tsx

import { spawn, spawnSync } from "child_process";
import * as crypto from "crypto";
import * as fs from "fs";
import * as os from "os";
import * as path from "path";

type AnyJson = Record<string, any>;

type HarnessArgs = {
  catalog: string;
  tasks: string;
  contract: string;
  framework: string;
  task: string;
  mode: string;
  live: boolean;
  gatewayUrl: string;
  outDir: string;
  frameworkRoot: string;
  timeoutMs: number;
};

type RunOutcome = {
  attempted: boolean;
  available: boolean;
  ok: boolean;
  status: "planned" | "completed" | "failed" | "skipped";
  summary: string;
  command?: string[];
  stdin?: string;
  uses_stdin?: boolean;
  endpoint?: string;
  stdout_ref?: string | null;
  stderr_ref?: string | null;
  response_ref?: string | null;
  duration_ms?: number;
};

const REPO_ROOT = process.cwd();
const DEFAULT_CATALOG = "validation/agent_runtime/task_harness/framework_capability_catalog.json";
const DEFAULT_TASKS = "validation/agent_runtime/task_harness/agentic_task_matrix.json";
const DEFAULT_CONTRACT = "validation/agent_runtime/task_harness/agent_runtime_task_harness_contract.json";
const DEFAULT_ARTIFACT_ROOT = "core/local/artifacts/agent-runtime-task-harness";

function usage(): string {
  return [
    "Agent Runtime Task Harness",
    "",
    "Dry-run catalog coverage:",
    "  node client/runtime/lib/ts_entrypoint.ts tests/tooling/scripts/ci/agent_runtime_task_harness.ts --mode=catalog",
    "",
    "Dry-run native/socket plans for one framework:",
    "  node client/runtime/lib/ts_entrypoint.ts tests/tooling/scripts/ci/agent_runtime_task_harness.ts --mode=both --framework=codex_cli",
    "",
    "Live run:",
    "  INFRING_AGENT_RUNTIME_TASK_HARNESS_LIVE=1 node client/runtime/lib/ts_entrypoint.ts tests/tooling/scripts/ci/agent_runtime_task_harness.ts --mode=both --framework=codex_cli",
    "",
    "Flags:",
    "  --framework=<id|all>",
    "  --task=<id|all>",
    "  --mode=<native|infring|both|catalog>",
    "  --gateway-url=http://127.0.0.1:4173",
    "  --framework-root=/path/to/framework",
    "  --out-dir=core/local/artifacts/agent-runtime-task-harness/custom",
    "  --live=1"
  ].join("\n");
}

function parseArgs(argv: string[]): HarnessArgs {
  const raw: Record<string, string | boolean> = {};
  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index];
    if (!token.startsWith("--")) {
      continue;
    }
    const trimmed = token.slice(2);
    const eq = trimmed.indexOf("=");
    if (eq >= 0) {
      raw[trimmed.slice(0, eq)] = trimmed.slice(eq + 1);
      continue;
    }
    const next = argv[index + 1];
    if (next && !next.startsWith("--")) {
      raw[trimmed] = next;
      index += 1;
    } else {
      raw[trimmed] = true;
    }
  }

  const liveRaw = String(raw.live ?? process.env.INFRING_AGENT_RUNTIME_TASK_HARNESS_LIVE ?? "0");
  const runId = new Date().toISOString().replace(/[^0-9]/g, "").slice(0, 14);
  return {
    catalog: String(raw.catalog ?? DEFAULT_CATALOG),
    tasks: String(raw.tasks ?? DEFAULT_TASKS),
    contract: String(raw.contract ?? DEFAULT_CONTRACT),
    framework: String(raw.framework ?? "all"),
    task: String(raw.task ?? "all"),
    mode: String(raw.mode ?? "catalog"),
    live: liveRaw === "1" || liveRaw === "true",
    gatewayUrl: String(raw["gateway-url"] ?? process.env.INFRING_GATEWAY_URL ?? "http://127.0.0.1:4173"),
    outDir: String(raw["out-dir"] ?? path.join(DEFAULT_ARTIFACT_ROOT, runId)),
    frameworkRoot: String(raw["framework-root"] ?? process.env.INFRING_AGENT_RUNTIME_FRAMEWORK_ROOT ?? ""),
    timeoutMs: Number(raw["timeout-ms"] ?? process.env.INFRING_AGENT_RUNTIME_TASK_TIMEOUT_MS ?? 120000)
  };
}

function readJson(relativePath: string): AnyJson {
  const fullPath = path.resolve(REPO_ROOT, relativePath);
  return JSON.parse(fs.readFileSync(fullPath, "utf8"));
}

function ensureDir(fullPath: string): void {
  fs.mkdirSync(fullPath, { recursive: true });
}

function writeJson(fullPath: string, value: any): void {
  ensureDir(path.dirname(fullPath));
  fs.writeFileSync(fullPath, `${JSON.stringify(value, null, 2)}\n`);
}

function writeText(fullPath: string, value: string): void {
  ensureDir(path.dirname(fullPath));
  fs.writeFileSync(fullPath, value.endsWith("\n") ? value : `${value}\n`);
}

function commandExists(command: string): boolean {
  const result = spawnSync("sh", ["-lc", `command -v ${shellQuote(command)} >/dev/null 2>&1`], {
    cwd: REPO_ROOT,
    stdio: "ignore"
  });
  return result.status === 0;
}

function firstAvailableCommand(commands: string[] = []): string | null {
  for (const command of commands) {
    if (commandExists(command)) {
      return command;
    }
  }
  return null;
}

function shellQuote(value: string): string {
  return `'${String(value).replace(/'/g, `'\\''`)}'`;
}

function resolveToken(value: string, context: Record<string, string>): string {
  return String(value).replace(/\{([a-zA-Z0-9_]+)\}/g, (_match, key) => context[key] ?? "");
}

function resolveListTemplate(values: string[], context: Record<string, string>): string[] {
  return values.map((value) => resolveToken(value, context)).filter((value) => value.length > 0);
}

function selectedRows(rows: AnyJson[], selected: string, idField = "id"): AnyJson[] {
  if (!selected || selected === "all") {
    return rows;
  }
  const wanted = new Set(selected.split(",").map((value) => value.trim()).filter(Boolean));
  return rows.filter((row) => wanted.has(String(row[idField])));
}

function taskPrompt(task: AnyJson): string {
  const turns = Array.isArray(task.turns) ? task.turns : [];
  return turns.join("\n\n--- next turn ---\n\n");
}

function makeHarnessWorkspace(outDir: string, frameworkId: string, taskId: string, task: AnyJson): string {
  const safeId = `${frameworkId}-${taskId}`.replace(/[^a-zA-Z0-9_.-]/g, "_");
  const workDir = path.resolve(REPO_ROOT, outDir, "workdirs", safeId);
  ensureDir(workDir);
  ensureDir(path.join(workDir, "output"));
  ensureDir(path.join(workDir, "input"));
  ensureDir(path.join(workDir, "fixture"));
  for (const fixture of Array.isArray(task.fixture_files) ? task.fixture_files : []) {
    const fixturePath = path.resolve(workDir, String(fixture.path));
    if (!fixturePath.startsWith(workDir)) {
      throw new Error(`fixture path escapes harness workspace: ${fixture.path}`);
    }
    writeText(fixturePath, String(fixture.content ?? ""));
  }
  if (task.large_context) {
    const largePath = path.resolve(workDir, String(task.large_context.path ?? "input/pastedtext.txt"));
    if (!largePath.startsWith(workDir)) {
      throw new Error(`large context path escapes harness workspace: ${task.large_context.path}`);
    }
    const repeat = Math.max(1, Number(task.large_context.repeat ?? 1));
    const prefix = String(task.large_context.content_prefix ?? "");
    writeText(largePath, new Array(repeat).fill(prefix).join("\n"));
  }
  return workDir;
}

function buildNativePlan(framework: AnyJson, task: AnyJson, workDir: string, args: HarnessArgs): RunOutcome {
  const native = framework.native_invocation ?? {};
  if (!native.supported) {
    return {
      attempted: false,
      available: false,
      ok: true,
      status: "skipped",
      summary: String(native.reason ?? "Native invocation is not supported for this framework.")
    };
  }
  const command = firstAvailableCommand(native.command_candidates ?? []);
  const prompt = taskPrompt(task);
  const context = {
    command: command ?? String((native.command_candidates ?? [framework.id])[0] ?? framework.id),
    cwd: workDir,
    prompt,
    repo: REPO_ROOT,
    framework_root: args.frameworkRoot
  };
  const argv = resolveListTemplate(native.argv_template ?? [], context);
  const stdin = native.stdin_template
    ? resolveToken(String(native.stdin_template), context)
    : "";
  return {
    attempted: true,
    available: Boolean(command),
    ok: Boolean(command),
    status: args.live ? "planned" : "planned",
    summary: command
      ? "Native command is available and planned."
      : `No native command found from candidates: ${(native.command_candidates ?? []).join(", ")}`,
    command: argv,
    stdin,
    uses_stdin: Boolean(stdin)
  };
}

function buildInfringPlan(framework: AnyJson, task: AnyJson, args: HarnessArgs): RunOutcome {
  const endpoint = `${args.gatewayUrl.replace(/\/+$/, "")}/api/shell-socket/agent-runtime/turn`;
  return {
    attempted: true,
    available: true,
    ok: true,
    status: "planned",
    summary: `InfRing socket turn planned for ${framework.socket_engine_id ?? framework.id}.`,
    endpoint
  };
}

function runCommand(argv: string[], cwd: string, timeoutMs: number, stdinText = ""): Promise<{ ok: boolean; stdout: string; stderr: string; duration_ms: number; exit_code: number | null; signal: NodeJS.Signals | null; timed_out: boolean }> {
  const started = Date.now();
  return new Promise((resolve) => {
    if (argv.length === 0) {
      resolve({ ok: false, stdout: "", stderr: "empty command", duration_ms: 0, exit_code: null, signal: null, timed_out: false });
      return;
    }
    const child = spawn(argv[0], argv.slice(1), {
      cwd,
      env: process.env,
      shell: false
    });
    let stdout = "";
    let stderr = "";
    let timedOut = false;
    const timer = setTimeout(() => {
      timedOut = true;
      child.kill("SIGTERM");
    }, timeoutMs);
    child.stdout.on("data", (chunk) => {
      stdout += String(chunk);
    });
    child.stderr.on("data", (chunk) => {
      stderr += String(chunk);
    });
    if (stdinText) {
      child.stdin.write(stdinText);
    }
    child.stdin.end();
    child.on("close", (code, signal) => {
      clearTimeout(timer);
      resolve({
        ok: code === 0 && !timedOut,
        stdout,
        stderr,
        duration_ms: Date.now() - started,
        exit_code: code,
        signal,
        timed_out: timedOut
      });
    });
    child.on("error", (error) => {
      clearTimeout(timer);
      resolve({
        ok: false,
        stdout,
        stderr: `${stderr}\n${error.message}`.trim(),
        duration_ms: Date.now() - started,
        exit_code: null,
        signal: null,
        timed_out: timedOut
      });
    });
  });
}

async function postJson(url: string, body: any, timeoutMs: number): Promise<{ ok: boolean; status: number; text: string; duration_ms: number }> {
  const fetchFn = (globalThis as any).fetch;
  if (typeof fetchFn !== "function") {
    return { ok: false, status: 0, text: "global fetch is unavailable in this Node runtime", duration_ms: 0 };
  }
  const started = Date.now();
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try {
    const response = await fetchFn(url, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
      signal: controller.signal
    });
    const text = await response.text();
    return { ok: response.ok, status: response.status, text, duration_ms: Date.now() - started };
  } catch (error: any) {
    return { ok: false, status: 0, text: String(error?.message ?? error), duration_ms: Date.now() - started };
  } finally {
    clearTimeout(timer);
  }
}

async function executeNative(plan: RunOutcome, workDir: string, outDir: string, frameworkId: string, taskId: string, timeoutMs: number, live: boolean): Promise<RunOutcome> {
  if (!plan.attempted || !plan.available || !plan.command) {
    return plan;
  }
  if (!live) {
    const { stdin: _stdin, ...safePlan } = plan;
    return {
      ...safePlan,
      ok: true,
      status: "planned",
      summary: `Dry-run native command: ${plan.command.join(" ")}${plan.uses_stdin ? " < prompt via stdin" : ""}`
    };
  }
  const result = await runCommand(plan.command, workDir, timeoutMs, plan.stdin ?? "");
  const stdoutPath = path.resolve(REPO_ROOT, outDir, "native", `${frameworkId}-${taskId}.stdout.txt`);
  const stderrPath = path.resolve(REPO_ROOT, outDir, "native", `${frameworkId}-${taskId}.stderr.txt`);
  writeText(stdoutPath, result.stdout);
  writeText(stderrPath, result.stderr);
  const { stdin: _stdin, ...safePlan } = plan;
  return {
    ...safePlan,
    ok: result.ok,
    status: result.ok ? "completed" : "failed",
    summary: result.ok
      ? "Native framework run completed."
      : `Native framework run failed${result.timed_out ? " after timeout" : ""}.`,
    stdout_ref: path.relative(REPO_ROOT, stdoutPath),
    stderr_ref: path.relative(REPO_ROOT, stderrPath),
    duration_ms: result.duration_ms
  };
}

async function executeInfring(plan: RunOutcome, framework: AnyJson, task: AnyJson, workDir: string, outDir: string, frameworkId: string, taskId: string, args: HarnessArgs): Promise<RunOutcome> {
  if (!plan.attempted) {
    return plan;
  }
  const payload = {
    type: "agent_runtime_task_harness_turn",
    harness_version: 1,
    engine_id: framework.socket_engine_id ?? framework.id,
    framework_id: framework.id,
    session_id: `harness-${framework.id}-${task.id}`,
    working_directory: workDir,
    message: taskPrompt(task),
    task_id: task.id,
    approval_policy: task.approval_policy ?? "none",
    expected_capabilities: task.expected_capabilities ?? [],
    large_context_ref: task.large_context?.path ?? null
  };
  if (!args.live) {
    return {
      ...plan,
      ok: true,
      status: "planned",
      summary: `Dry-run InfRing socket POST planned for ${payload.engine_id}.`
    };
  }
  const result = await postJson(String(plan.endpoint), payload, args.timeoutMs);
  const responsePath = path.resolve(REPO_ROOT, outDir, "infring", `${frameworkId}-${taskId}.response.txt`);
  writeText(responsePath, result.text);
  let projectionOk = result.ok;
  let projectionStatus = "";
  let projectionReason = "";
  try {
    const projection = JSON.parse(result.text);
    if (projection && typeof projection === "object") {
      if (projection.ok === false) {
        projectionOk = false;
      }
      projectionStatus = String(projection.status ?? "");
      projectionReason = String(projection.reason || projection.error_code || projection.text || "");
    }
  } catch {
    projectionReason = result.text.slice(0, 240);
  }
  return {
    ...plan,
    ok: projectionOk,
    status: projectionOk ? "completed" : "failed",
    summary: projectionOk
      ? `InfRing socket run completed with HTTP ${result.status}.`
      : `InfRing socket run failed with HTTP ${result.status}${projectionStatus ? ` (${projectionStatus})` : ""}${projectionReason ? `: ${projectionReason}` : ""}.`,
    response_ref: path.relative(REPO_ROOT, responsePath),
    duration_ms: result.duration_ms
  };
}

function scoreResult(task: AnyJson, native: RunOutcome, infring: RunOutcome, mode: string, live: boolean): AnyJson {
  const expected = new Set<string>(Array.isArray(task.expected_capabilities) ? task.expected_capabilities : []);
  const unknown = live ? "unknown" : "unknown";
  const score: AnyJson = {
    context_continuity: expected.has("conversation_context") || expected.has("multi_turn_continuity") ? unknown : "pass",
    activity_trace: expected.has("activity_dialog_stream") || expected.has("decision_dialog") || expected.has("tool_trace_visibility") ? unknown : "pass",
    approval_flow: expected.has("approval_pause_resume") || expected.has("permission_request") ? unknown : "pass",
    artifact_effect: expected.has("file_write") || expected.has("artifact_receipt") ? unknown : "pass",
    model_control: expected.has("model_selection") || expected.has("provider_identity") ? unknown : "pass",
    failure_reporting: expected.has("failure_reporting") || expected.has("hard_failure_chat_injection") ? unknown : "pass",
    parity: "unknown"
  };
  if (!live) {
    score.parity = "unknown";
    return score;
  }
  const nativeRelevant = mode === "native" || mode === "both";
  const infringRelevant = mode === "infring" || mode === "both";
  if (nativeRelevant && infringRelevant) {
    if (native.ok && infring.ok) {
      score.parity = "pass";
    } else if (native.ok !== infring.ok) {
      score.parity = "fail";
    } else {
      score.parity = "warn";
    }
  } else {
    score.parity = native.ok || infring.ok ? "pass" : "fail";
  }
  return score;
}

function buildCapabilityMatrix(catalog: AnyJson): AnyJson[] {
  const taxonomy = Array.isArray(catalog.capability_taxonomy) ? catalog.capability_taxonomy : [];
  return (catalog.frameworks ?? []).map((framework: AnyJson) => {
    const nativeFeatures = framework.native_features ?? {};
    const socketExpected = new Set<string>(framework.socket_features_expected ?? []);
    const missingFromSocket = taxonomy.filter((capability: string) => nativeFeatures[capability] === true && !socketExpected.has(capability));
    return {
      framework_id: framework.id,
      display_name: framework.display_name,
      known_native_capabilities: Object.entries(nativeFeatures)
        .filter(([_key, value]) => value === true || value === "policy_dependent" || value === "approval_required")
        .map(([key]) => key),
      unknown_native_capabilities: Object.entries(nativeFeatures)
        .filter(([_key, value]) => value === "unknown")
        .map(([key]) => key),
      socket_features_expected: Array.from(socketExpected),
      missing_from_socket_expectations: missingFromSocket,
      known_gaps: framework.known_gaps ?? []
    };
  });
}

function buildMarkdown(report: AnyJson): string {
  const lines: string[] = [];
  lines.push("# Agent Runtime Task Harness Report");
  lines.push("");
  lines.push(`- Run ID: ${report.run_id}`);
  lines.push(`- Generated: ${report.generated_at}`);
  lines.push(`- Mode: ${report.mode}`);
  lines.push(`- Live: ${report.live}`);
  lines.push(`- Frameworks: ${report.frameworks.join(", ") || "none"}`);
  lines.push(`- Tasks: ${report.tasks.join(", ") || "none"}`);
  lines.push("");
  lines.push("## Summary");
  lines.push("");
  lines.push(`- Planned: ${report.summary.planned}`);
  lines.push(`- Completed: ${report.summary.completed}`);
  lines.push(`- Failed: ${report.summary.failed}`);
  lines.push(`- Skipped: ${report.summary.skipped}`);
  lines.push("");
  lines.push("## Results");
  lines.push("");
  lines.push("| Framework | Task | Native | InfRing | Parity |");
  lines.push("|---|---|---:|---:|---:|");
  for (const row of report.framework_results) {
    lines.push(`| ${row.framework_id} | ${row.task_id} | ${row.native.status} | ${row.infring.status} | ${row.score.parity} |`);
  }
  lines.push("");
  lines.push("## Capability Gaps");
  lines.push("");
  for (const row of report.capability_matrix) {
    if ((row.missing_from_socket_expectations ?? []).length === 0 && (row.known_gaps ?? []).length === 0) {
      continue;
    }
    lines.push(`### ${row.display_name}`);
    lines.push("");
    if ((row.missing_from_socket_expectations ?? []).length > 0) {
      lines.push(`- Missing socket expectations: ${row.missing_from_socket_expectations.join(", ")}`);
    }
    for (const gap of row.known_gaps ?? []) {
      lines.push(`- Known gap: ${gap}`);
    }
    lines.push("");
  }
  return lines.join("\n");
}

async function main(): Promise<void> {
  const args = parseArgs(process.argv.slice(2));
  if (process.argv.includes("--help") || process.argv.includes("-h")) {
    console.log(usage());
    return;
  }
  if (!["native", "infring", "both", "catalog"].includes(args.mode)) {
    throw new Error(`invalid mode: ${args.mode}`);
  }

  const catalog = readJson(args.catalog);
  const taskMatrix = readJson(args.tasks);
  const contract = readJson(args.contract);
  const frameworks = selectedRows(catalog.frameworks ?? [], args.framework);
  const tasks = args.mode === "catalog" ? [] : selectedRows(taskMatrix.tasks ?? [], args.task);
  const outDir = path.resolve(REPO_ROOT, args.outDir);
  ensureDir(outDir);

  const runId = crypto.createHash("sha256")
    .update(`${Date.now()}-${os.hostname()}-${args.mode}-${args.framework}-${args.task}`)
    .digest("hex")
    .slice(0, 16);
  const capabilityMatrix = buildCapabilityMatrix(catalog);
  const frameworkResults: AnyJson[] = [];

  if (args.mode !== "catalog") {
    for (const framework of frameworks) {
      for (const task of tasks) {
        const workDir = makeHarnessWorkspace(args.outDir, framework.id, task.id, task);
        const nativePlan = buildNativePlan(framework, task, workDir, args);
        const infringPlan = buildInfringPlan(framework, task, args);
        const native = args.mode === "native" || args.mode === "both"
          ? await executeNative(nativePlan, workDir, args.outDir, framework.id, task.id, args.timeoutMs, args.live)
          : { attempted: false, available: false, ok: true, status: "skipped", summary: "Native mode not selected." } as RunOutcome;
        const infring = args.mode === "infring" || args.mode === "both"
          ? await executeInfring(infringPlan, framework, task, workDir, args.outDir, framework.id, task.id, args)
          : { attempted: false, available: false, ok: true, status: "skipped", summary: "InfRing mode not selected." } as RunOutcome;
        frameworkResults.push({
          framework_id: framework.id,
          task_id: task.id,
          native,
          infring,
          score: scoreResult(task, native, infring, args.mode, args.live)
        });
      }
    }
  }

  const summary = frameworkResults.reduce((acc: AnyJson, row: AnyJson) => {
    for (const side of ["native", "infring"]) {
      const status = row[side]?.status;
      if (status && acc[status] !== undefined) {
        acc[status] += 1;
      }
    }
    return acc;
  }, { planned: 0, completed: 0, failed: 0, skipped: 0 });

  const report = {
    type: "agent_runtime_task_harness_report",
    contract: contract.type,
    run_id: runId,
    generated_at: new Date().toISOString(),
    live: args.live,
    mode: args.mode,
    gateway_url: args.gatewayUrl,
    frameworks: frameworks.map((framework: AnyJson) => framework.id),
    tasks: tasks.map((task: AnyJson) => task.id),
    summary,
    framework_results: frameworkResults,
    capability_matrix: capabilityMatrix,
    artifact_paths: [] as string[]
  };

  const jsonPath = path.resolve(outDir, "agent_runtime_task_harness_report.json");
  const markdownPath = path.resolve(outDir, "AGENT_RUNTIME_TASK_HARNESS_REPORT.md");
  writeJson(jsonPath, report);
  writeText(markdownPath, buildMarkdown(report));
  report.artifact_paths = [path.relative(REPO_ROOT, jsonPath), path.relative(REPO_ROOT, markdownPath)];
  writeJson(jsonPath, report);

  console.log(JSON.stringify({
    ok: summary.failed === 0,
    type: report.type,
    run_id: runId,
    live: args.live,
    mode: args.mode,
    frameworks: report.frameworks.length,
    tasks: report.tasks.length,
    summary,
    artifact_paths: report.artifact_paths
  }, null, 2));
}

main().catch((error) => {
  console.error(JSON.stringify({
    ok: false,
    type: "agent_runtime_task_harness_error",
    error: String(error?.message ?? error)
  }, null, 2));
  process.exit(1);
});
