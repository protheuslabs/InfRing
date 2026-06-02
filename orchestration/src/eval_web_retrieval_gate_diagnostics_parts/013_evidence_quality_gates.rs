{
    gates.extend([
        web_gate(
            "web_5b_content_rich_candidates_present",
            packaged_evidence_present,
            content_rich_candidates_present,
            if content_rich_candidates_present {
                "packaged evidence includes materialized, content-rich candidate text rather than only thin search rows"
            } else if packaged_evidence_present {
                "packaged evidence exists but no content-rich candidate text was visible"
            } else {
                "content-rich candidates cannot be expected before packaged evidence"
            },
            vec![
                "retrieval_quality.materialized_candidate_count".to_string(),
                "retrieval_quality.content_rich_candidate_count".to_string(),
                "tool_result_quality.content_rich_candidate_count".to_string(),
                "tool_result_quality.materialized_candidate_count".to_string(),
                "evidence_pack_quality.content_rich_item_count".to_string(),
                "evidence_pack_quality.materialized_item_count".to_string(),
            ],
        ),
        web_gate(
            "web_5c_claim_extraction_present",
            packaged_evidence_present,
            claim_extraction_present,
            if claim_extraction_present {
                "packaged evidence includes extracted claim hints or equivalent claim-level facts for synthesis"
            } else if packaged_evidence_present {
                "packaged evidence exists but no claim hints or equivalent claim extraction were visible"
            } else {
                "claim extraction cannot be expected before packaged evidence"
            },
            vec![
                "retrieval_quality.claim_hint_count".to_string(),
                "retrieval_quality.classification_inputs.direct_contract_present".to_string(),
                "retrieval_quality.classification_inputs.direct_evidence_claim_count".to_string(),
                "tool_result_quality.claim_hint_count".to_string(),
                "evidence_pack.claim_hints".to_string(),
                "evidence_claims".to_string(),
            ],
        ),
        web_gate(
            "web_5d_source_quality_ready",
            packaged_evidence_present,
            source_quality_ready,
            if source_quality_ready {
                "packaged evidence contains source-backed, non-dominantly-low-quality material"
            } else if packaged_evidence_present {
                "packaged evidence exists but appears source-thin, low-confidence, boilerplate-heavy, or candidate-only"
            } else {
                "source quality cannot be assessed before packaged evidence exists"
            },
            evidence_quality_refs(&evidence_quality),
        ),
        web_gate(
            "web_5e_claim_quality_ready",
            claim_extraction_present,
            claim_quality_ready,
            if claim_quality_ready {
                "claim extraction produced concrete answer material rather than only titles, source labels, or boilerplate fragments"
            } else if claim_extraction_present {
                "claim extraction exists but the extracted strings look too thin, source-only, title-like, or boilerplate-heavy for synthesis"
            } else {
                "claim quality cannot be assessed before claim extraction exists"
            },
            evidence_quality_refs(&evidence_quality),
        ),
        web_gate(
            "web_5f_citation_renderability_ready",
            claim_extraction_present,
            citation_renderability_ready,
            if citation_renderability_ready {
                "claim-level or evidence-level material has enough source locator/title/domain data to render citations"
            } else if claim_extraction_present {
                "claims exist but do not carry enough source locator, title, or domain data for reliable citation rendering"
            } else {
                "citation renderability cannot be assessed before claim extraction exists"
            },
            evidence_quality_refs(&evidence_quality),
        ),
        web_gate(
            "web_5g_answerability_ready",
            packaged_evidence_present && claim_extraction_present,
            answerability_ready,
            if answerability_ready {
                "evidence has enough clean source material, concrete claims, citation data, and any source-sensitive authority coverage for a bounded useful answer"
            } else if packaged_evidence_present && claim_extraction_present {
                "evidence and claims exist, but the package is not yet strong enough for a coherent source-backed answer, often because coverage, claim, citation, or source-authority support is thin"
            } else {
                "answerability cannot be assessed before evidence packaging and claim extraction both exist"
            },
            evidence_quality_refs(&evidence_quality),
        ),
        web_gate(
            "web_5h_evidence_packet_contract_ready",
            packaged_evidence_present && claim_extraction_present && answerability_ready,
            evidence_packet_contract_ready,
            if evidence_packet_contract_ready {
                "at least half of selected evidence items carry source identity, source type, extract, concrete claim material, and query-relevance rationale"
            } else if packaged_evidence_present && claim_extraction_present && answerability_ready {
                "evidence appears answerable, but selected packets do not preserve enough source, extract, claim, and relevance fields for chat-safe synthesis"
            } else {
                "evidence packet contract cannot be assessed before answerable evidence exists"
            },
            evidence_quality_refs(&evidence_quality),
        ),
        web_gate(
            "web_5i_malformed_evidence_absent",
            packaged_evidence_present && claim_extraction_present,
            malformed_evidence_clean,
            if malformed_evidence_clean {
                "selected evidence text is free of stitched title tails, page chrome, and malformed claim fragments"
            } else {
                "selected evidence includes malformed answer material such as stitched title tails, page chrome, or clipped claim fragments"
            },
            evidence_quality_refs(&evidence_quality),
        ),
        web_gate(
            "web_5j_citation_titles_clean",
            citation_renderability_ready,
            citation_titles_clean,
            if citation_titles_clean {
                "citation titles are clean source labels rather than dangling fragments or page debris"
            } else {
                "citation metadata is renderable but some visible source titles contain dangling fragments or page debris"
            },
            evidence_quality_refs(&evidence_quality),
        ),
        web_gate(
            "web_7_usable_evidence_available",
            packaged_evidence_present || tool_attempted,
            usable_evidence
                && content_rich_candidates_present
                && claim_extraction_present
                && answerability_ready
                && evidence_packet_contract_ready
                && malformed_evidence_clean
                && citation_titles_clean,
            if usable_evidence
                && content_rich_candidates_present
                && claim_extraction_present
                && answerability_ready
                && evidence_packet_contract_ready
                && malformed_evidence_clean
                && citation_titles_clean
            {
                "retrieval quality classifies the packaged, materialized, claim-bearing, citation-ready evidence packet as usable"
            } else {
                "packaged output exists only as thin, unmaterialized, claim-poor, citation-poor, malformed, citation-title-debris, low-signal/no-results/degraded evidence, lacks the evidence-packet contract, or no usable evidence was available"
            },
            vec![
                "retrieval_quality.usable_evidence".to_string(),
                "retrieval_quality.status".to_string(),
                "retrieval_quality.materialized_candidate_count".to_string(),
                "retrieval_quality.content_rich_candidate_count".to_string(),
                "retrieval_quality.claim_hint_count".to_string(),
                "evidence_quality.evidence_packet_contract".to_string(),
                "evidence_quality.malformed_evidence_clean".to_string(),
                "evidence_quality.citation_titles_clean".to_string(),
            ],
        ),
        web_gate(
            "web_8_evidence_context_to_synthesis",
            packaged_evidence_present,
            evidence_context_to_synthesis,
            if evidence_context_to_synthesis {
                "evidence context reached the synthesis/finalization boundary"
            } else if packaged_evidence_present {
                "packaged evidence exists but synthesis context marker is absent"
            } else {
                "synthesis evidence context cannot be expected without packaged evidence"
            },
            vec!["5e_agent_received_evidence_context".to_string()],
        ),
    ]);
}
