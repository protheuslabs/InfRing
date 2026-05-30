import fs from "node:fs";
import path from "node:path";

type Json = Record<string, unknown>;

const root = process.cwd();

function arg(name: string, fallback = ""): string {
  const prefix = `--${name}=`;
  const direct = process.argv.find((item) => item.startsWith(prefix));
  if (direct) return direct.slice(prefix.length);
  const idx = process.argv.indexOf(`--${name}`);
  return idx >= 0 && process.argv[idx + 1] ? process.argv[idx + 1] : fallback;
}

function flag(name: string, fallback = false): boolean {
  const raw = arg(name, "");
  if (!raw) return process.argv.includes(`--${name}`) || fallback;
  return ["1", "true", "yes", "on"].includes(raw.trim().toLowerCase());
}

function readJson(rel: string): Json {
  return JSON.parse(fs.readFileSync(path.join(root, rel), "utf8")) as Json;
}

function statRow(rel: string): Json {
  const abs = path.join(root, rel);
  if (!fs.existsSync(abs)) {
    return { rel, exists: false, age_hours: null };
  }
  const stat = fs.statSync(abs);
  return {
    rel,
    exists: true,
    mtime_ms: stat.mtimeMs,
    age_hours: Number(((Date.now() - stat.mtimeMs) / 3_600_000).toFixed(2)),
    bytes: stat.size,
  };
}

function newest(rows: Json[]): Json | null {
  const existing = rows.filter((row) => row.exists === true && typeof row.mtime_ms === "number");
  existing.sort((a, b) => Number(b.mtime_ms) - Number(a.mtime_ms));
  return existing[0] || null;
}

const policyRel = arg("policy", "observability/sentinel/sentinel_activity_freshness_policy.json");
const policy = readJson(policyRel);
const heartbeatRows = Array.isArray(policy.heartbeat_artifacts)
  ? policy.heartbeat_artifacts.map(String).map(statRow)
  : [];
const dreamRows = Array.isArray(policy.dream_artifacts)
  ? policy.dream_artifacts.map(String).map(statRow)
  : [];
const heartbeatNewest = newest(heartbeatRows);
const dreamNewest = newest(dreamRows);
const maxHeartbeatAgeHours = Number(policy.max_heartbeat_age_hours || 24);
const maxDreamAgeHours = Number(policy.max_dream_age_hours || 36);
const heartbeatAge = heartbeatNewest ? Number(heartbeatNewest.age_hours) : Number.POSITIVE_INFINITY;
const dreamAge = dreamNewest ? Number(dreamNewest.age_hours) : Number.POSITIVE_INFINITY;
const heartbeatFresh = Number.isFinite(heartbeatAge) && heartbeatAge <= maxHeartbeatAgeHours;
const dreamFresh = Number.isFinite(dreamAge) && dreamAge <= maxDreamAgeHours;
const failures = [
  ...(heartbeatFresh ? [] : ["kernel_sentinel_heartbeat_missing_or_stale"]),
  ...(dreamFresh ? [] : ["kernel_sentinel_dream_missing_or_stale"]),
];
const payload = {
  ok: failures.length === 0,
  type: "kernel_sentinel_activity_freshness_guard",
  generated_at: new Date().toISOString(),
  trace_id: `observability:${new Date().toISOString()}:kernel-sentinel-activity-freshness`,
  span_id: `span:observability:${new Date().toISOString()}:kernel-sentinel-activity-freshness`,
  source_domain: "observability/sentinel",
  policy_path: policyRel,
  max_heartbeat_age_hours: maxHeartbeatAgeHours,
  max_dream_age_hours: maxDreamAgeHours,
  heartbeat_fresh: heartbeatFresh,
  dream_fresh: dreamFresh,
  heartbeat_newest: heartbeatNewest,
  dream_newest: dreamNewest,
  failures,
  heartbeat_artifacts: heartbeatRows,
  dream_artifacts: dreamRows,
  next_action: failures.length === 0
    ? "none"
    : "ensure the resident ops IPC daemon is running or run npm run -s ops:kernel-sentinel:heartbeat",
};

const outRel = arg("out-json", String(policy.output_path || "core/local/artifacts/kernel_sentinel_activity_freshness_guard_current.json"));
const outPath = path.join(root, outRel);
fs.mkdirSync(path.dirname(outPath), { recursive: true });
fs.writeFileSync(outPath, `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify({
  ok: payload.ok,
  failures,
  heartbeat_fresh: heartbeatFresh,
  dream_fresh: dreamFresh,
  out_json: outRel,
}, null, 2));

if (flag("strict", false) && !payload.ok) {
  process.exit(1);
}
