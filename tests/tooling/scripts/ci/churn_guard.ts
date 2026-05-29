#!/usr/bin/env node
/* eslint-disable no-console */
// TODO(rkapoor): Add threshold validation for weekly churn % - Q2 2026
import { execSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { cleanText, hasFlag, parseBool, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeJsonArtifact, writeTextArtifact } from '../../lib/result.ts';

const DEFAULT_OUT_JSON = 'core/local/artifacts/churn_guard_current.json';
const DEFAULT_OUT_MD = 'local/workspace/reports/CHURN_GUARD_CURRENT.md';
const SWARM_CODE_SURFACES = new Set([
  'core/layer0/ops/src/swarm_runtime.rs',
  'client/runtime/systems/autonomy/swarm_sessions_bridge.ts',
]);
const SWARM_TEST_SURFACES = new Set([
  'core/layer0/ops/tests/v9_swarm_runtime_integration.rs',
  'core/layer0/ops/tests/v6_infring_closure_integration.rs',
  'tests/client-memory-tools/swarm_sessions_bridge.test.ts',
  'tests/tooling/scripts/ci/swarm_protocol_audit_runner.ts',
]);
const SWARM_DOC_SURFACES = new Set([
  'docs/workspace/SRS.md',
  'docs/client/requirements/REQ-38-agent-orchestration-hardening.md',
]);
const ALLOWED_ASSURANCE_PHYSICAL_DOMAIN_MOVE_PAIRS = new Set([
  'tests/tooling/config/assurance_governance_registry.json->validation/governance/contracts/assurance_governance_registry.json',
  'tests/tooling/schemas/assurance_governance_registry.schema.json->validation/schemas/assurance_governance_registry.schema.json',
]);

function parseArgs(argv) {
  const strictRaw = readFlag(argv, 'strict');
  const commitGate =
    hasFlag(argv, 'commit-gate') ||
    parseBool(readFlag(argv, 'commit-gate'), false) ||
    process.env.CHURN_GUARD_COMMIT_GATE === '1';
  const allowGovernanceDocChurn =
    hasFlag(argv, 'allow-governance-doc-churn') ||
    parseBool(readFlag(argv, 'allow-governance-doc-churn'), false) ||
    process.env.ALLOW_GOVERNANCE_DOC_CHURN === '1' ||
    !commitGate;
  return {
    strict: hasFlag(argv, 'strict') || parseBool(strictRaw, false),
    allowGovernanceDocChurn,
    commitGate,
    outJson: cleanText(readFlag(argv, 'out-json') || DEFAULT_OUT_JSON, 400),
    outMarkdown: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MD, 400),
  };
}

function classifyPath(path) {
  if (
    path.startsWith('local/state/ops/daemon_control/') ||
    /^local\/workspace\/memory\/dashboard-ui.*\.(log|pid)$/i.test(path)
  ) {
    return 'session_churn';
  }
  if (SWARM_CODE_SURFACES.has(path) || SWARM_TEST_SURFACES.has(path)) {
    return 'swarm_surface_churn';
  }
  if (
    path === '.codex_worktrees/' ||
    path.startsWith('.codex_worktrees/') ||
    (!path.includes('/') && /^(cell|regional|swarm)[-_A-Za-z0-9]*\.(py|js|swarm)$/i.test(path))
  ) {
    return 'local_simulation_churn';
  }
  if (
    path === 'docs/workspace/todo/TODO.md' ||
    path === 'docs/workspace/SRS.md' ||
    path === 'docs/workspace/UPGRADE_BACKLOG.md' ||
    /^local\/workspace\/reports\/CODEX_FILE_LEDGER_.*\.(md|tsv)$/i.test(path) ||
    /^docs\/client\/requirements\/REQ-[^/]+\.md$/i.test(path)
  ) {
    return 'governance_doc_churn';
  }
  if (
    path.startsWith('local/') ||
    path.startsWith('simulated-commits/')
  ) {
    return 'local_simulation_churn';
  }
  if (
    path.startsWith('packages/lensmap/') ||
    path.startsWith('tests/fixtures/lensmap_') ||
    path === 'core/layer0/ops/src/bin/lensmap.rs'
  ) {
    return 'lensmap_churn';
  }
  if (
    /^core\/local\/artifacts\/.*_current\.json$/i.test(path) ||
    // Canonical benchmark snapshots are public auditable artifacts and are allowed in commit gate.
    // Keep non-canonical sampled/resample reports guarded as generated churn.
    /^docs\/client\/reports\/benchmark_matrix_resample[^/]*\.json$/i.test(path) ||
    (/^docs\/workspace\/SRS_.*CURRENT\.md$/i.test(path) || /^local\/workspace\/reports\/SRS_.*CURRENT\.md$/i.test(path)) ||
    path === 'docs/workspace/BLOCKED_EXTERNAL_EVIDENCE_STATUS.md' ||
    path === 'local/workspace/reports/BLOCKED_EXTERNAL_EVIDENCE_STATUS.md' ||
    path === 'docs/workspace/BLOCKED_EXTERNAL_RECONCILE_CANDIDATES.md' ||
    path === 'local/workspace/reports/BLOCKED_EXTERNAL_RECONCILE_CANDIDATES.md' ||
    path === 'docs/workspace/BLOCKED_EXTERNAL_UNBLOCK_PLAN.md' ||
    path === 'local/workspace/reports/BLOCKED_EXTERNAL_UNBLOCK_PLAN.md' ||
    path === 'docs/workspace/BLOCKED_EXTERNAL_PACKET_AUDIT.md' ||
    path === 'local/workspace/reports/BLOCKED_EXTERNAL_PACKET_AUDIT.md' ||
    path === 'docs/workspace/BLOCKED_EXTERNAL_TOP10.md' ||
    path === 'local/workspace/reports/BLOCKED_EXTERNAL_TOP10.md'
  ) {
    return 'generated_report_churn';
  }
  return 'other';
}

function detectSwarmCompanionGaps(rows) {
  const dirtyPaths = new Set(rows.map((row) => row.path));
  const touchesSwarmCode = [...dirtyPaths].some((path) => SWARM_CODE_SURFACES.has(path));
  const touchesSwarmTests = [...dirtyPaths].some((path) => SWARM_TEST_SURFACES.has(path));
  if (!touchesSwarmCode && !touchesSwarmTests) {
    return [];
  }
  const touchesSwarmDocs = [...dirtyPaths].some((path) => SWARM_DOC_SURFACES.has(path));
  const gaps = [];
  if (touchesSwarmCode && !touchesSwarmTests) {
    gaps.push({
      type: 'missing_swarm_tests',
      detail:
        'swarm runtime or bridge changed without updating swarm integration/bridge/audit coverage',
    });
  }
  if ((touchesSwarmCode || touchesSwarmTests) && !touchesSwarmDocs) {
    gaps.push({
      type: 'missing_swarm_docs',
      detail:
        'swarm runtime or audit changes must update SRS or REQ-38 orchestration hardening evidence',
    });
  }
  return gaps;
}

function parseStatus() {
  const raw = execSync('git status --porcelain=v1 -uall', { encoding: 'utf8' });
  if (!raw.trim()) return [];
  return raw
    .split('\n')
    .filter((line) => line.length > 0)
    .map((line) => {
      const status = line.slice(0, 2);
      const path = line.slice(3).trim();
      return { status, path, category: classifyPath(path) };
    });
}

function parseStagedStatus() {
  const raw = execSync('git diff --cached --name-status --find-renames', { encoding: 'utf8' });
  if (!raw.trim()) return [];
  return raw
    .split('\n')
    .filter((line) => line.length > 0)
    .map((line) => {
      const parts = line.split('\t');
      const code = parts[0] || '';
      const path = code.startsWith('R') && parts.length >= 3 ? parts[2] : parts[1] || '';
      const status = code.startsWith('R') ? 'R ' : `${code.slice(0, 1) || '?'} `;
      return { status, path, category: classifyPath(path) };
    });
}

function isDeleted(status) {
  return status.includes('D');
}

function isUntracked(status) {
  return status === '??';
}

function detectLikelyUnstagedMoves(rows) {
  const deleted = rows.filter((row) => isDeleted(row.status)).map((row) => row.path);
  const untracked = rows.filter((row) => isUntracked(row.status)).map((row) => row.path);
  const untrackedSet = new Set(untracked);
  const untrackedByBasename = new Map();
  for (const path of untracked) {
    const base = path.split('/').pop() || path;
    if (!untrackedByBasename.has(base)) {
      untrackedByBasename.set(base, []);
    }
    untrackedByBasename.get(base).push(path);
  }

  const prefixMoves = [
    ['apps/_shared/cognition/', 'client/cognition/shared/'],
    ['apps/habits/', 'client/cognition/habits/'],
    ['scripts/', 'tests/tooling/scripts/'],
  ];

  const pairs = [];
  for (const oldPath of deleted) {
    const expectedPaths = [];
    for (const [fromPrefix, toPrefix] of prefixMoves) {
      if (oldPath.startsWith(fromPrefix)) {
        expectedPaths.push(`${toPrefix}${oldPath.slice(fromPrefix.length)}`);
      }
    }
    if (oldPath === 'apps/_shared/run_infring_ops.ts') {
      expectedPaths.push('client/runtime/systems/ops/run_infring_ops.ts');
    }
    let match = expectedPaths.find((candidate) => untrackedSet.has(candidate)) || null;
    if (!match) {
      const base = oldPath.split('/').pop() || oldPath;
      const basenameCandidates = untrackedByBasename.get(base) || [];
      match = basenameCandidates[0] || null;
    }
    if (match) {
      pairs.push({ from: oldPath, to: match });
    }
  }
  return pairs;
}

function isAllowedAssurancePhysicalDomainMove(pair) {
  return ALLOWED_ASSURANCE_PHYSICAL_DOMAIN_MOVE_PAIRS.has(`${pair.from}->${pair.to}`);
}

function detectSessionChurnSignals() {
  const issues = [];

  try {
    const raw = execSync('ps -ax -o pid=,ppid=,command=', { encoding: 'utf8' });
    const rows = raw
      .split('\n')
      .map((line) => line.trim())
      .filter(Boolean);

    const parsedRows = rows
      .map((line) => {
        const match = line.match(/^(\d+)\s+(\d+)\s+(.+)$/);
        if (!match) return null;
        return {
          pid: Number(match[1]),
          ppid: Number(match[2]),
          command: match[3],
          raw: line,
        };
      })
      .filter(Boolean);

    const tsHosts = new Map(
      parsedRows
        .filter((row) => /ts_entrypoint\.ts .*infring_dashboard\.ts serve/.test(row.command))
        .map((row) => [row.pid, row]),
    );
    const opsHosts = parsedRows.filter((row) => /infring-ops dashboard-ui serve/.test(row.command));
    const dashboardHosts = [...tsHosts.values(), ...opsHosts];

    const hostGroups = new Set();
    for (const host of dashboardHosts) {
      if (/infring-ops dashboard-ui serve/.test(host.command) && tsHosts.has(host.ppid)) {
        hostGroups.add(`ts:${host.ppid}`);
      } else if (/ts_entrypoint\.ts .*infring_dashboard\.ts serve/.test(host.command)) {
        hostGroups.add(`ts:${host.pid}`);
      } else {
        hostGroups.add(`proc:${host.pid}`);
      }
    }

    if (hostGroups.size > 1) {
      issues.push({
        type: 'duplicate_dashboard_hosts',
        detail: `multiple dashboard host processes detected (${hostGroups.size})`,
        sample: dashboardHosts.map((row) => row.raw).slice(0, 4),
      });
    }
  } catch {
    // ignore runtime process inspection failures
  }

  const dashboardPidPath = resolve('local/state/ops/daemon_control/dashboard_ui.pid');
  if (existsSync(dashboardPidPath)) {
    try {
      const pid = String(readFileSync(dashboardPidPath, 'utf8') || '').trim();
      if (pid) {
        try {
          const probe = execSync(`ps -p ${pid} -o pid=`, { encoding: 'utf8' }).trim();
          if (!probe) {
            issues.push({
              type: 'stale_dashboard_pid_file',
              detail: `dashboard_ui.pid points to non-running pid ${pid}`,
              path: 'local/state/ops/daemon_control/dashboard_ui.pid',
            });
          }
        } catch {
          issues.push({
            type: 'stale_dashboard_pid_file',
            detail: `dashboard_ui.pid points to non-running pid ${pid}`,
            path: 'local/state/ops/daemon_control/dashboard_ui.pid',
          });
        }
      }
    } catch {
      // ignore pid-read failures
    }
  }

  return issues;
}

function toMarkdown(payload) {
  const lines = [];
  lines.push('# Churn Guard (Current)');
  lines.push('');
  lines.push(`Generated: ${payload.generated_at}`);
  lines.push('');
  lines.push('## Summary');
  lines.push(`- strict: ${payload.summary.strict}`);
  lines.push(`- commit_gate: ${payload.summary.commit_gate}`);
  lines.push(`- total_dirty_entries: ${payload.summary.total}`);
  lines.push(`- local_simulation_churn: ${payload.summary.local_simulation_churn}`);
  lines.push(`- lensmap_churn: ${payload.summary.lensmap_churn}`);
  lines.push(`- generated_report_churn: ${payload.summary.generated_report_churn}`);
  lines.push(`- session_churn: ${payload.summary.session_churn}`);
  lines.push(`- governance_doc_churn: ${payload.summary.governance_doc_churn}`);
  lines.push(`- swarm_surface_churn: ${payload.summary.swarm_surface_churn}`);
  lines.push(`- swarm_companion_gaps: ${payload.summary.swarm_companion_gaps}`);
  lines.push(`- allow_governance_doc_churn: ${payload.summary.allow_governance_doc_churn}`);
  lines.push(`- likely_unstaged_moves: ${payload.summary.likely_unstaged_moves}`);
  lines.push(`- allowed_assurance_physical_domain_moves: ${payload.summary.allowed_assurance_physical_domain_moves}`);
  lines.push(`- blocking_likely_unstaged_moves: ${payload.summary.blocking_likely_unstaged_moves}`);
  lines.push(`- untracked: ${payload.summary.untracked}`);
  lines.push(`- commit_gate_forbidden: ${payload.summary.commit_gate_forbidden}`);
  lines.push(`- other: ${payload.summary.other}`);
  lines.push(`- clean_pass: ${payload.summary.clean_pass}`);
  lines.push(`- commit_gate_pass: ${payload.summary.commit_gate_pass}`);
  lines.push(`- pass: ${payload.summary.pass}`);
  lines.push('');
  if (payload.session_churn_signals.length > 0) {
    lines.push('## Session Churn Signals');
    lines.push('| Type | Detail |');
    lines.push('| --- | --- |');
    for (const signal of payload.session_churn_signals) {
      lines.push(`| ${signal.type} | ${signal.detail} |`);
    }
    lines.push('');
    lines.push(
      'Remediation: keep one dashboard host process, clear stale dashboard pid state, and relaunch once.',
    );
    lines.push('');
  }
  if (payload.likely_unstaged_moves.length > 0) {
    lines.push('## Likely Unstaged Move Pairs');
    lines.push('| From (deleted) | To (untracked) | Classification |');
    lines.push('| --- | --- | --- |');
    const allowed = new Set(
      (payload.allowed_assurance_physical_domain_moves || []).map(
        (pair) => `${pair.from}->${pair.to}`,
      ),
    );
    for (const pair of payload.likely_unstaged_moves.slice(0, 80)) {
      const classification = allowed.has(`${pair.from}->${pair.to}`)
        ? 'allowed_assurance_physical_domain_move'
        : 'blocking_unstaged_move';
      lines.push(`| ${pair.from} | ${pair.to} | ${classification} |`);
    }
    lines.push('');
    lines.push('Remediation: blocking moves should be staged as a single rename set (`git add -A`) before continuing; allowed Assurance physical-domain moves are expected during migration burn-down.');
    lines.push('');
  }
  if (payload.swarm_companion_gaps.length > 0) {
    lines.push('## Swarm Companion Gaps');
    lines.push('| Type | Detail |');
    lines.push('| --- | --- |');
    for (const gap of payload.swarm_companion_gaps) {
      lines.push(`| ${gap.type} | ${gap.detail} |`);
    }
    lines.push('');
    lines.push(
      'Remediation: stage swarm runtime/bridge changes together with swarm tests and SRS/REQ evidence updates.',
    );
    lines.push('');
  }
  if (payload.rows.length > 0) {
    lines.push('| Status | Category | Path |');
    lines.push('| --- | --- | --- |');
    for (const row of payload.rows) {
      lines.push(`| ${row.status.trim()} | ${row.category} | ${row.path} |`);
    }
    lines.push('');
  }
  return `${lines.join('\n')}\n`;
}

function main() {
  const started = Date.now();
  const args = parseArgs(process.argv.slice(2));
  const rows = args.commitGate ? parseStagedStatus() : parseStatus();
  const sessionChurnSignals = args.commitGate ? [] : detectSessionChurnSignals();
  const likelyUnstagedMoves = detectLikelyUnstagedMoves(rows);
  const allowedAssurancePhysicalDomainMoves = likelyUnstagedMoves.filter(
    isAllowedAssurancePhysicalDomainMove,
  );
  const blockingLikelyUnstagedMoves = likelyUnstagedMoves.filter(
    (pair) => !isAllowedAssurancePhysicalDomainMove(pair),
  );
  const swarmCompanionGaps = detectSwarmCompanionGaps(rows);
  const untrackedRows = rows.filter((row) => isUntracked(row.status));
  const forbiddenCommitCategories = new Set([
    'local_simulation_churn',
    'lensmap_churn',
    'generated_report_churn',
    'session_churn',
  ]);
  const forbiddenCommitRows = rows.filter((row) => forbiddenCommitCategories.has(row.category));
  const governanceCommitRows = rows.filter((row) => row.category === 'governance_doc_churn');
  const nonGovernanceRows = rows.filter((row) => row.category !== 'governance_doc_churn');
  const governanceOnlyChurn = governanceCommitRows.length > 0 && nonGovernanceRows.length === 0;
  const commitGatePass =
    forbiddenCommitRows.length === 0 &&
    (args.allowGovernanceDocChurn || governanceCommitRows.length === 0 || !governanceOnlyChurn) &&
    blockingLikelyUnstagedMoves.length === 0 &&
    swarmCompanionGaps.length === 0 &&
    untrackedRows.length === 0 &&
    sessionChurnSignals.length === 0;

  const summary = {
    strict: args.strict,
    commit_gate: args.commitGate,
    scope: args.commitGate ? 'staged' : 'worktree',
    total: rows.length,
    local_simulation_churn: rows.filter((r) => r.category === 'local_simulation_churn').length,
    lensmap_churn: rows.filter((r) => r.category === 'lensmap_churn').length,
    generated_report_churn: rows.filter((r) => r.category === 'generated_report_churn').length,
    session_churn:
      rows.filter((r) => r.category === 'session_churn').length + sessionChurnSignals.length,
    governance_doc_churn: rows.filter((r) => r.category === 'governance_doc_churn').length,
    swarm_surface_churn: rows.filter((r) => r.category === 'swarm_surface_churn').length,
    swarm_companion_gaps: swarmCompanionGaps.length,
    allow_governance_doc_churn: args.allowGovernanceDocChurn,
    likely_unstaged_moves: likelyUnstagedMoves.length,
    allowed_assurance_physical_domain_moves: allowedAssurancePhysicalDomainMoves.length,
    blocking_likely_unstaged_moves: blockingLikelyUnstagedMoves.length,
    untracked: untrackedRows.length,
    commit_gate_forbidden: forbiddenCommitRows.length,
    other: rows.filter((r) => r.category === 'other').length,
  };
  summary.clean_pass =
    summary.local_simulation_churn === 0 &&
    summary.generated_report_churn === 0 &&
    summary.session_churn === 0 &&
    (summary.governance_doc_churn === 0 || args.allowGovernanceDocChurn) &&
    summary.swarm_companion_gaps === 0 &&
    summary.blocking_likely_unstaged_moves === 0;
  summary.commit_gate_pass = commitGatePass;
  summary.pass = args.commitGate ? summary.commit_gate_pass : summary.clean_pass;

  const failures = [];
  if (!summary.pass) {
    failures.push({
      id: args.commitGate ? 'commit_gate_failed' : 'clean_pass_failed',
      detail: JSON.stringify(summary),
    });
  }
  if (sessionChurnSignals.length > 0) {
    failures.push({
      id: 'session_churn_signals_present',
      detail: sessionChurnSignals.map((row) => row.type).join(','),
    });
  }
  if (blockingLikelyUnstagedMoves.length > 0) {
    failures.push({
      id: 'likely_unstaged_moves_present',
      detail: blockingLikelyUnstagedMoves.map((row) => `${row.from}->${row.to}`).join(',').slice(0, 500),
    });
  }
  if (swarmCompanionGaps.length > 0) {
    failures.push({
      id: 'swarm_companion_gaps_present',
      detail: swarmCompanionGaps.map((row) => row.type).join(','),
    });
  }

  const payload = {
    ok: summary.pass,
    type: 'churn_guard',
    generated_at: new Date().toISOString(),
    duration_ms: Date.now() - started,
    owner: 'ops',
    revision: currentRevision(process.cwd()),
    inputs: {
      strict: args.strict,
      commit_gate: args.commitGate,
      allow_governance_doc_churn: args.allowGovernanceDocChurn,
      out_json: args.outJson,
      out_markdown: args.outMarkdown,
    },
    summary,
    failures,
    artifact_paths: [args.outJson, args.outMarkdown],
    session_churn_signals: sessionChurnSignals,
    likely_unstaged_moves: likelyUnstagedMoves,
    allowed_assurance_physical_domain_moves: allowedAssurancePhysicalDomainMoves,
    blocking_likely_unstaged_moves: blockingLikelyUnstagedMoves,
    swarm_companion_gaps: swarmCompanionGaps,
    rows,
  };

  writeJsonArtifact(resolve(args.outJson), payload);
  writeTextArtifact(resolve(args.outMarkdown), toMarkdown(payload));
  return emitStructuredResult(payload, {
    outPath: '',
    strict: args.strict,
    ok: payload.ok,
  });
}

process.exit(main());
