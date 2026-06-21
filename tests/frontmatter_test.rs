use wiki::core::markdown::{
    parse_frontmatter, extract_wikilinks, extract_footnotes, extract_footnote_refs
};

#[test]
fn parse_simple_frontmatter() {
    let content = "---\ntitle: Hello\ntags: [a, b]\n---\n\nBody text.\n";
    let parsed = parse_frontmatter(content).unwrap();
    assert_eq!(parsed.frontmatter.get("title").unwrap(), "Hello");
    assert_eq!(parsed.body, "\nBody text.\n");
}

#[test]
fn parse_missing_frontmatter_returns_empty() {
    let parsed = parse_frontmatter("Just body text.\n").unwrap();
    assert!(parsed.frontmatter.is_null());
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
    let tags: Vec<String> = parsed.frontmatter.get("tags").unwrap()
        .as_sequence().unwrap()
        .iter().map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(tags, vec!["rust", "cli"]);
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
