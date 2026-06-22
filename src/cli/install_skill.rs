use std::path::{Path, PathBuf};

use crate::error::WikiError;

pub struct InstallSkillArgs {
    pub global: bool,
    pub workspace: Option<PathBuf>,
    pub target: Option<PathBuf>,
}

pub fn run(args: InstallSkillArgs) -> Result<(), WikiError> {
    let source = resolve_skill_source(args.target)?;

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
    std::os::unix::fs::symlink(&source, &target)?;

    println!(
        "✓ Installed skill: {} -> {}",
        target.display(),
        source.display()
    );
    Ok(())
}

fn resolve_skill_source(target: Option<PathBuf>) -> Result<PathBuf, WikiError> {
    let candidate = target.unwrap_or_else(|| PathBuf::from("agents/skills/wiki"));

    if candidate.join("SKILL.md").exists() {
        return Ok(candidate);
    }

    let nested = candidate.join("wiki");
    if nested.join("SKILL.md").exists() {
        return Ok(nested);
    }

    Err(WikiError::Other(anyhow::anyhow!(
        "skill source not found: {}",
        candidate.display()
    )))
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
