# Sentinel Agentic Loop Enforcement Policy

Status: active Sentinel policy
Date: 2026-06-09
Owner domain: observability.sentinel

Sentinel should treat the Agentic Implementation Loop as an anti-entropy mechanism.

## Purpose

Sentinel should flag non-trivial implementation work that cannot answer:

- What checkpoint objective does this serve?
- What owner or lane is responsible?
- What boundary constraints applied?
- What evidence proves the slice worked?
- What promotion outcome was chosen?

This is not a blocker for tiny typo or comment-only changes. It is a drift detector for meaningful system work.

## Finding shape

When Sentinel detects missing loop discipline, it should emit a compact finding with:

- `finding_type`: `agentic_loop_gap`
- `checkpoint_objective`
- `owner_guess`
- `affected_lane`
- `missing_fields`
- `boundary_risk`
- `evidence_gap`
- `root_cause_hypothesis`
- `next_action`
- `promotion_state`: `retry` or `reject`

## Red conditions

Sentinel should classify the finding as red when:

- work touched Shell, Gateway, adapters, Kernel, Orchestration, permissions, runtime sockets, model routing, CI, or release gates without explicit boundary constraints
- durable effects occurred without evidence or receipts
- external-engine success was counted as native InfRing proof
- Shell or adapter code gained authority that belongs to Gateway, Kernel, Orchestration, Validation, or Observability
- the work has no rollback plan and touches a fragile lane

## Yellow conditions

Sentinel should classify the finding as yellow when:

- the objective is clear but evidence is weak
- allowed lanes are clear but forbidden lanes are missing
- the work has evidence but no promotion state
- the work is useful but increases complexity without an offsetting simplification or anti-entropy note

## White/pass conditions

Sentinel should treat the loop as satisfied when the work has:

- checkpoint objective
- smallest slice
- owner or lane
- allowed and forbidden lanes
- boundary constraints
- evidence
- promotion rule or promotion outcome
- compact proof refs where applicable

## Relationship to validation

Sentinel detects drift and recommends action. Validation guards enforce minimum documentation and policy presence.

The canonical loop doctrine is:

```text
docs/workspace/AGENTIC_IMPLEMENTATION_LOOP.md
```

The next-version checkpoint is:

```text
docs/workspace/NEXT_VERSION_VISION.md
```
