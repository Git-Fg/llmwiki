---
name: lint
description: |
  Run quality checks over wiki pages, raw sources, and the log. Use
  when the user asks "check the wiki for issues", "lint the wiki",
  "find broken links", or "validate frontmatter".
whenToUse: |
  Do NOT use for searching or ingesting.
allowed-tools: Bash(wiki:*), Read
---

# Wiki — Lint

## Workflow

```bash
wiki lint --scope wiki --strict
```

## Scopes

- `wiki` — check compiled pages (default)
- `raw` — check raw sources
- `log` — check operational log
- `all` — check everything

## Flags

- `--strict` — exit non-zero on warnings
- `--json` — machine-readable output
- `--wiki <alias>` — lint a different wiki

## Common checks

- Frontmatter required on all pages
- Minimum wikilinks per page (default 2)
- Chunk tokens within bounds
- Broken wikilinks
- Embeddings coverage

## Configuration

Adjust thresholds in `wiki-root.toml`:

```bash
wiki config set wiki.require_wikilinks_min 3
```

This writes `[defaults].wiki.require_wikilinks_min = 3` (or `[<alias>].wiki...` with `--wiki <alias>`).
