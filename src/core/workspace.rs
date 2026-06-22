use crate::error::WikiError;
use std::path::{Path, PathBuf};

/// Discover the wiki workspace root by priority order:
/// 1. `--workspace <path>` flag
/// 2. `WIKI_WORKSPACE` env var
/// 3. Walk up from CWD looking for `.wiki/` directory
/// 4. `~/wiki` if it exists
pub fn discover_workspace(
    flag: Option<PathBuf>,
    env: Option<PathBuf>,
    cwd: PathBuf,
) -> Result<PathBuf, WikiError> {
    if let Some(p) = flag {
        return Ok(p.canonicalize().unwrap_or(p));
    }
    if let Some(p) = env {
        return Ok(p.canonicalize().unwrap_or(p));
    }
    if let Some(p) = walk_up_for_dot_wiki(&cwd) {
        return Ok(p);
    }
    if let Some(home) = home_dir() {
        let candidate = home.join("wiki");
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

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}
