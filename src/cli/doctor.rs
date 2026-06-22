use std::path::PathBuf;

use crate::core::config::{resolve_api_key, resolve_config};
use crate::core::workspace::discover_workspace;
use crate::error::WikiError;

pub struct DoctorArgs {
    pub workspace: Option<PathBuf>,
    pub json: bool,
}

pub async fn run(args: DoctorArgs) -> Result<(), WikiError> {
    let ws = discover_workspace(
        args.workspace.clone(),
        std::env::var("WIKI_WORKSPACE").ok().map(PathBuf::from),
        std::env::current_dir()?,
    )?;

    println!("✓ Workspace: {}", ws.display());

    let cfg = resolve_config(&ws)?;
    let mut cfg = cfg;
    if let Ok(base_url) = std::env::var("WIKI_NIM_BASE_URL") {
        cfg.nim.base_url = base_url;
    }
    println!("✓ Config loaded");
    println!("  Embed model: {}", cfg.nim.embed_model);
    println!("  NIM base URL: {}", cfg.nim.base_url);

    let api_key = resolve_api_key(&cfg.nim);
    if api_key.is_empty() {
        eprintln!(
            "✗ API key not set ({} or NVIDIA_API_KEY)",
            cfg.nim.api_key_env
        );
        std::process::exit(3);
    }
    println!("✓ API key set (length: {})", api_key.len());

    // base_url is the host (e.g. https://integrate.api.nvidia.com);
    // the NIM API lives under /v1/<endpoint>.
    let url = format!("{}/v1/models", cfg.nim.base_url.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(
            cfg.nim.request_timeout_secs.min(10),
        ))
        .build()?;
    match client.get(&url).bearer_auth(&api_key).send().await {
        Ok(resp) if resp.status().is_success() => println!("✓ NIM endpoint reachable"),
        Ok(resp) => {
            eprintln!("✗ NIM endpoint returned {}", resp.status());
            std::process::exit(3);
        }
        Err(e) => {
            eprintln!("✗ NIM endpoint unreachable: {}", e);
            std::process::exit(3);
        }
    }

    Ok(())
}
