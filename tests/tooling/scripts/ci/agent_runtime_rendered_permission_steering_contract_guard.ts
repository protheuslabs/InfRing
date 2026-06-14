import fs from 'node:fs';
import path from 'node:path';

type Violation = {
  id: string;
  path: string;
  detail: string;
};

type Check = {
  id: string;
  ok: boolean;
  path: string;
  detail: string;
};

const ROOT = process.cwd();
const OUT_JSON = 'core/local/artifacts/agent_runtime_rendered_permission_steering_contract_guard_current.json';
const OUT_MARKDOWN = 'local/workspace/reports/AGENT_RUNTIME_RENDERED_PERMISSION_STEERING_CONTRACT_GUARD_CURRENT.md';

const CHAT_SEND = 'client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/200-send-pipeline.part01.ts';
const CHAT_STEER = 'client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/060-suggestions-and-hints.ts';
const CHAT_QUEUE = 'client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/190-drawer-and-queue.part03.ts';
const GATEWAY_APPROVALS = 'gateway/runtime/agent_runtime/agent_runtime_approvals.ts';

function read(rel: string): string {
  return fs.readFileSync(path.join(ROOT, rel), 'utf8');
}

function includesAll(source: string, markers: string[]): boolean {
  return markers.every((marker) => source.includes(marker));
}

function blockBetween(source: string, startMarker: string, endMarker: string): string {
  const start = source.indexOf(startMarker);
  if (start < 0) return '';
  const end = source.indexOf(endMarker, start + startMarker.length);
  return end > start ? source.slice(start, end) : source.slice(start);
}

function pushCheck(checks: Check[], id: string, ok: boolean, rel: string, detail: string): void {
  checks.push({ id, ok, path: rel, detail });
}

function writeJson(rel: string, value: unknown): void {
  const target = path.join(ROOT, rel);
  fs.mkdirSync(path.dirname(target), { recursive: true });
  fs.writeFileSync(target, `${JSON.stringify(value, null, 2)}\n`);
}

function writeMarkdown(rel: string, report: any): void {
  const target = path.join(ROOT, rel);
  fs.mkdirSync(path.dirname(target), { recursive: true });
  const lines = [
    '# Agent Runtime rendered permission/steering contract guard',
    '',
    `- ok: ${report.ok}`,
    `- generated_at: ${report.generated_at}`,
    `- checks: ${report.summary.checks}`,
    `- violations: ${report.summary.violations}`,
    '',
    '## Violations',
    '',
  ];
  if (report.violations.length === 0) {
    lines.push('- none');
  } else {
    for (const violation of report.violations) {
      lines.push(`- ${violation.id}: ${violation.path} - ${violation.detail}`);
    }
  }
  lines.push('', '## Checks', '');
  for (const check of report.checks) {
    lines.push(`- ${check.ok ? 'pass' : 'fail'} ${check.id}: ${check.path}`);
  }
  fs.writeFileSync(target, `${lines.join('\n')}\n`);
}

function main(): void {
  const send = read(CHAT_SEND);
  const steer = read(CHAT_STEER);
  const queue = read(CHAT_QUEUE);
  const approvals = read(GATEWAY_APPROVALS);

  const checks: Check[] = [];

  pushCheck(
    checks,
    'active_runtime_composer_uses_steer_route',
    includesAll(send, ['runtime_steer_direct', 'steerPromptQueueItem(steerQueueId)']),
    CHAT_SEND,
    'When an external runtime turn is active, typed user text must be submitted as a Gateway steering request, not as a new plain chat turn.',
  );

  pushCheck(
    checks,
    'direct_steer_suppresses_extra_shell_notice',
    steer.includes('silent_steer_notice') && steer.includes('Steer injected into active workflow'),
    CHAT_STEER,
    'Direct steering should not add a duplicate Shell-authored chat notification.',
  );

  pushCheck(
    checks,
    'steering_route_enters_gateway_socket_api',
    steer.includes("/api/shell-socket/agent-runtime/steer"),
    CHAT_STEER,
    'Shell may only submit steering through the Gateway shell-socket route.',
  );

  pushCheck(
    checks,
    'queued_steering_preserves_user_text',
    steer.includes("queue_kind: 'agent_runtime_steer_followup'") &&
      steer.includes('text: text') &&
      !steer.includes('Continue with the queued user steering instruction'),
    CHAT_STEER,
    'Next-turn steering fallback must preserve the actual user steering text instead of replacing it with a generic prompt.',
  );

  pushCheck(
    checks,
    'late_steer_ack_drains_queue_when_idle',
    includesAll(steer, ['selfSteerQueueDrain', '_processQueue']),
    CHAT_STEER,
    'If the runtime ACK arrives after the turn has ended, Shell should drain the queued follow-up instead of stranding it.',
  );

  pushCheck(
    checks,
    'runtime_completion_drains_steering_followup',
    includesAll(send, ['drainQueueAfterRuntimeTurn', '_processQueue']),
    CHAT_SEND,
    'Runtime turn completion should trigger queued steering follow-up processing.',
  );

  const followupBlock = blockBetween(queue, "queueKind === 'agent_runtime_steer_followup'", 'if (!nextText.trim()');
  pushCheck(
    checks,
    'steering_followup_does_not_duplicate_user_bubble',
    followupBlock.length > 0 && !followupBlock.includes('appendUserChatMessage'),
    CHAT_QUEUE,
    'Queued steering follow-up already has a visible user steer row; replay must not add a duplicate user bubble.',
  );

  pushCheck(
    checks,
    'approval_pause_fetches_gateway_pending_projection',
    send.includes("/api/shell-socket/approvals/pending"),
    CHAT_SEND,
    'Shell approval UI must fetch bounded pending-approval projections from Gateway.',
  );

  pushCheck(
    checks,
    'approval_decision_posts_to_gateway_only',
    send.includes("/api/shell-socket/approvals/") && send.includes('/decision'),
    CHAT_SEND,
    'Shell approval UI must submit decisions to Gateway, not execute effects locally.',
  );

  pushCheck(
    checks,
    'approval_decision_preserves_gateway_arguments',
    includesAll(send, ['proposal_arguments', 'tool_call_ref', 'gatekeeper_kind']),
    CHAT_SEND,
    'Shell decision payloads must preserve Gateway-projected proposal refs/arguments and user gatekeeper identity.',
  );

  pushCheck(
    checks,
    'permission_pause_keeps_turn_open',
    includesAll(send, ['approval_pause_active = true', 'paused_pending_approval', 'thinkingMessage.streaming = true']),
    CHAT_SEND,
    'Permission-required turns must keep the active thinking row open while waiting for a decision.',
  );

  pushCheck(
    checks,
    'approval_resume_projects_receipts',
    includesAll(send, ['decision_receipt_ref', 'approved_effect_executed', 'approved_effect_display_text']),
    CHAT_SEND,
    'After approval, Shell must render Gateway receipt/effect projections rather than inventing a local outcome.',
  );

  pushCheck(
    checks,
    'gateway_dedupes_pending_approval_queue',
    includesAll(approvals, [
      'coalesced_with_existing_pending_approval',
      'duplicate_permission_request',
      'single_pending_permission_per_thread',
      'pendingProposalKey',
    ]),
    GATEWAY_APPROVALS,
    'Gateway must prevent duplicate pending approval prompts for the same proposed action.',
  );

  const violations: Violation[] = checks
    .filter((check) => !check.ok)
    .map((check) => ({
      id: check.id,
      path: check.path,
      detail: check.detail,
    }));

  const report = {
    type: 'agent_runtime_rendered_permission_steering_contract_guard',
    generated_at: new Date().toISOString(),
    trace_id: `validation:agent-runtime-rendered-permission-steering:${Date.now()}`,
    ok: violations.length === 0,
    authority_model: {
      shell: 'render_projection_and_submit_user_decision_only',
      gateway: 'approval_pause_resume_and_steering_authority',
      adapter: 'runtime_schema_translation_only',
    },
    summary: {
      checks: checks.length,
      passing: checks.filter((check) => check.ok).length,
      violations: violations.length,
    },
    checks,
    violations,
    evidence: {
      chat_send: CHAT_SEND,
      chat_steer: CHAT_STEER,
      chat_queue: CHAT_QUEUE,
      gateway_approvals: GATEWAY_APPROVALS,
    },
  };

  writeJson(OUT_JSON, report);
  writeMarkdown(OUT_MARKDOWN, report);

  if (!report.ok) {
    console.error(JSON.stringify(report, null, 2));
    process.exit(1);
  }
  console.log(JSON.stringify(report, null, 2));
}

main();
