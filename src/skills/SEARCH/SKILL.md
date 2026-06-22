---
name: search
description: |
  Run semantic search over embedded wiki pages. Use when the user asks
  "find X in my wiki", "search the wiki", "what pages mention Y",
  or wants vector similarity results.
whenToUse: |
  Do NOT use for RAG-style question answering (use `query`).
allowed-tools: Bash(wiki:*)
---

# Wiki — Search

## Workflow

```bash
wiki search "pharmacology of beta-blockers" --top-k 10 --threshold 0.3
```

## Flags

- `--top-k N` — number of results (default 5)
- `--threshold 0.0-1.0` — minimum similarity (default 0.0)
- `--model <name>` — override the embedding model
- `--json` — machine-readable output
- `--wiki <alias>` — search a different wiki without `cd`

## Tips

- Embeddings must exist first: `wiki embed`
- Lower threshold = more results; raise to 0.5+ for high-precision
- Combine with `wiki tree | grep` for slug-based filtering before search

## Multiple wikis

`wiki --wiki pharma search "..."` searches the pharma wiki without `cd`-ing into it.
