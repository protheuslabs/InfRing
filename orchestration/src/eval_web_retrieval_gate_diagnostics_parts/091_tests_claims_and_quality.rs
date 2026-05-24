#[test]
fn claim_extraction_gate_uses_direct_evidence_claim_contract_when_present() {
    let payload = json!({
        "pending_tool_request": {
            "tool_key": "batch_query",
            "input": {
                "query": "Give me news from this week",
                "queries": ["Give me news from this week"],
                "keywords": ["news", "this week"]
            }
        },
        "tools": [{
            "status": "partial"
        }],
        "evidence_refs": [
            {
                "title": "Generic current events page",
                "snippet": "A thin row that should not count as claim-backed evidence.",
                "claim_hints": ["Thin inferred hint"]
            }
        ]
    });
    let retrieval_quality = json!({
        "status": "usable",
        "candidate_count": 8,
        "evidence_count": 4,
        "content_rich_candidate_count": 4,
        "materialized_candidate_count": 4,
        "claim_hint_count": 6,
        "usable_evidence": false,
        "classification_inputs": {
            "direct_contract_present": true,
            "direct_evidence_claim_count": 0
        }
    });
    let query_metadata = json!({
        "metadata_present": true,
        "rich_query_pack_or_narrow_marker": true
    });
    let transitions = json!({
        "checkpoints": [{
            "checkpoint": "5e_agent_received_evidence_context",
            "status": "pass"
        }]
    });
    let diag =
        web_retrieval_gate_diagnostics(&payload, &retrieval_quality, &query_metadata, &transitions);
    let gate_5c = diag
        .get("gates")
        .and_then(Value::as_array)
        .and_then(|rows| {
            rows.iter().find(|row| {
                row.get("gate").and_then(Value::as_str) == Some("web_5c_claim_extraction_present")
            })
        })
        .cloned()
        .expect("web_5c gate");
    assert_eq!(gate_5c.get("status").and_then(Value::as_str), Some("fail"));
    assert_eq!(
        diag.pointer("/first_failed_gate").and_then(Value::as_str),
        Some("web_5c_claim_extraction_present")
    );
    assert_eq!(
        diag.pointer("/operator_metrics/claim_extraction/direct_evidence_claim_count")
            .and_then(Value::as_u64),
        Some(0)
    );
}

#[test]
fn evidence_quality_gates_reject_title_like_claim_fragments() {
    let payload = json!({
        "pending_tool_request": {
            "tool_key": "batch_query",
            "input": {
                "query": "give me news from this week",
                "queries": ["give me news from this week"],
                "keywords": ["news", "this week"]
            }
        },
        "tools": [{
            "status": "usable"
        }],
        "evidence_pack": [{
            "title": "Cooler this Week",
            "locator": "https://www.wlns.com/weather/cooler-this-week/",
            "source_domain": "wlns.com",
            "snippet": "WLNS 6 News reported a local weather story published during the week with cooler temperatures expected in Lansing, Michigan.",
            "claim_hints": ["Cooler this Week"]
        }]
    });
    let retrieval_quality = json!({
        "status": "usable",
        "candidate_count": 4,
        "evidence_count": 1,
        "content_rich_candidate_count": 1,
        "materialized_candidate_count": 1,
        "claim_hint_count": 1,
        "usable_evidence": true
    });
    let query_metadata = json!({
        "metadata_present": true,
        "rich_query_pack_or_narrow_marker": true
    });
    let transitions = json!({
        "checkpoints": [{
            "checkpoint": "5e_agent_received_evidence_context",
            "status": "pass"
        }]
    });
    let diag =
        web_retrieval_gate_diagnostics(&payload, &retrieval_quality, &query_metadata, &transitions);
    assert_eq!(
        diag.pointer("/first_failed_gate").and_then(Value::as_str),
        Some("web_5e_claim_quality_ready")
    );
    assert_eq!(
        diag.pointer("/evidence_quality/claim_quality_ready")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        diag.pointer("/evidence_quality/citation_renderability_ready")
            .and_then(Value::as_bool),
        Some(true)
    );
}

#[test]
fn evidence_quality_gates_pass_clean_source_backed_claims() {
    let payload = json!({
        "pending_tool_request": {
            "tool_key": "batch_query",
            "input": {
                "query": "compare web research APIs",
                "queries": ["compare web research APIs"],
                "keywords": ["web research APIs", "source links", "raw content"]
            }
        },
        "tools": [{
            "status": "usable"
        }],
        "evidence_pack": [{
            "title": "Search API documentation",
            "locator": "https://docs.example.com/search-api",
            "source_type": "official_docs",
            "source_kind": "official_docs",
            "source_domain": "docs.example.com",
            "snippet": "The documentation describes a search API that returns answer-ready result objects with source links, snippets, and raw content fields for retrieval workflows.",
            "relevant_extract": "The documentation describes a search API that returns answer-ready result objects with source links, snippets, and raw content fields for retrieval workflows.",
            "why_relevant_to_query": "It directly describes web research API result fields that matter for comparing retrieval tooling.",
            "claim_hints": [
                "The search API returns structured result objects with source links, snippets, and raw content fields for retrieval workflows."
            ]
        }]
    });
    let retrieval_quality = json!({
        "status": "usable",
        "candidate_count": 6,
        "evidence_count": 1,
        "content_rich_candidate_count": 1,
        "materialized_candidate_count": 1,
        "claim_hint_count": 1,
        "usable_evidence": true
    });
    let query_metadata = json!({
        "metadata_present": true,
        "rich_query_pack_or_narrow_marker": true
    });
    let transitions = json!({
        "checkpoints": [{
            "checkpoint": "5e_agent_received_evidence_context",
            "status": "pass"
        }]
    });
    let diag =
        web_retrieval_gate_diagnostics(&payload, &retrieval_quality, &query_metadata, &transitions);
    assert_eq!(
        diag.pointer("/first_failed_gate").and_then(Value::as_str),
        None
    );
    for gate_name in [
        "web_5d_source_quality_ready",
        "web_5e_claim_quality_ready",
        "web_5f_citation_renderability_ready",
        "web_5g_answerability_ready",
        "web_5h_evidence_packet_contract_ready",
        "web_7_usable_evidence_available",
    ] {
        let gate = diag
            .get("gates")
            .and_then(Value::as_array)
            .and_then(|rows| {
                rows.iter()
                    .find(|row| row.get("gate").and_then(Value::as_str) == Some(gate_name))
            })
            .cloned()
            .unwrap_or_else(|| panic!("missing {gate_name}"));
        assert_eq!(
            gate.get("status").and_then(Value::as_str),
            Some("pass"),
            "{gate_name}: {gate:#?}"
        );
    }
}

#[test]
fn source_quality_counts_relevant_extract_even_when_snippet_is_thin() {
    let payload = json!({
        "pending_tool_request": {
            "tool_key": "batch_query",
            "input": {
                "query": "compare web research APIs",
                "queries": ["compare web research APIs"],
                "keywords": ["web research APIs", "source links", "raw content"]
            }
        },
        "tools": [{
            "status": "usable"
        }],
        "evidence_pack": [{
            "title": "Search API documentation",
            "locator": "https://docs.example.com/search-api",
            "source_type": "official_docs",
            "source_kind": "official_docs",
            "source_domain": "docs.example.com",
            "snippet": "Docs home",
            "relevant_extract": "The documentation describes a search API that returns source links, snippets, raw content, and answer-ready result objects for retrieval workflows.",
            "why_relevant_to_query": "It directly describes web research API result fields that matter for comparing retrieval tooling.",
            "claim_hints": [
                "The search API returns source links, snippets, raw content, and answer-ready result objects."
            ]
        }]
    });
    let retrieval_quality = json!({
        "status": "usable",
        "candidate_count": 6,
        "evidence_count": 1,
        "content_rich_candidate_count": 1,
        "materialized_candidate_count": 1,
        "claim_hint_count": 1,
        "usable_evidence": true
    });
    let query_metadata = json!({
        "metadata_present": true,
        "rich_query_pack_or_narrow_marker": true
    });
    let transitions = json!({
        "checkpoints": [{
            "checkpoint": "5e_agent_received_evidence_context",
            "status": "pass"
        }]
    });
    let diag =
        web_retrieval_gate_diagnostics(&payload, &retrieval_quality, &query_metadata, &transitions);
    assert_eq!(
        diag.pointer("/evidence_quality/source_quality_ready")
            .and_then(Value::as_bool),
        Some(true),
        "{diag:#?}"
    );
    assert_eq!(
        diag.pointer("/first_failed_gate").and_then(Value::as_str),
        None,
        "{diag:#?}"
    );
}

#[test]
fn source_quality_accepts_clean_diverse_evidence_even_when_pack_has_extra_rows() {
    let payload = json!({
        "pending_tool_request": {
            "tool_key": "batch_query",
            "input": {
                "query": "compare two products for pet hair",
                "queries": ["compare two products for pet hair"],
                "keywords": ["product comparison", "pet hair", "review evidence"]
            }
        },
        "tools": [{
            "status": "usable"
        }],
        "evidence_refs": [
            {
                "title": "Independent pet hair vacuum test",
                "locator": "https://reviews.example.com/pet-hair-test",
                "source_type": "independent_review",
                "source_domain": "reviews.example.com",
                "snippet": "The independent review compares pickup, hair wrap behavior, bin handling, and day-to-day pet hair usability across both products.",
                "why_relevant_to_query": "It directly compares the requested products on pet hair performance.",
                "claim_hints": ["The review compares pet hair pickup and hair wrap behavior across both products."]
            },
            {
                "title": "Lab suction benchmark",
                "locator": "https://lab.example.com/vacuum-benchmark",
                "source_type": "lab_test",
                "source_domain": "lab.example.com",
                "snippet": "The benchmark records suction, carpet pickup, hard-floor pickup, and runtime measurements for the products being compared.",
                "why_relevant_to_query": "It gives measured performance data for the comparison.",
                "claim_hints": ["The benchmark records suction, pickup, and runtime measurements."]
            },
            {
                "title": "Retail spec sheet",
                "locator": "https://retailer.example.com/specs",
                "source_type": "retail_specs",
                "source_domain": "retailer.example.com",
                "snippet": "The spec sheet lists weight, warranty, included pet attachments, bin size, and filtration details for both products.",
                "why_relevant_to_query": "It fills practical buying-decision specs missing from review prose.",
                "claim_hints": ["The spec sheet lists warranty, attachments, bin size, and filtration details."]
            }
        ]
    });
    let retrieval_quality = json!({
        "status": "usable",
        "candidate_count": 20,
        "evidence_count": 10,
        "content_rich_candidate_count": 3,
        "materialized_candidate_count": 3,
        "claim_hint_count": 3,
        "usable_evidence": true
    });
    let query_metadata = json!({
        "metadata_present": true,
        "rich_query_pack_or_narrow_marker": true
    });
    let transitions = json!({
        "checkpoints": [{
            "checkpoint": "5e_agent_received_evidence_context",
            "status": "pass"
        }]
    });

    let diag =
        web_retrieval_gate_diagnostics(&payload, &retrieval_quality, &query_metadata, &transitions);

    assert_eq!(
        diag.pointer("/evidence_quality/source_quality_threshold_met")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        diag.pointer("/evidence_quality/clean_diverse_source_quality")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        diag.pointer("/evidence_quality/source_quality_ready")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_ne!(
        diag.pointer("/first_failed_gate").and_then(Value::as_str),
        Some("web_5d_source_quality_ready")
    );
}

#[test]
fn source_quality_respects_pack_source_thresholds_when_present() {
    let payload = json!({
        "pending_tool_request": {
            "tool_key": "batch_query",
            "input": {
                "query": "compare two products",
                "queries": ["compare two products"],
                "keywords": ["comparison", "reviews", "specs"]
            }
        },
        "tools": [{
            "status": "usable"
        }],
            "evidence_pack_quality": {
                "status": "usable",
            "usable_count": 2,
            "content_rich_item_count": 2,
            "claim_hint_count": 2,
            "source_domain_count": 1,
            "missing_facet_count": 0,
            "weak_facet_count": 0,
            "low_confidence_count": 0,
            "candidate_only_count": 0,
            "thresholds": {
                "min_usable_items": 2,
                "min_source_domains": 2
            }
        },
        "evidence_pack": [
            {
                "title": "Independent comparison",
                "locator": "https://reviews.example.com/product-comparison",
                "source_type": "independent_review",
                "source_domain": "reviews.example.com",
                "snippet": "The independent review compares measured pickup, runtime, reliability, warranty, and day-to-day usability across both products.",
                "relevant_extract": "The independent review compares measured pickup, runtime, reliability, warranty, and day-to-day usability across both products.",
                "why_relevant_to_query": "It directly compares the requested products across practical buying factors.",
                "claim_hints": ["The review compares measured pickup, runtime, reliability, warranty, and usability across both products."]
            },
            {
                "title": "Second independent comparison row",
                "locator": "https://reviews.example.com/product-comparison-details",
                "source_type": "independent_review",
                "source_domain": "reviews.example.com",
                "snippet": "The follow-up review row adds more details about warranty exclusions, maintenance, and common reliability complaints from the same source.",
                "relevant_extract": "The follow-up review row adds details about warranty exclusions, maintenance, and common reliability complaints from the same source.",
                "why_relevant_to_query": "It adds practical comparison detail, but does not add source diversity.",
                "claim_hints": ["The follow-up row adds warranty, maintenance, and reliability details from the same source."]
            }
        ]
    });
    let retrieval_quality = json!({
        "status": "usable",
        "candidate_count": 8,
        "evidence_count": 2,
        "content_rich_candidate_count": 2,
        "materialized_candidate_count": 2,
        "claim_hint_count": 2,
        "usable_evidence": true
    });
    let query_metadata = json!({
        "metadata_present": true,
        "rich_query_pack_or_narrow_marker": true
    });
    let transitions = json!({
        "checkpoints": [{
            "checkpoint": "5e_agent_received_evidence_context",
            "status": "pass"
        }]
    });
    let diag =
        web_retrieval_gate_diagnostics(&payload, &retrieval_quality, &query_metadata, &transitions);

    assert_eq!(
        diag.pointer("/evidence_quality/pack_source_thresholds_met")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        diag.pointer("/evidence_quality/source_quality_ready")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        diag.pointer("/first_failed_gate").and_then(Value::as_str),
        Some("web_5d_source_quality_ready")
    );
}

#[test]
fn answerability_allows_bounded_partial_coverage_when_majority_facets_are_covered() {
    let payload = json!({
        "pending_tool_request": {
            "tool_key": "batch_query",
            "input": {
                "query": "summarize current breakthroughs across fields",
                "queries": ["summarize current breakthroughs across fields"],
                "keywords": ["breakthroughs", "science", "current"]
            }
        },
        "tools": [{
            "status": "usable"
        }],
        "evidence_pack_quality": {
            "status": "usable",
            "usable_count": 3,
            "content_rich_item_count": 3,
            "claim_hint_count": 3,
            "source_domain_count": 3,
            "missing_facet_count": 1,
            "weak_facet_count": 0,
            "covered_facet_count": 2,
            "total_facet_count": 3,
            "covered_facet_ratio": 0.6666666667,
            "coverage_thresholds_met": true,
            "low_confidence_count": 0,
            "candidate_only_count": 0,
            "thresholds": {
                "min_usable_items": 2,
                "min_source_domains": 2,
                "min_covered_facet_ratio_for_usable": 0.6
            }
        },
        "evidence_pack": [
            {
                "title": "Research item A",
                "locator": "https://journal-a.example.com/article",
                "source_type": "scholarly_or_research",
                "source_domain": "journal-a.example.com",
                "snippet": "The article reports a concrete research result with enough source text for a bounded summary.",
                "relevant_extract": "The article reports a concrete research result with enough source text for a bounded summary.",
                "why_relevant_to_query": "It covers one requested current breakthrough facet.",
                "claim_hints": ["The article reports a concrete research result for one current breakthrough facet."]
            },
            {
                "title": "Research item B",
                "locator": "https://journal-b.example.com/article",
                "source_type": "scholarly_or_research",
                "source_domain": "journal-b.example.com",
                "snippet": "The second article gives another concrete finding and enough detail for citation-backed synthesis.",
                "relevant_extract": "The second article gives another concrete finding and enough detail for citation-backed synthesis.",
                "why_relevant_to_query": "It covers another requested current breakthrough facet.",
                "claim_hints": ["The second article gives another concrete finding for citation-backed synthesis."]
            },
            {
                "title": "Research item C",
                "locator": "https://journal-c.example.com/article",
                "source_type": "scholarly_or_research",
                "source_domain": "journal-c.example.com",
                "snippet": "The third article adds a separate source and concrete result, but the packet still leaves a requested facet missing.",
                "relevant_extract": "The third article adds a separate source and concrete result, but the packet still leaves a requested facet missing.",
                "why_relevant_to_query": "It adds source diversity while the coverage report still marks a missing facet.",
                "claim_hints": ["The third article adds a separate source and concrete result."]
            }
        ]
    });
    let retrieval_quality = json!({
        "status": "usable",
        "candidate_count": 10,
        "evidence_count": 3,
        "content_rich_candidate_count": 3,
        "materialized_candidate_count": 3,
        "claim_hint_count": 3,
        "usable_evidence": true
    });
    let query_metadata = json!({
        "metadata_present": true,
        "rich_query_pack_or_narrow_marker": true
    });
    let transitions = json!({
        "checkpoints": [{
            "checkpoint": "5e_agent_received_evidence_context",
            "status": "pass"
        }]
    });
    let diag =
        web_retrieval_gate_diagnostics(&payload, &retrieval_quality, &query_metadata, &transitions);

    assert_eq!(
        diag.pointer("/evidence_quality/source_quality_ready")
            .and_then(Value::as_bool),
        Some(true),
        "{diag:#?}"
    );
    assert_eq!(
        diag.pointer("/evidence_quality/pack_coverage_thresholds_met")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        diag.pointer("/evidence_quality/answerability_ready")
            .and_then(Value::as_bool),
        Some(true)
    );
}

#[test]
fn answerability_rejects_packets_when_coverage_gaps_dominate() {
    let payload = json!({
        "pending_tool_request": {
            "tool_key": "batch_query",
            "input": {
                "query": "summarize current breakthroughs across fields",
                "queries": ["summarize current breakthroughs across fields"],
                "keywords": ["breakthroughs", "science", "current"]
            }
        },
        "tools": [{
            "status": "usable"
        }],
        "evidence_pack_quality": {
            "status": "thin",
            "usable_count": 3,
            "content_rich_item_count": 3,
            "claim_hint_count": 3,
            "source_domain_count": 3,
            "missing_facet_count": 2,
            "weak_facet_count": 2,
            "covered_facet_count": 1,
            "total_facet_count": 5,
            "covered_facet_ratio": 0.2,
            "coverage_thresholds_met": false,
            "low_confidence_count": 0,
            "candidate_only_count": 0,
            "thresholds": {
                "min_usable_items": 2,
                "min_source_domains": 2,
                "min_covered_facet_ratio_for_usable": 0.6
            }
        },
        "evidence_pack": [
            {
                "title": "Research item A",
                "locator": "https://journal-a.example.com/article",
                "source_type": "scholarly_or_research",
                "source_domain": "journal-a.example.com",
                "snippet": "The article reports a concrete research result with enough source text for a bounded summary.",
                "relevant_extract": "The article reports a concrete research result with enough source text for a bounded summary.",
                "why_relevant_to_query": "It covers one requested current breakthrough facet.",
                "claim_hints": ["The article reports a concrete research result for one current breakthrough facet."]
            },
            {
                "title": "Research item B",
                "locator": "https://journal-b.example.com/article",
                "source_type": "scholarly_or_research",
                "source_domain": "journal-b.example.com",
                "snippet": "The second article gives another concrete finding and enough detail for citation-backed synthesis.",
                "relevant_extract": "The second article gives another concrete finding and enough detail for citation-backed synthesis.",
                "why_relevant_to_query": "It covers another requested current breakthrough facet.",
                "claim_hints": ["The second article gives another concrete finding for citation-backed synthesis."]
            },
            {
                "title": "Research item C",
                "locator": "https://journal-c.example.com/article",
                "source_type": "scholarly_or_research",
                "source_domain": "journal-c.example.com",
                "snippet": "The third article adds a separate source and concrete result, but the packet still leaves major requested coverage gaps.",
                "relevant_extract": "The third article adds a separate source and concrete result, but the packet still leaves major requested coverage gaps.",
                "why_relevant_to_query": "It adds source diversity while the coverage report still marks multiple missing and weak facets.",
                "claim_hints": ["The third article adds a separate source and concrete result."]
            }
        ]
    });
    let retrieval_quality = json!({
        "status": "usable",
        "candidate_count": 10,
        "evidence_count": 3,
        "content_rich_candidate_count": 3,
        "materialized_candidate_count": 3,
        "claim_hint_count": 3,
        "usable_evidence": true
    });
    let query_metadata = json!({
        "metadata_present": true,
        "rich_query_pack_or_narrow_marker": true
    });
    let transitions = json!({
        "checkpoints": [{
            "checkpoint": "5e_agent_received_evidence_context",
            "status": "pass"
        }]
    });
    let diag =
        web_retrieval_gate_diagnostics(&payload, &retrieval_quality, &query_metadata, &transitions);

    assert_eq!(
        diag.pointer("/evidence_quality/pack_coverage_thresholds_met")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        diag.pointer("/evidence_quality/answerability_ready")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        diag.pointer("/first_failed_gate").and_then(Value::as_str),
        Some("web_5g_answerability_ready")
    );
}

#[test]
fn evidence_packet_contract_gate_rejects_answerable_but_unexplained_packets() {
    let payload = json!({
        "pending_tool_request": {
            "tool_key": "batch_query",
            "input": {
                "query": "compare web research APIs",
                "queries": ["compare web research APIs"],
                "keywords": ["web research APIs", "source links", "raw content"]
            }
        },
        "tools": [{
            "status": "usable"
        }],
        "evidence_pack": [{
            "title": "Search API documentation",
            "locator": "https://docs.example.com/search-api",
            "source_type": "official_docs",
            "source_kind": "official_docs",
            "source_domain": "docs.example.com",
            "snippet": "The documentation describes a search API that returns answer-ready result objects with source links, snippets, and raw content fields for retrieval workflows.",
            "relevant_extract": "The documentation describes a search API that returns answer-ready result objects with source links, snippets, and raw content fields for retrieval workflows.",
            "claim_hints": [
                "The search API returns structured result objects with source links, snippets, and raw content fields for retrieval workflows."
            ]
        }]
    });
    let retrieval_quality = json!({
        "status": "usable",
        "candidate_count": 6,
        "evidence_count": 1,
        "content_rich_candidate_count": 1,
        "materialized_candidate_count": 1,
        "claim_hint_count": 1,
        "usable_evidence": true
    });
    let query_metadata = json!({
        "metadata_present": true,
        "rich_query_pack_or_narrow_marker": true
    });
    let transitions = json!({
        "checkpoints": [{
            "checkpoint": "5e_agent_received_evidence_context",
            "status": "pass"
        }]
    });
    let diag =
        web_retrieval_gate_diagnostics(&payload, &retrieval_quality, &query_metadata, &transitions);
    assert_eq!(
        diag.pointer("/first_failed_gate").and_then(Value::as_str),
        Some("web_5h_evidence_packet_contract_ready")
    );
    assert_eq!(
        diag.pointer("/evidence_quality/evidence_packet_contract/ready")
            .and_then(Value::as_bool),
        Some(false)
    );
    let empty = Vec::new();
    assert!(
        diag.pointer("/evidence_quality/evidence_packet_contract/missing_fields")
            .and_then(Value::as_array)
            .unwrap_or(&empty)
            .iter()
            .any(|row| row.as_str() == Some("why_relevant_to_query")),
        "{diag:#?}"
    );
}
