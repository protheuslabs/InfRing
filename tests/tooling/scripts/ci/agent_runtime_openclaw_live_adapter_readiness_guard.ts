#!/usr/bin/env node
/* eslint-disable no-console */

'use strict';

const fs = require('node:fs');
const path = require('node:path');

const ROOT = process.cwd();
const OUT_JSON = path.join(ROOT, 'core/local/artifacts/agent_runtime_openclaw_live_adapter_readiness_guard_current.json');
const REGISTRY_PATH = path.join(ROOT, 'validation/conformance/contracts/agent_runtime_engine_registry.json');
const GRADUATION_CONTRACT_PATH = path.join(ROOT, 'validation/conformance/contracts/agent_runtime_graduation_baseline_contract.json');

const ALLOWED_READINESS_STATUSES = new Set([
  'available',
  'installed_not_running',
  'not_connected',
  'not_downloaded',
  'configurable',
  'reference_checkout_entrypoint_available',
  'runtime_requirement_missing',
]);

function clean(value, max = 1000) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function ensureDir(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function rel(filePath) {
  return path.relative(ROOT, filePath).replace(/\\/g, '/');
}

function readJson(filePath, violations) {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch (error) {
    violations.push({
      kind: 'json_read_failed',
      path: rel(filePath),
      error: clean(error && error.message ? error.message : error, 1600),
    });
    return null;
  }
}

function writeReport(report) {
  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  const out = JSON.stringify(report, null, 2);
  if (report.ok) console.log(out);
  else console.error(out);
}

async function main() {
  const violations = [];
  const registry = readJson(REGISTRY_PATH, violations);
  const graduationContract = readJson(GRADUATION_CONTRACT_PATH, violations);
  const engines = Array.isArray(registry && registry.engines) ? registry.engines : [];
  const openclaw = engines.find((engine) => clean(engine && engine.engine_id, 120) === 'openclaw') || null;
  const baseline = graduationContract && graduationContract.current_baseline ? graduationContract.current_baseline : {};
  const dailyDriverAllowed = new Set(Array.isArray(baseline.daily_driver_allowed_now) ? baseline.daily_driver_allowed_now.map((item) => clean(item, 120)) : []);
  const secondarySamples = graduationContract && graduationContract.secondary_eval_samples && typeof graduationContract.secondary_eval_samples === 'object'
    ? graduationContract.secondary_eval_samples
    : {};
  const readinessSamples = Array.isArray(secondarySamples.live_adapter_readiness)
    ? secondarySamples.live_adapter_readiness.map((item) => clean(item, 120)).filter(Boolean)
    : [];

  if (!openclaw) violations.push({ kind: 'openclaw_registry_row_missing' });
  if (dailyDriverAllowed.has('openclaw')) violations.push({ kind: 'openclaw_must_not_be_daily_driver_allowed_from_readiness_probe' });
  if (!readinessSamples.includes('openclaw')) violations.push({ kind: 'openclaw_missing_from_secondary_live_adapter_readiness_samples' });

  let health = null;
  let submitTurn = null;
  if (openclaw) {
    const { createOpenClawEngineAdapter } = require(path.join(ROOT, 'adapters/runtime/agent_engines/openclaw.ts'));
    const adapter = createOpenClawEngineAdapter({ env: process.env });
    const message = {
      trace_id: `validation:openclaw-live-adapter-readiness:${Date.now()}`,
      request_id: `openclaw-readiness-${Date.now()}`,
      engine_id: 'openclaw',
      session_id: 'agent-runtime-openclaw-live-adapter-readiness',
      turn_id: 'openclaw-readiness-turn',
    };
    health = await adapter.health_check({ engine: openclaw, message });
    const status = clean(health && health.status, 160);
    if (!ALLOWED_READINESS_STATUSES.has(status)) {
      violations.push({ kind: 'openclaw_readiness_status_unexpected', status });
    }
    if (status === 'available') {
      submitTurn = await adapter.submit_turn({ engine: openclaw, message: { ...message, input: { text: 'readiness probe' } } });
      if (clean(submitTurn && submitTurn.error_code, 200) !== 'openclaw_live_turn_adapter_not_attached') {
        violations.push({
          kind: 'openclaw_health_available_must_not_imply_turn_adapter_attached',
          error_code: clean(submitTurn && submitTurn.error_code, 200),
          status: clean(submitTurn && submitTurn.status, 200),
        });
      }
    }
  }

  const healthStatus = clean(health && health.status, 160);
  const healthAvailable = healthStatus === 'available';
  const referenceCheckoutAvailable = healthStatus === 'reference_checkout_entrypoint_available';
  const runtimeRequirementMissing = healthStatus === 'runtime_requirement_missing';
  const installedNotRunning = healthStatus === 'installed_not_running';
  const report = {
    ok: violations.length === 0,
    type: 'agent_runtime_openclaw_live_adapter_readiness_guard',
    generated_at: new Date().toISOString(),
    source_domain: 'validation',
    owner_domain: 'validation.agent_runtime',
    layer: 'gateway',
    policy_path: rel(GRADUATION_CONTRACT_PATH),
    engine_id: 'openclaw',
    readiness: {
      status: healthStatus || 'not_checked',
      available: healthAvailable,
      reference_checkout_available: referenceCheckoutAvailable,
      runtime_requirement_missing: runtimeRequirementMissing,
      installed_not_running: installedNotRunning,
      discovery_source: clean(health && health.discovery_source, 200),
      url: clean(health && health.url, 500),
      command: clean(health && health.command, 500),
      resolved_path: clean(health && health.resolved_path, 500),
      health_path: clean(health && health.health_path, 200),
      health_status_code: Number(health && health.health_status_code) || 0,
      runtime_requirement: clean(health && health.runtime_requirement, 200),
      current_runtime: clean(health && health.current_runtime, 200),
      reason: clean(health && health.reason, 700),
    },
    promotion_effect: {
      changes_daily_driver_eligibility: false,
      live_adapter_evidence_pass: healthAvailable && submitTurn && clean(submitTurn.error_code, 200) !== 'openclaw_live_turn_adapter_not_attached',
      rule: 'Readiness probing is diagnostic only. OpenClaw still needs a real Gateway-mediated live turn adapter before daily-driver promotion.',
    },
    next_actions: healthAvailable
      ? [
          'Attach OpenClaw live turn submission behind Gateway and prove bounded activity, approval, receipts, and artifact effects.',
          'Keep OpenClaw blocked from daily-driver promotion until the full golden-pair baseline passes.',
        ]
      : runtimeRequirementMissing
        ? [
            'Install or switch to the required OpenClaw runtime environment.',
            'Start the OpenClaw Gateway/socket service and rerun this readiness guard.',
          ]
        : referenceCheckoutAvailable || installedNotRunning
          ? [
              'Start the OpenClaw Gateway/socket service from the detected command or reference checkout.',
              'Rerun this readiness guard, then run live Gateway-mediated adapter evidence.',
            ]
          : [
              'Install or configure OpenClaw using the registry discovery metadata.',
              'Set a custom OpenClaw URL/command if the service is installed outside default locations.',
            ],
    source_artifact_refs: [
      rel(REGISTRY_PATH),
      rel(GRADUATION_CONTRACT_PATH),
    ],
    violations,
  };

  writeReport(report);
  if (!report.ok) process.exit(1);
}

main().catch((error) => {
  writeReport({
    ok: false,
    type: 'agent_runtime_openclaw_live_adapter_readiness_guard',
    generated_at: new Date().toISOString(),
    error: clean(error && error.stack ? error.stack : error, 6000),
  });
  process.exit(1);
});
