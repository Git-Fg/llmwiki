use crate::core::config::Config;
use crate::core::registry::Registry;
use crate::core::workspace::pages_dir;
use crate::error::WikiError;
use anyhow::Context;
use std::path::PathBuf;

pub struct InitArgs {
    pub path: PathBuf,
    pub alias: Option<String>,
    pub flat: bool,
    pub subdir: bool,
    pub tags: Vec<String>,
}

pub fn run(args: InitArgs) -> Result<(), WikiError> {
    let target = if args.path.exists() && args.path.is_dir() {
        args.path.clone()
    } else {
        std::fs::create_dir_all(&args.path)
            .with_context(|| format!("create {}", args.path.display()))?;
        args.path.clone()
    };

    // v0.3.26+: scaffold default is FLAT (matches read-path default).
    // Use --subdir to get the legacy wiki/ subdir layout.
    // (v0.3.27 added additive wiki.exclude_dirs on top of this default.)
    let mut cfg =
        crate::core::config::load_config_unvalidated(&crate::core::config::config_paths(&target))
            .unwrap_or_else(|_| Config::default());

    // --subdir flag forces legacy subdir layout
    if args.subdir {
        cfg.wiki.pages_dir = "wiki".into();
    } else if args.flat {
        cfg.wiki.pages_dir = String::new();
    }
    // If neither flag: use whatever's in config (flat default = ""),
    // or if user explicitly set pages_dir in config, honor that.

    let pages_dir_path = pages_dir(&target, &cfg.wiki.pages_dir);

    // For flat layout, pages_dir_path is workspace root (already exists).
    // For subdir layout, create the subdirectory.
    if !cfg.wiki.pages_dir.is_empty() {
        std::fs::create_dir_all(&pages_dir_path)
            .with_context(|| format!("create {}", pages_dir_path.display()))?;
    }
    std::fs::create_dir_all(target.join("raw/articles")).context("create raw/articles/")?;
    std::fs::create_dir_all(target.join(".llmwiki-cli")).context("create .llmwiki-cli/")?;

    let today = std::process::Command::new("date")
        .arg("+%Y-%m-%d")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "2026-06-21".to_string());

    let template = include_str!("../../resources/page.template.md").replace("YYYY-MM-DD", &today);

    write_if_absent(&pages_dir_path.join("overview.md"), &template)?;
    write_if_absent(
        &pages_dir_path.join("log.md"),
        "# Log\n\nChronological record of wiki operations.\n",
    )?;
    write_if_absent(&target.join("raw/articles/.gitkeep"), "")?;
    write_if_absent(
        &target.join("index.md"),
        "# Index\n\n## Pages\n\nNo pages yet.\n",
    )?;
    write_if_absent(
        &target.join(".gitignore"),
        // `.llmwiki-cli/state/` is written by `wiki use <alias>` and
        // holds the per-workspace active-wiki pointer — it's session
        // state, not config, and must not be committed (mirrors how
        // `embeddings.jsonl` is generated locally per device).
        "embeddings.jsonl\n.env\n.env.local\n.llmwiki-cli/state/\n",
    )?;

    // Per-workspace config (v0.3.7). Safe to commit to the wiki repo so
    // team members share the same NIM/wiki settings for this workspace.
    // Keys default to `~/.llmwiki-cli/config.toml`; uncomment here to
    // override per-workspace.
    let workspace_config = "\
# Per-workspace llmwiki-cli config.
# Edit values here to override the per-computer ~/.llmwiki-cli/config.toml.
# Safe to commit to the wiki repo for team sharing.

[nim]
# base_url = \"https://integrate.api.nvidia.com\"
# embed_model = \"nvidia/nv-embedqa-e5-v5\"
# batch_size = 8

[wiki]
# pages_dir = \"\"            # relative to workspace root; \"wiki\" = legacy subdir layout (v0.3.26 default was \"wiki\", now \"\")
# exclude_dirs = [\"node_modules\", \".git\", \".opencode\", ...]  # bare basenames skipped at any depth (v0.3.26+)
# default_chunk_tokens = 512
# chunk_overlap_tokens = 128
# min_chunk_tokens = 32
";
    write_if_absent(&target.join(".llmwiki-cli/config.toml"), workspace_config)?;

    // Initialize git repo if not already one
    if !target.join(".git").exists() {
        std::process::Command::new("git")
            .arg("init")
            .arg(&target)
            .output()
            .context("git init")?;
    }

    // Auto-register in wiki-root.toml
    let alias = args
        .alias
        .clone()
        .or_else(|| target.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "wiki".to_string());

    let reg = Registry::discover().or_else(|_| {
        // No registry exists; create one at the conventional user-global slot.
        // `~/.agents/wiki-root.toml` is the highest-priority user-global
        // path and the conventional location for AI agent config (parallel
        // to `~/.agents/skills/wiki/`). Picking the lowest-priority
        // `~/wiki-root.toml` would create shadowing confusion later.
        let default_path = crate::core::registry::home_dir()
            .map(|h| h.join(".agents").join("wiki-root.toml"))
            .ok_or_else(|| {
                WikiError::Other(anyhow::anyhow!(
                    "cannot determine home directory: both $HOME and $USERPROFILE are unset. \
                     Set one of them, or set WIKI_ROOT_CONFIG to the wiki-root.toml you want to use."
                ))
            })?;
        if let Some(parent) = default_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&default_path, "# wiki-root.toml\n")?;
        Registry::load_from(&default_path)
    })?;

    let mut reg = reg;
    if let Err(e) = reg.add_entry(&alias, &target, &args.tags, None) {
        eprintln!("Warning: {e}");
    } else {
        reg.save()?;
        println!("Registered wiki '{alias}' in wiki-root.toml");
    }

    let layout_label = if args.flat || cfg.wiki.pages_dir.is_empty() {
        "flat (pages at workspace root)"
    } else {
        "subdirectory"
    };
    let config_path = target.join(".llmwiki-cli").join("config.toml");
    println!("✓ Initialized wiki at {}", target.display());
    println!("  Layout: {layout_label}");
    println!("  Config: {}", config_path.display());
    Ok(())
}

fn write_if_absent(path: &std::path::Path, content: &str) -> Result<(), WikiError> {
    if !path.exists() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(path, content).map_err(WikiError::Io)?;
    }
    Ok(())
}
