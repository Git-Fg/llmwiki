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

`wiki skill get` content is served by the CLI itself, so it is always
version-matched with the installed binary. **Always prefer `wiki skill get`
over guessing commands** — sub-skills are short, opinionated, never stale.

## Core gotchas (read once)

1. **`llmwiki-cli doctor` first** if anything is misbehaving. Catches
   missing API keys, NIM connectivity, broken config, orphans in one
   pass. `--json` is machine-readable for CI.
2. **Embeddings are gitignored.** After pulling wiki changes from
   another machine, run `llmwiki-cli embed` before `search` / `query`.
3. **`llmwiki-cli init` defaults to flat layout** (since v0.3.27).
   Use `--subdir` for the legacy `wiki/` subdir layout.
4. **`wiki.exclude_dirs` is additive** (since v0.3.27). User entries
   merge with built-in defaults (node_modules, .git, .opencode, etc.).
5. **Run `llmwiki-cli config validate` after editing any `*.toml`.**
   Catches typos, bad TOML, bad model names before opaque failures.
6. **Use `--wiki <alias>` to target a specific wiki without `cd`-ing.**
   Combined with `llmwiki-cli config list` to see registered aliases.

## Inline sub-skills (load via `llmwiki-cli skill get <name>`)

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
