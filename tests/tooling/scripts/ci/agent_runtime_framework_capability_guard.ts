#!/usr/bin/env node
/* eslint-disable no-console */

const fs = require('node:fs');
const path = require('node:path');

const ROOT = process.cwd();
const OUT_JSON = path.join(ROOT, 'core/local/artifacts/agent_runtime_framework_capability_guard_current.json');
const SOURCE_DOMAIN = 'validation';
const OWNER_DOMAIN = 'validation.agent_runtime';
const POLICY_PATH = 'validation/conformance/contracts/agent_runtime_engine_registry.json';
const LAYER = 'gateway';
const {
  createCliRuntimeEngineAdapter,
  nativeDirectMutationGrantActive,
  parseCliActivityOutput,
} = require(path.join(ROOT, 'adapters/runtime/agent_engines/cli_runtime_adapter.ts'));
const {
  codexSandboxMode,
  createCodexCliEngineAdapter,
} = require(path.join(ROOT, 'adapters/runtime/agent_engines/codex_cli.ts'));
const {
  claudePermissionArgs,
  createClaudeCodeEngineAdapter,
} = require(path.join(ROOT, 'adapters/runtime/agent_engines/claude_code.ts'));

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
  const registryPath = 'validation/conformance/contracts/agent_runtime_engine_registry.json';
  const posturePath = 'adapters/runtime/agent_engines/RELIABILITY_POSTURE.md';
  const contextPackContractPath = 'validation/conformance/contracts/agent_runtime_context_pack_contract.json';
  const taskMatrixPath = 'validation/agent_runtime/task_harness/agentic_task_matrix.json';
  const turnProjectionPath = 'gateway/runtime/agent_runtime/agent_runtime_turn_projection.ts';
  const structuredTransportPath = 'gateway/runtime/agent_runtime/agent_runtime_structured_transport.ts';
  const universalContractPath = 'validation/conformance/contracts/agent_runtime_universal_tools_contract.json';
  const universalGatewayPath = 'gateway/runtime/agent_runtime/universal_core_tools.ts';
  const codex = read(codexPath);
  const claude = read(claudePath);
  const grok = read(grokPath);
  const opencode = read(opencodePath);
  const shared = read(sharedPath);
  const posture = read(posturePath);
  const registry = JSON.parse(read(registryPath));
  const contextPackContract = JSON.parse(read(contextPackContractPath));
  const taskMatrix = JSON.parse(read(taskMatrixPath));
  const turnProjection = read(turnProjectionPath);
  const structuredTransport = read(structuredTransportPath);
  const universalContract = JSON.parse(read(universalContractPath));
  const universalGateway = read(universalGatewayPath);
  const violations = [];
  const canonicalUniversalToolIds = [
    'conversation.read',
    'memory.read',
    'memory.write_propose',
    'artifact.read',
    'artifact.create_propose',
    'permission.request',
  ];

  const contractToolIds = new Set(
    (Array.isArray(universalContract.tools) ? universalContract.tools : [])
      .map((row) => String(row && row.tool_id || ''))
      .filter(Boolean),
  );
  for (const toolId of canonicalUniversalToolIds) {
    if (!contractToolIds.has(toolId)) {
      push(violations, 'universal_tool_contract_missing_canonical_tool', universalContractPath, `Universal tool contract must include ${toolId}.`);
    }
  }
  for (const toolId of contractToolIds) {
    if (!canonicalUniversalToolIds.includes(toolId)) {
      push(violations, 'universal_tool_contract_unapproved_tool', universalContractPath, `Universal tool contract must not grow undeclared tool ${toolId}.`);
    }
  }
  const toolScope = universalContract.tool_surface_scope || {};
  if (
    toolScope.max_tool_count !== 6 ||
    toolScope.terminal_execution_allowed !== false ||
    toolScope.direct_file_write_allowed !== false ||
    toolScope.workflow_tools_allowed !== false ||
    toolScope.mutating_tools_must_be_proposal_only !== true
  ) {
    push(violations, 'universal_tool_scope_allows_native_authority', universalContractPath, 'Universal tool scope must stay tiny, proposal-only, and must forbid terminal execution/direct file write.');
  }
  const forbiddenUniversalization = new Set(Array.isArray(universalContract.forbidden_universalization) ? universalContract.forbidden_universalization : []);
  for (const forbidden of ['terminal_command_execution', 'direct_file_write', 'direct_kernel_mutation', 'direct_shell_socket_access']) {
    if (!forbiddenUniversalization.has(forbidden)) {
      push(violations, 'universal_tool_forbidden_universalization_missing', universalContractPath, `Universal tool contract must forbid ${forbidden}.`);
    }
  }
  const universalIdBlock = /const\s+UNIVERSAL_CORE_TOOL_IDS\s*=\s*Object\.freeze\(\[([\s\S]*?)\]\);/.exec(universalGateway);
  const universalIdBlockText = universalIdBlock ? universalIdBlock[1] : '';
  if (!universalIdBlockText) {
    push(violations, 'gateway_universal_tool_ids_block_missing', universalGatewayPath, 'Gateway universal tool implementation must declare UNIVERSAL_CORE_TOOL_IDS explicitly.');
  } else {
    for (const toolId of canonicalUniversalToolIds) {
      if (!universalIdBlockText.includes(`'${toolId}'`)) {
        push(violations, 'gateway_universal_tool_ids_missing_canonical_tool', universalGatewayPath, `Gateway universal tool ids must include ${toolId}.`);
      }
    }
    for (const forbidden of ['direct_file_write', 'native.direct_file_write', 'filesystem.direct_write', 'terminal_command_execution']) {
      if (universalIdBlockText.includes(forbidden)) {
        push(violations, 'native_mutation_grant_leaked_into_universal_tools', universalGatewayPath, `Native grant ${forbidden} must not appear in UNIVERSAL_CORE_TOOL_IDS.`);
      }
    }
  }
  if (!universalGateway.includes('direct_file_write_allowed: false') || !universalGateway.includes('terminal_execution_allowed: false')) {
    push(violations, 'gateway_universal_tool_scope_not_fail_closed', universalGatewayPath, 'Gateway universal tool scope must explicitly forbid direct file write and terminal execution.');
  }

  const contextPackRequiredFields = new Set(Array.isArray(contextPackContract.required_gateway_enriched_pack_fields) ? contextPackContract.required_gateway_enriched_pack_fields : []);
  if (!contextPackRequiredFields.has('runtime_stack_declaration')) {
    push(violations, 'runtime_stack_declaration_contract_field_missing', contextPackContractPath, 'Context pack contract must require runtime_stack_declaration in Gateway-enriched packs.');
  }
  const contextPackRules = contextPackContract.gateway_enriched_pack_rules || {};
  if (contextPackRules.runtime_stack_declaration_required !== true || contextPackRules.adapters_must_render_runtime_stack_declaration !== true) {
    push(violations, 'runtime_stack_declaration_contract_rule_missing', contextPackContractPath, 'Context pack rules must require Gateway attachment and adapter rendering of runtime_stack_declaration.');
  }
  if (!turnProjection.includes('function buildRuntimeStackDeclaration(') || !turnProjection.includes('pack.runtime_stack_declaration = runtimeStackDeclaration')) {
    push(violations, 'runtime_stack_declaration_gateway_attachment_missing', turnProjectionPath, 'Gateway turn projection must build and attach runtime_stack_declaration before adapter dispatch.');
  }
  for (const marker of ['host_substrate', 'active_engine_id', 'permission_owner', 'durable_effect_rule', 'receipt_rule']) {
    if (!turnProjection.includes(marker)) {
      push(violations, 'runtime_stack_declaration_gateway_marker_missing', turnProjectionPath, 'Gateway runtime stack declaration must include ' + marker + '.');
    }
  }
  if (!shared.includes('function renderRuntimeStackDeclarationPromptSection(') || !shared.includes('Runtime stack declaration:')) {
    push(violations, 'runtime_stack_declaration_adapter_prompt_missing', sharedPath, 'Shared CLI adapter must render runtime_stack_declaration in the bounded prompt-text compatibility preamble.');
  }
  if (!structuredTransport.includes('runtime_stack_declaration')) {
    push(violations, 'runtime_stack_declaration_structured_transport_missing', structuredTransportPath, 'Structured transport projection must carry runtime_stack_declaration for post-prompt-text runtimes.');
  }
  const runtimeStackTask = Array.isArray(taskMatrix.tasks) ? taskMatrix.tasks.find((task) => task && task.id === 'runtime_stack_layer_awareness') : null;
  if (!runtimeStackTask) {
    push(violations, 'runtime_stack_layer_awareness_task_missing', taskMatrixPath, 'Agent runtime task matrix must include runtime_stack_layer_awareness.');
  } else {
    const passSignals = new Set(Array.isArray(runtimeStackTask.pass_signals) ? runtimeStackTask.pass_signals : []);
    for (const signal of ['InfRing', 'host substrate', 'active engine', 'permission', 'approval', 'receipt', 'proposal']) {
      if (!passSignals.has(signal)) {
        push(violations, 'runtime_stack_layer_awareness_pass_signal_missing', taskMatrixPath, 'runtime_stack_layer_awareness must require pass signal ' + signal + '.');
      }
    }
  }

  if (!codex.includes("'--skip-git-repo-check'") && !codex.includes('"--skip-git-repo-check"')) {
    push(violations, 'codex_skip_git_repo_check_missing', codexPath, 'Codex must support arbitrary Gateway-selected workspaces, including non-git scratch directories.');
  }
  if (!codex.includes("'--sandbox'") || !codex.includes('codexSandboxMode(ctx)')) {
    push(violations, 'codex_sandbox_policy_missing', codexPath, 'Codex must keep Gateway permission policy mapped to Codex sandbox mode.');
  }
  if (!shared.includes('function nativeDirectMutationGrantActive(ctx)') || !shared.includes('nativeDirectMutationGrantActive,')) {
    push(violations, 'shared_native_direct_mutation_grant_primitive_missing', sharedPath, 'Codex and Claude must share Gateway direct-mutation grant parsing through cli_runtime_adapter.ts.');
  }
  if (!codex.includes('nativeDirectMutationGrantActive') || codex.includes('DIRECT_NATIVE_MUTATION_GRANTS') || codex.includes('always.some((toolId)')) {
    push(violations, 'codex_shared_mutation_grant_primitive_missing', codexPath, 'Codex must consume nativeDirectMutationGrantActive(ctx) from the shared CLI adapter instead of reimplementing grant parsing.');
  }
  if (!claude.includes('nativeDirectMutationGrantActive') || claude.includes('DIRECT_NATIVE_MUTATION_GRANTS') || claude.includes('always.some((toolId)')) {
    push(violations, 'claude_shared_mutation_grant_primitive_missing', claudePath, 'Claude Code must consume nativeDirectMutationGrantActive(ctx) from the shared CLI adapter instead of reimplementing grant parsing.');
  }
  const grantCtx = (alwaysAllowedToolCalls) => ({
    message: {
      context_pack: {
        universal_tool_grants: {
          permission_policy: {
            always_allowed_tool_calls: alwaysAllowedToolCalls,
          },
        },
      },
    },
  });
  const directGrantAliases = ['direct_file_write', 'native.direct_file_write', 'filesystem.direct_write'];
  for (const alias of directGrantAliases) {
    if (nativeDirectMutationGrantActive(grantCtx([alias])) !== true) {
      push(violations, 'shared_mutation_grant_alias_not_recognized', sharedPath, `Shared direct mutation primitive must recognize ${alias}.`);
    }
  }
  for (const denied of [[], ['artifact.create_propose'], ['permission.request'], ['memory.write_propose']]) {
    if (nativeDirectMutationGrantActive(grantCtx(denied)) !== false) {
      push(violations, 'shared_mutation_grant_false_positive', sharedPath, `Shared direct mutation primitive must not treat ${JSON.stringify(denied)} as native direct mutation authority.`);
    }
  }
  const goldenPairBaseline = registry.golden_pair_baseline || {};
  const goldenPairEngines = new Set(Array.isArray(goldenPairBaseline.engines) ? goldenPairBaseline.engines : []);
  const goldenPairOutcomes = new Set(Array.isArray(goldenPairBaseline.required_terminal_outcomes) ? goldenPairBaseline.required_terminal_outcomes : []);
  const goldenPairPrimitives = new Set(Array.isArray(goldenPairBaseline.required_shared_primitives) ? goldenPairBaseline.required_shared_primitives : []);
  for (const engineId of ['codex_cli', 'claude_code']) {
    if (!goldenPairEngines.has(engineId)) {
      push(violations, 'golden_pair_registry_engine_missing', registryPath, `Registry golden_pair_baseline must include ${engineId}.`);
    }
  }
  for (const outcome of ['completed_response', 'permission_required', 'approval_resume_completed', 'typed_failure']) {
    if (!goldenPairOutcomes.has(outcome)) {
      push(violations, 'golden_pair_registry_terminal_outcome_missing', registryPath, `Registry golden_pair_baseline must require ${outcome}.`);
    }
  }
  for (const primitive of ['native_direct_mutation_grant', 'text_proposal_pause', 'approval_resume_context', 'visible_final_response', 'typed_failure_projection']) {
    if (!goldenPairPrimitives.has(primitive)) {
      push(violations, 'golden_pair_registry_shared_primitive_missing', registryPath, `Registry golden_pair_baseline must require ${primitive}.`);
    }
  }
  if (goldenPairBaseline.enforced_by !== 'npm run ops:agent-runtime:framework-capability:guard') {
    push(violations, 'golden_pair_registry_enforcer_missing', registryPath, 'Registry golden_pair_baseline must point at the framework capability guard.');
  }
  const liveSmoke = registry.golden_pair_live_smoke || {};
  const liveSmokeEngines = new Set(Array.isArray(liveSmoke.engines) ? liveSmoke.engines : []);
  if (liveSmoke.status !== 'passing' || liveSmoke.scope !== 'bounded_live_response' || liveSmoke.required_outcome !== 'completed_visible_exact_marker') {
    push(violations, 'golden_pair_live_smoke_registry_missing', registryPath, 'Registry must distinguish passing bounded live response smoke from deeper real-work proof.');
  }
  for (const engineId of ['codex_cli', 'claude_code']) {
    if (!liveSmokeEngines.has(engineId)) {
      push(violations, 'golden_pair_live_smoke_engine_missing', registryPath, `Registry golden_pair_live_smoke must include ${engineId}.`);
    }
  }
  if (!String(liveSmoke.proof_artifact || '').includes('agent_runtime_cli_live_turn_smoke_current.json')) {
    push(violations, 'golden_pair_live_smoke_artifact_missing', registryPath, 'Registry golden_pair_live_smoke must point at the live-turn smoke artifact.');
  }
  const liveSmokeDoesNotProve = new Set(Array.isArray(liveSmoke.does_not_prove) ? liveSmoke.does_not_prove : []);
  for (const pendingProof of ['live_real_work_task_success', 'artifact_creation', 'approval_pause_resume_live', 'native_infring_intelligence_parity']) {
    if (!liveSmokeDoesNotProve.has(pendingProof)) {
      push(violations, 'golden_pair_live_smoke_pending_scope_missing', registryPath, `Registry golden_pair_live_smoke must mark ${pendingProof} as not proven by bounded smoke.`);
    }
  }
  for (const marker of [
    'Agent runtime reliability posture',
    'codex_cli',
    'claude_code',
    'completed_response',
    'permission_required',
    'approval_resume_completed',
    'typed_failure',
    'native_direct_mutation_grant',
    'live bounded response smoke',
    'live real-work task proof: pending',
    'external runtime promotion gate is active',
    'catalog-only engines',
  ]) {
    if (!posture.includes(marker)) {
      push(violations, 'agent_runtime_reliability_posture_marker_missing', posturePath, `Reliability posture doc must include marker: ${marker}.`);
    }
  }
  const promotionGate = registry.external_runtime_promotion_gate || {};
  if (promotionGate.status !== 'active' || promotionGate.reference_baseline !== 'golden_pair_baseline' || promotionGate.required_before_reliable_promotion !== true) {
    push(violations, 'external_runtime_promotion_gate_missing', registryPath, 'Registry must keep the external runtime promotion gate active and tied to the golden pair baseline.');
  }
  const promotionBlockedStatuses = new Set(Array.isArray(promotionGate.promotion_blocked_status_values) ? promotionGate.promotion_blocked_status_values : []);
  for (const requiredStatus of ['reliable', 'promoted', 'production_ready', 'baseline_covered', 'golden_pair_equivalent']) {
    if (!promotionBlockedStatuses.has(requiredStatus)) {
      push(violations, 'external_runtime_promotion_gate_status_missing', registryPath, `External runtime promotion gate must block status value ${requiredStatus} without golden-pair-equivalent proof.`);
    }
  }
  const nativeEngine = registry.validation_focus_policy && registry.validation_focus_policy.native_engine || 'infring_native';
  const activePromotionEngines = new Set(Array.isArray(registry.validation_focus_policy && registry.validation_focus_policy.active_promotion_engines) ? registry.validation_focus_policy.active_promotion_engines : []);
  const catalogOnlyEngines = new Set(Array.isArray(registry.validation_focus_policy && registry.validation_focus_policy.catalog_only_engines) ? registry.validation_focus_policy.catalog_only_engines : []);
  for (const engineId of catalogOnlyEngines) {
    if (activePromotionEngines.has(engineId)) {
      push(violations, 'catalog_engine_also_active_promotion_engine', registryPath, `Catalog-only runtime ${engineId} must not also be listed as an active promotion engine.`);
    }
  }
  const engineRows = Array.isArray(registry.engines) ? registry.engines : [];
  for (const row of engineRows) {
    const engineId = String(row && row.engine_id || '');
    if (!engineId || engineId === nativeEngine || goldenPairEngines.has(engineId)) continue;
    const statusValues = [
      row.status,
      row.promotion_status,
      row.reliability_status,
      row.baseline_status,
      row.validation_status,
    ].filter(Boolean).map((value) => String(value));
    const blockedStatus = statusValues.find((value) => promotionBlockedStatuses.has(value));
    if (blockedStatus) {
      const baseline = row.golden_pair_equivalent_baseline || row.primitive_reliability_baseline || {};
      const outcomes = new Set(Array.isArray(baseline.required_terminal_outcomes || baseline.terminal_outcomes) ? baseline.required_terminal_outcomes || baseline.terminal_outcomes : []);
      const primitives = new Set(Array.isArray(baseline.required_shared_primitives || baseline.shared_primitives) ? baseline.required_shared_primitives || baseline.shared_primitives : []);
      const hasAllOutcomes = [...goldenPairOutcomes].every((outcome) => outcomes.has(outcome));
      const hasAllPrimitives = [...goldenPairPrimitives].every((primitive) => primitives.has(primitive));
      if (baseline.status !== 'covered' || !baseline.proof_artifact || !hasAllOutcomes || !hasAllPrimitives) {
        push(violations, 'external_runtime_promoted_without_golden_pair_equivalent_baseline', registryPath, `${engineId} is marked ${blockedStatus} but does not declare covered golden-pair-equivalent primitive proof.`);
      }
    }
  }
  if (codexSandboxMode(grantCtx([])) !== 'read-only') {
    push(violations, 'codex_direct_mutation_default_mapping_wrong', codexPath, 'Codex must map missing Gateway direct mutation grant to read-only sandbox.');
  }
  for (const alias of directGrantAliases) {
    if (codexSandboxMode(grantCtx([alias])) !== 'workspace-write') {
      push(violations, 'codex_direct_mutation_grant_mapping_wrong', codexPath, `Codex must map Gateway direct mutation grant ${alias} to workspace-write sandbox.`);
    }
  }
  const claudeDefaultArgs = claudePermissionArgs(grantCtx([]));
  if (
    !Array.isArray(claudeDefaultArgs) ||
    !claudeDefaultArgs.includes('--permission-mode') ||
    !claudeDefaultArgs.includes('default') ||
    claudeDefaultArgs.includes('acceptEdits') ||
    claudeDefaultArgs.includes('--allowedTools')
  ) {
    push(violations, 'claude_direct_mutation_default_mapping_wrong', claudePath, 'Claude must map missing Gateway direct mutation grant to default permission mode without a write allowlist.');
  }
  const claudeGrantArgs = claudePermissionArgs(grantCtx(['direct_file_write']));
  if (
    !Array.isArray(claudeGrantArgs) ||
    !claudeGrantArgs.includes('--permission-mode') ||
    !claudeGrantArgs.includes('acceptEdits') ||
    !claudeGrantArgs.includes('--allowedTools') ||
    !claudeGrantArgs.includes('Read,Write,Edit,Bash')
  ) {
    push(violations, 'claude_direct_mutation_grant_mapping_wrong', claudePath, 'Claude must map Gateway direct mutation grant to acceptEdits with the bounded write allowlist.');
  }

  if (!claude.includes('nativeDirectMutationGrantActive') || !claude.includes("'--allowedTools'") || !claude.includes('Read,Write,Edit,Bash')) {
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

  const goldenRuntimeTurn = async (engineId, createEngineAdapter) => {
    const goldenProbe = createEngineAdapter({
      command: process.execPath,
      commandFallback: process.execPath,
      liveDispatch: true,
      versionArgs: ['--version'],
      runArgs: () => ['-e', `process.stdout.write(${JSON.stringify(`${adapterProbePayload}\n`)})`],
      promptBuilder: ({ current }) => current,
    });
    return goldenProbe.submit_turn({
      engine: { engine_id: engineId },
      message: {
        trace_id: `validation:agent-runtime-framework-capability-${engineId}-permission`,
        session_id: `framework-capability-${engineId}-permission-session`,
        turn_id: `framework-capability-${engineId}-permission-turn`,
        engine_id: engineId,
        working_directory: ROOT,
        input: { text: `Probe ${engineId} adapter permission pause handling.` },
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
  };
  for (const [engineId, createEngineAdapter] of [
    ['codex_cli', createCodexCliEngineAdapter],
    ['claude_code', createClaudeCodeEngineAdapter],
  ]) {
    const goldenTurn = await goldenRuntimeTurn(engineId, createEngineAdapter);
    if (
      !goldenTurn ||
      goldenTurn.status !== 'permission_required' ||
      !goldenTurn.permission_request ||
      goldenTurn.permission_request.tool_id !== 'artifact.create_propose' ||
      goldenTurn.permission_request.status !== 'paused_pending_approval' ||
      goldenTurn.permission_request.turn_status !== 'permission_required' ||
      goldenTurn.permission_request.resume_strategy !== 'gateway_apply_approved_effect' ||
      !goldenTurn.permission_request.proposal_arguments ||
      goldenTurn.permission_request.proposal_arguments.content !== 'adapter-permission-pause-wins'
    ) {
      push(violations, 'golden_runtime_text_proposal_turn_pause_missing', engineId, `${engineId} must preserve shared CLI adapter permission-pause behavior for text-emitted universal tool proposals.`);
    }
  }

  const goldenRuntimeResumeTurn = async (engineId, createEngineAdapter) => {
    const resumeMarker = `golden-${engineId}-approval-resume-visible`;
    const resumeProbe = createEngineAdapter({
      command: process.execPath,
      commandFallback: process.execPath,
      liveDispatch: true,
      versionArgs: ['--version'],
      runArgs: (prompt) => {
        const payload = JSON.stringify({
          type: 'text',
          part: {
            type: 'text',
            text: `golden resume completed\n${prompt}`,
          },
        });
        return ['-e', `process.stdout.write(${JSON.stringify(`${payload}\n`)})`];
      },
      promptBuilder: (args = {}) => {
        const pack = args.contextPack || args.context_pack || {};
        return [
          args.current || '',
          `approval_resume=${JSON.stringify(pack.approval_resume || {})}`,
          `universal_tool_grants=${JSON.stringify(pack.universal_tool_grants || {})}`,
        ].join('\n');
      },
    });
    const turn = await resumeProbe.submit_turn({
      engine: { engine_id: engineId },
      message: {
        trace_id: `validation:agent-runtime-framework-capability-${engineId}-resume`,
        session_id: `framework-capability-${engineId}-resume-session`,
        turn_id: `framework-capability-${engineId}-resume-turn`,
        engine_id: engineId,
        working_directory: ROOT,
        input: { text: `Continue after approval for ${engineId}.` },
        context_pack: {
          source_authority: 'validation.agent_runtime_framework_capability_guard',
          approval_resume: {
            decision: 'allow',
            decision_ref: resumeMarker,
            tool_id: 'artifact.create_propose',
            resume_strategy: 'grant_then_retry_next_turn',
          },
          universal_tool_grants: {
            tools: ['artifact.create_propose'],
            source_authority: 'validation_agent_runtime_framework_capability_guard',
            permission_policy: {
              always_allowed_tool_calls: ['direct_file_write'],
            },
          },
        },
        capability_budget: {
          max_turn_seconds: 30,
        },
      },
    });
    return { resumeMarker, turn };
  };
  for (const [engineId, createEngineAdapter] of [
    ['codex_cli', createCodexCliEngineAdapter],
    ['claude_code', createClaudeCodeEngineAdapter],
  ]) {
    const { resumeMarker, turn } = await goldenRuntimeResumeTurn(engineId, createEngineAdapter);
    const serializedTurn = JSON.stringify(turn || {});
    if (!turn || turn.status !== 'completed' || !serializedTurn.includes(resumeMarker) || !serializedTurn.includes('direct_file_write')) {
      push(violations, 'golden_runtime_approval_resume_context_missing', engineId, `${engineId} must preserve Gateway approval resume context and direct mutation grant visibility through the shared CLI adapter path.`);
    }
  }

  const goldenRuntimeCompletedTurn = async (engineId, createEngineAdapter) => {
    const finalMarker = `golden-${engineId}-visible-final-response`;
    const finalPayload = JSON.stringify({
      type: 'text',
      part: {
        type: 'text',
        text: finalMarker,
      },
    });
    const finalProbe = createEngineAdapter({
      command: process.execPath,
      commandFallback: process.execPath,
      liveDispatch: true,
      versionArgs: ['--version'],
      runArgs: () => ['-e', `process.stdout.write(${JSON.stringify(`${finalPayload}\n`)})`],
      promptBuilder: ({ current }) => current,
    });
    const turn = await finalProbe.submit_turn({
      engine: { engine_id: engineId },
      message: {
        trace_id: `validation:agent-runtime-framework-capability-${engineId}-completed-response`,
        session_id: `framework-capability-${engineId}-completed-response-session`,
        turn_id: `framework-capability-${engineId}-completed-response-turn`,
        engine_id: engineId,
        working_directory: ROOT,
        input: { text: `Probe ${engineId} completed response projection.` },
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
    return { finalMarker, turn };
  };
  for (const [engineId, createEngineAdapter] of [
    ['codex_cli', createCodexCliEngineAdapter],
    ['claude_code', createClaudeCodeEngineAdapter],
  ]) {
    const { finalMarker, turn } = await goldenRuntimeCompletedTurn(engineId, createEngineAdapter);
    const serializedTurn = JSON.stringify(turn || {});
    if (!turn || turn.status !== 'completed' || !serializedTurn.includes(finalMarker)) {
      push(violations, 'golden_runtime_completed_response_drop', engineId, `${engineId} completed turns must carry visible final response text through the shared CLI adapter path instead of producing a ghost turn.`);
    }
    if (turn && turn.status === 'completed' && (turn.permission_request || turn.error_code)) {
      push(violations, 'golden_runtime_completed_response_terminal_state_conflict', engineId, `${engineId} completed response turns must not also carry permission or failure terminal state fields.`);
    }
  }

  const goldenRuntimeFailureTurn = async (engineId, createEngineAdapter) => {
    const failureMarker = `golden-${engineId}-typed-failure-visible`;
    const failureProbe = createEngineAdapter({
      command: process.execPath,
      commandFallback: process.execPath,
      liveDispatch: true,
      versionArgs: ['--version'],
      runArgs: () => ['-e', `process.stderr.write(${JSON.stringify(`${failureMarker}\n`)}); process.exit(23);`],
      promptBuilder: ({ current }) => current,
    });
    const turn = await failureProbe.submit_turn({
      engine: { engine_id: engineId },
      message: {
        trace_id: `validation:agent-runtime-framework-capability-${engineId}-typed-failure`,
        session_id: `framework-capability-${engineId}-typed-failure-session`,
        turn_id: `framework-capability-${engineId}-typed-failure-turn`,
        engine_id: engineId,
        working_directory: ROOT,
        input: { text: `Probe ${engineId} typed hard failure projection.` },
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
    return { failureMarker, turn };
  };
  for (const [engineId, createEngineAdapter] of [
    ['codex_cli', createCodexCliEngineAdapter],
    ['claude_code', createClaudeCodeEngineAdapter],
  ]) {
    const { failureMarker, turn } = await goldenRuntimeFailureTurn(engineId, createEngineAdapter);
    const serializedTurn = JSON.stringify(turn || {});
    if (
      !turn ||
      turn.status !== 'failed' ||
      !turn.error_code ||
      !serializedTurn.includes(failureMarker) ||
      !serializedTurn.includes('external-cli-failure') ||
      turn.permission_request
    ) {
      push(violations, 'golden_runtime_typed_failure_projection_missing', engineId, `${engineId} hard failures must become typed, visible failed turns instead of dropped or completed ghost turns.`);
    }
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
    source_domain: SOURCE_DOMAIN,
    owner_domain: OWNER_DOMAIN,
    layer: LAYER,
    policy_path: POLICY_PATH,
    golden_pair_baseline: {
      status: violations.some((row) => String(row.kind || '').startsWith('golden_runtime_')) ? 'failed' : 'covered',
      engines: ['codex_cli', 'claude_code'],
      terminal_outcomes: ['completed_response', 'permission_required', 'approval_resume_completed', 'typed_failure'],
      shared_primitives: ['native_direct_mutation_grant', 'text_proposal_pause', 'approval_resume_context', 'visible_final_response', 'typed_failure_projection'],
      proof_scope: 'constructor_backed_fake_cli_turns',
      note: 'This proves the golden external runtime pair preserves shared adapter terminal-state semantics without invoking live provider CLIs.',
    },
    external_runtime_promotion_gate: {
      status: violations.some((row) => String(row.kind || '').startsWith('external_runtime_promotion_gate') || row.kind === 'catalog_engine_also_active_promotion_engine' || row.kind === 'external_runtime_promoted_without_golden_pair_equivalent_baseline') ? 'failed' : 'active',
      reference_baseline: 'golden_pair_baseline',
      blocked_status_values: ['reliable', 'promoted', 'production_ready', 'baseline_covered', 'golden_pair_equivalent'],
      catalog_only_engines: Array.isArray(registry.validation_focus_policy && registry.validation_focus_policy.catalog_only_engines)
        ? registry.validation_focus_policy.catalog_only_engines
        : [],
      rule: 'Non-golden external runtimes may remain selectable/cataloged, but cannot be promoted as reliable without golden-pair-equivalent primitive proof.',
    },
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
      golden_pair_permission_pause: !violations.some((row) => row.kind === 'golden_runtime_text_proposal_turn_pause_missing'),
      golden_pair_approval_resume_context: !violations.some((row) => row.kind === 'golden_runtime_approval_resume_context_missing'),
      golden_pair_completed_response_projection: !violations.some((row) => row.kind === 'golden_runtime_completed_response_drop' || row.kind === 'golden_runtime_completed_response_terminal_state_conflict'),
      golden_pair_typed_failure_projection: !violations.some((row) => row.kind === 'golden_runtime_typed_failure_projection_missing'),
      universal_tool_surface_tiny: !violations.some((row) => String(row.kind || '').startsWith('universal_tool_') || row.kind === 'gateway_universal_tool_ids_block_missing' || row.kind === 'gateway_universal_tool_ids_missing_canonical_tool'),
      native_mutation_not_universalized: !violations.some((row) => row.kind === 'native_mutation_grant_leaked_into_universal_tools' || row.kind === 'universal_tool_scope_allows_native_authority'),
      terminal_execution_not_universalized: !violations.some((row) => row.kind === 'universal_tool_scope_allows_native_authority' || row.kind === 'gateway_universal_tool_scope_not_fail_closed'),
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
    source_domain: SOURCE_DOMAIN,
    owner_domain: OWNER_DOMAIN,
    layer: LAYER,
    policy_path: POLICY_PATH,
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
