#!/usr/bin/env tsx
/* eslint-disable no-console */

// Operator command: bounded agent-runtime context pack preview.
// This is a read-only Gateway inspection helper for debugging what an external
// runtime engine receives before a turn. It must not expose raw full state.

type Args = {
  host: string;
  port: string;
  engine: string;
  agent: string;
  session: string;
  json: boolean;
};

function parseArgs(argv: string[]): Args {
  const out: Args = {
    host: process.env.INFRING_GATEWAY_HOST || '127.0.0.1',
    port: process.env.INFRING_GATEWAY_PORT || '4173',
    engine: process.env.INFRING_AGENT_RUNTIME_ENGINE || 'codex_cli',
    agent: process.env.INFRING_AGENT_ID || 'Research Golden Agent 5',
    session: process.env.INFRING_SESSION_ID || '',
    json: false,
  };
  for (const arg of argv) {
    if (arg === '--json') out.json = true;
    else if (arg.startsWith('--host=')) out.host = arg.slice('--host='.length);
    else if (arg.startsWith('--port=')) out.port = arg.slice('--port='.length);
    else if (arg.startsWith('--engine=')) out.engine = arg.slice('--engine='.length);
    else if (arg.startsWith('--agent=')) out.agent = arg.slice('--agent='.length);
    else if (arg.startsWith('--session=')) out.session = arg.slice('--session='.length);
    else if (arg === '--help' || arg === '-h') {
      console.log('Usage: npm run -s cmd -- runtime:agent-context-preview [--engine=codex_cli] [--agent=<id>] [--session=<id>] [--json]');
      process.exit(0);
    }
  }
  if (!out.session) out.session = out.agent;
  return out;
}

function text(value: unknown, max = 200): string {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

async function main(): Promise<void> {
  const args = parseArgs(process.argv.slice(2));
  const url = `http://${args.host}:${args.port}/api/shell-socket/agent-runtime/context-pack/preview`;
  const response = await fetch(url, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({
      engine_id: args.engine,
      agent_id: args.agent,
      session_id: args.session,
      permission_policy: {
        gatekeeper_kind: 'user',
        default_allow_read_tools: true,
        revoked_default_read_tools: [],
        always_allowed_tool_calls: [],
      },
    }),
  });
  const payload = await response.json().catch(() => null);
  if (!response.ok || !payload || payload.ok === false) {
    console.error(JSON.stringify({ ok: false, status: response.status, error: payload && (payload.error || payload.type) || 'context_preview_failed' }, null, 2));
    process.exit(1);
  }
  if (args.json) {
    console.log(JSON.stringify(payload, null, 2));
    return;
  }
  console.log(`Context pack preview: ${text(payload.engine_id)} / ${text(payload.session_id)}`);
  console.log(`source: ${text(payload.source_basis)} via ${text(payload.source_authority)}`);
  console.log(`rows: ${Number(payload.row_count) || 0} deduped / ${Number(payload.raw_row_count) || 0} raw; fragments: ${Array.isArray(payload.fragments) ? payload.fragments.length : 0}`);
  console.log(`kernel_materializer: ${payload.kernel_materializer_used ? 'yes' : 'no'} ${text(payload.kernel_materializer_mode, 80)}`.trim());
  if (payload.dedupe_policy) console.log(`dedupe: ${text(payload.dedupe_policy.type)} (${text(payload.dedupe_policy.key_basis)})`);
  console.log('fragments:');
  for (const fragment of Array.isArray(payload.fragments) ? payload.fragments : []) {
    const speaker = text(fragment.speaker_label || fragment.role || 'unknown', 80);
    const sourceKind = text(fragment.source_kind || fragment.kind || 'fragment', 80);
    const summary = text(fragment.summary, 220);
    console.log(`- ${speaker} [${sourceKind}] ${summary}`);
  }
}

main().catch((error) => {
  console.error(JSON.stringify({ ok: false, error: error && error.message ? error.message : String(error) }, null, 2));
  process.exit(1);
});
