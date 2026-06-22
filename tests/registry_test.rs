use llmwiki_cli::core::registry::Registry;
use std::io::Write;

fn write_tmp_toml(content: &str) -> std::path::PathBuf {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("wiki-root.toml");
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
    std::mem::forget(dir);
    path
}

#[test]
fn load_parses_defaults_and_entries() {
    let path = write_tmp_toml(
        r#"
[defaults.nim]
embed_model = "nvidia/nv-embed-v1"
base_url = "https://integrate.api.nvidia.com"

[defaults.wiki]
default_chunk_tokens = 512

[mywiki]
path = "/tmp/mywiki"
tags = ["test"]
description = "Test wiki"
"#,
    );
    let reg = Registry::load_from(&path).unwrap();
    assert!(reg.defaults.raw.is_some());
    assert_eq!(reg.entries.len(), 1);
    assert_eq!(reg.entries[0].alias, "mywiki");
    assert_eq!(reg.entries[0].path, std::path::PathBuf::from("/tmp/mywiki"));
}

#[test]
fn load_returns_error_for_missing_file() {
    let result = Registry::load_from(std::path::Path::new("/nonexistent/wiki-root.toml"));
    assert!(result.is_err());
}

#[test]
fn load_skips_non_wiki_tables() {
    let path = write_tmp_toml(
        r#"
[some_random_table]
foo = "bar"

[realwiki]
path = "/tmp/real"
"#,
    );
    let reg = Registry::load_from(&path).unwrap();
    assert_eq!(reg.entries.len(), 1);
    assert_eq!(reg.entries[0].alias, "realwiki");
}

#[test]
fn resolve_config_merges_defaults_and_overrides() {
    let path = write_tmp_toml(
        r#"
[defaults.nim]
embed_model = "nvidia/nv-embed-v1"
base_url = "https://integrate.api.nvidia.com"

[defaults.wiki]
default_chunk_tokens = 512

[pharma]
path = "/tmp/pharma"
description = "Pharma wiki"

[pharma.nim]
embed_model = "nvidia/nv-embedqa-e5-v5"
"#,
    );
    let reg = Registry::load_from(&path).unwrap();
    let cfg = reg.resolve_config("pharma").unwrap();

    // Override should win
    assert_eq!(cfg.nim.embed_model, "nvidia/nv-embedqa-e5-v5");
    // Default should fill in
    assert_eq!(cfg.nim.base_url, "https://integrate.api.nvidia.com");
    // Wiki defaults preserved
    assert_eq!(cfg.wiki.default_chunk_tokens, 512);
}

#[test]
fn resolve_config_uses_defaults_when_no_alias_override() {
    let path = write_tmp_toml(
        r#"
[defaults.nim]
embed_model = "default-model"

[simple]
path = "/tmp/simple"
"#,
    );
    let reg = Registry::load_from(&path).unwrap();
    let cfg = reg.resolve_config("simple").unwrap();
    assert_eq!(cfg.nim.embed_model, "default-model");
}

#[test]
fn resolve_active_cwd_prefix_match() {
    let path = write_tmp_toml(
        r#"
[mywiki]
path = "/tmp/mywiki"
description = "Test"
"#,
    );
    let reg = Registry::load_from(&path).unwrap();
    let (alias, _, _) = reg
        .resolve_active(
            None,
            None,
            None,
            None,
            std::path::Path::new("/tmp/mywiki/wiki/sub"),
        )
        .unwrap();
    assert_eq!(alias, "mywiki");
}

#[test]
fn resolve_active_single_wiki_shortcut() {
    let path = write_tmp_toml(
        r#"
[solo]
path = "/tmp/solo"
description = "Solo"
"#,
    );
    let reg = Registry::load_from(&path).unwrap();
    let (alias, _, _) = reg
        .resolve_active(None, None, None, None, std::path::Path::new("/etc"))
        .unwrap();
    assert_eq!(alias, "solo");
}

#[test]
fn resolve_active_flag_alias_wins() {
    let path = write_tmp_toml(
        r#"
[wiki1]
path = "/tmp/wiki1"
description = "One"

[wiki2]
path = "/tmp/wiki2"
description = "Two"
"#,
    );
    let reg = Registry::load_from(&path).unwrap();
    let (alias, _, _) = reg
        .resolve_active(
            Some("wiki2"),
            None,
            None,
            None,
            std::path::Path::new("/tmp/wiki1"),
        )
        .unwrap();
    assert_eq!(alias, "wiki2");
}

#[test]
fn resolve_active_errors_on_no_match() {
    let path = write_tmp_toml(
        r#"
[wiki1]
path = "/tmp/wiki1"
description = "One"

[wiki2]
path = "/tmp/wiki2"
description = "Two"
"#,
    );
    let reg = Registry::load_from(&path).unwrap();
    let result = reg.resolve_active(
        None,
        None,
        None,
        None,
        std::path::Path::new("/nonexistent/path"),
    );
    assert!(result.is_err());
}
