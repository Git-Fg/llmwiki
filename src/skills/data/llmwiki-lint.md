---
name: llmwiki-lint
description: |
  Hygiene checks on wiki pages, raw sources, and the log. Routes to
  `llmwiki-cli lint`. Use when the user asks to check wiki health, find
  broken wikilinks, validate frontmatter, or audit tag taxonomy.
  Do NOT use for: editing pages (use search/query to find, then native
  file tools to fix).
allowed-tools: Bash(llmwiki-cli:*)
---

# llmwiki-lint

Run quality checks across the wiki — broken wikilinks, missing frontmatter,
unknown tags, stale references.

## Commands

```bash
llmwiki-cli lint                       # full wiki lint
llmwiki-cli lint --scope wiki          # only the wiki/ pages
llmwiki-cli lint --scope raw           # only the raw/ sources
llmwiki-cli lint --fix                 # auto-repair where safe
```

## Workflow

1. `llmwiki-cli lint` to surface all issues at once
2. Fix by editing the offending page (use search/query to find the right one)
3. Re-run `llmwiki-cli lint --fix` to clean up safe issues automatically

## Sibling skills

- `wiki-search` / `wiki-query` — locate the page that needs fixing
- `wiki-troubleshooting` — when lint flags something the agent doesn't recognize

## Full reference

```bash
llmwiki-cli lint --help
```