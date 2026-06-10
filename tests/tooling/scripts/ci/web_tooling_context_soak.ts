#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
const ARTIFACT_DIR = path.join(ROOT, 'artifacts');
const STATE_DIR = path.join(ROOT, 'local', 'state', 'ops', 'web_tooling_context_soak');
const STATE_LATEST_PATH = path.join(STATE_DIR, 'latest.json');
const KNOWN_PACKAGE_HINTS = String(
  process.env.INFRING_WEB_TOOLING_SOAK_PACKAGES || 'infring-ops-core',
)
  .split(',')
  .map((value) => value.trim())
  .filter(Boolean);
const DEFAULT_TEST_PACKAGES =
  KNOWN_PACKAGE_HINTS.length > 0 ? KNOWN_PACKAGE_HINTS : ['infring-ops-core'];
const TIMEOUT_MS = Math.max(
  30_000,
  Number.parseInt(process.env.INFRING_WEB_TOOLING_SOAK_TIMEOUT_MS || '900000', 10) || 900_000,
);

type SoakCase = {
  id: string;
  lane: 'soak' | 'replay';
  test: string;
  taxonomyMarker?: string;
};

type SoakCaseResult = {
  id: string;
  lane: 'soak' | 'replay';
  test: string;
  matched_test?: string;
  resolved_package?: string;
  resolved_target?: string;
  resolved_command?: string;
  status: number;
  ok: boolean;
  duration_ms: number;
  timed_out: boolean;
  stdout_tail: string;
  stderr_tail: string;
};

type SoakReport = {
  type: 'web_tooling_context_soak_report';
  schema_version: 4;
  started_at: string;
  finished_at: string;
  ok: boolean;
  command: string;
  status: number;
  duration_ms: number;
  taxonomy: Record<string, unknown>;
  taxonomy_contract: Record<string, unknown>;
  taxonomy_source: string;
  replay_pack: Record<string, unknown>;
  tests: SoakCaseResult[];
  stdout_tail: string;
  stderr_tail: string;
};

const CASES: SoakCase[] = [
  {
    id: 'context_soak_32_turns',
    lane: 'soak',
    test: 'workflow_web_tooling_context_soak_32_turns_reports_zero_terminal_failures',
    taxonomyMarker: 'WEB_TOOLING_CONTEXT_SOAK_TAXONOMY=',
  },
  {
    id: 'replay_web_failure_final_contract',
    lane: 'replay',
    test: 'workflow_web_tool_failure_still_returns_final_user_response',
  },
  {
    id: 'replay_speculative_blocker_guard',
    lane: 'replay',
    test: 'workflow_repair_does_not_resurrect_prior_speculative_web_blocker_copy',
  },
];

const PROVIDER_FAILURE_MODE_ALLOWLIST = new Set([
  'provider_registry_missing',
  'provider_registry_empty',
  'provider_auth_missing',
  'provider_unreachable',
  'provider_partial_degradation',
  'search_provider_unavailable',
  'fetch_provider_unavailable',
  'provider_timeout',
  'provider_rate_limited',
]);

function nowIso(): string {
  return new Date().toISOString();
}

function tsSlug(iso: string): string {
  return iso.replaceAll(':', '-').replaceAll('.', '-');
}

function cleanText(raw: unknown, maxLen = 3200): string {
  return String(raw ?? '')
    .trim()
    .replace(/\s+/g, ' ')
    .slice(0, maxLen);
}

function writeJson(pathname: string, payload: unknown): void {
  fs.mkdirSync(path.dirname(pathname), { recursive: true });
  fs.writeFileSync(pathname, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
}

function parseTaxonomy(stdout: string, marker: string): { taxonomy: Record<string, unknown>; source: string } {
  const candidates = [marker, 'WORKFLOW_WEB_TOOLING_CONTEXT_SOAK_TAXONOMY=', 'WEB_TOOLING_SOAK_TAXONOMY='];
  for (const candidate of candidates) {
    const line = stdout
      .split('\n')
      .find((row) => row.trim().startsWith(candidate));
    if (!line) continue;
    const raw = line.trim().slice(candidate.length);
    try {
      return {
        taxonomy: JSON.parse(raw) as Record<string, unknown>,
        source: candidate,
      };
    } catch {
      return {
        taxonomy: {
          parse_error: 'taxonomy_json_parse_failed',
          raw: cleanText(raw, 1200),
        },
        source: candidate,
      };
    }
  }
  return {
    taxonomy: {
      parse_error: 'taxonomy_marker_missing',
    },
    source: 'missing',
  };
}

function asObject(value: unknown): Record<string, unknown> {
  if (value && typeof value === 'object' && !Array.isArray(value)) {
    return value as Record<string, unknown>;
  }
  return {};
}

function asArray(value: unknown): unknown[] {
  return Array.isArray(value) ? value : [];
}

type CargoTargetProfile = {
  packageName: string;
  args: string[];
  label: string;
};

function discoverTargetProfiles(): CargoTargetProfile[] {
  return [
    { packageName: '', args: [], label: 'default-target' },
    { packageName: '', args: ['--lib'], label: 'lib' },
    { packageName: '', args: ['--tests'], label: 'tests' },
    { packageName: '', args: ['--all-targets'], label: 'all-targets' },
  ];
}

type DiscoveredTest = {
  name: string;
  packageName: string;
  args: string[];
  targetLabel: string;
};

function listCargoTestsForProfile(profile: CargoTargetProfile): DiscoveredTest[] {
  const listed = spawnSync('cargo', ['test', '-p', profile.packageName, ...profile.args, '--', '--list'], {
    cwd: ROOT,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
    timeout: 60_000,
    killSignal: 'SIGKILL',
  });
  const stdout = String(listed.stdout || '');
  const lines = stdout
    .split('\n')
    .map((row) => row.trim())
    .filter(Boolean);
  const tests: DiscoveredTest[] = [];
  for (const line of lines) {
    const marker = ': test';
    const testIndex = line.lastIndexOf(marker);
    if (testIndex <= 0) continue;
    const name = line.slice(0, testIndex).trim();
    if (!name || name === 'test') continue;
    tests.push({
      name,
      packageName: profile.packageName,
      args: profile.args,
      targetLabel: profile.label,
    });
  }
  return tests;
}

function listCargoTests(): DiscoveredTest[] {
  const discovered: DiscoveredTest[] = [];
  const profiles = discoverTargetProfiles();
  for (const packageName of DEFAULT_TEST_PACKAGES) {
    for (const profile of profiles) {
      const discoveredForProfile = listCargoTestsForProfile({
        ...profile,
        packageName,
      });
      discovered.push(...discoveredForProfile);
    }
  }
  const seen = new Set<string>();
  const uniq: DiscoveredTest[] = [];
  for (const row of discovered) {
    const key = `${row.packageName}|${row.targetLabel}|${row.name}`;
    if (seen.has(key)) continue;
    seen.add(key);
    uniq.push(row);
  }
  return uniq;
}

type ResolvedTestRef = {
  requested: string;
  matched: string;
  packageName: string;
  args: string[];
  targetLabel: string;
};

function resolveCargoTest(rawName: string, candidates: DiscoveredTest[]): ResolvedTestRef | null {
  const exact = candidates.find((row) => row.name === rawName);
  if (exact) {
    return {
      requested: rawName,
      matched: exact.name,
      packageName: exact.packageName,
      args: exact.args,
      targetLabel: exact.targetLabel,
    };
  }

  const suffix = `::${rawName}`;
  const suffixMatch = candidates.find((row) => row.name.endsWith(suffix));
  if (suffixMatch) {
    return {
      requested: rawName,
      matched: suffixMatch.name,
      packageName: suffixMatch.packageName,
      args: suffixMatch.args,
      targetLabel: suffixMatch.targetLabel,
    };
  }

  const includes = candidates.find((row) => row.name.includes(rawName));
  if (!includes) return null;
  return {
    requested: rawName,
    matched: includes.name,
    packageName: includes.packageName,
    args: includes.args,
    targetLabel: includes.targetLabel,
  };
}

function buildRunCommand(packageName: string, args: string[], testName: string): string {
  const parts = ['cargo', 'test', '-p', packageName, ...args, testName, '--', '--nocapture'];
  return parts.join(' ');
}

function isNoSuchTestError(out: { stdout: string; stderr: string; status: number }): boolean {
  if (out.status === 0) return false;
  const merged = `${out.stdout}\n${out.stderr}`.toLowerCase();
  return (
    merged.includes('running 0 tests') ||
    merged.includes('no tests to run') ||
    merged.includes('no test name') ||
    merged.includes('could not find') ||
    merged.includes('no test target') ||
    merged.includes('error: no test name specified')
  );
}

function readBoolLike(value: unknown): boolean | null {
  if (typeof value === 'boolean') return value;
  if (typeof value === 'number') return value !== 0;
  if (typeof value === 'string') {
    const lowered = cleanText(value, 32).toLowerCase();
    if (['true', '1', 'yes', 'y', 'on'].includes(lowered)) return true;
    if (['false', '0', 'no', 'n', 'off'].includes(lowered)) return false;
  }
  return null;
}

function readNumberLike(value: unknown): number | null {
  if (typeof value === 'number' && Number.isFinite(value)) {
    return value;
  }
  if (typeof value === 'string') {
    const parsed = Number.parseFloat(cleanText(value, 64));
    if (Number.isFinite(parsed)) {
      return parsed;
    }
  }
  return null;
}

function normalizeTaxonomyContracts(raw: Record<string, unknown>): Record<string, unknown> {
  const taxonomy = { ...raw };
  const cache = asObject(taxonomy.cache);
  const qualityTelemetry = asObject(
    taxonomy.quality_telemetry ?? taxonomy.telemetry ?? taxonomy.metrics,
  );
  const providerFailures = asArray(
    taxonomy.provider_failures ?? taxonomy.provider_failure_events ?? taxonomy.failures,
  ).map((row) => asObject(row));

  const cacheSkipped =
    readBoolLike(taxonomy.cache_skipped) ??
    readBoolLike(cache.skipped) ??
    readBoolLike(cache.cache_skipped) ??
    false;
  const cacheSkipReason = cleanText(
    taxonomy.cache_skip_reason ?? cache.skip_reason ?? cache.reason ?? '',
    240,
  );
  const cacheWriteAllowed =
    readBoolLike(taxonomy.cache_write_allowed) ??
    readBoolLike(cache.write_allowed) ??
    false;
  const cacheWriteAttempted =
    readBoolLike(taxonomy.cache_write_attempted) ??
    readBoolLike(cache.write_attempted) ??
    false;
  const cacheStaleAgeSeconds =
    readNumberLike(taxonomy.cache_stale_age_seconds) ??
    readNumberLike(cache.stale_age_seconds) ??
    readNumberLike(cache.age_seconds) ??
    readNumberLike(taxonomy.age_seconds) ??
    null;
  const workflowLoopLeakCount =
    readNumberLike(taxonomy.workflow_loop_leak_count) ??
    readNumberLike(taxonomy.workflow_retry_loop_detected_count) ??
    readNumberLike(taxonomy.unexpected_state_loop_count) ??
    readNumberLike(qualityTelemetry.workflow_unexpected_state_loop_count) ??
    readNumberLike(qualityTelemetry.unexpected_state_loop_count) ??
    0;
  const fileToolRouteMisdirectionCount =
    readNumberLike(taxonomy.file_tool_route_misdirection_count) ??
    readNumberLike(taxonomy.route_misdirection_count) ??
    readNumberLike(qualityTelemetry.file_tool_route_misdirection_count) ??
    readNumberLike(qualityTelemetry.route_misdirection_count) ??
    0;

  taxonomy.provider_failures = providerFailures;
  taxonomy.cache_skipped = cacheSkipped;
  taxonomy.cache_skip_reason = cacheSkipReason;
  taxonomy.cache_write_allowed = cacheWriteAllowed;
  taxonomy.cache_write_attempted = cacheWriteAttempted;
  taxonomy.cache_stale_age_seconds = cacheStaleAgeSeconds;
  taxonomy.workflow_loop_leak_count = workflowLoopLeakCount;
  taxonomy.file_tool_route_misdirection_count = fileToolRouteMisdirectionCount;
  taxonomy.cache = {
    ...cache,
    skipped: cacheSkipped,
    skip_reason: cacheSkipReason,
    write_allowed: cacheWriteAllowed,
    write_attempted: cacheWriteAttempted,
    stale_age_seconds: cacheStaleAgeSeconds,
  };
  return taxonomy;
}

function normalizeProviderFailureMode(row: Record<string, unknown>): string {
  return cleanText(
    row.failure_mode
      ?? row.reason
      ?? row.code
      ?? row.kind
      ?? row.status
      ?? row.type
      ?? '',
    100,
  ).toLowerCase();
}

function evaluateTaxonomyContracts(taxonomy: Record<string, unknown>): Record<string, unknown> {
  const providerRows = asArray(taxonomy.provider_failures).map((row) => asObject(row));
  const providerFailureModes = new Set<string>();
  let providerFailureContractViolationCount = 0;
  for (const row of providerRows) {
    const mode = normalizeProviderFailureMode(row);
    if (!mode) {
      providerFailureContractViolationCount += 1;
      continue;
    }
    providerFailureModes.add(mode);
    if (!PROVIDER_FAILURE_MODE_ALLOWLIST.has(mode)) {
      providerFailureContractViolationCount += 1;
    }
  }
  const singularMode = cleanText(taxonomy.provider_failure_mode ?? '', 100).toLowerCase();
  if (singularMode) {
    providerFailureModes.add(singularMode);
    if (!PROVIDER_FAILURE_MODE_ALLOWLIST.has(singularMode)) {
      providerFailureContractViolationCount += 1;
    }
  }
  const cacheSkipped = readBoolLike(taxonomy.cache_skipped) ?? false;
  const cacheSkipReason = cleanText(taxonomy.cache_skip_reason ?? '', 240);
  const cacheWriteAllowed = readBoolLike(taxonomy.cache_write_allowed) ?? false;
  const cacheWriteAttempted = readBoolLike(taxonomy.cache_write_attempted) ?? false;
  const cacheStaleAgeSeconds = readNumberLike(taxonomy.cache_stale_age_seconds);
  const workflowLoopLeakCount = Math.max(
    0,
    Math.trunc(readNumberLike(taxonomy.workflow_loop_leak_count) ?? 0),
  );
  const fileToolRouteMisdirectionCount = Math.max(
    0,
    Math.trunc(readNumberLike(taxonomy.file_tool_route_misdirection_count) ?? 0),
  );
  const cacheSkipReasonMissingCount = cacheSkipped && !cacheSkipReason ? 1 : 0;
  const cacheWriteGateViolationCount =
    providerFailureModes.size > 0 && (cacheWriteAllowed || cacheWriteAttempted) ? 1 : 0;
  const staleAgeRequired = cacheSkipped || cacheWriteAttempted || providerFailureModes.size > 0;
  const cacheStaleAgeMissingCount =
    staleAgeRequired && (cacheStaleAgeSeconds == null || cacheStaleAgeSeconds < 0) ? 1 : 0;

  const failures: string[] = [];
  if (providerFailureContractViolationCount > 0) {
    failures.push('provider_failure_mode_contract_violation');
  }
  if (cacheSkipReasonMissingCount > 0) {
    failures.push('cache_skip_reason_missing');
  }
  if (cacheWriteGateViolationCount > 0) {
    failures.push('cache_write_fail_closed_violation');
  }
  if (cacheStaleAgeMissingCount > 0) {
    failures.push('cache_stale_age_metadata_missing');
  }
  if (workflowLoopLeakCount > 0) {
    failures.push('workflow_loop_leak_detected');
  }
  if (fileToolRouteMisdirectionCount > 0) {
    failures.push('file_tool_route_misdirection_detected');
  }

  return {
    ok: failures.length === 0,
    provider_failure_modes: Array.from(providerFailureModes).sort(),
    provider_failure_contract_violation_count: providerFailureContractViolationCount,
    cache_skipped: cacheSkipped,
    cache_skip_reason: cacheSkipReason,
    cache_skip_reason_missing_count: cacheSkipReasonMissingCount,
    cache_write_allowed: cacheWriteAllowed,
    cache_write_attempted: cacheWriteAttempted,
    cache_write_gate_violation_count: cacheWriteGateViolationCount,
    cache_stale_age_seconds: cacheStaleAgeSeconds,
    cache_stale_age_required: staleAgeRequired,
    cache_stale_age_missing_count: cacheStaleAgeMissingCount,
    workflow_loop_leak_count: workflowLoopLeakCount,
    file_tool_route_misdirection_count: fileToolRouteMisdirectionCount,
    failures,
  };
}

function runCargoTestWithTimeoutKill(
  testName: string,
  packageName: string,
  args: string[],
): {
  result: SoakCaseResult;
  stdoutRaw: string;
} {
  const started = Date.now();
  const commandArgs = ['test', '-p', packageName, ...args, testName, '--', '--nocapture'];
  const command = buildRunCommand(packageName, args, testName);
  const run = spawnSync('cargo', commandArgs, {
    cwd: ROOT,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
    timeout: TIMEOUT_MS,
    killSignal: 'SIGKILL',
  });
  const timeoutMessage = String(run.error?.message || '').toLowerCase();
  const timedOut =
    !!run.error && (timeoutMessage.includes('timed out') || timeoutMessage.includes('etimedout'));
  if (timedOut) {
    const profile = args.length > 0 ? ` ${args.join(' ')}` : '';
    const pattern = `cargo test -p ${packageName}${profile} ${testName}`;
    spawnSync('pkill', ['-TERM', '-f', pattern], { cwd: ROOT, stdio: 'ignore' });
    spawnSync('pkill', ['-KILL', '-f', pattern], { cwd: ROOT, stdio: 'ignore' });
  }
  const status = timedOut ? 124 : Number.isFinite(run.status) ? Number(run.status) : 1;
  const stdoutRaw = String(run.stdout || '');
  const stderrRaw = String(run.stderr || '');
  return {
    result: {
      id: '',
      lane: 'soak',
      test: testName,
      resolved_package: packageName,
      resolved_target: args.length > 0 ? args.join(' ') : 'default',
      resolved_command: command,
      status,
      ok: status === 0,
      duration_ms: Date.now() - started,
      timed_out: timedOut,
      stdout_tail: cleanText(stdoutRaw, 4_000),
      stderr_tail: cleanText(
        `${stderrRaw} ${timedOut ? `timeout_after_ms_${TIMEOUT_MS}` : ''}`.trim(),
        4_000,
      ),
    },
    stdoutRaw,
  };
}

function runMissingTestCase(row: SoakCase, reason: string): { result: SoakCaseResult; stdoutRaw: string } {
  const detail = `missing_test_case=${row.test}; reason=${reason}`;
  return {
    result: {
      id: row.id,
      lane: row.lane,
      test: row.test,
      status: 1,
      ok: false,
      duration_ms: 0,
      timed_out: false,
      stdout_tail: '',
      stderr_tail: cleanText(detail, 1200),
    },
    stdoutRaw: detail,
  };
}

function runCargoTestOrMissing(
  row: SoakCase,
  known: DiscoveredTest[],
  fallbackProfiles: CargoTargetProfile[],
): { result: SoakCaseResult; stdoutRaw: string; resolved: ResolvedTestRef | null } {
  const explicit = resolveCargoTest(row.test, known);
  const attemptQueue: CargoTargetProfile[] = [];
  if (explicit) {
    attemptQueue.push({
      packageName: explicit.packageName,
      args: explicit.args,
      label: explicit.targetLabel,
    });
  } else {
    for (const pkg of DEFAULT_TEST_PACKAGES) {
      for (const profile of fallbackProfiles) {
        attemptQueue.push({
          packageName: pkg,
          args: profile.args,
          label: `${pkg}:${profile.label}`,
        });
      }
    }
  }

  for (const attempt of attemptQueue) {
    const resolved: ResolvedTestRef = explicit
      ? {
          requested: row.test,
          matched: explicit.matched,
          packageName: attempt.packageName,
          args: attempt.args,
          targetLabel: explicit ? explicit.targetLabel : attempt.label,
        }
      : {
          requested: row.test,
          matched: row.test,
          packageName: attempt.packageName,
          args: attempt.args,
          targetLabel: attempt.label,
        };
    const out = runCargoTestWithTimeoutKill(resolved.matched, attempt.packageName, attempt.args);
    if (out.result.timed_out) {
      return { ...out, resolved };
    }
    if (out.result.status === 0) {
      return { ...out, resolved };
    }
    if (!isNoSuchTestError({ stdout: out.stdoutRaw, stderr: out.result.stderr_tail, status: out.result.status })) {
      return { ...out, resolved };
    }
  }

  return {
    ...runMissingTestCase(
      row,
      explicit
        ? 'no_matching_discovery_profile'
        : 'no_matching_cargo_test_found_in_list_output_or_direct_probe',
    ),
    resolved: null,
  };
}

const startedAt = nowIso();
const startedMs = Date.now();
const results: SoakCaseResult[] = [];
const knownTests = listCargoTests();
const fallbackProfiles = discoverTargetProfiles();
let taxonomy: Record<string, unknown> = {
  parse_error: 'taxonomy_marker_missing',
};
let taxonomySource = 'missing';
let commandExample = 'cargo test -p <package> [targets] <workflow-test-name> -- --nocapture';

for (const row of CASES) {
  const { result: run, stdoutRaw, resolved } = runCargoTestOrMissing(
    row,
    knownTests,
    fallbackProfiles,
  );
  const result: SoakCaseResult = {
    ...run,
    id: row.id,
    lane: row.lane,
    test: row.test,
    matched_test: resolved?.matched,
    resolved_package: resolved?.packageName ?? run.resolved_package,
    resolved_target: resolved?.targetLabel,
    resolved_command: run.resolved_command,
  };
  if (run.resolved_command) {
    commandExample = run.resolved_command;
  }
  if (row.taxonomyMarker) {
    const parsed = parseTaxonomy(stdoutRaw, row.taxonomyMarker);
    taxonomy = parsed.taxonomy;
    taxonomySource = parsed.source;
  }
  results.push(result);
}
taxonomy = normalizeTaxonomyContracts(taxonomy);
const taxonomyContract = evaluateTaxonomyContracts(taxonomy);

const replayRows = results.filter((row) => row.lane === 'replay');
const replayFailed = replayRows.filter((row) => !row.ok);
const replayPack = {
  total: replayRows.length,
  passed: replayRows.length - replayFailed.length,
  failed: replayFailed.length,
  failed_ids: replayFailed.map((row) => row.id),
};

const taxonomyError = cleanText(
  (taxonomy.parse_error as string) || '',
  120,
);
const allTestsPassed = results.every((row) => row.ok);
const ok = allTestsPassed && taxonomyError.length === 0 && taxonomyContract.ok === true;

const report: SoakReport = {
  type: 'web_tooling_context_soak_report',
  schema_version: 4,
  started_at: startedAt,
  finished_at: nowIso(),
  ok,
  command: commandExample,
  status: ok ? 0 : 1,
  duration_ms: Date.now() - startedMs,
  taxonomy,
  taxonomy_contract: taxonomyContract,
  taxonomy_source: taxonomySource,
  replay_pack: replayPack,
  tests: results,
  stdout_tail: cleanText(
    results.map((row) => `${row.id}: ${row.stdout_tail}`).join('\n'),
    4_000,
  ),
  stderr_tail: cleanText(
    results.map((row) => `${row.id}: ${row.stderr_tail}`).join('\n'),
    4_000,
  ),
};

fs.mkdirSync(ARTIFACT_DIR, { recursive: true });
const stamp = tsSlug(report.finished_at);
const stampedPath = path.join(ARTIFACT_DIR, `web_tooling_context_soak_report_${stamp}.json`);
const latestPath = path.join(ARTIFACT_DIR, 'web_tooling_context_soak_report_latest.json');
writeJson(stampedPath, report);
writeJson(latestPath, report);
writeJson(STATE_LATEST_PATH, report);

process.stdout.write(`${JSON.stringify(report)}\n`);
process.exit(ok ? 0 : 1);
