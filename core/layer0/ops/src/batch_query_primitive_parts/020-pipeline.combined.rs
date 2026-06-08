// Split from 020-pipeline.combined.rs into focused include parts for maintainability.
include!("020-pipeline.combined_parts/010-link-fetch-fallback-limit-to-stage-error.rs");
include!("020-pipeline.combined_parts/015-official-domain-probes.rs");
include!("020-pipeline.combined_parts/020-collect-candidates-from-stage-payload-to-retrieve-web-candidates-for.rs");
include!("020-pipeline.combined_parts/030-rerank-score-to-is-benign-partial-failure.rs");
include!("020-pipeline.combined_parts/040-api-batch-query.rs");
