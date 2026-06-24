---
name: wiki-search
description: |
  Find existing wiki content by semantic similarity over embedded chunks.
  Routes to `llmwiki-cli search`. Use when the user asks "find X in my
  wiki", "search the wiki", or "what pages mention Y".
  Do NOT use for: RAG question-answering (use wiki-query), browsing files
  directly (use native file tools).
allowed-tools: Bash(llmwiki-cli:*)
---

# wiki-search

Find existing content by semantic similarity over the embedded chunks.

## Commands

```bash
llmwiki-cli search "<query>"           # top 5 results
llmwiki-cli search "<query>" --json    # machine-readable
llmwiki-cli --wiki <alias> search ...   # search a different wiki without cd
```

## Workflow

1. Make sure embeddings exist: `llmwiki-cli embed` (one-time per page change)
2. `llmwiki-cli search "<query>"` to surface relevant pages
3. Narrow with `--top-k` and `--threshold`
4. Combine with `llmwiki-cli tree | grep <slug>` for slug-based filtering before search

## Sibling skills

- `wiki-query` — when the user wants a synthesized answer with citations
- `wiki-ingest` — when the content is missing (needs to be added first)
- `wiki-troubleshooting` — when `search` returns empty after `embed`

## Full reference

```bash
llmwiki-cli search --help
```