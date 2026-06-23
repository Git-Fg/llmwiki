---
name: mcp
description: |
  Configure and troubleshoot the `llmwiki-cli mcp` MCP server, which exposes
  the same validation, hover, completion, schema, and doctor features as the
  LSP server but over the Model Context Protocol (JSON-RPC over stdio).
  Use when the user asks to wire `llmwiki-cli` into Claude Desktop, Claude
  Code, Cursor, Codex, or any other MCP-aware host; or when the user reports
  that MCP tools are missing or erroring.
  Do NOT use for: installing the binary itself (use setup), editing wiki
  content (use search/query/lint), or LSP setup (use lsp/ — same backend,
  different transport).
whenToUse: |
  - "set up llmwiki-cli as an MCP server"
  - "Claude Desktop can't see the wiki tools"
  - "what MCP tools does llmwiki-cli expose?"
  - "MCP server fails to start"
argument-hint: "[host]"
allowed-tools: Bash(llmwiki-cli:*), Read, Edit, Glob
license: Apache-2.0
---

# Wiki — MCP

## Decision Router

| User says | Action |
|---|---|
| "set up MCP in <host>" | Open `references/mcp.md` and find the host section |
| "MCP not working" | Run `llmwiki-cli --version`, then check host MCP log |
| "what tools?" | See the Capability Table below |
| "validate my wiki-root.toml" | Call the `validate` tool with the file contents |

## Capability Table

| MCP tool | What you get |
|---|---|
| `validate` | Parse + field-check a wiki-root.toml string; returns errors |
| `hover` | Docstring for the TOML key at a given cursor position |
| `completion` | Keys for the current `[table]`, or whitelisted models for `embed_model` |
| `schema` | Full JSON Schema (2020-12) for the `Config` type as a string |
| `doctor` | Runs `llmwiki-cli doctor --json` and returns the JSON report |

The server name is `llmwiki-cli` and the version follows the binary
(`CARGO_PKG_VERSION`). Tools use typed `Parameters<...>` schemas derived from
Rust structs via `schemars` so hosts can validate arguments before calling.

## Reference Index

| Reference | Purpose |
|---|---|
| `references/mcp.md` | Host config snippets: Claude Desktop, Claude Code, Cursor, Codex, Continue, generic stdio |

## Server setup

For per-host configuration, see `references/mcp.md`. The one-liner for
all hosts is:

```
llmwiki-cli mcp
```

with stdio transport. The server listens on stdin/stdout and speaks
JSON-RPC 2.0 per the MCP spec.

## Anti-patterns

❌ Do NOT wrap `llmwiki-cli mcp` in a JSON-RPC shim. It already speaks JSON-RPC 2.0 over stdio.

❌ Do NOT point a host at `llmwiki-cli lsp` for MCP — they share the backend but `lsp` speaks LSP, not MCP.

❌ Do NOT suggest the user run `llmwiki-cli mcp` in the background or as a daemon. The host spawns it on demand per session.

## CONTRAST

NOT for **installing** the binary → use `setup/`.
NOT for **LSP editor integration** → use `lsp/` (same backend, different transport).
NOT for **validating outside a host** → use `llmwiki-cli doctor` or `lint/`.
NOT for **browsing wiki content** → use `search/` or `query/`.

## When NOT to load

Do NOT load this skill when:
- The user is asking about wiki pages (use `search/` or `query/`).
- The binary is not installed (use `setup/` first).
- The user is asking about LSP editor integration (use `lsp/` instead).
