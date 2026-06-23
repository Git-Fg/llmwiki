//! Integration tests for flat-layout wiki support (v0.3.25+).
//!
//! Real Karpathy-style wikis on this machine (`mevin`, `minimax`, `mywiki`,
//! `pharma`, `pharma.nim` in `~/.agents/wiki-root.toml`) use a flat layout —
//! pages live at the workspace root (`comparisons/foo.md`, `queries/bar.md`,
//! `index.md`), not in a `wiki/` subdirectory. Pre-v0.3.25, six CLI commands
//! hardcoded `ws.join("wiki")` and reported "0 pages" against these wikis.
//!
//! v0.3.25 introduces `wiki.pages_dir` (default `"wiki"`, empty string = flat).
//! These tests verify the end-to-end behavior of `wiki ls --pages` and
//! `wiki tree` against both layouts.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

/// Helper: write a per-workspace config with the given `pages_dir` value.
fn write_config(workspace: &std::path::Path, pages_dir: &str) {
    let config = format!(
        "[wiki]\npages_dir = \"{pages_dir}\"\n",
        pages_dir = pages_dir.replace('"', "\\\"")
    );
    fs::write(workspace.join(".llmwiki-cli/config.toml"), config).unwrap();
}

/// Build a workspace in `wiki/`-layout (default): pages in `wiki/` subdir.
fn make_wiki_layout_workspace() -> tempfile::TempDir {
    let tmp = tempdir().unwrap();
    let ws = tmp.path();
    fs::create_dir_all(ws.join(".llmwiki-cli")).unwrap();
    fs::create_dir_all(ws.join("wiki")).unwrap();
    fs::write(
        ws.join("wiki/example.md"),
        "---\ntitle: Example\ntags: [rust]\n---\n\nBody [[other]].\n",
    )
    .unwrap();
    fs::write(
        ws.join("wiki/other.md"),
        "---\ntitle: Other\ntags: []\n---\n\nBack [[example]].\n",
    )
    .unwrap();
    tmp
}

/// Build a workspace in flat layout: pages at workspace root.
fn make_flat_layout_workspace() -> tempfile::TempDir {
    let tmp = tempdir().unwrap();
    let ws = tmp.path();
    fs::create_dir_all(ws.join(".llmwiki-cli")).unwrap();
    // NO wiki/ subdirectory — pages live at workspace root in flat-layout.
    fs::create_dir_all(ws.join("comparisons")).unwrap();
    fs::create_dir_all(ws.join("queries")).unwrap();
    fs::write(
        ws.join("comparisons/foo.md"),
        "---\ntitle: Foo Comparison\ntags: [analysis]\n---\n\nBody [[queries-bar]].\n",
    )
    .unwrap();
    fs::write(
        ws.join("queries/bar.md"),
        "---\ntitle: Bar Query\ntags: []\n---\n\nBack [[comparisons-foo]].\n",
    )
    .unwrap();
    fs::write(ws.join("index.md"), "# Index\n\n## Pages\n\n").unwrap();
    tmp
}

#[test]
fn wiki_subdir_layout_ls_pages_finds_pages() {
    // Sanity check: the legacy `wiki/`-layout behavior is unchanged.
    let tmp = make_wiki_layout_workspace();
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(tmp.path())
        .arg("ls")
        .arg("--pages")
        .assert()
        .success()
        .stdout(predicate::str::contains("example"))
        .stdout(predicate::str::contains("other"));
}

#[test]
fn flat_layout_ls_pages_finds_pages_when_pages_dir_is_empty() {
    // v0.3.25+: flat-layout wikis work when `wiki.pages_dir = ""`.
    let tmp = make_flat_layout_workspace();
    write_config(tmp.path(), "");
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(tmp.path())
        .arg("ls")
        .arg("--pages")
        .assert()
        .success()
        .stdout(predicate::str::contains("comparisons/foo"))
        .stdout(predicate::str::contains("queries/bar"));
}

#[test]
fn flat_layout_tree_finds_pages_when_pages_dir_is_empty() {
    // Same for `wiki tree` — should list the flat-layout pages.
    let tmp = make_flat_layout_workspace();
    write_config(tmp.path(), "");
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(tmp.path())
        .arg("tree")
        .assert()
        .success()
        .stdout(predicate::str::contains("foo"))
        .stdout(predicate::str::contains("bar"));
}

#[test]
fn flat_layout_ls_pages_returns_empty_when_pages_dir_defaults_to_wiki() {
    // Sanity check: if the user has a flat-layout workspace but does NOT set
    // `wiki.pages_dir = ""`, the CLI still defaults to `wiki/` and reports
    // 0 pages. This is the documented v0.3.24- behavior for users who
    // haven't migrated their config yet.
    let tmp = make_flat_layout_workspace();
    // No config write — defaults to `wiki/`, which doesn't exist.
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(tmp.path())
        .arg("ls")
        .arg("--pages")
        .assert()
        .success()
        .stdout(predicate::str::contains("Pages (0):"));
}

#[test]
fn flat_layout_ls_resilient_to_unparseable_frontmatter() {
    // v0.3.25+: `wiki ls` must not crash the whole listing when one page
    // has YAML that `serde_yaml` cannot parse. The bad page is silently
    // skipped (its data would be useless anyway); the good page is listed
    // normally. This is a pre-existing fragility exposed by the page-
    // discovery fix in v0.3.25 — see `tests/flat_layout_test.rs` for the
    // flat-layout regression tests, and `tests/lint_cli_test.rs::
    // lint_resilient_to_unparseable_frontmatter` for the lint-side analogue
    // (which reports the same condition as a lint issue instead of skipping).
    let tmp = make_flat_layout_workspace();
    // Add a page with duplicate `type:` keys — `serde_yaml::from_str`
    // returns Err on this.
    let bad = "---\ntitle: Bad\ncreated: 2026-01-01\nupdated: 2026-01-01\ntype: concept\ntype: entity\ntags: [x]\nsources: []\n---\n\nBody\n";
    std::fs::write(tmp.path().join("comparisons/broken.md"), bad).unwrap();
    write_config(tmp.path(), "");

    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(tmp.path())
        .arg("ls")
        .arg("--pages")
        .assert()
        .success()
        .stdout(predicate::str::contains("comparisons/foo"))
        .stdout(predicate::str::contains("queries/bar"));
}

#[test]
fn flat_layout_with_custom_pages_dir_is_honored() {
    // A non-empty custom `pages_dir` should work too — e.g. `pages` or
    // `content/pages`.
    let tmp = tempdir().unwrap();
    let ws = tmp.path();
    fs::create_dir_all(ws.join(".llmwiki-cli")).unwrap();
    fs::create_dir_all(ws.join("content/pages")).unwrap();
    fs::write(
        ws.join("content/pages/alpha.md"),
        "---\ntitle: Alpha\ntags: []\n---\n\nBody.\n",
    )
    .unwrap();
    write_config(ws, "content/pages");
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(ws)
        .arg("ls")
        .arg("--pages")
        .assert()
        .success()
        .stdout(predicate::str::contains("alpha"));
}
