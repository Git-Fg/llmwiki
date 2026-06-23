# Changelog

## [0.3.0] - 2026-06-23 — BREAKING: rename to llmwiki-cli

**BREAKING CHANGES:**
- Crate name: `wiki` → `llmwiki-cli`
- Binary name: `wiki` → `llmwiki-cli`
- Reinstall: `cargo uninstall wiki && cargo install llmwiki-cli`

**Migration:**
- All existing scripts that invoke `wiki <subcommand>` must be updated to `llmwiki-cli <subcommand>`
- Existing wiki data, `wiki-root.toml`, and `~/.agents/skills/wiki/` are unchanged

**Added:**
- `llmwiki-cli lsp` — LSP server for `wiki-root.toml` (hover, completion, document symbols, diagnostics)
- `llmwiki-cli mcp` — MCP server (validate, hover, completion, schema, doctor)
- `llmwiki-cli config show-schema` — JSON Schema dump for editors
- `llmwiki-cli config validate` — field-level checks for `[defaults]` and every `[alias]`
- `validate_or_error()` called before NIM calls in `embed`/`search`/`query`
- Marketplace install: `install.sh` (POSIX) and `install.ps1` (PowerShell 7+) install `llmwiki-cli` + bundle the wiki skill into `~/.agents/skills/wiki/`
- Self-installing sub-skills: SETUP, LSP, MCP, INGEST, SEARCH, QUERY, LINT, MODELS, SYNC, TROUBLESHOOTING — all bundled in the binary and copy-installed via `llmwiki-cli install-skill`
- GitHub Actions release workflow building 6 targets: linux-musl (aarch64, x86_64), windows-gnu (aarch64, x86_64), apple-darwin (aarch64, x86_64)
- crates.io publish workflow

## [0.3.1] - 2026-06-23 — Multi-source wiki registry concatenation

**Added:**
- `wiki-root.toml` lookup now walks up from CWD for project-local
  `.agents/wiki-root.toml`. All sources (user-global chain + ancestor
  walk-up) are **concatenated**, with closer-to-CWD winning on alias
  conflict. Every wiki alias from every source is visible to CLI, LSP,
  and MCP — no shadowing, no silent fallbacks. Mirrors git (local +
  global), hk (per-project + per-user), Atmos (CWD + parent search).
- Improved error message: `WikiRootNotFound` now surfaces
  `$WIKI_ROOT_CONFIG=<path>` explicitly when the env var is set,
  saving users from guessing why their custom path was rejected.

**Changed:**
- `wiki config set/unset/add/rm` writes to the highest-priority
  registry file in scope (project-local if present, otherwise
  user-global). To edit a lower-priority file, set
  `$WIKI_ROOT_CONFIG` to point at it directly.

## 2026-06-22 — NIM URL convention and API key env

The CLI's NIM integration has been corrected so the default `base_url` is the
host only (e.g. `https://integrate.api.nvidia.com`) and every command appends
`/v1/<endpoint>` consistently. This matches the OpenAI-style convention and
keeps `WIKI_NIM_BASE_URL` overrides simple to point at a local mock or a
different host.

The previous default was `https://integrate.api.nvidia.com/v1`, which made
`wiki embed` and `wiki query` call `/v1/v1/embeddings` and `/v1/v1/chat/completions`
and return 404 against the real NIM API. The `wiki doctor` endpoint check
(`/v1/models`) had the same shape but was working only by coincidence of the
trailing path. The end-to-end wiremock test passed because `MockServer.uri()`
has no `/v1` suffix, masking the bug.

The API key resolver also now accepts `NVIDIA_API_KEY` as a fallback so users
with the upstream NVIDIA shell env don't need to re-export under a different
name. `NVIDIA_NIM_API_KEY` is still the primary lookup.

`wiki build` now detects raw sources with any extension (not just `.md`),
matching what `wiki ingest` actually writes — previously `.txt` (or any other)
sources were silently skipped.

## 2026-06-21 — Viewer removed

The SvelteKit viewer (`web/`), `wiki build-viewer`, and `wiki serve` commands
have been removed from the project. The wiki is consumed directly via the
CLI and the embedded agent skill — no static site is generated. This keeps
the tool focused on markdown + embeddings + agent-driven workflows.

## 2026-06-21 — Initial Rust port

Single-crate Rust CLI replacing the previous Python codebase. Markdown wiki
files + JSONL embeddings (gitignored, regenerated per device), NVIDIA NIM
embeddings + chat, embedded agent skill via `include_str!`.

## Importing an existing wiki

There is no `wiki import` command. External wiki layouts (e.g. an existing
`MyWiki` with `concepts/`, `entities/`, `raw/` directories) can be brought in
manually:

```bash
llmwiki-cli init ~/my-wiki                    # scaffold the CLI workspace
cp -r ~/MyWiki/concepts ~/my-wiki/wiki/
cp -r ~/MyWiki/entities ~/my-wiki/wiki/
# edit each file to add the required frontmatter (title, created, updated,
# type, tags, sources, schema_version) and [[wikilinks]] the CLI expects
llmwiki-cli embed                             # build the embedding index
```

A dedicated adapter was considered and rejected: automatic frontmatter
inference from parent directory names and wikilink rewriting are speculative
heuristics, not mechanical transformations, and the cost of a wrong inference
(mis-typed pages, broken links) outweighs the few manual copy/edit steps.

