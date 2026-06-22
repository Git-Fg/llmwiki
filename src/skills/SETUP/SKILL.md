---
name: setup
description: |
  Install the wiki CLI, create a wiki, register it in wiki-root.toml,
  set up the bundled skill, and verify the first-run setup. Use when
  the user asks about setup, first-run, llmwiki-cli init, llmwiki-cli config add,
  or installing the llmwiki-cli skill.
whenToUse: |
  Do NOT use for searching or querying an already-working wiki.
allowed-tools: Bash(llmwiki-cli:*)
---

# Wiki — Setup

## Filesystem layout

```
~/.agents/wiki-root.toml    # wiki registry + config (source of truth)
~/.agents/skills/wiki/      # installed skill bundle (hub + sub-skills)
~/my-wiki/
├── wiki/                    # compiled markdown (committed)
├── raw/                     # source materials (committed)
├── index.md                 # catalog (committed)
├── log.md                   # operational log (committed)
└── embeddings.jsonl         # NIM vectors (GITIGNORED)
```

## Install the CLI

```bash
cargo install --path /path/to/wiki
```

## First-run setup

1. **Initialize a wiki at a path** — `llmwiki-cli init` auto-registers in `wiki-root.toml`:
   ```bash
   llmwiki-cli init ~/my-wiki --alias mywiki --tag personal --tag reference
   ```
   Creates `wiki/`, `raw/articles/`, `index.md`, `log.md`, `.gitignore` and `git init`. **No `.wiki/` directory is created.**

2. **Register an existing wiki** (no `llmwiki-cli init`):
   ```bash
   llmwiki-cli config add <alias> <path> --tag tag1 --tag tag2 --description "Description"
   ```

3. **Verify the registry**:
   ```bash
   llmwiki-cli config list
   ```

4. **Find the active config**:
   ```bash
   llmwiki-cli config path
   ```

5. **Install the llmwiki-cli skill globally**:
   ```bash
   llmwiki-cli install-skill --global
   ```
   This creates `~/.agents/skills/wiki/` with the full skill bundle (hub + 8 sub-skills).

## Switching wikis

- By CWD: `cd ~/my-wiki && llmwiki-cli ls` (auto-detected)
- By flag: `llmwiki-cli --wiki pharma ls`
- By env: `WIKI_ACTIVE=pharma llmwiki-cli ls`

## Where the config lives

- `~/.agents/wiki-root.toml` — primary
- `~/.claude/wiki-root.toml` — fallback
- `~/wiki-root.toml` — last resort
- `$WIKI_ROOT_CONFIG` — env override (absolute path)

## Re-installing the skill

The installed skill is a copy, not a symlink. After upgrading the CLI, re-run:

```bash
llmwiki-cli install-skill --global
```

## Troubleshooting

- `wiki-root.toml not found` — `llmwiki-cli init` (creates one) or `llmwiki-cli config add`
- `alias not found` — `llmwiki-cli config list` to see registered wikis
- Old `.wiki/config.yaml` ignored — registry is the source of truth; safe to delete after migration
