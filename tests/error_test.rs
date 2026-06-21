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
