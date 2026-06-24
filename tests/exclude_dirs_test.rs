//! Integration tests for `wiki.exclude_dirs` (v0.3.26+).
//!
//! Default exclude list (from `default_exclude_dirs()` in
//! `src/core/config_types.rs`) must filter `node_modules/`, `.git/`,
//! `.opencode/`, etc. from every page-walking CLI command — `ls --pages`,
//! `tree`, `embed`, `lint --scope wiki`, `status`.
//!
//! Pages under excluded directories must NOT appear in listings, must NOT
//! be embedded, must NOT be linted.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

fn page_with_title(title: &str) -> String {
    format!(
        "---\nschema_version: 1\ntitle: {title}\ncreated: 2026-01-01\nupdated: 2026-01-01\ntype: concept\ntags: [x]\nsources: []\n---\n\nBody\n"
    )
}

/// Build a workspace with two real pages and one hidden inside `node_modules`.
/// Returns the TempDir (must stay alive for the assertion).
///
/// Uses flat layout: pages live at the workspace root. We pin this with a
/// per-workspace `wiki.pages_dir = ""` config because the v0.3.26 default is
/// still `"wiki"` (the default flips in Task 5).
fn make_workspace_with_excluded_dir() -> tempfile::TempDir {
    let tmp = tempdir().unwrap();
    let ws = tmp.path();
    fs::create_dir_all(ws.join(".llmwiki-cli")).unwrap();
    // Real pages at workspace root (flat layout — the v0.3.26 default).
    fs::write(ws.join("real-one.md"), page_with_title("Real One")).unwrap();
    fs::write(ws.join("real-two.md"), page_with_title("Real Two")).unwrap();
    // Excluded noise — must NOT appear in any listing.
    fs::create_dir_all(ws.join("node_modules/pkg")).unwrap();
    fs::write(
        ws.join("node_modules/pkg/leaked.md"),
        page_with_title("Leaked"),
    )
    .unwrap();
    // Also test the real-wiki-smoke-test case: `.opencode/`.
    fs::create_dir_all(ws.join(".opencode/scratch")).unwrap();
    fs::write(
        ws.join(".opencode/scratch/cached.md"),
        page_with_title("Cached"),
    )
    .unwrap();
    // Pin flat layout — pages at workspace root, not in `wiki/`.
    let config = "[wiki]\npages_dir = \"\"\n";
    fs::write(ws.join(".llmwiki-cli/config.toml"), config).unwrap();
    tmp
}

#[test]
fn default_excludes_filter_node_modules_from_ls_pages() {
    let tmp = make_workspace_with_excluded_dir();
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(tmp.path())
        .arg("ls")
        .arg("--pages")
        .assert()
        .success()
        .stdout(predicate::str::contains("real-one"))
        .stdout(predicate::str::contains("real-two"))
        .stdout(predicate::str::contains("Leaked").not())
        .stdout(predicate::str::contains("node_modules").not());
}

#[test]
fn default_excludes_filter_opencode_from_ls_pages() {
    let tmp = make_workspace_with_excluded_dir();
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(tmp.path())
        .arg("ls")
        .arg("--pages")
        .assert()
        .success()
        .stdout(predicate::str::contains("Cached").not())
        .stdout(predicate::str::contains(".opencode").not());
}

#[test]
fn default_excludes_filter_node_modules_from_tree() {
    let tmp = make_workspace_with_excluded_dir();
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(tmp.path())
        .arg("tree")
        .assert()
        .success()
        .stdout(predicate::str::contains("real-one"))
        .stdout(predicate::str::contains("real-two"))
        .stdout(predicate::str::contains("leaked").not());
}

#[test]
fn default_excludes_filter_node_modules_from_status_page_count() {
    let tmp = make_workspace_with_excluded_dir();
    let out = Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(tmp.path())
        .arg("status")
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    // 2 real pages, NOT 4 (which would include the leaked + cached).
    assert!(stdout.contains("Pages: 2"), "got: {stdout}");
    assert!(!stdout.contains("Pages: 4"), "got: {stdout}");
}

#[test]
fn default_excludes_filter_node_modules_from_lint() {
    let tmp = make_workspace_with_excluded_dir();
    // Lint must NOT report any issue from `node_modules/pkg/leaked.md`.
    let out = Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(tmp.path())
        .arg("lint")
        .assert()
        .get_output()
        .clone();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        !stdout.contains("leaked"),
        "leaked page linted despite exclusion: {stdout}"
    );
    assert!(
        !stdout.contains("node_modules"),
        "node_modules leaked into lint: {stdout}"
    );
}

#[test]
fn user_exclude_dirs_supplements_defaults() {
    // v0.3.27+: user exclude_dirs MERGES with defaults (additive).
    // User sets `wiki.exclude_dirs = ["node_modules", "secret"]` — the user's
    // custom "secret" applies, AND the defaults the user did NOT list
    // (e.g. `.opencode`) are RETAINED, so `.opencode/cached.md` is excluded too.
    let tmp = tempdir().unwrap();
    let ws = tmp.path();
    fs::create_dir_all(ws.join(".llmwiki-cli")).unwrap();
    fs::write(ws.join("real.md"), page_with_title("Real")).unwrap();
    fs::create_dir_all(ws.join("secret")).unwrap();
    fs::write(ws.join("secret/hidden.md"), page_with_title("Hidden")).unwrap();
    fs::create_dir_all(ws.join(".opencode")).unwrap();
    fs::write(ws.join(".opencode/cached.md"), page_with_title("Cached")).unwrap();
    fs::create_dir_all(ws.join("node_modules/pkg")).unwrap();
    fs::write(
        ws.join("node_modules/pkg/leaked.md"),
        page_with_title("Leaked"),
    )
    .unwrap();
    // Pin flat layout; user lists node_modules + secret explicitly.
    fs::write(
        ws.join(".llmwiki-cli/config.toml"),
        "[wiki]\npages_dir = \"\"\nexclude_dirs = [\"node_modules\", \"secret\"]\n",
    )
    .unwrap();
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(ws)
        .arg("ls")
        .arg("--pages")
        .assert()
        .success()
        .stdout(predicate::str::contains("real")) // real page visible
        .stdout(predicate::str::contains("hidden").not()) // user's custom exclude
        .stdout(predicate::str::contains("leaked").not()) // user explicitly listed
        .stdout(predicate::str::contains("Cached").not()); // retained DEFAULT (.opencode)
}

#[test]
fn user_exclude_dirs_merges_with_defaults() {
    // v0.3.27+: user exclude_dirs MERGES with defaults (additive).
    // The user adds ["secret"] and retains all defaults (node_modules, .opencode, etc.).
    let tmp = tempdir().unwrap();
    let ws = tmp.path();
    fs::create_dir_all(ws.join(".llmwiki-cli")).unwrap();
    fs::write(ws.join("real.md"), page_with_title("Real")).unwrap();
    fs::create_dir_all(ws.join("secret")).unwrap();
    fs::write(ws.join("secret/hidden.md"), page_with_title("Hidden")).unwrap();
    fs::create_dir_all(ws.join("node_modules/pkg")).unwrap();
    fs::write(
        ws.join("node_modules/pkg/leaked.md"),
        page_with_title("Leaked"),
    )
    .unwrap();
    fs::create_dir_all(ws.join(".opencode")).unwrap();
    fs::write(ws.join(".opencode/cached.md"), page_with_title("Cached")).unwrap();
    // User adds ["secret"] — should ALSO retain node_modules + .opencode from defaults.
    fs::write(
        ws.join(".llmwiki-cli/config.toml"),
        "[wiki]\npages_dir = \"\"\nexclude_dirs = [\"secret\"]\n",
    )
    .unwrap();
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(ws)
        .arg("ls")
        .arg("--pages")
        .assert()
        .success()
        .stdout(predicates::str::contains("Real"))
        .stdout(predicates::str::contains("Hidden").not()) // user's custom exclude
        .stdout(predicates::str::contains("Leaked").not()) // retained default
        .stdout(predicates::str::contains("Cached").not()); // retained default
}

#[test]
#[ignore = "requires NIM API"]
fn exclude_dirs_skips_excluded_pages_from_embed() {
    // v0.3.27+: llmwiki-cli embed must not embed pages under excluded dirs.
    let tmp = tempdir().unwrap();
    let ws = tmp.path();
    fs::create_dir_all(ws.join(".llmwiki-cli")).unwrap();
    fs::create_dir_all(ws.join("node_modules/pkg")).unwrap();
    fs::write(ws.join("keep.md"), page_with_title("Keep")).unwrap();
    fs::write(
        ws.join("node_modules/pkg/leaked.md"),
        page_with_title("Leaked"),
    )
    .unwrap();
    fs::write(
        ws.join(".llmwiki-cli/config.toml"),
        "[wiki]\npages_dir = \"\"\n",
    )
    .unwrap();
    // llmwiki-cli embed should succeed, and leaked.md should NOT appear in embeddings.jsonl.
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(ws)
        .arg("embed")
        .arg("--model")
        .arg("nvidia/nv-embed-v1")
        .assert()
        .success();
    let jsonl = fs::read_to_string(ws.join("embeddings.jsonl")).unwrap_or_default();
    assert!(
        !jsonl.contains("leaked"),
        "leaked.md embedded despite exclusion: {jsonl}"
    );
    assert!(jsonl.contains("keep"), "keep.md not embedded: {jsonl}");
}
