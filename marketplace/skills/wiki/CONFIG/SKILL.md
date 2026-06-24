---
name: config
description: |
  Manage wiki configuration: edit wiki-root.toml or per-workspace
  config.toml, validate changes, diagnose config issues. Use when the
  user asks to change a wiki's pages_dir, exclude_dirs, NIM model,
  or any other setting; or when a config typo causes errors.
  Do NOT use for: installing the binary itself (use setup), editing
  wiki content (use search/query/lint).
whenToUse: |
  - "add a wiki alias"
  - "change the embed model"
  - "validate wiki-root.toml"
  - "why is my config not taking effect?"
argument-hint: "[action]"
allowed-tools: Bash(llmwiki-cli:*), Read, Edit, Glob
license: Apache-2.0
---

# Wiki — CONFIG

## Decision Router

| User says | Action |
|-----------|--------|
| "edit wiki-root.toml" | 1. `wiki config show-effective`, 2. edit file, 3. `wiki config validate` |
| "add a wiki alias" | `wiki config add <alias> <path>` (interactive) or edit TOML directly |
| "change embed model" | `wiki config set <alias> nim.embed_model <model>` or edit TOML |
| "what does my config resolve to?" | `wiki config show-effective` |
| "validate my config" | `wiki config validate` |
| "diagnose a config problem" | `wiki doctor` then `wiki config validate` |
| "what keys are available?" | `wiki config show-schema` (or `--section wiki|nim` for scoped output) |

## Capability Table

| Command | What you get |
|---------|--------------|
| `wiki config list` | All registered aliases + defaults |
| `wiki config paths` | Search order for config files |
| `wiki config show-effective` | Resolved config with `<source>` attribution |
| `wiki config show-schema` | JSON Schema for the Config type; `--section` filters to wiki or nim |
| `wiki config validate` | Structural + unknown-key + field-level checks |
| `wiki config get <key>` | Single value (dotted key path) |
| `wiki config set <alias> <key> <value>` | Atomic write |
| `wiki config add <alias> <path>` | New alias |
| `wiki config rm <alias>` | Remove alias |
| `wiki config unset <alias> <key>` | Remove a key |
| `wiki config path` | Resolved config file path |
| `wiki config edit` | Open config in `$EDITOR` |

## The edit-config workflow

For AI agents editing wiki-root.toml or per-workspace config.toml:

1. **Read current state**: `wiki config show-effective`
2. **Discover available keys**: `wiki config show-schema` (filter with `--section wiki|nim`)
3. **Edit the file** (use Edit tool, preserve formatting)
4. **Validate after every change**: `wiki config validate`
5. **Diagnose if something feels wrong**: `wiki doctor`

Never skip step 4 — `wiki config validate` catches:

- Unknown keys (typos like `pages_dirr`)
- Invalid TOML syntax
- Invalid model names
- Bad chunk-size combinations
- Other field-level violations

## Anti-patterns

- Do NOT edit `wiki-root.toml` and skip `wiki config validate`.
- Do NOT guess TOML key names from structure — run `wiki config show-schema`.
- Do NOT spawn the binary as a long-running server — every command is one-shot.
- Do NOT add a JSON-RPC wrapper around `wiki config validate`.

## CONTRAST

Use this skill for config editing. Use SETUP for first-run install, SEARCH/QUERY for content, LINT for hygiene, TROUBLESHOOTING for runtime errors.
