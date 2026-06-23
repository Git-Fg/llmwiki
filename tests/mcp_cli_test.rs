use assert_cmd::Command;

#[test]
fn mcp_subcommand_help_exits_zero() {
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .args(["mcp", "--help"])
        .assert()
        .success()
        .stdout(predicates::str::contains("stdio"));
}
