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
use predicates::prelude::PredicateBooleanExt;
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
fn subprocess_config_validate_warns_on_unknown_keys() {
    // A typo'd config.toml under the alias's workspace must surface warnings
    // on stderr. serde silently ignores unknown fields, so without this check
    // the user would believe their config is correct.
    let tmp = tempfile::tempdir().unwrap();
    let workspace = tmp.path().join("ws");
    std::fs::create_dir_all(workspace.join(".llmwiki-cli")).unwrap();
    std::fs::write(
        workspace.join(".llmwiki-cli").join("config.toml"),
        "[wiki]\npages_dir = \"\"\ntypo_key = true\n\
         [nim]\nembed_model = \"nvidia/nv-embed-v1\"\nbad_key = 1\n",
    )
    .unwrap();
    let reg_path = tmp.path().join("wiki-root.toml");
    std::fs::write(
        &reg_path,
        format!("[w]\npath = {:?}\n", workspace.display()),
    )
    .unwrap();

    let output = isolated_cmd(&reg_path)
        .args(["config", "validate"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "validate should still pass (warnings, not errors): stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unknown key in [wiki]: typo_key"),
        "expected [wiki] typo_key warning in stderr: {stderr}"
    );
    assert!(
        stderr.contains("unknown key in [nim]: bad_key"),
        "expected [nim] bad_key warning in stderr: {stderr}"
    );
}

#[test]
fn subprocess_config_validate_no_warnings_for_clean_config() {
    // A valid config.toml with only known keys must produce NO warnings, so
    // we never cry wolf on a correct file.
    let tmp = tempfile::tempdir().unwrap();
    let workspace = tmp.path().join("ws");
    std::fs::create_dir_all(workspace.join(".llmwiki-cli")).unwrap();
    std::fs::write(
        workspace.join(".llmwiki-cli").join("config.toml"),
        "[wiki]\npages_dir = \"\"\n\n[nim.retry]\nmax_attempts = 5\n",
    )
    .unwrap();
    let reg_path = tmp.path().join("wiki-root.toml");
    std::fs::write(
        &reg_path,
        format!("[w]\npath = {:?}\n", workspace.display()),
    )
    .unwrap();

    let output = isolated_cmd(&reg_path)
        .args(["config", "validate"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unknown key"),
        "clean config must not emit unknown-key warnings: {stderr}"
    );
}

#[test]
fn config_validate_after_edit_catches_typo() {
    // v0.3.28+: AI agents that edit a wiki config (whether the central
    // `wiki-root.toml` or a per-workspace `.llmwiki-cli/config.toml`) should
    // run `wiki config validate` after every change. This test simulates
    // that workflow: write a valid config, run validate (passes, no
    // warnings), introduce a typo, run validate again (still exits 0 but
    // emits the unknown-key warning on stderr).
    let tmp = tempfile::tempdir().unwrap();
    let workspace = tmp.path().join("ws");
    std::fs::create_dir_all(workspace.join(".llmwiki-cli")).unwrap();
    let reg_path = tmp.path().join("wiki-root.toml");
    std::fs::write(
        &reg_path,
        format!("[mevin]\npath = {:?}\n", workspace.display()),
    )
    .unwrap();

    // 1. Valid config — validate should succeed with no warnings.
    std::fs::write(
        workspace.join(".llmwiki-cli").join("config.toml"),
        "[wiki]\npages_dir = \"\"\n\n[nim]\nembed_model = \"nvidia/nv-embed-v1\"\n",
    )
    .unwrap();

    let output = isolated_cmd(&reg_path)
        .args(["config", "validate"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "validate should pass for clean config: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unknown key"),
        "clean config must not emit unknown-key warnings: {stderr}"
    );

    // 2. Introduce a typo (pages_dir → pages_dur) — validate should still
    // exit 0 but surface the unknown-key warning on stderr so the agent
    // catches its mistake before committing it.
    std::fs::write(
        workspace.join(".llmwiki-cli").join("config.toml"),
        "[wiki]\npages_dur = \"\"\n\n[nim]\nembed_model = \"nvidia/nv-embed-v1\"\n",
    )
    .unwrap();

    let output = isolated_cmd(&reg_path)
        .args(["config", "validate"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "validate should still succeed (warnings, not errors): stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unknown key in [wiki]: pages_dur"),
        "expected [wiki] typo warning in stderr: {stderr}"
    );
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

// ─── `wiki config show-effective` (git config --show-origin style) ───

fn isolated_cmd_with_workspace_and_config(
    reg_path: &std::path::Path,
    home: &std::path::Path,
    workspace: &std::path::Path,
    extra_config: Option<(&std::path::Path, &str)>,
) -> Command {
    let mut cmd = isolated_cmd(reg_path);
    cmd.env("HOME", home);
    cmd.env_remove("LLMWIKI_CONFIG");
    cmd.arg("--workspace").arg(workspace);
    if let Some((dir, content)) = extra_config {
        std::fs::create_dir_all(dir.join(".llmwiki-cli")).unwrap();
        std::fs::write(dir.join(".llmwiki-cli").join("config.toml"), content).unwrap();
    }
    cmd
}

#[test]
fn show_effective_json_lists_every_key_with_source() {
    let tmp = tempfile::tempdir().unwrap();
    let reg_path = tmp.path().join("wiki-root.toml");
    std::fs::write(&reg_path, "# test\n").unwrap();
    let home = tmp.path().join("home");
    let workspace = tmp.path().join("ws");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&workspace).unwrap();
    std::fs::create_dir_all(workspace.join(".llmwiki-cli")).unwrap();
    std::fs::write(
        workspace.join(".llmwiki-cli").join("config.toml"),
        "[nim]\nembed_model = \"nvidia/nv-embedqa-e5-v5\"\n",
    )
    .unwrap();

    let mut cmd = isolated_cmd_with_workspace_and_config(&reg_path, &home, &workspace, None);
    let output = cmd
        .args(["config", "show-effective", "--json"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "show-effective --json failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(v.get("workspace").is_some());
    let entries = v["entries"].as_array().unwrap();
    assert!(!entries.is_empty());
    // Every entry has key/value/source.
    for e in entries {
        assert!(e.get("key").is_some(), "missing key: {e}");
        assert!(e.get("value").is_some(), "missing value: {e}");
        assert!(e.get("source").is_some(), "missing source: {e}");
    }
    // Find the nim.embed_model key — its source must be the per-workspace file.
    let embed = entries
        .iter()
        .find(|e| e["key"] == "nim.embed_model")
        .expect("nim.embed_model entry");
    let src = embed["source"].as_str().unwrap();
    assert!(
        src.contains(".llmwiki-cli/config.toml"),
        "expected source to mention per-workspace .llmwiki-cli/config.toml, got: {src}"
    );
    assert!(
        !src.contains("home"),
        "source should be per-workspace, not per-computer (HOME), got: {src}"
    );
    assert_eq!(embed["value"], "nvidia/nv-embedqa-e5-v5");
}

#[test]
fn show_effective_text_includes_source_column() {
    let tmp = tempfile::tempdir().unwrap();
    let reg_path = tmp.path().join("wiki-root.toml");
    std::fs::write(&reg_path, "# test\n").unwrap();
    let home = tmp.path().join("home");
    let workspace = tmp.path().join("ws");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&workspace).unwrap();

    let mut cmd = isolated_cmd_with_workspace_and_config(&reg_path, &home, &workspace, None);
    let output = cmd.args(["config", "show-effective"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Effective config"),
        "missing header: {stdout}"
    );
    assert!(
        stdout.contains("nim.embed_model"),
        "missing nim.embed_model line: {stdout}"
    );
    assert!(
        stdout.contains("<default>"),
        "expected <default> for keys not in any file: {stdout}"
    );
}

#[test]
fn show_effective_per_workspace_overrides_per_computer() {
    // Per-workspace wins; its source should be the per-workspace file.
    let tmp = tempfile::tempdir().unwrap();
    let reg_path = tmp.path().join("wiki-root.toml");
    std::fs::write(&reg_path, "# test\n").unwrap();
    let home = tmp.path().join("home");
    let workspace = tmp.path().join("ws");
    std::fs::create_dir_all(home.join(".llmwiki-cli")).unwrap();
    std::fs::create_dir_all(workspace.join(".llmwiki-cli")).unwrap();
    std::fs::write(
        home.join(".llmwiki-cli").join("config.toml"),
        "[nim]\nembed_model = \"nvidia/nv-embedqa-e5-v5\"\n",
    )
    .unwrap();
    std::fs::write(
        workspace.join(".llmwiki-cli").join("config.toml"),
        "[nim]\nembed_model = \"nvidia/nv-embedcode-7b-v1\"\n",
    )
    .unwrap();

    let mut cmd = isolated_cmd_with_workspace_and_config(&reg_path, &home, &workspace, None);
    let output = cmd
        .args(["config", "show-effective", "--json"])
        .output()
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let embed = v["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["key"] == "nim.embed_model")
        .unwrap();
    assert_eq!(embed["value"], "nvidia/nv-embedcode-7b-v1");
    let src = embed["source"].as_str().unwrap();
    assert!(src.contains("ws/.llmwiki-cli/config.toml"));
}

// ─── `wiki config config-edit` ───

#[test]
fn config_edit_picks_per_workspace_when_present() {
    // Use a stub `EDITOR` that just records the path it was given and exits 0.
    let tmp = tempfile::tempdir().unwrap();
    let reg_path = tmp.path().join("wiki-root.toml");
    std::fs::write(&reg_path, "# test\n").unwrap();
    let home = tmp.path().join("home");
    let workspace = tmp.path().join("ws");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(workspace.join(".llmwiki-cli")).unwrap();
    let per_ws = workspace.join(".llmwiki-cli").join("config.toml");
    std::fs::write(&per_ws, "[nim]\n").unwrap();

    // Stub editor: writes the path it received to a file and exits 0.
    let stub = tmp.path().join("stub_editor.sh");
    let log = tmp.path().join("editor_invocation.log");
    std::fs::write(
        &stub,
        format!("#!/bin/sh\necho \"$1\" > {}\n", log.display()),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    let mut cmd = isolated_cmd_with_workspace_and_config(&reg_path, &home, &workspace, None);
    cmd.env("EDITOR", &stub);
    let output = cmd.args(["config", "config-edit"]).output().unwrap();
    assert!(output.status.success(), "config-edit failed: {output:?}");
    let invoked = std::fs::read_to_string(&log).unwrap_or_default();
    assert!(
        invoked.contains(".llmwiki-cli/config.toml"),
        "stub editor should have been invoked with the per-workspace config, got: {invoked:?}"
    );
}

#[test]
fn config_edit_falls_back_to_per_workspace_candidate_when_none_exist() {
    // No config file exists. config-edit should still succeed and open the
    // per-workspace candidate (so the user can create one).
    let tmp = tempfile::tempdir().unwrap();
    let reg_path = tmp.path().join("wiki-root.toml");
    std::fs::write(&reg_path, "# test\n").unwrap();
    let home = tmp.path().join("home");
    let workspace = tmp.path().join("ws");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&workspace).unwrap();

    let stub = tmp.path().join("stub_editor.sh");
    let log = tmp.path().join("editor_invocation.log");
    std::fs::write(
        &stub,
        format!("#!/bin/sh\necho \"$1\" > {}\n", log.display()),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    let mut cmd = isolated_cmd_with_workspace_and_config(&reg_path, &home, &workspace, None);
    cmd.env("EDITOR", &stub);
    let output = cmd.args(["config", "config-edit"]).output().unwrap();
    assert!(
        output.status.success(),
        "config-edit should fall back to per-workspace candidate, failed: {output:?}"
    );
    let invoked = std::fs::read_to_string(&log).unwrap_or_default();
    assert!(
        invoked.contains("ws/.llmwiki-cli/config.toml"),
        "stub editor should have been invoked with the per-workspace candidate, got: {invoked:?}"
    );
}

// ─── Global --workspace propagation (regression for v0.3.9) ───
//
// These tests guard against future refactors that silently drop the global
// `--workspace` flag on a config subcommand. The bug fixed in v0.3.9 was that
// `ConfigCmd::ConfigEdit` was a unit variant with no subcommand `--workspace`
// field; clap's global-flag auto-fill couldn't reach the command, so
// `cmd_config_edit` re-discovered from CWD and failed with "workspace not
// found". The fix added a `--workspace` field on `ConfigEdit` so clap's
// auto-fill applies.

/// Every workspace-aware config subcommand must accept the global
/// `--workspace <path>` flag and successfully resolve the workspace from it.
/// This test runs a no-op-ish invocation against each subcommand and asserts
/// the output references the workspace path (not CWD).
#[test]
fn global_workspace_flag_propagates_to_every_config_subcommand() {
    let tmp = tempfile::tempdir().unwrap();
    let reg_path = tmp.path().join("wiki-root.toml");
    std::fs::write(&reg_path, "# test\n").unwrap();
    let home = tmp.path().join("home");
    let workspace = tmp.path().join("ws");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(workspace.join(".llmwiki-cli")).unwrap();
    std::fs::write(
        workspace.join(".llmwiki-cli").join("config.toml"),
        "[nim]\nembed_model = \"nvidia/nv-embedcode-7b-v1\"\n",
    )
    .unwrap();

    // Each entry: (subcommand + args, must reference this substring)
    let cases: &[(&[&str], &str)] = &[
        (&["config", "paths"], "ws"),
        (&["config", "show-effective"], "ws"),
        // `config-edit` needs an editor stub so it doesn't open vim.
        // The editor stub always exits 0; we only check exit status here.
        // (See config_edit_picks_per_workspace_when_present for full
        //  invocation-path verification.)
    ];

    for (args, must_contain) in cases {
        let output = isolated_cmd_with_workspace_and_config(&reg_path, &home, &workspace, None)
            .args(*args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "subcommand {args:?} failed with global --workspace: stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains(must_contain),
            "subcommand {args:?} output should reference workspace {must_contain:?}, got: {stdout}"
        );
    }
}

/// CI lint (test form): every `ConfigCmd` variant must be classified as
/// either **workspace-aware** (output references the workspace path) or
/// **registry-only** (output succeeds even when `--workspace` points at a
/// non-wiki directory). If a future variant is added that's neither, this
/// test fails — which is the intent.
///
/// Coverage matrix (update here when adding a new ConfigCmd variant):
///   - Workspace-aware: Paths, ConfigEdit, ShowEffective (asserted above
///     and in `global_workspace_flag_propagates_to_*` tests).
///   - Registry-only: Path, List, Get, Set, Unset, Add, Rm, Edit,
///     Validate, ShowSchema (asserted in
///     `registry_only_config_subcommands_ignore_workspace_flag`).
///
/// This test runs every variant with `--workspace <non-wiki>` and asserts
/// the subprocess does NOT error with "workspace not found" (which would
/// mean a registry-only command accidentally tried to discover a workspace).
#[test]
fn every_config_subcommand_is_either_workspace_aware_or_registry_only() {
    let tmp = tempfile::tempdir().unwrap();
    let reg_path = tmp.path().join("wiki-root.toml");
    std::fs::write(&reg_path, "# test\n").unwrap();
    let home = tmp.path().join("home");
    let workspace = tmp.path().join("not-a-wiki");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&workspace).unwrap();

    // Every ConfigCmd variant with safe no-op-or-print args. The EDITOR env
    // is set to `true` (a no-op binary on macOS/Linux that exits 0) so
    // config-edit doesn't open vim.
    let cases: &[&[&str]] = &[
        &["config", "path"],
        &["config", "paths"],
        &["config", "list"],
        &["config", "get", "nim.embed_model"],
        &[
            "config",
            "set",
            "nim.embed_model",
            "x",
            "--wiki",
            "testwiki",
        ],
        &["config", "unset", "nim.embed_model", "--wiki", "testwiki"],
        &["config", "add", "testwiki", "/tmp/testwiki"],
        &["config", "rm", "testwiki"],
        &["config", "edit"],
        &["config", "validate"],
        &["config", "show-schema"],
        &["config", "show-effective"],
    ];

    for args in cases {
        let mut cmd = isolated_cmd_with_workspace_and_config(&reg_path, &home, &workspace, None);
        cmd.env("EDITOR", "true");
        let output = cmd.args(*args).output().unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("workspace not found"),
            "subcommand {args:?} must NOT try to discover a workspace (it's either workspace-aware and should accept --workspace, or registry-only and should ignore it); stderr={stderr}"
        );
    }
}

/// `wiki config config-edit` specifically: with a stub editor and the global
/// `--workspace` flag set, the editor must be invoked with the per-workspace
/// config file path. This is the exact regression test for the v0.3.9 bug.
#[test]
fn global_workspace_flag_propagates_to_config_edit() {
    let tmp = tempfile::tempdir().unwrap();
    let reg_path = tmp.path().join("wiki-root.toml");
    std::fs::write(&reg_path, "# test\n").unwrap();
    let home = tmp.path().join("home");
    let workspace = tmp.path().join("ws");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(workspace.join(".llmwiki-cli")).unwrap();
    std::fs::write(
        workspace.join(".llmwiki-cli").join("config.toml"),
        "[nim]\nembed_model = \"nvidia/nv-embedcode-7b-v1\"\n",
    )
    .unwrap();

    let stub = tmp.path().join("stub_editor.sh");
    let log = tmp.path().join("editor_invocation.log");
    std::fs::write(
        &stub,
        format!("#!/bin/sh\necho \"$1\" > {}\n", log.display()),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    let output = isolated_cmd_with_workspace_and_config(&reg_path, &home, &workspace, None)
        .env("EDITOR", &stub)
        .args(["config", "config-edit"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "config-edit with global --workspace failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let invoked = std::fs::read_to_string(&log).unwrap_or_default();
    assert!(
        invoked.contains("ws/.llmwiki-cli/config.toml"),
        "editor should have received the per-workspace config (proving the global --workspace flag propagated); got: {invoked:?}"
    );
}

// ─── Registry-only config subcommands must ignore --workspace ───
//
// `wiki config add`, `rm`, `set`, `unset`, `list`, `get`, `path`, `edit`,
// `validate`, `show-schema` operate on the central registry (`wiki-root.toml`)
// which is intentionally workspace-independent. Asserting that they succeed
// even when a global `--workspace` points at a non-wiki directory guards
// against future drift where someone adds workspace-aware logic to a
// registry-only command (which would silently change the meaning of the
// command for users).

#[test]
fn registry_only_config_subcommands_ignore_workspace_flag() {
    let tmp = tempfile::tempdir().unwrap();
    let reg_path = tmp.path().join("wiki-root.toml");
    std::fs::write(&reg_path, "# empty registry\n").unwrap();
    let home = tmp.path().join("home");
    let workspace = tmp.path().join("not-a-wiki"); // intentionally not a wiki dir
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&workspace).unwrap();

    // Each entry: (subcommand + args, must NOT error with "workspace not found")
    let cases: &[&[&str]] = &[
        &["config", "list"],
        &["config", "path"],
        &["config", "get", "nim.embed_model"],
        &["config", "show-schema"],
    ];

    for args in cases {
        let output = isolated_cmd_with_workspace_and_config(&reg_path, &home, &workspace, None)
            .args(*args)
            .output()
            .unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "subcommand {args:?} should succeed even when --workspace points at a non-wiki dir; stderr={stderr}"
        );
        assert!(
            !stderr.contains("workspace not found"),
            "subcommand {args:?} must not look up workspace (registry-only); stderr={stderr}"
        );
    }
}

// ─── `wiki config show-effective` filters (v0.3.12) ───
//
// Two filters narrow the output:
//   - `[<prefix>]` positional: only keys starting with the prefix
//   - `--source <path>`: only keys whose source file matches the path
//
// Mirrors `git config --list --show-origin -- <pattern>` and
// `git config --file <path> --list`.

#[test]
fn show_effective_key_prefix_filter_narrows_output() {
    let tmp = tempfile::tempdir().unwrap();
    let reg_path = tmp.path().join("wiki-root.toml");
    std::fs::write(&reg_path, "# test\n").unwrap();
    let home = tmp.path().join("home");
    let workspace = tmp.path().join("ws");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(workspace.join(".llmwiki-cli")).unwrap();
    std::fs::write(
        workspace.join(".llmwiki-cli").join("config.toml"),
        "[nim]\nembed_model = \"nvidia/nv-embedcode-7b-v1\"\n",
    )
    .unwrap();

    let output = isolated_cmd_with_workspace_and_config(&reg_path, &home, &workspace, None)
        .args(["config", "show-effective", "nim."])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should include nim.* keys
    assert!(
        stdout.contains("nim.embed_model"),
        "missing nim.embed_model: {stdout}"
    );
    // Should NOT include wiki.* or other top-level keys
    assert!(
        !stdout.contains("wiki.default_chunk_tokens"),
        "filter should exclude non-nim keys, got: {stdout}"
    );
    assert!(
        stdout.contains("filtered: key=\"nim.\""),
        "missing filter note: {stdout}"
    );
}

#[test]
fn show_effective_source_filter_returns_only_keys_from_that_file() {
    let tmp = tempfile::tempdir().unwrap();
    let reg_path = tmp.path().join("wiki-root.toml");
    std::fs::write(&reg_path, "# test\n").unwrap();
    let home = tmp.path().join("home");
    let workspace = tmp.path().join("ws");
    std::fs::create_dir_all(home.join(".llmwiki-cli")).unwrap();
    std::fs::create_dir_all(workspace.join(".llmwiki-cli")).unwrap();
    // Per-workspace sets BOTH nim.embed_model and wiki.require_frontmatter
    let per_ws_path = workspace.join(".llmwiki-cli").join("config.toml");
    std::fs::write(
        &per_ws_path,
        "[nim]\nembed_model = \"nvidia/nv-embedcode-7b-v1\"\n[wiki]\nrequire_frontmatter = false\n",
    )
    .unwrap();
    // Per-computer sets nim.base_url (overridden by per-workspace? no, different key)
    std::fs::write(
        home.join(".llmwiki-cli").join("config.toml"),
        "[nim]\nbase_url = \"https://integrate.api.nvidia.com\"\n",
    )
    .unwrap();

    // Filter to per-workspace only
    let output = isolated_cmd_with_workspace_and_config(&reg_path, &home, &workspace, None)
        .args([
            "config",
            "show-effective",
            "--source",
            per_ws_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // nim.embed_model is set by per-workspace → should appear
    assert!(
        stdout.contains("nim.embed_model"),
        "expected nim.embed_model: {stdout}"
    );
    // wiki.require_frontmatter is set by per-workspace → should appear
    assert!(
        stdout.contains("wiki.require_frontmatter"),
        "expected wiki.require_frontmatter: {stdout}"
    );
    // nim.base_url is set by per-computer → should NOT appear (filtered out)
    assert!(
        !stdout.contains("nim.base_url"),
        "filter should exclude per-computer keys, got: {stdout}"
    );
}

#[test]
fn show_effective_key_and_source_combine() {
    let tmp = tempfile::tempdir().unwrap();
    let reg_path = tmp.path().join("wiki-root.toml");
    std::fs::write(&reg_path, "# test\n").unwrap();
    let home = tmp.path().join("home");
    let workspace = tmp.path().join("ws");
    std::fs::create_dir_all(home.join(".llmwiki-cli")).unwrap();
    std::fs::create_dir_all(workspace.join(".llmwiki-cli")).unwrap();
    let per_ws_path = workspace.join(".llmwiki-cli").join("config.toml");
    std::fs::write(
        &per_ws_path,
        "[nim]\nembed_model = \"x\"\n[wiki]\nrequire_frontmatter = false\n",
    )
    .unwrap();
    std::fs::write(
        home.join(".llmwiki-cli").join("config.toml"),
        "[nim]\nbase_url = \"y\"\n",
    )
    .unwrap();

    let output = isolated_cmd_with_workspace_and_config(&reg_path, &home, &workspace, None)
        .args([
            "config",
            "show-effective",
            "nim.",
            "--source",
            per_ws_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("nim.embed_model"));
    // wiki.* excluded by key filter
    assert!(!stdout.contains("wiki.require_frontmatter"));
    // nim.base_url excluded by source filter
    assert!(!stdout.contains("nim.base_url"));
}

#[test]
fn show_effective_overrides_only_hides_default_matching_keys() {
    // With no config files at all, every key matches its default → output
    // is empty (or near-empty: keys not in the default config like
    // `nim.embed_dim_override` are still shown). Adding a per-workspace
    // override of `nim.embed_model` must surface that key as the only
    // "override" in the output.
    let tmp = tempfile::tempdir().unwrap();
    let reg_path = tmp.path().join("wiki-root.toml");
    std::fs::write(&reg_path, "# test\n").unwrap();
    let home = tmp.path().join("home");
    let workspace = tmp.path().join("ws");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(workspace.join(".llmwiki-cli")).unwrap();
    std::fs::write(
        workspace.join(".llmwiki-cli").join("config.toml"),
        "[nim]\nembed_model = \"nvidia/nv-embedcode-7b-v1\"\n",
    )
    .unwrap();

    let output = isolated_cmd_with_workspace_and_config(&reg_path, &home, &workspace, None)
        .args(["config", "show-effective", "--overrides-only"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("nim.embed_model"),
        "nim.embed_model should appear (it's overridden): {stdout}"
    );
    // Default-matching keys like wiki.default_chunk_tokens=512 should NOT
    // appear in overrides-only output.
    assert!(
        !stdout.contains("wiki.default_chunk_tokens"),
        "default-matching keys should be hidden: {stdout}"
    );
    // nim.batch_size=8 is the default, should be hidden.
    assert!(
        !stdout.contains("nim.batch_size"),
        "default-matching keys should be hidden: {stdout}"
    );
}

#[test]
fn show_effective_overrides_only_surfaces_wiki_and_retry_overrides() {
    // Regression for the pre-v0.3.15 Config::merge() bug: it only handled
    // 3 nim.* fields and silently dropped all wiki.* and retry.* overrides
    // when set in per-computer / per-workspace config files. After the
    // switch to TOML-level deep merge, every field with `#[serde(default)]`
    // is handled uniformly.
    let tmp = tempfile::tempdir().unwrap();
    let reg_path = tmp.path().join("wiki-root.toml");
    std::fs::write(&reg_path, "# test\n").unwrap();
    let home = tmp.path().join("home");
    let workspace = tmp.path().join("ws");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(workspace.join(".llmwiki-cli")).unwrap();
    // Per-workspace sets fields that the old per-field merge would have
    // silently dropped:
    //   - wiki.default_chunk_tokens (wiki.* — old merge never touched this)
    //   - nim.retry.max_attempts    (nested struct — old merge never recursed)
    std::fs::write(
        workspace.join(".llmwiki-cli").join("config.toml"),
        "[wiki]\ndefault_chunk_tokens = 1024\n\
         [nim.retry]\nmax_attempts = 7\n",
    )
    .unwrap();

    let output = isolated_cmd_with_workspace_and_config(&reg_path, &home, &workspace, None)
        .args(["config", "show-effective", "--overrides-only"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("wiki.default_chunk_tokens"),
        "wiki.default_chunk_tokens should appear (overridden): {stdout}"
    );
    assert!(
        stdout.contains("nim.retry.max_attempts"),
        "nim.retry.max_attempts should appear (overridden): {stdout}"
    );
}

#[test]
fn show_schema_section_filters_output() {
    let tmp = tempfile::tempdir().unwrap();
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--workspace")
        .arg(tmp.path())
        .arg("config")
        .arg("show-schema")
        .arg("--section")
        .arg("wiki")
        .assert()
        .success()
        .stdout(predicates::str::contains("\"pages_dir\""))
        .stdout(predicates::str::contains("\"exclude_dirs\""))
        .stdout(predicates::str::contains("\"embed_model\"").not());
}
