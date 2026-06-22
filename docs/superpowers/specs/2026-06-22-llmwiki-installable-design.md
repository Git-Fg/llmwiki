# llmwiki Installable, Agent-Agnostic, LSP-Enabled — Design

**Status:** Draft, awaiting user review.
**Date:** 2026-06-22
**Author:** Brainstorm session + research notes.
**Target llmwiki version:** v0.3.0 (a clean break from v0.1.0; see §1.1).

> **Placeholder note**: `<owner>` throughout this spec refers to the GitHub username/org that will host `llmwiki` once the repo is published. Replace with the actual owner at implementation time. The user (Felix) is expected to choose between `Git-Fg/llmwiki` and a different owner; until that decision is made, every concrete URL keeps the `<owner>` form.

---

## 1. Goals, non-goals, scope

### 1.1 Goals

1. **One binary, three wire transports**: `llmwiki-cli` (CLI) + `llmwiki-cli lsp` (LSP over stdio) + `llmwiki-cli mcp` (MCP over stdio). The CLI surface includes `llmwiki-cli doctor` as a first-class diagnostic subcommand. Same domain code (`src/core/`) drives all three transports.
2. **Installable from a curl command**: `curl -LsSf https://github.com/<owner>/llmwiki/raw/main/install.sh | bash`. Pre-built binaries for `darwin-aarch64`, `darwin-x86_64`, `linux-aarch64`, `linux-x86_64`, `windows-x86_64` (msvc).
3. **Crates.io publishable**: `cargo install llmwiki-cli` works from a public crates.io release.
4. **Marketplace plugin for all 4 major AI runtimes**: Claude Code, Kimi Code, Cursor, Codex. Auto-installs on `/plugin install`, auto-updates per runtime policy.
5. **Skill bootstraps the CLI install**: First action of the skill is `curl install.sh | bash` then `llmwiki-cli doctor` to verify. User never manually reads install instructions.
6. **LSP server for `wiki-root.toml`**: diagnostics + hover + completion + documentSymbol. Stateless per request. Editors wire up via `command: "llmwiki-cli" args: ["lsp"]`.
7. **MCP server mirrors the same domain logic**: tools `validate`, `hover`, `completion`, `schema`, `doctor`. Cursor's AI consumes it directly without `mcp-language-server`.
8. **Cleanup follow-ups folded in**: `validate_or_error(&cfg)` in `embed.rs`, subprocess test for `llmwiki-cli search` fast-fail, `show-schema` output embedded in `SETUP/SKILL.md`.

### 1.2 Non-goals

- No GitHub Releases binary for every commit — only tagged releases.
- No Homebrew tap (curl installer covers macOS; if requested later, a separate spec).
- No GUI viewer (the removed `web/` Svelte viewer stays removed).
- No Mavis / MiniMax-specific runtime integration — `llmwiki-cli` stays generic.
- No `textDocument/formatting` LSP method — `taplo fmt` already exists and is generic.
- No VS Code / Cursor `.vsix` extension — Phase 6 deferred.
- No Windows `install.ps1` — Phase 6 deferred; Git Bash works.

### 1.3 Scope decision

**Single umbrella spec, 6 phases (Phase 0–5), shipped as a single coordinated release (v0.3.0).**

Decomposition rationale: every phase depends on the previous one (Phase 1 installability requires Phase 0 rename; Phase 2 plugin requires Phase 1 installability for the skill's bootstrap to work; Phase 4 LSP requires Phase 2 plugin to declare it; Phase 5 MCP requires Phase 4's shared domain layer). One spec keeps the cross-phase invariants in one place.

If at execution time a phase turns out to need its own focused spec (e.g. LSP evolves into a major research project), split it then. For now: one spec, one plan.

---

## 2. Architecture overview

```
┌─────────────────── llmwiki-cli (single binary) ────────────────────┐
│                                                                     │
│  ┌── clap entry: src/cli/* ──┐  ┌── tower-lsp-server: lsp ───────┐ │
│  │  init / ingest / search   │  │  publishDiagnostics            │ │
│  │  query / embed / lint /   │  │  hover                         │ │
│  │  ls / tree / doctor /     │  │  completion                    │ │
│  │  config { path,list,get,  │  │  documentSymbol                │ │
│  │  set,unset,add,rm,edit,   │  │                               │ │
│  │  validate,show-schema }   │  └───────────────────────────────┘ │
│  │  install-skill / status   │  ┌── rmcp: mcp ──────────────────┐ │
│  │  build / skill / models   │  │  validate / hover / completion │ │
│  │  lsp / mcp                │  │  schema / doctor               │ │
│  └───────────────────────────┘  └───────────────────────────────┘ │
│                                                                     │
│  ┌────────── src/core/ (shared domain) ───────────────────────────┐  │
│  │  registry.rs    workspace.rs    config.rs + validate()         │  │
│  │  markdown.rs    chunker.rs      embeddings.rs                  │  │
│  │  nim.rs         models_registry.rs                             │  │
│  │  lsp_domain.rs  (NEW — shared by lsp and mcp)                 │  │
│  └────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
                                   │
                                   ▼
       ┌────────── marketplace/ (co-located in main repo) ───────────┐
       │   .claude-plugin/plugin.json + .lsp.json                    │
       │   .kimi-plugin/plugin.json + .lsp.json                      │
       │   .cursor-plugin/plugin.json + .lsp.json                   │
       │   .codex-plugin/plugin.json + .lsp.json                    │
       │   skills/wiki/SKILL.md (hub) + SETUP/, INGEST/, SEARCH/,   │
       │     QUERY/, LINT/, MODELS/, SYNC/, TROUBLESHOOTING/,       │
       │     LSP/, MCP/                                              │
       │   install.sh (mirror of repo-root install.sh)               │
       │   marketplace.json (catalog entry)                          │
       │   scripts/validate.py (skill format linter)                │
       └─────────────────────────────────────────────────────────────┘
```

**Key invariants:**

- **Single binary, single source of truth.** LSP and MCP are clap subcommands, not separate binaries.
- **Stateless per request.** Both LSP and MCP re-parse TOML on every request. No in-memory document state. This dodges Claude Code bug #32067 (missing `didOpen`) and #32265 (empty results on repeated queries).
- **Domain logic shared.** A new `src/core/lsp_domain.rs` module is imported by both `src/cli/lsp.rs` and `src/cli/mcp.rs`. Zero duplication of parse/validate/schema-walking logic.

---

## 3. Phase 0 — Rename and cleanup follow-ups

Do these first. They unblock every later phase.

### 3.1 Rename `wiki` → `llmwiki-cli` (binary) and `llmwiki-cli` (crate)

**Scope of rename:**

| Layer | Old | New |
|---|---|---|
| `Cargo.toml` `[package] name` | `wiki` | `llmwiki-cli` |
| `Cargo.toml` `[[bin]] name` (default) | `wiki` | `llmwiki-cli` |
| clap `#[command(name = "wiki", ...)]` | `wiki` | `llmwiki-cli` |
| `tests/*_test.rs` `Command::cargo_bin("wiki")` | `wiki` | `llmwiki-cli` |
| `src/cli/install_skill.rs` `command: "wiki"` references | `wiki` | `llmwiki-cli` |
| `src/skills/**/*.md` command examples (`wiki search ...`) | `wiki` | `llmwiki-cli` |
| Plan / spec docs (current + historical) | `wiki` | `llmwiki-cli` |
| `install.sh` (when written) BINARY variable | `wiki` | `llmwiki-cli` |
| `agents/skills/wiki/SKILL.md` (build artifact) | command examples | `llmwiki-cli` |
| `README.md`, `CHANGELOG.md` | `wiki` | `llmwiki-cli` |
| `AGENTS.md` references | `wiki` | `llmwiki-cli` |

**NOT renamed** (because they are nouns, not program names):

- The skill bundle directory `~/.agents/skills/wiki/` — it's "the wiki skill", not "the wiki CLI skill".
- `wiki-root.toml`, `wiki/` subdirectory in a project, `wiki/` in `wiki/pharma/index.md` references — these are knowledge-base nouns.
- The skill content's noun uses: "the wiki contains...", "browse the wiki", etc.
- The project repo folder `/Users/felix/Documents/llmwiki/` — already correct.
- The skill frontmatter `name: wiki` (the skill is called "wiki" because that's what it manages) — stays.

**Breaking change note**: existing users who installed via `cargo install wiki` (or via the previous install script) will need to `cargo uninstall wiki` and reinstall as `cargo install llmwiki-cli`. The CHANGELOG must call this out as the v0.3.0 breaking change.

### 3.2 Cleanup follow-ups

1. **`src/cli/embed.rs`** — add `validate_or_error(&cfg)?;` after `let mut cfg = resolve_config(&ws)?;`. Catches `chunk_overlap_tokens >= default_chunk_tokens` before the chunker runs. Same pattern as `search.rs` / `query.rs`.

2. **`tests/search_test.rs`** (rename to `tests/lsp_search_test.rs`? — no, the test stays for `llmwiki-cli search`) — add `search_fails_fast_on_bad_embed_model`: spin up a wiremock NIM, build a `wiki-root.toml` with `[w.nim] embed_model = "nvidia/bogus"`, register the workspace alias via `WIKI_ROOT_CONFIG`, run `llmwiki-cli --wiki w search foo`, assert non-zero exit and stderr contains `"unsupported embed_model"`. Verify wiremock did NOT receive the embeddings request (proves the fail-fast).

3. **`src/skills/SETUP/SKILL.md`** — embed a fenced ` ```json` block of the JSON Schema produced by `llmwiki-cli config show-schema`, refreshed at build time by `build.rs`. Same mechanism as the existing `agents/skills/wiki/SKILL.md` stub generation.

**Effort**: half a day.

---

## 4. Phase 1 — Installability

### 4.1 `Cargo.toml` upgrades

```toml
[package]
name = "llmwiki-cli"
version = "0.3.0"
description = "Karpathy-style LLM Wiki CLI — manage multiple wikis, embed pages, search semantically"
license = "MIT"
repository = "https://github.com/<owner>/llmwiki"
homepage = "https://github.com/<owner>/llmwiki"
readme = "README.md"
keywords = ["llmwiki", "karpathy-wiki", "wiki", "knowledge-base", "rag"]
categories = ["knowledge-base", "command-line-utilities", "development-tools"]
edition = "2021"

[profile.release]
lto = "thin"
codegen-units = 1
strip = "symbols"

[package.metadata.binstall]
bin = "llmwiki-cli"
```

The version bump to 0.3.0 signals the rename + LSP + MCP additions. The `binstall` metadata lets users install via `cargo binstall llmwiki-cli` (no compilation needed).

### 4.2 `install.sh` (repo root)

~80 lines of POSIX bash, modeled on rustup and kimi-code's installer:

```bash
#!/usr/bin/env bash
set -euo pipefail

REPO="<owner>/llmwiki"
BINARY="llmwiki-cli"
INSTALL_DIR="${LLMWIKI_INSTALL_DIR:-$HOME/.local/bin}"

# 1. Detect OS and architecture.
# 2. Map to release target triple (e.g. darwin-aarch64-apple-darwin).
# 3. Fetch latest release tag from https://api.github.com/repos/$REPO/releases/latest.
# 4. Download https://github.com/$REPO/releases/download/$TAG/$BINARY-$TARGET.tar.gz
#    plus $BINARY-$TARGET.tar.gz.sha256.
# 5. Verify SHA256.
# 6. Extract and chmod +x.
# 7. Install to $INSTALL_DIR (mkdir -p if missing).
# 8. Print next-steps banner:
#      "Installed llmwiki-cli $VERSION to $INSTALL_DIR.
#       Add $INSTALL_DIR to PATH if not already.
#       Run 'llmwiki-cli doctor' to verify."
```

`install.sh` is also mirrored at `marketplace/install.sh` for the skill to reference via a stable raw URL.

### 4.3 GitHub Actions: `.github/workflows/`

**`ci.yml`** — every PR + push to main:

- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`
- `python marketplace/scripts/validate.py --strict`
- Cache: `Swatinem/rust-cache@v2`.

**`release.yml`** — push of `v*` tag:

```yaml
name: release
on:
  push:
    tags: ['v*']
jobs:
  release:
    strategy:
      matrix:
        target:
          - aarch64-apple-darwin
          - x86_64-apple-darwin
          - aarch64-unknown-linux-musl
          - x86_64-unknown-linux-musl
          - x86_64-pc-windows-msvc
    runs-on: ubuntu-latest  # cross-compile handles darwin+windows
    steps:
      - uses: actions/checkout@v4
      - uses: taiki-e/install-action@v2  # for cross
      - run: cross build --release --target ${{ matrix.target }} --bin llmwiki-cli
      - run: strip target/${{ matrix.target }}/release/llmwiki-cli
      - run: sha256sum target/.../llmwiki-cli > llmwiki-cli-${{ matrix.target }}.sha256
      - uses: softprops/action-gh-release@v2
        with:
          files: |
            llmwiki-cli-${{ matrix.target }}.tar.gz
            llmwiki-cli-${{ matrix.target }}.sha256
            install.sh
```

**`publish-crates.yml`** — runs after `release.yml` succeeds:

- `cargo publish` with a repo-scoped crates.io token stored as a secret.

**Effort**: 1 day.

---

## 5. Phase 2 — Agent-agnostic marketplace plugin

### 5.1 Directory structure

Co-located in the main repo at `marketplace/`:

```
marketplace/
  .claude-plugin/
    plugin.json
    .lsp.json
  .kimi-plugin/
    plugin.json
    .lsp.json
  .cursor-plugin/
    plugin.json
    .lsp.json
  .codex-plugin/
    plugin.json
    .lsp.json
  marketplace.json           # catalog entry
  skills/
    wiki/
      SKILL.md               # decision router
      SETUP/SKILL.md         # bootstrap CLI install
      INGEST/SKILL.md
      SEARCH/SKILL.md
      QUERY/SKILL.md
      LINT/SKILL.md
      MODELS/SKILL.md
      SYNC/SKILL.md
      TROUBLESHOOTING/SKILL.md
      LSP/SKILL.md           # NEW in Phase 4
      MCP/SKILL.md           # NEW in Phase 5
      evals/
        evals.json           # 3-5 prompts per sub-skill
      references/            # heavy docs loaded on demand
        registry-schema.md
        install.md
        lsp.md
        mcp.md
  install.sh                 # mirror of repo-root
  scripts/
    validate.py              # skill format linter (Python stdlib)
    evals.py                 # smoke-test runner
  CHANGELOG.md
  README.md
  VERSION                    # bumped by release.yml
```

### 5.2 The 4 sibling manifests

**`marketplace/.claude-plugin/plugin.json`** (canonical, 9 lines):

```json
{
  "name": "wiki",
  "version": "0.3.0",
  "description": "Karpathy-style LLM Wiki — manage multiple wikis, embed, search, lint",
  "author": { "name": "<owner>" },
  "license": "MIT",
  "skills": "./skills/",
  "lspServers": "./.lsp.json"
}
```

**`marketplace/.claude-plugin/.lsp.json`**:

```json
{
  "wiki-lsp": {
    "command": "llmwiki-cli",
    "args": ["lsp"],
    "extensionToLanguage": { ".toml": "toml" },
    "transport": "stdio"
  }
}
```

**`marketplace/.kimi-plugin/plugin.json`** (adds `skillInstructions` mapping generic verbs to native kimi tools):

```json
{
  "name": "wiki",
  "version": "0.3.0",
  "skills": "./skills/",
  "lspServers": "./.lsp.json",
  "skillInstructions": "Tool mapping: 'spawn a subagent' → Agent; 'run a shell command' → Bash; 'read a file' → Read; 'edit a file' → Edit. The plugin prefers LSP-driven diagnostics over re-reading files when available."
}
```

**Cursor and Codex manifests** mirror the Kimi Code shape minus the `skillInstructions` (or with a Cursor-/Codex-appropriate `skillInstructions` string).

### 5.3 Skill format conformance

Every `SKILL.md` in `marketplace/skills/wiki/**` conforms to the taches-principled-light rules enforced by `scripts/validate.py`:

**YAML frontmatter:**

```yaml
---
name: <sub-skill-name>
description: |
  <one-paragraph trigger description, <= 1024 chars>
whenToUse: |
  - <positive trigger 1>
  - <positive trigger 2>
  - NOT for: <negative trigger>
argument-hint: "[<arg>]"
allowed-tools: <comma-separated list>
license: MIT
---
```

**Body structure (in order):**

1. **Decision Router** table: `| User says | Action |`
2. **Reference Index** table: `| Reference | Purpose |`
3. Imperative citations: `MUST read references/X.md BEFORE Y.`
4. Procedure block (when to call which `llmwiki-cli` subcommand).
5. **Anti-patterns** section with `❌`.
6. **CONTRAST** section: "NOT for X — use the `<sibling-skill>` sub-skill instead."
7. Closing **When NOT to load** block.

**Body cap**: 500 lines; longer content moves to `references/`.

### 5.4 `marketplace/scripts/validate.py`

~400 lines of Python stdlib, mirrors taches-principled-light's `marketplace-validator`:

- Per-skill lint: frontmatter schema, name regex, name↔dir match, description length & format, body line count.
- Hardcoded tool-name blocklist (`HARDCODED_TOOL_NAMES = ["Agent", "Bash", "Read", "Edit", ...]`).
- Stale platform ref check (e.g. flags `name: claude-only` references in `.cursor-plugin/` files).
- Cross-reference integrity: every `references/X.md` path in a SKILL.md resolves.
- Manifest consistency across the 4 sibling manifests.
- Flags at three levels: `fail` (exit 2), `warn` (exit 1 with `--strict`), `info`.
- `--json` output mode for CI.

### 5.5 Skill rebuild into the binary

`build.rs` is updated to:

1. Copy `marketplace/skills/wiki/SKILL.md` → `agents/skills/wiki/SKILL.md` (existing behavior).
2. Copy each `marketplace/skills/wiki/<SUB>/SKILL.md` → `agents/skills/wiki/<SUB>/SKILL.md`.
3. Regenerate the `schema.json` block in `agents/skills/wiki/SETUP/SKILL.md` by calling `llmwiki-cli config show-schema` against a fixture `wiki-root.toml` (or, more cleanly, the same logic inline in build.rs using `schemars` directly).

`src/skills/mod.rs` continues to use `include_str!` for the 8 embedded sub-skill strings.

### 5.6 Marketplace auto-update

- If the marketplace is added to the **official Anthropic / kimi-code catalog**: auto-update ON by default.
- If **third-party / self-hosted**: auto-update OFF by default; users run `/plugin marketplace update wiki` (Claude Code) or `/plugins update wiki` (Cursor) explicitly.
- The skill's `TROUBLESHOOTING` page documents the update command per runtime.
- **Hard-dependency changes** (e.g. dropping a CLI flag) must be a major version bump with a CHANGELOG callout. Per the kimi-plugin-cc alpha.1 → alpha.2 cautionary tale.

**Effort**: 2-3 days.

---

## 6. Phase 3 — Auto-install via the skill

The `marketplace/skills/wiki/SETUP/SKILL.md` skill's first action is three-step verify-and-install, mirroring the `/kimi:setup` pattern from `linxule/kimi-plugin-cc`:

```markdown
## Before anything else

Run this block. If step 1 fails, run step 2, then re-run step 1.

\`\`\`bash
# 1. Detect
command -v llmwiki-cli && llmwiki-cli --version

# 2. Install (if missing)
curl -LsSf https://github.com/<owner>/llmwiki/raw/main/install.sh | bash

# 3. Verify
llmwiki-cli doctor
\`\`\`

If `llmwiki-cli doctor` reports an error, STOP. Do not proceed to
INGEST/SEARCH/etc. The subsequent commands will fail in opaque ways
on an unverified install.

If the user prefers no auto-install, they can pre-install via their
package manager (`cargo install llmwiki-cli`, `cargo binstall
llmwiki-cli`, or `brew install llmwiki-cli` once a tap exists) — the
skill just skips step 2 in that case.
```

**Companion slash command**: `SETUP/SKILL.md` declares `argument-hint: "[--check]"` so the runtime exposes it as `/wiki:setup [--check]`. With `--check`, the skill runs only step 1 + 3 (detect + verify), never step 2.

**Trust boundary**: the skill is a *verifier*, not an *auto-installer*. It does NOT execute `install.sh` for the user. The user runs the curl command themselves. This is the model `linxule/kimi-plugin-cc` uses, and it keeps the security posture clear (the skill is content, not executable).

**Edge cases the SETUP skill handles:**

- `command -v llmwiki-cli` succeeds but `--version` fails → corrupted install; print reinstall steps.
- `llmwiki-cli doctor` reports a missing API key → print `export NVIDIA_NIM_API_KEY=...` and pause.
- `llmwiki-cli doctor` reports no NIM connectivity → suggest `WIKI_NIM_BASE_URL` override.
- The user is on a restricted machine where `curl | bash` is blocked → fall back to `cargo install llmwiki-cli` instructions.

**Effort**: half a day.

---

## 7. Phase 4 — `llmwiki-cli lsp`

### 7.1 Crate additions

```toml
tower-lsp-server = "0.23"
lsp-types = "0.95"
toml_edit = "0.22"
```

All pure-Rust, no native deps, no async-runtime conflicts. Project already has `tokio` with `features = ["full"]`.

### 7.2 `src/cli/lsp.rs` (sketch)

```rust
use crate::cli::LspArgs;
use crate::error::WikiError;
use tower_lsp_server::{LspService, Server};
use tower_lsp_server::lsp_types::*;

struct Backend;

#[tower_lsp_server::async_trait]
impl tower_lsp_server::LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> jsonrpc::Result<InitializeResult> { ... }
    async fn initialized(&self, _: InitializedParams) { }
    async fn shutdown(&self) -> jsonrpc::Result<()> { Ok(()) }
    async fn did_open(&self, params: DidOpenTextDocumentParams) { ... }
    async fn did_change(&self, params: DidChangeTextDocumentParams) { ... }
    async fn did_close(&self, params: DidCloseTextDocumentParams) { ... }
    async fn hover(&self, params: HoverParams) -> jsonrpc::Result<Option<Hover>> { ... }
    async fn completion(&self, params: CompletionParams) -> jsonrpc::Result<Option<CompletionResponse>> { ... }
    async fn document_symbol(&self, params: DocumentSymbolParams) -> jsonrpc::Result<Option<DocumentSymbolResponse>> { ... }
}

pub async fn run(_args: LspArgs) -> Result<(), WikiError> {
    let (service, socket) = LspService::build(Backend).finish();
    Server::new(tokio::io::stdin(), tokio::io::stdout(), socket).serve(service).await;
    Ok(())
}
```

### 7.3 Method implementations (all stateless)

| Method | Behavior |
|---|---|
| `did_open` | Parse `params.text_document.text`. Cache nothing. Run domain functions; push diagnostics via `client.publish_diagnostics`. |
| `did_change` | Same as `did_open`, using `params.content_changes[0].text`. |
| `did_close` | `client.publish_diagnostics(uri, vec![])` to clear. |
| `hover` | Read URI from disk if not in params; find cursor key via `lsp_domain::key_at_position`; call `lsp_domain::hover_for`; return `Hover`. |
| `completion` | Same; `lsp_domain::completion_for` returns `Vec<CompletionItem>` based on parent path + schema. |
| `document_symbol` | `lsp_domain::symbols_for(text)` returns `Vec<DocumentSymbol>` (one `Namespace` per top-level table, one `Property` per leaf). |

### 7.4 `src/core/lsp_domain.rs` — shared with MCP

```rust
use tower_lsp_server::lsp_types::{Diagnostic, Hover, MarkupContent, MarkupKind, ...};

pub fn parse_config(text: &str) -> Result<Config, Vec<Diagnostic>>;
pub fn validate_config(cfg: &Config) -> Vec<Diagnostic>;
pub fn key_at_position(text: &str, line: u32, character: u32) -> Option<String>;
pub fn hover_for(key: &str) -> Option<Hover>;
pub fn completion_for(parent_path: &[&str], cfg: &Config) -> Vec<CompletionItem>;
pub fn symbols_for(text: &str) -> Vec<DocumentSymbol>;
```

Each function is pure, returns data, never touches LSP or MCP types in a way that prevents reuse. LSP handlers wrap results in LSP-shaped structs; MCP handlers wrap them in MCP-shaped structs.

### 7.5 Dodge the Claude Code bugs

- Re-parse the file content from `params.text_document.text` (or from disk) on every request. Never rely on `didOpen` having landed first.
- Return `None` / empty list on parse error rather than panicking. Diagnostics will surface the parse error separately.
- 200ms timeout per request — return early with empty result if exceeded.
- Don't cache `Vec<Diagnostic>` between requests — recompute every time. This dodges #32265 (stale empty-result bug).

### 7.6 Editor config snippets shipped in `skills/wiki/LSP/SKILL.md`

- **Helix** (`~/.config/helix/languages.toml`): 8 lines.
- **Neovim** 0.11+ (`~/.config/nvim/lsp/wiki.lua`): 10 lines.
- **Zed** (`~/.config/zed/settings.json`): 6 lines, with a documented caveat that custom LSP names need a WASM extension.
- **VS Code / Cursor**: requires a `.vsix` extension (Phase 6 — deferred). The skill documents the workaround: use a generic TOML LSP and call `llmwiki-cli doctor` for validation.

Also documented: the `mcp-language-server` bridge for users who already have it configured (covers Continue, Aider-via-mcp, etc.).

**Effort**: 3-4 days.

---

## 8. Phase 5 — `llmwiki-cli mcp`

### 8.1 Crate addition

```toml
rmcp = { version = "0.1", features = ["server", "macros"] }
```

### 8.2 `src/cli/mcp.rs` (sketch)

```rust
use crate::cli::McpArgs;
use crate::error::WikiError;
use rmcp::{ServerHandler, ServiceExt, model::*, transport::stdio};

#[derive(Clone)]
struct WikiMcp;

#[rmcp::tool_handler]
impl WikiMcp {
    async fn validate(&self, config_text: String) -> Result<CallToolResult, McpError> {
        // Re-use lsp_domain::parse_config + validate_config, return JSON.
    }
    async fn hover(&self, config_text: String, line: u32, character: u32) -> Result<CallToolResult, McpError> { ... }
    async fn completion(&self, config_text: String, line: u32, character: u32) -> Result<CallToolResult, McpError> { ... }
    async fn schema(&self) -> Result<CallToolResult, McpError> { ... }
    async fn doctor(&self, workspace: Option<String>) -> Result<CallToolResult, McpError> { ... }
}

impl ServerHandler for WikiMcp { ... }

pub async fn run(_args: McpArgs) -> Result<(), WikiError> {
    let service = WikiMcp.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
```

### 8.3 Tool surface

| Tool | Input | Output |
|---|---|---|
| `validate` | `{ config_text: string }` | `{ valid: bool, errors: string[], parsed: Config \| null }` |
| `hover` | `{ config_text, line, character }` | `{ contents: { kind: "markdown", value: string } } \| null` |
| `completion` | `{ config_text, line, character }` | `{ items: [{ label, kind, detail, documentation }] }` |
| `schema` | `{}` | `schemars::schema_for!(Config)` JSON |
| `doctor` | `{ workspace?: string }` | Same shape as `llmwiki-cli doctor --json` |

All 5 tools are stateless — `config_text` is a required argument on every call, never session state.

### 8.4 Why native MCP, not just LSP

Cursor's AI does NOT consume the editor's running LSP — it has its own embedding-based retrieval. The MCP bridge (`mcp-language-server`) is a workaround, but it spawns a fresh LSP process per call and has known position-mapping fragility (per the `Latias94/lspi` fork's history).

A native `llmwiki-cli mcp` gives Cursor's AI (and Claude Desktop, and Continue's MCP path) first-class access to the same domain logic, without position-mapping games. 5 well-defined tools beat 1 fuzzy LSP bridge.

### 8.5 Skill: `marketplace/skills/wiki/MCP/SKILL.md`

Short — 30 lines. Documents:

- How to register the MCP server in Claude Desktop (`claude_desktop_config.json`):
  ```json
  {
    "mcpServers": {
      "wiki": { "command": "llmwiki-cli", "args": ["mcp"] }
    }
  }
  ```
- How to register in Cursor (same JSON shape, `~/.cursor/mcp.json`).
- The 5 tools and what they're for.
- "When NOT to use" — for routine CLI work, prefer `llmwiki-cli <subcommand>` directly; the MCP path is for the AI agent to call from a tool.

**Effort**: 2-3 days.

---

## 9. Release infrastructure & CI

### 9.1 CI workflows (in `.github/workflows/`)

| File | Trigger | What |
|---|---|---|
| `ci.yml` | every PR + push to main | fmt, clippy, test, marketplace validator, skill smoke |
| `release.yml` | push of `v*` tag | cross-compile binaries, attach to GitHub Release, bump `marketplace/VERSION` |
| `publish-crates.yml` | after `release.yml` success | `cargo publish` with repo-scoped token |

### 9.2 Cross-compile matrix

```yaml
strategy:
  matrix:
    target:
      - aarch64-apple-darwin
      - x86_64-apple-darwin
      - aarch64-unknown-linux-musl
      - x86_64-unknown-linux-musl
      - x86_64-pc-windows-msvc
```

Uses `taiki-e/cross-toolchain` + `cargo-zigbuild` for portable Linux/Windows builds from an Ubuntu runner.

### 9.3 Marketplace versioning

`marketplace/VERSION` is a plain `0.3.0` string, bumped by `release.yml` as the last step. The 4 sibling `plugin.json` files reference this version. When Anthropic / Kimi Code catalogs pull the marketplace, they pin to this version.

**Effort**: included in Phase 1 (CI) + Phase 2 (marketplace.json + VERSION file).

---

## 10. Testing strategy

### 10.1 Test pyramid

| Layer | What | Where |
|---|---|---|
| Unit | Schema walking, `validate()`, TOML span mapping, completion generation, hover content | `src/core/lsp_domain.rs` `#[cfg(test)] mod tests` |
| Library integration | Cross-function domain tests against fixtures | same |
| CLI subprocess | `llmwiki-cli lsp --help`, `llmwiki-cli mcp --help`, error paths | `tests/lsp_cli_test.rs`, `tests/mcp_cli_test.rs` |
| LSP protocol | Spawn `llmwiki-cli lsp` as subprocess, send real JSON-RPC, verify responses | `tests/lsp_protocol_test.rs` using `lsp-server` in client mode |
| MCP protocol | Same pattern with `rmcp` client | `tests/mcp_protocol_test.rs` |
| Install script | `bash install-test/test.sh` in CI container | `.github/workflows/install-test.yml` |
| Marketplace | `python marketplace/scripts/validate.py --strict` | run in CI |
| Skill smoke | `bash tests/skill_smoke.sh` invokes `llmwiki-cli install-skill` and asserts the skill tree is correct | `tests/skill_smoke.sh` |
| E2E | Manual + documented in skill; not in CI (env-dependent: requires NIM API key) | — |

### 10.2 LSP protocol test (sketch)

```rust
// tests/lsp_protocol_test.rs
use lsp_server::{Connection, Message};

#[test]
fn lsp_hover_returns_docstring_for_embed_model() {
    let (mut conn, _child) = spawn_wiki_lsp();
    conn.initialize_start(InitializeParams { ..Default::default() }).unwrap();
    conn.initialize_finish(InitializeResult { capabilities: .. }).unwrap();

    let req = conn.request(1, "textDocument/hover", json!({
        "textDocument": { "uri": "file:///tmp/wiki-root.toml" },
        "position": { "line": 2, "character": 4 }
    })).unwrap();

    let resp = conn.handle_message().unwrap();
    let hover: Hover = serde_json::from_value(resp.result).unwrap();
    assert!(hover.contents.markup_value.contains("Embedding model"));
}
```

### 10.3 MCP protocol test (sketch)

```rust
// tests/mcp_protocol_test.rs
#[tokio::test]
async fn mcp_validate_catches_bad_model() {
    let mut client = spawn_wiki_mcp().await;
    let tools = client.list_tools().await.unwrap();
    assert!(tools.iter().any(|t| t.name == "validate"));

    let result = client.call_tool("validate", json!({
        "config_text": "[w]\npath=\"/tmp/w\"\n[w.nim]\nembed_model=\"nvidia/bogus\""
    })).await.unwrap();

    let content: ValidationReport = serde_json::from_value(result.content).unwrap();
    assert!(!content.valid);
    assert!(content.errors.iter().any(|e| e.contains("unsupported embed_model")));
}
```

### 10.4 Coverage target

80%+ of `src/core/lsp_domain.rs` lines (where LSP and MCP semantics live). The wire-format glue (tower-lsp, rmcp) is tested via the protocol-level tests, not direct unit tests.

---

## 11. Risks & open questions

### 11.1 Resolved during brainstorming

- **Crate name**: `llmwiki-cli`. Binary name: `llmwiki-cli`. The noun "wiki" stays in skill names, paths, and `wiki-root.toml`.
- **Architecture**: single crate, single binary, `lsp` and `mcp` are subcommands.
- **Distribution**: `curl install.sh | bash` + crates.io + GitHub Releases binaries.
- **Agent runtimes**: Claude Code + Kimi Code + Cursor + Codex in v1.
- **LSP + MCP**: both ship in v1. MCP is native (not bridge).

### 11.2 Resolved during research

- **AI-agent LSP consumption**: Claude Code uses `publishDiagnostics`, `hover`, `documentSymbol`, `definition`, `references`, `callHierarchy` — but does NOT use `completion` yet. Cursor's AI does not consume editor LSP at all (needs MCP). Continue's `languageServers` is editor-side. Aider / Cline inherit the host editor's LSP.
- **Marketplace auto-update**: ON for official Anthropic / Kimi Code catalogs; OFF for third-party. Document both in `TROUBLESHOOTING/SKILL.md`.
- **Claude Code bugs to dodge**: #32067 (missing `didOpen`), #32265 (empty results on repeated queries), #15521 (plugin load race). All mitigated by stateless-per-request server design.

### 11.3 Open risks (flagged during brainstorming)

| # | Risk | Mitigation |
|---|---|---|
| R1 | `rmcp` crate API may churn (it's relatively new) | Pin a specific minor version in Cargo.toml; watch release notes |
| R2 | Stateless LSP re-parses TOML on every request — fine for a few-KB file, slow for larger | Add a `didOpen`/`didChange` cache *later* if real-world usage shows lag; document the trade-off |
| R3 | Cursor's custom-LSP-name limitation — needs a WASM extension to register `wiki-lsp` | Document the workaround in `LSP/SKILL.md`. Ship `.vsix` + WASM extension in Phase 6 |
| R4 | Auto-update breakage on hard CLI changes (kimi-plugin-cc alpha.1 → alpha.2 cautionary tale) | Keep CLI surface stable across minor versions; mark breaking changes explicitly in CHANGELOG; TROUBLESHOOTING references breaking-changes section |
| R5 | crates.io first-publish friction (email confirmation, 2FA) | Maintainer runs first publish manually; CI handles subsequent |
| R6 | `install.sh` on Windows — works in Git Bash / WSL, awkward in cmd.exe | Document the Git Bash requirement. Ship `install.ps1` in Phase 6 |
| R7 | Co-located `marketplace/` vs separate `wiki-marketplace` repo — co-located chosen for simplicity, but tightens coupling between CLI source and marketplace metadata | If coupling becomes a problem, split into `wiki-marketplace` later |

---

## 12. Effort summary

| Phase | Description | Effort |
|---|---|---|
| 0 | Rename + 3 cleanup follow-ups | 0.5 day |
| 1 | Installability (Cargo.toml, install.sh, GitHub Releases CI, crates.io) | 1 day |
| 2 | Agent-agnostic plugin (4 manifests, skill restructure, validator) | 2-3 days |
| 3 | Auto-install via SETUP skill | 0.5 day |
| 4 | `llmwiki-cli lsp` (tower-lsp-server, 4 methods, domain layer) | 3-4 days |
| 5 | `llmwiki-cli mcp` (rmcp, 5 tools) | 2-3 days |
| **Total** | | **9-12 days** |

Ship as a coordinated release: **v0.3.0**.

---

## 13. Out of scope for v0.3.0 (Phase 6 candidates)

- VS Code / Cursor `.vsix` extension (requires TypeScript wrapper + WASM).
- Windows `install.ps1`.
- Homebrew tap.
- `textDocument/formatting` LSP method.
- LSP `didOpen`/`didChange` cache (only if stateless proves too slow).
- MCP `resources/list` for serving embedded skill content as MCP resources.
- In-binary plugin download (vs. marketplace-mediated install).
