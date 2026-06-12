#!/usr/bin/env tsx
/* eslint-disable no-console */

'use strict';

const fs = require('fs');
const path = require('path');

const ROOT = process.cwd();
const OUT_JSON = path.join(ROOT, 'core', 'local', 'artifacts', 'agent_runtime_cli_adapter_timeout_guard_current.json');
const SOURCE_DOMAIN = 'validation';
const OWNER_DOMAIN = 'validation.agent_runtime';
const POLICY_PATH = 'validation/conformance/contracts/agent_runtime_turn_outcome_contract.json';
const LAYER = 'gateway';
const { createAgentRuntimeTurnRouteHandler } = require(path.join(ROOT, 'gateway/runtime/agent_runtime/agent_runtime_turn_routes.ts'));
const { agentRuntimePreTurnFailureProjection } = require(path.join(ROOT, 'gateway/runtime/agent_runtime/agent_runtime_turn_projection.ts'));
const { createCliRuntimeEngineAdapter } = require(path.join(ROOT, 'adapters/runtime/agent_engines/cli_runtime_adapter.ts'));

function ensureDir(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function makeResponse() {
  let resolveEnd = null;
  const ended = new Promise((resolve) => {
    resolveEnd = resolve;
  });
  return {
    statusCode: 0,
    headers: null,
    body: '',
    writableEnded: false,
    destroyed: false,
    headersSent: false,
    writeHead(statusCode, headers) {
      this.statusCode = statusCode;
      this.headers = headers || {};
      this.headersSent = true;
    },
    end(chunk = '') {
      if (chunk) this.body += String(chunk);
      this.writableEnded = true;
      if (resolveEnd) resolveEnd();
    },
    waitForEnd(timeoutMs) {
      return Promise.race([
        ended,
        new Promise((resolve) => setTimeout(resolve, timeoutMs)),
      ]);
    },
  };
}

function parsePayload(res) {
  if (!res || !res.body) return null;
  try {
    return JSON.parse(res.body);
  } catch {
    return null;
  }
}

function ensureString(value, fallback = '') {
  return value == null ? fallback : String(value);
}

function cleanText(value, max = 4000) {
  return ensureString(value, '')
    .replace(/\\s+/g, ' ')
    .trim()
    .slice(0, max);
}

async function runFailureProbe() {
  const traceId = `validation:agent-runtime-cli-adapter-failure:${Date.now()}`;
  const failureBody = {
    type: 'turn.submit',
    agent_id: 'agent-runtime-cli-adapter-failure-guard',
    session_id: 'cli-adapter-failure-session',
    conversation_id: 'cli-adapter-failure-session',
    engine_id: 'codex_cli',
    message: 'cli adapter failure reason guard probe',
    input_text: 'cli adapter failure reason guard probe',
    capability_budget: {
      max_turn_seconds: 30,
    },
    test_probe: true,
  };
  const childScript = [
    `console.error(${JSON.stringify("You've hit your usage limit for GPT-5.3-Codex-Spark. Switch to another model now, or try again at 1:49 PM.")});`,
    'process.exit(7);',
  ].join('\n');

  let adapterRun = null;
  const adapter = createCliRuntimeEngineAdapter({
    engineId: 'codex_cli',
    command: process.execPath,
    commandFallback: process.execPath,
    liveDispatch: true,
    timeoutMs: 10000,
    versionArgs: ['--version'],
    runArgs: () => ['-e', childScript],
    runStdin: (prompt) => prompt,
    cwd: ROOT,
    afterRun: (run) => {
      adapterRun = run;
    },
  });

  const message = {
    ...failureBody,
    trace_id: traceId,
    request_id: `${traceId}:request`,
    turn_id: `${traceId}:turn`,
    engine_id: 'codex_cli',
    input: {
      text: failureBody.input_text,
    },
    input_text: failureBody.input_text,
    working_directory: ROOT,
    capability_budget: {
      max_turn_seconds: 30,
    },
  };

  const projection = await adapter.stream_turn({
    message,
    engine: { engine_id: 'codex_cli' },
    onActivity: () => {},
  });

  const reasonText = cleanText(projection && projection.reason);
  const outputText = cleanText(projection && (projection.output_text || projection.output_preview || projection.reason));
  const outputPreview = cleanText(projection && projection.output_preview);

  const violations = [];
  if (!projection) violations.push('cli_adapter_failure_projection_missing');
  if (!projection || projection.status !== 'failed') violations.push('cli_adapter_failure_status_not_failed');
  if (!projection || projection.error_code !== 'codex_cli_provider_quota_or_subscription_unavailable') violations.push(`cli_adapter_failure_code_unexpected:${projection && projection.error_code || 'missing'}`);
  if (!reasonText) violations.push('cli_adapter_failure_reason_missing');
  if (reasonText && reasonText.startsWith('{')) violations.push('cli_adapter_failure_reason_looks_unparsed_json');
  if (!reasonText.toLowerCase().includes('usage limit') || !reasonText.includes('GPT-5.3-Codex-Spark') || !reasonText.includes('1:49 PM')) {
    violations.push(`cli_adapter_failure_reason_not_explanatory:${reasonText}`);
  }
  if (!outputText.toLowerCase().includes('usage limit') || !outputText.includes('GPT-5.3-Codex-Spark') || !outputText.includes('1:49 PM')) {
    violations.push(`cli_adapter_failure_output_missing_usage_limit_text:${outputText}`);
  }
  if (!adapterRun || adapterRun.timed_out === true || Number.isFinite(Number(adapterRun.exit_code)) === false) {
    violations.push('cli_adapter_failure_exit_state_unexpected');
  }

  return {
    trace_id: traceId,
    projection: projection || null,
    adapter_run_exit_code: adapterRun ? adapterRun.exit_code : null,
    adapter_run_timed_out: adapterRun ? adapterRun.timed_out === true : null,
    reason_text: reasonText,
    output_text: outputText,
    output_preview: outputPreview,
    violations,
  };
}

async function runClaudeMalformedJsonFailureProbe() {
  const traceId = `validation:agent-runtime-cli-adapter-claude-malformed-json:${Date.now()}`;
  const claudeMessage = 'Claude Code subscription is unavailable because account usage is exhausted.';
  const childScript = [
    `process.stderr.write(${JSON.stringify(JSON.stringify({ type: 'error', error: { message: claudeMessage } }, null, 2))} + '\\n');`,
    'process.exit(9);',
  ].join('\n');

  const adapter = createCliRuntimeEngineAdapter({
    engineId: 'claude_code',
    command: process.execPath,
    commandFallback: process.execPath,
    liveDispatch: true,
    timeoutMs: 10000,
    versionArgs: ['--version'],
    runArgs: () => ['-e', childScript],
    runStdin: (prompt) => prompt,
    cwd: ROOT,
  });

  const projection = await adapter.stream_turn({
    message: {
      trace_id: traceId,
      request_id: `${traceId}:request`,
      turn_id: `${traceId}:turn`,
      engine_id: 'claude_code',
      agent_id: 'agent-runtime-cli-adapter-claude-malformed-json-guard',
      session_id: 'cli-adapter-claude-malformed-json-session',
      input: { text: 'claude malformed json failure probe' },
      input_text: 'claude malformed json failure probe',
      working_directory: ROOT,
      capability_budget: { max_turn_seconds: 30 },
    },
    engine: { engine_id: 'claude_code' },
    onActivity: () => {},
  });

  const reasonText = cleanText(projection && projection.reason);
  const outputText = cleanText(projection && (projection.output_text || projection.output_preview || projection.reason));
  const violations = [];
  if (!projection) violations.push('claude_malformed_json_projection_missing');
  if (!projection || projection.status !== 'failed') violations.push(`claude_malformed_json_status_unexpected:${projection && projection.status || 'missing'}`);
  if (!projection || projection.error_code !== 'claude_code_provider_quota_or_subscription_unavailable') {
    violations.push(`claude_malformed_json_code_unexpected:${projection && projection.error_code || 'missing'}`);
  }
  if (!reasonText.includes(claudeMessage)) violations.push(`claude_malformed_json_reason_missing_message:${reasonText}`);
  if (/unavailable:\s*\{\s*\.?$/i.test(reasonText) || reasonText.includes('unavailable: {.')) {
    violations.push(`claude_malformed_json_brace_reason_leaked:${reasonText}`);
  }
  if (!outputText.includes(claudeMessage)) violations.push(`claude_malformed_json_output_missing_message:${outputText}`);

  return {
    trace_id: traceId,
    status: projection && projection.status || null,
    error_code: projection && projection.error_code || null,
    reason_text: reasonText,
    output_text: outputText,
    violations,
  };
}

async function runClaudePluginTokenFailureProbe() {
  const traceId = `validation:agent-runtime-cli-adapter-claude-plugin-token:${Date.now()}`;
  const claudeMessage = 'Claude Code subscription is unavailable because account usage is exhausted.';
  const pluginToken = 'rust-analyzer-lsp@claude-plugins-official';
  const childScript = [
    `process.stderr.write(${JSON.stringify(JSON.stringify({ type: 'error', error: pluginToken, message: claudeMessage }, null, 2))} + '\\n');`,
    'process.exit(9);',
  ].join('\n');

  const adapter = createCliRuntimeEngineAdapter({
    engineId: 'claude_code',
    command: process.execPath,
    commandFallback: process.execPath,
    liveDispatch: true,
    timeoutMs: 10000,
    versionArgs: ['--version'],
    runArgs: () => ['-e', childScript],
    runStdin: (prompt) => prompt,
    cwd: ROOT,
  });

  const projection = await adapter.stream_turn({
    message: {
      trace_id: traceId,
      request_id: `${traceId}:request`,
      turn_id: `${traceId}:turn`,
      engine_id: 'claude_code',
      agent_id: 'agent-runtime-cli-adapter-claude-plugin-token-guard',
      session_id: 'cli-adapter-claude-plugin-token-session',
      input: { text: 'claude plugin token failure probe' },
      input_text: 'claude plugin token failure probe',
      working_directory: ROOT,
      capability_budget: { max_turn_seconds: 30 },
    },
    engine: { engine_id: 'claude_code' },
    onActivity: () => {},
  });

  const reasonText = cleanText(projection && projection.reason);
  const outputText = cleanText(projection && (projection.output_text || projection.output_preview || projection.reason));
  const violations = [];
  if (!projection) violations.push('claude_plugin_token_projection_missing');
  if (!projection || projection.status !== 'failed') violations.push(`claude_plugin_token_status_unexpected:${projection && projection.status || 'missing'}`);
  if (!projection || projection.error_code !== 'claude_code_provider_quota_or_subscription_unavailable') {
    violations.push(`claude_plugin_token_code_unexpected:${projection && projection.error_code || 'missing'}`);
  }
  if (!reasonText.includes(claudeMessage)) violations.push(`claude_plugin_token_reason_missing_message:${reasonText}`);
  if (reasonText.includes(pluginToken)) violations.push(`claude_plugin_token_reason_leaked_plugin:${reasonText}`);
  if (!outputText.includes(claudeMessage)) violations.push(`claude_plugin_token_output_missing_message:${outputText}`);
  if (outputText.includes(pluginToken)) violations.push(`claude_plugin_token_output_leaked_plugin:${outputText}`);

  return {
    trace_id: traceId,
    status: projection && projection.status || null,
    error_code: projection && projection.error_code || null,
    reason_text: reasonText,
    output_text: outputText,
    plugin_token: pluginToken,
    violations,
  };
}

async function runClaudePluginTokenFirstFailureProbe() {
  const traceId = `validation:agent-runtime-cli-adapter-claude-plugin-token-first:${Date.now()}`;
  const claudeMessage = 'Claude Code subscription is unavailable because account usage is exhausted.';
  const pluginToken = 'rust-analyzer-lsp@claude-plugins-official';
  const childScript = [
    `process.stderr.write(${JSON.stringify(JSON.stringify(pluginToken))} + '\\n');`,
    `process.stderr.write(${JSON.stringify(claudeMessage)} + '\\n');`,
    'process.exit(9);',
  ].join('\n');

  const adapter = createCliRuntimeEngineAdapter({
    engineId: 'claude_code',
    command: process.execPath,
    commandFallback: process.execPath,
    liveDispatch: true,
    timeoutMs: 10000,
    versionArgs: ['--version'],
    runArgs: () => ['-e', childScript],
    runStdin: (prompt) => prompt,
    cwd: ROOT,
  });

  const projection = await adapter.stream_turn({
    message: {
      trace_id: traceId,
      request_id: `${traceId}:request`,
      turn_id: `${traceId}:turn`,
      engine_id: 'claude_code',
      agent_id: 'agent-runtime-cli-adapter-claude-plugin-token-first-guard',
      session_id: 'cli-adapter-claude-plugin-token-first-session',
      input: { text: 'claude plugin token first failure probe' },
      input_text: 'claude plugin token first failure probe',
      working_directory: ROOT,
      capability_budget: { max_turn_seconds: 30 },
    },
    engine: { engine_id: 'claude_code' },
    onActivity: () => {},
  });

  const reasonText = cleanText(projection && projection.reason);
  const outputText = cleanText(projection && (projection.output_text || projection.output_preview || projection.reason));
  const violations = [];
  if (!projection) violations.push('claude_plugin_token_first_projection_missing');
  if (!projection || projection.status !== 'failed') violations.push(`claude_plugin_token_first_status_unexpected:${projection && projection.status || 'missing'}`);
  if (!projection || projection.error_code !== 'claude_code_provider_quota_or_subscription_unavailable') {
    violations.push(`claude_plugin_token_first_code_unexpected:${projection && projection.error_code || 'missing'}`);
  }
  if (!reasonText.includes(claudeMessage)) violations.push(`claude_plugin_token_first_reason_missing_message:${reasonText}`);
  if (reasonText.includes(pluginToken)) violations.push(`claude_plugin_token_first_reason_leaked_plugin:${reasonText}`);
  if (!outputText.includes(claudeMessage)) violations.push(`claude_plugin_token_first_output_missing_message:${outputText}`);
  if (outputText.includes(pluginToken)) violations.push(`claude_plugin_token_first_output_leaked_plugin:${outputText}`);

  return {
    trace_id: traceId,
    status: projection && projection.status || null,
    error_code: projection && projection.error_code || null,
    reason_text: reasonText,
    output_text: outputText,
    plugin_token: pluginToken,
    violations,
  };
}

function runPreTurnUsageLimitProjectionProbe() {
  const traceId = `validation:agent-runtime-pre-turn-usage-limit:${Date.now()}`;
  const usageLimitText = "You've hit your usage limit for GPT-5.3-Codex-Spark. Switch to another model now, or try again at 1:49 PM.";
  const projection = agentRuntimePreTurnFailureProjection(
    traceId,
    'codex_cli',
    'agent-runtime-pre-turn-usage-limit-guard',
    'pre-turn-usage-limit-session',
    `${traceId}:turn`,
    usageLimitText,
    {
      error: usageLimitText,
      provider_status: 'usage_limit',
      provider_event_type: 'provider.usage_limit',
    },
  );
  const displayText = cleanText(projection && (projection.display_text || projection.output_text || projection.text), 4000);
  const outputPreview = cleanText(projection && projection.output_preview, 4000);
  const violations = [];
  if (!projection) violations.push('pre_turn_usage_limit_projection_missing');
  if (!projection || projection.status !== 'failed_with_reason') violations.push(`pre_turn_usage_limit_status_unexpected:${projection && projection.status || 'missing'}`);
  if (!projection || projection.error_code !== 'codex_cli_provider_quota_or_subscription_unavailable') violations.push(`pre_turn_usage_limit_error_code_unexpected:${projection && projection.error_code || 'missing'}`);
  if (!displayText.includes('usage limit') || !displayText.includes('GPT-5.3-Codex-Spark') || !displayText.includes('1:49 PM')) {
    violations.push(`pre_turn_usage_limit_display_text_missing:${displayText}`);
  }
  if (!outputPreview.includes('usage limit') || !outputPreview.includes('GPT-5.3-Codex-Spark') || !outputPreview.includes('1:49 PM')) {
    violations.push(`pre_turn_usage_limit_output_preview_missing:${outputPreview}`);
  }
  return {
    trace_id: traceId,
    status: projection && projection.status || null,
    error_code: projection && projection.error_code || null,
    display_text: displayText,
    output_preview: outputPreview,
    violations,
  };
}

async function runDecisionDialogProbe() {
  const traceId = `validation:agent-runtime-cli-adapter-dialog:${Date.now()}`;
  const narrationText = 'I will inspect package.json before running the command.';
  const assistantRoleNarrationText = 'I am checking the available scripts before deciding what to run.';
  const finalText = 'One script category is ops commands.';
  const childScript = [
    'const rows = [',
    `  ${JSON.stringify({ type: 'thread.started', thread_id: 'thread-dialog-probe' })},`,
    `  ${JSON.stringify({ type: 'turn.started' })},`,
    `  ${JSON.stringify({ type: 'item.completed', item: { id: 'item_0', type: 'agent_message', text: narrationText } })},`,
    `  ${JSON.stringify({ type: 'message', role: 'assistant', id: 'msg_0', text: assistantRoleNarrationText })},`,
    `  ${JSON.stringify({ type: 'item.started', item: { id: 'item_1', type: 'command_execution', status: 'in_progress', command: 'node --version' } })},`,
    `  ${JSON.stringify({ type: 'item.completed', item: { id: 'item_1', type: 'command_execution', status: 'completed', command: 'node --version' } })},`,
    `  ${JSON.stringify({ type: 'item.completed', item: { id: 'item_2', type: 'agent_message', text: finalText } })},`,
    `  ${JSON.stringify({ type: 'turn.completed', usage: { input_tokens: 10, output_tokens: 5 } })},`,
    '];',
    'for (const row of rows) process.stdout.write(`${JSON.stringify(row)}\\n`);',
  ].join('\n');

  const streamedActivity = [];
  const adapter = createCliRuntimeEngineAdapter({
    engineId: 'codex_cli',
    command: process.execPath,
    commandFallback: process.execPath,
    liveDispatch: true,
    timeoutMs: 10000,
    versionArgs: ['--version'],
    runArgs: () => ['-e', childScript],
    runStdin: (prompt) => prompt,
    cwd: ROOT,
  });

  const projection = await adapter.stream_turn({
    message: {
      trace_id: traceId,
      request_id: `${traceId}:request`,
      turn_id: `${traceId}:turn`,
      engine_id: 'codex_cli',
      agent_id: 'agent-runtime-cli-adapter-dialog-guard',
      session_id: 'cli-adapter-dialog-session',
      input: { text: 'dialog probe' },
      input_text: 'dialog probe',
      working_directory: ROOT,
      capability_budget: { max_turn_seconds: 30 },
    },
    engine: { engine_id: 'codex_cli' },
    onActivity: (event) => {
      streamedActivity.push(event);
    },
  });

  const projectedActivity = projection && Array.isArray(projection.activity_events) ? projection.activity_events : [];
  const streamedDialog = streamedActivity.find((event) =>
    event && event.activity_kind === 'decision_dialog' && cleanText(event.display_text, 4000).includes(narrationText)
  );
  const projectedDialog = projectedActivity.find((event) =>
    event && event.activity_kind === 'decision_dialog' && cleanText(event.display_text, 4000).includes(narrationText)
  );
  const assistantRoleDialog = projectedActivity.find((event) =>
    event && event.activity_kind === 'decision_dialog' && cleanText(event.display_text, 4000).includes(assistantRoleNarrationText)
  );
  const finalLeakedIntoDialog = projectedActivity.some((event) =>
    event && event.activity_kind === 'decision_dialog' && cleanText(event.display_text, 4000).includes(finalText)
  );
  const commandProjected = projectedActivity.some((event) =>
    event && /command/.test(String(event.activity_kind || event.provider_event_type || '')) && cleanText(event.display_text, 4000).includes('node --version')
  );
  const outputText = cleanText(projection && (projection.output_text || projection.output_preview || projection.reason), 4000);
  const violations = [];
  if (!projection) violations.push('dialog_probe_projection_missing');
  if (!projection || projection.status !== 'completed') violations.push(`dialog_probe_status_unexpected:${projection && projection.status || 'missing'}`);
  if (!streamedDialog) violations.push('dialog_probe_streamed_decision_dialog_missing');
  if (!projectedDialog) violations.push('dialog_probe_projected_decision_dialog_missing');
  if (!assistantRoleDialog) violations.push('dialog_probe_assistant_role_decision_dialog_missing');
  if (finalLeakedIntoDialog) violations.push('dialog_probe_final_answer_leaked_into_decision_dialog');
  if (!commandProjected) violations.push('dialog_probe_command_activity_missing');
  if (!outputText.includes(finalText)) violations.push(`dialog_probe_final_output_missing:${outputText}`);

  return {
    trace_id: traceId,
    status: projection && projection.status || null,
    output_text: outputText,
    streamed_activity_count: streamedActivity.length,
    projected_activity_count: projectedActivity.length,
    streamed_dialog_text: streamedDialog ? cleanText(streamedDialog.display_text, 4000) : '',
    projected_dialog_text: projectedDialog ? cleanText(projectedDialog.display_text, 4000) : '',
    assistant_role_dialog_text: assistantRoleDialog ? cleanText(assistantRoleDialog.display_text, 4000) : '',
    command_projected: commandProjected,
    final_leaked_into_dialog: finalLeakedIntoDialog,
    violations,
  };
}

async function main() {
  const traceId = `validation:agent-runtime-cli-adapter-timeout:${Date.now()}`;
  const routeBody = {
    agent_id: 'agent-runtime-cli-adapter-timeout-guard',
    session_id: 'cli-adapter-timeout-session',
    conversation_id: 'cli-adapter-timeout-session',
    engine_id: 'codex_cli',
    message: 'cli adapter timeout guard fake runtime probe',
    input_text: 'cli adapter timeout guard fake runtime probe',
    capability_budget: {
      max_turn_seconds: 1,
    },
    test_probe: true,
  };
  const childScript = [
    'let tick = 0;',
    "process.stdout.write('{\"type\":\"turn.started\"}\\n');",
    "process.stderr.write('fake-cli-adapter-stderr-start\\n');",
    'setInterval(() => {',
    "  process.stdout.write('{\"type\":\"item.started\",\"item\":{\"type\":\"command\",\"command\":\"fake-' + tick + '\"}}\\n');",
    "  process.stderr.write('fake-cli-adapter-stderr-' + tick + '\\n');",
    '  tick += 1;',
    '}, 25);',
  ].join('\n');

  let projectionStarted = false;
  let adapterReturned = false;
  let adapterRun = null;
  const adapterActivity = [];
  const sent = [];
  const adapter = createCliRuntimeEngineAdapter({
    engineId: 'codex_cli',
    command: process.execPath,
    commandFallback: process.execPath,
    liveDispatch: true,
    timeoutMs: 10000,
    versionArgs: ['--version'],
    runArgs: () => ['-e', childScript],
    runStdin: (prompt) => prompt,
    cwd: ROOT,
    afterRun: (run) => {
      adapterRun = run;
    },
  });

  const handler = createAgentRuntimeTurnRouteHandler({
    turnProjectionStore: {
      agentRuntimeTurnProjection: async (_traceId, body, options = {}) => {
        projectionStarted = true;
        const adapterMessage = {
          ...body,
          trace_id: traceId,
          request_id: `${traceId}:request`,
          turn_id: `${traceId}:turn`,
          engine_id: 'codex_cli',
          input: {
            text: body.input_text || body.message,
          },
          input_text: body.input_text || body.message,
          working_directory: ROOT,
          capability_budget: {
            max_turn_seconds: 10,
          },
          abort_signal: options.abortSignal,
        };
        const result = await adapter.stream_turn({
          message: adapterMessage,
          engine: { engine_id: 'codex_cli' },
          onActivity: (event) => {
            adapterActivity.push(event);
            if (typeof options.onActivity === 'function') options.onActivity(event);
          },
        });
        adapterReturned = true;
        return result;
      },
    },
    contextPreviewProjectionStore: {
      agentRuntimeContextPackPreviewProjection: async () => ({ ok: true }),
    },
    steer: () => ({ ok: true }),
    createNativeOrchestrationClient: () => ({}),
    readJsonBody: async () => routeBody,
    sendJson: (res, statusCode, payload) => {
      sent.push({ statusCode, payload });
      if (!res.headersSent) {
        res.writeHead(statusCode, {
          'content-type': 'application/json; charset=utf-8',
          'cache-control': 'no-store',
        });
      }
      res.end(`${JSON.stringify(payload, null, 2)}\n`);
    },
  });

  const req = { method: 'POST' };
  const res = makeResponse();
  const handledPromise = handler.handleAgentRuntimeTurnRoute({
    req,
    res,
    pathname: '/api/shell-socket/agent-runtime/turn',
    traceId,
    flags: {},
  });
  handledPromise.catch(() => {});
  await res.waitForEnd(2500);
  await new Promise((resolve) => setTimeout(resolve, 1500));

  const payload = parsePayload(res) || sent[0] && sent[0].payload || null;
  const activityEvents = payload && Array.isArray(payload.activity_events)
    ? payload.activity_events
    : [];
  const launchActivityProjected = activityEvents.some((event) =>
    event &&
    event.provider_event_type === 'external_cli.launch' &&
    String(event.display_text || '').includes('Launching codex_cli CLI')
  );
  const timeoutActivityProjected = activityEvents.some((event) =>
    event &&
    event.provider_event_type === 'turn.timeout' &&
    event.status === 'timed_out'
  );
  const adapterSawCommandActivity = adapterActivity.some((event) =>
    event &&
    (event.provider_event_type === 'external_cli.launch' ||
      String(event.display_text || '').includes('fake-'))
  );
  const adapterStoppedBeforeOwnTimeout = !!(
    adapterReturned &&
    adapterRun &&
    adapterRun.timed_out !== true
  );
  const failureProbe = await runFailureProbe();
  const claudeMalformedJsonFailureProbe = await runClaudeMalformedJsonFailureProbe();
  const claudePluginTokenFailureProbe = await runClaudePluginTokenFailureProbe();
  const claudePluginTokenFirstFailureProbe = await runClaudePluginTokenFirstFailureProbe();
  const preTurnUsageLimitProbe = runPreTurnUsageLimitProjectionProbe();
  const decisionDialogProbe = await runDecisionDialogProbe();

  const violations = [];
  if (!projectionStarted) violations.push('projection_not_started');
  if (!res.writableEnded) violations.push('route_timeout_response_not_sent');
  if (res.statusCode !== 504) violations.push(`unexpected_status_code:${res.statusCode}`);
  if (!payload || payload.error_code !== 'agent_runtime_turn_route_timeout') {
    violations.push(`unexpected_error_code:${payload && payload.error_code || 'missing'}`);
  }
  if (!payload || payload.terminal_outcome !== 'timed_out_with_reason') {
    violations.push(`unexpected_terminal_outcome:${payload && payload.terminal_outcome || 'missing'}`);
  }
  if (!launchActivityProjected) violations.push('cli_adapter_launch_activity_missing_from_timeout_payload');
  if (!timeoutActivityProjected) violations.push('timeout_activity_missing_from_timeout_payload');
  if (!adapterSawCommandActivity) violations.push('cli_adapter_activity_not_observed');
  if (!adapterStoppedBeforeOwnTimeout) violations.push('cli_adapter_not_aborted_before_internal_timeout');
  if (sent.length !== 1) violations.push(`unexpected_send_count:${sent.length}`);
  if (failureProbe.violations.length) violations.push(...failureProbe.violations.map((violation) => `failure_probe_${violation}`));
  if (claudeMalformedJsonFailureProbe.violations.length) violations.push(...claudeMalformedJsonFailureProbe.violations.map((violation) => `claude_malformed_json_probe_${violation}`));
  if (claudePluginTokenFailureProbe.violations.length) violations.push(...claudePluginTokenFailureProbe.violations.map((violation) => `claude_plugin_token_probe_${violation}`));
  if (claudePluginTokenFirstFailureProbe.violations.length) violations.push(...claudePluginTokenFirstFailureProbe.violations.map((violation) => `claude_plugin_token_first_probe_${violation}`));
  if (preTurnUsageLimitProbe.violations.length) violations.push(...preTurnUsageLimitProbe.violations.map((violation) => `pre_turn_probe_${violation}`));
  if (decisionDialogProbe.violations.length) violations.push(...decisionDialogProbe.violations.map((violation) => `decision_dialog_probe_${violation}`));

  const report = {
    ok: violations.length === 0,
    guard: 'agent_runtime_cli_adapter_timeout_guard',
    type: 'agent_runtime_cli_adapter_timeout_guard',
    generated_at: new Date().toISOString(),
    source_domain: SOURCE_DOMAIN,
    owner_domain: OWNER_DOMAIN,
    layer: LAYER,
    policy_path: POLICY_PATH,
    trace_id: traceId,
    route: '/api/shell-socket/agent-runtime/turn',
    status_code: res.statusCode,
    error_code: payload && payload.error_code || null,
    terminal_outcome: payload && payload.terminal_outcome || null,
    projection_started: projectionStarted,
    route_response_sent: res.writableEnded,
    adapter_returned_after_abort: adapterReturned,
    adapter_run_timed_out: adapterRun ? adapterRun.timed_out === true : null,
    adapter_run_exit_code: adapterRun ? adapterRun.exit_code : null,
    adapter_activity_count: adapterActivity.length,
    cli_adapter_launch_activity_projected: launchActivityProjected,
    timeout_activity_projected: timeoutActivityProjected,
    send_count: sent.length,
    failure_probe: {
      status_code: failureProbe.projection && failureProbe.projection.status || null,
      error_code: failureProbe.projection && failureProbe.projection.error_code || null,
      reason_text: failureProbe.reason_text,
      output_text: failureProbe.output_text,
      output_preview: failureProbe.output_preview,
      adapter_run_exit_code: failureProbe.adapter_run_exit_code,
      adapter_run_timed_out: failureProbe.adapter_run_timed_out,
      trace_id: failureProbe.trace_id,
      violations: failureProbe.violations,
    },
    claude_malformed_json_failure_probe: claudeMalformedJsonFailureProbe,
    claude_plugin_token_failure_probe: claudePluginTokenFailureProbe,
    claude_plugin_token_first_failure_probe: claudePluginTokenFirstFailureProbe,
    pre_turn_usage_limit_probe: preTurnUsageLimitProbe,
    decision_dialog_probe: decisionDialogProbe,
    violations,
  };

  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`);
  console.log(JSON.stringify(report, null, 2));
  if (!report.ok) process.exit(1);
}

main().catch((error) => {
  ensureDir(OUT_JSON);
  const report = {
    ok: false,
    guard: 'agent_runtime_cli_adapter_timeout_guard',
    type: 'agent_runtime_cli_adapter_timeout_guard',
    generated_at: new Date().toISOString(),
    source_domain: SOURCE_DOMAIN,
    owner_domain: OWNER_DOMAIN,
    layer: LAYER,
    policy_path: POLICY_PATH,
    error: error && error.stack || String(error),
  };
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`);
  console.error(JSON.stringify(report, null, 2));
  process.exit(1);
});
