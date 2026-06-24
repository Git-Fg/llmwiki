// Integration test for the full wiki pipeline using a bundled wiremock server.
// No #[ignore] — runs in CI via `cargo test`.
//
// Covers: init → embed (wiremock NIM) → search → status → lint (pass/fail).
//
// Key invariants exercised:
// - WIKI_NIM_BASE_URL + NVIDIA_API_KEY fallback work end-to-end
// - The NimClient builds /v1/embeddings (not /v1/v1/embeddings)
// - Valid frontmatter passes; invalid frontmatter is detected

use assert_cmd::Command;
use predicates::str;
use wiremock::matchers::{bearer_token, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn valid_page(title: &str, outbound: Vec<&str>, body: &str) -> String {
    format!(
        "---\n\
         title: {title}\n\
         schema_version: 1\n\
         created: 2026-06-22\n\
         updated: 2026-06-22\n\
         type: concept\n\
         tags: [reference]\n\
         sources: [raw/page.md]\n\
         ---\n\n\
         # {title}\n\n\
         {body}\n\n\
         {links}\n",
        links = outbound
            .iter()
            .map(|l| format!("See also: {l}."))
            .collect::<Vec<_>>()
            .join("\n")
    )
}

fn wiki() -> Command {
    Command::cargo_bin("llmwiki-cli").unwrap()
}

#[tokio::test]
async fn full_pipeline_init_embed_search_status_lint() {
    let server = MockServer::start().await;

    // wiremock: verify the CLI sends POST /v1/embeddings (not /v1/v1/embeddings)
    Mock::given(method("POST"))
        .and(bearer_token("test-key"))
        .and(path("/v1/embeddings"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {"embedding": [0.9, 0.1, 0.0, 0.0]},
                {"embedding": [0.0, 0.1, 0.9, 0.0]},
                {"embedding": [0.5, 0.5, 0.0, 0.0]}
            ]
        })))
        .mount(&server)
        .await;

    let tmp = tempfile::tempdir().unwrap();
    let wiki_dir = tmp.path();

    // 1. init (use --subdir for legacy wiki/ layout since test fixtures use wiki/ paths)
    wiki()
        .arg("init")
        .arg(wiki_dir)
        .arg("--subdir")
        .assert()
        .success();

    // 2. remove the init-template files (overview.md, log.md) — they don't have
    //    valid frontmatter by lint's standards; the real content is what we test.
    let _ = std::fs::remove_file(wiki_dir.join("wiki/overview.md"));
    let _ = std::fs::remove_file(wiki_dir.join("wiki/log.md"));

    // 3. write valid fixtures (all required frontmatter, ≥2 outbound links)
    std::fs::write(
        wiki_dir.join("wiki/page-one.md"),
        valid_page(
            "Page One",
            vec!["[[page-two]]", "[[page-three]]"],
            "Body of page one.",
        ),
    )
    .unwrap();
    std::fs::write(
        wiki_dir.join("wiki/page-two.md"),
        valid_page(
            "Page Two",
            vec!["[[page-one]]", "[[page-three]]"],
            "Body of page two.",
        ),
    )
    .unwrap();
    std::fs::write(
        wiki_dir.join("wiki/page-three.md"),
        valid_page(
            "Page Three",
            vec!["[[page-one]]", "[[page-two]]"],
            "Body of page three.",
        ),
    )
    .unwrap();

    // 4. embed via wiremock NIM (NVIDIA_API_KEY fallback tested here)
    wiki()
        .arg("--workspace")
        .arg(wiki_dir)
        .env("NVIDIA_API_KEY", "test-key")
        .env("WIKI_NIM_BASE_URL", server.uri())
        .env_remove("NVIDIA_NIM_API_KEY")
        .arg("embed")
        .assert()
        .success();

    // 5. search returns ranked results
    wiki()
        .arg("--workspace")
        .arg(wiki_dir)
        .env("NVIDIA_API_KEY", "test-key")
        .env("WIKI_NIM_BASE_URL", server.uri())
        .arg("search")
        .arg("body one")
        .assert()
        .success()
        .stdout(str::contains("page-one"));

    // 6. status shows embedded pages
    wiki()
        .arg("--workspace")
        .arg(wiki_dir)
        .env("NVIDIA_API_KEY", "test-key")
        .env("WIKI_NIM_BASE_URL", server.uri())
        .arg("status")
        .assert()
        .success()
        .stdout(str::contains("Embedded:"));

    // 7. wiremock verifies the URL shape (no /v1/v1/)
    let received = server.received_requests().await.unwrap_or_default();
    let paths: Vec<&str> = received.iter().map(|r| r.url.path()).collect();
    assert!(
        !paths.iter().any(|p| p.contains("/v1/v1")),
        "no request should hit /v1/v1, got {paths:?}"
    );
    assert!(
        paths.contains(&"/v1/embeddings"),
        "expected POST /v1/embeddings, got {paths:?}"
    );

    // 8. lint passes on valid fixtures
    wiki()
        .arg("--workspace")
        .arg(wiki_dir)
        .arg("lint")
        .arg("--scope")
        .arg("wiki")
        .assert()
        .success();

    // 9. lint fails (code 2) on a broken fixture
    std::fs::write(wiki_dir.join("wiki/broken.md"), "no frontmatter at all").unwrap();
    wiki()
        .arg("--workspace")
        .arg(wiki_dir)
        .arg("lint")
        .arg("--scope")
        .arg("wiki")
        .assert()
        .code(2);
}
