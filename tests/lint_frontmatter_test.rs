use llmwiki_cli::lint::frontmatter::check_frontmatter;

#[test]
fn missing_frontmatter_is_error() {
    let issues = check_frontmatter("wiki/a.md", "Just body, no frontmatter.\n");
    assert!(issues
        .iter()
        .any(|i| i.code == "missing-frontmatter" && i.severity == "error"));
}

#[test]
fn missing_schema_version_is_error() {
    let content = "---\ntitle: X\ncreated: 2026-01-01\nupdated: 2026-01-01\ntype: concept\ntags: [concept]\nsources: []\n---\n\nBody\n";
    let issues = check_frontmatter("wiki/a.md", content);
    assert!(issues.iter().any(|i| i.code == "missing-schema-version"));
}

#[test]
fn missing_title_is_error() {
    let content = "---\nschema_version: 1\ncreated: 2026-01-01\nupdated: 2026-01-01\ntype: concept\ntags: [x]\nsources: [a.md]\n---\n\nBody\n";
    let issues = check_frontmatter("wiki/a.md", content);
    assert!(issues.iter().any(|i| i.code == "missing-title"));
}

#[test]
fn missing_type_is_error() {
    let content = "---\nschema_version: 1\ntitle: X\ncreated: 2026-01-01\nupdated: 2026-01-01\ntags: [x]\nsources: []\n---\n\nBody\n";
    let issues = check_frontmatter("wiki/a.md", content);
    assert!(issues.iter().any(|i| i.code == "missing-type"));
}

#[test]
fn invalid_type_value_is_error() {
    let content = "---\nschema_version: 1\ntitle: X\ncreated: 2026-01-01\nupdated: 2026-01-01\ntype: notes\ntags: [x]\nsources: []\n---\n\nBody\n";
    let issues = check_frontmatter("wiki/a.md", content);
    assert!(issues.iter().any(|i| i.code == "invalid-type"));
}

#[test]
fn unknown_tag_is_error() {
    let content = "---\nschema_version: 1\ntitle: X\ncreated: 2026-01-01\nupdated: 2026-01-01\ntype: concept\ntags: [bogus-tag-999]\nsources: []\n---\n\nBody\n";
    let issues = check_frontmatter("wiki/a.md", content);
    assert!(issues.iter().any(|i| i.code == "unknown-tag"));
}

#[test]
fn duplicate_tags_is_error() {
    let content = "---\nschema_version: 1\ntitle: X\ncreated: 2026-01-01\nupdated: 2026-01-01\ntype: concept\ntags: [concept, concept]\nsources: []\n---\n\nBody\n";
    let issues = check_frontmatter("wiki/a.md", content);
    assert!(issues.iter().any(|i| i.code == "duplicate-tag"));
}

#[test]
fn missing_sources_is_error() {
    let content = "---\nschema_version: 1\ntitle: X\ncreated: 2026-01-01\nupdated: 2026-01-01\ntype: concept\ntags: [x]\n---\n\nBody\n";
    let issues = check_frontmatter("wiki/a.md", content);
    assert!(issues.iter().any(|i| i.code == "missing-sources"));
}

#[test]
fn bad_filename_is_error() {
    let content = "---\nschema_version: 1\ntitle: X\ncreated: 2026-01-01\nupdated: 2026-01-01\ntype: concept\ntags: [x]\nsources: []\n---\n\nBody\n";
    let issues = check_frontmatter("wiki/BadName.md", content);
    assert!(issues.iter().any(|i| i.code == "bad-filename"));
}

#[test]
fn good_page_passes() {
    let content = "---\nschema_version: 1\ntitle: Good Page\ncreated: 2026-01-01\nupdated: 2026-01-01\ntype: concept\ntags: [concept, software]\nsources: [raw/articles/src.md]\n---\n\nBody\n";
    let issues = check_frontmatter("wiki/good-page.md", content);
    assert!(issues.is_empty(), "{issues:?}");
}
