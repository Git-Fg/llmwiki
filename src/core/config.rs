use crate::core::registry::Registry;
use crate::error::WikiError;
use std::path::{Path, PathBuf};

include!("config_types.rs");

impl Default for Config {
    fn default() -> Self {
        Config {
            nim: default_nim(),
            wiki: default_wiki(),
            config_version: 1,
        }
    }
}

use crate::core::registry::home_dir;

/// Ordered list of config file paths searched at startup.
///
/// Order convention: **lowest priority first, highest priority last** so
/// `load_config()`'s "last-wins" merge semantics give the intuitively-correct
/// result: the highest-priority file overrides everything below it. This
/// matches the standard CLI config convention (later overrides earlier),
/// used by pip, git, hk, mise, etc.
///
/// Concrete order returned by this function (when all slots are populated):
///   1. `~/.llmwiki-cli/config.toml` — per-computer fallback (lowest)
///   2. `<workspace>/.llmwiki-cli/config.toml` — per-workspace
///   3. `$LLMWIKI_CONFIG` env var (highest; only added when set + non-empty)
///
/// `wiki config paths` reverses this list before printing so users see the
/// "highest priority first" order they intuitively expect when debugging.
pub fn config_paths(workspace: &Path) -> Vec<PathBuf> {
    let mut lowest_first: Vec<PathBuf> = Vec::new();

    // Lowest priority: per-computer fallback.
    if let Some(home) = home_dir() {
        lowest_first.push(home.join(".llmwiki-cli").join("config.toml"));
    }

    // Middle: per-workspace (walk-up from `workspace` looking for
    // `.llmwiki-cli/config.toml`).
    if let Some(p) = walk_up_for_llmwiki_cli_config(workspace) {
        lowest_first.push(p);
    }

    // Highest priority: hard override via env var — exact file, no merging.
    if let Ok(p) = std::env::var("LLMWIKI_CONFIG") {
        if !p.is_empty() {
            lowest_first.push(PathBuf::from(p));
        }
    }

    lowest_first
}

/// Walk up from `start` to find the closest per-workspace config candidate.
/// Skips the user's HOME directory so `~/.llmwiki-cli/` is treated as the
/// per-computer config location, not as a workspace marker.
///
/// Always returns `Some`: either the closest `.llmwiki-cli/config.toml`
/// found in an ancestor (which may or may not exist on disk — `load_config`
/// skips missing files), or `<workspace>/.llmwiki-cli/config.toml` as the
/// default location if no ancestor carries one. Returning `Some`
/// unconditionally lets `wiki config paths` print the candidate location so
/// users see where to put a per-workspace config, even when the workspace
/// directory doesn't exist yet (e.g., scripted setup).
fn walk_up_for_llmwiki_cli_config(start: &Path) -> Option<PathBuf> {
    let canonical = start.canonicalize().ok();
    let home_canon = home_dir().and_then(|h| h.canonicalize().ok());
    // When canonicalize fails (path doesn't exist), walk the literal path
    // components instead of bailing. This keeps the candidate discoverable
    // for not-yet-created workspaces.
    let mut current: Option<PathBuf> =
        Some(canonical.clone().unwrap_or_else(|| start.to_path_buf()));
    while let Some(dir) = current {
        // Skip HOME — `~/.llmwiki-cli/` is the per-computer config and must
        // not be promoted to a workspace marker.
        if let Some(ref h) = home_canon {
            if dir == *h {
                current = dir.parent().map(PathBuf::from);
                continue;
            }
        }
        let candidate = dir.join(".llmwiki-cli").join("config.toml");
        // Prefer an existing file (walk-up found a real config) — its parent
        // ancestor already proved this is a workspace.
        if candidate.is_file() {
            return Some(candidate);
        }
        // Prefer a `.llmwiki-cli/` directory (the workspace marker) over
        // `<workspace>/.llmwiki-cli/config.toml` as the default — the marker
        // implies this directory is intentionally a workspace.
        if dir.join(".llmwiki-cli").is_dir() {
            return Some(candidate);
        }
        // Don't walk above the filesystem root. Also stop when we've
        // exhausted the canonical path; if we never canonicalized, fall
        // through to the default candidate below.
        current = match canonical {
            Some(_) => dir.parent().map(PathBuf::from),
            None => None,
        };
    }
    // No ancestor with `.llmwiki-cli/` and no `.llmwiki-cli/config.toml`:
    // default to `<workspace>/.llmwiki-cli/config.toml` so the user has a
    // discoverable target. `load_config` skips missing files.
    Some(start.join(".llmwiki-cli").join("config.toml"))
}

/// Load and merge every config file in `paths` (later wins per scalar key).
/// Parses TOML (matches `wiki-root.toml` format). Returns `Ok(default)` if
/// no file in `paths` exists. Validates the embedding model is whitelisted
/// after merging — callers that want to skip validation should call
/// `load_config_unvalidated` or run `validate` separately.
pub fn load_config(paths: &[PathBuf]) -> Result<Config, WikiError> {
    let cfg = load_config_unvalidated(paths)?;
    validate(&cfg).map_err(|errs| {
        WikiError::Other(anyhow::anyhow!(
            "config validation failed:\n  - {}",
            errs.join("\n  - ")
        ))
    })?;
    Ok(cfg)
}

/// Like `load_config` but skips the whitelist/model validation step.
/// Used by `wiki config validate` and by tests that want to inspect a
/// config regardless of whether it's valid.
pub fn load_config_unvalidated(paths: &[PathBuf]) -> Result<Config, WikiError> {
    // Deep-merge TOML values across all sources in priority order (lowest
    // priority first, highest priority last so it overrides). Then
    // deserialize the merged table into `Config` once — `#[serde(default)]`
    // on every field handles missing keys uniformly. This replaces the
    // older per-field `Config::merge()` which silently dropped overrides
    // for any field not explicitly listed (e.g. `wiki.*`).
    let mut merged_value: toml::Value = toml::Value::Table(toml::value::Table::new());
    for path in paths {
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(path)?;
        let partial: toml::Value = text.parse().map_err(|e| WikiError::ConfigInvalid {
            path: path.display().to_string(),
            line: 0,
            message: format!("TOML parse error: {e}"),
        })?;
        crate::core::registry::deep_merge_into(&mut merged_value, partial);
    }
    let cfg: Config =
        merged_value
            .try_into()
            .map_err(|e: toml::de::Error| WikiError::ConfigInvalid {
                path: paths
                    .iter()
                    .find(|p| p.exists())
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "<merged>".to_string()),
                line: 0,
                message: format!("Failed to deserialize merged config: {e}"),
            })?;
    Ok(cfg)
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

    load_config(&config_paths(workspace))
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
