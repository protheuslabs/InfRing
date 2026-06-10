# Next Version Vision: Unified Agent Runtime Checkpoint

Status: active checkpoint vision
Date: 2026-06-09

This document is not the permanent InfRing vision. The permanent vision lives in `INF_RING_VISION.md`.

This document defines the next major checkpoint: what the next version of InfRing must prove before the project should widen its ambition again.

## Checkpoint thesis

The next version of InfRing should prove that InfRing can be the coordinating substrate for multiple agent runtimes.

The system does not need to prove native recursive self-improvement yet. It does not need to out-code Codex, Claude Code, Grok Code, OpenCode, OpenClaw, Hermes, or other agent frameworks yet.

It needs to prove this:

```text
External agent frameworks can run through InfRing
with their useful native capabilities preserved,
while InfRing adds unified context, memory, auditability,
permission gates, receipts, model/runtime control, and observability.
```

The near-term win is not "InfRing is the smartest brain."

The near-term win is:

```text
InfRing is the safe operating face for many brains.
```

## Operating sequence

The ordering remains:

```text
First become useful with other brains.
Then make InfRing-native orchestration useful.
Then make InfRing smarter.
Then grow, tune, quantize, and manipulate native models.
Then pursue safe recursive self-improvement.
```

This checkpoint is the first line of that sequence.

Implementation for this checkpoint should follow the loop doctrine in `AGENTIC_IMPLEMENTATION_LOOP.md`: durable vision context, small slices, hard constraints, purity filters, compact evidence, and explicit promotion or rejection. The system should not rely on single large prompts as the primary implementation mechanism.

## Primary success target

The next version should make Codex and Claude Code reliable golden-path engines inside InfRing.

They should be able to run through InfRing with enough parity that a user can reasonably prefer using them through InfRing because InfRing adds:

- unified conversation continuity
- shared memory and context projections
- permission pause/resume
- artifact proposal and receipt flow
- working-directory control
- model/runtime selection
- visible activity and decision traces
- failure reporting
- cross-engine auditability

If Codex and Claude Code cannot be trusted as daily-driver engines through InfRing, the checkpoint is not complete.

## Secondary success target

After the golden pair is reliable, InfRing may widen to one secondary comparison engine.

Candidate secondary engines include:

- Grok Code
- OpenCode
- OpenClaw
- Hermes Agent

Other engines may remain cataloged, but cataloged does not mean promoted.

## Accounting rule

External-engine success and native InfRing intelligence proof must stay separate.

```text
External engine mediated work = substrate/platform proof.
InfRing-native useful work = native intelligence/runtime proof.
```

Both matter. They are not the same.

The next version is mainly substrate/platform proof.

Native InfRing useful-work proof should continue to exist as its own scoreboard, but it must not block this checkpoint unless it breaks the shared substrate.

## What must be true

The next version is ready when these are boringly reliable:

- A user can switch between Codex and Claude Code without losing conversation continuity.
- The active engine receives bounded InfRing context instead of starting cold.
- User messages and external-engine responses persist exactly like native messages.
- Permission-required actions pause the turn instead of ending as inert text.
- Approvals resume the same turn or produce a clear denial receipt.
- Approved artifact proposals are applied by Gateway, not by Shell.
- Denied proposals are visible, understandable, and auditable.
- The thinking/activity surface shows useful runtime activity, not generic event labels.
- Decision/dialog traces are preserved in a collapsible post-turn summary.
- Model menus are truthful for the selected runtime.
- Cloud, local, downloadable, installed, updatable, blocked, and auth-required states are distinguishable.
- Hard failures produce user-visible terminal states.
- No Shell direct-to-engine, Shell direct-to-adapter, Shell direct-to-Kernel, or Shell direct-to-Orchestration path is required.
- Gateway remains the membrane for runtime sockets, projections, approvals, and payload budgets.
- Adapters remain translators, not policy owners.
- Universal tools remain small and proposal-oriented.
- Live proof artifacts exist for harness runs and are easy to inspect.

## Minimum proof artifacts

The checkpoint should produce or retain compact evidence for:

- agent runtime task harness result
- live work replay result
- engine scorecard
- approval pause/resume proof
- context continuity proof
- transcript persistence proof
- hard failure injection proof
- model projection proof
- framework capability catalog
- Gateway/adapters ownership guard

The proof artifacts should be summaries with refs, not raw noise dumps.

## Non-goals for this checkpoint

The next version should not try to do everything.

Non-goals:

- Do not prove safe RSI.
- Do not train or quantize native specialist models.
- Do not manipulate KV cache or raw latent context.
- Do not make every external engine first-class at once.
- Do not universalize rich research, workflow, browser, shell, or terminal tools across all engines.
- Do not move cognition or authority into the dashboard.
- Do not treat prompt-text hydration as the final context transport.
- Do not let direct-write grants become a universal tool.
- Do not let Gateway router logic become planning, policy, memory, or validation.
- Do not let adapter compatibility exceptions become permanent.

## Golden-path user story

A user should be able to:

1. Open InfRing.
2. Choose Codex or Claude Code from the runtime menu.
3. Choose a valid model exposed by that runtime.
4. Ask for useful coding work.
5. See concise activity while the engine works.
6. Approve or deny risky actions in the chat flow.
7. Receive completed work or a clear blocked/failure state.
8. Switch engines and continue the same conversation.
9. Inspect what happened through receipts, traces, and summaries.

The experience should feel like using the native framework, but with InfRing's memory, auditability, permissions, and control wrapped around it.

## Boundary doctrine

The system shape for this checkpoint is:

```text
Shell or external client
  -> Gateway runtime socket
  -> Gateway agent runtime router
  -> adapter translator
  -> selected engine
```

The forbidden shape is:

```text
Shell
  -> adapter
  -> engine
```

or:

```text
Shell
  -> Orchestration / Kernel
```

The Shell renders projections and submits user decisions. It must not own runtime policy, memory authority, permission reconstruction, model truth, or engine transport.

## Current readiness estimate

As of this checkpoint draft:

- InfRing as safe coordinating substrate: about 60 percent.
- External framework parity through InfRing: about 40 to 45 percent.
- Codex and Claude Code as practical golden pair: improving, but not yet boringly reliable.
- Any-framework runtime neutrality: early.
- Native InfRing intelligence proof: separate and still immature.

These estimates are directional, not release gates. The release gates should be evidence artifacts and harness results.

## Highest-risk failure modes

The next version fails if:

- External engines work only by bypassing InfRing.
- Approval requests appear after the engine has already ended the turn.
- Context continuity depends on fragile prompt stuffing with no path toward structured transport.
- Messages from external engines are visual-only and do not persist as canonical conversation records.
- Gateway becomes a hidden planner/policy/memory owner.
- Adapters become business logic or authority surfaces.
- Shell regains cognition or direct runtime authority.
- Model/runtime menus show stale, fake, or misleading choices.
- Proof artifacts are absent, huge, noisy, or not comparable over time.
- The engine catalog grows faster than the golden path becomes reliable.

## Promotion rule

Do not promote another engine to first-class until Codex and Claude Code pass the core harness reliably.

Do not widen the universal tool surface until the small proposal-oriented surface is boringly safe.

Do not start the next intelligence-heavy phase until this substrate phase is useful in normal work.

## The checkpoint sentence

The next version is successful when InfRing can run Codex and Claude Code through Gateway as reliable daily-driver engines, preserve their useful native behavior, and add InfRing's unified memory, auditability, permissions, receipts, and control without making the Shell or adapters authority owners.
