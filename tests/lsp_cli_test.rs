use assert_cmd::Command;

#[test]
fn lsp_subcommand_help_exits_zero() {
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("lsp")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("stdio"));
}
