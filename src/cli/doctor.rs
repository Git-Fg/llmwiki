use std::path::PathBuf;

use crate::core::config::{resolve_api_key, resolve_config};
use crate::core::workspace::discover_workspace;
use crate::error::WikiError;

pub struct DoctorArgs {
    pub workspace: Option<PathBuf>,
    pub json: bool,
}

#[derive(serde::Serialize)]
struct DoctorReport {
    workspace: String,
    config_loaded: bool,
    embed_model: String,
    nim_base_url: String,
    api_key_length: usize,
    api_key_env: String,
    nim_reachable: bool,
    nim_status: Option<u16>,
    nim_error: Option<String>,
}

pub async fn run(args: DoctorArgs) -> Result<(), WikiError> {
    let ws = discover_workspace(
        args.workspace.clone(),
        std::env::var("WIKI_WORKSPACE").ok().map(PathBuf::from),
        std::env::current_dir()?,
    )?;

    let cfg_result = resolve_config(&ws);
    let mut cfg = match cfg_result {
        Ok(c) => c,
        Err(e) => {
            if args.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&DoctorReport {
                        workspace: ws.display().to_string(),
                        config_loaded: false,
                        embed_model: String::new(),
                        nim_base_url: String::new(),
                        api_key_length: 0,
                        api_key_env: String::new(),
                        nim_reachable: false,
                        nim_status: None,
                        nim_error: Some(format!("config load failed: {e}")),
                    })?
                );
                return Ok(());
            }
            eprintln!("Error: {}", e);
            std::process::exit(2);
        }
    };
    if let Ok(base_url) = std::env::var("WIKI_NIM_BASE_URL") {
        cfg.nim.base_url = base_url;
    }

    let api_key = resolve_api_key(&cfg.nim);
    let api_key_length = api_key.len();

    // base_url is the host (e.g. https://integrate.api.nvidia.com);
    // the NIM API lives under /v1/<endpoint>.
    let url = format!("{}/v1/models", cfg.nim.base_url.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(
            cfg.nim.request_timeout_secs.min(10),
        ))
        .build()?;

    let (nim_reachable, nim_status, nim_error) =
        match client.get(&url).bearer_auth(&api_key).send().await {
            Ok(resp) if resp.status().is_success() => (true, Some(resp.status().as_u16()), None),
            Ok(resp) => (
                false,
                Some(resp.status().as_u16()),
                Some(format!("HTTP {}", resp.status())),
            ),
            Err(e) => (false, None, Some(format!("{e}"))),
        };

    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&DoctorReport {
                workspace: ws.display().to_string(),
                config_loaded: true,
                embed_model: cfg.nim.embed_model,
                nim_base_url: cfg.nim.base_url,
                api_key_length,
                api_key_env: cfg.nim.api_key_env,
                nim_reachable,
                nim_status,
                nim_error,
            })?
        );
    } else {
        println!("✓ Workspace: {}", ws.display());
        println!("✓ Config loaded");
        println!("  Embed model: {}", cfg.nim.embed_model);
        println!("  NIM base URL: {}", cfg.nim.base_url);
        if api_key_length == 0 {
            eprintln!(
                "✗ API key not set ({} or NVIDIA_API_KEY)",
                cfg.nim.api_key_env
            );
            if nim_reachable {
                // some public endpoints don't require auth
                println!("  API key: not set (endpoint may be public)");
            } else {
                std::process::exit(3);
            }
        } else {
            println!("✓ API key set (length: {})", api_key_length);
        }
        if nim_reachable {
            println!("✓ NIM endpoint reachable");
        } else {
            eprintln!(
                "✗ NIM endpoint unreachable: {}",
                nim_error.unwrap_or_default()
            );
            std::process::exit(3);
        }
    }

    Ok(())
}
