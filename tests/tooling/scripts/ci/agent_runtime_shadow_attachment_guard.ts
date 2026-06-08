#!/usr/bin/env node
'use strict';

const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const ROOT = process.cwd();
const ARTIFACT_PATH = path.join(
  ROOT,
  'core/local/artifacts/agent_runtime_shadow_attachment_guard_current.json',
);
const { createCliRuntimeEngineAdapter } = require(path.join(
  ROOT,
  'adapters/runtime/agent_engines/cli_runtime_adapter.ts',
));

function ensureDir(fullPath) {
  fs.mkdirSync(fullPath, { recursive: true });
}

function writeText(fullPath, text) {
  ensureDir(path.dirname(fullPath));
  fs.writeFileSync(fullPath, text, 'utf8');
}

function clean(value, max = 1000) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function runtimeAttachmentPathFromContext(ctx) {
  const pack = ctx && ctx.message && ctx.message.context_pack;
  const refs = pack && pack.runtime_attachment_refs;
  const rows = refs && Array.isArray(refs.attachments) ? refs.attachments : [];
  const row = rows.find((item) => item && (item.local_read_path || item.read_path)) || {};
  return clean(row.local_read_path || row.read_path, 1200);
}

function buildProbeCommandScript(scriptPath) {
  writeText(scriptPath, [
    '#!/usr/bin/env node',
    "'use strict';",
    "const fs = require('node:fs');",
    "const path = require('node:path');",
    "const filePath = process.argv[2] || '';",
    "const shadow = process.env.INFRING_SHADOW_WORKING_DIRECTORY || '';",
    "const real = process.env.INFRING_REAL_WORKING_DIRECTORY || '';",
    "function inside(candidate, root) {",
    "  if (!candidate || !root) return false;",
    "  const c = path.resolve(candidate);",
    "  const r = path.resolve(root);",
    "  return c === r || c.startsWith(`${r}${path.sep}`);",
    "}",
    "const exists = filePath ? fs.existsSync(filePath) : false;",
    "const text = exists ? fs.readFileSync(filePath, 'utf8') : '';",
    "const inShadow = inside(filePath, shadow);",
    "const inReal = inside(filePath, real);",
    "const ok = exists && inShadow && !inReal && text.includes('SHADOW_ATTACHMENT_SECRET=shadow-attachment-wins');",
    "const payload = ok",
    "  ? 'shadow-attachment-wins'",
    "  : `shadow attachment failed: exists=${exists} in_shadow=${inShadow} in_real=${inReal} file=${filePath}`;",
    "process.stdout.write(`${JSON.stringify({ type: 'text', part: { type: 'text', text: payload } })}\\n`);",
    "process.exit(ok ? 0 : 2);",
  ].join('\n'));
  fs.chmodSync(scriptPath, 0o755);
}

async function main() {
  const scratch = fs.mkdtempSync(path.join(os.tmpdir(), 'infring-shadow-attachment-guard-'));
  const workDir = path.join(scratch, 'workspace');
  const attachmentPath = path.join(workDir, 'input', 'pastedtext.txt');
  const scriptPath = path.join(scratch, 'probe-cli.js');
  ensureDir(workDir);
  writeText(attachmentPath, [
    'Synthetic pasted text attachment.',
    'SHADOW_ATTACHMENT_SECRET=shadow-attachment-wins',
    'The external CLI must receive a shadow-local readable copy, not this original path.',
  ].join('\n'));
  buildProbeCommandScript(scriptPath);

  const adapter = createCliRuntimeEngineAdapter({
    engineId: 'shadow_attachment_probe',
    command: process.execPath,
    commandFallback: process.execPath,
    liveDispatch: true,
    versionArgs: ['--version'],
    runArgs: (_prompt, ctx) => [scriptPath, runtimeAttachmentPathFromContext(ctx)],
    promptBuilder: ({ current }) => current,
  });

  const ctx = {
    engine: { engine_id: 'shadow_attachment_probe' },
    message: {
      trace_id: 'validation:agent-runtime-shadow-attachment',
      request_id: 'shadow-attachment-request',
      engine_id: 'shadow_attachment_probe',
      agent_id: 'shadow-attachment-agent',
      session_id: 'shadow-attachment-session',
      turn_id: 'shadow-attachment-turn',
      working_directory: workDir,
      input: { text: 'Read the supplied pasted text attachment and return the secret value.' },
      context_pack: {
        source_authority: 'validation.agent_runtime_shadow_attachment_guard',
        runtime_attachment_refs: {
          type: 'agent_runtime_attachment_refs',
          source_authority: 'validation_shadow_attachment_fixture',
          attachment_count: 1,
          attachments: [
            {
              type: 'agent_runtime_attachment_ref',
              attachment_id: 'shadow-attachment-fixture',
              file_id: 'shadow-attachment-fixture',
              filename: 'pastedtext.txt',
              content_type: 'text/plain;charset=utf-8',
              source_kind: 'gateway_large_context_ref',
              size_bytes: fs.statSync(attachmentPath).size,
              local_read_path: attachmentPath,
              content_preview: 'SHADOW_ATTACHMENT_SECRET=shadow-attachment-wins',
              prompt_instruction: 'Read this file as supplemental pasted text context.',
            },
          ],
        },
        universal_tool_grants: {
          tools: [],
          source_authority: 'validation_shadow_attachment_fixture',
        },
      },
      capability_budget: {
        max_turn_seconds: 30,
      },
    },
  };

  let result = null;
  let error = null;
  try {
    result = await adapter.submit_turn(ctx);
  } catch (err) {
    error = {
      message: clean(err && err.message ? err.message : err, 1000),
    };
  }

  const output = clean(result && result.output_text, 2000);
  const ok = !error &&
    result &&
    result.status === 'completed' &&
    output.includes('shadow-attachment-wins') &&
    !output.includes('shadow attachment failed');
  const report = {
    ok: Boolean(ok),
    type: 'agent_runtime_shadow_attachment_guard',
    generated_at: new Date().toISOString(),
    mode: 'deterministic_cli_shadow_attachment_probe',
    policy: {
      shell_cognition_policy: 'No Shell involvement: runtime attachments are normalized by Gateway and translated by adapter seams.',
      shadow_workspace_policy: 'CLI runtimes may run in a shadow workspace; runtime attachment refs must remain readable there without granting direct writes to the real workspace.',
    },
    result: result ? {
      status: clean(result.status, 120),
      error_code: clean(result.error_code, 120),
      output_preview: output,
      activity_event_count: Number(result.activity_event_count) || 0,
      structured_activity: result.structured_activity === true,
      timed_out: result.timed_out === true,
    } : null,
    error,
    violations: ok ? [] : [
      'runtime_attachment_ref_was_not_available_as_shadow_local_file_for_cli_runtime',
    ],
  };
  writeText(ARTIFACT_PATH, `${JSON.stringify(report, null, 2)}\n`);
  try { fs.rmSync(scratch, { recursive: true, force: true }); } catch {}
  console.log(JSON.stringify(report, null, 2));
  process.exit(ok ? 0 : 1);
}

main().catch((error) => {
  const report = {
    ok: false,
    type: 'agent_runtime_shadow_attachment_guard',
    generated_at: new Date().toISOString(),
    error: {
      message: clean(error && error.message ? error.message : error, 1000),
    },
    violations: ['shadow_attachment_guard_crashed'],
  };
  writeText(ARTIFACT_PATH, `${JSON.stringify(report, null, 2)}\n`);
  console.log(JSON.stringify(report, null, 2));
  process.exit(1);
});
