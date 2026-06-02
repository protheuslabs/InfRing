# Coding Workflow Experiment Method

Status: active doctrine for native coding workflow development.

Purpose: keep coding-workflow work primitive-first, measurable, and comparable to proven coding systems instead of drifting into one-off patches.

## Non-negotiables

- Build primitives, not eval answers. Test-case-specific behavior is forbidden outside eval fixtures, judges, and reports.
- Treat lower levels as monotonic canaries. A higher-level improvement that breaks lower levels is presumed invalid until proven otherwise.
- Keep external frameworks in the loop. Infring changes must be compared against reference traces from systems such as Codex, Claude Code, Aider, ForgeCode, OpenHands, and any other locally available coding agent harnesses.
- Promote only obvious positive deltas. A single lucky pass is not promotion-grade evidence.
- Quarantine experiments quickly. If a patch adds moving parts without a clear pass-rate, speed, or failure-clarity gain, make it dormant or revert it and record the lesson.
- Measure time as a product property. Wall time, time to first mutation, provider latency, prompt size, system size, observation size, and tool count are part of the result.

## Required loop

1. Establish the current Infring failure shape with receipts and timing.
2. Check comparable external-framework traces for the same level or task shape.
3. Update the unified coding runtime model when a reference behavior is worth assimilating.
4. Patch the smallest primitive that explains the failure across task shapes.
5. Run the affected level plus lower-level canaries.
6. Record the result in the experiment ledger.
7. Promote, quarantine, or revert based on measured delta.

## Promotion standard

A patch can be promoted only when it satisfies all applicable checks:

- It improves the targeted failure or latency in repeated runs, not just one run.
- It does not regress lower-level canaries.
- It does not add eval-level names, paths, fixtures, or task-specific shortcuts to runtime behavior.
- It keeps workflow CD responsible for policy gates and keeps Rust responsible for primitive/tool execution.
- It preserves receipted final claims: no claimed file mutation, validation, or completion without receipts.

## External-framework loop

Reference frameworks are not competitors to optimize directly. They are behavioral probes.

Each meaningful Infring patch round should preserve at least one of these evidence links:

- A current trace from an external framework on the same level.
- A documented model entry that maps the external behavior to an Infring primitive.
- A ledger entry explaining why a reference behavior was rejected.

Useful reference signals include:

- How many model turns occur before the first mutation.
- Whether context is loaded by runtime, prompt, or model tool call.
- Whether the first edit is patch-only, full-file write, shell edit, or mixed.
- Whether failed validation is passed into the first mutation attempt or a later repair turn.
- Whether validation/repair is one compact loop or a broad rediscovery loop.
- How finalization is closed without adding unnecessary provider turns.

Use two loop sizes:

- `per-patch loop`: Infring plus one or two relevant reference frameworks on the active failure band, with bounded per-attempt timeouts and partial report writes.
- `checkpoint loop`: broader sweeps across all available coding frameworks after a patch series reaches a stable checkpoint.

## Patch boundaries

Prefer these primitive-shaped changes:

- Narrowing a tool menu by declared task shape.
- Moving policy from Rust into workflow CD.
- Adding a reusable receipt or evidence packet.
- Adding a bounded recovery lane with explicit activation evidence.
- Improving failure reporting so the parent workflow can continue or ask the user.

Avoid these patch smells:

- Level-specific runtime branches.
- Fixture path checks outside eval code.
- More provider turns to compensate for unclear contracts.
- Broad planning gates for simple mutation tasks.
- Optimizing one level while skipping lower-level canaries.

## Decision labels

Use these labels in the experiment ledger:

- `promoted`: repeated improvement with no lower-level regression.
- `kept_dormant`: useful primitive or trace surface, but not stable in the hot path.
- `reverted`: negative or unclear delta.
- `needs_reference_trace`: patch idea lacks external-framework behavioral evidence.
- `blocked_by_tooling`: primitive cannot be measured because the tool substrate is incomplete.
