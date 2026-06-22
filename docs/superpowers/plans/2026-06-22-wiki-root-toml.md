# Wiki Config — wiki-root.toml Centralization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace `.wiki/config.yaml` with `wiki-root.toml` as the single config source, add `wiki config` commands, restructure skills into proper `SKILL.md` folders.

**Architecture:** New `src/core/registry.rs` loads `wiki-root.toml` with deep-merge of `[defaults]` + per-alias overrides. All CLI commands call `registry.resolve_active()` instead of `discover_workspace()`. Skills migrate from monolithic `skill_md.md` + `topics/` to hub `WIKI.md` + sub-skill folders each with `SKILL.md`.

**Tech Stack:** Rust, clap (derive), serde, toml, serde_toml_merge (new), wiremock (tests), tokio.

**Spec:** `docs/superpowers/specs/2026-06-22-wiki-root-toml-design.md`

---

## Phase 1: Registry + Config Commands

Foundation phase. At the end: `wiki config path/list/get/set/unset/add/rm/edit` work. Existing commands still use old discovery (safe).

### Task 1: Add serde_toml_merge dependency

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add dependency**

In `Cargo.toml` `[dependencies]` section, add:
```toml
serde_toml_merge = "0.2"
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build`
Expected: compiles successfully, new crate downloads.

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "deps: add serde_toml_merge for TOML deep-merge"
```

---

### Task 2: Add new WikiError variants

**Files:**
- Modify: `src/error.rs`

- [ ] **Step 1: Write the failing test**

In `tests/error_test.rs`, add:
```rust
#[test]
fn wiki_root_not_found_displays_message() {
    let err = wiki::error::WikiError::WikiRootNotFound {
        searched: vec![
            std::path::PathBuf::from("/home/user/.agents/wiki-root.toml"),
            std::path::PathBuf::from("/home/user/.claude/wiki-root.toml"),
        ],
    };
    let msg = format!("{}", err);
    assert!(msg.contains("wiki-root.toml"));
    assert!(msg.contains("not found"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test error_test wiki_root_not_found`
Expected: FAIL — variant does not exist.

- [ ] **Step 3: Add error variants**

The existing `WikiError` already has `ConfigInvalid { path: String, line: usize, message: String }` and `Io(#[from] std::io::Error)`. We need to add two new variants and keep using the existing `ConfigInvalid` for parse errors:

In `src/error.rs`, add to the `WikiError` enum:
```rust
#[error("wiki-root.toml not found in any of: {searched:?}")]
WikiRootNotFound { searched: Vec<std::path::PathBuf> },

#[error("wiki alias '{alias}' not found in registry. Available: {available}")]
AliasNotFound { alias: String, available: String },
```

Note: The existing `ConfigInvalid { path: String, line: usize, message: String }` is reused for TOML parse errors (set `line: 0` when not applicable). When constructing `ConfigInvalid` in registry code, use `.to_string()` on path values and `0` for line when unknown.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test error_test wiki_root_not_found`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/error.rs tests/error_test.rs
git commit -m "error: add WikiRootNotFound, ConfigInvalid, AliasNotFound variants"
```

---

### Task 3: Create core/registry.rs — types and load()

**Files:**
- Create: `src/core/registry.rs`
- Modify: `src/core/mod.rs`
- Create: `tests/registry_test.rs`

- [ ] **Step 1: Write the failing test**

Create `tests/registry_test.rs`:
```rust
use std::io::Write;
use wiki::core::registry::Registry;

fn write_tmp_toml(content: &str) -> std::path::PathBuf {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("wiki-root.toml");
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
    // Leak the dir so the file persists for the test
    std::mem::forget(dir);
    path
}

#[test]
fn load_parses_defaults_and_entries() {
    let path = write_tmp_toml(r#"
[defaults.nim]
embed_model = "nvidia/nv-embed-v1"
base_url = "https://integrate.api.nvidia.com"

[defaults.wiki]
default_chunk_tokens = 512

[mywiki]
path = "/tmp/mywiki"
tags = ["test"]
description = "Test wiki"
"#);
    let reg = Registry::load_from(&path).unwrap();
    assert!(reg.defaults.is_some());
    assert_eq!(reg.entries.len(), 1);
    assert_eq!(reg.entries[0].alias, "mywiki");
    assert_eq!(reg.entries[0].path, std::path::PathBuf::from("/tmp/mywiki"));
}

#[test]
fn load_returns_empty_registry_for_missing_file() {
    let result = Registry::load_from(std::path::Path::new("/nonexistent/wiki-root.toml"));
    assert!(result.is_err());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test registry_test`
Expected: FAIL — module does not exist.

- [ ] **Step 3: Create registry.rs with types and load_from()**

Create `src/core/registry.rs`:
```rust
use crate::core::config::{Config, NimConfig, WikiConfig, RetryConfig};
use crate::error::WikiError;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::collections::HashMap;

/// A single wiki entry in the registry.
#[derive(Debug, Clone)]
pub struct WikiEntry {
    pub alias: String,
    pub path: PathBuf,
    pub tags: Vec<String>,
    pub description: String,
    pub what_to_read: Vec<String>,
    pub qmd_slug: Option<String>,
    /// Raw TOML table for this alias (for merge purposes)
    pub raw: toml::Value,
}

/// Global defaults applied to all wikis.
#[derive(Debug, Clone, Default)]
pub struct WikiDefaults {
    /// Raw TOML table for [defaults]
    pub raw: Option<toml::Value>,
}

/// The loaded wiki-root.toml registry.
#[derive(Debug, Clone)]
pub struct Registry {
    /// Path to the wiki-root.toml file
    pub root_path: PathBuf,
    /// Parsed [defaults] section
    pub defaults: WikiDefaults,
    /// All registered wiki aliases
    pub entries: Vec<WikiEntry>,
    /// Raw parsed TOML document (for set/unset operations)
    pub raw_doc: toml::Value,
}

/// Intermediate struct for parsing wiki-root.toml top-level.
#[derive(Debug, Deserialize)]
struct RootFile {
    #[serde(default)]
    defaults: Option<toml::Value>,
    // All other keys are wiki aliases (flattened)
    #[serde(flatten)]
    aliases: HashMap<String, toml::Value>,
}

impl Registry {
    /// Load wiki-root.toml from a specific path.
    pub fn load_from(path: &Path) -> Result<Self, WikiError> {
        let content = std::fs::read_to_string(path).map_err(|_| {
            WikiError::WikiRootNotFound {
                searched: vec![path.to_path_buf()],
            }
        })?;

        let raw_doc: toml::Value = content
            .parse()
            .map_err(|e| WikiError::ConfigInvalid {
                path: path.to_path_buf(),
                message: format!("TOML parse error: {}", e),
            })?;

        let parsed: RootFile = toml::from_str(&content).map_err(|e| {
            WikiError::ConfigInvalid {
                path: path.to_path_buf(),
                message: format!("TOML parse error: {}", e),
            }
        })?;

        let defaults = WikiDefaults {
            raw: parsed.defaults,
        };

        let entries = parsed
            .aliases
            .into_iter()
            .filter_map(|(alias, val)| {
                let table = val.as_table()?;
                // Skip non-table values
                if table.contains_key("path") || table.contains_key("description") {
                    let path = table
                        .get("path")
                        .and_then(|v| v.as_str())
                        .map(PathBuf::from)
                        .unwrap_or_default();
                    let tags = table
                        .get("tags")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default();
                    let description = table
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let what_to_read = table
                        .get("what_to_read")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default();
                    let qmd_slug = table
                        .get("qmd_slug")
                        .and_then(|v| v.as_str())
                        .map(String::from);

                    Some(WikiEntry {
                        alias,
                        path,
                        tags,
                        description,
                        what_to_read,
                        qmd_slug,
                        raw: val,
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(Registry {
            root_path: path.to_path_buf(),
            defaults,
            entries,
            raw_doc,
        })
    }

    /// Discover and load wiki-root.toml using the priority order.
    pub fn discover() -> Result<Self, WikiError> {
        let candidates = Self::candidate_paths();
        for candidate in &candidates {
            if candidate.exists() {
                return Self::load_from(candidate);
            }
        }
        Err(WikiError::WikiRootNotFound {
            searched: candidates,
        })
    }

    /// Get the list of candidate wiki-root.toml paths in priority order.
    pub fn candidate_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // 1. $WIKI_ROOT_CONFIG
        if let Ok(p) = std::env::var("WIKI_ROOT_CONFIG") {
            paths.push(PathBuf::from(p));
        }

        if let Some(home) = home_dir() {
            // 2. ~/.agents/wiki-root.toml
            paths.push(home.join(".agents").join("wiki-root.toml"));
            // 3. ~/.claude/wiki-root.toml
            paths.push(home.join(".claude").join("wiki-root.toml"));
            // 4. ~/wiki-root.toml
            paths.push(home.join("wiki-root.toml"));
        }

        paths
    }
}

/// Read $HOME on Unix, $USERPROFILE on Windows. No external deps.
fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}
```

- [ ] **Step 4: Add registry module to core/mod.rs**

In `src/core/mod.rs`, add:
```rust
pub mod registry;
```

- [ ] **Step 5: Add tempfile to Cargo.toml dev-deps**

In `Cargo.toml` `[dev-dependencies]`:
```toml
tempfile = "3"
```

No `dirs` dependency — `candidate_paths()` reads `$HOME` / `$USERPROFILE` directly from `std::env::var_os`.

- [ ] **Step 6: Run test to verify it passes**

Run: `cargo test --test registry_test`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add src/core/registry.rs src/core/mod.rs Cargo.toml Cargo.lock tests/registry_test.rs
git commit -m "feat(registry): add wiki-root.toml loader with type parsing"
```

---

### Task 4: Implement config deep-merge (resolve_config)

**Files:**
- Modify: `src/core/registry.rs`
- Modify: `tests/registry_test.rs`

- [ ] **Step 1: Write the failing test**

Append to `tests/registry_test.rs`:
```rust
#[test]
fn resolve_config_merges_defaults_and_overrides() {
    let path = write_tmp_toml(r#"
[defaults.nim]
embed_model = "nvidia/nv-embed-v1"
base_url = "https://integrate.api.nvidia.com"

[defaults.wiki]
default_chunk_tokens = 512

[pharma]
path = "/tmp/pharma"
description = "Pharma wiki"

[pharma.nim]
embed_model = "nvidia/nv-embedqa-e5-v5"
"#);
    let reg = Registry::load_from(&path).unwrap();
    let cfg = reg.resolve_config("pharma").unwrap();

    // Override should win
    assert_eq!(cfg.nim.embed_model, "nvidia/nv-embedqa-e5-v5");
    // Default should fill in
    assert_eq!(cfg.nim.base_url, "https://integrate.api.nvidia.com");
    // Wiki defaults preserved
    assert_eq!(cfg.wiki.default_chunk_tokens, 512);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test registry_test resolve_config`
Expected: FAIL — method does not exist.

- [ ] **Step 3: Implement resolve_config()**

Add to `impl Registry` in `src/core/registry.rs`:
```rust
/// Resolve the merged Config for a given alias.
/// Deep-merges [defaults] with [alias] overrides.
pub fn resolve_config(&self, alias: &str) -> Result<Config, WikiError> {
    let entry = self.entries.iter().find(|e| e.alias == alias).ok_or_else(|| {
        WikiError::AliasNotFound {
            alias: alias.to_string(),
            available: self
                .entries
                .iter()
                .map(|e| e.alias.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        }
    })?;

    // Start with defaults (or empty table)
    let defaults_table = self
        .defaults
        .raw
        .as_ref()
        .and_then(|v| v.as_table())
        .cloned()
        .unwrap_or_default();

    // Extract alias-specific override sections
    let alias_table = entry.raw.as_table().cloned().unwrap_or_default();

    // Merge: defaults first, then alias overrides
    let mut merged: toml::value::Table = defaults_table;

    // Deep merge alias keys into defaults
    for (key, value) in &alias_table {
        if key == "path" || key == "tags" || key == "description"
            || key == "what_to_read" || key == "qmd_slug"
        {
            continue; // skip metadata fields
        }
        // Deep merge: if both are tables, recurse; otherwise override
        if let Some(existing) = merged.get_mut(key) {
            deep_merge_into(existing, value.clone());
        } else {
            merged.insert(key.clone(), value.clone());
        }
    }

    let merged_value = toml::Value::Table(merged);

    // Deserialize into Config
    let cfg: Config = merged_value
        .try_into()
        .map_err(|e| WikiError::ConfigInvalid {
            path: self.root_path.clone(),
            message: format!("Failed to deserialize merged config: {}", e),
        })?;

    Ok(cfg)
}
```

Add the deep_merge_into helper function:
```rust
/// Recursively merge `src` into `dst`. Tables recurse, scalars override, arrays concatenate.
fn deep_merge_into(dst: &mut toml::Value, src: toml::Value) {
    match (dst, src) {
        (toml::Value::Table(dst_table), toml::Value::Table(src_table)) => {
            for (key, value) in src_table {
                if let Some(existing) = dst_table.get_mut(&key) {
                    deep_merge_into(existing, value);
                } else {
                    dst_table.insert(key, value);
                }
            }
        }
        (toml::Value::Array(dst_arr), toml::Value::Array(src_arr)) => {
            for item in src_arr {
                if !dst_arr.contains(&item) {
                    dst_arr.push(item);
                }
            }
        }
        (dst, src) => {
            *dst = src; // scalar override
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test registry_test resolve_config`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/core/registry.rs tests/registry_test.rs
git commit -m "feat(registry): implement deep-merge config resolution"
```

---

### Task 5: Implement resolve_active() discovery

**Files:**
- Modify: `src/core/registry.rs`
- Modify: `tests/registry_test.rs`

- [ ] **Step 1: Write the failing test**

Append to `tests/registry_test.rs`:
```rust
#[test]
fn resolve_active_cwd_prefix_match() {
    let path = write_tmp_toml(r#"
[mywiki]
path = "/tmp/mywiki"
description = "Test"
"#);
    let reg = Registry::load_from(&path).unwrap();

    // CWD inside a registered path should match
    let (alias, _, _) = reg
        .resolve_active(None, None, None, None, std::path::Path::new("/tmp/mywiki/wiki/sub"))
        .unwrap();
    assert_eq!(alias, "mywiki");
}

#[test]
fn resolve_active_single_wiki_shortcut() {
    let path = write_tmp_toml(r#"
[solo]
path = "/tmp/solo"
description = "Solo"
"#);
    let reg = Registry::load_from(&path).unwrap();

    // CWD doesn't match, but only 1 wiki → use it
    let (alias, _, _) = reg
        .resolve_active(None, None, None, None, std::path::Path::new("/etc"))
        .unwrap();
    assert_eq!(alias, "solo");
}

#[test]
fn resolve_active_flag_alias_wins() {
    let path = write_tmp_toml(r#"
[wiki1]
path = "/tmp/wiki1"
description = "One"

[wiki2]
path = "/tmp/wiki2"
description = "Two"
"#);
    let reg = Registry::load_from(&path).unwrap();

    let (alias, _, _) = reg
        .resolve_active(Some("wiki2"), None, None, None, std::path::Path::new("/tmp/wiki1"))
        .unwrap();
    assert_eq!(alias, "wiki2");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test registry_test resolve_active`
Expected: FAIL — method does not exist.

- [ ] **Step 3: Implement resolve_active()**

Add to `impl Registry` in `src/core/registry.rs`:
```rust
/// Resolve which wiki is active.
/// Returns (alias, workspace_path, merged_config).
///
/// Priority order (first match wins):
/// 1. flag_path (--workspace)
/// 2. flag_alias (--wiki)
/// 3. env WIKI_WORKSPACE
/// 4. env WIKI_ACTIVE
/// 5. CWD prefix match against registry paths
/// 6. Single-wiki shortcut
/// 7. Error
pub fn resolve_active(
    &self,
    flag_alias: Option<&str>,
    flag_path: Option<&Path>,
    env_alias: Option<&str>,
    env_path: Option<&str>,
    cwd: &Path,
) -> Result<(String, PathBuf, Config), WikiError> {
    // 1. --workspace <path>
    if let Some(p) = flag_path {
        let alias = self
            .entries
            .iter()
            .find(|e| e.path == p)
            .map(|e| e.alias.clone())
            .unwrap_or_else(|| p.file_name().unwrap_or_default().to_string_lossy().to_string());
        let cfg = self.resolve_config(&alias).unwrap_or_default();
        return Ok((alias, p.to_path_buf(), cfg));
    }

    // 2. --wiki <alias>
    if let Some(alias) = flag_alias {
        return self.resolve_by_alias(alias);
    }

    // 3. WIKI_WORKSPACE
    if let Some(p) = env_path {
        let path = PathBuf::from(p);
        let alias = self
            .entries
            .iter()
            .find(|e| e.path == path)
            .map(|e| e.alias.clone())
            .unwrap_or_else(|| {
                path.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string()
            });
        let cfg = self.resolve_config(&alias).unwrap_or_default();
        return Ok((alias, path, cfg));
    }

    // 4. WIKI_ACTIVE
    if let Some(alias) = env_alias {
        return self.resolve_by_alias(alias);
    }

    // 5. CWD prefix match
    for entry in &self.entries {
        if cwd.starts_with(&entry.path) {
            let cfg = self.resolve_config(&entry.alias)?;
            return Ok((entry.alias.clone(), entry.path.clone(), cfg));
        }
    }

    // 6. Single-wiki shortcut
    if self.entries.len() == 1 {
        let entry = &self.entries[0];
        let cfg = self.resolve_config(&entry.alias)?;
        return Ok((entry.alias.clone(), entry.path.clone(), cfg));
    }

    // 7. No match
    Err(WikiError::AliasNotFound {
        alias: format!("CWD={}", cwd.display()),
        available: self
            .entries
            .iter()
            .map(|e| e.alias.as_str())
            .collect::<Vec<_>>()
            .join(", "),
    })
}

/// Resolve by alias name.
fn resolve_by_alias(&self, alias: &str) -> Result<(String, PathBuf, Config), WikiError> {
    let entry = self.entries.iter().find(|e| e.alias == alias).ok_or_else(|| {
        WikiError::AliasNotFound {
            alias: alias.to_string(),
            available: self
                .entries
                .iter()
                .map(|e| e.alias.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        }
    })?;
    let cfg = self.resolve_config(&entry.alias)?;
    Ok((entry.alias.clone(), entry.path.clone(), cfg))
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test registry_test resolve_active`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/core/registry.rs tests/registry_test.rs
git commit -m "feat(registry): implement 7-step active wiki discovery"
```

---

### Task 6: Add --wiki global flag to Cli

**Files:**
- Modify: `src/cli/mod.rs`

- [ ] **Step 1: Add --wiki flag**

In `src/cli/mod.rs`, in the `Cli` struct, add alongside the existing `--workspace`:
```rust
/// Select wiki by alias from wiki-root.toml
#[arg(long, global = true)]
pub wiki: Option<String>,
```

- [ ] **Step 2: Verify it compiles and help shows it**

Run: `cargo build && cargo run -- --help 2>&1 | grep wiki`
Expected: shows `--wiki <WIKI>` in help output.

- [ ] **Step 3: Commit**

```bash
git add src/cli/mod.rs
git commit -m "feat(cli): add --wiki global flag for alias selection"
```

---

### Task 7: Add Config command and subcommand enum

**Files:**
- Modify: `src/cli/mod.rs`
- Create: `src/cli/config.rs`

- [ ] **Step 1: Add ConfigCmd enum and dispatch**

In `src/cli/mod.rs`, add to `Command` enum:
```rust
/// Manage wiki-root.toml configuration
Config {
    #[command(subcommand)]
    cmd: ConfigCmd,
},
```

Add the `ConfigCmd` enum (before the `Command` enum or after it):
```rust
#[derive(clap::Subcommand, Debug)]
pub enum ConfigCmd {
    /// Print the resolved wiki-root.toml file path
    Path,
    /// List all wikis or show merged config for a specific wiki
    List {
        /// Show config for this alias
        #[arg(long)]
        wiki: Option<String>,
        /// JSON output
        #[arg(long)]
        json: bool,
    },
    /// Get a config value by dotted key
    Get {
        /// e.g. nim.embed_model
        key: String,
        /// Wiki alias (defaults to [defaults])
        #[arg(long)]
        wiki: Option<String>,
    },
    /// Set a config value by dotted key
    Set {
        /// e.g. nim.embed_model
        key: String,
        /// e.g. nvidia/nv-embed-v1
        value: String,
        /// Wiki alias (defaults to [defaults])
        #[arg(long)]
        wiki: Option<String>,
    },
    /// Remove a config override (revert to default)
    Unset {
        /// e.g. nim.embed_model
        key: String,
        /// Wiki alias
        #[arg(long)]
        wiki: Option<String>,
    },
    /// Register a new wiki
    Add {
        /// Wiki alias
        alias: String,
        /// Path to wiki directory
        path: std::path::PathBuf,
        /// Tags (repeatable)
        #[arg(long = "tag", value_name = "TAG")]
        tags: Vec<String>,
        /// Description
        #[arg(long)]
        description: Option<String>,
    },
    /// Remove a wiki from the registry
    Rm {
        /// Wiki alias to remove
        alias: String,
    },
    /// Open wiki-root.toml in $EDITOR
    Edit,
}
```

- [ ] **Step 2: Create cli/config.rs with stub dispatch**

Create `src/cli/config.rs`:
```rust
use crate::cli::ConfigCmd;
use crate::error::WikiError;

pub async fn run(cmd: ConfigCmd) -> Result<(), WikiError> {
    match cmd {
        ConfigCmd::Path => cmd_path().await,
        ConfigCmd::List { wiki, json } => cmd_list(wiki.as_deref(), json).await,
        ConfigCmd::Get { key, wiki } => cmd_get(&key, wiki.as_deref()).await,
        ConfigCmd::Set { key, value, wiki } => cmd_set(&key, &value, wiki.as_deref()).await,
        ConfigCmd::Unset { key, wiki } => cmd_unset(&key, wiki.as_deref()).await,
        ConfigCmd::Add { alias, path, tags, description } => {
            cmd_add(&alias, &path, &tags, description.as_deref()).await
        }
        ConfigCmd::Rm { alias } => cmd_rm(&alias).await,
        ConfigCmd::Edit => cmd_edit().await,
    }
}

async fn cmd_path() -> Result<(), WikiError> {
    let reg = crate::core::registry::Registry::discover()?;
    println!("{}", reg.root_path.display());
    Ok(())
}

// TODO: implement in subsequent tasks
async fn cmd_list(_wiki: Option<&str>, _json: bool) -> Result<(), WikiError> {
    todo!("Task 8")
}
async fn cmd_get(_key: &str, _wiki: Option<&str>) -> Result<(), WikiError> {
    todo!("Task 9")
}
async fn cmd_set(_key: &str, _value: &str, _wiki: Option<&str>) -> Result<(), WikiError> {
    todo!("Task 10")
}
async fn cmd_unset(_key: &str, _wiki: Option<&str>) -> Result<(), WikiError> {
    todo!("Task 11")
}
async fn cmd_add(_alias: &str, _path: &std::path::Path, _tags: &[String], _desc: Option<&str>) -> Result<(), WikiError> {
    todo!("Task 12")
}
async fn cmd_rm(_alias: &str) -> Result<(), WikiError> {
    todo!("Task 13")
}
async fn cmd_edit() -> Result<(), WikiError> {
    todo!("Task 14")
}
```

- [ ] **Step 3: Add config module to cli/mod.rs dispatch**

In `src/cli/mod.rs`, in the `run()` function, add to the match:
```rust
Command::Config { cmd } => crate::cli::config::run(cmd).await,
```

Add at top of cli/mod.rs:
```rust
pub mod config;
```

- [ ] **Step 4: Verify wiki config path works**

Run: `cargo build && cargo run -- config path`
Expected: prints the resolved wiki-root.toml path (or error if not found).

Run: `cargo run -- config --help`
Expected: shows all subcommands.

- [ ] **Step 5: Commit**

```bash
git add src/cli/mod.rs src/cli/config.rs
git commit -m "feat(cli): add wiki config command family with path subcommand"
```

---

### Task 8: Implement wiki config list

**Files:**
- Modify: `src/cli/config.rs`

- [ ] **Step 1: Implement cmd_list**

Replace the `cmd_list` stub in `src/cli/config.rs`:
```rust
async fn cmd_list(wiki: Option<&str>, json: bool) -> Result<(), WikiError> {
    let reg = crate::core::registry::Registry::discover()?;

    match wiki {
        Some(alias) => {
            // Show merged config for specific wiki
            let cfg = reg.resolve_config(alias)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&cfg).unwrap());
            } else {
                let value = config_to_value(&cfg);
                print_value_dotted(&value, "");
            }
        }
        None => {
            // List all wikis
            if json {
                let entries: Vec<_> = reg
                    .entries
                    .iter()
                    .map(|e| serde_json::json!({
                        "alias": e.alias,
                        "path": e.path,
                        "tags": e.tags,
                        "description": e.description,
                    }))
                    .collect();
                println!("{}", serde_json::to_string_pretty(&entries).unwrap());
            } else {
                if reg.entries.is_empty() {
                    println!("No wikis registered. Use 'wiki config add <alias> <path>' to add one.");
                    return Ok(());
                }
                println!("{:<15} {:<40} {}", "ALIAS", "PATH", "TAGS");
                for entry in &reg.entries {
                    let tags = entry.tags.join(", ");
                    println!(
                        "{:<15} {:<40} {}",
                        entry.alias,
                        entry.path.display(),
                        tags
                    );
                }
            }
        }
    }
    Ok(())
}

/// Serialize `Config` to a `toml::Value` so we can navigate it reflectively.
/// New fields on `Config` are picked up automatically — no per-field code.
fn config_to_value(cfg: &crate::core::config::Config) -> toml::Value {
    toml::Value::try_from(cfg).expect("Config serialization to TOML is infallible")
}

/// Print a TOML value tree as `key = value` lines, recursing into tables with
/// dot-separated prefixes (e.g. `nim.retry.max_attempts = 3`).
fn print_value_dotted(value: &toml::Value, prefix: &str) {
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
                print_value_dotted(v, &full);
            } else {
                println!("{} = {}", full, format_value(v));
            }
        }
    }
}

fn format_value(v: &toml::Value) -> String {
    match v {
        toml::Value::String(s) => s.clone(),
        toml::Value::Integer(i) => i.to_string(),
        toml::Value::Boolean(b) => b.to_string(),
        toml::Value::Array(arr) => {
            let parts: Vec<String> = arr.iter().map(format_value).collect();
            format!("[{}]", parts.join(", "))
        }
        _ => "<table>".to_string(),
    }
}
```

- [ ] **Step 2: Add serde_json dependency if missing**

Check `Cargo.toml` — if `serde_json` is not in `[dependencies]`, add:
```toml
serde_json = "1"
```

- [ ] **Step 3: Verify it works**

Run: `cargo run -- config list`
Expected: table of all registered wikis.

Run: `cargo run -- config list --wiki pharma`
Expected: dotted-key config for pharma.

- [ ] **Step 4: Commit**

```bash
git add src/cli/config.rs Cargo.toml
git commit -m "feat(cli): implement wiki config list"
```

---

### Task 9: Implement wiki config get

**Files:**
- Modify: `src/cli/config.rs`

- [ ] **Step 1: Implement cmd_get**

Replace the `cmd_get` stub. Uses reflective `navigate()` over the TOML value tree so adding new config fields requires no code changes.

```rust
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

/// Walk a dotted key path (`nim.retry.max_attempts`) through a TOML value tree.
/// Unknown keys error with a dynamic list of valid keys.
fn navigate(root: &toml::Value, key: &str) -> Result<String, WikiError> {
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
                let valid: Vec<String> = table.keys().map(|k| {
                    let mut prefix = parts[..i].join(".");
                    if !prefix.is_empty() { prefix.push('.'); }
                    format!("{}{}", prefix, k)
                }).collect();
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
```

Note: `WikiError::ConfigInvalid` has the shape `{ path: String, line: usize, message: String }` — the plan originally wrote `path: PathBuf` / `message: String`. The final error uses three fields.

Also: `Registry::resolve_defaults()` was added in Phase 2 of this plan to satisfy `cmd_get` (no `--wiki`) reading `[defaults]` from the registry rather than `Config::default()`. Without it, `[defaults].nim.embed_model` overrides would be invisible to `wiki config get <key>`.

- [ ] **Step 2: Verify it works**

Run: `cargo run -- config get nim.embed_model`
Expected: prints the embed model from defaults.

Run: `cargo run -- config get nim.embed_model --wiki pharma`
Expected: prints merged value for pharma.

- [ ] **Step 3: Commit**

```bash
git add src/cli/config.rs
git commit -m "feat(cli): implement wiki config get"
```

---

### Task 10: Implement wiki config set (atomic write)

**Files:**
- Modify: `src/core/registry.rs` (add save method)
- Modify: `src/cli/config.rs`

- [ ] **Step 1: Implement Registry::save() with atomic write**

Add to `impl Registry` in `src/core/registry.rs`:
```rust
/// Persist changes back to wiki-root.toml (atomic write).
pub fn save(&self) -> Result<(), WikiError> {
    let content = toml::to_string_pretty(&self.raw_doc).map_err(|e| {
        WikiError::ConfigInvalid {
            path: self.root_path.clone(),
            message: format!("TOML serialization error: {}", e),
        }
    })?;

    // Atomic write: write to tmp, then rename
    let tmp_path = self.root_path.with_extension("toml.tmp");
    std::fs::write(&tmp_path, &content).map_err(|e| WikiError::Io(e))?;
    std::fs::rename(&tmp_path, &self.root_path).map_err(|e| WikiError::Io(e))?;
    Ok(())
}

/// Set a dotted key value in the raw TOML document.
/// If wiki_alias is Some, writes to [alias].<key>; otherwise writes to [defaults].<key>.
pub fn set_value(
    &mut self,
    key: &str,
    value: &str,
    wiki_alias: Option<&str>,
) -> Result<(), WikiError> {
    let parts: Vec<&str> = key.split('.').collect();
    let section_key = wiki_alias.unwrap_or("defaults");

    // Navigate/create the table path in raw_doc
    let doc = &mut self.raw_doc;
    if let toml::Value::Table(root) = doc {
        // Get or create the section table
        let section = root
            .entry(section_key.to_string())
            .or_insert_with(|| toml::Value::Table(toml::value::Table::new()));

        if let toml::Value::Table(section_table) = section {
            // Navigate dotted path within the section
            let mut current = section_table;
            for (i, part) in parts.iter().enumerate() {
                if i == parts.len() - 1 {
                    // Last part: set the value
                    let parsed_value = parse_toml_value(value);
                    current.insert(part.to_string(), parsed_value);
                } else {
                    // Intermediate: get or create sub-table
                    let next = current
                        .entry(part.to_string())
                        .or_insert_with(|| toml::Value::Table(toml::value::Table::new()));
                    if let toml::Value::Table(t) = next {
                        current = t;
                    } else {
                        return Err(WikiError::Other(anyhow::anyhow!(
                            "cannot navigate into non-table key '{}'",
                            part
                        )));
                    }
                }
            }
        } else {
            return Err(WikiError::Other(anyhow::anyhow!(
                "section '{}' is not a table",
                section_key
            )));
        }
    }

    Ok(())
}

/// Parse a string value into the appropriate TOML type.
fn parse_toml_value(s: &str) -> toml::Value {
    // Try as integer
    if let Ok(i) = s.parse::<i64>() {
        return toml::Value::Integer(i);
    }
    // Try as boolean
    if s == "true" {
        return toml::Value::Boolean(true);
    }
    if s == "false" {
        return toml::Value::Boolean(false);
    }
    // Default: string
    toml::Value::String(s.to_string())
}
```

Note: Add `Io(std::io::Error)` variant to `WikiError` if it doesn't exist.

- [ ] **Step 2: Implement cmd_set**

Replace the `cmd_set` stub in `src/cli/config.rs`:
```rust
async fn cmd_set(key: &str, value: &str, wiki: Option<&str>) -> Result<(), WikiError> {
    let mut reg = crate::core::registry::Registry::discover()?;
    reg.set_value(key, value, wiki)?;
    reg.save()?;
    let target = wiki.unwrap_or("defaults");
    println!("Set {} = {} in [{}]", key, value, target);
    Ok(())
}
```

- [ ] **Step 3: Verify it works**

Run: `cargo run -- config set nim.embed_model nvidia/test-model --wiki pharma`
Expected: writes `[pharma.nim] embed_model = "nvidia/test-model"` to wiki-root.toml.

Run: `cargo run -- config get nim.embed_model --wiki pharma`
Expected: prints `nvidia/test-model`.

- [ ] **Step 4: Commit**

```bash
git add src/core/registry.rs src/cli/config.rs src/error.rs
git commit -m "feat(cli): implement wiki config set with atomic write"
```

---

### Task 11: Implement wiki config unset

**Files:**
- Modify: `src/core/registry.rs`
- Modify: `src/cli/config.rs`

- [ ] **Step 1: Add unset_value to Registry**

Add to `impl Registry` in `src/core/registry.rs`:
```rust
/// Remove a dotted key from a wiki alias section.
pub fn unset_value(&mut self, key: &str, wiki_alias: &str) -> Result<(), WikiError> {
    let parts: Vec<&str> = key.split('.').collect();
    let doc = &mut self.raw_doc;

    if let toml::Value::Table(root) = doc {
        if let Some(toml::Value::Table(section)) = root.get_mut(wiki_alias) {
            // Navigate to parent of the last key
            let mut current = section;
            for (i, part) in parts.iter().enumerate() {
                if i == parts.len() - 1 {
                    current.remove(*part);
                    return Ok(());
                } else {
                    match current.get_mut(*part) {
                        Some(toml::Value::Table(t)) => current = t,
                        _ => {
                            return Err(WikiError::Other(anyhow::anyhow!(
                                "key '{}' not found in [{}]",
                                key, wiki_alias
                            )))
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
```

- [ ] **Step 2: Implement cmd_unset**

Replace the `cmd_unset` stub:
```rust
async fn cmd_unset(key: &str, wiki: Option<&str>) -> Result<(), WikiError> {
    let alias = wiki.ok_or_else(|| {
        WikiError::Other(anyhow::anyhow!(
            "--wiki <alias> is required for unset"
        ))
    })?;
    let mut reg = crate::core::registry::Registry::discover()?;
    reg.unset_value(key, alias)?;
    reg.save()?;
    println!("Unset {} from [{}]", key, alias);
    Ok(())
}
```

- [ ] **Step 3: Verify it works**

Run: `cargo run -- config unset nim.embed_model --wiki pharma`
Expected: removes override, value reverts to defaults.

- [ ] **Step 4: Commit**

```bash
git add src/core/registry.rs src/cli/config.rs
git commit -m "feat(cli): implement wiki config unset"
```

---

### Task 12: Implement wiki config add

**Files:**
- Modify: `src/core/registry.rs`
- Modify: `src/cli/config.rs`

- [ ] **Step 1: Add add_entry to Registry**

Add to `impl Registry`:
```rust
/// Add a new wiki entry to the registry.
pub fn add_entry(
    &mut self,
    alias: &str,
    path: &Path,
    tags: &[String],
    description: Option<&str>,
) -> Result<(), WikiError> {
    if self.entries.iter().any(|e| e.alias == alias) {
        return Err(WikiError::Other(anyhow::anyhow!(
            "alias '{}' already exists. Use 'wiki config rm {}' first.",
            alias, alias
        )));
    }

    let mut table = toml::value::Table::new();
    table.insert("path".to_string(), toml::Value::String(path.display().to_string()));
    if !tags.is_empty() {
        table.insert(
            "tags".to_string(),
            toml::Value::Array(tags.iter().map(|t| toml::Value::String(t.clone())).collect()),
        );
    }
    if let Some(desc) = description {
        table.insert("description".to_string(), toml::Value::String(desc.to_string()));
    }

    if let toml::Value::Table(root) = &mut self.raw_doc {
        root.insert(alias.to_string(), toml::Value::Table(table));
    }

    // Update entries list
    self.entries.push(WikiEntry {
        alias: alias.to_string(),
        path: path.to_path_buf(),
        tags: tags.to_vec(),
        description: description.unwrap_or("").to_string(),
        what_to_read: vec![],
        qmd_slug: None,
        raw: toml::Value::Table(toml::value::Table::new()),
    });

    Ok(())
}
```

- [ ] **Step 2: Implement cmd_add**

Replace the `cmd_add` stub:
```rust
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
```

- [ ] **Step 3: Verify**

Run: `cargo run -- config add testwiki /tmp/test --tag test`
Expected: adds entry, prints confirmation.

Run: `cargo run -- config list`
Expected: shows new entry.

Run: `cargo run -- config rm testwiki`
Expected: removes it (implemented next).

- [ ] **Step 4: Commit**

```bash
git add src/core/registry.rs src/cli/config.rs
git commit -m "feat(cli): implement wiki config add"
```

---

### Task 13: Implement wiki config rm

**Files:**
- Modify: `src/core/registry.rs`
- Modify: `src/cli/config.rs`

- [ ] **Step 1: Add remove_entry to Registry**

Add to `impl Registry`:
```rust
/// Remove a wiki entry from the registry.
pub fn remove_entry(&mut self, alias: &str) -> Result<(), WikiError> {
    if !self.entries.iter().any(|e| e.alias == alias) {
        return Err(WikiError::AliasNotFound {
            alias: alias.to_string(),
            available: self.entries.iter().map(|e| e.alias.as_str()).collect::<Vec<_>>().join(", "),
        });
    }

    if let toml::Value::Table(root) = &mut self.raw_doc {
        root.remove(alias);
    }

    self.entries.retain(|e| e.alias != alias);
    Ok(())
}
```

- [ ] **Step 2: Implement cmd_rm**

Replace the `cmd_rm` stub:
```rust
async fn cmd_rm(alias: &str) -> Result<(), WikiError> {
    let mut reg = crate::core::registry::Registry::discover()?;
    reg.remove_entry(alias)?;
    reg.save()?;
    println!("Removed wiki '{}'", alias);
    Ok(())
}
```

- [ ] **Step 3: Verify**

Run: `cargo run -- config add tmpwiki /tmp/wiki && cargo run -- config rm tmpwiki`
Expected: adds then removes cleanly.

- [ ] **Step 4: Commit**

```bash
git add src/core/registry.rs src/cli/config.rs
git commit -m "feat(cli): implement wiki config rm"
```

---

### Task 14: Implement wiki config edit

**Files:**
- Modify: `src/cli/config.rs`

- [ ] **Step 1: Implement cmd_edit**

Replace the `cmd_edit` stub:
```rust
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
```

- [ ] **Step 2: Verify**

Run: `EDITOR=echo cargo run -- config edit`
Expected: echoes the wiki-root.toml path.

- [ ] **Step 3: Commit**

```bash
git add src/cli/config.rs
git commit -m "feat(cli): implement wiki config edit"
```

---

### Task 15: Write config CLI integration tests

**Files:**
- Create: `tests/config_cli_test.rs`

- [ ] **Step 1: Write integration tests**

Create `tests/config_cli_test.rs`:
```rust
use std::io::Write;

fn write_tmp_toml(content: &str) -> (std::path::PathBuf, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("wiki-root.toml");
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
    (path, dir)
}

#[test]
fn config_path_prints_resolved_path() {
    let (path, _dir) = write_tmp_toml("[test]\npath = \"/tmp\"\n");
    // This tests that Registry::discover can find a file at a given path
    let reg = wiki::core::registry::Registry::load_from(&path).unwrap();
    assert_eq!(reg.root_path, path);
}

#[test]
fn config_list_shows_all_wikis() {
    let (path, _dir) = write_tmp_toml(r#"
[wiki1]
path = "/tmp/wiki1"
tags = ["a"]

[wiki2]
path = "/tmp/wiki2"
tags = ["b"]
"#);
    let reg = wiki::core::registry::Registry::load_from(&path).unwrap();
    assert_eq!(reg.entries.len(), 2);
}

#[test]
fn config_set_then_get_roundtrip() {
    let (path, _dir) = write_tmp_toml(r#"
[defaults.nim]
embed_model = "original"

[testwiki]
path = "/tmp/test"
"#);
    let mut reg = wiki::core::registry::Registry::load_from(&path).unwrap();
    reg.set_value("nim.embed_model", "new-model", Some("testwiki")).unwrap();
    reg.save().unwrap();

    // Reload and verify
    let reg2 = wiki::core::registry::Registry::load_from(&path).unwrap();
    let cfg = reg2.resolve_config("testwiki").unwrap();
    assert_eq!(cfg.nim.embed_model, "new-model");
}

#[test]
fn config_add_then_remove() {
    let (path, _dir) = write_tmp_toml("[existing]\npath = \"/tmp\"\n");
    let mut reg = wiki::core::registry::Registry::load_from(&path).unwrap();
    assert_eq!(reg.entries.len(), 1);

    reg.add_entry("newwiki", std::path::Path::new("/tmp/new"), &["tag1".to_string()], Some("desc")).unwrap();
    assert_eq!(reg.entries.len(), 2);

    reg.remove_entry("newwiki").unwrap();
    assert_eq!(reg.entries.len(), 1);
}

#[test]
fn config_unset_reverts_to_default() {
    let (path, _dir) = write_tmp_toml(r#"
[defaults.nim]
embed_model = "default-model"

[testwiki]
path = "/tmp/test"

[testwiki.nim]
embed_model = "override-model"
"#);
    let mut reg = wiki::core::registry::Registry::load_from(&path).unwrap();
    let cfg = reg.resolve_config("testwiki").unwrap();
    assert_eq!(cfg.nim.embed_model, "override-model");

    reg.unset_value("nim.embed_model", "testwiki").unwrap();
    reg.save().unwrap();

    let reg2 = wiki::core::registry::Registry::load_from(&path).unwrap();
    let cfg2 = reg2.resolve_config("testwiki").unwrap();
    assert_eq!(cfg2.nim.embed_model, "default-model");
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test --test config_cli_test`
Expected: all PASS.

- [ ] **Step 3: Commit**

```bash
git add tests/config_cli_test.rs
git commit -m "test: add config CLI integration tests"
```

---

## Phase 2: Discovery + Init Refactor

### Task 16: Create resolve helper for commands

**Files:**
- Modify: `src/core/registry.rs`

- [ ] **Step 1: Add convenience resolve() that reads env vars**

Add to `impl Registry`:
```rust
/// Convenience method: resolve active wiki from env + flags.
/// Reads WIKI_WORKSPACE and WIKI_ACTIVE from the environment.
pub fn resolve_from_env_and_flags(
    flag_alias: Option<&str>,
    flag_path: Option<&Path>,
) -> Result<(String, PathBuf, crate::core::config::Config), WikiError> {
    let env_workspace = std::env::var("WIKI_WORKSPACE").ok();
    let env_active = std::env::var("WIKI_ACTIVE").ok();
    let cwd = std::env::current_dir().map_err(|e| {
        WikiError::Other(anyhow::anyhow!("cannot get CWD: {}", e))
    })?;
    self.resolve_active(
        flag_alias,
        flag_path,
        env_active.as_deref(),
        env_workspace.as_deref(),
        &cwd,
    )
}
```

- [ ] **Step 2: Commit**

```bash
git add src/core/registry.rs
git commit -m "feat(registry): add resolve_from_env_and_flags convenience method"
```

---

### Task 17: Refactor wiki ls to use registry

**Files:**
- Modify: `src/cli/ls.rs`

- [ ] **Step 1: Replace discover_workspace with registry**

In `src/cli/ls.rs`, replace the workspace discovery logic with:
```rust
let reg = crate::core::registry::Registry::discover()?;
let (alias, workspace, config) = reg.resolve_from_env_and_flags(
    args.wiki.as_deref(),
    args.workspace.as_deref(),
)?;
```

Remove any `use crate::core::workspace::discover_workspace` imports.

Add `--wiki` flag pass-through from the global Cli if ls doesn't already receive it.

- [ ] **Step 2: Update ls --config to use registry**

The `--config` section should call `reg.resolve_config(&alias)` and print dotted keys.

- [ ] **Step 3: Run tests**

Run: `cargo test --test ls_test`
Expected: PASS (update test expectations if needed).

- [ ] **Step 4: Commit**

```bash
git add src/cli/ls.rs
git commit -m "refactor(ls): use registry.resolve_from_env_and_flags"
```

---

### Task 18-25: Refactor remaining commands

For each of these commands, replace `discover_workspace(...)` / `resolve_config(...)` with:
```rust
let reg = crate::core::registry::Registry::discover()?;
let (alias, workspace, config) = reg.resolve_from_env_and_flags(
    cli.wiki.as_deref(),
    cli.workspace.as_deref(),
)?;
```

Commands to refactor (one commit each):
- **Task 18:** `src/cli/search.rs` — update wiremock tests
- **Task 19:** `src/cli/query.rs`
- **Task 20:** `src/cli/embed.rs`
- **Task 21:** `src/cli/lint.rs`
- **Task 22:** `src/cli/ingest.rs`
- **Task 23:** `src/cli/tree.rs` and `src/cli/status.rs`
- **Task 24:** `src/cli/models.rs` and `src/cli/build.rs`

Each task: modify the file, update tests, commit.

---

### Task 25: Update wiki doctor

**Files:**
- Modify: `src/cli/doctor.rs`

- [ ] **Step 1: Add registry info to doctor output**

Add to the doctor output:
```rust
// Report wiki-root.toml path
let reg = crate::core::registry::Registry::discover();
match &reg {
    Ok(r) => {
        println!("wiki_root_path: {}", r.root_path.display());
        println!("registry_entries: {}", r.entries.len());
    }
    Err(e) => {
        println!("wiki_root_path: NOT FOUND ({})", e);
    }
}
```

Add active alias to output when discoverable.

- [ ] **Step 2: Update doctor_test.rs**

Update `tests/doctor_test.rs` to verify the new output fields.

- [ ] **Step 3: Commit**

```bash
git add src/cli/doctor.rs tests/doctor_test.rs
git commit -m "feat(doctor): report wiki_root_path and registry_entries"
```

---

### Task 26: Refactor wiki init — remove .wiki/ and add auto-register

**Files:**
- Modify: `src/cli/init.rs`
- Modify: `src/cli/mod.rs` (add --alias, --tag flags)
- Modify: `tests/init_test.rs`

- [ ] **Step 1: Update init Args struct**

In the Init args (either in `src/cli/init.rs` or `src/cli/mod.rs`), add:
```rust
/// Wiki alias for wiki-root.toml registration
#[arg(long)]
pub alias: Option<String>,

/// Tags for this wiki (repeatable)
#[arg(long = "tag", value_name = "TAG")]
pub tags: Vec<String>,
```

- [ ] **Step 2: Remove .wiki/ creation**

In `src/cli/init.rs`, remove:
- The `DEFAULT_CONFIG` constant
- Any `create_dir_all(.wiki)` calls
- Any `.wiki/config.yaml` write logic

- [ ] **Step 3: Add auto-registration**

At the end of init, after the wiki directory is created:
```rust
// Auto-register in wiki-root.toml
let alias = match &args.alias {
    Some(a) => a.clone(),
    None => path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "wiki".to_string()),
};

let mut reg = crate::core::registry::Registry::discover()
    .unwrap_or_else(|_| {
        // Create a new registry at ~/.agents/wiki-root.toml
        let default_path = crate::core::registry::Registry::candidate_paths()
            .into_iter()
            .next()
            .unwrap();
        // Ensure parent dir exists
        if let Some(parent) = default_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&default_path, "# wiki-root.toml\n");
        crate::core::registry::Registry::load_from(&default_path).unwrap()
    });

let _ = reg.add_entry(&alias, path, &args.tags, None);
let _ = reg.save();
println!("Registered wiki '{}' in wiki-root.toml", alias);
```

- [ ] **Step 4: Update init_test.rs**

Remove tests that check for `.wiki/config.yaml`. Add tests that verify auto-registration.

- [ ] **Step 5: Run tests**

Run: `cargo test --test init_test`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src/cli/init.rs src/cli/mod.rs tests/init_test.rs
git commit -m "refactor(init): remove .wiki/ creation, add --alias/--tag, auto-register"
```

---

### Task 27: Remove old config.rs YAML loading

**Files:**
- Modify: `src/core/config.rs`

- [ ] **Step 1: Remove load_config and resolve_config**

Remove the following from `src/core/config.rs`:
- `pub fn load_config(paths: &[PathBuf]) -> Result<Config, WikiError>` (or similar)
- `pub fn resolve_config(workspace: &Path) -> Result<Config, WikiError>` (or similar)
- Any serde_yaml usage for config loading

Keep:
- `Config`, `NimConfig`, `WikiConfig`, `RetryConfig` structs (used by registry.rs)
- `resolve_api_key()` function (used by NIM calls)

- [ ] **Step 2: Add Default impl for Config if missing**

Ensure `Config` implements `Default`:
```rust
impl Default for Config {
    fn default() -> Self {
        Config {
            nim: NimConfig::default(),
            wiki: WikiConfig::default(),
        }
    }
}
```

- [ ] **Step 3: Verify build**

Run: `cargo build`
Expected: compiles. If any command still references removed functions, update them.

- [ ] **Step 4: Commit**

```bash
git add src/core/config.rs
git commit -m "refactor(config): remove YAML config loading, keep only types and resolve_api_key"
```

---

### Task 28: Remove workspace.rs

**Files:**
- Delete: `src/core/workspace.rs`
- Modify: `src/core/mod.rs`

- [ ] **Step 1: Remove workspace module**

Delete `src/core/workspace.rs`.

In `src/core/mod.rs`, remove:
```rust
pub mod workspace;
```

- [ ] **Step 2: Fix any remaining references**

Run: `cargo build 2>&1 | grep workspace`
Fix any compilation errors by replacing with registry calls.

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "refactor: remove workspace.rs, fully replaced by registry.rs"
```

---

## Phase 3: Skill Bundle Restructure

### Task 29: Create SETUP/SKILL.md

**Files:**
- Create: `src/skills/SETUP/SKILL.md`
- Source: `src/skills/topics/setup.md`

- [ ] **Step 1: Create directory and file**

Create `src/skills/SETUP/SKILL.md` with frontmatter and updated content from `topics/setup.md`:
- Add YAML frontmatter (name, description, whenToUse, allowed-tools)
- Replace all `.wiki/config.yaml` references with `wiki config` commands
- Add `wiki init --alias <name> --tag <tag>` usage
- Add `wiki config add/list/path` documentation
- Add `wiki install-skill --global` for skill setup

- [ ] **Step 2: Commit**

```bash
git add src/skills/SETUP/SKILL.md
git commit -m "skill: migrate setup.md to SETUP/SKILL.md with frontmatter"
```

---

### Task 30-34: Create remaining sub-skill SKILL.md files

For each, create the directory, migrate content from `topics/`, add frontmatter, update references:

- **Task 30:** `src/skills/INGEST/SKILL.md` — from `topics/ingest.md`, add `wiki config add` reference
- **Task 31:** `src/skills/SEARCH/SKILL.md` — from `topics/search.md`, add `--wiki` note
- **Task 32:** `src/skills/QUERY/SKILL.md` — from `topics/query.md`, add `--wiki` note
- **Task 33:** `src/skills/LINT/SKILL.md` — from `topics/lint.md`, replace YAML config examples with dotted-key TOML
- **Task 34:** `src/skills/MODELS/SKILL.md` — from `topics/models.md`, replace YAML config examples with `wiki config set nim.embed_model`

Each: create file, commit.

---

### Task 35: Create SYNC/SKILL.md and TROUBLESHOOTING/SKILL.md

- **Task 35a:** `src/skills/SYNC/SKILL.md` — from `topics/sync.md`, remove `.wiki/config.yaml`, add `--wiki` flag and `wiki config add`
- **Task 35b:** `src/skills/TROUBLESHOOTING/SKILL.md` — from `topics/troubleshooting.md`, replace all `.wiki/config.yaml` fixes with `wiki config` commands

Each: create file, commit.

---

### Task 36: Convert skill_md.md to WIKI.md hub

**Files:**
- Rename: `src/skills/skill_md.md` → `src/skills/WIKI.md`
- Modify: `src/skills/WIKI.md`

- [ ] **Step 1: Rename file**

```bash
git mv src/skills/skill_md.md src/skills/WIKI.md
```

- [ ] **Step 2: Add hub frontmatter**

At the top of `WIKI.md`, add:
```yaml
---
name: wiki
description: |
  Personal markdown knowledge base (Karpathy-style LLM Wiki). Use when the
  user asks to ingest a source, search the wiki, answer a question against
  prior research, lint or maintain the wiki, set up a new wiki on a new
  device, or pick a different NVIDIA NIM embedding/reranking model. Always
  prefer the wiki's native file tools for browsing; reach for `wiki` CLI
  subcommands only when semantic search or NIM-backed operations are
  explicitly needed.
has-sub-skill: true
allowed-tools: Bash(wiki:*)
---
```

- [ ] **Step 3: Update file layout diagram**

Replace any `.wiki/config.yaml` references with:
```
~/.agents/wiki-root.toml    # wiki registry + config (source of truth)
~/.agents/skills/wiki/      # installed skill bundle
```

- [ ] **Step 4: Trim to routing hub**

Keep the hub to ~100-150 lines. Remove deep-dive content that now lives in sub-skills. Add a routing section listing sub-skills.

- [ ] **Step 5: Commit**

```bash
git add src/skills/WIKI.md
git commit -m "skill: convert skill_md.md to WIKI.md hub with frontmatter"
```

---

### Task 37: Update src/skills/mod.rs

**Files:**
- Modify: `src/skills/mod.rs`

- [ ] **Step 1: Update include_str! paths**

Replace the content of `src/skills/mod.rs`:
```rust
pub const SKILL_MD: &str = include_str!("WIKI.md");
pub const SETUP: &str = include_str!("SETUP/SKILL.md");
pub const INGEST: &str = include_str!("INGEST/SKILL.md");
pub const SEARCH: &str = include_str!("SEARCH/SKILL.md");
pub const QUERY: &str = include_str!("QUERY/SKILL.md");
pub const LINT: &str = include_str!("LINT/SKILL.md");
pub const MODELS: &str = include_str!("MODELS/SKILL.md");
pub const SYNC: &str = include_str!("SYNC/SKILL.md");
pub const TROUBLESHOOTING: &str = include_str!("TROUBLESHOOTING/SKILL.md");

pub const TOPICS: &[(&str, &str)] = &[
    ("setup", SETUP),
    ("ingest", INGEST),
    ("search", SEARCH),
    ("query", QUERY),
    ("lint", LINT),
    ("models", MODELS),
    ("sync", SYNC),
    ("troubleshooting", TROUBLESHOOTING),
];
```

- [ ] **Step 2: Verify build**

Run: `cargo build`
Expected: compiles.

- [ ] **Step 3: Commit**

```bash
git add src/skills/mod.rs
git commit -m "skill: update mod.rs to include all sub-skill SKILL.md files"
```

---

### Task 38: Update src/cli/skill.rs

**Files:**
- Modify: `src/cli/skill.rs`

- [ ] **Step 1: Update skill show to use new structure**

Update `wiki skill show [topic]` to serve from the new `TOPICS` array and `SKILL_MD` constant.

- [ ] **Step 2: Update skill list**

`wiki skill list` should list all 8 sub-skills from `TOPICS`.

- [ ] **Step 3: Verify**

Run: `cargo run -- skill list`
Expected: lists setup, ingest, search, query, lint, models, sync, troubleshooting.

Run: `cargo run -- skill show setup`
Expected: prints SETUP/SKILL.md content.

- [ ] **Step 4: Commit**

```bash
git add src/cli/skill.rs
git commit -m "refactor(skill): serve sub-skills from new folder structure"
```

---

### Task 39: Update src/cli/install_skill.rs

**Files:**
- Modify: `src/cli/install_skill.rs`

- [ ] **Step 1: Install full bundle**

Update `install_skill.rs` to:
1. Create `~/.agents/skills/wiki/` directory
2. Write `SKILL.md` (hub) from `skills::SKILL_MD`
3. For each sub-skill in `skills::TOPICS`, create `~/.agents/skills/wiki/<TOPIC>/SKILL.md`
4. No symlinks — direct file writes from binary-embedded content

```rust
use crate::skills;

pub fn run(args: &InstallSkillArgs) -> Result<(), WikiError> {
    let target = if args.global {
        std::env::var("HOME")
            .map(PathBuf::from)
            .map(|h| h.join(".agents/skills/wiki"))
            .map_err(|_| WikiError::Other(anyhow::anyhow!("HOME not set")))?
    } else {
        std::path::PathBuf::from(".agents/skills/wiki")
    };

    // Create target directory
    std::fs::create_dir_all(&target)?;

    // Write hub skill
    std::fs::write(target.join("SKILL.md"), skills::SKILL_MD)?;

    // Write each sub-skill
    for (name, content) in skills::TOPICS {
        let sub_dir = target.join(name.to_uppercase());
        std::fs::create_dir_all(&sub_dir)?;
        std::fs::write(sub_dir.join("SKILL.md"), content)?;
    }

    println!("Installed wiki skill bundle to {}", target.display());
    Ok(())
}
```

- [ ] **Step 2: Update tests**

Update `tests/install_skill_test.rs` or `tests/skill_smoke.sh` to verify the full bundle installs.

- [ ] **Step 3: Commit**

```bash
git add src/cli/install_skill.rs tests/
git commit -m "refactor(install-skill): install full hub + sub-skill bundle"
```

---

### Task 40: Update build.rs

**Files:**
- Modify: `build.rs`

- [ ] **Step 1: Generate hub stub from WIKI.md**

Update `build.rs` to copy `src/skills/WIKI.md` to `agents/skills/wiki/SKILL.md` at build time:
```rust
fn main() {
    // Only run if skills/WIKI.md exists
    let skill_src = std::path::Path::new("src/skills/WIKI.md");
    if skill_src.exists() {
        let dest = std::path::Path::new("agents/skills/wiki/SKILL.md");
        if let Some(parent) = dest.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(content) = std::fs::read_to_string(skill_src) {
            let _ = std::fs::write(dest, content);
            println!("cargo:rerun-if-changed=src/skills/WIKI.md");
        }
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add build.rs
git commit -m "build: generate hub SKILL.md from src/skills/WIKI.md"
```

---

### Task 41: Delete src/skills/topics/

**Files:**
- Delete: `src/skills/topics/`

- [ ] **Step 1: Remove old topics directory**

```bash
git rm -r src/skills/topics/
```

- [ ] **Step 2: Verify no references remain**

Run: `cargo build && cargo test`
Expected: everything passes.

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "skill: remove old topics/ directory, fully migrated to SKILL.md folders"
```

---

### Task 42: Final verification

- [ ] **Step 1: Format check**

Run: `cargo fmt --check`
Fix any issues with `cargo fmt`.

- [ ] **Step 2: Clippy**

Run: `cargo clippy --all-targets -- -D warnings`
Fix any warnings.

- [ ] **Step 3: Full test suite**

Run: `cargo test`
Expected: all tests pass.

- [ ] **Step 4: Manual smoke test**

```bash
cargo run -- config path
cargo run -- config list
cargo run -- config get nim.embed_model
cargo run -- config list --wiki pharma
cargo run -- skill list
cargo run -- skill show setup
cargo run -- doctor
```

- [ ] **Step 5: Commit any fixes**

```bash
git add -A
git commit -m "chore: final formatting and clippy fixes"
```

---

## Summary

| Phase | Tasks | Outcome |
|-------|-------|---------|
| 1 | Tasks 1-15 | `wiki config` commands work, registry module complete |
| 2 | Tasks 16-28 | All commands use registry, `.wiki/` removed, init auto-registers |
| 3 | Tasks 29-42 | Skills in proper `SKILL.md` folders, install-skill deploys full bundle |

Each phase produces a working, testable binary. Commit after every task.
