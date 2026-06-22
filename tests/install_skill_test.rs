use assert_cmd::Command;
use predicates::path::exists;
use predicates::Predicate;

#[test]
fn install_skill_global_creates_symlink() {
    let home = tempfile::tempdir().unwrap();
    let agents_dir = home.path().join(".agents/skills");
    let source_dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(source_dir.path().join("wiki")).unwrap();
    std::fs::write(source_dir.path().join("wiki/SKILL.md"), "# stub\n").unwrap();

    Command::cargo_bin("wiki")
        .unwrap()
        .env("HOME", home.path())
        .arg("install-skill")
        .arg("--global")
        .arg("--target")
        .arg(source_dir.path())
        .assert()
        .success();

    assert!(exists().eval(&agents_dir.join("wiki/SKILL.md")));
}

#[test]
fn install_skill_workspace_creates_local_symlink() {
    let tmp = tempfile::tempdir().unwrap();
    let workspace = tmp.path().join("wiki");
    let source_dir = tmp.path().join("source");
    std::fs::create_dir_all(&workspace).unwrap();
    std::fs::create_dir_all(source_dir.join("wiki")).unwrap();
    std::fs::write(source_dir.join("wiki/SKILL.md"), "# stub\n").unwrap();

    Command::cargo_bin("wiki")
        .unwrap()
        .arg("install-skill")
        .arg("--workspace")
        .arg(&workspace)
        .arg("--target")
        .arg(source_dir.as_path())
        .assert()
        .success();

    assert!(exists().eval(&workspace.join(".agents/skills/wiki/SKILL.md")));
}
