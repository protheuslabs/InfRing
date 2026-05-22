# Primitive-First Workflow Development Method

Status: hard repo-wide development method  
Scope: workflow CDs, tool CDs, runtime controllers, native agent loops, prompts that define behavior, coding/research eval development, and any production change made in response to an eval failure

Parent doctrine:

- `docs/workspace/primitive_first_system_doctrine.md`
- `docs/workspace/primitive_workflow_composition_doctrine.md`

## Purpose

Infring is intended to grow like an engine, not a pile of task-specific patches.

When a workflow fails an eval, the default repair must not be "patch the symptom." The default repair must be to identify which reusable primitive, composition boundary, tool substrate, or eval contract is missing or wrong.

This method exists to prevent workflow development from drifting into a fragile loop:

1. Run a higher-level eval.
2. Patch a shared runtime branch for that one failure.
3. Accidentally slow or break lower-level behavior.
4. Patch the lower level around the symptom.
5. Repeat until the system becomes a hidden collection of special cases.

That loop is forbidden for production workflow development.

## Core law

Model before patch.

Eval failures are evidence. They are not direct implementation instructions.

Before changing production workflow or runtime behavior in response to an eval failure, classify the failure as one of:

- Missing primitive
- Bad primitive contract
- Bad primitive implementation
- Bad composition boundary
- Bad lane or profile selection
- Bad tool substrate
- Bad validation or receipt contract
- Bad eval fixture or judge
- Real provider/model failure
- Legitimate user-input-needed breakpoint

Only after classification should a patch be made.

Before patching production behavior, identify the failed primitive boundary, compare it against available reference-system behavior, and patch the smallest primitive, contract, profile, adapter, or composition rule that generalizes.

A patch that only makes the immediate eval symptom disappear is not progress unless it also strengthens the reusable model. If the failure cannot be explained as a primitive/model gap, bad boundary, bad tool substrate, bad profile selection, bad eval, or clear local bug, stop and update the model first.

## Required method

### 0. Human-guided method loop

When a human operator is actively guiding workflow development, use the guidance to enforce method discipline, not to skip it.

Default loop:

1. Run one focused eval or reproduction.
2. Classify the failure before patching.
3. Check reference artifacts when the failure resembles a general agent-behavior problem.
4. Patch only the owning primitive, contract, tool substrate, profile, adapter, composition rule, or eval boundary.
5. Rerun the relevant lower-level gate to prove the patch did not poison the primitive spine.
6. Move up only after the lower capability remains stable.

The operator's most important role is to catch patch momentum. If the implementation starts chasing symptoms, adding level-specific branches, routing too much work through heavyweight paths, or weakening lower-level behavior to satisfy a higher-level eval, stop and return to failure classification.

This process is part of the development method. It is not optional ceremony.

### 1. Reference before invention

When building or repairing important workflow behavior, first check relevant successful systems or stored reference artifacts when available.

For coding workflows, preferred references include successful coding-agent systems and local artifacts already collected from them, such as Codex-style patch loops, OpenHands-style sandbox trajectories, ForgeCode-style planning/execution/review behavior, aider-style diff discipline, and other repo-local coding-agent traces.

Reference artifacts should answer:

- What primitive behavior does the successful system use?
- What does it do before reading files?
- What does it do before writing files?
- How does it decide whether context is needed?
- How does it recover from invalid output?
- How does it validate completion?
- How does it summarize uncertainty, failure, or user-input needs?
- What is its fast path for simple work?

Do not copy surface wording when the useful artifact is the underlying primitive behavior.

### 2. Build the runtime model

Describe the intended behavior as a reusable model before editing production paths.

For coding workflows, the canonical model should decompose into reusable stages:

1. Intent classification
2. Project state classification
3. Context acquisition
4. Task planning
5. Executable manifest or patch artifact creation
6. Tool execution
7. Receipt journaling
8. Validation
9. Semantic completion check
10. Bounded repair
11. Final receipt-backed synthesis

If a stage is not needed for a simple task, the model must allow a cheaper path.

For coding workflows, the current canonical execution spine is:

```text
fast task/intent contract
-> compact executable manifest or patch artifact
-> deterministic native tool execution
-> receipt and evidence capture
-> validation or semantic completion checks when required
-> bounded strong-model repair only after concrete failure evidence
-> final receipt-backed synthesis
```

The strong model is a repair/planning resource, not a blanket replacement for the fast path. Escalation to a stronger model must be triggered by evidence such as malformed manifest output, missing mutation receipts, failed validation, failed public-interface checks, repeated no-progress actions, or user-owned ambiguity.

### 3. Convert model gaps into primitive contracts

Each missing behavior should become one of:

- A primitive
- A composite over declared primitives
- A lane selection contract
- A tool contract
- A schema validator
- A profile/config surface
- An adapter capability declaration
- An eval-only fixture or judge rule

The primitive or contract must define:

- Inputs
- Outputs
- Receipts or evidence
- Failure modes
- Retry or repair policy
- Stop conditions
- Non-goals
- Extension surface
- Speed budget when latency matters

### 4. Patch only the owning boundary

Patch the smallest correct owner.

Allowed examples:

- Tool implementation bug -> tool substrate
- Receipt missing for all file writes -> file tool receipt contract
- Existing-project discovery missing -> context acquisition primitive
- Repair loop too vague -> bounded repair primitive
- Simple create-file task too slow -> lane selection or fast-path primitive
- Level-specific judge mismatch -> eval fixture or judge

Forbidden examples:

- Add benchmark names to runtime code.
- Add prompt-phrase branches to a global controller.
- Make all tasks run high-level planning because one hard eval needed planning.
- Add one fixture's file layout to a shared primitive.
- Patch a lower-level workflow around a regression caused by higher-level behavior.
- Route every task through the heaviest model because one task shape needed stronger reasoning.

### 5. Preserve monotonic gates

Higher-level improvements must not regress lower-level behavior.

Before treating a higher-level workflow change as progress, rerun the relevant lower-level gates for the same capability family.

If a Level `N` change breaks Level `0..N-1`, assume the abstraction boundary is wrong until proven otherwise.

### 6. Treat speed as part of correctness

A workflow that is logically correct but takes the heavyweight path for trivial work is still structurally suspect.

Tiny tasks must have cheap paths. Larger tasks may use heavier planning, discovery, memory, and checkpoint loops.

Workflow changes should declare expected latency class when speed matters:

- Fast path: context-free or single-file tasks
- Normal path: small existing-project edits
- Heavy path: multi-file, validation-heavy, or checkpointed work
- Long-horizon path: project-scale work with memory, roadmap, and stop conditions

### 7. Use traces, not vibes

When behavior is unclear, collect or inspect runtime traces from Infring and reference systems.

The trace comparison question is:

"Which primitive behavior does the reference system have that Infring lacks or composes incorrectly?"

The answer should become a primitive/contract/model update, not a one-off prompt patch.

### 8. Quarantine eval specificity

Eval-specific details belong only in eval fixtures, judges, golden files, reproduction cases, or test-only harnesses.

They must not leak into:

- Production prompts
- Workflow CDs
- Runtime controllers
- Native tool selection
- Primitive contracts
- Shared policy code

## Coding workflow application

For native coding work, the method is:

1. Keep Level 1 simple create/mutate tasks fast and boring.
2. Stabilize Level 2 existing-project edits through generic context, manifest, mutation, validation, and repair primitives.
3. Do not make Level 2 depend on Level 8 project-memory behavior.
4. Do not make Level 1 depend on existing-project discovery.
5. Promote a higher level only after lower relevant levels still pass.
6. If repeated patching does not improve reliability, stop and rebuild the runtime model from reference traces.
7. Prefer fast-manifest generation plus deterministic execution before strong-model repair; do not use strong-first routing as the default fix for validation or multi-file failures.

## Required change-review questions

Before merging a workflow/runtime patch, ask:

- What failure class does this patch address?
- Which primitive or composition boundary owns the fix?
- Is this based on a reference artifact, a documented model gap, or a clear local bug?
- Would this behavior still be correct outside the current eval?
- Could this slow or break a lower-level path?
- Is any specificity quarantined to eval/test boundaries?
- Does the change create or improve a reusable contract?
- Is there a cheaper path for simple tasks?
- What receipt, validation, or failure artifact proves the behavior?

## Stop rule

If the same level keeps failing after multiple local patches, stop patching and do a method reset:

1. Reclassify failures.
2. Compare Infring traces against reference traces.
3. Update the behavioral model.
4. Identify missing primitives or bad composition boundaries.
5. Patch only after the owning boundary is clear.

Spinning on patches without model updates is method regression.
