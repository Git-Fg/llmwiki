use llmwiki_cli::core::workspace::discover_workspace;

mod common;

#[test]
fn discover_from_flag_overrides_all() {
    let dir = tempfile::tempdir().unwrap();
    let result = discover_workspace(
        Some(dir.path().to_path_buf()),
        None,
        None,
        None,
        std::env::current_dir().unwrap(),
    );
    assert_eq!(result.unwrap(), dir.path().canonicalize().unwrap());
}

#[test]
fn discover_from_env_var() {
    let dir = tempfile::tempdir().unwrap();
    let result = discover_workspace(
        None,
        None,
        Some(dir.path().to_path_buf()),
        None,
        std::env::current_dir().unwrap(),
    );
    assert_eq!(result.unwrap(), dir.path().canonicalize().unwrap());
}

#[test]
fn discover_walks_up_to_find_dot_llmwiki_cli() {
    let tmp = common::isolated_tempdir();
    let wiki_root = tmp.path();
    let nested = wiki_root.join("a/b/c");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::create_dir(wiki_root.join(".llmwiki-cli")).unwrap();

    let result = discover_workspace(None, None, None, None, nested);
    assert_eq!(result.unwrap(), wiki_root.canonicalize().unwrap());
}

#[test]
fn discover_returns_error_when_nothing_found() {
    // The single-wiki shortcut in `discover_workspace` falls back to
    // `Registry::discover()` if no `.llmwiki-cli/` is found on the
    // walk-up. On a CI runner that has a global wiki-root.toml in its
    // `$HOME` or any ancestor directory, this shortcut kicks in and the
    // test would falsely pass with `Ok`. Isolate `$HOME` and CWD to a
    // fresh tempdir with no wiki artifacts so the error path is exercised.
    let tmp = common::isolated_tempdir();
    common::with_lock(|| {
        common::with_home_and_cwd(tmp.path(), tmp.path(), || {
            let result = discover_workspace(None, None, None, None, tmp.path().to_path_buf());
            assert!(
                result.is_err(),
                "expected error when nothing found, got: {result:?}"
            );
        });
    });
}
