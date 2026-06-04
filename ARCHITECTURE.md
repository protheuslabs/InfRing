# InfRing Architecture

InfRing is built as a Rust-first deterministic Kernel runtime with an explicit split between:
- Authoritative Kernel (`core/**` path compatibility)
- Orchestration Control Plane (`orchestration/**`)
- Gateway boundary membrane (`gateway/**`)
- Presentation Shell (`client/**` path compatibility)

Canonical architecture contract:
- `docs/SYSTEM-ARCHITECTURE-SPECS.md` (InfRing Layering Specification v1.0)
- `docs/workspace/orchestration_ownership_policy.md` (Kernel/Orchestration/Shell boundary policy)

Boundary axiom:
- Kernel decides what is true and allowed.
- Orchestration decides what should happen next.
- Shell decides how it is shown and collected.

Transition note (docs-first):
- The canonical coordination subsystem name is Orchestration Control Plane.
- `Tower`, `Cognition Control Plane`, and `Cognition Plane (Orchestration Control Plane)` are non-canonical historical/metaphor terms, not active ownership labels.
- Internal code/path naming transitions remain incremental where path stability is required.
- Shell canonicalization notes: `docs/workspace/shell_transition_notes.md`.

Canonical ownership vocabulary:

| Concept owner | Path / compatibility wording | Rule |
|---|---|---|
| Kernel | `core/**`, historical `Core` | Use Kernel for authority/truth. Use `core/**` only for filesystem paths. |
| Orchestration Control Plane | `orchestration/**`, rejected metaphor `Tower` | Use Orchestration Control Plane for coordination ownership. Do not use Tower as a subsystem name. |
| Shell | `client/**`, historical `Client` | Use Shell for presentation ownership. Use `client/**` only for filesystem paths. |
| Gateways | `gateway/**`, historical compatibility hosts under `adapters/runtime/**` | Use Gateways for external membrane ownership. `adapters/**` is translator-only and must not be used as the canonical Gateway implementation path. |
| Adapters | `adapters/**` | Use Adapters for provider/framework/runtime translation behind Gateway sockets only. Adapters do not own ingress policy, route authority, permission policy, payload budgets, or Shell-facing sockets. |
| Assurance | `validation/**`, `observability/**`, governance registries | Validation judges controlled behavior; Observability watches live behavior; Governance derives gates/verdicts. |

## InfRing Direction

InfRing is the target operating model: a portable autonomous substrate that runs unchanged across desktop, server, embedded, and high-assurance profiles.

- Rust kernel remains the single source of truth.
- Conduit is the only TS <-> Rust bridge.
- TS is reserved for flexible surfaces (UI, marketplace, extensions, experimentation).
- Orchestration Control Plane authority is Rust-first (`orchestration/src/**`); `orchestration/**` must remain at least `95%` Rust by tracked source lines, and TypeScript under `orchestration/scripts/**` is adapter-only and must stay minimal.
- The dashboard, CLI, SDK, and future shells must connect to the system through Gateway sockets. A Shell must never directly host Gateway policy, route authority, runtime engine routing, or adapter dispatch.

## Three-Plane Metakernel

InfRing is explicitly modeled as a substrate-independent metakernel with three planes:

1. Safety plane (`planes/safety`, implemented in `core/`): deterministic authority stack with strict upward-only flow:
   - `core/layer_minus_one/` - Exotic Hardware Template (thin substrate adapter contract)
   - `core/layer0/` - Safety Plane origin (sacred/immutable contract)
   - `core/layer1/` - Policy Engine + deterministic receipts
   - `core/layer2/` - Scheduling + execution orchestration
   - `core/layer3/` - OS Personality Template (traditional OS growth layer)
2. Cognition plane (`planes/cognition`, historical broad plane label implemented across `orchestration/` and `client/`):
   - Orchestration Control Plane: decomposition, coordination, sequencing, recovery, and result shaping/packaging (among other things in non-canonical coordination).
   - Presentation Shell: rendering, input, UX shells, and presentation-local state. `client/**` remains a path name, not a conceptual owner.
   - Gateway boundary membrane: external ambiguity firewall and socket host under `gateway/**`. Gateway is not a Shell and not an Adapter.
3. Substrate plane (`planes/substrate`): runtime/backend descriptors for CPU/MCU/GPU/NPU/QPU/neural channels with explicit degradation contracts and fallback declarations.

Hard boundary:
- AI can propose; Kernel authority decides.
- Shell <-> system communication enters through Gateway sockets, then Nexus/Conduit/Scrambler to the correct owner. Shell <-> Kernel direct communication is forbidden.
- Every substrate must declare fallback/degradation behavior.

Formal contract surfaces:
- Boundary/formal specs: `planes/spec/`
- Inter-plane contract schemas: `planes/contracts/`

## Kernel Stack Contract

The deterministic Kernel stack is now explicitly layered and growth-safe:

- Layer 0 is immutable and proof-preserving.
- Layer -1 is where exotic hardware paradigms are adapted into standard envelopes.
- Layer 3 is where full OS personality capabilities grow (processes, VFS, drivers, syscalls, namespaces, windowing, networking).
- Cognition remains outside numbered Kernel layers and never becomes root-of-correctness.
- Executable wrappers:
  - Layer -1 exotic base wrapper: `core/layer_minus_one/exotic_wrapper`
  - Layer 3 full OS extension wrapper: `core/layer3/os_extension_wrapper`

Driver analogy:

- `core/` is the drivetrain, brakes, and stability control.
- `orchestration/` is the driving control plane (decomposition + pacing + recovery + packaging).
- `client/` is the Shell path compatibility surface (steering wheel, dashboard, and infotainment).
- `gateway/` is the skin/boundary membrane. It owns Shell/CLI/SDK/external-agent sockets, ingress normalization, payload budgets, permission gate entrypoints, trace propagation at the boundary, and bounded egress projections.
- `adapters/` are translators behind Gateway sockets. They may understand provider/framework-specific protocols, but they may not own Gateway policy or become the Shell's path into a runtime.
- Conduit is the harness between orchestration and Kernel boundaries.

REQ-27 authority implementation:

- Importance scoring engine: `core/layer0/ops/src/importance.rs`
- Priority ordering + queue metadata: `core/layer0/ops/src/attention_queue.rs`
- Layer2 initiative primitives (score/action/priority queue shaping): `core/layer2/execution/src/initiative.rs`
- Regression guard (no subconscious authority in shell): `client/runtime/systems/ops/subconscious_boundary_guard.ts`
- Cockpit + layer wrapper delta requirements: `docs/client/requirements/REQ-33-cockpit-stream-and-layer-wrappers.md`

Migration note:
- Strictly follow InfRing Layering Specification v1.0 with upward-only flow:
  `Layer -1 -> Layer 0 -> Layer 1 -> Layer 2 -> Layer 3 -> Cognition`.
- Existing `layer0/ops` authority lanes remain active while Layer2 ownership is completed incrementally without runtime regressions.

## Mech-Suit Cockpit Runtime

- `infringd` now defaults to `attach` semantics (attach-or-start) for cockpit-first operation.
- Startup origin-integrity checks support degraded timeout mode with deterministic retry scheduling, rather than hard startup deadlocks.
- Attention queue drain supports wait-based delivery (`--wait-ms`) for long-lived subscription behavior through conduit receipts.

## Filesystem Mapping (Authoritative)

| Plane | Contract Location | Implementation Location | Mutable Runtime Location |
|---|---|---|---|
| Safety | `planes/safety/` | `core/layer_minus_one/`, `core/layer0/`, `core/layer1/`, `core/layer2/`, `core/layer3/` | `core/local/` |
| Cognition | `planes/cognition/` | `orchestration/` (Orchestration Control Plane coordination) + `gateway/` (external boundary membrane and sockets) + `client/` (Shell runtime path compatibility: `systems`, `lib`, `config`, `packages`, `tools`, `tests`, `observability`, `apps`, `developer`) + `adapters/` (translator-only integration bridges behind Gateway sockets) | `client/runtime/local/` + `core/local/` (receipted orchestration artifacts) |
| Substrate | `planes/substrate/` | Template gateways in `core/layer_minus_one/` + capability descriptors under `planes/substrate/` | `core/local/` + `client/runtime/local/` |

Additional split rules:

- Source of truth code: `core/`, `orchestration/`, and the shell path `client/`.
- Runtime/user/device/instance data: shell runtime path `client/runtime/local/` and `core/local/` only.
- Legacy compatibility links are disabled by default. Canonical runtime roots are direct:
  - `client/runtime/local/*` for shell runtime data
  - `core/local/*` for core runtime data

## Direct Wiring Policy

- Deprecated compat surfaces (`client/runtime/state`, root `state/`, root `local/`) are not valid runtime paths.
- Shell wrappers must call Kernel authority through conduit/scrambler only; no policy authority exists in TS compatibility shells.
- Migration tooling may provide one-time compatibility options, but defaults are direct to canonical roots.
- Canonical path constants are centralized in:
  - TS: `client/runtime/lib/runtime_path_registry.ts`
  - Rust (conduit): `core/layer2/conduit/src/runtime_paths.rs`

## Conversation Eye (Default)

`conversation_eye` is a default cognition-plane sensory collector:

- Collector source: `client/cognition/adaptive/sensory/eyes/collectors/conversation_eye.ts`
- Synthesizer: `client/runtime/systems/sensory/conversation_eye_synthesizer.ts`
- Bootstrap/auto-provision: `client/runtime/systems/sensory/conversation_eye_bootstrap.ts`

Provisioning contract:

- Every `local:init` run auto-ensures `conversation_eye` exists in the eyes catalog.
- Runtime synthesis output is written to `client/runtime/local/state/memory/conversation_eye/nodes.jsonl`.
- Synthesized nodes are tagged with the conversation taxonomy:
  `conversation`, `decision`, `insight`, `directive`, `t1`.

Conversation hierarchy additions:

- Synthesized nodes now include leveled memory metadata:
  - `node1` (highest), `tag2`, `jot3` (lowest)
- Nodes include deterministic hex IDs and XML-style payload boundaries for low-cost parsing.
- Weekly node admission is quota-bound (10/week default) with bounded level-1 promotion overrides.

## Dream Sequencer + Auto Recall (Memory Integrity)

Memory relevance is continuously reordered through a dream-cycle sequencer:

- Matrix builder: `client/runtime/systems/memory/memory_matrix.ts`
- Sequencer runner: `client/runtime/systems/memory/dream_sequencer.ts`
- Auto recall lane: `client/runtime/systems/memory/memory_auto_recall.ts`

Contracts:

- Tag-memory matrix stores every indexed tag with ranked node IDs and scores.
- Scoring combines memory level (`node1>tag2>jot3`), recency, and dream inclusion signals.
- Dream cycle runs trigger sequencer reorder passes and emit updated ranked tags.
- New memory filings can trigger bounded top-match recall pushes to attention queue through conduit only.

Context guard:

- `memory_recall` query path enforces a hard context budget contract:
  - `--context-budget-tokens` (default `8000`, floor `256`)
  - `--context-budget-mode=trim|reject`
- Trim mode reduces excerpt/summaries to fit budget; reject mode fails closed with `context_budget_exceeded`.

## Low-Burn Reflexes

Shell cognition (repo path `client/cognition/**`) exposes a compact reflex set for frequent operations under strict output caps:

- Registry/runner: `client/cognition/reflexes/index.ts`
- Reflexes: `read_snippet`, `write_quick`, `summarize_brief`, `git_status`, `memory_lookup`
- Each reflex response is capped at `<=150` estimated tokens.

## Why Root Is Governed

Repository root is curated by contract, not by visual minimalism alone.

Canonical source and product roots are still:

- source/runtime authorities (`core/`, `surface/`, shell path `client/`, `adapters/`, `apps/`, `packages/`, `planes/`, `tests/`)
- documentation and governance (`README.md`, `ARCHITECTURE.md`, `docs/`, licenses, roadmap/governance files)
- build/bootstrap/operator metadata (`Cargo.toml`, `package.json`, lockfiles, CI/deploy manifests, `setup/`, `xtask/`)

The live root also contains managed support zones that are intentionally tolerated while the repo hardening program continues:

- generated/build/vendor bulk (`target/`, `node_modules/`, `artifacts/`, `audit_reports/`, `dist/`)
- local operator/runtime state (`local/`, `core/local/`, `client/runtime/local/`)
- research/reference surfaces (`benchmarks/`, `proofs/`, `research/`, `tools/`)

The governing rule is:

- canonical source authority stays inside `core/`, `surface/`, shell path `client/`, `adapters/`, `tests/`, and deletable `apps/`
- per-instance runtime state belongs under `client/runtime/local/` and `core/local/`
- root-level support zones are allowed only when they are explicitly covered by the root-surface contract or tracked as deprecated debt

This means root is reviewable by policy even when it is not cosmetically tiny. Search/navigation tooling should default away from generated bulk, while the root-surface contract continues to burn down deprecated entries over time.

`planes/` is the living architectural contract surface. If code and docs diverge, `planes/*` + this file define the expected target state.

## System Map

```mermaid
flowchart TB
    SUBSTRATE["Exotic/Classic Hardware Substrates"]
    LNEG1["Layer -1: Exotic Hardware Template"]
    L0["Layer 0: Safety Plane (Immutable Origin)"]
    L1["Layer 1: Policy + Deterministic Receipts"]
    L2["Layer 2: Scheduling + Execution"]
    L3["Layer 3: OS Personality Template"]
    CONDUIT["Conduit + Scrambler"]
    ORCH["Orchestration Control Plane"]
    GATEWAY["Gateway Boundary Socket"]
    UI["Presentation Shell Surface"]
    CLI["Operator Surface (infring/infringctl/infringd)"]
    RECEIPTS["Deterministic Receipts + State Artifacts"]

    SUBSTRATE --> LNEG1
    LNEG1 --> L0
    L0 --> L1
    L1 --> L2
    L2 --> L3
    L3 --> CONDUIT
    ORCH --> CONDUIT
    UI --> GATEWAY
    CLI --> GATEWAY
    GATEWAY --> ORCH
    GATEWAY --> CONDUIT
    L0 --> RECEIPTS
    L1 --> RECEIPTS
    L2 --> RECEIPTS
```

Runtime subsystem ownership, interfaces, failure modes, and lane links are tracked in the generated map:

- [Generated System Map](docs/client/architecture/SYSTEM_MAP.md)
- Source registry: `client/runtime/config/system_map_registry.json`
- Generator lane: `client/runtime/systems/ops/system_map_generator.ts`

## Runtime Flow

1. A command enters from CLI or the Presentation Shell.
2. Gateway socket ingress classifies, bounds, normalizes, traces, and admits or rejects the request.
3. Orchestration Control Plane decomposes, coordinates, sequences, recovers, and shapes/packages results through explicit contracts when coordination is required.
4. Conduit normalizes the Kernel-bound request into a typed envelope.
4. Layer 3 maps envelope into deterministic execution intents.
5. Layer 2 schedules execution; Layer 1 enforces policy/receipts.
6. Layer 0 evaluates constitution/safety gates and binds receipts to safety state.
7. Layer -1 executes through the active substrate template with declared fallback behavior.
8. Crossing + validation receipts are emitted for auditability.

## Portability Contract

- With TS present: conduit-backed control-plane orchestration and rich operator surfaces.
- Without TS: Rust Kernel runtime still runs with no behavior drift.

## Related Docs

- [Getting Started](docs/client/GETTING_STARTED.md)
- Conduit Requirement (REQ-05)
- [Rust Primitive Requirement](docs/client/requirements/REQ-08-rust-core-primitives.md)
- [Layered Templates Requirement](docs/client/requirements/REQ-31-layered-templates-and-os-personality.md)
- [Mech-Suit Cockpit Runtime Requirement](docs/client/requirements/REQ-32-mech-suit-cockpit-persistent-push.md)
- [Executable System Map Requirement](docs/client/requirements/REQ-34-executable-system-map-registry.md)
- [Security Posture](docs/client/SECURITY_POSTURE.md)
- [Three-Plane Model](planes/README.md)
- [Three-Plane Formal Spec Surface](planes/spec/README.md)
- [Planes Contract Registry](planes/contracts/README.md)
- [Layer Rulebook](docs/client/architecture/LAYER_RULEBOOK.md)
- [Generated System Map](docs/client/architecture/SYSTEM_MAP.md)
