use wiki::core::config::load_config;

#[test]
fn load_defaults_when_no_files_exist() {
    let cfg = load_config(&[]).unwrap();
    assert_eq!(cfg.nim.embed_model, "nvidia/nv-embed-v1");
    assert_eq!(cfg.nim.api_key_env, "NVIDIA_NIM_API_KEY");
}

#[test]
fn load_workspace_overrides_global() {
    let tmp = tempfile::tempdir().unwrap();
    let global = tmp.path().join("global.yaml");
    let workspace = tmp.path().join("workspace.yaml");
    std::fs::write(&global, "nim:\n  embed_model: \"nvidia/nv-embedqa-e5-v5\"\n").unwrap();
    std::fs::write(&workspace, "nim:\n  embed_model: \"nvidia/nv-embed-v1\"\n").unwrap();

    let cfg = load_config(&[global, workspace]).unwrap();
    assert_eq!(cfg.nim.embed_model, "nvidia/nv-embed-v1");
}

#[test]
fn load_unsupported_model_returns_error() {
    let tmp = tempfile::tempdir().unwrap();
    let bad = tmp.path().join("bad.yaml");
    std::fs::write(&bad, "nim:\n  embed_model: \"nvidia/invalid-model\"\n").unwrap();
    let err = load_config(&[bad]).unwrap_err();
    let s = format!("{}", err);
    assert!(s.contains("Unsupported") || s.contains("invalid") || s.contains("Unsupported embedding model"));
}

#[test]
fn load_invalid_yaml_returns_config_invalid() {
    let tmp = tempfile::tempdir().unwrap();
    let bad = tmp.path().join("bad.yaml");
    std::fs::write(&bad, "nim:\n  embed_model: \"nvidia/nv-embed-v1\"\n invalid: [unclosed").unwrap();
    let err = load_config(&[bad]).unwrap_err();
    let s = format!("{}", err);
    assert!(s.contains("config invalid") || s.contains("YAML"));
}
