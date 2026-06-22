use std::path::PathBuf;

use crate::core::config::{resolve_api_key, resolve_config};
use crate::core::embeddings::{cosine_similarity, EmbeddingsFile};
use crate::core::nim::NimClient;
use crate::core::workspace::discover_workspace;
use crate::error::WikiError;

pub struct SearchArgs {
    pub workspace: Option<PathBuf>,
    pub query: String,
    pub top_k: usize,
    pub threshold: f32,
    pub model: Option<String>,
    pub json: bool,
}

pub async fn run(args: SearchArgs) -> Result<(), WikiError> {
    let ws = discover_workspace(
        args.workspace.clone(),
        std::env::var("WIKI_WORKSPACE").ok().map(PathBuf::from),
        std::env::current_dir()?,
    )?;
    let mut cfg = resolve_config(&ws)?;
    if let Ok(base_url) = std::env::var("WIKI_NIM_BASE_URL") {
        cfg.nim.base_url = base_url;
    }

    let model = args.model.unwrap_or(cfg.nim.embed_model.clone());
    let jsonl_path = ws.join("embeddings.jsonl");
    let emb_file = EmbeddingsFile::read_from(&jsonl_path)?;
    if emb_file.pages.is_empty() {
        return Err(WikiError::NoEmbeddings);
    }

    let api_key = resolve_api_key(&cfg.nim);
    let client = NimClient::new(cfg.nim.base_url.clone(), api_key)
        .with_max_attempts(cfg.nim.retry.max_attempts)
        .with_backoff_ms(cfg.nim.retry.backoff_ms);
    let query_vec = client
        .embed(&[args.query.as_str()], &model, "query")
        .await?;
    let q = query_vec
        .first()
        .ok_or_else(|| WikiError::NimUnreachable("empty embedding response".into()))?;

    let mut scored: Vec<(String, f32, usize, usize, usize)> = vec![];
    for page in &emb_file.pages {
        if page.model != model {
            continue;
        }
        for (i, chunk) in page.chunks.iter().enumerate() {
            let score = cosine_similarity(q, &chunk.embedding);
            if score >= args.threshold {
                scored.push((page.path.clone(), score, i, chunk.start, chunk.end));
            }
        }
    }
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(args.top_k);

    if args.json {
        let json_results: Vec<serde_json::Value> = scored
            .iter()
            .map(|(p, s, i, start, end)| {
                serde_json::json!({
                    "path": p,
                    "score": s,
                    "chunk_index": i,
                    "start": start,
                    "end": end,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "query": args.query,
                "model": model,
                "results": json_results,
            }))
            .unwrap()
        );
    } else {
        println!("\n✓ {} result(s) for \"{}\":\n", scored.len(), args.query);
        for (path, score, _idx, _start, _end) in &scored {
            println!("  [{:.3}] {}\n", score, path);
        }
    }
    Ok(())
}
