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
    std::fs::create_dir(tmp.path().join(".llmwiki-cli")).unwrap();

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

    Command::cargo_bin("llmwiki-cli")
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

#[tokio::test]
async fn doctor_json_output() {
    let tmp = tempfile::tempdir().unwrap();
    let wiki = tmp.path();
    std::fs::create_dir(wiki.join(".llmwiki-cli")).unwrap();

    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(bearer_token("test-key"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list", "data": []
        })))
        .mount(&mock_server)
        .await;

    let output = Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(wiki)
        .env("WIKI_NIM_BASE_URL", mock_server.uri())
        .env("NVIDIA_API_KEY", "test-key")
        .env_remove("NVIDIA_NIM_API_KEY")
        .arg("doctor")
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v["config_loaded"], true);
    assert_eq!(v["nim_reachable"], true);
    assert_eq!(v["api_key_length"], 8);
    assert!(v.get("workspace").is_some());
}

#[tokio::test]
async fn doctor_reports_missing_api_key() {
    let tmp = tempfile::tempdir().unwrap();
    let wiki = tmp.path();
    std::fs::create_dir(wiki.join(".llmwiki-cli")).unwrap();

    let mock_server = MockServer::start().await;

    // wiremock rejects unauthenticated requests with 401
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({
            "error": "unauthorized"
        })))
        .mount(&mock_server)
        .await;

    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(wiki)
        .env("WIKI_NIM_BASE_URL", mock_server.uri())
        .env_remove("NVIDIA_NIM_API_KEY")
        .env_remove("NVIDIA_API_KEY")
        .arg("doctor")
        .assert()
        .code(3)
        .stderr(str::contains("API key"));
}

// ─── v0.3.12: doctor reports per-key config source attribution ───

#[tokio::test]
async fn doctor_json_includes_config_sources_attribution() {
    // When both per-workspace and per-computer config files set the same
    // key, the doctor JSON must report which file each key came from.
    // This is the same attribution shown by `llmwiki-cli config show-effective`
    // and lets users audit precedence without running a separate command.
    let tmp = tempfile::tempdir().unwrap();
    let reg_path = tmp.path().join("wiki-root.toml");
    std::fs::write(&reg_path, "# empty registry\n").unwrap();
    let home = tmp.path().join("home");
    let workspace = tmp.path().join("ws");
    std::fs::create_dir_all(home.join(".llmwiki-cli")).unwrap();
    std::fs::create_dir_all(workspace.join(".llmwiki-cli")).unwrap();

    // Per-workspace sets nim.embed_model and wiki.require_frontmatter
    std::fs::write(
        workspace.join(".llmwiki-cli").join("config.toml"),
        "[nim]\nembed_model = \"nvidia/nv-embedcode-7b-v1\"\n[wiki]\nrequire_frontmatter = false\n",
    )
    .unwrap();
    // Per-computer sets nim.base_url (different key from per-workspace)
    std::fs::write(
        home.join(".llmwiki-cli").join("config.toml"),
        "[nim]\nbase_url = \"https://integrate.api.nvidia.com\"\n",
    )
    .unwrap();

    let mock_server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path("/v1/models"))
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(json!({
            "object": "list", "data": []
        })))
        .mount(&mock_server)
        .await;

    let output = Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(&workspace)
        .env("WIKI_ROOT_CONFIG", &reg_path)
        .env("HOME", &home)
        .env_remove("USERPROFILE")
        .env("WIKI_NIM_BASE_URL", mock_server.uri())
        .env("NVIDIA_NIM_API_KEY", "test-key")
        .env_remove("LLMWIKI_CONFIG")
        .arg("doctor")
        .arg("--json")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "doctor failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let sources = v["config_sources"]
        .as_object()
        .expect("config_sources must be an object");

    // nim.embed_model → per-workspace
    let embed_src = sources["nim.embed_model"]
        .as_str()
        .expect("nim.embed_model must be attributed");
    assert!(
        embed_src.contains("ws/.llmwiki-cli/config.toml"),
        "nim.embed_model should come from per-workspace; got: {embed_src}"
    );
    // nim.base_url → per-computer
    let base_src = sources["nim.base_url"]
        .as_str()
        .expect("nim.base_url must be attributed");
    assert!(
        base_src.contains("home/.llmwiki-cli/config.toml"),
        "nim.base_url should come from per-computer; got: {base_src}"
    );
    // wiki.require_frontmatter → per-workspace
    let fm_src = sources["wiki.require_frontmatter"]
        .as_str()
        .expect("wiki.require_frontmatter must be attributed");
    assert!(
        fm_src.contains("ws/.llmwiki-cli/config.toml"),
        "wiki.require_frontmatter should come from per-workspace; got: {fm_src}"
    );
}
