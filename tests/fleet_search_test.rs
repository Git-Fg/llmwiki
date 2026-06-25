//! Integration tests for fleet fallback search (v0.3.37+).
//!
//! When `discover_workspace()` fails AND the user didn't pin a wiki
//! explicitly, `search` and `query` fall back to fleet mode — embedding
//! the query once and searching every registered wiki with
//! `embeddings.jsonl`.
//!
//! These tests use wiremock to mock the NIM embeddings endpoint, so they
//! run in CI without a real API key.

use assert_cmd::Command;
use std::io::Write;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Write a wiki-root.toml with the given aliases pointing at the given
/// workspace paths. Returns the path to the registry file.
fn write_registry(
    dir: &std::path::Path,
    entries: &[(&str, &std::path::Path)],
) -> std::path::PathBuf {
    let reg_path = dir.join("wiki-root.toml");
    let mut body = String::new();
    for (alias, path) in entries {
        body.push_str(&format!("[{}]\npath = \"{}\"\n\n", alias, path.display()));
    }
    let mut f = std::fs::File::create(&reg_path).unwrap();
    f.write_all(body.as_bytes()).unwrap();
    reg_path
}

/// Write a minimal embeddings.jsonl with one page containing the given
/// embedding vector. The page path and model match what the CLI expects.
fn write_embeddings(wiki_dir: &std::path::Path, page_path: &str, embedding: Vec<f32>) {
    let entry = serde_json::json!({
        "path": page_path,
        "sha256": "fake-hash-000000000000000000000000000000000000000000000000000000",
        "model": "nvidia/nv-embed-v1",
        "dim": embedding.len(),
        "chunked": false,
        "chunks": [{
            "start": 0,
            "end": 100,
            "tokens": 50,
            "embedding": embedding,
        }],
        "embedded_at": "2026-06-24T00:00:00Z",
    });
    let jsonl = wiki_dir.join("embeddings.jsonl");
    let mut f = std::fs::File::create(&jsonl).unwrap();
    writeln!(f, "{entry}").unwrap();
}

/// Write a minimal wiki page file so fleet query can read content.
fn write_page(wiki_dir: &std::path::Path, page: &str, content: &str) {
    let path = wiki_dir.join(format!("{page}.md"));
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, content).unwrap();
}

fn wiki() -> Command {
    Command::cargo_bin("llmwiki-cli").unwrap()
}

/// Helper: run `search` with a hermetic registry and no explicit workspace/wiki.
/// This forces the fleet fallback path (no CWD match, no env, no flag).
fn fleet_cmd(reg_path: &std::path::Path, nim_url: &str) -> Command {
    let mut cmd = wiki();
    cmd.env("WIKI_ROOT_CONFIG", reg_path)
        .env("NVIDIA_NIM_API_KEY", "test-key")
        .env("WIKI_NIM_BASE_URL", nim_url)
        .env_remove("WIKI_WORKSPACE")
        .env_remove("WIKI_ACTIVE");
    // CWD must NOT be inside any wiki — use a temp dir that is NOT registered
    let tmp = tempfile::tempdir().unwrap();
    // Leak the tempdir so it stays alive for the duration of the command
    std::mem::forget(tmp);
    cmd
}

// ─── Fleet search ────────────────────────────────────────────────────────────

/// Two wikis with embeddings, one query → results from both wikis appear.
/// The query embedding is [1.0, 0.0, 0.0] which has cosine similarity 1.0
/// with wiki-alpha's page (also [1, 0, 0]) and 0.0 with wiki-beta's page
/// ([0, 1, 0]). So only wiki-alpha should appear in results.
#[tokio::test]
async fn fleet_search_returns_results_from_matching_wiki() {
    let server = MockServer::start().await;

    // Mock: the query embedding is [1.0, 0.0, 0.0]
    Mock::given(method("POST"))
        .and(path("/v1/embeddings"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{"embedding": [1.0, 0.0, 0.0]}]
        })))
        .mount(&server)
        .await;

    let reg_dir = tempfile::tempdir().unwrap();
    let wiki_a = tempfile::tempdir().unwrap();
    let wiki_b = tempfile::tempdir().unwrap();

    // Wiki A: page with embedding [1, 0, 0] — matches query
    write_embeddings(wiki_a.path(), "alpha-page.md", vec![1.0, 0.0, 0.0]);
    write_page(wiki_a.path(), "alpha-page", "# Alpha\nContent");

    // Wiki B: page with embedding [0, 1, 0] — orthogonal to query
    write_embeddings(wiki_b.path(), "beta-page.md", vec![0.0, 1.0, 0.0]);
    write_page(wiki_b.path(), "beta-page", "# Beta\nContent");

    let reg_path = write_registry(
        reg_dir.path(),
        &[("alpha-wiki", wiki_a.path()), ("beta-wiki", wiki_b.path())],
    );

    let output = fleet_cmd(&reg_path, &server.uri())
        .arg("search")
        .arg("unique keyword")
        .arg("--json")
        .output()
        .expect("fleet search must run");

    assert!(
        output.status.success(),
        "fleet search failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v["fleet"], true, "fleet flag should be true");
    assert!(
        v["wikis_searched"].as_array().unwrap().len() == 2,
        "should have searched 2 wikis: {v}"
    );
    // Only alpha-wiki's page should match (cosine sim 1.0 > threshold)
    let results = v["results"].as_array().unwrap();
    assert!(!results.is_empty(), "should have at least one result");
    assert_eq!(
        results[0]["wiki"], "alpha-wiki",
        "top result should be from alpha-wiki: {v}"
    );
}

/// Fleet search skips wikis that don't have embeddings.jsonl.
#[tokio::test]
async fn fleet_search_skips_wikis_without_embeddings() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/embeddings"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{"embedding": [1.0, 0.0]}]
        })))
        .mount(&server)
        .await;

    let reg_dir = tempfile::tempdir().unwrap();
    let wiki_with_emb = tempfile::tempdir().unwrap();
    let wiki_without_emb = tempfile::tempdir().unwrap();

    // Only wiki A has embeddings
    write_embeddings(wiki_with_emb.path(), "page.md", vec![1.0, 0.0]);
    write_page(wiki_with_emb.path(), "page", "# Page\nContent");

    // Wiki B exists but has no embeddings.jsonl
    std::fs::write(wiki_without_emb.path().join("README.md"), "not a wiki").unwrap();

    let reg_path = write_registry(
        reg_dir.path(),
        &[
            ("with-emb", wiki_with_emb.path()),
            ("no-emb", wiki_without_emb.path()),
        ],
    );

    let output = fleet_cmd(&reg_path, &server.uri())
        .arg("search")
        .arg("keyword")
        .arg("--json")
        .output()
        .expect("fleet search must run");

    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v["wikis_skipped"], 1, "should skip 1 wiki: {v}");
    let searched = v["wikis_searched"].as_array().unwrap();
    assert_eq!(searched.len(), 1, "should search 1 wiki: {v}");
    assert_eq!(searched[0], "with-emb");
}

/// When --wiki is passed but the alias doesn't resolve, the error should
/// be the explicit-resolution error (NOT fleet fallback).
#[tokio::test]
async fn fleet_search_respects_explicit_wiki_failure() {
    let server = MockServer::start().await;

    let reg_dir = tempfile::tempdir().unwrap();
    let reg_path = write_registry(reg_dir.path(), &[]);

    let output = fleet_cmd(&reg_path, &server.uri())
        .arg("--wiki")
        .arg("nonexistent-alias")
        .arg("search")
        .arg("keyword")
        .output()
        .expect("search must run");

    // Should FAIL (explicit --wiki was given, alias doesn't exist)
    assert!(
        !output.status.success(),
        "should fail on explicit --wiki with unknown alias"
    );
}

/// Single-wiki search (via --wiki) should NOT produce fleet output.
#[tokio::test]
async fn single_wiki_search_no_fleet_flag() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/embeddings"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{"embedding": [1.0, 0.0]}]
        })))
        .mount(&server)
        .await;

    let reg_dir = tempfile::tempdir().unwrap();
    let wiki_dir = tempfile::tempdir().unwrap();
    write_embeddings(wiki_dir.path(), "page.md", vec![1.0, 0.0]);
    write_page(wiki_dir.path(), "page", "# Page\nContent");

    let reg_path = write_registry(reg_dir.path(), &[("single", wiki_dir.path())]);

    let mut cmd = wiki();
    cmd.env("WIKI_ROOT_CONFIG", &reg_path)
        .env("NVIDIA_NIM_API_KEY", "test-key")
        .env("WIKI_NIM_BASE_URL", server.uri())
        .env_remove("WIKI_WORKSPACE")
        .env_remove("WIKI_ACTIVE");

    let output = cmd
        .arg("--wiki")
        .arg("single")
        .arg("search")
        .arg("keyword")
        .arg("--json")
        .output()
        .expect("single-wiki search must run");

    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        v["fleet"], false,
        "single-wiki search should NOT set fleet=true: {v}"
    );
    assert!(
        v.get("wikis_searched").is_none(),
        "single-wiki search should NOT have wikis_searched: {v}"
    );
}
