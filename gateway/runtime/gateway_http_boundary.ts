#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::http-boundary.
//
// Gateway owns bounded external HTTP body parsing, JSON response projection,
// and proxy header filtering. Legacy adapter hosts may serve sockets while
// migration continues, but they must delegate these boundary mechanics here.

'use strict';

const http = require('node:http');

const HOP_BY_HOP_HEADERS = new Set([
  'connection',
  'host',
  'keep-alive',
  'proxy-authenticate',
  'proxy-authorization',
  'te',
  'trailers',
  'transfer-encoding',
  'upgrade',
]);

function ignoreGatewayStreamErrors(stream) {
  if (!stream || typeof stream.on !== 'function') return;
  if (stream.__infringIgnoreErrorsInstalled) return;
  stream.__infringIgnoreErrorsInstalled = true;
  stream.on('error', () => {});
}

function sendGatewayJson(res, statusCode, value) {
  res.writeHead(statusCode, {
    'content-type': 'application/json; charset=utf-8',
    'cache-control': 'no-store',
  });
  res.end(`${JSON.stringify(value, null, 2)}\n`);
}

function readGatewayJsonBody(req, maxBytes = 65536) {
  return new Promise((resolve, reject) => {
    const chunks = [];
    let total = 0;
    ignoreGatewayStreamErrors(req);
    req.on('data', (chunk) => {
      const next = Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk);
      total += next.length;
      if (total > maxBytes) {
        reject(new Error('request_body_too_large'));
        return;
      }
      chunks.push(next);
    });
    req.on('end', () => {
      if (!chunks.length) {
        resolve({});
        return;
      }
      try {
        resolve(JSON.parse(Buffer.concat(chunks).toString('utf8') || '{}'));
      } catch {
        reject(new Error('request_body_invalid_json'));
      }
    });
    req.on('error', reject);
  });
}

function filterGatewayProxyHeaders(headers, host, traceId = '') {
  const out = {};
  for (const [key, value] of Object.entries(headers || {})) {
    if (!value || HOP_BY_HOP_HEADERS.has(String(key).toLowerCase())) continue;
    out[key] = value;
  }
  out.host = host;
  if (traceId) out['x-infring-trace-id'] = traceId;
  return out;
}

function gatewayProxyTargetFromOptions(options = {}) {
  return {
    host: options.apiHost || options.host,
    port: options.apiPort || options.port,
    headerHost: `${options.apiHost || options.host}:${options.apiPort || options.port}`,
    requestTraceId: typeof options.requestTraceId === 'function' ? options.requestTraceId : () => '',
  };
}

function proxyGatewayHttpRequest(req, res, options = {}) {
  const target = gatewayProxyTargetFromOptions(options);
  return new Promise((resolve, reject) => {
    ignoreGatewayStreamErrors(req);
    ignoreGatewayStreamErrors(res);
    ignoreGatewayStreamErrors(req && req.socket);
    ignoreGatewayStreamErrors(res && res.socket);
    const upstream = http.request({
      host: target.host,
      port: target.port,
      method: req.method || 'GET',
      path: req.url || '/',
      headers: filterGatewayProxyHeaders(req.headers, target.headerHost, target.requestTraceId(req)),
    }, (upstreamRes) => {
      ignoreGatewayStreamErrors(upstreamRes);
      ignoreGatewayStreamErrors(upstreamRes.socket);
      res.writeHead(upstreamRes.statusCode || 502, upstreamRes.headers);
      upstreamRes.pipe(res);
      upstreamRes.on('end', resolve);
      upstreamRes.on('error', reject);
    });
    ignoreGatewayStreamErrors(upstream);
    upstream.on('error', reject);
    req.pipe(upstream);
  });
}

function proxyGatewayUpgrade(req, socket, head, options = {}) {
  const target = gatewayProxyTargetFromOptions(options);
  ignoreGatewayStreamErrors(req);
  ignoreGatewayStreamErrors(req && req.socket);
  ignoreGatewayStreamErrors(socket);
  const upstream = http.request({
    host: target.host,
    port: target.port,
    path: req.url || '/',
    headers: {
      ...filterGatewayProxyHeaders(req.headers, target.headerHost, target.requestTraceId(req)),
      connection: 'Upgrade',
      upgrade: req.headers.upgrade || 'websocket',
    },
  });
  upstream.on('upgrade', (upstreamRes, upstreamSocket, upstreamHead) => {
    ignoreGatewayStreamErrors(upstreamRes);
    ignoreGatewayStreamErrors(upstreamSocket);
    const headerLines = [`HTTP/1.1 ${upstreamRes.statusCode || 101} ${upstreamRes.statusMessage || 'Switching Protocols'}`];
    for (const [key, value] of Object.entries(upstreamRes.headers || {})) {
      if (Array.isArray(value)) value.forEach((entry) => headerLines.push(`${key}: ${entry}`));
      else if (value != null) headerLines.push(`${key}: ${value}`);
    }
    socket.write(`${headerLines.join('\r\n')}\r\n\r\n`);
    if (head && head.length) upstreamSocket.write(head);
    if (upstreamHead && upstreamHead.length) socket.write(upstreamHead);
    upstreamSocket.pipe(socket).pipe(upstreamSocket);
  });
  upstream.on('response', (upstreamRes) => {
    ignoreGatewayStreamErrors(upstreamRes);
    socket.write(`HTTP/1.1 ${upstreamRes.statusCode || 502} ${upstreamRes.statusMessage || 'Bad Gateway'}\r\nConnection: close\r\n\r\n`);
    upstreamRes.pipe(socket);
  });
  upstream.on('error', () => { try { socket.destroy(); } catch {} });
  upstream.end();
}

module.exports = {
  HOP_BY_HOP_HEADERS,
  ignoreGatewayStreamErrors,
  sendGatewayJson,
  readGatewayJsonBody,
  filterGatewayProxyHeaders,
  proxyGatewayHttpRequest,
  proxyGatewayUpgrade,
};
