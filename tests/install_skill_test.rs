use assert_cmd::Command;
use predicates::path::exists;
use predicates::Predicate;

#[test]
fn install_skill_global_writes_bundle() {
    let home = tempfile::tempdir().unwrap();
    let agents_dir = home.path().join(".agents/skills");

    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .env("HOME", home.path())
        .arg("install-skill")
        .arg("--global")
        .assert()
        .success();

    assert!(exists().eval(&agents_dir.join("wiki/SKILL.md")));
    assert!(exists().eval(&agents_dir.join("wiki/SETUP/SKILL.md")));
    assert!(exists().eval(&agents_dir.join("wiki/MCP/SKILL.md")));
}

#[test]
fn install_skill_workspace_writes_local_bundle() {
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
    assert!(exists().eval(&workspace.join(".agents/skills/wiki/SETUP/SKILL.md")));
}
