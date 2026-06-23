use crate::core::registry::Registry;
use crate::error::WikiError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Config {
    #[serde(default = "default_nim")]
    pub nim: NimConfig,
    #[serde(default = "default_wiki")]
    pub wiki: WikiConfig,
    #[serde(default)]
    pub config_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct NimConfig {
    #[serde(default = "default_nim_base_url")]
    pub base_url: String,
    #[serde(default = "default_embed_model")]
    pub embed_model: String,
    #[serde(default)]
    pub rerank_model: String,
    #[serde(default)]
    pub embed_dim_override: Option<usize>,
    #[serde(default = "default_api_key_env")]
    pub api_key_env: String,
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
    #[serde(default = "default_timeout")]
    pub request_timeout_secs: u64,
    #[serde(default)]
    pub retry: RetryConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, schemars::JsonSchema)]
pub struct RetryConfig {
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,
    #[serde(default = "default_backoff")]
    pub backoff_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct WikiConfig {
    #[serde(default = "default_chunk_tokens")]
    pub default_chunk_tokens: usize,
    #[serde(default = "default_chunk_overlap")]
    pub chunk_overlap_tokens: usize,
    #[serde(default = "default_min_chunk")]
    pub min_chunk_tokens: usize,
    #[serde(default = "default_true")]
    pub require_frontmatter: bool,
    #[serde(default = "default_wikilinks_min")]
    pub require_wikilinks_min: usize,
}

fn default_nim() -> NimConfig {
    NimConfig {
        base_url: default_nim_base_url(),
        embed_model: default_embed_model(),
        rerank_model: String::new(),
        embed_dim_override: None,
        api_key_env: default_api_key_env(),
        batch_size: default_batch_size(),
        request_timeout_secs: default_timeout(),
        retry: RetryConfig::default(),
    }
}

fn default_wiki() -> WikiConfig {
    WikiConfig {
        default_chunk_tokens: default_chunk_tokens(),
        chunk_overlap_tokens: default_chunk_overlap(),
        min_chunk_tokens: default_min_chunk(),
        require_frontmatter: true,
        require_wikilinks_min: default_wikilinks_min(),
    }
}

fn default_nim_base_url() -> String {
    "https://integrate.api.nvidia.com".into()
}

fn default_embed_model() -> String {
    "nvidia/nv-embed-v1".into()
}

fn default_api_key_env() -> String {
    "NVIDIA_NIM_API_KEY".into()
}

fn default_batch_size() -> usize {
    8
}

fn default_timeout() -> u64 {
    30
}

fn default_max_attempts() -> u32 {
    3
}

fn default_backoff() -> u64 {
    500
}

fn default_chunk_tokens() -> usize {
    512
}

fn default_chunk_overlap() -> usize {
    128
}

fn default_min_chunk() -> usize {
    32
}

fn default_wikilinks_min() -> usize {
    2
}

fn default_true() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        Config {
            nim: default_nim(),
            wiki: default_wiki(),
            config_version: 1,
        }
    }
}

pub fn load_config(paths: &[PathBuf]) -> Result<Config, WikiError> {
    let mut merged: Config = Config::default();
    for path in paths {
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(path)?;
        let partial: Config =
            serde_yaml::from_str(&text).map_err(|e| WikiError::ConfigInvalid {
                path: path.display().to_string(),
                line: e.location().map(|l| l.line()).unwrap_or(0),
                message: e.to_string(),
            })?;
        merged = merge(merged, partial);
    }

    // Validate that the configured embedding/rerank model is whitelisted
    let whitelisted = [
        "nvidia/nv-embed-v1",
        "nvidia/nv-embedqa-e5-v5",
        "nvidia/nv-embedcode-7b-v1",
        "nvidia/llama-nemotron-embed-1b-v2",
        "nvidia/llama-nemotron-embed-vl-1b-v2",
        "nvidia/llama-nemotron-rerank-1b-v2",
        "nvidia/llama-nemotron-rerank-vl-1b-v2",
        "nvidia/nv-rerankqa-mistral-4b-v3",
    ];
    if !merged.nim.embed_model.is_empty() && !whitelisted.contains(&merged.nim.embed_model.as_str())
    {
        return Err(WikiError::ConfigInvalid {
            path: "validation".into(),
            line: 0,
            message: format!("Unsupported embedding model: {}", merged.nim.embed_model),
        });
    }

    Ok(merged)
}

fn merge(mut base: Config, over: Config) -> Config {
    if !over.nim.embed_model.is_empty() {
        base.nim.embed_model = over.nim.embed_model;
    }
    if !over.nim.rerank_model.is_empty() {
        base.nim.rerank_model = over.nim.rerank_model;
    }
    if over.nim.embed_dim_override.is_some() {
        base.nim.embed_dim_override = over.nim.embed_dim_override;
    }
    base.config_version = over.config_version;
    base
}

pub fn resolve_config(workspace: &Path) -> Result<Config, WikiError> {
    // Prefer wiki-root.toml registry if it has a matching entry
    if let Ok(reg) = Registry::discover() {
        // Canonicalize workspace to match registry paths
        let ws_canon = workspace
            .canonicalize()
            .unwrap_or_else(|_| workspace.to_path_buf());
        for entry in &reg.entries {
            if entry.path == ws_canon || entry.path == workspace {
                return reg.resolve_config(&entry.alias);
            }
        }
    }

    // Legacy: load from ~/.config/wiki/config.yaml and .wiki/config.yaml
    let mut paths = vec![home_dir().map(|h| h.join(".config/wiki/config.yaml"))];
    paths.push(Some(workspace.join(".wiki/config.yaml")));
    let paths: Vec<PathBuf> = paths.into_iter().flatten().collect();
    load_config(&paths)
}

/// Field-level validation independent of `load_config`. Used by `wiki config
/// validate` and by callers that load a `Config` directly from the registry.
/// Returns `Ok(())` if all whitelisted values are valid; otherwise a list of
/// human-readable errors.
pub fn validate(cfg: &Config) -> Result<(), Vec<String>> {
    let mut errs = Vec::new();
    let whitelisted = [
        "nvidia/nv-embed-v1",
        "nvidia/nv-embedqa-e5-v5",
        "nvidia/nv-embedcode-7b-v1",
        "nvidia/llama-nemotron-embed-1b-v2",
        "nvidia/llama-nemotron-embed-vl-1b-v2",
        "nvidia/llama-nemotron-rerank-1b-v2",
        "nvidia/llama-nemotron-rerank-vl-1b-v2",
        "nvidia/nv-rerankqa-mistral-4b-v3",
    ];
    if !cfg.nim.embed_model.is_empty() && !whitelisted.contains(&cfg.nim.embed_model.as_str()) {
        errs.push(format!(
            "unsupported embed_model: {} (allowed: {})",
            cfg.nim.embed_model,
            whitelisted.join(", ")
        ));
    }
    if cfg.nim.batch_size == 0 {
        errs.push("nim.batch_size must be >= 1".into());
    }
    if cfg.wiki.default_chunk_tokens == 0 {
        errs.push("wiki.default_chunk_tokens must be >= 1".into());
    }
    if cfg.wiki.chunk_overlap_tokens >= cfg.wiki.default_chunk_tokens {
        errs.push(format!(
            "wiki.chunk_overlap_tokens ({}) must be < wiki.default_chunk_tokens ({})",
            cfg.wiki.chunk_overlap_tokens, cfg.wiki.default_chunk_tokens
        ));
    }
    if cfg.wiki.min_chunk_tokens > cfg.wiki.default_chunk_tokens {
        errs.push(format!(
            "wiki.min_chunk_tokens ({}) must be <= wiki.default_chunk_tokens ({})",
            cfg.wiki.min_chunk_tokens, cfg.wiki.default_chunk_tokens
        ));
    }
    if errs.is_empty() {
        Ok(())
    } else {
        Err(errs)
    }
}

/// Convenience wrapper that returns a single `WikiError` instead of a `Vec`.
/// Use this from CLI handlers that just want to fail fast with one error.
pub fn validate_or_error(cfg: &Config) -> Result<(), crate::error::WikiError> {
    validate(cfg)
        .map_err(|errs| crate::error::WikiError::Other(anyhow::anyhow!(errs.join("\n  - "))))
}

use crate::core::registry::home_dir;

/// Resolve the NIM API key, trying (in order):
/// 1. The configured env var (e.g. `NVIDIA_NIM_API_KEY`)
/// 2. The common `NVIDIA_API_KEY` fallback so shells that already
///    export the upstream NVIDIA name still work out-of-the-box.
pub fn resolve_api_key(cfg: &NimConfig) -> String {
    if let Ok(v) = std::env::var(&cfg.api_key_env) {
        if !v.is_empty() {
            return v;
        }
    }
    if cfg.api_key_env != "NVIDIA_API_KEY" {
        if let Ok(v) = std::env::var("NVIDIA_API_KEY") {
            if !v.is_empty() {
                return v;
            }
        }
    }
    String::new()
}
