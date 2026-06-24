use std::path::PathBuf;

use crate::core::config::resolve_config;
use crate::core::workspace::{discover_workspace, pages_dir};
use crate::error::WikiError;

pub struct TreeArgs {
    pub workspace: Option<PathBuf>,
    pub wiki: Option<String>,
    pub json: bool,
}

#[derive(serde::Serialize)]
struct TreeEntry {
    slug: String,
    path: String,
    title: Option<String>,
    tags: Vec<String>,
    embedded: bool,
}

#[derive(serde::Serialize)]
struct TreeOutput {
    entries: Vec<TreeEntry>,
}

pub fn run(args: TreeArgs) -> Result<(), WikiError> {
    let ws = discover_workspace(
        args.workspace.clone(),
        args.wiki.as_deref(),
        std::env::var("WIKI_WORKSPACE").ok().map(PathBuf::from),
        std::env::var("WIKI_ACTIVE").ok().as_deref(),
        std::env::current_dir()?,
    )?;
    let cfg = resolve_config(&ws)?;

    let wiki_dir = pages_dir(&ws, &cfg.wiki.pages_dir);
    if !wiki_dir.exists() {
        if args.json {
            println!(
                "{}",
                serde_json::to_string(&TreeOutput { entries: vec![] })?
            );
        } else {
            println!("(empty)");
        }
        return Ok(());
    }

    let mut entries = Vec::new();

    for entry in crate::core::workspace::walk_pages(&wiki_dir, &cfg.wiki.exclude_dirs) {
        let entry = entry.map_err(|e| anyhow::anyhow!(e))?;
        if entry.path().extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        if !crate::core::workspace::is_wiki_page_entry(&ws, entry.path()) {
            continue;
        }
        let rel = entry
            .path()
            .strip_prefix(&ws)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");

        let slug = entry
            .path()
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        let content = std::fs::read_to_string(entry.path()).unwrap_or_default();
        // Resilience: skip pages with unparseable frontmatter rather
        // than failing the whole `wiki tree` listing.
        let Ok(parsed) = crate::core::markdown::parse_frontmatter(&content) else {
            continue;
        };
        let title = parsed.frontmatter.as_ref().and_then(|fm| fm.title.clone());

        let tags = parsed
            .frontmatter
            .as_ref()
            .map(|fm| fm.tags.clone())
            .unwrap_or_default();

        // Check if embedded
        let embedded = if let Ok(emb) =
            crate::core::embeddings::EmbeddingsFile::read_from(&ws.join("embeddings.jsonl"))
        {
            emb.pages.iter().any(|p| p.path == rel)
        } else {
            false
        };

        entries.push(TreeEntry {
            slug,
            path: rel,
            title,
            tags,
            embedded,
        });
    }

    entries.sort_by(|a, b| a.slug.cmp(&b.slug));

    if entries.is_empty() {
        if args.json {
            println!(
                "{}",
                serde_json::to_string(&TreeOutput { entries: vec![] })?
            );
        } else {
            println!("(empty)");
        }
        return Ok(());
    }

    if args.json {
        println!("{}", serde_json::to_string_pretty(&TreeOutput { entries })?);
    } else {
        for e in &entries {
            let tag_str = if e.tags.is_empty() {
                String::new()
            } else {
                format!(" [{}]", e.tags.join(", "))
            };
            let emb_marker = if e.embedded { " ✓" } else { "" };
            let title_str = e.title.as_deref().unwrap_or("");
            println!("{}  {}{}{}", e.slug, title_str, tag_str, emb_marker);
        }
    }

    Ok(())
}
