#!/usr/bin/env node
/* eslint-disable no-console */

export type ShellSocketCapabilityId =
  | 'get_runtime_status'
  | 'list_agents'
  | 'list_sessions'
  | 'get_message_window'
  | 'get_message_detail'
  | 'submit_input'
  | 'submit_message_result'
  | 'subscribe_events'
  | 'search'
  | 'submit_issue'
  | 'submit_approval_decision'
  | 'login_session'
  | 'logout_session'
  | 'list_models'
  | 'list_providers'
  | 'discover_models'
  | 'download_model'
  | 'upsert_custom_model'
  | 'delete_custom_model'
  | 'save_provider_key'
  | 'remove_provider_key'
  | 'test_provider'
  | 'complete_provider'
  | 'set_provider_url'
  | 'start_provider_oauth'
  | 'poll_provider_oauth'
  | 'set_config'
  | 'set_model'
  | 'update_agent_config'
  | 'update_agent_mode'
  | 'update_agent_tools'
  | 'create_agent'
  | 'archive_agent'
  | 'revive_agent'
  | 'clone_agent'
  | 'clear_agent_history'
  | 'delete_archived_agent'
  | 'delete_all_archived_agents'
  | 'archive_all_agents'
  | 'stop_agent'
  | 'create_session'
  | 'switch_session'
  | 'delete_session'
  | 'list_memory_kv'
  | 'set_memory_kv'
  | 'delete_memory_kv'
  | 'request_agent_suggestions'
  | 'read_agent_file_artifact'
  | 'save_agent_file_artifact'
  | 'export_agent_folder_artifact'
  | 'create_workflow'
  | 'update_workflow'
  | 'delete_workflow'
  | 'run_workflow'
  | 'create_cron_job'
  | 'set_cron_job_enabled'
  | 'delete_cron_job'
  | 'run_schedule'
  | 'set_trigger_enabled'
  | 'delete_trigger'
  | 'start_channel_qr'
  | 'configure_channel'
  | 'test_channel'
  | 'remove_channel_config'
  | 'detect_migration_source'
  | 'scan_migration_source'
  | 'run_migration'
  | 'install_hand_dependencies'
  | 'check_hand_dependencies'
  | 'activate_hand'
  | 'pause_hand_instance'
  | 'resume_hand_instance'
  | 'deactivate_hand_instance'
  | 'install_skill'
  | 'uninstall_skill'
  | 'create_skill'
  | 'send_comms_message'
  | 'post_comms_task'
  | 'upsert_eye'
  | 'set_git_tree'
  | 'fresh_session'
  | 'compact_session'
  | 'submit_terminal_command';

export type ShellSocketRouteDefinition = {
  capabilityId: ShellSocketCapabilityId;
  method: 'GET' | 'POST';
  path: string;
  pathParams?: string[];
  queryParams?: string[];
};

export type ShellSocketClientOptions = {
  baseUrl?: string;
  fetchImpl?: (input: string, init?: Record<string, unknown>) => Promise<any>;
  defaultHeaders?: Record<string, string>;
};

export type ShellSocketRequestOptions = {
  query?: Record<string, unknown>;
  body?: unknown;
  headers?: Record<string, string>;
};

export const SHELL_SOCKET_ROUTES: ReadonlyArray<ShellSocketRouteDefinition> = Object.freeze([
  { capabilityId: 'get_runtime_status', method: 'GET', path: '/api/shell-socket/runtime-status' },
  { capabilityId: 'list_agents', method: 'GET', path: '/api/shell-socket/agents', queryParams: ['cursor', 'limit'] },
  { capabilityId: 'list_sessions', method: 'GET', path: '/api/shell-socket/agents/{agent_id}/sessions', pathParams: ['agent_id'], queryParams: ['cursor', 'limit'] },
  { capabilityId: 'get_message_window', method: 'GET', path: '/api/shell-socket/sessions/{session_id}/messages', pathParams: ['session_id'], queryParams: ['cursor', 'limit'] },
  { capabilityId: 'get_message_detail', method: 'GET', path: '/api/shell-socket/details/{detail_ref}', pathParams: ['detail_ref'], queryParams: ['view', 'limit'] },
  { capabilityId: 'submit_input', method: 'POST', path: '/api/shell-socket/input' },
  { capabilityId: 'submit_message_result', method: 'POST', path: '/api/shell-socket/agents/{agent_id}/message', pathParams: ['agent_id'] },
  { capabilityId: 'subscribe_events', method: 'GET', path: '/api/shell-socket/sessions/{session_id}/events', pathParams: ['session_id'], queryParams: ['cursor'] },
  { capabilityId: 'search', method: 'GET', path: '/api/shell-socket/search', queryParams: ['q', 'scope', 'cursor', 'limit'] },
  { capabilityId: 'submit_issue', method: 'POST', path: '/api/shell-socket/issues' },
  { capabilityId: 'submit_approval_decision', method: 'POST', path: '/api/shell-socket/approvals/{approval_id}/decision', pathParams: ['approval_id'] },
  { capabilityId: 'login_session', method: 'POST', path: '/api/shell-socket/auth/login' },
  { capabilityId: 'logout_session', method: 'POST', path: '/api/shell-socket/auth/logout' },
  { capabilityId: 'list_models', method: 'GET', path: '/api/shell-socket/models', queryParams: ['cursor', 'limit'] },
  { capabilityId: 'list_providers', method: 'GET', path: '/api/shell-socket/providers', queryParams: ['cursor', 'limit'] },
  { capabilityId: 'discover_models', method: 'POST', path: '/api/shell-socket/models/discover' },
  { capabilityId: 'download_model', method: 'POST', path: '/api/shell-socket/models/download' },
  { capabilityId: 'upsert_custom_model', method: 'POST', path: '/api/shell-socket/models/custom' },
  { capabilityId: 'delete_custom_model', method: 'POST', path: '/api/shell-socket/models/custom/delete' },
  { capabilityId: 'save_provider_key', method: 'POST', path: '/api/shell-socket/providers/{provider_id}/key', pathParams: ['provider_id'] },
  { capabilityId: 'remove_provider_key', method: 'POST', path: '/api/shell-socket/providers/{provider_id}/key/remove', pathParams: ['provider_id'] },
  { capabilityId: 'test_provider', method: 'POST', path: '/api/shell-socket/providers/{provider_id}/test', pathParams: ['provider_id'] },
  { capabilityId: 'complete_provider', method: 'POST', path: '/api/shell-socket/providers/{provider_id}/complete', pathParams: ['provider_id'] },
  { capabilityId: 'set_provider_url', method: 'POST', path: '/api/shell-socket/providers/{provider_id}/url', pathParams: ['provider_id'] },
  { capabilityId: 'start_provider_oauth', method: 'POST', path: '/api/shell-socket/providers/{provider_id}/oauth/start', pathParams: ['provider_id'] },
  { capabilityId: 'poll_provider_oauth', method: 'POST', path: '/api/shell-socket/providers/{provider_id}/oauth/poll', pathParams: ['provider_id'] },
  { capabilityId: 'set_config', method: 'POST', path: '/api/shell-socket/config/set' },
  { capabilityId: 'set_model', method: 'POST', path: '/api/shell-socket/agents/{agent_id}/model', pathParams: ['agent_id'] },
  { capabilityId: 'update_agent_config', method: 'POST', path: '/api/shell-socket/agents/{agent_id}/config', pathParams: ['agent_id'] },
  { capabilityId: 'update_agent_mode', method: 'POST', path: '/api/shell-socket/agents/{agent_id}/mode', pathParams: ['agent_id'] },
  { capabilityId: 'update_agent_tools', method: 'POST', path: '/api/shell-socket/agents/{agent_id}/tools', pathParams: ['agent_id'] },
  { capabilityId: 'create_agent', method: 'POST', path: '/api/shell-socket/agents/create' },
  { capabilityId: 'archive_agent', method: 'POST', path: '/api/shell-socket/agents/{agent_id}/archive', pathParams: ['agent_id'] },
  { capabilityId: 'revive_agent', method: 'POST', path: '/api/shell-socket/agents/{agent_id}/revive', pathParams: ['agent_id'] },
  { capabilityId: 'clone_agent', method: 'POST', path: '/api/shell-socket/agents/{agent_id}/clone', pathParams: ['agent_id'] },
  { capabilityId: 'clear_agent_history', method: 'POST', path: '/api/shell-socket/agents/{agent_id}/history/clear', pathParams: ['agent_id'] },
  { capabilityId: 'delete_archived_agent', method: 'POST', path: '/api/shell-socket/agents/{agent_id}/archived/delete', pathParams: ['agent_id'] },
  { capabilityId: 'delete_all_archived_agents', method: 'POST', path: '/api/shell-socket/agents/archived/delete-all' },
  { capabilityId: 'archive_all_agents', method: 'POST', path: '/api/shell-socket/agents/archive-all' },
  { capabilityId: 'stop_agent', method: 'POST', path: '/api/shell-socket/agents/{agent_id}/stop', pathParams: ['agent_id'] },
  { capabilityId: 'create_session', method: 'POST', path: '/api/shell-socket/agents/{agent_id}/sessions', pathParams: ['agent_id'] },
  { capabilityId: 'switch_session', method: 'POST', path: '/api/shell-socket/agents/{agent_id}/sessions/{session_id}/switch', pathParams: ['agent_id', 'session_id'] },
  { capabilityId: 'delete_session', method: 'POST', path: '/api/shell-socket/agents/{agent_id}/sessions/{session_id}/delete', pathParams: ['agent_id', 'session_id'] },
  { capabilityId: 'list_memory_kv', method: 'GET', path: '/api/shell-socket/agents/{agent_id}/memory/kv', pathParams: ['agent_id'] },
  { capabilityId: 'set_memory_kv', method: 'POST', path: '/api/shell-socket/agents/{agent_id}/memory/kv/{key}', pathParams: ['agent_id', 'key'] },
  { capabilityId: 'delete_memory_kv', method: 'POST', path: '/api/shell-socket/agents/{agent_id}/memory/kv/{key}/delete', pathParams: ['agent_id', 'key'] },
  { capabilityId: 'request_agent_suggestions', method: 'POST', path: '/api/shell-socket/agents/{agent_id}/suggestions', pathParams: ['agent_id'] },
  { capabilityId: 'read_agent_file_artifact', method: 'POST', path: '/api/shell-socket/agents/{agent_id}/artifacts/file/read', pathParams: ['agent_id'] },
  { capabilityId: 'save_agent_file_artifact', method: 'POST', path: '/api/shell-socket/agents/{agent_id}/files/{file_name}/save', pathParams: ['agent_id', 'file_name'] },
  { capabilityId: 'export_agent_folder_artifact', method: 'POST', path: '/api/shell-socket/agents/{agent_id}/artifacts/folder/export', pathParams: ['agent_id'] },
  { capabilityId: 'create_workflow', method: 'POST', path: '/api/shell-socket/workflows' },
  { capabilityId: 'update_workflow', method: 'POST', path: '/api/shell-socket/workflows/{workflow_id}/update', pathParams: ['workflow_id'] },
  { capabilityId: 'delete_workflow', method: 'POST', path: '/api/shell-socket/workflows/{workflow_id}/delete', pathParams: ['workflow_id'] },
  { capabilityId: 'run_workflow', method: 'POST', path: '/api/shell-socket/workflows/{workflow_id}/run', pathParams: ['workflow_id'] },
  { capabilityId: 'create_cron_job', method: 'POST', path: '/api/shell-socket/scheduler/jobs' },
  { capabilityId: 'set_cron_job_enabled', method: 'POST', path: '/api/shell-socket/scheduler/jobs/{job_id}/enable', pathParams: ['job_id'] },
  { capabilityId: 'delete_cron_job', method: 'POST', path: '/api/shell-socket/scheduler/jobs/{job_id}/delete', pathParams: ['job_id'] },
  { capabilityId: 'run_schedule', method: 'POST', path: '/api/shell-socket/scheduler/jobs/{job_id}/run', pathParams: ['job_id'] },
  { capabilityId: 'set_trigger_enabled', method: 'POST', path: '/api/shell-socket/scheduler/triggers/{trigger_id}/enable', pathParams: ['trigger_id'] },
  { capabilityId: 'delete_trigger', method: 'POST', path: '/api/shell-socket/scheduler/triggers/{trigger_id}/delete', pathParams: ['trigger_id'] },
  { capabilityId: 'start_channel_qr', method: 'POST', path: '/api/shell-socket/channels/{channel_id}/qr/start', pathParams: ['channel_id'] },
  { capabilityId: 'configure_channel', method: 'POST', path: '/api/shell-socket/channels/{channel_id}/configure', pathParams: ['channel_id'] },
  { capabilityId: 'test_channel', method: 'POST', path: '/api/shell-socket/channels/{channel_id}/test', pathParams: ['channel_id'] },
  { capabilityId: 'remove_channel_config', method: 'POST', path: '/api/shell-socket/channels/{channel_id}/configure/remove', pathParams: ['channel_id'] },
  { capabilityId: 'detect_migration_source', method: 'GET', path: '/api/shell-socket/migration/detect' },
  { capabilityId: 'scan_migration_source', method: 'POST', path: '/api/shell-socket/migration/scan' },
  { capabilityId: 'run_migration', method: 'POST', path: '/api/shell-socket/migration/run' },
  { capabilityId: 'install_hand_dependencies', method: 'POST', path: '/api/shell-socket/hands/{hand_id}/install-deps', pathParams: ['hand_id'] },
  { capabilityId: 'check_hand_dependencies', method: 'POST', path: '/api/shell-socket/hands/{hand_id}/check-deps', pathParams: ['hand_id'] },
  { capabilityId: 'activate_hand', method: 'POST', path: '/api/shell-socket/hands/{hand_id}/activate', pathParams: ['hand_id'] },
  { capabilityId: 'pause_hand_instance', method: 'POST', path: '/api/shell-socket/hands/instances/{instance_id}/pause', pathParams: ['instance_id'] },
  { capabilityId: 'resume_hand_instance', method: 'POST', path: '/api/shell-socket/hands/instances/{instance_id}/resume', pathParams: ['instance_id'] },
  { capabilityId: 'deactivate_hand_instance', method: 'POST', path: '/api/shell-socket/hands/instances/{instance_id}/deactivate', pathParams: ['instance_id'] },
  { capabilityId: 'install_skill', method: 'POST', path: '/api/shell-socket/skills/install' },
  { capabilityId: 'uninstall_skill', method: 'POST', path: '/api/shell-socket/skills/uninstall' },
  { capabilityId: 'create_skill', method: 'POST', path: '/api/shell-socket/skills/create' },
  { capabilityId: 'send_comms_message', method: 'POST', path: '/api/shell-socket/comms/send' },
  { capabilityId: 'post_comms_task', method: 'POST', path: '/api/shell-socket/comms/task' },
  { capabilityId: 'upsert_eye', method: 'POST', path: '/api/shell-socket/eyes' },
  { capabilityId: 'set_git_tree', method: 'POST', path: '/api/shell-socket/agents/{agent_id}/git-tree', pathParams: ['agent_id'] },
  { capabilityId: 'fresh_session', method: 'POST', path: '/api/shell-socket/agents/{agent_id}/fresh-session', pathParams: ['agent_id'] },
  { capabilityId: 'compact_session', method: 'POST', path: '/api/shell-socket/agents/{agent_id}/compact-session', pathParams: ['agent_id'] },
  { capabilityId: 'submit_terminal_command', method: 'POST', path: '/api/shell-socket/terminal/commands' },
]);

const ROUTES_BY_CAPABILITY = new Map(SHELL_SOCKET_ROUTES.map((route) => [route.capabilityId, route]));

function cleanSegment(value: unknown): string {
  return encodeURIComponent(String(value == null ? '' : value).trim());
}

function normalizeBaseUrl(value: unknown): string {
  const raw = String(value == null ? '' : value).trim();
  if (!raw) return '';
  return raw.replace(/\/+$/, '');
}

function appendQuery(path: string, query: Record<string, unknown> = {}, allowed: string[] = []): string {
  const params = new URLSearchParams();
  for (const key of allowed) {
    const value = query[key];
    if (value == null || value === '') continue;
    params.set(key, String(value));
  }
  const encoded = params.toString();
  return encoded ? `${path}?${encoded}` : path;
}

function fillPath(route: ShellSocketRouteDefinition, query: Record<string, unknown> = {}): string {
  let out = route.path;
  for (const key of route.pathParams || []) {
    const value = query[key];
    if (value == null || value === '') throw new Error(`missing_shell_socket_path_param:${route.capabilityId}:${key}`);
    out = out.replace(`{${key}}`, cleanSegment(value));
  }
  return appendQuery(out, query, route.queryParams || []);
}

function resolveFetch(fetchImpl?: ShellSocketClientOptions['fetchImpl']): ShellSocketClientOptions['fetchImpl'] {
  if (fetchImpl) return fetchImpl;
  const globalFetch = (globalThis as any).fetch;
  if (typeof globalFetch === 'function') return globalFetch.bind(globalThis);
  throw new Error('shell_socket_fetch_unavailable');
}

export class ShellSocketGatewayClient {
  private readonly baseUrl: string;
  private readonly fetchImpl: NonNullable<ShellSocketClientOptions['fetchImpl']>;
  private readonly defaultHeaders: Record<string, string>;

  constructor(options: ShellSocketClientOptions = {}) {
    this.baseUrl = normalizeBaseUrl(options.baseUrl);
    this.fetchImpl = resolveFetch(options.fetchImpl);
    this.defaultHeaders = Object.freeze({ ...(options.defaultHeaders || {}) });
  }

  routeFor(capabilityId: ShellSocketCapabilityId): ShellSocketRouteDefinition {
    const route = ROUTES_BY_CAPABILITY.get(capabilityId);
    if (!route) throw new Error(`unknown_shell_socket_capability:${capabilityId}`);
    return route;
  }

  urlFor(capabilityId: ShellSocketCapabilityId, query: Record<string, unknown> = {}): string {
    const route = this.routeFor(capabilityId);
    return `${this.baseUrl}${fillPath(route, query)}`;
  }

  async request<T = unknown>(
    capabilityId: ShellSocketCapabilityId,
    options: ShellSocketRequestOptions = {},
  ): Promise<T> {
    const route = this.routeFor(capabilityId);
    const headers = {
      accept: 'application/json',
      ...(route.method === 'POST' ? { 'content-type': 'application/json' } : {}),
      ...this.defaultHeaders,
      ...(options.headers || {}),
    };
    const init: Record<string, unknown> = { method: route.method, headers };
    if (route.method === 'POST') init.body = JSON.stringify(options.body || {});
    const response = await this.fetchImpl(this.urlFor(capabilityId, options.query || {}), init);
    const text = typeof response?.text === 'function' ? await response.text() : '';
    const payload = text ? JSON.parse(text) : {};
    if (!response || response.ok === false) {
      const status = Number(response?.status || 0);
      const error = new Error(`shell_socket_gateway_request_failed:${capabilityId}:${status || 'unknown'}`);
      (error as any).payload = payload;
      throw error;
    }
    return payload as T;
  }

  getRuntimeStatus<T = unknown>(): Promise<T> {
    return this.request<T>('get_runtime_status');
  }

  listAgents<T = unknown>(query: { cursor?: string; limit?: number } = {}): Promise<T> {
    return this.request<T>('list_agents', { query });
  }

  listSessions<T = unknown>(agentId: string, query: { cursor?: string; limit?: number } = {}): Promise<T> {
    return this.request<T>('list_sessions', { query: { ...query, agent_id: agentId } });
  }

  getMessageWindow<T = unknown>(sessionId: string, query: { cursor?: string; limit?: number } = {}): Promise<T> {
    return this.request<T>('get_message_window', { query: { ...query, session_id: sessionId } });
  }

  getMessageDetail<T = unknown>(detailRef: string, query: { view?: string; limit?: number } = {}): Promise<T> {
    return this.request<T>('get_message_detail', { query: { ...query, detail_ref: detailRef } });
  }

  submitInput<T = unknown>(input: unknown): Promise<T> {
    return this.request<T>('submit_input', { body: input });
  }

  submitMessageResult<T = unknown>(agentId: string, input: unknown): Promise<T> {
    return this.request<T>('submit_message_result', { query: { agent_id: agentId }, body: input });
  }

  subscribeEvents<T = unknown>(sessionId: string, query: { cursor?: string } = {}): Promise<T> {
    return this.request<T>('subscribe_events', { query: { ...query, session_id: sessionId } });
  }

  search<T = unknown>(query: { q: string; scope?: string; cursor?: string; limit?: number }): Promise<T> {
    return this.request<T>('search', { query });
  }

  submitIssue<T = unknown>(issue: unknown): Promise<T> {
    return this.request<T>('submit_issue', { body: issue });
  }

  submitApprovalDecision<T = unknown>(approvalId: string, decision: unknown): Promise<T> {
    return this.request<T>('submit_approval_decision', { query: { approval_id: approvalId }, body: decision });
  }

  loginSession<T = unknown>(request: unknown): Promise<T> {
    return this.request<T>('login_session', { body: request });
  }

  logoutSession<T = unknown>(request: unknown = {}): Promise<T> {
    return this.request<T>('logout_session', { body: request });
  }

  listModels<T = unknown>(query: { cursor?: string; limit?: number } = {}): Promise<T> {
    return this.request<T>('list_models', { query });
  }

  listProviders<T = unknown>(query: { cursor?: string; limit?: number } = {}): Promise<T> {
    return this.request<T>('list_providers', { query });
  }

  discoverModels<T = unknown>(request: unknown): Promise<T> {
    return this.request<T>('discover_models', { body: request });
  }

  downloadModel<T = unknown>(request: unknown): Promise<T> {
    return this.request<T>('download_model', { body: request });
  }

  upsertCustomModel<T = unknown>(request: unknown): Promise<T> {
    return this.request<T>('upsert_custom_model', { body: request });
  }

  deleteCustomModel<T = unknown>(request: unknown): Promise<T> {
    return this.request<T>('delete_custom_model', { body: request });
  }

  saveProviderKey<T = unknown>(providerId: string, request: unknown): Promise<T> {
    return this.request<T>('save_provider_key', { query: { provider_id: providerId }, body: request });
  }

  removeProviderKey<T = unknown>(providerId: string, request: unknown = {}): Promise<T> {
    return this.request<T>('remove_provider_key', { query: { provider_id: providerId }, body: request });
  }

  testProvider<T = unknown>(providerId: string, request: unknown = {}): Promise<T> {
    return this.request<T>('test_provider', { query: { provider_id: providerId }, body: request });
  }

  completeProvider<T = unknown>(providerId: string, request: unknown): Promise<T> {
    return this.request<T>('complete_provider', { query: { provider_id: providerId }, body: request });
  }

  setProviderUrl<T = unknown>(providerId: string, request: unknown): Promise<T> {
    return this.request<T>('set_provider_url', { query: { provider_id: providerId }, body: request });
  }

  startProviderOAuth<T = unknown>(providerId: string, request: unknown = {}): Promise<T> {
    return this.request<T>('start_provider_oauth', { query: { provider_id: providerId }, body: request });
  }

  pollProviderOAuth<T = unknown>(providerId: string, request: unknown): Promise<T> {
    return this.request<T>('poll_provider_oauth', { query: { provider_id: providerId }, body: request });
  }

  setConfig<T = unknown>(request: unknown): Promise<T> {
    return this.request<T>('set_config', { body: request });
  }

  setModel<T = unknown>(agentId: string, modelSelection: unknown): Promise<T> {
    return this.request<T>('set_model', { query: { agent_id: agentId }, body: modelSelection });
  }

  updateAgentConfig<T = unknown>(agentId: string, config: unknown): Promise<T> {
    return this.request<T>('update_agent_config', { query: { agent_id: agentId }, body: config });
  }

  updateAgentMode<T = unknown>(agentId: string, mode: unknown): Promise<T> {
    return this.request<T>('update_agent_mode', { query: { agent_id: agentId }, body: mode });
  }

  updateAgentTools<T = unknown>(agentId: string, tools: unknown): Promise<T> {
    return this.request<T>('update_agent_tools', { query: { agent_id: agentId }, body: tools });
  }

  createAgent<T = unknown>(request: unknown): Promise<T> {
    return this.request<T>('create_agent', { body: request });
  }

  archiveAgent<T = unknown>(agentId: string, request: unknown = {}): Promise<T> {
    return this.request<T>('archive_agent', { query: { agent_id: agentId }, body: request });
  }

  reviveAgent<T = unknown>(agentId: string, request: unknown = {}): Promise<T> {
    return this.request<T>('revive_agent', { query: { agent_id: agentId }, body: request });
  }

  cloneAgent<T = unknown>(agentId: string, request: unknown): Promise<T> {
    return this.request<T>('clone_agent', { query: { agent_id: agentId }, body: request });
  }

  clearAgentHistory<T = unknown>(agentId: string, request: unknown = {}): Promise<T> {
    return this.request<T>('clear_agent_history', { query: { agent_id: agentId }, body: request });
  }

  deleteArchivedAgent<T = unknown>(agentId: string, request: unknown = {}): Promise<T> {
    return this.request<T>('delete_archived_agent', { query: { agent_id: agentId }, body: request });
  }

  deleteAllArchivedAgents<T = unknown>(request: unknown = {}): Promise<T> {
    return this.request<T>('delete_all_archived_agents', { body: request });
  }

  archiveAllAgents<T = unknown>(request: unknown = {}): Promise<T> {
    return this.request<T>('archive_all_agents', { body: request });
  }

  stopAgent<T = unknown>(agentId: string, request: unknown = {}): Promise<T> {
    return this.request<T>('stop_agent', { query: { agent_id: agentId }, body: request });
  }

  createSession<T = unknown>(agentId: string, request: unknown = {}): Promise<T> {
    return this.request<T>('create_session', { query: { agent_id: agentId }, body: request });
  }

  switchSession<T = unknown>(agentId: string, sessionId: string, request: unknown = {}): Promise<T> {
    return this.request<T>('switch_session', { query: { agent_id: agentId, session_id: sessionId }, body: request });
  }

  deleteSession<T = unknown>(agentId: string, sessionId: string, request: unknown = {}): Promise<T> {
    return this.request<T>('delete_session', { query: { agent_id: agentId, session_id: sessionId }, body: request });
  }

  listMemoryKv<T = unknown>(agentId: string): Promise<T> {
    return this.request<T>('list_memory_kv', { query: { agent_id: agentId } });
  }

  setMemoryKv<T = unknown>(agentId: string, key: string, request: unknown): Promise<T> {
    return this.request<T>('set_memory_kv', { query: { agent_id: agentId, key }, body: request });
  }

  deleteMemoryKv<T = unknown>(agentId: string, key: string, request: unknown = {}): Promise<T> {
    return this.request<T>('delete_memory_kv', { query: { agent_id: agentId, key }, body: request });
  }

  requestAgentSuggestions<T = unknown>(agentId: string, request: unknown = {}): Promise<T> {
    return this.request<T>('request_agent_suggestions', { query: { agent_id: agentId }, body: request });
  }

  readAgentFileArtifact<T = unknown>(agentId: string, request: unknown): Promise<T> {
    return this.request<T>('read_agent_file_artifact', { query: { agent_id: agentId }, body: request });
  }

  saveAgentFileArtifact<T = unknown>(agentId: string, fileName: string, request: unknown): Promise<T> {
    return this.request<T>('save_agent_file_artifact', { query: { agent_id: agentId, file_name: fileName }, body: request });
  }

  exportAgentFolderArtifact<T = unknown>(agentId: string, request: unknown): Promise<T> {
    return this.request<T>('export_agent_folder_artifact', { query: { agent_id: agentId }, body: request });
  }

  createWorkflow<T = unknown>(request: unknown): Promise<T> {
    return this.request<T>('create_workflow', { body: request });
  }

  updateWorkflow<T = unknown>(workflowId: string, request: unknown): Promise<T> {
    return this.request<T>('update_workflow', { query: { workflow_id: workflowId }, body: request });
  }

  deleteWorkflow<T = unknown>(workflowId: string, request: unknown = {}): Promise<T> {
    return this.request<T>('delete_workflow', { query: { workflow_id: workflowId }, body: request });
  }

  runWorkflow<T = unknown>(workflowId: string, request: unknown = {}): Promise<T> {
    return this.request<T>('run_workflow', { query: { workflow_id: workflowId }, body: request });
  }

  createCronJob<T = unknown>(request: unknown): Promise<T> {
    return this.request<T>('create_cron_job', { body: request });
  }

  setCronJobEnabled<T = unknown>(jobId: string, request: unknown): Promise<T> {
    return this.request<T>('set_cron_job_enabled', { query: { job_id: jobId }, body: request });
  }

  deleteCronJob<T = unknown>(jobId: string, request: unknown = {}): Promise<T> {
    return this.request<T>('delete_cron_job', { query: { job_id: jobId }, body: request });
  }

  runSchedule<T = unknown>(jobId: string, request: unknown = {}): Promise<T> {
    return this.request<T>('run_schedule', { query: { job_id: jobId }, body: request });
  }

  setTriggerEnabled<T = unknown>(triggerId: string, request: unknown): Promise<T> {
    return this.request<T>('set_trigger_enabled', { query: { trigger_id: triggerId }, body: request });
  }

  deleteTrigger<T = unknown>(triggerId: string, request: unknown = {}): Promise<T> {
    return this.request<T>('delete_trigger', { query: { trigger_id: triggerId }, body: request });
  }

  startChannelQr<T = unknown>(channelId: string, request: unknown = {}): Promise<T> {
    return this.request<T>('start_channel_qr', { query: { channel_id: channelId }, body: request });
  }

  configureChannel<T = unknown>(channelId: string, request: unknown): Promise<T> {
    return this.request<T>('configure_channel', { query: { channel_id: channelId }, body: request });
  }

  testChannel<T = unknown>(channelId: string, request: unknown = {}): Promise<T> {
    return this.request<T>('test_channel', { query: { channel_id: channelId }, body: request });
  }

  removeChannelConfig<T = unknown>(channelId: string, request: unknown = {}): Promise<T> {
    return this.request<T>('remove_channel_config', { query: { channel_id: channelId }, body: request });
  }

  detectMigrationSource<T = unknown>(): Promise<T> {
    return this.request<T>('detect_migration_source');
  }

  scanMigrationSource<T = unknown>(request: unknown): Promise<T> {
    return this.request<T>('scan_migration_source', { body: request });
  }

  runMigration<T = unknown>(request: unknown): Promise<T> {
    return this.request<T>('run_migration', { body: request });
  }

  installHandDependencies<T = unknown>(handId: string, request: unknown = {}): Promise<T> {
    return this.request<T>('install_hand_dependencies', { query: { hand_id: handId }, body: request });
  }

  checkHandDependencies<T = unknown>(handId: string, request: unknown = {}): Promise<T> {
    return this.request<T>('check_hand_dependencies', { query: { hand_id: handId }, body: request });
  }

  activateHand<T = unknown>(handId: string, request: unknown): Promise<T> {
    return this.request<T>('activate_hand', { query: { hand_id: handId }, body: request });
  }

  pauseHandInstance<T = unknown>(instanceId: string, request: unknown = {}): Promise<T> {
    return this.request<T>('pause_hand_instance', { query: { instance_id: instanceId }, body: request });
  }

  resumeHandInstance<T = unknown>(instanceId: string, request: unknown = {}): Promise<T> {
    return this.request<T>('resume_hand_instance', { query: { instance_id: instanceId }, body: request });
  }

  deactivateHandInstance<T = unknown>(instanceId: string, request: unknown = {}): Promise<T> {
    return this.request<T>('deactivate_hand_instance', { query: { instance_id: instanceId }, body: request });
  }

  installSkill<T = unknown>(request: unknown): Promise<T> {
    return this.request<T>('install_skill', { body: request });
  }

  uninstallSkill<T = unknown>(request: unknown): Promise<T> {
    return this.request<T>('uninstall_skill', { body: request });
  }

  createSkill<T = unknown>(request: unknown): Promise<T> {
    return this.request<T>('create_skill', { body: request });
  }

  sendCommsMessage<T = unknown>(request: unknown): Promise<T> {
    return this.request<T>('send_comms_message', { body: request });
  }

  postCommsTask<T = unknown>(request: unknown): Promise<T> {
    return this.request<T>('post_comms_task', { body: request });
  }

  upsertEye<T = unknown>(request: unknown): Promise<T> {
    return this.request<T>('upsert_eye', { body: request });
  }

  setGitTree<T = unknown>(agentId: string, treeSelection: unknown): Promise<T> {
    return this.request<T>('set_git_tree', { query: { agent_id: agentId }, body: treeSelection });
  }

  freshSession<T = unknown>(agentId: string, request: unknown = {}): Promise<T> {
    return this.request<T>('fresh_session', { query: { agent_id: agentId }, body: request });
  }

  compactSession<T = unknown>(agentId: string, request: unknown = {}): Promise<T> {
    return this.request<T>('compact_session', { query: { agent_id: agentId }, body: request });
  }

  submitTerminalCommand<T = unknown>(command: unknown): Promise<T> {
    return this.request<T>('submit_terminal_command', { body: command });
  }
}

export function createShellSocketGatewayClient(options: ShellSocketClientOptions = {}): ShellSocketGatewayClient {
  return new ShellSocketGatewayClient(options);
}

export function shellSocketClientSelfTest(): Record<string, unknown> {
  const routeIds = SHELL_SOCKET_ROUTES.map((route) => route.capabilityId);
  const uniqueRouteIds = new Set(routeIds);
  const client = new ShellSocketGatewayClient({
    fetchImpl: async () => ({ ok: true, status: 200, text: async () => '{}' }),
  });
  return {
    ok: routeIds.length === uniqueRouteIds.size && routeIds.length === 81,
    type: 'shell_socket_gateway_client_self_test',
    route_count: routeIds.length,
    unique_route_count: uniqueRouteIds.size,
    sample_urls: {
      status: client.urlFor('get_runtime_status'),
      sessions: client.urlFor('list_sessions', { agent_id: 'agent-a', cursor: 'c1', limit: 10 }),
      detail: client.urlFor('get_message_detail', { detail_ref: 'detail-a', view: 'summary', limit: 1 }),
    },
  };
}

if (typeof process !== 'undefined' && Array.isArray(process.argv) && process.argv.some((arg) => arg === '--self-test=1' || arg === '--self-test')) {
  const result = shellSocketClientSelfTest();
  process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
  process.exitCode = result.ok ? 0 : 1;
}
