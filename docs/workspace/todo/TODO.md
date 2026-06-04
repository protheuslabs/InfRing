# TODO

Updated: 2026-06-03T15:18:53.667Z

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
- active_items: 21
- red: 3
- yellow: 10
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
- `AGENT-RUNTIME-ADAPTER-CONTEXT-INJECTION` — Inject shared context into CLI and socket adapters
  owner: `codex`
  deadline: `2026-06-17`
  source_family: `Agent Runtime Context Bridge`
  work_gate: `real_work`
  real_work_score: `5`
  summary: Render the shared envelope into a consistent bounded adapter preamble for Codex, Claude Code, Grok Code, custom socket engines, and native Infring without engine-specific prompt drift.
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
- `AGENT-RUNTIME-MEMORY-BRIDGE` — Bridge unified memory into runtime context
  owner: `codex`
  deadline: `2026-06-17`
  source_family: `Agent Runtime Context Bridge`
  work_gate: `real_work`
  real_work_score: `5`
  summary: Attach bounded relevant memory projections and refs to the runtime envelope so engine switches preserve durable Infring context without exposing raw memory dumps.

- `AGENT-RUNTIME-STRUCTURED-TRANSPORT` — Define structured runtime transport modes
  owner: `codex`
  deadline: `2026-06-24`
  source_family: `Agent Runtime Structured Transport Migration`
  work_gate: `reliability`
  real_work_score: `5`
  summary: Define prompt_text_compat, structured_json, and native_session_bridge modes so prompt-text remains bootstrap-only and every adapter declares its migration target.
- `AGENT-RUNTIME-PROMPT-TEXT-GUARD` — Guard prompt-text compatibility as transitional
  owner: `codex`
  deadline: `2026-06-24`
  source_family: `Agent Runtime Structured Transport Migration`
  work_gate: `simplification`
  real_work_score: `4`
  summary: Extend conformance so prompt-text hacks require explicit transitional markers, structured targets, and migration reporting.
- `AGENT-RUNTIME-TRANSPORT-MIGRATION-REPORT` — Track engines still using prompt text
  owner: `codex`
  deadline: `2026-06-24`
  source_family: `Agent Runtime Structured Transport Migration`
  work_gate: `reliability`
  real_work_score: `4`
  summary: Publish an adapter migration report showing current transport mode, target mode, and whether each engine is prompt-text-dependent.
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

