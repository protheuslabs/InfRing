#!/usr/bin/env node
/* eslint-disable no-console */

'use strict';

const childProcess = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const ROOT = process.cwd();
const OUT_JSON = path.join(ROOT, 'core/local/artifacts/agent_runtime_secondary_live_adapter_readiness_guard_current.json');
const CONTRACT_PATH = path.join(ROOT, 'validation/conformance/contracts/agent_runtime_graduation_baseline_contract.json');

const ENGINES = [
  {
    engine_id: 'grok_code',
    display_name: 'Grok Code',
    mapping_ref: 'core/local/artifacts/agent_runtime_grok_code_mapping_probe_current.json',
    live_probe_ref: 'core/local/artifacts/agent_runtime_grok_code_live_acceptance_probe_current.json',
    expected_mapping_type: 'agent_runtime_grok_code_mapping_probe',
    expected_live_type: 'agent_runtime_grok_code_live_acceptance_probe',
    command: process.env.INFRING_GROK_CODE_BIN || process.env.INFRING_GROK_CODE_PATH || 'grok',
    opt_in_env: 'AGENT_RUNTIME_GROK_CODE_LIVE=1',
    command_required: true,
    package_required: '',
  },
  {
    engine_id: 'opencode',
    display_name: 'OpenCode',
    mapping_ref: 'core/local/artifacts/agent_runtime_opencode_sdk_mapping_probe_current.json',
    live_probe_ref: 'core/local/artifacts/agent_runtime_opencode_sdk_live_acceptance_probe_current.json',
    expected_mapping_type: 'agent_runtime_opencode_sdk_mapping_probe',
    expected_live_type: 'agent_runtime_opencode_sdk_live_acceptance_probe',
    command: process.env.INFRING_OPENCODE_BIN || process.env.INFRING_OPENCODE_PATH || 'opencode',
    opt_in_env: 'AGENT_RUNTIME_OPENCODE_SDK_LIVE=1',
    command_required: false,
    package_required: '@opencode-ai/sdk',
  },
  {
    engine_id: 'hermes_agent',
    display_name: 'Hermes Agent',
    mapping_ref: 'core/local/artifacts/agent_runtime_hermes_agent_mapping_probe_current.json',
    live_probe_ref: 'core/local/artifacts/agent_runtime_hermes_agent_live_acceptance_probe_current.json',
    expected_mapping_type: 'agent_runtime_hermes_agent_mapping_probe',
    expected_live_type: 'agent_runtime_hermes_agent_live_acceptance_probe',
    command: process.env.INFRING_HERMES_AGENT_BIN || process.env.INFRING_HERMES_AGENT_PATH || 'hermes',
    opt_in_env: 'AGENT_RUNTIME_HERMES_AGENT_LIVE=1',
    command_required: true,
    package_required: '',
    reference_checkout_path: 'references/coding-agent-systems/hermes-agent',
    setup_commands: [
      'cd references/coding-agent-systems/hermes-agent && ./setup-hermes.sh',
      'curl -fsSL https://hermes-agent.nousresearch.com/install.sh | bash',
    ],
  },
];

function clean(value, max = 1000) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function ensureDir(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function rel(filePath) {
  return path.relative(ROOT, filePath).replace(/\\/g, '/');
}

function readJsonRel(relPath) {
  try {
    return JSON.parse(fs.readFileSync(path.join(ROOT, relPath), 'utf8'));
  } catch {
    return null;
  }
}

function commandProbe(command) {
  const text = clean(command, 500);
  if (!text) return { available: false, resolved_path: '', reason: 'command_empty' };
  if (path.isAbsolute(text)) {
    return {
      available: fs.existsSync(text),
      resolved_path: fs.existsSync(text) ? text : '',
      reason: fs.existsSync(text) ? 'absolute_path_exists' : 'absolute_path_missing',
    };
  }
  const child = childProcess.spawnSync('/bin/sh', ['-lc', `command -v ${JSON.stringify(text)}`], {
    cwd: ROOT,
    encoding: 'utf8',
    timeout: 5000,
    maxBuffer: 64 * 1024,
  });
  return {
    available: child.status === 0 && clean(child.stdout, 500) !== '',
    resolved_path: clean(child.stdout, 500),
    reason: child.status === 0 ? 'command_found' : 'command_not_found',
  };
}

function packageProbe(packageName) {
  const name = clean(packageName, 200);
  if (!name) return { required: false, available: true, resolved_path: '', reason: 'not_required' };
  try {
    const resolved = require.resolve(name, { paths: [ROOT] });
    return { required: true, available: true, resolved_path: clean(resolved, 700), reason: 'package_found' };
  } catch (error) {
    return { required: true, available: false, resolved_path: '', reason: 'package_not_found', error: clean(error && error.message, 500) };
  }
}

function referenceCheckoutProbe(engine) {
  const relPath = clean(engine.reference_checkout_path, 500);
  if (!relPath) return { available: false, path: '', setup_hint: '', reason: 'not_declared' };
  const absPath = path.join(ROOT, relPath);
  const setupScript = path.join(absPath, 'setup-hermes.sh');
  const readme = path.join(absPath, 'README.md');
  return {
    available: fs.existsSync(absPath) && fs.existsSync(readme),
    path: relPath,
    setup_hint: fs.existsSync(setupScript) ? `${relPath}/setup-hermes.sh` : '',
    reason: fs.existsSync(absPath) && fs.existsSync(readme)
      ? 'reference_checkout_available'
      : 'reference_checkout_missing',
  };
}

function statusFor(engine, mapping, liveProbe, command, pkg, referenceCheckout) {
  const mappingReady = mapping && mapping.ok === true && mapping.native_transport_probe_status === 'candidate_mapping_ready';
  const liveAccepted = liveProbe && liveProbe.ok === true && liveProbe.accepted_by_runtime === true && liveProbe.typed_turn_api_available === true;
  if (!mappingReady) return 'mapping_not_ready';
  if (liveAccepted) return 'live_probe_previously_accepted';
  if (pkg.required && !pkg.available) return 'sdk_package_missing';
  if (engine.command_required && !command.available && referenceCheckout.available) return 'reference_checkout_available';
  if (engine.command_required && !command.available) return 'runtime_command_missing';
  if (liveProbe && liveProbe.ok === true && liveProbe.live_enabled === false) return 'ready_for_opt_in_live_probe';
  if (liveProbe && liveProbe.provider_unavailable === true) return 'provider_or_auth_unavailable';
  if (liveProbe && liveProbe.ok !== true) return 'live_probe_needs_attention';
  return 'live_probe_artifact_missing';
}

function liveProbeScriptRef(engine) {
  return engine.live_probe_ref
    .replace('core/local/artifacts/', 'tests/tooling/scripts/ci/')
    .replace(/_current\.json$/, '.ts');
}

function nextActions(engine, status, command, pkg) {
  if (status === 'live_probe_previously_accepted') {
    return [
      `Run live Gateway-mediated adapter evidence for ${engine.display_name}; the native acceptance probe has previously accepted typed transport.`,
      'Keep this diagnostic separate from daily-driver promotion until full golden-pair baseline evidence passes.',
    ];
  }
  if (status === 'ready_for_opt_in_live_probe') {
    return [
      `Run ${engine.opt_in_env} node client/runtime/lib/ts_entrypoint.ts ${liveProbeScriptRef(engine)} when provider/API usage is intentionally allowed.`,
      'If the opt-in probe accepts, follow with Gateway-mediated live adapter evidence before promotion.',
    ];
  }
  if (status === 'sdk_package_missing') {
    return [
      `Install ${engine.package_required} for ${engine.display_name}, then rerun this readiness guard.`,
      `After the SDK is present, run the opt-in probe with ${engine.opt_in_env}.`,
    ];
  }
  if (status === 'runtime_command_missing') {
    return [
      `Install or configure ${engine.display_name}; expected command was ${engine.command}.`,
      `Set a custom command/path env var if installed outside PATH, then rerun this readiness guard.`,
    ];
  }
  if (status === 'reference_checkout_available') {
    const setup = Array.isArray(engine.setup_commands) && engine.setup_commands.length
      ? engine.setup_commands[0]
      : `Install or configure ${engine.display_name}`;
    return [
      `Set up the checked-out ${engine.display_name} reference runtime with: ${setup}`,
      `Then set a command/path override if needed and rerun this readiness guard before ${engine.opt_in_env}.`,
    ];
  }
  if (status === 'provider_or_auth_unavailable') {
    return [
      `Restore provider/auth/usage readiness for ${engine.display_name}, then rerun the opt-in live probe.`,
      'Keep provider failures visible but non-promotional until the probe accepts.',
    ];
  }
  if (status === 'mapping_not_ready') {
    return [
      `Fix the candidate mapping probe for ${engine.display_name} before attempting live acceptance.`,
    ];
  }
  return [
    `Generate or repair the disabled live acceptance artifact for ${engine.display_name}.`,
    `Then run this readiness guard again before spending live provider usage.`,
  ];
}

function writeEngineReport(engine, row) {
  const outPath = path.join(ROOT, `core/local/artifacts/agent_runtime_${engine.engine_id}_live_adapter_readiness_guard_current.json`);
  ensureDir(outPath);
  fs.writeFileSync(outPath, `${JSON.stringify(row, null, 2)}\n`, 'utf8');
  return rel(outPath);
}

function main() {
  const contract = readJsonRel('validation/conformance/contracts/agent_runtime_graduation_baseline_contract.json') || {};
  const readinessSamples = contract.secondary_eval_samples && Array.isArray(contract.secondary_eval_samples.live_adapter_readiness)
    ? contract.secondary_eval_samples.live_adapter_readiness.map((item) => clean(item, 120))
    : [];
  const violations = [];
  const rows = ENGINES.map((engine) => {
    const mapping = readJsonRel(engine.mapping_ref);
    const liveProbe = readJsonRel(engine.live_probe_ref);
    const command = commandProbe(engine.command);
    const pkg = packageProbe(engine.package_required);
    const referenceCheckout = referenceCheckoutProbe(engine);
    const status = statusFor(engine, mapping, liveProbe, command, pkg, referenceCheckout);
    if (!readinessSamples.includes(engine.engine_id)) {
      violations.push({ kind: 'engine_missing_from_secondary_live_adapter_readiness_samples', engine_id: engine.engine_id });
    }
    if (!mapping || mapping.type !== engine.expected_mapping_type) {
      violations.push({ kind: 'mapping_artifact_missing_or_wrong_type', engine_id: engine.engine_id, artifact_ref: engine.mapping_ref });
    }
    if (!liveProbe || liveProbe.type !== engine.expected_live_type) {
      violations.push({ kind: 'live_probe_artifact_missing_or_wrong_type', engine_id: engine.engine_id, artifact_ref: engine.live_probe_ref });
    }
    const row = {
      ok: true,
      type: 'agent_runtime_secondary_live_adapter_readiness_guard',
      generated_at: new Date().toISOString(),
      source_domain: 'validation',
      owner_domain: 'validation.agent_runtime',
      layer: 'gateway',
      policy_path: rel(CONTRACT_PATH),
      engine_id: engine.engine_id,
      display_name: engine.display_name,
      readiness: {
        status,
        reason: status === 'ready_for_opt_in_live_probe'
          ? 'Candidate mapping and disabled live acceptance probe are ready; live provider/runtime use is still opt-in.'
          : status === 'live_probe_previously_accepted'
            ? 'A prior live acceptance probe succeeded; full Gateway-mediated live adapter evidence is still required.'
            : status === 'sdk_package_missing'
              ? `Required SDK package ${engine.package_required} is not resolvable from this workspace.`
            : status === 'runtime_command_missing'
              ? `Runtime command ${engine.command} is not currently discoverable on PATH or as an absolute file.`
              : status === 'reference_checkout_available'
                ? `Runtime command ${engine.command} is not discoverable yet, but the reference checkout exists at ${referenceCheckout.path}.`
                : status === 'provider_or_auth_unavailable'
                  ? 'Previous opt-in live probe reached a provider/auth/usage readiness boundary.'
                  : status,
        mapping_ready: mapping && mapping.ok === true && mapping.native_transport_probe_status === 'candidate_mapping_ready',
        live_probe_mode: clean(liveProbe && liveProbe.mode, 160),
        live_probe_ok: liveProbe ? liveProbe.ok === true : false,
        live_probe_ref: engine.live_probe_ref,
        opt_in_env: engine.opt_in_env,
        command: clean(engine.command, 500),
        command_available: command.available,
        command_resolved_path: command.resolved_path,
        command_reason: command.reason,
        package_required: engine.package_required,
        package_available: pkg.available,
        package_resolved_path: pkg.resolved_path,
        package_reason: pkg.reason,
        reference_checkout_available: referenceCheckout.available,
        reference_checkout_path: referenceCheckout.path,
        reference_checkout_setup_hint: referenceCheckout.setup_hint,
        reference_checkout_reason: referenceCheckout.reason,
      },
      promotion_effect: {
        changes_daily_driver_eligibility: false,
        live_adapter_evidence_pass: false,
        rule: 'Readiness diagnostics may refine next actions but never count as daily-driver or golden-pair-equivalent live adapter evidence.',
      },
      next_actions: nextActions(engine, status, command, pkg),
      source_artifact_refs: [
        engine.mapping_ref,
        engine.live_probe_ref,
        rel(CONTRACT_PATH),
      ],
      violations: [],
    };
    const artifactRef = writeEngineReport(engine, row);
    return { ...row, artifact_ref: artifactRef };
  });
  const report = {
    ok: violations.length === 0,
    type: 'agent_runtime_secondary_live_adapter_readiness_guard',
    generated_at: new Date().toISOString(),
    source_domain: 'validation',
    owner_domain: 'validation.agent_runtime',
    layer: 'gateway',
    policy_path: rel(CONTRACT_PATH),
    purpose: 'Diagnose readiness for secondary runtime opt-in live adapter probes without spending provider/runtime usage or changing promotion eligibility.',
    engine_ids: ENGINES.map((engine) => engine.engine_id),
    summary: {
      engine_count: rows.length,
      ready_for_opt_in_count: rows.filter((row) => row.readiness.status === 'ready_for_opt_in_live_probe').length,
      previously_accepted_count: rows.filter((row) => row.readiness.status === 'live_probe_previously_accepted').length,
      blocked_count: rows.filter((row) => !['ready_for_opt_in_live_probe', 'live_probe_previously_accepted'].includes(row.readiness.status)).length,
      statuses: rows.map((row) => ({ engine_id: row.engine_id, status: row.readiness.status })),
    },
    rows,
    source_artifact_refs: [
      rel(CONTRACT_PATH),
      ...rows.map((row) => row.artifact_ref),
    ],
    violations,
  };
  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  const out = JSON.stringify(report, null, 2);
  if (report.ok) console.log(out);
  else console.error(out);
  if (!report.ok) process.exit(1);
}

main();
