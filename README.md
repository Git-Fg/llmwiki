# wiki

[![CI](https://github.com/Git-Fg/llmwiki/actions/workflows/ci.yml/badge.svg)](https://github.com/Git-Fg/llmwiki/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/llmwiki-cli.svg)](https://crates.io/crates/llmwiki-cli)
[![License](https://img.shields.io/crates/l/llmwiki-cli.svg)](https://github.com/Git-Fg/llmwiki/blob/main/LICENSE)

A markdown-based personal knowledge base, built on Andrej Karpathy's LLM Wiki pattern. Single Rust binary, NVIDIA NIM embeddings, agent skill integration, tailnet-friendly sync, multi-wiki registry.

## Install

Choose your platform:

```bash
# Linux, macOS, Windows-with-Git-for-Windows
curl -LsSf https://github.com/Git-Fg/llmwiki/releases/latest/download/install.sh | sh
```

```powershell
# Windows PowerShell 7+
irm https://github.com/Git-Fg/llmwiki/releases/latest/download/install.ps1 | iex
```

```bash
# Or any platform with the Rust toolchain installed
cargo install llmwiki-cli --locked
```

Then install the agent skill:

```bash
llmwiki-cli install-skill --global
```

This installs `llmwiki-cli` to `~/.local/bin/llmwiki-cli` (or `%LOCALAPPDATA%\llmwiki-cli\bin\llmwiki-cli.exe` on Windows). Add `~/.local/bin` to your shell PATH if not already there.

### Alternative: Workspace-local skill

```bash
llmwiki-cli install-skill  # installs to .agents/skills/wiki/ in current workspace
```

Use `--global` for system-wide access, omit for workspace-local.

## First-run setup

```bash
export NVIDIA_NIM_API_KEY="nvapi-..."        # Get one at https://build.nvidia.com/
llmwiki-cli init ~/my-wiki                         # create your wiki
cd ~/my-wiki
llmwiki-cli doctor                                 # verify config + NIM reachability
```

`llmwiki-cli init` auto-registers the wiki in `wiki-root.toml` (default: `~/.agents/wiki-root.toml`). To manage multiple wikis, see the [Multi-wiki registry](#multi-wiki-registry) section below.

## Quick Help

```bash
llmwiki-cli help              # full command reference
llmwiki-cli skill show        # complete agent skill guide
llmwiki-cli skill list        # list all skill topics
```

## Daily use

```bash
cd ~/my-wiki
git pull                                    # get latest markdown from other devices
llmwiki-cli ingest ~/Downloads/article.pdf         # add a source (auto-compiles to wiki pages)
llmwiki-cli search "what did I write about X?"     # semantic search
llmwiki-cli query "summarize the latest AI notes"  # RAG with citations
llmwiki-cli lint                                   # hygiene check
git add . && git commit -m "ingest: X" && git push
```

## Multi-wiki registry

The CLI supports multiple wikis via `wiki-root.toml` — a git-friendly TOML registry that the CLI reads. Concatenation semantics match `git config` (local + global):

- **Project-local** (highest priority): `.agents/wiki-root.toml` at any ancestor of CWD. Closer-to-CWD wins.
- **User-global** (lowest priority, loaded first): `~/wiki-root.toml`, `~/.claude/wiki-root.toml`, `~/.agents/wiki-root.toml`.

All sources merge — every wiki from every file is visible. `wiki config set/add/rm/unset` writes to the highest-priority file (project-local if present, otherwise user-global). To target a specific file, set `$WIKI_ROOT_CONFIG` to its absolute path.

```bash
llmwiki-cli config list                 # show all registered wikis
llmwiki-cli config path                 # print the active wiki-root.toml path
llmwiki-cli config get nim.embed_model  # read a config value
llmwiki-cli config set nim.embed_model nvidia/nv-embedqa-e5-v5 --wiki work
llmwiki-cli --wiki work search "X"      # target a specific wiki by alias
```

## Per-workspace & per-computer config (v0.3.7+)

Beyond the multi-wiki registry, NIM and wiki settings can be split between a **per-computer** default and a **per-workspace** override. Resolution priority (highest wins):

1. `$LLMWIKI_CONFIG` env var — points at a single config file (highest priority, hard override).
2. `<workspace>/.llmwiki-cli/config.toml` — per-workspace override, found by walking up from the resolved workspace looking for the closest `.llmwiki-cli/` ancestor.
3. `~/.llmwiki-cli/config.toml` — per-computer fallback (hidden dotfile directory).
4. Built-in defaults.

Use `wiki config paths` to see the resolved search order for the current workspace, `wiki config show-effective` to see which file overrode which key (mirrors `git config --list --show-origin`), and `wiki config config-edit` to open the highest-priority existing config file (or the per-workspace candidate if none exists) in `$EDITOR`.

### `wiki config show-effective` filters

Three orthogonal filters narrow the output for common audit workflows:

- `[<prefix>]` (positional) — only show keys starting with the prefix.
  Example: `wiki config show-effective nim.` shows only the `[nim]` table.
- `--source <path>` — only show keys whose source file matches the path.
  Example: `wiki config show-effective --source ~/.llmwiki-cli/config.toml`
  answers "what did the per-computer fallback set?".
- `--overrides-only` — hide keys whose value equals the built-in default.
  Surfaces only the keys your config files actually changed (the most
  useful subset for "what did my config do?").

Filters can be combined and apply to both text and JSON output. Example combining all three:

```bash
wiki config show-effective nim. --source ./config.toml --overrides-only --json
```

## Architecture

- **Wiki content**: Markdown files in `wiki/`, sources in `raw/`, catalog in `index.md`, log in `log.md`. All committed to git.
- **Registry**: `wiki-root.toml` (TOML, git-committed) — one entry per wiki with `[path, tags, description, what_to_read, qmd_slug, [alias.nim], …]`. Multi-wiki, multi-source, project-local + user-global merging.
- **Embeddings**: `embeddings.jsonl` (gitignored, regenerated per device via `llmwiki-cli embed`).
- **Config**: `[defaults]` + per-alias `[alias]` tables in `wiki-root.toml`. Legacy `~/.config/wiki/config.yaml` fallback still supported.
- **CLI**: Single Rust binary. No database.
- **Skill**: stub at `~/.agents/skills/wiki/SKILL.md` (copied via `llmwiki-cli install-skill`). Full content served by `llmwiki-cli skill show [topic]`.
- **Sync**: Git between devices. Embeddings regenerated locally.
- **No viewer**: The wiki is consumed directly via the CLI; no static site is generated.

## Commands

```
llmwiki-cli init <path>             scaffold a new wiki
llmwiki-cli ingest <source>         add raw source + log entry
llmwiki-cli build                   list pending raw sources
llmwiki-cli embed                   compute embeddings (--skip-existing)
llmwiki-cli search <query>          semantic search
llmwiki-cli query <question>        RAG-style query with citations
llmwiki-cli lint                    hygiene checks
llmwiki-cli ls [--pages|--raw|--embed|--links|--config]
                                   granular workspace listing
llmwiki-cli tree                    flat, grep-friendly page listing
llmwiki-cli models                  list supported NIM models
llmwiki-cli doctor                  diagnose config + NIM
llmwiki-cli status                  show wiki stats
llmwiki-cli config <subcommand>     manage wiki-root.toml (get/set/unset/add/rm/list/path/edit/validate/show-schema)
llmwiki-cli install-skill           install the bundled skill
llmwiki-cli skill show [topic]      print skill content
llmwiki-cli version                 print version
```

Run `llmwiki-cli --help` for the full list.

## Embedding models

Default: `nvidia/nv-embed-v1` (4096 dims, non-commercial). Other supported models:

- `nvidia/nv-embedqa-e5-v5` (1024 dims, commercial)
- `nvidia/nv-embedcode-7b-v1` (4096 dims, commercial)
- `nvidia/llama-nemotron-embed-1b-v2` (2048 dims, Matryoshka, commercial)
- `nvidia/llama-nemotron-embed-vl-1b-v2` (multimodal)
- Plus 3 reranker models

Run `llmwiki-cli models` for full specs. Change via `wiki config set nim.embed_model <model>`.

## Documentation

- Design spec: `docs/superpowers/specs/2026-06-21-karpathy-wiki-design.md`
- Multi-source registry spec: `docs/superpowers/specs/2026-06-23-multi-wiki-resolution-v032-design.md`
- Agent behavioral layer: `AGENTS.md`
- Full agent skill: `skills/SKILL.md` (bundled, `llmwiki-cli skill get` for inline sub-skills)

## License

Apache 2.0.
