use crate::core::registry::Registry;
use crate::error::WikiError;
use anyhow::Context;
use std::path::PathBuf;

pub struct InitArgs {
    pub path: PathBuf,
    pub alias: Option<String>,
    pub tags: Vec<String>,
}

pub fn run(args: InitArgs) -> Result<(), WikiError> {
    let target = if args.path.exists() && args.path.is_dir() {
        args.path.clone()
    } else {
        std::fs::create_dir_all(&args.path)
            .with_context(|| format!("create {}", args.path.display()))?;
        args.path.clone()
    };

    std::fs::create_dir_all(target.join("wiki")).context("create wiki/")?;
    std::fs::create_dir_all(target.join("raw/articles")).context("create raw/articles/")?;

    let today = std::process::Command::new("date")
        .arg("+%Y-%m-%d")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "2026-06-21".to_string());

    let template = include_str!("../../resources/page.template.md").replace("YYYY-MM-DD", &today);

    write_if_absent(&target.join("wiki/overview.md"), &template)?;
    write_if_absent(
        &target.join("wiki/log.md"),
        "# Log\n\nChronological record of wiki operations.\n",
    )?;
    write_if_absent(&target.join("raw/articles/.gitkeep"), "")?;
    write_if_absent(
        &target.join("index.md"),
        "# Index\n\n## Pages\n\nNo pages yet.\n",
    )?;
    write_if_absent(
        &target.join(".gitignore"),
        "embeddings.jsonl\n.env\n.env.local\n",
    )?;

    // Initialize git repo if not already one
    if !target.join(".git").exists() {
        std::process::Command::new("git")
            .arg("init")
            .arg(&target)
            .output()
            .context("git init")?;
    }

    // Auto-register in wiki-root.toml
    let alias = args
        .alias
        .clone()
        .or_else(|| target.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "wiki".to_string());

    let reg = Registry::discover().or_else(|_| {
        // No registry exists; create one at the conventional user-global slot.
        // `~/.agents/wiki-root.toml` is the highest-priority user-global
        // path and the conventional location for AI agent config (parallel
        // to `~/.agents/skills/wiki/`). Picking the lowest-priority
        // `~/wiki-root.toml` would create shadowing confusion later.
        let default_path = crate::core::registry::home_dir()
            .map(|h| h.join(".agents").join("wiki-root.toml"))
            .ok_or_else(|| WikiError::Other(anyhow::anyhow!("no home dir")))?;
        if let Some(parent) = default_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&default_path, "# wiki-root.toml\n")?;
        Registry::load_from(&default_path)
    })?;

    let mut reg = reg;
    if let Err(e) = reg.add_entry(&alias, &target, &args.tags, None) {
        eprintln!("Warning: {}", e);
    } else {
        reg.save()?;
        println!("Registered wiki '{}' in wiki-root.toml", alias);
    }

    println!("✓ Initialized wiki at {}", target.display());
    Ok(())
}

fn write_if_absent(path: &std::path::Path, content: &str) -> Result<(), WikiError> {
    if !path.exists() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(path, content).map_err(WikiError::Io)?;
    }
    Ok(())
}
