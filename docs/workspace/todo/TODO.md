# TODO

Updated: 2026-06-02T18:15:11.445Z

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
- active_items: 29
- red: 6
- yellow: 14
- white: 9

## Red Section (Do Immediately)
- `SHELL-CLEANUP` — Finish the Shell source-of-truth cleanup
  owner: `unassigned`
  deadline: `2026-05-07`
  source_family: `Shell Cleanup Wave`
  work_gate: `reliability`
  real_work_score: `5`
  summary: Extract remaining authority and source-of-truth responsibilities into Orchestration/Gateway-owned paths while preserving the fragile legacy Shell files until headless replacement paths are proven; do not delete or refactor legacy Shell behavior in this lane.
- `SHELL-EXTRACT-CHAT` — Move chat and session projections behind Gateway
  owner: `unassigned`
  deadline: `2026-05-07`
  source_family: `Shell Authority Extraction Without Legacy Shell Mutation`
  work_gate: `reliability`
  real_work_score: `5`
  summary: Make chat/session list and message-window loading available as bounded Gateway projections backed by Orchestration/headless proof, so the system can run without the legacy Shell while leaving existing Shell behavior untouched.
- `SHELL-EXTRACT-COMMANDS` — Move interactive command authority out of Shell
  owner: `unassigned`
  deadline: `2026-05-07`
  source_family: `Shell Authority Extraction Without Legacy Shell Mutation`
  work_gate: `reliability`
  real_work_score: `5`
  summary: Route slash commands, model switching, model failover, prompt queue execution, and terminal execution through typed Orchestration/Gateway ingress receipts; preserve legacy Shell helpers as compatibility callers until the new path is proven.
- `SHELL-EXTRACT-DETAILS` — Move message and tool details behind lazy routes
  owner: `unassigned`
  deadline: `2026-05-07`
  source_family: `Shell Authority Extraction Without Legacy Shell Mutation`
  work_gate: `reliability`
  real_work_score: `5`
  summary: Expose message, tool, artifact, trace, and workflow detail fetches by stable refs through bounded audited Gateway/Orchestration routes, without removing legacy Shell rendering or local compatibility paths yet.
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
- `SHELL-EXTRACT-CACHE` — Replace Shell conversation cache and search with projections
  owner: `unassigned`
  deadline: `2026-05-14`
  source_family: `Shell Authority Extraction Without Legacy Shell Mutation`
  work_gate: `reliability`
  real_work_score: `4`
  summary: Move conversation cache/search to bounded preview/index routes so the browser keeps only cursors, previews, counts, and refs rather than cloned full messages or raw tool payloads.
- `SHELL-EXTRACT-EVAL` — Move report/eval issue flow behind Gateway ingress
  owner: `unassigned`
  deadline: `2026-05-14`
  source_family: `Shell Authority Extraction Without Legacy Shell Mutation`
  work_gate: `reliability`
  real_work_score: `4`
  summary: Make chat-local report issue send bounded refs/context through Gateway to Orchestration eval without Shell-owned policy, GitHub submission, or raw context upload.
- `SHELL-EXTRACT-HEALTH` — Make health and connectivity purely Gateway projected
  owner: `unassigned`
  deadline: `2026-05-14`
  source_family: `Shell Authority Extraction Without Legacy Shell Mutation`
  work_gate: `reliability`
  real_work_score: `4`
  summary: Make runtime health, connectivity, release status, and degraded-state labels come from bounded Gateway status projections so Shell does not infer readiness or failure truth.
- `SHELL-EXTRACT-LIFECYCLE` — Move agent and session lifecycle to receipts
  owner: `unassigned`
  deadline: `2026-05-14`
  source_family: `Shell Authority Extraction Without Legacy Shell Mutation`
  work_gate: `reliability`
  real_work_score: `4`
  summary: Ensure create, select, archive, reset, retry, and new-agent initialization flows work through Gateway/Core/Orchestration receipts with headless proof.
- `SHELL-EXTRACT-WORKFLOW` — Project workflow stage and thought status from Orchestration
  owner: `unassigned`
  deadline: `2026-05-14`
  source_family: `Shell Authority Extraction Without Legacy Shell Mutation`
  work_gate: `reliability`
  real_work_score: `4`
  summary: Make Orchestration emit workflow_stage_label and workflow_thought_preview projections so thinking bubbles render owner-provided status instead of Shell-authored fallback text.
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
- `ARTENG-GATEWAY-ROUTER` — Add Gateway agent runtime router seam
  owner: `codex`
  deadline: `2026-06-20`
  source_family: `Agent Runtime Engine Socket Architecture`
  work_gate: `reliability`
  real_work_score: `5`
  summary: Add a Gateway-side router abstraction that selects an engine_id and dispatches through the normalized agent runtime contract without letting Shell or external engines call Orchestration or Kernel directly.
- `ARTENG-INFRING-NATIVE` — Wrap Orchestration as infring_native runtime engine
  owner: `codex`
  deadline: `2026-06-23`
  source_family: `Agent Runtime Engine Socket Architecture`
  work_gate: `real_work`
  real_work_score: `5`
  summary: Expose orchestration/** as engine_id infring_native with the same start_session, submit_turn, stream_events, cancel, collect_artifacts, health_check, and emit_receipts contract used by external engines; do not physically restructure orchestration yet.
- `ARTENG-EXTERNAL-ADAPTER-FIRST` — Implement first external agent runtime adapter
  owner: `codex`
  deadline: `2026-06-27`
  source_family: `Agent Runtime Engine Socket Architecture`
  work_gate: `real_work`
  real_work_score: `5`
  summary: Implement one bounded external adapter, preferably codex_cli or claude_code, behind the Gateway runtime router; normalize its stream into assistant deltas, tool projections, artifact refs, receipts, errors, and final result events.
- `ARTENG-TRACE-OBSERVABILITY` — Record normalized agent runtime traces
  owner: `codex`
  deadline: `2026-06-27`
  source_family: `Agent Runtime Engine Socket Architecture`
  work_gate: `reliability`
  real_work_score: `5`
  summary: Add Observability evidence for engine turns: engine_id, trace_id, session_id, health, latency, failure class, tool proposal refs, artifact refs, receipt refs, and final status, without leaking raw external framework payloads.
- `ARTENG-CONFORMANCE` — Add agent runtime engine conformance checks
  owner: `codex`
  deadline: `2026-06-30`
  source_family: `Agent Runtime Engine Socket Architecture`
  work_gate: `reliability`
  real_work_score: `5`
  summary: Add Validation checks for registry shape, socket event schema, trace propagation, cancellation, bounded payloads, receipt emission, capability-denial behavior, and artifact/detail-ref handling across infring_native and external adapters.

## White Section (Do At Leisure)
- `TRACE-IMPL` — Implement end-to-end unified trace_id propagation
  owner: `unassigned`
  deadline: `2026-05-23`
  source_family: `Universal Trace Runtime Implementation Checklist`
  work_gate: `reliability`
  real_work_score: `4`
  summary: Implement end-to-end unified trace_id propagation from initial request through Orchestration, workflows, tools, Kernel receipts, Sentinel, and final response.
- `ARTENG-DASHBOARD-TOGGLE` — Add dashboard runtime-engine toggle through Gateway only
  owner: `codex`
  deadline: `after_agent_runtime_router`
  source_family: `Agent Runtime Engine Socket Architecture`
  work_gate: `real_work`
  real_work_score: `4`
  summary: Add the UI selector only after the Gateway socket/router and at least one engine adapter are proven; Shell sends engine_id and displays normalized projections only, with no engine-specific execution logic.
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

