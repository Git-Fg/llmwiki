use std::io::Write;

fn write_tmp_toml(content: &str) -> (std::path::PathBuf, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("wiki-root.toml");
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
    (path, dir)
}

/// Run `wiki config paths` with a temp HOME, temp WIKI_ROOT_CONFIG, and
/// given `--workspace`. Returns (status, stdout).
fn run_config_paths(workspace: &std::path::Path) -> (i32, String) {
    use assert_cmd::Command;
    let tmp = tempfile::tempdir().unwrap();
    let registry = tmp.path().join("wiki-root.toml");
    std::fs::write(&registry, "# test wiki-root.toml\n").unwrap();

    let output = Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .env("WIKI_ROOT_CONFIG", &registry)
        .env("HOME", tmp.path()) // per-computer config also empty
        .env_remove("USERPROFILE")
        .env_remove("LLMWIKI_CONFIG")
        .arg("--workspace")
        .arg(workspace)
        .arg("config")
        .arg("paths")
        .output()
        .unwrap();
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).to_string(),
    )
}

/// Run `wiki config paths --json`. Returns parsed JSON.
fn run_config_paths_json(workspace: &std::path::Path) -> serde_json::Value {
    use assert_cmd::Command;
    let tmp = tempfile::tempdir().unwrap();
    let registry = tmp.path().join("wiki-root.toml");
    std::fs::write(&registry, "# test wiki-root.toml\n").unwrap();

    let output = Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .env("WIKI_ROOT_CONFIG", &registry)
        .env("HOME", tmp.path())
        .env_remove("USERPROFILE")
        .env_remove("LLMWIKI_CONFIG")
        .arg("--workspace")
        .arg(workspace)
        .arg("config")
        .arg("paths")
        .arg("--json")
        .output()
        .unwrap();
    serde_json::from_slice(&output.stdout).expect("config paths --json must return valid JSON")
}

#[test]
fn config_paths_prints_search_order_with_status() {
    let tmp = tempfile::tempdir().unwrap();
    let workspace = tmp.path();
    std::fs::create_dir_all(workspace.join(".llmwiki-cli")).unwrap();
    std::fs::write(
        workspace.join(".llmwiki-cli").join("config.toml"),
        "[nim]\n",
    )
    .unwrap();

    let (code, stdout) = run_config_paths(workspace);
    assert_eq!(code, 0, "config paths failed: {stdout}");
    assert!(
        stdout.contains("Workspace:"),
        "missing workspace line: {stdout}"
    );
    assert!(
        stdout.contains("Config search order"),
        "missing header: {stdout}"
    );
    assert!(
        stdout.contains("per-workspace"),
        "missing per-workspace label: {stdout}"
    );
    assert!(
        stdout.contains("per-computer"),
        "missing per-computer label: {stdout}"
    );
    assert!(
        stdout.contains("[exists"),
        "missing exists marker: {stdout}"
    );
}

#[test]
fn config_paths_json_returns_workspace_and_paths_array() {
    let tmp = tempfile::tempdir().unwrap();
    let workspace = tmp.path();
    let v = run_config_paths_json(workspace);
    assert!(v.get("workspace").is_some());
    assert!(v.get("paths").is_some());
    let paths = v["paths"].as_array().unwrap();
    assert!(!paths.is_empty());
    for entry in paths {
        assert!(entry.get("source").is_some());
        assert!(entry.get("path").is_some());
        assert!(entry.get("exists").is_some());
    }
}

#[test]
fn config_paths_reports_missing_per_workspace() {
    let tmp = tempfile::tempdir().unwrap();
    let workspace = tmp.path();
    // No .llmwiki-cli/config.toml in workspace.
    let (code, stdout) = run_config_paths(workspace);
    assert_eq!(code, 0);
    // Per-workspace should be reported as missing.
    let has_missing = stdout
        .lines()
        .any(|l| l.contains("[missing") && l.contains("per-workspace"));
    assert!(
        has_missing,
        "expected per-workspace missing entry: {stdout}"
    );
}

#[test]
fn config_path_prints_resolved_path() {
    let (path, _dir) = write_tmp_toml("[test]\npath = \"/tmp\"\n");
    let reg = llmwiki_cli::core::registry::Registry::load_from(&path).unwrap();
    assert_eq!(reg.root_path, path);
}

#[test]
fn config_list_shows_all_wikis() {
    let (path, _dir) = write_tmp_toml(
        r#"
[wiki1]
path = "/tmp/wiki1"
tags = ["a"]

[wiki2]
path = "/tmp/wiki2"
tags = ["b"]
"#,
    );
    let reg = llmwiki_cli::core::registry::Registry::load_from(&path).unwrap();
    assert_eq!(reg.entries.len(), 2);
}

#[test]
fn config_set_then_get_roundtrip() {
    let (path, _dir) = write_tmp_toml(
        r#"
[defaults.nim]
embed_model = "original"

[testwiki]
path = "/tmp/test"
"#,
    );
    let mut reg = llmwiki_cli::core::registry::Registry::load_from(&path).unwrap();
    reg.set_value("nim.embed_model", "new-model", Some("testwiki"))
        .unwrap();
    reg.save().unwrap();

    let reg2 = llmwiki_cli::core::registry::Registry::load_from(&path).unwrap();
    let cfg = reg2.resolve_config("testwiki").unwrap();
    assert_eq!(cfg.nim.embed_model, "new-model");
}

#[test]
fn config_add_then_remove() {
    let (path, _dir) = write_tmp_toml("[existing]\npath = \"/tmp\"\n");
    let mut reg = llmwiki_cli::core::registry::Registry::load_from(&path).unwrap();
    assert_eq!(reg.entries.len(), 1);

    reg.add_entry(
        "newwiki",
        std::path::Path::new("/tmp/new"),
        &["tag1".to_string()],
        Some("desc"),
    )
    .unwrap();
    assert_eq!(reg.entries.len(), 2);

    reg.remove_entry("newwiki").unwrap();
    assert_eq!(reg.entries.len(), 1);
}

#[test]
fn config_unset_reverts_to_default() {
    let (path, _dir) = write_tmp_toml(
        r#"
[defaults.nim]
embed_model = "default-model"

[testwiki]
path = "/tmp/test"

[testwiki.nim]
embed_model = "override-model"
"#,
    );
    let mut reg = llmwiki_cli::core::registry::Registry::load_from(&path).unwrap();
    let cfg = reg.resolve_config("testwiki").unwrap();
    assert_eq!(cfg.nim.embed_model, "override-model");

    reg.unset_value("nim.embed_model", "testwiki").unwrap();
    reg.save().unwrap();

    let reg2 = llmwiki_cli::core::registry::Registry::load_from(&path).unwrap();
    let cfg2 = reg2.resolve_config("testwiki").unwrap();
    assert_eq!(cfg2.nim.embed_model, "default-model");
}

#[test]
fn config_set_atomic_write_preserves_other_entries() {
    let (path, _dir) = write_tmp_toml(
        r#"
[keep]
path = "/tmp/keep"
description = "untouched"

[modify]
path = "/tmp/modify"
description = "modify me"
"#,
    );
    let mut reg = llmwiki_cli::core::registry::Registry::load_from(&path).unwrap();
    reg.set_value("embed_model", "newmodel", Some("modify"))
        .unwrap();
    reg.save().unwrap();

    // Reload and verify both entries still present
    let reg2 = llmwiki_cli::core::registry::Registry::load_from(&path).unwrap();
    assert_eq!(reg2.entries.len(), 2);
    let keep = reg2.entries.iter().find(|e| e.alias == "keep").unwrap();
    assert_eq!(keep.description, "untouched");
    let modify = reg2.entries.iter().find(|e| e.alias == "modify").unwrap();
    assert_eq!(modify.description, "modify me");
}

// ---------- Subprocess tests for the `wiki config` CLI surface ----------
//
// All subprocess tests redirect `WIKI_ROOT_CONFIG` to a temp file so the
// user's real registry is never mutated. The temp dir is kept alive for the
// duration of the test via `_dir`.

use assert_cmd::Command;
use predicates::str;

fn isolated_cmd(reg_path: &std::path::Path) -> Command {
    let mut cmd = Command::cargo_bin("llmwiki-cli").unwrap();
    cmd.env("WIKI_ROOT_CONFIG", reg_path).env_remove("HOME");
    cmd
}

fn isolated_registry_with(content: &str) -> (std::path::PathBuf, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("wiki-root.toml");
    std::fs::write(&path, content).unwrap();
    (path, dir)
}

#[test]
fn subprocess_config_path_prints_resolved_path() {
    let (reg_path, _dir) = isolated_registry_with("[x]\npath = \"/tmp/x\"\n");
    isolated_cmd(&reg_path)
        .arg("config")
        .arg("path")
        .assert()
        .success()
        .stdout(str::contains(reg_path.to_str().unwrap()));
}

#[test]
fn subprocess_config_list_shows_aliases() {
    let (reg_path, _dir) = isolated_registry_with(
        r#"
[alpha]
path = "/tmp/alpha"

[beta]
path = "/tmp/beta"
tags = ["foo"]
"#,
    );
    isolated_cmd(&reg_path)
        .arg("config")
        .arg("list")
        .assert()
        .success()
        .stdout(str::contains("alpha"))
        .stdout(str::contains("beta"));
}

#[test]
fn subprocess_config_get_default_value() {
    let (reg_path, _dir) = isolated_registry_with("");
    isolated_cmd(&reg_path)
        .arg("config")
        .arg("get")
        .arg("nim.embed_model")
        .assert()
        .success()
        .stdout(str::contains("nvidia/nv-embed-v1"));
}

#[test]
fn subprocess_config_get_nested_key() {
    let (reg_path, _dir) = isolated_registry_with(
        r#"
[defaults.nim]
embed_model = "nvidia/nv-embed-v1"

[defaults.nim.retry]
max_attempts = 7

[w]
path = "/tmp/w"
"#,
    );
    isolated_cmd(&reg_path)
        .args(["config", "get", "nim.retry.max_attempts", "--wiki", "w"])
        .assert()
        .success()
        .stdout(str::contains("7"));
}

#[test]
fn subprocess_config_get_unknown_key_lists_valid() {
    let (reg_path, _dir) = isolated_registry_with("[w]\npath = \"/tmp/w\"\n");
    isolated_cmd(&reg_path)
        .arg("config")
        .arg("get")
        .arg("nim.bogus")
        .assert()
        .failure()
        .stderr(str::contains("unknown config key"));
}

#[test]
fn subprocess_config_get_default_value_from_registry_defaults() {
    // Without --wiki, `config get` reads [defaults] from the registry, not
    // Config::default(). This proves [defaults] override the default Config.
    let (reg_path, _dir) = isolated_registry_with(
        r#"
[defaults.nim]
embed_model = "default-override-model"
"#,
    );
    isolated_cmd(&reg_path)
        .args(["config", "get", "nim.embed_model"])
        .assert()
        .success()
        .stdout(str::contains("default-override-model"));
}

#[test]
fn subprocess_config_set_then_get_roundtrip() {
    let (reg_path, _dir) = isolated_registry_with("[w]\npath = \"/tmp/w\"\n");
    isolated_cmd(&reg_path)
        .args([
            "config",
            "set",
            "nim.embed_model",
            "nvidia/nv-embedqa-e5-v5",
            "--wiki",
            "w",
        ])
        .assert()
        .success()
        .stdout(str::contains(
            "Set nim.embed_model = nvidia/nv-embedqa-e5-v5 in [w]",
        ));

    isolated_cmd(&reg_path)
        .args(["config", "get", "nim.embed_model", "--wiki", "w"])
        .assert()
        .success()
        .stdout(str::contains("nvidia/nv-embedqa-e5-v5"));
}

#[test]
fn subprocess_config_unset_requires_wiki_alias() {
    let (reg_path, _dir) = isolated_registry_with("[w]\npath = \"/tmp/w\"\n");
    isolated_cmd(&reg_path)
        .args(["config", "unset", "nim.embed_model"])
        .assert()
        .failure()
        .stderr(str::contains("--wiki"));
}

#[test]
fn subprocess_config_unset_reverts_to_default() {
    let (reg_path, _dir) = isolated_registry_with(
        r#"
[defaults.nim]
embed_model = "default-model"

[w]
path = "/tmp/w"

[w.nim]
embed_model = "override-model"
"#,
    );
    isolated_cmd(&reg_path)
        .args(["config", "unset", "nim.embed_model", "--wiki", "w"])
        .assert()
        .success();

    isolated_cmd(&reg_path)
        .args(["config", "get", "nim.embed_model", "--wiki", "w"])
        .assert()
        .success()
        .stdout(str::contains("default-model"));
}

#[test]
fn subprocess_config_add_then_rm() {
    let (reg_path, _dir) = isolated_registry_with("");
    let workspace = tempfile::tempdir().unwrap();

    isolated_cmd(&reg_path)
        .args([
            "config",
            "add",
            "newwiki",
            workspace.path().to_str().unwrap(),
            "--tag",
            "tag1",
            "--tag",
            "tag2",
            "--description",
            "a test wiki",
        ])
        .assert()
        .success()
        .stdout(str::contains("Added wiki 'newwiki'"));

    isolated_cmd(&reg_path)
        .args(["config", "list"])
        .assert()
        .success()
        .stdout(str::contains("newwiki"));

    isolated_cmd(&reg_path)
        .args(["config", "rm", "newwiki"])
        .assert()
        .success()
        .stdout(str::contains("Removed wiki 'newwiki'"));

    isolated_cmd(&reg_path)
        .args(["config", "list"])
        .assert()
        .success()
        .stdout(str::contains("No wikis registered"));
}

#[test]
fn subprocess_config_list_with_wiki_prints_all_keys() {
    let (reg_path, _dir) = isolated_registry_with("[w]\npath = \"/tmp/w\"\n");
    isolated_cmd(&reg_path)
        .args(["config", "list", "--wiki", "w"])
        .assert()
        .success()
        .stdout(str::contains("nim.embed_model"))
        .stdout(str::contains("nim.base_url"))
        .stdout(str::contains("nim.retry.max_attempts"))
        .stdout(str::contains("wiki.require_frontmatter"));
}

#[test]
fn subprocess_config_list_with_unknown_alias_errors() {
    let (reg_path, _dir) = isolated_registry_with("[w]\npath = \"/tmp/w\"\n");
    isolated_cmd(&reg_path)
        .args(["config", "list", "--wiki", "bogus"])
        .assert()
        .failure()
        .stderr(str::contains("bogus"));
}

#[test]
fn subprocess_config_validate_passes_for_valid_registry() {
    let (reg_path, _dir) = isolated_registry_with(
        r#"
[w]
path = "/tmp/w"
"#,
    );
    isolated_cmd(&reg_path)
        .args(["config", "validate"])
        .assert()
        .success()
        .stdout(str::contains("[w]"));
}

#[test]
fn subprocess_config_validate_detects_bad_embed_model() {
    let (reg_path, _dir) = isolated_registry_with(
        r#"
[w]
path = "/tmp/w"

[w.nim]
embed_model = "nvidia/not-a-real-model"
"#,
    );
    isolated_cmd(&reg_path)
        .args(["config", "validate"])
        .assert()
        .failure()
        .stdout(str::contains("[w]"))
        .stdout(str::contains("unsupported embed_model"));
}

#[test]
fn subprocess_config_validate_detects_zero_batch_size() {
    let (reg_path, _dir) = isolated_registry_with(
        r#"
[w]
path = "/tmp/w"

[w.nim]
batch_size = 0
"#,
    );
    isolated_cmd(&reg_path)
        .args(["config", "validate"])
        .assert()
        .failure()
        .stdout(str::contains("batch_size"));
}

#[test]
fn subprocess_config_validate_handles_empty_registry() {
    let (reg_path, _dir) = isolated_registry_with("");
    isolated_cmd(&reg_path)
        .args(["config", "validate"])
        .assert()
        .success()
        .stdout(str::contains("[defaults]"));
}

#[test]
fn subprocess_config_get_returns_unset_for_optional_field() {
    // `nim.embed_dim_override` is Option<usize>. When not set, it should
    // report "(unset)" rather than "unknown config key".
    let (reg_path, _dir) = isolated_registry_with("[w]\npath = \"/tmp/w\"\n");
    isolated_cmd(&reg_path)
        .args(["config", "get", "nim.embed_dim_override"])
        .assert()
        .success()
        .stdout(str::contains("(unset)"));
}

#[test]
fn subprocess_config_get_returns_value_for_set_optional_field() {
    let (reg_path, _dir) = isolated_registry_with(
        r#"
[w]
path = "/tmp/w"

[w.nim]
embed_dim_override = 768
"#,
    );
    isolated_cmd(&reg_path)
        .args(["config", "get", "nim.embed_dim_override", "--wiki", "w"])
        .assert()
        .success()
        .stdout(str::contains("768"));
}

#[test]
fn subprocess_config_show_schema_outputs_valid_json() {
    let (reg_path, _dir) = isolated_registry_with("");
    let output = isolated_cmd(&reg_path)
        .args(["config", "show-schema"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v["title"], "Config");
    assert!(v["properties"]["nim"].is_object());
    assert!(v["$defs"]["NimConfig"].is_object());
}
