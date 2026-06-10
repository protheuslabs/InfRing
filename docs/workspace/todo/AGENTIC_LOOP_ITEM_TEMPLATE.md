# Agentic Loop TODO Item Template

Use this template for non-trivial TODO/SRS work items that an agent may execute.

```md
## Title

Status:
Owner:
Priority:
Checkpoint:

Objective:

Smallest slice:

Allowed lanes:

Forbidden lanes:

Boundary constraints:

Evidence:

Promotion rule:

Known risks:

Rollback plan:
```

## Field guidance

- `Checkpoint` should name the active vision/checkpoint document or SRS row.
- `Objective` should explain why this work matters.
- `Smallest slice` should keep the work bounded enough to review and roll back.
- `Allowed lanes` should name the domains the agent may touch.
- `Forbidden lanes` should name fragile or out-of-scope domains.
- `Boundary constraints` should state the purity rules that must not be violated.
- `Evidence` should describe the proof required before promotion.
- `Promotion rule` should say what makes the result `promote`, `retry`, or `reject`.
- `Known risks` should name likely drift, coupling, or regression modes.
- `Rollback plan` should state how to undo the slice if it fails.

## Minimal inline form

For compact registry rows, use this minimum shape:

```text
Objective:
Smallest slice:
Allowed lanes:
Forbidden lanes:
Evidence:
Promotion rule:
```
