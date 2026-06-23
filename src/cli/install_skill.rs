use std::path::{Path, PathBuf};

use crate::error::WikiError;
use crate::skills;

pub struct InstallSkillArgs {
    pub global: bool,
    pub workspace: Option<PathBuf>,
}

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

    // Write the hub SKILL.md from the binary-embedded content
    std::fs::write(target.join("SKILL.md"), skills::SKILL_MD)?;

    // Write each sub-skill from the binary-embedded content
    for (name, content) in skills::TOPICS {
        let sub_dir = target.join(name.to_uppercase());
        std::fs::create_dir_all(&sub_dir)?;
        std::fs::write(sub_dir.join("SKILL.md"), content)?;
    }

    println!(
        "✓ Installed skill bundle to {} (1 hub + {} sub-skills)",
        target.display(),
        skills::TOPICS.len()
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
