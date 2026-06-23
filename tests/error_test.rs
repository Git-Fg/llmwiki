use llmwiki_cli::error::WikiError;

#[test]
fn error_display_does_not_leak_secrets() {
    let err = WikiError::NimApiKeyMissing;
    let s = format!("{err}");
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
    let s = format!("{err}");
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
        from_env: None,
    };
    let msg = format!("{err}");
    assert!(msg.contains("wiki-root.toml"));
    assert!(msg.contains("not found"));
    // When WIKI_ROOT_CONFIG is set, the message should surface it.
    // The registry now stores a pre-formatted suffix in `from_env`
    // describing what went wrong (missing / empty / is-a-directory / not-a-file).
    let err_env = WikiError::WikiRootNotFound {
        searched: vec![std::path::PathBuf::from("/nope/wiki-root.toml")],
        from_env: Some(" (WIKI_ROOT_CONFIG=/nope/wiki-root.toml did not exist)".to_string()),
    };
    let msg_env = format!("{err_env}");
    assert!(msg_env.contains("WIKI_ROOT_CONFIG=/nope/wiki-root.toml"));
    assert!(msg_env.contains("did not exist"));
}

#[test]
fn wiki_root_config_empty_string_distinguished_from_missing() {
    let err = WikiError::WikiRootNotFound {
        searched: vec![std::path::PathBuf::from("")],
        from_env: Some(
            " (WIKI_ROOT_CONFIG is set to an empty string; unset it or point it at a real file)"
                .to_string(),
        ),
    };
    let msg = format!("{err}");
    assert!(msg.contains("empty string"));
}

#[test]
fn wiki_root_config_directory_distinguished_from_missing() {
    let err = WikiError::WikiRootNotFound {
        searched: vec![std::path::PathBuf::from("/tmp")],
        from_env: Some(
            " (WIKI_ROOT_CONFIG=/tmp exists but is a directory, not a file)".to_string(),
        ),
    };
    let msg = format!("{err}");
    assert!(msg.contains("is a directory, not a file"));
}

#[test]
fn alias_not_found_lists_available() {
    let err = WikiError::AliasNotFound {
        alias: "missing".to_string(),
        available: "pharma, mevin".to_string(),
    };
    let msg = format!("{err}");
    assert!(msg.contains("missing"));
    assert!(msg.contains("pharma"));
}
