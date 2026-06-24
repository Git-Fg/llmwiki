use crate::core::config::{resolve_api_key, resolve_config, validate_or_error};
use crate::core::embeddings::{cosine_similarity, EmbeddingsFile};
use crate::core::nim::NimClient;
use crate::core::workspace::discover_workspace;
use crate::error::WikiError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
    let ws = discover_workspace(
        args.workspace.clone(),
        args.wiki.as_deref(),
        std::env::var("WIKI_WORKSPACE").ok().map(PathBuf::from),
        std::env::var("WIKI_ACTIVE").ok().as_deref(),
        std::env::current_dir()?,
    )?;
    let mut cfg = resolve_config(&ws)?;
    validate_or_error(&cfg)?;
    if let Ok(base_url) = std::env::var("WIKI_NIM_BASE_URL") {
        cfg.nim.base_url = base_url;
    }
    let model = args.model.unwrap_or(cfg.nim.embed_model.clone());
    let llm_model = args
        .llm_model
        .unwrap_or_else(|| "meta/llama-3.1-70b-instruct".into());

    let jsonl_path = ws.join("embeddings.jsonl");
    let emb_file = EmbeddingsFile::read_from(&jsonl_path)?;
    if emb_file.pages.is_empty() {
        return Err(WikiError::NoEmbeddings);
    }

    let api_key = resolve_api_key(&cfg.nim);
    let client = NimClient::new(cfg.nim.base_url.clone(), api_key.clone())
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
    let context = context_parts.join("\n");

    let system =
        "You are answering questions about the user's personal knowledge base. Use ONLY the provided context. Cite with [^N] footnote markers.";
    let user = format!(
        "Context:\n{}\n\nQuestion: {}\n\nAnswer with citations.",
        context, args.question
    );

    let chat = ChatRequest {
        model: &llm_model,
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

    let url = format!("{}/v1/chat/completions", cfg.nim.base_url);
    let resp = client
        .http()
        .post(&url)
        .bearer_auth(&api_key)
        .json(&chat)
        .send()
        .await
        .map_err(|e| WikiError::NimUnreachable(e.to_string()))?;
    let parsed: ChatResponse = resp.json().await.map_err(|e| WikiError::Other(e.into()))?;
    let answer = parsed
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .unwrap_or_default();

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
