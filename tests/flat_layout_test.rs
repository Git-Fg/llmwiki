//! Integration tests for flat-layout wiki support (v0.3.25+).
//!
//! Real Karpathy-style wikis on this machine (`mevin`, `minimax`, `mywiki`,
//! `pharma`, `pharma.nim` in `~/.agents/wiki-root.toml`) use a flat layout —
//! pages live at the workspace root (`comparisons/foo.md`, `queries/bar.md`,
//! `index.md`), not in a `wiki/` subdirectory. Pre-v0.3.25, six CLI commands
//! hardcoded `ws.join("wiki")` and reported "0 pages" against these wikis.
//!
//! v0.3.25 introduces `wiki.pages_dir` (default `"wiki"`, empty string = flat).
//! These tests verify the end-to-end behavior of `llmwiki-cli ls --pages` and
//! `llmwiki-cli tree` against both layouts.

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
    // Same for `llmwiki-cli tree` — should list the flat-layout pages.
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
fn wiki_subdir_layout_finds_pages_with_prefix_when_default_is_flat() {
    // v0.3.26+ with the flat default and no explicit config: `walk_pages`
    // recursively descends into the workspace root, so pages in a `wiki/`
    // subdir ARE discovered (with `wiki/`-prefixed slugs). This is
    // backward-compatible with pre-v0.3.26 wikis that haven't migrated
    // their config yet — users still see their pages, just with prefix.
    // For clean slugs without the `wiki/` prefix, set `pages_dir = "wiki"`
    // explicitly (see `wiki_subdir_layout_works_with_explicit_pages_dir`).
    let tmp = make_wiki_layout_workspace();
    // No config write — pages_dir defaults to "" → walks workspace root.
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(tmp.path())
        .arg("ls")
        .arg("--pages")
        .assert()
        .success()
        .stdout(predicate::str::contains("wiki/example"))
        .stdout(predicate::str::contains("wiki/other"));
}

#[test]
fn wiki_subdir_layout_works_with_explicit_pages_dir() {
    // Counterpart: with `wiki.pages_dir = "wiki"` set, the legacy layout
    // works exactly as before v0.3.26.
    let tmp = make_wiki_layout_workspace();
    write_config(tmp.path(), "wiki");
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
fn flat_layout_ls_resilient_to_unparseable_frontmatter() {
    // v0.3.25+: `llmwiki-cli ls` must not crash the whole listing when one page
    // has YAML that `serde-saphyr` cannot parse. The bad page is silently
    // skipped (its data would be useless anyway); the good page is listed
    // normally. This is a pre-existing fragility exposed by the page-
    // discovery fix in v0.3.25 — see `tests/flat_layout_test.rs` for the
    // flat-layout regression tests, and `tests/lint_cli_test.rs::
    // lint_resilient_to_unparseable_frontmatter` for the lint-side analogue
    // (which reports the same condition as a lint issue instead of skipping).
    let tmp = make_flat_layout_workspace();
    // Add a page with duplicate `type:` keys — `serde_saphyr::from_str`
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

#[test]
fn default_pages_dir_is_empty_string_for_flat_layout() {
    // v0.3.26+: with no config, the CLI defaults to flat layout (pages at
    // workspace root). The pre-v0.3.26 default of `"wiki"` is gone. Users
    // who want the legacy subdirectory layout set `pages_dir = "wiki"` explicitly.
    let tmp = make_flat_layout_workspace();
    // No config write — defaults to `""` (flat), so the workspace-root
    // pages ARE discovered.
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
