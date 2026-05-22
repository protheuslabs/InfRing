# Coding Primitive Foundation Reset

Status: active reset ledger  
Parent doctrine: `docs/workspace/primitive_first_system_doctrine.md`
Current reset spine: `docs/workspace/coding_primitive_spine_reset.md`

## Purpose

This ledger tracks the reset from level-chasing coding behavior toward an engine-grade primitive foundation.

The coding workflow should be rebuilt upward from reusable primitives, not downward from eval failures. Any behavior that only exists because a higher-level eval failed is suspect until it is re-expressed as a primitive, composite, contract, policy, profile/config surface, or eval-only fixture.

## First guard run

Command:

```bash
npm run -s ops:primitive-first:guard
```

Result on 2026-05-20:

- `ok`: false
- `files_scanned`: 5069
- `violations`: 29
- `primitive_registry_entries`: 7

Interpretation:

- The guard is doing useful work, but v1 was noisy.
- Several reported issues were false positives from generic error normalization or request-path parsing.
- The real coding concern is not the false positives. The real coding concern is level/eval-specific behavior embedded in the coding workflow and native coding evidence gates.

## Confirmed coding foundation contamination

| Finding | Location | Classification | Repair direction | Status |
|---|---|---|---|---|
| Level-specific evidence contract was embedded in the high-level coding program builder. | `orchestration/src/control_plane/workflows/lab/frameworks/coding/local_coding_program_builder.workflow.json` | `hardcoding_contamination` | Replaced with general `existing_project_evidence_contract` and moved the primitive foundation toward reusable runtime, context, mode, patch, evidence, and final receipt contracts. | patched-primitive-reset |
| Level-specific stage name exists in normal coding workflow stages. | `orchestration/src/control_plane/workflows/lab/frameworks/coding/local_coding_program_builder.workflow.json` | `hardcoding_contamination` | Rename/replace `existing_project_evidence_contract_self_check` with a general existing-project evidence self-check. | open |
| Native product-slice evidence scanned changed source for benchmark-shaped terms such as retryable, schema version, idempotence, malformed record handling, mixed v1/v2, sequence, and import/export. | `core/layer2/agent_surface/src/native_evidence.rs` | `hardcoding_contamination` | Remove domain-term evidence checks from the primitive gate. Use generic mutation/category evidence in production; leave domain specifics to eval judges or project contracts. | patched-first-wave |
| Official coding operator repair prompt included benchmark-shaped repair examples around persistence/model/report/import/export/retryable behavior. | `orchestration/src/control_plane/workflows/official/coding_project_operator.workflow.json` | `hardcoding_contamination` | Replace domain-shaped instructions with generic evidence categories: source, tests, public interface, config, docs, checkpoint artifacts. | patched-first-wave |
| Python-only preserved API behavior signature exists in native agent runtime. | `core/layer2/agent_surface/src/agent.rs` | `primitive_candidate` | Keep temporarily as a preservation primitive candidate, but move toward a language-neutral public-surface preservation contract. | open |
| New-file fast path exists in native agent runtime. | `core/layer2/agent_surface/src/agent.rs` | `valid_primitive_candidate` | Keep only if it remains context-free, new-file only, and cannot slow or narrow existing-project paths. Register as a primitive/lane if it remains production behavior. | open |

## First cleanup patch

Completed in this reset wave:

- Tuned the primitive-first guard so it no longer treats generic `message`, `request`, `input`, or `query` variable parsing as prompt-phrase hardcoding.
- Added workflow-CD detection for `levelN_` literals so level-shaped workflow contamination becomes visible.
- Removed domain-shaped changed-source evidence checks from the native coding evidence gate.
- Generalized official coding repair prompt text away from persistence/report/import-export/retryable fixture shapes.

## Reset sequence from here

0. Use the cross-system primitive architecture ledger before inventing new coding primitives:
   `docs/workspace/primitive_coding_architecture_assimilation_ledger.md`
1. Implement the artifact-backed missing primitives before adding more task-specific eval repair:
   `step_budgeted_trajectory_runtime`, `incremental_receipt_journal`, and `public_interface_verification`.
2. Run the primitive-first guard against the new `existing_project_evidence_contract` and primitive spine once validation is authorized.
3. Move Existing-project artifact names and fixture paths into eval fixtures only.
4. Register the general existing-project evidence contract in the primitive capability registry.
5. Re-run `ops:primitive-first:guard` and fix true positives before allowlisting anything.
6. Audit the native agent runtime for remaining level/eval-specific branches.
7. Move Python-only/public API preservation behavior into the language/profile surface of `public_interface_verification`.
8. Keep the new-file fast path only if it remains context-free, receipt-bearing, and does not slow existing-project paths.
9. Restart coding evals from Level 1 upward only after the primitive foundation is clean enough that higher-level work cannot poison lower levels.

## Artifact-backed primitive fix for current regression

The current Level 1/2 regression should not be repaired with another benchmark-shaped prompt or runtime branch.

The pulled coding-system artifacts point to a primitive fix:

- mini-SWE-agent persists trajectory after each step and enforces step/cost limits.
- SWE-ReX separates runtime liveness, command timeout, exit code, output, and failure reason.
- Cline and Continue stream tool-call lifecycle events and keep pending/failed tool calls explicit.
- Aider treats edit/validation failure as structured repair input rather than final-answer optimism.

Infring should therefore solve the issue at these reusable seams:

- `step_budgeted_trajectory_runtime`: every coding run has step/tool/time budgets, terminal status, and partial result refs.
- `incremental_receipt_journal`: model turns, tool starts, tool completions, mutations, validations, blockers, and terminal status are appended before risky boundaries.
- `public_interface_verification`: user-requested public surfaces are extracted into an artifact and verified through language/profile adapters before final success.

These primitives are intentionally not Python-specific, level-specific, CSV-specific, or benchmark-specific.

## Non-goals

- Do not delete useful native file tools.
- Do not throw away Workflow CD structure.
- Do not abandon ForgeCode-derived primitives.
- Do not chase Level 8+ reliability until lower-level primitives are clean and monotonic.
