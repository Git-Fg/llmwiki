---
name: wiki-query
description: |
  RAG question-answering over the wiki. Routes to `llmwiki-cli query`.
  Use when the user asks "answer a question against my notes", "what
  does my wiki say about X", or wants a synthesized answer with sources.
  Do NOT use for: simple search (use wiki-search), browsing files directly.
allowed-tools: Bash(llmwiki-cli:*)
---

# wiki-query

Ask a RAG question over the wiki — retrieves relevant chunks and asks the
LLM to answer with citations.

## Commands

```bash
llmwiki-cli query "<question>"                       # answer + citations
llmwiki-cli query "<question>" --json                # machine-readable
llmwiki-cli --wiki <alias> query ...                 # query a different wiki
llmwiki-cli query "<q>" --llm-model <model>          # override the LLM
```

## Workflow

1. Make sure embeddings exist: `llmwiki-cli embed`
2. `llmwiki-cli query "<question>"` — answer is synthesized from retrieved chunks
3. Citations are included by default; strip with `--no-citations` if needed

## Sibling skills

- `wiki-search` — when the user just wants the relevant pages, not an answer
- `wiki-models` — when the user wants to switch the LLM or embedding model

## Full reference

```bash
llmwiki-cli query --help
```