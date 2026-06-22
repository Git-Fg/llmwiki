use wiki::error::WikiError;

#[test]
fn error_display_does_not_leak_secrets() {
    let err = WikiError::NimApiKeyMissing;
    let s = format!("{}", err);
    assert!(!s.contains("NVIDIA_NIM_API_KEY="));
    assert!(s.contains("API key"));
}

#[test]
fn config_invalid_has_location() {
    let err = WikiError::ConfigInvalid {
        path: "/tmp/config.yaml".into(),
        line: 7,
        message: "bad value".into(),
    };
    let s = format!("{}", err);
    assert!(s.contains("/tmp/config.yaml"));
    assert!(s.contains("line 7"));
}

#[test]
fn wiki_root_not_found_lists_searched_paths() {
    let err = WikiError::WikiRootNotFound {
        searched: vec![
            std::path::PathBuf::from("/home/user/.agents/wiki-root.toml"),
            std::path::PathBuf::from("/home/user/.claude/wiki-root.toml"),
        ],
    };
    let msg = format!("{}", err);
    assert!(msg.contains("wiki-root.toml"));
    assert!(msg.contains("not found"));
}

#[test]
fn alias_not_found_lists_available() {
    let err = WikiError::AliasNotFound {
        alias: "missing".to_string(),
        available: "pharma, mevin".to_string(),
    };
    let msg = format!("{}", err);
    assert!(msg.contains("missing"));
    assert!(msg.contains("pharma"));
}
