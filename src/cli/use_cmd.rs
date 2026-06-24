//! `llmwiki-cli use <alias>` — per-workspace active-wiki pointer.
//!
//! Writes the alias to `<workspace>/.llmwiki-cli/state/active-wiki`
//! (single line, just the alias). The resolution chain checks this
//! file as step 5.5 (between env-vars and CWD-prefix match), so users
//! who work on a project with one primary wiki can stop typing
//! `--wiki mevin` for every command.
//!
//! `llmwiki-cli use --unset` removes the pointer.
//!
//! `llmwiki-cli use` with no args prints the current value (if any) and
//! the path of the pointer file — same shape as `llmwiki-cli config current`
//! for the registry side.

use crate::core::registry::Registry;
use crate::error::WikiError;
use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct UseArgs {
    /// Override the workspace used as the walk-up start. `from_global`
    /// receives the value of the top-level `--workspace` flag so
    /// `llmwiki-cli --workspace <ws> use mevin` works.
    #[arg(from_global)]
    pub workspace: Option<PathBuf>,
    /// Select wiki by alias from wiki-root.toml. `from_global`
    /// receives the value of the top-level `--wiki` flag.
    #[arg(from_global)]
    pub wiki: Option<String>,
    /// Alias to set as the workspace's active wiki. Omit (with no
    /// flags) to print the current active-wiki pointer.
    pub alias: Option<String>,
    /// Remove the active-wiki pointer instead of setting it.
    #[arg(long)]
    pub unset: bool,
    /// JSON output
    #[arg(long)]
    pub json: bool,
}

const ACTIVE_WIKI_FILE: &str = ".llmwiki-cli/state/active-wiki";

/// Path of the active-wiki pointer for a given workspace root.
/// Public so the resolution chain in `core::registry` can use it
/// without re-implementing the path convention.
pub fn active_wiki_path(workspace: &std::path::Path) -> std::path::PathBuf {
    workspace.join(ACTIVE_WIKI_FILE)
}

pub fn run(args: UseArgs) -> Result<(), WikiError> {
    // Resolve the workspace the same way the rest of the CLI does, so
    // `llmwiki-cli use mevin` (no --workspace) writes to the active workspace.
    let workspace = crate::core::workspace::discover_workspace(
        args.workspace.clone(),
        args.wiki.as_deref(),
        std::env::var("WIKI_WORKSPACE").ok().map(PathBuf::from),
        std::env::var("WIKI_ACTIVE").ok().as_deref(),
        std::env::current_dir()?,
    )?;

    let pointer_path = active_wiki_path(&workspace);

    // Three modes:
    //   1. --unset (no alias)         → remove the pointer
    //   2. alias provided             → write the alias
    //   3. nothing                    → print the current pointer (if any)
    if args.unset {
        return run_unset(&pointer_path, &workspace, args.json);
    }

    match args.alias {
        Some(alias) => run_set(&workspace, &pointer_path, &alias, args.json),
        None => run_show(&workspace, &pointer_path, args.json),
    }
}

fn run_set(
    workspace: &std::path::Path,
    pointer_path: &std::path::Path,
    alias: &str,
    json: bool,
) -> Result<(), WikiError> {
    // Validate the alias exists in the registry. Without this check,
    // `llmwiki-cli use typo` would silently set a broken pointer that
    // resolves to nothing at next use.
    let reg = Registry::discover()?;
    if !reg.entries.iter().any(|e| e.alias == alias) {
        let available = reg
            .entries
            .iter()
            .map(|e| e.alias.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(WikiError::Other(anyhow::anyhow!(
            "alias '{alias}' is not registered. Available: {available}. \
             Run `llmwiki-cli config add {alias} <path>` to register it, or \
             `llmwiki-cli config list` to see all registered wikis."
        )));
    }

    // Ensure the state/ directory exists.
    if let Some(parent) = pointer_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| WikiError::Other(anyhow::anyhow!("create {}: {e}", parent.display())))?;
    }

    // Write the alias as the sole content of the file. No newline
    // is required for the reader; `trim()` handles either form.
    std::fs::write(pointer_path, alias.as_bytes())
        .map_err(|e| WikiError::Other(anyhow::anyhow!("write {}: {e}", pointer_path.display())))?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "action": "set",
                "alias": alias,
                "workspace": workspace,
                "pointer": pointer_path,
            }))?
        );
    } else {
        println!("✓ Set active wiki for {} to '{alias}'", workspace.display());
        println!("  pointer: {}", pointer_path.display());
        println!();
        println!("(From now on, any `llmwiki-cli` command in this workspace will");
        println!(" resolve to '{alias}' without needing --wiki.)");
        println!(" Override with --wiki, $WIKI_ACTIVE, or `llmwiki-cli use <other>`.");
    }
    Ok(())
}

fn run_unset(
    pointer_path: &std::path::Path,
    workspace: &std::path::Path,
    json: bool,
) -> Result<(), WikiError> {
    let existed = pointer_path.is_file();
    if existed {
        std::fs::remove_file(pointer_path).map_err(|e| {
            WikiError::Other(anyhow::anyhow!("remove {}: {e}", pointer_path.display()))
        })?;
    }

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "action": "unset",
                "alias": null,
                "workspace": workspace,
                "pointer": pointer_path,
                "existed": existed,
            }))?
        );
    } else if existed {
        println!("✓ Removed active-wiki pointer for {}", workspace.display());
    } else {
        println!("✓ No active-wiki pointer to remove (already absent) for {}", workspace.display());
    }
    Ok(())
}

fn run_show(
    workspace: &std::path::Path,
    pointer_path: &std::path::Path,
    json: bool,
) -> Result<(), WikiError> {
    let current = if pointer_path.is_file() {
        std::fs::read_to_string(pointer_path)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    } else {
        None
    };

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "alias": current,
                "workspace": workspace,
                "pointer": pointer_path,
                "is_set": current.is_some(),
            }))?
        );
    } else {
        match &current {
            Some(alias) => {
                println!("Active wiki for this workspace: {alias}");
                println!("  workspace: {}", workspace.display());
                println!("  pointer:   {}", pointer_path.display());
            }
            None => {
                println!("No active wiki set for this workspace.");
                println!("  workspace: {}", workspace.display());
                println!("  hint:      run `llmwiki-cli use <alias>` to set one.");
            }
        }
    }
    Ok(())
}
