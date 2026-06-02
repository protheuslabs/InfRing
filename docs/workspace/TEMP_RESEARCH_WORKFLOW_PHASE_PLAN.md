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

- We are working the Phase 2 downstream lane while keeping Phase 1 calibration honest.
- The good-evidence lane now has a 100-case fixture with ready evidence packets and no exact-answer keys.
- The current downstream scoreboard should distinguish three things: live synthesis quality, offline regrade of saved answers, and production handoff/package shape.
- The next action is to expand beyond the first five downstream cases and find the next general failure class, while treating any "sounds good but hard failed" mismatch as a grader-calibration bug unless the visible answer is actually weak.

## Iteration Log

### 2026-06-01: generic discourse-term answer-alignment calibration

- `baseline_artifact`: `core/local/artifacts/research_perfect_evidence_replay_after_listing_patch_live5.json`
- `patch_name`: ignore generic discourse term `whichever` in answer-unit evidence alignment
- `failure_class_targeted`: grader hard-failed a visibly useful answer because a generic connective word was treated as an unsupported concrete claim term
- `hypothesis`: If generic discourse terms are excluded from answer-specific claim terms, then useful answers with strong evidence support will not hard-fail on non-claim language, while unsupported concrete named claims still fail.
- `files_changed`: `orchestration/src/eval_research_golden_scoring_parts/051_answer_alignment.rs`, `orchestration/src/eval_research_golden_scoring_parts/092_tests_relevance_and_citations.rs`
- `proof_tests`: `cargo test --manifest-path orchestration/Cargo.toml answer_alignment_ignores_checklist_and_interface_scaffold_terms -- --nocapture`
- `eval_command`: `cargo run --quiet --manifest-path orchestration/Cargo.toml --bin eval_runtime -- research-perfect-evidence --mode=test-input-regrade --strict=0 --limit=5 --responses=core/local/artifacts/research_perfect_evidence_test_mode_responses_after_listing_patch_live5.json --out=core/local/artifacts/research_perfect_evidence_regrade_after_whichever_patch_live5.json --out-latest=artifacts/research_perfect_evidence_regrade_after_whichever_patch_live5.json --out-markdown=artifacts/research_perfect_evidence_regrade_after_whichever_patch_live5.md`
- `before_metrics`: live replay `5 cases`, `pass_rate=0.8`, `excellent_rate=0.8`, `synthesized_sounds_good_rate=1.0`; failed case `perfect_ev_002_js_heavy_scraping` had `score=100`, visible answer sounded good, and only failed on `answer_units_not_traceable_to_evidence` caused by unsupported term `whichever`.
- `after_metrics`: offline regrade `pass_rate=1.0`, `excellent_rate=1.0`, `synthesized_sounds_good_rate=1.0`, `synthesis_hard_failures=0`; production handoff replay also `pass_rate=1.0`, `excellent_rate=1.0`, `handoff_contract_pass_rate=1.0`, `source_refs_present_rate=1.0`, `citation_package_present_rate=1.0`.
- `visible_output_delta`: no answer-generation change; the same useful JS-heavy scraping answer is no longer mislabeled as a hard failure.
- `decision`: `kept`
- `reason`: This is a generic grader calibration fix, not a prompt/domain patch. It removes false hard failures from discourse scaffolding while leaving concrete unsupported claim checks intact.
- `follow_up`: Run a larger downstream replay/regrade sample to find the next visible-answer or handoff failure class.

### 2026-06-01: presentation-label and temporal-scaffold answer-alignment calibration

- `baseline_artifact`: `core/local/artifacts/research_perfect_evidence_replay_after_whichever_patch_live20.json`
- `patch_name`: ignore temporal/scaffold term `during` and presentation-label `Target:` without suppressing `Target` as a named entity
- `failure_class_targeted`: Excellent false negatives from answer-unit alignment treating non-claim scaffold words as unsupported concrete terms
- `hypothesis`: If the answer-alignment term extractor ignores generic temporal scaffolding and presentation labels while preserving named entities in ordinary prose, then useful answers will stop losing Excellent on formatting/scaffold text and entity-support checks will remain intact.
- `files_changed`: `orchestration/src/eval_research_golden_scoring_parts/051_answer_alignment.rs`, `orchestration/src/eval_research_golden_scoring_parts/092_tests_relevance_and_citations.rs`
- `proof_tests`: `cargo test --manifest-path orchestration/Cargo.toml answer_alignment_ignores_checklist_and_interface_scaffold_terms -- --nocapture`; `cargo test --manifest-path orchestration/Cargo.toml answer_alignment_keeps_target_as_named_entity_when_not_a_label -- --nocapture`
- `eval_command`: `cargo run --quiet --manifest-path orchestration/Cargo.toml --bin eval_runtime -- research-perfect-evidence --mode=test-input-regrade --strict=0 --limit=20 --responses=core/local/artifacts/research_perfect_evidence_test_mode_responses_after_whichever_patch_live20.json --out=core/local/artifacts/research_perfect_evidence_regrade_after_scaffold_terms_live20.json --out-latest=artifacts/research_perfect_evidence_regrade_after_scaffold_terms_live20.json --out-markdown=artifacts/research_perfect_evidence_regrade_after_scaffold_terms_live20.md`
- `before_metrics`: live 20-case downstream replay `pass_rate=1.0`, `excellent_rate=0.85`, `synthesized_sounds_good_rate=1.0`, `synthesis_hard_failures=0`; non-Excellent cases were `perfect_ev_004_scientific_breakthroughs_april_2026`, `perfect_ev_012_us_privacy_laws_2026`, and `perfect_ev_013_noise_canceling_headphones`.
- `after_metrics`: offline regrade `pass_rate=1.0`, `excellent_rate=0.95`, `synthesized_sounds_good_rate=1.0`, `synthesis_hard_failures=0`; production handoff replay `handoff_contract_pass_rate=1.0`, `source_refs_present_rate=1.0`, `citation_package_present_rate=1.0`.
- `visible_output_delta`: no answer-generation change; same saved answers were regraded. The two scaffold-driven false Excellent misses now pass, while the privacy-law case remains non-Excellent because the answer honestly reports that the provided evidence lacks named statutes/jurisdictions requested by the prompt.
- `decision`: `kept`
- `reason`: The patch targets a generic extraction failure class and adds a guard that `Target` still counts as a named entity outside a presentation-label context.
- `follow_up`: Continue with larger or randomized downstream batches. If remaining non-Excellent cases are caused by evidence lacking requested specificity, do not patch synthesis/grading; route that back to evidence quality or fixture quality.

### 2026-06-02: broad recent evidence queries can promote authoritative rows

- `baseline_artifact`: `core/local/artifacts/web_tooling_user_pool_random5_after_authority_guard_fresh.json`
- `patch_name`: split strict current-news freshness from broad recent evidence-landscape freshness
- `failure_class_targeted`: pack-ready candidate promotion vetoed authoritative, on-topic evidence because `recent` was treated like `latest`/`this week`
- `hypothesis`: If broad recent evidence-landscape prompts can use stable authoritative rows even when exact freshness is unproven, then candidate supply that already has strong source-backed material will become evidence refs/claims instead of failing at `web_5_packaged_evidence_present`.
- `files_changed`: `core/layer2/ops/src/retrieval_policy_parts/010-freshness-and-relevance.rs`
- `proof_tests`: `cargo test --manifest-path core/layer2/ops/Cargo.toml recent_evidence_landscape_allows_authoritative_rows_without_exact_freshness -- --nocapture`; `cargo test --manifest-path core/layer2/ops/Cargo.toml relative_current_freshness -- --nocapture`; `cargo test --manifest-path core/layer2/ops/Cargo.toml stable_authoritative_evidence -- --nocapture`
- `eval_command`: `cargo run --quiet --manifest-path orchestration/Cargo.toml --bin eval_runtime -- web-tooling-golden --live=1 --base-url=http://127.0.0.1:5174 --cases=validation/evals/fixtures/research_user_prompt_pool_v1.json --request-packs-from=/tmp/nonexistent_web_tooling_request_packs_empty.json --sample-size=5 --sample-seed=random:1f230cdf199ca467 --strict=0 --timeout-seconds=120 --out=core/local/artifacts/web_tooling_user_pool_random5_after_recent_evidence_patch_fixed_seed.json --out-latest=artifacts/web_tooling_user_pool_random5_after_recent_evidence_patch_fixed_seed_latest.json --out-markdown=local/workspace/reports/WEB_TOOLING_USER_POOL_RANDOM5_AFTER_RECENT_EVIDENCE_PATCH_FIXED_SEED.md`
- `before_metrics`: fixed-seed live web tooling batch `5 cases`, `success_rate=0.8`, `transport_failures=0`; failed case `research_pool_016_superconductor_claims` first failed at `web_5_packaged_evidence_present`, `retrieval_status=conflicting_provider_state`, `candidate_count=65`, `content_rich_candidate_count=51`, `evidence_ref_count=0`, `evidence_claim_count=0`, `evidence_pack_quality.status=absent`.
- `after_metrics`: same seed and same selected case IDs `success_rate=1.0`, `transport_failures=0`, all web-tooling gates `5/5`; `research_pool_016_superconductor_claims` now has `retrieval_status=usable`, `candidate_count=45`, `content_rich_candidate_count=6`, `evidence_ref_count=3`, `evidence_claim_count=3`, `evidence_pack_quality.status=usable`.
- `visible_output_delta`: tooling-level change only; the evidence package now contains citable claims from Yale News, GWU, and MIT instead of returning a no-results/low-signal state despite relevant candidates existing.
- `decision`: `kept`
- `reason`: The patch targets a generic freshness-policy failure class. Strict current-news windows still require current signals, while broad "recent evidence/claims landscape" prompts no longer discard authoritative source-backed evidence solely because exact freshness is unproven.
- `follow_up`: Run a larger random broad web-tooling batch to check whether this generalizes, then inspect any remaining first-failure gate before touching downstream synthesis.

### 2026-06-02: rejected broad `for X` coverage-modifier promotion

- `baseline_artifact`: `core/local/artifacts/web_tooling_user_pool_random10_after_recent_evidence_patch.json`
- `patch_name`: infer `for X` clauses into required coverage facets when explicit metadata is thin
- `failure_class_targeted`: source-sensitive answerability failure where the original prompt carried an important modifier (`for women`) but explicit required coverage only preserved the broad topic (`creatine`)
- `hypothesis`: If `for X` modifiers are preserved as coverage facets, then early query lanes should keep the user's intended audience/population/use-case and improve source-sensitive answerability.
- `files_changed`: reverted changes to `core/layer0/ops/src/batch_query_primitive_parts/018-request-and-cache.rs` and `core/layer0/ops/src/batch_query_primitive_parts/042-cache-rewrite-tests.rs`
- `proof_tests`: `cargo test --manifest-path core/layer0/ops/Cargo.toml explicit_metadata_query_plan_preserves_for_clause_modifiers_as_facets -- --nocapture` passed before the patch was reverted
- `eval_command`: `cargo run --quiet --manifest-path orchestration/Cargo.toml --bin eval_runtime -- web-tooling-golden --live=1 --base-url=http://127.0.0.1:5174 --cases=validation/evals/fixtures/research_user_prompt_pool_v1.json --request-packs-from=/tmp/nonexistent_web_tooling_request_packs_empty.json --sample-size=10 --sample-seed=random:eddc2b3c310292df --strict=0 --timeout-seconds=120 --out=core/local/artifacts/web_tooling_user_pool_random10_after_for_modifier_patch_fixed_seed.json --out-latest=artifacts/web_tooling_user_pool_random10_after_for_modifier_patch_fixed_seed_latest.json --out-markdown=local/workspace/reports/WEB_TOOLING_USER_POOL_RANDOM10_AFTER_FOR_MODIFIER_PATCH_FIXED_SEED.md`
- `before_metrics`: fixed-seed live web-tooling batch `success_rate=0.9`; weakest gates were `web_5g_answerability_ready=0.9` and `web_7_usable_evidence_available=0.9`; lone failed case was `research_pool_077_creatine_for_women`, first failed at `web_5g_answerability_ready`.
- `after_metrics`: same seed dropped to `success_rate=0.5`; first failures included `web_3b1_provider_quota_not_rate_limited=0.8`, `web_5c_claim_extraction_present=0.7`, `web_5d_source_quality_ready=0.5`, and `web_7_usable_evidence_available=0.5`.
- `visible_output_delta`: the target case did preserve `women` as a facet, but this produced a weak standalone `women source-backed evidence` lane and coincided with provider degradation; unrelated cases also regressed.
- `decision`: `reverted`
- `reason`: The patch was too blunt. It changed early query shape for unrelated prompts and did not produce an obvious positive fixed-seed improvement. The idea may still be valid, but modifier handling needs a more careful representation than promoting every `for X` clause to a standalone facet.
- `follow_up`: If revisiting modifier preservation, treat modifiers as anchored query constraints attached to a topic/entity, not independent facets that can generate standalone broad searches.

### 2026-06-02: informational selection prompts include promising approaches

- `baseline_artifact`: `core/local/artifacts/research_perfect_evidence_replay_random20_after_named_scaffold_patch_fresh.json`
- `patch_name`: accept generic "which approaches/strategies look most promising" selection prompts as decision-value prompts
- `failure_class_targeted`: grader Excellent false negative where a useful answer identified promising approaches and blockers, but `decision_value` stayed false because the prompt shape was broader than laws/products/programs/options.
- `hypothesis`: If informational selection detection includes approach/strategy/promising language, then broad research answers that name a supported set of approaches with evaluative language will earn decision value without requiring a fixed output format or domain-specific facts.
- `files_changed`: `orchestration/src/eval_research_golden_scoring_parts/060_prompt_entities.rs`, `orchestration/src/eval_research_golden_scoring_parts/090_tests_retrieval_and_query.rs`
- `proof_tests`: `cargo test --manifest-path orchestration/Cargo.toml decision_value_accepts_promising_approaches_selection_prompts -- --nocapture`
- `eval_command`: `cargo run --quiet --manifest-path orchestration/Cargo.toml --bin eval_runtime -- research-perfect-evidence --mode=test-input-regrade --strict=0 --limit=20 --responses=core/local/artifacts/research_perfect_evidence_responses_random20_after_named_scaffold_patch_fresh.json --out=core/local/artifacts/research_perfect_evidence_regrade_random20_after_promising_approaches_patch.json --out-latest=artifacts/research_perfect_evidence_regrade_random20_after_promising_approaches_patch_latest.json --out-markdown=local/workspace/reports/RESEARCH_PERFECT_EVIDENCE_REGRADE_RANDOM20_AFTER_PROMISING_APPROACHES_PATCH.md`
- `before_metrics`: 20-case downstream perfect-evidence replay `pass_rate=1.0`, `excellent_rate=0.9`, `synthesized_sounds_good_rate=1.0`, `synthesis_hard_failures=0`; `perfect_ev_034_crispr_delivery_progress` missed Excellent only on `missing_decision_value` despite a useful answer naming delivery approaches and blockers.
- `after_metrics`: offline regrade of the same 20 saved outputs `pass_rate=1.0`, `excellent_rate=0.95`, `synthesized_sounds_good_rate=1.0`, `synthesis_hard_failures=0`; the CRISPR case now earns Excellent.
- `visible_output_delta`: no answer-generation change; this is grader calibration. The remaining non-Excellent case is `perfect_ev_012_us_privacy_laws_2026`, where the prompt asks which state laws matter but the synthetic evidence does not name specific statutes or jurisdictions, so the useful bounded answer correctly cannot earn Excellent.
- `decision`: `kept`
- `reason`: The patch targets a reusable prompt-shape class rather than a specific domain, improves a known human/grader mismatch, and leaves evidence-specificity limits intact.
- `follow_up`: Decide whether to repair the privacy fixture as an evidence-input quality issue, then run another fresh random 20-case downstream replay or shift back to native evidence quality once downstream remains at 19/20+ Excellent.

### 2026-06-02: rejected safe weak-facet coverage recovery query construction

- `baseline_artifact`: `core/local/artifacts/web_tooling_user_pool_random10_after_recent_evidence_patch.json`
- `patch_name`: trigger coverage-gap recovery from pack-ready facet gaps and allow no-entity templates to become short facet-only recovery queries
- `failure_class_targeted`: weak facet coverage looked satisfied by candidate-only/thin rows, and no-entity recovery could fall through to long `{query} {facet}` concatenations.
- `hypothesis`: If weak facets require pack-ready evidence before being considered covered, and no-entity recovery uses short facet-source-class lanes, then weak evidence packets should recover without broad prompt-shaped follow-up queries.
- `files_changed`: reverted changes to `core/layer2/ops/src/retrieval_policy_parts/010-freshness-and-relevance.rs`
- `proof_tests`: `cargo test --manifest-path core/layer2/ops/Cargo.toml coverage_gap_recovery_targets_pack_ready_gaps_with_short_facet_queries -- --nocapture` passed while the experiment was applied; `cargo test --manifest-path core/layer2/ops/Cargo.toml evidence_packet_promotion_tests -- --nocapture` passed while the experiment was applied.
- `eval_command`: `cargo run --quiet --manifest-path orchestration/Cargo.toml --bin eval_runtime -- web-tooling-golden --live=1 --base-url=http://127.0.0.1:5174 --cases=validation/evals/fixtures/research_user_prompt_pool_v1.json --request-packs-from=/tmp/nonexistent_web_tooling_request_packs_empty.json --sample-size=10 --sample-seed=random:eddc2b3c310292df --strict=0 --timeout-seconds=120 --out=core/local/artifacts/web_tooling_user_pool_random10_after_safe_weak_facet_recovery_patch_fixed_seed.json --out-latest=artifacts/web_tooling_user_pool_random10_after_safe_weak_facet_recovery_patch_fixed_seed_latest.json --out-markdown=local/workspace/reports/WEB_TOOLING_USER_POOL_RANDOM10_AFTER_SAFE_WEAK_FACET_RECOVERY_PATCH_FIXED_SEED.md`
- `before_metrics`: fixed-seed live web-tooling batch `success_rate=0.9`; weakest gates were `web_5g_answerability_ready=0.9` and `web_7_usable_evidence_available=0.9`; lone failed case was `research_pool_077_creatine_for_women`.
- `after_metrics`: same seed dropped to `success_rate=0.6`; weak gates included `web_3b1_provider_quota_not_rate_limited=0.9`, `web_5c_claim_extraction_present=0.8`, `web_5g_answerability_ready=0.7`, and `web_7_usable_evidence_available=0.6`.
- `visible_output_delta`: the intended meeting-overload target regressed from a usable packet with three evidence refs to `no_results` with first failure `web_3b1_provider_quota_not_rate_limited`. The generated recovery path did not produce a broad product improvement.
- `decision`: `reverted`
- `reason`: The patch made recovery more aggressive in a way that increased provider pressure and reduced fixed-seed success. The local invariant was real, but the live pipeline needs a safer recovery-admission policy, not simply more/generated recovery lanes.
- `follow_up`: Target recovery admission and provider-budget economics next: do not spend scarce live provider budget on generated recovery unless the existing candidate set proves recoverable and the recovery lane is likely to improve answer-ready evidence.

### 2026-06-02: compact outcome-evidence claim extraction

- `baseline_artifact`: `core/local/artifacts/web_tooling_user_pool_random10_after_recent_evidence_patch.json`
- `patch_name`: treat compact outcome/effectiveness statements as answerable claim evidence when they have specific anchors
- `failure_class_targeted`: authoritative source snippets with result phrasing such as `appears to be effective for improving...` were present in provider output but failed claim extraction and pack-readiness because they were shorter than the usual content-rich threshold.
- `hypothesis`: If compact outcome statements with specific answer anchors count as claim material and answerable extract substance, then good authoritative rows can become evidence without adding query lanes or domain-specific rules.
- `files_changed`: `core/layer2/ops/src/retrieval_policy_parts/010-freshness-and-relevance.rs`
- `proof_tests`: `cargo test --manifest-path core/layer2/ops/Cargo.toml outcome_evidence_language_becomes_claim_hint_for_authoritative_rows -- --nocapture`; `cargo test --manifest-path core/layer2/ops/Cargo.toml evidence_packet_promotion_tests -- --nocapture`
- `eval_command`: `cargo run --quiet --manifest-path orchestration/Cargo.toml --bin eval_runtime -- web-tooling-golden --live=1 --base-url=http://127.0.0.1:5174 --cases=validation/evals/fixtures/research_user_prompt_pool_v1.json --request-packs-from=/tmp/nonexistent_web_tooling_request_packs_empty.json --sample-size=10 --sample-seed=random:eddc2b3c310292df --strict=0 --timeout-seconds=120 --out=core/local/artifacts/web_tooling_user_pool_random10_after_outcome_claim_patch_fixed_seed.json --out-latest=artifacts/web_tooling_user_pool_random10_after_outcome_claim_patch_fixed_seed_latest.json --out-markdown=local/workspace/reports/WEB_TOOLING_USER_POOL_RANDOM10_AFTER_OUTCOME_CLAIM_PATCH_FIXED_SEED.md`
- `before_metrics`: fixed-seed live web-tooling batch `success_rate=0.9`; weakest gates were `web_5g_answerability_ready=0.9` and `web_7_usable_evidence_available=0.9`; lone failed case was `research_pool_077_creatine_for_women`, with `source_authority_ready=false`, `authority_grade_source_domain_count=1`, and a women-specific PMC row present in the actionable pool but not pack-ready.
- `after_metrics`: same seed live run `success_rate=0.8`; target case `research_pool_077_creatine_for_women` passed with `answerability_ready=true`, `source_authority_ready=true`, `authority_grade_source_domain_count=2`, and selected evidence from `pmc.ncbi.nlm.nih.gov`, `sciencedirect.com`, and `lifestylemedicine.stanford.edu`. Aggregate failures were `research_pool_100_meeting_overload_remote_teams` at `web_3b1_provider_quota_not_rate_limited` after Tavily/Exa circuit-open signals, and `research_pool_031_state_ai_regulation` at `web_5d_source_quality_ready` after provider fallback returned off-topic weak material.
- `visible_output_delta`: tooling-level change only; the target evidence packet now includes the concrete claim `Creatine supplementation among pre-menopausal females appears to be effective for improving strength and exercise performance` from the PMC row instead of selecting weaker source/title-like snippets.
- `decision`: `kept`
- `reason`: The targeted generic failure disappeared and local negative guards for shells, vague teasers, bylines, and media rows still pass. The aggregate live score is lower because late cases hit provider circuit/quota degradation, which is a separate upstream bottleneck already visible in the gates.
- `follow_up`: Next iteration should target provider circuit/budget behavior: avoid letting late-run provider degradation collapse otherwise unrelated cases, and separate provider-state noise from evidence-extraction regressions in A/B decisions.

### 2026-06-01: interrogative title-shell evidence quality calibration

- `baseline_artifact`: `core/local/artifacts/web_tooling_user_pool_random5_after_metric_alignment.json`
- `patch_name`: reject short interrogative article titles as claim evidence when they lack factual anchors
- `failure_class_targeted`: article title/question shells were being counted as concrete claim material, making hard evidence-quality gates look greener than the human soft read
- `hypothesis`: If short interrogative title shells without dates, numbers, or reported/announced/found markers are treated as low-quality claim text, evidence packets will expose source rows that are searchable headings rather than usable answer material.
- `files_changed`: `orchestration/src/eval_web_retrieval_gate_diagnostics_parts/050_evidence_quality.rs`, `orchestration/src/eval_web_retrieval_gate_diagnostics_parts/091_tests_claims_and_quality.rs`
- `proof_tests`: `cargo test --manifest-path orchestration/Cargo.toml evidence_quality_gates_reject_interrogative_title_shells_as_claims -- --nocapture`; `cargo test --manifest-path orchestration/Cargo.toml evidence_quality_ -- --nocapture`; `cargo test --manifest-path orchestration/Cargo.toml source_quality_ -- --nocapture`
- `eval_command`: `cargo run --quiet --manifest-path orchestration/Cargo.toml --bin eval_runtime -- evidence-quality-grade --strict=0 --input core/local/artifacts/web_tooling_user_pool_random5_after_metric_alignment.json --out core/local/artifacts/evidence_quality_grade_random5_after_interrogative_shell_guard.json --out-latest artifacts/evidence_quality_grade_random5_after_interrogative_shell_guard_latest.json --out-markdown local/workspace/reports/EVIDENCE_QUALITY_GRADE_RANDOM5_AFTER_INTERROGATIVE_SHELL_GUARD.md`
- `before_metrics`: random live web-tooling batch selected seed `random:5003c3efc78b4076`; hard gates passed `5/5`, but soft inspection found the enterprise AI coding assistant case included `How GitHub Copilot compares to other AI coding assistants` as a claim-like evidence row.
- `after_metrics`: offline evidence replay still passed `5/5`, but `research_pool_021_enterprise_ai_coding_assistants` now has `low_quality_claim_count=1` and marks the title-shell evidence row low quality.
- `visible_output_delta`: no final-answer generation change; the metric now reflects a real evidence defect that was previously invisible.
- `decision`: `kept`
- `reason`: The patch targets a generic evidence-shape failure, not a domain or query. It catches the observed bad row and preserves clean source-backed claim positives.
- `follow_up`: Continue soft-checking random web-tooling packets for cases where gates pass but the evidence would still be awkward or misleading for a synthesizing agent.

### 2026-06-01: source-identity consistency evidence quality calibration

- `baseline_artifact`: `core/local/artifacts/evidence_quality_grade_random5_after_interrogative_shell_guard.json`
- `patch_name`: require generic source identities to match strong publisher signatures in extracted content
- `failure_class_targeted`: a packet row could say the source/URL was one host while the materialized text visibly named another publisher, and the row still counted as ready evidence.
- `hypothesis`: If generic `Web result from <domain>` rows contain a strong `<Publisher> / menu` signature that does not match the row domain, the packet should mark that row as source-identity inconsistent and low quality.
- `files_changed`: `orchestration/src/eval_web_retrieval_gate_diagnostics_parts/050_evidence_quality.rs`, `orchestration/src/eval_web_retrieval_gate_diagnostics_parts/091_tests_claims_and_quality.rs`
- `proof_tests`: `cargo test --manifest-path orchestration/Cargo.toml evidence_packet_contract_rejects_generic_source_identity_with_different_publisher_signature -- --nocapture`; `cargo test --manifest-path orchestration/Cargo.toml evidence_quality_gates_pass_clean_source_backed_claims -- --nocapture`; `cargo test --manifest-path orchestration/Cargo.toml evidence_quality_ -- --nocapture`
- `eval_command`: `cargo run --quiet --manifest-path orchestration/Cargo.toml --bin eval_runtime -- evidence-quality-grade --strict=0 --input core/local/artifacts/web_tooling_user_pool_random5_after_metric_alignment.json --out core/local/artifacts/evidence_quality_grade_random5_after_source_identity_guard.json --out-latest artifacts/evidence_quality_grade_random5_after_source_identity_guard_latest.json --out-markdown local/workspace/reports/EVIDENCE_QUALITY_GRADE_RANDOM5_AFTER_SOURCE_IDENTITY_GUARD.md`
- `before_metrics`: the state AI regulation case contained a row with `source_domain=gazzettaufficiale.it` and locator under `gazzettaufficiale.it`, but the extracted content preview named `Wilson Sonsini / menu`; the row was previously packet-ready.
- `after_metrics`: random5 offline replay still passed `5/5`, but `research_pool_031_state_ai_regulation` now reports `low_quality_flags=["source_identity_mismatch"]`, `missing_fields=["source_identity_consistency"]`, and the mismatched row is `packet_ready=false`.
- `visible_output_delta`: no final-answer generation change; the evidence packet now exposes and discounts a contaminated source row while preserving the rest of the answerable packet.
- `decision`: `kept`
- `reason`: This is a reusable source-fidelity check. It catches a real mismatch from the random lane and does not reject the clean positive source-backed evidence test.
- `follow_up`: The next soft-inspection target is whether source-sensitive topics, especially health/medical questions, need a generic source-authority/readiness metric separate from answer synthesis.

### 2026-06-01: source-sensitive authority readiness calibration

- `baseline_artifact`: `core/local/artifacts/evidence_quality_grade_random5_after_source_identity_guard.json`
- `patch_name`: add source-authority readiness for source-sensitive evidence requests
- `failure_class_targeted`: source-sensitive questions could pass answerability with citable but authority-thin packets, making health/medical evidence look greener than a human soft read would accept.
- `hypothesis`: If source-sensitive requests require at least two authority-grade source domains for full answerability, thin packets can still be source/claim/citation-visible but will no longer be marked answer-ready.
- `files_changed`: `orchestration/src/eval_web_retrieval_gate_diagnostics_parts/050_evidence_quality.rs`, `orchestration/src/eval_web_retrieval_gate_diagnostics_parts/013_evidence_quality_gates.rs`, `orchestration/src/eval_web_retrieval_gate_diagnostics_parts/031_case_metrics.rs`, `orchestration/src/eval_web_retrieval_gate_diagnostics_parts/091_tests_claims_and_quality.rs`, `orchestration/src/eval_evidence_quality_grade.rs`
- `proof_tests`: `cargo test --manifest-path orchestration/Cargo.toml source_sensitive_answerability_requires_multiple_authority_grade_domains -- --nocapture`; `cargo test --manifest-path orchestration/Cargo.toml evidence_quality_ -- --nocapture`; `cargo test --manifest-path orchestration/Cargo.toml answerability_ -- --nocapture`; `cargo test --manifest-path orchestration/Cargo.toml source_quality_ -- --nocapture`
- `eval_command`: `cargo run --quiet --manifest-path orchestration/Cargo.toml --bin eval_runtime -- evidence-quality-grade --strict=0 --input core/local/artifacts/web_tooling_user_pool_random5_after_metric_alignment.json --out core/local/artifacts/evidence_quality_grade_random5_after_source_authority_guard.json --out-latest artifacts/evidence_quality_grade_random5_after_source_authority_guard_latest.json --out-markdown local/workspace/reports/EVIDENCE_QUALITY_GRADE_RANDOM5_AFTER_SOURCE_AUTHORITY_GUARD.md`
- `before_metrics`: random5 evidence replay after source-identity guard passed `5/5`; creatine for women passed despite only one authority-grade domain (`news.cuanschutz.edu`) plus a YouTube row already marked low quality.
- `after_metrics`: random5 replay now passes `4/5`; weakest gate/readiness is `source_authority_ready=0.8`; `answerability_ready=0.8`; `bounded_answerability_ready=0.8`; source quality, claim quality, citation renderability, and packet contract remain `1.0`.
- `visible_output_delta`: no final-answer generation change; evidence diagnostics now say the creatine packet is source-sensitive but not authority-ready, while menopause remains ready with `medlineplus.gov` and `pmc.ncbi.nlm.nih.gov`.
- `decision`: `kept`
- `reason`: The patch targeted a reusable source-sensitivity failure class and moved the exact soft-inspected bad case without knocking down the neighboring health case with multiple authority-grade sources.
- `follow_up`: Next useful iteration is to run a fresh random web-tooling sample and see whether `source_authority_ready` exposes more authority-thin sensitive packets or whether the next bottleneck moves back to downstream synthesis quality.

### 2026-06-01: random sampling support for downstream perfect-evidence replay

- `baseline_artifact`: `core/local/artifacts/research_perfect_evidence_replay_after_scaffold_terms_live20.json`
- `patch_name`: add `--sample-size` and `--sample-seed` to `research-perfect-evidence --mode=test-input-replay`
- `failure_class_targeted`: fixed first-N downstream batches could indirectly overfit to familiar cases and did not preserve a reproducible random sample identity.
- `hypothesis`: If the replay harness can draw a recorded random sample from the 100-case fixture, then downstream progress can be measured on broader case mixes while still supporting exact seeded A/B comparisons.
- `files_changed`: `orchestration/src/eval_research_perfect_evidence.rs`
- `proof_tests`: smoke replay with `--sample-size=1 --sample-seed=test-seed` selected `perfect_ev_089_teletherapy_options` and emitted `case_selection`.
- `eval_command`: `cargo run --quiet --manifest-path orchestration/Cargo.toml --bin eval_runtime -- research-perfect-evidence --mode=test-input-replay --strict=0 --limit=10 --sample-size=10 --timeout-seconds=120 --synthesis-retry-count=1 --out=core/local/artifacts/research_perfect_evidence_replay_random10_after_sampling.json --out-latest=artifacts/research_perfect_evidence_replay_random10_after_sampling.json --out-markdown=artifacts/research_perfect_evidence_replay_random10_after_sampling.md --responses-out=core/local/artifacts/research_perfect_evidence_test_mode_responses_random10_after_sampling.json`
- `before_metrics`: no random sampling support.
- `after_metrics`: selected seed `random:9c78c2bcf2e4e09c`; random10 baseline `pass_rate=0.9`, `excellent_rate=0.6`, `synthesized_sounds_good_rate=1.0`.
- `visible_output_delta`: no synthesis behavior change; the harness now exposes the selected case ids, category counts, tags, seed, and execution count.
- `decision`: `kept`
- `reason`: This is generic eval infrastructure that reduces familiar-case bias and enables exact seeded A/B checks.
- `follow_up`: Use seeded reruns when evaluating a proposed synthesis or grader patch.

### 2026-06-01: acronym-expansion scaffold answer-alignment calibration

- `baseline_artifact`: `/tmp/research_perfect_sample_smoke.json`
- `patch_name`: ignore generic acronym-expansion scaffold terms `assistance`, `program`, and `programs`
- `failure_class_targeted`: grader hard-failed a good teletherapy answer because it expanded `EAPs` to `Employee Assistance Programs` and treated the generic expansion words as unsupported claims.
- `hypothesis`: If common acronym-expansion scaffolding is excluded from answer-specific concrete terms, useful answers can expand abbreviations naturally without weakening unsupported-claim checks.
- `files_changed`: `orchestration/src/eval_research_golden_scoring_parts/051_answer_alignment.rs`, `orchestration/src/eval_research_golden_scoring_parts/092_tests_relevance_and_citations.rs`
- `proof_tests`: `cargo test --manifest-path orchestration/Cargo.toml answer_alignment_ignores_acronym_expansion_scaffold_terms -- --nocapture`
- `eval_command`: `cargo run --quiet --manifest-path orchestration/Cargo.toml --bin eval_runtime -- research-perfect-evidence --mode=test-input-regrade --strict=0 --limit=1 --responses=/tmp/research_perfect_sample_smoke_responses.json --out=/tmp/research_perfect_sample_smoke_regrade.json --out-latest=/tmp/research_perfect_sample_smoke_regrade_latest.json --out-markdown=/tmp/research_perfect_sample_smoke_regrade.md`
- `before_metrics`: smoke selected `perfect_ev_089_teletherapy_options`; visible answer sounded good but failed on unsupported acronym-expansion scaffold terms.
- `after_metrics`: same saved response regraded `pass=true`, `excellent=true`, `synthesized_sounds_good_rate=1.0`.
- `visible_output_delta`: no answer-generation change; the grader no longer mistakes generic acronym expansion wording for unsupported claim substance.
- `decision`: `kept`
- `reason`: Class-level calibration, not a domain patch.
- `follow_up`: Continue distinguishing scaffold false positives from real synthesis hallucination.

### 2026-06-01: no unsourced detail expansion in evidence-to-answer synthesis

- `baseline_artifact`: `core/local/artifacts/research_perfect_evidence_replay_random10_after_sampling.json`
- `patch_name`: tighten synthesis contract against expanding evidence labels into unsourced subexamples/details
- `failure_class_targeted`: the model produced a useful-looking answer but filled supported categories with plausible unsourced details; `perfect_ev_008_ev_charging_business_models` failed because it expanded evidence-supported EV charging business models into unsupported specifics.
- `hypothesis`: If the synthesis prompt and production workflow contract say that category/model/entity/risk labels must not be expanded into unsourced details, then answers should stay useful while concrete claims remain traceable to the evidence.
- `files_changed`: `orchestration/src/eval_research_perfect_evidence.rs`, `orchestration/src/control_plane/workflows/official/research_synthesize_verify.workflow.json`, `core/layer0/ops/src/dashboard_compat_api_parts/set_config_payload_parts/190_route_blocks/agent_scope_full_parts/workflows/research_synthesize_verify_v1.workflow.json`, `core/layer0/ops/src/dashboard_compat_api_parts/set_config_payload_parts/190_route_blocks/agent_scope_full_parts/047-turn-workflow-synthesis_parts/009-part.rs`, `core/layer0/ops/src/dashboard_compat_api_parts/set_config_payload_parts/190_route_blocks/agent_scope_full_parts/046a-workflow-reader.rs`
- `proof_tests`: `cargo test --manifest-path orchestration/Cargo.toml answer_alignment_ignores_acronym_expansion_scaffold_terms -- --nocapture`; `cargo test --manifest-path core/layer0/ops/Cargo.toml workflow_reader_final_answer_contract_uses_general_research_shapes -- --nocapture`
- `eval_command`: `cargo run --quiet --manifest-path orchestration/Cargo.toml --bin eval_runtime -- research-perfect-evidence --mode=test-input-replay --strict=0 --limit=10 --sample-size=10 --sample-seed=random:9c78c2bcf2e4e09c --timeout-seconds=120 --synthesis-retry-count=1 --out=core/local/artifacts/research_perfect_evidence_replay_random10_after_no_embellish.json --out-latest=artifacts/research_perfect_evidence_replay_random10_after_no_embellish.json --out-markdown=artifacts/research_perfect_evidence_replay_random10_after_no_embellish.md --responses-out=core/local/artifacts/research_perfect_evidence_test_mode_responses_random10_after_no_embellish.json`
- `before_metrics`: same seed random10 `pass_rate=0.9`, `excellent_rate=0.6`, `synthesized_sounds_good_rate=1.0`; non-Excellent cases were moving checklist, personal privacy cleanup, EV charging business models, and AI agent market named.
- `after_metrics`: same seed random10 `pass_rate=1.0`, `excellent_rate=1.0`, `synthesized_sounds_good_rate=1.0`, `synthesis_hard_failures=0`; targeted EV case also `pass=true`, `excellent=true`, `score=100`.
- `generalization_check`: fresh random10 seed `random:2d8026e2e3731860` also returned `pass_rate=1.0`, `excellent_rate=1.0`, `synthesized_sounds_good_rate=1.0`, with selected cases spanning travel, civil rights, AI law, health evidence limits, energy markets, vector databases, battery technology, scientific breakthroughs, migraine prevention, and diabetes medication.
- `visible_output_delta`: EV answer stopped adding the earlier unsupported retail/hospitality/free-charging specifics and gave a bounded, evidence-traceable answer. The same-seed outputs read coherently and directly answer the prompts.
- `decision`: `kept`
- `reason`: The patch targets a generic evidence-boundary failure and improved the same seeded sample without forcing a visible format or case-specific answer.
- `follow_up`: Run another fresh random sample. If it holds, move to the next weakest downstream class; if it regresses, inspect whether the issue is evidence insufficiency, grader calibration, or synthesis over/under-answering.

### 2026-06-01: fresh random20 grader calibration for scaffold terms and user-named options

- `baseline_artifact`: `core/local/artifacts/research_perfect_evidence_replay_random20_fresh_after_no_embellish.json`
- `patch_name`: ignore generic outline/pronoun/action scaffold terms and let covered user-named options satisfy specificity
- `failure_class_targeted`: useful answers were being down-scored by grading mechanics rather than by visible answer quality. Traceability treated scaffold words such as `inclusion`, `rather`, `they`, and `add` as unsupported concrete claims, and the specificity lane undercounted single-word options that the user explicitly named and the answer covered.
- `hypothesis`: If the grader ignores non-substantive scaffold terms and credits covered user-stated named options for option/comparison specificity, then saved useful answers should regrade higher without changing the answer text.
- `files_changed`: `orchestration/src/eval_research_golden_scoring_parts/051_answer_alignment.rs`, `orchestration/src/eval_research_golden_scoring_parts/092_tests_relevance_and_citations.rs`, `orchestration/src/eval_research_golden_scoring_parts/010_case_grade.rs`, `orchestration/src/eval_research_golden_scoring_parts/053_requested_specificity.rs`, `orchestration/src/eval_research_golden_scoring_parts/093_tests_excellent_and_sources.rs`
- `proof_tests`: `cargo test --manifest-path orchestration/Cargo.toml answer_alignment_ignores_outline_and_pronoun_scaffold_terms -- --nocapture`; `cargo test --manifest-path orchestration/Cargo.toml requested_specificity_is_ready_when_user_named_options_are_covered -- --nocapture`
- `eval_command`: `cargo run --quiet --manifest-path orchestration/Cargo.toml --bin eval_runtime -- research-perfect-evidence --mode=test-input-replay --strict=0 --limit=20 --sample-size=20 --timeout-seconds=120 --synthesis-retry-count=1 --out=core/local/artifacts/research_perfect_evidence_replay_random20_fresh_after_no_embellish.json --out-latest=artifacts/research_perfect_evidence_replay_random20_fresh_after_no_embellish.json --out-markdown=artifacts/research_perfect_evidence_replay_random20_fresh_after_no_embellish.md --responses-out=core/local/artifacts/research_perfect_evidence_test_mode_responses_random20_fresh_after_no_embellish.json`
- `before_metrics`: fresh random20 seed `random:08489795654bacce` live replay `pass_rate=0.95`, `excellent_rate=0.85`, `synthesized_sounds_good_rate=1.0`; after scaffold-term regrade only, same saved answers moved to `pass_rate=1.0`, `excellent_rate=0.95`.
- `after_metrics`: same saved random20 responses after named-option specificity calibration `pass_rate=1.0`, `excellent_rate=1.0`, `synthesized_sounds_good_rate=1.0`, `synthesis_hard_failures=0`.
- `handoff_check`: production handoff replay on the same 20 responses returned `handoff_contract_pass_rate=1.0`, `source_refs_present_rate=1.0`, `citation_package_present_rate=1.0`, `evidence_claims_present_rate=1.0`, `visible_source_llm_final_rate=1.0`, and `source_inventory_like_rate=0.0`.
- `visible_output_delta`: no answer-generation change. The cybersecurity, vector database, and cordless vacuum answers already read as coherent/useful; the grader now agrees instead of treating scaffolding or user-named single-word brands as missing evidence.
- `decision`: `kept`
- `reason`: Both patches are generic grading calibration fixes. They do not force a response format, do not encode domain facts, and improved saved outputs that already passed the soft human-usefulness proxy.
- `follow_up`: Run a larger live downstream sample only if we need more confidence. Otherwise, shift back to the upstream evidence quality/tooling lane because downstream perfect-evidence behavior is now stable on several random samples.

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
- `eval_command`: `cargo run --manifest-path orchestration/Cargo.toml --bin eval_runtime -- research-golden --live=1 --base-url=http://127.0.0.1:5173 --limit=2 --sample-seed=random:6d09e741b184df5d5052ac05fd27b875bc9829f3c62ca6da69e37eb955207a47 --timeout-seconds=90 --out=core/local/artifacts/research_golden_after_headline_shell_calibration_live2.json --out-latest=artifacts/research_golden_after_headline_shell_calibration_live2.json --out-markdown=artifacts/research_golden_after_headline_shell_calibration_live2.md --failures-out=local/state/ops/research_golden/headline_shell_calibration_live2_failures.jsonl`
- `before_metrics`: title-inventory guard run still showed `claim_quality_ready=1/2` even though the AI drug discovery sample row's only visible claim was a question-style headline/dateline shell.
- `after_metrics`: deterministic fixture proves the same generic row shape now fails source quality, claim quality, and evidence-packet readiness instead of masquerading as answerable evidence. Live two-case run: `average_score=96.0`, `passed_cases=1/2`, `excellent_cases=0/2`, workflow gates `4/4`, `web_5d_source_quality_ready=0/2`, `web_5e_claim_quality_ready=0/2`, `web_5f_citation_renderability_ready=1/2`, `web_5g_answerability_ready=0/2`, `web_5h_evidence_packet_contract_ready=0/2`, `web_7_usable_evidence_available=0/2`. The top first failure is now cleanly `web_5d_source_quality_ready` in both measured cases.
- `visible_output_delta`: no direct user-facing text change intended.
- `decision`: `kept`
- `reason`: This is a measurement-alignment patch. It prevents metrics from rewarding the exact fragmentary source-row behavior the user flagged as bad, without imposing a final-answer format or topic-specific rule.
- `follow_up`: Next patch should target the candidate-to-evidence-card boundary: provider rows are plentiful, but selected rows lack real extracted claims and relevant extracts. Treat this as retrieval/evidence-packaging quality, not workflow routing.

### 2026-05-30: evidence-packet substance boundary

- `baseline_artifact`: `artifacts/research_golden_after_headline_shell_calibration_live2.json`
- `patch_name`: require source identity, source type, relevant extract substance, and concrete claim material before a candidate row can become usable evidence.
- `failure_class_targeted`: `candidate_row_promoted_to_evidence_without_extraction` / provider titles, datelines, question headlines, or explicit `counts_as_usable_evidence` flags being accepted as answerable evidence.
- `hypothesis`: If source/provider rows are forced to prove evidence-packet substance before promotion, the tooling gates will stop hiding retrieval weakness behind inflated evidence counts, and the next upstream bottleneck will be visible.
- `files_changed`: `core/layer2/ops/src/retrieval_policy_parts/010-freshness-and-relevance.rs`, `core/layer2/ops/src/retrieval_policy_parts/911-relevance-tests.rs`
- `proof_tests`: `cargo test --manifest-path core/layer2/ops/Cargo.toml` passed `53/53`; `cargo test --manifest-path orchestration/Cargo.toml --bin eval_runtime evidence_quality_gates -- --nocapture` passed `3/3`.
- `eval_command`: `cargo run --manifest-path orchestration/Cargo.toml --bin eval_runtime -- web-tooling-golden --live=1 --base-url=http://127.0.0.1:5173 --limit=2 --timeout-seconds=90 --out=core/local/artifacts/web_tooling_after_evidence_packet_substance_gate.json --out-latest=artifacts/web_tooling_after_evidence_packet_substance_gate.json --out-markdown=artifacts/web_tooling_after_evidence_packet_substance_gate.md`
- `after_metrics`: `measured_cases=2`, `passed_cases=0/2`; upstream request/transport/access/raw-row gates passed `2/2`; first failed gate was `web_4c_search_provider_surface_ready=0/2`, followed by evidence-pack, claim-quality, answerability, and usable-evidence gates at `0/2`.
- `visible_output_delta`: no direct user-facing text change intended; the web-tooling lane now refuses to call off-topic provider rows evidence.
- `decision`: `kept`
- `reason`: This is a promotion-integrity patch. It did not improve retrieval, but it made the telemetry more honest: the current live blocker is provider/query candidate supply, not evidence counts pretending that weak rows are usable.
- `follow_up`: The next one-change patch should target `web_4c_search_provider_surface_ready`: inspect the generated query lanes and provider-surface diagnostics, then fix the generic request-to-search-query boundary if instruction scaffolding or action verbs are contaminating search terms.

### 2026-05-30: explicit query-lane scaffold stripping experiment

- `baseline_artifact`: `artifacts/web_tooling_after_evidence_packet_substance_gate.json`
- `patch_name`: canonicalize explicit query-pack lanes and strip leading tool-instruction scaffold such as `use web research to`.
- `failure_class_targeted`: `web_4c_search_provider_surface_ready` / generated query lanes potentially contaminated by action scaffolding instead of search terms.
- `hypothesis`: If explicit query-pack lanes are normalized through the same generic search-query canonicalizer, provider-surface readiness should improve without hardcoding any topic or answer shape.
- `files_changed`: `core/layer0/ops/src/batch_query_primitive_parts/018-request-and-cache.rs`, `core/layer0/ops/src/batch_query_primitive_parts/010-core.combined_parts/030-looks-like-instructional-query-to-looks-like-empty-duckduckgo-instan.rs`, `core/layer0/ops/src/batch_query_primitive_parts/042-cache-rewrite-tests.rs`
- `proof_tests`: The new scaffold-stripping fixture and neighboring framework-catalog query-plan fixtures passed while the experiment was applied.
- `eval_command`: `cargo run --manifest-path orchestration/Cargo.toml --bin eval_runtime -- web-tooling-golden --live=1 --base-url=http://127.0.0.1:5173 --limit=2 --timeout-seconds=90 --out=core/local/artifacts/web_tooling_after_query_lane_scaffold_strip.json --out-latest=artifacts/web_tooling_after_query_lane_scaffold_strip.json --out-markdown=artifacts/web_tooling_after_query_lane_scaffold_strip.md`
- `before_metrics`: evidence-packet substance run had `measured_cases=2`, `passed_cases=0/2`, upstream request/transport/access/raw-row gates passed `2/2`, and `web_4c_search_provider_surface_ready=0/2`.
- `after_metrics`: `measured_cases=2`, `passed_cases=0/2`, `transport_failures=1`, `web_3a_tool_transport_completed=1/2`, `web_4c_search_provider_surface_ready=0/2`, `web_5_packaged_evidence_present=0/2`, `web_7_usable_evidence_available=0/2`.
- `visible_output_delta`: no user-facing improvement measured; one case timed out before a tool payload and the completed case still returned `status=no_results` with zero provider candidates.
- `decision`: `reverted`
- `reason`: The unit-level query-lane cleanup worked structurally, but the live metric did not move and transport became worse in the two-case sample. Under the one-measurable-change rule, this is not worth keeping.
- `follow_up`: Treat query scaffold contamination as a lower-confidence hypothesis. The next patch should inspect provider execution and provider-surface readiness directly: identify whether the first live loss is provider timeout/budget handling, provider admission/circuit state, or empty provider rows after filtering.

### 2026-05-30: primary provider-chain budget experiment

- `baseline_artifact`: `artifacts/web_tooling_after_evidence_packet_substance_gate.json`
- `patch_name`: cap the primary batch-query search provider chain to one provider per lane.
- `failure_class_targeted`: `web_4c_search_provider_surface_ready` / strong providers being exhausted or locally denied after each query lane walked the full provider chain.
- `hypothesis`: If each lane spends at most one primary provider attempt, Tavily/Exa should be less likely to trip local rate-limit policy and provider-surface readiness should improve.
- `files_changed`: `core/layer0/ops/config/batch_query_policy.json`, `core/layer0/ops/src/batch_query_primitive_parts/020-pipeline.combined_parts/010-link-fetch-fallback-limit-to-stage-error.rs`, `core/layer0/ops/src/batch_query_primitive_parts/042-cache-rewrite-tests.rs`
- `proof_tests`: `cargo test --manifest-path core/layer0/ops/Cargo.toml batch_primary_search_request_caps_provider_chain_for_rate_budget -- --nocapture` passed while the experiment was applied.
- `eval_command`: `INFRING_BATCH_QUERY_CACHE_MODE=disabled cargo run --manifest-path orchestration/Cargo.toml --bin eval_runtime -- web-tooling-golden --live=1 --base-url=http://127.0.0.1:5173 --limit=2 --timeout-seconds=90 --out=core/local/artifacts/web_tooling_after_primary_provider_budget.json --out-latest=artifacts/web_tooling_after_primary_provider_budget.json --out-markdown=artifacts/web_tooling_after_primary_provider_budget.md`
- `before_metrics`: evidence-packet substance run had `measured_cases=2`, `passed_cases=0/2`, upstream request/transport/access/raw-row gates passed `2/2`, and `web_4c_search_provider_surface_ready=0/2`.
- `after_metrics`: `measured_cases=2`, `passed_cases=0/2`, `transport_failures=1`, one case first failed at `web_3a_tool_transport_completed`, and the completed case still first failed at `web_4c_search_provider_surface_ready` with `provider_degraded`, `provider_error`, `strong_search_provider_missing`, and `web_conduit_policy_denied`.
- `visible_output_delta`: no user-facing improvement measured.
- `decision`: `reverted`
- `reason`: The provider-chain cap did not reduce the observed local policy denial in the live run and made transport worse in the two-case sample. It also did not address the second-pass recovery path, where receipts still showed Tavily/Exa denied by `rate_limit_exceeded`.
- `follow_up`: The next patch should target provider scheduling rather than provider list length: inspect the local web-conduit rate-limit ledger/circuit state and either defer strong-provider recovery until budget is available or expose a clean `provider_budget_exhausted` failure instead of spending repeated denied attempts.

### 2026-05-30: provider quota/circuit diagnostic boundary

- `baseline_artifact`: `artifacts/web_tooling_after_provider_http_error_boundary.json`
- `patch_name`: preserve structured provider HTTP quota/billing errors and carry circuit last-error context into web-tooling diagnostics.
- `failure_class_targeted`: `provider_account_or_quota_failure_mislabeled_as_no_relevant_results`.
- `hypothesis`: If Tavily/Exa HTTP account/quota failures survive normalization and circuit-open payloads carry the underlying last error, the first failed tooling gate should move upstream to provider quota/access instead of downstream no-results or weak evidence.
- `files_changed`: `core/layer0/ops/src/web_conduit_parts/030-serper-bing-and-fetch.rs`, `core/layer0/ops/src/web_conduit_parts/050-search-providers_parts/001-segment.rs`, `core/layer0/ops/src/web_conduit_parts/060-search-orchestration_parts/001-segment_parts/670-api-search-parts/000-combined.rs`, `core/layer0/ops/src/web_conduit_parts/080-tests_parts/010-mod-tests_parts/010-status-and-provider-catalog-tests.rs`, `core/layer0/ops/src/web_conduit_provider_runtime_parts/010-provider-chain-and-health.rs`, `core/layer0/ops/src/web_conduit_provider_runtime_parts/017-provider-public-contracts.rs`, `core/layer0/ops/src/web_conduit_provider_runtime_parts/020-cache-and-tests.rs`, `orchestration/src/eval_web_retrieval_gate_diagnostics_parts/080_access_blockers.rs`, `orchestration/src/eval_web_retrieval_gate_diagnostics_parts/090_tests_access_and_provider.rs`
- `proof_tests`: `cargo test --manifest-path core/layer0/ops/Cargo.toml structured_search_http_errors_preserve_provider_account_boundary -- --nocapture` passed; `cargo test --manifest-path core/layer0/ops/Cargo.toml provider_failure_class_treats_quota_and_billing_as_throttle_boundary -- --nocapture` passed; `cargo test --manifest-path orchestration/Cargo.toml --bin eval_runtime access_blocker_detects_provider_quota_snake_case_errors -- --nocapture` passed.
- `eval_command`: `INFRING_BATCH_QUERY_CACHE_MODE=disabled cargo run --manifest-path orchestration/Cargo.toml --bin eval_runtime -- web-tooling-golden --live=1 --base-url=http://127.0.0.1:5173 --limit=2 --timeout-seconds=90 --out=core/local/artifacts/web_tooling_after_circuit_quota_visibility.json --out-latest=artifacts/web_tooling_after_circuit_quota_visibility.json --out-markdown=artifacts/web_tooling_after_circuit_quota_visibility.md`
- `before_metrics`: prior live artifact showed Tavily HTTP `432` and Exa HTTP `402` normalized as `no_relevant_results`; after the circuit opened, `web_3b1_provider_quota_not_rate_limited` still passed because the payload only exposed `provider_circuit_open`.
- `after_metrics`: `measured_cases=2`, `passed_cases=0/2`, `transport_failures=1`, `web_3a_tool_transport_completed=1/2`, `web_3b1_provider_quota_not_rate_limited=1/2`, `web_3b_access_not_blocked_or_throttled=1/2`, `web_4b_search_provider_circuit_closed=1/2`, `web_4c_search_provider_surface_ready=0/2`, `web_7_usable_evidence_available=0/2`. The completed case now first fails at `web_3b1_provider_quota_not_rate_limited` with `quota_exceeded` and `provider_circuit_open:exa_provider_quota_exceeded_or_billing_required_http_402`.
- `visible_output_delta`: no user-facing improvement intended.
- `decision`: `kept`
- `reason`: This is a diagnostic-fidelity patch. It proves the current strong-provider lane is blocked by provider account/quota/circuit state in at least one measured case, not by generic relevance or synthesis.
- `follow_up`: Next one-change patch should target the remaining transport failure at `web_3a_tool_transport_completed` or the provider readiness path that can leave `web_4c_search_provider_surface_ready=0/2`; do not tune synthesis until a live run shows usable evidence entering the workflow.

### 2026-05-30: parallel second-pass recovery lanes

- `baseline_artifact`: `artifacts/web_tooling_after_circuit_quota_visibility.json`
- `patch_name`: run second-pass recovery query lanes through the same bounded parallel window as the initial retrieval wave.
- `failure_class_targeted`: `tool_transport_timeout_before_payload` caused by sequential recovery lanes each spending the full query timeout.
- `hypothesis`: If recovery lanes are bounded in parallel instead of serialized, the tool should return a structured partial/no-results payload before the harness transport timeout, allowing downstream gates to expose the real provider/evidence boundary.
- `files_changed`: `core/layer0/ops/src/batch_query_primitive_parts/020-pipeline.combined_parts/040-api-batch-query_parts/000-combined.rs`
- `proof_tests`: `cargo test --manifest-path core/layer0/ops/Cargo.toml query_timeout_policy_defaults_and_clamps -- --nocapture` passed after the patch compiled the affected crate.
- `eval_command`: `INFRING_BATCH_QUERY_CACHE_MODE=disabled cargo run --manifest-path orchestration/Cargo.toml --bin eval_runtime -- web-tooling-golden --live=1 --base-url=http://127.0.0.1:5173 --limit=2 --timeout-seconds=90 --out=core/local/artifacts/web_tooling_after_parallel_recovery_lanes.json --out-latest=artifacts/web_tooling_after_parallel_recovery_lanes.json --out-markdown=artifacts/web_tooling_after_parallel_recovery_lanes.md`
- `before_metrics`: `measured_cases=2`, `passed_cases=0/2`, `transport_failures=1`, `web_3a_tool_transport_completed=1/2`, `web_4_raw_candidates_present=1/2`, `web_4c_search_provider_surface_ready=0/2`, `web_5b_content_rich_candidates_present=1/2`, `web_7_usable_evidence_available=0/2`.
- `after_metrics`: `measured_cases=2`, `passed_cases=0/2`, `transport_failures=0`, `web_3a_tool_transport_completed=2/2`, `web_4_raw_candidates_present=2/2`, `web_4c_search_provider_surface_ready=1/2`, `web_5b_content_rich_candidates_present=2/2`, `web_5d_source_quality_ready=0/2`, `web_5g_answerability_ready=0/2`, `web_7_usable_evidence_available=0/2`.
- `visible_output_delta`: no direct user-facing text measured; this patch makes the tooling return a structured payload instead of timing out.
- `decision`: `kept`
- `reason`: This is a runtime-bounding patch with a clear positive gate movement. It did not solve evidence quality, but it removed an upstream transport blocker and made the next failure concrete.
- `follow_up`: Work top-down from the new state: provider quota/circuit remains visible in one case, while the non-transport comparison case now reaches `web_5d_source_quality_ready` and `web_5g_answerability_ready`, meaning the next primitive work should target source diversity/evidence packet sufficiency rather than transport.

### 2026-05-31: query-lane ordering and temporal-scope experiment

- `baseline_artifact`: `/tmp/web_tooling_semiconductor_after_request_repair.json`
- `patch_name`: prioritize current/source-class follow-up lanes, remove duplicate primary prompt from `queries`, and preserve relative time scope in derived lanes.
- `failure_class_targeted`: `web_5d_source_quality_ready` / source diversity stayed below the evidence-pack threshold after instruction-scaffold pollution was repaired.
- `hypothesis`: If the bounded execution budget spends fewer lanes on duplicate/generic searches and more lanes on current/source-class searches, the same provider budget should surface at least two pack-ready source domains.
- `files_changed`: `orchestration/src/eval_web_tooling_golden/request_packs.rs`, `orchestration/src/eval_web_tooling_golden/tests.rs`
- `proof_tests`: `cargo test --manifest-path orchestration/Cargo.toml --bin eval_runtime report_request_pack_repairs_instruction_scaffold_pollution -- --nocapture` passed while the experiment was applied; `cargo test --manifest-path orchestration/Cargo.toml --bin eval_runtime derived_request_pack -- --nocapture` passed while the experiment was applied.
- `eval_command`: `cargo run --quiet --manifest-path orchestration/Cargo.toml --bin eval_runtime -- web-tooling-golden --live=1 --cases=/tmp/research_semiconductor_single_case.json --request-packs-from=/tmp/research_live_random5_smoke_a.json --limit=1 --strict=0 --timeout-seconds=90 --out=/tmp/web_tooling_semiconductor_after_temporal_scope_queries.json --out-latest=/tmp/web_tooling_semiconductor_after_temporal_scope_queries_latest.json --out-markdown=/tmp/web_tooling_semiconductor_after_temporal_scope_queries.md`
- `before_metrics`: baseline after request repair had `web_5d_source_quality_ready=0/1`, `web_5g_answerability_ready=0/1`, `web_7_usable_evidence_available=0/1`, `evidence_pack_quality.usable_count=1`, and `source_domain_count=1`.
- `after_metrics`: the lane-order, follow-up-only, and temporal-scope variants all remained at `web_5d_source_quality_ready=0/1`, `web_5g_answerability_ready=0/1`, `web_7_usable_evidence_available=0/1`, `evidence_pack_quality.usable_count=1`, and `source_domain_count=1`.
- `visible_output_delta`: no user-facing output improvement measured; the web-tooling artifact still summarized one Substack source as the only usable evidence.
- `decision`: `reverted`
- `reason`: The query plan became cleaner, but the targeted gate did not move. This also stacked several query-lane changes before proving the first one, so the experiment is logged as a rejected direction under the one-measurable-change rule.
- `follow_up`: Stop tuning query-lane order for this failure. The next primitive target should be candidate-to-evidence promotion/materialization: the provider returns multiple topical domains, but most rows are rejected as `freshness_unproven`, listing/index shells, or insufficient extracted answer material before they can become pack-ready evidence.

### 2026-05-31: freshness materialization experiment

- `baseline_artifact`: `/tmp/web_tooling_semiconductor_after_request_repair.json`
- `patch_name`: force page extraction for current-intent candidates with negative recency adjustment and sufficient query overlap.
- `failure_class_targeted`: Topical provider rows were present, but many never became pack-ready evidence because they stayed `freshness_unproven`.
- `hypothesis`: If current/topical rows are materialized before promotion, the fetch stage can discover publication/freshness signals and produce more than one usable source domain.
- `files_changed`: `core/layer0/ops/src/batch_query_primitive_parts/015-intent-and-quality.rs`, `core/layer0/ops/src/batch_query_primitive_parts/042-cache-rewrite-tests.rs`
- `proof_tests`: `cargo test --manifest-path core/layer0/ops/Cargo.toml current_unverified_candidate_gets_page_extraction_for_freshness_check -- --nocapture` passed while the experiment was applied.
- `eval_command`: `cargo run --quiet --manifest-path orchestration/Cargo.toml --bin eval_runtime -- web-tooling-golden --live=1 --cases=/tmp/research_semiconductor_single_case.json --request-packs-from=/tmp/research_live_random5_smoke_a.json --limit=1 --strict=0 --timeout-seconds=90 --out=/tmp/web_tooling_semiconductor_after_freshness_materialization.json --out-latest=/tmp/web_tooling_semiconductor_after_freshness_materialization_latest.json --out-markdown=/tmp/web_tooling_semiconductor_after_freshness_materialization.md`
- `before_metrics`: baseline after request repair had `web_5d_source_quality_ready=0/1`, `web_5g_answerability_ready=0/1`, `web_7_usable_evidence_available=0/1`, `evidence_pack_quality.usable_count=1`, and `source_domain_count=1`.
- `after_metrics`: the live run stayed at `web_5d_source_quality_ready=0/1`, `web_5g_answerability_ready=0/1`, and `web_7_usable_evidence_available=0/1`; the pack still had `usable_count=1` and `source_domain_count=1`.
- `visible_output_delta`: no user-facing quality improvement measured. Extra materialization selected a stale/irrelevant HBS PDF and a shell-like SIA page before creating any additional pack-ready source.
- `decision`: `reverted`
- `reason`: The patch changed fetch behavior but did not improve the targeted evidence gate. In this form, "fetch freshness-unproven current rows" is too blunt and can spend the bounded materialization budget on stale or non-answer-bearing documents.
- `follow_up`: Target candidate selection before materialization rather than simply widening materialization. The next patch should prioritize answer-bearing topical rows for fetch/promotion and demote stale PDFs, broad reports, and shell pages earlier in the materialization queue.

### 2026-05-31: stale dated link preflight experiment

- `baseline_artifact`: `/tmp/web_tooling_semiconductor_after_request_repair.json`
- `patch_name`: reject explicitly stale dated page-extraction links for current-window queries before fetch.
- `failure_class_targeted`: The bounded materialization budget was being spent on stale documents, especially an HBS 2021 PDF, while the evidence pack stayed at one usable source.
- `hypothesis`: If stale URLs and stale local context are rejected before page extraction, the materialization budget should shift toward current answer-bearing candidates and improve source diversity.
- `files_changed`: `core/layer0/ops/src/batch_query_primitive_parts/020-pipeline.combined_parts/010-link-fetch-fallback-limit-to-stage-error.rs`, `core/layer0/ops/src/batch_query_primitive_parts/042-cache-rewrite-tests.rs`
- `proof_tests`: `cargo test --manifest-path core/layer0/ops/Cargo.toml current_page_extraction_rejects_explicitly_stale_dated_links -- --nocapture` passed while the experiment was applied.
- `eval_command`: `cargo run --quiet --manifest-path orchestration/Cargo.toml --bin eval_runtime -- web-tooling-golden --live=1 --cases=/tmp/research_semiconductor_single_case.json --request-packs-from=/tmp/research_live_random5_smoke_a.json --limit=1 --strict=0 --timeout-seconds=90 --out=/tmp/web_tooling_semiconductor_after_stale_link_url_reject.json --out-latest=/tmp/web_tooling_semiconductor_after_stale_link_url_reject_latest.json --out-markdown=/tmp/web_tooling_semiconductor_after_stale_link_url_reject.md`
- `before_metrics`: baseline after request repair had `web_5d_source_quality_ready=0/1`, `web_5g_answerability_ready=0/1`, `web_7_usable_evidence_available=0/1`, `evidence_pack_quality.usable_count=1`, and `source_domain_count=1`.
- `after_metrics`: the live run stayed at `web_5d_source_quality_ready=0/1`, `web_5g_answerability_ready=0/1`, and `web_7_usable_evidence_available=0/1`; the pack still had `usable_count=1` and `source_domain_count=1`.
- `visible_output_delta`: no user-facing quality improvement measured. The same HBS PDF still appeared as a full materialized non-pack-ready row, which indicates it is not controlled by the patched preflight path, or it is entering from a later artifact/promotion path.
- `decision`: `reverted`
- `reason`: The proof test was valid but the live gate did not move. Keeping this would add complexity without demonstrated value.
- `follow_up`: Inspect the artifact-to-ranked-candidate path directly. The next measurable patch should target why `document_page_artifact_page_enriched` rows with `freshness_unproven` and zero claim hints can be selected ahead of better provider rows, rather than trying to block them only at URL preflight.

### 2026-05-31: Tavily time-window payload experiment

- `baseline_artifact`: `/tmp/web_tooling_semiconductor_after_fresh_dashboard_diag.json`
- `patch_name`: add Tavily `time_range` plus `topic=news` / `days` when the query explicitly contains a relative time window.
- `failure_class_targeted`: For a current-window request, provider rows were topical but not fresh enough to become pack-ready evidence.
- `hypothesis`: If the provider receives the user's explicit time window instead of only the natural-language query, candidate supply should shift toward fresh/current rows and improve evidence packaging.
- `files_changed`: `core/layer0/ops/src/web_conduit_parts/050-search-providers_parts/001-segment.rs`, `core/layer0/ops/src/web_conduit_parts/080-tests_parts/010-mod-tests_parts/030-search-query-shape-and-filter-tests.rs`
- `proof_tests`: `cargo test --manifest-path core/layer0/ops/Cargo.toml tavily_payload_ -- --nocapture` passed while the experiment was applied.
- `eval_command`: `cargo run --quiet --manifest-path orchestration/Cargo.toml --bin eval_runtime -- web-tooling-golden --live=1 --cases=/tmp/research_semiconductor_single_case.json --request-packs-from=/tmp/research_live_random5_smoke_a.json --limit=1 --strict=0 --timeout-seconds=90 --out=/tmp/web_tooling_semiconductor_after_tavily_time_window.json --out-latest=/tmp/web_tooling_semiconductor_after_tavily_time_window_latest.json --out-markdown=/tmp/web_tooling_semiconductor_after_tavily_time_window.md`
- `before_metrics`: after the selected-row diagnostics patch, the narrow case reached `web_5d_source_quality_ready=0/1`, `web_5g_answerability_ready=0/1`, and `web_7_usable_evidence_available=0/1` with `evidence_pack_quality.usable_count=1` and `source_domain_count=1`.
- `after_metrics`: the live run regressed to `web_5_packaged_evidence_present=0/1`, `web_5c_claim_extraction_present=0/1`, `web_5d_source_quality_ready=0/1`, and `web_7_usable_evidence_available=0/1`; `evidence_pack_quality.status=absent`, `usable_count=0`, and `source_domain_count=0`.
- `visible_output_delta`: negative. The tool still found raw provider rows, but the pack became empty and the first meaningful failure moved upstream to missing packaged evidence.
- `decision`: `reverted`
- `reason`: Passing provider-payload unit tests were not enough; the live artifact showed worse evidence packaging. The current version of this provider-specific shaping is too blunt for the measured failure.
- `follow_up`: Do not keep provider-specific time-window shaping until we can prove it improves candidate quality across a small random set. The next primitive target should stay at the artifact/ranked-candidate boundary: explain and then reduce promotion of selected non-pack-ready rows before touching provider payloads again.

### 2026-05-31: local-date freshness boundary

- `baseline_artifact`: `/tmp/web_tooling_semiconductor_after_time_window_revert.json`
- `patch_name`: evaluate relative freshness windows from local machine date instead of UTC `now_iso`.
- `failure_class_targeted`: `this month` requests near a UTC date boundary treated current local-month sources as stale, blocking evidence packaging.
- `hypothesis`: If freshness policy uses local date for user-facing relative windows, May 2026 sources should not become stale merely because UTC has crossed into June while the user's machine is still on May 31.
- `files_changed`: `core/layer2/ops/src/retrieval_policy_parts/010-freshness-and-relevance.rs`
- `proof_tests`: `cargo test --manifest-path core/layer2/ops/Cargo.toml relative_current_freshness_accepts_local_month_url_slug -- --nocapture` passed; `cargo test --manifest-path core/layer2/ops/Cargo.toml selection_diagnostics_explain_selected_non_pack_ready_rows -- --nocapture` passed.
- `eval_command`: `cargo run --quiet --manifest-path orchestration/Cargo.toml --bin eval_runtime -- web-tooling-golden --live=1 --cases=/tmp/research_semiconductor_single_case.json --request-packs-from=/tmp/research_live_random5_smoke_a.json --limit=1 --strict=0 --timeout-seconds=90 --out=/tmp/web_tooling_semiconductor_after_local_freshness.json --out-latest=/tmp/web_tooling_semiconductor_after_local_freshness_latest.json --out-markdown=/tmp/web_tooling_semiconductor_after_local_freshness.md`
- `before_metrics`: `web_5_packaged_evidence_present=0/1`, `web_5c_claim_extraction_present=0/1`, `web_5d_source_quality_ready=0/1`, `web_6_provider_not_empty_or_degraded=0/1`, and `evidence_pack_quality.status=absent`.
- `after_metrics`: `web_5_packaged_evidence_present=1/1`, `web_5c_claim_extraction_present=1/1`, `web_5e_claim_quality_ready=1/1`, `web_5f_citation_renderability_ready=1/1`, `web_5h_evidence_packet_contract_ready=1/1`, `web_6_provider_not_empty_or_degraded=1/1`, `web_8_evidence_context_to_synthesis=1/1`; remaining failures are `web_5d_source_quality_ready=0/1`, `web_5g_answerability_ready=0/1`, and `web_7_usable_evidence_available=0/1` because the pack still has one usable source domain.
- `visible_output_delta`: positive diagnostic and packaging movement. The tool returned a partial evidence pack instead of an absent pack, with citable claim hints and citation metadata preserved.
- `decision`: `kept`
- `reason`: This is a primitive time-boundary fix and it moved the most-upstream failure from missing packaged evidence back down to source-quality insufficiency.
- `follow_up`: Work the next gate, `web_5d_source_quality_ready`, by improving source diversity/source quality for already-returned candidates. Do not tune final synthesis while the evidence pack still only has one usable source domain.

### 2026-05-31: provider payload diversity reserve experiment

- `baseline_artifact`: `/tmp/web_tooling_semiconductor_after_local_freshness.json`
- `patch_name`: reserve two page-extraction slots for provider payload links when a stage has a three-link fetch budget.
- `failure_class_targeted`: `web_5d_source_quality_ready` stayed failed because only one source domain became usable evidence while stale or non-pack-ready rows could still consume selected/materialization attention.
- `hypothesis`: If provider-returned article links get enough fetch budget to prove themselves, the evidence pack should gain a second usable source domain and move `web_5d_source_quality_ready`.
- `files_changed`: `core/layer0/ops/src/batch_query_primitive_parts/020-pipeline.combined_parts/010-link-fetch-fallback-limit-to-stage-error.rs`, `core/layer0/ops/src/batch_query_primitive_parts/040-tests.rs`, `core/layer0/ops/src/batch_query_primitive_parts/040-tests.rs.parts/020-page-extraction-selection.inc`
- `proof_tests`: `cargo test --manifest-path core/layer0/ops/Cargo.toml page_extraction_reserves_payload_article_diversity_when_candidates_score_higher -- --nocapture` passed while the experiment was applied.
- `eval_command`: `cargo run --quiet --manifest-path orchestration/Cargo.toml --bin eval_runtime -- web-tooling-golden --live=1 --cases=/tmp/research_semiconductor_single_case.json --request-packs-from=/tmp/research_live_random5_smoke_a.json --limit=1 --strict=0 --timeout-seconds=90 --out=/tmp/web_tooling_semiconductor_after_payload_diversity.json --out-latest=/tmp/web_tooling_semiconductor_after_payload_diversity_latest.json --out-markdown=/tmp/web_tooling_semiconductor_after_payload_diversity.md`
- `before_metrics`: after local-date freshness, `web_5_packaged_evidence_present=1/1`, `web_5c_claim_extraction_present=1/1`, `web_5d_source_quality_ready=0/1`, `web_5g_answerability_ready=0/1`, `web_7_usable_evidence_available=0/1`, `evidence_pack_quality.usable_count=1`, and `source_domain_count=1`.
- `after_metrics`: the live run still had `web_5d_source_quality_ready=0/1`, `web_5g_answerability_ready=0/1`, and `web_7_usable_evidence_available=0/1`; the pack still had `usable_count=1`, `source_domain_count=1`, and `status=thin`.
- `visible_output_delta`: no positive user-facing or evidence-quality movement measured. The selected rows shifted, but only one `semi.org` row became usable evidence.
- `decision`: `reverted`
- `reason`: The focused selector test was valid, but the live gate did not move. The bottleneck is not just payload-link fetch-slot crowd-out; it is that returned/promoted rows still fail freshness, materialization, or answer-bearing claim requirements before becoming pack-ready evidence.
- `follow_up`: Target the candidate-to-evidence promotion boundary next. Specifically inspect why provider rows with topical snippets become `trusted_structured_feed` but remain `freshness_unproven`, `link_directory_or_aggregator_shell`, or `not_materialized_or_non_evidence_payload`, and patch the most upstream generic cause there.

### 2026-06-01: relative current-window query lane

- `baseline_artifact`: `/tmp/web_tooling_global_news_after_month.json` before canonical policy correction; live query plan lacked an absolute month lane even for `give me global news from this week`.
- `patch_name`: add visible local month/year query lanes for relative current-window requests and wire `{current_month_name}` into the canonical batch-query policy.
- `failure_class_targeted`: Relative time-window requests were sent as vague natural-language lanes, leaving providers to infer the actual date window.
- `hypothesis`: If a query says `today`, `this week`, or `this month`, the query plan should include the local current month/year as plain query text without assuming a topic-specific answer shape.
- `files_changed`: `core/layer0/ops/src/batch_query_primitive_parts/018-request-and-cache.rs`, `core/layer0/ops/src/batch_query_primitive_parts/010-core.combined_parts/020-policy-rel-to-instruction-tail-regex.rs`, `core/layer0/ops/src/batch_query_primitive_parts/042-cache-rewrite-tests.rs`, `core/layer0/ops/config/batch_query_policy.json`, `client/runtime/config/batch_query_policy.json`
- `proof_tests`: `cargo test --manifest-path core/layer0/ops/Cargo.toml --lib relative_current_query_plan_adds_visible_local_month_lane -- --nocapture` passed.
- `live_probe_command`: `INFRING_BATCH_QUERY_CACHE_MODE=disabled cargo run --quiet --manifest-path core/layer0/ops/Cargo.toml --bin infring-ops -- batch-query query --source=web --aperture=small --query='give me global news from this week'`
- `after_metrics`: query plan now includes `global news from this week june 2026` as an executed initial lane.
- `visible_output_delta`: query planning became more faithful and diagnosable, but evidence quality did not move; the live pack stayed `thin` with `usable_count=1` and `source_domain_count=1`.
- `decision`: `kept`
- `reason`: This is a primitive date-window correctness fix. It does not solve evidence quality by itself, but it removes an upstream ambiguity without hardcoding any subject matter or final-answer format.
- `follow_up`: Continue at the evidence boundary. The next measurable target remains converting more than one returned/current source into pack-ready evidence.

### 2026-06-01: candidate-embedded article link extraction experiment

- `baseline_artifact`: `/tmp/web_tooling_global_news_after_month.json`
- `patch_name`: let page extraction consider article links embedded inside provider candidate snippets/listing text.
- `failure_class_targeted`: Provider rows for broad news queries contained section/listing pages with concrete article links, but only the listing locator or top-level payload links were considered for fetch.
- `hypothesis`: If embedded article links are extracted under the existing fetch budget, evidence should gain concrete article candidates from more than one source domain.
- `files_changed_while_applied`: `core/layer0/ops/src/batch_query_primitive_parts/020-pipeline.combined_parts/010-link-fetch-fallback-limit-to-stage-error.rs`, `core/layer0/ops/src/batch_query_primitive_parts/042-cache-rewrite-tests.rs`
- `proof_tests`: `cargo test --manifest-path core/layer0/ops/Cargo.toml --lib page_extraction_uses_article_links_embedded_in_listing_candidate_snippets -- --nocapture` passed while the experiment was applied.
- `eval_artifact`: `/tmp/web_tooling_global_news_after_embedded_links.json`
- `before_metrics`: `evidence_pack_quality.status=thin`, `usable_count=1`, `source_domain_count=1`, `evidence_refs_count=1`.
- `after_metrics`: live run stayed `evidence_pack_quality.status=thin`, `usable_count=1`, `source_domain_count=1`, `evidence_refs_count=1`; status regressed from `ok` to `partial`.
- `visible_output_delta`: negative/no improvement. Extra fetch work produced blocker-shell materialization and many `candidate_embedded_broad_query_non_article_link` rejections without adding usable evidence.
- `decision`: `reverted`
- `reason`: The local selector behavior was plausible, but the live gate did not move and the run became noisier. Keeping it would add complexity without evidence-quality improvement.
- `follow_up`: Do not widen fetch fanout blindly. The next patch should first improve diagnostics around why current, article-like provider candidates fail `pack_ready` after retrieval, then target the most common generic blocker.

### 2026-06-01: broad current listing evidence boundary

- `baseline_artifact`: `/tmp/web_tooling_global_news_after_month.json`
- `patch_name`: allow broad current-window structured listing rows to become evidence when they contain a current signal and a concrete claim hint; also recognize compact relative ages like `1h ago`.
- `failure_class_targeted`: The tool had enough current provider rows to answer partially, but source-quality gates failed because section/listing pages with concrete current claims were treated as non-evidence payloads.
- `hypothesis`: For broad current-window requests with no distinctive subject terms, trusted structured feed rows can be useful evidence if they carry current signal plus an answer-bearing claim. They should not be rejected solely because the URL is a section/listing path.
- `files_changed`: `core/layer2/ops/src/retrieval_policy_parts/010-freshness-and-relevance.rs`, `core/layer2/ops/tests/retrieval_policy_instruction_scaffold.rs`
- `proof_tests`: `cargo test --manifest-path core/layer2/ops/Cargo.toml broad_current_structured_listing_with_current_claim_is_pack_ready -- --nocapture` passed.
- `eval_artifact`: `/tmp/web_tooling_global_news_after_listing_evidence_fixed.json`
- `before_metrics`: `evidence_pack_quality.status=thin`, `usable_count=1`, `source_domain_count=1`, `evidence_refs_count=1`, `evidence_claims_count=1`.
- `after_metrics`: `evidence_pack_quality.status=usable`, `usable_count=2`, `source_domain_count=2`, `evidence_refs_count=2`, `evidence_claims_count=2`, selected domains `nytimes.com` and `cbsnews.com`.
- `visible_output_delta`: positive at the tooling/evidence layer. The pack now contains two current citable evidence items instead of one thin source.
- `decision`: `kept`
- `reason`: This moved the targeted source-quality/evidence-sufficiency metric without changing final-answer format or adding topic/domain-specific answer rules. It is a generic boundary rule for broad current-window retrieval.
- `follow_up`: Run the workflow/synthesis path with this improved evidence pack and check whether the agent now turns the two evidence items into a coherent answer rather than a source fragment list. If not, the next bottleneck is downstream evidence-to-answer synthesis rather than web-tooling candidate supply for this case.

### 2026-06-01: random web-tooling evidence-quality lane and facet-count calibration

- `baseline_artifact`: `core/local/artifacts/web_tooling_golden_user_pool_sample5_current.json`
- `patch_name`: add random sampling to the web-tooling golden harness; cap bounded-answerability required covered facets by the number of facets actually requested.
- `failure_class_targeted`: Repeated first-N web-tooling runs could create hidden test-query overfitting, and the stricter bounded evidence gate incorrectly failed one-facet prompts even when the single requested facet was fully covered.
- `hypothesis`: If web tooling is sampled from the 100-query user pool and evidence grading adapts to the query's actual facet count, the evidence lane will show whether retrieval data itself is answer-ready without hardcoding specific query subjects.
- `files_changed`: `orchestration/src/eval_web_tooling_golden/mod.rs`, `orchestration/src/eval_web_retrieval_gate_diagnostics_parts/050_evidence_quality.rs`, `orchestration/src/eval_web_retrieval_gate_diagnostics_parts/091_tests_claims_and_quality.rs`
- `proof_tests`: `cargo test --manifest-path orchestration/Cargo.toml web_tooling -- --nocapture` passed; `cargo test --manifest-path orchestration/Cargo.toml bounded_answerability_caps_required_facets_to_requested_facet_count -- --nocapture` passed.
- `eval_command`: `cargo run --quiet --manifest-path orchestration/Cargo.toml --bin eval_runtime -- web-tooling-golden --strict=0 --cases validation/evals/fixtures/research_user_prompt_pool_v1.json --sample-size 5 --timeout-seconds 120 --out core/local/artifacts/web_tooling_golden_user_pool_sample5_current.json --out-latest artifacts/web_tooling_golden_user_pool_sample5_latest.json --out-markdown local/workspace/reports/WEB_TOOLING_GOLDEN_USER_POOL_SAMPLE5.md`
- `evidence_grade_command`: `cargo run --quiet --manifest-path orchestration/Cargo.toml --bin eval_runtime -- evidence-quality-grade --strict=0 --input core/local/artifacts/web_tooling_golden_user_pool_sample5_current.json --out core/local/artifacts/evidence_quality_grade_user_pool_sample5_after_facet_cap.json --out-latest artifacts/evidence_quality_grade_user_pool_sample5_latest.json --out-markdown local/workspace/reports/EVIDENCE_QUALITY_GRADE_USER_POOL_SAMPLE5.md`
- `after_metrics`: sampled cases came from 100-case pool with seed `random:5ac87d377511f0c8`; web-tooling gates passed `5/5`; transport failures `0/5`; usable evidence `5/5`; evidence quality readiness `5/5`; bounded answerability improved from `3/5` to `5/5` after facet-count calibration.
- `visible_output_delta`: positive diagnostics. This run indicates the API-backed tooling can return answer-ready packets for diverse sampled queries, so the next useful measurement should feed the same retrieved evidence into the downstream answer path.
- `decision`: `kept`
- `reason`: The sampling change protects against first-N overfitting, and the facet rule is a generic grading correction rather than a query-specific exception.
- `follow_up`: Run a downstream synthesis/handoff batch using retrieved evidence from this sampled tooling artifact. If user-facing answers still fragment, target the evidence-to-answer bridge; if they sound good, move back to larger sampled tooling/evidence batches.

### 2026-06-01: live web evidence to downstream answer replay

- `baseline_artifact`: `core/local/artifacts/web_tooling_golden_user_pool_sample5_current.json`
- `patch_name`: allow downstream test-input replay readiness to accept real tool evidence shapes, not only synthetic fixture shapes.
- `failure_class_targeted`: The downstream replay setup rejected live tool packets because they lacked fixture-only `id` fields, used `source_type`/`source_class` in addition to `source_kind`, and sometimes had a two-item answer-ready evidence pack.
- `hypothesis`: If real tool evidence can enter the downstream-only replay without being rewritten into synthetic fixture shape, we can measure the evidence-to-answer bridge independently of live web retrieval.
- `files_changed`: `orchestration/src/eval_research_perfect_evidence.rs`
- `derived_dataset`: `core/local/artifacts/research_web_tooling_sample5_downstream_dataset.json`
- `readiness_artifact`: `core/local/artifacts/research_web_tooling_sample5_downstream_readiness_after_real_tool_shape_v2.json`
- `replay_artifact`: `core/local/artifacts/research_web_tooling_sample5_downstream_replay_after_real_tool_shape.json`
- `responses_artifact`: `core/local/artifacts/research_web_tooling_sample5_downstream_responses_after_real_tool_shape.json`
- `after_metrics`: downstream readiness `5/5`; replay `5/5` pass, `5/5` excellent, `5/5` sounds_good, lane isolation `5/5`, transport failures `0/5`, synthesis hard failures `0/5`.
- `manual_soft_read`: sampled outputs were coherent bounded answers, not source-title dumps. They directly answered travel, privacy-law, JWST, Quebec weekend, and home-inventory prompts while naming evidence gaps when packets lacked specifics.
- `decision`: `kept`
- `reason`: This is a harness fidelity fix. It lets the eval measure real retrieved evidence packets without hardcoding subjects or forcing a visible final-answer format.
- `follow_up`: Run a larger random web-tooling sample and then replay only the ready evidence packets downstream. The next primitive question is whether the API-backed tooling keeps producing answer-ready packets across broader random coverage, not whether the downstream bridge can handle a clean five-case sample.

### 2026-06-01: 20-case downstream replay evaluator calibration

- `baseline_artifact`: `core/local/artifacts/research_web_tooling_sample20_ready_downstream_replay.json`
- `patch_name`: calibrate generic relevance/traceability scoring for compound terms, related adjective forms, low-risk wording gaps, connector words, and action-oriented practical guidance.
- `failure_class_targeted`: Saved user-facing answers sounded useful in manual soft review, but hard scoring falsely failed or downgraded them because `socialmedia`/`gigworker` did not match spaced evidence text, `European` did not match `Europe`, one unsupported action/connective word could block traceability, and practical action plans did not count as decision value.
- `hypothesis`: If grading better matches evidence semantics without adding subject-specific expected answers, the same saved responses should regrade as pass/excellent without changing the model output.
- `files_changed`: `orchestration/src/eval_research_golden_scoring_parts/051_answer_alignment.rs`, `orchestration/src/eval_research_golden_scoring_parts/052_answer_risk_signals.rs`, `orchestration/src/eval_research_golden_scoring_parts/071_retrieval_relevance.rs`, `orchestration/src/eval_research_golden_scoring_parts/082_response_signals.rs`, `orchestration/src/eval_research_golden_scoring_parts/090_tests_retrieval_and_query.rs`, `orchestration/src/eval_research_golden_scoring_parts/092_tests_relevance_and_citations.rs`
- `proof_tests`: `cargo test --manifest-path orchestration/Cargo.toml prompt_relevance_supports_compact_compound_topic_terms -- --nocapture` passed; `cargo test --manifest-path orchestration/Cargo.toml answer_unit_alignment_single_low_risk_wording_gap_is_not_significant -- --nocapture` passed; `cargo test --manifest-path orchestration/Cargo.toml answer_unit_alignment_supports_related_adjective_forms -- --nocapture` passed; `cargo test --manifest-path orchestration/Cargo.toml decision_value_accepts_action_oriented_practical_guidance -- --nocapture` passed; `cargo test --manifest-path orchestration/Cargo.toml answer_unit_specific_terms_ignore_connective_words -- --nocapture` passed.
- `regrade_command`: `cargo run --quiet --manifest-path orchestration/Cargo.toml --bin eval_runtime -- research-perfect-evidence --mode test-input-regrade --strict=0 --cases core/local/artifacts/research_web_tooling_sample20_ready_downstream_dataset.json --responses core/local/artifacts/research_web_tooling_sample20_ready_downstream_responses.json --limit 14 --out core/local/artifacts/research_web_tooling_sample20_ready_downstream_regrade_after_eval_calibration_v2.json --out-latest artifacts/research_web_tooling_sample20_ready_downstream_regrade_after_eval_calibration_latest.json --out-markdown local/workspace/reports/RESEARCH_WEB_TOOLING_SAMPLE20_READY_DOWNSTREAM_REGRADE_AFTER_EVAL_CALIBRATION.md`
- `before_metrics`: saved downstream replay from 14 ready live-evidence packets had pass `11/14`, Excellent `8/14`, sounds_good `14/14`.
- `after_metrics`: offline regrade of the same saved responses has pass `14/14`, Excellent `13/14`, sounds_good `14/14`, lane isolation `14/14`, transport failures `0/14`.
- `remaining_blocker`: `research_pool_018_sodium_ion_batteries` is still non-Excellent because retrieval relevance finds only one relevant evidence item; this looks like evidence-depth/source-diversity rather than downstream synthesis failure.
- `decision`: `kept`
- `reason`: This corrected evaluator false negatives against unchanged saved answers. The patch is semantic and generic; it does not encode expected answers for any query.
- `follow_up`: Re-run a fresh random web-tooling sample only if provider quota/cost is acceptable. Otherwise, the next primitive target is upstream evidence readiness: raise source-quality/answerability/usable-evidence rates beyond the 20-case live sample's `14/20` evidence-ready and `12/20` web-tooling pass rates.

### 2026-06-01: subject-preserving explicit query-pack planning

- `baseline_artifact`: `core/local/artifacts/web_tooling_golden_user_pool_sample20_current.json`
- `patch_name`: preserve subject-bearing query text when a request already supplies explicit query lanes or query metadata.
- `failure_class_targeted`: Some explicit research packs were canonicalized into criteria-only tails such as `practicality and workflow fit` or `operational burden retrieval quality and lock`. Under the initial-lane execution limit, those weak standalone lanes could starve entity-rich searches and lead to missing claim hints or thin evidence.
- `hypothesis`: If explicit query packs keep the original subject/entities while still stripping only leading search-control phrasing, the first executable lanes should stay relevant without adding topic-specific query assumptions.
- `files_changed`: `core/layer0/ops/src/batch_query_primitive_parts/018-request-and-cache.rs`, `core/layer0/ops/src/batch_query_primitive_parts/042-cache-rewrite-tests.rs`, `orchestration/src/eval_web_tooling_golden/direct_tool.rs`, `orchestration/src/eval_web_tooling_golden/tests.rs`
- `proof_tests`: `cargo test --manifest-path core/layer0/ops/Cargo.toml --lib explicit_metadata_query_plan_does_not_promote_criterion_tail -- --nocapture` passed; `cargo test --manifest-path core/layer0/ops/Cargo.toml --lib query_plan -- --nocapture` passed; `cargo build --manifest-path core/layer0/ops/Cargo.toml --bin infring-ops` passed; `cargo test --manifest-path orchestration/Cargo.toml direct_tool_sample_preserves_cache_and_query_metadata_diagnostics -- --nocapture` passed; `cargo test --manifest-path orchestration/Cargo.toml live_batch_query_tooling_eval_bypasses_cache_by_default -- --nocapture` passed.
- `before_metrics`: In the 20-case web-tooling artifact, `research_pool_024_bookkeeping_automation` failed first at `web_5c_claim_extraction_present`; diagnostics showed a criterion-only first lane and selected evidence collapsing to a Puzzle login URL despite the request naming QuickBooks, Xero, Pilot, and Puzzle. A similar criteria-first pattern appeared in the vector retrieval stack sample.
- `live_validation_artifact`: `core/local/artifacts/web_tooling_subject_preserving_focus2_after_patch_rebuilt_server.json`
- `measurement_note`: The first focused live run reused an old `target/debug/infring-ops` binary and reproduced the stale criteria-first behavior. After rebuilding the serving binary and running with cache disabled, the focused validation reflected the patch. The compact direct-tool sample now records `cache_status`, `cache_mode`, and `query_metadata` so this measurement trap is visible next time.
- `after_metrics`: Rebuilt-server focused validation on `research_pool_006_vector_retrieval_stack` and `research_pool_024_bookkeeping_automation` passed all web-tooling gates `2/2`. `web_5c`, `web_5d`, `web_5e`, `web_5f`, `web_5g`, `web_5h`, and `web_7` all moved from failing in the stale-binary run to `2/2`. The first executed lane now preserves subject and entities instead of criteria-only text. Vector evidence improved from `thin`, `usable_count=1`, `source_domain_count=1`, `claim_hint_count=1` to `usable`, `usable_count=6`, `source_domain_count=6`, `claim_hint_count=9`, `evidence_claim_count=8`. Bookkeeping improved from `absent`, `usable_count=0`, `source_domain_count=0`, `claim_hint_count=0` to `usable`, `usable_count=6`, `source_domain_count=6`, `claim_hint_count=9`, `evidence_claim_count=8`.
- `visible_output_delta`: positive at the tooling/evidence layer. The tool now returns multi-source, claim-bearing evidence packets for the two failures that previously collapsed to criteria-only searches.
- `decision`: `kept`
- `reason`: This is a generic planner-correctness fix: explicit packs should not be reduced to a non-subject facet. It does not hardcode any domain, answer, or query category.
- `follow_up`: Run a larger random web-tooling sample against a rebuilt/current serving binary. If the evidence-ready rate moves up, derive a downstream replay dataset from the ready packets and check whether user-facing answers stay coherent.

### 2026-06-01: random10 live-tooling packets to downstream replay split

- `baseline_artifact`: `core/local/artifacts/web_tooling_subject_preserving_focus2_after_patch_rebuilt_server.json`
- `patch_name`: split downstream replay measurement so not-ready evidence inputs are reported separately from transport/harness failures.
- `failure_class_targeted`: A downstream replay using live-tooling packets mixed two different facts in the same failure bucket: synthesis worked on ready evidence, while source-diversity-blocked packets never reached synthesis. The summary labeled those skipped cases as transport/harness failures, obscuring whether the current blocker was evidence supply or answer generation.
- `hypothesis`: If evidence-input readiness is a first-class bucket, we can tell whether the evidence-to-answer bridge is failing or whether upstream tooling still needs better source diversity and answer-ready packets.
- `files_changed`: `orchestration/src/eval_research_perfect_evidence.rs`
- `proof_tests`: `cargo test --manifest-path orchestration/Cargo.toml replay_summary_splits_transport_from_synthesis_quality -- --nocapture` passed.
- `web_tooling_artifact`: `core/local/artifacts/web_tooling_user_pool_random10_after_subject_preserving.json`
- `web_tooling_seed`: `random:a24c415bf77d08da`
- `derived_dataset`: `core/local/artifacts/research_web_tooling_random10_ready_downstream_dataset_after_subject_preserving.json`
- `readiness_artifact`: `core/local/artifacts/research_web_tooling_random10_ready_downstream_readiness_after_subject_preserving.json`
- `replay_artifact`: `core/local/artifacts/research_web_tooling_random10_ready_downstream_replay_after_measurement_split.json`
- `responses_artifact`: `core/local/artifacts/research_web_tooling_random10_ready_downstream_responses_after_measurement_split.json`
- `web_tooling_metrics`: random 10 had clean upstream gates for request shape, query metadata, tool attempt, transport, access/bot/quota/config, raw candidates, provider rows, and evidence context handoff. Evidence readiness remained mixed: `web_5_packaged_evidence_present=9/10`, `web_5d_source_quality_ready=7/10`, `web_5g_answerability_ready=7/10`, `web_6_provider_not_empty_or_degraded=8/10`, `web_7_usable_evidence_available=7/10`.
- `readiness_metrics`: 7 live-tooling packets were replay-payload-ready; 5 were readiness-clean; 2 were blocked by `source_kind_diversity_lt_2`.
- `downstream_metrics_after_split`: raw replay pass `5/7`, Excellent `5/7`; evidence-input-not-ready `2/7`; transport/harness failures `0/7`; lane leaks `0/7`; synthesized cases `5`; synthesized pass `5/5`; synthesized Excellent `5/5`; synthesized sounds-good `5/5`.
- `manual_soft_read`: The synthesized outputs were coherent bounded answers, not source dumps. Weak or missing evidence was named as a limitation while still answering from what was supported.
- `decision`: `kept`
- `reason`: This is a measurement correction, not a query-specific patch. It makes the current primitive blocker visible: ready evidence turns into useful answers, while some random live-tooling packets still fail before synthesis because evidence diversity/answerability is insufficient.
- `follow_up`: Work upstream on evidence packet readiness rather than downstream synthesis formatting. The next measurable target is increasing `web_5d_source_quality_ready`, `web_5g_answerability_ready`, and `web_7_usable_evidence_available` on random user-pool samples without adding subject-specific rules.

### 2026-06-01: broad multi-facet lane frontloading

- `baseline_artifact`: `core/local/artifacts/web_tooling_focus_carbon_removal_after_query_dedup.json`
- `patch_name`: frontload all requested topical facets into source-backed query lanes before replaying extra explicit/follow-up lanes.
- `failure_class_targeted`: A broad multi-facet query had explicit metadata for three requested facets, but the live execution/recovery window still spent lanes on repeated early-facet variants before reaching the last facet. The submitted plan contained the missing facet lane, but it was too late to affect retrieval under the bounded lane budget.
- `hypothesis`: If a request supplies multiple topical facets, the initial metadata budget should reserve enough front slots for `combined topic + each topical facet source-backed evidence`, so every requested facet has an early chance to produce evidence before narrower variants repeat earlier facets.
- `files_changed`: `core/layer0/ops/src/batch_query_primitive_parts/018-request-and-cache.rs`, `core/layer0/ops/src/batch_query_primitive_parts/042-cache-rewrite-tests.rs`
- `proof_tests`: `cargo test --manifest-path core/layer0/ops/Cargo.toml --lib broad_multi_facet_query_plan_allocates_each_requested_facet -- --nocapture` passed; `cargo test --manifest-path core/layer0/ops/Cargo.toml --lib query_plan -- --nocapture` passed; `cargo build --manifest-path core/layer0/ops/Cargo.toml --bin infring-ops` passed.
- `focused_validation_artifact`: `core/local/artifacts/web_tooling_focus_carbon_removal_after_facet_frontload.json`
- `before_metrics`: focused carbon-removal tooling case failed `0/1`; `status=no_results`; `evidence_pack_quality=absent`; first failed gate `web_3b1_provider_quota_not_rate_limited`; downstream gates `web_5_packaged_evidence_present`, `web_5c`, `web_5d`, `web_5g`, and `web_7` failed. The deferred window did not reach `biochar source-backed evidence`.
- `after_metrics`: focused carbon-removal tooling case passed `1/1`; `status=partial`; `evidence_pack_quality=usable`; evidence items `2`; evidence claims `3`; all web-tooling gates passed `1/1`; weakest gates empty. Coverage moved to direct air capture `covered`, biochar `covered`, mineralization `weak`.
- `decision`: `kept`
- `reason`: This is a generic query-planning fix for broad multi-facet requests under bounded execution budgets. It does not encode the carbon-removal topic, domains, expected answer, or output format.
- `follow_up`: Re-run a random live-tooling batch on the rebuilt binary. If evidence readiness improves broadly, derive another downstream replay set; if not, target the next upstream evidence-packet blocker from the random batch metrics.

### 2026-06-01: deferred recovery lane budget probe

- `baseline_artifact`: `core/local/artifacts/web_tooling_user_pool_random10_after_facet_frontload.json`
- `patch_name`: temporarily increase deferred recovery lanes from `2` to `4`.
- `failure_class_targeted`: Two random sampled cases had only one usable source domain after the bounded initial wave and two deferred recovery lanes, failing first at `web_5d_source_quality_ready`.
- `hypothesis`: If thin evidence was caused by insufficient recovery lane budget, executing two more already-submitted deferred lanes should raise the same seeded batch above `8/10`.
- `files_changed_temporarily`: `core/layer0/ops/config/batch_query_policy.json`, `client/runtime/config/batch_query_policy.json`
- `validation_artifact`: `core/local/artifacts/web_tooling_user_pool_random10_after_deferred_recovery4.json`
- `before_metrics`: same seed `random:f1071dd61cd9a532`; web-tooling pass `8/10`; failing gates `web_5d_source_quality_ready=8/10`, `web_5g_answerability_ready=8/10`, `web_7_usable_evidence_available=8/10`.
- `after_metrics`: same seed stayed `8/10`; same three gates stayed `8/10`. The two failed cases did execute extra deferred lanes, but evidence remained `thin`.
- `decision`: `reverted`
- `reason`: The patch increased retrieval work without an obvious positive metric movement. The current blocker is not simply recovery lane count; it is candidate promotion/source-quality selection after candidate supply exists.
- `follow_up`: Target the source-quality promotion path: when a packet is thin but candidate rows include additional relevant domains, identify why the second domain is rejected (`freshness_unproven`, low relevance, PDF/materialization, source-class penalty) and patch the generic promotion rule rather than spending more lanes.

### 2026-06-01: generic claim-signal promotion probe

- `baseline_artifact`: `core/local/artifacts/web_tooling_user_pool_random10_after_facet_frontload.json`
- `patch_name`: temporarily broaden generic claim-signal detection for outcome verbs and treat broad time/window words as non-distinctive query scaffold.
- `failure_class_targeted`: The same seeded random batch had two single-source evidence packets. Diagnostics showed additional content-rich rows with missing claim hints, including source snippets whose useful claim text used verbs such as `drive`, `granted`, or `uncovers`.
- `hypothesis`: If the thin packets were caused by the claim extractor missing generic outcome verbs or over-treating time/scaffold terms as distinctive topical terms, more rows should become pack-ready evidence and raise `web_5d_source_quality_ready`, `web_5g_answerability_ready`, and `web_7_usable_evidence_available`.
- `files_changed_temporarily`: `core/layer2/ops/src/retrieval_policy_parts/010-freshness-and-relevance.rs`, `core/layer2/ops/src/retrieval_policy_support/010-core.combined_parts/050-looks-like-definition-candidate-to-extract-metric-focused-fragment.rs`
- `proof_test`: focused unit probe `claim_hints_accept_generic_outcome_verbs_in_source_snippets` passed while the temporary patch was present.
- `stale_validation_artifact`: `core/local/artifacts/web_tooling_user_pool_random10_after_claim_signal_patch.json`
- `rebuilt_focus_validation_artifact`: `core/local/artifacts/web_tooling_focus3_after_claim_signal_patch_rebuilt.json`
- `before_metrics`: same seed `random:f1071dd61cd9a532`; web-tooling pass `8/10`; failing gates `web_5d_source_quality_ready=8/10`, `web_5g_answerability_ready=8/10`, `web_7_usable_evidence_available=8/10`.
- `stale_measurement_note`: The first 10-case run was invalid for patch judgment because the serving `target/debug/infring-ops` binary had not been rebuilt; diagnostics still showed the Nature/Springer rows with `claim_hint_count=0`.
- `rebuilt_focus_metrics`: after rebuilding `infring-ops`, focused validation on the two original failed cases plus self-checkout still failed `0/3`; `web_5d_source_quality_ready=0/3`, `web_5g_answerability_ready=0/3`, `web_7_usable_evidence_available=0/3`. The run also surfaced provider rate-limit/degradation on GLP-1 and persistent `freshness_unproven` blocks on otherwise relevant official/authoritative rows.
- `decision`: `reverted`
- `reason`: The patch improved isolated extractor behavior but did not move the rebuilt live gate positively. The live blocker is not safely solved by broadly relaxing claim-hint admission.
- `follow_up`: Target freshness eligibility for stable, authoritative, query-relevant evidence. Several failed rows have concrete claim hints and trusted/public-source signals but are excluded solely because current-intent freshness is unproven.

### 2026-06-01: stable authoritative freshness eligibility

- `baseline_artifact`: `core/local/artifacts/web_tooling_user_pool_random10_after_facet_frontload.json`
- `patch_name`: allow stable authoritative evidence to survive `freshness_unproven` for broad current evidence requests, while keeping exact month/year windows strict.
- `failure_class_targeted`: Broad current evidence requests could reject authoritative, content-rich, query-relevant sources solely because the row did not prove a fresh timestamp, leaving otherwise useful packets thin.
- `hypothesis`: If stable authoritative rows are admitted only when they are trusted, content-rich or metric-rich, query-overlapping, non-shell, and not for explicit month/year windows or live-update/news requests, then broad evidence packets should gain source quality without letting stale rows answer dated windows.
- `files_changed`: `core/layer2/ops/src/retrieval_policy_parts/010-freshness-and-relevance.rs`
- `proof_tests`: `cargo test --manifest-path core/layer2/ops/Cargo.toml stable_authoritative_evidence -- --nocapture` passed; `cargo build --manifest-path core/layer0/ops/Cargo.toml --bin infring-ops` passed.
- `focused_validation_artifact`: `core/local/artifacts/web_tooling_focus3_after_stable_authoritative_freshness.json`
- `random_validation_artifact`: `core/local/artifacts/web_tooling_user_pool_random10_after_stable_authoritative_freshness.json`
- `before_metrics`: same seed `random:f1071dd61cd9a532`; web-tooling pass `8/10`; `web_5d_source_quality_ready=8/10`, `web_5g_answerability_ready=8/10`, `web_7_usable_evidence_available=8/10`. GLP-1 and April-2026 science failed.
- `after_metrics`: focused three-case run improved from rebuilt `0/3` to `2/3`, fixing GLP-1 and self-checkout while leaving the exact April-2026 science window strict. Same-seed random run stayed web-tooling pass `8/10`; `web_5d_source_quality_ready` improved `8/10 -> 9/10`, while `web_5g_answerability_ready` and `web_7_usable_evidence_available` stayed `8/10`.
- `decision`: `kept`
- `reason`: The patch produced a clear upstream source-quality improvement and fixed a real broad-current failure class without relaxing exact dated windows. The overall random pass rate did not move because the next blocker shifted to answerability/source decomposition.
- `follow_up`: Continue top-down from the remaining source/answerability miss. The April-2026 science failure now shows good-looking source material inside provider previews that never becomes independent candidate evidence.

### 2026-06-01: provider preview claim-extraction probe

- `baseline_artifact`: `core/local/artifacts/web_tooling_focus3_after_stable_authoritative_freshness.json`
- `patch_name`: temporarily add generic event/action claim verbs and mark temporal/evidence-status words as weak relevance terms.
- `failure_class_targeted`: The April-2026 science failure had candidate diagnostics with concrete-looking snippets such as `FDA granted breakthrough therapy designation...` but zero claim hints.
- `hypothesis`: If the blocker is claim extraction vocabulary or scaffold-overlap, then adding generic event verbs and demoting temporal/status scaffold terms should turn the same focused case into packaged evidence.
- `files_changed_temporarily`: `core/layer2/ops/src/retrieval_policy_parts/010-freshness-and-relevance.rs`, `core/layer2/ops/src/retrieval_policy_support/010-core.combined_parts/050-looks-like-definition-candidate-to-extract-metric-focused-fragment.rs`
- `proof_tests`: `cargo test --manifest-path core/layer2/ops/Cargo.toml concrete_event_claim_hints_include_generic_grant_and_designation_verbs -- --nocapture` passed while the temporary patch was present; `cargo test --manifest-path core/layer2/ops/Cargo.toml structured_feed_title_dateline_shells_need_extraction_before_usable_evidence -- --nocapture` also passed.
- `validation_artifact`: `core/local/artifacts/web_tooling_focus3_after_event_claim_scaffold_patch.json`
- `before_metrics`: focused validation after stable freshness passed `2/3`; April-2026 science remained thin/low-relevance with only one packaged evidence item in that run.
- `after_metrics`: focused validation stayed `2/3`. The April-2026 science case still failed before packaging; diagnostics showed the useful TargetedOnc material inside Tavily `provider_results.content_preview`, not as an independent candidate row that claim extraction could process.
- `decision`: `reverted`
- `reason`: The unit-level extraction fix was plausible but did not move the live focused metric. The live blocker is upstream of claim extraction: provider preview text can contain multiple title/URL/snippet records, but only the provider result shell enters candidate promotion.
- `follow_up`: Target provider-result decomposition. Split structured provider previews containing `Title — URL — snippet` records into independent candidate rows before ranking/materialization, guarded by URL safety, dedupe, query relevance, and shell filters.

### 2026-06-01: provider preview decomposition plus claim extraction probe

- `baseline_artifact`: `core/local/artifacts/web_tooling_focus3_after_stable_authoritative_freshness.json`
- `patch_name`: temporarily split multi-URL rendered provider preview strings into candidate rows, then reapply generic event/action claim verbs and scaffold term weakening.
- `failure_class_targeted`: The April-2026 science case sometimes had useful citable-looking material trapped inside Tavily preview text, then zero claim hints after candidate selection.
- `hypothesis`: If provider preview decomposition exposes embedded URL records and the claim extractor recognizes generic event verbs, the focused science case should move from absent/thin evidence to a claim-bearing evidence pack.
- `files_changed_temporarily`: `core/layer2/ops/src/retrieval_policy_parts/020-candidate-rows.rs`, `core/layer2/ops/src/retrieval_policy_parts/010-freshness-and-relevance.rs`, `core/layer2/ops/src/retrieval_policy_support/010-core.combined_parts/050-looks-like-definition-candidate-to-extract-metric-focused-fragment.rs`
- `proof_tests`: `cargo test --manifest-path core/layer2/ops/Cargo.toml rendered_provider_preview_splits_embedded_url_records_into_candidates -- --nocapture` passed while the temporary patch was present; `cargo test --manifest-path core/layer2/ops/Cargo.toml concrete_event_claim_hints_include_generic_grant_and_designation_verbs -- --nocapture` passed; `cargo test --manifest-path core/layer2/ops/Cargo.toml structured_feed_title_dateline_shells_need_extraction_before_usable_evidence -- --nocapture` passed.
- `decomposition_only_artifact`: `core/local/artifacts/web_tooling_focus3_after_provider_preview_decomposition.json`
- `combined_validation_artifact`: `core/local/artifacts/web_tooling_focus3_after_provider_preview_decomposition_plus_claims.json`
- `before_metrics`: focused stable-freshness validation passed `2/3`; `web_5_packaged_evidence_present=3/3`, `web_5c_claim_extraction_present=3/3`, `web_5d_source_quality_ready=2/3`, `web_7_usable_evidence_available=2/3`.
- `decomposition_only_metrics`: focused pass stayed `2/3`; `web_5_packaged_evidence_present=3/3` and `web_6_provider_not_empty_or_degraded=3/3`, but `web_5c_claim_extraction_present` fell to `2/3`. Diagnostics did show `targetedonc.com` as an independent selected candidate in that run, proving the parser could expose the embedded row.
- `combined_after_metrics`: focused pass stayed `2/3`; `web_5_packaged_evidence_present=2/3`, `web_5c_claim_extraction_present=2/3`, `web_6_provider_not_empty_or_degraded=2/3`, `web_7_usable_evidence_available=2/3`. The science case returned a different low-confidence provider set and did not produce a usable pack.
- `decision`: `reverted`
- `reason`: The combined patch did not produce an obvious positive live metric movement and made some focused gates worse in the validation run. The live focus case is too provider-variable to validate this parser change safely without a deterministic provider-preview fixture lane.
- `follow_up`: Build or use a deterministic tooling fixture that feeds the exact multi-URL provider preview shape through candidate selection/evidence packaging. Only then retry live if the fixture proves the parser row becomes pack-ready without relaxing shell guards.

### 2026-06-01: deterministic provider-preview evidence bridge

- `baseline_artifact`: `core/local/artifacts/web_tooling_focus3_after_stable_authoritative_freshness.json`
- `patch_name`: split structured provider preview strings containing multiple embedded result records, then admit generic event/action claims and date/category scaffold as non-distinctive claim-overlap terms.
- `failure_class_targeted`: Provider API previews can contain useful `title -> URL -> snippet` records inside a single `content_preview`; previously only the first URL became a candidate, and a later concrete claim such as an official designation could be buried or fail claim overlap.
- `hypothesis`: If the exact multi-URL provider-preview shape is decomposed before candidate promotion, and broad date/category wording does not force false distinctive overlap, then a useful embedded row can become independent claim-backed evidence without relaxing shell guards.
- `files_changed`: `core/layer2/ops/src/retrieval_policy_parts/020-candidate-rows.rs`, `core/layer2/ops/src/retrieval_policy_parts/010-freshness-and-relevance.rs`, `core/layer2/ops/src/retrieval_policy_support/010-core.combined_parts/050-looks-like-definition-candidate-to-extract-metric-focused-fragment.rs`
- `proof_tests`: `cargo test --manifest-path core/layer2/ops/Cargo.toml rendered_provider_preview -- --nocapture` passed; `cargo test --manifest-path core/layer2/ops/Cargo.toml stable_authoritative_evidence -- --nocapture` passed; `cargo test --manifest-path core/layer2/ops/Cargo.toml structured_feed_title_dateline_shells_need_extraction_before_usable_evidence -- --nocapture` passed; `cargo build --manifest-path core/layer0/ops/Cargo.toml --bin infring-ops` passed.
- `focused_validation_artifact`: `core/local/artifacts/web_tooling_focus3_after_provider_preview_bridge.json`
- `before_metrics`: focused stable-freshness validation passed `2/3`; April-2026 science remained failed because useful provider-preview material sometimes did not become independent claim-backed evidence.
- `after_metrics`: deterministic bridge fixture passed: the embedded TargetedOnc-style row became an independent candidate and then a `counts_as_usable_evidence=true` evidence row with a concrete claim hint. Live focused validation passed only `1/3`; `web_5_packaged_evidence_present=2/3`, `web_5c_claim_extraction_present=2/3`, `web_5d_source_quality_ready=1/3`, `web_5g_answerability_ready=1/3`, `web_7_usable_evidence_available=1/3`.
- `visible_output_delta`: positive in deterministic evidence-bridge coverage, not positive in this live provider sample. The live April-2026 science case did not return the previously observed TargetedOnc-style embedded record; it returned mostly low-confidence/off-intent science-listing candidates, so there was no good embedded claim for the new bridge to rescue.
- `decision`: `kept_for_deterministic_bridge_pending_broader_live_confirmation`
- `reason`: This is a generic extraction-boundary fix with active tests proving the failure class. The live run shows the current bottleneck is now candidate/provider material quality when no rescuable embedded record is present, not the provider-preview bridge itself.
- `follow_up`: Target the provider side-channel and materialization path: when `provider_results.links` contains plausible source URLs but selected candidates are low-confidence/off-intent, determine whether link ranking, prefetch rejection, fetch quality, or materialization is preventing those links from becoming evidence. Validate with deterministic fixtures before another live batch.

### 2026-06-01: provider side-channel materialization bridge

- `baseline_artifact`: `core/local/artifacts/web_tooling_focus3_after_provider_preview_bridge.json`
- `patch_name`: let provider-side result links survive relevance preflight when temporal/scaffold query terms are the only missing overlap, and admit generic source claims phrased with verbs like `highlights`, `include`, and `advances`.
- `failure_class_targeted`: A live science case exposed plausible source URLs in `provider_results.links`, but those links did not become packaged evidence. A deterministic fixture reproduced the same boundary: the source URL was rejected before materialization because month/scaffold terms were treated as distinctive topical requirements, then the fetched page's generic claim phrasing failed pack readiness.
- `hypothesis`: If provider side-channel URLs are relevant apart from temporal/scaffold wording, and the fetched page contains concrete generic claim verbs, the URL should become citable evidence without relaxing source shell, URL safety, or dated-window freshness guards.
- `files_changed`: `core/layer0/ops/src/batch_query_primitive_parts/010-core.combined_parts/050-looks-like-definition-candidate-to-extract-metric-focused-fragment.rs`, `core/layer2/ops/src/retrieval_policy_parts/010-freshness-and-relevance.rs`, `core/layer0/ops/src/batch_query_primitive_parts/040-tests.rs.parts/010-segment.inc`
- `proof_tests`: `cargo test --manifest-path core/layer0/ops/Cargo.toml --lib provider_side_links_are_fetched_into_packaged_evidence_when_search_summary_is_thin -- --nocapture` passed; `cargo test --manifest-path core/layer0/ops/Cargo.toml --lib search_payload_prefers_result_link_locator_over_search_engine_request_url -- --nocapture` passed; `cargo test --manifest-path core/layer2/ops/Cargo.toml rendered_provider_preview -- --nocapture` passed; `cargo test --manifest-path core/layer2/ops/Cargo.toml stable_authoritative_evidence -- --nocapture` passed; `git diff --check` passed.
- `after_metrics`: deterministic provider-side link fixture now moves from thin search preview to fetched source URL, evidence ref, and non-empty evidence pack. The adjacent locator test was updated to model the stricter modern path by adding a follow-up page fixture, and still proves the result URL is used instead of the search-engine request URL.
- `decision`: `kept_pending_live_confirmation`
- `reason`: This is a generic side-channel materialization fix, not a query-specific answer patch. It preserves the stricter rule that snippets alone are not enough; the source URL has to fetch into evidence before synthesis can use it.
- `follow_up`: Rebuild/run a small live tooling focus batch on the same three cases. If the science case still fails, the next primitive blocker is no longer side-channel link admission; it is provider candidate quality or fetched-page substance for the URLs returned in that live sample.

### 2026-06-01: query-grounded rationale and media-shell claim guard

- `baseline_artifact`: `core/local/artifacts/web_tooling_focus3_after_scaffold_token_prefetch_bridge_rerun.json`
- `patch_name`: make `why_relevant_to_query` report only request terms that actually occur in the candidate, and reject media duration/view-count/playlist shells before they can become evidence claim units.
- `failure_class_targeted`: The live science case had two distinct quality leaks after the gates turned green: relevance rationales could name candidate-only words as "query terms", and `evidence_claims` could promote a video playlist/view-count fragment as a usable claim.
- `hypothesis`: If relevance rationales are grounded in actual request-term overlap, and media metadata shells fail the same answer-substance path as page chrome/headline shells, the evidence handoff should become less misleading without lowering web-tooling pass rates.
- `files_changed`: `core/layer2/ops/src/retrieval_policy_parts/010-freshness-and-relevance.rs`, `orchestration/src/eval_web_retrieval_gate_diagnostics_parts/050_evidence_quality.rs`, `orchestration/src/eval_web_retrieval_gate_diagnostics_parts/091_tests_claims_and_quality.rs`
- `proof_tests`: `cargo test --manifest-path core/layer2/ops/Cargo.toml evidence_packet_promotion_tests -- --nocapture` passed; `cargo test --manifest-path orchestration/Cargo.toml eval_web_retrieval_gate_diagnostics::tests -- --nocapture` passed; `cargo build --manifest-path core/layer0/ops/Cargo.toml --bin infring-ops` passed.
- `focused_validation_artifacts`: `core/local/artifacts/web_tooling_focus3_after_query_grounded_rationales_rerun.json`, `core/local/artifacts/web_tooling_focus3_after_media_metric_shell_guard.json`
- `before_metrics`: focused validation after the provider-side materialization bridge passed `3/3`, but the science case still showed misleading rationale text and one promoted media-metadata claim: `59:02 ... 65K views ... Live Playlist`.
- `after_metrics`: focused validation stayed `3/3`; all web-tooling gates stayed `1.0`; transport failures stayed `0`; the promoted media-metadata claim disappeared from the science case, reducing science `evidence_claims` from `8` to `6`.
- `decision`: `kept`
- `reason`: This is a generic evidence-handoff fidelity fix. It did not merely satisfy a gate: it removed a visibly bad claim unit while preserving the live pass rate.
- `follow_up`: The next primitive weakness is not media metadata; it is index/listing or teaser snippets that contain claim-shaped text but are still weak synthesis material, such as source home-page headline clusters and vague teaser claims. The next patch should target generic listing/teaser answerability rather than any specific domain or query.

### 2026-06-01: homepage headline-cluster guard

- `baseline_artifact`: `core/local/artifacts/web_tooling_focus3_after_media_metric_shell_guard.json`
- `patch_name`: reject source homepage or index snippets that look like linked headline/teaser clusters before they become evidence rows or claim units.
- `failure_class_targeted`: A source homepage with several linked headlines could provide claim-shaped fragments, then appear as answer-ready evidence even though it was really a navigational/news-index surface rather than a source-specific evidence page.
- `hypothesis`: If headline clusters require source-specific substance instead of being promoted from "latest/source/headlines" homepage context, the evidence handoff should contain fewer misleading claim units while keeping valid article/trend pages available.
- `files_changed`: `core/layer2/ops/src/retrieval_policy_parts/010-freshness-and-relevance.rs`
- `proof_tests`: `cargo test --manifest-path core/layer2/ops/Cargo.toml evidence_packet_promotion_tests -- --nocapture` passed; `cargo build --manifest-path core/layer0/ops/Cargo.toml --bin infring-ops` passed; `git diff --check` passed.
- `focused_validation_artifact`: `core/local/artifacts/web_tooling_focus3_after_headline_cluster_guard.json`
- `before_metrics`: focused validation after the media-shell guard passed `3/3`; science case had `6` evidence claims and still admitted `ScienceDaily Your source for the latest research news` headline-cluster claims.
- `after_metrics`: focused validation stayed `3/3`; all web gates stayed passing; science evidence claims dropped from `6` to `4`; `ScienceDaily Your source for the latest research news` was marked with `headline_cluster_or_teaser_index`, `pack_ready=false`, and was not selected as claim-ready evidence.
- `decision`: `kept`
- `reason`: This made the live evidence set smaller and cleaner without sacrificing pass rate. It is a generic evidence-quality guard, not a domain or query-specific rule.
- `follow_up`: The remaining visible weakness is page/snippet substance quality inside otherwise citable article-like rows. CAS still surfaces useful trend bullets through a chrome-heavy extract, and Sciencing still surfaces a vague teaser claim. The next patch should grade extracted claim support more strictly: prefer claim units whose support snippet contains the claim's concrete terms and demote vague teaser language unless a materialized source page provides enough detail.

### 2026-06-01: claim specificity and navigation-chrome guards

- `baseline_artifact`: `core/local/artifacts/web_tooling_focus3_after_headline_cluster_guard.json`
- `patch_name`: clean claim-list markers, reject vague teaser claims without a concrete answer anchor, and reject navigation/footer chrome even when rows are explicitly marked usable.
- `failure_class_targeted`: Claim-shaped fragments could still become synthesis-ready evidence when they were either teaser language without a named object (`could lead to a revolutionary finding`) or page navigation/footer text (`Contact Staff Mission Statement Policies...`).
- `hypothesis`: If the claim promotion path refuses vague teaser text and page chrome, live evidence claims should become cleaner without weakening provider/candidate gates.
- `files_changed`: `core/layer2/ops/src/retrieval_policy_parts/010-freshness-and-relevance.rs`
- `proof_tests`: `cargo test --manifest-path core/layer2/ops/Cargo.toml evidence_packet_promotion_tests -- --nocapture` passed after each patch stage; `cargo build --manifest-path core/layer0/ops/Cargo.toml --bin infring-ops` passed; `git diff --check` passed.
- `claim_specificity_artifact`: `core/local/artifacts/web_tooling_focus3_after_claim_specificity_guard.json`
- `combined_validation_artifact`: `core/local/artifacts/web_tooling_focus3_after_claim_specificity_and_chrome_guard.json`
- `after_metrics`: focused validation stayed `3/3`; all web-tooling gates stayed `1.0`; transport failures stayed `0`. The earlier vague Sciencing-style teaser was removed by the specificity guard. The later navigation-chrome row from the science case disappeared after the navigation/footer guard.
- `decision`: `kept`
- `reason`: Both patches made visible claim material cleaner while preserving the focused live pass rate. They are generic evidence-handoff fidelity guards, not query/domain-specific rules.
- `follow_up`: The remaining visible weakness is article/opinion lede and byline material promoted as answer claims. Examples in the current science artifact include a Guardian first-person/opinion lede, a Science News article lede framed by movie/book references, and a Drug Discovery News author-bio sentence. The next patch should target generic claim support quality: reject byline/author-bio claims, opinion/first-person commentary as factual answer units, and stylized article lead-ins unless the support snippet contains a concrete event, finding, decision, number, or named entity that directly answers the query.

### 2026-06-01: author-bio and first-person opinion claim guard

- `baseline_artifact`: `core/local/artifacts/web_tooling_focus3_after_claim_specificity_and_chrome_guard.json`
- `patch_name`: reject author/editor bio fragments and first-person opinion ledes before they can count as concrete claim material or become `evidence_claims`.
- `failure_class_targeted`: Tooling could promote article metadata or opinion prose as an answer unit, such as an author/editor bio sentence or a first-person aside like `we all know`.
- `hypothesis`: If author bios and first-person opinion ledes fail the claim-safety path, synthesis should receive fewer non-answer claim units without lowering web-tooling gate pass rates.
- `files_changed`: `core/layer2/ops/src/retrieval_policy_parts/010-freshness-and-relevance.rs`
- `proof_tests`: `cargo test --manifest-path core/layer2/ops/Cargo.toml evidence_packet_promotion_tests -- --nocapture` passed with `15/15`; `cargo build --manifest-path core/layer0/ops/Cargo.toml --bin infring-ops` passed; `git diff --check` passed.
- `invalid_transport_artifact`: `core/local/artifacts/web_tooling_focus3_after_author_opinion_guard.json` failed due to dashboard transport dropping before requests and should not be used for quality comparison.
- `focused_validation_artifact`: `core/local/artifacts/web_tooling_focus3_after_author_opinion_guard_rerun.json`
- `after_metrics`: focused validation stayed `3/3`; all web-tooling gates stayed `1.0`; transport failures stayed `0`. The live sample did not reproduce the exact author-bio row, but the deterministic fixture proves that failure class now fails claim promotion, and live validation showed no regression.
- `decision`: `kept_fixture_confirmed_live_no_regression`
- `reason`: This is a generic claim-safety fix. It is not query/domain-specific, and it blocks a previously observed visible failure class without reducing live gate pass rates.
- `follow_up`: The current visible weakness is claim-boundary and article/category-page contamination. Examples include an FDA claim with a trailing unrelated headline, a category-page claim beginning `Science Science in 2026...`, a stylized article lead-in from Science News, and source-page snippets that include `Share`, newsroom, or publication chrome. The next patch should target generic claim-boundary cleanup: trim after adjacent-headline/title transitions, reject repeated section/category prefixes, and demote stylized lead-ins unless they contain concrete answer support.

### 2026-06-01: claim-boundary cleanup

- `baseline_artifact`: `core/local/artifacts/web_tooling_focus3_after_author_opinion_guard_rerun.json`
- `patch_name`: trim adjacent headline tails, collapse repeated category prefixes, and reject stylized reference lead-ins before evidence claims reach synthesis.
- `failure_class_targeted`: Good claims could carry unrelated adjacent headlines or category labels, and stylized article lead-ins could be promoted as answer claims.
- `hypothesis`: If claim cleanup runs before claim safety checks and evidence-claim construction, synthesis should receive cleaner claim units without weakening source/provider gates.
- `files_changed`: `core/layer2/ops/src/retrieval_policy_parts/010-freshness-and-relevance.rs`
- `proof_tests`: `cargo test --manifest-path core/layer2/ops/Cargo.toml evidence_packet_promotion_tests -- --nocapture` passed with `18/18`; `cargo build --manifest-path core/layer0/ops/Cargo.toml --bin infring-ops` passed.
- `stale_server_artifact`: `core/local/artifacts/web_tooling_focus3_after_claim_boundary_cleanup.json` initially measured an old dashboard process and should not be used as the first live comparison.
- `focused_validation_artifact`: `core/local/artifacts/web_tooling_focus3_after_claim_boundary_cleanup.json` after replacing the dashboard process with the rebuilt binary.
- `after_metrics`: focused validation passed `2/3`; `web_5d_source_quality_ready`, `web_5g_answerability_ready`, and `web_7_usable_evidence_available` were each `2/3`. Visible evidence quality improved: the FDA claim lost its glued `Scientific breakthroughs...` headline, the repeated `Science Science...` prefix was trimmed, and the stylized movie-list lede no longer became selected evidence.
- `decision`: `kept_visible_quality_positive_next_gate_exposed`
- `reason`: The patch removed visibly bad claim contamination and exposed the next real bottleneck: relevant metric/survey rows could still fail pack promotion, causing source diversity and answerability to fail on the self-checkout case.
- `follow_up`: Patch the metric/survey claim path so adjacent title tails are cleaned before pack-readiness checks, without relaxing source-quality thresholds.

### 2026-06-01: metric claim-boundary cleanup

- `baseline_artifact`: `core/local/artifacts/web_tooling_focus3_after_claim_boundary_cleanup.json`
- `patch_name`: route metric-focused fragments through claim-boundary cleanup, trim dangling joiner tails after adjacent headlines, and fix `%` detection in the metric-rich regex.
- `failure_class_targeted`: Metric or survey claims could be blocked from evidence promotion when a provider snippet glued a second headline onto the end, such as a percentage claim followed by a title-like tail.
- `hypothesis`: If metric-rich factual prefixes are cleaned before substance checks, useful secondary evidence sources should survive promotion, improving source diversity and bounded answerability without lowering the quality bar.
- `files_changed`: `core/layer2/ops/src/retrieval_policy_parts/010-freshness-and-relevance.rs`, `core/layer2/ops/src/retrieval_policy_support/010-core.combined_parts/040-looks-like-empty-duckduckgo-instant-shell-to-looks-like-metric-rich.rs`
- `proof_tests`: `cargo test --manifest-path core/layer2/ops/Cargo.toml metric_claim_boundary_cleanup_trims_adjacent_headline_tails -- --nocapture` passed; `cargo test --manifest-path core/layer2/ops/Cargo.toml evidence_packet_promotion_tests -- --nocapture` passed with `19/19`; `cargo build --manifest-path core/layer0/ops/Cargo.toml --bin infring-ops` passed.
- `focused_validation_artifact`: `core/local/artifacts/web_tooling_focus3_after_metric_claim_boundary_cleanup.json`
- `after_metrics`: focused live tooling validation passed `3/3`; all web-tooling gates were `1.0`; `weakest_gates=[]`; transport failures were `0`. The previously failing self-checkout case moved to `status=usable`, `source_domain_count=2`, `usable_count=2`, `claim_hint_count=2`, and `evidence_claim_count=2`.
- `decision`: `kept`
- `reason`: This was an obvious positive delta: it fixed the deterministic failure class and moved the focused live lane from `2/3` to `3/3` without introducing a query/domain-specific rule.
- `follow_up`: Run a larger randomized tooling batch to verify the same gate stability holds beyond the three focused cases. If it holds, move downstream to full workflow synthesis quality with the tooling lane treated as provisionally green for this slice.

### 2026-06-02: packaged-source-quality diagnostic calibration

- `baseline_artifact`: `core/local/artifacts/web_tooling_user_pool_random5_after_incident_facet_bridge.json`
- `patch_name`: let `web_5d_source_quality_ready` trust a usable, source-diverse `evidence_pack_quality` object instead of failing on duplicated/noisy auxiliary evidence rows.
- `failure_class_targeted`: The diagnostic could mark a case as source-quality failed even when the actual packaged evidence handed to synthesis was usable, cited, source-diverse, content-rich, and coverage-complete.
- `hypothesis`: If the diagnostic treats the packaged evidence object as authoritative when its own quality thresholds are met, false `web_5d` failures should disappear while genuinely thin packets remain failed.
- `files_changed`: `orchestration/src/eval_web_retrieval_gate_diagnostics_parts/050_evidence_quality.rs`, `orchestration/src/eval_web_retrieval_gate_diagnostics_parts/091_tests_claims_and_quality.rs`
- `proof_tests`: `cargo test --manifest-path orchestration/Cargo.toml source_quality_trusts_usable_pack_over_noisy_auxiliary_rows -- --nocapture` passed.
- `eval_command`: `cargo run --quiet --manifest-path orchestration/Cargo.toml --bin eval_runtime -- web-tooling-golden --live=1 --base-url=http://127.0.0.1:5173 --cases=validation/evals/fixtures/research_user_prompt_pool_v1.json --request-packs-from=/tmp/nonexistent_web_tooling_request_packs_empty.json --sample-size=5 --sample-seed=random:5b31982a1bf35178 --strict=0 --timeout-seconds=120 --out=core/local/artifacts/web_tooling_user_pool_seed5_after_pack_source_quality_calibration.json --out-latest=artifacts/web_tooling_user_pool_seed5_after_pack_source_quality_calibration_latest.json --out-markdown=local/workspace/reports/WEB_TOOLING_USER_POOL_SEED5_AFTER_PACK_SOURCE_QUALITY_CALIBRATION.md`
- `before_metrics`: random 5-case tooling lane passed `3/5`; `web_5d_source_quality_ready=0.6`; `web_5g_answerability_ready=0.6`; `web_7_usable_evidence_available=0.6`. The home blood-pressure monitor case had a usable pack with `usable_count=6`, `source_domain_count=5`, and `covered_facet_ratio=1.0` but still failed `web_5d`.
- `after_metrics`: same seeded lane passed `4/5`; `web_5d_source_quality_ready=0.8`; `web_5g_answerability_ready=0.8`; `web_7_usable_evidence_available=0.8`. The blood-pressure case passed. The right-to-repair case remained failed with a genuinely thin pack: `covered_facet_ratio=0.333`, `weak_facet_count=2`.
- `decision`: `kept`
- `reason`: This was a generic metric calibration fix. It removed a false failure without hiding the remaining real coverage/source-quality issue.
- `follow_up`: Target query planning for single-subject multi-facet prompts, where useful subject/facet lanes existed but were deferred behind duplicate primary and generic official-site lanes.

### 2026-06-02: subject-facet query-lane frontloading

- `baseline_artifact`: `core/local/artifacts/web_tooling_user_pool_seed5_after_pack_source_quality_calibration.json`
- `patch_name`: canonicalize query-plan dedup keys and frontload metadata-derived subject/facet lanes when coverage metadata exists.
- `failure_class_targeted`: The planner submitted useful combined subject/facet lanes but executed only the first few lanes; duplicate primary queries and generic `official site` lanes consumed the first execution window.
- `hypothesis`: If canonical duplicates are deduped and coverage metadata lanes are placed before generic discovery lanes, multi-facet prompts should retrieve evidence that covers each requested facet instead of returning a thin single-subject packet.
- `files_changed`: `core/layer0/ops/src/batch_query_primitive_parts/018-request-and-cache.rs`, `core/layer0/ops/src/batch_query_primitive_parts/042-cache-rewrite-tests.rs`
- `proof_tests`: `cargo test --manifest-path core/layer0/ops/Cargo.toml --lib single_subject_multi_facet_query_plan_frontloads_subject_facet_lanes -- --nocapture` passed; `cargo build --manifest-path core/layer0/ops/Cargo.toml --bin infring-ops` passed.
- `eval_command`: `cargo run --quiet --manifest-path orchestration/Cargo.toml --bin eval_runtime -- web-tooling-golden --live=1 --base-url=http://127.0.0.1:5174 --cases=validation/evals/fixtures/research_user_prompt_pool_v1.json --request-packs-from=/tmp/nonexistent_web_tooling_request_packs_empty.json --sample-size=5 --sample-seed=random:5b31982a1bf35178 --strict=0 --timeout-seconds=120 --out=core/local/artifacts/web_tooling_user_pool_seed5_after_subject_facet_frontload.json --out-latest=artifacts/web_tooling_user_pool_seed5_after_subject_facet_frontload_latest.json --out-markdown=local/workspace/reports/WEB_TOOLING_USER_POOL_SEED5_AFTER_SUBJECT_FACET_FRONTLOAD.md`
- `before_metrics`: same seeded lane passed `4/5`; right-to-repair executed duplicate/generic lanes first and produced `status=thin`, `covered_facet_ratio=0.333`, `weak_facet_count=2`.
- `after_metrics`: same seeded lane passed `5/5`; all web-tooling gates were `1.0`; `weakest_gates=[]`; transport failures were `0`. The right-to-repair executed plan began with the cleaned primary plus subject/facet lanes, and its packet moved to `status=usable`, `covered_facet_ratio=1.0`, `usable_count=6`, `source_domain_count=6`, `weak_facet_count=0`.
- `decision`: `kept`
- `reason`: This was a generic planner improvement with an obvious positive delta on the same seeded batch. It fixed the exposed upstream bottleneck without adding query/domain-specific assumptions.
- `follow_up`: Run a new random 5-case tooling sample. If the tooling lane stays green, move back to full workflow synthesis quality and manually inspect whether the user-facing answers are coherent given these evidence packets.

### 2026-06-02: entity-facet objective anchor guard

- `baseline_artifact`: `core/local/artifacts/web_tooling_user_pool_random5_after_subject_facet_frontload_fresh.json`
- `patch_name`: require entity-only coverage rows to also carry at least one objective facet when the request declares both entities and non-entity facets.
- `failure_class_targeted`: The tooling could mark broad entity pages as usable evidence when they only matched a generic entity label, such as a public-sector definition page for a SaaS data-residency request.
- `hypothesis`: If entity coverage must anchor to the requested objective facet, false-positive evidence packs should become thin instead of feeding irrelevant claims to synthesis.
- `files_changed`: `core/layer2/ops/src/retrieval_policy_parts/010-freshness-and-relevance.rs`, `core/layer2/ops/tests/retrieval_policy_instruction_scaffold.rs`
- `proof_tests`: `cargo test --manifest-path core/layer2/ops/Cargo.toml entity_only_rows_need_objective_anchor_when_facets_exist -- --nocapture` passed; `cargo test --manifest-path core/layer2/ops/Cargo.toml --test retrieval_policy_instruction_scaffold -- --nocapture` passed with `4/4`; `cargo build --manifest-path core/layer0/ops/Cargo.toml --bin infring-ops` passed.
- `eval_command`: `cargo run --quiet --manifest-path orchestration/Cargo.toml --bin eval_runtime -- web-tooling-golden --live=1 --base-url=http://127.0.0.1:5174 --cases=validation/evals/fixtures/research_user_prompt_pool_v1.json --request-packs-from=/tmp/nonexistent_web_tooling_request_packs_empty.json --sample-size=5 --sample-seed=random:3686d89a8bcb0409 --strict=0 --timeout-seconds=120 --out=core/local/artifacts/web_tooling_user_pool_seed3686_after_objective_anchor.json --out-latest=artifacts/web_tooling_user_pool_seed3686_after_objective_anchor_latest.json --out-markdown=local/workspace/reports/WEB_TOOLING_USER_POOL_SEED3686_AFTER_OBJECTIVE_ANCHOR.md`
- `before_metrics`: the prior artifact passed `5/5` hard gates, but manual soft inspection found false positives. The data-residency SaaS case was graded `status=usable` with `usable_count=6`, `covered_facet_ratio=1.0`, and claims from generic public-sector definition/trade pages.
- `after_metrics`: replayed same seed passed `2/5`; provider state was not comparable because the replay hit quota/circuit-open and anti-bot/provider starvation. The data-residency case moved to `status=thin`, `covered_facet_ratio=0.333`, and the generic public-sector rows no longer reached usable/citable evidence.
- `decision`: `kept_measurement_fidelity_positive_provider_state_not_comparable`
- `reason`: This did not improve answerability, but it removed a concrete false positive and made the gates more honest. The lower hard pass rate reflects stricter evidence acceptance plus current provider/circuit limits, not a downstream synthesis regression.
- `follow_up`: Treat the next primitive bottleneck as provider/candidate supply under stricter evidence selection. Work upstream from `web_3b1_provider_quota_not_rate_limited`, `web_4b_search_provider_circuit_closed`, and `web_5d/web_5g/web_7` rather than loosening evidence quality.

### 2026-06-02: web-conduit rate-limit boundary normalization

- `baseline_artifact`: `core/local/artifacts/web_tooling_user_pool_seed3686_after_objective_anchor.json`
- `patch_name`: normalize local web-conduit policy rate-limit denial into `web_conduit_rate_limited` instead of generic `web_conduit_policy_denied`.
- `failure_class_targeted`: Tavily could be locally denied by the web-conduit rate limiter, but diagnostics saw only a generic policy denial, making the primitive bottleneck harder to separate from provider config or content quality.
- `hypothesis`: If policy-denied search payloads preserve the specific rate-limit reason, web tooling gates can isolate rate/capacity failures from provider/data-quality failures.
- `files_changed`: `core/layer0/ops/src/web_conduit_parts/050-search-providers_parts/004-segment.rs`, `core/layer0/ops/src/web_conduit_parts/080-tests_parts/010-mod-tests_parts/010-status-and-provider-catalog-tests.rs`
- `proof_tests`: `cargo test --manifest-path core/layer0/ops/Cargo.toml search_payload_error_preserves_policy_rate_limit_boundary -- --nocapture` passed; `cargo build --manifest-path core/layer0/ops/Cargo.toml --bin infring-ops` passed.
- `eval_artifact`: `core/local/artifacts/web_tooling_user_pool_seed3686_after_rate_limit_boundary.json`
- `after_metrics`: same seeded 5-case live tooling lane passed `5/5`; all web-tooling gates were `1.0`; transport failures were `0`.
- `decision`: `kept`
- `reason`: Provider state became clean enough to compare, and the rate-limit boundary is now explicit instead of hidden behind a generic policy error.
- `follow_up`: Manual soft inspection showed the hard metrics were too green: the April science case passed even though most claim material was title/byline, teaser, prediction, or dangling-fragment text. The next primitive target is evidence-quality grading, not provider supply.

### 2026-06-02: direct tool-quality contract conflict guard

- `baseline_artifact`: `core/local/artifacts/web_tooling_user_pool_seed3686_after_rate_limit_boundary.json`
- `patch_name`: treat a direct usable evidence pack as conflicting when the direct tool-quality flags say claim/content/materialized evidence is missing.
- `failure_class_targeted`: A direct `evidence_pack_quality.status=usable` object could override the tool's own lower-level quality flags and allow excellent/evidence-ready scoring even when the tool said claim hints, content-rich evidence, or materialized evidence were missing.
- `hypothesis`: If direct pack readiness conflicts with direct quality flags, retrieval quality should surface `conflicting_provider_state` instead of rewarding low-evidence fallback.
- `files_changed`: `orchestration/src/eval_research_golden_scoring_parts/070_retrieval_quality.rs`, `orchestration/src/eval_research_golden_scoring_parts/090_tests_retrieval_and_query.rs`
- `proof_tests`: `cargo test --manifest-path orchestration/Cargo.toml direct_usable_pack_conflicts_with_missing_quality_flags -- --nocapture` passed.
- `replay_artifact`: `core/local/artifacts/evidence_quality_grade_seed3686_after_contract_conflict.json`
- `after_metrics`: evidence-quality replay still reported `5/5` ready, which proved the evidence-quality grader itself was still over-crediting weak claim text.
- `decision`: `kept_measurement_boundary_positive`
- `reason`: This closed one contradiction in retrieval-quality scoring, but it intentionally did not solve the separate evidence-quality claim-text problem.
- `follow_up`: Patch evidence-quality itself so weak claim strings cannot count as answer-ready evidence merely because the packet has source URLs and usable aggregate counters.

### 2026-06-02: evidence-quality shell-claim guard

- `baseline_artifact`: `core/local/artifacts/evidence_quality_grade_seed3686_after_contract_conflict.json`
- `patch_name`: include root-level tool query metadata in evidence-quality request terms and mark packet rows low-quality when their claim material is only a title/byline shell, editorial teaser/projection shell, or dangling claim fragment.
- `failure_class_targeted`: Evidence-quality grading could mark weak source fragments as answerable evidence: title/byline/date shells, `look out for` teaser text, unattributed future projections, and truncated claims ending in tails such as `based on data`.
- `hypothesis`: If packet readiness requires clean claim text, the metric should stop calling visibly weak evidence packets ready while keeping clean source-backed claims passable.
- `files_changed`: `orchestration/src/eval_web_retrieval_gate_diagnostics_parts/050_evidence_quality.rs`, `orchestration/src/eval_web_retrieval_gate_diagnostics_parts/091_tests_claims_and_quality.rs`
- `proof_tests`: `cargo test --manifest-path orchestration/Cargo.toml evidence_quality_gates_reject_shell_claim_packets_despite_usable_pack_counts -- --nocapture` passed.
- `replay_command`: `cargo run --quiet --manifest-path orchestration/Cargo.toml --bin eval_runtime -- evidence-quality-grade --strict=0 --input core/local/artifacts/web_tooling_user_pool_seed3686_after_rate_limit_boundary.json --out core/local/artifacts/evidence_quality_grade_seed3686_after_claim_shell_guard.json --out-latest artifacts/evidence_quality_grade_seed3686_after_claim_shell_guard_latest.json --out-markdown local/workspace/reports/EVIDENCE_QUALITY_GRADE_SEED3686_AFTER_CLAIM_SHELL_GUARD.md`
- `before_metrics`: same saved 5-case evidence-quality replay reported `5/5` ready, including the manually bad April science case.
- `after_metrics`: replay reported `4/5` ready. The April science case now fails `claim_quality_ready`, `answerability_ready`, `evidence_packet_contract_ready`, and `bounded_answerability_ready`; `low_quality_claim_count=8/10`; `low_quality_flags=["low_quality_claim_text"]`.
- `decision`: `kept`
- `reason`: This is an obvious positive measurement correction: the exact case that looked bad to a human is no longer graded as ready, and the patch is structural rather than query/domain-specific.
- `follow_up`: `source_quality_ready` still reports `5/5` on the replay because usable pack aggregate counters can still override observed low-quality packet rows. The next primitive calibration should make `source_quality_pack_ready` respect low-quality observed rows inside the actual evidence packet, while still allowing clean packets to survive noisy auxiliary rows.

### 2026-06-02: source-quality packet-observation guard

- `baseline_artifact`: `core/local/artifacts/evidence_quality_grade_seed3686_after_claim_shell_guard.json`
- `patch_name`: require `source_quality_pack_ready` to respect the observed `evidence_pack` contract ready rate instead of trusting aggregate pack counters alone.
- `failure_class_targeted`: Source quality could still pass when aggregate `evidence_pack_quality` counters said usable even though the actual packet rows failed claim/packet readiness.
- `hypothesis`: If source-quality pack readiness requires at least half of the actual packet rows to be contract-ready, source quality should agree with claim quality and answerability on visibly weak packets while clean packets remain protected from noisy auxiliary rows.
- `files_changed`: `orchestration/src/eval_web_retrieval_gate_diagnostics_parts/050_evidence_quality.rs`, `orchestration/src/eval_web_retrieval_gate_diagnostics_parts/091_tests_claims_and_quality.rs`
- `proof_tests`: `cargo test --manifest-path orchestration/Cargo.toml evidence_quality_ -- --nocapture` passed with `4/4`; `cargo test --manifest-path orchestration/Cargo.toml source_quality_ -- --nocapture` passed with `4/4`.
- `replay_command`: `cargo run --quiet --manifest-path orchestration/Cargo.toml --bin eval_runtime -- evidence-quality-grade --strict=0 --input core/local/artifacts/web_tooling_user_pool_seed3686_after_rate_limit_boundary.json --out core/local/artifacts/evidence_quality_grade_seed3686_after_pack_observed_ready.json --out-latest artifacts/evidence_quality_grade_seed3686_after_pack_observed_ready_latest.json --out-markdown local/workspace/reports/EVIDENCE_QUALITY_GRADE_SEED3686_AFTER_PACK_OBSERVED_READY.md`
- `before_metrics`: same saved 5-case replay had `ready_cases=4/5`, but `source_quality_ready=5/5`; the April science case failed claim/answerability/packet gates while still passing source quality.
- `after_metrics`: replay stayed `ready_cases=4/5`; `source_quality_ready` now reports `4/5`; the April science case has `source_quality_ready=false`, `source_quality_pack_ready=false`, and `source_quality_pack_observed_packet_ready=false`.
- `decision`: `kept`
- `reason`: This fixed a metric contradiction without changing the final ready count or weakening the clean-pack/noisy-auxiliary protection.
- `follow_up`: The evidence-quality metrics are now internally aligned for this replay. The next step should return to a live/random tooling or workflow batch and inspect whether the top remaining failures are evidence-supply quality, evidence-to-answer synthesis, or soft user-facing answer quality.

### 2026-06-02: rejected anchored source-quality recovery in execution-limited lane

- `baseline_artifact`: `core/local/artifacts/web_tooling_user_pool_random10_after_recent_evidence_patch.json`
- `patch_name`: prefer anchored second-pass source-quality queries over deferred submitted lanes when an execution-limited first wave lacks source quality.
- `failure_class_targeted`: A source-sensitive case could have usable rows but weak authority/diversity, suggesting the recovery lane should seek stronger source backing instead of replaying deferred fragments.
- `hypothesis`: If source-quality recovery generated anchored source-backed/primary/official queries from the full user request, the same seeded 10-case run would move from `9/10` to at least `10/10` or clearly improve the failed source-sensitive case.
- `files_changed_then_reverted`: `core/layer0/ops/src/batch_query_primitive_parts/018-request-and-cache.rs`, `core/layer0/ops/src/batch_query_primitive_parts/020-pipeline.combined_parts/040-api-batch-query_parts/000-combined.rs`, `core/layer0/ops/src/batch_query_primitive_parts/042-cache-rewrite-tests.rs`
- `proof_tests_before_revert`: `cargo test --manifest-path core/layer0/ops/Cargo.toml source_quality_recovery_prefers_anchored_source_queries_before_deferred_lanes -- --nocapture` passed; `cargo test --manifest-path core/layer0/ops/Cargo.toml single_subject_multi_facet_query_plan_frontloads_subject_facet_lanes -- --nocapture` passed.
- `eval_artifact`: `core/local/artifacts/web_tooling_user_pool_random10_after_source_quality_recovery_patch_fixed_seed.json`
- `after_metrics`: same fixed seed stayed `9/10`; weakest gates remained `web_5g_answerability_ready=9/10` and `web_7_usable_evidence_available=9/10`.
- `diagnosis`: The patch did not actually execute in the live artifact because `second_pass_recovery.used=false` for the observed cases. The apparent creatine-case improvement came from provider/result variability, not from the new source-quality branch. The new failing case was `research_pool_100_meeting_overload_remote_teams`, where one requested facet had only weak/candidate-only support.
- `decision`: `reverted`
- `reason`: No obvious positive measured effect on the fixed seed, and the changed branch was not used by the run. Keeping it would violate the one-change measurement rule.
- `follow_up`: Target weak facet recovery. Current coverage-gap recovery only generates queries for completely missing facets; it should also recover facets that are present only as weak/candidate-only evidence.

### 2026-06-02: rejected pack-ready weak-facet coverage recovery

- `baseline_artifact`: `core/local/artifacts/web_tooling_user_pool_random10_after_recent_evidence_patch.json`
- `patch_name`: make coverage-gap recovery compute missing facets from pack-ready candidates instead of broad candidate-level usable rows.
- `failure_class_targeted`: A requested facet could have candidate-only/weak evidence and suppress recovery, even though synthesis did not receive pack-ready support for that facet.
- `hypothesis`: If recovery considered weak/candidate-only facet coverage a real gap, the same fixed 10-case seed would recover `research_pool_100_meeting_overload_remote_teams` and move the batch above `9/10`.
- `files_changed_then_reverted`: `core/layer2/ops/src/retrieval_policy_parts/010-freshness-and-relevance.rs`
- `proof_tests_before_revert`: `cargo test --manifest-path core/layer2/ops/Cargo.toml coverage_gap_recovery_targets_facets_without_pack_ready_evidence -- --nocapture` passed; `cargo test --manifest-path core/layer2/ops/Cargo.toml evidence_packet_promotion_tests -- --nocapture` passed with `24/24`.
- `eval_artifact`: `core/local/artifacts/web_tooling_user_pool_random10_after_weak_facet_recovery_patch_fixed_seed.json`
- `after_metrics`: same fixed seed stayed `9/10`, but the failure shape worsened. `research_pool_100_meeting_overload_remote_teams` went from thin/partial evidence to `status=no_results`, `evidence_pack_quality.status=absent`, and first failed at `web_3d_browser_materialization_not_failed`; weak gates expanded from two downstream evidence gates to materialization, packaging, claim, source-quality, answerability, provider, and usable-evidence gates.
- `diagnosis`: The patch did make `second_pass_recovery.used=true` with `reason=coverage_gap`, but the generated recovery lane was a long concatenation of the full query plus a facet phrase. That extra lane did not improve candidate material and coincided with no pack-ready evidence for the case.
- `decision`: `reverted`
- `reason`: It targeted a real weakness, but the implementation was too blunt and produced a worse live artifact without improving pass rate.
- `follow_up`: Revisit weak-facet recovery only with safer query construction: prefer already-submitted deferred facet lanes such as `remote teams source-backed evidence` / `meeting overload independent analysis`, or generate short anchored subject+facet lanes, not full-query concatenations. Add a deterministic unit test for query shape before another live run.

### 2026-06-02: leading fragment claim guard

- `baseline_artifact`: `core/local/artifacts/web_tooling_user_pool_random3_after_provider_budget_boundary.json`
- `patch_name`: reject leading sentence-fragment claim hints before they can count as synthesis-safe answer material.
- `failure_class_targeted`: Live evidence packets could contain useful sources but promote clipped answer fragments such as `, 3-5 g/day...` or `. Japanese-American...` as concrete claims, making metrics look clean while the user-facing answer risked sounding like pasted snippets.
- `hypothesis`: If leading punctuation and continuation-glue claims are not synthesis-safe, extraction will prefer cleaner sentence claims when available and evidence-quality replay will expose fragment debt instead of reporting zero weak claim material.
- `files_changed`: `core/layer2/ops/src/retrieval_policy_parts/010-freshness-and-relevance.rs`, `core/layer2/ops/src/retrieval_policy_parts/911-relevance-tests.rs`, `orchestration/src/eval_web_retrieval_gate_diagnostics_parts/050_evidence_quality.rs`, `orchestration/src/eval_web_retrieval_gate_diagnostics_parts/091_tests_claims_and_quality.rs`
- `proof_tests`: `cargo test --manifest-path core/layer2/ops/Cargo.toml claim_text_is_synthesis_safe_rejects_leading_dangling_fragments -- --nocapture` passed; `cargo test --manifest-path core/layer2/ops/Cargo.toml claim_extraction_skips_clipped_metric_fragments_when_clean_claim_exists -- --nocapture` passed; `cargo test --manifest-path orchestration/Cargo.toml evidence_quality_gates_reject_leading_dangling_claim_fragments -- --nocapture` passed.
- `replay_command`: `cargo run --quiet --manifest-path orchestration/Cargo.toml --bin eval_runtime -- evidence-quality-grade --strict=0 --input core/local/artifacts/web_tooling_user_pool_random3_after_provider_budget_boundary.json --out core/local/artifacts/evidence_quality_grade_random3_after_fragment_guard.json --out-latest artifacts/evidence_quality_grade_random3_after_fragment_guard_latest.json --out-markdown local/workspace/reports/EVIDENCE_QUALITY_GRADE_RANDOM3_AFTER_FRAGMENT_GUARD.md`
- `before_metrics`: same 3-case evidence-quality replay reported `3/3` ready and `low_quality_claim_count=0` on all cases, despite manual inspection finding clipped claim fragments in the creatine packet.
- `after_metrics`: replay still reports `3/3` ready because enough clean evidence remains, but the creatine case now reports `low_quality_claim_count=4`, `low_quality_claim_rate=0.4`, `concrete_claim_rate=0.6`, `low_quality_flags=["low_quality_claim_text"]`, and sample rows mark the clipped claim packets as not packet-ready.
- `decision`: `kept`
- `reason`: This is a positive metric alignment patch and a production extraction guard. It does not hardcode a query domain; it prevents any topic from treating leading punctuation/continuation fragments as clean answer claims.
- `follow_up`: Restart or rebuild the live tooling service before the next live run so the production extraction change is reflected in fresh `/api/batch-query` outputs, then use a small random web-tooling sample to confirm fragment counts fall in newly generated evidence packets.

### 2026-06-02: stitched claim-tail guard

- `baseline_artifact`: `core/local/artifacts/web_tooling_user_pool_random3_after_fragment_guard_fresh.json`
- `patch_name`: reject stitched/truncated claim strings with single-letter dangling tails, title-comparison tails, and continuation-like starts.
- `failure_class_targeted`: Fresh evidence packets no longer leaked leading punctuation fragments, but still promoted stitched claim text such as `There is also a fair question of whether you n ... f`, `Its weakness is that it doesn’t ... Comparison`, and image-alt/continuation text beginning with `representing ...`.
- `hypothesis`: If these generic stitched-tail artifacts are not synthesis-safe, fresh extraction should stop promoting them, and evidence-quality replay should stop counting them as clean claim material.
- `files_changed`: `core/layer2/ops/src/retrieval_policy_parts/010-freshness-and-relevance.rs`, `orchestration/src/eval_web_retrieval_gate_diagnostics_parts/050_evidence_quality.rs`, `orchestration/src/eval_web_retrieval_gate_diagnostics_parts/091_tests_claims_and_quality.rs`
- `proof_tests`: `cargo test --manifest-path core/layer2/ops/Cargo.toml claim_text_is_synthesis_safe_rejects_leading_dangling_fragments -- --nocapture` passed; `cargo test --manifest-path core/layer2/ops/Cargo.toml claim_extraction_skips_clipped_metric_fragments_when_clean_claim_exists -- --nocapture` passed; `cargo test --manifest-path orchestration/Cargo.toml evidence_quality_gates_reject_stitched_title_tail_claim_fragments -- --nocapture` passed.
- `replay_command`: `cargo run --quiet --manifest-path orchestration/Cargo.toml --bin eval_runtime -- evidence-quality-grade --strict=0 --input core/local/artifacts/web_tooling_user_pool_random3_after_fragment_guard_fresh.json --out core/local/artifacts/evidence_quality_grade_random3_after_stitched_claim_guard.json --out-latest artifacts/evidence_quality_grade_random3_after_stitched_claim_guard_latest.json --out-markdown local/workspace/reports/EVIDENCE_QUALITY_GRADE_RANDOM3_AFTER_STITCHED_CLAIM_GUARD.md`
- `before_metrics`: fresh 3-case evidence-quality replay reported `3/3` ready, `avg_concrete_claim_rate=0.9139`, `avg_low_quality_evidence_rate=0.0718`; docs-portal only surfaced `low_quality_claim_count=2` even though manual inspection found multiple stitched fragments.
- `after_metrics`: replay still reports `3/3` ready because enough clean evidence remains, but `avg_concrete_claim_rate` drops to `0.7000`, `avg_low_quality_evidence_rate` rises to `0.1690`, docs-portal now reports `low_quality_claim_count=6`, and the e-reader image-alt fragment now reports `low_quality_claim_count=2`.
- `decision`: `kept`
- `reason`: This is a metric alignment and production extraction improvement. It is not query-specific; it rejects generic malformed answer-claim shapes that can appear for any topic.
- `service_restart`: Dashboard/API was restarted after the production patch. A stale orphaned dashboard child had to be killed with `kill -9` before the watchdog stabilized a fresh listener. Final health probe returned `status=healthy` on `http://127.0.0.1:4173/api/health`.
- `follow_up`: Run one fresh live sample after this restart only if we can afford the provider spend; otherwise continue with deterministic extraction tests and offline evidence-quality replay. The next metric question is whether `claim_quality_ready` should remain true when a case is answerable but has a high low-quality-claim rate, or whether downstream synthesis should receive only the clean claims.

### 2026-06-02: evidence-claim handoff quality split

- `baseline_artifact`: `core/local/artifacts/web_tooling_user_pool_random3_after_fragment_guard_fresh.json`
- `patch_name`: split raw packet claim quality from promoted `evidence_claims` handoff quality.
- `failure_class_targeted`: Broad evidence-quality gates could pass even when malformed claim strings were present in the material that synthesis might consume. The prior metrics did not clearly distinguish raw `evidence_pack.claim_hints` noise from promoted `evidence_claims` noise.
- `hypothesis`: If the diagnostics separately count promoted handoff claims, we can tell whether bad snippets are diagnostic-only source-packet debt or actual synthesis-input debt, and we can target the next patch without guessing.
- `files_changed`: `core/layer2/ops/src/retrieval_policy_parts/010-freshness-and-relevance.rs`, `orchestration/src/eval_web_retrieval_gate_diagnostics_parts/050_evidence_quality.rs`, `orchestration/src/eval_web_retrieval_gate_diagnostics_parts/031_case_metrics.rs`, `orchestration/src/eval_web_retrieval_gate_diagnostics_parts/032_aggregate_readouts.rs`, `orchestration/src/eval_web_retrieval_gate_diagnostics_parts/021_measurement_report.rs`, `orchestration/src/eval_web_retrieval_gate_diagnostics_parts/091_tests_claims_and_quality.rs`, `orchestration/src/eval_evidence_quality_grade.rs`
- `proof_tests`: `cargo test --manifest-path core/layer2/ops/Cargo.toml evidence_claim_promotion_filters_stitched_claim_fragments -- --nocapture` passed; `cargo test --manifest-path orchestration/Cargo.toml evidence_quality_distinguishes_packet_noise_from_clean_handoff_claims -- --nocapture` passed; `git diff --check` passed.
- `replay_command`: `cargo run --quiet --manifest-path orchestration/Cargo.toml --bin eval_runtime -- evidence-quality-grade --strict=0 --input core/local/artifacts/web_tooling_user_pool_random3_after_fragment_guard_fresh.json --out core/local/artifacts/evidence_quality_grade_random3_after_handoff_claim_split.json --out-latest artifacts/evidence_quality_grade_random3_after_handoff_claim_split_latest.json --out-markdown local/workspace/reports/EVIDENCE_QUALITY_GRADE_RANDOM3_AFTER_HANDOFF_CLAIM_SPLIT.md`
- `after_metrics`: offline replay still reports `ready_cases=3/3`, but the new handoff lane reports `avg_promoted_claims_per_case=6.667`, `handoff_concrete_claim_rate=0.708`, `handoff_low_quality_claim_rate=0.292`, and `handoff_citation_ready_claim_rate=0.708`. All three replayed cases have `handoff_claim_quality_ready=false`.
- `decision`: `kept`
- `reason`: This is a measurement-alignment patch, not a domain/query special case. It exposes the remaining failure more precisely: stale artifacts contain bad promoted evidence claims even when broad evidence gates pass.
- `follow_up`: Restart/rebuild the live tooling service after the production extraction guard, then run a small fresh sample. If fresh `handoff_low_quality_claim_rate` remains above zero, target the evidence-claim promotion path; if it drops, move downstream to final synthesis quality.

### 2026-06-02: nonblocking provider degradation with usable direct evidence

- `baseline_artifact`: `core/local/artifacts/web_tooling_user_pool_random3_after_handoff_split_fresh.json`
- `patch_name`: classify stale missing-evidence flags against measured direct-pack counts before marking a provider-state conflict.
- `failure_class_targeted`: A fresh live case had clean, citable promoted evidence claims but still failed `web_6_provider_not_empty_or_degraded` and `web_7_usable_evidence_available` because provider/quota telemetry and stale missing-evidence flags overrode the usable direct evidence contract.
- `hypothesis`: If degraded-provider telemetry stays visible but only blocks supply when candidate/evidence/materialization counts are actually absent, then clean direct evidence packets will remain usable while truly contradictory packets still fail.
- `files_changed`: `orchestration/src/eval_research_golden_scoring_parts/070_retrieval_quality.rs`, `orchestration/src/eval_research_golden_scoring_parts/090_tests_retrieval_and_query.rs`
- `proof_tests`: `cargo test --manifest-path orchestration/Cargo.toml direct_usable_pack_overrides_stale_missing_flags_when_counts_are_present -- --nocapture` passed; `cargo test --manifest-path orchestration/Cargo.toml direct_usable_pack_conflicts_with_missing_quality_flags -- --nocapture` passed.
- `before_metrics`: fresh 3-case web-tooling sample had `success_rate=0.667`; the failed science case had `candidate_count=76`, `evidence_count=36`, `content_rich_candidate_count=15`, `direct_evidence_claim_count=6`, clean handoff claims, but `retrieval_status=conflicting_provider_state` and `usable_evidence=false`.
- `after_metrics`: targeted scorer tests show the fresh-run shape now classifies as `status=usable`, `usable_evidence=true`, `provider_degradation_nonblocking=true`, and no `direct_tool_quality_contract_conflict`; the hard contradiction fixture still classifies as `conflicting_provider_state`. The same seeded live 3-case web-tooling batch now reports `success_rate=1.0`, every web gate `3/3`, and the science case now has `retrieval_status=usable`, `usable_evidence=true`, `direct_evidence_claim_count=6`, `answerability_ready=true`, and `handoff_low_quality_claim_rate=0.0`.
- `evidence_quality_replay`: `core/local/artifacts/evidence_quality_grade_random3_after_provider_degradation_classification_patch.json` reports `ready_cases=3/3`, `pass_rate=1.0`, `handoff_low_quality_claim_rate=0.0`, `handoff_concrete_claim_rate=1.0`, and `handoff_citation_ready_claim_rate=1.0`.
- `decision`: `kept`
- `reason`: This is a generic classification fix. It does not ignore provider degradation; it prevents stale/blocker telemetry from erasing evidence that synthesis can actually use.
- `follow_up`: Move from web-tooling packet classification back to downstream answer quality: run a small workflow/perfect-evidence or full research batch and inspect whether clean evidence now becomes useful user-facing prose.

### 2026-06-02: local area-selection scope entity calibration

- `baseline_artifact`: `core/local/artifacts/research_perfect_evidence_replay_random5_after_provider_degradation_classification_patch.json`
- `patch_name`: do not require broad all-caps local scope entities as literal answer coverage in neighborhood/area-selection prompts.
- `failure_class_targeted`: A visibly useful NYC neighborhood answer hard-failed because `NYC` was treated as an uncovered required entity even though the answer stayed on the neighborhood-selection task and covered the concrete `Times Square` exclusion.
- `hypothesis`: If broad local scope acronyms are treated as prompt scope rather than mandatory answer entities for area-selection prompts, then useful local recommendation answers will not hard-fail while concrete constraints remain enforced.
- `files_changed`: `orchestration/src/eval_research_golden_scoring_parts/060_prompt_entities.rs`, `orchestration/src/eval_research_golden_scoring_parts/090_tests_retrieval_and_query.rs`
- `proof_tests`: `cargo test --manifest-path orchestration/Cargo.toml entity_coverage_accepts_named_place_in_negative_constraint -- --nocapture` passed; `cargo test --manifest-path orchestration/Cargo.toml user_stated_entities_remain_query_scope -- --nocapture` passed; `cargo test --manifest-path orchestration/Cargo.toml hidden_fixture_entities_do_not_hard_fail_broad_discovery_prompts -- --nocapture` passed.
- `before_metrics`: random 5-case downstream perfect-evidence replay had `pass_rate=0.8`, `excellent_rate=0.6`, `synthesis_hard_failures=1`, and `synthesized_sounds_good_rate=1.0`; the hard failure was `perfect_ev_078_nyc_first_time_neighborhoods` with `entity_coverage_low:0.50`.
- `after_metrics`: offline regrade of the same saved responses reports `pass_rate=1.0`, `excellent_rate=0.8`, `synthesis_hard_failures=0`, and `synthesized_sounds_good_rate=1.0`.
- `decision`: `kept`
- `reason`: This is a grader-calibration fix for a reusable local-scope failure class. It does not add a city-specific alias and still requires concrete named constraints like `Times Square`.
- `follow_up`: Target the remaining Excellent false negative in the same regrade: answer-unit traceability treats scaffold wording such as `Named the strongest...` as an unsupported concrete term.

### 2026-06-02: answer-trace scaffold term `named`

- `baseline_artifact`: `core/local/artifacts/research_perfect_evidence_regrade_random5_after_local_scope_entity_patch.json`
- `patch_name`: ignore `named` as answer-unit traceability scaffold.
- `failure_class_targeted`: A useful shortlist answer missed Excellent because the answer-unit alignment lane treated the reporting verb `named` in `Named the strongest high-end pick...` as an unsupported concrete claim term.
- `hypothesis`: If `named` is excluded as generic reporting/scaffold language, concrete product/model terms remain traceable while the answer does not fail on wording that is not itself a factual entity.
- `files_changed`: `orchestration/src/eval_research_golden_scoring_parts/051_answer_alignment.rs`, `orchestration/src/eval_research_golden_scoring_parts/092_tests_relevance_and_citations.rs`
- `proof_tests`: `cargo test --manifest-path orchestration/Cargo.toml answer_alignment_ignores_checklist_and_interface_scaffold_terms -- --nocapture` passed.
- `before_metrics`: after local-scope calibration, the same fixed five saved responses had `pass_rate=1.0`, `excellent_rate=0.8`, `synthesis_hard_failures=0`, and `synthesized_sounds_good_rate=1.0`; the only Excellent blocker was `answer_units_not_traceable_to_evidence` caused by unsupported term `named`.
- `after_metrics`: offline regrade of the same saved responses reports `pass_rate=1.0`, `excellent_rate=1.0`, `synthesis_hard_failures=0`, and `synthesized_sounds_good_rate=1.0`.
- `fresh_validation`: a new random 5-case downstream perfect-evidence replay with seed `random:be7140c0a83f3e75` reports `pass_rate=1.0`, `excellent_rate=1.0`, `synthesized_sounds_good_rate=1.0`, `synthesis_hard_failures=0`, and `test_input_lane_leaks=0`.
- `decision`: `kept`
- `reason`: This is a generic answer-alignment calibration fix. It removes a false unsupported-term signal without relaxing concrete model/product traceability.
- `follow_up`: With downstream perfect-evidence looking clean on two 5-case samples, the next decision is whether to scale this lane to 20 cases or return to live full research where retrieval and synthesis are exercised together.

### 2026-06-02: privacy-law fixture specificity repair

- `baseline_artifact`: `core/local/artifacts/research_perfect_evidence_regrade_random20_after_promising_approaches_patch.json`
- `patch_name`: add named state privacy laws to the synthetic perfect-evidence packet.
- `failure_class_targeted`: A "perfect evidence" fixture asked the answer to identify which state laws matter, but the evidence only described state-by-state privacy compliance at a broad level. The model correctly produced a bounded gap answer, which kept the case from earning Excellent.
- `hypothesis`: If the fixture supplies named laws for a prompt that asks for named laws, then the downstream answer can produce a direct, specific response without changing workflow behavior or grader rules.
- `files_changed`: `validation/evals/fixtures/research_perfect_evidence_dataset_v1.json`
- `proof_tests`: `jq empty validation/evals/fixtures/research_perfect_evidence_dataset_v1.json` passed.
- `eval_command`: `cargo run --quiet --manifest-path orchestration/Cargo.toml --bin eval_runtime -- research-perfect-evidence --mode=test-input-replay --strict=0 --sample-size=20 --sample-seed=random:c8dddc0d5909232c --limit=20 --timeout-seconds=90 --out=core/local/artifacts/research_perfect_evidence_replay_random20_after_privacy_fixture_patch_fixed_seed.json --out-latest=artifacts/research_perfect_evidence_replay_random20_after_privacy_fixture_patch_fixed_seed_latest.json --out-markdown=local/workspace/reports/RESEARCH_PERFECT_EVIDENCE_REPLAY_RANDOM20_AFTER_PRIVACY_FIXTURE_PATCH_FIXED_SEED.md --responses-out=core/local/artifacts/research_perfect_evidence_responses_random20_after_privacy_fixture_patch_fixed_seed.json`
- `before_metrics`: fixed-seed 20-case regrade after the promising-approaches patch had `pass_rate=1.0`, `excellent_rate=0.95`, `synthesized_sounds_good_rate=1.0`; the only non-Excellent case was `perfect_ev_012_us_privacy_laws_2026` with `requested_specificity_not_excellent_ready` because evidence lacked named statutes or jurisdictions.
- `after_metrics`: fresh fixed-seed replay moved `perfect_ev_012_us_privacy_laws_2026` to `excellent=true`, `score=100`. The batch still reported `pass_rate=1.0`, `excellent_rate=0.95`, and `synthesized_sounds_good_rate=1.0` because a different generated answer exposed a separate scaffold-term false negative.
- `visible_output_delta`: the privacy answer now has enough evidence to name California CCPA/CPRA, Colorado CPA, Virginia CDPA, Connecticut CTDPA, Texas TDPSA, and Oregon Consumer Privacy Act instead of saying the evidence lacks named jurisdictions.
- `decision`: `kept`
- `reason`: This repairs the test input lane rather than the workflow. A perfect-evidence fixture should be answer-ready for the specificity its prompt requests.
- `follow_up`: Patch the separate robot-vacuum traceability miss only if inspection shows a generic grader false negative rather than an unsupported answer.

### 2026-06-02: answer-trace scaffold term `why`

- `baseline_artifact`: `core/local/artifacts/research_perfect_evidence_replay_random20_after_privacy_fixture_patch_fixed_seed.json`
- `patch_name`: ignore `why` as answer-unit traceability scaffold.
- `failure_class_targeted`: A useful shortlist answer missed Excellent because the answer-unit alignment lane treated the label word `why` in `Why it's on the list` as an unsupported concrete claim term.
- `hypothesis`: If `why` is excluded as generic explanatory label language, concrete product/model terms remain traceable while the answer does not fail on formatting text.
- `files_changed`: `orchestration/src/eval_research_golden_scoring_parts/051_answer_alignment.rs`, `orchestration/src/eval_research_golden_scoring_parts/092_tests_relevance_and_citations.rs`
- `proof_tests`: `cargo test --manifest-path orchestration/Cargo.toml answer_alignment_ignores_checklist_and_interface_scaffold_terms -- --nocapture` passed.
- `eval_command`: `cargo run --quiet --manifest-path orchestration/Cargo.toml --bin eval_runtime -- research-perfect-evidence --mode=test-input-regrade --strict=0 --limit=20 --responses=core/local/artifacts/research_perfect_evidence_responses_random20_after_privacy_fixture_patch_fixed_seed.json --out=core/local/artifacts/research_perfect_evidence_regrade_random20_after_why_scaffold_patch.json --out-latest=artifacts/research_perfect_evidence_regrade_random20_after_why_scaffold_patch_latest.json --out-markdown=local/workspace/reports/RESEARCH_PERFECT_EVIDENCE_REGRADE_RANDOM20_AFTER_WHY_SCAFFOLD_PATCH.md`
- `before_metrics`: fixed-seed 20-case replay after the privacy fixture repair had `pass_rate=1.0`, `excellent_rate=0.95`, `synthesized_sounds_good_rate=1.0`; the only non-Excellent case was `perfect_ev_040_robot_vacuum_shortlist` with `answer_units_not_traceable_to_evidence` caused solely by unsupported term `why`.
- `after_metrics`: offline regrade of the same saved responses reports `pass_rate=1.0`, `excellent_rate=1.0`, `synthesized_sounds_good_rate=1.0`, `synthesis_hard_failures=0`, `test_input_lane_leaks=0`, and `average_score=100.0`.
- `visible_output_delta`: no answer-generation change; the same useful product shortlist is no longer mislabeled as unsupported because of a presentation label.
- `decision`: `kept`
- `reason`: This is a generic scaffold-term calibration, not a product-specific patch. It preserves concrete evidence traceability while removing non-claim label text from the term set.
- `follow_up`: Run a fresh random 20-case downstream replay without a fixed seed to test whether the good-evidence-to-good-answer lane stays near 20/20 beyond this known sample; if it does, shift back upstream to native evidence/tooling quality.

### 2026-06-02: Unicode dash and generic offer-word normalization

- `baseline_artifact`: `core/local/artifacts/research_perfect_evidence_replay_random20_after_why_scaffold_patch_fresh.json`
- `patch_name`: normalize Unicode dash variants for compare text and ignore generic `offer/offers` scaffold verbs in answer alignment.
- `failure_class_targeted`: Fresh random downstream replay had 20/20 human-sounding answers but one hard false failure from `GLP‑1` using a non-ASCII hyphen and one Excellent false negative from `offers` being treated as an unsupported concrete claim term.
- `hypothesis`: If compare normalization treats dash variants equivalently and answer alignment ignores generic offering/presentation verbs, useful evidence-backed answers should no longer fail on typography or non-claim verbs.
- `files_changed`: `orchestration/src/eval_research_golden_utils.rs`, `orchestration/src/eval_research_golden_scoring_parts/051_answer_alignment.rs`, `orchestration/src/eval_research_golden_scoring_parts/090_tests_retrieval_and_query.rs`, `orchestration/src/eval_research_golden_scoring_parts/092_tests_relevance_and_citations.rs`
- `proof_tests`: `cargo test --manifest-path orchestration/Cargo.toml entity_coverage_accepts_unicode_dash_variants -- --nocapture` passed; `cargo test --manifest-path orchestration/Cargo.toml answer_alignment_ignores_checklist_and_interface_scaffold_terms -- --nocapture` passed.
- `eval_command`: `cargo run --quiet --manifest-path orchestration/Cargo.toml --bin eval_runtime -- research-perfect-evidence --mode=test-input-regrade --strict=0 --limit=20 --responses=core/local/artifacts/research_perfect_evidence_responses_random20_after_why_scaffold_patch_fresh.json --out=core/local/artifacts/research_perfect_evidence_regrade_random20_after_dash_offer_patch.json --out-latest=artifacts/research_perfect_evidence_regrade_random20_after_dash_offer_patch_latest.json --out-markdown=local/workspace/reports/RESEARCH_PERFECT_EVIDENCE_REGRADE_RANDOM20_AFTER_DASH_OFFER_PATCH.md`
- `before_metrics`: fresh random 20-case replay with seed `random:b05be55abec8eae9` had `pass_rate=0.95`, `excellent_rate=0.9`, `synthesized_sounds_good_rate=1.0`; hard failure was `perfect_ev_022_glp1_weight_management` on `entity_coverage_low:0.00` despite the answer visibly covering GLP-1; Excellent miss was `perfect_ev_018_tokyo_first_time_bases` from unsupported term `offers`.
- `after_metrics`: offline regrade of the same saved responses reports `pass_rate=1.0`, `excellent_rate=1.0`, `synthesized_sounds_good_rate=1.0`, `synthesis_hard_failures=0`, `test_input_lane_leaks=0`, and `average_score=99.8`.
- `visible_output_delta`: no answer-generation change; the same saved answers are now graded according to their visible usefulness and evidence support.
- `decision`: `kept`
- `reason`: This is a broad text-normalization and scaffold-term calibration. It is not query-specific and should reduce false negatives across medical/product names and recommendation prose.
- `follow_up`: Downstream perfect-evidence now looks clean on a fixed 20-case seed and a fresh random 20-case seed after regrade. The next highest-ROI step is to return to upstream native evidence/tooling quality, because the good-evidence-to-good-answer lane is no longer the dominant blocker.
