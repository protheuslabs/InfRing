# TODO

Updated: 2026-06-06T23:52:39.256Z

## How To Use This File
- This is the live operating board, not the historical ledger.
- Canonical structured data lives in [todo_registry.json](/Users/jay/.openclaw/workspace/docs/workspace/todo/todo_registry.json).
- Completed items must be moved to [TODO_ARCHIVE.md](/Users/jay/.openclaw/workspace/docs/workspace/todo/TODO_ARCHIVE.md) instead of being left here.
- Archive history is rendered from [todo_archive_registry.json](/Users/jay/.openclaw/workspace/docs/workspace/todo/todo_archive_registry.json) and the preserved legacy appendix at [TODO_ARCHIVE_LEGACY.md](/Users/jay/.openclaw/workspace/docs/workspace/todo/TODO_ARCHIVE_LEGACY.md).
- Run manual commands through `npm run -s ops:todo:board -- <command>` so JSON and Markdown stay in sync.
- Every active item must declare `owner` and `deadline`.
- Active items should declare `work_gate` as `real_work`, `reliability`, or `simplification` when possible; `real_work` is the practical TODO gate for the usability law.
- Allowed deadline values: exact date like `2026-05-07`, `none`, `external`, or dependency-shaped values like `after_red_section`.
- Deadline promotion policy: items due in <= 7 days belong in Red; items due in <= 14 days belong in Yellow; everything later stays in White unless manually escalated.

## Rollup
- active_items: 31
- red: 4
- yellow: 19
- white: 8

## Red Section (Do Immediately)
- `SHELL-CLEANUP` — Finish the Shell source-of-truth cleanup
  owner: `unassigned`
  deadline: `2026-05-07`
  source_family: `Shell Cleanup Wave`
  work_gate: `reliability`
  real_work_score: `5`
  summary: Extract remaining authority and source-of-truth responsibilities into Orchestration/Gateway-owned paths while preserving the fragile legacy Shell files until headless replacement paths are proven; do not delete or refactor legacy Shell behavior in this lane.
- `SHPURGE` — Finish the Shell authority purge
  owner: `unassigned`
  deadline: `2026-05-07`
  source_family: `Shell Authority Purge Completion Backlog`
  work_gate: `reliability`
  real_work_score: `5`
  summary: Finish the Shell authority purge so the Shell becomes projection/input only and stops acting like a shadow runtime.
- `ALPINE-PURGE` — Remove the remaining Alpine boot/runtime dependency
  owner: `unassigned`
  deadline: `2026-05-10`
  source_family: `Shell Alpine Purge Wave`
  work_gate: `reliability`
  real_work_score: `5`
  summary: Remove the remaining Alpine boot/runtime dependency once the retirement guard is green.
- `GATEWAY-PHYSICAL-REROOT` — Re-root Gateway boundary code out of adapters
  owner: `codex`
  deadline: `2026-06-10`
  source_family: `Gateway Physical Domain Cleanup`
  work_gate: `reliability`
  real_work_score: `5`
  summary: Move Gateway socket hosts, ingress normalization, payload budgets, permission entrypoints, and runtime route authority into gateway/** while leaving adapters/** as translator-only provider/framework bridges behind Gateway sockets.

## Yellow Section (Do Soon)
- `HYGIENE-RUST-UNUSED-IMPORTS` — Remove unused Rust imports in narrow batches
  owner: `codex`
  deadline: `2026-05-19`
  source_family: `Rust Dead-Code Hygiene Wave`
  work_gate: `simplification`
  real_work_score: `3`
  summary: No unused-import warnings were observed before cargo check stopped on the outside-lane dashboard compat unclosed delimiter. This item is blocked until that compile blocker clears and a complete warning surface can be captured.
- `HYGIENE-COMBINED-DEAD-DELETE` — Delete dead combined artifacts in safe batches
  owner: `codex`
  deadline: `2026-05-20`
  source_family: `Combined Rust Artifact Burn-Down`
  work_gate: `simplification`
  real_work_score: `3`
  summary: Deletion candidates exist in the combined artifact classification, but deletion is blocked until the outside-lane dashboard compat compile blocker is fixed so domain batches can be validated.
- `HYGIENE-COMBINED-DECOMPOSE-LIVE` — Decompose live combined artifact debt
  owner: `codex`
  deadline: `2026-05-20`
  source_family: `Combined Rust Artifact Burn-Down`
  work_gate: `simplification`
  real_work_score: `3`
  summary: Live split-debt artifacts are classified, but decomposition is blocked until the outside-lane dashboard compat compile blocker is fixed so owner-scoped renames can be validated.
- `WF-UTILITY` — Build the workflow utility spine
  owner: `unassigned`
  deadline: `2026-05-20`
  source_family: `Workflow Utility Spine After Shell Purge`
  work_gate: `real_work`
  real_work_score: `5`
  summary: Build the workflow utility spine so the system is useful for real work after Shell de-authority.
- `AGENT-RUNTIME-ACTIVITY-TRACE-PARITY` — Normalize external runtime activity into useful collapsible traces
  owner: `codex`
  deadline: `2026-06-17`
  source_family: `Unified Agent Runtime Framework`
  work_gate: `real_work`
  real_work_score: `5`
  summary: Capture and normalize framework activity streams into concise user-facing traces with real status text, tool calls, file changes, command results, failure events, and post-turn decision dialog, while keeping raw evidence behind refs and payload budgets.
- `AGENT-RUNTIME-ADAPTER-CONTEXT-INJECTION` — Inject shared context into CLI and socket adapters
  owner: `codex`
  deadline: `2026-06-17`
  source_family: `Agent Runtime Context Bridge`
  work_gate: `real_work`
  real_work_score: `5`
  summary: Render the shared envelope into a consistent bounded adapter preamble for Codex, Claude Code, Grok Code, custom socket engines, and native Infring without engine-specific prompt drift.
- `AGENT-RUNTIME-APPROVAL-PAUSE-RESUME` — Make approval gates pause and resume runtime turns
  owner: `codex`
  deadline: `2026-06-17`
  source_family: `Unified Agent Runtime Framework`
  work_gate: `reliability`
  real_work_score: `5`
  summary: When Codex, Claude Code, Grok Code, native Infring, or socket engines hit a gated action, Gateway should create a permission request, pause the turn, wait for the user or future gatekeeper decision, and resume or fail with a durable decision receipt instead of letting the agent finish blocked.
- `AGENT-RUNTIME-CONTEXT-BUILDER` — Build Gateway-owned runtime context assembly
  owner: `codex`
  deadline: `2026-06-17`
  source_family: `Agent Runtime Context Bridge`
  work_gate: `real_work`
  real_work_score: `5`
  summary: Have Gateway assemble bounded conversation windows, summaries, refs, session identity, and context budgets for every runtime turn instead of passing only the latest prompt.
- `AGENT-RUNTIME-CONTEXT-CONTRACT` — Define the shared AgentRuntimeTurnEnvelope contract
  owner: `codex`
  deadline: `2026-06-17`
  source_family: `Agent Runtime Context Bridge`
  work_gate: `reliability`
  real_work_score: `5`
  summary: Create the engine-agnostic context envelope contract so native Infring, Codex, Claude Code, Grok Code, and socket engines receive the same bounded conversation, memory, permission, artifact, and tool-grant shape.
- `AGENT-RUNTIME-CONTEXT-EVAL` — Prove context continuity while switching engines
  owner: `codex`
  deadline: `2026-06-17`
  source_family: `Agent Runtime Context Bridge`
  work_gate: `real_work`
  real_work_score: `5`
  summary: Add live/context-continuity evals that establish conversation facts, switch among native, Codex, Claude Code, and Grok Code, and verify the new engine can code or reason using prior conversation and unified memory.
- `AGENT-RUNTIME-CORE-TOOLS` — Define universal core tools for swappable engines
  owner: `codex`
  deadline: `2026-06-17`
  source_family: `Agent Runtime Context Bridge`
  work_gate: `reliability`
  real_work_score: `5`
  summary: Separate universal tools like conversation read, memory read/write-propose, artifact refs, and permission requests from native-only workflow tools, with Gateway enforcement and receipts.
- `AGENT-RUNTIME-DURABLE-RECEIPTS` — Make external runtime effects receipt-first
  owner: `codex`
  deadline: `2026-06-17`
  source_family: `Unified Agent Runtime Framework`
  work_gate: `reliability`
  real_work_score: `5`
  summary: Ensure every external runtime effect that matters has a durable InfRing receipt, including selected engine/model, context pack ref, approval decision, tool proposal, accepted artifact change, command outcome, final response, and failure classification.
- `AGENT-RUNTIME-ENGINE-SCORECARD` — Create an engine parity and usefulness scorecard
  owner: `codex`
  deadline: `2026-06-17`
  source_family: `Unified Agent Runtime Framework`
  work_gate: `real_work`
  real_work_score: `4`
  summary: Track each runtime engine against the same capabilities: discovery, model catalog, context continuity, useful work, approval pausing, universal tools, durable receipts, activity traces, error injection, and reload persistence.
- `AGENT-RUNTIME-LIVE-WORK-EVALS` — Add live useful-work evals for every selectable runtime
  owner: `codex`
  deadline: `2026-06-17`
  source_family: `Unified Agent Runtime Framework`
  work_gate: `real_work`
  real_work_score: `5`
  summary: Create live evals that ask each selectable runtime to perform practical agent work through InfRing, such as creating a small app, editing a file, reading prior context, using approved tools, and reporting receipts, then score whether it actually worked.
- `AGENT-RUNTIME-MEMORY-BRIDGE` — Bridge unified memory into runtime context
  owner: `codex`
  deadline: `2026-06-17`
  source_family: `Agent Runtime Context Bridge`
  work_gate: `real_work`
  real_work_score: `5`
  summary: Attach bounded relevant memory projections and refs to the runtime envelope so engine switches preserve durable Infring context without exposing raw memory dumps.
- `AGENT-RUNTIME-SOCKET-LIVE` — Make the Agent Runtime Gateway socket live
  owner: `codex`
  deadline: `2026-06-17`
  source_family: `Gateway Physical Domain Cleanup`
  work_gate: `reliability`
  real_work_score: `5`
  summary: Graduate /ws/agent-runtime from contracted_not_live to a live Gateway socket so dashboard, CLI, SDK, and future shells submit turns through the same UI-agnostic boundary instead of adapter-hosted HTTP shims.
- `AGENT-RUNTIME-STRUCTURED-TRANSPORT` — Migrate external engines beyond prompt-text context hydration
  owner: `codex`
  deadline: `2026-06-17`
  source_family: `Unified Agent Runtime Framework`
  work_gate: `reliability`
  real_work_score: `5`
  summary: Move external engines from prompt_text_compat toward structured_json or native session bridges where available, preserving the same AgentRuntimeTurnEnvelope while reducing token waste, ambiguity, duplicate transcript rows, and prompt-layer drift.
- `AGENT-RUNTIME-UNIFIED-ERROR-INJECTION` — Inject hard runtime failures into chat as first-class messages
  owner: `codex`
  deadline: `2026-06-17`
  source_family: `Unified Agent Runtime Framework`
  work_gate: `reliability`
  real_work_score: `4`
  summary: When an external framework fails from auth, quota, subscription, missing binary, timeout, payload budget, or transport failure, Gateway should emit a bounded chat-visible failure message plus trace refs and next actions instead of silently ending the turn.
- `INFRING-NATIVE-RUNTIME-PARITY` — Make InfRing Native obey the same runtime abstraction as external engines
  owner: `codex`
  deadline: `2026-06-17`
  source_family: `Unified Agent Runtime Framework`
  work_gate: `reliability`
  real_work_score: `5`
  summary: Route InfRing Native through the same engine_id, context envelope, available_models, activity event, permission request, artifact ref, and receipt surfaces used by external engines so native orchestration is swappable rather than special-cased.

## White Section (Do At Leisure)
- `TRACE-IMPL` — Implement end-to-end unified trace_id propagation
  owner: `unassigned`
  deadline: `2026-05-23`
  source_family: `Universal Trace Runtime Implementation Checklist`
  work_gate: `reliability`
  real_work_score: `4`
  summary: Implement end-to-end unified trace_id propagation from initial request through Orchestration, workflows, tools, Kernel receipts, Sentinel, and final response.
- `SHELL-NEXT` — Build Shell-next only after authority extraction proofs
  owner: `unassigned`
  deadline: `after_red_section`
  source_family: `Shell Authority Extraction Without Legacy Shell Mutation`
  work_gate: `real_work`
  real_work_score: `3`
  summary: Start the clean Shell-next route as a projection/input-only UI after the headless capability matrix and high-risk authority extraction routes are proven.
- `SVELTE` — Keep remaining Svelte migration subordinate to Shell authority purge
  owner: `unassigned`
  deadline: `after_red_section`
  source_family: `Chat Dashboard Alpine to Svelte Migration Wave`
  work_gate: `simplification`
  real_work_score: `2`
  summary: Remaining Svelte migration and memory profiling items are worth doing, but they are subordinate to the broader Shell authority purge.
- `EXTERNAL-BLOCKERS` — Keep externally blocked work parked in the archive
  owner: `unassigned`
  deadline: `external`
  source_family: `External Blockers`
  work_gate: `reliability`
  real_work_score: `1`
  summary: External blockers from the previous ledger remain parked in the archive until the required evidence packets or human approvals exist.
- `ASSIMILATION-LONGHORIZON` — Park longer-horizon assimilation and runtime work
  owner: `unassigned`
  deadline: `none`
  source_family: `Long-Horizon Assimilation and Runtime Work`
  work_gate: `real_work`
  real_work_score: `2`
  summary: Assimilation and longer-horizon runtime work can stay parked here until the red and yellow closure work is materially better.
- `BACKLOG-PARKED` — Keep lower-pressure valid backlog items parked
  owner: `unassigned`
  deadline: `none`
  source_family: `Parked Valid Backlog Items`
  work_gate: `simplification`
  real_work_score: `1`
  summary: These are still valid backlog items, but they are not the current forcing function.
- `OS` — Keep Layer 3 and OS-readiness work parked behind current closure work
  owner: `unassigned`
  deadline: `none`
  source_family: `OS-Readiness Wave - Layer 3 to True OS Migration`
  work_gate: `real_work`
  real_work_score: `2`
  summary: Layer 3 and OS-readiness work remains important, but it should stay behind the current Shell, workflow, and trace closure push.
- `TAURI-UI` — Keep Tauri migration strategically queued
  owner: `unassigned`
  deadline: `none`
  source_family: `Tauri Desktop App Migration + Memory Fix Wave`
  work_gate: `real_work`
  real_work_score: `2`
  summary: Tauri migration remains strategically valuable, but it should not outrun the deeper architecture cleanup already underway.

## Archive Rule
- When an item in this file is completed, remove it from this live board and append it to [TODO_ARCHIVE.md](/Users/jay/.openclaw/workspace/docs/workspace/todo/TODO_ARCHIVE.md) through the scripted flow.
- Do not let completed rows accumulate here again.
- Treat Markdown as a rendered operator surface, not the canonical mutation target.
