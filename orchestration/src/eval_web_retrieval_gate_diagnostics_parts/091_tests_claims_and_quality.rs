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
        Some("web_5d_source_quality_ready")
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
fn evidence_quality_gates_reject_leading_dangling_claim_fragments() {
    let payload = json!({
        "pending_tool_request": {
            "tool_key": "batch_query",
            "input": {
                "query": "what does the evidence say about creatine for women",
                "queries": ["what does the evidence say about creatine for women"],
                "keywords": ["creatine", "women", "evidence"]
            }
        },
        "tools": [{
            "status": "usable"
        }],
        "evidence_pack": [{
            "title": "Common questions and misconceptions about creatine",
            "locator": "https://pmc.ncbi.nlm.nih.gov/articles/PMC7871530",
            "source_domain": "pmc.ncbi.nlm.nih.gov",
            "source_type": "public_institution",
            "relevant_extract": "Lower daily creatine supplementation dosing strategies are well established for increasing intramuscular creatine stores.",
            "snippet": "Lower daily creatine supplementation dosing strategies are well established for increasing intramuscular creatine stores.",
            "claim_hints": [", 3-5 g/day are well established throughout the scientific literature for increasing intramuscular creatine stores"],
            "why_relevant_to_query": "Selected because it discusses creatine supplementation evidence for women."
        }]
    });
    let retrieval_quality = json!({
        "status": "usable",
        "candidate_count": 3,
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
        diag.pointer("/evidence_quality/claim_quality_ready")
            .and_then(Value::as_bool),
        Some(false)
    );
    let has_low_quality_claim_flag = diag
        .pointer("/evidence_quality/low_quality_flags")
        .and_then(Value::as_array)
        .map(|flags| {
            flags
                .iter()
                .any(|flag| flag.as_str() == Some("low_quality_claim_text"))
        })
        .unwrap_or(false);
    assert!(has_low_quality_claim_flag, "{diag:#?}");
}

#[test]
fn evidence_quality_gates_reject_stitched_title_tail_claim_fragments() {
    let payload = json!({
        "pending_tool_request": {
            "tool_key": "batch_query",
            "input": {
                "query": "compare documentation portal options",
                "queries": ["compare documentation portal options"],
                "keywords": ["documentation", "portal", "comparison"]
            }
        },
        "tools": [{
            "status": "usable"
        }],
        "evidence_pack": [
            {
                "title": "API Documentation Tool Comparison",
                "locator": "https://example.com/docs-tools",
                "source_domain": "example.com",
                "source_type": "independent_analysis",
                "relevant_extract": "The comparison discusses multiple documentation tools, but the extracted claim is clipped.",
                "snippet": "The comparison discusses multiple documentation tools, but the extracted claim is clipped.",
                "claim_hints": ["There is also a fair question of whether you n Mintlify Alternatives: 6 Tools Compared f"],
                "why_relevant_to_query": "Selected because it compares documentation portal tools."
            },
            {
                "title": "Docusaurus vs Mintlify Comparison",
                "locator": "https://example.com/docusaurus-mintlify",
                "source_domain": "example.com",
                "source_type": "independent_analysis",
                "relevant_extract": "The row includes a stitched title tail instead of a clean product claim.",
                "snippet": "The row includes a stitched title tail instead of a clean product claim.",
                "claim_hints": ["Its weakness is that it doesn’t Docusaurus vs Mintlify Comparison"],
                "why_relevant_to_query": "Selected because it compares documentation portal tools."
            }
        ]
    });
    let retrieval_quality = json!({
        "status": "usable",
        "candidate_count": 3,
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
        diag.pointer("/evidence_quality/claim_quality_ready")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        diag.pointer("/evidence_quality/evidence_packet_contract/ready")
            .and_then(Value::as_bool),
        Some(false)
    );
    let low_quality_claim_count = diag
        .pointer("/evidence_quality/low_quality_claim_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    assert!(low_quality_claim_count >= 2, "{diag:#?}");
}

#[test]
fn evidence_quality_distinguishes_packet_noise_from_clean_handoff_claims() {
    let payload = json!({
        "pending_tool_request": {
            "tool_key": "batch_query",
            "input": {
                "query": "compare documentation portal options",
                "queries": ["compare documentation portal options"],
                "keywords": ["documentation", "portal", "comparison"]
            }
        },
        "tools": [{
            "status": "usable"
        }],
        "evidence_pack": [
            {
                "title": "API Documentation Tool Comparison",
                "locator": "https://example.com/docs-tools",
                "source_domain": "example.com",
                "source_type": "independent_analysis",
                "relevant_extract": "The comparison discusses multiple documentation tools, but one extracted packet hint is clipped.",
                "snippet": "The comparison discusses multiple documentation tools, but one extracted packet hint is clipped.",
                "claim_hints": ["There is also a fair question of whether you n Mintlify Alternatives: 6 Tools Compared f"],
                "why_relevant_to_query": "Selected because it compares documentation portal tools."
            },
            {
                "title": "Docusaurus vs Mintlify Comparison",
                "locator": "https://example.com/docusaurus-mintlify",
                "source_domain": "example.com",
                "source_type": "independent_analysis",
                "relevant_extract": "The row includes a stitched title tail instead of a clean product claim.",
                "snippet": "The row includes a stitched title tail instead of a clean product claim.",
                "claim_hints": ["Its weakness is that it doesn’t Docusaurus vs Mintlify Comparison"],
                "why_relevant_to_query": "Selected because it compares documentation portal tools."
            }
        ],
        "evidence_claims": [{
            "claim": "The comparison says Docusaurus is stronger for code-owned documentation while Mintlify is positioned around hosted documentation portals.",
            "title": "Docusaurus vs Mintlify Comparison",
            "locator": "https://example.com/docusaurus-mintlify",
            "source_title": "Docusaurus vs Mintlify Comparison",
            "source_locator": "https://example.com/docusaurus-mintlify",
            "source_domain": "example.com",
            "support_snippet": "The comparison says Docusaurus is stronger for code-owned documentation while Mintlify is positioned around hosted documentation portals."
        }]
    });
    let retrieval_quality = json!({
        "status": "usable",
        "candidate_count": 3,
        "evidence_count": 2,
        "content_rich_candidate_count": 2,
        "materialized_candidate_count": 2,
        "claim_hint_count": 2,
        "usable_evidence": true,
        "classification_inputs": {
            "direct_contract_present": true,
            "direct_evidence_claim_count": 1
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

    assert!(
        diag.pointer("/evidence_quality/low_quality_claim_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            >= 2,
        "{diag:#?}"
    );
    assert_eq!(
        diag.pointer("/evidence_quality/handoff_claim_count")
            .and_then(Value::as_u64),
        Some(1),
        "{diag:#?}"
    );
    assert_eq!(
        diag.pointer("/evidence_quality/handoff_low_quality_claim_count")
            .and_then(Value::as_u64),
        Some(0),
        "{diag:#?}"
    );
    assert_eq!(
        diag.pointer("/evidence_quality/handoff_claim_quality_ready")
            .and_then(Value::as_bool),
        Some(true),
        "{diag:#?}"
    );
    assert_eq!(
        diag.pointer("/operator_metrics/evidence_quality/handoff_low_quality_claim_rate")
            .and_then(Value::as_f64),
        Some(0.0),
        "{diag:#?}"
    );
}

#[test]
fn evidence_quality_gate_rejects_mixed_good_and_malformed_fragments() {
    let payload = json!({
        "pending_tool_request": {
            "tool_key": "batch_query",
            "input": {
                "query": "compare documentation portal options",
                "queries": ["compare documentation portal options"],
                "keywords": ["documentation", "portal", "comparison"]
            }
        },
        "tools": [{
            "status": "usable"
        }],
        "evidence_pack": [
            {
                "title": "Code-owned documentation platforms",
                "locator": "https://example.com/code-owned-docs",
                "source_domain": "example.com",
                "source_type": "independent_analysis",
                "relevant_extract": "The analysis says code-owned documentation platforms can fit teams that want reviewable docs changes and custom frontend control.",
                "snippet": "The analysis says code-owned documentation platforms can fit teams that want reviewable docs changes and custom frontend control.",
                "claim_hints": ["Code-owned documentation platforms can fit teams that want reviewable docs changes and custom frontend control."],
                "why_relevant_to_query": "Selected because it compares documentation portal options for documentation teams."
            },
            {
                "title": "Hosted documentation portal tradeoffs",
                "locator": "https://example.org/hosted-docs",
                "source_domain": "example.org",
                "source_type": "independent_analysis",
                "relevant_extract": "Hosted documentation portals can reduce infrastructure burden while trading off some code-level customization.",
                "snippet": "Hosted documentation portals can reduce infrastructure burden while trading off some code-level customization.",
                "claim_hints": ["Hosted documentation portals can reduce infrastructure burden while trading off some code-level customization."],
                "why_relevant_to_query": "Selected because it compares documentation portal options for documentation teams."
            },
            {
                "title": "Documentation portal buyer notes",
                "locator": "https://example.net/docs-buyer-notes",
                "source_domain": "example.net",
                "source_type": "independent_analysis",
                "relevant_extract": "The source compares hosted portal setup speed with self-managed customization, but one extracted hint is visibly stitched.",
                "snippet": "The source compares hosted portal setup speed with self-managed customization, but one extracted hint is visibly stitched.",
                "claim_hints": [
                    "Hosted portal setup can be faster than self-managed customization for teams without dedicated documentation infrastructure.",
                    "The article compares platform tradeoffs but leaves the recommendation incomplet Documentation Portal Buyers Guide"
                ],
                "why_relevant_to_query": "Selected because it compares documentation portal options for documentation teams."
            }
        ]
    });
    let retrieval_quality = json!({
        "status": "usable",
        "candidate_count": 3,
        "evidence_count": 3,
        "content_rich_candidate_count": 3,
        "materialized_candidate_count": 3,
        "claim_hint_count": 4,
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
        Some("web_5i_malformed_evidence_absent"),
        "{diag:#?}"
    );
    assert_eq!(
        diag.pointer("/evidence_quality/malformed_evidence_clean")
            .and_then(Value::as_bool),
        Some(false),
        "{diag:#?}"
    );
    assert!(
        diag.pointer("/evidence_quality/malformed_evidence_fragment_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            >= 1,
        "{diag:#?}"
    );
    assert_eq!(
        diag.pointer("/evidence_quality/source_quality_ready")
            .and_then(Value::as_bool),
        Some(true),
        "{diag:#?}"
    );
    assert_eq!(
        diag.pointer("/evidence_quality/claim_quality_ready")
            .and_then(Value::as_bool),
        Some(true),
        "{diag:#?}"
    );
}

#[test]
fn evidence_quality_gates_reject_headline_dateline_rows_as_answerable_evidence() {
    let payload = json!({
        "pending_tool_request": {
            "tool_key": "batch_query",
            "input": {
                "query": "research an arbitrary current topic",
                "queries": ["research an arbitrary current topic"],
                "keywords": ["current", "source-backed"]
            }
        },
        "tools": [{
            "status": "usable"
        }],
        "evidence_pack": [{
            "title": "What’s really happening inside AI’s black box? Berkeley researchers have answers",
            "locator": "https://example.edu/news/ai-black-box",
            "source_type": "scholarly_or_research",
            "source_domain": "example.edu",
            "snippet": "What’s really happening inside AI’s black box? Berkeley researchers have answers University of California, Berkeley Published: Mon, 20 Apr 2026 07:00:00 GMT. Source: University of California, Berkeley (example.edu).",
            "relevant_extract": "What’s really happening inside AI’s black box? Berkeley researchers have answers University of California, Berkeley Published: Mon, 20 Apr 2026 07:00:00 GMT. Source: University of California, Berkeley (example.edu).",
            "why_relevant_to_query": "The row was selected by a search provider for the requested current topic.",
            "claim_hints": [
                "What’s really happening inside AI’s black box? Berkeley researchers have answers University of California"
            ]
        }]
    });
    let retrieval_quality = json!({
        "status": "usable",
        "candidate_count": 8,
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
        Some(false),
        "{diag:#?}"
    );
    assert_eq!(
        diag.pointer("/evidence_quality/source_quality_pack_ready")
            .and_then(Value::as_bool),
        Some(false),
        "{diag:#?}"
    );
    assert_eq!(
        diag.pointer("/evidence_quality/source_quality_ready")
            .and_then(Value::as_bool),
        Some(false),
        "{diag:#?}"
    );
    assert_eq!(
        diag.pointer("/evidence_quality/claim_quality_ready")
            .and_then(Value::as_bool),
        Some(false),
        "{diag:#?}"
    );
    assert_eq!(
        diag.pointer("/evidence_quality/evidence_packet_contract/ready")
            .and_then(Value::as_bool),
        Some(false),
        "{diag:#?}"
    );
    assert_eq!(
        diag.pointer("/first_failed_gate").and_then(Value::as_str),
        Some("web_5d_source_quality_ready"),
        "{diag:#?}"
    );
}

#[test]
fn evidence_quality_gates_reject_shell_claim_packets_despite_usable_pack_counts() {
    let payload = json!({
        "query": "give me a source-backed update on confirmed current research milestones",
        "query_metadata": {
            "keywords": ["source-backed", "confirmed", "current research", "milestones"],
            "required_coverage": {
                "facets": ["confirmed milestones", "current research"]
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
            "missing_facet_count": 0,
            "weak_facet_count": 0,
            "covered_facet_count": 2,
            "total_facet_count": 2,
            "covered_facet_ratio": 1.0,
            "coverage_thresholds_met": true,
            "low_confidence_count": 0,
            "candidate_only_count": 0,
            "thresholds": {
                "min_usable_items": 2,
                "min_source_domains": 2
            }
        },
        "evidence_pack": [
            {
                "title": "Current research roundup",
                "locator": "https://roundup.example.com/research",
                "source_type": "analysis",
                "source_domain": "roundup.example.com",
                "snippet": "Current Research Milestones So Far By Example Author May 17, 2026 6:25 am EST Example Images This year has seen some notable research activity.",
                "relevant_extract": "Current Research Milestones So Far By Example Author May 17, 2026 6:25 am EST Example Images This year has seen some notable research activity.",
                "why_relevant_to_query": "Selected because it overlaps current research terms.",
                "claim_hints": ["Current Research Milestones So Far By Example Author May 17, 2026 6:25 am EST Example Images This year has seen some notable research activity"]
            },
            {
                "title": "Future research milestones",
                "locator": "https://future.example.com/research",
                "source_type": "analysis",
                "source_domain": "future.example.com",
                "snippet": "Look out for new devices, new trials, and other possible milestones later this year.",
                "relevant_extract": "Look out for new devices, new trials, and other possible milestones later this year.",
                "why_relevant_to_query": "Selected because it mentions research milestones.",
                "claim_hints": ["Look out for new devices, new trials, and other possible milestones later this year"]
            },
            {
                "title": "Approval brief",
                "locator": "https://briefs.example.com/research",
                "source_type": "news",
                "source_domain": "briefs.example.com",
                "snippet": "The agency granted a designation to a new candidate based on data",
                "relevant_extract": "The agency granted a designation to a new candidate based on data",
                "why_relevant_to_query": "Selected because it looks like a confirmed research milestone.",
                "claim_hints": ["The agency granted a designation to a new candidate based on data"]
            }
        ]
    });
    let retrieval_quality = json!({
        "status": "usable",
        "candidate_count": 9,
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
        diag.pointer("/evidence_quality/claim_quality_ready")
            .and_then(Value::as_bool),
        Some(false),
        "{diag:#?}"
    );
    assert_eq!(
        diag.pointer("/evidence_quality/evidence_packet_contract/ready")
            .and_then(Value::as_bool),
        Some(false),
        "{diag:#?}"
    );
    assert_eq!(
        diag.pointer("/evidence_quality/answerability_ready")
            .and_then(Value::as_bool),
        Some(false),
        "{diag:#?}"
    );
    let empty = Vec::new();
    assert!(
        diag.pointer("/evidence_quality/low_quality_flags")
            .and_then(Value::as_array)
            .unwrap_or(&empty)
            .iter()
            .any(|row| row.as_str() == Some("low_quality_claim_text")),
        "{diag:#?}"
    );
    assert_eq!(
        diag.pointer("/first_failed_gate").and_then(Value::as_str),
        Some("web_5d_source_quality_ready"),
        "{diag:#?}"
    );
}

#[test]
fn evidence_quality_gates_reject_interrogative_title_shells_as_claims() {
    let payload = json!({
        "query": "compare software tools for an engineering team",
        "query_metadata": {
            "keywords": ["software tools", "engineering team", "comparison"]
        },
        "tools": [{
            "status": "usable"
        }],
        "evidence_pack": [{
            "title": "How Tool A compares to other engineering assistants",
            "locator": "https://analysis.example.com/tool-a-comparison",
            "source_type": "analysis",
            "source_domain": "analysis.example.com",
            "snippet": "How Tool A compares to other engineering assistants",
            "relevant_extract": "How Tool A compares to other engineering assistants",
            "why_relevant_to_query": "Selected because it is a comparison article for the requested tool category.",
            "claim_hints": ["How Tool A compares to other engineering assistants"]
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
        diag.pointer("/evidence_quality/claim_quality_ready")
            .and_then(Value::as_bool),
        Some(false),
        "{diag:#?}"
    );
    assert_eq!(
        diag.pointer("/first_failed_gate").and_then(Value::as_str),
        Some("web_5d_source_quality_ready"),
        "{diag:#?}"
    );
}

#[test]
fn evidence_packet_contract_rejects_generic_source_identity_with_different_publisher_signature() {
    let payload = json!({
        "query": "summarize engineering research updates",
        "query_metadata": {
            "keywords": ["engineering", "research", "updates"]
        },
        "tools": [{
            "status": "usable"
        }],
        "evidence_pack": [{
            "title": "Web result from archive.example",
            "locator": "https://archive.example/research/update",
            "source_type": "analysis",
            "source_domain": "archive.example",
            "snippet": "New engineering research update. Acme Research / menu Acme Research announced a 2026 engineering benchmark update for robotics teams.",
            "relevant_extract": "New engineering research update. Acme Research / menu Acme Research announced a 2026 engineering benchmark update for robotics teams.",
            "why_relevant_to_query": "Selected because it discusses engineering research updates.",
            "claim_hints": [
                "Acme Research announced a 2026 engineering benchmark update for robotics teams."
            ]
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

    let empty = Vec::new();
    let missing_fields = diag
        .pointer("/evidence_quality/evidence_packet_contract/missing_fields")
        .and_then(Value::as_array)
        .unwrap_or(&empty);
    assert!(
        missing_fields
            .iter()
            .any(|row| row.as_str() == Some("source_identity_consistency")),
        "{diag:#?}"
    );
    assert!(
        diag.pointer("/evidence_quality/low_quality_flags")
            .and_then(Value::as_array)
            .unwrap_or(&empty)
            .iter()
            .any(|row| row.as_str() == Some("source_identity_mismatch")),
        "{diag:#?}"
    );
    assert_eq!(
        diag.pointer("/evidence_quality/source_quality_ready")
            .and_then(Value::as_bool),
        Some(false),
        "{diag:#?}"
    );
    assert_eq!(
        diag.pointer("/first_failed_gate").and_then(Value::as_str),
        Some("web_5d_source_quality_ready"),
        "{diag:#?}"
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
fn evidence_packet_contract_rejects_unaligned_source_backed_fragments() {
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
            "title": "Retail operations benchmark",
            "locator": "https://reports.example.com/retail-ops",
            "source_type": "industry_report",
            "source_kind": "industry_report",
            "source_domain": "reports.example.com",
            "snippet": "The report says store operators reduced checkout wait times by 18 percent after adding better queue staffing and aisle monitoring.",
            "relevant_extract": "The report says store operators reduced checkout wait times by 18 percent after adding better queue staffing and aisle monitoring.",
            "why_relevant_to_query": "Selected because it overlaps provider result terms: benchmark, staffing, monitoring.",
            "claim_hints": [
                "Store operators reduced checkout wait times by 18 percent after adding better queue staffing and aisle monitoring."
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
        diag.pointer("/evidence_quality/evidence_packet_contract/ready")
            .and_then(Value::as_bool),
        Some(false),
        "{diag:#?}"
    );
    let missing_fields = diag
        .pointer("/evidence_quality/evidence_packet_contract/missing_fields")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(
        missing_fields
            .iter()
            .any(|row| row.as_str() == Some("query_relevance_alignment")),
        "{diag:#?}"
    );
    assert_eq!(
        diag.pointer("/first_failed_gate").and_then(Value::as_str),
        Some("web_5d_source_quality_ready"),
        "{diag:#?}"
    );
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
fn source_quality_trusts_usable_pack_over_noisy_auxiliary_rows() {
    let mut noisy_auxiliary_claims = Vec::new();
    for index in 0..16 {
        noisy_auxiliary_claims.push(json!({
            "claim": format!("@charset \"UTF-8\"; modal overlay height:100% auxiliary row {index}"),
            "source_domain": "style-dump.example.com",
            "locator": format!("https://style-dump.example.com/{index}"),
            "title": "Auxiliary style dump",
            "support_snippet": "@charset \"UTF-8\"; .modal{height:100%;overflow:auto}",
            "confidence": "usable"
        }));
    }
    let payload = json!({
        "pending_tool_request": {
            "tool_key": "batch_query",
            "input": {
                "query": "research home blood pressure monitors and accuracy claims",
                "queries": ["research home blood pressure monitors and accuracy claims"],
                "keywords": ["home blood pressure monitors", "accuracy", "validation"]
            }
        },
        "tools": [{
            "status": "usable"
        }],
        "evidence_pack_quality": {
            "status": "usable",
            "usable_count": 6,
            "content_rich_item_count": 6,
            "claim_hint_count": 6,
            "source_domain_count": 5,
            "missing_facet_count": 0,
            "weak_facet_count": 0,
            "covered_facet_count": 1,
            "total_facet_count": 1,
            "covered_facet_ratio": 1.0,
            "coverage_thresholds_met": true,
            "low_confidence_count": 0,
            "candidate_only_count": 0,
            "thresholds": {
                "min_usable_items": 2,
                "min_source_domains": 2,
                "min_covered_facet_ratio_for_usable": 0.6
            }
        },
        "evidence_claims": noisy_auxiliary_claims,
        "evidence_pack": [
            {
                "title": "Validated blood pressure monitors",
                "locator": "https://health.example.com/home-bp-monitor-validation",
                "source_type": "medical_guidance",
                "source_domain": "health.example.com",
                "snippet": "The guidance explains that consumers should choose validated upper-arm monitors and compare home readings with a clinic measurement.",
                "relevant_extract": "The guidance explains that consumers should choose validated upper-arm monitors and compare home readings with a clinic measurement.",
                "why_relevant_to_query": "It directly addresses home blood pressure monitor accuracy and validation.",
                "claim_hints": ["Consumers should choose validated upper-arm monitors and compare home readings with a clinic measurement."]
            },
            {
                "title": "Monitor validation registry",
                "locator": "https://registry.example.org/validated-devices",
                "source_type": "primary_reference",
                "source_domain": "registry.example.org",
                "snippet": "The registry lists validated blood pressure devices and advises checking whether a model has passed an accepted validation protocol.",
                "relevant_extract": "The registry lists validated blood pressure devices and advises checking whether a model has passed an accepted validation protocol.",
                "why_relevant_to_query": "It gives source-backed evidence for which accuracy claims should be trusted.",
                "claim_hints": ["A validation registry can show whether a blood pressure monitor passed an accepted validation protocol."]
            }
        ]
    });
    let retrieval_quality = json!({
        "status": "usable",
        "candidate_count": 24,
        "evidence_count": 18,
        "content_rich_candidate_count": 6,
        "materialized_candidate_count": 6,
        "claim_hint_count": 18,
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

    assert!(
        diag.pointer("/evidence_quality/low_quality_evidence_rate")
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
            > 0.25,
        "{diag:#?}"
    );
    assert_eq!(
        diag.pointer("/evidence_quality/observed_source_quality_ready")
            .and_then(Value::as_bool),
        Some(false),
        "{diag:#?}"
    );
    assert_eq!(
        diag.pointer("/evidence_quality/source_quality_pack_ready")
            .and_then(Value::as_bool),
        Some(true),
        "{diag:#?}"
    );
    assert_eq!(
        diag.pointer("/evidence_quality/source_quality_ready")
            .and_then(Value::as_bool),
        Some(true),
        "{diag:#?}"
    );
    assert_ne!(
        diag.pointer("/first_failed_gate").and_then(Value::as_str),
        Some("web_5d_source_quality_ready"),
        "{diag:#?}"
    );
}

#[test]
fn source_sensitive_answerability_requires_multiple_authority_grade_domains() {
    let payload = json!({
        "pending_tool_request": {
            "tool_key": "batch_query",
            "input": {
                "query": "research current evidence on supplement use and patient outcomes",
                "queries": ["research current evidence on supplement use and patient outcomes"],
                "keywords": ["current evidence", "supplement", "patient outcomes"]
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
            "source_domain_count": 2,
            "missing_facet_count": 0,
            "weak_facet_count": 0,
            "covered_facet_count": 1,
            "total_facet_count": 1,
            "covered_facet_ratio": 1.0,
            "coverage_thresholds_met": true,
            "low_confidence_count": 0,
            "candidate_only_count": 0,
            "thresholds": {
                "min_usable_items": 2,
                "min_source_domains": 2
            }
        },
        "evidence_pack": [
            {
                "title": "Supplement evidence review",
                "locator": "https://research.example.edu/supplement-review",
                "source_type": "scholarly_or_research",
                "source_domain": "research.example.edu",
                "snippet": "The review reports that the supplement has plausible benefits for one outcome, but the evidence remains mixed across populations.",
                "relevant_extract": "The review reports that the supplement has plausible benefits for one outcome, but the evidence remains mixed across populations.",
                "why_relevant_to_query": "It directly addresses current evidence on supplement outcomes.",
                "claim_hints": ["The supplement has plausible benefits for one outcome, but evidence remains mixed across populations."]
            },
            {
                "title": "Supplement expert discussion",
                "locator": "https://video.example.com/supplement-discussion",
                "source_type": "consumer_video",
                "source_domain": "video.example.com",
                "snippet": "A clinician interview says some patients report benefits, but the discussion does not cite a second authority-grade source.",
                "relevant_extract": "A clinician interview says some patients report benefits, but the discussion does not cite a second authority-grade source.",
                "why_relevant_to_query": "It discusses patient supplement outcomes but is not an authority-grade source.",
                "claim_hints": ["Some patients report benefits, but the discussion does not cite a second authority-grade source."]
            }
        ]
    });
    let retrieval_quality = json!({
        "status": "usable",
        "candidate_count": 6,
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
        diag.pointer("/evidence_quality/source_quality_ready")
            .and_then(Value::as_bool),
        Some(true),
        "{diag:#?}"
    );
    assert_eq!(
        diag.pointer("/evidence_quality/source_authority_sensitive")
            .and_then(Value::as_bool),
        Some(true),
        "{diag:#?}"
    );
    assert_eq!(
        diag.pointer("/evidence_quality/source_authority_ready")
            .and_then(Value::as_bool),
        Some(false),
        "{diag:#?}"
    );
    assert_eq!(
        diag.pointer("/evidence_quality/authority_grade_source_domain_count")
            .and_then(Value::as_u64),
        Some(1),
        "{diag:#?}"
    );
    assert_eq!(
        diag.pointer("/evidence_quality/answerability_ready")
            .and_then(Value::as_bool),
        Some(false),
        "{diag:#?}"
    );
    assert_eq!(
        diag.pointer("/first_failed_gate").and_then(Value::as_str),
        Some("web_5g_answerability_ready"),
        "{diag:#?}"
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
        diag.pointer("/evidence_quality/bounded_answerability_ready")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        diag.pointer("/first_failed_gate").and_then(Value::as_str),
        Some("web_5g_answerability_ready")
    );
}

#[test]
fn bounded_answerability_accepts_thin_but_coherent_evidence_packages() {
    let payload = json!({
        "pending_tool_request": {
            "tool_key": "batch_query",
            "input": {
                "query": "practical data residency requirements for SaaS buyers selling into Europe and the US public sector",
                "queries": ["practical data residency requirements for SaaS buyers selling into Europe and the US public sector"],
                "keywords": ["data residency", "Europe", "US public sector", "FedRAMP", "NIS2"],
                "required_coverage": {
                    "entities": ["Europe", "US public sector", "FedRAMP", "NIS2"],
                    "facets": ["requirements", "procurement", "compliance", "operational impact"]
                }
            }
        },
        "tools": [{
            "status": "usable"
        }],
        "evidence_pack_quality": {
            "status": "thin",
            "usable_count": 4,
            "content_rich_item_count": 4,
            "claim_hint_count": 6,
            "source_domain_count": 4,
            "missing_facet_count": 1,
            "weak_facet_count": 2,
            "covered_facet_count": 4,
            "total_facet_count": 8,
            "covered_facet_ratio": 0.5,
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
                "title": "NIS2 scope",
                "locator": "https://commission.example.com/nis2",
                "source_type": "official_docs",
                "source_kind": "official_docs",
                "source_domain": "commission.example.com",
                "snippet": "NIS2 expands obligations for cloud and digital providers selling into Europe.",
                "relevant_extract": "NIS2 expands obligations for cloud and digital providers selling into Europe.",
                "why_relevant_to_query": "It directly affects SaaS compliance requirements for European buyers.",
                "claim_hints": ["NIS2 expands obligations for cloud and digital providers selling into Europe."]
            },
            {
                "title": "FedRAMP Rev5",
                "locator": "https://fedramp.example.com/rev5",
                "source_type": "official_docs",
                "source_kind": "official_docs",
                "source_domain": "fedramp.example.com",
                "snippet": "FedRAMP Rev5 requirements apply broadly to cloud services sold into the US public sector.",
                "relevant_extract": "FedRAMP Rev5 requirements apply broadly to cloud services sold into the US public sector.",
                "why_relevant_to_query": "It captures the practical compliance baseline for US public-sector SaaS sales.",
                "claim_hints": ["FedRAMP Rev5 requirements apply broadly to cloud services sold into the US public sector."]
            },
            {
                "title": "Sovereign cloud advisory",
                "locator": "https://lawfirm.example.com/sovereign-cloud",
                "source_type": "analysis",
                "source_kind": "web_page_enriched",
                "source_domain": "lawfirm.example.com",
                "snippet": "European buyers increasingly treat data residency and transfer controls as procurement gates rather than optional assurances.",
                "relevant_extract": "European buyers increasingly treat data residency and transfer controls as procurement gates rather than optional assurances.",
                "why_relevant_to_query": "It gives practical procurement context for Europe.",
                "claim_hints": ["European buyers increasingly treat data residency and transfer controls as procurement gates rather than optional assurances."]
            },
            {
                "title": "Public sector compliance overview",
                "locator": "https://publicsector.example.com/compliance",
                "source_type": "analysis",
                "source_kind": "web_page_enriched",
                "source_domain": "publicsector.example.com",
                "snippet": "StateRAMP and adjacent controls can add operational overhead even when FedRAMP is the headline requirement.",
                "relevant_extract": "StateRAMP and adjacent controls can add operational overhead even when FedRAMP is the headline requirement.",
                "why_relevant_to_query": "It adds operational impact context for US public-sector sales.",
                "claim_hints": ["StateRAMP and adjacent controls can add operational overhead even when FedRAMP is the headline requirement."]
            }
        ]
    });
    let retrieval_quality = json!({
        "status": "usable",
        "candidate_count": 18,
        "evidence_count": 4,
        "content_rich_candidate_count": 4,
        "materialized_candidate_count": 4,
        "claim_hint_count": 6,
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
        diag.pointer("/evidence_quality/bounded_answerability_ready")
            .and_then(Value::as_bool),
        Some(true),
        "{diag:#?}"
    );
    assert_eq!(
        diag.pointer("/evidence_quality/answerability_ready")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        diag.pointer("/first_failed_gate").and_then(Value::as_str),
        None
    );
}

#[test]
fn bounded_answerability_caps_required_facets_to_requested_facet_count() {
    let payload = json!({
        "pending_tool_request": {
            "tool_key": "batch_query",
            "input": {
                "query": "summarize current US state privacy law changes",
                "queries": ["summarize current US state privacy law changes"],
                "keywords": ["US state privacy laws", "2026", "consumer privacy"],
                "required_coverage": {
                    "facets": ["state privacy law landscape"]
                }
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
            "missing_facet_count": 0,
            "weak_facet_count": 0,
            "covered_facet_count": 1,
            "total_facet_count": 1,
            "covered_facet_ratio": 1.0,
            "coverage_thresholds_met": true,
            "low_confidence_count": 0,
            "candidate_only_count": 0,
            "thresholds": {
                "min_usable_items": 2,
                "min_source_domains": 2
            }
        },
        "evidence_pack": [
            {
                "title": "State privacy tracker",
                "locator": "https://privacy.example.com/tracker",
                "source_type": "analysis",
                "source_kind": "web_page_enriched",
                "source_domain": "privacy.example.com",
                "snippet": "Multiple US states have comprehensive consumer privacy laws in effect in 2026.",
                "relevant_extract": "Multiple US states have comprehensive consumer privacy laws in effect in 2026.",
                "why_relevant_to_query": "It addresses the requested state privacy law landscape.",
                "claim_hints": ["Multiple US states have comprehensive consumer privacy laws in effect in 2026."]
            },
            {
                "title": "Legal update",
                "locator": "https://law.example.com/privacy-2026",
                "source_type": "analysis",
                "source_kind": "web_page_enriched",
                "source_domain": "law.example.com",
                "snippet": "New state privacy laws took effect in 2026 and enforcement activity is increasing.",
                "relevant_extract": "New state privacy laws took effect in 2026 and enforcement activity is increasing.",
                "why_relevant_to_query": "It adds a second citable source on the same requested facet.",
                "claim_hints": ["New state privacy laws took effect in 2026 and enforcement activity is increasing."]
            },
            {
                "title": "Policy brief",
                "locator": "https://policy.example.com/privacy",
                "source_type": "analysis",
                "source_kind": "web_page_enriched",
                "source_domain": "policy.example.com",
                "snippet": "The US still lacks a uniform federal consumer privacy law, leaving state laws to drive compliance work.",
                "relevant_extract": "The US still lacks a uniform federal consumer privacy law, leaving state laws to drive compliance work.",
                "why_relevant_to_query": "It gives context for why the state-law landscape matters.",
                "claim_hints": ["The US lacks a uniform federal consumer privacy law, so state laws drive compliance work."]
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
        diag.pointer(
            "/evidence_quality/pack_thresholds/min_covered_facets_for_bounded_answerability"
        )
        .and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        diag.pointer("/evidence_quality/bounded_answerability_ready")
            .and_then(Value::as_bool),
        Some(true),
        "{diag:#?}"
    );
    assert_eq!(
        diag.pointer("/first_failed_gate").and_then(Value::as_str),
        None
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
