use assert_cmd::Command;
use predicates::str;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn setup_wiki_with_embeddings() -> (tempfile::TempDir, MockServer) {
    let server = MockServer::start().await;
    let tmp = tempfile::tempdir().unwrap();
    let wiki = tmp.path();
    std::fs::create_dir_all(wiki.join("wiki")).unwrap();
    std::fs::write(
        wiki.join("wiki/a.md"),
        "---\ntitle: A\n---\n\nContent about attention mechanisms.\n",
    )
    .unwrap();
    std::fs::write(
        wiki.join("wiki/b.md"),
        "---\ntitle: B\n---\n\nContent about transformers and embeddings.\n",
    )
    .unwrap();

    let emb = r#"{"path":"wiki/a.md","sha256":"x","model":"nvidia/nv-embed-v1","dim":4,"chunked":false,"chunks":[{"start":0,"end":40,"tokens":10,"embedding":[0.9,0.1,0.0,0.0]}],"embedded_at":"2026-06-21T00:00:00Z"}
{"path":"wiki/b.md","sha256":"y","model":"nvidia/nv-embed-v1","dim":4,"chunked":false,"chunks":[{"start":0,"end":40,"tokens":10,"embedding":[0.0,0.1,0.9,0.0]}],"embedded_at":"2026-06-21T00:00:00Z"}"#;
    std::fs::write(wiki.join("embeddings.jsonl"), emb).unwrap();
    (tmp, server)
}

#[tokio::test]
async fn search_returns_top_match() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/embeddings"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{"embedding": [1.0, 0.0, 0.0, 0.0]}]
        })))
        .mount(&server)
        .await;

    let (tmp, _s) = setup_wiki_with_embeddings().await;
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(tmp.path())
        .env("NVIDIA_NIM_API_KEY", "test-key")
        .env("WIKI_NIM_BASE_URL", server.uri())
        .arg("search")
        .arg("attention")
        .assert()
        .success()
        .stdout(str::contains("wiki/a.md"));
}

#[tokio::test]
async fn search_json_output() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/embeddings"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{"embedding": [1.0, 0.0, 0.0, 0.0]}]
        })))
        .mount(&server)
        .await;

    let (tmp, _s) = setup_wiki_with_embeddings().await;
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(tmp.path())
        .env("NVIDIA_NIM_API_KEY", "test-key")
        .env("WIKI_NIM_BASE_URL", server.uri())
        .arg("search")
        .arg("attention")
        .arg("--json")
        .assert()
        .success()
        .stdout(str::contains("\"path\":\"wiki/a.md\""));
}

#[tokio::test]
async fn search_returns_error_when_no_embeddings() {
    let tmp = tempfile::tempdir().unwrap();
    let wiki = tmp.path();
    std::fs::create_dir_all(wiki.join("wiki")).unwrap();
    std::fs::write(wiki.join("wiki/a.md"), "Body").unwrap();

    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(wiki)
        .env("NVIDIA_NIM_API_KEY", "test-key")
        .env("WIKI_NIM_BASE_URL", "http://localhost:1")
        .arg("search")
        .arg("test")
        .assert()
        .failure()
        .stderr(str::contains("no embeddings"));
}

#[tokio::test]
async fn search_fails_fast_on_bad_embed_model() {
    use wiremock::matchers::{method, path};

    let server = MockServer::start().await;
    // Mount a strict matcher: if the server receives ANY request, the test fails.
    Mock::given(method("POST"))
        .and(path("/v1/embeddings"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{"embedding": [0.1, 0.2]}]
        })))
        .expect(0) // STRICT: zero calls expected
        .mount(&server)
        .await;

    // Build workspace + isolated registry with a bogus embed_model.
    let tmp = tempfile::tempdir().unwrap();
    let wiki = tmp.path();
    std::fs::create_dir_all(wiki.join("wiki")).unwrap();
    std::fs::write(
        wiki.join("wiki/a.md"),
        "---\ntitle: A\n---\n\nBody about attention.\n",
    )
    .unwrap();
    let jsonl = r#"{"path":"wiki/a.md","sha256":"x","model":"nvidia/nv-embed-v1","dim":2,"chunked":false,"chunks":[{"start":0,"end":20,"tokens":5,"embedding":[0.9,0.1]}],"embedded_at":"2026-06-22T00:00:00Z"}"#;
    std::fs::write(wiki.join("embeddings.jsonl"), jsonl).unwrap();

    let reg_dir = tempfile::tempdir().unwrap();
    let reg_path = reg_dir.path().join("wiki-root.toml");
    std::fs::write(
        &reg_path,
        format!(
            r#"
[w]
path = "{}"

[w.nim]
embed_model = "nvidia/this-model-does-not-exist"
"#,
            wiki.display()
        ),
    )
    .unwrap();

    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .env("WIKI_ROOT_CONFIG", &reg_path)
        .env("NVIDIA_NIM_API_KEY", "test-key")
        .env("WIKI_NIM_BASE_URL", server.uri())
        .arg("--wiki")
        .arg("w")
        .arg("search")
        .arg("attention")
        .assert()
        .failure()
        .stderr(predicates::str::contains("unsupported embed_model"));

    server.verify().await; // Asserts zero requests reached the server.
}
