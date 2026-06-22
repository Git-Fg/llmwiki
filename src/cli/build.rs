use std::path::PathBuf;

use crate::core::workspace::discover_workspace;
use crate::error::WikiError;

pub struct BuildArgs {
    pub workspace: Option<PathBuf>,
    pub since: Option<String>,
    pub dry_run: bool,
}

pub fn run(args: BuildArgs) -> Result<(), WikiError> {
    let ws = discover_workspace(
        args.workspace.clone(),
        std::env::var("WIKI_WORKSPACE").ok().map(PathBuf::from),
        std::env::current_dir()?,
    )?;

    let raw_dir = ws.join("raw");
    let mut pending = vec![];
    if raw_dir.exists() {
        for entry in walkdir::WalkDir::new(&raw_dir) {
            let entry = entry.map_err(|e| anyhow::anyhow!(e))?;
            if !entry.file_type().is_file() {
                continue;
            }
            // Read the head; treat as a pending source if it has the
            // `ingested:` frontmatter marker. The extension is not
            // significant — `ingest` accepts any source file.
            if let Ok(content) = std::fs::read_to_string(entry.path()) {
                if content.starts_with("---") && content.contains("ingested:") {
                    pending.push(entry.path().to_path_buf());
                }
            }
        }
    }

    if pending.is_empty() {
        println!("No pending raw sources to compile.");
        return Ok(());
    }

    if args.dry_run {
        println!("Would compile {} source(s):", pending.len());
        for p in pending {
            println!("  - {}", p.strip_prefix(&ws).unwrap().display());
        }
        return Ok(());
    }

    println!("Compiling {} source(s) via LLM...", pending.len());
    println!(
        "Note: LLM-driven compile requires external LLM call. In v1, this is a manual agent task."
    );
    println!("The agent should:");
    println!("  1. Read each pending source");
    println!("  2. Create or update wiki pages");
    println!("  3. Update index.md and log.md");
    Ok(())
}
