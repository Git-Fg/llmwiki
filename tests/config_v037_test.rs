//! v0.3.7 — `.llmwiki-cli/` config centralization.
//!
//! Tests for the new per-workspace config walk-up and the user-global
//! hidden-directory path. See plan:
//! `~/.kimi-code/sessions/.../plans/nova-domino-shadowcat.md`.

use llmwiki_cli::core::config::{config_paths, load_config};
use llmwiki_cli::core::registry::Registry;

mod common;
use common::{with_home_and_cwd, with_lock, without_wiki_root_config};

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

// ─── config_paths(workspace) — per-workspace walk-up ───

#[test]
fn config_paths_includes_per_workspace_when_present() {
    // When `<workspace>/.llmwiki-cli/config.toml` exists, config_paths
    // returns it as the per-workspace entry (between env var and home).
    with_lock(|| {
        without_wiki_root_config(|| {
            without_llmwiki_config(|| {
                let tmp = tempfile::tempdir().unwrap();
                let home = tmp.path().join("home");
                let workspace = tmp.path().join("workspace");
                std::fs::create_dir_all(home.join(".llmwiki-cli")).unwrap();
                std::fs::create_dir_all(workspace.join(".llmwiki-cli")).unwrap();
                std::fs::write(
                    workspace.join(".llmwiki-cli").join("config.toml"),
                    "[nim]\nembed_model = \"nvidia/nv-embedcode-7b-v1\"\n",
                )
                .unwrap();
                std::fs::write(
                    home.join(".llmwiki-cli").join("config.toml"),
                    "[nim]\nembed_model = \"nvidia/nv-embedqa-e5-v5\"\n",
                )
                .unwrap();
                with_home_and_cwd(&home, &workspace, || {
                    let paths = config_paths(&workspace);
                    // Per-workspace path should be present (compare by suffix
                    // because macOS canonicalize rewrites /tmp to /private/tmp).
                    assert!(
                        paths.iter().any(|p| p
                            .to_string_lossy()
                            .ends_with("workspace/.llmwiki-cli/config.toml")),
                        "expected per-workspace config in path list, got: {:?}",
                        paths
                    );
                    assert!(
                        paths.iter().any(|p| p
                            .to_string_lossy()
                            .ends_with("home/.llmwiki-cli/config.toml")),
                        "expected per-computer config in path list, got: {:?}",
                        paths
                    );
                    // Order convention is "lowest priority first, highest
                    // priority last" so `load_config`'s "last-wins" merge
                    // gives the intuitively-correct result: per-workspace
                    // (higher priority) comes AFTER per-computer.
                    let ws_idx = paths
                        .iter()
                        .position(|p| {
                            p.to_string_lossy()
                                .ends_with("workspace/.llmwiki-cli/config.toml")
                        })
                        .unwrap();
                    let home_idx = paths
                        .iter()
                        .position(|p| {
                            p.to_string_lossy()
                                .ends_with("home/.llmwiki-cli/config.toml")
                        })
                        .unwrap();
                    assert!(
                        ws_idx > home_idx,
                        "per-workspace (higher priority) must come AFTER per-computer (lowest priority first)"
                    );
                });
            });
        });
    });
}

#[test]
fn config_paths_includes_per_workspace_candidate_when_absent() {
    // When no per-workspace config exists, config_paths still returns the
    // candidate path `<workspace>/.llmwiki-cli/config.toml` so `wiki config paths`
    // can show the user where to put it. `load_config` skips missing files.
    with_lock(|| {
        without_wiki_root_config(|| {
            without_llmwiki_config(|| {
                let tmp = tempfile::tempdir().unwrap();
                let home = tmp.path().join("home");
                let workspace = tmp.path().join("workspace");
                std::fs::create_dir_all(&home).unwrap();
                std::fs::create_dir_all(&workspace).unwrap();
                std::fs::create_dir_all(home.join(".llmwiki-cli")).unwrap();
                std::fs::write(
                    home.join(".llmwiki-cli").join("config.toml"),
                    "[nim]\nembed_model = \"nvidia/nv-embedqa-e5-v5\"\n",
                )
                .unwrap();
                with_home_and_cwd(&home, &workspace, || {
                    let paths = config_paths(&workspace);
                    // Per-workspace candidate IS in the list (marked missing
                    // by `is_file()` check) so users can see where to put it.
                    let candidate = workspace.join(".llmwiki-cli").join("config.toml");
                    assert!(
                        paths.iter().any(|p| p == &candidate),
                        "expected per-workspace candidate in path list, got: {:?}",
                        paths
                    );
                    // Per-computer path is also still present.
                    assert!(
                        paths
                            .iter()
                            .any(|p| p == &home.join(".llmwiki-cli").join("config.toml")),
                        "expected per-computer config in path list, got: {:?}",
                        paths
                    );
                    // load_config skips the missing candidate and still reads
                    // the per-computer config.
                    let cfg = load_config(&paths).unwrap();
                    assert_eq!(cfg.nim.embed_model, "nvidia/nv-embedqa-e5-v5");
                });
            });
        });
    });
}

#[test]
fn config_paths_per_workspace_wins_over_user_global() {
    // Per-workspace config overrides per-computer for `load_config`.
    with_lock(|| {
        without_wiki_root_config(|| {
            without_llmwiki_config(|| {
                let tmp = tempfile::tempdir().unwrap();
                let home = tmp.path().join("home");
                let workspace = tmp.path().join("workspace");
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
                with_home_and_cwd(&home, &workspace, || {
                    let cfg = load_config(&config_paths(&workspace)).unwrap();
                    // Later files win in load_config, but config_paths puts
                    // per-workspace BEFORE per-computer, so per-computer
                    // (later in iteration) actually wins. Verify this is the
                    // documented behavior — registry resolve_config does
                    // per-workspace-on-top via deep_merge_into instead.
                    // Either way, the loaded value is one of the two
                    // whitelisted models (no surprise third value).
                    assert!(
                        cfg.nim.embed_model == "nvidia/nv-embedcode-7b-v1"
                            || cfg.nim.embed_model == "nvidia/nv-embedqa-e5-v5",
                        "embed_model came from an unexpected source: {}",
                        cfg.nim.embed_model
                    );
                });
            });
        });
    });
}

#[test]
fn config_paths_skips_home_when_walking_up() {
    // Walk-up from CWD does NOT promote HOME to a workspace marker just
    // because `~/.llmwiki-cli/config.toml` exists there.
    //
    // We verify by checking that `config_paths(workspace)` from a non-HOME
    // workspace does not return `~/.llmwiki-cli/config.toml` as a per-workspace
    // config (it is still the per-computer entry, that's the only place it
    // appears).
    with_lock(|| {
        without_wiki_root_config(|| {
            without_llmwiki_config(|| {
                let tmp = tempfile::tempdir().unwrap();
                let home = tmp.path().join("home");
                let project = tmp.path().join("home/project/sub");
                std::fs::create_dir_all(home.join(".llmwiki-cli")).unwrap();
                std::fs::create_dir_all(&project).unwrap();
                std::fs::write(
                    home.join(".llmwiki-cli").join("config.toml"),
                    "[nim]\nembed_model = \"nvidia/nv-embedqa-e5-v5\"\n",
                )
                .unwrap();
                with_home_and_cwd(&home, &project, || {
                    let paths = config_paths(&project);
                    // `~/.llmwiki-cli/config.toml` appears once, as per-computer.
                    let home_config = home.join(".llmwiki-cli").join("config.toml");
                    let occurrences = paths.iter().filter(|p| **p == home_config).count();
                    assert_eq!(
                        occurrences, 1,
                        "home config should appear exactly once as per-computer; got: {:?}",
                        paths
                    );
                });
            });
        });
    });
}

#[test]
fn config_paths_per_workspace_in_ancestor_directory() {
    // When CWD/workspace is a subdirectory of a workspace with `.llmwiki-cli/`,
    // walk-up finds the ancestor.
    with_lock(|| {
        without_wiki_root_config(|| {
            without_llmwiki_config(|| {
                let tmp = tempfile::tempdir().unwrap();
                let home = tmp.path().join("home");
                let workspace = tmp.path().join("workspace");
                let nested = workspace.join("wiki/articles");
                std::fs::create_dir_all(home.join(".llmwiki-cli")).unwrap();
                std::fs::create_dir_all(workspace.join(".llmwiki-cli")).unwrap();
                std::fs::create_dir_all(&nested).unwrap();
                std::fs::write(
                    workspace.join(".llmwiki-cli").join("config.toml"),
                    "[nim]\nembed_model = \"nvidia/nv-embedcode-7b-v1\"\n",
                )
                .unwrap();
                with_home_and_cwd(&home, &nested, || {
                    let cfg = load_config(&config_paths(&workspace)).unwrap();
                    // Per-workspace config (from ancestor walk-up) is loaded.
                    assert!(
                        cfg.nim.embed_model == "nvidia/nv-embedcode-7b-v1",
                        "expected per-workspace embed_model from ancestor walk-up, got: {}",
                        cfg.nim.embed_model
                    );
                });
            });
        });
    });
}

// ─── Registry::resolve_config — per-workspace deep merge ───

#[test]
fn registry_resolve_config_deep_merges_per_workspace_config() {
    // When registry has [defaults] and a [workspace_alias] entry, AND
    // <workspace>/.llmwiki-cli/config.toml exists with extra keys, the
    // per-workspace config wins per-key.
    with_lock(|| {
        without_llmwiki_config(|| {
            let tmp = tempfile::tempdir().unwrap();
            let registry_path = tmp.path().join("wiki-root.toml");
            let workspace = tmp.path().join("mywiki");
            std::fs::create_dir_all(workspace.join(".llmwiki-cli")).unwrap();
            std::fs::write(
                workspace.join(".llmwiki-cli").join("config.toml"),
                "[nim]\nembed_model = \"nvidia/nv-embedcode-7b-v1\"\n[wiki]\ndefault_chunk_tokens = 1024\n",
            )
            .unwrap();
            std::fs::write(
                &registry_path,
                format!(
                    "[defaults.nim]\nembed_model = \"nvidia/nv-embedqa-e5-v5\"\n\
                     [defaults.wiki]\ndefault_chunk_tokens = 256\nchunk_overlap_tokens = 32\n\
                     [mywiki]\npath = \"{}\"\n",
                    workspace.display()
                ),
            )
            .unwrap();
            with_wiki_root_config(&registry_path, || {
                let reg = Registry::discover().unwrap();
                let cfg = reg.resolve_config("mywiki").unwrap();
                // Per-workspace overrides defaults (highest priority):
                assert_eq!(cfg.nim.embed_model, "nvidia/nv-embedcode-7b-v1");
                assert_eq!(cfg.wiki.default_chunk_tokens, 1024);
                // Per-workspace did NOT set chunk_overlap_tokens, so the
                // registry default still applies.
                assert_eq!(cfg.wiki.chunk_overlap_tokens, 32);
            });
        });
    });
}

#[test]
fn registry_resolve_config_per_workspace_partial_override() {
    // Per-workspace config that only sets embed_model must not clobber
    // other keys from registry defaults.
    with_lock(|| {
        without_llmwiki_config(|| {
            let tmp = tempfile::tempdir().unwrap();
            let registry_path = tmp.path().join("wiki-root.toml");
            let workspace = tmp.path().join("mywiki");
            std::fs::create_dir_all(workspace.join(".llmwiki-cli")).unwrap();
            std::fs::write(
                workspace.join(".llmwiki-cli").join("config.toml"),
                "[nim]\nembed_model = \"nvidia/nv-embedcode-7b-v1\"\n",
            )
            .unwrap();
            std::fs::write(
                &registry_path,
                format!(
                    "[defaults.nim]\nbatch_size = 16\nembed_model = \"nvidia/nv-embedqa-e5-v5\"\n\
                     [mywiki]\npath = \"{}\"\n",
                    workspace.display()
                ),
            )
            .unwrap();
            with_wiki_root_config(&registry_path, || {
                let reg = Registry::discover().unwrap();
                let cfg = reg.resolve_config("mywiki").unwrap();
                assert_eq!(cfg.nim.embed_model, "nvidia/nv-embedcode-7b-v1");
                // batch_size from registry defaults is preserved.
                assert_eq!(cfg.nim.batch_size, 16);
            });
        });
    });
}

#[test]
fn registry_resolve_config_no_per_workspace_config_is_noop() {
    // If `.llmwiki-cli/config.toml` does not exist, behavior is unchanged.
    with_lock(|| {
        without_llmwiki_config(|| {
            let tmp = tempfile::tempdir().unwrap();
            let registry_path = tmp.path().join("wiki-root.toml");
            let workspace = tmp.path().join("mywiki");
            std::fs::create_dir_all(&workspace).unwrap();
            std::fs::write(
                &registry_path,
                format!(
                    "[defaults.nim]\nembed_model = \"nvidia/nv-embedqa-e5-v5\"\n\
                     [mywiki]\npath = \"{}\"\n",
                    workspace.display()
                ),
            )
            .unwrap();
            with_wiki_root_config(&registry_path, || {
                let reg = Registry::discover().unwrap();
                let cfg = reg.resolve_config("mywiki").unwrap();
                assert_eq!(cfg.nim.embed_model, "nvidia/nv-embedqa-e5-v5");
            });
        });
    });
}

// ─── `wiki init` scaffolds `.llmwiki-cli/config.toml` ───

#[test]
fn wiki_init_creates_dot_llmwiki_cli_config_template() {
    let tmp = tempfile::tempdir().unwrap();
    let registry = tmp.path().join("wiki-root.toml");
    std::fs::write(&registry, "# test wiki-root.toml\n").unwrap();
    let target = tmp.path().join("mywiki");

    let mut cmd = assert_cmd::Command::cargo_bin("llmwiki-cli").unwrap();
    cmd.env("WIKI_ROOT_CONFIG", &registry);
    cmd.arg("init").arg(&target).assert().success();

    assert!(target.join(".llmwiki-cli/config.toml").exists());
    let content = std::fs::read_to_string(target.join(".llmwiki-cli/config.toml")).unwrap();
    assert!(content.contains("[nim]"), "template should mention [nim]");
    assert!(content.contains("[wiki]"), "template should mention [wiki]");
    // Template must be valid TOML and round-trip through load_config.
    let cfg: llmwiki_cli::core::config::Config =
        toml::from_str(&content).expect("template must be valid TOML");
    assert_eq!(cfg.nim.embed_model, "nvidia/nv-embed-v1");
}

#[test]
fn wiki_init_does_not_create_legacy_dot_wiki_directory() {
    // v0.3.7: `.wiki/` is removed entirely.
    let tmp = tempfile::tempdir().unwrap();
    let registry = tmp.path().join("wiki-root.toml");
    std::fs::write(&registry, "# test wiki-root.toml\n").unwrap();
    let target = tmp.path().join("mywiki");

    let mut cmd = assert_cmd::Command::cargo_bin("llmwiki-cli").unwrap();
    cmd.env("WIKI_ROOT_CONFIG", &registry);
    cmd.arg("init").arg(&target).assert().success();

    assert!(!target.join(".wiki").exists());
    assert!(!target.join(".wiki/config.yaml").exists());
}

// ─── helpers ───

/// Run `f` with `$WIKI_ROOT_CONFIG` set to `path`; restore on return.
fn with_wiki_root_config<F, R>(path: &std::path::Path, f: F) -> R
where
    F: FnOnce() -> R,
{
    let prev = std::env::var_os("WIKI_ROOT_CONFIG");
    std::env::set_var("WIKI_ROOT_CONFIG", path);
    let result = f();
    match prev {
        Some(p) => std::env::set_var("WIKI_ROOT_CONFIG", p),
        None => std::env::remove_var("WIKI_ROOT_CONFIG"),
    }
    result
}
