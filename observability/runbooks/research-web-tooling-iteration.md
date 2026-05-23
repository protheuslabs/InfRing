# Research Web Tooling Iteration Runbook

Use this runbook when research answers feel bad, vague, fragmentary, or over-scored even though the workflow gates look healthy.

## Purpose

The research workflow should be improved upstream-down. Do not patch synthesis or workflow routing while the evidence packet is not good enough for a competent agent to answer from.

The guiding question is:

> Could a careful human or agent produce a coherent, useful, source-backed answer from the evidence packet returned by the tool?

If the answer is no, the next fix belongs in web tooling, retrieval, candidate selection, materialization, evidence packaging, or the evaluator that measures those layers.

## Iteration Loop

1. Run the web tooling lane before the full workflow.
   Prefer a tooling-only probe when diagnosing evidence quality. Full workflow runs are useful only after the evidence packet looks answer-ready.

2. Inspect the actual evidence packet.
   Read selected sources, snippets/extracts, claim hints, source domains, relevance reasons, and citation fields. Counts alone are not enough.

3. Find the most upstream failing web gate.
   Work top-down. Do not patch a downstream gate while an upstream gate is failing.

4. Patch one gate class at a time.
   Keep the fix generic. Do not hardcode sample queries, topics, domains, products, news categories, or expected facts.

5. Separate measurement fixes from capability fixes.
   A measurement fix makes the report more honest. A capability fix makes the returned evidence better. Record which one happened.

6. Re-run the smallest useful probe.
   Use diverse prompts once the focused probe improves. Use full workflow only to validate end-to-end behavior after the tooling lane is healthier.

7. Do a manual “feels good” smoke check.
   This is not authoritative grading. It is a flag for obvious user frustration: source dumps, non-answers, broken tables, truncated text, irrelevant headlines, or answers a real user would reject.

## Gate Order

Use this order for web-tooling work:

1. `web_1_request_shape_present`
2. `web_2_query_metadata_present`
3. `web_3_tool_attempt_recorded`
4. access/provider gates: `web_3b*`, `web_4*`
5. `web_5_packaged_evidence_present`
6. `web_5b_content_rich_candidates_present`
7. `web_5c_claim_extraction_present`
8. `web_5d_source_quality_ready`
9. `web_5e_claim_quality_ready`
10. `web_5f_citation_renderability_ready`
11. `web_5g_answerability_ready`
12. `web_5h_evidence_packet_contract_ready`
13. `web_7_usable_evidence_available`
14. `web_8_evidence_context_to_synthesis`

If workflow gates pass but `web_5d_source_quality_ready` or `web_5g_answerability_ready` fail, the workflow is probably doing its job. The primitive failure is the evidence packet.

## Pass/Excellent Discipline

Do not let a safe fallback masquerade as excellent.

A limitation-heavy answer can be honest and still not useful. It should not earn excellent unless the original user goal was to determine whether evidence exists or the answer genuinely resolves the request from strong evidence.

Excellent research output requires:

- source-quality-ready evidence
- answerability-ready evidence
- citation-renderable evidence
- evidence packets carrying source identity, source type, extract, claim material, and query-relevance rationale
- final answer units traceable to the retrieved evidence
- no visible tool/process dump as a substitute for an answer

## Anti-Patterns

- Do not tune synthesis around one sample query.
- Do not encode assumptions like “news query,” “AI landscape query,” or product-specific behavior into the primitive research path.
- Do not celebrate a higher score until the output would be useful in a real chat.
- Do not keep running full workflow probes when the evidence packet is clearly weak.
- Do not treat provider/raw candidate counts as evidence quality.
- Do not let duplicated packet rows dilute or inflate quality metrics without checking actual selected evidence.

## Progress Definition

Progress is real when at least one of these becomes true:

- the most upstream failing gate moves downstream for the right reason
- actual selected evidence is more source-backed, citable, relevant, and extract-rich
- a manual smoke check says the answer is less frustrating
- the evaluator becomes more honest about bad output

If metrics improve but the output still feels bad, assume the metric is incomplete and patch the measurement before trusting the score.

