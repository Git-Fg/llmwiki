use assert_cmd::Command;
use predicates::path::exists;
use predicates::prelude::*;

/// Run `wiki` with WIKI_ROOT_CONFIG pointed at an isolated temp registry,
/// so tests don't pollute the real `~/.agents/wiki-root.toml`.
fn make_cmd() -> (tempfile::TempDir, std::path::PathBuf, Command) {
    let tmp = tempfile::tempdir().unwrap();
    let registry = tmp.path().join("wiki-root.toml");
    std::fs::write(&registry, "# test wiki-root.toml\n").unwrap();
    let mut cmd = Command::cargo_bin("llmwiki-cli").unwrap();
    cmd.env("WIKI_ROOT_CONFIG", &registry);
    (tmp, registry, cmd)
}

#[test]
fn wiki_init_creates_scaffold() {
    let (tmp, _registry, mut cmd) = make_cmd();
    let target = tmp.path().join("mywiki");
    cmd.arg("init").arg(&target).assert().success();

    assert!(exists().eval(&target.join("wiki/overview.md")));
    assert!(exists().eval(&target.join("wiki/log.md")));
    assert!(exists().eval(&target.join("raw/articles/.gitkeep")));
    assert!(exists().eval(&target.join("index.md")));
    assert!(!exists().eval(&target.join(".wiki/config.yaml")));
    assert!(exists().eval(&target.join(".gitignore")));
}

#[test]
fn wiki_init_creates_git_repo() {
    let (tmp, _registry, mut cmd) = make_cmd();
    let target = tmp.path().join("mywiki");
    cmd.arg("init").arg(&target).assert().success();
    assert!(target.join(".git").exists());
}

#[test]
fn wiki_init_does_not_create_wiki_config_yaml() {
    let (tmp, _registry, mut cmd) = make_cmd();
    let target = tmp.path().join("mywiki");
    cmd.arg("init").arg(&target).assert().success();
    assert!(!target.join(".wiki/config.yaml").exists());
}

#[test]
fn wiki_init_registers_in_wiki_root_toml() {
    let (_tmp, registry, mut cmd) = make_cmd();
    let target = _tmp.path().join("mywiki");
    cmd.arg("init")
        .arg(&target)
        .arg("--alias")
        .arg("test-alias")
        .assert()
        .success();

    let content = std::fs::read_to_string(&registry).unwrap();
    assert!(content.contains("test-alias"));
    assert!(content.contains(&target.display().to_string()));
}
