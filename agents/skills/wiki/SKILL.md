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
allowed-tools: Bash(wiki:*)
---

# wiki

Run `wiki skill show` for the full guide. The skill content is shipped
inside the `wiki` binary itself, so it always matches the installed version.

```bash
wiki init                         # scaffold a new wiki
wiki ingest <source>              # add raw source + compile
wiki build                        # compile pending raw sources
wiki embed                        # compute embeddings
wiki search <query>               # semantic search
wiki query <question>             # RAG-style query with citations
wiki lint                         # hygiene checks
wiki models                       # list supported NIM models
wiki doctor                       # diagnose config + NIM
wiki status                       # show wiki stats
wiki install-skill                # install the bundled skill
wiki skill show [topic]           # print skill content
wiki skill list                   # list skill topics
wiki help                         # full command reference
```
