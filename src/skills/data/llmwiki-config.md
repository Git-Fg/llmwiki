---
name: wiki-config
description: |
  Edit wiki-root.toml or per-workspace config.toml. Routes to
  `llmwiki-cli config {show-effective, show-schema, validate, edit}`.
  Use when the user asks to change a wiki's pages_dir, exclude_dirs,
  NIM model, or any other setting; or when a config typo causes errors.
  Do NOT use for: installing the binary (wiki-setup), editing wiki content.
allowed-tools: Bash(llmwiki-cli:*)
---

# wiki-config

Manage the wiki config — see what's resolved, discover available keys,
edit safely, validate every change.

## Commands

```bash
llmwiki-cli config current                    # active wiki alias + resolution source
llmwiki-cli config show-effective              # resolved config with source attribution
llmwiki-cli config show-schema [--section wiki|nim]   # JSON Schema; filter by section
llmwiki-cli config edit                        # open wiki-root.toml in $EDITOR
llmwiki-cli config validate                    # catches typos, bad TOML, bad model names
```

## Workflow

1. `llmwiki-cli config show-effective` — see what's resolved
2. `llmwiki-cli config show-schema --section wiki|nim` — discover available keys
3. Modify the config file (preserve formatting)
4. `llmwiki-cli config validate` — after every change
5. `llmwiki-cli doctor` — full diagnostic if something still feels off

## Sibling skills

- `wiki-setup` — when `doctor` fails before config validation
- `wiki-troubleshooting` — when validation passes but behavior is wrong

## Full reference

```bash
llmwiki-cli config --help
```