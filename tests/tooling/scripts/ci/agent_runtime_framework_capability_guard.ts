#!/usr/bin/env node
/* eslint-disable no-console */

const fs = require('node:fs');
const path = require('node:path');

const ROOT = process.cwd();
const OUT_JSON = path.join(ROOT, 'core/local/artifacts/agent_runtime_framework_capability_guard_current.json');

function read(rel) {
  return fs.readFileSync(path.join(ROOT, rel), 'utf8');
}

function ensureDir(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function push(violations, kind, path, detail = '') {
  violations.push({ kind, path, detail });
}

function main() {
  const codexPath = 'adapters/runtime/agent_engines/codex_cli.ts';
  const claudePath = 'adapters/runtime/agent_engines/claude_code.ts';
  const grokPath = 'adapters/runtime/agent_engines/grok_code.ts';
  const opencodePath = 'adapters/runtime/agent_engines/opencode.ts';
  const sharedPath = 'adapters/runtime/agent_engines/cli_runtime_adapter.ts';
  const codex = read(codexPath);
  const claude = read(claudePath);
  const grok = read(grokPath);
  const opencode = read(opencodePath);
  const shared = read(sharedPath);
  const violations = [];

  if (!codex.includes("'--skip-git-repo-check'") && !codex.includes('"--skip-git-repo-check"')) {
    push(violations, 'codex_skip_git_repo_check_missing', codexPath, 'Codex must support arbitrary Gateway-selected workspaces, including non-git scratch directories.');
  }
  if (!codex.includes("'--sandbox'") || !codex.includes('codexSandboxMode(ctx)')) {
    push(violations, 'codex_sandbox_policy_missing', codexPath, 'Codex must keep Gateway permission policy mapped to Codex sandbox mode.');
  }

  if (!claude.includes('mutationGrant') || !claude.includes("'--allowedTools'") || !claude.includes('Read,Write,Edit,Bash')) {
    push(violations, 'claude_native_tool_allowlist_mapping_missing', claudePath, 'Claude Code needs native tool allowlist mapping only after Gateway mutation grant is active.');
  }
  if (!claude.includes("'--permission-mode'") || !claude.includes("'acceptEdits'")) {
    push(violations, 'claude_permission_mode_mapping_missing', claudePath, 'Claude Code must use acceptEdits only for Gateway-approved mutating turns.');
  }

  if (!grok.includes('mutationGrant') || !grok.includes("'--always-approve'")) {
    push(violations, 'grok_native_approval_mapping_missing', grokPath, 'Grok Code needs native approval mapping only after Gateway mutation grant is active.');
  }
  if (!grok.includes("'--permission-mode'") || !grok.includes("'acceptEdits'")) {
    push(violations, 'grok_permission_mode_mapping_missing', grokPath, 'Grok Code must use acceptEdits only for Gateway-approved mutating turns.');
  }

  if (!opencode.includes('mutationGrant') || !opencode.includes("'--dangerously-skip-permissions'")) {
    push(violations, 'opencode_permission_mode_mapping_missing', opencodePath, 'OpenCode must use --dangerously-skip-permissions only for explicit native direct-mutation grants.');
  }

  for (const [adapterName, adapterPath, adapterSource] of [
    ['codex_cli', codexPath, codex],
    ['claude_code', claudePath, claude],
    ['grok_code', grokPath, grok],
    ['opencode', opencodePath, opencode],
  ]) {
    if (adapterSource.includes("always.includes('artifact.create_propose')") || adapterSource.includes("always.includes('permission.request')")) {
      push(violations, 'proposal_tool_mapped_to_native_mutation', adapterPath, `${adapterName} must not map proposal or permission-request grants to native direct-edit modes.`);
    }
  }

  if (!shared.includes('dedupeFailureLines(')) {
    push(violations, 'cli_failure_dedupe_missing', sharedPath, 'External CLI provider failures must not flood chat with repeated raw stderr/stdout blocks.');
  }
  if (!shared.includes('external runtime provider is unavailable')) {
    push(violations, 'cli_provider_failure_projection_missing', sharedPath, 'Provider quota/auth/billing failures need compact user-facing projection text.');
  }
  if (!shared.includes('const outputText = run.ok ? (parsed.output_text || failureText) : failureText;')) {
    push(violations, 'cli_failed_turn_uses_raw_output', sharedPath, 'Failed external CLI turns should project the classified failure text, not raw duplicated provider output.');
  }

  const report = {
    ok: violations.length === 0,
    type: 'agent_runtime_framework_capability_guard',
    generated_at: new Date().toISOString(),
    checks: {
      codex_non_git_workspace: true,
      claude_gateway_mutation_grant_mapping: true,
      grok_gateway_mutation_grant_mapping: true,
      compact_provider_failure_projection: true,
    },
    violations,
  };
  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`);
  console.log(JSON.stringify(report, null, 2));
  if (!report.ok) process.exit(1);
}

main();
