// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)
// V6-TOOL-001: Batch Query Primitive (MVP)

include_parts!(
    "batch_query_primitive_parts/010-core.rs",
    "batch_query_primitive_parts/015-intent-and-quality.rs",
    "batch_query_primitive_parts/016-retrieval-policy-api.rs",
    "batch_query_primitive_parts/017-promotion-diagnostics.rs",
    "batch_query_primitive_parts/018-request-and-cache.rs",
    "batch_query_primitive_parts/020-pipeline.rs",
    "batch_query_primitive_parts/021-summary-and-guidance.rs",
    "batch_query_primitive_parts/030-run.rs",
);

#[cfg(test)]
include!("batch_query_primitive_parts/040-tests.rs");
#[cfg(test)]
include!("batch_query_primitive_parts/042-cache-rewrite-tests.rs");
