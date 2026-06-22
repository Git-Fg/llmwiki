# Search

Use semantic search for conceptual lookups ("what did I write about X?"). Use the agent's native file tools (Grep, Read) for exact matches and known pages.

```bash
wiki search "sparse attention mechanisms"
wiki search "RAG pipelines" --top-k 20 --threshold 0.5
wiki search "..." --json          # machine-readable
```

Workflow:
1. Try `Read`/`Grep` first when you know the page name
2. Fall back to `wiki search` for fuzzy/conceptual matches
3. Open the top result with `Read`
4. If the answer is substantive, consider creating a new wiki page

Search returns chunks with file path and cosine similarity score. Use `--top-k 10` as a starting point.