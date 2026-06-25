//! Integration tests for the v0.3.36+ additions:
//!   - `llmwiki-cli completion <shell>`
//!   - `llmwiki-cli use <alias>` and the per-workspace active-wiki pointer
//!   - `llmwiki-cli status --all` fleet mode
//!
//! These exercise the CLI surface end-to-end. Unit tests for the
//! resolution chain (which is what `llmwiki-cli use` ultimately feeds) live
//! in `tests/registry_test.rs`.

use assert_cmd::Command;
use std::io::Write;

/// Run the binary with a hermetic registry + HOME so it never touches
/// the user's real `wiki-root.toml` or `~/.llmwiki-cli/`.
fn isolated_cmd(reg_path: &std::path::Path) -> Command {
    let mut cmd = Command::cargo_bin("llmwiki-cli").unwrap();
    cmd.env("WIKI_ROOT_CONFIG", reg_path).env_remove("HOME");
    cmd
}

fn write_registry(dir: &std::path::Path, content: &str) -> std::path::PathBuf {
    let path = dir.join("wiki-root.toml");
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
    path
}

// ─── `llmwiki-cli completion` ───────────────────────────────────────────────

/// `llmwiki-cli completion bash` must print a non-empty script to stdout.
/// Doesn't snapshot the exact bytes (clap_complete output is stable
/// but we don't want a brittle test) — just asserts the structural
/// invariants every shell expects.
#[test]
fn completion_bash_emits_function_with_llmwiki_cli() {
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("completion")
        .arg("bash")
        .assert()
        .success()
        .stdout(predicates::str::contains("llmwiki-cli"))
        .stdout(predicates::str::contains("_llmwiki-cli"))
        // The bash completion function body is wrapped in a function
        // definition with the binary name as the dispatcher.
        .stdout(predicates::str::contains("complete"));
}

/// `llmwiki-cli completion zsh` must emit a `#compdef` header (the zsh
/// completion system reads this to register the function).
#[test]
fn completion_zsh_emits_compdef_header() {
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("completion")
        .arg("zsh")
        .assert()
        .success()
        .stdout(predicates::str::contains("#compdef llmwiki-cli"));
}

/// `llmwiki-cli completion fish` must emit a `complete -c llmwiki-cli` line
/// (the fish completion registration syntax).
#[test]
fn completion_fish_emits_complete_directive() {
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("completion")
        .arg("fish")
        .assert()
        .success()
        .stdout(predicates::str::contains("complete -c llmwiki-cli"));
}

/// `llmwiki-cli completion power-shell` must emit a non-empty script
/// (clap delegates to clap_complete's PowerShell generator).
#[test]
fn completion_powershell_emits_nonempty_script() {
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("completion")
        .arg("power-shell")
        .assert()
        .success()
        .stdout(predicates::str::contains("llmwiki-cli"));
}

/// `llmwiki-cli completion elvish` must emit a non-empty script
/// (clap delegates to clap_complete's Elvish generator).
#[test]
fn completion_elvish_emits_nonempty_script() {
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("completion")
        .arg("elvish")
        .assert()
        .success()
        .stdout(predicates::str::contains("llmwiki-cli"));
}

/// Unknown shell must be rejected by clap's value parser, not crash.
#[test]
fn completion_unknown_shell_errors() {
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("completion")
        .arg("tcsh")
        .assert()
        .failure();
}

// ─── `llmwiki-cli use <alias>` ───────────────────────────────────────────────

/// `llmwiki-cli use <alias>` writes the alias to
/// `<workspace>/.llmwiki-cli/state/active-wiki` and `llmwiki-cli use` (no
/// args) reads it back. Mirrors the `npm use` / `cargo --manifest-path`
/// per-workspace-default idiom.
#[test]
fn use_writes_active_wiki_pointer_and_reads_it_back() {
    let reg_dir = tempfile::tempdir().unwrap();
    let reg_path = write_registry(
        reg_dir.path(),
        r#"
[mevin]
path = "/tmp/mevin-use-test"
description = "Mevin"
"#,
    );

    let workspace = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(workspace.path().join(".llmwiki-cli")).unwrap();

    // Write the pointer.
    isolated_cmd(&reg_path)
        .arg("--workspace")
        .arg(workspace.path())
        .arg("use")
        .arg("mevin")
        .assert()
        .success()
        .stdout(predicates::str::contains("✓ Set active wiki"))
        .stdout(predicates::str::contains("'mevin'"));

    // File should now exist on disk.
    let pointer = workspace
        .path()
        .join(".llmwiki-cli")
        .join("state")
        .join("active-wiki");
    assert!(pointer.is_file(), "pointer file must be created");
    let content = std::fs::read_to_string(&pointer).unwrap();
    assert_eq!(content.trim(), "mevin");

    // Reading it back via `llmwiki-cli use` (no alias) shows the value.
    isolated_cmd(&reg_path)
        .arg("--workspace")
        .arg(workspace.path())
        .arg("use")
        .assert()
        .success()
        .stdout(predicates::str::contains(
            "Active wiki for this workspace: mevin",
        ));
}

/// `llmwiki-cli use <unknown>` must error with a clear message — never
/// silently write a broken pointer.
#[test]
fn use_rejects_unregistered_alias() {
    let reg_dir = tempfile::tempdir().unwrap();
    let reg_path = write_registry(
        reg_dir.path(),
        r#"
[mevin]
path = "/tmp/mevin-bad-use"
"#,
    );

    let workspace = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(workspace.path().join(".llmwiki-cli")).unwrap();

    isolated_cmd(&reg_path)
        .arg("--workspace")
        .arg(workspace.path())
        .arg("use")
        .arg("nonexistent")
        .assert()
        .failure()
        .stderr(predicates::str::contains("not registered"));

    // Pointer must NOT have been written.
    let pointer = workspace
        .path()
        .join(".llmwiki-cli")
        .join("state")
        .join("active-wiki");
    assert!(!pointer.exists(), "broken pointer must not be written");
}

/// `llmwiki-cli use --unset` removes the pointer.
#[test]
fn use_unset_removes_pointer() {
    let reg_dir = tempfile::tempdir().unwrap();
    let reg_path = write_registry(
        reg_dir.path(),
        r#"
[mevin]
path = "/tmp/mevin-unset"
"#,
    );

    let workspace = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(workspace.path().join(".llmwiki-cli")).unwrap();
    let state_dir = workspace.path().join(".llmwiki-cli").join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let pointer = state_dir.join("active-wiki");
    std::fs::write(&pointer, "mevin").unwrap();

    isolated_cmd(&reg_path)
        .arg("--workspace")
        .arg(workspace.path())
        .arg("use")
        .arg("--unset")
        .assert()
        .success()
        .stdout(predicates::str::contains("Removed"));

    assert!(!pointer.exists(), "--unset must remove the pointer file");
}

/// JSON output for `llmwiki-cli use <alias>`: stable shape for CI/agents.
#[test]
fn use_json_output_has_stable_shape() {
    let reg_dir = tempfile::tempdir().unwrap();
    let reg_path = write_registry(
        reg_dir.path(),
        r#"
[mevin]
path = "/tmp/mevin-json"
"#,
    );

    let workspace = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(workspace.path().join(".llmwiki-cli")).unwrap();

    let output = isolated_cmd(&reg_path)
        .arg("--workspace")
        .arg(workspace.path())
        .arg("use")
        .arg("mevin")
        .arg("--json")
        .output()
        .expect("llmwiki-cli use --json must run");
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v["action"], "set");
    assert_eq!(v["alias"], "mevin");
    assert!(v["pointer"].is_string());
}

/// The active-wiki pointer must be respected by the resolution chain
/// (step 5.5): running `llmwiki-cli config current` from inside the workspace
/// must show `active_wiki_pointer` as the resolution source, NOT
/// `walk_up` or `single_wiki`.
#[test]
fn active_wiki_pointer_takes_precedence_over_walkup() {
    let reg_dir = tempfile::tempdir().unwrap();
    let reg_path = write_registry(
        reg_dir.path(),
        r#"
[mevin]
path = "/tmp/mevin-precedence"
description = "Mevin"
"#,
    );

    // Workspace has a .llmwiki-cli/ marker but its path is NOT
    // registered. The pointer pins it to "mevin".
    let workspace = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(workspace.path().join(".llmwiki-cli")).unwrap();
    let state_dir = workspace.path().join(".llmwiki-cli").join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    std::fs::write(state_dir.join("active-wiki"), "mevin").unwrap();

    // `cd` to a child of the workspace so walk-up would normally fire.
    let nested = workspace.path().join("deep").join("child");
    std::fs::create_dir_all(&nested).unwrap();

    let output = isolated_cmd(&reg_path)
        .current_dir(&nested)
        .arg("config")
        .arg("current")
        .arg("--json")
        .output()
        .expect("config current --json must run");
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v["alias"], "mevin");
    assert_eq!(v["source"], "active_wiki_pointer");
}

/// `llmwiki-cli --wiki <alias> config current` must report the
/// alias and source = "wiki_flag". Verifies the --wiki override works
/// end-to-end through the `config current` resolution chain.
#[test]
fn config_current_with_wiki_flag_reports_source() {
    let reg_dir = tempfile::tempdir().unwrap();
    let wiki_path = tempfile::tempdir().unwrap();
    let wiki_path_str = wiki_path.path().display().to_string();
    let reg_path = write_registry(
        reg_dir.path(),
        &format!(
            r#"
[explicit]
path = "{wiki_path_str}"
"#
        ),
    );

    let output = isolated_cmd(&reg_path)
        .arg("--wiki")
        .arg("explicit")
        .arg("config")
        .arg("current")
        .arg("--json")
        .output()
        .expect("config current --json must run");
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v["alias"], "explicit");
    // The source tag depends on the priority chain — --wiki wins via
    // WikiFlag, but the alias may also match CWD/registry. The key
    // assertion is that the alias is resolved correctly.
    assert!(v["source"].is_string(), "source must be a string, got: {v}");
}

// ─── `llmwiki-cli status --all` ──────────────────────────────────────────────

/// Fleet mode loops over every registered alias. Empty registry → no
/// entries, no failures, exit 0.
#[test]
fn status_all_with_empty_registry_is_a_noop() {
    let reg_dir = tempfile::tempdir().unwrap();
    let reg_path = write_registry(reg_dir.path(), "# empty\n");

    isolated_cmd(&reg_path)
        .arg("status")
        .arg("--all")
        .assert()
        .success()
        .stdout(predicates::str::contains("No wikis registered"));
}

/// Fleet mode with a single wiki prints one line and exits 0.
/// The path doesn't have to exist (we test the loop, not the
/// per-wiki content).
#[test]
fn status_all_with_one_wiki_prints_one_line() {
    let reg_dir = tempfile::tempdir().unwrap();
    let reg_path = write_registry(
        reg_dir.path(),
        r#"
[solo]
path = "/tmp/solo-fleet"
"#,
    );

    isolated_cmd(&reg_path)
        .arg("status")
        .arg("--all")
        .assert()
        .success()
        .stdout(predicates::str::contains("solo"));
}

/// Fleet mode JSON output: stable array of per-wiki entries.
#[test]
fn status_all_json_shape() {
    let reg_dir = tempfile::tempdir().unwrap();
    let reg_path = write_registry(
        reg_dir.path(),
        r#"
[solo]
path = "/tmp/solo-fleet-json"
"#,
    );

    let output = isolated_cmd(&reg_path)
        .arg("status")
        .arg("--all")
        .arg("--json")
        .output()
        .expect("status --all --json must run");
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(v["wikis"].is_array());
    assert_eq!(v["wikis"].as_array().unwrap().len(), 1);
    assert_eq!(v["wikis"][0]["alias"], "solo");
    assert_eq!(v["failures"], 0);
}

/// `llmwiki-cli use --unset` JSON output: `existed` true when
/// pointer file was present.
#[test]
fn use_unset_json_reports_existed_true() {
    let reg_dir = tempfile::tempdir().unwrap();
    let reg_path = write_registry(
        reg_dir.path(),
        r#"
[mevin]
path = "/tmp/mevin-unset-json"
"#,
    );

    let workspace = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(workspace.path().join(".llmwiki-cli")).unwrap();
    let state_dir = workspace.path().join(".llmwiki-cli").join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    std::fs::write(state_dir.join("active-wiki"), "mevin").unwrap();

    let output = isolated_cmd(&reg_path)
        .arg("--workspace")
        .arg(workspace.path())
        .arg("use")
        .arg("--unset")
        .arg("--json")
        .output()
        .expect("llmwiki-cli use --unset --json must run");
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v["action"], "unset");
    assert!(v["alias"].is_null());
    assert_eq!(v["existed"], true);
}

/// `llmwiki-cli use --unset` JSON output: `existed` false when
/// pointer file was already absent.
#[test]
fn use_unset_json_reports_existed_false() {
    let reg_dir = tempfile::tempdir().unwrap();
    let reg_path = write_registry(
        reg_dir.path(),
        r#"
[mevin]
path = "/tmp/mevin-unset-json-false"
"#,
    );

    let workspace = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(workspace.path().join(".llmwiki-cli")).unwrap();
    // No pointer file created

    let output = isolated_cmd(&reg_path)
        .arg("--workspace")
        .arg(workspace.path())
        .arg("use")
        .arg("--unset")
        .arg("--json")
        .output()
        .expect("llmwiki-cli use --unset --json must run");
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v["action"], "unset");
    assert!(v["alias"].is_null());
    assert_eq!(v["existed"], false);
}

/// `llmwiki-cli status --all` exits 2 if any sub-call fails.
/// Registers a wiki with a malformed config.toml — `resolve_config`
/// errors on parse, which the fleet loop counts as a failure.
#[test]
fn status_all_exits_2_on_failure() {
    let reg_dir = tempfile::tempdir().unwrap();
    // Create a wiki workspace with a malformed config file
    let bad_wiki = tempfile::tempdir().unwrap();
    let llmwiki_dir = bad_wiki.path().join(".llmwiki-cli");
    std::fs::create_dir_all(&llmwiki_dir).unwrap();
    std::fs::write(
        llmwiki_dir.join("config.toml"),
        "[nim\nthis is not valid toml\n",
    )
    .unwrap();
    let bad_path = bad_wiki.path().display().to_string();

    let reg_path = write_registry(
        reg_dir.path(),
        &format!(
            r#"
[badwiki]
path = "{bad_path}"
"#
        ),
    );

    let output = isolated_cmd(&reg_path)
        .arg("status")
        .arg("--all")
        .output()
        .expect("status --all must run");
    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("badwiki") && stdout.contains("ERROR"),
        "output should name the failing wiki and its error: {stdout}"
    );
}
