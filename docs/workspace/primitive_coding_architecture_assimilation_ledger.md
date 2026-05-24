# Primitive Coding Architecture Assimilation Ledger

Status: active  
Purpose: extract proven coding-agent architecture patterns from successful systems before rebuilding Infring's coding workflow foundation.

## Why this ledger exists

Infring is intended to become an AI engine / operating-system seed, not a one-off coding agent. That means the coding workflow must be built from reusable primitives and contracts, not from eval-shaped prompt/runtime patches.

The rule for this ledger is:

```text
Assimilate mechanics, not product-specific hacks.
```

Imported ideas must become one of:

- Primitive
- Composite
- Tool CD
- Workflow CD
- Schema
- Policy
- Adapter/profile/config surface
- Eval/test fixture
- Rejected pattern

## Reference sources available locally

| System | Local reference | Status | Use in this pass |
|---|---|---|---|
| ForgeCode | `local/workspace/assimilations/ForgeCode-Assimilation` | rich local repo plus prior assimilation ledger | Primary source for coding primitives, runtime loop, file tools, repair, checkpoints, tool normalization, and governance/eval boundaries. |
| OpenHands | `local/workspace/research/openhands-intake` | staged intake map plus local vendor clone reference | Primary source for controller/event loop, runtime/sandbox boundary, state tracker, stuck detection, action execution server, and plugin/runtime boundaries. |
| Aider | `local/workspace/research/aider-intake` | staged intake plus one imported capability | Primary source for repo-map/context, edit discipline, file-call reliability, and HTML/noise-slimdown style extraction patterns. |
| Cline | `local/workspace/research/cline-intake` | staged intake with imported capabilities and many captured candidates | Primary source for workspace search, terminal output truncation, session overlap guard, streaming tool-call dedupe, permission auto-allow, checkpoint restore UX, tool-name normalization, and headless task-runner patterns. |
| Codex | `validation/fixtures/assimilation/codex` | large file ledger fixture | Secondary source for coding-agent loop and tool discipline patterns; must be treated as fixture/reference material unless specific mechanics are extracted into primitives. |
| OpenFang | `local/tmp/external/openfang` | local external repo with agent templates | Secondary source for agent template manifests, specialist role catalog, model-tier/fallback policy, and agent fleet capability manifests. |
| OpenAI cookbook coding-agent examples | `references/research-systems-study/repos/openai-cookbook-deep-research/examples/Build_a_coding_agent_with_GPT-5.1.ipynb` and related agent examples | local reference repo | Secondary source for SDK-style coding-agent composition, tool-using agent examples, and multi-agent orchestration examples. |
| DeerFlow / LangGraph-style agent harness | `references/research-systems-study/repos/deer-flow` | local reference repo | Secondary source for pure-parameter agent factory, public client API layering, middleware/features/config separation, and subagent/user isolation tests. |
| Research-agent systems | `references/research-systems-study/repos/*` | local multi-repo study | Tertiary source for multi-agent orchestration, evidence loops, benchmark harnessing, and LangGraph patterns when useful to coding workflow composition. |
| Coding agent artifact corpus | `references/coding-agent-systems` | pulled primary artifacts with exact commit SHAs | Current source/docs/config artifacts from SWE-agent, mini-SWE-agent, SWE-ReX, Aider, OpenHands, Cline, Roo Code, Continue, and Goose. Use `artifact_manifest.json`, `_pulled_repos.tsv`, and `artifact_index.md` to drive primitive extraction. |
| Continue | `references/coding-agent-systems/continue` | pulled primary artifact corpus | Source for IDE context providers, model/provider config layering, tool/provider abstraction, and assistant configuration surfaces. |

## Online primary-source artifacts to pull patterns from

This section is the higher-authority source list for the primitive reset. Local ledgers are useful, but the goal is to copy proven mechanics from successful coding systems using their current public repos/docs where available.

Pulled artifact corpus:

- `references/coding-agent-systems/_pulled_repos.tsv` records exact repository URLs and commit SHAs.
- `references/coding-agent-systems/artifact_manifest.json` records pulled file counts and sample files.
- `references/coding-agent-systems/artifact_index.md` indexes candidate controller, runtime, context, edit, mode, tool, and validation files.

| System | Online artifact | Evidence signal | Primitive patterns to extract |
|---|---|---|---|
| SWE-agent | `https://github.com/SWE-agent/SWE-agent` | GitHub-issue-to-patch agent, NeurIPS 2024 project, public repo, coding benchmark lineage | Issue-to-patch loop, agent-computer interface, config-driven agent construction, patch artifact production, SWE-bench task framing. |
| SWE-ReX | `https://github.com/SWE-agent/SWE-ReX` | Sandboxed shell runtime powering SWE-agent; supports local/remote/cloud and parallel runs | Runtime execution abstraction, sandbox session management, exit-code/output receipts, interactive shell sessions, parallel agent execution. |
| Aider | `https://aider.chat/docs/repomap.html` and `https://aider.chat/docs/more/edit-formats.html` | Widely used terminal coding agent with repo map and model-specific edit formats | Repo-map context primitive, graph-ranked context budget, diff/search-replace edit formats, architect/editor split, model-specific edit-format selection. |
| OpenHands | `https://github.com/All-Hands-AI/OpenHands` and OpenHands SDK docs/blog | Large open-source coding-agent platform with runtime/controller/sandbox architecture | Event-sourced controller, action/observation protocol, runtime sandbox boundary, state/replay/stuck detection, SDK tool composition. |
| Cline | `https://github.com/cline/cline` | Popular autonomous IDE coding agent; human-in-the-loop file/command approval | Workspace file/AST/regex discovery, file edit + terminal command approval, streaming tool-call projection, session state, permissions UX. |
| Roo Code | `https://github.com/RooCodeInc/Roo-Code` | Cline-derived multi-mode coding agent with Code/Architect/Ask/Debug/custom modes | Mode-as-manifest architecture, role-specific model/tool policy, orchestrator/debug modes, custom mode contracts. |
| Continue | `https://docs.continue.dev/customize/custom-providers` | Mature IDE assistant with typed context providers and repo-map provider | Context-provider plugin system, `@File`, `@Code`, `@Diff`, `@Terminal`, `@Codebase`, `@Search`, `@Tree`, `@Problems`, `@repo-map`, MCP context provider. |
| Goose | `https://github.com/aaif-goose/goose` | Rust-based local/desktop/CLI/API agent with MCP/ACP and many providers | Local agent runtime packaging, extension/provider distribution, MCP/ACP integration, desktop/CLI/API parity, custom distributions. |
| OpenAI cookbook coding-agent examples | `references/research-systems-study/repos/openai-cookbook-deep-research/examples/Build_a_coding_agent_with_GPT-5.1.ipynb` and online cookbook equivalents | Official examples for building tool-using and coding agents | SDK-style tool loop examples, structured outputs, multi-agent orchestration examples, simple-to-advanced educational scaffolds. |
| DeerFlow / LangGraph agent harness | Local repo plus online project if needed | Public agent factory and middleware layering reference | Pure-parameter agent factory, config/features/extra-middleware separation, public client API, subagent isolation tests. |

Online import rule:

```text
Prefer primary artifacts: official repos, official docs, papers, and benchmark docs.
Use third-party summaries only to discover sources, not as authority for primitive contracts.
```

## Import boundaries

Do import:

- Durable mechanics
- State machines
- Tool contracts
- Failure classifications
- Retry/repair budgets
- Context selection patterns
- Sandboxing/isolation boundaries
- Receipt and evidence shapes
- Event/replay semantics

Do not import:

- Prompt phrase hacks
- Benchmark-specific success checks
- Product-specific file names or app shapes
- Provider examples as generic behavior
- UI authority
- Language-specific behavior unless wrapped in a language/profile primitive
- Eval fixture assumptions into production code

## Source priority for coding reset

Primary coding-agent sources:

- ForgeCode
- OpenHands
- Aider
- Cline

Secondary architecture/runtime sources:

- Codex fixture ledger
- OpenFang
- OpenAI cookbook coding-agent examples
- DeerFlow / LangGraph-style agent harness

Tertiary supporting sources:

- Research-agent systems under `references/research-systems-study/repos/**`

Missing but useful later:

- SWE-agent / Devin-style public eval writeups if local references are added

## Portable primitive patterns

### 1. Event-sourced controller loop

Source evidence:

- OpenHands `controller/agent_controller.py`
- OpenHands `controller/state/state.py`
- OpenHands `controller/state/state_tracker.py`
- OpenHands `controller/replay.py`

Portable pattern:

- Agent execution should be a sequence of typed events, actions, observations, state transitions, and terminal artifacts.
- Replay should be possible from filtered action/observation history.
- Delegation and hidden events should be explicitly filtered from user-visible synthesis.
- Iteration/budget limits should be first-class state, not prompt text.

Infring target:

- `local_runtime_execution_loop`
- `conversation_state_persistence`
- `public_reasoning_trace_protocol`
- `final_receipt_synthesis` primitive candidate

Required reset action:

- Move runtime-controller task-shape decisions into declared workflow/tool contracts and typed events.

### 2. Runtime/sandbox/action execution boundary

Source evidence:

- OpenHands `runtime/base.py`
- OpenHands `runtime/action_execution_server.py`
- OpenHands runtime implementation map
- ForgeCode `local_coding_operator_integration_guard`

Portable pattern:

- Separate controller decisions from action execution.
- Execution boundary owns liveness, locking, sandbox/workspace identity, plugin readiness, tool observation serialization, timeout/error envelopes, and cleanup.
- Local no-sandbox execution should be an explicit risk state, not the silent default.

Infring target:

- Native Infring agent execution lane
- `local_code_edit_execution`
- `sandbox_worktree_isolation`
- `validation_command_runner`
- Gateway/adapters for non-local providers

Required reset action:

- Keep native file/command mutation behind tool receipts and permission receipts.
- Do not let workflow prompt text become execution authority.

### 3. Safe file operation primitives

Source evidence:

- ForgeCode `forge_fs`
- ForgeCode safe read/write/patch workflows already assimilated
- OpenHands agent-skills file ops/file editor patterns
- Aider file-call reliability candidate

Portable pattern:

- File read/write/patch must be standalone primitives.
- Reads are bounded and hash-bearing.
- Writes snapshot old content before overwrite.
- Patches use exact-context or structured edit semantics and fail on ambiguity.
- File search/navigation is separate from mutation.

Infring target:

- `safe_file_read`
- `safe_file_write`
- `safe_file_patch`
- `repo_context_assessment`
- `local_context_pack_builder`

Required reset action:

- Register all file primitives in the primitive capability registry with input/output/receipt contracts.
- Keep task-specific file names in eval fixtures or caller context, not primitives.

### 4. Context selection before execution

Source evidence:

- ForgeCode `repo_context_assessment`, `forge_walker`, `forge_repo`
- OpenHands state/history tracker
- Aider repo map and file-call patterns

Portable pattern:

- Existing-project work needs bounded context selection before mutation.
- Context selection should emit a context pack: selected files, rationale, excluded paths, validation commands, risks, open questions, and confidence.
- Context is advisory. Current workspace files remain authoritative.

Infring target:

- `repo_context_assessment`
- `local_context_pack_builder`
- `resume_context_retrieval`
- `project_context_capture`

Required reset action:

- Let higher-level coding workflow decide when context is needed.
- Keep new-file greenfield tasks on the direct write lane only when no existing context is required.

### 5. Bounded edit slice

Source evidence:

- ForgeCode local code edit execution
- Aider edit/patch discipline
- OpenHands action execution serialization

Portable pattern:

- One edit slice should have a declared goal, input context, allowed files/scope, planned mutations, validation command policy, and terminal artifact.
- The slice should not decide full product architecture unless explicitly delegated.

Infring target:

- `local_code_edit_execution`
- `plan_artifact_create`
- `existing_project_evidence_contract` candidate

Required reset action:

- Replace `existing_project_evidence_contract` with a general existing-project evidence contract.

### 5a. Small scoped edit artifact

Source evidence:

- Aider `editblock_prompts.py` and `editblock_fenced_prompts.py`
- Aider Level 3 speed isolation trace in `docs/workspace/aider_level3_speed_isolation.md`
- Infring bounded patch artifact lane receipts

Portable pattern:

- When relevant files are already selected and compact, do not enter a broad
  agent tool loop.
- Send selected file contents in one provider request and ask only for an edit
  artifact.
- Keep the edit artifact small, exact, and deterministic: SEARCH/REPLACE old
  text must match supplied context.
- Apply the artifact through the runtime's safe patch tool and trust only
  mutation receipts.

Infring target:

- `small_scoped_edit_artifact`
- bounded patch artifact profile selection
- `safe_file_patch`
- `validation_command_runner`
- `final_receipt_synthesis`

Required reset action:

- Keep this as a Tier 2 artifact profile, not a global controller shortcut.
- The primitive may know compact edit-artifact mechanics, but it may not know
  eval levels, fixture files, symbols, or project-specific layouts.
- Emit phase timing before further optimization so speed work remains evidence
  driven.
- Omit model routing for now; model substitution belongs in a later profile or
  policy once same-model behavior is stable.
- Retry the general bounded patch artifact profile with the same selected
  context before falling back from `small_scoped_edit_artifact` to the full
  native tool loop.
- Keep this profile dormant by default until repeated same-model runs prove that
  it beats the general bounded artifact lane without causing timeout cascades.
- The general bounded artifact lane gets one compact retry for invalid artifact
  output or artifact-call timeout; after that, quick-edit workflows should
  return structured artifact failure instead of wandering into the open native
  loop unless explicitly configured otherwise.

### 6. Validation and repair loop

Source evidence:

- ForgeCode `validation_command_runner`, `failure_diagnosis`, `bounded_repair_loop`
- OpenHands stuck detector and runtime error vocabulary
- Aider edit-feedback loop patterns

Portable pattern:

- Validation is a primitive command receipt, not proof by assertion.
- Failed validation becomes structured repair input.
- Repair loops have attempt budgets, scope limits, undo/escalation, and blocker reporting.
- Stuck/doom-loop signals are separate from normal validation failure.

Infring target:

- `validation_command_runner`
- `failure_diagnosis`
- `bounded_repair_loop`
- `doom_loop_interrupt`
- `tool_error_reflection`

Required reset action:

- Keep validation required only when the task, contract, or workflow requires it.
- Avoid forcing validation into context-free single-file tasks unless requested.

### 7. Checkpoint and memory handoff

Source evidence:

- ForgeCode `checkpoint_handoff`
- ForgeCode conversation persistence and tracker patterns
- OpenHands state persistence/resume

Portable pattern:

- Long-running coding should stop at explicit checkpoints.
- Checkpoints emit changed files, validation, risks, excluded scope, next slice, and memory/update refs.
- Memory is a higher-level composite concern, not a primitive edit-loop requirement.

Infring target:

- `checkpoint_handoff`
- `project_context_capture`
- `checkpoint_memory_write`
- `resume_context_retrieval`
- Higher-level coding project operator

Required reset action:

- Keep memory/checkpoint behavior out of Level 0/1 primitives unless the primitive is specifically a checkpoint/memory primitive.

### 8. Tool schema and tool-call normalization

Source evidence:

- ForgeCode `tool_schema_registry`
- ForgeCode `tool_call_normalization`
- OpenHands runtime action/observation protocols

Portable pattern:

- Tools need declared names, aliases, schemas, validation, normalization, receipts, and error envelopes.
- Tool-call repair should normalize shape, not infer benchmark semantics.

Infring target:

- Tool CDs
- Native file tool capability pack
- Tool broker/request validation

Required reset action:

- Promote local file tools and command tools into explicit Tool CD surfaces where possible.
- Keep Rust as executor/player, not hidden workflow author.

### 9. Stuck detection and doom-loop interruption

Source evidence:

- OpenHands `controller/stuck.py`
- ForgeCode doom-loop reminder and tool-error reflection

Portable pattern:

- Detect repeated identical actions, repeated errors, repeated monologues, context-window loops, and no-progress tool sequences.
- Stuck detection should change strategy or escalate, not keep patching blindly.

Infring target:

- `doom_loop_interrupt`
- `failure_diagnosis`
- `bounded_repair_loop`
- Observability findings

Required reset action:

- Make repeated lower-level regression after higher-level patch a first-class `primitive_composition_boundary_violation`.

### 10. Final receipt synthesis

Source evidence:

- ForgeCode display/result packaging
- OpenHands observation/state tracker
- Infring native receipt gates

Portable pattern:

- Final answer should be synthesized from terminal artifact and receipts.
- It must distinguish changed files, validation run/not run, blockers, and residual risk.
- It must not expose hidden chain-of-thought or raw tool payloads.

Infring target:

- `final_receipt_synthesis` primitive candidate
- `public_reasoning_trace_protocol`
- coding final output contracts

Required reset action:

- Add `final_receipt_synthesis` to primitive registry once defined.

### 11. Workspace search and mention/file discovery

Source evidence:

- Cline `CLINE-FILE-SEARCH-001`
- Aider `AIDER-FILECALL-001`
- ForgeCode `forge_walker` / repo traversal primitives

Portable pattern:

- File discovery should be a primitive, not a side effect of broad shell commands.
- Workspace search should support fuzzy ranking, multi-root inputs, repo boundary gates, missing-ripgrep diagnostics, and tracked-file filtering where appropriate.
- Mention/file picker behavior belongs in Shell/composer projection, while authoritative file discovery belongs in Kernel/Orchestration tooling.

Infring target:

- `repo_context_assessment`
- `local_context_pack_builder`
- workspace file search Tool CD candidate

Required reset action:

- Do not let the coding runtime guess file context from benchmark phrases.
- Route file discovery through explicit primitives/tool contracts.

### 12. Streaming tool-call dedupe and task state sync

Source evidence:

- Cline stream dedupe candidates
- OpenHands action/observation event protocol
- ForgeCode stream/event handling

Portable pattern:

- Streaming updates need stable event IDs, tool-call IDs, partial-message merge, and duplicate suppression.
- User-visible progress should be a projection of events, not a second source of truth.

Infring target:

- `public_reasoning_trace_protocol`
- `conversation_state_persistence`
- Shell Socket projection contracts
- native agent receipt stream

Required reset action:

- Ensure coding final answers and progress UI consume receipt/event refs rather than raw tool payloads or repeated stream fragments.

### 13. Permission, auto-approval, and confirmation policy

Source evidence:

- Cline permission auto-allow candidate
- ForgeCode operation permission gate
- OpenHands runtime action boundary

Portable pattern:

- Repeated operations may be auto-approved only inside a scoped, explicit policy with audit receipts.
- Permission policy should be data/config/Kernel-owned, not prompt-owned.

Infring target:

- `operation_permission_gate`
- native file/command tool permission templates
- Kernel policy

Required reset action:

- Keep `file.read`, `file.write`, `file.patch`, and `command.run` permissions explicit and receipt-bound.

### 14. Agent templates and specialist role manifests

Source evidence:

- OpenFang `agents/*/agent.toml`
- ForgeCode sage/muse/forge role taxonomy
- Cline subagent tree/status candidates

Portable pattern:

- Agent roles should be manifests with capabilities, model/fallback policy, tool permissions, memory permissions, and dispatch scope.
- Specialist roles should be optional composition resources, not hidden runtime branches.

Infring target:

- agent capability manifests
- higher-level coding operator
- future multi-agent coding composite

Required reset action:

- Keep the primitive single-agent coding workflow clean before adding multi-agent specialist composition.

### 15. Pure-parameter agent factory and middleware layering

Source evidence:

- DeerFlow RFC for `create_deerflow_agent`
- LangGraph agent factory patterns
- OpenAI cookbook agent examples

Portable pattern:

- Public agent API should support simple defaults and explicit parameter injection.
- Config, feature/middleware replacement, and extra middleware should be separate layers.
- Global filesystem config should be a fallback, not the only construction path.

Infring target:

- native Infring agent creation path
- provider/model binding
- workflow/tool capability packs

Required reset action:

- Make coding-agent creation reproducible from explicit manifests and capability packs.
- Do not rely on ambient dashboard/session state as hidden construction input.

### 16. Agent-computer interface and issue-to-patch loop

Source evidence:

- SWE-agent
- SWE-ReX
- SWE-bench/SWE-agent papers and public benchmark framing

Portable pattern:

- Treat coding work as an agent-computer loop over a repository, an issue/task, tools, observations, and a patch artifact.
- Keep execution environment independent from agent logic.
- Make command execution return structured output, exit code, completion state, and session identity.
- Support parallel runs without changing agent logic.

Infring target:

- Native coding worker lane
- `validation_command_runner`
- `local_code_edit_execution`
- `existing_project_evidence_contract`
- future sandbox/session primitive

Required reset action:

- Add a sandbox/session abstraction around command execution so local/Docker/cloud execution can share one primitive contract.
- Make patch/diff artifact production an explicit terminal artifact for issue-fix style tasks.

### 17. IDE context-provider model

Source evidence:

- Continue context providers
- Cline file mentions/search
- Aider repo map

Portable pattern:

- Context should be requested through typed providers: file, code symbol, diff, current file, terminal, docs, open files, web, codebase, folder, search, URL, clipboard, tree, problems, debugger, repo map, MCP.
- Providers are plugins/config entries, not runtime prompt hacks.

Infring target:

- `local_context_pack_builder`
- `repo_context_assessment`
- Shell composer context projection
- Tool CD context providers

Required reset action:

- Turn context sources into a context-provider registry with ownership and receipts.

### 18. Mode-as-manifest architecture

Source evidence:

- Roo Code modes
- OpenFang agent templates
- ForgeCode sage/muse/forge roles
- Cline custom modes / settings

Portable pattern:

- Coding agents should expose modes/roles as manifests with purpose, tools, model policy, permissions, and output contract.
- Modes should not be hardcoded branches in the runtime.

Infring target:

- agent manifests
- coding workflow mode contracts
- primitive capability registry

Required reset action:

- Define `architect`, `coder`, `debugger`, `reviewer`, and `orchestrator` as manifest-level capabilities only after the single-agent primitive path is clean.

### 19. Local agent distribution and extension surface

Source evidence:

- Goose
- Cline/Roo/Continue extension ecosystems
- OpenHands SDK/runtime

Portable pattern:

- Successful coding systems expose multiple surfaces over one engine: CLI, desktop/editor, API/SDK, and extensions.
- Extensions/tools should bind through an explicit protocol such as MCP/ACP or Tool CDs.

Infring target:

- Shell Socket
- Tool CDs
- Gateway/adapter boundary
- native agent runtime

Required reset action:

- Keep local coding capability engine-owned and expose it through Shell/API/CLI as projections, not separate implementations.

### 20. Step-budgeted trajectory runtime

Source evidence:

- mini-SWE-agent `agents/default.py`
- Cline runtime turn streaming
- Continue stream thunk wrappers
- OpenHands event/action/observation controller patterns

Portable pattern:

- Coding runs should persist a trajectory before they rely on a final response.
- Each model turn and tool result should become durable run evidence.
- Step limit, tool-call limit, time budget, cost budget, and terminal reason should be explicit runtime state.
- Timeout or abort should return a structured partial artifact, not erase receipts because final stdout was missing.

Infring target:

- `step_budgeted_trajectory_runtime`
- `incremental_receipt_journal`
- native Infring agent run lane
- final receipt synthesis

Required reset action:

- Treat missing final stdout as an incomplete run with partial receipts, not as proof that no work happened.
- Add runtime support for flushing model/tool/mutation/validation receipts before each risky boundary.

### 21. Public-interface verification

Source evidence:

- Aider validation and repair loop docs
- SWE-agent issue-to-patch framing
- OpenHands action/observation protocol
- Continue tool-call and policy evaluation flow

Portable pattern:

- Passing generic validation is not enough when the user requested a named public surface.
- Public surface requirements include functions, classes, constructor args, exports, CLI commands, file outputs, API routes, schema fields, and configuration keys.
- Language-specific probes should live in adapters/profiles, not in the primitive.
- Missing or malformed public surface is a semantic failure that routes to bounded repair or a structured blocker.

Infring target:

- `public_interface_verification`
- `validation_command_runner`
- `bounded_repair_loop`
- `final_receipt_synthesis`

Required reset action:

- Add a public-interface requirement artifact between mutation/validation and final synthesis.
- Do not hardcode Python, CSV, pricing, router, or benchmark-specific probes in production runtime.

## Canonical primitive library proposal

### Level 0 primitives

| Primitive | Source inspiration | Owner target | Notes |
|---|---|---|---|
| `safe_file_read` | ForgeCode, OpenHands file skills | Orchestration + native file tool executor | Already exists; needs complete registry contract. |
| `safe_file_write` | ForgeCode | Orchestration + native file tool executor | Already exists; keep generic. |
| `safe_file_patch` | ForgeCode, Aider | Orchestration + native file tool executor | Already exists; exact/stale/ambiguous patch semantics. |
| `validation_command_runner` | ForgeCode, OpenHands runtime | Orchestration/native command executor | Already exists; should be command receipt primitive. |
| `repo_context_assessment` | ForgeCode, Aider | Orchestration | Existing lab primitive; should feed context pack. |
| `local_context_pack_builder` | ForgeCode + Aider context | Orchestration | Already registered. |
| `source_test_pair_context_selection` | Aider, mini-SWE-agent | Orchestration/native runtime | Select the smallest likely source/test context pair before existing-project mutation; must be bounded, layout-agnostic, and disabled for micro create-file work. |
| `plan_artifact_create` | ForgeCode planning | Orchestration | Existing lab primitive. |
| `failure_diagnosis` | ForgeCode, OpenHands stuck/runtime errors | Orchestration/Observability | Existing lab primitive. |
| `checkpoint_handoff` | ForgeCode tracker/display | Orchestration | Existing lab primitive. |
| `project_context_capture` | ForgeCode/OpenHands persistence | Orchestration/Memory | Existing lab primitive. |
| `resume_context_retrieval` | OpenHands state resume | Orchestration/Memory | Existing lab primitive. |
| `final_receipt_synthesis` | ForgeCode display, OpenHands observations | Orchestration | Missing; should be created. |
| `existing_project_evidence_contract` | Current Existing-project contamination generalized | Orchestration/Validation | Missing; should replace `existing_project_evidence_contract`. |
| `workspace_file_discovery` | Cline, Aider, ForgeCode | Kernel/Orchestration Tool CD | Should wrap repo-bound fuzzy/path search and missing-tool diagnostics. |
| `stream_event_dedupe` | Cline, OpenHands, ForgeCode | Observability/Shell projection | Should suppress duplicate tool/progress projection without owning runtime truth. |
| `permission_scope_tracker` | Cline, ForgeCode | Kernel/Governance | Should track scoped auto-approval and confirmation boundaries. |
| `sandbox_session_runtime` | SWE-ReX, OpenHands | Kernel/Orchestration/Gateway | Should abstract local/Docker/cloud command sessions with receipts. |
| `step_budgeted_trajectory_runtime` | mini-SWE-agent, Cline, Continue, OpenHands | Orchestration/native runtime | Seed primitive for step limits, terminal status, and durable partial trajectories. |
| `incremental_receipt_journal` | mini-SWE-agent, Cline, Continue, OpenHands | Orchestration/native runtime | Seed primitive for append-only model/tool/mutation/validation receipts. |
| `context_provider_registry` | Continue, Aider, Cline | Orchestration/Tooling/Shell projection | Should make context sources pluggable and receipt-bearing. |
| `mode_manifest_registry` | Roo Code, OpenFang, ForgeCode | Orchestration/Governance | Should define roles/modes without runtime hardcoding. |
| `patch_artifact_synthesis` | SWE-agent, Aider | Orchestration/Validation | Should produce issue-fix patches/diffs as terminal artifacts when task shape requires. |
| `bounded_patch_artifact_lane` | Aider Level 3 speed isolation | Orchestration/native runtime | Fast path for small confident existing-project edits: selected files in context, one patch artifact model call, deterministic patch apply, validation receipt, synthetic final. |
| `public_interface_verification` | Aider, SWE-agent, OpenHands, Continue | Orchestration/Validation | Seed primitive for requested public surface verification before final success. |
| `owner_module_completion_gate` | Aider lint/test repair loop, mini-SWE-agent source-before-tests loop | Orchestration/Validation/native runtime | Treat test-only public API additions as incomplete until the source owner module defines or exports the imported symbol; failures should produce repair hints, not final success. |
| `provider_timeout_recovery_artifact` | mini-SWE-agent trajectory status, SWE-ReX runtime failure receipts | Orchestration/native runtime | Convert timeout after successful mutation into receipt-backed partial progress or bounded recovery instead of losing the run. |

### Level 1 composites

| Composite | Child primitives | Notes |
|---|---|---|
| `local_code_edit_execution` | file read/write/patch + validation command | Existing; should become boring edit-slice executor. |
| `local_context_loop_guard` | tool access, doom-loop, compaction, tool-error reflection | Existing; should not own mutation semantics. |
| `local_tooling_surface_guard` | schema registry, tool normalization, MCP, commands, skills | Existing; should not encode eval cases. |
| `local_runtime_execution_loop` | request transform, stream runner, dispatch, lifecycle, persistence | Existing; should be event/receipt driven. |

### Level 2+ composites

| Composite | Role | Notes |
|---|---|---|
| `bounded_repair_loop` | diagnose -> repair -> validate -> undo/escalate | Existing; registered as composite. |
| `local_coding_program_builder` | bounded project slice builder | Should remove Existing-project naming and depend on general evidence contracts. |
| `coding_project_operator` | higher-level architecture/checkpoint/memory operator | Should own high-level policy, not primitive semantics. |

## Immediate patch plan

### Patch 1: replace level-specific evidence contract

Problem:

- `local_coding_program_builder.workflow.json` contains `existing_project_evidence_contract`.
- This is production workflow contamination by eval level.

Patch:

- Rename contract to `existing_project_evidence_contract`.
- Rename stage from `existing_project_evidence_contract_self_check` to `existing_project_evidence_self_check`.
- Replace artifact paths like `workflow_artifacts/existing_project_slice_handoff.json` with general names like `workflow_artifacts/existing_project_slice_handoff.json`.
- Move Existing-project artifact expectations to eval fixtures/judges only.

### Patch 2: register missing primitives

Add registry entries for:

- `repo_context_assessment`
- `plan_artifact_create`
- `local_code_edit_execution`
- `failure_diagnosis`
- `checkpoint_handoff`
- `project_context_capture`
- `resume_context_retrieval`
- `final_receipt_synthesis` once created
- `existing_project_evidence_contract` once created

### Patch 3: separate runtime task lanes from prompt phrase matching

Problem:

- Native runtime currently classifies lanes from prompt text.
- Some of this is valid task interpretation, but it should be declared and bounded by contracts.

Patch:

- Move lane definitions into workflow/tool contracts or config.
- Runtime can evaluate declared lane predicates, but it should not invent them.
- Keep `new_file_fast_path` only as a registered lane with explicit activation and non-goals.

### Patch 4: create final receipt synthesis primitive

Purpose:

- Standardize final coding output from receipts and terminal artifact.

Inputs:

- Changed file receipts
- Validation receipts
- Blocker receipts
- Public reasoning rollup refs
- Requirement checklist

Output:

- User-visible status
- Changed files
- Validation summary
- Blockers/residual risk
- Next step

### Patch 5: restart evals from Level 1

Only after patches 1-4:

- Run Level 1 first.
- Move upward only when lower levels stay green.
- If higher-level work regresses lower levels, classify as `primitive_composition_boundary_violation`.

### Patch 6: fold Cline/Codex/OpenFang/DeerFlow sources into the primitive registry

Use these references to add or refine primitive candidates:

- Cline -> workspace search, streaming dedupe, permission scope tracking, checkpoint restore UX, headless task runner.
- Codex fixture ledger -> tool discipline and coding-loop comparison evidence, not production hardcoding.
- OpenFang -> agent manifest and specialist-role catalog patterns.
- DeerFlow/OpenAI cookbook -> public agent factory and middleware/config layering.

### Patch 7: add online-proven coding-agent primitives

Use online primary-source artifacts to add/refine:

- SWE-agent/SWE-ReX -> `sandbox_session_runtime`, `patch_artifact_synthesis`, issue-to-patch loop.
- Continue -> `context_provider_registry`.
- Roo Code/OpenFang/ForgeCode -> `mode_manifest_registry`.
- Goose -> local agent distribution and MCP/ACP extension-surface contracts.

## Decision

We should not invent the next coding architecture from scratch.

The best path is:

```text
ForgeCode file/edit/repair/checkpoint primitives
+ OpenHands event/runtime/state/stuck boundaries
+ Aider repo-map/edit/context discipline
+ Cline workspace search/session/permission/streaming UX mechanics
+ Codex/OpenFang/DeerFlow/OpenAI cookbook agent composition references
-> Infring primitive registry and workflow CDs
-> native-agent evals from Level 1 upward
```

This gives us a scalable coding workflow foundation without copying product-specific behavior or rebuilding known-good mechanics from vibes.
## Primitive candidate: `bounded_direct_edit_lane`

Status: `v1_wired_behind_workflow_flags`

Evidence source:

- Codex live runtime batch: `references/coding-agent-systems/runtime_trace_harness/reports/codex_level3_level4_batch_20260523_130434/report.json`
- Pattern extraction: `references/coding-agent-systems/runtime_trace_harness/reports/codex_level3_level4_batch_20260523_130434/codex_runtime_pattern_extraction.md`

Observed reference performance:

- Codex Level 3: `5/5` pass, average wall `23.9s`, average first mutation `13.7s`.
- Codex Level 4: `5/5` pass, average wall `39.3s`, average first mutation `26.1s`.

Primitive definition:

`bounded_direct_edit_lane` performs bounded local reads, direct native file mutation, validation/probe command execution, and terminal receipt synthesis for small or bounded existing-project coding tasks.

Why this belongs in the primitive library:

- It generalizes a successful runtime behavior observed in Codex.
- It solves a primitive failure mode in Infring: artifact-generation timeout before mutation.
- It composes upward into larger coding workflows without relying on level-specific cases.

Required sub-primitives:

- `bounded_context_selection`
- `safe_file_read`
- `safe_file_patch`
- `safe_file_write`
- `validation_command_runner`
- `bounded_repair_loop`
- `final_receipt_synthesis`

Promotion gate:

- Must pass lower-level monotonic coding evals before being used for higher-level workflow improvements.
- Must expose receipts for context read, mutation, validation/probe, repair attempts, and terminal result.
- Must fail closed with a structured blocker when file context, permissions, mutation, or validation evidence is missing.

Implementation notes:

- Runtime v1 is wired in `core/layer2/agent_surface/src/agent.rs`.
- Lab workflow flag is wired in `orchestration/src/control_plane/workflows/lab/composites/coding/local_coding_phase1_mutation_spine.workflow.json`.
- Official coding workflow flag is wired in `orchestration/src/control_plane/workflows/official/coding_project_operator.workflow.json`.
- The lane currently reuses the native direct tool loop rather than introducing a second tool dispatcher.

Anti-hardcoding note:

This primitive must not mention Level 3, Level 4, `math_tools.py`, `slug_tools.py`, or any eval fixture content in runtime logic. Fixture-specific expectations belong only in eval harnesses.

## Primitive candidate: `pre_mutation_validation_bootstrap`

Status: `v1_wired_inside_bounded_direct_edit_lane`

Evidence source:

- Infring Level 4 validation-guided repair traces from May 23, 2026.
- Codex/Aider comparison traces showing that explicit setup/validation actions
  should not consume avoidable repair-oriented model turns.

Observed failure mode:

- The task explicitly requested a validation command before editing.
- Runtime had enough information to run that command directly.
- The model sometimes emitted repair and validation calls in one batch but in
  the wrong order, causing the staged controller to block the useful mutation.

Primitive definition:

`pre_mutation_validation_bootstrap` runs a user-specified validation command as
native evidence before the first repair-oriented provider turn.

Why this belongs in the primitive library:

- It generalizes "observe the failure first" repair behavior.
- It avoids burning a provider turn on an explicitly specified local command.
- It gives the repair model real failing output before mutation.
- It keeps final truthfulness receipt-backed without loosening mutation gates.

Required inputs:

- explicit project root
- explicit validation-before-edit intent
- concrete validation command extracted from the task

Non-goals:

- infer broad test commands from vague requests
- run validation without user/workflow permission
- hardcode eval level names or fixture files

## Primitive candidate: `completion_evidence_repair_routing`

Status: `v2_patch_applied`

Evidence source:

- Infring Level 3 twenty-run reliability batch from May 23, 2026.

Observed failure mode:

- Product source mutation succeeded.
- The requested test mutation was missing.
- Existing validation still passed because the old test suite did not cover the
  new behavior.
- Repair routing drifted toward generic product/source repair instead of the
  missing test mutation.
- Terminal synthesis could still emit a success marker despite unresolved test
  evidence.

Primitive definition:

`completion_evidence_repair_routing` maps unresolved completion evidence to the
most specific repair stage before generic product repair.

Rules:

- if product mutation already exists and test evidence is missing, route repair
  to `test_mutation`
- live staged execution must use the same priority, so a product mutation
  followed by missing test evidence routes to test mutation before generic
  product repair
- passing validation does not satisfy an explicit test-change requirement unless
  a test mutation receipt exists
- bounded direct edit must emit `partial_blocked`, not success, while required
  changed-path or test-change evidence remains unresolved
- final success must recompute required changed-path and test-change evidence
  from prompt plus receipts, not only from transient repair-reason state
- no eval level names or fixture file names may appear in runtime logic

## Rejected pattern: fixture-derived implementation synthesis

Status: `forbidden_in_production`

Incident source:

- A local coding optimization attempted to inspect public API bindings plus
  Python tests/probes and synthesize missing function bodies directly in the
  Rust runtime.
- The path could pass a narrow eval quickly by deriving an implementation body
  from fixture-shaped evidence instead of improving general coding ability.

Why this is rejected:

- It creates fake capability: the benchmark passes without proving the agent can
  plan, edit, validate, or repair arbitrary software.
- It violates primitive monotonicity because a higher-level eval patch can
  distort lower-level runtime behavior.
- It makes metrics untrustworthy by moving task-specific reasoning into hidden
  production branches.

Forbidden production behaviors:

- derive arithmetic/string/router/API/data-transform implementation bodies from
  local probe examples or assertions
- branch on package names, symbol names, fixture file names, eval levels, or
  benchmark prompt shapes
- return success from deterministic synthesis that bypasses the generic edit
  primitive or model edit plan

Allowed replacement:

- use local evidence only for context selection, owner-file constraints,
  validation commands, and completion evidence
- ask the model or a declared general edit primitive for the edit plan
- apply edits through generic file tools with receipts
- validate using command receipts and repair through bounded generic repair
- keep fixture-specific expected behavior in eval fixtures and scorers only

Enforcement:

- `ops:primitive-first:guard` rejects production fixture/test/probe-derived
  implementation synthesis patterns.
- `ops:coding-primitive-hardcoding:guard` is the focused clean gate for this
  specific failure class.
- Any future deterministic coding synthesis must be registered as a primitive,
  have cross-case tests, document its extension surface, and remain disabled in
  production until promoted.

## Primitive candidate: `bounded_fast_edit_preflight`

Status: `v1_wired_inside_bounded_direct_edit_lane`

Evidence source:

- Controlled Level 5 comparison on May 24, 2026 using the same control model:
  Claude Code reached first mutation in about 13s, ForgeCode in about 15s,
  while Infring reached first mutation in about 46s.

Observed failure mode:

- Infring loaded bounded context and validation evidence but still exposed the
  full discovery/read/write/command tool menu before the first mutation.
- The model could spend extra turns on redundant read/list/status calls even
  after runtime had already supplied the bounded local context.
- Later finalization and repair machinery could add useful safety, but it
  should not delay the first concrete edit for small bounded project changes.

Primitive definition:

`bounded_fast_edit_preflight` converts already-collected bounded context and
explicit validation/probe receipts into a first-edit constraint: after preflight
context is available, the next provider turn should use mutation and command
tools rather than reopening broad discovery.

Rules:

- activate only inside the generic bounded direct edit lane
- require receipt-backed local context before narrowing the first tool menu
- do not infer implementation logic from tests, probes, symbols, package names,
  eval levels, or fixture shapes
- allow only file write/patch and command run tools until the first mutation
- keep context and validation evidence as placement/diagnostic input, not as
  deterministic implementation synthesis
- return a structured blocker if the preflight context proves mutation is unsafe

Expected effect:

- preserve primitive-first behavior
- reduce redundant pre-mutation context turns
- move first mutation closer to successful reference-agent timing without
  hiding task-specific code in the runtime

## Primitive candidate: `first_edit_batch_contract`

Status: `v1_wired_inside_bounded_fast_edit_preflight`

Evidence source:

- Infring Level 5 after `bounded_fast_edit_preflight` still reached first
  mutation around 44s despite reducing provider calls and tool calls.
- Claude Code and ForgeCode traces show the useful pattern: once context and
  validation evidence exist, the next model turn emits an edit batch rather
  than continuing broad discovery.

Primitive definition:

`first_edit_batch_contract` constrains the first provider turn after bounded
preflight context to a compact mutation-oriented contract.

Rules:

- activate only after bounded direct edit preflight has successful read-context
  receipts
- expose only mutation and command tools for that first edit turn
- use a compact system prompt that requires JSON tool calls only
- require at least one file write/patch before validation if validation was
  already observed
- runtime-block command execution until the first edit batch has a successful
  file write/patch receipt
- permit a structured blocker when receipts prove mutation is unsafe
- do not infer implementation bodies from fixtures, probes, tests, symbol names,
  package names, or eval levels

Expected effect:

- convert preloaded context into faster first mutation without weakening
  receipt-backed safety
- keep speed optimization primitive-level and reusable across bounded coding
  tasks rather than tied to any eval case

Expected effect:

- reduce first-mutation latency
- preserve receipt-backed safety
- keep public interface edits as generic placement/validation work rather than
  case-specific synthesis
