use crate::cli::ConfigCmd;
use crate::error::WikiError;

pub async fn run(cmd: ConfigCmd) -> Result<(), WikiError> {
    match cmd {
        ConfigCmd::Path => cmd_path().await,
        ConfigCmd::List { wiki, json } => cmd_list(wiki.as_deref(), json).await,
        ConfigCmd::Get { key, wiki } => cmd_get(&key, wiki.as_deref()).await,
        ConfigCmd::Set { key, value, wiki } => cmd_set(&key, &value, wiki.as_deref()).await,
        ConfigCmd::Unset { key, wiki } => cmd_unset(&key, wiki.as_deref()).await,
        ConfigCmd::Add {
            alias,
            path,
            tags,
            description,
        } => cmd_add(&alias, &path, &tags, description.as_deref()).await,
        ConfigCmd::Rm { alias } => cmd_rm(&alias).await,
        ConfigCmd::Edit => cmd_edit().await,
        ConfigCmd::Validate => cmd_validate().await,
        ConfigCmd::ShowSchema => cmd_show_schema().await,
    }
}

async fn cmd_path() -> Result<(), WikiError> {
    let reg = crate::core::registry::Registry::discover()?;
    println!("{}", reg.root_path.display());
    Ok(())
}

async fn cmd_list(wiki: Option<&str>, json: bool) -> Result<(), WikiError> {
    let reg = crate::core::registry::Registry::discover()?;

    match wiki {
        Some(alias) => {
            let cfg = reg.resolve_config(alias)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&cfg).unwrap());
            } else {
                let value = config_to_value(&cfg);
                print_value_dotted(&value, "");
            }
        }
        None => {
            if json {
                let entries: Vec<_> = reg
                    .entries
                    .iter()
                    .map(|e| {
                        serde_json::json!({
                            "alias": e.alias,
                            "path": e.path,
                            "tags": e.tags,
                            "description": e.description,
                        })
                    })
                    .collect();
                println!("{}", serde_json::to_string_pretty(&entries).unwrap());
            } else {
                if reg.entries.is_empty() {
                    println!(
                        "No wikis registered. Use 'wiki config add <alias> <path>' to add one."
                    );
                    return Ok(());
                }
                println!("{:<15} {:<40} TAGS", "ALIAS", "PATH");
                for entry in &reg.entries {
                    let tags = entry.tags.join(", ");
                    println!("{:<15} {:<40} {}", entry.alias, entry.path.display(), tags);
                }
            }
        }
    }
    Ok(())
}

async fn cmd_get(key: &str, wiki: Option<&str>) -> Result<(), WikiError> {
    let reg = crate::core::registry::Registry::discover()?;
    let cfg = match wiki {
        Some(alias) => reg.resolve_config(alias)?,
        None => reg.resolve_defaults()?,
    };

    let value = config_to_value(&cfg);
    let rendered = navigate(&value, key)?;
    println!("{}", rendered);
    Ok(())
}

async fn cmd_set(key: &str, value: &str, wiki: Option<&str>) -> Result<(), WikiError> {
    let mut reg = crate::core::registry::Registry::discover()?;
    reg.set_value(key, value, wiki)?;
    reg.save()?;
    let target = wiki.unwrap_or("defaults");
    println!("Set {} = {} in [{}]", key, value, target);
    Ok(())
}

async fn cmd_unset(key: &str, wiki: Option<&str>) -> Result<(), WikiError> {
    let alias = wiki
        .ok_or_else(|| WikiError::Other(anyhow::anyhow!("--wiki <alias> is required for unset")))?;
    let mut reg = crate::core::registry::Registry::discover()?;
    reg.unset_value(key, alias)?;
    reg.save()?;
    println!("Unset {} from [{}]", key, alias);
    Ok(())
}

async fn cmd_add(
    alias: &str,
    path: &std::path::Path,
    tags: &[String],
    description: Option<&str>,
) -> Result<(), WikiError> {
    let mut reg = crate::core::registry::Registry::discover()?;
    reg.add_entry(alias, path, tags, description)?;
    reg.save()?;
    println!("Added wiki '{}' → {}", alias, path.display());
    Ok(())
}

async fn cmd_rm(alias: &str) -> Result<(), WikiError> {
    let mut reg = crate::core::registry::Registry::discover()?;
    reg.remove_entry(alias)?;
    reg.save()?;
    println!("Removed wiki '{}'", alias);
    Ok(())
}

async fn cmd_edit() -> Result<(), WikiError> {
    let reg = crate::core::registry::Registry::discover()?;
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    let status = std::process::Command::new(&editor)
        .arg(&reg.root_path)
        .status()
        .map_err(|e| WikiError::Other(anyhow::anyhow!("failed to launch {}: {}", editor, e)))?;
    if !status.success() {
        return Err(WikiError::Other(anyhow::anyhow!(
            "editor exited with status {}",
            status
        )));
    }
    Ok(())
}

/// Print the JSON Schema for the `Config` type. Useful for editor autocomplete,
/// LSP configuration, or external tooling.
async fn cmd_show_schema() -> Result<(), WikiError> {
    let schema = schemars::schema_for!(crate::core::config::Config);
    println!(
        "{}",
        serde_json::to_string_pretty(&schema).expect("schema serialization is infallible")
    );
    Ok(())
}

/// Load the registry, parse `[defaults]` + every `[alias]` into a `Config`,
/// and run field-level checks. Reports a summary; exits non-zero on any
/// failure.
async fn cmd_validate() -> Result<(), WikiError> {
    use crate::core::config::validate;
    let reg = crate::core::registry::Registry::discover()?;
    let mut failures = 0usize;

    // Validate [defaults] if present
    if reg.defaults.raw.is_some() {
        match reg.resolve_defaults() {
            Ok(cfg) => match validate(&cfg) {
                Ok(()) => println!("✓ [defaults]"),
                Err(errs) => {
                    println!("✗ [defaults]");
                    for e in errs {
                        println!("    {}", e);
                    }
                    failures += 1;
                }
            },
            Err(e) => {
                println!("✗ [defaults] — {}", e);
                failures += 1;
            }
        }
    } else {
        println!("· [defaults] (not set, using built-in defaults)");
    }

    // Validate each alias
    for entry in &reg.entries {
        match reg.resolve_config(&entry.alias) {
            Ok(cfg) => match validate(&cfg) {
                Ok(()) => println!("✓ [{}]", entry.alias),
                Err(errs) => {
                    println!("✗ [{}]", entry.alias);
                    for e in errs {
                        println!("    {}", e);
                    }
                    failures += 1;
                }
            },
            Err(e) => {
                println!("✗ [{}] — {}", entry.alias, e);
                failures += 1;
            }
        }
    }

    if failures == 0 {
        Ok(())
    } else {
        Err(WikiError::Other(anyhow::anyhow!(
            "{} wiki(s) failed validation",
            failures
        )))
    }
}

/// Serialize `Config` to a `toml::Value` so we can navigate it reflectively.
/// Adding a new field to `Config` is automatically picked up — no per-field
/// code to keep in sync.
pub(crate) fn config_to_value(cfg: &crate::core::config::Config) -> toml::Value {
    toml::Value::try_from(cfg).expect("Config serialization to TOML is infallible")
}

/// Resolve a dotted key like `nim.retry.max_attempts` against a TOML value tree.
pub(crate) fn navigate(root: &toml::Value, key: &str) -> Result<String, WikiError> {
    let parts: Vec<&str> = key.split('.').collect();
    let mut current = root;
    for (i, part) in parts.iter().enumerate() {
        let table = current.as_table().ok_or_else(|| {
            WikiError::Other(anyhow::anyhow!(
                "config key '{}' is not a table at segment '{}'",
                key,
                parts[..i].join(".")
            ))
        })?;
        match table.get(*part) {
            Some(v) => current = v,
            None => {
                // Known `Option<T>` fields that are absent when unset.
                if OPTIONAL_KEYS.contains(&key) {
                    return Ok("(unset)".into());
                }
                let valid: Vec<String> = table
                    .keys()
                    .map(|k| {
                        let mut prefix = parts[..i].join(".");
                        if !prefix.is_empty() {
                            prefix.push('.');
                        }
                        format!("{}{}", prefix, k)
                    })
                    .collect();
                return Err(WikiError::Other(anyhow::anyhow!(
                    "unknown config key '{}'. Valid keys: {}",
                    key,
                    valid.join(", ")
                )));
            }
        }
    }
    Ok(format_value(current))
}

/// Dotted keys whose value is `Option<T>` in `Config`. These are absent from
/// the TOML value tree when `None`, so we report "(unset)" rather than error.
const OPTIONAL_KEYS: &[&str] = &["nim.embed_dim_override"];

/// Human-readable rendering of any `toml::Value` (no surrounding quotes on strings).
pub(crate) fn format_value(v: &toml::Value) -> String {
    match v {
        toml::Value::String(s) => s.clone(),
        toml::Value::Integer(i) => i.to_string(),
        toml::Value::Float(f) => f.to_string(),
        toml::Value::Boolean(b) => b.to_string(),
        toml::Value::Datetime(dt) => dt.to_string(),
        toml::Value::Array(arr) => {
            let parts: Vec<String> = arr.iter().map(format_value).collect();
            format!("[{}]", parts.join(", "))
        }
        toml::Value::Table(_) => "<table>".to_string(),
    }
}

/// Collect every leaf key in a TOML value tree as `(dotted_key, value)` pairs.
pub(crate) fn collect_dotted(value: &toml::Value, prefix: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    collect_dotted_into(value, prefix, &mut out);
    out
}

fn collect_dotted_into(value: &toml::Value, prefix: &str, out: &mut Vec<(String, String)>) {
    if let toml::Value::Table(table) = value {
        let mut keys: Vec<&String> = table.keys().collect();
        keys.sort();
        for k in keys {
            let v = &table[k];
            let full = if prefix.is_empty() {
                k.clone()
            } else {
                format!("{}.{}", prefix, k)
            };
            if matches!(v, toml::Value::Table(_)) {
                collect_dotted_into(v, &full, out);
            } else {
                out.push((full, format_value(v)));
            }
        }
    }
}

/// Print a TOML value tree as `key = value` lines, recursing into tables with
/// dot-separated prefixes (e.g. `nim.retry.max_attempts = 3`).
fn print_value_dotted(value: &toml::Value, prefix: &str) {
    for (key, value) in collect_dotted(value, prefix) {
        println!("{} = {}", key, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_value() -> toml::Value {
        let toml_src = r#"
[nim]
embed_model = "nvidia/nv-embed-v1"
base_url = "https://integrate.api.nvidia.com"
batch_size = 8

[nim.retry]
max_attempts = 3
backoff_ms = 500

[wiki]
default_chunk_tokens = 512
require_frontmatter = true
"#;
        toml::from_str(toml_src).unwrap()
    }

    #[test]
    fn navigate_resolves_nested_key() {
        let v = sample_value();
        assert_eq!(
            navigate(&v, "nim.embed_model").unwrap(),
            "nvidia/nv-embed-v1"
        );
        assert_eq!(navigate(&v, "nim.retry.max_attempts").unwrap(), "3");
        assert_eq!(
            navigate(&v, "nim.base_url").unwrap(),
            "https://integrate.api.nvidia.com"
        );
        assert_eq!(navigate(&v, "wiki.require_frontmatter").unwrap(), "true");
    }

    #[test]
    fn navigate_unknown_key_lists_valid_keys() {
        let v = sample_value();
        let err = navigate(&v, "nim.bogus").unwrap_err().to_string();
        assert!(err.contains("unknown config key 'nim.bogus'"));
        assert!(err.contains("embed_model"));
        assert!(err.contains("base_url"));
    }

    #[test]
    fn navigate_wrong_type_for_segment() {
        let v = sample_value();
        let err = navigate(&v, "nim.embed_model.bogus")
            .unwrap_err()
            .to_string();
        assert!(err.contains("not a table"));
    }

    #[test]
    fn format_value_renders_strings_without_quotes() {
        assert_eq!(format_value(&toml::Value::String("abc".into())), "abc");
        assert_eq!(format_value(&toml::Value::Integer(42)), "42");
        assert_eq!(format_value(&toml::Value::Boolean(true)), "true");
        assert_eq!(format_value(&toml::Value::Boolean(false)), "false");
    }

    #[test]
    fn print_value_dotted_emits_all_leaf_keys() {
        let v = sample_value();
        fn collect(v: &toml::Value, prefix: &str, out: &mut Vec<String>) {
            if let toml::Value::Table(t) = v {
                let mut keys: Vec<&String> = t.keys().collect();
                keys.sort();
                for k in keys {
                    let vv = &t[k];
                    let full = if prefix.is_empty() {
                        k.clone()
                    } else {
                        format!("{prefix}.{k}")
                    };
                    if matches!(vv, toml::Value::Table(_)) {
                        collect(vv, &full, out);
                    } else {
                        out.push(full);
                    }
                }
            }
        }
        let mut keys = Vec::new();
        collect(&v, "", &mut keys);
        assert!(keys.contains(&"nim.embed_model".to_string()));
        assert!(keys.contains(&"nim.retry.max_attempts".to_string()));
        assert!(keys.contains(&"wiki.default_chunk_tokens".to_string()));
    }

    #[test]
    fn config_to_value_covers_all_fields() {
        let cfg = crate::core::config::Config::default();
        let v = config_to_value(&cfg);
        let table = v.as_table().unwrap();
        assert!(table.contains_key("nim"));
        assert!(table.contains_key("wiki"));
        let nim = table["nim"].as_table().unwrap();
        assert!(nim.contains_key("embed_model"));
        assert!(nim.contains_key("base_url"));
        assert!(nim.contains_key("api_key_env"));
        assert!(nim.contains_key("batch_size"));
        assert!(nim.contains_key("request_timeout_secs"));
        assert!(nim.contains_key("retry"));
        let retry = nim["retry"].as_table().unwrap();
        assert!(retry.contains_key("max_attempts"));
        assert!(retry.contains_key("backoff_ms"));
        let wiki = table["wiki"].as_table().unwrap();
        assert!(wiki.contains_key("default_chunk_tokens"));
        assert!(wiki.contains_key("chunk_overlap_tokens"));
        assert!(wiki.contains_key("min_chunk_tokens"));
        assert!(wiki.contains_key("require_frontmatter"));
        assert!(wiki.contains_key("require_wikilinks_min"));
    }
}
