use crate::core::config::Config;
use crate::core::registry::{home_dir, Registry};
use crate::error::WikiError;
use std::path::{Path, PathBuf};

/// Canonical default directory basenames excluded from wiki page walks.
///
/// This is the runtime source of truth for exclusion defaults. It is injected
/// by [`config::load_config_unvalidated`] and [`Registry::resolve_config`] so
/// that user-provided `wiki.exclude_dirs` **merges with** (rather than
/// replaces) these entries (v0.3.27+, additive semantics).
///
/// **Keep in sync** with `default_exclude_dirs()` in `src/core/config_types.rs`
/// — that function provides the serde default and JSON Schema `default` for the
/// `exclude_dirs` field. The `default_exclude_dirs_matches_constant` test in
/// [`pages_dir_tests`] guards against drift.
///
/// See the doc comment on `default_exclude_dirs()` for the rationale behind
/// each entry.
pub const DEFAULT_EXCLUDE_DIRS: &[&str] = &[
    // Dev-project noise (qmd + Foam union; cargo-style excludes)
    "node_modules",
    ".git",
    "target",
    "dist",
    "build",
    ".next",
    ".cache",
    ".turbo",
    ".venv",
    "venv",
    "env",
    "__pycache__",
    ".idea",
    ".vscode",
    // Wiki-specific noise (real-wiki smoke test, 2026-06-24)
    ".opencode",
    ".claude",
    ".mavis",
    ".harness",
    ".serena",
    ".principled",
    ".swe-bench",
];

/// Merge canonical default exclusion dirs into `cfg.wiki.exclude_dirs`.
///
/// User-provided entries are preserved; any default entry not already present
/// is appended (dedup). This implements the additive semantics introduced in
/// v0.3.27: a user who sets `exclude_dirs = ["secret"]` retains all built-in
/// defaults (`node_modules`, `.git`, `.opencode`, …) rather than silently
/// replacing them.
///
/// Called from both config-load paths — [`load_config_unvalidated`] and
/// [`Registry::resolve_config`] — so additive merging is consistent regardless
/// of whether the workspace is resolved via the registry or the config-file
/// chain.
pub fn merge_exclude_defaults(cfg: &mut Config) {
    for default in DEFAULT_EXCLUDE_DIRS {
        if !cfg.wiki.exclude_dirs.iter().any(|x| x == *default) {
            cfg.wiki.exclude_dirs.push(default.to_string());
        }
    }
}

/// Discover the wiki workspace root.
///
/// Priority order:
/// 1. `--workspace <path>` flag
/// 2. `--wiki <alias>` flag (looks up path in registry)
/// 3. `WIKI_WORKSPACE` env var
/// 4. `WIKI_ACTIVE` env var (looks up path in registry)
/// 5. wiki-root.toml registry (CWD match against registered paths)
/// 6. Walk up from CWD looking for `.llmwiki-cli/` directory (skip HOME)
/// 7. Single-wiki shortcut (registry has exactly one entry)
pub fn discover_workspace(
    flag: Option<PathBuf>,
    wiki_alias: Option<&str>,
    env: Option<PathBuf>,
    env_active: Option<&str>,
    cwd: PathBuf,
) -> Result<PathBuf, WikiError> {
    if let Some(p) = flag {
        return Ok(p.canonicalize().unwrap_or(p));
    }

    // Try wiki alias from flag
    if let Some(alias) = wiki_alias {
        if let Ok(reg) = Registry::discover() {
            if let Some(entry) = reg.entries.iter().find(|e| e.alias == alias) {
                return Ok(entry.path.clone());
            }
        }
    }

    if let Some(p) = env {
        return Ok(p.canonicalize().unwrap_or(p));
    }

    // Try WIKI_ACTIVE
    if let Some(alias) = env_active {
        if let Ok(reg) = Registry::discover() {
            if let Some(entry) = reg.entries.iter().find(|e| e.alias == alias) {
                return Ok(entry.path.clone());
            }
        }
    }

    // Try registry CWD match
    if let Ok(reg) = Registry::discover() {
        for entry in &reg.entries {
            if cwd.starts_with(&entry.path) {
                return Ok(entry.path.clone());
            }
        }
    }

    // Walk up from CWD looking for `.llmwiki-cli/` (skip HOME so
    // `~/.llmwiki-cli/` is treated as per-computer config, not a workspace).
    if let Some(p) = walk_up_for_llmwiki_cli_dir(&cwd) {
        // Per-workspace active-wiki pointer (`llmwiki-cli use <alias>`): look
        // up the alias in the registry and return ITS path, not the workspace
        // marker. The pointer is the user's explicit "this project uses THAT
        // wiki" signal — it should override walk-up so the resolution chain
        // stays consistent with `Registry::resolve_active`.
        let pointer = p.join(".llmwiki-cli").join("state").join("active-wiki");
        if pointer.is_file() {
            if let Ok(content) = std::fs::read_to_string(&pointer) {
                let alias = content.trim();
                if !alias.is_empty() {
                    if let Ok(reg) = Registry::discover() {
                        if let Some(entry) = reg.entries.iter().find(|e| e.alias == alias) {
                            return Ok(entry.path.clone());
                        }
                    }
                }
            }
        }
        return Ok(p);
    }

    // Single-wiki shortcut: if the registry has exactly one entry and we
    // haven't matched anything else, default to it. Avoids forcing the user
    // to pass `--wiki` for a single-wiki install.
    if let Ok(reg) = Registry::discover() {
        if reg.entries.len() == 1 {
            return Ok(reg.entries[0].path.clone());
        }
    }

    Err(WikiError::WorkspaceNotFound)
}

/// Walk up from `start` collecting the closest ancestor containing a
/// `.llmwiki-cli/` directory. Skips the user's HOME so `~/.llmwiki-cli/`
/// is treated as the per-computer config location, not as a workspace marker.
fn walk_up_for_llmwiki_cli_dir(start: &Path) -> Option<PathBuf> {
    walk_up_for_llmwiki_cli_dir_public(start)
}

/// Public wrapper used by [`crate::core::registry::Registry::resolve_active`]
/// for the walk-up resolution step. Kept separate from the private helper
/// above so callers can't accidentally rely on a non-public function while
/// the internal implementation may still move (e.g. add an `Option<bool>`
/// flag for "skip HOME" in a future change).
pub(crate) fn walk_up_for_llmwiki_cli_dir_public(start: &Path) -> Option<PathBuf> {
    let canonical = start.canonicalize().ok()?;
    let home_canon = home_dir().and_then(|h| h.canonicalize().ok());
    let mut current: Option<PathBuf> = Some(canonical);
    while let Some(dir) = current {
        if let Some(ref h) = home_canon {
            if dir == *h {
                current = dir.parent().map(PathBuf::from);
                continue;
            }
        }
        if dir.join(".llmwiki-cli").is_dir() {
            return Some(dir);
        }
        current = dir.parent().map(PathBuf::from);
    }
    None
}

/// Resolve the wiki pages directory for a workspace.
///
/// Returns the workspace itself when `wiki.pages_dir` is empty
/// (flat-layout Karpathy-style wikis where pages live at the workspace
/// root), or `workspace.join(pages_dir)` otherwise. Used by `ls --pages`,
/// `tree`, `embed`, `lint --scope wiki`, and `status` — six call sites
/// that previously hardcoded `ws.join("wiki")` and broke on flat-layout
/// wikis.
///
/// v0.3.25+: surfaced by the pre-release real-wiki smoke test
/// (see `AGENTS.md` "Pre-release real-wiki smoke test").
pub fn pages_dir(workspace: &Path, pages_dir_config: &str) -> PathBuf {
    if pages_dir_config.is_empty() {
        workspace.to_path_buf()
    } else {
        workspace.join(pages_dir_config)
    }
}

/// Walk a directory looking for wiki pages, skipping excluded directories.
///
/// `root` is typically the result of [`pages_dir`]. `exclude_dirs` is a
/// list of bare directory basenames (not paths) to skip at any depth.
/// Matching is case-sensitive against `entry.file_name()`. When an entry
/// matches, `filter_entry` returns `false`, which causes walkdir to skip
/// both the entry AND its descendants.
///
/// Only file entries are yielded (directories are walked for descent but
/// not returned — callers want pages, not the directory tree).
///
/// Why a helper: v0.3.26+ replaces six call sites in `src/cli/{ls,tree,
/// embed,lint,status}.rs` that previously used `walkdir::WalkDir::new(...)
/// directly. Centralising the filter keeps the exclusion semantics in one
/// place and matches the `walk_pages` semantics the AGENTS.md pre-release
/// real-wiki smoke test asserts on (filter `node_modules/`, `.opencode/`,
/// `.harness/`, etc.).
///
/// Returns an iterator of `walkdir::Result<walkdir::DirEntry>`. Callers
/// currently propagate errors via `entry.map_err(|e| anyhow::anyhow!(e))?`
/// (lint, tree, embed, ls) — a single walkdir error aborts the command.
/// If a future caller needs resilience (skip a single unreadable dir
/// rather than abort), use `.filter_map(|e| e.ok())` instead.
pub fn walk_pages<'a>(
    root: &'a Path,
    exclude_dirs: &'a [String],
) -> impl Iterator<Item = walkdir::Result<walkdir::DirEntry>> + 'a {
    walkdir::WalkDir::new(root)
        .into_iter()
        .filter_entry(move |entry| {
            // Always allow the root entry itself (matches walkdir's expectation
            // that filter_entry returns true for the root on first call).
            if entry.path() == root {
                return true;
            }
            // Skip excluded directory basenames at any depth. `entry.file_name()`
            // returns the last component — for `node_modules` at
            // `/tmp/ws/node_modules/foo`, it returns `OsStr("foo")`. For the
            // directory itself at `/tmp/ws/node_modules`, it returns
            // `OsStr("node_modules")`.
            if entry.file_type().is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    if exclude_dirs.iter().any(|x| x == name) {
                        return false;
                    }
                }
            }
            true
        })
        .filter(|e| e.as_ref().ok().is_some_and(|e| e.file_type().is_file()))
}

/// Returns the relative path from `base` to `entry_path` as a
/// `String` with forward slashes. Returns `None` if `entry_path`
/// is not under `base` (e.g. via a symlink that escaped the
/// workspace). The forward-slash conversion is the agent-friendly
/// format — `walkdir` returns native separators (backslash on
/// Windows), but `llmwiki-cli ls`, `llmwiki-cli tree`, and `llmwiki-cli embed` all
/// report paths with `/` regardless of OS so the output is
/// stable across machines and shell-pipable.
///
/// Used by `ls`, `tree`, `embed`, and `status` to replace the
/// `strip_prefix(ws).unwrap().to_string_lossy().replace('\\', "/")`
/// pattern that used to panic if a symlink escaped the workspace
/// (a real risk for `walk_pages` when `wiki.exclude_dirs` is
/// misconfigured and a symlinked dir points at `/etc`, for
/// example). The new helper degrades gracefully: a path that
/// can't be relativized is dropped from the listing rather than
/// crashing the whole command.
pub fn rel_path(base: &Path, entry_path: &Path) -> Option<String> {
    let rel = entry_path.strip_prefix(base).ok()?;
    Some(rel.to_string_lossy().replace('\\', "/"))
}

/// Filter helper for the 5 read-path commands (ls, tree, embed, lint, status).
/// Returns `true` if `entry_path` should be treated as a wiki page; `false`
/// if it should be skipped (project metadata, raw source, operation log, etc.).
///
/// Filters three categories:
/// - Workspace-root `index.md` and `log.md`: the wiki registry and
///   chronological operation log. They live at the workspace root by
///   convention (both pre-v0.3.26 in `wiki/` and v0.3.26+ flat at the
///   root). Filtered only at the workspace root — a subdirectory's
///   `index.md` (e.g. `research/decompilation/index.md`) is a legitimate
///   entry-point page and is kept.
/// - Anything under `raw/`: raw sources handled by `--scope raw`, not
///   by the page walks. Detected via the entry's path containing a
///   `raw/` component relative to the workspace root.
/// - `wiki.exclude_dirs` is handled by `walk_pages` itself via
///   `filter_entry`, so this helper doesn't re-check it.
///
/// `workspace_root` is the resolved workspace (`ws`); `entry_path` is
/// typically `entry.path()` from a `walkdir::DirEntry`. If `entry_path`
/// isn't under `workspace_root` (e.g. an absolute path outside the wiki),
/// the helper defaults to including it — callers should not feed in
/// paths from elsewhere.
pub fn is_wiki_page_entry(workspace_root: &Path, entry_path: &Path) -> bool {
    let rel = match entry_path.strip_prefix(workspace_root) {
        Ok(r) => r,
        Err(_) => return true,
    };
    let components: Vec<_> = rel.components().collect();
    if components.len() == 1 {
        if let Some(name) = entry_path.file_name().and_then(|n| n.to_str()) {
            if name == "index.md" || name == "log.md" {
                return false;
            }
        }
    }
    for component in &components {
        if let std::path::Component::Normal(os) = component {
            if os.to_str() == Some("raw") {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod pages_dir_tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn default_subdir_returns_workspace_join_wiki() {
        let ws = PathBuf::from("/tmp/wiki");
        let result = pages_dir(&ws, "wiki");
        assert_eq!(result, PathBuf::from("/tmp/wiki/wiki"));
    }

    #[test]
    fn empty_string_returns_workspace_root_for_flat_layout() {
        let ws = PathBuf::from("/Users/felix/Documents/MinimaxCode/minimax-code-wiki");
        let result = pages_dir(&ws, "");
        assert_eq!(result, ws);
    }

    #[test]
    fn custom_subdir_is_honored() {
        let ws = PathBuf::from("/tmp/wiki");
        let result = pages_dir(&ws, "pages");
        assert_eq!(result, PathBuf::from("/tmp/wiki/pages"));
    }

    #[test]
    fn nested_subdir_is_preserved() {
        let ws = PathBuf::from("/tmp/wiki");
        let result = pages_dir(&ws, "content/pages");
        assert_eq!(result, PathBuf::from("/tmp/wiki/content/pages"));
    }

    use std::fs;

    #[test]
    fn walk_pages_returns_all_md_files_when_no_excludes() {
        let tmp = tempfile::tempdir().unwrap();
        let ws = tmp.path();
        fs::create_dir_all(ws.join("a/b")).unwrap();
        fs::write(ws.join("a/x.md"), "x").unwrap();
        fs::write(ws.join("a/b/y.md"), "y").unwrap();
        let mut paths: Vec<String> = walk_pages(ws, &[])
            .filter_map(|e| e.ok())
            .map(|e| {
                e.path()
                    .strip_prefix(ws)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect();
        paths.sort();
        assert_eq!(paths, vec!["a/b/y.md".to_string(), "a/x.md".to_string()]);
    }

    #[test]
    fn walk_pages_skips_excluded_dir_and_does_not_descend() {
        let tmp = tempfile::tempdir().unwrap();
        let ws = tmp.path();
        fs::create_dir_all(ws.join("node_modules/pkg")).unwrap();
        fs::write(ws.join("node_modules/pkg/lib.md"), "noise").unwrap();
        fs::write(ws.join("keep.md"), "real").unwrap();
        let excludes = vec!["node_modules".to_string()];
        let paths: Vec<String> = walk_pages(ws, &excludes)
            .filter_map(|e| e.ok())
            .map(|e| {
                e.path()
                    .strip_prefix(ws)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect();
        assert_eq!(paths, vec!["keep.md".to_string()]);
    }

    #[test]
    fn walk_pages_matches_dotted_dir_basename() {
        let tmp = tempfile::tempdir().unwrap();
        let ws = tmp.path();
        fs::create_dir_all(ws.join(".opencode/scratch")).unwrap();
        fs::write(ws.join(".opencode/scratch/page.md"), "noise").unwrap();
        fs::write(ws.join("real.md"), "ok").unwrap();
        let excludes = vec![".opencode".to_string()];
        let paths: Vec<String> = walk_pages(ws, &excludes)
            .filter_map(|e| e.ok())
            .map(|e| {
                e.path()
                    .strip_prefix(ws)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect();
        assert_eq!(paths, vec!["real.md".to_string()]);
    }

    #[test]
    fn is_wiki_page_entry_filters_workspace_root_index_and_log() {
        let ws = Path::new("/tmp/wiki");
        assert!(!is_wiki_page_entry(ws, &ws.join("index.md")));
        assert!(!is_wiki_page_entry(ws, &ws.join("log.md")));
    }

    #[test]
    fn is_wiki_page_entry_keeps_subdirectory_index_and_log() {
        // A `research/decompilation/index.md` is a legitimate entry-point page;
        // only workspace-root `index.md`/`log.md` are filtered.
        let ws = Path::new("/tmp/wiki");
        assert!(is_wiki_page_entry(
            ws,
            &ws.join("research/decompilation/index.md")
        ));
        assert!(is_wiki_page_entry(ws, &ws.join("research/notes/log.md")));
    }

    #[test]
    fn is_wiki_page_entry_filters_raw_at_any_depth() {
        let ws = Path::new("/tmp/wiki");
        assert!(!is_wiki_page_entry(ws, &ws.join("raw/foo.md")));
        assert!(!is_wiki_page_entry(ws, &ws.join("raw/articles/x.md")));
        assert!(!is_wiki_page_entry(ws, &ws.join("nested/raw/y.md")));
    }

    #[test]
    fn is_wiki_page_entry_keeps_real_pages() {
        let ws = Path::new("/tmp/wiki");
        assert!(is_wiki_page_entry(ws, &ws.join("foo.md")));
        assert!(is_wiki_page_entry(ws, &ws.join("comparisons/foo.md")));
        assert!(is_wiki_page_entry(ws, &ws.join("AGENTS.md")));
        assert!(is_wiki_page_entry(ws, &ws.join("README.md")));
    }

    #[test]
    fn is_wiki_page_entry_returns_true_for_paths_outside_workspace() {
        // Defensive default: callers should not pass these, but the helper
        // doesn't fault on them.
        let ws = Path::new("/tmp/wiki");
        assert!(is_wiki_page_entry(ws, Path::new("/other/foo.md")));
    }

    #[test]
    fn default_exclude_dirs_matches_constant() {
        // Guards against drift between DEFAULT_EXCLUDE_DIRS (the runtime
        // source for additive merging) and default_exclude_dirs() (the serde
        // default + JSON Schema default).
        let from_fn = crate::core::config::default_exclude_dirs();
        let from_const: Vec<String> = DEFAULT_EXCLUDE_DIRS.iter().map(|s| s.to_string()).collect();
        assert_eq!(
            from_fn, from_const,
            "DEFAULT_EXCLUDE_DIRS and default_exclude_dirs() drifted out of sync"
        );
    }
}
