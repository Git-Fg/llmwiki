use assert_cmd::Command;
use predicates::str;
use serde_json::json;
use wiremock::matchers::{bearer_token, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn doctor_uses_correct_models_endpoint() {
    // Regression: ensure doctor hits /models not /v1/models
    // since base_url already ends in /v1.
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir(tmp.path().join(".wiki")).unwrap();

    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(bearer_token("test-key"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list", "data": []
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    Command::cargo_bin("wiki")
        .unwrap()
        .arg("--workspace")
        .arg(tmp.path())
        .env("WIKI_NIM_BASE_URL", mock_server.uri())
        .env("NVIDIA_API_KEY", "test-key")
        .env_remove("NVIDIA_NIM_API_KEY")
        .arg("doctor")
        .assert()
        .code(0);

    // Verify the request landed on /v1/models.
    let reqs = mock_server.received_requests().await.unwrap_or_default();
    let paths: Vec<String> = reqs.iter().map(|r| r.url.path().to_string()).collect();
    assert!(
        paths.iter().any(|p| p == "/v1/models"),
        "expected /v1/models request, got {paths:?}"
    );
    assert!(
        !paths.iter().any(|p| p.contains("/v1/v1")),
        "doctor hit the wrong path: {paths:?}"
    );
}

#[test]
fn doctor_reports_missing_api_key() {
    let tmp = tempfile::tempdir().unwrap();
    let wiki = tmp.path();
    std::fs::create_dir(wiki.join(".wiki")).unwrap();

    Command::cargo_bin("wiki")
        .unwrap()
        .arg("--workspace")
        .arg(wiki)
        .env_remove("NVIDIA_NIM_API_KEY")
        .env_remove("NVIDIA_API_KEY")
        .arg("doctor")
        .assert()
        .code(3)
        .stderr(str::contains("API key"));
}
