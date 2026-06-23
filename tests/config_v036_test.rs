//! v0.3.6 — Config discovery simplified to `~/llmwiki-cli/config.toml` with
//! `$LLMWIKI_CONFIG` override. Legacy `~/.config/wiki/config.yaml` removed.
//! YAML parsing removed — TOML only, matching `wiki-root.toml` format.

use llmwiki_cli::core::config::{config_paths, load_config};
use std::path::PathBuf;

mod common;
use common::{with_home_and_cwd, with_lock, without_wiki_root_config};

/// Run `f` with `$LLMWIKI_CONFIG` set to `path`; restore on return.
fn with_llmwiki_config<F, R>(path: &std::path::Path, f: F) -> R
where
    F: FnOnce() -> R,
{
    let prev = std::env::var_os("LLMWIKI_CONFIG");
    std::env::set_var("LLMWIKI_CONFIG", path);
    let result = f();
    match prev {
        Some(p) => std::env::set_var("LLMWIKI_CONFIG", p),
        None => std::env::remove_var("LLMWIKI_CONFIG"),
    }
    result
}

/// Run `f` with `$LLMWIKI_CONFIG` unset; restore on return.
fn without_llmwiki_config<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let prev = std::env::var_os("LLMWIKI_CONFIG");
    std::env::remove_var("LLMWIKI_CONFIG");
    let result = f();
    if let Some(p) = prev {
        std::env::set_var("LLMWIKI_CONFIG", p);
    }
    result
}

// ─── config_paths() helper ───

#[test]
fn config_paths_env_var_takes_priority() {
    with_lock(|| {
        without_llmwiki_config(|| {
            let tmp = tempfile::tempdir().unwrap();
            let custom = tmp.path().join("my-config.toml");
            with_llmwiki_config(&custom, || {
                let paths = config_paths();
                assert_eq!(paths[0], custom);
            });
        });
    });
}

#[test]
fn config_paths_falls_back_to_home_llmwiki_cli() {
    with_lock(|| {
        without_llmwiki_config(|| {
            let tmp = tempfile::tempdir().unwrap();
            let home = tmp.path().join("home");
            std::fs::create_dir_all(&home).unwrap();
            with_home_and_cwd(&home, &home, || {
                let paths = config_paths();
                assert_eq!(
                    paths.last().unwrap(),
                    &home.join("llmwiki-cli").join("config.toml")
                );
            });
        });
    });
}

#[test]
fn config_paths_ignores_empty_env_var() {
    with_lock(|| {
        without_llmwiki_config(|| {
            let tmp = tempfile::tempdir().unwrap();
            let home = tmp.path().join("home");
            std::fs::create_dir_all(&home).unwrap();
            with_home_and_cwd(&home, &home, || {
                let prev = std::env::var_os("LLMWIKI_CONFIG");
                std::env::set_var("LLMWIKI_CONFIG", "");
                let paths = config_paths();
                // Empty env var should NOT be added to the list.
                assert!(!paths.iter().any(|p| p.to_string_lossy().is_empty()));
                if let Some(p) = prev {
                    std::env::set_var("LLMWIKI_CONFIG", p);
                } else {
                    std::env::remove_var("LLMWIKI_CONFIG");
                }
            });
        });
    });
}

// ─── load_config() with TOML ───

#[test]
fn load_config_reads_toml_from_env_var() {
    with_lock(|| {
        without_wiki_root_config(|| {
            let tmp = tempfile::tempdir().unwrap();
            let custom = tmp.path().join("override.toml");
            std::fs::write(
                &custom,
                "[nim]\nembed_model = \"nvidia/nv-embedqa-e5-v5\"\n",
            )
            .unwrap();
            with_llmwiki_config(&custom, || {
                let cfg = load_config(&config_paths()).unwrap();
                assert_eq!(cfg.nim.embed_model, "nvidia/nv-embedqa-e5-v5");
            });
        });
    });
}

#[test]
fn load_config_reads_toml_from_home_llmwiki_cli() {
    with_lock(|| {
        without_wiki_root_config(|| {
            without_llmwiki_config(|| {
                let tmp = tempfile::tempdir().unwrap();
                let home = tmp.path().join("home");
                std::fs::create_dir_all(home.join("llmwiki-cli")).unwrap();
                std::fs::write(
                    home.join("llmwiki-cli").join("config.toml"),
                    "[nim]\nembed_model = \"nvidia/nv-embedcode-7b-v1\"\n",
                )
                .unwrap();
                with_home_and_cwd(&home, &home, || {
                    let cfg = load_config(&config_paths()).unwrap();
                    assert_eq!(cfg.nim.embed_model, "nvidia/nv-embedcode-7b-v1");
                });
            });
        });
    });
}

#[test]
fn load_config_returns_default_when_no_files_exist() {
    with_lock(|| {
        without_wiki_root_config(|| {
            without_llmwiki_config(|| {
                let tmp = tempfile::tempdir().unwrap();
                let home = tmp.path().join("home");
                std::fs::create_dir_all(&home).unwrap();
                with_home_and_cwd(&home, &home, || {
                    let cfg = load_config(&config_paths()).unwrap();
                    assert_eq!(cfg.nim.embed_model, "nvidia/nv-embed-v1");
                });
            });
        });
    });
}

#[test]
fn load_config_rejects_yaml_files() {
    // Legacy YAML support is removed. A `.yaml` file in the path list
    // should be ignored (not loaded) since the path doesn't exist as .toml.
    with_lock(|| {
        without_wiki_root_config(|| {
            without_llmwiki_config(|| {
                let tmp = tempfile::tempdir().unwrap();
                let yaml = tmp.path().join("legacy.yaml");
                std::fs::write(&yaml, "nim:\n  embed_model: \"nvidia/nv-embedqa-e5-v5\"\n")
                    .unwrap();
                // load_config only looks at files passed to it; the legacy
                // ~/.config/wiki path is no longer in config_paths().
                let paths: Vec<PathBuf> = vec![yaml];
                let cfg = load_config(&paths);
                // YAML file is not loaded (load_config expects TOML), so
                // parsing fails with a config invalid error.
                assert!(cfg.is_err());
            });
        });
    });
}

#[test]
fn config_paths_does_not_include_legacy_dot_config_wiki() {
    // Legacy `~/.config/wiki/config.yaml` is removed in v0.3.6.
    with_lock(|| {
        without_llmwiki_config(|| {
            let tmp = tempfile::tempdir().unwrap();
            let home = tmp.path().join("home");
            std::fs::create_dir_all(home.join(".config").join("wiki")).unwrap();
            std::fs::write(
                home.join(".config").join("wiki").join("config.yaml"),
                "nim:\n  embed_model: \"nvidia/invalid\"\n",
            )
            .unwrap();
            with_home_and_cwd(&home, &home, || {
                let paths = config_paths();
                assert!(
                    !paths
                        .iter()
                        .any(|p| p.to_string_lossy().contains(".config/wiki")),
                    "legacy ~/.config/wiki path should not appear in config_paths()"
                );
            });
        });
    });
}

#[test]
fn config_paths_does_not_include_workspace_local_yaml() {
    // Legacy `<workspace>/.wiki/config.yaml` is removed in v0.3.6.
    with_lock(|| {
        without_llmwiki_config(|| {
            let tmp = tempfile::tempdir().unwrap();
            let home = tmp.path().join("home");
            let workspace = tmp.path().join("workspace");
            std::fs::create_dir_all(&home).unwrap();
            std::fs::create_dir_all(workspace.join(".wiki")).unwrap();
            std::fs::write(
                workspace.join(".wiki").join("config.yaml"),
                "nim:\n  embed_model: \"nvidia/invalid\"\n",
            )
            .unwrap();
            with_home_and_cwd(&home, &workspace, || {
                let paths = config_paths();
                assert!(
                    !paths
                        .iter()
                        .any(|p| p.to_string_lossy().ends_with(".wiki/config.yaml")),
                    "workspace-local .wiki/config.yaml should not appear in config_paths()"
                );
            });
        });
    });
}
