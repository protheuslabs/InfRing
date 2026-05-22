use super::eval_research_golden_utils::*;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

include!("eval_research_golden_scoring_parts/010_case_grade.rs");
include!("eval_research_golden_scoring_parts/020_response_gate_basics.rs");
include!("eval_research_golden_scoring_parts/030_citation_artifacts.rs");
include!("eval_research_golden_scoring_parts/040_response_contracts.rs");
include!("eval_research_golden_scoring_parts/050_soft_quality.rs");
include!("eval_research_golden_scoring_parts/051_answer_alignment.rs");
include!("eval_research_golden_scoring_parts/052_answer_risk_signals.rs");
include!("eval_research_golden_scoring_parts/060_prompt_entities.rs");
include!("eval_research_golden_scoring_parts/070_retrieval_quality.rs");
include!("eval_research_golden_scoring_parts/071_retrieval_relevance.rs");
include!("eval_research_golden_scoring_parts/079_structured_snippets.rs");
include!("eval_research_golden_scoring_parts/080_excellent_provider_metrics.rs");
include!("eval_research_golden_scoring_parts/081_materialization_content.rs");
include!("eval_research_golden_scoring_parts/082_response_signals.rs");

#[cfg(test)]
mod tests {
    use super::*;

    include!("eval_research_golden_scoring_parts/090_tests_retrieval_and_query.rs");
    include!("eval_research_golden_scoring_parts/091_tests_contracts_and_entities.rs");
    include!("eval_research_golden_scoring_parts/092_tests_relevance_and_citations.rs");
    include!("eval_research_golden_scoring_parts/093_tests_excellent_and_sources.rs");
}
