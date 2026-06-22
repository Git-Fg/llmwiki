use assert_cmd::Command;
use predicates::str;

#[test]
fn build_no_pending_returns_message() {
    let tmp = tempfile::tempdir().unwrap();
    let wiki = tmp.path();
    std::fs::create_dir_all(wiki.join("raw/articles")).unwrap();

    Command::cargo_bin("wiki")
        .unwrap()
        .arg("--workspace")
        .arg(wiki)
        .arg("build")
        .assert()
        .success()
        .stdout(str::contains("No pending"));
}

#[test]
fn build_dry_run_lists_pending() {
    let tmp = tempfile::tempdir().unwrap();
    let wiki = tmp.path();
    std::fs::create_dir_all(wiki.join("raw/articles")).unwrap();
    std::fs::write(
        wiki.join("raw/articles/source.md"),
        "---\nsource_type: article\ningested: 2026-01-01\nsha256: abc\n---\n\nBody\n",
    )
    .unwrap();

    Command::cargo_bin("wiki")
        .unwrap()
        .arg("--workspace")
        .arg(wiki)
        .arg("build")
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(str::contains("source.md"));
}
