use assert_cmd::Command;
use predicates::path::exists;
use predicates::prelude::*;

#[test]
fn wiki_init_creates_scaffold() {
    let tmp = tempfile::tempdir().unwrap();
    let target = tmp.path().join("mywiki");

    Command::cargo_bin("wiki")
        .unwrap()
        .arg("init")
        .arg(&target)
        .assert()
        .success();

    assert!(exists().eval(&target.join("wiki/overview.md")));
    assert!(exists().eval(&target.join("wiki/log.md")));
    assert!(exists().eval(&target.join("raw/articles/.gitkeep")));
    assert!(exists().eval(&target.join("index.md")));
    assert!(exists().eval(&target.join(".wiki/config.yaml")));
    assert!(exists().eval(&target.join(".gitignore")));
}

#[test]
fn wiki_init_creates_git_repo() {
    let tmp = tempfile::tempdir().unwrap();
    let target = tmp.path().join("mywiki");

    Command::cargo_bin("wiki")
        .unwrap()
        .arg("init")
        .arg(&target)
        .assert()
        .success();

    assert!(target.join(".git").exists());
}

#[test]
fn wiki_init_writes_default_config() {
    let tmp = tempfile::tempdir().unwrap();
    let target = tmp.path().join("mywiki");

    Command::cargo_bin("wiki")
        .unwrap()
        .arg("init")
        .arg(&target)
        .assert()
        .success();

    let cfg = std::fs::read_to_string(target.join(".wiki/config.yaml")).unwrap();
    assert!(cfg.contains("nvidia/nv-embed-v1"));
}
