import net from 'node:net';
import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process';
const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge.ts');

export const CONDUIT_SCHEMA_ID = 'infring_conduit';
export const CONDUIT_SCHEMA_VERSION = '1.0';
export const MAX_CONDUIT_MESSAGE_TYPES = 10;

export type TsCommand =
  | { type: 'start_agent'; agent_id: string }
  | { type: 'stop_agent'; agent_id: string }
  | { type: 'query_receipt_chain'; from_hash?: string | null; limit?: number | null }
  | { type: 'list_active_agents' }
  | { type: 'get_system_status' }
  | { type: 'apply_policy_update'; patch_id: string; patch: unknown }
  | {
      type: 'install_extension';
      extension_id: string;
      wasm_sha256: string;
      capabilities: string[];
      plugin_type?: 'cognition_reflex' | 'substrate_adapter' | 'memory_backend';
      version?: string;
      wasm_component_path?: string;
      signature?: string;
      provenance?: string;
      recovery_max_attempts?: number;
      recovery_backoff_ms?: number;
    };

export const TS_COMMAND_TYPES = [
  'start_agent',
  'stop_agent',
  'query_receipt_chain',
  'list_active_agents',
  'get_system_status',
  'apply_policy_update',
  'install_extension',
] as const;

export type AgentLifecycleState = 'started' | 'stopped';

export type RustEvent =
  | { type: 'agent_lifecycle'; state: AgentLifecycleState; agent_id: string }
  | { type: 'receipt_added'; receipt_hash: string }
  | { type: 'system_feedback'; status: string; detail: unknown; violation_reason?: string | null };

export const RUST_EVENT_TYPES = [
  'agent_lifecycle',
  'receipt_added',
  'system_feedback',
] as const;

const BRIDGE_MESSAGE_TYPE_COUNT = TS_COMMAND_TYPES.length + RUST_EVENT_TYPES.length;
if (BRIDGE_MESSAGE_TYPE_COUNT > MAX_CONDUIT_MESSAGE_TYPES) {
  throw new Error(
    `conduit_message_budget_exceeded:${BRIDGE_MESSAGE_TYPE_COUNT}>${MAX_CONDUIT_MESSAGE_TYPES}`,
  );
}

export interface CapabilityToken {
  token_id: string;
  subject: string;
  capabilities: string[];
  issued_at_ms: number;
  expires_at_ms: number;
  signature: string;
}

export interface CommandSecurityMetadata {
  client_id: string;
  key_id: string;
  nonce: string;
  signature: string;
  capability_token: CapabilityToken;
}

export interface CommandEnvelope {
  schema_id: typeof CONDUIT_SCHEMA_ID;
  schema_version: typeof CONDUIT_SCHEMA_VERSION;
  request_id: string;
  ts_ms: number;
  command: TsCommand;
  security: CommandSecurityMetadata;
}

export interface ValidationReceipt {
  ok: boolean;
  fail_closed: boolean;
  reason: string;
  policy_receipt_hash: string;
  security_receipt_hash: string;
  receipt_hash: string;
}

export interface CrossingReceipt {
  crossing_id: string;
  direction: 'TsToRust' | 'RustToTs';
  command_type: string;
  deterministic_hash: string;
  ts_ms: number;
}

export interface ResponseEnvelope {
  schema_id: typeof CONDUIT_SCHEMA_ID;
  schema_version: typeof CONDUIT_SCHEMA_VERSION;
  request_id: string;
  ts_ms: number;
  event: RustEvent;
  validation: ValidationReceipt;
  crossing: CrossingReceipt;
  receipt_hash: string;
}

export interface ConduitClientSecurityConfig {
  client_id: string;
  signing_key_id: string;
  signing_secret: string;
  token_key_id: string;
  token_secret: string;
  token_ttl_ms: number;
}

type Transport = {
  sendLine(line: string): Promise<string>;
  close(): Promise<void>;
};

type StdioTransportOptions = {
  timeoutMs?: number;
};

process.env.INFRING_OPS_USE_PREBUILT = '0';
process.env.INFRING_OPS_LOCAL_TIMEOUT_MS = process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || '120000';
const conduitSecurityBridge = createOpsLaneBridge(
  __dirname,
  'conduit_client_security',
  'conduit-client-security-kernel',
);

function parseLastJson(stdout: string): Record<string, unknown> | null {
  const lines = String(stdout || '')
    .trim()
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    try {
      return JSON.parse(lines[i]) as Record<string, unknown>;
    } catch {}
  }
  return null;
}

function parseResponseEnvelope(raw: string): ResponseEnvelope {
  const parsed = parseLastJson(raw);
  if (!parsed) {
    throw new Error('conduit_response_decode_failed');
  }
  if (
    typeof parsed.request_id !== 'string' ||
    !parsed.event ||
    typeof parsed.event !== 'object' ||
    !parsed.validation ||
    typeof parsed.validation !== 'object' ||
    typeof parsed.receipt_hash !== 'string'
  ) {
    throw new Error('conduit_response_invalid');
  }
  return parsed as ResponseEnvelope;
}

function runConduitSecurityKernel(command: string, payload: Record<string, unknown>): Record<string, unknown> {
  const encoded = Buffer.from(JSON.stringify(payload), 'utf8').toString('base64');
  const out = conduitSecurityBridge.run([command, `--payload-base64=${encoded}`]);
  const parsed =
    out && out.payload && typeof out.payload === 'object'
      ? (out.payload as Record<string, unknown>)
      : parseLastJson(String((out && out.stdout) || ''));
  const status = out && Number.isFinite(Number(out.status)) ? Number(out.status) : 1;
  if (!parsed || status !== 0 || parsed.ok !== true) {
    throw new Error(`conduit_security_kernel_${command}_failed:${status}`);
  }
  return parsed.payload && typeof parsed.payload === 'object'
    ? (parsed.payload as Record<string, unknown>)
    : parsed;
}

function conduitProductionModeActive(): boolean {
  const values = [
    process.env.NODE_ENV,
    process.env.INFRING_ENV,
    process.env.INFRING_RUNTIME_ENV,
    process.env.INFRING_PROFILE,
  ].map((value) => String(value || '').trim().toLowerCase());
  return values.some((value) => value === 'production' || value === 'prod' || value === 'release');
}

function assertConduitFailSoftAllowed(reason: string): void {
  if (conduitProductionModeActive()) {
    throw new Error(`conduit_security_kernel_${reason}_failed_closed_production`);
  }
}

function resolveSecurityConfigViaKernel(
  override?: Partial<ConduitClientSecurityConfig>,
): ConduitClientSecurityConfig {
  try {
    return runConduitSecurityKernel('resolve-security-config', { override }) as ConduitClientSecurityConfig;
  } catch {
    assertConduitFailSoftAllowed('resolve_security_config');
    return {
      client_id: 'conduit-client-fail-soft',
      signing_key_id: 'conduit-client-fail-soft-signing-key',
      signing_secret: 'conduit-client-fail-soft-signing-secret',
      token_key_id: 'conduit-client-fail-soft-token-key',
      token_secret: 'conduit-client-fail-soft-token-secret',
      token_ttl_ms: Number.isFinite(Number(override?.token_ttl_ms)) && Number(override?.token_ttl_ms) > 0
        ? Math.floor(Number(override?.token_ttl_ms))
        : 60000,
    };
  }
}

function resolveTransportPolicyViaKernel(timeoutMs?: number): { stdio_timeout_ms: number } {
  const payload: Record<string, unknown> = {};
  if (Number.isFinite(Number(timeoutMs)) && Number(timeoutMs) > 0) {
    payload.timeout_ms = Math.floor(Number(timeoutMs));
  }
  try {
    return runConduitSecurityKernel('resolve-transport-policy', payload) as { stdio_timeout_ms: number };
  } catch {
    return {
      stdio_timeout_ms: Number.isFinite(Number(timeoutMs)) && Number(timeoutMs) > 0
        ? Math.floor(Number(timeoutMs))
        : 30000,
    };
  }
}

function buildEnvelopeViaKernel(
  request_id: string,
  ts_ms: number,
  command: TsCommand,
  security: ConduitClientSecurityConfig,
): CommandEnvelope {
  if (String(process.env.INFRING_CONDUIT_TS_FALLBACK || '') === '1') {
    assertConduitFailSoftAllowed('build_envelope_ts_fallback');
    const token = `fallback-token-${request_id}-${security.client_id}-${ts_ms}`;
    return {
      schema_id: CONDUIT_SCHEMA_ID,
      schema_version: CONDUIT_SCHEMA_VERSION,
      request_id,
      ts_ms,
      command,
      security: {
        client_id: security.client_id,
        key_id: security.signing_key_id,
        nonce: `fallback-nonce-${ts_ms}`,
        signature: `fallback-signature-${request_id}-${ts_ms}`,
        capability_token: {
          token_id: `fallback-token-${request_id}`,
          subject: security.client_id,
          capabilities: ['conduit_send'],
          issued_at_ms: ts_ms,
          expires_at_ms: ts_ms + Math.max(1, Math.floor(security.token_ttl_ms)),
          signature: token,
        },
      },
    };
  }
  const payload = { request_id, ts_ms, command, security };
  try {
    return runConduitSecurityKernel('build-envelope', payload) as CommandEnvelope;
  } catch {
    assertConduitFailSoftAllowed('build_envelope');
    const previousFallback = process.env.INFRING_CONDUIT_TS_FALLBACK;
    process.env.INFRING_CONDUIT_TS_FALLBACK = '1';
    try {
      return buildEnvelopeViaKernel(request_id, ts_ms, command, security);
    } finally {
      if (previousFallback == null) {
        delete process.env.INFRING_CONDUIT_TS_FALLBACK;
      } else {
        process.env.INFRING_CONDUIT_TS_FALLBACK = previousFallback;
      }
    }
  }
}

class UnixSocketTransport implements Transport {
  private readonly timeoutMs = 30_000;

  constructor(private readonly socketPath: string) {}

  async sendLine(line: string): Promise<string> {
    return new Promise((resolve, reject) => {
      const socket = net.createConnection({ path: this.socketPath });
      let out = '';
      let settled = false;
      const settle = (fn: () => void) => {
        if (settled) return;
        settled = true;
        cleanup();
        socket.destroy();
        fn();
      };
      const timer = setTimeout(() => {
        settle(() => reject(new Error(`conduit_unix_socket_timeout:${this.timeoutMs}`)));
      }, this.timeoutMs);
      const cleanup = () => {
        clearTimeout(timer);
        socket.off('connect', onConnect);
        socket.off('data', onData);
        socket.off('error', onError);
        socket.off('close', onClose);
      };
      const onConnect = () => {
        socket.write(line.endsWith('\n') ? line : `${line}\n`);
      };
      const onData = (chunk: string) => {
        out += chunk;
        if (out.includes('\n')) {
          const firstLine = out.slice(0, out.indexOf('\n')).trim();
          settle(() => resolve(firstLine));
        }
      };
      const onError = (error: Error) => {
        settle(() => reject(error));
      };
      const onClose = () => {
        if (!out.trim()) {
          settle(() => reject(new Error('conduit_unix_socket_empty_response')));
        }
      };
      socket.setEncoding('utf8');
      socket.on('connect', onConnect);
      socket.on('data', onData);
      socket.on('error', onError);
      socket.on('close', onClose);
    });
  }

  async close(): Promise<void> {
    return Promise.resolve();
  }
}

class StdioTransport implements Transport {
  private readonly proc: ChildProcessWithoutNullStreams;
  private readonly timeoutMs: number;

  constructor(command: string, args: string[] = [], cwd?: string, options: StdioTransportOptions = {}) {
    this.proc = spawn(command, args, { cwd, stdio: 'pipe' });
    // Prevent uncaught EPIPE events when child processes exit before accepting stdin writes.
    this.proc.stdin.on('error', () => {});
    const policy = resolveTransportPolicyViaKernel(options.timeoutMs);
    const configured = Number(policy && policy.stdio_timeout_ms);
    this.timeoutMs = Number.isFinite(configured) && configured > 0 ? Math.floor(configured) : 30000;
  }

  async sendLine(line: string): Promise<string> {
    return new Promise((resolve, reject) => {
      let out = '';
      let settled = false;
      const settle = (fn: () => void) => {
        if (settled) return;
        settled = true;
        cleanup();
        fn();
      };
      const onData = (chunk: string | Buffer) => {
        out += chunk.toString();
        if (parseLastJson(out)) {
          settle(() => resolve(out.trim()));
        }
      };
      const onErr = (chunk: string | Buffer) => {
        settle(() => reject(new Error(`conduit_stdio_error:${chunk.toString().trim()}`)));
      };
      const onExit = (code: number | null) => {
        settle(() => reject(new Error(`conduit_stdio_exit:${code ?? 'unknown'}`)));
      };
      const timer = setTimeout(() => {
        settle(() => reject(new Error(`conduit_stdio_timeout:${this.timeoutMs}`)));
      }, this.timeoutMs);
      const cleanup = () => {
        clearTimeout(timer);
        this.proc.stdout.off('data', onData);
        this.proc.stderr.off('data', onErr);
        this.proc.off('exit', onExit);
      };

      this.proc.stdout.on('data', onData);
      this.proc.stderr.on('data', onErr);
      this.proc.once('exit', onExit);
      if (this.proc.exitCode !== null || this.proc.stdin.destroyed || !this.proc.stdin.writable) {
        settle(() => reject(new Error(`conduit_stdio_exit:${this.proc.exitCode ?? 'unknown'}`)));
        return;
      }
      try {
        this.proc.stdin.write(line.endsWith('\n') ? line : `${line}\n`, (error?: Error | null) => {
          if (error) {
            settle(() => reject(new Error(`conduit_stdio_exit:${error.message}`)));
          }
        });
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        settle(() => reject(new Error(`conduit_stdio_exit:${message}`)));
      }
    });
  }

  async close(): Promise<void> {
    if (!this.proc.killed) {
      this.proc.kill('SIGTERM');
    }
  }
}

export class ConduitClient {
  private constructor(
    private readonly transport: Transport,
    private readonly security: ConduitClientSecurityConfig,
  ) {}

  static overUnixSocket(path: string, security?: Partial<ConduitClientSecurityConfig>): ConduitClient {
    return new ConduitClient(new UnixSocketTransport(path), resolveSecurityConfig(security));
  }

  static overStdio(
    command: string,
    args: string[] = [],
    cwd?: string,
    security?: Partial<ConduitClientSecurityConfig>,
    options: StdioTransportOptions = {},
  ): ConduitClient {
    return new ConduitClient(new StdioTransport(command, args, cwd, options), resolveSecurityConfig(security));
  }

  async send(command: TsCommand, requestId?: string): Promise<ResponseEnvelope> {
    const ts_ms = Date.now();
    const request_id = requestId ?? `ts-${ts_ms}`;
    const envelope = buildEnvelopeViaKernel(request_id, ts_ms, command, this.security);

    const line = JSON.stringify(envelope);
    const raw = await this.transport.sendLine(line);
    return parseResponseEnvelope(raw);
  }

  async close(): Promise<void> {
    await this.transport.close();
  }
}

function resolveSecurityConfig(
  override?: Partial<ConduitClientSecurityConfig>,
): ConduitClientSecurityConfig {
  const candidate = override || {};
  const hasCompleteOverride =
    typeof candidate.client_id === 'string' &&
    candidate.client_id.trim().length > 0 &&
    typeof candidate.signing_key_id === 'string' &&
    candidate.signing_key_id.trim().length > 0 &&
    typeof candidate.signing_secret === 'string' &&
    candidate.signing_secret.trim().length > 0 &&
    typeof candidate.token_key_id === 'string' &&
    candidate.token_key_id.trim().length > 0 &&
    typeof candidate.token_secret === 'string' &&
    candidate.token_secret.trim().length > 0 &&
    Number.isFinite(Number(candidate.token_ttl_ms)) &&
    Number(candidate.token_ttl_ms) > 0;
  if (hasCompleteOverride) {
    return {
      client_id: String(candidate.client_id),
      signing_key_id: String(candidate.signing_key_id),
      signing_secret: String(candidate.signing_secret),
      token_key_id: String(candidate.token_key_id),
      token_secret: String(candidate.token_secret),
      token_ttl_ms: Math.floor(Number(candidate.token_ttl_ms)),
    };
  }
  return resolveSecurityConfigViaKernel(override);
}
