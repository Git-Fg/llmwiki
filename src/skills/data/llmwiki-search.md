---
name: llmwiki-search
description: |
  Find existing wiki content by semantic similarity over embedded chunks.
  Routes to `llmwiki-cli search`. Use when the user asks "find X in my
  wiki", "search the wiki", or "what pages mention Y".
  Do NOT use for: RAG question-answering (use llmwiki-query), browsing files
  directly (use native file tools).
allowed-tools: Bash(llmwiki-cli:*)
---

# llmwiki-search

Find existing content by semantic similarity over the embedded chunks.

## Commands

```bash
llmwiki-cli search "<query>"           # top 5 results from active wiki
llmwiki-cli search "<query>" --json    # machine-readable
llmwiki-cli --wiki <alias> search ...   # search a different wiki without cd
```

## Fleet fallback (v0.3.37+)

If no workspace can be resolved (no `--workspace`, `--wiki`,
`$WIKI_WORKSPACE`, `$WIKI_ACTIVE`, no CWD prefix match, no
`.llmwiki-cli/` walk-up, registry has >1 entry), `search` automatically
falls back to **fleet mode**: it embeds the query once, searches every
registered wiki that has `embeddings.jsonl`, and returns merged results
tagged with their source wiki alias. Wikis without embeddings are
skipped silently (test fixtures, empty wikis).

Fleet output example:

```text
✓ 12 result(s) for "type safety" (searched 4 wiki(s)):

  [minimax           0.892] comparisons/rust-enum-patterns.md
  [mywiki            0.871] concepts/type-safety-in-rust.md
  [mevin             0.834] guides/agent-device-design.md
```

Fleet JSON output adds `fleet: true`, `wikis_searched: [...]`, and a
`wiki` field on each result.

To force single-wiki mode (and respect an explicit `--workspace` /
`--wiki` that fails to resolve), pass the flag — the fallback only
fires when no explicit signal was given.

## Workflow

1. Make sure embeddings exist: `llmwiki-cli embed` (one-time per page change)
2. `llmwiki-cli search "<query>"` to surface relevant pages
3. Narrow with `--top-k` and `--threshold`
4. Combine with `llmwiki-cli tree | grep <slug>` for slug-based filtering before search

## Sibling skills

- `llmwiki-query` — when the user wants a synthesized answer with citations
- `llmwiki-ingest` — when the content is missing (needs to be added first)
- `llmwiki-troubleshooting` — when `search` returns empty after `embed`

## Full reference

```bash
llmwiki-cli search --help
```