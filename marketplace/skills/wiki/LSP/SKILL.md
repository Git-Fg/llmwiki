---
name: lsp
description: |
  Configure and troubleshoot the `llmwiki-cli lsp` LSP server, which provides
  hover, completion, document-symbol, and diagnostic features for `wiki-root.toml`.
  Use when the user asks to set up the LSP in their editor, asks why their
  editor isn't recognizing `wiki-root.toml`, or asks about supported LSP features.
  Do NOT use for: installing the binary itself (use setup), editing wiki
  content (use search/query/lint).
whenToUse: |
  - "set up the LSP in Helix/Neovim/Zed"
  - "the LSP isn't recognizing wiki-root.toml"
  - "what features does the llmwiki-cli LSP support?"
argument-hint: "[editor]"
allowed-tools: Bash(llmwiki-cli:*), Read, Edit, Glob
license: Apache-2.0
---

# Wiki — LSP

## Decision Router

| User says | Action |
|---|---|
| "set up LSP in <editor>" | Open `references/lsp.md` and find the editor section |
| "LSP not working" | Run `llmwiki-cli --version` then check editor LSP log |
| "what features?" | See the Capability Table below |
| "validate my wiki-root.toml" | Open the file in the editor; LSP publishes diagnostics on save |

## Capability Table

| LSP method | What you get |
|---|---|
| `textDocument/hover` | Docstring for the TOML key under cursor |
| `textDocument/completion` | Keys for the current `[table]`, or whitelisted models for `embed_model` |
| `textDocument/documentSymbol` | Top-level tables (`[nim]`, `[wiki]`) as Namespace outline |
| `textDocument/publishDiagnostics` | Validation errors as you type (`unsupported embed_model`, TOML parse errors, etc.) |

## Reference Index

| Reference | Purpose |
|---|---|
| `references/lsp.md` | Editor config snippets: Helix, Neovim, Zed, VS Code |

## Editor setup

For per-editor configuration, see `references/lsp.md`. The one-liner for
all editors is:

```
llmwiki-cli lsp
```

with stdio transport. The server listens on stdin/stdout.

## Anti-patterns

❌ Do NOT add a JSON-RPC wrapper around `llmwiki-cli lsp`. It already speaks LSP over stdio.

❌ Do NOT suggest the user use `llmwiki-cli` for completion inside `wiki-root.toml` without LSP — that's exactly what the LSP provides.

## CONTRAST

NOT for **installing** the binary → use `setup/`.
NOT for **validating outside the editor** → use `lint/` or `llmwiki-cli doctor`.
NOT for **browsing wiki content** → use `search/` or `query/`.

## When NOT to load

Do NOT load this skill when:
- The user is asking about wiki pages (use `search/` or `query/`).
- The binary is not installed (use `setup/` first).
- The user is asking about MCP integration (use `mcp/`).