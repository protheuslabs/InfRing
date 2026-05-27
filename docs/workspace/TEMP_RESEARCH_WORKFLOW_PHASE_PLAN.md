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

Always use this loop:

1. Run a small batch.
2. Pull disagreement cases.
3. Review them with human judgment.
4. Patch one failure class.
5. Re-run a small batch.
6. Only then run the larger seeded batch.

Current focus:

- We are in Phase 1.
- We have a first provisional five-case review set recorded so we can bootstrap calibration without waiting on more live runs.
- The provisional judge-human run shows a strong disagreement signal: exact agreement is 0.20, and the dominant mismatch is that bounded/thin answers are being overcalled as `correct` instead of `partial` or `incorrect`.
- The next action is to collect more human labels and then patch the highest-ROI user-facing quality failure class, while also making future saved batches replay-gradeable.
