pub(super) fn citation_artifact_summary(payload: &Value) -> Value {
    let mut seen = BTreeSet::<String>::new();
    let mut items = Vec::<Value>::new();
    for (pointer, artifact_path) in CITATION_ARTIFACT_POINTERS {
        if let Some(value) = payload.pointer(pointer) {
            collect_citation_artifact_items(value, artifact_path, 0, &mut seen, &mut items);
        }
        if items.len() >= 24 {
            break;
        }
    }
    json!({
        "schema_version": 1,
        "retained_count": items.len() as u64,
        "items": items,
        "note": "Compact citation/source/evidence refs retained with each eval row so answer quality can be inspected without opening raw session artifacts."
    })
}

fn collect_citation_artifact_items(
    value: &Value,
    artifact_path: &str,
    depth: usize,
    seen: &mut BTreeSet<String>,
    out: &mut Vec<Value>,
) {
    if depth > 6 || out.len() >= 24 {
        return;
    }
    match value {
        Value::Array(rows) => {
            for row in rows {
                collect_citation_artifact_items(row, artifact_path, depth + 1, seen, out);
                if out.len() >= 24 {
                    break;
                }
            }
        }
        Value::Object(map) => {
            if let Some((key, item)) = compact_citation_artifact_item(map, artifact_path) {
                if seen.insert(key) {
                    out.push(item);
                }
                return;
            }
            for key in [
                "citations",
                "sources",
                "source_refs",
                "evidence",
                "evidence_refs",
                "evidence_pack",
                "evidence_pack_candidates",
                "findings",
            ] {
                if let Some(child) = map.get(key) {
                    collect_citation_artifact_items(child, artifact_path, depth + 1, seen, out);
                    if out.len() >= 24 {
                        break;
                    }
                }
            }
        }
        _ => {}
    }
}

fn compact_citation_artifact_item(
    map: &serde_json::Map<String, Value>,
    artifact_path: &str,
) -> Option<(String, Value)> {
    let citation_id = artifact_string_field(map, &["citation_id", "id", "ref_id"]);
    let title = artifact_string_field(map, &["title", "name", "headline"]);
    let locator = artifact_string_field(map, &["locator", "url", "source_url", "href", "link"]);
    let source_domain = artifact_string_field(map, &["source_domain", "domain", "host"]);
    let source_kind = artifact_string_field(map, &["source_kind", "kind", "type"]);
    let provider = artifact_string_field(map, &["provider", "provider_name"]);
    let snippet = artifact_string_field(
        map,
        &[
            "snippet",
            "summary",
            "excerpt",
            "description",
            "text",
            "body",
            "content",
        ],
    );
    if citation_id.is_empty()
        && title.is_empty()
        && locator.is_empty()
        && source_domain.is_empty()
        && snippet.is_empty()
    {
        return None;
    }

    let identity = [
        citation_id.as_str(),
        locator.as_str(),
        title.as_str(),
        source_domain.as_str(),
        snippet.as_str(),
    ]
    .iter()
    .filter(|part| !part.is_empty())
    .copied()
    .collect::<Vec<_>>()
    .join("|");
    if identity.is_empty() {
        return None;
    }

    let mut out = serde_json::Map::new();
    out.insert(
        "artifact_path".to_string(),
        Value::String(artifact_path.to_string()),
    );
    insert_artifact_string(&mut out, "citation_id", &citation_id, 120);
    insert_artifact_string(&mut out, "title", &title, 240);
    insert_artifact_string(&mut out, "locator", &locator, 500);
    insert_artifact_string(&mut out, "source_domain", &source_domain, 160);
    insert_artifact_string(&mut out, "source_kind", &source_kind, 120);
    insert_artifact_string(&mut out, "provider", &provider, 120);
    insert_artifact_string(&mut out, "snippet", &snippet, 500);
    for key in ["confidence", "score", "rank", "used_for"] {
        if let Some(value) = map.get(key) {
            if value.is_string() || value.is_number() || value.is_boolean() {
                out.insert(key.to_string(), value.clone());
            }
        }
    }
    for key in ["quality_flags", "coverage_facets", "claim_hints"] {
        if let Some(value) = map.get(key) {
            if value.is_array() {
                out.insert(key.to_string(), value.clone());
            }
        }
    }
    Some((normalize_for_compare(&identity), Value::Object(out)))
}

fn artifact_string_field(map: &serde_json::Map<String, Value>, keys: &[&str]) -> String {
    keys.iter()
        .find_map(|key| {
            map.get(*key)
                .and_then(Value::as_str)
                .map(|raw| clean_text(raw, 1_000))
                .filter(|raw| !raw.is_empty())
        })
        .unwrap_or_default()
}

fn insert_artifact_string(
    out: &mut serde_json::Map<String, Value>,
    key: &str,
    value: &str,
    max_len: usize,
) {
    let cleaned = clean_text(value, max_len);
    if !cleaned.is_empty() {
        out.insert(key.to_string(), Value::String(cleaned));
    }
}

fn response_has_inline_citation_signal(response_text: &str) -> bool {
    let normalized = normalize_for_compare(response_text);
    [
        "http://",
        "https://",
        "source:",
        "sources:",
        "citation",
        "citations",
        "according to",
        "the docs",
        "official docs",
        "release notes",
        "changelog",
        "paper",
        "study",
    ]
    .iter()
    .any(|needle| normalized.contains(*needle))
        || text_contains_domain_like_source_marker(&normalized)
}

fn text_contains_domain_like_source_marker(text: &str) -> bool {
    text.split_whitespace().any(|token| {
        let cleaned = token
            .trim_matches(|ch: char| {
                !ch.is_ascii_alphanumeric() && ch != '.' && ch != '/' && ch != ':' && ch != '-'
            })
            .trim_start_matches("http://")
            .trim_start_matches("https://")
            .trim_start_matches("www.");
        let host = cleaned
            .split('/')
            .next()
            .unwrap_or("")
            .chars()
            .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '.' || *ch == '-')
            .collect::<String>();
        let labels = host
            .split('.')
            .filter(|label| !label.is_empty())
            .collect::<Vec<_>>();
        if labels.len() < 2 {
            return false;
        }
        let tld = labels.last().copied().unwrap_or("");
        if !(2..=24).contains(&tld.len()) || !tld.chars().all(|ch| ch.is_ascii_alphabetic()) {
            return false;
        }
        labels
            .iter()
            .any(|label| label.chars().any(|ch| ch.is_ascii_alphabetic()))
    })
}

