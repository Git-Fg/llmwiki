use std::collections::HashSet;
use std::path::PathBuf;

use crate::core::config::resolve_config;
use crate::core::embeddings::EmbeddingsFile;
use crate::core::registry::Registry;
use crate::core::workspace::{discover_workspace, pages_dir};
use crate::error::WikiError;

pub struct StatusArgs {
    pub workspace: Option<PathBuf>,
    pub wiki: Option<String>,
    /// Loop over every registered alias and print a one-line summary
    /// per wiki. Exits non-zero if any sub-call failed. Used for
    /// fleet-wide health checks — the multi-wiki equivalent of
    /// `llmwiki-cli status`.
    pub all: bool,
    pub json: bool,
}

pub fn run(args: StatusArgs) -> Result<(), WikiError> {
    if args.all {
        return run_fleet(args.json);
    }
    run_single(args)
}

fn run_single(args: StatusArgs) -> Result<(), WikiError> {
    let ws = discover_workspace(
        args.workspace,
        args.wiki.as_deref(),
        std::env::var("WIKI_WORKSPACE").ok().map(PathBuf::from),
        std::env::var("WIKI_ACTIVE").ok().as_deref(),
        std::env::current_dir()?,
    )?;
    let cfg = resolve_config(&ws)?;
    let stats = compute_stats(&ws, &cfg.wiki.pages_dir, &cfg.wiki.exclude_dirs)?;

    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "workspace": ws.display().to_string(),
                "pages": stats.page_count,
                "embedded_pages": stats.embedded_pages,
                "embedding_chunks": stats.total_chunks,
                "raw_sources": stats.raw_count,
            }))?
        );
    } else {
        println!("\nWiki: {}", ws.display());
        println!("  Pages: {}", stats.page_count);
        println!(
            "  Embedded: {} ({} chunks)",
            stats.embedded_pages, stats.total_chunks
        );
        println!("  Raw sources: {}", stats.raw_count);
    }
    Ok(())
}

/// Loop over every registered alias, compute the same stats as
/// `run_single`, and print a one-line summary per wiki. Exits 0 if
/// every wiki was readable; exits 2 if any wiki failed.
///
/// Output shape:
/// - Human: `mevin   pages=12  embedded=12(48chunks)  raw=3   OK`
/// - JSON:  `{ "wikis": [...], "failures": 0 }`
///
/// Used by agents that want a single call to answer "is the whole
/// wiki fleet healthy?" — analogous to `agent-browser session list`
/// but per-wiki.
fn run_fleet(json: bool) -> Result<(), WikiError> {
    let reg = Registry::discover()?;

    if reg.entries.is_empty() {
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "wikis": [],
                    "failures": 0,
                    "note": "registry has no entries; nothing to do",
                }))?
            );
        } else {
            println!("No wikis registered. Run `llmwiki-cli init <path>` to add one.");
        }
        return Ok(());
    }

    let mut failures: u32 = 0;
    let mut entries: Vec<serde_json::Value> = Vec::new();

    for entry in &reg.entries {
        // Best-effort: surface each failure but keep going so the agent
        // sees the whole fleet in one call. A wiki whose path no longer
        // exists is a real failure, not a reason to abort the loop.
        match resolve_config(&entry.path) {
            Ok(cfg) => {
                let stats = compute_stats(&entry.path, &cfg.wiki.pages_dir, &cfg.wiki.exclude_dirs);
                match stats {
                    Ok(s) => {
                        if json {
                            entries.push(serde_json::json!({
                                "alias": entry.alias,
                                "path": entry.path,
                                "pages": s.page_count,
                                "embedded_pages": s.embedded_pages,
                                "embedding_chunks": s.total_chunks,
                                "raw_sources": s.raw_count,
                                "ok": true,
                            }));
                        } else {
                            println!(
                                "{:<20}  pages={:<5}  embedded={:<5}({} chunks)  raw={}",
                                entry.alias,
                                s.page_count,
                                s.embedded_pages,
                                s.total_chunks,
                                s.raw_count
                            );
                        }
                    }
                    Err(e) => {
                        failures += 1;
                        if json {
                            entries.push(serde_json::json!({
                                "alias": entry.alias,
                                "path": entry.path,
                                "ok": false,
                                "error": e.to_string(),
                            }));
                        } else {
                            println!("{:<20}  ERROR: {e}", entry.alias);
                        }
                    }
                }
            }
            Err(e) => {
                failures += 1;
                if json {
                    entries.push(serde_json::json!({
                        "alias": entry.alias,
                        "path": entry.path,
                        "ok": false,
                        "error": format!("resolve_config failed: {e}"),
                    }));
                } else {
                    println!("{:<20}  ERROR: {e}", entry.alias);
                }
            }
        }
    }

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "wikis": entries,
                "failures": failures,
            }))?
        );
    } else if failures > 0 {
        println!();
        println!("{failures} wiki(s) failed.");
    }

    if failures > 0 {
        // Use a distinct exit code (2) so agents can distinguish
        // "fleet had failures" from "this command itself errored" (1)
        // and from "all good" (0).
        std::process::exit(2);
    }
    Ok(())
}

/// Stats for a single wiki. `embedded_pages` and `total_chunks` come
/// from `embeddings.jsonl`; `page_count` from a walk over the pages
/// dir (using the wiki's `pages_dir` + `exclude_dirs`); `raw_count`
/// from a walk over the raw/ dir.
struct WikiStats {
    page_count: usize,
    embedded_pages: usize,
    total_chunks: usize,
    raw_count: usize,
}

fn compute_stats(
    ws: &std::path::Path,
    pages_dir_config: &str,
    exclude_dirs: &[String],
) -> Result<WikiStats, WikiError> {
    let mut page_count = 0;
    let wiki_dir = pages_dir(ws, pages_dir_config);
    if wiki_dir.exists() {
        for entry in crate::core::workspace::walk_pages(&wiki_dir, exclude_dirs) {
            let entry = entry.map_err(|e| anyhow::anyhow!(e))?;
            if entry.path().extension().and_then(|s| s.to_str()) != Some("md") {
                continue;
            }
            if !crate::core::workspace::is_wiki_page_entry(ws, entry.path()) {
                continue;
            }
            page_count += 1;
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

    Ok(WikiStats {
        page_count,
        embedded_pages: embedded_pages.len(),
        total_chunks,
        raw_count,
    })
}
