use std::path::{Path, PathBuf};

use crate::core::config::{resolve_api_key, resolve_config, validate_or_error};
use crate::core::embeddings::{cosine_similarity, EmbeddingsFile};
use crate::core::nim::NimClient;
use crate::core::registry::Registry;
use crate::core::workspace::discover_workspace;
use crate::error::WikiError;

pub struct SearchArgs {
    pub workspace: Option<PathBuf>,
    pub wiki: Option<String>,
    pub query: String,
    pub top_k: usize,
    pub threshold: f32,
    pub model: Option<String>,
    pub json: bool,
}

/// One scored chunk hit. `(wiki_alias, path, score, chunk_index, start, end)`.
/// `wiki_alias` is empty in single-wiki mode.
pub type ScoredChunk = (String, String, f32, usize, usize, usize);

/// Score every chunk in `emb_file` whose model matches `model` against the
/// query embedding, returning only those above `threshold`. Pulled out of
/// `run()` so the same loop is used by both single-wiki and fleet paths.
fn score_chunks(
    emb_file: &EmbeddingsFile,
    query_vec: &[f32],
    model: &str,
    threshold: f32,
    wiki_alias: &str,
) -> Vec<ScoredChunk> {
    let mut scored: Vec<ScoredChunk> = vec![];
    for page in &emb_file.pages {
        if page.model != model {
            continue;
        }
        for (i, chunk) in page.chunks.iter().enumerate() {
            let score = cosine_similarity(query_vec, &chunk.embedding);
            if score >= threshold {
                scored.push((
                    wiki_alias.to_string(),
                    page.path.clone(),
                    score,
                    i,
                    chunk.start,
                    chunk.end,
                ));
            }
        }
    }
    scored
}

pub async fn run(args: SearchArgs) -> Result<(), WikiError> {
    // Try the normal single-wiki resolution. If it fails AND the
    // user didn't try to pin a wiki/workspace explicitly, fall back
    // to fleet search across every registered wiki that has
    // embeddings. This matches the "just works" UX the user
    // expects — agents running blind shouldn't have to know
    // which wiki they're in.
    let workspace_result = discover_workspace(
        args.workspace.clone(),
        args.wiki.as_deref(),
        std::env::var("WIKI_WORKSPACE").ok().map(PathBuf::from),
        std::env::var("WIKI_ACTIVE").ok().as_deref(),
        std::env::current_dir()?,
    );

    match workspace_result {
        Ok(ws) => run_single(&ws, args).await,
        Err(e) => {
            // If the user explicitly passed --workspace/--wiki, respect
            // the error (they asked for something specific that we
            // couldn't satisfy). Only fall back to fleet on the
            // no-explicit-signal case.
            let has_explicit = args.workspace.is_some()
                || args.wiki.is_some()
                || std::env::var("WIKI_WORKSPACE").is_ok()
                || std::env::var("WIKI_ACTIVE").is_ok();
            if has_explicit {
                return Err(e);
            }
            run_fleet(args).await
        }
    }
}

async fn run_single(ws: &Path, args: SearchArgs) -> Result<(), WikiError> {
    let mut cfg = resolve_config(ws)?;
    validate_or_error(&cfg)?;
    if let Ok(base_url) = std::env::var("WIKI_NIM_BASE_URL") {
        cfg.nim.base_url = base_url;
    }

    let model = args.model.unwrap_or_else(|| cfg.nim.embed_model.clone());
    let jsonl_path = ws.join("embeddings.jsonl");
    let emb_file = EmbeddingsFile::read_from(&jsonl_path)?;
    if emb_file.pages.is_empty() {
        return Err(WikiError::NoEmbeddings);
    }

    let api_key = resolve_api_key(&cfg.nim);
    let client = NimClient::with_timeout(
        cfg.nim.base_url.clone(),
        api_key,
        cfg.nim.request_timeout_secs,
    )
    .with_max_attempts(cfg.nim.retry.max_attempts)
    .with_backoff_ms(cfg.nim.retry.backoff_ms);
    let query_vec = client
        .embed(&[args.query.as_str()], &model, "query")
        .await?;
    let q = query_vec
        .first()
        .ok_or_else(|| WikiError::NimUnreachable("empty embedding response".into()))?;

    let mut scored = score_chunks(&emb_file, q, &model, args.threshold, "");
    scored.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(args.top_k);

    if args.json {
        let json_results: Vec<serde_json::Value> = scored
            .iter()
            .map(|(_, p, s, i, start, end)| {
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
                "fleet": false,
                "results": json_results,
            }))
            .unwrap()
        );
    } else {
        println!("\n✓ {} result(s) for \"{}\":\n", scored.len(), args.query);
        for (_, path, score, _idx, _start, _end) in &scored {
            println!("  [{score:.3}] {path}\n");
        }
    }
    Ok(())
}

/// Fleet search: embed the query once, score against every registered
/// wiki's embeddings, merge by score, tag each hit with its source
/// wiki alias. Used as an implicit fallback when `discover_workspace`
/// can't resolve a single workspace AND the user didn't pin one.
///
/// Performance: 1 NIM embed call + N cosine-similarity passes, where N
/// is the number of wikis with `embeddings.jsonl` on disk. Wikis
/// without embeddings are skipped silently — most test fixtures fall
/// into this bucket. The loop is bounded by O(total_chunks) and runs
/// on a single thread; under 1s for typical fleets.
async fn run_fleet(args: SearchArgs) -> Result<(), WikiError> {
    let reg = Registry::discover()?;

    // Resolve a config to discover the embed model + NIM base URL.
    // For fleet mode we use the FIRST entry's config (they all share
    // the same NIM model by convention); if that fails, fall back
    // to built-in defaults.
    let cfg = match reg.entries.first() {
        Some(entry) => resolve_config(&entry.path).unwrap_or_default(),
        None => crate::core::config::Config::default(),
    };
    // Apply the same WIKI_NIM_BASE_URL override that single-wiki mode uses.
    // Without this, fleet mode would always hit the production NIM endpoint
    // even when the user set the env override for testing/debugging.
    let mut cfg = cfg;
    if let Ok(base_url) = std::env::var("WIKI_NIM_BASE_URL") {
        cfg.nim.base_url = base_url;
    }
    let model = args
        .model
        .clone()
        .unwrap_or_else(|| cfg.nim.embed_model.clone());

    let api_key = resolve_api_key(&cfg.nim);
    let client = NimClient::with_timeout(
        cfg.nim.base_url.clone(),
        api_key,
        cfg.nim.request_timeout_secs,
    )
    .with_max_attempts(cfg.nim.retry.max_attempts)
    .with_backoff_ms(cfg.nim.retry.backoff_ms);

    // Skip wikis without embeddings.jsonl. Cheap stat() check — most
    // test fixtures / empty wikis are filtered out instantly.
    let candidates: Vec<&crate::core::registry::WikiEntry> = reg
        .entries
        .iter()
        .filter(|e| e.path.join("embeddings.jsonl").is_file())
        .collect();

    if candidates.is_empty() {
        return Err(WikiError::NoEmbeddings);
    }

    let query_vec = client
        .embed(&[args.query.as_str()], &model, "query")
        .await?;
    let q = query_vec
        .first()
        .ok_or_else(|| WikiError::NimUnreachable("empty embedding response".into()))?;

    let mut all_scored: Vec<ScoredChunk> = Vec::new();
    let mut wikis_searched: Vec<String> = Vec::new();
    let mut wikis_skipped = 0usize;

    for entry in &reg.entries {
        let jsonl = entry.path.join("embeddings.jsonl");
        if !jsonl.is_file() {
            wikis_skipped += 1;
            continue;
        }
        // EmbeddingsFile::read_from returns empty pages vec if file
        // is missing — we already filtered above, but be defensive.
        let Ok(emb_file) = EmbeddingsFile::read_from(&jsonl) else {
            wikis_skipped += 1;
            continue;
        };
        wikis_searched.push(entry.alias.clone());
        all_scored.extend(score_chunks(
            &emb_file,
            q,
            &model,
            args.threshold,
            &entry.alias,
        ));
    }

    all_scored.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    all_scored.truncate(args.top_k);

    if args.json {
        let json_results: Vec<serde_json::Value> = all_scored
            .iter()
            .map(|(wiki, p, s, i, start, end)| {
                serde_json::json!({
                    "wiki": wiki,
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
                "fleet": true,
                "wikis_searched": wikis_searched,
                "wikis_skipped": wikis_skipped,
                "results": json_results,
            }))
            .unwrap()
        );
    } else {
        println!(
            "\n✓ {} result(s) for \"{}\" (searched {} wiki(s)):\n",
            all_scored.len(),
            args.query,
            wikis_searched.len()
        );
        for (wiki, path, score, _idx, _start, _end) in &all_scored {
            if wiki.is_empty() {
                println!("  [{score:.3}] {path}\n");
            } else {
                println!("  [{wiki:<16} {score:.3}] {path}\n");
            }
        }
        if wikis_skipped > 0 && all_scored.is_empty() {
            println!(
                "(Skipped {} wiki(s) without embeddings.jsonl — run `llmwiki-cli embed` to populate them.)",
                wikis_skipped
            );
        }
    }
    Ok(())
}
