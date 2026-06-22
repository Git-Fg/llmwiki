use std::fs;
use std::path::Path;

#[derive(schemars::JsonSchema)]
#[allow(dead_code)] // fields are consumed by schemars::schema_for!, not by Rust code
struct Config {
    #[schemars(description = "NIM API client configuration")]
    nim: NimConfig,
    #[schemars(description = "Wiki page chunking and lint settings")]
    wiki: WikiConfig,
    #[schemars(description = "Schema version of this config")]
    config_version: u32,
}

#[derive(schemars::JsonSchema)]
#[allow(dead_code)] // fields are consumed by schemars::schema_for!, not by Rust code
struct NimConfig {
    #[schemars(description = "NIM API base URL (no /v1 suffix)")]
    base_url: String,
    #[schemars(
        description = "Embedding model identifier (must be in the whitelisted NVIDIA NIM set)"
    )]
    embed_model: String,
    #[schemars(description = "Re-ranking model identifier (empty = disabled)")]
    rerank_model: String,
    #[schemars(description = "Override embedding dimension (empty = use model default)")]
    embed_dim_override: Option<usize>,
    #[schemars(description = "Env var name holding the NIM API key")]
    api_key_env: String,
    #[schemars(description = "Embedding request batch size (1+)")]
    batch_size: usize,
    #[schemars(description = "NIM request timeout in seconds")]
    request_timeout_secs: u64,
    #[schemars(description = "Retry policy for failed NIM calls")]
    retry: RetryConfig,
}

#[derive(schemars::JsonSchema)]
#[allow(dead_code)] // fields are consumed by schemars::schema_for!, not by Rust code
struct RetryConfig {
    #[schemars(description = "Maximum attempts per NIM call")]
    max_attempts: u32,
    #[schemars(description = "Backoff between retries in milliseconds")]
    backoff_ms: u64,
}

#[derive(schemars::JsonSchema)]
#[allow(dead_code)] // fields are consumed by schemars::schema_for!, not by Rust code
struct WikiConfig {
    #[schemars(description = "Default chunk size in tokens")]
    default_chunk_tokens: usize,
    #[schemars(description = "Chunk overlap in tokens (must be < default_chunk_tokens)")]
    chunk_overlap_tokens: usize,
    #[schemars(description = "Minimum chunk size in tokens (must be <= default_chunk_tokens)")]
    min_chunk_tokens: usize,
    #[schemars(description = "Require YAML frontmatter on every page")]
    require_frontmatter: bool,
    #[schemars(description = "Minimum wikilink count per page (0 = no minimum)")]
    require_wikilinks_min: usize,
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/skills/WIKI.md");
    println!("cargo:rerun-if-changed=src/skills/SETUP/SKILL.md");
    println!("cargo:rerun-if-changed=src/core/config.rs");

    let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let manifest_path = Path::new(&manifest_dir);

    // Generate the hub SKILL.md stub from src/skills/WIKI.md
    let hub_src = manifest_path.join("src/skills/WIKI.md");
    if let Ok(content) = fs::read_to_string(&hub_src) {
        let out_path = manifest_path.join("agents/skills/wiki/SKILL.md");
        if let Some(parent) = out_path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                println!(
                    "cargo:warning=failed to create skill dir {:?}: {}",
                    parent, e
                );
            }
        }
        if let Err(e) = fs::write(&out_path, content) {
            println!(
                "cargo:warning=failed to write hub SKILL.md {:?}: {}",
                out_path, e
            );
        }
    }

    // Inject JSON Schema into SETUP/SKILL.md (between BEGIN SCHEMA / END SCHEMA markers).
    // Always rewrites the block so the schema stays in sync on every build.
    let setup_path = manifest_path.join("src/skills/SETUP/SKILL.md");
    if let Ok(content) = fs::read_to_string(&setup_path) {
        let begin_marker = "<!-- BEGIN SCHEMA -->";
        let end_marker = "<!-- END SCHEMA -->";
        if let (Some(begin), Some(end)) = (content.find(begin_marker), content.find(end_marker)) {
            if end > begin {
                let schema = schemars::schema_for!(Config);
                let schema_json =
                    serde_json::to_string_pretty(&schema).expect("schema is always serializable");
                // Preserve the markdown code-fence wrapper around the JSON
                let fenced = format!("```json\n{}\n```", schema_json);
                let mut new_content = String::with_capacity(content.len() + schema_json.len());
                new_content.push_str(&content[..=begin + begin_marker.len()]);
                new_content.push('\n');
                new_content.push_str(&fenced);
                new_content.push('\n');
                new_content.push_str(&content[end..]);
                if let Err(e) = fs::write(&setup_path, new_content) {
                    println!(
                        "cargo:warning=failed to inject JSON schema into SETUP/SKILL.md: {}",
                        e
                    );
                }
            }
        }
    }
}
