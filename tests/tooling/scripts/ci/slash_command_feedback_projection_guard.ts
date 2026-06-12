#!/usr/bin/env tsx
/* eslint-disable no-console */

'use strict';

const fs = require('fs');
const path = require('path');

const ROOT = process.cwd();
const OUT_JSON = path.join(ROOT, 'core', 'local', 'artifacts', 'slash_command_feedback_projection_guard_current.json');
const REGISTRY_PATH = path.join(ROOT, 'validation', 'conformance', 'contracts', 'agent_runtime_engine_registry.json');
const PAGE_STATE_PATH = path.join(ROOT, 'client', 'runtime', 'systems', 'ui', 'infring_static', 'js', 'pages', 'chat.ts.parts', '010-chat-state.ts');
const RUNTIME_COMMAND_PATH = path.join(ROOT, 'client', 'runtime', 'systems', 'ui', 'infring_static', 'js', 'pages', 'chat.ts.parts', '110-failover-and-health.part01.ts');
const NATIVE_COMMAND_PATH = path.join(ROOT, 'client', 'runtime', 'systems', 'ui', 'infring_static', 'js', 'pages', 'chat.ts.parts', '110-failover-and-health.part02.ts');
const FEEDBACK_HELPER_PATH = path.join(ROOT, 'client', 'runtime', 'systems', 'ui', 'infring_static', 'js', 'pages', 'chat.ts.parts', '192-slash-alias-and-alerts.ts');
const SVELTE_SOURCE_PATH = path.join(ROOT, 'client', 'runtime', 'systems', 'ui', 'infring_static', 'js', 'svelte', 'chat_input_footer_shell_svelte_source.ts');
const SVELTE_BUNDLE_PATH = path.join(ROOT, 'client', 'runtime', 'systems', 'ui', 'infring_static', 'js', 'svelte', 'chat_input_footer_shell.bundle.ts');
const {
  createAgentRuntimeCommandCatalogStore,
} = require(path.join(ROOT, 'gateway', 'runtime', 'agent_runtime', 'agent_runtime_command_catalog.ts'));

function ensureDir(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function readText(filePath) {
  return fs.readFileSync(filePath, 'utf8');
}

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, 'utf8'));
}

function cleanText(value, maxLen = 4000) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function includesAll(source, patterns) {
  return patterns.every((pattern) => source.includes(pattern));
}

function flatten(groups) {
  const rows = [];
  for (const group of Array.isArray(groups) ? groups : []) {
    for (const row of Array.isArray(group && group.commands) ? group.commands : []) rows.push(row);
  }
  return rows;
}

async function main() {
  const registry = readJson(REGISTRY_PATH);
  const pageState = readText(PAGE_STATE_PATH);
  const runtimeCommandSource = readText(RUNTIME_COMMAND_PATH);
  const nativeCommandSource = readText(NATIVE_COMMAND_PATH);
  const feedbackHelperSource = readText(FEEDBACK_HELPER_PATH);
  const svelteSource = readText(SVELTE_SOURCE_PATH);
  const svelteBundle = readText(SVELTE_BUNDLE_PATH);
  const store = createAgentRuntimeCommandCatalogStore({
    loadRegistry: () => ({ engines: registry.engines || [] }),
    loadSelection: () => ({ engine_id: 'codex_cli' }),
  });

  const traceId = `validation:slash-command-feedback:${Date.now()}`;
  const codexProjection = store.agentRuntimeCommandCatalogProjection(traceId, { engine_id: 'codex_cli' });
  const claudeProjection = store.agentRuntimeCommandCatalogProjection(traceId, { engine_id: 'claude_code' });
  const codexRows = flatten(codexProjection.groups);
  const claudeRows = flatten(claudeProjection.groups);
  const codexStatus = codexRows.find((row) => row.intent_id === 'runtime.refresh_status' && row.display_command === '/status');
  const codexLogin = codexRows.find((row) => row.intent_id === 'runtime.authenticate' && row.display_command === '/login');
  const claudeLogin = claudeRows.find((row) => row.intent_id === 'runtime.authenticate' && row.display_command === '/login');
  const codexStatusAction = await store.agentRuntimeCommandActionProjection(traceId, {
    engine_id: 'codex_cli',
    intent_id: 'runtime.refresh_status',
  });
  const codexLoginAction = await store.agentRuntimeCommandActionProjection(traceId, {
    engine_id: 'codex_cli',
    intent_id: 'runtime.authenticate',
  });
  const claudeLoginAction = await store.agentRuntimeCommandActionProjection(traceId, {
    engine_id: 'claude_code',
    intent_id: 'runtime.authenticate',
  });

  const violations = [];
  if (!pageState.includes('slashCommandFeedback: null')) violations.push('page_state_missing_slash_command_feedback');
  if (!pageState.includes('_slashCommandFeedbackTimer: 0')) violations.push('page_state_missing_feedback_timer');
  if (!includesAll(runtimeCommandSource, [
    'executeAgentRuntimeSlashCommand',
    'publishSlashCommandFeedback(command',
    'Sending command intent to Gateway.',
    'terminal_outcome',
    'manual_action_required',
  ])) {
    violations.push('runtime_command_feedback_publication_missing');
  }
  if (!includesAll(nativeCommandSource, [
    'publishSlashCommandFeedback(selectedSlashRow',
    'Command accepted.',
  ])) {
    violations.push('native_command_feedback_publication_missing');
  }
  if (!includesAll(feedbackHelperSource, [
    'publishSlashCommandFeedback: function',
    'slash_command_feedback_projection',
    'source_authority',
    'gateway.agent_runtime_command_catalog',
    'dismissSlashCommandFeedback',
  ])) {
    violations.push('feedback_helper_missing_required_projection_fields');
  }
  if (!includesAll(svelteSource, [
    'slashCommandFeedback: null',
    'slash-command-feedback-row',
    'slashFeedbackTitle',
    'dismissSlashFeedback',
  ])) {
    violations.push('svelte_source_missing_feedback_render');
  }
  if (!includesAll(svelteBundle, [
    'slash-command-feedback-row',
    'slash-command-feedback-item',
  ])) {
    violations.push('svelte_bundle_missing_feedback_render');
  }
  if (!codexStatus) violations.push('codex_status_command_missing');
  if (!codexLogin) violations.push('codex_login_command_missing');
  if (!claudeLogin) violations.push('claude_login_command_missing');
  if (!codexStatusAction || codexStatusAction.status !== 'completed') {
    violations.push(`codex_status_action_unexpected:${cleanText(codexStatusAction && codexStatusAction.status, 160) || 'missing'}`);
  }
  if (!cleanText(codexStatusAction && codexStatusAction.display_text).includes('codex_cli status refreshed')) {
    violations.push('codex_status_action_display_text_missing');
  }
  if (!codexLoginAction || codexLoginAction.status !== 'manual_action_required') {
    violations.push(`codex_login_action_unexpected:${cleanText(codexLoginAction && codexLoginAction.status, 160) || 'missing'}`);
  }
  if (!cleanText(codexLoginAction && codexLoginAction.display_text).toLowerCase().includes('codex login')) {
    violations.push('codex_login_manual_action_text_missing');
  }
  if (!claudeLoginAction || claudeLoginAction.status !== 'manual_action_required') {
    violations.push(`claude_login_action_unexpected:${cleanText(claudeLoginAction && claudeLoginAction.status, 160) || 'missing'}`);
  }

  const report = {
    ok: violations.length === 0,
    guard: 'slash_command_feedback_projection_guard',
    type: 'slash_command_feedback_projection_guard',
    generated_at: new Date().toISOString(),
    source_domain: 'validation',
    owner_domain: 'validation.agent_runtime',
    layer: 'shell_projection_over_gateway_command_catalog',
    trace_id: traceId,
    checked_files: [
      path.relative(ROOT, PAGE_STATE_PATH),
      path.relative(ROOT, RUNTIME_COMMAND_PATH),
      path.relative(ROOT, NATIVE_COMMAND_PATH),
      path.relative(ROOT, FEEDBACK_HELPER_PATH),
      path.relative(ROOT, SVELTE_SOURCE_PATH),
      path.relative(ROOT, SVELTE_BUNDLE_PATH),
    ],
    codex_status_action: codexStatusAction,
    codex_login_action: codexLoginAction,
    claude_login_action: claudeLoginAction,
    visual_projection: {
      state_field: 'slashCommandFeedback',
      rendered_row_class: 'slash-command-feedback-row',
      prompt_queue_style_reused: true,
      shell_executes_raw_runtime_command: false,
    },
    violations,
  };

  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`);
  console.log(JSON.stringify(report, null, 2));
  if (!report.ok) process.exit(1);
}

main().catch((error) => {
  const report = {
    ok: false,
    guard: 'slash_command_feedback_projection_guard',
    type: 'slash_command_feedback_projection_guard',
    generated_at: new Date().toISOString(),
    source_domain: 'validation',
    owner_domain: 'validation.agent_runtime',
    layer: 'shell_projection_over_gateway_command_catalog',
    error: cleanText(error && error.stack ? error.stack : error, 6000),
    violations: ['slash_command_feedback_projection_guard_crashed'],
  };
  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`);
  console.error(JSON.stringify(report, null, 2));
  process.exit(1);
});
