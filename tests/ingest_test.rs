use assert_cmd::Command;
use predicates::path::exists;
use predicates::Predicate;
use sha2::{Digest, Sha256};

#[test]
fn ingest_adds_file_to_raw() {
    let tmp = tempfile::tempdir().unwrap();
    let wiki = tmp.path();
    std::fs::create_dir_all(wiki.join(".wiki")).unwrap();
    std::fs::create_dir_all(wiki.join("raw/articles")).unwrap();

    let source = tmp.path().join("source.md");
    std::fs::write(&source, "Source content.\n").unwrap();

    Command::cargo_bin("wiki")
        .unwrap()
        .arg("--workspace")
        .arg(wiki)
        .arg("ingest")
        .arg(&source)
        .assert()
        .success();

    assert!(exists().eval(&wiki.join("raw/articles/source.md")));

    let content = std::fs::read_to_string(wiki.join("raw/articles/source.md")).unwrap();
    assert!(content.contains("source_type:"));
    assert!(content.contains("sha256:"));

    let body_content = "Source content.\n";
    let expected = hex::encode(Sha256::digest(body_content.as_bytes()));
    assert!(content.contains(&expected));
}

#[test]
fn ingest_updates_log() {
    let tmp = tempfile::tempdir().unwrap();
    let wiki = tmp.path();
    std::fs::create_dir_all(wiki.join(".wiki")).unwrap();
    std::fs::create_dir_all(wiki.join("raw/articles")).unwrap();
    std::fs::write(wiki.join("log.md"), "# Log\n").unwrap();

    let source = tmp.path().join("source.md");
    std::fs::write(&source, "Content.\n").unwrap();

    Command::cargo_bin("wiki")
        .unwrap()
        .arg("--workspace")
        .arg(wiki)
        .arg("ingest")
        .arg(&source)
        .arg("--no-compile")
        .assert()
        .success();

    let log = std::fs::read_to_string(wiki.join("log.md")).unwrap();
    assert!(log.contains("ingest"));
}
