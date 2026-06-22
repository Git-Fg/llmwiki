---
name: troubleshooting
description: |
  Diagnose and fix common wiki issues — workspace not found, NIM
  unreachable, model not whitelisted, embeddings missing, broken
  wikilinks. Use when the user reports any wiki error.
whenToUse: |
  Do NOT use for routine search/query/ingest.
allowed-tools: Bash(wiki:*), Read
---

# Wiki — Troubleshooting

## "wiki-root.toml not found"

```bash
llmwiki-cli config path        # see where the CLI is looking
llmwiki-cli config list        # see all registered wikis
llmwiki-cli config add <alias> <path>   # register a wiki
llmwiki-cli config validate    # check that [defaults] and every alias parse + pass field validation
llmwiki-cli init <path> --alias <name>  # create a new wiki
```

## "alias not found"

```bash
llmwiki-cli config list        # see all registered aliases
wiki --wiki <alias> <cmd>  # use the right alias
```

## "workspace not found"

```bash
wiki --workspace /path/to/wiki <cmd>   # explicit path
WIKI_ACTIVE=<alias> wiki <cmd>         # env var
cd /path/to/wiki                        # CWD inside the wiki
```

## "API key not set"

```bash
echo $NVIDIA_NIM_API_KEY
export NVIDIA_NIM_API_KEY="nvapi-..."   # add to ~/.zshrc
```

## "NIM endpoint unreachable"

```bash
llmwiki-cli doctor             # detailed report
llmwiki-cli config get nim.base_url
WIKI_NIM_BASE_URL=https://integrate.api.nvidia.com llmwiki-cli doctor
```

## "wrong embedding model"

```bash
llmwiki-cli config get nim.embed_model --wiki <alias>
llmwiki-cli config set nim.embed_model nvidia/<model> --wiki <alias>
llmwiki-cli config show-schema    # JSON Schema for the full Config (for editor autocomplete)
```

## "no embeddings yet"

```bash
llmwiki-cli embed
llmwiki-cli config validate   # catches bad embed_model / chunk token sizes before NIM call
```

## "no wiki found" (no CWD match, no flag, no env)

```bash
llmwiki-cli config list        # see all wikis
wiki --wiki <alias> <cmd>
```

## "broken wikilink"

```bash
llmwiki-cli lint --scope wiki --strict
```

## Where the config lives

- `~/.agents/wiki-root.toml` — primary
- `~/.claude/wiki-root.toml` — fallback
- `~/wiki-root.toml` — last resort
- `$WIKI_ROOT_CONFIG` — env override
