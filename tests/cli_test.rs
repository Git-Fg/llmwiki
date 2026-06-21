use assert_cmd::Command;

#[test]
fn wiki_version_prints_version() {
    Command::cargo_bin("wiki")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicates::str::contains("wiki 0.1.0"));
}
