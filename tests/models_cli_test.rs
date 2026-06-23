use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
use predicates::str;

#[test]
fn wiki_models_lists_all() {
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("models")
        .assert()
        .success()
        .stdout(str::contains("nv-embed-v1"))
        .stdout(str::contains("llama-nemotron-embed-1b-v2"));
}

#[test]
fn wiki_models_embed_filters() {
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("models")
        .arg("--embed")
        .assert()
        .success()
        .stdout(str::contains("nv-embed-v1"))
        .stdout(str::contains("rerank").not());
}

#[test]
fn wiki_models_commercial_filters() {
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("models")
        .arg("--commercial")
        .assert()
        .success()
        .stdout(str::contains("non-commercial").not());
}

#[test]
fn wiki_models_json_outputs_structured() {
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("models")
        .arg("--json")
        .assert()
        .success()
        .stdout(str::contains("\"name\": \"nvidia/nv-embed-v1\""));
}
