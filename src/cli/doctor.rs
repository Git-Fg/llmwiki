use std::path::PathBuf;

use crate::core::config::{resolve_api_key, resolve_config};
use crate::core::registry::Registry;
use crate::core::workspace::discover_workspace;
use crate::error::WikiError;

pub struct DoctorArgs {
    pub workspace: Option<PathBuf>,
    pub wiki: Option<String>,
    pub json: bool,
}

#[derive(serde::Serialize)]
struct DoctorReport {
    workspace: String,
    active_alias: String,
    wiki_root_path: String,
    registry_entries: usize,
    config_loaded: bool,
    embed_model: String,
    nim_base_url: String,
    api_key_length: usize,
    api_key_env: String,
    nim_reachable: bool,
    nim_status: Option<u16>,
    nim_error: Option<String>,
    embed_model_available: bool,
    /// Full reflective config dump as `key → value` pairs (dotted keys).
    config: std::collections::BTreeMap<String, String>,
}

pub async fn run(args: DoctorArgs) -> Result<(), WikiError> {
    // Report wiki-root.toml info
    let registry_info = match Registry::discover() {
        Ok(reg) => Some((reg.root_path.display().to_string(), reg.entries.len())),
        Err(_) => None,
    };

    let ws = discover_workspace(
        args.workspace.clone(),
        args.wiki.as_deref(),
        std::env::var("WIKI_WORKSPACE").ok().map(PathBuf::from),
        std::env::var("WIKI_ACTIVE").ok().as_deref(),
        std::env::current_dir()?,
    )?;

    // Determine active alias
    let active_alias = if let Some(w) = &args.wiki {
        w.clone()
    } else if let Ok(reg) = Registry::discover() {
        reg.entries
            .iter()
            .find(|e| e.path == ws)
            .map(|e| e.alias.clone())
            .unwrap_or_default()
    } else {
        String::new()
    };

    let cfg_result = resolve_config(&ws);
    let mut cfg = match cfg_result {
        Ok(c) => c,
        Err(e) => {
            if args.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&DoctorReport {
                        workspace: ws.display().to_string(),
                        active_alias: active_alias.clone(),
                        wiki_root_path: registry_info
                            .as_ref()
                            .map(|(p, _)| p.clone())
                            .unwrap_or_default(),
                        registry_entries: registry_info.as_ref().map(|(_, c)| *c).unwrap_or(0),
                        config_loaded: false,
                        embed_model: String::new(),
                        nim_base_url: String::new(),
                        api_key_length: 0,
                        api_key_env: String::new(),
                        nim_reachable: false,
                        nim_status: None,
                        nim_error: Some(format!("config load failed: {e}")),
                        embed_model_available: false,
                        config: Default::default(),
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

    // Reflective dump of all config keys (dotted) for machine-readable diagnostics.
    let config_dump: std::collections::BTreeMap<String, String> =
        crate::cli::config::collect_dotted(&crate::cli::config::config_to_value(&cfg), "")
            .into_iter()
            .collect();

    // base_url is the host (e.g. https://integrate.api.nvidia.com);
    // the NIM API lives under /v1/<endpoint>.
    let url = format!("{}/v1/models", cfg.nim.base_url.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(
            cfg.nim.request_timeout_secs.min(10),
        ))
        .build()?;

    let (nim_reachable, nim_status, nim_error, embed_model_available) =
        match client.get(&url).bearer_auth(&api_key).send().await {
            Ok(resp) if resp.status().is_success() => {
                let status = resp.status();
                let embed_model_available = if let Ok(body) = resp.json::<serde_json::Value>().await
                {
                    body.get("data")
                        .and_then(|d| d.as_array())
                        .map(|arr| {
                            arr.iter().any(|m| {
                                m.get("id")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s == cfg.nim.embed_model)
                                    .unwrap_or(false)
                            })
                        })
                        .unwrap_or(false)
                } else {
                    false
                };
                (true, Some(status.as_u16()), None, embed_model_available)
            }
            Ok(resp) => (
                false,
                Some(resp.status().as_u16()),
                Some(format!("HTTP {}", resp.status())),
                false,
            ),
            Err(e) => (false, None, Some(format!("{e}")), false),
        };

    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&DoctorReport {
                workspace: ws.display().to_string(),
                active_alias,
                wiki_root_path: registry_info
                    .as_ref()
                    .map(|(p, _)| p.clone())
                    .unwrap_or_default(),
                registry_entries: registry_info.as_ref().map(|(_, c)| *c).unwrap_or(0),
                config_loaded: true,
                embed_model: cfg.nim.embed_model,
                nim_base_url: cfg.nim.base_url,
                api_key_length,
                api_key_env: cfg.nim.api_key_env,
                nim_reachable,
                nim_status,
                nim_error,
                embed_model_available,
                config: config_dump,
            })?
        );
    } else {
        println!("✓ Workspace: {}", ws.display());
        if let Some((path, count)) = &registry_info {
            println!("✓ Wiki registry: {} ({} entries)", path, count);
        } else {
            println!("✗ Wiki registry: not found");
        }
        if !active_alias.is_empty() {
            println!("✓ Active alias: {}", active_alias);
        }
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
