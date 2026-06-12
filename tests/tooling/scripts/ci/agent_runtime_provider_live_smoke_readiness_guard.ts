#!/usr/bin/env node
/* eslint-disable no-console */

'use strict';

const childProcess = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const ROOT = process.cwd();
const ENTRYPOINT = path.join(ROOT, 'client/runtime/lib/ts_entrypoint.ts');
const OUT_JSON = path.join(ROOT, 'core/local/artifacts/agent_runtime_provider_live_smoke_readiness_guard_current.json');
const LIVE_SMOKE_ARTIFACT = path.join(ROOT, 'core/local/artifacts/agent_runtime_cli_live_turn_smoke_current.json');
const DISABLED_PROBE_ARTIFACT = path.join(ROOT, 'core/local/artifacts/agent_runtime_provider_live_smoke_readiness_skip_probe.json');
const LIVE_SMOKE_SCRIPT = 'tests/tooling/scripts/ci/agent_runtime_cli_live_turn_smoke.ts';
const REGISTRY_PATH = 'validation/conformance/contracts/agent_runtime_engine_registry.json';
const TURN_OUTCOME_PATH = 'validation/conformance/contracts/agent_runtime_turn_outcome_contract.json';
const PARITY_CONTRACT_PATH = 'validation/conformance/contracts/agent_runtime_framework_feature_parity_contract.json';
const PACKAGE_PATH = 'package.json';
const GOLDEN_PAIR = ['codex_cli', 'claude_code'];

function clean(value, max = 4000) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function ensureDir(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function readJson(relOrAbs) {
  const filePath = path.isAbsolute(relOrAbs) ? relOrAbs : path.join(ROOT, relOrAbs);
  return JSON.parse(fs.readFileSync(filePath, 'utf8'));
}

function readText(relOrAbs) {
  const filePath = path.isAbsolute(relOrAbs) ? relOrAbs : path.join(ROOT, relOrAbs);
  return fs.readFileSync(filePath, 'utf8');
}

function sameList(actual, expected) {
  const left = Array.isArray(actual) ? actual.map((item) => clean(item, 120)).filter(Boolean) : [];
  return left.length === expected.length && expected.every((item, index) => left[index] === item);
}

function main() {
  const registry = readJson(REGISTRY_PATH);
  const turnOutcome = readJson(TURN_OUTCOME_PATH);
  const parity = readJson(PARITY_CONTRACT_PATH);
  const packageJson = readJson(PACKAGE_PATH);
  const liveSmokeSource = readText(LIVE_SMOKE_SCRIPT);
  const disabledRun = childProcess.spawnSync(process.execPath, [
    ENTRYPOINT,
    LIVE_SMOKE_SCRIPT,
    `--out=${path.relative(ROOT, DISABLED_PROBE_ARTIFACT)}`,
  ], {
    cwd: ROOT,
    encoding: 'utf8',
    env: {
      ...process.env,
      INFRING_AGENT_RUNTIME_LIVE_TURN_SMOKE: '0',
      INFRING_AGENT_RUNTIME_LIVE_TURN_ENGINES: '',
    },
    timeout: 30000,
    maxBuffer: 1024 * 1024,
  });

  let disabledArtifact = {};
  try {
    disabledArtifact = readJson(DISABLED_PROBE_ARTIFACT);
  } catch {
    disabledArtifact = {};
  }

  const liveSmoke = registry.golden_pair_live_smoke || {};
  const liveRequirements = turnOutcome.live_smoke_requirements || {};
  const paritySmoke = parity.provider_live_smoke || {};
  const packageCommand = clean((packageJson.scripts || {})['ops:agent-runtime:cli-live-turn:smoke'], 1000);
  const violations = [];

  if (disabledRun.status !== 0) {
    violations.push({
      kind: 'disabled_live_smoke_run_failed',
      status: disabledRun.status,
      stderr_preview: clean(disabledRun.stderr, 1000),
    });
  }
  if (disabledArtifact.type !== 'agent_runtime_cli_live_turn_smoke') violations.push({ kind: 'disabled_artifact_type_unexpected', actual: disabledArtifact.type || 'missing' });
  if (disabledArtifact.mode !== 'skipped_not_live') violations.push({ kind: 'disabled_artifact_mode_unexpected', actual: disabledArtifact.mode || 'missing' });
  if (!sameList(disabledArtifact.requested_engines, GOLDEN_PAIR)) violations.push({ kind: 'disabled_default_engines_not_golden_pair', actual: disabledArtifact.requested_engines || [] });
  if (!sameList(liveSmoke.engines, GOLDEN_PAIR)) violations.push({ kind: 'registry_live_smoke_engines_not_golden_pair', actual: liveSmoke.engines || [] });
  if (liveSmoke.proof_artifact !== 'core/local/artifacts/agent_runtime_cli_live_turn_smoke_current.json') violations.push({ kind: 'registry_live_smoke_artifact_unexpected', actual: liveSmoke.proof_artifact || 'missing' });
  if (liveSmoke.readiness_guard !== LIVE_SMOKE_SCRIPT.replace('cli_live_turn_smoke', 'provider_live_smoke_readiness_guard')) violations.push({ kind: 'registry_readiness_guard_unexpected', actual: liveSmoke.readiness_guard || 'missing' });
  if (liveSmoke.readiness_artifact !== 'core/local/artifacts/agent_runtime_provider_live_smoke_readiness_guard_current.json') violations.push({ kind: 'registry_readiness_artifact_unexpected', actual: liveSmoke.readiness_artifact || 'missing' });
  if (!sameList(liveRequirements.engines, GOLDEN_PAIR)) violations.push({ kind: 'turn_outcome_live_smoke_engines_not_golden_pair', actual: liveRequirements.engines || [] });
  if (liveRequirements.script !== LIVE_SMOKE_SCRIPT) violations.push({ kind: 'turn_outcome_live_smoke_script_unexpected', actual: liveRequirements.script || 'missing' });
  if (liveRequirements.must_write_artifact !== 'core/local/artifacts/agent_runtime_cli_live_turn_smoke_current.json') violations.push({ kind: 'turn_outcome_live_smoke_artifact_unexpected', actual: liveRequirements.must_write_artifact || 'missing' });
  if (!sameList(paritySmoke.engines, GOLDEN_PAIR)) violations.push({ kind: 'parity_provider_live_smoke_engines_not_golden_pair', actual: paritySmoke.engines || [] });
  if (!new Set(['installed_opt_in_provider_live_pending', 'bounded_provider_live_smoke_passing']).has(paritySmoke.status)) {
    violations.push({ kind: 'parity_provider_live_smoke_status_unexpected', actual: paritySmoke.status || 'missing' });
  }
  if (!packageCommand.includes('--engines=codex_cli,claude_code')) violations.push({ kind: 'package_live_smoke_command_missing_golden_pair', actual: packageCommand });
  if (!packageCommand.includes('--allow-provider-unavailable=1')) violations.push({ kind: 'package_live_smoke_command_missing_provider_unavailable_flag', actual: packageCommand });
  if (!liveSmokeSource.includes("|| 'codex_cli,claude_code'")) violations.push({ kind: 'live_smoke_default_not_golden_pair' });
  if (liveSmokeSource.includes("|| 'codex_cli,claude_code,grok_code'")) violations.push({ kind: 'live_smoke_default_still_includes_catalog_engine' });

  const report = {
    ok: violations.length === 0,
    guard: 'agent_runtime_provider_live_smoke_readiness_guard',
    type: 'agent_runtime_provider_live_smoke_readiness_guard',
    generated_at: new Date().toISOString(),
    source_domain: 'validation',
    owner_domain: 'validation.agent_runtime',
    layer: 'gateway',
    contract_path: PARITY_CONTRACT_PATH,
    registry_path: REGISTRY_PATH,
    turn_outcome_contract_path: TURN_OUTCOME_PATH,
    live_smoke_script: LIVE_SMOKE_SCRIPT,
    live_smoke_artifact: 'core/local/artifacts/agent_runtime_cli_live_turn_smoke_current.json',
    disabled_probe_artifact: 'core/local/artifacts/agent_runtime_provider_live_smoke_readiness_skip_probe.json',
    golden_pair: GOLDEN_PAIR,
    disabled_default_run: {
      exit_status: disabledRun.status,
      mode: disabledArtifact.mode || '',
      requested_engines: disabledArtifact.requested_engines || [],
      submitted_provider_turns: false,
      canonical_live_artifact_preserved: true,
    },
    live_command: packageCommand,
    policy: {
      default_run_must_not_submit_provider_turns: true,
      default_engines_are_primary_external_pair_only: true,
      catalog_engines_require_explicit_override: true,
      provider_unavailable_is_allowed_for_local_smoke_only: true,
      live_smoke_does_not_prove_native_intelligence: true,
    },
    violations,
  };
  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`);
  console.log(JSON.stringify(report, null, 2));
  if (!report.ok) process.exit(1);
}

main();
