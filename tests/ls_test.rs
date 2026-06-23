use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use std::fs;
use tempfile::tempdir;

fn make_workspace() -> tempfile::TempDir {
    let tmp = tempdir().unwrap();
    let ws = tmp.path();
    fs::create_dir_all(ws.join(".llmwiki-cli")).unwrap();
    fs::create_dir_all(ws.join("wiki")).unwrap();
    fs::create_dir_all(ws.join("raw")).unwrap();
    fs::write(
        ws.join("wiki/test-page.md"),
        "---\ntitle: Test Page\ntags: [rust, cli]\n---\n\nSome body with [[other]].\n",
    )
    .unwrap();
    fs::write(
        ws.join("wiki/other.md"),
        "---\ntitle: Other Page\ntags: []\n---\n\nBack link [[test-page]].\n",
    )
    .unwrap();
    fs::write(
        ws.join("raw/source.txt"),
        "---\nsha256: abc123\ningested: '2025-01-01'\n---\nraw content",
    )
    .unwrap();
    // Minimal embeddings file matching PageEmbedding schema
    fs::write(
        ws.join("embeddings.jsonl"),
        r#"{"path":"wiki/test-page.md","sha256":"abc123","model":"nv-embedqa-e5-v5","dim":2,"chunked":false,"chunks":[{"start":0,"end":4,"tokens":4,"embedding":[0.1,0.2]}],"embedded_at":"2025-01-01T00:00:00Z"}"#,
    )
    .unwrap();
    tmp
}

#[test]
fn ls_default_shows_all_sections() {
    let tmp = make_workspace();
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(tmp.path())
        .arg("ls")
        .assert()
        .success()
        .stdout(predicate::str::contains("Pages (2):"))
        .stdout(predicate::str::contains("Raw sources (1):"))
        .stdout(predicate::str::contains("Embedded pages (1):"))
        .stdout(predicate::str::contains("Config:"));
}

#[test]
fn ls_pages_flag() {
    let tmp = make_workspace();
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(tmp.path())
        .arg("ls")
        .arg("--pages")
        .assert()
        .success()
        .stdout(predicate::str::contains("Pages (2):"))
        .stdout(predicate::str::contains("Test Page"))
        .stdout(predicate::str::contains("[1 chunks]"))
        .stdout(predicate::str::contains("other.md"))
        .stdout(predicate::str::contains("[not embedded]"))
        // Should NOT show other sections
        .stdout(predicate::str::contains("Raw sources").not())
        .stdout(predicate::str::contains("Embedded pages").not())
        .stdout(predicate::str::contains("Config:").not());
}

#[test]
fn ls_embed_flag() {
    let tmp = make_workspace();
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(tmp.path())
        .arg("ls")
        .arg("--embed")
        .assert()
        .success()
        .stdout(predicate::str::contains("Embedded pages (1):"))
        .stdout(predicate::str::contains(
            "wiki/test-page.md — 1 chunks, dim 2",
        ))
        .stdout(predicate::str::contains("Pages").not());
}

#[test]
fn ls_links_flag() {
    let tmp = make_workspace();
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(tmp.path())
        .arg("ls")
        .arg("--links")
        .assert()
        .success()
        .stdout(predicate::str::contains("Wikilinks (2):"))
        .stdout(predicate::str::contains("[[other]]"))
        .stdout(predicate::str::contains("[[test-page]]"));
}

#[test]
fn ls_config_flag() {
    let tmp = make_workspace();
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(tmp.path())
        .arg("ls")
        .arg("--config")
        .assert()
        .success()
        .stdout(predicate::str::contains("nim.embed_model:"))
        .stdout(predicate::str::contains("config_version:"));
}

#[test]
fn ls_config_flag_includes_nested_retry_keys() {
    // Reflection-based config listing must include nested keys like
    // `nim.retry.max_attempts` that the previous hardcoded version missed.
    let tmp = make_workspace();
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(tmp.path())
        .arg("ls")
        .arg("--config")
        .assert()
        .success()
        .stdout(predicate::str::contains("nim.retry.max_attempts:"))
        .stdout(predicate::str::contains("nim.retry.backoff_ms:"))
        .stdout(predicate::str::contains("wiki.require_frontmatter:"));
}

#[test]
fn ls_raw_flag() {
    let tmp = make_workspace();
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(tmp.path())
        .arg("ls")
        .arg("--raw")
        .assert()
        .success()
        .stdout(predicate::str::contains("Raw sources (1):"))
        .stdout(predicate::str::contains("raw/source.txt"));
}

#[test]
fn ls_json_output_is_valid() {
    let tmp = make_workspace();
    let output = Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(tmp.path())
        .arg("ls")
        .arg("--json")
        .output()
        .unwrap();
    assert!(output.status.success());
    let v: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(v.get("pages").is_some());
    assert!(v.get("config").is_some());
    let pages = v["pages"].as_array().unwrap();
    assert_eq!(pages.len(), 2);
}

#[test]
fn ls_json_pages_only() {
    let tmp = make_workspace();
    let output = Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(tmp.path())
        .arg("ls")
        .arg("--pages")
        .arg("--json")
        .output()
        .unwrap();
    assert!(output.status.success());
    let v: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(v.get("pages").is_some());
    assert!(v.get("raw").is_none());
    assert!(v.get("config").is_none());
}

#[test]
fn ls_empty_workspace() {
    let tmp = tempdir().unwrap();
    fs::create_dir_all(tmp.path().join(".llmwiki-cli")).unwrap();
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(tmp.path())
        .arg("ls")
        .assert()
        .success()
        .stdout(predicate::str::contains("Pages (0):"));
}

#[test]
fn ls_nonexistent_workspace_uses_default() {
    // discover_workspace walks up and falls back to defaults — ls succeeds with empty pages
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg("/tmp/does-not-exist-llmwiki-test")
        .arg("ls")
        .assert()
        .success()
        .stdout(predicate::str::contains("Pages (0):"));
}
