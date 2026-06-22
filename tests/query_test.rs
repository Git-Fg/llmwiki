use assert_cmd::Command;
use predicates::str;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn setup_wiki() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    let wiki = tmp.path();
    std::fs::create_dir_all(wiki.join("wiki")).unwrap();
    std::fs::write(
        wiki.join("wiki/a.md"),
        "---\ntitle: A\n---\n\nContent about attention mechanisms.\n",
    )
    .unwrap();
    let emb = r#"{"path":"wiki/a.md","sha256":"x","model":"nvidia/nv-embed-v1","dim":4,"chunked":false,"chunks":[{"start":0,"end":40,"tokens":10,"embedding":[0.9,0.1,0.0,0.0]}],"embedded_at":"2026-06-21T00:00:00Z"}"#;
    std::fs::write(wiki.join("embeddings.jsonl"), emb).unwrap();
    tmp
}

#[tokio::test]
async fn query_returns_synthesized_answer() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/embeddings"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{"embedding": [1.0, 0.0, 0.0, 0.0]}]
        })))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [{"message": {"content": "Attention mechanisms are explained in the wiki[^1]."}}]
        })))
        .mount(&server)
        .await;

    let tmp = setup_wiki();
    Command::cargo_bin("wiki")
        .unwrap()
        .arg("--workspace")
        .arg(tmp.path())
        .env("NVIDIA_NIM_API_KEY", "test-key")
        .env("WIKI_NIM_BASE_URL", server.uri())
        .arg("query")
        .arg("what is attention?")
        .assert()
        .success()
        .stdout(str::contains("Attention mechanisms"));
}

#[tokio::test]
async fn query_json_output_includes_citations() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/embeddings"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{"embedding": [1.0, 0.0, 0.0, 0.0]}]
        })))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [{"message": {"content": "Answer[^1]."}}]
        })))
        .mount(&server)
        .await;

    let tmp = setup_wiki();
    Command::cargo_bin("wiki")
        .unwrap()
        .arg("--workspace")
        .arg(tmp.path())
        .env("NVIDIA_NIM_API_KEY", "test-key")
        .env("WIKI_NIM_BASE_URL", server.uri())
        .arg("query")
        .arg("x")
        .arg("--json")
        .assert()
        .success()
        .stdout(str::contains("\"citations\""));
}
