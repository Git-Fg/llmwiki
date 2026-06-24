use assert_cmd::Command;
use predicates::path::exists;
use predicates::Predicate;

#[test]
fn install_skill_global_writes_hub_only() {
    let home = tempfile::tempdir().unwrap();
    let agents_dir = home.path().join(".agents/skills");

    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .env("HOME", home.path())
        .arg("install-skill")
        .arg("--global")
        .assert()
        .success();

    // v0.3.29: only the hub is installed. Sub-skills are served at runtime
    // via `llmwiki-cli skill get <topic>` from the embedded binary.
    assert!(exists().eval(&agents_dir.join("wiki/SKILL.md")));

    // Sanity: no stale sub-skill directory exists from a prior install.
    assert!(
        !agents_dir.join("wiki").join("SETUP").exists(),
        "stale SETUP subdir should not exist in v0.3.29 layout"
    );
}

#[test]
fn install_skill_workspace_writes_local_hub_only() {
    let tmp = tempfile::tempdir().unwrap();
    let workspace = tmp.path().join("wiki");
    std::fs::create_dir_all(&workspace).unwrap();

    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("install-skill")
        .arg("--workspace")
        .arg(&workspace)
        .assert()
        .success();

    assert!(exists().eval(&workspace.join(".agents/skills/wiki/SKILL.md")));
}
