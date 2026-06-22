---
title: Wiki Config — wiki-root.toml Centralization
date: 2026-06-22
status: draft
tags: [design, spec, config, wiki-root, registry, multi-wiki, tom]
---

# Wiki Config — wiki-root.toml Centralization

## Context

The current `wiki` CLI has three separate config concerns that are not well unified:

1. **Workspace discovery**: `discover_workspace()` walks up from CWD for `.wiki/`, falls back to `~/wiki`
2. **Per-workspace config**: `.wiki/config.yaml` (YAML, shallow merge, only overrides two fields)
3. **Wiki registry**: A `~/.agents/wiki-root.toml` TOML file already exists and holds wiki aliases (`pharma`, `mevin`, `mywiki`, `minimax`), but the CLI **does not read it at all**

This design replaces `.wiki/config.yaml` discovery logic with `wiki-root.toml` as the single source of truth, adds a `wiki config` command family to manage it, and updates all bundled skill content accordingly.

## 1. Goals

- `wiki-root.toml` is the only config file the CLI writes or reads
- `.wiki/` directory is removed from all discovery paths; it becomes a state-only directory (or is removed entirely)
- `wiki config set/get/list/add/rm` provides ergonomic management
- All existing wikis continue to work without migration
- The bundled skill content is updated to reflect the new commands
- The existing `wiki-root.toml` format (per-alias `path`, `tags`, `description`, `what_to_read`, `qmd_slug`) is preserved and extended

## 2. TOML Structure

### 2.1 File Location (Priority Order)

```
1. $WIKI_ROOT_CONFIG  env var (absolute path)
2. ~/.agents/wiki-root.toml   ← primary (already exists)
3. ~/.claude/wiki-root.toml   ← fallback
4. ~/wiki-root.toml           ← last resort
```

`wiki config path` prints the resolved path.

### 2.2 Full Schema

```toml
# ============================================================
# [defaults] — global defaults applied to all wikis unless
# overridden per-alias. All fields are optional.
# ============================================================
[defaults.nim]
base_url            = "https://integrate.api.nvidia.com"
embed_model         = "nvidia/nv-embed-v1"
rerank_model        = ""                                    # disabled by default
api_key_env         = "NVIDIA_NIM_API_KEY"
batch_size          = 8
request_timeout_secs = 30

[defaults.nim.retry]
max_attempts = 3
backoff_ms   = 500

[defaults.wiki]
default_chunk_tokens    = 512
chunk_overlap_tokens    = 128
min_chunk_tokens        = 32
require_frontmatter     = true
require_wikilinks_min   = 2

# ============================================================
# Registered wikis — one [alias] table per wiki.
# Existing fields are preserved; new fields are optional.
# ============================================================
[pharma]
path        = "/Users/felix/Documents/PharmaWiki"
tags        = ["pharmacologie", "français", "dfasp1", "médecine", "révision"]
what_to_read = ["SCHEMA.md", "index.md"]
qmd_slug    = "pharma"
description = """Pharmacologie et sciences pharmaceutiques…"""

# Optional per-alias overrides (deep-merge with [defaults])
[pharma.nim]
embed_model = "nvidia/nv-embedqa-e5-v5"

[pharma.wiki]
default_chunk_tokens = 1024

[mevin]
path        = "/Users/felix/Documents/Tauri2/mevin-tauri2/wiki"
tags        = ["rust", "tauri", "svelte", "opencode", "desktop-app", "mavis", "sidecar"]
what_to_read = ["SCHEMA.md", "index.md"]
qmd_slug    = "mevin"
description = """Mevin project wiki — Tauri 2 desktop app…"""

[mywiki]
path        = "/Users/felix/Documents/MyWiki"
tags        = ["personal", "knowledge-base", "ai", "engineering", "reference"]
what_to_read = ["SCHEMA.md", "index.md"]
qmd_slug    = "mywiki"
description = """Personal knowledge base…"""

[minimax]
path        = "/Users/felix/Documents/MinimaxCode/minimax-code-wiki"
tags        = ["minimax", "mavis", "agent-runtime", "electron", "skills", "mcp", "decompilation"]
what_to_read = ["SCHEMA.md", "index.md"]
qmd_slug    = "minimax"
description = """MiniMax Code wiki…"""
```

### 2.3 Merge Semantics

- **Tables (`[section]`)** — recursively deep-merged
- **Scalars (strings, ints, bools)** — right wins (override)
- **Arrays (e.g. `tags`)** — concatenated, then deduplicated
- **TOML merge implementation** — use `serde_toml_merge` crate (MIT, 166 lines, does exactly this)

Final resolved config for `pharma`:
```toml
nim:
  base_url            = "https://integrate.api.nvidia.com"   # from [defaults]
  embed_model         = "nvidia/nv-embedqa-e5-v5"             # from [pharma.nim]
  rerank_model        = ""                                    # from [defaults]
  api_key_env         = "NVIDIA_NIM_API_KEY"                  # from [defaults]
  batch_size          = 8                                     # from [defaults]
  request_timeout_secs = 30                                   # from [defaults]
  retry:
    max_attempts = 3
    backoff_ms   = 500                                        # from [defaults]
wiki:
  default_chunk_tokens    = 1024                              # from [pharma.wiki]
  chunk_overlap_tokens    = 128                               # from [defaults]
  min_chunk_tokens        = 32                               # from [defaults]
  require_frontmatter     = true                             # from [defaults]
  require_wikilinks_min   = 2                                # from [defaults]
```

## 3. Config Module Changes (`src/core/config.rs` → `src/core/registry.rs`)

### 3.1 New Module: `src/core/registry.rs`

```rust
// Replaces discover_workspace() + resolve_config() logic
// Thin wrapper around wiki-root.toml loading + merging

pub struct WikiEntry {
    pub alias: String,
    pub path: PathBuf,
    pub tags: Vec<String>,
    pub description: String,
    pub what_to_read: Vec<String>,
    pub qmd_slug: Option<String>,
    pub nim_override: Option<NimConfig>,
    pub wiki_override: Option<WikiConfig>,
}

pub struct WikiDefaults {
    pub nim: NimConfig,
    pub wiki: WikiConfig,
}

pub struct Registry {
    pub root_path: PathBuf,
    pub defaults: Option<WikiDefaults>,
    pub entries: Vec<WikiEntry>,
}

impl Registry {
    /// Load wiki-root.toml with deep-merge from [defaults]
    pub fn load() -> Result<Self, WikiError>;

    /// Resolve which wiki is active given CLI flag, env, and CWD
    pub fn resolve_active(
        &self,
        flag_alias: Option<&str>,
        flag_path: Option<&Path>,
        env_alias: Option<&str>,
        env_path: Option<&Path>,
        cwd: &Path,
    ) -> Result<(String, PathBuf, Config), WikiError>;

    /// Get the merged Config for a given alias
    pub fn resolve_config(&self, alias: &str) -> Result<Config, WikiError>;

    /// Persist a change to wiki-root.toml (atomic write)
    pub fn save(&self) -> Result<(), WikiError>;
}
```

### 3.2 Workspace Discovery Priority

```
1. --workspace <path>   CLI flag (absolute path, overrides alias)
2. --wiki <alias>       CLI flag (selects by alias)
3. WIKI_WORKSPACE        env var (absolute path)
4. WIKI_ACTIVE           env var (alias)
5. CWD prefix match      longest prefix of any [alias].path == CWD or ancestor
6. Single-wiki shortcut   if only 1 entry in registry and no CWD match, use it
7. Heuristic probe       if CWD or parent has wiki/ + raw/ + index.md (no .wiki/ needed)
8. → Error: WorkspaceNotFound
```

`.wiki/` directory is **removed from discovery**. The walk-up function in `workspace.rs` is replaced with CWD prefix matching against registry paths.

### 3.3 Config Resolution

Each command calls:
```rust
let (alias, ws_path, cfg) = registry.resolve_active(
    args.wiki.as_deref(),
    args.workspace.as_deref(),
    env_active.as_deref(),
    env_workspace.as_deref(),
    &cwd,
)?;
// alias for error messages; ws_path for FS ops; cfg for NIM/wiki settings
```

The old `resolve_config(workspace: &Path)` → `load_config(paths: &[PathBuf])` chain is removed. Config resolution is fully in `registry.rs`.

### 3.4 CLI Flag Additions

In `src/cli/mod.rs` `Cli` struct, add:
```rust
#[arg(long, global = true)]
pub wiki: Option<String>,
```
This is in addition to the existing `--workspace` flag. Both are global.

### 3.5 Error Handling

- File not found → create empty template, hint user to `wiki init` or `wiki config add`
- TOML parse error → `WikiError::ConfigInvalid { path, line, message }`, exit 4
- Alias not found → list all available aliases, exit 1
- Atomic write on save → `*.tmp` + rename

### 3.6 Deprecations

- `src/core/workspace.rs` → becomes a stub or is removed; its functions move into `registry.rs`
- `src/core/config.rs` → `load_config(paths)` and `resolve_config(workspace)` are removed
- All `discover_workspace(...)` boilerplate in each CLI command file is replaced with one `registry.resolve_active()` call
- YAML config loading (serde_yaml) is removed from the config path entirely (TOML only for wiki-root.toml)

## 4. CLI Commands

### 4.1 New Command: `Config` (Subcommand Family)

Added to `Command` enum in `src/cli/mod.rs`:

```rust
Config {
    #[command(subcommand)]
    pub cmd: ConfigCmd,
}

#[derive(Subcommand)]
pub enum ConfigCmd {
    /// Print the resolved wiki-root.toml path
    Path,
    /// List all wikis and their config (merged, dotted keys)
    List {
        #[arg(long)]
        wiki: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Get a specific config key
    Get {
        key: String,
        #[arg(long)]
        wiki: Option<String>,
    },
    /// Set a config key to a value
    Set {
        key: String,
        value: String,
        #[arg(long)]
        wiki: Option<String>,
    },
    /// Unset a config key (revert to default)
    Unset {
        key: String,
        #[arg(long)]
        wiki: Option<String>,
    },
    /// Add a new wiki to the registry
    Add {
        alias: String,
        path: std::path::PathBuf,
        #[arg(long, multiple = true)]
        tag: Vec<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        what_to_read: Option<String>,  # comma-separated
    },
    /// Remove a wiki from the registry
    Rm {
        alias: String,
    },
    /// Open wiki-root.toml in $EDITOR
    Edit,
}
```

### 4.2 `wiki config set` Key Syntax

Dotted keys map to TOML paths:

| Key | TOML path |
|-----|-----------|
| `nim.embed_model` | `[alias].nim.embed_model` (or `[defaults].nim.embed_model` if no `--wiki`) |
| `nim.base_url` | `[alias].nim.base_url` |
| `wiki.default_chunk_tokens` | `[alias].wiki.default_chunk_tokens` |
| `tags` | `[alias].tags` (appends to array) |

If `--wiki <alias>` is given: writes to `[alias].nim.embed_model` etc.
If `--wiki` is omitted: writes to `[defaults].nim.embed_model`.

### 4.3 `wiki init` Updates

```bash
wiki init <path> [--alias <name>] [--tag <tag> [--tag ...]] [--name <display-name>]
```

Behavior:
1. Create `wiki/`, `raw/articles/`, `index.md`, `log.md`, `.gitignore` (no `.wiki/`)
2. `git init` if no `.git`
3. Auto-register in `wiki-root.toml`:
   - `[<alias>]` with `path`, `tags` (from `--tag`), `description` (from `--name` or auto-generated)
   - If no `--alias`: prompt or use directory basename
4. Print the resolved alias so the agent knows which one to use next

`.wiki/config.yaml` is **never created**. The `DEFAULT_CONFIG` const in `src/cli/init.rs` is removed.

### 4.4 `wiki ls --config` Updates

`wiki ls --config` prints all resolved config keys (merged from defaults + alias) as dotted keys, same as `wiki config list --wiki <alias>`.

### 4.5 `wiki doctor` Updates

Extends to report:
- `wiki_root_path`: resolved `wiki-root.toml` path
- `active_alias`: which alias is active
- `registry_entries`: count of registered wikis

## 5. File Layout Changes

### 5.1 User's Wiki (Simplified)

```
~/my-wiki/
├── wiki/                        # compiled markdown (committed)
├── raw/                         # source materials (committed)
├── index.md                     # catalog (committed)
├── log.md                       # operational log (committed)
└── embeddings.jsonl             # NIM vectors (GITIGNORED)
```

No `.wiki/` directory created by `wiki init`. The `.wiki/` folder (if it exists on old wikis) is ignored by discovery — it contains only state files which are gitignored anyway.

### 5.2 Skill Bundle Structure (Proper Agent Skill Layout)

Following the agentskills.io / Kimi Code / Claude Code convention:
- **One directory = one skill**
- **Every skill directory has a `SKILL.md`**
- **Sub-skills are directories with their own `SKILL.md`**
- **References are optional docs loaded on demand**

Current repo has:
```
src/skills/
├── skill_md.md                    # monolithic main skill
└── topics/                        # loose topic markdown files
```

Target structure:
```
src/skills/
├── mod.rs                         # Rust module that bundles all SKILL.md files
├── SETUP/
│   ├── SKILL.md                   # setup workflow (was topics/setup.md)
│   └── README.md                  # optional: topic overview
├── INGEST/
│   ├── SKILL.md                   # ingest workflow (was topics/ingest.md)
│   └── README.md
├── SEARCH/
│   ├── SKILL.md                   # search workflow (was topics/search.md)
│   └── README.md
├── QUERY/
│   ├── SKILL.md                   # query workflow (was topics/query.md)
│   └── README.md
├── LINT/
│   ├── SKILL.md                   # lint workflow (was topics/lint.md)
│   └── README.md
├── MODELS/
│   ├── SKILL.md                   # models workflow (was topics/models.md)
│   └── README.md
├── SYNC/
│   ├── SKILL.md                   # sync workflow (was topics/sync.md)
│   └── README.md
├── TROUBLESHOOTING/
│   ├── SKILL.md                   # troubleshooting workflow (was topics/troubleshooting.md)
│   └── README.md
└── WIKI.md                        # hub skill (was skill_md.md)
```

This matches the pattern in `~/.agents/skills/AGENTS.md`:
```
~/.agents/skills/
├── <skill-name>/SKILL.md          # hub skill
│   ├── <sub-skill>/SKILL.md       # sub-skill
│   └── references/                # optional reference docs
└── AGENTS.md                      # skill authoring guidelines
```

**Important:** The actual skill content is **not** the loose `topics/*.md` files. Each subdirectory gets its own `SKILL.md` file. The old `topics/*.md` files are **not** bundled — they are migrated into proper sub-skill folders.

### 5.3 Skill Frontmatter Convention

Each `SKILL.md` file should have frontmatter:

```yaml
---
name: setup
description: |
  Install the wiki CLI, create a wiki, register it in wiki-root.toml,
  set up the bundled skill, and verify the first-run setup. Use when
  the user asks about setup, first-run, wiki init, wiki config add,
  or installing the wiki skill.
whenToUse: |
  Do NOT use for searching or querying an already-working wiki.
allowed-tools: Bash(wiki:*)
---
```

For hub skill `WIKI.md`:
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

### 5.4 How the CLI Bundles Skills

The Rust binary embeds **all `SKILL.md` files** via `include_str!` in `src/skills/mod.rs`:

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

`wiki skill show setup` prints the full `SETUP/SKILL.md` content.
`wiki skill list` lists all sub-skill names with line counts.

### 5.5 How `wiki install-skill` Installs the Bundle

`wiki install-skill --global` creates a **proper skill directory** at `~/.agents/skills/wiki/`:

```
~/.agents/skills/wiki/
├── SKILL.md                       # hub skill (small, routes to sub-skills)
├── SETUP/
│   └── SKILL.md                   # sub-skill
├── INGEST/
│   └── SKILL.md                   # sub-skill
├── SEARCH/
│   └── SKILL.md                   # sub-skill
├── QUERY/
│   └── SKILL.md                   # sub-skill
├── LINT/
│   └── SKILL.md                   # sub-skill
├── MODELS/
│   └── SKILL.md                   # sub-skill
├── SYNC/
│   └── SKILL.md                   # sub-skill
└── TROUBLESHOOTING/
    └── SKILL.md                   # sub-skill
```

The **hub skill** (`~/.agents/skills/wiki/SKILL.md`) is small and routes to sub-skills:

```markdown
# wiki

Run `wiki skill show <topic>` for the full workflow content.
The CLI serves the exact version bundled in the binary.

Available sub-skills:
- `setup` — install, init, first-run
- `ingest` — adding sources and compiling
- `search` — semantic search usage
- `query` — RAG query usage
- `lint` — hygiene checks
- `models` — NIM model selection
- `sync` — git sync across tailnet devices
- `troubleshooting` — common errors

Use `wiki skill show <topic>` to load the full content.
```

The **sub-skills** are copied into the installed skill directory. They are not symlinks to the source tree — they are **bundled in the binary** and installed by `wiki install-skill`.

### 5.6 `build.rs` Updates

`build.rs` should generate the hub skill stub from a template, not hardcode the entire skill content:

```rust
fn main() {
    let stub = generate_hub_skill_stub();
    let out_path = manifest_path.join("agents/skills/wiki/SKILL.md");
    fs::write(&out_path, stub).ok();
}
```

The hub stub is generated from `src/skills/WIKI.md` or a small template.

### 5.7 Module Structure

```
src/
├── core/
│   ├── registry.rs              # NEW: wiki-root.toml loading, resolve, merge
│   ├── config.rs                 # MODIFIED: only keep NimConfig, WikiConfig, RetryConfig structs + resolve_api_key()
│   ├── workspace.rs              # TO-BE-DEPRECATED: stub that calls registry.rs
│   ├── markdown.rs
│   ├── embeddings.rs
│   ├── chunker.rs
│   ├── nim.rs
│   ├── models_registry.rs
│   └── mod.rs
├── cli/
│   ├── mod.rs                    # add Config cmd, --wiki flag, wiki config dispatch
│   ├── config.rs                 # NEW: wiki config set/get/list/add/rm/edit
│   ├── init.rs                   # MODIFIED: remove .wiki/ creation, add auto-register
│   ├── ls.rs                     # MODIFIED: call registry.resolve_active()
│   ├── search.rs                 # MODIFIED: call registry.resolve_active()
│   ├── query.rs                  # MODIFIED: call registry.resolve_active()
│   ├── embed.rs                  # MODIFIED: call registry.resolve_active()
│   ├── lint.rs                   # MODIFIED: call registry.resolve_active()
│   ├── doctor.rs                 # MODIFIED: call registry.resolve_active(), extend output
│   ├── ingest.rs                 # MODIFIED: call registry.resolve_active()
│   ├── status.rs                 # MODIFIED: call registry.resolve_active()
│   ├── tree.rs                   # MODIFIED: call registry.resolve_active()
│   ├── build.rs
│   ├── models.rs
│   ├── skill.rs                  # MODIFIED: show sub-skill by name
│   └── install_skill.rs          # MODIFIED: install full skill bundle
├── skills/
│   ├── mod.rs                    # MODIFIED: include all SKILL.md files
│   ├── WIKI.md                   # hub skill (was skill_md.md)
│   ├── SETUP/
│   │   └── SKILL.md              # setup workflow (was topics/setup.md)
│   ├── INGEST/
│   │   └── SKILL.md              # ingest workflow (was topics/ingest.md)
│   ├── SEARCH/
│   │   └── SKILL.md              # search workflow (was topics/search.md)
│   ├── QUERY/
│   │   └── SKILL.md              # query workflow (was topics/query.md)
│   ├── LINT/
│   │   └── SKILL.md              # lint workflow (was topics/lint.md)
│   ├── MODELS/
│   │   └── SKILL.md              # models workflow (was topics/models.md)
│   ├── SYNC/
│   │   └── SKILL.md              # sync workflow (was topics/sync.md)
│   └── TROUBLESHOOTING/
│       └── SKILL.md              # troubleshooting workflow (was topics/troubleshooting.md)
└── error.rs                      # add WikiRootNotFound variant
```

### 5.8 Skill Install Command (`src/cli/install_skill.rs`)

New behavior:
1. Resolve source from `src/skills/` (bundled in binary, not from `agents/skills/wiki`)
2. Create target directory (`~/.agents/skills/wiki` or workspace-local `.agents/skills/wiki`)
3. Copy all `SKILL.md` files into target
4. Write a small `README.md` in target (optional)
5. No symlinks to source tree — the skill is **bundled in the binary**

This means the installed skill always matches the installed CLI version.

### 5.9 Skill Show Command (`src/cli/skill.rs`)

New behavior:
```
wiki skill show [topic]
```
- `topic=None` → print hub skill `WIKI.md`
- `topic=Some("setup")` → print `SETUP/SKILL.md`
- `topic=Some("ingest")` → print `INGEST/SKILL.md`
- `topic=Some("models")` → print `MODELS/SKILL.md`
- etc.

`wiki skill list` lists all sub-skill names with line counts.

### 5.10 Skill Content Migration Map

| Old File | New Location | Notes |
|----------|--------------|-------|
| `skill_md.md` | `WIKI.md` | Hub skill, small, routes to sub-skills |
| `topics/setup.md` | `SETUP/SKILL.md` | Full frontmatter + setup workflow |
| `topics/ingest.md` | `INGEST/SKILL.md` | Full frontmatter + ingest workflow |
| `topics/search.md` | `SEARCH/SKILL.md` | Full frontmatter + search workflow |
| `topics/query.md` | `QUERY/SKILL.md` | Full frontmatter + query workflow |
| `topics/lint.md` | `LINT/SKILL.md` | Full frontmatter + lint workflow |
| `topics/models.md` | `MODELS/SKILL.md` | Full frontmatter + models workflow |
| `topics/sync.md` | `SYNC/SKILL.md` | Full frontmatter + sync workflow |
| `topics/troubleshooting.md` | `TROUBLESHOOTING/SKILL.md` | Full frontmatter + troubleshooting workflow |

The old `topics/` directory is deleted. All content lives in proper `SKILL.md` files.

### 5.11 User's Wiki (Simplified)

```
~/my-wiki/
├── wiki/                        # compiled markdown (committed)
├── raw/                         # source materials (committed)
├── index.md                     # catalog (committed)
├── log.md                       # operational log (committed)
└── embeddings.jsonl             # NIM vectors (GITIGNORED)
```

No `.wiki/` directory created by `wiki init`. The `.wiki/` folder (if it exists on old wikis) is ignored by discovery — it contains only state files which are gitignored anyway.

## 6. Skill Content Updates

All skill content lives as proper `SKILL.md` files in `src/skills/<TOPIC>/SKILL.md` and is embedded via `include_str!` in `src/skills/mod.rs`.

### 6.1 `WIKI.md` — Hub Skill

**Location:** `src/skills/WIKI.md`

**Changes:**
- Update file layout diagram: remove `.wiki/config.yaml`, note that config lives in `wiki-root.toml`
- Add `wiki config` command reference
- Update "When to Use" trigger to include "asks about wiki config"
- Add section on multi-wiki: `--wiki <alias>` and `WIKI_ACTIVE`
- Keep it short (~100-150 lines max) — it routes to sub-skills

**Updated layout block:**
```
~/.agents/wiki-root.toml    # wiki registry + config (source of truth)
~/.agents/skills/wiki/      # installed skill bundle (hub + sub-skills)
~/my-wiki/
├── wiki/                    # compiled markdown (committed)
├── raw/                     # source materials (committed)
├── index.md                 # catalog (committed)
├── log.md                   # operational log (committed)
└── embeddings.jsonl         # NIM vectors (GITIGNORED)
```

### 6.2 `SETUP/SKILL.md`

**Location:** `src/skills/SETUP/SKILL.md`

**Changes:**
- `wiki init` no longer creates `.wiki/`
- New: `wiki init /path/to/wiki --alias pharma --tag medicine`
- New: auto-registers to `wiki-root.toml`
- New: `wiki config path` — find the config file
- New: `wiki config add <alias> <path>` for manual registration
- New: `wiki config list` — show all registered wikis
- New: `wiki install-skill --global` installs the full skill bundle
- Remove: all references to `.wiki/config.yaml`

### 6.3 `SEARCH/SKILL.md`

**Location:** `src/skills/SEARCH/SKILL.md`

**Changes:**
- Minor: add note that `--wiki <alias>` can switch wiki without `cd`
- No structural changes

### 6.4 `QUERY/SKILL.md`

**Location:** `src/skills/QUERY/SKILL.md`

**Changes:**
- Minor: add note about `--wiki <alias>`

### 6.5 `INGEST/SKILL.md`

**Location:** `src/skills/INGEST/SKILL.md`

**Changes:**
- Add: `wiki config add <alias> <path>` for registering a new wiki before ingesting
- No structural changes

### 6.6 `LINT/SKILL.md`

**Location:** `src/skills/LINT/SKILL.md`

**Changes:**
- Update config example from YAML to TOML/dotted key:
  ```bash
  wiki config set wiki.require_wikilinks_min 3  # in wiki-root.toml [defaults] or [alias]
  ```
- Remove: all `.wiki/config.yaml` references

### 6.7 `MODELS/SKILL.md`

**Location:** `src/skills/MODELS/SKILL.md`

**Changes:**
- Replace YAML config example with TOML/dotted key:
  ```bash
  wiki config set nim.embed_model nvidia/llama-nemotron-embed-1b-v2
  ```
- Note: `--wiki <alias>` can override per wiki
- Remove: `.wiki/config.yaml` references

### 6.8 `SYNC/SKILL.md`

**Location:** `src/skills/SYNC/SKILL.md`

**Changes:**
- Update new-device setup: `wiki init` now auto-registers, no `.wiki/` directory
- Add: `wiki config add` for manual registration
- Add: `--wiki <alias>` flag to switch between wikis without changing directories
- Remove: `.wiki/config.yaml` references

### 6.9 `TROUBLESHOOTING/SKILL.md`

**Location:** `src/skills/TROUBLESHOOTING/SKILL.md`

**Changes:**
- Replace `workspace not found` fix: `wiki config list` to see registered wikis, `wiki config add <alias> <path>` or `wiki --wiki pharma <cmd>`
- Replace `wrong model` fix: `wiki config set nim.embed_model <model>`, no `.wiki/config.yaml`
- Add: `wiki config path` if the agent can't find the config file
- Add: `WIKI_ACTIVE=<alias>` env var as alternative to `--wiki`
- Remove: all `.wiki/config.yaml` references

## 7. Backward Compatibility

Existing wikis that have `.wiki/config.yaml` but are NOT in `wiki-root.toml`:
1. `wiki config add <auto> <path>` to register them
2. Their `.wiki/config.yaml` is ignored; the registry has final say
3. `wiki init` never creates `.wiki/` again

This means existing `.wiki/config.yaml` files on disk are harmless but ignored. They can be safely gitignored/deleted after migration.

## 8. Testing Plan

| Test File | Coverage |
|-----------|----------|
| `tests/config_test.rs` → `tests/registry_test.rs` | Load, merge, set/get/list/unset, atomic write, parse errors, defaults |
| `tests/discovery_test.rs` | 8-step discovery priority |
| `tests/init_test.rs` | `wiki init` → auto-adds to wiki-root.toml |
| `tests/cli_test.rs` | `wiki config *` commands |
| `tests/doctor_test.rs` | Extended output includes wiki-root path, active alias |
| `tests/e2e_test.rs` | Full pipeline with wiki-root.toml |
| Unit tests `core/registry.rs` | CWD prefix matching, TOML merge, alias resolution |

## 9. Migration Checklist

After implementation, the following must be verified:

- [ ] `wiki config path` prints the resolved path
- [ ] `wiki config list` shows all 4 existing wikis (pharma, mevin, mywiki, minimax)
- [ ] `wiki config get nim.embed_model --wiki pharma` returns merged value from `[defaults]`
- [ ] `wiki config set nim.embed_model nvidia/llama-nemotron-embed-1b-v2 --wiki pharma` creates `[pharma.nim]` section
- [ ] `wiki config unset nim.embed_model --wiki pharma` removes override, reverts to defaults
- [ ] `wiki config add newwiki /path/to/newwiki` registers new entry
- [ ] `wiki config rm newwiki` removes entry
- [ ] `wiki config edit` opens editor
- [ ] `wiki --wiki pharma search "..."` works without CWD match
- [ ] `wiki ls --config` shows merged config
- [ ] `wiki doctor` reports `wiki_root_path` and `active_alias`
- [ ] `wiki init /tmp/test-wiki --alias test` creates dir and auto-registers
- [ ] Existing wikis without wiki-root.toml entry: `wiki ls` still works (heuristic probe)
- [ ] All skill content updated: no `.wiki/config.yaml` references remain
- [ ] `cargo clippy --all-targets -- -D warnings` passes
- [ ] `cargo test` passes

## 10. Dependencies

Add to `Cargo.toml`:
```toml
serde_toml_merge = "0.2"   # deep TOML merge (MIT, no_std compatible)
toml = "0.8"               # already present
```

No new runtime deps beyond what already exists.

## 11. Scope Boundaries

**In scope:**
- `wiki-root.toml` as single config source
- `wiki config` command family
- `--wiki <alias>` global flag
- Auto-discovery of active wiki from CWD
- `wiki init` auto-registration
- Skill content updates (hub + sub-skills, proper folder layout)
- Backward compat with existing wikis (heuristic fallback)

**Out of scope:**
- Automatic migration of `.wiki/config.yaml` to `wiki-root.toml`
- Multi-device sync beyond git
- Web viewer (already removed)
- Config schema versioning/migration (future work)
- Skill signing / trust levels (agentskills.io security model is still evolving)

## Appendix A: Skill Bundle Diff Summary

| Old File | New Location | Change Type | Summary |
|----------|--------------|-------------|---------|
| `skill_md.md` | `WIKI.md` | Modify | Hub skill, routes to sub-skills, remove `.wiki/config.yaml` |
| `topics/setup.md` | `SETUP/SKILL.md` | Rewrite | `wiki init --alias`, `wiki config add/list/path`, skill install |
| `topics/search.md` | `SEARCH/SKILL.md` | Minor | `--wiki <alias>` note |
| `topics/query.md` | `QUERY/SKILL.md` | Minor | `--wiki <alias>` note |
| `topics/ingest.md` | `INGEST/SKILL.md` | Minor | `wiki config add` reference |
| `topics/lint.md` | `LINT/SKILL.md` | Modify | TOML/dotted-key config examples |
| `topics/models.md` | `MODELS/SKILL.md` | Modify | TOML/dotted-key config examples, `--wiki` per-wiki override |
| `topics/sync.md` | `SYNC/SKILL.md` | Modify | `--wiki` flag, `wiki config add`, no `.wiki/` |
| `topics/troubleshooting.md` | `TROUBLESHOOTING/SKILL.md` | Modify | Replace all `.wiki/config.yaml` with `wiki config` commands |

**All new `SKILL.md` files get frontmatter** (`name`, `description`, `whenToUse`, `allowed-tools`) following the agentskills.io / Kimi Code convention.

## Appendix B: Skill Authoring Research

### Sources Reviewed

1. **Kimi Help Center — Using Skills in Kimi Code**
   - Skill = subdirectory in Skills directory + `SKILL.md`
   - Built-in → User → Project priority
   - `/skill:<name>` invocation
   - Supports Flow Skills with `type: flow` frontmatter

2. **Deep Dive SKILL.md (Part 1/2)**
   - Skill anatomy: `SKILL.md` + optional scripts/references/assets
   - 3-level loading: identity → full instructions → deep references
   - Avoid Claude-specific undocumented features
   - Prefer portable agentskills.io standard

3. **Local skill monorepo (`~/.agents/skills/AGENTS.md`)**
   - Hub skill (`SKILL.md`) routes to sub-skills
   - Sub-skill directories each have their own `SKILL.md`
   - Frontmatter convention: `name`, `description`, `whenToUse`, `has-sub-skill`, `disableModelInvocation`
   - Token budget for pure routing hubs: ≤500 tokens
   - Cross-reference pattern: `/skill:<sub-skill-name>`

4. **Existing `agent-browser` skill**
   - Hub skill is small, routes to `skills get core`, `skills get electron`, etc.
   - Full content served by CLI to keep instructions versioned
   - Pre-flight and cleanup sections in hub skill

### Design Implications

- `wiki` should follow the **hub + sub-skill** pattern, not monolithic `skill_md.md`
- `wiki skill show <topic>` should print the exact `SKILL.md` content from the binary
- `wiki install-skill --global` should install the **full skill bundle** (hub + all sub-skills) to `~/.agents/skills/wiki/`
- The installed skill should be a **copy**, not a symlink, so it always matches the CLI version
- Each sub-skill should have full frontmatter and be self-contained

## Appendix C: Decision Log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-06-22 | Centralize in `wiki-root.toml` | User has existing TOML with 4 wikis; keeping `.wiki/config.yaml` is dual-source confusion |
| 2026-06-22 | Per-alias `[section].nim` overrides | Enables per-wiki model selection without duplication |
| 2026-06-22 | CWD prefix match over `.wiki/` walk-up | Works for any directory structure, no `.wiki/` required |
| 2026-06-22 | Heuristic fallback for un-registered wikis | Existing wikis without wiki-root.toml entry still work |
| 2026-06-22 | Dotted-key CLI syntax | Familiar (kubectl, git config), works for any TOML depth |
| 2026-06-22 | Hub + sub-skill skill bundle | Matches agentskills.io / Kimi Code / Claude Code convention; cleaner than monolithic skill |
| 2026-06-22 | Installed skill is a copy, not symlink | Ensures installed skill always matches CLI version, no broken source-tree paths |