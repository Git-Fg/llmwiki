use chrono::Utc;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

use crate::core::chunker::chunk_text;
use crate::core::config::{resolve_api_key, resolve_config};
use crate::core::embeddings::{ChunkEmbed, EmbeddingsFile, PageEmbedding};
use crate::core::models_registry::{load_registry, Role};
use crate::core::nim::NimClient;
use crate::core::workspace::discover_workspace;
use crate::error::WikiError;

pub struct EmbedArgs {
    pub workspace: Option<PathBuf>,
    pub model: Option<String>,
    pub dims: Option<usize>,
    pub skip_existing: bool,
    pub batch_size: Option<usize>,
}

pub async fn run(args: EmbedArgs) -> Result<(), WikiError> {
    let ws = discover_workspace(
        args.workspace.clone(),
        std::env::var("WIKI_WORKSPACE").ok().map(PathBuf::from),
        std::env::current_dir()?,
    )?;
    let mut cfg = resolve_config(&ws)?;
    if let Ok(base_url) = std::env::var("WIKI_NIM_BASE_URL") {
        cfg.nim.base_url = base_url;
    }

    let model_name = args.model.unwrap_or(cfg.nim.embed_model.clone());
    let model_info = load_registry()
        .into_iter()
        .find(|m| m.name == model_name && m.role == Role::Embed)
        .ok_or_else(|| {
            crate::error::WikiError::Other(anyhow::anyhow!("unknown embed model: {}", model_name))
        })?;
    let api_key = resolve_api_key(&cfg.nim);
    let client = NimClient::new(cfg.nim.base_url.clone(), api_key)
        .with_max_attempts(cfg.nim.retry.max_attempts)
        .with_backoff_ms(cfg.nim.retry.backoff_ms);

    let wiki_dir = ws.join("wiki");
    let jsonl_path = ws.join("embeddings.jsonl");
    let mut emb_file = EmbeddingsFile::read_from(&jsonl_path)?;

    let mut pages: Vec<PathBuf> = vec![];
    if wiki_dir.exists() {
        for entry in walkdir::WalkDir::new(&wiki_dir) {
            let e = entry.map_err(|e| WikiError::Other(e.into()))?;
            if e.path().extension().and_then(|s| s.to_str()) == Some("md") {
                pages.push(e.path().to_path_buf());
            }
        }
    }
    pages.sort();

    let batch_size = args.batch_size.unwrap_or(cfg.nim.batch_size).max(1);
    let dim = args.dims.unwrap_or(model_info.dim);
    let mut updated = 0;

    for page_path in pages {
        let rel = page_path
            .strip_prefix(&ws)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let content = std::fs::read_to_string(&page_path)?;
        let digest = Sha256::digest(content.as_bytes());
        let sha = digest
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>();
        if args.skip_existing {
            if let Some(existing) = emb_file.find_page(&rel, &model_name) {
                if existing.sha256 == sha {
                    continue;
                }
            }
        }

        let chunks = chunk_text(
            &content,
            cfg.wiki.default_chunk_tokens,
            cfg.wiki.chunk_overlap_tokens,
            1,
        );
        if chunks.is_empty() {
            continue;
        }

        let mut embeddings = Vec::with_capacity(chunks.len());
        for chunk_batch in chunks.chunks(batch_size) {
            let texts: Vec<&str> = chunk_batch.iter().map(|c| c.content.as_str()).collect();
            embeddings.extend(client.embed(&texts, &model_name, "passage").await?);
        }

        let chunk_embs: Vec<ChunkEmbed> = chunks
            .iter()
            .zip(embeddings.iter())
            .map(|(c, e)| ChunkEmbed {
                start: c.start_char,
                end: c.start_char + c.content.len(),
                tokens: c.token_count,
                embedding: e.clone(),
            })
            .collect();

        emb_file.remove_page(&rel);
        emb_file.pages.push(PageEmbedding {
            path: rel.clone(),
            sha256: sha,
            model: model_name.clone(),
            dim,
            chunked: chunk_embs.len() > 1,
            chunks: chunk_embs,
            embedded_at: Utc::now().to_rfc3339(),
        });
        updated += 1;
    }

    emb_file.write_to(&jsonl_path)?;
    println!("✓ Embedded {} page(s)", updated);
    Ok(())
}
