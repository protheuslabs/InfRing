# Primitive-First System Doctrine

Status: hard repo-wide policy  
Scope: production code, runtime prompts, policies, contracts, schemas, adapters, tools, workflows, Shell surfaces, Kernel authority, Orchestration, Gateways, docs that define behavior, and eval/test boundaries

## Purpose

Infring must grow from reusable primitives into more specific capabilities. Specific behavior can be built on top of primitives, but it must not be hidden inside production code as one-off case logic.

The system should become easier to reason about as it grows. Hardcoded special cases do the opposite: they make one eval or scenario pass while making the foundation less general, less testable, and more likely to regress lower-level behavior.

## Core law

Production behavior must be primitive first.

Hardcoding behavior for a specific case is forbidden in production paths unless the behavior is expressed through a reusable primitive, declared contract, configuration surface, profile, adapter boundary, policy, schema validator, or explicit composition layer.

Specific uses must be built on top of primitives. They must not be embedded inside the primitive.

## Forbidden production hardcoding

Production code, prompts, workflow CDs, Tool CDs, adapters, Shell logic, Kernel authority, Gateway logic, docs that define runtime behavior, and orchestration policies must not branch on a narrow case such as:

- A benchmark name or eval level
- A prompt phrase or canned request wording
- A fixture file name
- A generated app shape
- A specific language or framework example
- A named demo object or test scenario
- A provider-specific example that should be adapter/config metadata
- A local path shape that only exists for one test
- A magic output phrase that exists only to satisfy a verifier
- A one-off source-code structure unless the structure is declared by a general schema or contract

These belong in tests, eval fixtures, docs examples, or adapter/profile/config layers, not hidden production logic.

## Allowed exception for tests and evals

Hardcoding is allowed inside explicit test and eval boundaries.

Allowed examples:

- Test fixture file names
- Expected output strings
- Eval prompts
- Scoring rubrics
- Reproduction cases for a known bug
- Benchmark-specific assertions
- Golden files
- Controlled failure injections

Boundary rule:

Hardcoded test/eval details must not leak into production runtime behavior, production prompts, shared controllers, primitive contracts, or generic policy code.

## Correct abstraction path

When a narrow case seems to require a special branch, do this instead:

1. Name the underlying general capability.
2. Decide the smallest owning layer for that capability.
3. Implement or extend a primitive with a bounded contract.
4. Expose specificity through data, configuration, profile packs, adapters, or a composite workflow.
5. Add tests or evals for the specific case at the boundary.
6. Keep lower-level primitives valid for unrelated consumers.

The default answer to "this one case needs special handling" is not "add a case branch." The default answer is "what primitive is missing?"

Canonical promotion policy:

- `docs/workspace/special_case_promotion_policy.md`

Canonical primitive registry policy:

- `docs/workspace/primitive_capability_registry_policy.md`

Canonical primitive registry:

- `validation/conformance/contracts/primitive_capability_registry.json`

Canonical workflow development method:

- `docs/workspace/primitive_first_workflow_development_method.md`

## Acceptable production forms

Specific behavior can exist in production only when represented as one of:

- A reusable primitive
- A declared composite over primitives
- A typed workflow or Tool CD contract
- A schema validator
- A policy that applies to a class of cases
- A profile pack or configuration file
- An adapter capability declaration
- A data-driven rule with a declared owner and extension path
- A safety gate whose condition is general and auditable

## Repo-wide application

Kernel/Core compatibility paths:

- Own primitive authority, validation, receipts, safety, and execution boundaries.
- Must not absorb product, benchmark, or prompt-specific behavior as hidden branches.

Orchestration:

- Owns coordination, workflow selection, composition, and stop conditions.
- Must keep workflow-specific doctrine out of generic runtime quality unless represented as a workflow-scoped extension.

Gateways and adapters:

- Own external membrane details and provider-specific translation.
- May encode provider capabilities as adapter metadata.
- Must not let provider examples become generic system behavior.

Shell and client surfaces:

- Own projection and interaction.
- Must not become authority for policy, routing, hidden state, or special-case behavior.

Validation and evals:

- May hardcode fixtures, prompts, expected outputs, and scoring rules.
- Must keep those details quarantined from production logic.

Documentation:

- Normative docs must describe primitives, contracts, policies, and composition boundaries.
- Example docs may use specific cases, but examples are not production authority.

## Monotonicity requirement

Higher-level changes must be monotonic over lower-level primitives.

If a high-level feature, eval level, workflow, or product-specific behavior breaks a lower-level path, treat it as an abstraction failure first. Do not patch the lower level around the symptom until the owning boundary is clear.

## Development method requirement

Workflow and runtime development must follow the Primitive-First Workflow Development Method.

Eval failures are evidence, not direct patch instructions. Before production behavior is changed in response to an eval failure, the failure must be classified as a missing primitive, bad primitive contract, bad primitive implementation, bad composition boundary, bad lane/profile selection, bad tool substrate, bad validation/receipt contract, bad eval/judge, real provider/model failure, or legitimate user-input-needed breakpoint.

When reference artifacts from successful systems are available, use them before inventing new behavior. The useful artifact is the underlying primitive behavior, not the surface wording.

Patching without updating the model, primitive contract, composition boundary, tool contract, profile, or eval boundary is method regression unless the patch is an obvious local implementation bug.

## Review checklist

Before merging or promoting a change, ask:

- Is this behavior primitive-first?
- Does this belong in the engine, or should it be content/config/profile/eval-specific behavior built on top of the engine?
- Is the specificity represented as data, config, profile, adapter metadata, or composition rather than a hidden branch?
- Would this change still make sense outside the current benchmark or prompt?
- Could a lower-level consumer be slowed, narrowed, or broken by this behavior?
- Are hardcoded details confined to tests, evals, examples, or fixtures?
- Does the production path expose a general contract that future specific uses can build on?
- Was any eval failure classified before production behavior changed?
- Did the patch update the correct model, primitive, composition boundary, tool contract, profile, or eval boundary rather than only the symptom?
- If reference artifacts exist for this behavior, were they checked or was the reason for invention documented?
- Does the change preserve a cheap path for simple tasks when speed matters?

## Violation signals

Treat these as doctrine violations:

- Production code checks for a named eval, benchmark, fixture, or prompt phrase.
- A global prompt or controller contains case-specific instructions for one task shape.
- A primitive becomes a pile of special cases from higher-level composites.
- A test fixture shape becomes a runtime assumption.
- A provider example becomes a generic tool contract.
- Lower-level evals regress after a higher-level patch.
- Simple tasks become slow because high-level behavior always runs.
- The only way to change behavior is to edit Rust or TypeScript case branches instead of changing a contract, policy, config, adapter declaration, or workflow/tool CD.
- Eval failures trigger repeated production patches without failure classification or model/contract updates.
- A successful external-system pattern is copied as prompt wording instead of being converted into a reusable primitive behavior.

When these signals appear, stop adding special cases and repair the abstraction boundary.
