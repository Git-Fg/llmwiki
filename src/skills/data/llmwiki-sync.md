---
name: llmwiki-sync
description: |
  New-device setup, cross-device git sync. Routes to `llmwiki-cli init`,
  `llmwiki-cli install-skill`, and `git` operations on the wiki repo.
  Use when the user asks to set up the wiki on a new machine, sync
  wikis across devices, or back up a wiki to git.
  Do NOT use for: routine wiki work (the setup should already be done).
allowed-tools: Bash(llmwiki-cli:*), Bash(git:*)
---

# llmwiki-sync

Get the wiki onto a new machine, or sync wikis between devices via git.

## Commands

```bash
llmwiki-cli init <path>              # scaffold a new wiki
llmwiki-cli install-skill --global   # install the skill bundle
llmwiki-cli config list              # see registered wikis
git clone <git-url> <path>           # pull an existing wiki from another machine
```

## Workflow (new device)

1. `llmwiki-cli doctor` — verify the install
2. `llmwiki-cli install-skill --global` — install the bundle
3. `git clone` the wiki repo from the canonical host
4. `llmwiki-cli doctor --wiki <alias>` — verify NIM + config

## Workflow (sync across devices)

1. Commit + push from the source machine: `git push`
2. Pull on the target: `git pull`
3. `llmwiki-cli embed` if pages changed (embeddings.jsonl is gitignored)

## Sibling skills

- `llmwiki-setup` — first-run install / bootstrap
- `llmwiki-troubleshooting` — when sync breaks (merge conflicts, missing aliases)

## Full reference

```bash
llmwiki-cli init --help
llmwiki-cli install-skill --help
```