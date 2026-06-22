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

# wiki

Personal Karpathy-style LLM Wiki — markdown + JSONL embeddings, no database.

## Filesystem layout

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

## When to use `wiki` CLI

The user has markdown notes. Use the `wiki` CLI when:
- **Search/query** — semantic search or RAG against NIM embeddings
- **Add sources** — `wiki ingest` to add a new source
- **Manage config** — switch embedding model, manage multiple wikis
- **Setup on a new device** — install CLI, register wikis, install skill

Otherwise: use `Read` / `Grep` / `Glob` directly on the wiki's `wiki/`, `raw/`, and `index.md` files.

## Sub-skills

Run `wiki skill show <topic>` to load the full content for a sub-skill.

| Topic | Use when |
|-------|----------|
| `setup` | First-run install, init, registry, skill install |
| `ingest` | Adding a source file to the wiki |
| `search` | Semantic search by vector similarity |
| `query` | RAG question-answering with citations |
| `lint` | Hygiene checks, broken links, frontmatter |
| `models` | Switch embedding/reranking model |
| `sync` | New-device setup, tailnet git sync |
| `troubleshooting` | Diagnose wiki errors |

## Quick commands

```bash
wiki config list                    # show registered wikis
wiki config validate                # check [defaults] + every alias parses + passes
wiki --wiki pharma search "..."     # semantic search
wiki --wiki pharma ingest foo.md    # add a source
wiki --wiki pharma query "..."      # RAG question
wiki --wiki pharma lint             # check hygiene
wiki doctor                          # diagnose NIM + config
```

## Multiple wikis

- `--wiki <alias>` selects a wiki without `cd`-ing
- `WIKI_ACTIVE=<alias>` env var equivalent
- The CLI discovers wikis by CWD prefix match against `wiki-root.toml` paths
- All config lives in `wiki-root.toml` (no more `.wiki/config.yaml`)
