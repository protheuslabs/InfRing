# Agentic Implementation Loop Doctrine

Status: active implementation doctrine
Date: 2026-06-09

This document defines how agents should build InfRing.

It is not the permanent vision. It is not a one-shot prompt template. It is the working doctrine for converting vision into implementation without letting agentic coding drift, sprawl, or corrupt subsystem boundaries.

## Core thesis

InfRing should not be built by isolated single prompts.

InfRing should be built by bounded agentic loops:

```text
vision context
  -> checkpoint objective
  -> small implementation slice
  -> constrained execution
  -> validation and purity filters
  -> evidence
  -> promotion, retry, or reject
```

The agent is not trusted because it sounds confident. The agent is trusted only when the loop produces bounded evidence that survives the system's checks.

## Why this matters

Large agent-built systems fail when implementation is guided only by local prompt intent.

Common failure modes:

- The agent follows the immediate prompt but violates the long-term architecture.
- Temporary compatibility paths become permanent.
- Shells and adapters regain authority because they are the shortest path to a visible feature.
- Validation checks become documentation instead of gates.
- The codebase accumulates many partial systems that sound aligned but do not compose.
- The project becomes larger without becoming more useful.

The implementation loop exists to keep each change pointed at the main vision while forcing it through practical constraints.

## Operating rule

Every non-trivial agentic implementation turn should answer five questions:

1. What vision or checkpoint objective does this serve?
2. What is the smallest useful slice?
3. What boundaries must not be crossed?
4. What evidence proves the slice worked?
5. What filter decides whether this should be promoted, retried, or rejected?

If those questions cannot be answered, the work is not ready for autonomous execution.

## The loop

### 1. Orient

Load only the relevant doctrine:

- permanent vision
- next-version checkpoint vision
- subsystem ownership policy
- active TODO or SRS row
- local contract for the surface being touched

Do not hydrate the whole repository into the agent unless the task truly requires it.

### 2. Select the slice

Choose one small objective that improves usefulness, reliability, simplicity, or anti-entropy.

The slice should be small enough that:

- ownership is clear
- affected files are predictable
- expected evidence is knowable before implementation
- rollback is realistic

### 3. Constrain execution

Apply hard constraints before implementation begins.

Examples:

- Shell may render projections but must not own runtime authority.
- Gateway may admit, route, project, budget, and record, but must not become Orchestration, Kernel, Validation, or Sentinel.
- Adapters may translate private schemas but must not own policy.
- Universal tools remain proposal-oriented.
- External runtimes may not silently mutate durable state.
- Context packs are bounded projections, not raw full-state dumps.
- Runtime success and native InfRing intelligence proof stay separate ledgers.

These are not suggestions. They are purity constraints.

### 4. Execute

The agent implements the smallest slice inside the declared constraints.

Preferred execution shape:

```text
read minimum context
plan exact edits
apply edits
produce evidence
stop
```

The agent should not opportunistically refactor unrelated areas, broaden scope, or "clean up" fragile lanes without explicit permission.

### 5. Filter

Run the slice through filters appropriate to the change.

Filter categories:

- boundary filter: did authority stay in the right layer?
- usefulness filter: does this improve real work?
- simplicity filter: did complexity go down or stay justified?
- reliability filter: does failure become clearer or safer?
- evidence filter: is there proof beyond assertion?
- anti-entropy filter: did this reduce drift, duplication, stale state, or hidden coupling?

Filters may be human review, Sentinel review, validation guards, harness runs, proof artifacts, or CI gates.

### 6. Promote, retry, or reject

The loop ends with one of three outcomes:

- `promote`: the slice is useful, bounded, evidenced, and aligned.
- `retry`: the direction is right but evidence or implementation is incomplete.
- `reject`: the slice violates architecture, adds unjustified complexity, or fails its proof.

No change should be treated as successful only because code was written.

## Soft constraints vs hard constraints

Soft constraints shape agent behavior:

- vision docs
- checkpoint docs
- `AGENTS.md`
- `CLAUDE.md`
- SRS notes
- design commentary
- style guidance
- preferred patterns

Hard constraints enforce behavior:

- CI gates
- validation guards
- payload budgets
- route contracts
- permission gates
- sandbox policy
- ownership manifests
- forbidden dependency checks
- proof-pack requirements
- Sentinel-enforced policies

Soft constraints help agents aim. Hard constraints stop agents from drifting.

InfRing needs both.

## Purity checks

Purity checks decide whether implementation stayed true to the intended architecture.

Required purity checks for the current Agent Runtime checkpoint:

- Shell remains projection-only.
- Gateway remains the external membrane.
- Adapter code remains translator-only.
- Kernel authority is not mirrored.
- Orchestration is not bypassed for native workflow authority.
- Universal tools remain minimal and proposal-oriented.
- External engine success is not counted as native InfRing intelligence proof.
- Runtime socket paths remain Gateway-owned.
- Approval pause/resume remains Gateway-owned.
- Durable effects require receipts.
- Raw/full state is not exposed through default projections.

If a change passes tests but fails purity, it should not be promoted.

## Evidence expectations

Useful evidence is compact and inspectable.

Good evidence:

- names the objective
- names the owner or subsystem
- records pass/fail status
- includes trace IDs or artifact refs
- summarizes the important result
- keeps raw logs behind refs
- states known gaps
- can be compared over time

Bad evidence:

- huge raw dumps
- screenshots with no verdict
- logs with no owner
- one-off local claims
- artifacts that cannot be reproduced or compared
- green checks that do not prove the thing users care about

## Agent roles

Different agents may participate in the loop, but their roles must stay distinct.

Possible roles:

- implementer: makes the bounded change
- reviewer: checks correctness and regressions
- sentinel: looks for entropy, drift, duplication, stale policy, and boundary violations
- evaluator: tests task performance and usefulness
- governance guard: enforces contracts and required evidence
- human gatekeeper: approves risky or ambiguous promotion

The same engine may perform multiple roles in a small task, but the evidence should still identify which role is being played.

## Current checkpoint application

For the Unified Agent Runtime checkpoint, the loop should prioritize:

- Codex and Claude Code reliability through InfRing
- context continuity across engines
- approval pause/resume that truly pauses
- transcript persistence for external runtime turns
- decision/activity trace capture
- truthful model/runtime menus
- Gateway-owned runtime sockets
- compact proof artifacts

The loop should avoid:

- promoting every framework at once
- adding cognition to Shell
- making adapters policy owners
- overloading Gateway router logic
- broadening universal tools
- treating prompt-text context hydration as the final architecture

## Relationship to prompts

Prompts still matter, but prompts are not the system.

The durable system is:

```text
doctrine
constraints
tools
permissions
loops
evidence
promotion rules
```

Prompts should start or steer loops. They should not be the only place where architecture, safety, memory, validation, or ownership lives.

## The checkpoint sentence

InfRing implementation should proceed by small, constrained, evidence-producing agentic loops that preserve the main vision while proving real usefulness one promoted slice at a time.
