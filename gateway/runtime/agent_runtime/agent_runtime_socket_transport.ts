#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::agent-runtime::socket-transport.
//
// This is the transport bridge for /ws/agent-runtime. It owns WebSocket
// framing, upgrade admission, and byte-level transport bounds only; Agent
// Runtime semantics remain in agent_runtime_socket_handler.ts.

'use strict';

const crypto = require('node:crypto');

const WS_GUID = '258EAFA5-E914-47DA-95CA-C5AB0DC85B11';
const MAX_SOCKET_MESSAGE_BYTES = 65536;

function cleanText(value, maxLen = 240) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function isAgentRuntimeSocketPath(pathname) {
  return String(pathname || '') === '/ws/agent-runtime';
}

function ignoreSocketError(socket) {
  if (!socket || typeof socket.on !== 'function') return;
  if (socket.__infringAgentRuntimeSocketIgnoreErrors) return;
  socket.__infringAgentRuntimeSocketIgnoreErrors = true;
  socket.on('error', () => {});
}

function websocketAcceptKey(key) {
  return crypto.createHash('sha1').update(`${cleanText(key, 200)}${WS_GUID}`).digest('base64');
}

function encodeAgentRuntimeSocketFrame(payload, options = {}) {
  const opcode = Number(options.opcode || 1);
  const mask = options.mask === true;
  const body = Buffer.isBuffer(payload) ? payload : Buffer.from(String(payload == null ? '' : payload), 'utf8');
  const length = body.length;
  let headerLength = 2;
  if (length >= 126 && length <= 65535) headerLength += 2;
  else if (length > 65535) headerLength += 8;
  const maskKey = mask ? crypto.randomBytes(4) : null;
  const header = Buffer.alloc(headerLength + (mask ? 4 : 0));
  let offset = 0;
  header[offset++] = 0x80 | (opcode & 0x0f);
  if (length < 126) {
    header[offset++] = (mask ? 0x80 : 0) | length;
  } else if (length <= 65535) {
    header[offset++] = (mask ? 0x80 : 0) | 126;
    header.writeUInt16BE(length, offset);
    offset += 2;
  } else {
    header[offset++] = (mask ? 0x80 : 0) | 127;
    header.writeUInt32BE(0, offset);
    offset += 4;
    header.writeUInt32BE(length, offset);
    offset += 4;
  }
  let encodedBody = body;
  if (maskKey) {
    maskKey.copy(header, offset);
    offset += 4;
    encodedBody = Buffer.alloc(body.length);
    for (let index = 0; index < body.length; index += 1) encodedBody[index] = body[index] ^ maskKey[index % 4];
  }
  return Buffer.concat([header, encodedBody]);
}

function decodeAgentRuntimeSocketFrames(input, options = {}) {
  let buffer = Buffer.isBuffer(input) ? input : Buffer.from(input || '');
  const frames = [];
  const expectMasked = options.expectMasked === true;
  const maxBytes = Math.max(1024, Number(options.maxBytes || MAX_SOCKET_MESSAGE_BYTES) || MAX_SOCKET_MESSAGE_BYTES);
  while (buffer.length >= 2) {
    const first = buffer[0];
    const second = buffer[1];
    const fin = (first & 0x80) !== 0;
    const opcode = first & 0x0f;
    const masked = (second & 0x80) !== 0;
    let length = second & 0x7f;
    let offset = 2;
    if (!fin) return { ok: false, error: 'fragmented_frames_not_supported', frames, remaining: Buffer.alloc(0) };
    if (expectMasked && !masked) return { ok: false, error: 'client_frame_not_masked', frames, remaining: Buffer.alloc(0) };
    if (length === 126) {
      if (buffer.length < offset + 2) break;
      length = buffer.readUInt16BE(offset);
      offset += 2;
    } else if (length === 127) {
      if (buffer.length < offset + 8) break;
      const high = buffer.readUInt32BE(offset);
      offset += 4;
      const low = buffer.readUInt32BE(offset);
      offset += 4;
      if (high !== 0) return { ok: false, error: 'frame_too_large', frames, remaining: Buffer.alloc(0) };
      length = low;
    }
    if (length > maxBytes) return { ok: false, error: 'frame_payload_too_large', frames, remaining: Buffer.alloc(0) };
    const maskOffset = offset;
    if (masked) offset += 4;
    if (buffer.length < offset + length) break;
    const payload = Buffer.from(buffer.subarray(offset, offset + length));
    if (masked) {
      const key = buffer.subarray(maskOffset, maskOffset + 4);
      for (let index = 0; index < payload.length; index += 1) payload[index] = payload[index] ^ key[index % 4];
    }
    frames.push({
      fin,
      opcode,
      masked,
      payload,
      text: payload.toString('utf8'),
    });
    buffer = buffer.subarray(offset + length);
  }
  return { ok: true, frames, remaining: buffer };
}

function sendSocketHttpError(socket, statusCode, reason) {
  try {
    socket.write(`HTTP/1.1 ${statusCode} ${reason}\r\nConnection: close\r\nContent-Length: 0\r\n\r\n`);
  } catch {}
  try { socket.destroy(); } catch {}
}

function createAgentRuntimeSocketTransport(options = {}) {
  const socketHandler = options.socketHandler;
  const maxMessageBytes = Math.max(1024, Number(options.maxMessageBytes || MAX_SOCKET_MESSAGE_BYTES) || MAX_SOCKET_MESSAGE_BYTES);
  if (!socketHandler || typeof socketHandler.handleAgentRuntimeSocketMessage !== 'function') {
    throw new Error('agent_runtime_socket_transport_handler_missing');
  }

  function sendJson(socket, event) {
    if (!socket || socket.destroyed) return;
    const payload = JSON.stringify(event || {});
    try { socket.write(encodeAgentRuntimeSocketFrame(payload)); } catch {}
  }

  function handleUpgrade(args = {}) {
    const req = args.req;
    const socket = args.socket;
    const head = args.head;
    const flags = args.flags || {};
    if (!req || !socket) return false;
    const requestUrl = new URL(req.url || '/', `http://${flags.host || '127.0.0.1'}:${flags.port || 0}`);
    if (!isAgentRuntimeSocketPath(requestUrl.pathname)) return false;
    ignoreSocketError(socket);
    const upgrade = cleanText(req.headers && req.headers.upgrade, 40).toLowerCase();
    const key = cleanText(req.headers && req.headers['sec-websocket-key'], 200);
    if (upgrade !== 'websocket' || !key) {
      sendSocketHttpError(socket, 400, 'Bad Request');
      return true;
    }
    socket.write([
      'HTTP/1.1 101 Switching Protocols',
      'Upgrade: websocket',
      'Connection: Upgrade',
      `Sec-WebSocket-Accept: ${websocketAcceptKey(key)}`,
      'Cache-Control: no-store',
      '',
      '',
    ].join('\r\n'));

    let pending = Buffer.alloc(0);
    let chain = Promise.resolve();
    const handleFrame = (frame) => {
      if (frame.opcode === 8) {
        try { socket.end(encodeAgentRuntimeSocketFrame(Buffer.alloc(0), { opcode: 8 })); } catch {}
        return Promise.resolve();
      }
      if (frame.opcode === 9) {
        try { socket.write(encodeAgentRuntimeSocketFrame(frame.payload, { opcode: 10 })); } catch {}
        return Promise.resolve();
      }
      if (frame.opcode !== 1) return Promise.resolve();
      let message = null;
      try {
        message = JSON.parse(frame.text || '{}');
      } catch {
        sendJson(socket, {
          type: 'error',
          error_code: 'agent_runtime_socket_invalid_json',
          reason: 'Agent Runtime socket frame was not valid JSON.',
          source_authority: 'gateway.runtime.agent_runtime_socket_transport',
        });
        return Promise.resolve();
      }
      return Promise.resolve(socketHandler.handleAgentRuntimeSocketMessage(
        message,
        (event) => sendJson(socket, event),
        { flags, req },
      )).catch((error) => {
        sendJson(socket, {
          type: 'error',
          trace_id: cleanText(message && message.trace_id, 200),
          request_id: cleanText(message && message.request_id, 200),
          engine_id: cleanText(message && message.engine_id, 120),
          error_code: 'agent_runtime_socket_handler_error',
          reason: cleanText(error && error.message ? error.message : error, 1000),
          source_authority: 'gateway.runtime.agent_runtime_socket_transport',
        });
      });
    };

    const consume = (chunk) => {
      pending = Buffer.concat([pending, Buffer.from(chunk || '')]);
      const decoded = decodeAgentRuntimeSocketFrames(pending, {
        expectMasked: true,
        maxBytes: maxMessageBytes,
      });
      if (!decoded.ok) {
        sendJson(socket, {
          type: 'error',
          error_code: decoded.error,
          reason: 'Agent Runtime socket frame rejected by Gateway transport policy.',
          source_authority: 'gateway.runtime.agent_runtime_socket_transport',
        });
        try { socket.end(encodeAgentRuntimeSocketFrame(Buffer.alloc(0), { opcode: 8 })); } catch {}
        pending = Buffer.alloc(0);
        return;
      }
      pending = decoded.remaining;
      for (const frame of decoded.frames) chain = chain.then(() => handleFrame(frame));
    };
    if (head && head.length) consume(head);
    socket.on('data', consume);
    return true;
  }

  return {
    handleUpgrade,
    tryHandle: (req, socket, head, extra = {}) => handleUpgrade({ req, socket, head, ...extra }),
  };
}

module.exports = {
  MAX_SOCKET_MESSAGE_BYTES,
  isAgentRuntimeSocketPath,
  websocketAcceptKey,
  encodeAgentRuntimeSocketFrame,
  decodeAgentRuntimeSocketFrames,
  createAgentRuntimeSocketTransport,
};
