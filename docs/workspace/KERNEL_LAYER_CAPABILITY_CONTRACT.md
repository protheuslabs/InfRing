# Kernel Layer Capability Contract

Status: hard repo-wide policy

Purpose:
- Keep the kernel layer model aligned with the system goal of shedding downward onto increasingly constrained hardware.
- Prevent capability drift where a lower layer quietly accumulates assumptions that belong in a richer runtime band.

Core law:

> A capability belongs in the lowest layer whose hardware assumptions, runtime dependencies, and semantic weight can support it without avoidable policy inflation.

Lower layers do not need to contain every future capability. They define the smallest survivable capability envelope for a class of hardware and runtime conditions. Higher layers unlock additional capability classes when the hardware and runtime budget make that reasonable.

## Layer Model

| Layer | Intended hardware/runtime band | Unlocks | Must shed when |
| --- | --- | --- | --- |
| `Layer -1` | Exotic or non-standard substrates | Adapters that translate ternary, quantum, biological, or future substrate signals into standard envelopes | The adapter introduces policy, cognition, or upward authority instead of translation-only behavior |
| `Layer 0` | Microcontroller, toaster-class, bare-metal, or near-bare-metal survivable core | Deterministic receipts, bounded task/resource primitives, minimal local storage/transport abstractions, fail-closed safety kernels | The capability requires rich networking, browser semantics, provider orchestration, semantic scoring, or workload assumptions not survivable on constrained devices |
| `Layer 1` | Small edge/embedded/low-power systems with more runtime support than Layer 0 | Generic reusable higher functions, richer storage/isolation, optional network or retrieval primitives | The capability now depends on orchestration policy, evaluation logic, or multi-surface coordination |
| `Layer 2` | Full runtime with orchestration and policy budget | Scheduling, orchestration, semantic interpretation, evaluation, diagnostics, policy truth | The capability requires full OS personality or rich userland service surfaces |
| `Layer 3` | Full OS/userland capability envelope | Process/service/namespace/driver/syscall/windowing/networking personality surfaces | The capability is actually substrate translation or lower-layer authority in disguise |

## Placement Rules

1. Hardware-envelope-first:
   - If a capability assumes browser automation, public-web retrieval, provider credentials, rich TLS/DNS stacks, or heavyweight semantic evaluation, it does not belong in `Layer 0`.
2. Semantic-weight-first:
   - If a module must interpret user intent, score answerability, judge source quality, rewrite domain queries, or run eval-style diagnostics, it belongs above `Layer 0`.
3. Progressive unlocks:
   - A higher layer may depend on lower-layer primitives.
   - A lower layer must not absorb a higher layer's assumptions just because the feature is strategically important.
4. Explicit shed triggers:
   - Every layer should be describable in terms of what gets dropped when hardware or runtime support shrinks.

## Layer 0 Hard Boundary

`Layer 0` is the smallest universal kernel band, not the place where every "core" feature goes.

Layer 0 is for:
- deterministic receipts and envelopes
- bounded resource accounting
- tiny local task primitives
- minimal safety/fail-closed kernels
- small local storage/transport abstractions that still make sense on constrained hardware

Layer 0 is not for:
- web retrieval
- browser automation
- search/provider orchestration
- query rewriting
- freshness/news/time-window interpretation
- source-quality or answerability scoring
- workflow/eval diagnostics
- domain/topic-specific semantic policy

If a capability cannot plausibly survive on microcontroller-class or toaster-class constraints, it should not be introduced into `core/layer0/`.

## Repo Enforcement

This contract is enforced through:
- changed-file placement guard: `ops:layer-placement:check`
- whole-repo placement report: `ops:layer-placement:report`
- layer0 dependency boundary proof: `ops:layer0:dependency-boundary:guard`
- layer capability trend report: `ops:layer-capability:trend:report`
- layer capability trend guard: `ops:layer-capability:trend:guard`
- sentinel policy verification: `ops:ksent:policy:guard`

Authoritative policy/config surfaces:
- [client/runtime/config/layer_placement_policy.json](/Users/jay/.openclaw/workspace/client/runtime/config/layer_placement_policy.json)
- [observability/sentinel/layer_capability_enforcement_policy.json](/Users/jay/.openclaw/workspace/observability/sentinel/layer_capability_enforcement_policy.json)

## Practical Test

When placing or reviewing a capability, ask:

1. Could this survive on a highly constrained target without pretending richer runtime support exists?
2. Does it introduce semantic or evaluation policy instead of a reusable primitive?
3. Is it only "important", or is it actually foundational at this hardware band?

If the answer to (1) is "no" or the answer to (2) is "yes", move it upward.
