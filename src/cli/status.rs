use std::collections::HashSet;
use std::path::PathBuf;

use crate::core::config::resolve_config;
use crate::core::embeddings::EmbeddingsFile;
use crate::core::workspace::{discover_workspace, pages_dir};
use crate::error::WikiError;

pub struct StatusArgs {
    pub workspace: Option<PathBuf>,
    pub wiki: Option<String>,
    pub json: bool,
}

pub fn run(args: StatusArgs) -> Result<(), WikiError> {
    let ws = discover_workspace(
        args.workspace.clone(),
        args.wiki.as_deref(),
        std::env::var("WIKI_WORKSPACE").ok().map(PathBuf::from),
        std::env::var("WIKI_ACTIVE").ok().as_deref(),
        std::env::current_dir()?,
    )?;
    let cfg = resolve_config(&ws)?;

    let mut page_count = 0;
    let wiki_dir = pages_dir(&ws, &cfg.wiki.pages_dir);
    if wiki_dir.exists() {
        for entry in walkdir::WalkDir::new(&wiki_dir) {
            let entry = entry.map_err(|e| anyhow::anyhow!(e))?;
            if entry.path().extension().and_then(|s| s.to_str()) == Some("md") {
                page_count += 1;
            }
        }
    }

    let emb = EmbeddingsFile::read_from(&ws.join("embeddings.jsonl"))?;
    let embedded_pages: HashSet<String> = emb.pages.iter().map(|p| p.path.clone()).collect();
    let total_chunks = emb.pages.iter().map(|p| p.chunks.len()).sum::<usize>();

    let raw_count = if ws.join("raw").exists() {
        let mut count = 0;
        for entry in walkdir::WalkDir::new(ws.join("raw")) {
            let entry = entry.map_err(|e| anyhow::anyhow!(e))?;
            if entry.path().is_file() {
                count += 1;
            }
        }
        count
    } else {
        0
    };

    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "workspace": ws.display().to_string(),
                "pages": page_count,
                "embedded_pages": embedded_pages.len(),
                "embedding_chunks": total_chunks,
                "raw_sources": raw_count,
            }))?
        );
    } else {
        println!("\nWiki: {}", ws.display());
        println!("  Pages: {page_count}");
        println!(
            "  Embedded: {} ({} chunks)",
            embedded_pages.len(),
            total_chunks
        );
        println!("  Raw sources: {raw_count}");
    }

    Ok(())
}
