# Temp Research Workflow Phase Plan

Purpose: move the research workflow toward the desired end state without drifting into case-specific patching or metric theater.

Desired end state:

- A user can ask a broad research question.
- The system retrieves external evidence.
- The agent gives a directly useful answer that responds to the original query.
- If the evidence is weak, the agent says so honestly while still giving the best bounded answer the evidence supports.

Working rule:

- Human usefulness is the north star.
- If metrics say "good" and a real human says "bad," the metrics are wrong.
- If metrics say "bad" and a real human says "good," the metrics are wrong.
- Patches must target reusable failure classes, not individual prompts.

## Evaluation Lanes

We will use three different eval lanes on purpose:

1. Repro lane
   - Fixed known cases only.
   - Purpose: verify that a specific bug or failure class is actually gone.
   - Current repro fixture:
     - `/Users/jay/.openclaw/workspace/local/state/ops/research_human_calibration/phase1_cases_fixture.json`
   - Rule: this lane is for regression proof, not for the main progress score.

2. Random broad lane
   - Random sample from the full 100-case prompt pool.
   - Purpose: measure whether improvements generalize instead of only helping familiar prompts.
   - Source pool:
     - `/Users/jay/.openclaw/workspace/validation/evals/fixtures/research_user_prompt_pool_v1.json`
   - Rule: this is the main progress metric.
   - Operational rule:
     - use `--sample-size=<n>`
     - do not pin `--sample-seed`
     - record the emitted runtime random seed from the report

3. Holdout lane
   - A stable subset of cases that we do not use for day-to-day debugging.
   - Purpose: catch indirect overfitting to the queries we happen to inspect most often.
   - Rule: only use this lane for occasional calibration checks, not for every patch.

Evaluation lane policy:

- A patch is allowed to use the repro lane to prove a bug was fixed.
- A patch is not considered broadly successful until the random broad lane also looks healthy.
- If repro improves but random broad regresses, treat that as local overfitting until proven otherwise.
- If random broad improves but the answer text still looks bad, the grader or metrics are still wrong.

## Phase 1: Calibrate the grader

Goal:

- Make the grader line up with human judgment closely enough that batch movement means something.

What belongs in scope:

- Disagreement cases where automated grading and human judgment are likely to differ.
- A small number of positive controls so the grader is not only trained on failures.
- Class-level grading fixes only.

What does not belong in scope:

- New retrieval providers.
- Workflow rewrites.
- Domain-specific answer templates.

Success condition for Phase 1:

- We have 10-20 manually labeled cases.
- We can point to specific recurring disagreement classes.
- We patch those disagreement classes and reduce future disagreement on reruns.

Phase 1 checklist:

- [x] Inspect the current grading path and identify what the grader actually uses.
- [x] Find the highest-value disagreement cases from saved research eval artifacts.
- [x] Seed a reusable human calibration queue with positive, negative, and borderline examples.
- [ ] Collect 10-20 human verdicts with short notes.
- [ ] Re-evaluate those same saved outputs against the current grader state.
- [ ] Run judge-human agreement and calibration stats on the reviewed set.
- [ ] Patch the single highest-ROI disagreement class.
- [ ] Re-run a small research batch and check whether disagreement drops.

Phase 1 artifacts:

- Human-readable queue:
  - `/Users/jay/.openclaw/workspace/local/workspace/reports/RESEARCH_PHASE1_HUMAN_CALIBRATION_QUEUE.md`
- Machine-readable queue:
  - `/Users/jay/.openclaw/workspace/core/local/artifacts/research_phase1_human_calibration_queue_current.json`
- Human label template:
  - `/Users/jay/.openclaw/workspace/local/state/ops/research_human_calibration/phase1_review_template.jsonl`
- Provisional codex review labels:
  - `/Users/jay/.openclaw/workspace/local/state/ops/research_human_calibration/phase1_review_codex_provisional.jsonl`
- Provisional judge-human agreement report:
  - `/Users/jay/.openclaw/workspace/core/local/artifacts/research_phase1_judge_human_agreement_provisional.json`
  - `/Users/jay/.openclaw/workspace/local/workspace/reports/RESEARCH_PHASE1_JUDGE_HUMAN_AGREEMENT_PROVISIONAL.md`

Phase 1 infra note:

- Current saved research batch reports are useful for review, but not fully replay-gradeable with the current scorer because they do not preserve the raw response payload in a clean `response_payload` form.
- Future calibration-friendly batches should retain the raw scored payload alongside the summarized case row so the current grader can be rerun offline without distortion.

How to grade each queued answer:

- `correct`: directly answered the question in a useful way.
- `partial`: somewhat useful, but missing important synthesis, specificity, grounding, or closure.
- `incorrect`: did not really answer the question, was misleading, was mostly fragments/process talk, or was too off-target to be useful.

Human note guidance:

- Keep notes short.
- Prefer "what was missing?" over general commentary.
- Good examples:
  - `useful direct recommendation; evidence limits are clear without taking over`
  - `mostly source recap; never really answers the user question`
  - `good update, but too list-like and missing why the changes matter`

## Phase 2: Fix downstream answer quality assuming good data

Goal:

- Make the final answer directly useful when retrieval is already decent.

Primary targets:

- Answer-unit usefulness for the original prompt.
- Final answer shape: answer first, limits second.
- Honest bounded insufficiency behavior instead of source dumps or process summaries.

Rule:

- Do not widen scope into retrieval changes until the downstream answer path is smoother.

## Phase 3: Improve upstream retrieval quality

Goal:

- Increase how often the workflow receives answer-ready evidence.

Primary targets:

- Candidate quality.
- Materialization quality.
- Claim extraction quality.
- Answerability-ready evidence rate.

## Iteration method

Hard rule:

- Make exactly one measurable change per iteration.
- Before patching, write down the expected metric or visible-output improvement.
- Test against the same lane shape used for the baseline.
- Keep the patch only if it makes an obvious positive difference.
- If the result is neutral, noisy, or negative, revert the patch and record it in the iteration log.
- Never stack a second speculative patch on top of an unproven first patch.
- Do not count "the tests still pass" as evidence of product improvement; tests are safety checks, not the success metric.
- A patch can be a verifier/grader change, runtime handoff change, retrieval/tooling change, or prompt/CD contract change, but it must name one failure class and one expected movement.

Always use this loop:

1. Establish a baseline report and save its seed, command, artifact path, pass rate, excellent rate, weakest gates, and 2-3 representative visible outputs.
2. Pick exactly one failure class from the baseline.
3. State the hypothesis in one sentence: "If we change X, metric/output Y should improve because Z."
4. Patch only that failure class.
5. Run the narrowest relevant proof test for breakage.
6. Re-run the same eval lane shape against the baseline query set or the same random seed when doing an A/B check.
7. Compare the result against the baseline.
8. If the improvement is obvious, keep the patch and record why.
9. If the improvement is not obvious, revert the patch and record why it was rejected.
10. Only after keep/revert is decided may another patch begin.
11. Use the holdout lane periodically to check for hidden overfitting.

Minimum iteration log entry:

- `timestamp`
- `baseline_artifact`
- `patch_name`
- `failure_class_targeted`
- `hypothesis`
- `files_changed`
- `proof_tests`
- `eval_command`
- `before_metrics`
- `after_metrics`
- `visible_output_delta`
- `decision`: `kept` or `reverted`
- `reason`
- `follow_up`

Patch decision standard:

- Keep: a targeted failure disappears or clearly improves, the visible answer is more useful to a human, and no upstream gate or adjacent query class obviously regresses.
- Revert: the patch only moves the scorer, makes output more awkward, causes unrelated gates to fall, depends on a specific prompt/domain, or has noisy/ambiguous results.
- Log but defer: the idea seems plausible but needs a better eval lane or more data before implementation.

Current focus:

- We are in Phase 1.
- We have a first provisional five-case review set recorded so we can bootstrap calibration without waiting on more live runs.
- The provisional judge-human run shows a strong disagreement signal: exact agreement is 0.20, and the dominant mismatch is that bounded/thin answers are being overcalled as `correct` instead of `partial` or `incorrect`.
- The next action is to make the random broad lane the main scoreboard again, keep the fixed five-case set strictly as a repro lane, and continue collecting human labels where the random broad lane disagrees with human usefulness.

## Iteration Log

### 2026-05-29: generic content-claim fallback experiment

- `baseline_artifact`: `artifacts/research_golden_after_depth_guard_live2.json`
- `patch_name`: generic content-rich claim fallback for usable evidence rows
- `failure_class_targeted`: `web_5d_source_quality_ready` / `web_5e_claim_quality_ready`
- `hypothesis`: If usable, content-rich evidence rows can supply claim hints from cleaned extracts when the strict claim extractor misses them, then source/claim quality gates should improve because fewer real evidence rows will be demoted to candidate-only.
- `files_changed`: `core/layer2/ops/src/retrieval_policy_parts/010-freshness-and-relevance.rs`, temporary compiled regression in `core/layer2/ops/src/retrieval_policy.rs`
- `proof_tests`: `cargo test -p infring-ops-core-v1 content_rich_usable_rows_can_supply_generic_claim_hints --lib -- --nocapture` passed while experiment was applied.
- `eval_command`: `cargo run --manifest-path orchestration/Cargo.toml --bin eval_runtime -- research-golden --live=1 --base-url=http://127.0.0.1:5173 --limit=2 --sample-seed=random:6d09e741b184df5d5052ac05fd27b875bc9829f3c62ca6da69e37eb955207a47 --timeout-seconds=90 --out=core/local/artifacts/research_golden_after_generic_claim_patch_live2.json --out-latest=artifacts/research_golden_after_generic_claim_patch_live2.json --out-markdown=artifacts/research_golden_after_generic_claim_patch_live2.md --failures-out=local/state/ops/research_golden/generic_claim_patch_live2_failures.jsonl`
- `before_metrics`: comparable fixed-seed two-case lane had `web_5d_source_quality_ready=0/2`, `web_5e_claim_quality_ready=0/2`, `web_5h_evidence_packet_contract_ready=0/2`, `web_7_usable_evidence_available=0/2`; case 2 score previously around 82.
- `after_metrics`: `average_score=91.5`, `passed_cases=1/2`, `excellent_cases=0/2`, `web_5d_source_quality_ready=0/2`, `web_5e_claim_quality_ready=0/2`, `web_5h_evidence_packet_contract_ready=0/2`, `web_7_usable_evidence_available=0/2`; case 2 score moved to 87 but still failed.
- `visible_output_delta`: nuclear-permitting response became more coherent and cited ADVANCE Act/NRC/NEIMA material, but the answer still failed and the web evidence gates did not move.
- `decision`: `reverted`
- `reason`: The targeted source/claim/evidence-packet gates did not improve. Diagnostics showed the live synthesis/eval path was scanning `response_workflow.synthesis_input.evidence_pack` rows missing `claim_hints`, `relevant_extract`, `source_type`, and `why_relevant_to_query`, so the real failure is evidence-packet contract carry-through/handoff, not only layer2 generic claim extraction.
- `follow_up`: Patch the runtime/tool-result handoff so synthesis input evidence rows preserve the existing evidence-packet contract fields, then rerun the same seed before touching claim extraction again.

### 2026-05-29: evidence-ref packet title promotion experiment

- `baseline_artifact`: `artifacts/research_golden_after_generic_claim_patch_live2.json`
- `patch_name`: promote claim-like citation/evidence-ref titles into synthesis evidence-packet fields
- `failure_class_targeted`: `web_5h_evidence_packet_contract_ready` / `packaged evidence refs missing packet fields`
- `hypothesis`: If citation-like evidence refs carry source type, a relevant extract, a query-relevance rationale, and claim hints when the title is claim-like, then the evidence-packet contract gate should improve and synthesis should have more usable evidence context.
- `files_changed`: `core/layer0/ops/src/dashboard_compat_api_parts/set_config_payload_parts/190_route_blocks/agent_scope_full_parts/045-tool-recovery-and-turn-persistence.rs`, `core/layer0/ops/src/dashboard_compat_api_parts/set_config_payload_parts/190_route_blocks/agent_scope_full_parts/047-turn-workflow-synthesis_parts/902-workflow-fallback-tests.rs`
- `proof_tests`: `cargo test -p infring-ops-core --lib synthesis_input_preserves_claim_like_evidence_refs_as_packets -- --nocapture` passed while experiment was applied; `cargo test -p infring-ops-core --lib workflow_fallback_tests -- --nocapture` passed with 148/148.
- `eval_command`: `cargo run --manifest-path orchestration/Cargo.toml --bin eval_runtime -- research-golden --live=1 --base-url=http://127.0.0.1:5173 --limit=2 --sample-seed=random:6d09e741b184df5d5052ac05fd27b875bc9829f3c62ca6da69e37eb955207a47 --timeout-seconds=90 --out=core/local/artifacts/research_golden_after_evidence_ref_packet_handoff_live2.json --out-latest=artifacts/research_golden_after_evidence_ref_packet_handoff_live2.json --out-markdown=artifacts/research_golden_after_evidence_ref_packet_handoff_live2.md --failures-out=local/state/ops/research_golden/evidence_ref_packet_handoff_live2_failures.jsonl`
- `before_metrics`: `average_score=91.5`, `passed_cases=1/2`, `excellent_cases=0/2`, `web_5d_source_quality_ready=0/2`, `web_5e_claim_quality_ready=0/2`, `web_5h_evidence_packet_contract_ready=0/2`, `web_7_usable_evidence_available=0/2`.
- `after_metrics`: `passed_cases=0/2`, `excellent_cases=0/2`; drug-discovery case regressed from `score=96/pass` to `score=82/fail`; nuclear-permitting stayed `score=87/fail`; `web_5h_evidence_packet_contract_ready` improved only to `1/2`, while `web_5d_source_quality_ready=0/2`, `web_5e_claim_quality_ready=0/2`, `web_5g_answerability_ready=0/2`, and `web_7_usable_evidence_available=0/2` remained failed.
- `visible_output_delta`: the drug-discovery answer collapsed into a single truncated fragment: `173 AI-discovered drugs are in clinical trials...`; this is worse for a user even though one packet-contract metric moved.
- `decision`: `reverted`
- `reason`: The patch looked like metric movement rather than product improvement. It promoted citation-title text into packet fields, but the live answer became less useful and the upstream source/claim/answerability gates stayed bad.
- `follow_up`: Do not infer evidence packets from titles. Target the earlier source-quality/claim-quality boundary instead: preserve or extract real materialized snippets/extracts into evidence packets before synthesis, and treat title-only rows as citations/source refs unless backed by substantive content.

### 2026-05-29: traceability reject repair guard experiment

- `baseline_artifact`: `artifacts/research_golden_after_evidence_ref_packet_handoff_live2.json`
- `patch_name`: prevent traceability-rejected excerpt fragments from becoming visible fallback answers
- `failure_class_targeted`: `post_tool_synthesis_not_useful` / `rejected excerpt surfaced as final answer`
- `hypothesis`: If a verifier rejected an answer because answer units were not traceable to evidence, the runtime repair path should rebuild from traceable evidence or emit clean insufficiency rather than surfacing the rejected excerpt, improving visible answer quality and the 6a synthesis checkpoint.
- `files_changed`: `core/layer0/ops/src/dashboard_compat_api_parts/set_config_payload_parts/190_route_blocks/agent_scope_full_parts/047-turn-workflow-synthesis_parts/007-part.rs`, `core/layer0/ops/src/dashboard_compat_api_parts/set_config_payload_parts/190_route_blocks/agent_scope_full_parts/047-turn-workflow-synthesis_parts/907-workflow-fallback-tests.rs`
- `proof_tests`: `cargo test -p infring-ops-core --lib traceability_reject_repair_does_not_surface_untraceable_excerpt_fragment -- --nocapture` passed while experiment was applied; `cargo test -p infring-ops-core --lib reject_excerpt -- --nocapture` passed 2/2 while experiment was applied.
- `eval_command`: `cargo run --manifest-path orchestration/Cargo.toml --bin eval_runtime -- research-golden --live=1 --base-url=http://127.0.0.1:5173 --limit=2 --sample-seed=random:6d09e741b184df5d5052ac05fd27b875bc9829f3c62ca6da69e37eb955207a47 --timeout-seconds=90 --out=core/local/artifacts/research_golden_after_traceability_repair_guard_live2.json --out-latest=artifacts/research_golden_after_traceability_repair_guard_live2.json --out-markdown=artifacts/research_golden_after_traceability_repair_guard_live2.md --failures-out=local/state/ops/research_golden/traceability_repair_guard_live2_failures.jsonl`
- `before_metrics`: previous failed experiment had `passed_cases=0/2`, `excellent_cases=0/2`, scores `82` and `87`, and weakest gates `web_5d_source_quality_ready=0/2`, `web_5e_claim_quality_ready=0/2`, `web_5g_answerability_ready=0/2`, `web_5h_evidence_packet_contract_ready=1/2`, `web_7_usable_evidence_available=0/2`.
- `after_metrics`: `passed_cases=0/2`, `excellent_cases=0/2`, scores remained `82` and `87`; weakest gates remained `web_5d_source_quality_ready=0/2`, `web_5e_claim_quality_ready=0/2`, `web_5g_answerability_ready=0/2`, `web_5h_evidence_packet_contract_ready=1/2`, `web_7_usable_evidence_available=0/2`.
- `visible_output_delta`: the drug-discovery output changed from a truncated clinical-trials fragment to a complete one-sentence Isomorphic Labs fragment, but it still did not answer the original prompt.
- `decision`: `reverted`
- `reason`: The patch slightly changed the symptom shape but did not improve score, pass rate, or the core user-facing usefulness problem. It still left the system able to surface a single narrow answer unit where the user asked for a broader research synthesis.
- `follow_up`: Move upstream of fallback repair. The next target should be answer construction from multiple evidence units and/or the source-quality boundary, not another visible fallback guard.

### 2026-05-29: domain-diverse facet evidence selection experiment

- `baseline_artifact`: `artifacts/research_golden_after_traceability_repair_guard_live2.json`
- `patch_name`: prefer unseen source domains during facet-covered evidence selection
- `failure_class_targeted`: `web_5d_source_quality_ready` / selected evidence pack collapses to too few usable source domains
- `hypothesis`: If multiple candidates satisfy a required facet, choosing an unseen source domain should improve pack source-domain diversity and move `web_5d_source_quality_ready`.
- `files_changed`: `core/layer2/ops/src/retrieval_policy_parts/010-freshness-and-relevance.rs`, temporary non-wired regression in `core/layer2/ops/src/retrieval_policy_parts/911-relevance-tests.rs`
- `proof_tests`: `cargo test -p infring-ops-core-v1 facet_evidence_selection_prefers_new_source_domains --lib -- --nocapture` compiled but ran 0 tests because this regression file is not wired into the current Cargo test target; `cargo test -p infring-ops-core --lib facet_evidence_selection_prefers_new_source_domains -- --nocapture` also ran 0 tests.
- `eval_command`: `cargo run --manifest-path orchestration/Cargo.toml --bin eval_runtime -- research-golden --live=1 --base-url=http://127.0.0.1:5173 --limit=2 --sample-seed=random:6d09e741b184df5d5052ac05fd27b875bc9829f3c62ca6da69e37eb955207a47 --timeout-seconds=90 --out=core/local/artifacts/research_golden_after_domain_diverse_facet_selection_live2.json --out-latest=artifacts/research_golden_after_domain_diverse_facet_selection_live2.json --out-markdown=artifacts/research_golden_after_domain_diverse_facet_selection_live2.md --failures-out=local/state/ops/research_golden/domain_diverse_facet_selection_live2_failures.jsonl`
- `before_metrics`: `passed_cases=0/2`, `excellent_cases=0/2`, scores `82` and `87`; weakest gates included `web_5d_source_quality_ready=0/2`, `web_5e_claim_quality_ready=0/2`, `web_5g_answerability_ready=0/2`, `web_5h_evidence_packet_contract_ready=1/2`, `web_7_usable_evidence_available=0/2`.
- `after_metrics`: `passed_cases=2/2`, `excellent_cases=0/2`, scores `96` and `96`; however the targeted web gates did not improve: `web_5d_source_quality_ready=0/2`, `web_5e_claim_quality_ready=0/2`, `web_5g_answerability_ready=0/2`, `web_7_usable_evidence_available=0/2`; adjacent gates worsened with `web_5f_citation_renderability_ready=1/2` and `web_5h_evidence_packet_contract_ready=0/2`.
- `visible_output_delta`: both user-facing answers became passable bounded answers, but the source/claim-quality bottleneck remained and the evidence-packet/citation readiness metrics worsened.
- `decision`: `reverted`
- `reason`: The pass-rate movement was not aligned with the targeted bottleneck and may be live-run/query-generation variance. Keeping it would violate the one-change rule because the source-quality gate did not move and adjacent web evidence gates regressed.
- `follow_up`: Next patch should target the actual first-failure evidence quality report directly: inspect why `response_workflow.synthesis_input.evidence_pack` rows are not citation/claim/evidence-packet ready, and add instrumentation that preserves selected pack row diagnostics in the eval artifact before changing selection again.

### 2026-05-29: evidence-quality row sampling instrumentation

- `baseline_artifact`: `artifacts/research_golden_after_domain_diverse_facet_selection_live2.json`
- `patch_name`: preserve evidence-quality sample rows in web gate diagnostics
- `failure_class_targeted`: unclear `web_5d_source_quality_ready` / `web_5e_claim_quality_ready` boundary where aggregate metrics existed but failing row shape was not visible.
- `hypothesis`: If the eval artifact includes compact failing evidence rows, then the next behavioral patch can target the real row shape instead of guessing from aggregate counts.
- `files_changed`: `orchestration/src/eval_web_retrieval_gate_diagnostics_parts/050_evidence_quality.rs`
- `proof_tests`: `cargo check --manifest-path orchestration/Cargo.toml` passed; `cargo test --manifest-path orchestration/Cargo.toml --lib eval_web_retrieval_gate_diagnostics -- --nocapture` compiled but ran 0 matching tests.
- `eval_command`: `cargo run --manifest-path orchestration/Cargo.toml --bin eval_runtime -- research-golden --live=1 --base-url=http://127.0.0.1:5173 --limit=2 --sample-seed=random:6d09e741b184df5d5052ac05fd27b875bc9829f3c62ca6da69e37eb955207a47 --timeout-seconds=90 --out=core/local/artifacts/research_golden_after_evidence_quality_samples_live2.json --out-latest=artifacts/research_golden_after_evidence_quality_samples_live2.json --out-markdown=artifacts/research_golden_after_evidence_quality_samples_live2.md --failures-out=local/state/ops/research_golden/evidence_quality_samples_live2_failures.jsonl`
- `before_metrics`: `web_5d_source_quality_ready=0/2`, `web_5e_claim_quality_ready=0/2`, `web_5g_answerability_ready=0/2`, `web_7_usable_evidence_available=0/2`; row-level failure shape was not visible in the saved report.
- `after_metrics`: drug-discovery `score=100/pass`, nuclear-permitting `score=87/fail`, `excellent=0/2`; source/claim/answerability gates still failed. Nuclear now exposes three sample rows showing a single `www.energypolicy.columbia.edu` evidence packet carried through tool attempts, synthesis input, and tools. Drug-discovery reports 20 aggregate evidence items but zero sample rows, proving the saved artifact lacks row-level evidence despite aggregate counts.
- `visible_output_delta`: no intended product-output change; this was diagnostic-only.
- `decision`: `kept`
- `reason`: The change did not pretend to improve product quality, but it did separate two different failure classes: thin carried-through evidence rows versus aggregate evidence that is not available as inspectable packet rows. The diagnostic now also reports `row_sample_status` so future runs make that distinction explicit.
- `follow_up`: Next behavioral patch should target evidence row preservation/extraction before synthesis: when retrieval reports usable/materialized evidence, the saved workflow/eval payload must expose the actual evidence rows or classify the boundary as `aggregate_evidence_not_packaged_for_synthesis`.

### 2026-05-29: observed-row source-quality diagnostic correction

- `baseline_artifact`: `artifacts/research_golden_after_evidence_quality_row_status_live2.json`
- `patch_name`: allow observed clean evidence rows to satisfy source-quality thresholds
- `failure_class_targeted`: `web_5d_source_quality_ready` false negative caused by aggregate `evidence_pack_quality_status` blocking source quality even when row scanning observed clean evidence rows and source domains.
- `hypothesis`: If source-quality diagnostics consider observed row evidence separately from packet-contract quality, then `web_5d_source_quality_ready` should stop being the first blocker while claim quality, packet contract, and answerability remain separate downstream gates.
- `files_changed`: `orchestration/src/eval_web_retrieval_gate_diagnostics_parts/050_evidence_quality.rs`
- `proof_tests`: `cargo check --manifest-path orchestration/Cargo.toml` passed with existing warnings only.
- `eval_command`: `cargo run --manifest-path orchestration/Cargo.toml --bin eval_runtime -- research-golden --live=1 --base-url=http://127.0.0.1:5173 --limit=2 --sample-seed=random:6d09e741b184df5d5052ac05fd27b875bc9829f3c62ca6da69e37eb955207a47 --timeout-seconds=90 --out=core/local/artifacts/research_golden_after_row_observed_source_quality_live2.json --out-latest=artifacts/research_golden_after_row_observed_source_quality_live2.json --out-markdown=artifacts/research_golden_after_row_observed_source_quality_live2.md --failures-out=local/state/ops/research_golden/row_observed_source_quality_live2_failures.jsonl`
- `before_metrics`: same two-case lane had `web_5d_source_quality_ready=0/2`, `web_5e_claim_quality_ready=0/2`, `web_5g_answerability_ready=0/2`, `web_5h_evidence_packet_contract_ready=1/2`, and `web_7_usable_evidence_available=0/2`.
- `after_metrics`: `web_5d_source_quality_ready=2/2`, `web_5e_claim_quality_ready=1/2`, `web_5g_answerability_ready=0/2`, `web_5h_evidence_packet_contract_ready=1/2`, and `web_7_usable_evidence_available=0/2`. Overall research pass remained weak at `1/2`, excellent stayed `0/2`.
- `visible_output_delta`: no real user-facing quality improvement; the drug-discovery answer was still a single narrow answer unit instead of a useful synthesis.
- `decision`: `kept`
- `reason`: This is a measurement correction, not a product-quality fix. It successfully moved the falsely upstream source-quality blocker out of the way while leaving answerability and claim/packet issues visible.
- `follow_up`: Next iteration should target `web_5g_answerability_ready` by inspecting why citable/clean evidence rows are not becoming enough concrete, query-useful answer units. Do not patch visible answer style until answerability evidence is actually present or explicitly unavailable.

### 2026-05-29: evidence-packet claim widening experiment

- `baseline_artifact`: `artifacts/research_golden_after_row_observed_source_quality_live2.json`
- `patch_name`: infer missing evidence-packet `source_type`, `claim_hints`, and `why_relevant_to_query` from substantive extracts.
- `failure_class_targeted`: `web_5e_claim_quality_ready` / `web_5h_evidence_packet_contract_ready` where rows carried extracts but missed packet fields.
- `hypothesis`: If packet normalization extracts generic claim-like sentences from substantive snippets/extracts, then claim quality and packet-contract readiness should improve without changing visible answer format.
- `files_changed`: `core/layer0/ops/src/dashboard_compat_api_parts/set_config_payload_parts/190_route_blocks/agent_scope_full_parts/045-tool-recovery-and-turn-persistence.rs`, `core/layer0/ops/src/dashboard_compat_api_parts/set_config_payload_parts/190_route_blocks/agent_scope_full_parts/047-turn-workflow-synthesis_parts/003-part.rs`
- `proof_tests`: `cargo test -p infring-ops-core --lib workflow_synthesis_input_upgrades_substantive -- --nocapture` passed while experiment was applied; `cargo check --manifest-path orchestration/Cargo.toml` passed.
- `eval_command`: `cargo run --manifest-path orchestration/Cargo.toml --bin eval_runtime -- research-golden --live=1 --base-url=http://127.0.0.1:5173 --limit=2 --sample-seed=random:6d09e741b184df5d5052ac05fd27b875bc9829f3c62ca6da69e37eb955207a47 --timeout-seconds=90 --out=core/local/artifacts/research_golden_after_packet_claim_widening_live2.json --out-latest=artifacts/research_golden_after_packet_claim_widening_live2.json --out-markdown=artifacts/research_golden_after_packet_claim_widening_live2.md --failures-out=local/state/ops/research_golden/packet_claim_widening_live2_failures.jsonl`
- `before_metrics`: `average_score=89.0`, `passed_cases=1/2`, `excellent_cases=0/2`, `web_5d_source_quality_ready=2/2`, `web_5e_claim_quality_ready=1/2`, `web_5g_answerability_ready=0/2`, `web_5h_evidence_packet_contract_ready=1/2`, `web_7_usable_evidence_available=0/2`.
- `after_metrics`: `average_score=86.0`, `passed_cases=1/2`, `excellent_cases=0/2`, `web_5d_source_quality_ready=1/2`, `web_5e_claim_quality_ready=0/2`, `web_5g_answerability_ready=0/2`, `web_5h_evidence_packet_contract_ready=1/2`, `web_7_usable_evidence_available=0/2`.
- `visible_output_delta`: the drug-discovery answer remained a narrow one-sentence fragment and scored `82/fail`; the final LLM path rejected an attempted answer as `answer_units_not_traceable_to_evidence`, then the workflow surfaced `tool_evidence_fallback_used`.
- `decision`: `reverted`
- `reason`: The patch passed the local structural test but did not improve the measured live gates and regressed source/claim-quality readiness. It was another packet-field repair, while the actual visible failure is now the post-rejection synthesis/fallback path.
- `follow_up`: Target the next concrete weak point: when final LLM synthesis is rejected for evidence traceability, the workflow should not fall through to a single clipped evidence fragment. Either the retry path must receive the exact traceable evidence needed to answer the original prompt, or the system should emit a complete bounded answer that explicitly explains what can and cannot be supported.

### 2026-05-29: traceability reject no-excerpt-salvage experiment

- `baseline_artifact`: `artifacts/research_golden_after_packet_claim_widening_live2.json`
- `patch_name`: remove rejected-excerpt salvage from evidence-depth/traceability fallback ladder.
- `failure_class_targeted`: `post_tool_synthesis_not_useful` / traceability-rejected LLM answer becoming visible as a thin fragment.
- `hypothesis`: If traceability rejects cannot fall through to `excerpt_sentence_salvage`, thin rejected snippets should become complete bounded insufficiency answers instead of bad pseudo-answers.
- `files_changed`: `core/layer0/ops/src/dashboard_compat_api_parts/set_config_payload_parts/190_route_blocks/agent_scope_full_parts/047-turn-workflow-synthesis_parts/007-part.rs`, `core/layer0/ops/src/dashboard_compat_api_parts/set_config_payload_parts/190_route_blocks/agent_scope_full_parts/047-turn-workflow-synthesis_parts/907-workflow-fallback-tests.rs`
- `proof_tests`: `cargo test -p infring-ops-core --lib traceability_reject -- --nocapture` passed while experiment was applied; `cargo check --manifest-path orchestration/Cargo.toml` passed.
- `eval_command`: `cargo run --manifest-path orchestration/Cargo.toml --bin eval_runtime -- research-golden --live=1 --base-url=http://127.0.0.1:5173 --limit=2 --sample-seed=random:6d09e741b184df5d5052ac05fd27b875bc9829f3c62ca6da69e37eb955207a47 --timeout-seconds=90 --out=core/local/artifacts/research_golden_after_traceability_no_excerpt_salvage_live2.json --out-latest=artifacts/research_golden_after_traceability_no_excerpt_salvage_live2.json --out-markdown=artifacts/research_golden_after_traceability_no_excerpt_salvage_live2.md --failures-out=local/state/ops/research_golden/traceability_no_excerpt_salvage_live2_failures.jsonl`
- `before_metrics`: previous live run had `average_score=86.0`, `passed_cases=1/2`, `excellent_cases=0/2`, and the drug-discovery response was a one-sentence Isomorphic Labs fragment after `answer_units_not_traceable_to_evidence`.
- `after_metrics`: `average_score=86.0`, `passed_cases=0/2`, `excellent_cases=0/2`; `web_2_query_metadata_present=0/2`, `web_5d_source_quality_ready=0/2`, `web_5e_claim_quality_ready=0/2`, `web_5g_answerability_ready=0/2`, `web_7_usable_evidence_available=0/2`.
- `visible_output_delta`: the drug-discovery answer changed from the Isomorphic fragment to a worse title-shell answer: `What’s really happening inside AI’s black box? Berkeley researchers have answers University of California...`
- `decision`: `reverted`
- `reason`: The patch removed one bad salvage route but the fallback system selected another source-title shell. This proves the primitive issue is not one salvage branch; it is that source-title/headline rows can still be classified as acceptable answer material after synthesis rejection.
- `follow_up`: Next patch should target the generic answer-unit/source-shell detector and fallback acceptance criteria: source titles, article headlines, and question-style titles must never satisfy `fallback_response_has_substantive_depth` or the 6a synthesis checkpoint unless backed by source extract text that directly answers the original prompt.

### 2026-05-29: recovered batch-query metadata hydration

- `baseline_artifact`: `artifacts/research_golden_baseline_after_reverts_live2.json`
- `patch_name`: hydrate declared batch-query metadata for recovered raw-message requests
- `failure_class_targeted`: `web_2_query_metadata_present` / recovered `batch_query` requests carried only `source`, `query`, and `aperture`.
- `hypothesis`: If recovery uses the workflow-declared request repair contract to carry visible query lanes, keywords, required coverage, and metadata policy, the web tooling lane will stop starting from a thin request without adding domain-specific query assumptions.
- `files_changed`: `core/layer0/ops/src/dashboard_compat_api_parts/set_config_payload_parts/190_route_blocks/agent_scope_full_parts/046b-manual-toolbox-pending-request.rs`
- `proof_tests`: `cargo test --manifest-path core/layer0/ops/Cargo.toml --lib manual_toolbox_pending_request_tests -- --nocapture` passed.
- `eval_command`: `cargo run --manifest-path orchestration/Cargo.toml --bin eval_runtime -- research-golden --live=1 --base-url=http://127.0.0.1:5173 --limit=2 --sample-seed=random:6d09e741b184df5d5052ac05fd27b875bc9829f3c62ca6da69e37eb955207a47 --timeout-seconds=90 --out=core/local/artifacts/research_golden_after_query_metadata_recovery_live2.json --out-latest=artifacts/research_golden_after_query_metadata_recovery_live2.json --out-markdown=artifacts/research_golden_after_query_metadata_recovery_live2.md --failures-out=local/state/ops/research_golden/query_metadata_recovery_live2_failures.jsonl`
- `before_metrics`: `average_score=89.0`, `passed_cases=1/2`, `excellent_cases=0/2`, `web_2_query_metadata_present=0/2`, query lanes/case `0.0`, keywords/case `0.0`.
- `after_metrics`: `average_score=96.0`, `passed_cases=2/2`, `excellent_cases=0/2`, `web_2_query_metadata_present=2/2`, query lanes/case `3.0`, followups/case `2.0`, keywords/case `12.0`; next weakest gates became `web_5d_source_quality_ready=1/2`, `web_5e_claim_quality_ready=1/2`, `web_5g_answerability_ready=0/2`.
- `visible_output_delta`: the title-shell drug-discovery failure became a bounded answer that distinguished one supported AI-interpretability result from the unsupported drug-discovery facets. Nuclear permitting remained bounded but not excellent due weak evidence quality.
- `decision`: `kept`
- `reason`: This directly fixed the most upstream failed gate and improved pass rate without hardcoding a sample query or visible answer format. It moves the bottleneck downstream from request planning to evidence/source quality.
- `follow_up`: Work top-down from the new weakest gates: source-quality and answerability. The next patch should use row-level evidence diagnostics to explain why source-quality is still only `1/2` and why answerable evidence is not becoming enough concrete, citable answer material.

### 2026-05-29: facet stopword tightening experiment

- `baseline_artifact`: `artifacts/research_golden_after_query_metadata_recovery_live2.json`
- `patch_name`: remove generic filler terms from recovered query facets
- `failure_class_targeted`: poor generated follow-up lane such as `United States research current status`.
- `hypothesis`: If query facet extraction drops generic filler words, generated follow-up lanes should keep more distinctive subject terms and improve source quality.
- `files_changed`: `core/layer0/ops/src/dashboard_compat_api_parts/set_config_payload_parts/190_route_blocks/agent_scope_full_parts/046b-manual-toolbox-pending-request.rs`
- `proof_tests`: `cargo test --manifest-path core/layer0/ops/Cargo.toml --lib manual_toolbox_pending_request_tests -- --nocapture` passed while experiment was applied.
- `eval_command`: `cargo run --manifest-path orchestration/Cargo.toml --bin eval_runtime -- research-golden --live=1 --base-url=http://127.0.0.1:5173 --limit=2 --sample-seed=random:6d09e741b184df5d5052ac05fd27b875bc9829f3c62ca6da69e37eb955207a47 --timeout-seconds=90 --out=core/local/artifacts/research_golden_after_query_metadata_recovery_tightened_live2.json --out-latest=artifacts/research_golden_after_query_metadata_recovery_tightened_live2.json --out-markdown=artifacts/research_golden_after_query_metadata_recovery_tightened_live2.md --failures-out=local/state/ops/research_golden/query_metadata_recovery_tightened_live2_failures.jsonl`
- `before_metrics`: metadata recovery run had `average_score=96.0`, `passed_cases=2/2`, `web_2_query_metadata_present=2/2`, `web_5d_source_quality_ready=1/2`, `web_5g_answerability_ready=0/2`.
- `after_metrics`: `average_score=87.5`, `passed_cases=1/2`, `excellent_cases=0/2`; `web_2_query_metadata_present` stayed `2/2` and some downstream gates improved, but nuclear permitting regressed to `82/fail`.
- `visible_output_delta`: the tightened lanes produced a worse user-facing outcome on the nuclear case despite better-looking lower-level gates.
- `decision`: `reverted`
- `reason`: This violated the one-change rule. The metric movement did not align with user-facing quality, so the experiment is logged as a dead end rather than kept.
- `follow_up`: Do not tune query facet stopwords blindly. Use the new metadata hydration as the stable baseline and target source-quality/answerability with evidence-row diagnostics, preferably by inspecting actual selected evidence rows before changing query generation again.

### 2026-05-29: compound subject query-lane experiment

- `baseline_artifact`: `artifacts/research_golden_after_restart_metadata_kept_live2.json`
- `patch_name`: preserve compound subject phrases and add targeted primary-source query lanes during recovered batch-query metadata repair
- `failure_class_targeted`: `web_5d_source_quality_ready` / recovered metadata reduced compound subjects such as `AI drug discovery` to a broad entity like `AI`, and generated generic lanes such as `United States research current status`.
- `hypothesis`: If recovered query metadata carries compound subject phrases into `required_coverage.entities` and targeted source-class lanes, source quality should improve without relying on prompt-specific rules.
- `files_changed`: `core/layer0/ops/src/dashboard_compat_api_parts/set_config_payload_parts/190_route_blocks/agent_scope_full_parts/046b-manual-toolbox-pending-request.rs`
- `proof_tests`: `cargo test --manifest-path core/layer0/ops/Cargo.toml --lib manual_toolbox_pending_request_tests -- --nocapture` passed while experiment was applied.
- `eval_command`: `cargo run --manifest-path orchestration/Cargo.toml --bin eval_runtime -- research-golden --live=1 --base-url=http://127.0.0.1:5173 --limit=2 --sample-seed=random:6d09e741b184df5d5052ac05fd27b875bc9829f3c62ca6da69e37eb955207a47 --timeout-seconds=90 --out=core/local/artifacts/research_golden_after_compound_subject_lanes_live2.json --out-latest=artifacts/research_golden_after_compound_subject_lanes_live2.json --out-markdown=artifacts/research_golden_after_compound_subject_lanes_live2.md --failures-out=local/state/ops/research_golden/compound_subject_lanes_live2_failures.jsonl`
- `before_metrics`: `average_score=93.0`, `passed_cases=2/2`, `excellent_cases=0/2`, `web_5d_source_quality_ready=0/2`, `web_5g_answerability_ready=0/2`, `web_7_usable_evidence_available=0/2`.
- `after_metrics`: `average_score=92.0`, `passed_cases=1/2`, `excellent_cases=0/2`, `web_5d_source_quality_ready=2/2`, `web_5g_answerability_ready=0/2`, `web_7_usable_evidence_available=0/2`.
- `visible_output_delta`: source-quality metrics improved, but the drug-discovery answer regressed into a source-title fragment after `answer_units_not_traceable_to_evidence`: `From Radiology to Drug Discovery, Survey Reveals AI Is Delivering Clear Return on Investment in Healthcare. Integrating AI and Machine Learning in Drug Discovery and Development PharmTech.`
- `decision`: `reverted`
- `reason`: The targeted web-tooling gate improved, but user-facing pass rate and average score regressed. This violates the one-measurable-change rule because better source-quality accounting did not produce a better answer.
- `follow_up`: The artifact suggests the next primitive failure is post-retrieval answerability and fallback acceptance: when the LLM rejects an answer as untraceable, fallback must not surface article titles as the answer. Fix the fallback/answer-unit path before trying to broaden query lanes again.

### 2026-05-29: source-title fallback acceptance guard

- `baseline_artifact`: `artifacts/research_golden_after_restart_metadata_kept_live2.json`
- `patch_name`: reject title/source-inventory shaped fallback text as substantive answer depth after traceability or evidence-depth verifier rejects.
- `failure_class_targeted`: `post_tool_synthesis_not_useful` / source titles and article-headline inventories being accepted as user-facing answers after the LLM response was rejected as untraceable.
- `hypothesis`: If fallback depth checks treat source-title inventories as low-information, rejected LLM synthesis cannot degrade into visible title fragments; it must either rebuild from traceable evidence units or emit a bounded insufficiency answer.
- `files_changed`: `core/layer0/ops/src/dashboard_compat_api_parts/set_config_payload_parts/190_route_blocks/agent_scope_full_parts/047-turn-workflow-synthesis_parts/007-part.rs`, `core/layer0/ops/src/dashboard_compat_api_parts/set_config_payload_parts/190_route_blocks/agent_scope_full_parts/047-turn-workflow-synthesis_parts/903-workflow-fallback-tests.rs`, `core/layer0/ops/src/dashboard_compat_api_parts/set_config_payload_parts/190_route_blocks/agent_scope_full_parts/047-turn-workflow-synthesis_parts/907-workflow-fallback-tests.rs`
- `proof_tests`: `cargo test --manifest-path core/layer0/ops/Cargo.toml --lib title_shell_fallback_is_not_substantive_depth_for_traceability_reject -- --nocapture` passed; `cargo test --manifest-path core/layer0/ops/Cargo.toml --lib workflow_fallback_tests -- --nocapture` passed with `147 passed`.
- `eval_command`: `cargo run --manifest-path orchestration/Cargo.toml --bin eval_runtime -- research-golden --live=1 --base-url=http://127.0.0.1:5173 --limit=2 --sample-seed=random:6d09e741b184df5d5052ac05fd27b875bc9829f3c62ca6da69e37eb955207a47 --timeout-seconds=90 --out=core/local/artifacts/research_golden_after_title_inventory_fallback_guard_live2.json --out-latest=artifacts/research_golden_after_title_inventory_fallback_guard_live2.json --out-markdown=artifacts/research_golden_after_title_inventory_fallback_guard_live2.md --failures-out=local/state/ops/research_golden/title_inventory_fallback_guard_live2_failures.jsonl`
- `before_metrics`: `average_score=93.0`, `passed_cases=2/2`, `excellent_cases=0/2`, `web_5d_source_quality_ready=0/2`, `web_5f_citation_renderability_ready=1/2`, `web_5g_answerability_ready=0/2`, `web_7_usable_evidence_available=0/2`.
- `after_metrics`: `average_score=93.0`, `passed_cases=2/2`, `excellent_cases=0/2`, `web_5d_source_quality_ready=1/2`, `web_5f_citation_renderability_ready=2/2`, `web_5g_answerability_ready=0/2`, `web_7_usable_evidence_available=0/2`.
- `visible_output_delta`: the known title-shell failure did not recur. Drug discovery remained bounded and evidence-limited; nuclear permitting remained bounded and explicit about insufficient source-backed evidence.
- `decision`: `kept`
- `reason`: This patch does not solve answerability, but it blocks a concrete bad-output class without reducing pass rate or average score. It also keeps the system honest when synthesis rejects an untraceable answer.
- `follow_up`: The top remaining primitive failure is still answerable evidence quality after retrieval: `web_5g_answerability_ready=0/2` and `web_7_usable_evidence_available=0/2`. Next work should inspect selected evidence rows for why concrete, citable answer units are not being produced, before changing query generation again.

### 2026-05-30: subject-phrase metadata narrowing experiment

- `baseline_artifact`: `artifacts/research_golden_after_title_inventory_fallback_guard_live2.json`
- `patch_name`: preserve subject phrases after generic relation anchors during recovered batch-query metadata repair.
- `failure_class_targeted`: `web_5d_source_quality_ready` / broad recovered entity coverage such as `AI` or `United States` losing lower-case compound subjects.
- `hypothesis`: If recovered query metadata preserves narrow subject phrases as entities, the search lanes should produce more relevant candidate evidence without hardcoding a sample query.
- `files_changed`: `core/layer0/ops/src/dashboard_compat_api_parts/set_config_payload_parts/190_route_blocks/agent_scope_full_parts/046b-manual-toolbox-pending-request.rs`
- `proof_tests`: `cargo test --manifest-path core/layer0/ops/Cargo.toml --lib manual_toolbox_pending_request_tests -- --nocapture` passed while experiment was applied.
- `eval_command`: `cargo run --manifest-path orchestration/Cargo.toml --bin eval_runtime -- research-golden --live=1 --base-url=http://127.0.0.1:5173 --limit=2 --sample-seed=random:6d09e741b184df5d5052ac05fd27b875bc9829f3c62ca6da69e37eb955207a47 --timeout-seconds=90 --out=core/local/artifacts/research_golden_after_subject_phrase_metadata_live2.json --out-latest=artifacts/research_golden_after_subject_phrase_metadata_live2.json --out-markdown=artifacts/research_golden_after_subject_phrase_metadata_live2.md --failures-out=local/state/ops/research_golden/subject_phrase_metadata_live2_failures.jsonl`
- `before_metrics`: `average_score=93.0`, `passed_cases=2/2`, `excellent_cases=0/2`, `query_lanes_per_case=3.0`, `required_entities_per_case=1.0`, `web_5d_source_quality_ready=1/2`, `web_5g_answerability_ready=0/2`, `web_7_usable_evidence_available=0/2`.
- `after_metrics`: `average_score=93.0`, `passed_cases=1/2`, `excellent_cases=0/2`, `query_lanes_per_case=4.5`, `required_entities_per_case=2.5`, `web_5d_source_quality_ready=1/2`, `web_5g_answerability_ready=0/2`, `web_7_usable_evidence_available=0/2`.
- `visible_output_delta`: the drug-discovery case fell from pass to fail because unsupported answer-unit traceability surfaced again, even though query metadata became richer.
- `decision`: `reverted`
- `reason`: The patch improved metadata volume but did not improve the upstream quality gates and reduced live pass rate. Under the one-measurable-change rule, richer query metadata is not enough unless it improves user-facing or gate quality.
- `follow_up`: Do not try another query-lane expansion yet. Consult the research-system artifacts for a more primitive post-retrieval patch around evidence compaction, evidence-card construction, or answer-unit grounding, because `web_5g_answerability_ready` and `web_7_usable_evidence_available` remain the stable bottleneck.

### 2026-05-30: headline/dateline evidence-quality calibration

- `baseline_artifact`: `artifacts/research_golden_after_title_inventory_fallback_guard_live2.json`
- `artifact_guidance`: Jina/Tavily/Firecrawl ledgers all warn that search rows, provider headlines, and raw provider summaries are candidates until extraction produces answerable evidence cards.
- `patch_name`: classify headline/question/dateline shells as low-quality evidence and claims in web-tooling diagnostics.
- `failure_class_targeted`: `web_5e_claim_quality_ready` false positive where a source title such as `What’s really happening... Published... Source...` was counted as concrete claim material.
- `hypothesis`: If diagnostics stop counting headline/dateline rows as clean evidence or concrete claims, `web_5g_answerability_ready` and `web_7_usable_evidence_available` will better reflect whether the tooling returned text an LLM can actually use.
- `files_changed`: `orchestration/src/eval_web_retrieval_gate_diagnostics_parts/050_evidence_quality.rs`, `orchestration/src/eval_web_retrieval_gate_diagnostics_parts/091_tests_claims_and_quality.rs`
- `proof_tests`: `cargo test --manifest-path orchestration/Cargo.toml --bin eval_runtime evidence_quality_gates -- --nocapture` passed `3/3`.
- `eval_command`: not rerun live for this calibration patch; it changes diagnostic classification over existing evidence rows, not retrieval/runtime behavior.
- `before_metrics`: title-inventory guard run still showed `claim_quality_ready=1/2` even though the AI drug discovery sample row's only visible claim was a question-style headline/dateline shell.
- `after_metrics`: deterministic fixture proves the same generic row shape now fails source quality, claim quality, and evidence-packet readiness instead of masquerading as answerable evidence.
- `visible_output_delta`: no direct user-facing text change intended.
- `decision`: `kept`
- `reason`: This is a measurement-alignment patch. It prevents metrics from rewarding the exact fragmentary source-row behavior the user flagged as bad, without imposing a final-answer format or topic-specific rule.
- `follow_up`: Next live run should use this stricter diagnostic to identify the true first useful-data bottleneck. If gates now look worse, that is expected honesty, not a product regression.
