use std::path::{Path, PathBuf};

use crate::error::WikiError;
use crate::skills;

pub struct InstallSkillArgs {
    pub global: bool,
    pub workspace: Option<PathBuf>,
}

/// Install the small `wiki` skill to the agent's skills directory.
///
/// v0.3.29 simplification: only the hub is installed to disk. Inline
/// sub-skills (`wiki-search`, `wiki-config`, etc.) are served on demand via
/// `llmwiki-cli skill get <topic>` from bytes embedded in the binary
/// (`rust-embed` over `skills/`). This keeps the install layout minimal —
/// one file at `~/.agents/skills/wiki/SKILL.md` — and avoids any
/// marketplace / validator / multi-plugin-manifest machinery.
pub fn run(args: InstallSkillArgs) -> Result<(), WikiError> {
    let target = if args.global {
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
