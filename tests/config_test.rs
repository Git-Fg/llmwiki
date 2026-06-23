use llmwiki_cli::core::config::load_config;

#[test]
fn load_defaults_when_no_files_exist() {
    let cfg = load_config(&[]).unwrap();
    assert_eq!(cfg.nim.embed_model, "nvidia/nv-embed-v1");
    assert_eq!(cfg.nim.api_key_env, "NVIDIA_NIM_API_KEY");
}

#[test]
fn load_workspace_overrides_global() {
    let tmp = tempfile::tempdir().unwrap();
    let global = tmp.path().join("global.toml");
    let workspace = tmp.path().join("workspace.toml");
    std::fs::write(
        &global,
        "[nim]\nembed_model = \"nvidia/nv-embedqa-e5-v5\"\n",
    )
    .unwrap();
    std::fs::write(&workspace, "[nim]\nembed_model = \"nvidia/nv-embed-v1\"\n").unwrap();

    let cfg = load_config(&[global, workspace]).unwrap();
    assert_eq!(cfg.nim.embed_model, "nvidia/nv-embed-v1");
}

#[test]
fn load_unsupported_model_returns_error() {
    let tmp = tempfile::tempdir().unwrap();
    let bad = tmp.path().join("bad.toml");
    std::fs::write(&bad, "[nim]\nembed_model = \"nvidia/invalid-model\"\n").unwrap();
    let err = load_config(&[bad]).unwrap_err();
    let s = format!("{}", err);
    assert!(
        s.contains("Unsupported")
            || s.contains("invalid")
            || s.contains("Unsupported embedding model")
    );
}

#[test]
fn load_invalid_toml_returns_config_invalid() {
    let tmp = tempfile::tempdir().unwrap();
    let bad = tmp.path().join("bad.toml");
    std::fs::write(&bad, "[nim\nembed_model = \"x\"\n").unwrap();
    let err = load_config(&[bad]).unwrap_err();
    let s = format!("{}", err);
    assert!(s.contains("config invalid") || s.contains("TOML"));
}

#[test]
fn validate_accepts_defaults() {
    let cfg = llmwiki_cli::core::config::Config::default();
    assert!(llmwiki_cli::core::config::validate(&cfg).is_ok());
}

#[test]
fn validate_rejects_unknown_embed_model() {
    let mut cfg = llmwiki_cli::core::config::Config::default();
    cfg.nim.embed_model = "nvidia/not-a-real-model".into();
    let errs = llmwiki_cli::core::config::validate(&cfg).unwrap_err();
    assert!(errs.iter().any(|e| e.contains("unsupported embed_model")));
}

#[test]
fn validate_rejects_zero_batch_size() {
    let mut cfg = llmwiki_cli::core::config::Config::default();
    cfg.nim.batch_size = 0;
    let errs = llmwiki_cli::core::config::validate(&cfg).unwrap_err();
    assert!(errs.iter().any(|e| e.contains("batch_size must be >= 1")));
}

#[test]
fn validate_rejects_overlap_ge_chunk_tokens() {
    let mut cfg = llmwiki_cli::core::config::Config::default();
    cfg.wiki.default_chunk_tokens = 100;
    cfg.wiki.chunk_overlap_tokens = 100; // equal — invalid
    let errs = llmwiki_cli::core::config::validate(&cfg).unwrap_err();
    assert!(errs.iter().any(|e| e.contains("chunk_overlap_tokens")));
}

#[test]
fn validate_rejects_min_chunk_larger_than_default() {
    let mut cfg = llmwiki_cli::core::config::Config::default();
    cfg.wiki.default_chunk_tokens = 100;
    cfg.wiki.min_chunk_tokens = 200;
    let errs = llmwiki_cli::core::config::validate(&cfg).unwrap_err();
    assert!(errs.iter().any(|e| e.contains("min_chunk_tokens")));
}

#[test]
fn validate_accepts_whitelisted_embed_model() {
    let mut cfg = llmwiki_cli::core::config::Config::default();
    cfg.nim.embed_model = "nvidia/nv-embedqa-e5-v5".into();
    assert!(llmwiki_cli::core::config::validate(&cfg).is_ok());
}
