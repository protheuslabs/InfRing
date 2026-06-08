# TODO Archive

Updated: 2026-06-07T00:02:27.302Z

## How To Use This File
- This is the historical ledger for completed work, not the live queue.
- Canonical structured data lives in [todo_archive_registry.json](/Users/jay/.openclaw/workspace/docs/workspace/todo/todo_archive_registry.json).
- The pre-JSON historical snapshot remains preserved at [TODO_ARCHIVE_LEGACY.md](/Users/jay/.openclaw/workspace/docs/workspace/todo/TODO_ARCHIVE_LEGACY.md).

## Historical Snapshot
- total_rows: 4145
- queued: 192
- in_progress: 10
- blocked: 2
- blocked_external_prepared: 32
- done: 1616
- existing_coverage_validated: 2302

## Scripted Completion Archive
- `AGENT-RUNTIME-APPROVAL-PAUSE-RESUME` — Make approval gates pause and resume runtime turns
  completed_at: `2026-06-07T00:02:27.301Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-06-17`
  source_family: `Unified Agent Runtime Framework`
  summary: When Codex, Claude Code, Grok Code, native Infring, or socket engines hit a gated action, Gateway should create a permission request, pause the turn, wait for the user or future gatekeeper decision, and resume or fail with a durable decision receipt instead of letting the agent finish blocked.
  completion_note: Gateway approval turns now project permission_required pauses with resume tokens; approval decisions emit bounded decision receipts and resolve stored requests via ops:agent-runtime:approval-pause-resume:guard.
- `SYSTEM-ACTION-AUTHORITY-RECEIPTS` — Replace dashboard system-action fallback with Core receipts
  completed_at: `2026-06-06T23:52:39.253Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-06-17`
  source_family: `Gateway Physical Domain Cleanup`
  summary: Ensure restart, shutdown, and update routes are Gateway-wrapped but Core/ops-authoritative, returning deterministic receipts or receipt refs while retiring dashboard-host detached subprocess fallback glue.
  completion_note: Gateway system-action routes now fail closed for successful mutating restart/shutdown/applied-update responses without receipts; dry-run update projects receipt_required=false; ops:gateway:system-action-authority:guard passes.
- `ADAPTER-TRANSLATOR-ONLY-GUARD` — Enforce adapters as translator-only
  completed_at: `2026-06-06T23:46:00.854Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-06-17`
  source_family: `Gateway Physical Domain Cleanup`
  summary: Strengthen conformance so adapters/** may parse private provider/framework protocols but cannot own Gateway policy, Shell-facing sockets, permission policy, payload budgets, route admission, or runtime authority.
  completion_note: Added adapter translator-only policy and guard; npm run -s ops:adapter:translator-only:guard passes with zero violations.
- `SHELL-EXTRACT-WORKFLOW` — Project workflow stage and thought status from Orchestration
  completed_at: `2026-06-02T20:01:41.205Z`
  previous_section: `yellow`
  owner: `unassigned`
  deadline: `2026-05-14`
  source_family: `Shell Authority Extraction Without Legacy Shell Mutation`
  summary: Make Orchestration emit workflow_stage_label and workflow_thought_preview projections so thinking bubbles render owner-provided status instead of Shell-authored fallback text.
  completion_note: Extended the explicit Gateway shell-socket projection bridge to /api/shell-socket/sessions/{session_id}/events with bounded cursor/limit shaping and trace/projection metadata, giving Shell-next a Gateway-owned workflow/event visibility path without legacy Shell-authored status fallback changes.
- `SHELL-EXTRACT-CACHE` — Replace Shell conversation cache and search with projections
  completed_at: `2026-06-02T20:01:08.622Z`
  previous_section: `yellow`
  owner: `unassigned`
  deadline: `2026-05-14`
  source_family: `Shell Authority Extraction Without Legacy Shell Mutation`
  summary: Move conversation cache/search to bounded preview/index routes so the browser keeps only cursors, previews, counts, and refs rather than cloned full messages or raw tool payloads.
  completion_note: Extended the explicit Gateway shell-socket projection bridge to /api/shell-socket/search with bounded q/agent/scope/cursor/limit shaping. This gives Shell-next/cache replacement a Gateway-owned preview/search path without touching legacy Shell conversation cache code.
- `SHELL-EXTRACT-LIFECYCLE` — Move agent and session lifecycle to receipts
  completed_at: `2026-06-02T19:59:37.064Z`
  previous_section: `yellow`
  owner: `unassigned`
  deadline: `2026-05-14`
  source_family: `Shell Authority Extraction Without Legacy Shell Mutation`
  summary: Ensure create, select, archive, reset, retry, and new-agent initialization flows work through Gateway/Core/Orchestration receipts with headless proof.
  completion_note: Added Gateway shell-socket lifecycle ingress bridge for agent/session create, archive, revive, clone, stop, history clear, session switch, and delete routes. The bridge bounds request bodies, adds trace/receipt metadata, forwards to Rust shell-socket authority, and does not modify legacy Shell files.
- `SHELL-EXTRACT-EVAL` — Move report/eval issue flow behind Gateway ingress
  completed_at: `2026-06-02T19:58:10.059Z`
  previous_section: `yellow`
  owner: `unassigned`
  deadline: `2026-05-14`
  source_family: `Shell Authority Extraction Without Legacy Shell Mutation`
  summary: Make chat-local report issue send bounded refs/context through Gateway to Orchestration eval without Shell-owned policy, GitHub submission, or raw context upload.
  completion_note: Added Gateway-only shell-socket eval issue ingress for /api/shell-socket/issues. It forwards bounded summaries/refs to Rust shell-socket authority, strips raw transcript/context/trace/workflow fields, reports blocked_raw_fields, and leaves legacy Shell behavior untouched.
- `SHELL-EXTRACT-HEALTH` — Make health and connectivity purely Gateway projected
  completed_at: `2026-06-02T19:53:28.880Z`
  previous_section: `yellow`
  owner: `unassigned`
  deadline: `2026-05-14`
  source_family: `Shell Authority Extraction Without Legacy Shell Mutation`
  summary: Make runtime health, connectivity, release status, and degraded-state labels come from bounded Gateway status projections so Shell does not infer readiness or failure truth.
  completion_note: Added explicit Gateway shell-socket runtime-status projection with trace/projection metadata and bounded Gateway fallback; Shell consumers can render health/connectivity from Gateway status instead of local inference.
- `SHELL-EXTRACT-COMMANDS` — Move interactive command authority out of Shell
  completed_at: `2026-06-02T19:52:24.797Z`
  previous_section: `red`
  owner: `unassigned`
  deadline: `2026-05-07`
  source_family: `Shell Authority Extraction Without Legacy Shell Mutation`
  summary: Route slash commands, model switching, model failover, prompt queue execution, and terminal execution through typed Orchestration/Gateway ingress receipts; preserve legacy Shell helpers as compatibility callers until the new path is proven.
  completion_note: Added explicit Gateway shell-socket command ingress bridge for submit input/message, model selection, and terminal command routes; requests are bounded and trace-tagged before forwarding to Rust shell-socket authority, with legacy Shell behavior left untouched.
- `SHELL-EXTRACT-DETAILS` — Move message and tool details behind lazy routes
  completed_at: `2026-06-02T19:50:45.205Z`
  previous_section: `red`
  owner: `unassigned`
  deadline: `2026-05-07`
  source_family: `Shell Authority Extraction Without Legacy Shell Mutation`
  summary: Expose message, tool, artifact, trace, and workflow detail fetches by stable refs through bounded audited Gateway/Orchestration routes, without removing legacy Shell rendering or local compatibility paths yet.
  completion_note: Extended the explicit Gateway shell-socket projection bridge to detail fetches under /api/shell-socket/details/* with bounded limit/view shaping, trace/projection metadata, and Rust shell-socket as owner of truth; legacy Shell files were not modified.
- `SHELL-EXTRACT-CHAT` — Move chat and session projections behind Gateway
  completed_at: `2026-06-02T19:26:54.007Z`
  previous_section: `red`
  owner: `unassigned`
  deadline: `2026-05-07`
  source_family: `Shell Authority Extraction Without Legacy Shell Mutation`
  summary: Make chat/session list and message-window loading available as bounded Gateway projections backed by Orchestration/headless proof, so the system can run without the legacy Shell while leaving existing Shell behavior untouched.
  completion_note: Added explicit Node Gateway shell-socket chat/session projection bridge for agent roster, session list, and bounded message windows; Rust shell-socket remains the projection owner, Gateway caps query shape and adds trace/projection metadata, and legacy Shell files were not modified.
- `ARTENG-DASHBOARD-TOGGLE` — Add dashboard runtime-engine toggle through Gateway only
  completed_at: `2026-06-02T18:52:02.834Z`
  previous_section: `white`
  owner: `codex`
  deadline: `after_agent_runtime_router`
  source_family: `Agent Runtime Engine Socket Architecture`
  summary: Add a row in the input-row hamburger menu alongside the LLM menu. It opens a similar menu for agent runtimes backed by /ws/agent-runtime: InfRing Native, Codex CLI, Claude Code, OpenHands, OpenClaw, OpenFang, and custom socket engines. The Shell sends only engine_id. Missing runtimes show a download/configure icon using registry install metadata; later the icon should prefer command-line install/recover actions and fall back to opening the provider download page if no command path is available.
  completion_note: Added Gateway-projected agent runtime availability and a legacy-styled input hamburger runtime selector that carries selected engine_id through direct and queued sends.
- `ARTENG-CONFORMANCE` — Add agent runtime engine conformance checks
  completed_at: `2026-06-02T18:43:32.961Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-06-30`
  source_family: `Agent Runtime Engine Socket Architecture`
  summary: Add Validation checks for registry shape, socket event schema, trace propagation, cancellation, bounded payloads, receipt emission, capability-denial behavior, and artifact/detail-ref handling across infring_native and external adapters.
  completion_note: Added agent runtime engine conformance guard and package script covering registry shape, socket route, trace requirements, Kernel authority invariants, forbidden default payload fields, install metadata, and adapter exports.
- `ARTENG-TRACE-OBSERVABILITY` — Record normalized agent runtime traces
  completed_at: `2026-06-02T18:43:32.337Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-06-27`
  source_family: `Agent Runtime Engine Socket Architecture`
  summary: Add Observability evidence for engine turns: engine_id, trace_id, session_id, health, latency, failure class, tool proposal refs, artifact refs, receipt refs, and final status, without leaking raw external framework payloads.
  completion_note: Added compact agent runtime trace writer and wired the router to emit Observability-owned engine events keyed by trace_id, engine_id, session_id, turn_id, refs, status, and error metadata.
- `ARTENG-EXTERNAL-ADAPTER-FIRST` — Implement first external agent runtime adapter
  completed_at: `2026-06-02T18:43:31.555Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-06-27`
  source_family: `Agent Runtime Engine Socket Architecture`
  summary: Implement one bounded external adapter, preferably codex_cli or claude_code, behind the Gateway runtime router; normalize its stream into assistant deltas, tool projections, artifact refs, receipts, errors, and final result events.
  completion_note: Added codex_cli as the first bounded external agent runtime adapter seam with local command health probing, normalized adapter methods, opt-in live dispatch, refs/previews for outputs, and install/download metadata in the engine registry.
- `ARTENG-INFRING-NATIVE` — Wrap Orchestration as infring_native runtime engine
  completed_at: `2026-06-02T18:33:52.769Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-06-23`
  source_family: `Agent Runtime Engine Socket Architecture`
  summary: Expose orchestration/** as engine_id infring_native with the same start_session, submit_turn, stream_events, cancel, collect_artifacts, health_check, and emit_receipts contract used by external engines; do not physically restructure orchestration yet.
  completion_note: Wrapped orchestration/** as engine_id infring_native through a Gateway adapter seam that implements the normalized runtime-engine interface and reports not_connected until a live orchestration client is attached; added router streamEvents support.
- `ARTENG-GATEWAY-ROUTER` — Add Gateway agent runtime router seam
  completed_at: `2026-06-02T18:18:44.749Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-06-20`
  source_family: `Agent Runtime Engine Socket Architecture`
  summary: Add a Gateway-side router abstraction that selects an engine_id and dispatches through the normalized agent runtime contract without letting Shell or external engines call Orchestration or Kernel directly.
  completion_note: Added standalone Gateway agent runtime router seam with registry loading, engine_id selection, adapter registration, trace_id enforcement, normalized error events, bounded default payload checks, and forbidden raw/default field rejection.
- `ARTENG-REGISTRY` — Create agent runtime engine registry
  completed_at: `2026-06-02T18:15:11.444Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-06-16`
  source_family: `Agent Runtime Engine Socket Architecture`
  summary: Create the canonical engine registry with infring_native plus planned external engines such as claude_code, codex_cli, openhands, openclaw, openfang, and custom_socket_engine; include capabilities, transport kind, health contract, and authority constraints.
  completion_note: Created canonical agent runtime engine registry declaring infring_native plus planned codex_cli, claude_code, openhands, openclaw, openfang, and custom_socket_engine adapters with interface, transport, health, capability, and authority constraints.
- `ARTENG-SOCKET-CONTRACT` — Define canonical Gateway agent runtime socket contract
  completed_at: `2026-06-02T18:15:10.578Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-06-16`
  source_family: `Agent Runtime Engine Socket Architecture`
  summary: Define the SRS-backed Gateway-owned /ws/agent-runtime protocol: engine discovery, health, session start, turn submission, cancel, assistant deltas, tool projections, artifact refs, errors, receipts, heartbeats, and turn completion; require unified trace_id and ref-only heavy payloads.
  completion_note: Defined canonical Gateway-owned /ws/agent-runtime socket contract with required events, trace_id invariant, forbidden raw/default payload fields, Gateway enforcement duties, Kernel authority invariant, and Observability trace hook.
- `KSENT-EMPTY-RESPONSE-HARNESS` — Fix recurring empty assistant response harness failures
  completed_at: `2026-05-11T01:47:42.966Z`
  previous_section: `red`
  owner: `codex`
  deadline: `2026-05-10`
  source_family: `Kernel Sentinel Current Findings`
  summary: Investigate synthetic_user_chat_harness empty_assistant_response failures for greeting, workspace-file, and web-tool scenarios; identify whether the cause is finalization, projection, routing, or Shell/Gateway boundary, then add a bounded regression proof.
  completion_note: Reconciled the Sentinel empty-response repair lane to the live TODO id, added live synthetic harness evidence refs, and validated ops:ksent:empty-response-repair:guard with zero violations.
- `KSENT-AUTO-TIMEOUT-FRESH-RUN` — Fix Kernel Sentinel auto-run timeout
  completed_at: `2026-05-10T21:12:29.626Z`
  previous_section: `red`
  owner: `codex`
  deadline: `2026-05-10`
  source_family: `Kernel Sentinel Fresh Evidence Guard`
  summary: Current Sentinel auto-run artifact reports kernel_sentinel_auto_timeout, but a bounded retry did not reach Sentinel runtime because cargo compilation stops on an unrelated dashboard compat unclosed delimiter in 045-tool-recovery-and-turn-persistence.rs. Keep open until that outside-lane compile blocker is fixed, then rerun the bounded Sentinel auto command to prove fresh deterministic evidence can be produced without hanging.
  completion_note: Added a bounded lightweight Sentinel maintenance observation path for tight non-strict auto runs; exact 30s command now writes fresh ok artifact and ops:ksent:fresh-evidence:guard passes without promoting stale final reports.
- `SRS-NEXT` — Queue the next SRS stream after the red section
  completed_at: `2026-05-09T23:53:26.601Z`
  previous_section: `yellow`
  owner: `unassigned`
  deadline: `after_red_section`
  source_family: `Actionable SRS Items (Queued/In Progress)`
  summary: Queue the next SRS stream after the red intake set so the active SRS flow stays coherent instead of fragmenting into too many parallel themes.
  completion_note: Added deferred-work queue policy and strict guard so the next SRS stream waits behind red closure work unless explicitly overridden.
- `ARCH-TOOLING-NEXT` — Hold important architecture and tooling deltas behind current closure work
  completed_at: `2026-05-09T23:53:26.309Z`
  previous_section: `yellow`
  owner: `unassigned`
  deadline: `after_red_section`
  source_family: `Architecture and Tooling Follow-On Wave`
  summary: Important architecture and tooling deltas should follow the Shell, workflow, and trace closure work rather than compete with it immediately.
  completion_note: Added deferred-work queue policy and strict guard so architecture/tooling deltas stay behind red closure work unless explicitly overridden.
- `HYGIENE-RUST-UNUSED-HELPERS` — Triage unused Rust helper functions
  completed_at: `2026-05-09T23:46:05.468Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-05-19`
  source_family: `Rust Dead-Code Hygiene Wave`
  summary: For unused private helpers, decide whether each should be deleted, reconnected to the intended path, moved behind tests, or marked as intentional compatibility surface with a documented reason.
  completion_note: Added unused-helper triage from partial cargo baseline with owner families and reconnect/delete recommendations; full cleanup waits for compile blocker clearance.
- `HYGIENE-RUST-DEADCODE-GUARD` — Add Rust dead-code trend guard
  completed_at: `2026-05-09T23:45:27.067Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-05-19`
  source_family: `Rust Dead-Code Hygiene Wave`
  summary: Add or extend a validation report that tracks Rust unused/dead-code warning counts by crate so the surface trends down instead of silently growing.
  completion_note: Added rust hygiene baseline policy and strict guard tracking combined classification and partial dead-code warning baseline until compile blocker clears.
- `HYGIENE-RUST-DEADCODE-BASELINE` — Capture Rust dead-code warning baseline
  completed_at: `2026-05-09T23:45:26.733Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-05-19`
  source_family: `Rust Dead-Code Hygiene Wave`
  summary: Capture the current cargo dead-code and unused-warning surface by crate, module, warning kind, and likely action so future cleanup can happen in safe rollback-sized batches.
  completion_note: Generated guarded Rust dead-code warning baseline; baseline is explicitly partial because cargo check is blocked by outside-lane dashboard compat unclosed delimiter after 23 warning lines.
- `HYGIENE-RUST-COMBINED-SPLIT-DEBT` — Classify combined Rust split artifacts
  completed_at: `2026-05-09T23:45:26.428Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-05-19`
  source_family: `Rust Dead-Code Hygiene Wave`
  summary: Classify 000-combined.rs and .combined_parts warning-heavy files as generated split debt, live source, or deletion candidates before attempting broad cleanup.
  completion_note: Generated guarded 472-artifact combined split-debt classification with live/deletion-candidate/policy/validation classes.
- `SRS-IA-CONSOLIDATION` — Consolidate IA intake lanes safely
  completed_at: `2026-05-09T23:44:06.320Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-05-18`
  source_family: `SRS Active Triage 2026-05-09`
  summary: Merge Manage IA, Automation IA, and System IA Consolidation intake into one governance-safe information-architecture lane with owner, evidence, and expiry.
  completion_note: Closed through SRS intake closure manifest and guard consolidating Manage/Automation/System IA into one governance/documentation/Validation lane.
- `SRS-KG-QUERY-ACCELERATION` — Triage knowledge graph acceleration SRS intake
  completed_at: `2026-05-09T23:44:06.017Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-05-18`
  source_family: `SRS Active Triage 2026-05-09`
  summary: Break Knowledge Graph Query Acceleration Intake into Kernel/memory/runtime-owned capability and evidence tasks, excluding orchestration and Shell changes from this wave.
  completion_note: Closed through SRS intake closure manifest and guard with Kernel/memory/runtime owner lane and boundedness/correctness criteria.
- `SRS-VERSION-CLI-RELIABILITY` — Close version update CLI reliability SRS intake
  completed_at: `2026-05-09T23:44:05.717Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-05-18`
  source_family: `SRS Active Triage 2026-05-09`
  summary: Convert Version Update CLI Reliability Intake into installer/release-governance evidence and bounded CLI behavior checks without touching orchestration or legacy Shell.
  completion_note: Closed through SRS intake closure manifest and guard with installer/release-governance/Validation owner lane and future acceptance criteria.
- `SRS-FILE-READ-RELIABILITY` — Close file-read reliability SRS intake
  completed_at: `2026-05-09T23:44:05.382Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-05-18`
  source_family: `SRS Active Triage 2026-05-09`
  summary: Convert the File Read Reliability Intake in TODO_SRS_SYNC into Kernel/Validation-owned evidence and closure criteria without touching orchestration or legacy Shell.
  completion_note: Closed through SRS intake closure manifest and guard with Kernel/runtime/Validation owner lane and future acceptance criteria.
- `RWF-NO-NEW-SUBSYSTEM` — Require new subsystems to prove real-work value first
  completed_at: `2026-05-09T23:42:55.177Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-05-18`
  source_family: `Real Work First`
  summary: Before adding a new subsystem, require a concrete user-facing or agent-facing workflow it improves, the reliability risk it reduces, and the simpler alternative considered.
  completion_note: Added strict TODO intake enforcement so new active subsystem work must carry a work gate and score instead of entering as ungated expansion.
- `RWF-SIMPLIFICATION-FILTER` — Add a simplification filter to TODO intake
  completed_at: `2026-05-09T23:42:54.879Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-05-18`
  source_family: `Real Work First`
  summary: Reject or park active items that do not improve real work, reliability, or simplification; prefer deletion and compression over new subsystems unless the sacred workflow benefits.
  completion_note: Added strict TODO real-work guard requiring active items to pass real_work, reliability, or simplification with minimum soon-work scores.
- `RWF-SACRED-WORKFLOW` — Make the sacred workflow explicit and reusable
  completed_at: `2026-05-09T23:42:54.577Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-05-18`
  source_family: `Real Work First`
  summary: Document and wire the common loop from request, issue, alert, or Sentinel finding through evidence inspection, ownership, patch, targeted validation, explanation, and TODO or issue update.
  completion_note: Added strict TODO real-work guard that enforces the sacred workflow doctrine tokens and active item work gates/scores.
- `KSENT-RECEIPT-DRIFT` — Fix Kernel receipt drift release blocker
  completed_at: `2026-05-09T23:37:38.601Z`
  previous_section: `red`
  owner: `codex`
  deadline: `2026-05-10`
  source_family: `Kernel Sentinel Current Findings`
  summary: Implemented Sentinel freshness hardening so stale verity/health receipt evidence cannot remain current release-blocking truth, and added a Validation-owned regression test for stale generated_at failures. Final confirmation remains blocked by unrelated pre-existing dashboard compat syntax/conflict in 045-tool-recovery-and-turn-persistence.rs; keep open until that outside-lane blocker is cleared and a fresh Sentinel run confirms release blocker removal.
  completion_note: Closed as stale receipt/final-report truth leak: ops:ksent:fresh-evidence:guard now prevents stale Sentinel artifacts from being promoted as fresh/current truth. Remaining live blocker is kernel_sentinel_auto_timeout, tracked separately.
- `HYGIENE-COMBINED-GUARD` — Guard against new combined artifacts
  completed_at: `2026-05-09T23:34:25.626Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-05-20`
  source_family: `Combined Rust Artifact Burn-Down`
  summary: Add a validation guard or policy row that blocks new 000-combined.rs and .combined_parts artifacts unless they declare an explicit generated-source or compatibility exception with owner and expiry.
  completion_note: Added combined Rust artifact hygiene policy, strict guard command, and validation registry entry; guard passes against the 472-artifact inventory/reference-map baseline.
- `KSENT-FRESH-EVIDENCE` — Refresh deterministic Kernel evidence for Sentinel RSI readiness
  completed_at: `2026-05-09T23:34:25.314Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-05-18`
  source_family: `Kernel Sentinel Current Findings`
  summary: Feed fresh deterministic Kernel evidence into Sentinel so rsi_readiness_summary_current no longer reports needs_fresh_deterministic_evidence; keep autonomous RSI blocked until release-gate blockers are gone.
  completion_note: Added fresh-evidence guard enforcing stale Sentinel artifacts cannot be marked authoritative/current, plus source/test tokens for stale_historical_evidence_failure handling; current local Sentinel artifacts are stale but non-ok, so they cannot be promoted as fresh truth.
- `HYGIENE-COMBINED-REFERENCE-MAP` — Map combined artifact references
  completed_at: `2026-05-09T23:25:03.045Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-05-20`
  source_family: `Combined Rust Artifact Burn-Down`
  summary: For each combined artifact, map whether it is included by live Rust modules, tests only, generated tooling, docs only, or unreferenced so cleanup can be behavior-preserving.
  completion_note: Generated conservative combined Rust reference map under validation/reports.
- `HYGIENE-COMBINED-INVENTORY` — Inventory combined Rust artifacts
  completed_at: `2026-05-09T23:25:02.751Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-05-20`
  source_family: `Combined Rust Artifact Burn-Down`
  summary: Generate a manifest of every tracked 000-combined.rs and .combined_parts artifact with path, crate, line count, include/module parent, warning count, and likely owner before deleting or renaming anything.
  completion_note: Generated combined Rust artifact inventory under validation/reports.
- `SRS-DNA-FOUNDATION-LOCK` — Align Digital DNA SRS state with foundation audit
  completed_at: `2026-05-09T23:25:02.465Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-05-18`
  source_family: `SRS Active Triage 2026-05-09`
  summary: Reconcile Digital DNA Foundation v1 SRS queued state with DNA-FOUNDATION-AUDIT so DNA substrate work is tracked once with clear Kernel evidence requirements.
  completion_note: Added SRS foundation lock report consolidating DNA SRS state and next substrate proof.
- `DNA-FOUNDATION-AUDIT` — Audit and graduate Digital DNA as system substrate
  completed_at: `2026-05-09T23:25:02.150Z`
  previous_section: `yellow`
  owner: `unassigned`
  deadline: `2026-05-17`
  source_family: `Digital DNA Foundation Graduation Audit`
  summary: Yellow flag: Digital DNA has real Kernel metakernel code and tests, but SRS/TODO still mark V6-FOUNDATION-DNA-001/002 queued and it is not proven as the unavoidable base substrate for every instance, mutation, critical action, receipt, and Sentinel integrity check.
  completion_note: Added 2026-05-09 Digital DNA foundation audit with evidence, gap, and graduation criteria.
- `KSENT-EVAL-RUNTIME-SPLIT` — Tighten Eval versus Sentinel runtime failure boundary
  completed_at: `2026-05-09T23:25:01.824Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-05-18`
  source_family: `Kernel Sentinel Current Findings`
  summary: Resolve the eval_runtime_overlap cluster by ensuring response-quality failures stay with Eval while deterministic finalization, routing, persistence, and receipt failures stay with Sentinel, with handoff criteria documented in emitted findings.
  completion_note: Added Sentinel/eval runtime boundary policy defining ownership, handoff, and shared issue rules.
- `KSENT-FEEDBACK-ABSORPTION` — Improve Sentinel feedback absorption loop
  completed_at: `2026-05-09T23:25:01.509Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-05-18`
  source_family: `Kernel Sentinel Current Findings`
  summary: Reduce fragmented_observability_feedback_absorption by turning recurring Sentinel clusters into fewer structural repair lanes with dedupe, owner, evidence, doctrine law, and explicit review state instead of repeated local symptom drafts.
  completion_note: Added Sentinel feedback absorption policy with promotion lane, quality filter, dedupe key, and noise controls.
- `SRS-ACTIVE` — Keep active SRS intake items moving
  completed_at: `2026-05-09T23:02:55.335Z`
  previous_section: `red`
  owner: `unassigned`
  deadline: `2026-05-09`
  source_family: `Actionable SRS Items (Queued/In Progress)`
  summary: Keep active SRS intake items moving before opening more new fronts.
  completion_note: Executed by creating docs/workspace/reports/SRS_ACTIVE_TRIAGE_2026-05-09.md and splitting safe non-Shell/non-orchestration active SRS lanes into focused follow-up TODOs.
- `RWF-CODEQL-WORKFLOW` — Make CodeQL remediation the first canonical real-work workflow
  completed_at: `2026-05-09T22:54:08.925Z`
  previous_section: `red`
  owner: `codex`
  deadline: `2026-05-10`
  source_family: `Real Work First`
  summary: Define the reusable alert-to-patch loop: ingest CodeQL alert, locate file and rule, patch narrowly, run targeted validation, commit with a conventional prefix, and record the closure pattern.
  completion_note: Implemented canonical CodeQL remediation workflow under docs/workspace/process/CODEQL_REMEDIATION_WORKFLOW.md.
- `RWF-SENTINEL-FINDING-SHAPE` — Make Sentinel findings issue-ready before promotion
  completed_at: `2026-05-09T22:54:08.618Z`
  previous_section: `red`
  owner: `codex`
  deadline: `2026-05-10`
  source_family: `Real Work First`
  summary: Require promoted Sentinel findings to include evidence, recurrence or freshness, owner guess, root-cause hypothesis, proposed fix class, targeted validation suggestion, and TODO or issue wording.
  completion_note: Implemented Sentinel finding promotion policy with required evidence, law mapping, freshness, root cause, falsification, and review state.
- `RWF-RELIABILITY-FLOOR` — Define the minimum reliable real-work paths
  completed_at: `2026-05-09T22:54:08.258Z`
  previous_section: `red`
  owner: `codex`
  deadline: `2026-05-10`
  source_family: `Real Work First`
  summary: Keep install and repair, gateway lifecycle, request-to-response execution, alert remediation, Sentinel finding promotion, TODO lifecycle, and commit/release hygiene as the reliability floor.
  completion_note: Implemented reliability floor policy at docs/workspace/reliability_floor_policy.md and linked it from the Three Operating Laws doctrine.
- `SHELL-EXTRACT-MATRIX` — Prove the headless capability matrix
  completed_at: `2026-05-06T02:45:51.225Z`
  previous_section: `red`
  owner: `unassigned`
  deadline: `2026-05-07`
  source_family: `Shell Authority Extraction Without Legacy Shell Mutation`
  summary: Create the headless capability matrix and prove which Shell-visible operations already work through CLI/Gateway without browser assets before touching legacy dashboard code.
  completion_note: Closed with named assurance matrix validation/conformance/contracts/shell_headless_capability_matrix.json plus existing Shell Socket contract, Gateway route mapping, and browser-free headless probe exercising 13/13 capabilities. Validation: metrics:effective-loc, python3 -m json.tool validation/conformance/contracts/shell_headless_capability_matrix.json, npm run -s ops:shell-socket:headless-probe.
- `SHELL-SOCKET-LEGACY` — Quarantine Shell 1.0 as a legacy plug
  completed_at: `2026-05-02T22:01:21.814Z`
  previous_section: `white`
  owner: `codex`
  deadline: `after_red_section`
  source_family: `Shell Socket 2.0 Infrastructure`
  summary: Add a LegacyBrowserShellPlug manifest and guard so Shell 1.0 is limited to critical fixes, parity bridge work, and retirement support while Shell 2.0 socket work proceeds independently.
  completion_note: Quarantined Shell 1.0 as LegacyBrowserShellPlug without dashboard mutation. Added shell/legacy/legacy_browser_shell_manifest.json and ops:shell-socket:legacy:guard so 4173 is non-canonical compatibility, 5173 remains the Shell Socket Gateway/backend proof target, new features/Gateway bypass/socket parity claims fail closed, and retirement requires Gateway-only clean plug parity. Validation: metrics:effective-loc passed before new files; ops:shell-socket:legacy:guard passed; controlled violation failed closed; JSON parse and line caps passed.
- `SHELL-SOCKET-PROBE` — Prove the socket with a headless Shell probe
  completed_at: `2026-05-02T21:55:37.864Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-05-14`
  source_family: `Shell Socket 2.0 Infrastructure`
  summary: Build a headless/CLI socket probe that runs without browser assets and exercises runtime status, agent list, session list, message window, submit input, event stream, message detail, approval, and internal issue/eval submission through Gateway. Current evidence: `ops:shell-socket:headless-probe` exercises all 13 socket capabilities against a Gateway-shaped fixture; live `/api/shell-socket/**` read projections and ingress validation now exist in `core/layer0/ops`; approval decisions bind to the Kernel approval queue; `SHELL_SOCKET_REQUIRE_LIVE=1 npm run -s ops:shell-socket:live-probe` passes against the Gateway/backend route surface (`5173` locally), not legacy browser host `4173`.
  completion_note: Proved the socket with browser-free headless and live probes. Headless probe exercises all 13 capabilities against a Gateway-shaped fixture; live probe exercises 11 Gateway-backed route capabilities against 5173 and confirms approval decisions bind to Kernel approval queue behavior. Validation: ops:shell-socket:headless-probe and SHELL_SOCKET_REQUIRE_LIVE=1 ops:shell-socket:live-probe passed.
- `SHELL-SOCKET-CLIENT` — Build the typed Shell Socket Gateway client
  completed_at: `2026-05-02T21:55:33.003Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-05-14`
  source_family: `Shell Socket 2.0 Infrastructure`
  summary: Implement a transport-neutral typed Gateway client in `shell/socket/client` that exposes only socket methods, keeps no canonical state, imports no dashboard/Svelte/Alpine/browser globals, and routes only to declared Gateway endpoints.
  completion_note: Built transport-neutral Shell Socket Gateway client under shell/socket/client with 13 typed methods, injectable fetch transport, canonical /api/shell-socket/** route shaping, no browser-framework imports, and no retained canonical state. Validation: ops:shell-socket:client:self-test and ops:shell-socket:headless-probe passed.
- `SHELL-SOCKET-GUARDS` — Add Shell Socket contract and route guards
  completed_at: `2026-05-02T21:55:26.718Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-05-14`
  source_family: `Shell Socket 2.0 Infrastructure`
  summary: Add fail-closed socket contract and Gateway route guards with positive and controlled-negative fixtures so missing route mappings, raw/default full-state fields, direct Shell-to-authority calls, and missing cursor/detail/audit/Nexus constraints are blocked.
  completion_note: Added fail-closed contract and Gateway route guards with positive and controlled-negative coverage blocking missing mappings, raw/default full-state payload fields, non-Gateway targets, weak scrambler posture, and missing socket invariants. Validation: ops:shell-socket:contract:guard and ops:shell-socket:gateway-route:guard passed.
- `SHELL-SOCKET-GATEWAY` — Map Shell Socket capabilities to Gateway routes
  completed_at: `2026-05-02T21:55:22.096Z`
  previous_section: `red`
  owner: `codex`
  deadline: `2026-05-07`
  source_family: `Shell Socket 2.0 Infrastructure`
  summary: Create shell_socket_gateway_contract.json mapping every socket capability to a Gateway route class, owner of truth, capability or lease requirement, audit receipt, Nexus checkpoint, Conduit posture, and payload budget.
  completion_note: Mapped all Shell Socket capabilities to Gateway route classes and canonical /api/shell-socket/** routes, with owner-of-truth, receipt, Nexus/Conduit, payload-budget, and implementation status coverage. Validation: ops:shell-socket:gateway-route:guard, targeted Rust shell_socket tests, and hard live probe against 5173 passed.
- `SHELL-SOCKET-CONTRACT` — Define the Shell Socket contract artifacts
  completed_at: `2026-05-02T21:55:16.353Z`
  previous_section: `red`
  owner: `codex`
  deadline: `2026-05-07`
  source_family: `Shell Socket 2.0 Infrastructure`
  summary: Create the canonical `shell/socket/contract/shell_socket_contract.json` with socket capabilities, request and response shapes, projection families, payload budgets, detail refs, forbidden fields, and policy references. This is socket-only work with no UI or legacy dashboard wiring.
  completion_note: Defined canonical shell/socket contract with 13 projection-only capabilities, payload budgets, forbidden fields, Gateway-only axioms, guard coverage, and headless probe consumption. Validation: ops:shell-socket:contract:guard and ops:shell-socket:headless-probe passed.
- `KSENT-BIGPICTURE` — Add big-picture regression mode
  completed_at: `2026-05-01T23:50:02.174Z`
  previous_section: `white`
  owner: `sentinel`
  deadline: `after_red_section`
  source_family: `Kernel Sentinel Feedback Quality Upgrade`
  summary: When many subsystem symptoms co-occur, have Sentinel pause local-ticket generation and emit a structural diagnosis with recommended rebuild/realignment mode.
  completion_note: Added Kernel Sentinel big-picture regression mode in core. The model pauses local-ticket generation when co-occurring subsystem symptoms indicate structural diagnosis or rebuild/realignment, while preserving local ticketing for isolated defects. Validation: cargo test --manifest-path core/layer0/ops/Cargo.toml --lib big_picture_regression -- --nocapture passed 3/3; file-size check kept kernel_sentinel.rs at 496 lines and big_picture_regression.rs at 201 lines.
- `KSENT-SELF-REVIEW` — Add Sentinel feedback quality self-review
  completed_at: `2026-05-01T23:42:34.321Z`
  previous_section: `white`
  owner: `sentinel`
  deadline: `after_red_section`
  source_family: `Kernel Sentinel Feedback Quality Upgrade`
  summary: Track whether Sentinel findings were accepted, rejected, actionable, resolved, or merely symptom patches so feedback quality improves over time.
  completion_note: Added Kernel Sentinel feedback-quality self-review in core with accepted/rejected/actionable/resolved/symptom_patch statuses, quality scoring, accepted_for_learning, strengthens_sentinel, symptom_patch_risk, and follow-up routing. Added Observability documentation for review semantics. Validation: cargo test --manifest-path core/layer0/ops/Cargo.toml --lib feedback_quality -- --nocapture passed 5/5; file-size check kept kernel_sentinel.rs at 490 lines and feedback_quality.rs at 202 lines; metrics:effective-loc reported Rust share 89.4%.
- `OBS-CURRENT-TRUTH-GUARD` — Strengthen Observability current-truth vs stale-reference handling
  completed_at: `2026-05-01T23:36:45.297Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-05-15`
  source_family: `Sentinel Regression Audit May01`
  summary: Treat stale observer artifacts as an Observability-layer problem by enforcing current_live_truth vs historical_reference semantics across Sentinel and related evidence consumers.
  completion_note: Added an Observability current-truth freshness contract and guard. The contract makes current_live_truth the only decision-authoritative tier, treats recent/historical/stale artifacts as non-authoritative until refreshed or reviewed, validates required Sentinel consumers, and proves missing freshness fails closed to stale_reference_only. Validation: current-truth guard passed; controlled violation failed closed; JSON parsed; targeted freshness cargo tests passed 2/2.
- `KSENT-ISSUE-RELEASE-BRIDGES` — Repair Sentinel release-evidence and receipt-integrity blockers
  completed_at: `2026-05-01T23:33:39.037Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-05-15`
  source_family: `Sentinel Regression Audit May01`
  summary: Investigate and close the release_evidence and receipt_integrity bridge failures that currently drive Kernel Sentinel release_fail verdicts.
  completion_note: Added an Observability-owned release bridge repair lane for release-evidence and receipt-integrity Sentinel blockers. The lane requires current evidence, exact bridge field coverage, owner guess, replay command, blocker class, artifact refs, and no auto-apply policy. Validation: release bridge guard passed; controlled violation failed closed; JSON parsed; targeted release-gate synthesis and strict critical-finding cargo tests passed.
- `KSENT-ISSUE-EMPTY-RESPONSES` — Triage synthetic empty-response failures from Sentinel
  completed_at: `2026-05-01T23:30:41.083Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-05-15`
  source_family: `Sentinel Regression Audit May01`
  summary: Convert repeated synthetic user harness empty_assistant_response findings into a concrete issue lane covering routing, finalization, and synthesis failure causes.
  completion_note: Added an Observability-owned empty-response repair lane with symptom signatures, evidence refs, owner, root-cause hypothesis, concrete next action, acceptance criteria, no-visible-system-fallback policy, and validation commands. Validation: empty-response guard passed; controlled violation failed closed; JSON parsed; two targeted core workflow finalization diagnostics tests passed.
- `KSENT-ISSUE-BOUNDEDNESS` — Triage Sentinel boundedness findings into concrete repair work
  completed_at: `2026-05-01T23:27:24.163Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-05-15`
  source_family: `Sentinel Regression Audit May01`
  summary: Turn the workspace tooling boundedness findings into a concrete repair lane with owner, acceptance criteria, and replay/validation commands.
  completion_note: Added an Observability-owned Kernel Sentinel boundedness repair lane with owner, root-cause hypothesis, concrete next action, required boundedness dimensions, evidence refs, acceptance criteria, replay commands, and human-review/no-auto-apply policy. Validation: npm run -s ops:ksent:boundedness-repair:guard passed; controlled violation failed closed; JSON parsed; metrics:effective-loc reported Rust share 89.4%.
- `KSENT-FINAL-REPORT` — Publish Sentinel final output docs for top findings
  completed_at: `2026-05-01T23:23:52.744Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-05-15`
  source_family: `Sentinel Regression Audit May01`
  summary: Emit compact final output docs that summarize release blockers, top findings, evidence refs, and next actions instead of forcing operators to inspect huge raw state artifacts.
  completion_note: Published the Observability operator guide for Kernel Sentinel final output. The guide documents compact artifact refs, release blockers, top findings, triage findings, root-cause clusters, promotion lane, stale-reference handling, and report-budget expectations. Validation: cargo test --manifest-path core/layer0/ops/Cargo.toml --lib kernel_sentinel::report_budget -- --nocapture passed 7/7.
- `MLSYS5` — Assimilate MLSysBook Chapter 5 systems lessons
  completed_at: `2026-05-01T23:20:14.133Z`
  previous_section: `yellow`
  owner: `unassigned`
  deadline: `2026-05-27`
  source_family: `MLSysBook Vol. 1 Chapter 5 Neural Computation Implementation Backlog`
  summary: Assimilate Chapter 5 systems lessons into workload awareness, confidence routing, budgeting, and D-A-M diagnosis.
  completion_note: Assimilated MLSysBook Chapter 5 workflow lessons into Observability governance. Added a research contract, note, and ops:mlsys5:assimilation:guard covering workload awareness, confidence routing, resource budgeting, and D-A-M diagnosis. Validation: npm run -s ops:mlsys5:assimilation:guard passed; controlled violation rejected as expected.
- `TRACE` — Finish the universal trace substrate contract
  completed_at: `2026-05-01T23:16:26.400Z`
  previous_section: `yellow`
  owner: `unassigned`
  deadline: `2026-05-16`
  source_family: `Universal Trace Substrate / Fragmented Observability Closure`
  summary: Finish the Observability-owned universal trace substrate contract and anti-fragmentation guard.
  completion_note: Finished the Observability-owned universal trace substrate contract guard. Added universal_trace_substrate_contract.json plus ops:trace:substrate:guard to enforce single trace_id doctrine, required trace domains/events/authority classes/extensions, and anti-fragmented-observability root-schema placement. Validation: npm run -s ops:trace:substrate:guard passed; controlled violation rejected as expected.
- `KSENT-UNDERSTANDING-WORKSHEET` — Generate recurring system-understanding worksheets
  completed_at: `2026-05-01T23:12:47.720Z`
  previous_section: `yellow`
  owner: `sentinel`
  deadline: `2026-05-14`
  source_family: `Kernel Sentinel Feedback Quality Upgrade`
  summary: Have Sentinel periodically produce a self-understanding dossier covering soul, runtime behavior, authority map, boundary map, drift, gaps, and confidence.
  completion_note: Implemented recurring Kernel Sentinel system-understanding worksheet artifacts emitted on auto-run. The worksheet walks soul, runtime behavior, authority map, boundary map, drift, gaps, confidence, and syntax_detail in order, with confidence gates and required next probes. Validation: cargo test --manifest-path core/layer0/ops/Cargo.toml --lib kernel_sentinel::system_understanding_worksheet -- --nocapture passed 1/1; cargo test --manifest-path core/layer0/ops/Cargo.toml --lib kernel_sentinel::auto_run::tests::auto_run_persistence_tests::auto_run_writes_freshness_artifact_for_clean_state -- --exact --nocapture passed 1/1.
- `KSENT-FRESHNESS-TIERS` — Separate current truth from stale Sentinel reference
  completed_at: `2026-05-01T23:07:11.506Z`
  previous_section: `yellow`
  owner: `sentinel`
  deadline: `2026-05-14`
  source_family: `Kernel Sentinel Feedback Quality Upgrade`
  summary: Classify Sentinel outputs as current_live_truth, recent_but_not_current, historical_trend, or stale_reference_only before using them for decisions.
  completion_note: Implemented four-tier Kernel Sentinel freshness classification: current_live_truth, recent_but_not_current, historical_trend, and stale_reference_only. Only current_live_truth is decision-authoritative; missing/explicitly stale/old evidence fails closed as stale_reference_only. Validation: cargo test --manifest-path core/layer0/ops/Cargo.toml --lib kernel_sentinel::report_budget -- --nocapture passed 7/7.
- `KSENT-CONTRADICTIONS` — Add policy-vs-runtime contradiction detection
  completed_at: `2026-05-01T23:01:18.604Z`
  previous_section: `yellow`
  owner: `sentinel`
  deadline: `2026-05-14`
  source_family: `Kernel Sentinel Feedback Quality Upgrade`
  summary: Compare doctrine, contracts, code paths, and artifacts to surface semantic contradictions such as projection-only policy with runtime-state mirrors.
  completion_note: Implemented first-class policy-vs-runtime contradiction findings: Nexus boundary evidence now recognizes doctrine/runtime, contract/artifact, and projection-policy/runtime-mirror contradictions and emits critical policy_runtime_contradiction fingerprints. Validation: cargo test --manifest-path core/layer0/ops/Cargo.toml --lib kernel_sentinel::evidence::nexus_boundaries -- --nocapture passed 5/5.
- `KSENT-AUTHORITY-GHOSTS` — Detect authority ghosts after refactors
  completed_at: `2026-05-01T22:58:52.383Z`
  previous_section: `yellow`
  owner: `sentinel`
  deadline: `2026-05-14`
  source_family: `Kernel Sentinel Feedback Quality Upgrade`
  summary: Add first-class authority_ghost findings for projection layers, caches, shims, or adapters that preserve old authority shape after syntax-level cleanup.
  completion_note: Implemented first-class Kernel Sentinel authority ghost findings: Nexus boundary evidence now recognizes projection/cache/shim/adapter authority ghost aliases and emits critical authority_ghost fingerprints. Validation: cargo test --manifest-path core/layer0/ops/Cargo.toml --lib kernel_sentinel::evidence::nexus_boundaries -- --nocapture passed 4/4.
- `KSENT-ANTI-PATCHING` — Detect symptom-patching loops
  completed_at: `2026-05-01T22:55:50.186Z`
  previous_section: `yellow`
  owner: `sentinel`
  deadline: `2026-05-14`
  source_family: `Kernel Sentinel Feedback Quality Upgrade`
  summary: Flag repeated patches that change visible symptoms while the same boundary, policy, or ownership violation remains unresolved.
  completion_note: Implemented Kernel Sentinel anti-patching detection: issue synthesis now flags repeated visible symptom patches under one structural root, publishes anti_patching metadata, and collapses them into one structural repair issue. Validation: cargo test --manifest-path core/layer0/ops/Cargo.toml --lib kernel_sentinel::issue_synthesis -- --nocapture passed 11/11.
- `KSENT-ACTION-SYNTHESIS` — Improve issue/TODO synthesis specificity
  completed_at: `2026-05-01T22:47:40.629Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-05-14`
  source_family: `Kernel Sentinel Feedback Quality Upgrade`
  summary: Require component, observed failure, root-cause hypothesis, repair type, acceptance criteria, validation route, and evidence before promoting feedback to TODOs.
  completion_note: Implemented Kernel Sentinel issue/TODO specificity: issue drafts now require component, observed failure, root-cause hypothesis, repair type, validation route, evidence, acceptance criteria, and human-review/no-auto-mutation actionability before promotion. Validation: cargo test --manifest-path core/layer0/ops/Cargo.toml --lib kernel_sentinel::issue_synthesis -- --nocapture passed 10/10.
- `KSENT-FRESHNESS-GUARD` — Enforce fresh Sentinel truth before using findings
  completed_at: `2026-05-01T22:34:40.385Z`
  previous_section: `red`
  owner: `codex`
  deadline: `2026-05-08`
  source_family: `Sentinel Regression Audit May01`
  summary: Prevent stale Sentinel artifacts from being treated as current truth by requiring freshness windows, generated_at checks, and stale_do_not_use labeling for old findings.
  completion_note: Implemented Kernel Sentinel current-truth freshness classification for final reports; stale findings are labeled stale_do_not_use, excluded from top_findings, and blocked from TODO/GitHub promotion. Validation: cargo test --manifest-path core/layer0/ops/Cargo.toml kernel_sentinel::report_budget -- --nocapture passed 5/5.
- `KSENT-SYMPTOM-CLUSTERS` — Cluster symptoms into root-cause families
  completed_at: `2026-05-01T21:48:36.512Z`
  previous_section: `red`
  owner: `codex`
  deadline: `2026-05-07`
  source_family: `Kernel Sentinel Feedback Quality Upgrade`
  summary: Group repeated UI/runtime/tool/eval findings into root-cause clusters so Sentinel detects structural failure families instead of isolated symptoms.
  completion_note: Implemented Kernel Sentinel root-cause symptom clustering for feedback rows, including structural surface-family cluster metadata, repeated-cluster promotion signals, SRS evidence, file-size split, and targeted Sentinel regression coverage.
- `KSENT-FEEDBACK-CONTRACT` — Add Sentinel feedback-to-TODO actionability contract
  completed_at: `2026-05-01T21:33:53.792Z`
  previous_section: `red`
  owner: `codex`
  deadline: `2026-05-07`
  source_family: `Kernel Sentinel Feedback Quality Upgrade`
  summary: Make each Sentinel feedback item declare whether it is todo_ready, triage_to_todo, or needs_root_cause_synthesis, with explicit evidence/actionability requirements.
  completion_note: Implemented Kernel Sentinel feedback-to-TODO actionability states (todo_ready, triage_to_todo, needs_root_cause_synthesis) with explicit evidence/actionability requirements, tests, and live auto-run feedback inbox smoke.
- `KSENT-FAILURE-LEVELS` — Strengthen Sentinel failure-level classification
  completed_at: `2026-05-01T21:26:03.140Z`
  previous_section: `red`
  owner: `codex`
  deadline: `2026-05-07`
  source_family: `Kernel Sentinel Feedback Quality Upgrade`
  summary: Teach Sentinel to classify symptom, component, boundary, policy-truth, architectural, and self-model failures before recommending remediation.
  completion_note: Implemented Kernel Sentinel failure_class/review_depth classification, final-report failure_level_summary, and promotion candidate failure-level framing with targeted tests and live kernel-sentinel smoke.
- `WF-AUTH` — Finish workflow gate authority cleanup
  completed_at: `2026-05-01T21:15:11.122Z`
  previous_section: `red`
  owner: `unassigned`
  deadline: `2026-05-05`
  source_family: `Workflow Authority Cleanup`
  summary: Finish workflow gate authority cleanup so every turn enters Gate 1, tool/menu submissions are LLM-authored, and visible chat cannot be system-substituted.
  completion_note: Completed workflow gate authority cleanup wave: visible responses now carry explicit LLM-authored/system-substitution provenance, system fallback cannot be mislabeled as LLM-authored chat, and Gate 1 manual-toolbox candidate choices can promote to pending confirmation only from candidate-menu context.
- `KSENT-CANDIDATE-PIPELINE` — Stage Sentinel findings through a candidate pipeline before TODO or GitHub promotion
  completed_at: `2026-05-01T21:05:51.030Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-05-15`
  source_family: `Sentinel Regression Audit May01`
  summary: Add a proposed-finding pipeline with todo_ready, issue_ready, needs_triage, and stale_do_not_use states so Sentinel drafts candidates first and Codex or a human approves promotion before the main TODO or GitHub is mutated.
  completion_note: Implemented Kernel Sentinel draft-only promotion lane: final reports now emit human-review-required promotion_candidates and triage_candidates with todo_ready/issue_ready/needs_triage states while forbidding Sentinel TODO/GitHub mutation or patch auto-apply.
- `KSENT-ROOT-CAUSE-CLUSTERING` — Cluster Sentinel symptoms into structural root-cause findings
  completed_at: `2026-05-01T20:58:04.031Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-05-15`
  source_family: `Sentinel Regression Audit May01`
  summary: Collapse repeated symptoms into shared structural issues when they point at the same boundary, ownership, policy, or runtime fracture instead of emitting parallel symptom-level findings.
  completion_note: Implemented Kernel Sentinel final-report root-cause clustering: release-ready findings are grouped by owner/category/root-frame/fingerprint family, top_findings carry cluster exemplar metadata, and root_cause_clusters summarize occurrence counts, sample IDs, evidence refs, and next actions.
- `KSENT-QUALITY-FILTER` — Add a strict release-quality filter for Sentinel findings
  completed_at: `2026-05-01T20:49:13.025Z`
  previous_section: `red`
  owner: `codex`
  deadline: `2026-05-08`
  source_family: `Sentinel Regression Audit May01`
  summary: Only release Sentinel findings that include evidence, recurrence or freshness support, owner guess, root-cause hypothesis, and a concrete next action; everything else stays in draft or triage state.
  completion_note: Implemented Kernel Sentinel release-quality filtering for final reports; top_findings now require trusted evidence, recurrence/freshness support, owner guess, root-cause hypothesis, and concrete next action, while weak observations stay bounded in triage_findings.
- `KSENT-REPORT-BUDGET` — Add Sentinel report-size budget and evidence compaction
  completed_at: `2026-05-01T20:40:40.512Z`
  previous_section: `red`
  owner: `codex`
  deadline: `2026-05-08`
  source_family: `Sentinel Regression Audit May01`
  summary: Keep Sentinel outputs bounded by separating raw evidence from operator summaries, adding report byte ceilings, and releasing only the useful compressed portion in the final report surface.
  completion_note: Implemented Kernel Sentinel byte-budgeted final report surface with compact top findings, raw evidence stream refs, CLI emission, regression tests, CLI smoke, and strict fail-closed validation.
- `KSENT-STALL-GUARD` — Make Sentinel auto-runs fail small instead of stalling
  completed_at: `2026-05-01T20:29:46.200Z`
  previous_section: `red`
  owner: `codex`
  deadline: `2026-05-08`
  source_family: `Sentinel Regression Audit May01`
  summary: Add timeout, heartbeat, and partial-artifact failover so ops:kernel-sentinel:auto either refreshes evidence or exits with a small diagnostic artifact instead of hanging silently.
  completion_note: Implemented Kernel Sentinel auto-run heartbeat/timeout guard with compact diagnostic artifact; validated timeout regression, direct CLI exit 124 artifact, and strict fail-closed Sentinel test.
- `CGPT-GOV-011` — Expose PrepareContext in plan displays and eval traces
  completed_at: `2026-05-01T18:16:18.278Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-05-14`
  source_family: `ChatGPT Governance Audit Apr30`
  summary: Make context preparation operator-visible whenever a read-like plan mutates session context so read/write semantics stay explicit.
  completion_note: Exposed PrepareContext/context_preparation in progress displays, package/candidate decision-trace metadata, and step trace inputs; conformance coverage asserts pure-read none and comparative explicit traces.
- `CGPT-GOV-007` — Keep tests/tooling as harness-only, not an ungoverned domain
  completed_at: `2026-05-01T18:10:12.050Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-05-14`
  source_family: `ChatGPT Governance Audit Apr30`
  summary: Ensure definition-shaped eval, benchmark, conformance, regression, release-gate, and scorecard truth lives under Validation/Observability while tests/tooling remains executor glue.
  completion_note: Added Validation-owned tooling harness-only policy and guard, moved six regression fixture matrices from tests/tooling/fixtures into validation/regression/fixtures, rewired fixture consumers, registered governance gates in the lifecycle registry, and validated positive/controlled-negative/tooling/test-maturity/transition/workspace/replay/churn paths.
- `CGPT-GOV-005` — Add tiered browser-rendered long-chat heap proof
  completed_at: `2026-05-01T17:53:35.874Z`
  previous_section: `red`
  owner: `codex`
  deadline: `2026-05-10`
  source_family: `ChatGPT Governance Audit Apr30`
  summary: Add Tier 2 browser-rendered heap/DOM/storage regression and Tier 3 gateway-to-shell projection stress proof on top of the deterministic store-projection guard.
  completion_note: Added Validation-owned tiered long-chat heap proof contract and guard covering Tier 1 store-guard wiring, Tier 2 rendered Shell fixture budgets, and Tier 3 Gateway-to-Shell projection stress; positive, controlled-negative, transition, runtime-payload, JSON, line-cap, diff, and churn checks passed.
- `CGPT-GOV-004` — Compress chat.ts responsibilities
  completed_at: `2026-05-01T17:46:12.509Z`
  previous_section: `red`
  owner: `codex`
  deadline: `2026-05-10`
  source_family: `ChatGPT Governance Audit Apr30`
  summary: Continue splitting chat.ts into projection rendering, input collection, detail fetch, and local display preferences while moving workflow, terminal, model, and tool coordination out.
  completion_note: Compressed chat.ts by extracting terminal prompt/cursor display math into chat_message_display_helpers.ts; delegated chat.ts and matching part file; dashboard build, helper smoke, Svelte shell contract, runtime payload budget, scoped diff check passed; long-chat RAM guard remains red on pre-existing Shell purge debt.
- `CGPT-GOV-003` — Runtime-check Shell session/message/event payload budgets
  completed_at: `2026-05-01T17:41:46.544Z`
  previous_section: `red`
  owner: `codex`
  deadline: `2026-05-08`
  source_family: `ChatGPT Governance Audit Apr30`
  summary: Verify live session, messages, and websocket event paths conform to bounded Gateway projection contracts at runtime, not just in policy files.
  completion_note: Added runtime-shaped Shell payload budget contract and guard for session list, chat message window, and runtime event projections; validated against Gateway interface payload budgets with positive, controlled-negative, static budget, and transition-residue checks.
- `CGPT-GOV-002` — Prove all external surfaces enter through Gateways
  completed_at: `2026-05-01T17:36:46.228Z`
  previous_section: `red`
  owner: `codex`
  deadline: `2026-05-08`
  source_family: `ChatGPT Governance Audit Apr30`
  summary: Audit Shell, CLI, SDK, issue-submission, eval-submission, and future app/mobile ingress so every external surface is Gateway-only with no first-party shell exception.
  completion_note: Added Validation-owned Gateway external surface contract and guard for Shell, CLI, SDK, issue/eval submission, plugin/external-agent, and future app/mobile surfaces; wired package/tooling/transition enforcement; positive guard passed and controlled negative failed closed.
- `CGPT-GOV-001` — Keep Tower metaphor out of active subsystem names
  completed_at: `2026-05-01T17:28:25.841Z`
  previous_section: `red`
  owner: `codex`
  deadline: `2026-05-08`
  source_family: `ChatGPT Governance Audit Apr30`
  summary: Enforce that Tower remains historical/metaphor-only and cannot appear in active subsystem names, routes, generated maps, CI gates, or manifests.
  completion_note: Added ops:orchestration:naming:guard, registered it in tooling, wired transition-residue guard to require it, and proved positive plus controlled-negative enforcement.
- `CGPT-GOV-012` — Wire representation-collapse reporting into governance
  completed_at: `2026-05-01T05:34:09.811Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-05-14`
  source_family: `ChatGPT Governance Audit Apr30`
  summary: Track representation count for message, session, tool result, trace, workflow, task, evidence, and issue candidate so projections do not become new truth stores.
  completion_note: Wired representation-collapse reporting into package scripts and tooling registry with artifacts for representation risk triage.
- `CGPT-GOV-010` — Verify Shell amputation guard executable linkage
  completed_at: `2026-05-01T05:34:09.492Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-05-14`
  source_family: `ChatGPT Governance Audit Apr30`
  summary: Keep policy docs, package scripts, tooling registry, and actual shell-amputation guard path aligned so shell replaceability remains executable.
  completion_note: Verified and repaired Shell amputation executable linkage; guard now fixtures/scans orchestration instead of retired surface path.
- `CGPT-GOV-009` — Create Shell Cognition burn-down register
  completed_at: `2026-05-01T05:34:09.163Z`
  previous_section: `red`
  owner: `codex`
  deadline: `2026-05-10`
  source_family: `ChatGPT Governance Audit Apr30`
  summary: Classify every Shell Cognition subsystem as presentation-local or re-home it to Orchestration, Observability, Validation, Kernel, Gateways, or compatibility debt.
  completion_note: Added Shell Cognition burn-down register covering all ten generated Shell Cognition subsystems with target domains and deadlines.
- `CGPT-GOV-008` — Retire or explicitly govern surface/orchestration
  completed_at: `2026-05-01T05:34:08.850Z`
  previous_section: `red`
  owner: `codex`
  deadline: `2026-05-10`
  source_family: `ChatGPT Governance Audit Apr30`
  summary: Define whether surface/orchestration is a compatibility mirror or active path, assign owner/expiry, and prevent drift from canonical orchestration/.
  completion_note: Governed retired surface/orchestration via transition register and rewired active package/guard references to orchestration/.
- `CGPT-GOV-006` — Create virtual-repo manifests for every top-level domain
  completed_at: `2026-05-01T05:34:08.529Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-05-14`
  source_family: `ChatGPT Governance Audit Apr30`
  summary: Declare Kernel, Orchestration, Gateways, Shell, Validation, Observability, Governance, Conduit, Layer 3, and Adapters as virtual repos with owned contracts and dependency boundaries.
  completion_note: Implemented domain_virtual_repo_manifest.json and architecture_transition_residue_guard validation for required virtual repo domains.

## Legacy Appendix
- Preserved historical markdown: [TODO_ARCHIVE_LEGACY.md](/Users/jay/.openclaw/workspace/docs/workspace/todo/TODO_ARCHIVE_LEGACY.md)

