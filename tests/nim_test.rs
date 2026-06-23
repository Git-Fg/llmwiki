use llmwiki_cli::core::nim::NimClient;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn embed_returns_vectors() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/embeddings"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{"embedding": [0.1, 0.2, 0.3]}]
        })))
        .mount(&server)
        .await;

    let client = NimClient::new(server.uri(), "test-key".into());
    let result = client
        .embed(&["hello"], "nvidia/nv-embed-v1", "passage")
        .await
        .unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0], vec![0.1, 0.2, 0.3]);
}

#[tokio::test]
async fn embed_retries_on_500() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/embeddings"))
        .respond_with(ResponseTemplate::new(500))
        .up_to_n_times(2)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/embeddings"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{"embedding": [1.0]}]
        })))
        .mount(&server)
        .await;

    let client = NimClient::new(server.uri(), "test-key".into()).with_max_attempts(3);
    let result = client.embed(&["x"], "model", "passage").await.unwrap();
    assert_eq!(result[0], vec![1.0]);
}

#[tokio::test]
async fn embed_returns_error_on_401() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/embeddings"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;

    let client = NimClient::new(server.uri(), "bad-key".into()).with_max_attempts(1);
    let result = client.embed(&["x"], "model", "passage").await;
    assert!(result.is_err());
}
