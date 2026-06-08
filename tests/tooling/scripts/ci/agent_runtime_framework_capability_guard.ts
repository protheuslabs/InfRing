#!/usr/bin/env node
/* eslint-disable no-console */

const fs = require('node:fs');
const path = require('node:path');

const ROOT = process.cwd();
const OUT_JSON = path.join(ROOT, 'core/local/artifacts/agent_runtime_framework_capability_guard_current.json');
const {
  createCliRuntimeEngineAdapter,
  parseCliActivityOutput,
} = require(path.join(ROOT, 'adapters/runtime/agent_engines/cli_runtime_adapter.ts'));

function read(rel) {
  return fs.readFileSync(path.join(ROOT, rel), 'utf8');
}

function ensureDir(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function push(violations, kind, path, detail = '') {
  violations.push({ kind, path, detail });
}

async function main() {
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

  const opencodeTextRow = JSON.stringify({
    type: 'text',
    timestamp: 1780948702828,
    sessionID: 'ses_probe',
    part: {
      id: 'prt_probe',
      messageID: 'msg_probe',
      sessionID: 'ses_probe',
      type: 'text',
      text: 'shadow-attachment-wins',
    },
  });
  const parsedOpenCodeOutput = parseCliActivityOutput(
    `${JSON.stringify({ type: 'step_start', sessionID: 'ses_probe', part: { type: 'step-start' } })}\n${opencodeTextRow}\n${JSON.stringify({ type: 'step_finish', sessionID: 'ses_probe', part: { type: 'step-finish' } })}\n`,
    '',
    {
      engine: { engine_id: 'opencode' },
      message: {
        trace_id: 'validation:agent-runtime-framework-capability',
        session_id: 'framework-capability-session',
        turn_id: 'framework-capability-turn',
        engine_id: 'opencode',
      },
    },
    'opencode',
  );
  if (!String(parsedOpenCodeOutput && parsedOpenCodeOutput.output_text || '').includes('shadow-attachment-wins')) {
    push(violations, 'opencode_text_part_output_projection_missing', sharedPath, 'OpenCode-style JSONL text rows with part.text must become final assistant output.');
  }

  const permissionProbeCtx = {
    engine: { engine_id: 'codex_cli' },
    message: {
      trace_id: 'validation:agent-runtime-framework-capability-permission',
      session_id: 'framework-capability-permission-session',
      turn_id: 'framework-capability-permission-turn',
      engine_id: 'codex_cli',
      working_directory: ROOT,
    },
  };
  const proposalText = [
    'Permission required: create a tiny app file.',
    JSON.stringify({
      type: 'infring_universal_tool_proposal',
      tool_id: 'artifact.create_propose',
      reason: 'Create a tiny app requested by the user.',
      arguments: {
        path: 'tmp/agent-runtime-permission-probe.txt',
        mime_type: 'text/plain',
        content: 'permission-pause-wins',
      },
    }),
  ].join('\n');
  const parsedProposalPermission = parseCliActivityOutput(
    `${JSON.stringify({ type: 'text', part: { type: 'text', text: proposalText } })}\n`,
    '',
    permissionProbeCtx,
    'codex_cli',
  );
  const proposalRequest = parsedProposalPermission && parsedProposalPermission.permission_request;
  if (
    !proposalRequest ||
    proposalRequest.tool_id !== 'artifact.create_propose' ||
    proposalRequest.status !== 'paused_pending_approval' ||
    proposalRequest.resume_strategy !== 'gateway_apply_approved_effect' ||
    !proposalRequest.proposal_arguments ||
    proposalRequest.proposal_arguments.content !== 'permission-pause-wins'
  ) {
    push(violations, 'universal_tool_proposal_permission_pause_missing', sharedPath, 'Text-emitted universal tool proposals must become Gateway permission pauses with sanitized proposal arguments.');
  }

  const parsedDenialPermission = parseCliActivityOutput(
    `${JSON.stringify({
      type: 'text',
      part: {
        type: 'text',
        text: 'Blocked action: create tmp/agent-runtime-permission-denial.txt. Permission reason: runtime filesystem is read-only and approval is required.',
      },
    })}\n`,
    '',
    permissionProbeCtx,
    'codex_cli',
  );
  const denialRequest = parsedDenialPermission && parsedDenialPermission.permission_request;
  if (
    !denialRequest ||
    denialRequest.tool_id !== 'artifact.create_propose' ||
    denialRequest.status !== 'paused_pending_approval' ||
    denialRequest.resume_strategy !== 'grant_then_retry_next_turn'
  ) {
    push(violations, 'permission_denial_text_pause_missing', sharedPath, 'Text-emitted permission denials must become Gateway permission pauses instead of ending the agent turn as plain text.');
  }

  const adapterProposalText = [
    'Permission required: create a tiny app file.',
    JSON.stringify({
      type: 'infring_universal_tool_proposal',
      tool_id: 'artifact.create_propose',
      reason: 'Create a tiny app requested by the user through a CLI adapter turn.',
      arguments: {
        path: 'tmp/agent-runtime-adapter-permission-probe.txt',
        mime_type: 'text/plain',
        content: 'adapter-permission-pause-wins',
      },
    }),
  ].join('\n');
  const adapterProbePayload = JSON.stringify({
    type: 'text',
    part: {
      type: 'text',
      text: adapterProposalText,
    },
  });
  const adapterProbe = createCliRuntimeEngineAdapter({
    engineId: 'adapter_permission_probe',
    command: process.execPath,
    commandFallback: process.execPath,
    liveDispatch: true,
    versionArgs: ['--version'],
    runArgs: () => ['-e', `process.stdout.write(${JSON.stringify(`${adapterProbePayload}\n`)})`],
    promptBuilder: ({ current }) => current,
  });
  const adapterTurn = await adapterProbe.submit_turn({
    engine: { engine_id: 'adapter_permission_probe' },
    message: {
      trace_id: 'validation:agent-runtime-framework-capability-adapter-permission',
      session_id: 'framework-capability-adapter-permission-session',
      turn_id: 'framework-capability-adapter-permission-turn',
      engine_id: 'adapter_permission_probe',
      working_directory: ROOT,
      input: { text: 'Probe adapter-level permission pause handling.' },
      context_pack: {
        source_authority: 'validation.agent_runtime_framework_capability_guard',
        universal_tool_grants: {
          tools: [],
          source_authority: 'validation_agent_runtime_framework_capability_guard',
        },
      },
      capability_budget: {
        max_turn_seconds: 30,
      },
    },
  });
  if (
    !adapterTurn ||
    adapterTurn.status !== 'permission_required' ||
    !adapterTurn.permission_request ||
    adapterTurn.permission_request.tool_id !== 'artifact.create_propose' ||
    adapterTurn.permission_request.status !== 'paused_pending_approval' ||
    adapterTurn.permission_request.turn_status !== 'permission_required' ||
    adapterTurn.permission_request.resume_strategy !== 'gateway_apply_approved_effect' ||
    !adapterTurn.permission_request.proposal_arguments ||
    adapterTurn.permission_request.proposal_arguments.content !== 'adapter-permission-pause-wins'
  ) {
    push(violations, 'adapter_text_proposal_turn_pause_missing', sharedPath, 'A full CLI adapter turn that emits a universal tool proposal as text must return status=permission_required with a pending Gateway approval request.');
  }

  const adapterResumeProbe = createCliRuntimeEngineAdapter({
    engineId: 'adapter_resume_probe',
    command: process.execPath,
    commandFallback: process.execPath,
    liveDispatch: true,
    versionArgs: ['--version'],
    runArgs: (prompt) => {
      const script = `
        const prompt = process.argv.slice(1).join('\\n');
        const required = [
          'Approval resume:',
          'approved_tool_id: artifact.create_propose',
          'decision: allow_once',
          'approval_id: approval-framework-capability-resume',
          'resume_token: resume-token-framework-capability',
          'decision_receipt_ref: receipt/agent-runtime-approval-decision/framework-capability'
        ];
        const missing = required.filter((needle) => !prompt.includes(needle));
        if (missing.length) {
          process.stderr.write('missing approval resume context: ' + missing.join(', '));
          process.exit(2);
        }
        process.stdout.write(JSON.stringify({ type: 'text', part: { type: 'text', text: 'approval-resume-context-wins' } }) + '\\n');
      `;
      return ['-e', script, prompt];
    },
  });
  const adapterResumeTurn = await adapterResumeProbe.submit_turn({
    engine: { engine_id: 'adapter_resume_probe' },
    message: {
      trace_id: 'validation:agent-runtime-framework-capability-approval-resume',
      session_id: 'framework-capability-approval-resume-session',
      turn_id: 'framework-capability-approval-resume-turn',
      engine_id: 'adapter_resume_probe',
      working_directory: ROOT,
      input: { text: 'Continue after the approved artifact creation.' },
      context_pack: {
        source_authority: 'validation.agent_runtime_framework_capability_guard',
        approval_resume: {
          approval_id: 'approval-framework-capability-resume',
          resume_token: 'resume-token-framework-capability',
          approved_tool_id: 'artifact.create_propose',
          approval_decision: 'allow_once',
          decision_receipt_ref: 'receipt/agent-runtime-approval-decision/framework-capability',
        },
      },
      capability_budget: {
        max_turn_seconds: 30,
      },
    },
  });
  if (
    !adapterResumeTurn ||
    adapterResumeTurn.status !== 'completed' ||
    !String(adapterResumeTurn.output_text || '').includes('approval-resume-context-wins')
  ) {
    push(violations, 'adapter_approval_resume_context_missing', sharedPath, 'A resumed CLI adapter turn must receive approval_resume context with approval id, resume token, decision, approved tool, and receipt ref.');
  }

  const adapterResumeFollowupProbe = createCliRuntimeEngineAdapter({
    engineId: 'adapter_resume_followup_probe',
    command: process.execPath,
    commandFallback: process.execPath,
    liveDispatch: true,
    versionArgs: ['--version'],
    runArgs: (prompt) => {
      const script = `
        const prompt = process.argv.slice(1).join('\\n');
        const hasTerseFollowup = /Current user turn:\\s*\\n\\s*continue\\b/i.test(prompt);
        const required = [
          'Approval resume:',
          'approved_tool_id: artifact.create_propose',
          'decision: allow_once',
          'approval_id: approval-framework-capability-followup',
          'resume_token: resume-token-framework-capability-followup',
          'decision_receipt_ref: receipt/agent-runtime-approval-decision/framework-capability-followup'
        ];
        const missing = required.filter((needle) => !prompt.includes(needle));
        if (!hasTerseFollowup || missing.length) {
          process.stderr.write('missing resumable follow-up context: ' + [
            hasTerseFollowup ? '' : 'terse continue turn',
            ...missing
          ].filter(Boolean).join(', '));
          process.exit(2);
        }
        process.stdout.write(JSON.stringify({ type: 'text', part: { type: 'text', text: 'approval-resume-followup-wins' } }) + '\\n');
      `;
      return ['-e', script, prompt];
    },
  });
  const adapterResumeFollowupTurn = await adapterResumeFollowupProbe.submit_turn({
    engine: { engine_id: 'adapter_resume_followup_probe' },
    message: {
      trace_id: 'validation:agent-runtime-framework-capability-approval-resume-followup',
      session_id: 'framework-capability-approval-resume-followup-session',
      turn_id: 'framework-capability-approval-resume-followup-turn',
      engine_id: 'adapter_resume_followup_probe',
      working_directory: ROOT,
      input: { text: 'continue' },
      context_pack: {
        source_authority: 'validation.agent_runtime_framework_capability_guard',
        approval_resume: {
          approval_id: 'approval-framework-capability-followup',
          resume_token: 'resume-token-framework-capability-followup',
          approved_tool_id: 'artifact.create_propose',
          approval_decision: 'allow_once',
          decision_receipt_ref: 'receipt/agent-runtime-approval-decision/framework-capability-followup',
        },
      },
      capability_budget: {
        max_turn_seconds: 30,
      },
    },
  });
  if (
    !adapterResumeFollowupTurn ||
    adapterResumeFollowupTurn.status !== 'completed' ||
    !String(adapterResumeFollowupTurn.output_text || '').includes('approval-resume-followup-wins')
  ) {
    push(violations, 'adapter_approval_resume_followup_missing', sharedPath, 'A resumed CLI adapter turn must support terse follow-up input such as "continue" by carrying approval_resume context forward.');
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
      opencode_text_part_output_projection: !violations.some((row) => row.kind === 'opencode_text_part_output_projection_missing'),
      universal_tool_proposal_permission_pause: !violations.some((row) => row.kind === 'universal_tool_proposal_permission_pause_missing'),
      permission_denial_text_pause: !violations.some((row) => row.kind === 'permission_denial_text_pause_missing'),
      adapter_text_proposal_turn_pause: !violations.some((row) => row.kind === 'adapter_text_proposal_turn_pause_missing'),
      adapter_approval_resume_context: !violations.some((row) => row.kind === 'adapter_approval_resume_context_missing'),
      adapter_approval_resume_followup: !violations.some((row) => row.kind === 'adapter_approval_resume_followup_missing'),
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
    type: 'agent_runtime_framework_capability_guard',
    generated_at: new Date().toISOString(),
    error: String(error && error.message ? error.message : error),
    violations: [
      {
        kind: 'agent_runtime_framework_capability_guard_crashed',
        path: 'tests/tooling/scripts/ci/agent_runtime_framework_capability_guard.ts',
        detail: String(error && error.stack ? error.stack : error),
      },
    ],
  };
  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`);
  console.log(JSON.stringify(report, null, 2));
  process.exit(1);
});
