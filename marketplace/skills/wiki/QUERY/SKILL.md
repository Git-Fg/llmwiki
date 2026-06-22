---
name: query
description: |
  Ask a RAG question over the wiki — retrieves relevant chunks and asks
  the LLM to answer with citations. Use when the user asks "answer a
  question against my notes", "what does my wiki say about X", or
  wants a synthesized answer with sources.
whenToUse: |
  Do NOT use for simple search (use `search`).
allowed-tools: Bash(llmwiki-cli:*)
---

# Wiki — Query

## Workflow

```bash
llmwiki-cli query "What are the contraindications for beta-blockers in asthma?" \
  --top-k 5 --llm-model nvidia/llama-3.3-nemotron-super-49b-v1
```

## Flags

- `--top-k N` — number of retrieved chunks (default 5)
- `--model <embed-model>` — override embedding model
- `--llm-model <model>` — override the LLM that synthesizes the answer
- `--no-citations` — strip the citation footer
- `--json` — machine-readable output
- `--wiki <alias>` — query a different wiki

## Notes

- Requires embeddings (`llmwiki-cli embed`) and a working NIM endpoint
- The CLI handles chunk retrieval + LLM call automatically
- The answer always cites the source pages
