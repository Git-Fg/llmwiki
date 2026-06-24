use llmwiki_cli::core::markdown::{
    extract_footnote_refs, extract_footnotes, extract_wikilinks, parse_frontmatter,
};

#[test]
fn parse_simple_frontmatter() {
    let content = "---\ntitle: Hello\ntags: [a, b]\n---\n\nBody text.\n";
    let parsed = parse_frontmatter(content).unwrap();
    assert_eq!(parsed.frontmatter.unwrap().title.as_deref(), Some("Hello"));
    assert_eq!(parsed.body, "\nBody text.\n");
}

#[test]
fn parse_missing_frontmatter_returns_empty() {
    let parsed = parse_frontmatter("Just body text.\n").unwrap();
    assert!(parsed.frontmatter.is_none());
    assert_eq!(parsed.body, "Just body text.\n");
}

#[test]
fn parse_unclosed_frontmatter_returns_error() {
    let content = "---\ntitle: Bad\n\nBody without closing.\n";
    assert!(parse_frontmatter(content).is_err());
}

#[test]
fn parse_yaml_list_value() {
    let content = "---\ntags: [rust, cli]\nsources: [raw/a.md, raw/b.md]\n---\n\nBody.\n";
    let parsed = parse_frontmatter(content).unwrap();
    assert_eq!(
        parsed.frontmatter.unwrap().tags,
        vec!["rust".to_string(), "cli".to_string()]
    );
}

#[test]
fn extract_simple_wikilinks() {
    let body = "See [[attention]] and [[transformers|Transformers]] for context.";
    let links = extract_wikilinks(body);
    assert_eq!(links, vec!["attention", "transformers"]);
}

#[test]
fn extract_handles_no_wikilinks() {
    let body = "No links here.";
    assert!(extract_wikilinks(body).is_empty());
}

#[test]
fn extract_handles_nested_brackets() {
    let body = "[[a/b/c]]";
    assert_eq!(extract_wikilinks(body), vec!["a/b/c"]);
}

#[test]
fn extract_footnote_definitions() {
    let body = "Claim[^1].\n\n[^1]: source.pdf, p.3";
    let defs = extract_footnotes(body);
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0].id, "1");
    assert!(defs[0].body.contains("source.pdf"));
}

#[test]
fn extract_footnote_references() {
    let body = "First[^a] and second[^b].";
    let refs = extract_footnote_refs(body);
    assert_eq!(refs, vec!["a", "b"]);
}

#[test]
fn extract_handles_no_footnotes() {
    let body = "No footnotes here.";
    assert!(extract_footnotes(body).is_empty());
    assert!(extract_footnote_refs(body).is_empty());
}

#[test]
fn parse_frontmatter_returns_typed_struct_with_all_known_fields() {
    let md = "\
---
title: Hello
type: concept
tags: [a, b]
sources: [raw/page.md]
# confidence is polymorphic across wikis (numeric on minimax,
# string on mevin/mywiki/pharma). Preserved as String so both
# shapes round-trip; consumers parse to f64 if needed.
confidence: high
created: '2026-01-01'
updated: '2026-06-24'
schema_version: 2
status: stable
kind: reference
domain: pharma
maturity: reviewed
reviewed: '2026-05-01'
aliases: [hello-world]
description: A greeting
related: [other-page]
source_type: article
sha256: abc123
ingested: '2026-01-02'
name: hello
# descriptions is a locale → text map (minimax SKILL.md style).
# None of the other 3 wikis use this field.
descriptions:
  en: English description
  zh-Hans: 中文描述
---
Body content
";
    let parsed = parse_frontmatter(md).unwrap();
    let fm = parsed
        .frontmatter
        .expect("frontmatter block should be present");
    assert_eq!(fm.title.as_deref(), Some("Hello"));
    assert_eq!(fm.page_type.as_deref(), Some("concept"));
    assert_eq!(fm.tags, vec!["a", "b"]);
    assert_eq!(fm.sources, vec!["raw/page.md"]);
    assert_eq!(fm.confidence.as_deref(), Some("high"));
    assert_eq!(fm.created.as_deref(), Some("2026-01-01"));
    assert_eq!(fm.updated.as_deref(), Some("2026-06-24"));
    assert_eq!(fm.schema_version, Some(2));
    assert_eq!(fm.status.as_deref(), Some("stable"));
    assert_eq!(fm.kind.as_deref(), Some("reference"));
    assert_eq!(fm.domain.as_deref(), Some("pharma"));
    assert_eq!(fm.maturity.as_deref(), Some("reviewed"));
    assert_eq!(fm.reviewed.as_deref(), Some("2026-05-01"));
    assert_eq!(fm.aliases, vec!["hello-world"]);
    assert_eq!(fm.description.as_deref(), Some("A greeting"));
    assert_eq!(fm.related, vec!["other-page"]);
    assert_eq!(fm.source_type.as_deref(), Some("article"));
    assert_eq!(fm.sha256.as_deref(), Some("abc123"));
    assert_eq!(fm.ingested.as_deref(), Some("2026-01-02"));
    assert_eq!(fm.name.as_deref(), Some("hello"));
    assert_eq!(
        fm.descriptions,
        std::collections::BTreeMap::from([
            ("en".to_string(), "English description".to_string()),
            ("zh-Hans".to_string(), "中文描述".to_string()),
        ])
    );
    assert!(fm.extra.is_empty());
}

#[test]
fn parse_frontmatter_typo_lands_in_extra_not_known_field() {
    // "titel" (typo) is not a known field, so it lands in `extra`, not `title`.
    let md = "---\ntitel: Hello\n---\n";
    let parsed = parse_frontmatter(md).unwrap();
    let fm = parsed
        .frontmatter
        .expect("frontmatter block should be present");
    assert_eq!(
        fm.title, None,
        "typo 'titel' must NOT populate the typed `title` field"
    );
    assert_eq!(
        fm.extra.get("titel").and_then(|v| v.as_str()),
        Some("Hello"),
        "typo 'titel' must be preserved in the `extra` flatten bag",
    );
}

#[test]
fn parse_frontmatter_preserves_niche_extra_fields() {
    // `avatar` and `timezone` are niche fields (1-10 occurrences in the audit
    // across the 4 real flat-layout wikis). They are NOT typed fields, so they
    // must round-trip through `extra` without being rejected.
    let md = "\
---
title: X
avatar: /img/x.png
timezone: Europe/Paris
session: 2026-01-01T00:00:00Z
triggers: [a, b]
---
";
    let parsed = parse_frontmatter(md).unwrap();
    let fm = parsed
        .frontmatter
        .expect("frontmatter block should be present");
    assert_eq!(fm.title.as_deref(), Some("X"));
    assert_eq!(fm.extra.len(), 4);
    assert!(fm.extra.contains_key("avatar"));
    assert!(fm.extra.contains_key("timezone"));
    assert!(fm.extra.contains_key("session"));
    assert!(fm.extra.contains_key("triggers"));
}

#[test]
fn parse_frontmatter_rejects_duplicate_top_level_keys() {
    // Duplicate top-level keys must still error (`serde-saphyr` rejects them,
    // same as the previous `serde_yaml` dependency did). The
    // frontmatter-yaml-parse lint code depends on this.
    let md = "---\ntype: page\ntype: query\n---\n";
    assert!(
        parse_frontmatter(md).is_err(),
        "duplicate top-level keys must fail to parse",
    );
}

#[test]
fn parse_frontmatter_empty_block_yields_default_struct() {
    // Empty `---\n---` block: previously returned `Value::Null`. Now returns
    // `Some(Frontmatter::default())`. This is Decision #1 from Task 4+5:
    // we now distinguish "no block" (None) from "empty block" (Some(default())).
    let md = "---\n---\nBody after empty frontmatter";
    let parsed = parse_frontmatter(md).unwrap();
    let fm = parsed
        .frontmatter
        .expect("empty block must yield Some(default)");
    assert_eq!(fm.title, None);
    assert!(fm.tags.is_empty());
    assert!(fm.sources.is_empty());
    assert!(fm.extra.is_empty());
    assert_eq!(parsed.body, "Body after empty frontmatter");
}

#[test]
fn parse_frontmatter_handles_pipeline_type_expression() {
    // From minimax-code-wiki audit: a `type` value can be a pipeline
    // expression like `"entity | concept | comparison | query | schema |
    // summary"`. The typed struct uses `page_type: Option<String>` (not
    // `enum PageType`) precisely because of these expressions.
    let md = "---\ntype: entity | concept | comparison | query | schema | summary\n---\n";
    let parsed = parse_frontmatter(md).unwrap();
    let fm = parsed
        .frontmatter
        .expect("frontmatter block should be present");
    assert_eq!(
        fm.page_type.as_deref(),
        Some("entity | concept | comparison | query | schema | summary"),
    );
}

#[test]
fn parse_frontmatter_no_block_yields_none() {
    // No `---` block at all → `frontmatter: None`. Distinct from the
    // empty-block case (which yields `Some(default())`).
    let md = "Just body content, no frontmatter delimiter at all.\n";
    let parsed = parse_frontmatter(md).unwrap();
    assert!(parsed.frontmatter.is_none());
    assert_eq!(parsed.body, md);
}

#[test]
fn parse_frontmatter_accepts_string_form_for_tags_and_sources() {
    // MyWiki writes `tags` and `sources` as bare scalars (e.g.
    // `sources: raw/articles/foo.md`) instead of inline or block lists.
    // The typed struct normalizes both shapes to Vec<String> via the
    // `deserialize_string_or_vec` custom deserializer. Without this,
    // 100+ mywiki pages would fail to parse.
    let md = "\
---
title: Scalar tags and sources
tags: model
sources: raw/articles/single-source.md
---
Body
";
    let parsed = parse_frontmatter(md).unwrap();
    let fm = parsed.frontmatter.expect("frontmatter present");
    assert_eq!(fm.tags, vec!["model"]);
    assert_eq!(fm.sources, vec!["raw/articles/single-source.md"]);
}

#[test]
fn parse_frontmatter_accepts_numeric_or_string_confidence() {
    // `confidence` is polymorphic across wikis: numeric on minimax
    // (0.8, 0.85, 0.9, 0.95), string on mevin/mywiki/pharma
    // ("high", "medium", "low", or pipeline expressions like
    // "high | medium | low"). Preserved as String so both round-trip.
    for value in ["0.85", "high", "high | medium | low"] {
        let md = format!("---\ntitle: t\nconfidence: {value}\n---\n");
        let parsed = parse_frontmatter(&md).unwrap();
        let fm = parsed.frontmatter.expect("frontmatter present");
        assert_eq!(fm.confidence.as_deref(), Some(value), "input: {value}");
    }
}

#[test]
fn parse_frontmatter_accepts_descriptions_as_locale_map() {
    // minimax-code-wiki SKILL.md files use `descriptions:` as a
    // locale → text mapping (e.g. `descriptions: { zh-Hans: "..." }`).
    // Typed as BTreeMap<String, String> to capture this shape; none
    // of the other 3 wikis use this field.
    let md = "\
---
title: skill
descriptions:
  zh-Hans: 中文描述
  en: English description
---
Body
";
    let parsed = parse_frontmatter(md).unwrap();
    let fm = parsed.frontmatter.expect("frontmatter present");
    assert_eq!(
        fm.descriptions.get("zh-Hans").map(String::as_str),
        Some("中文描述")
    );
    assert_eq!(
        fm.descriptions.get("en").map(String::as_str),
        Some("English description")
    );
}
