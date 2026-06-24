---
name: wiki
description: |
  Personal Karpathy-style LLM Wiki. Minimal agent skill — points the
  agent at `llmwiki-cli skill list` / `llmwiki-cli skill get <topic>`
  for inline sub-skills and `llmwiki-cli <command> --help` for full
  flag reference. Use when the user asks to ingest a source, search
  the wiki, answer a question against prior research, lint or maintain
  the wiki, set up on a new device, or pick a different NVIDIA NIM model.
  Prefer the wiki's native file tools for browsing; reach for the CLI
  only when semantic search or NIM-backed operations are needed.
allowed-tools: Bash(llmwiki-cli:*)
---

# wiki

Minimal bootstrap skill. The CLI is the source of truth for everything below.

## Auto-discover

```bash
llmwiki-cli skill list                  # every inline sub-skill
llmwiki-cli skill get <topic>           # load one (e.g. wiki-search, wiki-config)
llmwiki-cli <command> --help            # full flag reference for any command
```

The `llmwiki-cli skill get <topic>` content is served by the CLI itself, so it
is always version-matched with the installed binary. **Always prefer
`llmwiki-cli skill get` over guessing commands** — the inline sub-skills are
short, opinionated, and never go stale.

## Core gotchas (read once, save yourself debugging time)

1. **Always run `llmwiki-cli doctor` first** if anything is misbehaving. It
   catches missing API keys, NIM connectivity issues, broken config, and
   orphans in one pass. `llmwiki-cli doctor --json` is machine-readable for
   CI / scripts.

2. **Embeddings are gitignored.** After pulling wiki changes from another
   machine, run `llmwiki-cli embed` to rebuild `embeddings.jsonl` before
   `llmwiki-cli search` or `llmwiki-cli query` will return useful results.

3. **`llmwiki-cli init` defaults to flat layout** (since v0.3.27). Use
   `llmwiki-cli init --subdir` for the legacy `wiki/` subdir layout.

4. **`wiki.exclude_dirs` is additive** (since v0.3.27). User-provided
   entries merge with built-in defaults (node_modules, .git, .opencode,
   .claude, .mavis, .harness, .principled, etc.), not replace them.

5. **Always run `llmwiki-cli config validate` after editing any `*.toml` file.**
   It catches typos, invalid TOML syntax, invalid model names, and bad
   chunk-size combinations before they cause opaque failures downstream.

6. **Use `--wiki <alias>` to target a specific wiki without `cd`-ing.**
   Combined with `llmwiki-cli config list` to see registered aliases, this
   is the fastest way to operate on multiple wikis.

## Internal command summary

| Command | Purpose |
|---|---|
| `llmwiki-cli init [path]` | Scaffold a new wiki |
| `llmwiki-cli ingest <file>` | Add a raw source + append to `log.md` |
| `llmwiki-cli build` | Compile pending raw sources into pages |
| `llmwiki-cli embed` | Build `embeddings.jsonl` over wiki pages |
| `llmwiki-cli search <query>` | Semantic search by vector similarity |
| `llmwiki-cli query <question>` | RAG question-answering with citations |
| `llmwiki-cli lint` | Hygiene checks (frontmatter, links, tags) |
| `llmwiki-cli doctor` | Diagnose config + NIM + workspace |
| `llmwiki-cli status` | Coverage stats (pages / embeddings / raw) |
| `llmwiki-cli tree` | Flat, grep-friendly page listing |
| `llmwiki-cli ls [--pages\|--raw\|--embed\|--links\|--config] [--json]` | Granular workspace listing |
| `llmwiki-cli models` | List whitelisted NVIDIA NIM models |
| `llmwiki-cli config <sub>` | show-effective / show-schema / validate / edit |
| `llmwiki-cli skill list` | List inline sub-skills |
| `llmwiki-cli skill get <topic>` | Load one inline sub-skill |
| `llmwiki-cli install-skill [--global\|--workspace <path>]` | Install this bundle |

## Setup (two commands)

```bash
# 1. Install the CLI
cargo install llmwiki-cli              # or: curl ... install.sh | sh

# 2. Install the skill globally (optional — for AI agents)
llmwiki-cli install-skill --global     # writes ~/.agents/skills/wiki/SKILL.md
```

## Environment

```bash
export NVIDIA_NIM_API_KEY="nvapi-..."     # required — get at https://build.nvidia.com/
# NVIDIA_API_KEY is the fallback if NVIDIA_NIM_API_KEY is unset
# WIKI_NIM_BASE_URL overrides the default https://integrate.api.nvidia.com
```

## Inline sub-skill index (load via `llmwiki-cli skill get <name>`)

| Topic | Use when |
|---|---|
| `wiki-setup` | First-run install, init, registry, skill install |
| `wiki-config` | Edit wiki-root.toml or per-workspace config.toml |
| `wiki-ingest` | Add a source file to the wiki |
| `wiki-search` | Find content by semantic similarity |
| `wiki-query` | RAG question-answering with citations |
| `wiki-lint` | Hygiene checks, broken wikilinks, frontmatter |
| `wiki-models` | Switch embedding / reranking model |
| `wiki-sync` | New-device setup, tailnet git sync |
| `wiki-troubleshooting` | Diagnose wiki errors |