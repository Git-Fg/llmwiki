use assert_cmd::Command;
use predicates::str;

#[test]
fn skill_show_prints_full_content() {
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("skill")
        .arg("show")
        .assert()
        .success()
        .stdout(str::contains("Personal Karpathy-style"));
}

#[test]
fn skill_show_topic_filters() {
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("skill")
        .arg("show")
        .arg("search")
        .assert()
        .success()
        .stdout(str::contains("semantic similarity"));
}

#[test]
fn skill_list_shows_all_topics() {
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("skill")
        .arg("list")
        .assert()
        .success()
        .stdout(str::contains("setup"))
        .stdout(str::contains("ingest"))
        .stdout(str::contains("troubleshooting"));
}

#[test]
fn skill_show_unknown_topic_errors() {
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("skill")
        .arg("show")
        .arg("nonexistent")
        .assert()
        .failure();
}

#[test]
fn skill_list_json_outputs_valid_json() {
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("skill")
        .arg("list")
        .arg("--json")
        .assert()
        .success()
        .stdout(str::contains("\"name\""))
        .stdout(str::contains("\"lines\""))
        .stdout(str::contains("wiki-search"))
        .stdout(str::contains("wiki-config"));
}

#[test]
fn skill_get_all_prints_every_subskill() {
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("skill")
        .arg("get")
        .arg("--all")
        .assert()
        .success()
        .stdout(str::contains("=== wiki-search ==="))
        .stdout(str::contains("=== wiki-config ==="))
        .stdout(str::contains("=== wiki-troubleshooting ==="));
}

#[test]
fn skill_get_all_with_topic_errors() {
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("skill")
        .arg("get")
        .arg("wiki-search")
        .arg("--all")
        .assert()
        .failure()
        .stderr(str::contains("cannot use --all with a topic"));
}
