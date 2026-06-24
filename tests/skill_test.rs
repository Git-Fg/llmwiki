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
        .stdout(str::contains("Your LLM's persistent memory"));
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
        .stdout(str::contains("llmwiki-search"))
        .stdout(str::contains("llmwiki-config"));
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
        .stdout(str::contains("=== llmwiki-search ==="))
        .stdout(str::contains("=== llmwiki-config ==="))
        .stdout(str::contains("=== llmwiki-troubleshooting ==="));
}

#[test]
fn skill_get_all_with_topic_errors() {
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("skill")
        .arg("get")
        .arg("llmwiki-search")
        .arg("--all")
        .assert()
        .failure()
        .stderr(str::contains("cannot use --all with a topic"));
}

/// v0.3.36+: legacy `wiki-X` topic names are NOT supported aliases
/// (hard cut). `skill get wiki-search` must fail the same as any
/// other unknown topic name. Guards against accidental alias
/// re-introduction.
#[test]
fn skill_get_rejects_legacy_wiki_topic_name() {
    use predicates::prelude::PredicateBooleanExt;
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("skill")
        .arg("get")
        .arg("wiki-search")
        .assert()
        .failure()
        .stderr(str::contains("unknown topic").or(str::contains("not found")));
}

#[test]
fn skill_install_writes_to_llmwiki_directory() {
    // v0.3.36+: install path is `~/.agents/skills/llmwiki/`, not
    // `.../wiki/`. Hard cut — no backward-compat path.
    let tmp = tempfile::tempdir().expect("tempdir");
    let home = tmp.path().to_path_buf();

    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .env("HOME", &home)
        .env("USERPROFILE", &home) // Windows compat
        .arg("install-skill")
        .arg("--global")
        .assert()
        .success();

    let skill_path = home.join(".agents/skills/llmwiki/SKILL.md");
    assert!(
        skill_path.exists(),
        "skill should install to ~/.agents/skills/llmwiki/SKILL.md, got: {}",
        skill_path.display()
    );
}
