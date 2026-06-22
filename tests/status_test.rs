use assert_cmd::Command;
use predicates::str;

#[test]
fn status_reports_page_count() {
    let tmp = tempfile::tempdir().unwrap();
    let wiki = tmp.path();
    std::fs::create_dir_all(wiki.join("wiki")).unwrap();
    std::fs::write(wiki.join("wiki/a-page.md"), "body").unwrap();
    std::fs::write(wiki.join("wiki/b-page.md"), "body").unwrap();

    Command::cargo_bin("wiki")
        .unwrap()
        .arg("--workspace")
        .arg(wiki)
        .arg("status")
        .assert()
        .success()
        .stdout(str::contains("Pages: 2"));
}

#[test]
fn status_reports_embedding_coverage() {
    let tmp = tempfile::tempdir().unwrap();
    let wiki = tmp.path();
    std::fs::create_dir_all(wiki.join("wiki")).unwrap();
    std::fs::write(wiki.join("wiki/a-page.md"), "body").unwrap();
    std::fs::write(wiki.join("embeddings.jsonl"), "").unwrap();

    Command::cargo_bin("wiki")
        .unwrap()
        .arg("--workspace")
        .arg(wiki)
        .arg("status")
        .assert()
        .success()
        .stdout(str::contains("Embedded:"));
}
