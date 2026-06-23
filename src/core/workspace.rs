use crate::core::registry::{home_dir, Registry};
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
/// 6. Walk up from CWD looking for `.llmwiki-cli/` directory (skip HOME)
/// 7. Single-wiki shortcut (registry has exactly one entry)
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

    // Walk up from CWD looking for `.llmwiki-cli/` (skip HOME so
    // `~/.llmwiki-cli/` is treated as per-computer config, not a workspace).
    if let Some(p) = walk_up_for_llmwiki_cli_dir(&cwd) {
        return Ok(p);
    }

    // Single-wiki shortcut: if the registry has exactly one entry and we
    // haven't matched anything else, default to it. Avoids forcing the user
    // to pass `--wiki` for a single-wiki install.
    if let Ok(reg) = Registry::discover() {
        if reg.entries.len() == 1 {
            return Ok(reg.entries[0].path.clone());
        }
    }

    Err(WikiError::WorkspaceNotFound)
}

/// Walk up from `start` collecting the closest ancestor containing a
/// `.llmwiki-cli/` directory. Skips the user's HOME so `~/.llmwiki-cli/`
/// is treated as the per-computer config location, not as a workspace marker.
fn walk_up_for_llmwiki_cli_dir(start: &Path) -> Option<PathBuf> {
    let canonical = start.canonicalize().ok()?;
    let home_canon = home_dir().and_then(|h| h.canonicalize().ok());
    let mut current: Option<PathBuf> = Some(canonical);
    while let Some(dir) = current {
        if let Some(ref h) = home_canon {
            if dir == *h {
                current = dir.parent().map(PathBuf::from);
                continue;
            }
        }
        if dir.join(".llmwiki-cli").is_dir() {
            return Some(dir);
        }
        current = dir.parent().map(PathBuf::from);
    }
    None
}
