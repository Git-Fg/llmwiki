use assert_cmd::Command;
use predicates::str;

#[test]
fn lint_detects_missing_frontmatter() {
    let tmp = tempfile::tempdir().unwrap();
    let wiki = tmp.path();
    std::fs::create_dir_all(wiki.join("wiki")).unwrap();
    std::fs::write(wiki.join("wiki/bad-page.md"), "No frontmatter here.\n").unwrap();
    std::fs::write(wiki.join("index.md"), "# Index\n").unwrap();
    std::fs::write(wiki.join("log.md"), "# Log\n").unwrap();

    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(wiki)
        .arg("lint")
        .assert()
        .code(2)
        .stdout(str::contains("missing-frontmatter"));
}

#[test]
fn lint_detects_orphan_page() {
    let tmp = tempfile::tempdir().unwrap();
    let wiki = tmp.path();
    std::fs::create_dir_all(wiki.join("wiki")).unwrap();
    let fm = "---\nschema_version: 1\ntitle: A\ncreated: 2026-01-01\nupdated: 2026-01-01\ntype: concept\ntags: [concept]\nsources: []\n---\n\nBody [[beta-page]]\n";
    std::fs::write(wiki.join("wiki/a-page.md"), fm).unwrap();
    std::fs::write(
        wiki.join("wiki/b-page.md"),
        fm.replace("created: 2026-01-01", "created: 2026-01-02"),
    )
    .unwrap();
    std::fs::write(wiki.join("index.md"), "# Index\n").unwrap();
    std::fs::write(wiki.join("log.md"), "# Log\n").unwrap();

    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(wiki)
        .arg("lint")
        .assert()
        .code(2)
        .stdout(str::contains("orphan-page"));
}

#[test]
fn lint_passes_on_wellformed_wiki() {
    let tmp = tempfile::tempdir().unwrap();
    let wiki = tmp.path();
    std::fs::create_dir_all(wiki.join("wiki")).unwrap();
    std::fs::create_dir_all(wiki.join("raw/articles")).unwrap();

    let page_a = "---\nschema_version: 1\ntitle: Alpha\ncreated: 2026-01-01\nupdated: 2026-01-01\ntype: concept\ntags: [concept, software]\nsources: [raw/articles/src.md]\n---\n\nBody [[beta-page]] [[gamma-page]]\n";
    let page_b = "---\nschema_version: 1\ntitle: Beta\ncreated: 2026-01-01\nupdated: 2026-01-01\ntype: entity\ntags: [entity, tool]\nsources: [raw/articles/src.md]\n---\n\nBody [[alpha-page]] [[gamma-page]]\n";
    let page_c = "---\nschema_version: 1\ntitle: Gamma\ncreated: 2026-01-01\nupdated: 2026-01-01\ntype: concept\ntags: [concept]\nsources: [raw/articles/src.md]\n---\n\nBody [[alpha-page]] [[beta-page]]\n";
    std::fs::write(wiki.join("wiki/alpha-page.md"), page_a).unwrap();
    std::fs::write(wiki.join("wiki/beta-page.md"), page_b).unwrap();
    std::fs::write(wiki.join("wiki/gamma-page.md"), page_c).unwrap();

    std::fs::write(
        wiki.join("index.md"),
        "# Index\n\n## Concepts\n- [Alpha](wiki/alpha-page.md)\n- [Gamma](wiki/gamma-page.md)\n\n## Entities\n- [Beta](wiki/beta-page.md)\n",
    )
    .unwrap();
    std::fs::write(
        wiki.join("log.md"),
        "# Log\n\n## [2026-01-01] ingest | Added alpha, beta, gamma\n- Sources processed: src.md\n",
    )
    .unwrap();
    std::fs::write(wiki.join("raw/articles/src.md"), "source content").unwrap();

    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(wiki)
        .arg("lint")
        .assert()
        .code(0);
}
