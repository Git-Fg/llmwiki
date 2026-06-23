use assert_cmd::Command;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn setup_wiki_with_page() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    let wiki = tmp.path();
    std::fs::create_dir_all(wiki.join("wiki")).unwrap();
    std::fs::create_dir_all(wiki.join(".llmwiki-cli")).unwrap();
    std::fs::write(wiki.join("wiki/a.md"), "---\ntitle: A\n---\n\nBody of A.\n").unwrap();
    std::fs::write(
        wiki.join(".llmwiki-cli/config.toml"),
        "config_version = 1\n",
    )
    .unwrap();
    tmp
}

#[tokio::test]
async fn embed_writes_jsonl() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/embeddings"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{"embedding": [0.1, 0.2]}, {"embedding": [0.3, 0.4]}]
        })))
        .mount(&server)
        .await;

    let tmp = setup_wiki_with_page();
    let wiki_url = server.uri();
    let api_key = "test-key";

    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(tmp.path())
        .env("NVIDIA_NIM_API_KEY", api_key)
        .env("WIKI_NIM_BASE_URL", &wiki_url)
        .arg("embed")
        .assert()
        .success();

    let jsonl = std::fs::read_to_string(tmp.path().join("embeddings.jsonl")).unwrap();
    assert!(jsonl.contains("\"path\":\"wiki/a.md\""));
    assert!(jsonl.contains("\"sha256\""));
}

#[tokio::test]
async fn embed_skip_existing_skips_unchanged() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/embeddings"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{"embedding": [0.1, 0.2]}]
        })))
        .expect(1)
        .mount(&server)
        .await;
    let tmp = setup_wiki_with_page();
    let wiki_url = server.uri();

    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(tmp.path())
        .env("NVIDIA_NIM_API_KEY", "test-key")
        .env("WIKI_NIM_BASE_URL", &wiki_url)
        .arg("embed")
        .assert()
        .success();

    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(tmp.path())
        .env("NVIDIA_NIM_API_KEY", "test-key")
        .env("WIKI_NIM_BASE_URL", &wiki_url)
        .arg("embed")
        .arg("--skip-existing")
        .assert()
        .success();

    let jsonl = std::fs::read_to_string(tmp.path().join("embeddings.jsonl")).unwrap();
    assert_eq!(jsonl.lines().count(), 1);
}
