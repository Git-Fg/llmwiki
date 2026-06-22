use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::core::config::resolve_config;
use crate::core::embeddings::EmbeddingsFile;
use crate::core::markdown::{extract_wikilinks, parse_frontmatter};
use crate::core::workspace::discover_workspace;
use crate::error::WikiError;

pub struct LsArgs {
    pub workspace: Option<PathBuf>,
    pub pages: bool,
    pub raw: bool,
    pub embed: bool,
    pub links: bool,
    pub config: bool,
    pub json: bool,
}

#[derive(serde::Serialize)]
struct PageEntry {
    path: String,
    title: String,
    tags: Vec<String>,
    outbound: usize,
    inbound: usize,
    embedded: bool,
    chunks: usize,
    lines: usize,
}

#[derive(serde::Serialize)]
struct RawEntry {
    path: String,
    file_type: String,
    sha256: String,
    ingested: String,
    bytes: u64,
    frontmatter_ok: bool,
}

#[derive(serde::Serialize)]
struct EmbedEntry {
    page: String,
    chunks: usize,
    dim: usize,
}

#[derive(serde::Serialize)]
struct LinkEntry {
    from: String,
    to: String,
}

#[derive(serde::Serialize)]
struct ConfigEntry {
    key: String,
    value: String,
}

#[derive(serde::Serialize)]
struct LsOutput {
    pages: Option<Vec<PageEntry>>,
    raw: Option<Vec<RawEntry>>,
    embed: Option<Vec<EmbedEntry>>,
    links: Option<Vec<LinkEntry>>,
    config: Option<Vec<ConfigEntry>>,
}

pub fn run(args: LsArgs) -> Result<(), WikiError> {
    let ws = discover_workspace(
        args.workspace.clone(),
        std::env::var("WIKI_WORKSPACE").ok().map(PathBuf::from),
        std::env::current_dir()?,
    )?;

    let show_all = !args.pages && !args.raw && !args.embed && !args.links && !args.config;
    let cfg = resolve_config(&ws)?;

    // --- pages ---
    let pages = if show_all || args.pages {
        Some(build_page_entries(&ws, &cfg)?)
    } else {
        None
    };

    // --- raw ---
    let raw = if show_all || args.raw {
        Some(build_raw_entries(&ws)?)
    } else {
        None
    };

    // --- embed ---
    let embed = if show_all || args.embed {
        Some(build_embed_entries(&ws)?)
    } else {
        None
    };

    // --- links ---
    let links = if args.links {
        Some(build_link_entries(&ws)?)
    } else {
        None
    };

    // --- config ---
    let config = if show_all || args.config {
        Some(build_config_entries(&cfg))
    } else {
        None
    };

    let output = LsOutput {
        pages,
        raw,
        embed,
        links,
        config,
    };

    if args.json {
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        print_human(&output);
    }

    Ok(())
}

fn build_page_entries(
    ws: &Path,
    _cfg: &crate::core::config::Config,
) -> Result<Vec<PageEntry>, WikiError> {
    let emb = EmbeddingsFile::read_from(&ws.join("embeddings.jsonl"))?;
    let embedded_pages: HashMap<String, &crate::core::embeddings::PageEmbedding> =
        emb.pages.iter().map(|p| (p.path.clone(), p)).collect();

    let wiki_dir = ws.join("wiki");
    if !wiki_dir.exists() {
        return Ok(vec![]);
    }

    let mut all_slugs: Vec<String> = vec![];
    let mut page_files: Vec<PathBuf> = vec![];
    for entry in walkdir::WalkDir::new(&wiki_dir) {
        let entry = entry.map_err(|e| anyhow::anyhow!(e))?;
        if entry.path().extension().and_then(|s| s.to_str()) == Some("md") {
            let rel = entry
                .path()
                .strip_prefix(ws)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            let slug = entry
                .path()
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            all_slugs.push(slug);
            page_files.push(entry.path().to_path_buf());
            let _ = rel; // keep for debugging but not used now
        }
    }

    // compute inbound link counts
    let mut inbound: HashMap<String, usize> = HashMap::new();
    for path in &page_files {
        let content = std::fs::read_to_string(path).unwrap_or_default();
        if let Ok(parsed) = parse_frontmatter(&content) {
            let links = extract_wikilinks(&parsed.body);
            for link in &links {
                let target_slug = link.split('/').next_back().unwrap_or(link).to_lowercase();
                for candidate in &page_files {
                    let candidate_slug = candidate
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    if candidate_slug == target_slug && candidate != path {
                        let rel = candidate
                            .strip_prefix(ws)
                            .unwrap()
                            .to_string_lossy()
                            .replace('\\', "/");
                        *inbound.entry(rel).or_insert(0) += 1;
                    }
                }
            }
        }
    }

    let mut entries = vec![];
    for path in &page_files {
        let rel = path
            .strip_prefix(ws)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let content = std::fs::read_to_string(path).unwrap_or_default();
        let parsed = parse_frontmatter(&content)?;
        let outbound = extract_wikilinks(&parsed.body).len();
        let inbound_count = inbound.get(&rel).copied().unwrap_or(0);
        let lines = content.lines().count();

        let title = parsed
            .frontmatter
            .as_mapping()
            .and_then(|m| m.get("title"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let tags = parsed
            .frontmatter
            .as_mapping()
            .and_then(|m| m.get("tags"))
            .and_then(|v| v.as_sequence())
            .map(|seq| {
                seq.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let emb_info = embedded_pages.get(&rel);
        entries.push(PageEntry {
            path: rel,
            title,
            tags,
            outbound,
            inbound: inbound_count,
            embedded: emb_info.is_some(),
            chunks: emb_info.map(|e| e.chunks.len()).unwrap_or(0),
            lines,
        });
    }

    entries.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(entries)
}

fn build_raw_entries(ws: &Path) -> Result<Vec<RawEntry>, WikiError> {
    let raw_dir = ws.join("raw");
    if !raw_dir.exists() {
        return Ok(vec![]);
    }

    let mut entries = vec![];
    for entry in walkdir::WalkDir::new(&raw_dir) {
        let entry = entry.map_err(|e| anyhow::anyhow!(e))?;
        if !entry.file_type().is_file() {
            continue;
        }
        let rel = entry
            .path()
            .strip_prefix(ws)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let content = std::fs::read_to_string(entry.path()).unwrap_or_default();
        let parsed = parse_frontmatter(&content)?;
        let fm_ok = parsed.frontmatter.as_mapping().is_some();

        let file_type = entry
            .path()
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        let (sha256, ingested) = if let Some(fm) = parsed.frontmatter.as_mapping() {
            let sha = fm
                .get("sha256")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let ing = fm
                .get("ingested")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            (sha, ing)
        } else {
            (String::new(), String::new())
        };

        let bytes = std::fs::metadata(entry.path())
            .map(|m| m.len())
            .unwrap_or(0);

        entries.push(RawEntry {
            path: rel,
            file_type,
            sha256,
            ingested,
            bytes,
            frontmatter_ok: fm_ok,
        });
    }

    entries.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(entries)
}

fn build_embed_entries(ws: &Path) -> Result<Vec<EmbedEntry>, WikiError> {
    let emb = EmbeddingsFile::read_from(&ws.join("embeddings.jsonl"))?;
    let mut entries: Vec<EmbedEntry> = emb
        .pages
        .iter()
        .map(|p| EmbedEntry {
            page: p.path.clone(),
            chunks: p.chunks.len(),
            dim: p.chunks.first().map(|c| c.embedding.len()).unwrap_or(0),
        })
        .collect();
    entries.sort_by(|a, b| a.page.cmp(&b.page));
    Ok(entries)
}

fn build_link_entries(ws: &Path) -> Result<Vec<LinkEntry>, WikiError> {
    let wiki_dir = ws.join("wiki");
    if !wiki_dir.exists() {
        return Ok(vec![]);
    }

    let mut entries = vec![];
    for entry in walkdir::WalkDir::new(&wiki_dir) {
        let entry = entry.map_err(|e| anyhow::anyhow!(e))?;
        if entry.path().extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        let from = entry
            .path()
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        let content = std::fs::read_to_string(entry.path()).unwrap_or_default();
        let parsed = parse_frontmatter(&content)?;
        let links = extract_wikilinks(&parsed.body);
        for link in &links {
            let to = link.split('/').next_back().unwrap_or(link).to_string();
            entries.push(LinkEntry {
                from: from.clone(),
                to,
            });
        }
    }

    entries.sort_by(|a, b| a.from.cmp(&b.from).then(a.to.cmp(&b.to)));
    Ok(entries)
}

fn build_config_entries(cfg: &crate::core::config::Config) -> Vec<ConfigEntry> {
    vec![
        ConfigEntry {
            key: "nim.embed_model".into(),
            value: cfg.nim.embed_model.clone(),
        },
        ConfigEntry {
            key: "nim.base_url".into(),
            value: cfg.nim.base_url.clone(),
        },
        ConfigEntry {
            key: "nim.api_key_env".into(),
            value: cfg.nim.api_key_env.clone(),
        },
        ConfigEntry {
            key: "nim.batch_size".into(),
            value: cfg.nim.batch_size.to_string(),
        },
        ConfigEntry {
            key: "nim.request_timeout_secs".into(),
            value: cfg.nim.request_timeout_secs.to_string(),
        },
        ConfigEntry {
            key: "wiki.default_chunk_tokens".into(),
            value: cfg.wiki.default_chunk_tokens.to_string(),
        },
        ConfigEntry {
            key: "wiki.chunk_overlap_tokens".into(),
            value: cfg.wiki.chunk_overlap_tokens.to_string(),
        },
        ConfigEntry {
            key: "wiki.min_chunk_tokens".into(),
            value: cfg.wiki.min_chunk_tokens.to_string(),
        },
        ConfigEntry {
            key: "wiki.require_wikilinks_min".into(),
            value: cfg.wiki.require_wikilinks_min.to_string(),
        },
        ConfigEntry {
            key: "config_version".into(),
            value: cfg.config_version.to_string(),
        },
    ]
}

fn print_human(output: &LsOutput) {
    if let Some(pages) = &output.pages {
        println!("\nPages ({}):\n", pages.len());
        for p in pages {
            let embedded_marker = if p.embedded {
                format!(" [{} chunks]", p.chunks)
            } else {
                " [not embedded]".into()
            };
            println!(
                "  {} — {}{}",
                p.path,
                if p.title.is_empty() {
                    "(no title)"
                } else {
                    &p.title
                },
                embedded_marker,
            );
            println!(
                "    lines:{}  out:{}  in:{}  tags:[{}]",
                p.lines,
                p.outbound,
                p.inbound,
                p.tags.join(", ")
            );
        }
    }

    if let Some(raw) = &output.raw {
        println!("\nRaw sources ({}):\n", raw.len());
        for r in raw {
            let sha_short = if r.sha256.len() > 12 {
                &r.sha256[..12]
            } else {
                &r.sha256
            };
            println!(
                "  {} — {}B, type:{}, fm:{}, sha:{}{}",
                r.path,
                r.bytes,
                r.file_type,
                if r.frontmatter_ok { "ok" } else { "missing" },
                if sha_short.is_empty() {
                    "none"
                } else {
                    sha_short
                },
                if r.ingested.is_empty() {
                    String::new()
                } else {
                    format!(", ingested:{}", r.ingested)
                },
            );
        }
    }

    if let Some(embed) = &output.embed {
        println!("\nEmbedded pages ({}):\n", embed.len());
        for e in embed {
            println!("  {} — {} chunks, dim {}", e.page, e.chunks, e.dim);
        }
    }

    if let Some(links) = &output.links {
        println!("\nWikilinks ({}):\n", links.len());
        for l in links {
            println!("  [[{}]] → [[{}]]", l.from, l.to);
        }
    }

    if let Some(config) = &output.config {
        println!("\nConfig:\n");
        for c in config {
            println!("  {}: {}", c.key, c.value);
        }
    }
}
