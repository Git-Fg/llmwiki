use assert_cmd::Command;

#[test]
fn wiki_version_prints_version() {
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicates::str::contains("llmwiki-cli 0.3.0"));
}
