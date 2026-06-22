---
name: wiki-agent
description: |
  Minimal agent skill for the wiki CLI. Provides quick access to the wiki help system,
  lists available internal commands, and strongly steers the agent to use `wiki skill show`
  for full documentation and `wiki help` for command reference.
allowed-tools: Bash(wiki:*)
---

# Wiki Agent Skill

This is a **minimal bootstrap skill** — it tells you how to access the real help.

## 🔑 Golden Rule

**Always run `wiki skill show` first** when working with the wiki. The full skill content
(live, version-matched) is served by the CLI itself, not this stub.

## Quick Access

```bash
# Full skill guide (always up-to-date)
wiki skill show

# List all skill topics
wiki skill list

# Full command reference
wiki help

# Diagnose your setup
wiki doctor
```

## Internal Commands Available

| Command | Purpose |
|---------|---------|
| `wiki init [path]` | Scaffold a new wiki |
| `wiki ingest <source>` | Add raw source + compile to wiki pages |
| `wiki build` | List pending raw sources |
| `wiki embed` | Compute embeddings (or `--skip-existing`) |
| `wiki search <query>` | Semantic vector search |
| `wiki query <question>` | RAG-style query with citations |
| `wiki lint` | Hygiene checks (frontmatter, links, tags) |
| `wiki models` | List supported NVIDIA NIM models |
| `wiki doctor` | Diagnose config + NIM connectivity |
| `wiki status` | Show wiki stats (pages, embeddings, raw) |
| `wiki install-skill [--global]` | Install the bundled skill |
| `wiki skill show [topic]` | Print full skill content |
| `wiki skill list` | List all skill topics |
| `wiki tree` | Flat, grep-friendly page listing |
| `wiki ls [--pages\|--raw\|--embed\|--links\|--config] [--json]` | Granular workspace listing |

## Setup (Two Steps)

```bash
# 1. Install the CLI
cargo install --path .

# 2. Install the skill globally (or to workspace)
wiki install-skill --global
```

The skill installs to `~/.agents/skills/wiki/` (or `.agents/skills/wiki/` in your workspace).

## Environment

```bash
export NVIDIA_NIM_API_KEY="nvapi-..."   # Get at https://build.nvidia.com/
# or
export NVIDIA_API_KEY="nvapi-..."       # Fallback
```

## Skill Topics (via `wiki skill list`)

- `overview` — wiki philosophy & architecture
- `commands` — full command reference
- `ingest` — adding sources (PDF, text, markdown)
- `search` — semantic search patterns
- `query` — RAG query patterns
- `lint` — quality checks
- `models` — embedding/reranker model selection
- `sync` — git-based sync across devices