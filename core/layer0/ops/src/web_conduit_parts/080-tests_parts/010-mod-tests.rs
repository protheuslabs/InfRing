#[cfg(test)]
mod tests {
    use super::*;
    include!("010-mod-tests_parts/010-status-and-provider-catalog-tests.rs");
    include!("010-mod-tests_parts/020-fetch-policy-and-provider-contract-tests.rs");
    include!("010-mod-tests_parts/030-search-query-shape-and-filter-tests.rs");
    include!("010-mod-tests_parts/040-domain-normalization-and-render-tests.rs");
    include!("010-mod-tests_parts/050-browser-materialization-contract-tests.rs");
    include!("010-mod-tests_parts/060-browser-serp-provider-tests.rs");
}
