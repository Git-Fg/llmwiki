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

#[test]
fn install_skill_install_path_overrides_default() {
    // v0.3.30: `--install-path <dir>` lets users target a specific host's
    // skills directory (e.g. `~/.claude/skills/wiki` for Claude Code).
    let tmp = tempfile::tempdir().unwrap();
    let custom = tmp.path().join("custom-host/skills/wiki");

    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("install-skill")
        .arg("--install-path")
        .arg(&custom)
        .assert()
        .success();

    assert!(exists().eval(&custom.join("SKILL.md")));
    // Hub contents must be byte-identical with the embedded source.
    let installed = std::fs::read_to_string(custom.join("SKILL.md")).unwrap();
    let embedded = llmwiki_cli::skills::hub();
    assert_eq!(installed, embedded);
}

#[test]
fn install_skill_install_path_expands_tilde() {
    // `--install-path ~/...` should expand to $HOME on Unix.
    let home = tempfile::tempdir().unwrap();
    let custom = home.path().join("tilde-target");

    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .env("HOME", home.path())
        .arg("install-skill")
        .arg("--install-path")
        .arg(format!(
            "~/{}",
            custom.file_name().unwrap().to_string_lossy()
        ))
        .assert()
        .success();

    assert!(exists().eval(&custom.join("SKILL.md")));
}
