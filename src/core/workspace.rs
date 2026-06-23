use crate::core::registry::Registry;
use crate::error::WikiError;
use std::path::{Path, PathBuf};

/// Discover the wiki workspace root.
///
/// Priority order:
/// 1. `--workspace <path>` flag
/// 2. `--wiki <alias>` flag (looks up path in registry)
/// 3. `WIKI_WORKSPACE` env var
/// 4. `WIKI_ACTIVE` env var (looks up path in registry)
/// 5. wiki-root.toml registry (CWD match against registered paths)
/// 6. Walk up from CWD looking for `.wiki/` directory
/// 7. `~/llmwiki-cli/` if it has `.wiki/` (user-global workspace)
pub fn discover_workspace(
    flag: Option<PathBuf>,
    wiki_alias: Option<&str>,
    env: Option<PathBuf>,
    env_active: Option<&str>,
    cwd: PathBuf,
) -> Result<PathBuf, WikiError> {
    if let Some(p) = flag {
        return Ok(p.canonicalize().unwrap_or(p));
    }

    // Try wiki alias from flag
    if let Some(alias) = wiki_alias {
        if let Ok(reg) = Registry::discover() {
            if let Some(entry) = reg.entries.iter().find(|e| e.alias == alias) {
                return Ok(entry.path.clone());
            }
        }
    }

    if let Some(p) = env {
        return Ok(p.canonicalize().unwrap_or(p));
    }

    // Try WIKI_ACTIVE
    if let Some(alias) = env_active {
        if let Ok(reg) = Registry::discover() {
            if let Some(entry) = reg.entries.iter().find(|e| e.alias == alias) {
                return Ok(entry.path.clone());
            }
        }
    }

    // Try registry CWD match
    if let Ok(reg) = Registry::discover() {
        for entry in &reg.entries {
            if cwd.starts_with(&entry.path) {
                return Ok(entry.path.clone());
            }
        }
    }

    if let Some(p) = walk_up_for_dot_wiki(&cwd) {
        return Ok(p);
    }
    if let Some(home) = home_dir() {
        // User-global workspace at `~/llmwiki-cli/.wiki/` — mirrors the
        // `~/llmwiki-cli/config.toml` path used by `config_paths()`.
        let candidate = home.join("llmwiki-cli");
        if candidate.join(".wiki").exists() {
            return Ok(candidate.canonicalize().unwrap_or(candidate));
        }
    }
    Err(WikiError::WorkspaceNotFound)
}

fn walk_up_for_dot_wiki(start: &Path) -> Option<PathBuf> {
    let mut current = start.canonicalize().ok()?;
    loop {
        if current.join(".wiki").exists() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

use crate::core::registry::home_dir;
