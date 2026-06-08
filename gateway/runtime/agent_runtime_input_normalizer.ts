// Gateway-owned Agent Runtime input normalization.
//
// Policy lives here, not in Shell and not in engine adapters:
// - Shells may provide UX hints, but Gateway enforces payload/context shape.
// - Engine adapters receive normalized text + attachment refs and translate only.
// - Oversized inline text becomes a temp attachment before runtime dispatch.

const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { createHash } = require('node:crypto');

const LARGE_TEXT_ATTACHMENT_CHAR_THRESHOLD = Math.max(
  2000,
  Number(process.env.INFRING_AGENT_RUNTIME_LARGE_TEXT_ATTACHMENT_CHARS || 6000) || 6000,
);
const LARGE_TEXT_ATTACHMENT_LINE_THRESHOLD = Math.max(
  20,
  Number(process.env.INFRING_AGENT_RUNTIME_LARGE_TEXT_ATTACHMENT_LINES || 80) || 80,
);
const LARGE_TEXT_ATTACHMENT_MAX_CHARS = Math.max(
  32768,
  Number(process.env.INFRING_AGENT_RUNTIME_LARGE_TEXT_ATTACHMENT_MAX_CHARS || 262144) || 262144,
);

function cleanText(value, max = 1000) {
  return String(value == null ? '' : value)
    .replace(/[\u0000-\u0008\u000b\u000c\u000e-\u001f\u007f]/g, ' ')
    .replace(/\s+/g, ' ')
    .trim()
    .slice(0, max);
}

function cleanDisplayText(value, max = 12000) {
  return String(value == null ? '' : value)
    .replace(/[\u0000-\u0008\u000b\u000c\u000e-\u001f\u007f]/g, ' ')
    .trim()
    .slice(0, max);
}

function cleanEngineId(value) {
  return cleanText(value, 120).toLowerCase().replace(/[^a-z0-9_.-]/g, '_');
}

function cleanTranscriptComponent(value, max = 160) {
  return cleanText(value, max).replace(/[^a-zA-Z0-9_.:-]/g, '_');
}

function extractRawInputText(body) {
  const input = body && body.input;
  const inputPayload = input && typeof input === 'object'
    ? (input.text != null ? input.text : (input.message != null ? input.message : input.content))
    : input;
  const value = body && (body.message != null ? body.message : (body.text != null ? body.text : inputPayload));
  return String(value == null ? '' : value);
}

function extractAttachmentRows(body) {
  const input = body && body.input && typeof body.input === 'object' ? body.input : {};
  const candidates = [
    body && body.attachments,
    body && body.attachment_refs,
    body && body.files,
    input && input.attachments,
    input && input.attachment_refs,
    input && input.files,
  ];
  const rows = [];
  for (const candidate of candidates) {
    if (Array.isArray(candidate)) rows.push(...candidate);
  }
  const largeContextRef = cleanText(
    body && (
      body.large_context_ref ||
      body.large_paste_ref ||
      body.pasted_text_ref ||
      input.large_context_ref ||
      input.large_paste_ref ||
      input.pasted_text_ref
    ),
    1000,
  );
  if (largeContextRef) {
    const workingDirectory = cleanText(
      body && (
        body.working_directory ||
        body.current_working_directory ||
        body.present_working_directory ||
        body.cwd
      ),
      1000,
    );
    const candidatePath = path.isAbsolute(largeContextRef)
      ? largeContextRef
      : (workingDirectory ? path.resolve(workingDirectory, largeContextRef) : '');
    const safeReadPath = candidatePath && (
      path.isAbsolute(largeContextRef) ||
      (workingDirectory && candidatePath.startsWith(path.resolve(workingDirectory)))
    ) ? candidatePath : '';
    let sizeBytes = 0;
    let preview = '';
    if (safeReadPath) {
      try {
        const stat = fs.statSync(safeReadPath);
        if (stat.isFile()) {
          sizeBytes = stat.size;
          preview = fs.readFileSync(safeReadPath, 'utf8').slice(0, 12000);
        }
      } catch {}
    }
    rows.push({
      type: 'agent_runtime_attachment_ref',
      attachment_id: `large_context_ref_${createHash('sha256').update(largeContextRef).digest('hex').slice(0, 16)}`,
      file_id: `large_context_ref_${createHash('sha256').update(largeContextRef).digest('hex').slice(0, 16)}`,
      filename: path.basename(largeContextRef) || 'pastedtext.txt',
      content_type: 'text/plain;charset=utf-8',
      source_kind: 'gateway_large_context_ref',
      source_authority: 'gateway_agent_runtime_input_normalizer',
      size_bytes: sizeBytes,
      local_read_path: safeReadPath,
      content_preview: preview,
      prompt_instruction: safeReadPath
        ? `Read ${safeReadPath} as supplemental user-provided large pasted text context. Do not ask the user to paste it again.`
        : `Use attachment ref ${largeContextRef} as supplemental user-provided large pasted text context. Do not ask the user to paste it again.`,
    });
  }
  return rows;
}

function shouldMaterializeLargeText(rawText) {
  const value = String(rawText == null ? '' : rawText);
  if (value.length >= LARGE_TEXT_ATTACHMENT_CHAR_THRESHOLD) return true;
  if (value.length < 2000) return false;
  const lineCount = value.split(/\r\n|\r|\n/).length;
  return lineCount >= LARGE_TEXT_ATTACHMENT_LINE_THRESHOLD;
}

function materializeLargeTextAttachment({ traceId, engineId, agentId, sessionId, turnId, rawText }) {
  const normalized = String(rawText == null ? '' : rawText).replace(/\r\n?/g, '\n');
  const contentHash = createHash('sha256').update(normalized).digest('hex');
  const fileId = `large_paste_${contentHash.slice(0, 16)}`;
  const storedText = normalized.length > LARGE_TEXT_ATTACHMENT_MAX_CHARS
    ? `${normalized.slice(0, LARGE_TEXT_ATTACHMENT_MAX_CHARS)}\n\n[InfRing attachment note: source text exceeded the backend temp attachment cap and was truncated.]`
    : normalized;
  const safeSession = cleanTranscriptComponent(sessionId, 96) || 'session';
  const safeTurn = cleanTranscriptComponent(turnId, 96) || `turn_${Date.now().toString(36)}`;
  const filename = 'pastedtext.txt';
  let localReadPath = '';
  let writeError = '';
  try {
    const attachmentDir = path.join(os.tmpdir(), 'infring-agent-runtime-attachments', safeSession, safeTurn);
    fs.mkdirSync(attachmentDir, { recursive: true });
    localReadPath = path.join(attachmentDir, filename);
    fs.writeFileSync(localReadPath, storedText, 'utf8');
  } catch (error) {
    writeError = cleanText(error && error.message ? error.message : error, 240);
  }
  const promptInstruction = localReadPath
    ? `Read ${localReadPath} as supplemental user-provided context. Do not ask the user to paste it again.`
    : 'Use this large pasted text attachment preview as supplemental user-provided context; full temp materialization failed.';
  return {
    type: 'agent_runtime_attachment_ref',
    attachment_id: fileId,
    file_id: fileId,
    filename,
    content_type: 'text/plain;charset=utf-8',
    source_kind: 'gateway_large_text_attachment',
    source_authority: 'gateway_agent_runtime_input_normalizer',
    size_bytes: Buffer.byteLength(normalized, 'utf8'),
    stored_bytes: Buffer.byteLength(storedText, 'utf8'),
    truncated: storedText !== normalized,
    local_read_path: localReadPath,
    materialization_error: writeError,
    content_preview: cleanDisplayText(normalized, 12000),
    prompt_instruction: promptInstruction,
    trace_id: cleanText(traceId, 200),
    engine_id: cleanEngineId(engineId),
    agent_id: cleanText(agentId, 160),
    session_id: cleanText(sessionId, 200),
    turn_id: cleanText(turnId, 200),
  };
}

function normalizeAttachmentRefs(value) {
  const rows = Array.isArray(value) ? value : [];
  return rows
    .map((item, index) => {
      const source = item && typeof item === 'object' ? item : {};
      const filename = cleanText(source.filename || source.name || source.path || `attachment-${index + 1}`, 240);
      const fileId = cleanText(source.file_id || source.id || source.upload_id || '', 240);
      const contentType = cleanText(source.content_type || source.mime_type || 'application/octet-stream', 120);
      const sourceKind = cleanText(source.source_kind || source.kind || 'file_attachment', 80);
      const contentPreview = cleanDisplayText(source.content_preview || source.text_preview || source.preview || '', 12000);
      const localReadPath = cleanText(source.local_read_path || source.read_path || source.temp_path || '', 1000);
      if (!filename && !fileId && !contentPreview) return null;
      return {
        type: 'agent_runtime_attachment_ref',
        attachment_id: fileId || `attachment-${index + 1}`,
        file_id: fileId,
        filename,
        content_type: contentType,
        source_kind: sourceKind,
        size_bytes: Math.max(0, Number(source.size_bytes || source.size || 0) || 0),
        local_read_path: localReadPath,
        content_preview: contentPreview,
        prompt_instruction: cleanDisplayText(
          source.prompt_instruction || 'Treat this attachment as supplemental user-provided context. Use the filename/ref when citing it; do not ask the user to paste it again.',
          800,
        ),
      };
    })
    .filter(Boolean)
    .slice(0, 12);
}

function normalizeAgentRuntimeTurnInput({ body, traceId, engineId, agentId, sessionId, turnId }) {
  const rawText = extractRawInputText(body);
  const attachmentRefs = normalizeAttachmentRefs(extractAttachmentRows(body));
  let text = cleanDisplayText(rawText, 24000);
  let largeTextAttachment = null;
  if (shouldMaterializeLargeText(rawText)) {
    largeTextAttachment = materializeLargeTextAttachment({
      traceId,
      engineId,
      agentId,
      sessionId,
      turnId,
      rawText,
    });
    attachmentRefs.unshift(largeTextAttachment);
    text = cleanDisplayText(
      `User provided a large pasted text attachment (${largeTextAttachment.filename}, ${largeTextAttachment.size_bytes} bytes). ` +
        `${largeTextAttachment.local_read_path ? `Read it from ${largeTextAttachment.local_read_path}.` : 'Use the runtime attachment ref and preview.'}`,
      1200,
    );
  }
  if (!text && attachmentRefs.length) {
    text = cleanDisplayText(`User provided ${attachmentRefs.length} runtime attachment${attachmentRefs.length === 1 ? '' : 's'} for this turn.`, 1200);
  }
  return { text, attachmentRefs: attachmentRefs.slice(0, 12), largeTextAttachment };
}

module.exports = {
  normalizeAgentRuntimeTurnInput,
  normalizeAttachmentRefs,
  shouldMaterializeLargeText,
};
