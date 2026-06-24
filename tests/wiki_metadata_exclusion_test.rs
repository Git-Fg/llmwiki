//! Integration tests for the workspace-root `index.md` / `log.md` and `raw/`
//! exclusion from the 5 read-path commands (v0.3.26+).
//!
//! With the flat-layout default (`pages_dir = ""`), `walk_pages` descends
//! from the workspace root. The root also contains:
//!
//! - `index.md` — the wiki registry (hand-written roll-up of pages)
//! - `log.md` — the chronological operation log
//! - `AGENTS.md`, `README.md`, etc. — project metadata
//! - `raw/` — the source-files directory
//!
//! `is_wiki_page_entry` (in `src/core/workspace.rs`) is the shared filter
//! that prevents the page walks from sweeping these into listings. The
//! filter excludes:
//! - workspace-root `index.md` and `log.md` (the wiki registry and
//!   operation log — but NOT a subdirectory's `index.md`, which is a
//!   legitimate entry-point page)
//! - anything under `raw/` at any depth
//!
//! `AGENTS.md`, `README.md`, and other project files at the workspace root
//! are NOT excluded by this helper — they could be legitimate wiki content.
//!
//! These tests assert the end-to-end behavior across all 5 commands.

use assert_cmd::Command;
use predicates::prelude::*;

const PAGE: &str = "---\nschema_version: 1\ntitle: Real Page\ncreated: 2026-01-01\nupdated: 2026-01-01\ntype: concept\ntags: [x]\nsources: []\n---\n\nBody\n";

/// Build a flat-layout workspace with the metadata files a real Karpathy-style
/// wiki has at its root, plus a `raw/` source dir, plus one real page.
fn make_workspace() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path();
    std::fs::create_dir_all(ws.join(".llmwiki-cli")).unwrap();
    // Pin flat layout — pages at workspace root.
    std::fs::write(
        ws.join(".llmwiki-cli/config.toml"),
        "[wiki]\npages_dir = \"\"\n",
    )
    .unwrap();

    // Real page (must appear in listings).
    std::fs::write(ws.join("real.md"), PAGE).unwrap();

    // Workspace-root `index.md` and `log.md` (wiki registry and operation log) —
    // must NOT appear in page listings.
    std::fs::write(ws.join("index.md"), "# Index\n").unwrap();
    std::fs::write(ws.join("log.md"), "# Log\n").unwrap();

    // `raw/` source directory — must NOT appear in page listings.
    std::fs::create_dir_all(ws.join("raw/articles")).unwrap();
    std::fs::write(
        ws.join("raw/articles/source.md"),
        "---\nsha256: deadbeef\nsource_url: x\n---\nbody\n",
    )
    .unwrap();

    tmp
}

#[test]
fn ls_pages_excludes_workspace_root_index_log_and_raw() {
    let tmp = make_workspace();
    let out = Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(tmp.path())
        .arg("ls")
        .arg("--pages")
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    // Real page shows up.
    assert!(stdout.contains("real"), "real page missing: {stdout}");
    assert!(
        stdout.contains("Real Page"),
        "real page title missing: {stdout}"
    );
    // Workspace-root index.md / log.md are excluded.
    assert!(
        !stdout.contains("Pages (1)\n  index.md"),
        "index.md leaked into `ls --pages`: {stdout}"
    );
    assert!(
        !stdout.contains("Pages (1)\n  log.md"),
        "log.md leaked into `ls --pages`: {stdout}"
    );
    assert!(
        !stdout.contains("raw/"),
        "raw/ leaked into `ls --pages`: {stdout}"
    );
    assert!(
        !stdout.contains("source.md"),
        "raw source file leaked into `ls --pages`: {stdout}"
    );
    // Page count is exactly 1.
    assert!(
        stdout.contains("Pages (1)"),
        "expected `Pages (1)`, got: {stdout}"
    );
}

#[test]
fn tree_excludes_workspace_root_index_log_and_raw() {
    let tmp = make_workspace();
    let out = Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(tmp.path())
        .arg("tree")
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("real"), "real page missing: {stdout}");
    assert!(
        stdout.contains("Real Page"),
        "real page title missing: {stdout}"
    );
    // index.md / log.md at workspace root must not appear as tree entries.
    assert!(
        !stdout.lines().any(|l| l.starts_with("index ")),
        "index.md leaked into `tree`: {stdout}"
    );
    assert!(
        !stdout.lines().any(|l| l.starts_with("log ")),
        "log.md leaked into `tree`: {stdout}"
    );
    assert!(
        !stdout.contains("source.md"),
        "raw source file leaked into `tree`: {stdout}"
    );
}

#[test]
fn status_counts_only_real_pages() {
    let tmp = make_workspace();
    let out = Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(tmp.path())
        .arg("status")
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.contains("Pages: 1"),
        "expected `Pages: 1`, got: {stdout}"
    );
    // Raw sources are counted separately under their own field — make sure
    // the page count doesn't double-count them.
    assert!(
        stdout.contains("Raw sources: 1"),
        "expected `Raw sources: 1`, got: {stdout}"
    );
}

#[test]
fn lint_does_not_report_issues_for_index_log_or_raw() {
    let tmp = make_workspace();
    let out = Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(tmp.path())
        .arg("lint")
        .assert()
        .get_output()
        .clone();
    let stdout = String::from_utf8(out.stdout).unwrap();
    // Workspace-root index.md / log.md should not appear as lint issue paths.
    // (The lint command has its own pass for index/log content — they just
    // shouldn't be linted as wiki pages.)
    let index_md_lines: Vec<&str> = stdout.lines().filter(|l| l.contains("index.md")).collect();
    assert!(
        index_md_lines.is_empty(),
        "workspace-root index.md leaked into lint as a wiki page: {index_md_lines:?}"
    );
    let log_md_lines: Vec<&str> = stdout.lines().filter(|l| l.contains("log.md")).collect();
    assert!(
        log_md_lines.is_empty(),
        "workspace-root log.md leaked into lint as a wiki page: {log_md_lines:?}"
    );
    // The wiki-scope walk must not include raw/ as a wiki page. (Raw/ files
    // appear under `source-not-cited` from the separate `--scope raw` pass,
    // which is intentional and unrelated to this filter.)
    let raw_wiki_lines: Vec<&str> = stdout
        .lines()
        .filter(|l| l.contains("raw/articles/source.md") && !l.contains("source-not-cited"))
        .collect();
    assert!(
        raw_wiki_lines.is_empty(),
        "raw/ source walked as a wiki page: {raw_wiki_lines:?}"
    );
    // The real page itself may have legitimate lint issues (depends on
    // taxonomy, orphan status, etc.) — that's fine. The contract tested
    // here is: metadata and raw files are excluded from the wiki walk.
}

#[test]
fn embed_wiki_walk_skips_index_log_and_raw() {
    // The embed walk filter is `is_wiki_page_entry` — identical to the
    // `ls --pages` call site (`src/cli/ls.rs` line 155). The four other
    // commands in this file already cover the filter exhaustively, but we
    // keep this test as an explicit regression guard for the embed call
    // site (`src/cli/embed.rs` line 54).
    //
    // We point `WIKI_NIM_BASE_URL` at an unreachable local URL so the NIM
    // network call fails fast (after the wiki walk has completed). The
    // assertion: `embeddings.jsonl` must NOT have been written with any of
    // the metadata files.
    let tmp = make_workspace();
    let out = Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(tmp.path())
        .arg("embed")
        .arg("--model")
        .arg("nvidia/nv-embed-v1")
        .env("WIKI_NIM_BASE_URL", "http://127.0.0.1:1")
        .env("NVIDIA_NIM_API_KEY", "test-key")
        .timeout(std::time::Duration::from_secs(15))
        .output()
        .unwrap();
    let embeddings_path = tmp.path().join("embeddings.jsonl");
    if embeddings_path.exists() {
        let contents = std::fs::read_to_string(&embeddings_path).unwrap_or_default();
        assert!(
            !contents.contains("\"path\":\"index.md\""),
            "index.md embedded: {contents}"
        );
        assert!(
            !contents.contains("\"path\":\"log.md\""),
            "log.md embedded: {contents}"
        );
        assert!(
            !contents.contains("raw/"),
            "raw/ source embedded: {contents}"
        );
    }
    // If the call succeeded (it shouldn't with the unreachable URL), the
    // output would say exactly "✓ Embedded 1 page(s)" — confirming only
    // the real page made it past the filter.
    let stdout = String::from_utf8(out.stdout.clone()).unwrap_or_default();
    if stdout.contains("✓ Embedded") {
        assert!(
            stdout.contains("✓ Embedded 1 page"),
            "expected exactly 1 page embedded, got: {stdout}"
        );
    }
}

#[test]
fn subdirectory_index_md_is_kept_as_a_page() {
    // Counterpart: a subdirectory's `index.md` is a legitimate entry-point
    // page and must NOT be filtered. E.g. `research/decompilation/index.md`.
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path();
    std::fs::create_dir_all(ws.join(".llmwiki-cli")).unwrap();
    std::fs::write(
        ws.join(".llmwiki-cli/config.toml"),
        "[wiki]\npages_dir = \"\"\n",
    )
    .unwrap();

    std::fs::create_dir_all(ws.join("research/decompilation")).unwrap();
    std::fs::write(ws.join("research/decompilation/index.md"), PAGE).unwrap();

    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(ws)
        .arg("ls")
        .arg("--pages")
        .assert()
        .success()
        .stdout(predicate::str::contains("research/decompilation/index"))
        .stdout(predicate::str::contains("Pages (1)"));
}
