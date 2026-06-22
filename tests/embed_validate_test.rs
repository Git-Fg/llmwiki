use assert_cmd::Command;
use std::io::Write;

fn isolated_registry(content: &str) -> (std::path::PathBuf, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("wiki-root.toml");
    std::fs::File::create(&path).unwrap().write_all(content.as_bytes()).unwrap();
    (path, dir)
}

#[test]
fn embed_fails_fast_when_chunk_overlap_exceeds_default() {
    let workspace = tempfile::tempdir().unwrap();
    let wiki = workspace.path();
    std::fs::create_dir_all(wiki.join("wiki")).unwrap();
    std::fs::write(wiki.join("wiki/page.md"), "---\ntitle: A\n---\n\nBody.\n").unwrap();

    let (reg, _d) = isolated_registry(&format!(
        r#"
[w]
path = "{}"

[w.wiki]
default_chunk_tokens = 100
chunk_overlap_tokens = 200
"#,
        wiki.display()
    ));

    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .env("WIKI_ROOT_CONFIG", &reg)
        .arg("--wiki")
        .arg("w")
        .arg("embed")
        .assert()
        .failure()
        .stderr(predicates::str::contains("chunk_overlap_tokens"));
}
