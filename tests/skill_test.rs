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
