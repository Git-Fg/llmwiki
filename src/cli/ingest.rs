use chrono::Utc;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

use crate::core::workspace::discover_workspace;
use crate::error::WikiError;

pub struct IngestArgs {
    pub workspace: Option<PathBuf>,
    pub wiki: Option<String>,
    pub source: PathBuf,
    pub no_compile: bool,
    pub source_type: Option<String>,
}

pub fn run(args: IngestArgs) -> Result<(), WikiError> {
    let ws = discover_workspace(
        args.workspace.clone(),
        args.wiki.as_deref(),
        std::env::var("WIKI_WORKSPACE").ok().map(PathBuf::from),
        std::env::var("WIKI_ACTIVE").ok().as_deref(),
        std::env::current_dir()?,
    )?;

    if !args.source.exists() {
        return Err(WikiError::Other(anyhow::anyhow!(
            "source not found: {}",
            args.source.display()
        )));
    }

    let filename = args
        .source
        .file_name()
        .ok_or_else(|| WikiError::Other(anyhow::anyhow!("source has no filename")))?
        .to_string_lossy()
        .to_string();
    let dest_dir = ws.join("raw/articles");
    std::fs::create_dir_all(&dest_dir)?;
    let dest = dest_dir.join(&filename);

    let body = std::fs::read_to_string(&args.source)?;
    let sha = hex::encode(Sha256::digest(body.as_bytes()));
    let today = Utc::now().format("%Y-%m-%d").to_string();

    let frontmatter = format!(
        "---\nsource_type: {}\ningested: {}\nsha256: {}\n---\n\n",
        args.source_type.clone().unwrap_or_else(|| "article".into()),
        today,
        sha
    );

    std::fs::write(&dest, format!("{frontmatter}{body}"))?;
    println!("✓ Added source to {}", dest.display());

    let log_path = ws.join("log.md");
    let log_entry = format!(
        "\n## [{}] ingest | {}\n- Added source: {}\n",
        today,
        filename,
        dest.strip_prefix(&ws).unwrap().display()
    );
    let mut log = std::fs::read_to_string(&log_path).unwrap_or_else(|_| "# Log\n".into());
    log.push_str(&log_entry);
    std::fs::write(&log_path, log)?;

    if !args.no_compile {
        println!("Note: LLM-driven compile pass is not yet implemented in v1. Run `wiki build` to trigger it.");
    }

    Ok(())
}
