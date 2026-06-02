use super::eval_research_golden_utils::*;
use serde_json::{json, Value};
use std::collections::BTreeMap;

const WEB_GATE_TARGET_PASS_RATE: f64 = 0.95;

include!("eval_web_retrieval_gate_diagnostics_parts/010_diagnostics.rs");
include!("eval_web_retrieval_gate_diagnostics_parts/020_gate_counts_and_metrics.rs");
include!("eval_web_retrieval_gate_diagnostics_parts/021_measurement_report.rs");
include!("eval_web_retrieval_gate_diagnostics_parts/030_metric_helpers.rs");
include!("eval_web_retrieval_gate_diagnostics_parts/031_case_metrics.rs");
include!("eval_web_retrieval_gate_diagnostics_parts/032_aggregate_readouts.rs");
include!("eval_web_retrieval_gate_diagnostics_parts/040_source_inventory.rs");
include!("eval_web_retrieval_gate_diagnostics_parts/050_evidence_quality.rs");
include!("eval_web_retrieval_gate_diagnostics_parts/060_provider_supply.rs");
include!("eval_web_retrieval_gate_diagnostics_parts/070_browser_materialization.rs");
include!("eval_web_retrieval_gate_diagnostics_parts/080_access_blockers.rs");

pub fn evidence_quality_diagnostics(payload: &Value, retrieval_quality: &Value) -> Value {
    web_evidence_quality_diagnostics(payload, retrieval_quality)
}

#[cfg(test)]
mod tests {
    use super::*;

    include!("eval_web_retrieval_gate_diagnostics_parts/090_tests_access_and_provider.rs");
    include!("eval_web_retrieval_gate_diagnostics_parts/091_tests_claims_and_quality.rs");
    include!("eval_web_retrieval_gate_diagnostics_parts/092_tests_failure_ordering.rs");
    include!("eval_web_retrieval_gate_diagnostics_parts/093_tests_browser_recovery.rs");
}
