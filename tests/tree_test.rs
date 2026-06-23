use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use std::fs;
use tempfile::tempdir;

fn make_wiki() -> tempfile::TempDir {
    let tmp = tempdir().unwrap();
    let ws = tmp.path();
    fs::create_dir_all(ws.join(".llmwiki-cli")).unwrap();
    fs::create_dir_all(ws.join("wiki")).unwrap();
    fs::write(
        ws.join("wiki/overview.md"),
        "---\ntitle: Overview\ntags: [meta]\n---\n\nWelcome.\n",
    )
    .unwrap();
    fs::write(ws.join("wiki/log.md"), "# Log\n\nChronological record.\n").unwrap();
    fs::write(
        ws.join("wiki/test-page.md"),
        "---\ntitle: Test Page\ntags: [rust, cli]\n---\n\nBody with [[overview]].\n",
    )
    .unwrap();
    tmp
}

#[test]
fn tree_default_shows_all_pages() {
    let tmp = make_wiki();
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(tmp.path())
        .arg("tree")
        .assert()
        .success()
        .stdout(predicate::str::contains("overview"))
        .stdout(predicate::str::contains("Overview"))
        .stdout(predicate::str::contains("test-page"))
        .stdout(predicate::str::contains("Test Page"))
        .stdout(predicate::str::contains("rust, cli"))
        .stdout(predicate::str::contains("log"));
}

#[test]
fn tree_json_output_is_valid() {
    let tmp = make_wiki();
    let output = Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(tmp.path())
        .arg("tree")
        .arg("--json")
        .output()
        .unwrap();
    assert!(output.status.success());
    let v: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(v.get("entries").is_some());
    let entries = v["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 3);
}

#[test]
fn tree_json_entries_have_expected_fields() {
    let tmp = make_wiki();
    let output = Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(tmp.path())
        .arg("tree")
        .arg("--json")
        .output()
        .unwrap();
    let v: Value = serde_json::from_slice(&output.stdout).unwrap();
    let entries = v["entries"].as_array().unwrap();
    for e in entries {
        assert!(e.get("slug").is_some());
        assert!(e.get("path").is_some());
        assert!(e.get("title").is_some());
        assert!(e.get("tags").is_some());
        assert!(e.get("embedded").is_some());
    }
}

#[test]
fn tree_empty_workspace() {
    let tmp = tempdir().unwrap();
    fs::create_dir_all(tmp.path().join(".llmwiki-cli")).unwrap();
    fs::create_dir_all(tmp.path().join("wiki")).unwrap();
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(tmp.path())
        .arg("tree")
        .assert()
        .success()
        .stdout(predicate::str::contains("(empty)"));
}

#[test]
fn tree_empty_workspace_json() {
    let tmp = tempdir().unwrap();
    fs::create_dir_all(tmp.path().join(".llmwiki-cli")).unwrap();
    fs::create_dir_all(tmp.path().join("wiki")).unwrap();
    let output = Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(tmp.path())
        .arg("tree")
        .arg("--json")
        .output()
        .unwrap();
    assert!(output.status.success());
    let v: Value = serde_json::from_slice(&output.stdout).unwrap();
    let entries = v["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 0);
}

#[test]
fn tree_nonexistent_workspace_uses_default() {
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg("/tmp/does-not-exist-llmwiki-tree-test")
        .arg("tree")
        .assert()
        .success()
        .stdout(predicate::str::contains("(empty)"));
}
