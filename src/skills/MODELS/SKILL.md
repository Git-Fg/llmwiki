---
name: models
description: |
  Select and configure the NVIDIA NIM embedding and reranking models.
  Use when the user asks "switch embedding model", "use a different
  reranker", "what models are available", or "configure NIM".
whenToUse: |
  Do NOT use for embedding existing pages (use `wiki embed`).
allowed-tools: Bash(wiki:*)
---

# Wiki — Models

## List whitelisted models

```bash
wiki models --embed
wiki models --rerank
wiki models --commercial
```

## Set the embedding model

For all wikis (default):
```bash
wiki config set nim.embed_model nvidia/llama-nemotron-embed-1b-v2
```

For a specific wiki:
```bash
wiki config set nim.embed_model nvidia/nv-embedqa-e5-v5 --wiki pharma
```

This writes `[pharma.nim] embed_model = "..."` to `wiki-root.toml`.

## Verify

```bash
wiki doctor
```

## Supported whitelisted models

- `nvidia/nv-embed-v1` — general-purpose
- `nvidia/nv-embedqa-e5-v5` — QA-tuned
- `nvidia/nv-embedcode-7b-v1` — code-aware
- `nvidia/llama-nemotron-embed-1b-v2` — 1B param
- `nvidia/llama-nemotron-embed-vl-1b-v2` — multimodal
- `nvidia/llama-nemotron-rerank-1b-v2` — reranker
- `nvidia/llama-nemotron-rerank-vl-1b-v2` — multimodal reranker
- `nvidia/nv-rerankqa-mistral-4b-v3` — QA reranker

## API key resolution

The CLI reads the env var named by `nim.api_key_env` (default `NVIDIA_NIM_API_KEY`), falling back to `NVIDIA_API_KEY`.
