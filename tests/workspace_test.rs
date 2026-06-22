use wiki::core::workspace::discover_workspace;

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
fn discover_walks_up_to_find_dot_wiki() {
    let tmp = tempfile::tempdir().unwrap();
    let wiki_root = tmp.path();
    let nested = wiki_root.join("a/b/c");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::create_dir(wiki_root.join(".wiki")).unwrap();

    let result = discover_workspace(None, None, None, None, nested);
    assert_eq!(result.unwrap(), wiki_root.canonicalize().unwrap());
}

#[test]
fn discover_returns_error_when_nothing_found() {
    let tmp = tempfile::tempdir().unwrap();
    let result = discover_workspace(None, None, None, None, tmp.path().to_path_buf());
    assert!(result.is_err());
}
