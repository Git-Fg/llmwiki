---
name: llmwiki-ingest
description: |
  Add a source file to the wiki. Routes to `llmwiki-cli ingest`. Use
  when the user asks to add a new source, ingest a file, or append
  to the wiki.
  Do NOT use for: reading existing pages (use search/query), editing pages.
allowed-tools: Bash(llmwiki-cli:*)
---

# llmwiki-ingest

Add a source file to `raw/` and append a log entry. Pair with `embed`
to make the new source searchable.

## Commands

```bash
llmwiki-cli ingest <file>           # add one source to raw/
llmwiki-cli ingest <dir>/*.md       # add a glob of files to raw/
llmwiki-cli embed                   # (separate command) build embeddings.jsonl
```

## Workflow

1. Place the source file in `raw/` (or pass a path to `ingest`)
2. `llmwiki-cli ingest <file>` — copies to `raw/<category>/` and appends to `log.md`
3. `llmwiki-cli embed` — rebuilds embeddings.jsonl so the new content is searchable
4. `llmwiki-cli build` — for raw sources that need LLM-driven compilation into wiki pages
5. `llmwiki-cli lint --scope raw` — confirm the raw/frontmatter contract holds

## Sibling skills

- `llmwiki-search` — find the freshly-ingested content
- `llmwiki-query` — RAG question-answering over the new content
- `llmwiki-lint` — check that the ingest produced valid pages

## Full reference

```bash
llmwiki-cli ingest --help
llmwiki-cli embed --help
```