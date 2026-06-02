use infring_ops_core_v1::retrieval_policy::{
    candidate_coverage_facets_api, candidate_quality_flags_api, evidence_claims_from_pack_api,
    evidence_pack_from_ranked_candidates_api,
};
use serde_json::{json, Value};

fn semiconductor_query() -> &'static str {
    "Research the biggest semiconductor industry moves this month. Focus on developments that would matter to builders or investors, not generic stock chatter."
}

fn evidence_pack_for(candidate: Value) -> Value {
    evidence_pack_for_query(semiconductor_query(), candidate)
}

fn evidence_pack_for_query(query: &str, candidate: Value) -> Value {
    evidence_pack_from_ranked_candidates_api(
        &json!({}),
        query,
        &json!([]),
        2,
        &json!([{
            "candidate": candidate,
            "score": 1.0
        }]),
        3,
    )
}

fn usable_item_count(pack: &Value) -> usize {
    pack.as_array()
        .map(|rows| {
            rows.iter()
                .filter(|row| {
                    row.get("counts_as_usable_evidence")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                })
                .count()
        })
        .unwrap_or(0)
}

fn required_coverage_facets() -> Value {
    json!([
        {
            "id": "facet_01",
            "kind": "entity",
            "requested_text": "public sector",
            "terms": ["public", "sector"],
            "distinctive_terms": ["public", "sector"]
        },
        {
            "id": "facet_02",
            "kind": "facet",
            "requested_text": "data residency",
            "terms": ["data", "residency"],
            "distinctive_terms": ["data", "residency"]
        }
    ])
}

#[test]
fn instruction_scaffold_overlap_does_not_promote_off_intent_source() {
    let candidate = json!({
        "source_kind": "tavily_api_search_result",
        "title": "Loading...",
        "locator": "https://jira.atlassian.com/browse/CONFSERVER-90564",
        "snippet": "IMPORTANT: JAC is a Public system and anyone on the internet will be able to view the data in the created JAC tickets. Comments: Focus moves inappropriately when user dismisses the modal dialog. If it is fixed in a Server product, the resolution will be Fixed and the Fix Version field will indicate the product version that contains the fix.",
        "excerpt_hash": "focus-modal-dialog",
        "timestamp": infring_ops_core_v1::now_iso(),
        "permissions": "public_web;trusted_structured_feed",
        "status_code": 200
    });

    let pack = evidence_pack_for(candidate);
    assert_eq!(usable_item_count(&pack), 0, "{pack:#?}");
}

#[test]
fn topical_semiconductor_source_still_promotes_after_scaffold_filtering() {
    let now = infring_ops_core_v1::now_iso();
    let snippet = format!(
        "{now}. Semiconductor industry moves this month include AI memory, equipment, and EDA investment signals that matter to builders and investors."
    );
    let candidate = json!({
        "source_kind": "tavily_api_search_result",
        "title": "Semiconductor Industry Outlook 2026",
        "locator": "https://example.com/semiconductor-industry-outlook-2026",
        "snippet": snippet,
        "excerpt_hash": "semiconductor-outlook",
        "timestamp": now,
        "permissions": "public_web;trusted_structured_feed",
        "status_code": 200
    });

    let pack = evidence_pack_for(candidate);
    assert_eq!(usable_item_count(&pack), 1, "{pack:#?}");
}

#[test]
fn entity_only_rows_need_objective_anchor_when_facets_exist() {
    let facets = required_coverage_facets();
    let generic_entity = json!({
        "source_kind": "browser_materialized_page",
        "title": "Public sector definition",
        "locator": "https://example.edu/public-sector-definition",
        "snippet": "The public sector refers to any part of a state or national economy tied to government programs, services, agencies, and public employment.",
        "excerpt_hash": "generic-public-sector",
        "timestamp": infring_ops_core_v1::now_iso(),
        "permissions": "public_web;browser_materialized",
        "status_code": 200
    });
    let anchored_entity = json!({
        "source_kind": "browser_materialized_page",
        "title": "Public-sector SaaS data residency controls",
        "locator": "https://example.org/public-sector-data-residency",
        "snippet": "Public-sector SaaS vendors must document data residency controls, where customer data is stored, and how cross-border access is limited for government buyers.",
        "excerpt_hash": "anchored-public-sector",
        "timestamp": infring_ops_core_v1::now_iso(),
        "permissions": "public_web;browser_materialized",
        "status_code": 200
    });

    assert_eq!(
        candidate_coverage_facets_api(&facets, &generic_entity, 1),
        Vec::<String>::new()
    );
    assert_eq!(
        candidate_coverage_facets_api(&facets, &anchored_entity, 1),
        vec!["facet_01".to_string(), "facet_02".to_string()]
    );

    let pack = evidence_pack_from_ranked_candidates_api(
        &json!({}),
        "Research data-residency requirements for SaaS buyers selling into the public sector.",
        &facets,
        1,
        &json!([
            {"candidate": generic_entity, "score": 0.99},
            {"candidate": anchored_entity, "score": 0.92}
        ]),
        2,
    );
    let rows = pack.as_array().expect("evidence pack rows");
    assert_eq!(
        rows[0]
            .pointer("/counts_as_usable_evidence")
            .and_then(Value::as_bool),
        Some(false),
        "{pack:#?}"
    );
    assert!(
        rows[0]
            .pointer("/quality_flags")
            .and_then(Value::as_array)
            .is_some_and(|flags| flags
                .iter()
                .any(|flag| { flag.as_str() == Some("entity_facet_objective_anchor_missing") })),
        "{pack:#?}"
    );
    assert_eq!(
        rows[1]
            .pointer("/counts_as_usable_evidence")
            .and_then(Value::as_bool),
        Some(true),
        "{pack:#?}"
    );
}

#[test]
fn broad_current_structured_listing_with_current_claim_is_pack_ready() {
    let query = "Give me the biggest world news from this week.";
    let candidate = json!({
        "source_kind": "tavily_api_search_result",
        "title": "World News - Example Daily",
        "locator": "https://example.com/section/world",
        "snippet": "2. India's Hindu Right Has a New Hero: A 17th-Century Warrior King. 1h ago By Genevieve Glatsky. Supporters of Abelardo De La Espriella wore the yellow jerseys of the beloved national soccer team at a rally on May 31, 2026.",
        "excerpt_hash": "broad-current-listing",
        "timestamp": "2026-05-31T13:00:00Z",
        "permissions": "public_web",
        "status_code": 200
    });

    let flags = candidate_quality_flags_api(query, &candidate, 0.68);
    assert!(
        !flags.iter().any(|flag| flag == "listing_or_index_path"),
        "{flags:?}"
    );
    let pack = evidence_pack_for_query(query, candidate);
    assert_eq!(usable_item_count(&pack), 1, "{pack:#?}");
}

#[test]
fn access_denied_human_confirmation_copy_is_not_usable_evidence() {
    let query = "family-friendly neighborhoods to stay in Chicago for museums";
    let candidate = json!({
        "source_kind": "browser_materialized_page",
        "title": "Access to this page has been denied",
        "locator": "https://www.niche.com/places-to-live/search/best-neighborhoods-for-families/m/chicago-metro-area",
        "snippet": "Access to this page has been denied Press & Hold to confirm you are a human (and not a bot). Reference ID cdb78505-5e4d-11f1-a5bf-9554fd9390f7",
        "excerpt_hash": "niche-access-denied",
        "timestamp": infring_ops_core_v1::now_iso(),
        "permissions": "public_web;browser_materialized",
        "status_code": 200
    });

    let flags = candidate_quality_flags_api(query, &candidate, 0.92);
    assert!(flags.iter().any(|flag| flag == "junk_marker"), "{flags:#?}");

    let pack = evidence_pack_for_query(query, candidate);
    assert_eq!(usable_item_count(&pack), 0, "{pack:#?}");
}

#[test]
fn tag_inventory_listicle_shell_does_not_become_usable_evidence() {
    let query = "family-friendly neighborhoods to stay in Chicago for museums, transit access, and walkability";
    let candidate = json!({
        "source_kind": "tavily_api_search_result",
        "title": "Stay in Chicago Family Hotels + Neighborhood Guide",
        "locator": "https://example.test/chicago-neighborhoods-where-to-stay-with-kids",
        "snippet": "5 of our favorite Chicago neighborhoods that are best for families with kids + family hotel recommendations. .chicago .family travel // Family Travel | Travel with Kids | US Travel | USA | United States | Midwest Travel | Chicago Itinerary | Vacation Ideas | Things to Do | Best Neighborhoods |",
        "excerpt_hash": "listicle-tag-inventory",
        "timestamp": infring_ops_core_v1::now_iso(),
        "permissions": "public_web;trusted_structured_feed",
        "status_code": 200
    });

    let pack = evidence_pack_for_query(query, candidate);
    assert_eq!(usable_item_count(&pack), 0, "{pack:#?}");
    assert!(
        pack.pointer("/0/quality_flags")
            .and_then(Value::as_array)
            .is_some_and(|flags| flags
                .iter()
                .any(|flag| flag.as_str() == Some("malformed_evidence_material"))),
        "{pack:#?}"
    );
}

#[test]
fn adjacent_title_tail_is_trimmed_before_evidence_pack_promotion() {
    let query = "family-friendly neighborhoods to stay in Chicago for museums, transit access, and walkability";
    let candidate = json!({
        "source_kind": "tavily_api_search_result",
        "title": "Family-friendly Chicago neighborhood guide",
        "locator": "https://example.test/best-places-to-stay-in-chicago-for-families",
        "snippet": "Family-friendly areas like Lincoln Park, Old Town, and River North offer a blend of safety, proximity to transit, and access to cultural spots. Top 5 Family Friendly Neighborhoods in Chicago | Example Real Estate",
        "excerpt_hash": "adjacent-title-tail",
        "timestamp": infring_ops_core_v1::now_iso(),
        "permissions": "public_web;trusted_structured_feed",
        "status_code": 200
    });

    let pack = evidence_pack_for_query(query, candidate);
    let extract = pack
        .pointer("/0/relevant_extract")
        .and_then(Value::as_str)
        .unwrap_or("");
    let hints = pack
        .pointer("/0/claim_hints")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    assert_eq!(usable_item_count(&pack), 1, "{pack:#?}");
    assert!(extract.contains("Lincoln Park"), "{pack:#?}");
    assert!(!extract.contains("Top 5"), "{pack:#?}");
    assert!(
        hints
            .iter()
            .filter_map(Value::as_str)
            .all(|hint| !hint.contains("Top 5")),
        "{pack:#?}"
    );
}

#[test]
fn page_intro_extract_starts_at_answer_body_when_possible() {
    let query = "where to stay in Chicago near museums and transit";
    let candidate = json!({
        "source_kind": "tavily_api_search_result",
        "title": "Where to Stay in Chicago",
        "locator": "https://example.test/where-to-stay/chicago",
        "snippet": "Our guide to where to stay in Chicago uncovers and reveals the details of the most popular neighbourhoods for visitors to Chicago, offering useful tips and recommendations which we hope will help you match your travel plans to a neighbourhood’s atmosphere. The neighbourhoods of central Chicago, radiating around the Downtown area, are where you’ll find most of the highlights visitors are here for: the museums and galleries.",
        "excerpt_hash": "page-intro-body",
        "timestamp": infring_ops_core_v1::now_iso(),
        "permissions": "public_web;trusted_structured_feed",
        "status_code": 200
    });

    let pack = evidence_pack_for_query(query, candidate);
    let extract = pack
        .pointer("/0/relevant_extract")
        .and_then(Value::as_str)
        .unwrap_or("");

    assert_eq!(usable_item_count(&pack), 1, "{pack:#?}");
    assert!(
        extract.starts_with("The neighbourhoods of central Chicago"),
        "{pack:#?}"
    );
    assert!(!extract.starts_with("Our guide"), "{pack:#?}");
}

#[test]
fn social_community_source_does_not_back_ordinary_recommendation_evidence() {
    let query = "family-friendly neighborhoods to stay in Chicago for museums";
    let candidate = json!({
        "source_kind": "tavily_api_search_result",
        "title": "Safe areas to stay in Chicago for a family with kids",
        "locator": "https://www.facebook.com/groups/chicagotraveltips/posts/1840061113621067",
        "snippet": "These neighborhoods are close to top museums like the Field Museum, Shedd Aquarium, and the Museum of Science and Industry. Millennium Park is Neighborhood treasures.",
        "excerpt_hash": "facebook-community-chicago",
        "timestamp": infring_ops_core_v1::now_iso(),
        "permissions": "public_web;trusted_structured_feed",
        "status_code": 200
    });

    let flags = candidate_quality_flags_api(query, &candidate, 0.94);
    assert!(
        flags
            .iter()
            .any(|flag| flag == "social_or_community_source"),
        "{flags:#?}"
    );

    let pack = evidence_pack_for_query(query, candidate);
    assert_eq!(usable_item_count(&pack), 0, "{pack:#?}");
}

#[test]
fn social_community_source_can_back_explicit_sentiment_research() {
    let query = "What are travelers saying in social media and community reviews about family-friendly Chicago neighborhoods near museums?";
    let candidate = json!({
        "source_kind": "tavily_api_search_result",
        "title": "Travelers discuss family-friendly Chicago neighborhoods",
        "locator": "https://www.facebook.com/groups/chicagotraveltips/posts/1840061113621067",
        "snippet": "In a Chicago travel community discussion, multiple parents said neighborhoods near Millennium Park and the museum campus were easier for family trips because transit access and museum proximity reduced travel friction.",
        "excerpt_hash": "facebook-community-sentiment-chicago",
        "timestamp": infring_ops_core_v1::now_iso(),
        "permissions": "public_web;trusted_structured_feed",
        "status_code": 200
    });

    let flags = candidate_quality_flags_api(query, &candidate, 0.94);
    assert!(
        !flags
            .iter()
            .any(|flag| flag == "social_or_community_source"),
        "{flags:#?}"
    );

    let pack = evidence_pack_for_query(query, candidate);
    assert_eq!(usable_item_count(&pack), 1, "{pack:#?}");
}

#[test]
fn page_intro_inventory_copy_does_not_count_as_answerable_claim_material() {
    let query = "family-friendly neighborhoods to stay in a city for museums, transit access, and walkability";
    let candidate = json!({
        "source_kind": "tavily_api_search_result",
        "title": "Family-friendly city guide",
        "locator": "https://example.com/guides/family-friendly-city",
        "snippet": "Mountain lodges and desert resorts claim the most celebrated properties across the region. Family-Friendly City. Below is a selection of my family picks. Be in touch for more recommendations about neighborhoods near museums and transit.",
        "excerpt_hash": "page-intro-inventory-copy",
        "timestamp": infring_ops_core_v1::now_iso(),
        "permissions": "public_web;trusted_structured_feed",
        "status_code": 200
    });

    let pack = evidence_pack_for_query(query, candidate);
    assert_eq!(usable_item_count(&pack), 0, "{pack:#?}");
}

#[test]
fn concrete_recommendation_claim_still_counts_as_answerable_claim_material() {
    let query = "family-friendly neighborhoods to stay in a city for museums, transit access, and walkability";
    let candidate = json!({
        "source_kind": "tavily_api_search_result",
        "title": "Family-friendly city neighborhood guide",
        "locator": "https://example.com/guides/family-friendly-neighborhoods",
        "snippet": "Central neighborhoods near the museum campus are easier for family stays because they combine walkability, short transit rides, and direct access to major museums.",
        "excerpt_hash": "concrete-family-neighborhood-claim",
        "timestamp": infring_ops_core_v1::now_iso(),
        "permissions": "public_web;trusted_structured_feed",
        "status_code": 200
    });

    let pack = evidence_pack_for_query(query, candidate);
    assert_eq!(usable_item_count(&pack), 1, "{pack:#?}");
}

#[test]
fn list_intro_recommendation_copy_does_not_count_as_answerable_claim_material() {
    let query = "family-friendly neighborhoods to stay in a city for museums, transit access, and walkability";
    let candidate = json!({
        "source_kind": "tavily_api_search_result",
        "title": "Best family neighborhoods",
        "locator": "https://example.com/family-neighborhoods",
        "snippet": "These neighborhoods are as much fun to visit as they are to live in. Photo Credit: City Tourism Board. Every family's list of reasons is different, as is what they're looking for. We suggest 10 to put at the top of the consideration list with family-friendly offerings that are as much fun",
        "excerpt_hash": "generic-list-intro-recommendation-copy",
        "timestamp": infring_ops_core_v1::now_iso(),
        "permissions": "public_web;trusted_structured_feed",
        "status_code": 200
    });

    let pack = evidence_pack_for_query(query, candidate);
    assert_eq!(usable_item_count(&pack), 0, "{pack:#?}");
}

#[test]
fn direct_recommendation_copy_still_counts_as_answerable_claim_material() {
    let query = "family-friendly neighborhoods to stay in a city for museums, transit access, and walkability";
    let candidate = json!({
        "source_kind": "tavily_api_search_result",
        "title": "Best family neighborhoods",
        "locator": "https://example.com/family-neighborhoods",
        "snippet": "The guide suggests staying in central museum-district neighborhoods because they keep families within walking distance of major museums and near multiple transit lines.",
        "excerpt_hash": "direct-family-neighborhood-recommendation-copy",
        "timestamp": infring_ops_core_v1::now_iso(),
        "permissions": "public_web;trusted_structured_feed",
        "status_code": 200
    });

    let pack = evidence_pack_for_query(query, candidate);
    assert_eq!(usable_item_count(&pack), 1, "{pack:#?}");
}

#[test]
fn adjacent_forum_title_tail_is_trimmed_from_claim_hints() {
    let query = "family-friendly neighborhoods to stay in a city for museums, transit access, and walkability";
    let candidate = json!({
        "source_kind": "tavily_api_search_result",
        "title": "Where to stay in the city",
        "locator": "https://example.com/where-to-stay",
        "snippet": "Central neighborhoods near the downtown area are where families will find most museum highlights, short transit rides, walkable blocks, family restaurants, and easy hotel access and odd Where to Stay in the City (Main Thread) – City Forum – Travel L",
        "excerpt_hash": "adjacent-forum-title-tail",
        "timestamp": infring_ops_core_v1::now_iso(),
        "permissions": "public_web;trusted_structured_feed",
        "status_code": 200
    });

    let pack = evidence_pack_for_query(query, candidate);
    assert_eq!(usable_item_count(&pack), 1, "{pack:#?}");
    let claim = pack
        .pointer("/0/claim_hints/0")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(
        claim.contains("Central neighborhoods near the downtown area"),
        "{pack:#?}"
    );
    assert!(!claim.contains("Main Thread"), "{pack:#?}");
    assert!(!claim.contains("Forum"), "{pack:#?}");
}

#[test]
fn adjacent_title_tail_is_trimmed_from_relevant_extract() {
    let query = "family-friendly neighborhoods to stay in a city for museums, transit access, and walkability";
    let candidate = json!({
        "source_kind": "tavily_api_search_result",
        "title": "Where to stay in the city",
        "locator": "https://example.com/where-to-stay",
        "snippet": "Central neighborhoods near the downtown area are where families will find most museum highlights, short transit rides, and walkable streets and odd Where to Stay in the City",
        "excerpt_hash": "adjacent-title-tail-extract",
        "timestamp": infring_ops_core_v1::now_iso(),
        "permissions": "public_web;trusted_structured_feed",
        "status_code": 200
    });

    let pack = evidence_pack_for_query(query, candidate);
    assert_eq!(usable_item_count(&pack), 1, "{pack:#?}");
    let extract = pack
        .pointer("/0/relevant_extract")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(
        extract.contains("Central neighborhoods near the downtown area"),
        "{pack:#?}"
    );
    assert!(!extract.contains("Where to Stay"), "{pack:#?}");
}

#[test]
fn adjacent_title_marker_tail_is_trimmed_from_claim_hints() {
    let query =
        "family-friendly neighborhoods to stay in a city for museums, transit access, and walkability";
    let candidate = json!({
        "source_kind": "tavily_api_search_result",
        "title": "Where to stay in the city",
        "locator": "https://example.com/where-to-stay",
        "snippet": "The neighbourhoods of central city, radiating around the downtown area, are where families will find most of the highlights visitors are here for: the museums and gall Where to Stay",
        "excerpt_hash": "adjacent-title-marker-tail-claim",
        "timestamp": infring_ops_core_v1::now_iso(),
        "permissions": "public_web;trusted_structured_feed",
        "status_code": 200
    });

    let pack = evidence_pack_for_query(query, candidate);
    assert_eq!(usable_item_count(&pack), 1, "{pack:#?}");
    let claim = pack
        .pointer("/0/claim_hints/0")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(
        claim.contains("The neighbourhoods of central city"),
        "{pack:#?}"
    );
    assert!(!claim.contains("Where to Stay"), "{pack:#?}");
    assert!(!claim.contains("gall"), "{pack:#?}");
}

#[test]
fn symbol_glued_adjacent_title_tail_is_trimmed_from_evidence_material() {
    let query =
        "family-friendly neighborhoods to stay in a city for museums, transit access, and walkability";
    let candidate = json!({
        "source_kind": "tavily_api_search_result",
        "title": "Where to Stay in the City",
        "locator": "https://example.com/where-to-stay",
        "snippet": "The neighbourhoods of central city, radiating around the downtown area, are where families will find most of the highlights visitors are here for: the museums and gall ️ Where to Stay in the City: Guide with 11 TOP Areas! (+Map)",
        "excerpt_hash": "symbol-glued-adjacent-title-tail",
        "timestamp": infring_ops_core_v1::now_iso(),
        "permissions": "public_web;trusted_structured_feed",
        "status_code": 200
    });

    let pack = evidence_pack_for_query(query, candidate);
    assert_eq!(usable_item_count(&pack), 1, "{pack:#?}");
    let extract = pack
        .pointer("/0/relevant_extract")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(
        extract.contains("The neighbourhoods of central city"),
        "{pack:#?}"
    );
    assert!(!extract.contains("Where to Stay"), "{pack:#?}");
    assert!(!extract.contains("gall"), "{pack:#?}");
    let claim_hints = pack
        .pointer("/0/claim_hints")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(
        claim_hints
            .iter()
            .filter_map(Value::as_str)
            .all(|claim| !claim.contains("Where to Stay") && !claim.contains("gall")),
        "{pack:#?}"
    );
}

#[test]
fn dangling_fragment_source_title_falls_back_to_domain_label() {
    let query =
        "family-friendly neighborhoods to stay in a city for museums, transit access, and walkability";
    let candidate = json!({
        "source_kind": "tavily_api_search_result",
        "title": "and gall Where to Stay in the City - A Local's Neighborhood Guide",
        "locator": "https://local.example/blog/where-to-stay",
        "snippet": "For visitors, central neighborhoods near downtown keep families close to major museums, transit stations, walkable restaurants, and short rides between activities.",
        "excerpt_hash": "dangling-fragment-source-title",
        "timestamp": infring_ops_core_v1::now_iso(),
        "permissions": "public_web;trusted_structured_feed",
        "status_code": 200
    });

    let pack = evidence_pack_for_query(query, candidate);
    assert_eq!(usable_item_count(&pack), 1, "{pack:#?}");
    assert_eq!(
        pack.pointer("/0/title").and_then(Value::as_str),
        Some("Web result from local.example"),
        "{pack:#?}"
    );

    let claims = evidence_claims_from_pack_api(&json!({}), &pack, 3);
    assert_eq!(
        claims.pointer("/0/source_title").and_then(Value::as_str),
        Some("Web result from local.example"),
        "{claims:#?}"
    );
}

#[test]
fn page_debris_source_title_falls_back_without_blocking_clean_evidence() {
    let query =
        "family-friendly neighborhoods to stay in a city for museums, transit access, and walkability";
    let candidate = json!({
        "source_kind": "tavily_api_search_result",
        "title": "visit as BEST PLACES TO STAY IN THE CITY FOR FAMILIES - example",
        "locator": "https://family.example/best-places-to-stay",
        "snippet": "Family-friendly areas like River North, Old Town, and Lincoln Park offer a blend of safety, proximity to transit, and access to cultural spots.",
        "excerpt_hash": "page-debris-source-title",
        "timestamp": infring_ops_core_v1::now_iso(),
        "permissions": "public_web;trusted_structured_feed",
        "status_code": 200
    });

    let pack = evidence_pack_for_query(query, candidate);
    assert_eq!(usable_item_count(&pack), 1, "{pack:#?}");
    assert_eq!(
        pack.pointer("/0/title").and_then(Value::as_str),
        Some("Web result from family.example"),
        "{pack:#?}"
    );
    assert!(
        pack.pointer("/0/claim_hints/0")
            .and_then(Value::as_str)
            .is_some_and(|claim| claim.contains("River North")),
        "{pack:#?}"
    );
}

#[test]
fn lowercase_fragment_title_lead_falls_back_without_blocking_clean_evidence() {
    let query =
        "family-friendly neighborhoods to stay in a city for museums, transit access, and walkability";
    let candidate = json!({
        "source_kind": "tavily_api_search_result",
        "title": "safety affordability BEST PLACES TO STAY IN THE CITY FOR FAMILIES - example",
        "locator": "https://planning.example/best-family-neighborhoods",
        "snippet": "Family-friendly areas with safer streets, transit access, and walkable cultural attractions help families reduce travel friction during museum-focused trips.",
        "excerpt_hash": "lowercase-fragment-source-title",
        "timestamp": infring_ops_core_v1::now_iso(),
        "permissions": "public_web;trusted_structured_feed",
        "status_code": 200
    });

    let pack = evidence_pack_for_query(query, candidate);
    assert_eq!(usable_item_count(&pack), 1, "{pack:#?}");
    assert_eq!(
        pack.pointer("/0/title").and_then(Value::as_str),
        Some("Web result from planning.example"),
        "{pack:#?}"
    );
    assert!(
        pack.pointer("/0/claim_hints/0")
            .and_then(Value::as_str)
            .is_some_and(|claim| claim.contains("Family-friendly areas")),
        "{pack:#?}"
    );
}

#[test]
fn adjacent_title_marker_suffix_is_trimmed_from_relevant_extract() {
    let query =
        "family-friendly neighborhoods to stay in a city for museums, transit access, and walkability";
    let candidate = json!({
        "source_kind": "tavily_api_search_result",
        "title": "Family hotels in the city",
        "locator": "https://example.com/family-hotels",
        "snippet": "For families visiting the city, the best areas to stay are Lincoln Park and Streeterville, as both offer excellent access to parks, museums, and family-friendly Where to Stay in the City Guide",
        "excerpt_hash": "adjacent-title-marker-tail-extract",
        "timestamp": infring_ops_core_v1::now_iso(),
        "permissions": "public_web;trusted_structured_feed",
        "status_code": 200
    });

    let pack = evidence_pack_for_query(query, candidate);
    assert_eq!(usable_item_count(&pack), 1, "{pack:#?}");
    let extract = pack
        .pointer("/0/relevant_extract")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(
        extract.contains("the best areas to stay are Lincoln Park and Streeterville"),
        "{pack:#?}"
    );
    assert!(!extract.contains("Where to Stay"), "{pack:#?}");
    assert!(!extract.contains("family-friendly"), "{pack:#?}");
}

#[test]
fn leading_page_title_prefix_is_trimmed_from_relevant_extract() {
    let query =
        "family-friendly neighborhoods to stay in a city for museums, transit access, and walkability";
    let candidate = json!({
        "source_kind": "tavily_api_search_result",
        "title": "Local neighborhood guide",
        "locator": "https://example.com/local-neighborhood-guide",
        "snippet": ". Where to Stay in the City - A Local Neighborhood Guide. For visitors, central neighborhoods near downtown keep families close to museums, transit, walkable restaurants, and short rides between activities. But, if travelers stay longer and",
        "excerpt_hash": "leading-page-title-prefix-extract",
        "timestamp": infring_ops_core_v1::now_iso(),
        "permissions": "public_web;trusted_structured_feed",
        "status_code": 200
    });

    let pack = evidence_pack_for_query(query, candidate);
    assert_eq!(usable_item_count(&pack), 1, "{pack:#?}");
    let extract = pack
        .pointer("/0/relevant_extract")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(extract.starts_with("For visitors"), "{pack:#?}");
    assert!(!extract.contains("Where to Stay"), "{pack:#?}");
    assert!(
        !extract.contains("But, if travelers stay longer"),
        "{pack:#?}"
    );
}

#[test]
fn incomplete_terminal_claim_fragment_is_not_promoted() {
    let query =
        "family-friendly neighborhoods to stay in a city for museums, transit access, and walkability";
    let candidate = json!({
        "source_kind": "tavily_api_search_result",
        "title": "Most walkable neighborhoods",
        "locator": "https://example.com/walkable-neighborhoods",
        "snippet": "River North is one of the most walkable neighborhoods in the city, with parks, museums, transit stops, restaurants, and everyday services close together. Lincoln Park is one of the city's most",
        "excerpt_hash": "incomplete-terminal-claim-fragment",
        "timestamp": infring_ops_core_v1::now_iso(),
        "permissions": "public_web;trusted_structured_feed",
        "status_code": 200
    });

    let pack = evidence_pack_for_query(query, candidate);
    assert_eq!(usable_item_count(&pack), 1, "{pack:#?}");
    let extract = pack
        .pointer("/0/relevant_extract")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(
        extract.contains("River North is one of the most walkable neighborhoods"),
        "{pack:#?}"
    );
    assert!(!extract.contains("Lincoln Park is one of"), "{pack:#?}");
    let claim_hints = pack
        .pointer("/0/claim_hints")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(
        claim_hints
            .iter()
            .filter_map(Value::as_str)
            .all(|claim| !claim.contains("Lincoln Park is one of")),
        "{pack:#?}"
    );
}
