# Primitive Workflow Composition Doctrine

Status: active  
Scope: workflow CDs, native coding workflows, workflow composition ledgers, and coding eval promotion

Repo-wide parent doctrine: `docs/workspace/primitive_first_system_doctrine.md`

Required development method: `docs/workspace/primitive_first_workflow_development_method.md`

## Purpose

Infring workflows must be built from abstract, reusable building blocks. A higher-level workflow may compose lower-level workflows, route between them, and add policy around them, but it must not contaminate the lower-level primitives it depends on.

This doctrine exists because a higher-level coding eval regression exposed the failure mode clearly: trying to make a harder workflow level pass by patching shared primitive behavior can break easier workflow levels. That is not just an eval failure. It is an architecture violation.

## Core law

Higher-level workflow changes must be monotonic over lower-level primitives.

That means a change made for Level `N` must not make Level `0..N-1` behavior:

- Less general
- Less reliable
- Slower on tasks that do not need the new behavior
- More dependent on benchmark-specific wording
- More dependent on hidden runtime hardcoding
- More likely to skip a previously valid primitive path
- More likely to require context, validation, memory, or checkpoints when the lower-level task does not need them

If a higher-level change breaks a lower-level workflow or eval, assume the composition boundary is wrong until proven otherwise.

## Primitive-first anti-hardcoding rule

Production workflow, tool, and runtime behavior must follow the repo-wide Primitive-First System Doctrine.

Hardcoding behavior for a specific benchmark, prompt phrase, file name, fixture shape, app type, language, framework, or narrow case is forbidden unless that hardcoding lives only inside an explicit eval or test fixture.

Allowed exception:

- Eval cases may hardcode expected inputs, expected outputs, fixture file names, scoring rules, and failure assertions.
- Test cases may hardcode reproduction details needed to prove a specific bug is fixed.
- Those hardcoded details must not leak into production workflow JSON, runtime prompts, native tool selection, shared controllers, or primitive workflow contracts.

Production behavior must instead be expressed as one of:

- A reusable primitive workflow
- A composite workflow that routes between primitives
- A declared lane with a general activation rule
- A tool contract or schema validator
- A safety policy that applies across a class of cases
- A data/config contract that can be extended without editing runtime code

If a specific case appears to require hardcoding, the default repair is to name the underlying general capability and build that primitive first. More specific uses must be built on top of the primitive, not embedded inside it.

## Definitions

Primitive workflow:

- Level `0`
- Does one bounded thing
- Has no child workflows
- Owns no benchmark-specific policy
- Is useful outside the current composite
- Can be tested without knowing which higher-level workflow called it

Composite workflow:

- Level `N > 0`
- Calls child workflows
- Owns sequencing, routing, stop conditions, checkpoint policy, and cross-slice coordination for its level
- Must depend on child contracts rather than child internals
- Must not rewrite primitive semantics to satisfy a higher-level target

Workflow level:

- `level = max(child.workflow_composition.primitive_level) + 1`
- Adding more Level `0` primitives does not raise a composite level unless it changes the highest child level used

Lane:

- A declared routing path that chooses a workflow, tool family, or policy branch
- May select primitives based on task shape
- Must not silently mutate primitive behavior globally

## Non-regression rule

Lower levels are foundation tests. They must stay boring.

Before promoting or declaring progress on a higher-level coding workflow, the relevant lower-level gates must still pass. If Level `8` work breaks Level `1` or Level `2`, the fix is not to patch Level `1` around the new bug. The fix is to isolate the Level `8` behavior behind a higher-level composite, a new primitive, or an explicit lane.

## Correct repair pattern

When higher-level work causes lower-level regression:

1. Stop treating it as a local prompt bug.
2. Identify which shared primitive, runtime prompt, or tool path absorbed level-specific behavior.
3. Move level-specific policy into the owning composite workflow or an explicitly named child workflow.
4. Keep the primitive generic and narrow.
5. Add or update regression gates so the lower level cannot silently degrade again.

## Method gate for eval-driven changes

Do not treat a failed workflow eval as a direct instruction to patch runtime behavior.

Before changing production workflow, prompt, tool, or runtime behavior because of an eval result:

1. Classify the failure using the Primitive-First Workflow Development Method.
2. Check available reference artifacts from successful systems when the behavior is non-trivial.
3. Update the behavioral model or primitive contract if the failure reveals a missing abstraction.
4. Patch only the smallest owning boundary.
5. Rerun lower-level gates for the same capability family before promoting the higher-level result.

Repeated patch/rerun cycles without model or contract updates are composition-method regressions.

## Allowed primitive strengthening

A primitive may be improved if the improvement is true for all consumers.

Allowed examples:

- Safer file write receipts
- Better error reporting
- More deterministic tool selection
- Clearer failure artifacts
- A faster path for genuinely context-free new-file tasks

Not allowed examples:

- A Level `8` project-memory rule added to the global edit loop
- A benchmark-specific file shape encoded in Rust or global prompt text
- A validation requirement that makes simple new-file tasks wait on unnecessary discovery
- A source-code preservation rule that only makes sense for one generated fixture but changes all patch tasks
- A final-output rule that claims success without the primitive receipts the lower-level workflow requires
- A prompt phrase such as "hello world", "task router", or "multi-module app" causing hidden production behavior that is not declared as a reusable lane or workflow contract

## CD and runtime boundary

Workflow JSON is the CD. Rust is the CD player.

Therefore:

- Workflow interaction behavior belongs in workflow JSON.
- Runtime code may implement tools, validate schemas, bind receipts, and enforce Kernel policy.
- Runtime code must not secretly route, specialize, or rewrite workflow semantics for a higher-level benchmark.
- If changing a workflow JSON file cannot change the behavior without editing Rust, that behavior is migration debt unless it is a true primitive/tool implementation, safety policy, or schema validator.

## Coding workflow application

For coding workflows:

- The higher-level coding operator owns project initialization, architecture/stack decisions, checkpoint sizing, memory policy, discovery policy, and multi-slice stop conditions.
- The primitive coding execution loop owns one bounded edit/proof slice.
- Project bootstrap, checkpoint-memory bootstrap, and first mutation must be separately owned primitive stages when the coding operator needs all three.
- File tools, command tools, receipts, and validation repair must remain generic primitives.
- Level-specific eval capabilities should become explicit primitives or composites, not hidden branches in the shared controller.
- A live coding eval failure must report whether it is a primitive failure, composite-routing failure, tool failure, validation failure, or user-input-needed breakpoint.

Current coding split reference: `docs/workspace/coding_workflow_primitive_split.md`.

## Promotion checklist

Before promoting a workflow or declaring a higher-level coding improvement:

- The workflow level is correct in `workflow_composition.primitive_level`.
- Child workflow IDs are declared in the composition contract.
- New behavior is owned by the smallest correct workflow level.
- Case-specific hardcoding exists only in eval/test fixtures, never in production runtime, prompts, or primitive contracts.
- Eval-driven changes classify the failure before patching production behavior.
- Non-trivial workflow behavior is checked against available reference artifacts or documents why invention is required.
- Patches update the owning model, primitive, composition boundary, tool contract, profile, or eval boundary.
- Lower-level gates that depend on the same primitives still pass.
- No benchmark-specific semantics were added to global runtime prompt policy.
- Any new primitive has a bounded contract and can be tested independently.
- Final answers are backed by receipts or explicit failure artifacts.

## Doctrine violation signal

These are architecture smells, not normal local bugs:

- A higher-level eval fix makes Level `1` or Level `2` fail.
- A primitive accumulates task-family policy from multiple composites.
- A workflow can only pass when Rust recognizes prompt wording that the workflow JSON does not declare.
- Simple tasks become slow because high-level memory, discovery, or checkpoint policy always runs.
- Regression fixes repeatedly patch symptoms instead of moving behavior to the owning workflow level.
- A production path branches on a named eval case, fixture file, or benchmark-specific object shape.

When these signals appear, repair the composition boundary before adding more special cases.
