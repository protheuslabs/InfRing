#!/usr/bin/env node
// SPDX-License-Identifier: Apache-2.0
// Layer ownership: observability/sentinel (Kernel Sentinel trace completeness guard)

import fs from "node:fs";
import path from "node:path";

type Json = Record<string, unknown>;

const root = process.cwd();
const policyRel = "observability/sentinel/sentinel_trace_completeness_policy.json";
const policyPath = path.join(root, policyRel);
const policy = JSON.parse(fs.readFileSync(policyPath, "utf8")) as Json;
const strict = process.argv.includes("--strict=1") || process.argv.includes("--strict");

function readJson(rel: string): Json | null {
  try {
    return JSON.parse(fs.readFileSync(path.join(root, rel), "utf8")) as Json;
  } catch {
    return null;
  }
}

function readJsonl(rel: string): Json[] {
  try {
    return fs
      .readFileSync(path.join(root, rel), "utf8")
      .split(/\r?\n/)
      .map((line) => line.trim())
      .filter(Boolean)
      .map((line) => JSON.parse(line) as Json);
  } catch {
    return [];
  }
}

function hasTrace(row: Json | null): boolean {
  return typeof row?.trace_id === "string" && row.trace_id.trim().length > 0;
}

const violations: Json[] = [];
const checked: Json[] = [];

for (const rel of Array.isArray(policy.required_json_paths) ? policy.required_json_paths.map(String) : []) {
  const payload = readJson(rel);
  const exists = fs.existsSync(path.join(root, rel));
  const ok = exists && hasTrace(payload);
  checked.push({ path: rel, kind: "json", exists, has_trace_id: hasTrace(payload), type: payload?.type || null });
  if (!ok) {
    violations.push({
      kind: exists ? "sentinel_json_missing_trace_id" : "sentinel_json_missing",
      path: rel,
      type: payload?.type || null,
    });
  }
}

for (const rel of Array.isArray(policy.optional_json_paths) ? policy.optional_json_paths.map(String) : []) {
  const payload = readJson(rel);
  const exists = fs.existsSync(path.join(root, rel));
  const ok = !exists || hasTrace(payload);
  checked.push({
    path: rel,
    kind: "json",
    required: false,
    exists,
    has_trace_id: hasTrace(payload),
    type: payload?.type || null,
  });
  if (!ok) {
    violations.push({
      kind: "sentinel_optional_json_missing_trace_id",
      path: rel,
      type: payload?.type || null,
    });
  }
}

for (const rel of Array.isArray(policy.required_jsonl_paths) ? policy.required_jsonl_paths.map(String) : []) {
  const exists = fs.existsSync(path.join(root, rel));
  const rows = readJsonl(rel);
  const missingRows = rows
    .map((row, index) => ({ row, index }))
    .filter(({ row }) => !hasTrace(row))
    .map(({ row, index }) => ({ index, type: row.type || null, fingerprint: row.fingerprint || row.finding_fingerprint || null }));
  checked.push({ path: rel, kind: "jsonl", exists, row_count: rows.length, missing_trace_row_count: missingRows.length });
  if (!exists) {
    violations.push({ kind: "sentinel_jsonl_missing", path: rel });
  } else if (missingRows.length > 0) {
    violations.push({
      kind: "sentinel_jsonl_rows_missing_trace_id",
      path: rel,
      missing_row_count: missingRows.length,
      examples: missingRows.slice(0, 10),
    });
  }
}

const traceId = `observability:${new Date().toISOString()}:kernel-sentinel-trace-completeness`;
const payload = {
  trace_id: traceId,
  span_id: `span:${traceId}`,
  parent_span_id: null,
  source_domain: "observability",
  ok: violations.length === 0,
  type: "kernel_sentinel_trace_completeness_guard",
  generated_at: new Date().toISOString(),
  strict,
  policy_path: policyRel,
  checked_count: checked.length,
  violations,
  checked,
};

const outRel = typeof policy.report_path === "string" ? policy.report_path : "core/local/artifacts/kernel_sentinel_trace_completeness_guard_current.json";
const outPath = path.join(root, outRel);
fs.mkdirSync(path.dirname(outPath), { recursive: true });
fs.writeFileSync(outPath, `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify(payload, null, 2));

if (strict && !payload.ok) {
  process.exit(1);
}
