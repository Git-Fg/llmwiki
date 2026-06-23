//! Tests for the multi-source registry loader.
//!
//! Each test isolates HOME and CWD via a tempdir so existing user-global
//! state (which may exist on developer machines) cannot leak in.
//!
//! Tests in this file mutate global environment variables (`HOME`,
//! `WIKI_ROOT_CONFIG`, current directory). They are serialized via the
//! process-wide `common::TEST_LOCK` mutex shared with
//! `registry_discovery_v032_test.rs`.

mod common;

use common::{
    with_home_and_cwd, with_lock, with_wiki_root_config, without_wiki_root_config, write_registry,
};
use llmwiki_cli::core::registry::Registry;

#[test]
fn loads_user_global_when_no_project_local() {
    with_lock(|| {
        without_wiki_root_config(|| {
            let tmp = tempfile::tempdir().unwrap();
            let home = tmp.path();
            write_registry(
                &home.join(".agents/wiki-root.toml"),
                r#"
[defaults]
chunk_size = 256

[global-wiki]
path = "/tmp/global-wiki"
description = "global"
"#,
            );
            let project = tmp.path().join("project");
            std::fs::create_dir_all(&project).unwrap();
            with_home_and_cwd(home, &project, || {
                let reg = Registry::discover().unwrap();
                let aliases: Vec<&str> = reg.entries.iter().map(|e| e.alias.as_str()).collect();
                assert_eq!(aliases, vec!["global-wiki"]);
            });
        });
    });
}

#[test]
fn concatenates_user_global_and_project_local() {
    with_lock(|| {
        without_wiki_root_config(|| {
            let tmp = tempfile::tempdir().unwrap();
            let home = tmp.path();
            let project = tmp.path().join("project");
            std::fs::create_dir_all(&project).unwrap();

            write_registry(
                &home.join(".agents/wiki-root.toml"),
                r#"
[global-wiki]
path = "/tmp/global-wiki"
description = "global"
"#,
            );
            write_registry(
                &project.join(".agents/wiki-root.toml"),
                r#"
[project-wiki]
path = "/tmp/project-wiki"
description = "project"
"#,
            );

            with_home_and_cwd(home, &project, || {
                let reg = Registry::discover().unwrap();
                let aliases: Vec<&str> = reg.entries.iter().map(|e| e.alias.as_str()).collect();
                assert!(aliases.contains(&"global-wiki"), "missing global alias");
                assert!(aliases.contains(&"project-wiki"), "missing project alias");
                assert_eq!(aliases.len(), 2, "expected 2 aliases, got: {aliases:?}");
            });
        });
    });
}

#[test]
fn project_local_overrides_user_global_on_alias_conflict() {
    with_lock(|| {
        without_wiki_root_config(|| {
            let tmp = tempfile::tempdir().unwrap();
            let home = tmp.path();
            let project = tmp.path().join("project");
            std::fs::create_dir_all(&project).unwrap();

            write_registry(
                &home.join(".agents/wiki-root.toml"),
                r#"
[shared]
path = "/tmp/global-wiki"
description = "from global"
"#,
            );
            write_registry(
                &project.join(".agents/wiki-root.toml"),
                r#"
[shared]
path = "/tmp/project-wiki"
description = "from project"
"#,
            );

            with_home_and_cwd(home, &project, || {
                let reg = Registry::discover().unwrap();
                assert_eq!(reg.entries.len(), 1);
                assert_eq!(reg.entries[0].alias, "shared");
                assert_eq!(
                    reg.entries[0].description, "from project",
                    "project-local should override user-global on alias conflict"
                );
            });
        });
    });
}

#[test]
fn ancestor_walks_up_to_find_project_registry() {
    with_lock(|| {
        without_wiki_root_config(|| {
            let tmp = tempfile::tempdir().unwrap();
            let home = tmp.path();
            let repo = tmp.path().join("projects/repo");
            let sub = repo.join("sub/deep");
            std::fs::create_dir_all(&sub).unwrap();
            write_registry(
                &repo.join(".agents/wiki-root.toml"),
                r#"
[repo-wiki]
path = "/tmp/repo-wiki"
description = "at repo root"
"#,
            );

            with_home_and_cwd(home, &sub, || {
                let reg = Registry::discover().unwrap();
                let aliases: Vec<&str> = reg.entries.iter().map(|e| e.alias.as_str()).collect();
                assert_eq!(aliases, vec!["repo-wiki"]);
            });
        });
    });
}

#[test]
fn closer_ancestor_wins_over_further_ancestor() {
    with_lock(|| {
        without_wiki_root_config(|| {
            let tmp = tempfile::tempdir().unwrap();
            let home = tmp.path();
            let outer = tmp.path().join("projects/repo");
            let inner = outer.join("sub");
            std::fs::create_dir_all(&inner).unwrap();

            write_registry(
                &outer.join(".agents/wiki-root.toml"),
                r#"
[w]
path = "/tmp/outer"
description = "outer"
"#,
            );
            write_registry(
                &inner.join(".agents/wiki-root.toml"),
                r#"
[w]
path = "/tmp/inner"
description = "inner"

[inner-only]
path = "/tmp/inner-only"
description = "inner-only"
"#,
            );

            with_home_and_cwd(home, &inner, || {
                let reg = Registry::discover().unwrap();
                assert_eq!(reg.entries.len(), 2);
                let w = reg.entries.iter().find(|e| e.alias == "w").unwrap();
                assert_eq!(w.description, "inner", "closer ancestor must win");
                assert!(reg.entries.iter().any(|e| e.alias == "inner-only"));
            });
        });
    });
}

#[test]
fn wiki_root_config_env_short_circuits_everything() {
    with_lock(|| {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("project");
        std::fs::create_dir_all(&project).unwrap();
        write_registry(
            &project.join(".agents/wiki-root.toml"),
            r#"
[ignored]
path = "/tmp/ignored"
description = "should not load"
"#,
        );
        let override_path = tmp.path().join("override.toml");
        write_registry(
            &override_path,
            r#"
[only]
path = "/tmp/only"
description = "from override"
"#,
        );

        let result = with_wiki_root_config(&override_path, Registry::discover);
        let reg = result.unwrap();
        let aliases: Vec<&str> = reg.entries.iter().map(|e| e.alias.as_str()).collect();
        assert_eq!(
            aliases,
            vec!["only"],
            "WIKI_ROOT_CONFIG must skip all other sources"
        );
        // root_path should be the override path (may differ in canonicalization
        // due to macOS /var → /private/var symlink; compare canonical forms).
        let expected = override_path
            .canonicalize()
            .unwrap_or(override_path.clone());
        let actual = reg
            .root_path
            .canonicalize()
            .unwrap_or(reg.root_path.clone());
        assert_eq!(actual, expected);
    });
}

#[test]
fn wiki_root_config_missing_file_reports_env_in_error() {
    with_lock(|| {
        let result = with_wiki_root_config(
            std::path::Path::new("/nonexistent/path/wiki-root.toml"),
            Registry::discover,
        );
        let err = result.unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("WIKI_ROOT_CONFIG"),
            "error should mention WIKI_ROOT_CONFIG; got: {msg}",
        );
    });
}

#[test]
fn no_registry_anywhere_errors() {
    with_lock(|| {
        without_wiki_root_config(|| {
            let tmp = tempfile::tempdir().unwrap();
            let project = tmp.path().join("project");
            std::fs::create_dir_all(&project).unwrap();
            with_home_and_cwd(tmp.path(), &project, || {
                let result = Registry::discover();
                assert!(result.is_err(), "expected WikiRootNotFound");
            });
        });
    });
}

#[test]
fn empty_aliases_in_user_global_still_loads() {
    with_lock(|| {
        without_wiki_root_config(|| {
            let tmp = tempfile::tempdir().unwrap();
            let home = tmp.path();
            let project = tmp.path().join("p");
            std::fs::create_dir_all(&project).unwrap();
            write_registry(&home.join(".agents/wiki-root.toml"), "# no aliases\n");
            write_registry(
                &project.join(".agents/wiki-root.toml"),
                "[p]\npath = \"/p\"\n",
            );

            with_home_and_cwd(home, &project, || {
                let reg = Registry::discover().unwrap();
                let aliases: Vec<&str> = reg.entries.iter().map(|e| e.alias.as_str()).collect();
                assert_eq!(aliases, vec!["p"]);
            });
        });
    });
}

#[test]
fn malformed_project_local_errors_without_falling_back() {
    with_lock(|| {
        without_wiki_root_config(|| {
            let tmp = tempfile::tempdir().unwrap();
            let home = tmp.path();
            let project = tmp.path().join("p");
            std::fs::create_dir_all(&project).unwrap();
            write_registry(
                &home.join(".agents/wiki-root.toml"),
                "[ok]\npath = \"/ok\"\n",
            );
            write_registry(
                &project.join(".agents/wiki-root.toml"),
                "this is not valid TOML = = =\n",
            );

            with_home_and_cwd(home, &project, || {
                let result = Registry::discover();
                assert!(result.is_err(), "malformed project-local must error");
            });
        });
    });
}

#[test]
fn candidate_paths_order_user_global_before_project_local() {
    with_lock(|| {
        without_wiki_root_config(|| {
            let tmp = tempfile::tempdir().unwrap();
            let home = tmp.path();
            let project = tmp.path().join("p");
            std::fs::create_dir_all(&project).unwrap();

            write_registry(&home.join("wiki-root.toml"), "[x]\npath = \"/x\"\n");
            write_registry(&home.join(".claude/wiki-root.toml"), "[y]\npath = \"/y\"\n");
            write_registry(&home.join(".agents/wiki-root.toml"), "[z]\npath = \"/z\"\n");
            write_registry(
                &project.join(".agents/wiki-root.toml"),
                "[w]\npath = \"/w\"\n",
            );

            with_home_and_cwd(home, &project, || {
                let paths = Registry::candidate_paths();
                let strs: Vec<String> = paths
                    .iter()
                    .map(|p| p.to_string_lossy().to_string())
                    .collect();
                assert!(
                    strs[0].ends_with("/wiki-root.toml")
                        && !strs[0].contains(".claude")
                        && !strs[0].contains(".agents"),
                    "first path should be bare ~/wiki-root.toml; got: {}",
                    strs[0]
                );
                assert!(strs[1].ends_with(".claude/wiki-root.toml"));
                assert!(strs[2].ends_with(".agents/wiki-root.toml"));
                let last = strs.last().unwrap();
                assert!(
                    last.contains("/p/.agents/wiki-root.toml"),
                    "last path should be project-local; got: {}",
                    last
                );
            });
        });
    });
}

#[test]
fn root_path_points_to_highest_priority_file_after_merge() {
    with_lock(|| {
        without_wiki_root_config(|| {
            let tmp = tempfile::tempdir().unwrap();
            let home = tmp.path();
            let project = tmp.path().join("p");
            std::fs::create_dir_all(&project).unwrap();

            write_registry(&home.join(".agents/wiki-root.toml"), "[a]\npath = \"/a\"\n");
            write_registry(
                &project.join(".agents/wiki-root.toml"),
                "[b]\npath = \"/b\"\n",
            );

            with_home_and_cwd(home, &project, || {
                let reg = Registry::discover().unwrap();
                let canonical = reg
                    .root_path
                    .canonicalize()
                    .unwrap_or(reg.root_path.clone());
                let expected = project
                    .join(".agents/wiki-root.toml")
                    .canonicalize()
                    .unwrap_or(project.join(".agents/wiki-root.toml"));
                assert_eq!(canonical, expected);
            });
        });
    });
}

#[test]
fn concat_makes_user_global_visible_to_project_cwd() {
    // Smoke test for the headline behavior: a wiki alias registered in the
    // user-global registry MUST be resolvable when running from a project cwd
    // that doesn't define that alias itself. This is what makes "all wikis
    // available to all future ai agents" true.
    with_lock(|| {
        without_wiki_root_config(|| {
            let tmp = tempfile::tempdir().unwrap();
            let home = tmp.path();
            let project = tmp.path().join("p");
            std::fs::create_dir_all(&project).unwrap();

            write_registry(
                &home.join(".agents/wiki-root.toml"),
                r#"
[shared-knowledge]
path = "/tmp/shared-knowledge"
description = "shared across all projects"
"#,
            );
            // No project-local registry at all.

            with_home_and_cwd(home, &project, || {
                let reg = Registry::discover().unwrap();
                assert!(
                    reg.entries.iter().any(|e| e.alias == "shared-knowledge"),
                    "user-global alias must be visible from any cwd"
                );
            });
        });
    });
}
