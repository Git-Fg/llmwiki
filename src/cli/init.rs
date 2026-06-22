use crate::error::WikiError;
use anyhow::Context;
use std::path::PathBuf;

pub fn run(path: PathBuf) -> Result<(), WikiError> {
    let target = if path.exists() && path.is_dir() {
        path.clone()
    } else {
        std::fs::create_dir_all(&path).with_context(|| format!("create {}", path.display()))?;
        path.clone()
    };

    std::fs::create_dir_all(target.join("wiki")).context("create wiki/")?;
    std::fs::create_dir_all(target.join("raw/articles")).context("create raw/articles/")?;
    std::fs::create_dir_all(target.join(".wiki")).context("create .wiki/")?;

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
    write_if_absent(&target.join(".wiki/config.yaml"), DEFAULT_CONFIG)?;
    write_if_absent(&target.join(".gitignore"), "embeddings.jsonl\n.wiki/embed-watch.pid\n.wiki/embed-watch.log\n.wiki/cache/\n.env\n.env.local\n")?;

    // Initialize git repo if not already one
    if !target.join(".git").exists() {
        std::process::Command::new("git")
            .arg("init")
            .arg(&target)
            .output()
            .context("git init")?;
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

const DEFAULT_CONFIG: &str = r#"config_version: 1

nim:
  embed_model: "nvidia/nv-embed-v1"

wiki:
  default_chunk_tokens: 512
"#;
