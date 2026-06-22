use assert_cmd::Command;
use predicates::str;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
#[ignore]
async fn full_pipeline_init_embed_search_lint() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/embeddings"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {"embedding": [0.9, 0.1, 0.0, 0.0]},
                {"embedding": [0.0, 0.1, 0.9, 0.0]}
            ]
        })))
        .mount(&server)
        .await;

    let tmp = tempfile::tempdir().unwrap();
    let wiki = tmp.path();

    Command::cargo_bin("wiki")
        .unwrap()
        .arg("init")
        .arg(wiki)
        .assert()
        .success();

    std::fs::create_dir_all(wiki.join("wiki")).unwrap();
    std::fs::write(
        wiki.join("wiki/page-one.md"),
        "---\ntitle: Page One\ncreated: 2026-06-21\nupdated: 2026-06-21\ntype: concept\ntags: [test]\nsources: []\n---\n\nBody one [[page-two]].\n",
    )
    .unwrap();
    std::fs::write(
        wiki.join("wiki/page-two.md"),
        "---\ntitle: Page Two\ncreated: 2026-06-21\nupdated: 2026-06-21\ntype: concept\ntags: [test]\nsources: []\n---\n\nBody two [[page-one]].\n",
    )
    .unwrap();

    Command::cargo_bin("wiki")
        .unwrap()
        .arg("--workspace")
        .arg(wiki)
        .env("NVIDIA_NIM_API_KEY", "test-key")
        .env("WIKI_NIM_BASE_URL", server.uri())
        .arg("embed")
        .assert()
        .success();

    Command::cargo_bin("wiki")
        .unwrap()
        .arg("--workspace")
        .arg(wiki)
        .env("NVIDIA_NIM_API_KEY", "test-key")
        .env("WIKI_NIM_BASE_URL", server.uri())
        .arg("search")
        .arg("body")
        .assert()
        .success()
        .stdout(str::contains("page-one"));

    Command::cargo_bin("wiki")
        .unwrap()
        .arg("--workspace")
        .arg(wiki)
        .arg("lint")
        .assert()
        .code(0);

    Command::cargo_bin("wiki")
        .unwrap()
        .arg("--workspace")
        .arg(wiki)
        .arg("status")
        .assert()
        .success()
        .stdout(str::contains("Pages: 3"));
}
