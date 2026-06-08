{
    gates.extend([
        web_gate(
            "web_3d_browser_materialization_not_failed",
            blocker_recovery_lane_visible,
            !browser_materialization_failed_hard,
            if browser_materialization_failed_hard {
                "browser-materialization was the active blocking recovery lane and reported failure, timeout, navigation failure, or extraction failure"
            } else if browser_materialization_failed {
                "browser-materialization reported a non-blocking failed enrichment attempt, but another materialization failure reason was more upstream"
            } else if blocker_recovery_lane_visible {
                "browser-materialization recovery lane was visible and no recovery failure signal was detected"
            } else {
                "browser-materialization failure cannot be inspected when the recovery lane is not visible"
            },
            browser_materialization_recovery
                .get("artifact_refs")
                .and_then(Value::as_array)
                .map(|rows| {
                    rows.iter()
                        .filter_map(Value::as_str)
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                })
                .filter(|rows| !rows.is_empty())
                .unwrap_or_else(|| {
                    vec![
                        "tool_result_quality.browser_materialization".to_string(),
                        "runtime_web_tools_metadata.browser_materialization".to_string(),
                    ]
                }),
        ),
        web_gate(
            "web_4a_search_provider_configuration_usable",
            tool_attempted,
            provider_config_usable,
            if provider_config_usable {
                "search provider configuration is usable enough for this run"
            } else if tool_attempted {
                "search provider supply is constrained by missing credentials, missing strong provider, or provider admission/configuration state"
            } else {
                "provider configuration cannot be inspected before a tool attempt"
            },
            provider_supply_refs(&provider_supply),
        ),
        web_gate(
            "web_4b_search_provider_circuit_closed",
            tool_attempted,
            provider_circuits_closed,
            if provider_circuits_closed {
                if provider_circuit_open_detected {
                    "a provider circuit-open signal was present, but another provider path produced usable candidates and evidence"
                } else {
                    "no search-provider circuit-open signal was detected"
                }
            } else if tool_attempted {
                "one or more search providers were skipped by an open circuit breaker"
            } else {
                "provider circuit state cannot be inspected before a tool attempt"
            },
            provider_supply_refs(&provider_supply),
        ),
        web_gate(
            "web_4c_search_provider_surface_ready",
            tool_attempted,
            provider_surface_ready,
            if provider_surface_ready {
                if provider_surface_degraded {
                    "search surface reported non-fatal provider degradation, but usable candidates and evidence were produced"
                } else {
                    "search tool surface did not report degraded execution"
                }
            } else if tool_attempted {
                "search tool surface reported degraded execution before usable candidates could be produced"
            } else {
                "provider surface readiness cannot be inspected before a tool attempt"
            },
            provider_supply_refs(&provider_supply),
        ),
        web_gate(
            "web_4d_provider_raw_rows_available",
            tool_attempted,
            provider_raw_rows_available,
            if provider_raw_rows_available {
                "provider attempts produced raw rows before filtering or promotion"
            } else if tool_attempted {
                "provider attempts produced no raw rows"
            } else {
                "raw provider rows cannot be inspected before a tool attempt"
            },
            provider_supply_refs(&provider_supply),
        ),
        web_gate(
            "web_4e_browser_serp_external_urls_extracted",
            browser_serp_attempted,
            !browser_serp_attempted || browser_serp_external_urls_extracted,
            if !browser_serp_attempted {
                "browser SERP was not attempted in this run, so organic URL extraction is not applicable"
            } else if browser_serp_external_urls_extracted {
                "browser SERP produced at least one external URL that survived provider filtering"
            } else {
                "browser SERP was attempted but did not produce any external organic URLs that survived filtering"
            },
            provider_supply_refs(&provider_supply),
        ),
        web_gate(
            "web_4e_provider_candidates_survive_filtering",
            provider_raw_rows_available || tool_attempted,
            provider_candidates_survive_filtering,
            if provider_candidates_survive_filtering {
                "some provider rows survived filtering into candidate rows"
            } else if provider_raw_rows_available {
                "provider rows existed, but all were filtered, rejected, low-confidence-only, or not promoted into candidates"
            } else {
                "candidate filtering cannot be inspected before raw provider rows"
            },
            provider_supply_refs(&provider_supply),
        ),
        web_gate(
            "web_4_raw_candidates_present",
            tool_attempted,
            raw_candidates_present,
            if raw_candidates_present {
                "provider returned raw candidates, search rows, or equivalent candidate artifacts"
            } else if tool_attempted {
                "tool ran but no raw candidates or provider rows were visible"
            } else {
                "raw candidates cannot be expected before a tool attempt"
            },
            vec![
                "retrieval_quality.candidate_count".to_string(),
                "5b_raw_provider_result_present".to_string(),
            ],
        ),
        web_gate(
            "web_5_packaged_evidence_present",
            raw_candidates_present || tool_attempted,
            packaged_evidence_present,
            if packaged_evidence_present {
                "candidate output was packaged into evidence refs, findings, sources, or equivalent artifacts"
            } else if raw_candidates_present {
                "raw candidates were present but no packaged evidence artifact was visible"
            } else {
                "packaged evidence cannot be expected before raw candidates"
            },
            vec![
                "retrieval_quality.evidence_count".to_string(),
                "5c_packaged_tool_result_present".to_string(),
                "5d_evidence_refs_extracted".to_string(),
            ],
        ),
        web_gate(
            "web_6_provider_not_empty_or_degraded",
            tool_attempted,
            provider_not_empty_or_degraded,
            if provider_not_empty_or_degraded {
                "provider status is not empty, absent, degraded, or no-results"
            } else {
                "provider status indicates no results, degraded transport, contradictory provider state, absent raw output, or no extracted evidence"
            },
            vec![
                "retrieval_quality.status".to_string(),
                "retrieval_quality.quality_flags".to_string(),
            ],
        ),
    ]);
}
