//! Regression tests for v0.3.2 multi-source registry hardening.
//!
//! Each test isolates HOME and CWD via the shared helpers in `tests/common`
//! so existing user-global state (which may exist on developer machines)
//! cannot leak in. Tests in this file mutate global environment variables
//! (`HOME`, `WIKI_ROOT_CONFIG`, current directory) and are serialized via
//! the process-wide `common::TEST_LOCK` mutex shared with
//! `registry_discovery_test.rs`.

mod common;

use common::{with_home_and_cwd, with_lock, without_wiki_root_config, write_registry};
use llmwiki_cli::core::registry::Registry;
use std::fs;

// ─── T2: H1 regression — alias sub-keys preserved when only top-level overridden ───

#[test]
fn alias_subkeys_preserved_when_only_top_level_overridden() {
    // The bug H1: when user-global and project-local both define `[shared]`
    // and the lower file has nested sections like `[shared.nim]`, the lower
    // file's nested section was silently dropped on merge.
    with_lock(|| {
        without_wiki_root_config(|| {
            let tmp = tempfile::tempdir().unwrap();
            let home = tmp.path();
            let project = tmp.path().join("p");
            std::fs::create_dir_all(&project).unwrap();

            write_registry(
                &home.join(".agents/wiki-root.toml"),
                r#"
[shared]
path = "/A"
description = "from global"

[shared.nim]
embed_model = "GLOBAL_MODEL"
api_key_env = "GLOBAL_KEY"
"#,
            );
            write_registry(
                &project.join(".agents/wiki-root.toml"),
                r#"
[shared]
path = "/B"
description = "from project"
"#,
            );

            with_home_and_cwd(home, &project, || {
                let reg = Registry::discover().unwrap();
                assert_eq!(reg.entries.len(), 1);
                let entry = &reg.entries[0];
                assert_eq!(entry.alias, "shared");
                assert_eq!(
                    entry.description, "from project",
                    "project-local description should win"
                );

                // H1 fix: [shared.nim] from the lower file should survive.
                let raw = entry.raw.as_table().expect("alias table");
                let nim = raw
                    .get("nim")
                    .and_then(|v| v.as_table())
                    .expect("H1: [shared.nim] sub-table should be preserved");
                assert_eq!(
                    nim.get("embed_model").and_then(|v| v.as_str()),
                    Some("GLOBAL_MODEL"),
                    "H1: lower file's [shared.nim].embed_model must survive project-local override"
                );
                assert_eq!(
                    nim.get("api_key_env").and_then(|v| v.as_str()),
                    Some("GLOBAL_KEY"),
                    "H1: lower file's [shared.nim].api_key_env must survive"
                );
            });
        });
    });
}

// ─── T1: direct load_all test (independent of HOME/CWD) ───

#[test]
fn load_all_merges_aliases_from_multiple_files() {
    with_lock(|| {
        let tmp = tempfile::tempdir().unwrap();
        let p1 = tmp.path().join("a.toml");
        let p2 = tmp.path().join("b.toml");
        write_registry(
            &p1,
            r#"
[alpha]
path = "/a"
description = "from a"
"#,
        );
        write_registry(
            &p2,
            r#"
[beta]
path = "/b"
description = "from b"
"#,
        );
        let reg = Registry::load_all(&[p1, p2]).unwrap();
        let aliases: Vec<&str> = reg.entries.iter().map(|e| e.alias.as_str()).collect();
        assert_eq!(aliases.len(), 2);
        assert!(aliases.contains(&"alpha"));
        assert!(aliases.contains(&"beta"));
    });
}

#[test]
fn load_all_with_overlapping_alias_uses_deep_merge() {
    with_lock(|| {
        let tmp = tempfile::tempdir().unwrap();
        let p1 = tmp.path().join("a.toml");
        let p2 = tmp.path().join("b.toml");
        write_registry(
            &p1,
            r#"
[shared]
path = "/A"
description = "from a"

[shared.nim]
embed_model = "MODEL_A"
"#,
        );
        write_registry(
            &p2,
            r#"
[shared]
path = "/B"
description = "from b"
"#,
        );
        let reg = Registry::load_all(&[p1, p2]).unwrap();
        assert_eq!(reg.entries.len(), 1);
        let entry = &reg.entries[0];
        assert_eq!(entry.path.to_string_lossy(), "/B");
        assert_eq!(entry.description, "from b");
        let raw = entry.raw.as_table().unwrap();
        let nim = raw.get("nim").and_then(|v| v.as_table()).unwrap();
        assert_eq!(
            nim.get("embed_model").and_then(|v| v.as_str()),
            Some("MODEL_A")
        );
    });
}

#[test]
fn load_all_returns_wiki_root_not_found_when_all_missing() {
    with_lock(|| {
        let tmp = tempfile::tempdir().unwrap();
        let paths = vec![tmp.path().join("nope1.toml"), tmp.path().join("nope2.toml")];
        let result = Registry::load_all(&paths);
        let err = result.unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("wiki-root.toml"));
        assert!(msg.contains("nope1.toml"));
        assert!(msg.contains("nope2.toml"));
    });
}

// ─── T3: user-global chain precedence by alias ───

#[test]
fn user_global_chain_agents_wins_over_claude() {
    with_lock(|| {
        without_wiki_root_config(|| {
            let tmp = tempfile::tempdir().unwrap();
            let home = tmp.path();
            let project = tmp.path().join("p");
            std::fs::create_dir_all(&project).unwrap();

            write_registry(
                &home.join(".claude/wiki-root.toml"),
                r#"
[shared]
path = "/claude-version"
description = "from claude"
"#,
            );
            write_registry(
                &home.join(".agents/wiki-root.toml"),
                r#"
[shared]
path = "/agents-version"
description = "from agents"
"#,
            );

            with_home_and_cwd(home, &project, || {
                let reg = Registry::discover().unwrap();
                assert_eq!(reg.entries.len(), 1);
                assert_eq!(reg.entries[0].description, "from agents");
            });
        });
    });
}

#[test]
fn user_global_chain_claude_wins_over_bare_home() {
    with_lock(|| {
        without_wiki_root_config(|| {
            let tmp = tempfile::tempdir().unwrap();
            let home = tmp.path();
            let project = tmp.path().join("p");
            std::fs::create_dir_all(&project).unwrap();

            write_registry(
                &home.join("wiki-root.toml"),
                r#"
[shared]
path = "/bare-version"
description = "from bare"
"#,
            );
            write_registry(
                &home.join(".claude/wiki-root.toml"),
                r#"
[shared]
path = "/claude-version"
description = "from claude"
"#,
            );

            with_home_and_cwd(home, &project, || {
                let reg = Registry::discover().unwrap();
                assert_eq!(reg.entries.len(), 1);
                assert_eq!(reg.entries[0].description, "from claude");
            });
        });
    });
}

// ─── T5: empty string + directory WIKI_ROOT_CONFIG error messages ───

#[test]
fn wiki_root_config_empty_string_reports_empty_in_error() {
    with_lock(|| {
        let prev = std::env::var_os("WIKI_ROOT_CONFIG");
        std::env::set_var("WIKI_ROOT_CONFIG", "");
        let result = Registry::discover();
        if let Some(p) = prev {
            std::env::set_var("WIKI_ROOT_CONFIG", p);
        } else {
            std::env::remove_var("WIKI_ROOT_CONFIG");
        }
        let err = result.unwrap_err();
        let msg = format!("{}", err);
        assert!(
            msg.contains("empty string"),
            "error must mention 'empty string'; got: {}",
            msg
        );
    });
}

#[test]
fn wiki_root_config_directory_reports_directory_in_error() {
    with_lock(|| {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("is-a-directory");
        std::fs::create_dir_all(&dir).unwrap();
        let prev = std::env::var_os("WIKI_ROOT_CONFIG");
        std::env::set_var("WIKI_ROOT_CONFIG", &dir);
        let result = Registry::discover();
        if let Some(p) = prev {
            std::env::set_var("WIKI_ROOT_CONFIG", p);
        } else {
            std::env::remove_var("WIKI_ROOT_CONFIG");
        }
        let err = result.unwrap_err();
        let msg = format!("{}", err);
        assert!(
            msg.contains("is a directory, not a file"),
            "error must distinguish directory case; got: {}",
            msg
        );
    });
}

// ─── M1 regression: dedupe when HOME is an ancestor of CWD ───

#[test]
fn candidate_paths_dedupes_when_cwd_walks_into_home_agents() {
    // Reproduce M1: if HOME/.agents/wiki-root.toml exists AND CWD is
    // somewhere INSIDE HOME, the walk-up visits HOME and adds it as a
    // project-local candidate, while the user-global chain also adds it.
    // Without dedup the same physical file would appear in `paths` twice.
    with_lock(|| {
        without_wiki_root_config(|| {
            let tmp = tempfile::tempdir().unwrap();
            let home = tmp.path();
            let project = home.join("p/sub");
            std::fs::create_dir_all(&project).unwrap();

            // Single file that lives at HOME/.agents/wiki-root.toml AND is
            // an ancestor of CWD (so walk-up would re-add it).
            write_registry(&home.join(".agents/wiki-root.toml"), "[g]\npath = \"/g\"\n");

            with_home_and_cwd(home, &project, || {
                let paths = Registry::candidate_paths();
                let strs: Vec<String> = paths
                    .iter()
                    .map(|p| p.to_string_lossy().to_string())
                    .collect();
                // The HOME .agents/wiki-root.toml path must appear exactly
                // once even though the walk-up would re-add it.
                let agents_count = strs
                    .iter()
                    .filter(|s| s.ends_with(".agents/wiki-root.toml"))
                    .count();
                assert_eq!(
                    agents_count, 1,
                    "HOME/.agents/wiki-root.toml should appear once after dedup; paths: {:?}",
                    strs
                );
                let total = strs.len();
                let unique: std::collections::HashSet<_> = strs.iter().collect();
                assert_eq!(
                    unique.len(),
                    total,
                    "all paths must be unique after dedup; paths: {:?}",
                    strs
                );
            });
        });
    });
}

// ─── T6: symlinked CWD walk-up ───

#[test]
fn walk_up_resolves_symlinked_cwd() {
    with_lock(|| {
        without_wiki_root_config(|| {
            let tmp = tempfile::tempdir().unwrap();
            let home = tmp.path();
            let real_project = tmp.path().join("real-project");
            std::fs::create_dir_all(&real_project).unwrap();
            write_registry(
                &real_project.join(".agents/wiki-root.toml"),
                "[r]\npath = \"/real\"\n",
            );

            // Create a symlink to the real project dir; cd into the symlink.
            let symlink_path = tmp.path().join("symlink-project");
            std::os::unix::fs::symlink(&real_project, &symlink_path).unwrap();

            // The symlink must not exist outside unix — skip on Windows.
            with_home_and_cwd(home, &symlink_path, || {
                let reg = Registry::discover().unwrap();
                let aliases: Vec<&str> = reg.entries.iter().map(|e| e.alias.as_str()).collect();
                assert!(
                    aliases.contains(&"r"),
                    "walk-up must find .agents/wiki-root.toml via symlinked CWD; got: {:?}",
                    aliases
                );
            });
        });
    });
}

// ─── T7: graceful fallback when HOME and USERPROFILE are both unset ───

#[test]
fn candidate_paths_with_no_home_falls_back_to_project_local_only() {
    with_lock(|| {
        without_wiki_root_config(|| {
            let tmp = tempfile::tempdir().unwrap();
            let project = tmp.path().join("p");
            std::fs::create_dir_all(&project).unwrap();
            write_registry(
                &project.join(".agents/wiki-root.toml"),
                "[p]\npath = \"/p\"\n",
            );

            let prev_home = std::env::var_os("HOME");
            let prev_userprofile = std::env::var_os("USERPROFILE");
            std::env::remove_var("HOME");
            std::env::remove_var("USERPROFILE");

            let result: Result<Registry, _> = std::env::set_current_dir(&project)
                .map_err(|e| format!("set_current_dir: {}", e))
                .and_then(|_| Registry::discover().map_err(|e| format!("discover: {}", e)));

            // (Restore cwd regardless.)
            let _ = std::env::set_current_dir(tmp.path());
            if let Some(h) = prev_home {
                std::env::set_var("HOME", h);
            }
            if let Some(u) = prev_userprofile {
                std::env::set_var("USERPROFILE", u);
            }

            let reg = result.expect("discover should succeed with only project-local");
            let aliases: Vec<&str> = reg.entries.iter().map(|e| e.alias.as_str()).collect();
            assert_eq!(aliases, vec!["p"]);
        });
    });
}

// ─── T8: reg.entries has no alias duplicates after merge ───

#[test]
fn merged_registry_has_unique_aliases() {
    with_lock(|| {
        without_wiki_root_config(|| {
            let tmp = tempfile::tempdir().unwrap();
            let home = tmp.path();
            let project = tmp.path().join("p");
            std::fs::create_dir_all(&project).unwrap();

            write_registry(&home.join(".agents/wiki-root.toml"), "[s]\npath = \"/a\"\n");
            write_registry(
                &project.join(".agents/wiki-root.toml"),
                "[s]\npath = \"/b\"\n",
            );

            with_home_and_cwd(home, &project, || {
                let reg = Registry::discover().unwrap();
                let mut aliases: Vec<&str> = reg.entries.iter().map(|e| e.alias.as_str()).collect();
                aliases.sort();
                let original_len = aliases.len();
                aliases.dedup();
                assert_eq!(
                    aliases.len(),
                    original_len,
                    "entries must have unique aliases"
                );
            });
        });
    });
}

// ─── M3 deferred: documented behavior — set_value silently creates override section ───
// This is acknowledged in CHANGELOG. The test below LOCKS IN the current behavior
// so future fixes can change it deliberately.

#[test]
fn set_value_creates_section_for_alias_from_lower_priority_file() {
    // Documented behavior: set_value writes to self.raw_doc (highest-priority
    // file). If the alias was loaded from a lower file, this effectively
    // creates a project-local override section. Future v0.3.3 may require
    // --create-override; this test pins the current behavior.
    with_lock(|| {
        without_wiki_root_config(|| {
            let tmp = tempfile::tempdir().unwrap();
            let home = tmp.path();
            let project = tmp.path().join("p");
            std::fs::create_dir_all(&project).unwrap();

            write_registry(
                &home.join(".agents/wiki-root.toml"),
                r#"
[shared]
path = "/A"
description = "from global"
"#,
            );
            write_registry(
                &project.join(".agents/wiki-root.toml"),
                r#"
[other]
path = "/O"
"#,
            );

            with_home_and_cwd(home, &project, || {
                let mut reg = Registry::discover().unwrap();
                // `shared` came from user-global, not in raw_doc (which is the
                // highest-priority file = project-local).
                assert!(reg.entries.iter().any(|e| e.alias == "shared"));
                // Setting a value under [shared] creates a new section in the
                // highest-priority file.
                reg.set_value("description", "from project", Some("shared"))
                    .unwrap();
                let raw_table = reg.raw_doc.as_table().unwrap();
                let shared = raw_table.get("shared").and_then(|v| v.as_table());
                assert!(
                    shared.is_some(),
                    "set_value should have created a [shared] section in raw_doc"
                );
                assert_eq!(
                    shared.unwrap().get("description").and_then(|v| v.as_str()),
                    Some("from project")
                );
            });
        });
    });
}

// ─── v0.3.3 H1: remove_entry / unset_value MUST error on lower-priority aliases ───

#[test]
fn remove_entry_errors_on_alias_from_lower_priority_file() {
    with_lock(|| {
        without_wiki_root_config(|| {
            let tmp = tempfile::tempdir().unwrap();
            let home = tmp.path().join("home");
            let cwd = tmp.path().join("proj");
            fs::create_dir_all(home.join(".agents")).unwrap();
            fs::create_dir_all(&cwd).unwrap();

            // Lower file (home) defines [shared]; higher file (project-local)
            // defines [other]. Merged registry has both visible.
            write_registry(
                &home.join(".agents").join("wiki-root.toml"),
                r#"
[shared]
path = "/home/wiki"
"#,
            );
            write_registry(
                &cwd.join(".agents").join("wiki-root.toml"),
                r#"
[other]
path = "/proj/wiki"
"#,
            );

            with_home_and_cwd(&home, &cwd, || {
                let reg = Registry::discover().expect("merged registry");
                assert!(reg.entries.iter().any(|e| e.alias == "shared"));
                assert!(reg.entries.iter().any(|e| e.alias == "other"));

                let mut reg = reg;
                // Removing [other] (defined in project-local = active scope) succeeds.
                reg.remove_entry("other").expect("remove from active scope");

                // Removing [shared] (loaded from home = lower-priority) MUST error.
                let err = reg.remove_entry("shared").unwrap_err();
                let msg = err.to_string();
                assert!(
                    msg.contains("lower-priority wiki-root.toml"),
                    "expected lower-priority message, got: {}",
                    msg
                );
                assert!(
                    msg.contains("WIKI_ROOT_CONFIG"),
                    "expected WIKI_ROOT_CONFIG hint, got: {}",
                    msg
                );
            });
        });
    });
}

#[test]
fn unset_value_errors_on_alias_from_lower_priority_file() {
    with_lock(|| {
        without_wiki_root_config(|| {
            let tmp = tempfile::tempdir().unwrap();
            let home = tmp.path().join("home");
            let cwd = tmp.path().join("proj");
            fs::create_dir_all(home.join(".agents")).unwrap();
            fs::create_dir_all(&cwd).unwrap();

            write_registry(
                &home.join(".agents").join("wiki-root.toml"),
                r#"
[shared]
path = "/home/wiki"
description = "from home"
"#,
            );
            write_registry(
                &cwd.join(".agents").join("wiki-root.toml"),
                r#"
[shared]
description = "override"
"#,
            );

            with_home_and_cwd(&home, &cwd, || {
                let mut reg = Registry::discover().expect("merged registry");
                // [shared] is in BOTH files; the active write target is
                // project-local (highest priority), so unset on a key in
                // the project-local scope succeeds.
                reg.unset_value("description", "shared")
                    .expect("unset from active scope");

                // Reload a fresh merged registry and try to unset a key that's
                // only in the lower file (path). The alias [shared] IS in
                // raw_doc (project-local has it via override), so this should
                // succeed at the scope check but error at the key-not-found
                // step. This documents that path-only unset is handled by the
                // inner "key not found" error, not by the scope error.
                let mut reg2 = Registry::discover().expect("reload");
                let err = reg2.unset_value("path", "shared").unwrap_err();
                let msg = err.to_string();
                assert!(
                    msg.contains("key 'path' not found in [shared]"),
                    "expected key-not-found error, got: {}",
                    msg
                );
            });
        });
    });
}

#[test]
fn remove_entry_works_when_alias_is_in_active_scope() {
    with_lock(|| {
        without_wiki_root_config(|| {
            let tmp = tempfile::tempdir().unwrap();
            let home = tmp.path().join("home");
            fs::create_dir_all(home.join(".agents")).unwrap();
            write_registry(
                &home.join(".agents").join("wiki-root.toml"),
                r#"
[shared]
path = "/home/wiki"
"#,
            );

            with_home_and_cwd(&home, &home, || {
                let mut reg = Registry::discover().expect("registry");
                reg.remove_entry("shared")
                    .expect("remove from active scope");
                assert!(reg.entries.iter().all(|e| e.alias != "shared"));
                assert!(reg
                    .raw_doc
                    .as_table()
                    .map(|t| !t.contains_key("shared"))
                    .unwrap_or(true));
            });
        });
    });
}

// ─── v0.3.4 H1: remove_entry save + discover roundtrip ───

#[test]
fn remove_entry_save_then_discover_alias_is_gone() {
    with_lock(|| {
        without_wiki_root_config(|| {
            let tmp = tempfile::tempdir().unwrap();
            let home = tmp.path().join("home");
            fs::create_dir_all(home.join(".agents")).unwrap();
            write_registry(
                &home.join(".agents").join("wiki-root.toml"),
                r#"
[shared]
path = "/home/wiki"
"#,
            );

            with_home_and_cwd(&home, &home, || {
                let mut reg = Registry::discover().expect("registry");
                reg.remove_entry("shared")
                    .expect("remove from active scope");
                reg.save().expect("save after remove");
                // Re-discover from disk
                let reg2 = Registry::discover().expect("reload");
                assert!(reg2.entries.iter().all(|e| e.alias != "shared"));
                assert!(reg2
                    .raw_doc
                    .as_table()
                    .map(|t| !t.contains_key("shared"))
                    .unwrap_or(true));
            });
        });
    });
}

#[test]
fn remove_entry_errors_on_lower_priority_save_then_discover_alias_persists() {
    with_lock(|| {
        without_wiki_root_config(|| {
            let tmp = tempfile::tempdir().unwrap();
            let home = tmp.path().join("home");
            let cwd = tmp.path().join("proj");
            fs::create_dir_all(home.join(".agents")).unwrap();
            fs::create_dir_all(&cwd).unwrap();

            // Lower file defines [shared]; higher defines [other]
            write_registry(
                &home.join(".agents").join("wiki-root.toml"),
                r#"
[shared]
path = "/home/wiki"
"#,
            );
            write_registry(
                &cwd.join(".agents").join("wiki-root.toml"),
                r#"
[other]
path = "/proj/wiki"
"#,
            );

            with_home_and_cwd(&home, &cwd, || {
                let mut reg = Registry::discover().expect("merged registry");
                assert!(reg.entries.iter().any(|e| e.alias == "shared"));
                assert!(reg.entries.iter().any(|e| e.alias == "other"));

                // Removing [shared] from lower-priority MUST error
                let err = reg.remove_entry("shared").unwrap_err();
                let msg = err.to_string();
                assert!(
                    msg.contains("lower-priority wiki-root.toml"),
                    "expected lower-priority message, got: {}",
                    msg
                );

                // save() was NOT called (error returned early), so re-discover
                // should still have the alias
                let reg2 = Registry::discover().expect("reload");
                assert!(reg2.entries.iter().any(|e| e.alias == "shared"));
            });
        });
    });
}

// ─── v0.3.4 H2: unset_value creates intermediate tables like set_value ───

#[test]
fn unset_value_creates_intermediate_tables_like_set_value() {
    with_lock(|| {
        without_wiki_root_config(|| {
            let tmp = tempfile::tempdir().unwrap();
            let home = tmp.path().join("home");
            fs::create_dir_all(home.join(".agents")).unwrap();
            write_registry(
                &home.join(".agents").join("wiki-root.toml"),
                r#"
[shared]
path = "/home/wiki"
"#,
            );

            with_home_and_cwd(&home, &home, || {
                let mut reg = Registry::discover().expect("registry");
                // set_value creates [shared.nim] intermediate table via dotted key
                reg.set_value("nim.embed_model", "nvidia/model", Some("shared"))
                    .expect("set_value creates intermediate tables");
                // unset_value should traverse the created tables and succeed
                reg.unset_value("nim.embed_model", "shared")
                    .expect("unset_value traverses created tables");
                // Verify it's gone
                let raw = reg.raw_doc.as_table().unwrap();
                let shared = raw.get("shared").and_then(|v| v.as_table()).unwrap();
                assert!(
                    shared.get("nim").and_then(|v| v.as_table()).is_none(),
                    "nim table should be removed when empty"
                );
            });
        });
    });
}

// ─── v0.3.3 M3: tags merge is array union-dedupe, not scalar override ───

#[test]
fn tags_array_union_dedupes_on_merge() {
    with_lock(|| {
        without_wiki_root_config(|| {
            let tmp = tempfile::tempdir().unwrap();
            let home = tmp.path().join("home");
            let cwd = tmp.path().join("proj");
            fs::create_dir_all(home.join(".agents")).unwrap();
            fs::create_dir_all(&cwd).unwrap();

            write_registry(
                &home.join(".agents").join("wiki-root.toml"),
                r#"
[shared]
path = "/home/wiki"
tags = ["rust", "linux"]
"#,
            );
            write_registry(
                &cwd.join(".agents").join("wiki-root.toml"),
                r#"
[shared]
path = "/home/wiki"
tags = ["rust", "wasm"]
"#,
            );

            with_home_and_cwd(&home, &cwd, || {
                let reg = Registry::discover().expect("merged registry");
                let shared = reg
                    .entries
                    .iter()
                    .find(|e| e.alias == "shared")
                    .expect("[shared] alias");
                // Union-dedupe: rust appears in both, linux only in lower,
                // wasm only in higher. Result is {rust, linux, wasm}.
                assert_eq!(shared.tags.len(), 3);
                assert!(shared.tags.contains(&"rust".to_string()));
                assert!(shared.tags.contains(&"linux".to_string()));
                assert!(shared.tags.contains(&"wasm".to_string()));
            });
        });
    });
}

// ─── v0.3.3 L2: add_entry's WikiEntry.raw is no longer empty ───

#[test]
fn add_entry_populates_entry_raw_table() {
    with_lock(|| {
        without_wiki_root_config(|| {
            let tmp = tempfile::tempdir().unwrap();
            let home = tmp.path().join("home");
            fs::create_dir_all(home.join(".agents")).unwrap();
            write_registry(&home.join(".agents").join("wiki-root.toml"), "# empty\n");

            with_home_and_cwd(&home, &home, || {
                let mut reg = Registry::discover().expect("registry");
                reg.add_entry(
                    "newwiki",
                    std::path::Path::new("/some/path"),
                    &["tag1".to_string()],
                    Some("a description"),
                )
                .expect("add_entry");
                let entry = reg
                    .entries
                    .iter()
                    .find(|e| e.alias == "newwiki")
                    .expect("newwiki entry");
                let table = entry.raw.as_table().expect("raw is table");
                assert_eq!(
                    table.get("path").and_then(|v| v.as_str()),
                    Some("/some/path")
                );
                assert_eq!(
                    table.get("description").and_then(|v| v.as_str()),
                    Some("a description")
                );
                assert!(table.contains_key("tags"));
            });
        });
    });
}
