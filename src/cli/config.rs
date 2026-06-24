use crate::cli::ConfigCmd;
use crate::core::registry::{ResolutionSource, ResolvedWiki};
use crate::error::WikiError;
use std::path::{Path, PathBuf};

pub async fn run(cmd: ConfigCmd) -> Result<(), WikiError> {
    match cmd {
        ConfigCmd::Path => cmd_path().await,
        ConfigCmd::Paths { workspace, json } => cmd_paths(workspace, json).await,
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
        ConfigCmd::ConfigEdit { workspace } => cmd_config_edit(workspace).await,
        ConfigCmd::Validate => cmd_validate().await,
        ConfigCmd::ShowSchema { section } => cmd_show_schema(section).await,
        ConfigCmd::Current {
            workspace,
            wiki,
            json,
        } => cmd_current(workspace.as_deref(), wiki.as_deref(), json).await,
        ConfigCmd::ShowEffective {
            workspace,
            json,
            key_prefix,
            source,
            overrides_only,
        } => cmd_show_effective(workspace, json, key_prefix, source, overrides_only).await,
    }
}

async fn cmd_path() -> Result<(), WikiError> {
    let reg = crate::core::registry::Registry::discover()?;
    println!("{}", reg.root_path.display());
    Ok(())
}

/// Print the resolved config search order with each path's existence status.
/// Uses `--workspace` if given; otherwise runs full workspace discovery so
/// the output matches what every other CLI command would see.
async fn cmd_paths(workspace: Option<PathBuf>, json: bool) -> Result<(), WikiError> {
    use crate::core::config::config_paths;
    use crate::core::workspace::discover_workspace;

    let ws = match workspace {
        Some(p) => p,
        None => discover_workspace(
            None,
            None,
            std::env::var("WIKI_WORKSPACE").ok().map(PathBuf::from),
            std::env::var("WIKI_ACTIVE").ok().as_deref(),
            std::env::current_dir()?,
        )?,
    };

    let paths = config_paths(&ws);
    let entries: Vec<(String, String, bool)> = paths
        .iter()
        .map(|p| {
            let label = label_for(p);
            let exists = p.is_file();
            (label, p.display().to_string(), exists)
        })
        .collect();

    if json {
        // JSON output preserves the underlying list order ("lowest priority
        // first") so machine consumers see the actual order `load_config`
        // iterates and merges.
        let json_entries: Vec<_> = entries
            .iter()
            .map(|(label, path, exists)| {
                serde_json::json!({
                    "source": label,
                    "path": path,
                    "exists": exists,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "workspace": ws.display().to_string(),
                "paths": json_entries,
                "merge_order_note": "lowest priority first; later entries override earlier (last-wins merge)",
            }))?
        );
    } else {
        // Human output reverses the list so users see "highest priority first"
        // (the intuitive order when debugging "why is this key overriding
        // that one?"). The underlying `paths` vec is still "lowest priority
        // first" for `load_config`'s last-wins merge.
        println!("Workspace: {}", ws.display());
        println!("Config search order (highest priority first):");
        for (label, path, exists) in entries.iter().rev() {
            let status = if *exists { "exists  " } else { "missing " };
            println!("  [{status}] {label:<14} {path}");
        }
        if !entries.iter().any(|(_, _, e)| *e) {
            println!(
                "(no config file found — falling back to built-in defaults; \
                 set LLMWIKI_CONFIG or write ~/.llmwiki-cli/config.toml to override)"
            );
        }
    }
    Ok(())
}

/// Tag a config path with the source it came from, for `wiki config paths`
/// output. Helps users tell which priority slot a path occupies without
/// having to compare canonical paths.
fn label_for(p: &std::path::Path) -> String {
    let s = p.to_string_lossy();
    if std::env::var("LLMWIKI_CONFIG").ok().as_deref() == Some(&s) {
        "LLMWIKI_CONFIG".into()
    } else if s.contains("/.llmwiki-cli/config.toml") || s.ends_with(".llmwiki-cli/config.toml") {
        // Per-workspace if `workspace` was the walk-up start (path is inside
        // the workspace); per-computer if it lives under HOME.
        if let Some(home) = crate::core::registry::home_dir() {
            if p.starts_with(home.join(".llmwiki-cli")) {
                return "per-computer".into();
            }
        }
        "per-workspace".into()
    } else {
        "unknown".into()
    }
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
    println!("{rendered}");
    Ok(())
}

async fn cmd_set(key: &str, value: &str, wiki: Option<&str>) -> Result<(), WikiError> {
    let mut reg = crate::core::registry::Registry::discover()?;
    reg.set_value(key, value, wiki)?;
    reg.save()?;
    let target = wiki.unwrap_or("defaults");
    println!("Set {key} = {value} in [{target}]");
    Ok(())
}

async fn cmd_unset(key: &str, wiki: Option<&str>) -> Result<(), WikiError> {
    let alias = wiki
        .ok_or_else(|| WikiError::Other(anyhow::anyhow!("--wiki <alias> is required for unset")))?;
    let mut reg = crate::core::registry::Registry::discover()?;
    reg.unset_value(key, alias)?;
    reg.save()?;
    println!("Unset {key} from [{alias}]");
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
    println!("Removed wiki '{alias}'");
    Ok(())
}

async fn cmd_edit() -> Result<(), WikiError> {
    let reg = crate::core::registry::Registry::discover()?;
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    let status = std::process::Command::new(&editor)
        .arg(&reg.root_path)
        .status()
        .map_err(|e| WikiError::Other(anyhow::anyhow!("failed to launch {editor}: {e}")))?;
    if !status.success() {
        return Err(WikiError::Other(anyhow::anyhow!(
            "editor exited with status {status}",
        )));
    }
    Ok(())
}

/// Open the highest-priority config file in `$EDITOR`. Order:
/// 1. `$LLMWIKI_CONFIG` (if set)
/// 2. existing per-workspace config (`<workspace>/.llmwiki-cli/config.toml`)
/// 3. existing per-computer config (`~/.llmwiki-cli/config.toml`)
/// 4. per-workspace candidate (creates a new file when saved)
///
/// This is the config-file analog of `wiki config edit` (which opens
/// `wiki-root.toml`). Lets users edit either registry or per-workspace config
/// without remembering the path.
///
/// Accepts an optional `--workspace` override (inherited from the global
/// `--workspace` flag). Falls back to full workspace discovery when unset.
async fn cmd_config_edit(workspace_override: Option<PathBuf>) -> Result<(), WikiError> {
    use crate::core::config::config_paths;
    use crate::core::workspace::discover_workspace;

    let ws = match workspace_override {
        Some(p) => p,
        None => discover_workspace(
            None,
            None,
            std::env::var("WIKI_WORKSPACE").ok().map(PathBuf::from),
            std::env::var("WIKI_ACTIVE").ok().as_deref(),
            std::env::current_dir()?,
        )?,
    };
    let paths = config_paths(&ws);

    // Pick the first EXISTING file; fall back to the per-workspace candidate
    // (always in the list as the second entry, since `config_paths` is
    // "lowest priority first": per-computer at 0, per-workspace at 1,
    // LLMWIKI_CONFIG at 2 when set) so the user can create one.
    let target = paths
        .iter()
        .find(|p| p.is_file())
        .cloned()
        .or_else(|| {
            // Per-workspace candidate is the second-lowest slot when
            // LLMWIKI_CONFIG isn't set, or the third slot when it is.
            let candidate_idx = if std::env::var("LLMWIKI_CONFIG").is_ok() {
                2
            } else {
                1
            };
            paths.get(candidate_idx).cloned()
        })
        .ok_or_else(|| WikiError::Other(anyhow::anyhow!("no config path candidate available")))?;

    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    println!("Opening {} in {}", target.display(), editor);
    let status = std::process::Command::new(&editor)
        .arg(&target)
        .status()
        .map_err(|e| WikiError::Other(anyhow::anyhow!("failed to launch {editor}: {e}")))?;
    if !status.success() {
        return Err(WikiError::Other(anyhow::anyhow!(
            "editor exited with status {status}",
        )));
    }
    Ok(())
}

/// Print every effective config key alongside the source file it came from,
/// mirroring `git config --list --show-origin`. Deep-merges all loaded files
/// (per-computer → per-workspace → LLMWIKI_CONFIG → defaults) so the LAST
/// file containing a key is reported as its source — same semantics as git.
///
/// Optional filters:
///   - `key_prefix`: only show keys starting with this prefix (e.g. `nim.`)
///   - `source`: only show keys whose source file matches this path
///     (canonicalized comparison; useful for "what did THIS file set?")
async fn cmd_show_effective(
    workspace: Option<PathBuf>,
    json: bool,
    key_prefix: Option<String>,
    source: Option<PathBuf>,
    overrides_only: bool,
) -> Result<(), WikiError> {
    use crate::core::config::{config_paths, load_config_unvalidated};
    use crate::core::workspace::discover_workspace;

    let ws = match workspace {
        Some(p) => p,
        None => discover_workspace(
            None,
            None,
            std::env::var("WIKI_WORKSPACE").ok().map(PathBuf::from),
            std::env::var("WIKI_ACTIVE").ok().as_deref(),
            std::env::current_dir()?,
        )?,
    };
    let paths = config_paths(&ws);

    // Canonicalize the source filter (if provided) so path-display differences
    // (e.g. /tmp vs /private/tmp on macOS) don't defeat the comparison.
    let source_filter_canon: Option<PathBuf> = source
        .as_ref()
        .and_then(|p| p.canonicalize().ok().or_else(|| Some(p.clone())));

    // For each path that exists, parse it and record its leaf keys. Then merge
    // by key so the LAST path (highest priority) wins; the surviving key's
    // source is reported. This mirrors what `load_config` does for the
    // resolved Config, plus the per-source attribution.
    let mut origin: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
    let mut ordered_keys: Vec<String> = Vec::new();
    for p in &paths {
        if !p.is_file() {
            continue;
        }
        let text = std::fs::read_to_string(p).map_err(WikiError::Io)?;
        let parsed: toml::Value = text.parse().map_err(|e| WikiError::ConfigInvalid {
            path: p.display().to_string(),
            line: 0,
            message: format!("TOML parse error: {e}"),
        })?;
        for (key, _value) in collect_dotted(&parsed, "") {
            if !origin.contains_key(&key) {
                ordered_keys.push(key.clone());
            }
            origin.insert(key, p.display().to_string());
        }
    }

    // Build the final merged Config (what every command sees) and the value
    // for each origin-tracked key. Default-only keys are included so users
    // see the full picture.
    let final_cfg = load_config_unvalidated(&paths)?;
    let value_tree = config_to_value(&final_cfg);
    let all_keys: Vec<(String, String)> = collect_dotted(&value_tree, "").into_iter().collect();

    // For `--overrides-only`: compute the default value for every key so we
    // can exclude keys whose effective value equals the default. This is the
    // single most useful audit filter — most keys match defaults and the
    // user usually only cares about the ones that don't.
    let default_tree = config_to_value(&crate::core::config::Config::default());
    let default_keys: std::collections::BTreeMap<String, String> =
        collect_dotted(&default_tree, "").into_iter().collect();

    // Apply the optional filters.
    let filtered: Vec<(String, String)> = all_keys
        .into_iter()
        .filter(|(key, _)| match &key_prefix {
            Some(prefix) => key.starts_with(prefix.as_str()),
            None => true,
        })
        .filter(|(key, _)| match &source_filter_canon {
            Some(filter) => match origin.get(key) {
                Some(src) => source_path_matches(src, filter),
                None => false,
            },
            None => true,
        })
        // `--overrides-only`: hide keys whose value equals the built-in
        // default. A key with no entry in `default_keys` is, by definition,
        // an override (it isn't part of the default config), so it's kept.
        .filter(|(key, val)| !overrides_only || !is_default_value(key, val, &default_keys))
        .collect();

    let filter_note = {
        let mut parts: Vec<String> = Vec::new();
        if let Some(p) = &key_prefix {
            parts.push(format!("key={p:?}"));
        }
        if let Some(s) = &source {
            parts.push(format!("source={}", s.display()));
        }
        if overrides_only {
            parts.push("overrides-only".to_string());
        }
        if parts.is_empty() {
            String::new()
        } else {
            format!(" (filtered: {})", parts.join(", "))
        }
    };

    if json {
        let entries: Vec<_> = filtered
            .iter()
            .map(|(key, val)| {
                serde_json::json!({
                    "key": key,
                    "value": val,
                    "source": origin.get(key).cloned().unwrap_or_else(|| "<default>".into()),
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "workspace": ws.display().to_string(),
                "filter": filter_note,
                "entries": entries,
            }))?
        );
    } else {
        println!("Workspace: {}", ws.display());
        println!("Effective config (git config --show-origin style){filter_note}:",);
        for (key, val) in &filtered {
            let src = origin
                .get(key)
                .cloned()
                .unwrap_or_else(|| "<default>".into());
            // Trim the path display to just the meaningful part so the table
            // is readable when paths are long.
            let src_short = shorten_path_for_display(&src);
            println!("  {key:<40} = {val:<60} ({src_short})");
        }
    }

    // Keep `ordered_keys` reachable so the compiler doesn't warn; it's the
    // order in which keys were first observed (mostly for future use).
    let _ = ordered_keys;
    Ok(())
}

/// Compare a stored source-path string (as printed by `Path::display()`)
/// against a user-supplied filter path. Handles the macOS `/tmp` ↔
/// `/private/tmp` canonicalization asymmetry by comparing canonical paths
/// when both succeed. Returns `false` (not a fallback prefix match) when
/// either side can't be canonicalized — a literal prefix match could
/// falsely equate `/home/u/.llmwiki-cli` with `/home/u/.llmwiki-cli-extra`,
/// which is worse UX than a missed-but-precise match.
fn source_path_matches(stored: &str, filter: &std::path::Path) -> bool {
    let stored_path = std::path::Path::new(stored);
    match (stored_path.canonicalize().ok(), filter.canonicalize()) {
        (Some(a), Ok(b)) => a == b,
        _ => false,
    }
}

/// For `--overrides-only`: is the given `(key, value)` pair equal to the
/// built-in default for that key? A key absent from `default_keys` (e.g. an
/// `Option<T>` field that defaults to `None` and isn't rendered in the TOML
/// tree) is, by definition, an override relative to the default — kept.
fn is_default_value(
    key: &str,
    value: &str,
    default_keys: &std::collections::BTreeMap<String, String>,
) -> bool {
    match default_keys.get(key) {
        Some(default_value) => default_value == value,
        None => false,
    }
}

/// Shorten a path for `wiki config show-effective` output. Replaces HOME
/// prefix with `~`, workspace suffix with `<workspace>/...`.
fn shorten_path_for_display(p: &str) -> String {
    let home = crate::core::registry::home_dir()
        .map(|h| h.display().to_string())
        .unwrap_or_default();
    if !home.is_empty() && p.starts_with(&home) {
        // p.starts_with(&home) confirmed the prefix matches byte-for-byte,
        // so home.len() is at a char boundary of p.
        #[expect(
            clippy::string_slice,
            reason = "starts_with(&home) just confirmed p[..home.len()] == home; home.len() is at a char boundary"
        )]
        return format!("~{}", &p[home.len()..]);
    }
    p.to_string()
}

/// Print the JSON Schema for the `Config` type. Optional `--section <wiki|nim>`
/// filters output to just that section so agents can scope schema discovery.
async fn cmd_show_schema(section: Option<String>) -> Result<(), WikiError> {
    let mut schema = schemars::schema_for!(crate::core::config::Config);
    if let Some(sec) = section.as_deref() {
        // `Config` exposes `wiki: WikiConfig` and `nim: NimConfig` (with a
        // nested `RetryConfig` reachable only through `nim`), so the section
        // key maps 1:1 to a `$defs` entry name — except for the extra nested
        // def reachable from `nim`.
        let (key, def): (&str, &[&str]) = match sec {
            "wiki" => ("wiki", &["WikiConfig"]),
            "nim" => ("nim", &["NimConfig", "RetryConfig"]),
            _ => unreachable!("clap value_parser rejects this"),
        };
        // Drop the other section from the top-level properties.
        if let Some(props) = schema
            .as_object_mut()
            .and_then(|o| o.get_mut("properties"))
            .and_then(|p| p.as_object_mut())
        {
            let drop: Vec<String> = props
                .keys()
                .filter(|k| k.as_str() != key)
                .cloned()
                .collect();
            for k in drop {
                props.remove(&k);
            }
        }
        if let Some(req) = schema
            .as_object_mut()
            .and_then(|o| o.get_mut("required"))
            .and_then(|r| r.as_array_mut())
        {
            req.retain(|v| v.as_str() != Some(key));
        }
        // Drop unrelated `$defs` entries so the output actually reflects the
        // requested section (e.g. `--section wiki` no longer leaks `embed_model`
        // through `$defs.NimConfig`).
        if let Some(defs) = schema
            .as_object_mut()
            .and_then(|o| o.get_mut("$defs"))
            .and_then(|d| d.as_object_mut())
        {
            let drop: Vec<String> = defs
                .keys()
                .filter(|k| !def.contains(&k.as_str()))
                .cloned()
                .collect();
            for k in drop {
                defs.remove(&k);
            }
        }
    }
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
                        println!("    {e}");
                    }
                    failures += 1;
                }
            },
            Err(e) => {
                println!("✗ [defaults] — {e}");
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
                        println!("    {e}");
                    }
                    failures += 1;
                }
            },
            Err(e) => {
                println!("✗ [{}] — {}", entry.alias, e);
                failures += 1;
            }
        }

        // Check this alias's config.toml files for unknown keys. serde
        // silently skips unrecognized fields, so a typo like `pages_dirr`
        // would otherwise leave the user thinking their config is correct.
        // Uses `entry.path` (registry-only — no workspace discovery) so
        // `config validate` stays classified as a registry-only subcommand.
        for path in crate::core::config::config_paths(&entry.path) {
            if path.is_file() {
                match crate::core::config::validate_config_file(&path) {
                    Ok(warnings) => {
                        for w in &warnings {
                            eprintln!("[warn] {}: {}", path.display(), w);
                        }
                    }
                    Err(e) => eprintln!("[error] {}: {e}", path.display()),
                }
            }
        }
    }

    if failures == 0 {
        Ok(())
    } else {
        Err(WikiError::Other(anyhow::anyhow!(
            "{failures} wiki(s) failed validation",
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
                        format!("{prefix}{k}")
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
                format!("{prefix}.{k}")
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
        println!("{key} = {value}");
    }
}

/// Print the active wiki: alias, workspace, resolution source, and the
/// registry file the alias was loaded from. Exits 0 even when no wiki
/// resolves — this is a report command, not a do-command, so it should
/// never break user scripts that probe for "is there a wiki here?"
///
/// Mirrors the one-liner printed by `llmwiki-cli` with no subcommand
/// (see `cli::run`), but adds the registry path and supports `--json`.
///
/// `flag_workspace` and `flag_wiki` are the global `--workspace` and
/// `--wiki` flags, threaded through `ConfigCmd::Current` via
/// `from_global`. The resolver combines them with the `WIKI_WORKSPACE`
/// and `WIKI_ACTIVE` env vars, so users can probe "what would
/// `llmwiki-cli --wiki mevin` resolve to?" by running
/// `llmwiki-cli --wiki mevin config current`.
async fn cmd_current(
    flag_workspace: Option<&Path>,
    flag_wiki: Option<&str>,
    json: bool,
) -> Result<(), WikiError> {
    let cwd = std::env::current_dir()?;
    let reg = crate::core::registry::Registry::discover()?;
    let resolved = reg.resolve_active_optional(
        flag_wiki,
        flag_workspace,
        std::env::var("WIKI_ACTIVE").ok().as_deref(),
        std::env::var("WIKI_WORKSPACE").ok().as_deref(),
        &cwd,
    );

    match resolved {
        Some(r) => print_current_resolved(&r, &reg.root_path, json),
        None => print_current_unresolved(&reg.root_path, json),
    }
    Ok(())
}

/// Print a single resolved-wiki report. JSON shape:
///
/// ```json
/// {
///   "alias": "mevin",
///   "workspace": "/Users/.../mevin-tauri2/wiki",
///   "source": "flag_alias",
///   "source_label": "--wiki flag",
///   "registry_file": "/Users/.../.agents/wiki-root.toml"
/// }
/// ```
fn print_current_resolved(r: &ResolvedWiki, registry_path: &std::path::Path, json: bool) {
    if json {
        // `ResolutionSource` is `Serialize` + `#[serde(rename_all = "snake_case")]`
        // so the variant names below are stable machine identifiers.
        let payload = serde_json::json!({
            "alias": r.alias,
            "workspace": r.path,
            "source": r.source,
            "source_label": r.source.label(),
            "registry_file": registry_path,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload)
                .expect("ResolutionSource + PathBuf + String are all JSON-safe")
        );
    } else {
        println!("Active wiki:");
        println!("  alias:     {}", r.alias);
        println!("  workspace: {}", r.path.display());
        println!("  source:    {}", r.source);
        println!("  registry:  {}", registry_path.display());
        println!();
        println!("(Run `llmwiki-cli doctor` to validate; `llmwiki-cli config show-effective` for resolved keys.)");
    }
}

/// Print a "no active wiki" report with hints. Same JSON keys as
/// [`print_current_resolved`] but with `null` for alias/workspace/source,
/// plus a `note` field explaining how to fix it.
fn print_current_unresolved(registry_path: &std::path::Path, json: bool) {
    if json {
        let payload = serde_json::json!({
            "alias": null,
            "workspace": null,
            "source": null,
            "source_label": ResolutionSource::WalkUp.label(), // placeholder for type stability
            "registry_file": registry_path,
            "note": "no active wiki — pass --wiki <alias>, set WIKI_ACTIVE, or cd into a registered wiki path",
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload)
                .expect("JSON serialization of the no-wiki payload is infallible")
        );
    } else {
        println!("No active wiki.");
        println!("  registry:  {}", registry_path.display());
        println!("  hint:      pass --wiki <alias>, set $WIKI_ACTIVE, or cd into a registered wiki path.");
        println!("  hint:      run `llmwiki-cli config list` to see available aliases.");
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
