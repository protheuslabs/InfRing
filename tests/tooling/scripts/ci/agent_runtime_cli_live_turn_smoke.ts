#!/usr/bin/env node
/* eslint-disable no-console */

// Opt-in live-turn smoke for local CLI runtime engines. This intentionally runs
// real external agent turns only when explicitly requested because these engines
// may spend provider quota. It uses Gateway adapter discovery instead of any
// machine-specific hardcoded user path.

const fs = require('node:fs');
const path = require('node:path');
const ROOT = process.cwd();
const outPath = 'core/local/artifacts/agent_runtime_cli_live_turn_smoke_current.json';

function argValue(name) {
  const prefix = `${name}=`;
  const match = process.argv.slice(2).find((arg) => arg === name || arg.startsWith(prefix));
  if (!match) return '';
  if (match === name) return '1';
  return match.slice(prefix.length);
}

function cleanString(value, max = 4000) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function load(rel) {
  return require(path.join(ROOT, rel));
}

const live = argValue('--live') === '1' || process.env.INFRING_AGENT_RUNTIME_LIVE_TURN_SMOKE === '1';
const requestedEngines = cleanString(argValue('--engines') || process.env.INFRING_AGENT_RUNTIME_LIVE_TURN_ENGINES || 'codex_cli,claude_code,grok_code', 1000)
  .split(',')
  .map((entry) => cleanString(entry, 120))
  .filter(Boolean);
const expected = cleanString(argValue('--expected') || 'INFRING_RUNTIME_TURN_OK', 200);
const timeoutSeconds = Math.max(10, Math.min(Number(argValue('--timeout-seconds') || process.env.INFRING_AGENT_RUNTIME_LIVE_TURN_TIMEOUT_SECONDS || 180) || 180, 300));
const prompt = [
  'This is an InfRing Gateway runtime adapter live-turn smoke test.',
  'Do not inspect files. Do not run tools. Do not modify anything. Do not use web search.',
  `Reply with exactly: ${expected}`,
].join(' ');

function classifyTurnOutcome(turn, output) {
  const status = cleanString(turn && turn.status, 120);
  if (status === 'completed' && output) return 'completed';
  if (turn && turn.permission_request) return 'permission_required';
  if (turn && turn.timed_out) return 'timed_out_with_reason';
  if (status === 'timed_out') return 'timed_out_with_reason';
  if (status === 'failed' && (turn.error_code || turn.reason || output)) return 'failed_with_reason';
  return 'silent_or_invalid';
}

async function main() {
  if (!live) {
    const payload = {
      ok: true,
      type: 'agent_runtime_cli_live_turn_smoke',
      mode: 'skipped_not_live',
      reason: 'Set --live=1 or INFRING_AGENT_RUNTIME_LIVE_TURN_SMOKE=1 to run real external agent turns.',
      requested_engines: requestedEngines,
    };
    fs.mkdirSync(path.dirname(path.join(ROOT, outPath)), { recursive: true });
    fs.writeFileSync(path.join(ROOT, outPath), `${JSON.stringify(payload, null, 2)}\n`);
    console.log(JSON.stringify(payload, null, 2));
    return;
  }

  const { createAgentRuntimeRouter } = load('adapters/runtime/agent_engines/agent_runtime_router.ts');
  const { createCodexCliEngineAdapter } = load('adapters/runtime/agent_engines/codex_cli.ts');
  const { createClaudeCodeEngineAdapter } = load('adapters/runtime/agent_engines/claude_code.ts');
  const { createGrokCodeEngineAdapter } = load('adapters/runtime/agent_engines/grok_code.ts');

  const router = createAgentRuntimeRouter({ root: ROOT, disableTraceWriter: true });
  const factories = {
    codex_cli: () => createCodexCliEngineAdapter({ liveDispatch: true }),
    claude_code: () => createClaudeCodeEngineAdapter({ liveDispatch: true }),
    grok_code: () => createGrokCodeEngineAdapter({ liveDispatch: true }),
  };

  const results = [];
  for (const engineId of requestedEngines) {
    if (!factories[engineId]) {
      results.push({ engine_id: engineId, ok: false, status: 'unsupported_engine' });
      continue;
    }
    router.registerAdapter(engineId, factories[engineId]());
    const base = {
      trace_id: `trace-live-turn-${engineId}-${Date.now()}`,
      request_id: `request-live-turn-${engineId}`,
      engine_id: engineId,
      session_id: `live-turn-smoke-${engineId}`,
      turn_id: `turn-${Date.now()}`,
    };
    const health = await router.healthCheck({ type: 'engine.health', ...base });
    let turn = null;
    if (health.status === 'available') {
      turn = await router.submitTurn({
        type: 'turn.submit',
        ...base,
        input: { text: prompt },
        scope: { workspace: 'none', mutation_allowed: false, tools_allowed: false },
        capability_budget: {
          tool_calls: 0,
          file_reads: 0,
          file_writes: 0,
          command_runs: 0,
          max_turn_seconds: timeoutSeconds,
          shell_projection_only: true,
        },
      });
    }
    const output = cleanString(turn && (turn.output_preview || turn.output_text || turn.delta || turn.reason), 4000);
    const turnOutcome = classifyTurnOutcome(turn, output);
    const silentOutcome = turnOutcome === 'silent_or_invalid';
    results.push({
      engine_id: engineId,
      ok: health.status === 'available' && turn && turn.type === 'turn.complete' && turnOutcome === 'completed' && output.includes(expected),
      turn_outcome: turnOutcome,
      silent_outcome: silentOutcome,
      health_status: health.status,
      discovery_source: health.discovery_source,
      command: health.command,
      resolved_path: health.resolved_path || null,
      version_preview: cleanString(health.version_preview, 500),
      turn_type: turn && turn.type,
      turn_status: turn && turn.status,
      error_code: (turn && turn.error_code) || (health && health.error_code) || null,
      reason: cleanString(turn && turn.reason, 1000),
      timed_out: turn && turn.timed_out === true,
      timeout_ms: Number(turn && turn.timeout_ms) || timeoutSeconds * 1000,
      output_preview: output,
    });
  }

  const payload = {
    ok: results.every((row) => row.ok),
    type: 'agent_runtime_cli_live_turn_smoke',
    mode: 'live',
    expected,
    timeout_seconds: timeoutSeconds,
    turn_outcome_contract: 'validation/conformance/contracts/agent_runtime_turn_outcome_contract.json',
    results,
  };
  fs.mkdirSync(path.dirname(path.join(ROOT, outPath)), { recursive: true });
  fs.writeFileSync(path.join(ROOT, outPath), `${JSON.stringify(payload, null, 2)}\n`);
  console.log(JSON.stringify(payload, null, 2));
  if (!payload.ok) process.exit(1);
}

main().catch((err) => {
  const payload = {
    ok: false,
    type: 'agent_runtime_cli_live_turn_smoke',
    error: cleanString(err && err.stack ? err.stack : err, 4000),
  };
  fs.mkdirSync(path.dirname(path.join(ROOT, outPath)), { recursive: true });
  fs.writeFileSync(path.join(ROOT, outPath), `${JSON.stringify(payload, null, 2)}\n`);
  console.error(JSON.stringify(payload, null, 2));
  process.exit(1);
});
