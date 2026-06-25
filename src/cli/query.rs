use crate::core::config::{resolve_api_key, resolve_config, validate_or_error};
use crate::core::embeddings::{cosine_similarity, EmbeddingsFile};
use crate::core::nim::NimClient;
use crate::core::registry::Registry;
use crate::core::workspace::discover_workspace;
use crate::error::WikiError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub struct QueryArgs {
    pub workspace: Option<PathBuf>,
    pub wiki: Option<String>,
    pub question: String,
    pub top_k: usize,
    pub model: Option<String>,
    pub llm_model: Option<String>,
    pub no_citations: bool,
    pub json: bool,
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    max_tokens: u32,
    temperature: f32,
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessageOwned,
}

#[derive(Deserialize)]
struct ChatMessageOwned {
    content: String,
}

pub async fn run(args: QueryArgs) -> Result<(), WikiError> {
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

async fn run_single(ws: &Path, args: QueryArgs) -> Result<(), WikiError> {
    let mut cfg = resolve_config(ws)?;
    validate_or_error(&cfg)?;
    if let Ok(base_url) = std::env::var("WIKI_NIM_BASE_URL") {
        cfg.nim.base_url = base_url;
    }
    let model = args.model.unwrap_or_else(|| cfg.nim.embed_model.clone());
    let llm_model = args
        .llm_model
        .clone()
        .unwrap_or_else(|| "meta/llama-3.1-70b-instruct".into());

    let jsonl_path = ws.join("embeddings.jsonl");
    let emb_file = EmbeddingsFile::read_from(&jsonl_path)?;
    if emb_file.pages.is_empty() {
        return Err(WikiError::NoEmbeddings);
    }

    let api_key = resolve_api_key(&cfg.nim);
    let client = NimClient::with_timeout(
        cfg.nim.base_url.clone(),
        api_key.clone(),
        cfg.nim.request_timeout_secs,
    )
    .with_max_attempts(cfg.nim.retry.max_attempts)
    .with_backoff_ms(cfg.nim.retry.backoff_ms);

    let query_vec = client
        .embed(&[args.question.as_str()], &model, "query")
        .await?;
    let q = &query_vec[0];

    let mut scored: Vec<(String, f32)> = vec![];
    for page in &emb_file.pages {
        if page.model != model {
            continue;
        }
        for chunk in &page.chunks {
            let score = cosine_similarity(q, &chunk.embedding);
            if score >= 0.2 {
                scored.push((page.path.clone(), score));
            }
        }
    }
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(args.top_k);

    // Build context from matching pages
    let mut context_parts: Vec<String> = vec![];
    let mut citation_paths: Vec<String> = vec![];
    for (i, (path, _score)) in scored.iter().enumerate() {
        let full_path = ws.join(path);
        let content = std::fs::read_to_string(&full_path).unwrap_or_default();
        let preview: String = content.chars().take(500).collect();
        context_parts.push(format!("[{}] {}:\n{}\n", i + 1, path, preview));
        citation_paths.push(path.clone());
    }

    let answer = call_llm(
        &client,
        &api_key,
        &cfg.nim.base_url,
        &llm_model,
        &context_parts.join("\n"),
        &args.question,
    )
    .await?;

    if args.json {
        let out = serde_json::json!({
            "question": args.question,
            "answer": answer,
            "citations": citation_paths,
        });
        println!("{}", serde_json::to_string_pretty(&out).unwrap());
    } else {
        println!("\n{answer}\n");
        if !args.no_citations && !citation_paths.is_empty() {
            println!("\nSources:");
            for (i, p) in citation_paths.iter().enumerate() {
                println!("  [{}] {}", i + 1, p);
            }
        }
    }
    Ok(())
}

/// Fleet query: search across all wikis with embeddings, then pass the
/// merged top-k context to a single LLM call. Citations are tagged with
/// the source wiki alias (`wiki/path`).
async fn run_fleet(args: QueryArgs) -> Result<(), WikiError> {
    let reg = Registry::discover()?;
    let cfg = match reg.entries.first() {
        Some(entry) => resolve_config(&entry.path).unwrap_or_default(),
        None => crate::core::config::Config::default(),
    };
    let mut cfg = cfg;
    if let Ok(base_url) = std::env::var("WIKI_NIM_BASE_URL") {
        cfg.nim.base_url = base_url;
    }
    let model = args
        .model
        .clone()
        .unwrap_or_else(|| cfg.nim.embed_model.clone());
    let llm_model = args
        .llm_model
        .clone()
        .unwrap_or_else(|| "meta/llama-3.1-70b-instruct".into());

    let api_key = resolve_api_key(&cfg.nim);
    let client = NimClient::with_timeout(
        cfg.nim.base_url.clone(),
        api_key.clone(),
        cfg.nim.request_timeout_secs,
    )
    .with_max_attempts(cfg.nim.retry.max_attempts)
    .with_backoff_ms(cfg.nim.retry.backoff_ms);

    let query_vec = client
        .embed(&[args.question.as_str()], &model, "query")
        .await?;
    let q = &query_vec[0];

    // Collect scored hits across all wikis with embeddings.
    // Each hit carries its wiki alias + base path for content retrieval.
    let mut all_scored: Vec<(String, f32, String, PathBuf)> = vec![];
    let mut wikis_searched = 0usize;

    for entry in &reg.entries {
        let jsonl = entry.path.join("embeddings.jsonl");
        if !jsonl.is_file() {
            continue;
        }
        let Ok(emb_file) = EmbeddingsFile::read_from(&jsonl) else {
            continue;
        };
        wikis_searched += 1;
        for page in &emb_file.pages {
            if page.model != model {
                continue;
            }
            for chunk in &page.chunks {
                let score = cosine_similarity(q, &chunk.embedding);
                if score >= 0.2 {
                    all_scored.push((
                        page.path.clone(),
                        score,
                        entry.alias.clone(),
                        entry.path.clone(),
                    ));
                }
            }
        }
    }

    if all_scored.is_empty() {
        return Err(WikiError::NoEmbeddings);
    }

    all_scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    all_scored.truncate(args.top_k);

    // Build context from matching pages — read from the correct wiki
    let mut context_parts: Vec<String> = vec![];
    let mut citation_paths: Vec<String> = vec![];
    for (i, (path, _score, wiki, base)) in all_scored.iter().enumerate() {
        let full_path = base.join(path);
        let content = std::fs::read_to_string(&full_path).unwrap_or_default();
        let preview: String = content.chars().take(500).collect();
        context_parts.push(format!("[{}] [{wiki}] {}:\n{}\n", i + 1, path, preview));
        citation_paths.push(format!("{wiki}/{path}"));
    }

    let answer = call_llm(
        &client,
        &api_key,
        &cfg.nim.base_url,
        &llm_model,
        &context_parts.join("\n"),
        &args.question,
    )
    .await?;

    if args.json {
        let out = serde_json::json!({
            "question": args.question,
            "answer": answer,
            "fleet": true,
            "wikis_searched": wikis_searched,
            "citations": citation_paths,
        });
        println!("{}", serde_json::to_string_pretty(&out).unwrap());
    } else {
        println!("\n{answer}\n");
        if !args.no_citations && !citation_paths.is_empty() {
            println!("\nSources (wiki/path):");
            for (i, p) in citation_paths.iter().enumerate() {
                println!("  [{}] {}", i + 1, p);
            }
        }
    }
    Ok(())
}

/// Shared LLM call used by both single-wiki and fleet query.
async fn call_llm(
    client: &NimClient,
    api_key: &str,
    base_url: &str,
    llm_model: &str,
    context: &str,
    question: &str,
) -> Result<String, WikiError> {
    let system =
        "You are answering questions about the user's personal knowledge base. Use ONLY the provided context. Cite with [^N] footnote markers.";
    let user = format!("Context:\n{context}\n\nQuestion: {question}\n\nAnswer with citations.");

    let chat = ChatRequest {
        model: llm_model,
        messages: vec![
            ChatMessage {
                role: "system",
                content: system,
            },
            ChatMessage {
                role: "user",
                content: &user,
            },
        ],
        max_tokens: 1024,
        temperature: 0.2,
    };

    let url = format!("{base_url}/v1/chat/completions");
    let resp = client
        .http()
        .post(&url)
        .bearer_auth(api_key)
        .json(&chat)
        .send()
        .await
        .map_err(|e| WikiError::NimUnreachable(e.to_string()))?;
    let parsed: ChatResponse = resp.json().await.map_err(|e| WikiError::Other(e.into()))?;
    Ok(parsed
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .unwrap_or_default())
}
