use thiserror::Error;

#[derive(Error, Debug)]
pub enum WikiError {
    #[error("config invalid at {path} line {line}: {message}")]
    ConfigInvalid {
        path: String,
        line: usize,
        message: String,
    },

    #[error("NVIDIA NIM API key not set. Set NVIDIA_NIM_API_KEY env var or add to shell rc.")]
    NimApiKeyMissing,

    #[error("NIM unreachable: {0}")]
    NimUnreachable(String),

    #[error(
        "workspace not found. Use --workspace <path>, set WIKI_WORKSPACE, or cd into a wiki folder"
    )]
    WorkspaceNotFound,

    #[error("wiki-root.toml not found in any of: {searched:?}{}", from_env.as_deref().unwrap_or_default())]
    WikiRootNotFound {
        searched: Vec<std::path::PathBuf>,
        from_env: Option<String>,
    },

    #[error("wiki alias '{alias}' not found in registry. Available: {available}")]
    AliasNotFound { alias: String, available: String },

    #[error("no embeddings yet. Run `wiki embed` first.")]
    NoEmbeddings,

    #[error("unknown topic '{0}'. Run `wiki skill list` for available topics.")]
    UnknownSkillTopic(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Yaml(#[from] serde_saphyr::Error),

    #[error(transparent)]
    Http(#[from] reqwest::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
