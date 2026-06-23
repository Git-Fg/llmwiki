use std::path::PathBuf;

use crate::core::config::{resolve_api_key, resolve_config};
use crate::core::registry::{home_dir, Registry};
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
    // One-time migration notice for users on the v0.3.6 path. v0.3.7 moved
    // per-computer config from `~/llmwiki-cli/config.toml` (non-hidden) to
    // `~/.llmwiki-cli/config.toml` (hidden). If the legacy file still exists,
    // tell the user to move it — the legacy file is NOT loaded anymore.
    if let Some(msg) = legacy_config_notice() {
        eprintln!("{}", msg);
    }

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

/// Returns the user-visible migration notice to print when a legacy
/// `~/llmwiki-cli/config.toml` (v0.3.6 path) still exists and the new
/// `~/.llmwiki-cli/config.toml` (v0.3.7 path) does not. Returns `None` when
/// no notice is needed (legacy absent OR new already in place).
fn legacy_config_notice() -> Option<String> {
    let home = home_dir()?;
    let legacy = home.join("llmwiki-cli").join("config.toml");
    let new = home.join(".llmwiki-cli").join("config.toml");
    if legacy.is_file() && !new.is_file() {
        Some(format!(
            "Note: legacy config at {} is no longer loaded. Move it to {} \
             (e.g. `mv {} {}`).",
            legacy.display(),
            new.display(),
            legacy.display(),
            new.display()
        ))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Returns the home directory used by `legacy_config_notice` — captured
    /// here so a test can swap $HOME without polluting other tests.
    fn notice_for_home(home: &std::path::Path) -> Option<String> {
        let prev = std::env::var_os("HOME");
        std::env::set_var("HOME", home);
        std::env::remove_var("USERPROFILE");
        let result = legacy_config_notice();
        match prev {
            Some(p) => std::env::set_var("HOME", p),
            None => std::env::remove_var("HOME"),
        }
        result
    }

    #[test]
    fn notice_fires_when_legacy_exists_and_new_does_not() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("llmwiki-cli")).unwrap();
        std::fs::write(
            tmp.path().join("llmwiki-cli").join("config.toml"),
            "[nim]\n",
        )
        .unwrap();

        let msg = notice_for_home(tmp.path());
        assert!(
            msg.is_some(),
            "expected migration notice when only legacy exists"
        );
        let msg = msg.unwrap();
        assert!(msg.contains("legacy config"));
        assert!(msg.contains(".llmwiki-cli/config.toml"));
    }

    #[test]
    fn notice_suppressed_when_new_already_in_place() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("llmwiki-cli")).unwrap();
        std::fs::create_dir_all(tmp.path().join(".llmwiki-cli")).unwrap();
        std::fs::write(
            tmp.path().join("llmwiki-cli").join("config.toml"),
            "[nim]\n",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join(".llmwiki-cli").join("config.toml"),
            "[nim]\n",
        )
        .unwrap();

        let msg = notice_for_home(tmp.path());
        assert!(
            msg.is_none(),
            "expected NO notice when new path is already in place; got: {:?}",
            msg
        );
    }

    #[test]
    fn notice_suppressed_when_neither_path_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let msg = notice_for_home(tmp.path());
        assert!(
            msg.is_none(),
            "expected NO notice when neither path exists; got: {:?}",
            msg
        );
    }
}
