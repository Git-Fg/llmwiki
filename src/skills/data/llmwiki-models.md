---
name: llmwiki-models
description: |
  Switch the embedding or reranking model. Routes to
  `llmwiki-cli models` and the `nim.*` config keys. Use when the user
  asks to use a different NVIDIA NIM model, switch embedding dimensions,
  or set a reranking model.
  Do NOT use for: installing the binary (wiki-setup), changing
  non-model config (wiki-config).
allowed-tools: Bash(llmwiki-cli:*)
---

# llmwiki-models

List whitelisted NVIDIA NIM models and switch the active embedding / reranking model.

## Commands

```bash
llmwiki-cli models                              # list all whitelisted models
llmwiki-cli config show-effective | grep nim    # see current embed / rerank model
llmwiki-cli config edit                         # change `nim.embed_model` etc.
llmwiki-cli config validate                     # catch bad model names
```

## Workflow

1. `llmwiki-cli models` to see what's available
2. `llmwiki-cli config show-effective | grep nim` to see what's currently active
3. `llmwiki-cli config edit` → set `nim.embed_model = "..."` (and rerank_model if used)
4. `llmwiki-cli config validate` to catch typos
5. `llmwiki-cli embed` to rebuild embeddings with the new model

## Sibling skills

- `wiki-config` — full config editing workflow
- `wiki-troubleshooting` — when `embed` fails after a model change

## Full reference

```bash
llmwiki-cli models --help
```