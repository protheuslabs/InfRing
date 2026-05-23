# Research Area

This directory holds durable research artifacts and notes that should be easy to inspect outside high-churn runtime state.

Current wired producer:

- `autotest_doctor` writes mirrored rollback/broken-piece research bundles to `research/autotest_doctor/`.
- `inversion` stores human-facing rollout and safety research for high-sensitivity inversion governance.

Runtime-generated JSON artifacts under `research/autotest_doctor/` are intentionally git-ignored to avoid tree churn.

## Related Runbooks

- [Research Web Tooling Iteration](../runbooks/research-web-tooling-iteration.md) defines the reusable method for diagnosing weak research output upstream-down: tooling lane first, evidence packet inspection, one failing web gate at a time, then full workflow validation.
