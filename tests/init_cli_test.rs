//! Integration tests for `wiki init --flat` (v0.3.26+).
//!
//! The `--flat` flag scaffolds a flat-layout wiki (no `wiki/` subdir, all
//! pages at workspace root). Plain `wiki init` (without `--flat`) keeps
//! the legacy `wiki/` subdir scaffold for backward compatibility with
//! existing tests + tooling.
//!
//! Uses the same `WIKI_ROOT_CONFIG` isolation pattern as `init_test.rs`
//! so the tests do NOT pollute the user's real `~/.agents/wiki-root.toml`.

use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

/// Build a `llmwiki-cli` Command with `WIKI_ROOT_CONFIG` pointed at a
/// fresh empty registry under a tempdir, so the test cannot register
/// itself into the user's real `~/.agents/wiki-root.toml`.
fn isolated_cmd() -> (tempfile::TempDir, std::path::PathBuf, Command) {
    let tmp = tempdir().unwrap();
    let registry = tmp.path().join("wiki-root.toml");
    std::fs::write(&registry, "# test wiki-root.toml\n").unwrap();
    let cmd = Command::cargo_bin("llmwiki-cli").unwrap();
    (tmp, registry, cmd)
}

#[test]
fn init_with_flat_flag_creates_no_wiki_subdir() {
    let (_tmp, registry, mut cmd) = isolated_cmd();
    let target = _tmp.path().join("mywiki");
    cmd.env("WIKI_ROOT_CONFIG", &registry)
        .arg("init")
        .arg(&target)
        .arg("--flat")
        .arg("--alias")
        .arg("test-flat")
        .assert()
        .success();
    // No `wiki/` subdir.
    assert!(
        !target.join("wiki").exists(),
        "wiki/ subdir should not exist with --flat"
    );
    // Pages live at workspace root.
    assert!(target.join("index.md").exists());
    assert!(target.join("overview.md").exists());
    assert!(target.join("log.md").exists());
    // Config contains the flat-layout hint in the scaffolded comment.
    let cfg = fs::read_to_string(target.join(".llmwiki-cli/config.toml")).unwrap();
    assert!(
        cfg.contains("pages_dir = \"\""),
        "config scaffold should reference new default: {cfg}"
    );
}

#[test]
fn init_without_flat_flag_uses_subdirectory_layout() {
    // v0.3.26+: plain `wiki init` (no `--flat`) scaffolds the legacy
    // `wiki/` subdir layout for backward compatibility with existing
    // tests + tooling. Users opt into flat layout via `--flat`.
    let (_tmp, registry, mut cmd) = isolated_cmd();
    let target = _tmp.path().join("mywiki");
    cmd.env("WIKI_ROOT_CONFIG", &registry)
        .arg("init")
        .arg(&target)
        .arg("--alias")
        .arg("test-default")
        .assert()
        .success();
    assert!(
        target.join("wiki").exists(),
        "plain init scaffolds wiki/ subdir for backward compat"
    );
    assert!(target.join("wiki/overview.md").exists());
    assert!(target.join("wiki/log.md").exists());
}

#[test]
fn init_prints_layout_and_config_path() {
    // v0.3.26+: `wiki init` must report what layout it picked and where
    // the config file lives so users can audit.
    let (_tmp, registry, mut cmd) = isolated_cmd();
    let target = _tmp.path().join("mywiki");
    cmd.env("WIKI_ROOT_CONFIG", &registry)
        .arg("init")
        .arg(&target)
        .arg("--flat")
        .arg("--alias")
        .arg("test-output")
        .assert()
        .success()
        .stdout(predicates::str::contains("Layout:"))
        .stdout(predicates::str::contains("flat"))
        .stdout(predicates::str::contains("Config:"))
        .stdout(predicates::str::contains(".llmwiki-cli/config.toml"));
}
