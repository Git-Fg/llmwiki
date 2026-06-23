use assert_cmd::Command;

#[test]
fn version_prints_current_cargo_version() {
    // Verify the binary's --version matches the version in Cargo.toml.
    // This catches accidental version drift between source and binary.
    let cargo_version = std::fs::read_to_string("Cargo.toml")
        .expect("Cargo.toml readable")
        .lines()
        .find(|l| l.starts_with("version = "))
        .expect("version line present")
        .split('"')
        .nth(1)
        .expect("version is a quoted string")
        .to_string();
    let expected = format!("llmwiki-cli {cargo_version}");
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicates::str::contains(&expected));
}
