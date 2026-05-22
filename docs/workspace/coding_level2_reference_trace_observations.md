# Coding Level 2 Reference Trace Observations

Status: fresh reference capture  
Generated from: `references/coding-agent-systems/runtime_trace_harness/level2_reference_runtime_trace.py`  
Trace artifact: `references/coding-agent-systems/level2_reference_runtime_trace_observations.json`

## Purpose

This capture compares Infring's native Level 2 coding failures against local reference coding-agent systems.

The goal is not to claim every reference repo was run as a full external LLM coding agent. Most local checkouts expose source contracts, tool surfaces, benchmark templates, or runtime controllers rather than a cheap credential-free agent entrypoint.

The useful comparison is primitive shape:

```text
context or contract selection
-> bounded mutation
-> validation receipt
-> semantic completion probe
-> repair or terminal artifact
```

## Systems covered

- mini-SWE-agent
- SWE-agent
- Aider
- OpenHands
- Cline
- Continue
- SWE-ReX
- Goose
- Roo Code
- ForgeCode

## Execution depth

Only `mini-SWE-agent` produced a live provider-free Level 2 task trace in this pass.

That trace completed the `multi_requirement_vertical_slice` task shape through:

1. Existing context read
2. Mutation
3. Validation command
4. Public API semantic probe
5. Terminal submit
6. Trajectory persistence

The remaining systems were captured as source/contract projections from their local artifacts.

This distinction matters: use the reference output to compare runtime topology and primitive boundaries, not raw pass rate.

## Key reference finding

The strongest shared pattern is:

```text
validation passing is not the same as task completion
```

Reference systems expose or imply a distinct semantic completion/progress gate after mutation and validation.

Examples:

- mini-SWE-agent: explicit action/observation loop plus trajectory persistence.
- SWE-agent: discovery, exact edit, validation, and submit are distinct tool surfaces.
- Aider: context selection, diff discipline, and lint/test feedback are separate repair signals.
- OpenHands: actions, observations, state, stuck detection, and runtime boundary are separate.
- ForgeCode: validation result contracts, retry reflection, doom-loop interruption, and pending todo completion gates are first-class.

## Direct comparison to Infring Level 2 20-run

Latest Infring Level 2 result:

```text
15/20 pass
```

Observed failure classes:

- `provider_timeout_no_mutation`
- `semantic_probe_after_validation_failed`
- `public_api_shape_or_placement_miss`

Reference comparison:

- Timeout should become a structured recovery or terminal artifact with a cheaper continuation path, not a full run that produces no mutation.
- Semantic probes should be explicit post-validation actions inside the workflow/runtime, not only hidden judge checks.
- Public API shape should be verified before finalization, especially owner-module placement, callable-vs-property shape, and result-object fields.

## What this says we should build next

The next primitive should be:

```text
post_validation_semantic_completion_probe
```

Contract:

- Inputs: task contract, changed files, validation command/result, public API requirements, semantic probe command or generated probe spec.
- Outputs: semantic probe receipt, completion verdict, repair hints, terminal artifact.
- Failure modes: probe failed, missing public symbol, wrong callable/property shape, wrong return type, provider timeout, missing mutation receipts.
- Repair policy: one bounded repair attempt using probe stderr/stdout plus changed-file context.
- Non-goals: no benchmark-specific branches, no fixture-specific names in production, no hidden success without receipts.

The timeout side should become:

```text
provider_timeout_recovery_artifact
```

Contract:

- If provider times out before mutation, emit a structured partial artifact.
- Prefer a cheaper continuation prompt or fallback lane over simply failing after the full model budget.
- Preserve the no-fake-success rule: no mutation receipt means no implementation success.

## Method implication

Do not patch Level 2 by adding more task-specific prompt hints.

Patch the primitive boundary:

```text
model_manifest_planner
-> mutation
-> validation
-> semantic probe
-> bounded semantic repair
-> final receipt synthesis
```

The reference traces support this as a general coding-agent runtime pattern, not a Level 2-only fix.

