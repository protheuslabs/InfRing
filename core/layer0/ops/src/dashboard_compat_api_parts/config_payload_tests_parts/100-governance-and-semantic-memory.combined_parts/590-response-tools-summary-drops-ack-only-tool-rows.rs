fn response_tools_summary_drops_ack_only_tool_rows() {
    let synthesized = response_tools_summary_for_user(
        &[json!({
            "name": "web_search",
            "is_error": false,
            "result": "Web search completed."
        })],
        4,
    );
    assert!(synthesized.is_empty());
}

#[test]
fn response_tools_summary_drops_promotional_title_shell_snippets() {
    let synthesized = response_tools_summary_for_user(
        &[json!({
            "name": "batch_query",
            "is_error": false,
            "evidence_refs": [
                {
                    "title": "Web result from fool.com",
                    "snippet": "Out Now Motley Fool Stock Advisor's list of Top 10 Stocks to BUY NOW. These are the stocks our analysts believe are the best positioned to beat the market."
                },
                {
                    "title": "Chicago neighborhood guide",
                    "snippet": "Lincoln Park and the Near North Side give families strong museum access, CTA connectivity, and easy walking routes."
                }
            ]
        })],
        4,
    );
    assert!(!synthesized.is_empty());
    assert!(synthesized.contains("Lincoln Park"));
    assert!(!synthesized.contains("Motley Fool"));
    assert!(!synthesized.contains("Top 10 Stocks"));
}

#[test]
