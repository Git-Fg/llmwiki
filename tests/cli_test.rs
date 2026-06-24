use assert_cmd::Command;
use std::io::Write;

#[test]
fn version_prints_current_cargo_version() {
    // Verify the binary's --version matches the version in Cargo.toml.
    // This catches accidental version drift between source and binary.
    let cargo_version = std::fs::read_to_string("Cargo.toml")
        .expect("Cargo.toml readable")
        .lines()
        .find(|l| l.starts_with("version = "))
        .expect("version line present")
        .split('"')
        .nth(1)
        .expect("version is a quoted string")
        .to_string();
    let expected = format!("llmwiki-cli {cargo_version}");
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicates::str::contains(&expected));
}

// ─── Bare-`llmwiki-cli` startup banner ────────────────────────────────
//
// Running the binary with no subcommand must print:
//  1. The version (so the user can verify which build they have).
//  2. The active wiki alias, workspace, and resolution source (or a
//     graceful "no active wiki" line when the resolver returns nothing).
//  3. A hint pointing at `llmwiki-cli --help` and `llmwiki-cli doctor`.
// This is the single most useful thing the user can run to orient
// themselves when they don't know which wiki the CLI is operating on.

/// Single-wiki registry → bare `llmwiki-cli` shows the version + the
/// active wiki + the resolution source.
#[test]
fn bare_llmwiki_cli_shows_active_wiki() {
    let dir = tempfile::tempdir().unwrap();
    let registry = dir.path().join("wiki-root.toml");
    let mut f = std::fs::File::create(&registry).unwrap();
    f.write_all(
        br#"
[solo]
path = "/tmp/solo-bare"
description = "Solo bare test"
"#,
    )
    .unwrap();

    // CWD must NOT match any registered path so we hit the single-wiki
    // shortcut (the most common "I just want to know what wiki this is"
    // scenario).
    let cwd = tempfile::tempdir().unwrap();
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .env("WIKI_ROOT_CONFIG", &registry)
        .env("HOME", dir.path())
        .env_remove("USERPROFILE")
        .current_dir(cwd.path())
        .assert()
        .success()
        // Version line
        .stdout(predicates::str::contains("llmwiki-cli "))
        // Active wiki line
        .stdout(predicates::str::contains("Active wiki:  solo ("))
        // Source attribution
        .stdout(predicates::str::contains("via:        "))
        // Hint to the full help / doctor
        .stdout(predicates::str::contains("`llmwiki-cli --help`"));
}

/// No registry at all → bare `llmwiki-cli` still prints the version and
/// a hint, never errors. Users on first-run hit this case.
#[test]
fn bare_llmwiki_cli_no_registry_degrades_gracefully() {
    // Use an empty HOME + an empty WIKI_ROOT_CONFIG (pointing at a
    // nonexistent file) so Registry::discover fails.
    let dir = tempfile::tempdir().unwrap();
    Command::cargo_bin("llmwiki-cli")
        .unwrap()
        .env("WIKI_ROOT_CONFIG", dir.path().join("nonexistent.toml"))
        .env("HOME", dir.path())
        .env_remove("USERPROFILE")
        .current_dir(dir.path())
        .assert()
        .success() // never errors
        .stdout(predicates::str::contains("llmwiki-cli "))
        .stdout(predicates::str::contains(
            "No wiki-root.toml registry found",
        ));
}
