---
name: wiki-setup
description: |
  First-run install, init, registry, skill install. Routes to
  `llmwiki-cli doctor` and `llmwiki-cli install-skill`. Use when the
  user reports wiki errors, before any other wiki skill runs, or
  when `command -v llmwiki-cli` returns nothing.
  Do NOT use for: routine wiki work (the binary should already be installed).
allowed-tools: Bash(llmwiki-cli:*)
---

# wiki-setup

Verify the `llmwiki-cli` install and bootstrap it on the user's machine.

## Commands

```bash
llmwiki-cli doctor                  # diagnose install + config + NIM
llmwiki-cli install-skill --global  # install skill bundle to ~/.agents/skills/
llmwiki-cli init <path>             # scaffold a new wiki at <path>
```

## When this skill loads

- "wiki is broken" / "wiki not found" / "llmwiki-cli: command not found"
- First action in any new session before running ingest / search / query

## Sibling skills

- `wiki-troubleshooting` — when `doctor` reports an error
- `wiki-config` — when `doctor` succeeds but a config setting needs changing

## Full reference

```bash
llmwiki-cli doctor --help
llmwiki-cli install-skill --help
llmwiki-cli init --help
```