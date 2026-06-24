use std::path::{Path, PathBuf};

use crate::error::WikiError;
use crate::skills;

pub struct InstallSkillArgs {
    pub global: bool,
    pub workspace: Option<PathBuf>,
    /// Override the install directory. By default:
    ///   --global          → $HOME/.agents/skills/wiki
    ///   --workspace <ws>  → <ws>/.agents/skills/wiki
    /// Use --install-path to install to a different host's skills directory,
    /// e.g. `--install-path ~/.claude/skills/wiki` for Claude Code,
    /// `--install-path ~/.cursor/skills/wiki` for Cursor (if it supports
    /// SKILL.md), or `--install-path ~/.kimi/skills/wiki` for Kimi Code's
    /// brand path. The `~` is expanded to $HOME; relative paths are
    /// resolved against the workspace (or $HOME for --global).
    pub install_path: Option<PathBuf>,
}

/// Install the small `wiki` skill to the agent's skills directory.
///
/// v0.3.29 simplification: only the hub is installed to disk. Inline
/// sub-skills (`wiki-search`, `wiki-config`, etc.) are served on demand via
/// `llmwiki-cli skill get <topic>` from bytes embedded in the binary
/// (`rust-embed` over `skills/`). This keeps the install layout minimal —
/// one file at `~/.agents/skills/wiki/SKILL.md` — and avoids any
/// marketplace / validator / multi-plugin-manifest machinery.
///
/// v0.3.30: `--install-path <dir>` lets users target a specific host's
/// skills directory (Claude Code reads `~/.claude/skills/`, Cursor reads
/// `~/.cursor/skills/`, Kimi reads `~/.kimi/skills/` + the generic
/// `~/.agents/skills/` fallback). The default `~/.agents/skills/wiki`
/// works for Kimi and any other host that follows the
/// [agentskills.io](https://agentskills.io/) cross-host convention.
pub fn run(args: InstallSkillArgs) -> Result<(), WikiError> {
    let target = if let Some(p) = args.install_path {
        expand_tilde(p)
    } else if args.global {
        let home =
            std::env::var("HOME").map_err(|_| WikiError::Other(anyhow::anyhow!("HOME not set")))?;
        PathBuf::from(home).join(".agents/skills/wiki")
    } else {
        let ws = args
            .workspace
            .unwrap_or_else(|| std::env::current_dir().unwrap());
        ws.join(".agents/skills/wiki")
    };

    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    remove_existing_target(&target)?;
    std::fs::create_dir_all(&target)?;

    // Write the hub SKILL.md from the binary-embedded content. Sub-skills
    // are not installed — `llmwiki-cli skill get <topic>` serves them from
    // the binary at runtime.
    std::fs::write(target.join("SKILL.md"), skills::hub().as_bytes())?;

    println!(
        "✓ Installed wiki skill to {}\n  Inline sub-skills served via `llmwiki-cli skill get <topic>`",
        target.display()
    );
    Ok(())
}

/// Expand a leading `~` or `~user` in `path` to the user's home directory.
/// Falls back to the input unchanged if HOME is unset or the prefix is
/// not a tilde.
fn expand_tilde(path: PathBuf) -> PathBuf {
    let Some(s) = path.to_str() else {
        return path;
    };
    if s == "~" {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home);
        }
    } else if let Some(rest) = s.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    path
}

fn remove_existing_target(target: &Path) -> Result<(), WikiError> {
    let Ok(metadata) = std::fs::symlink_metadata(target) else {
        return Ok(());
    };

    if metadata.file_type().is_symlink() {
        std::fs::remove_file(target)?;
    } else if metadata.is_dir() {
        std::fs::remove_dir_all(target)?;
    } else {
        std::fs::remove_file(target)?;
    }

    Ok(())
}
