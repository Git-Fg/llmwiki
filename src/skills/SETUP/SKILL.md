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

## JSON Schema (for editor autocomplete)

The full JSON Schema for `wiki-root.toml` is regenerated at build time. Editors with YAML Schema support (VS Code's Red Hat YAML extension, IntelliJ, Neovim with `coc-yaml`, etc.) can point their schema association at this block. AI agents reading the skill will see the canonical schema as part of the documentation.

<!-- BEGIN SCHEMA -->

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "Config",
  "type": "object",
  "required": [
    "config_version",
    "nim",
    "wiki"
  ],
  "properties": {
    "config_version": {
      "description": "Schema version of this config",
      "type": "integer",
      "format": "uint32",
      "minimum": 0.0
    },
    "nim": {
      "description": "NIM API client configuration",
      "allOf": [
        {
          "$ref": "#/definitions/NimConfig"
        }
      ]
    },
    "wiki": {
      "description": "Wiki page chunking and lint settings",
      "allOf": [
        {
          "$ref": "#/definitions/WikiConfig"
        }
      ]
    }
  },
  "definitions": {
    "NimConfig": {
      "type": "object",
      "required": [
        "api_key_env",
        "base_url",
        "batch_size",
        "embed_model",
        "request_timeout_secs",
        "rerank_model",
        "retry"
      ],
      "properties": {
        "api_key_env": {
          "description": "Env var name holding the NIM API key",
          "type": "string"
        },
        "base_url": {
          "description": "NIM API base URL (no /v1 suffix)",
          "type": "string"
        },
        "batch_size": {
          "description": "Embedding request batch size (1+)",
          "type": "integer",
          "format": "uint",
          "minimum": 0.0
        },
        "embed_dim_override": {
          "description": "Override embedding dimension (empty = use model default)",
          "type": [
            "integer",
            "null"
          ],
          "format": "uint",
          "minimum": 0.0
        },
        "embed_model": {
          "description": "Embedding model identifier (must be in the whitelisted NVIDIA NIM set)",
          "type": "string"
        },
        "request_timeout_secs": {
          "description": "NIM request timeout in seconds",
          "type": "integer",
          "format": "uint64",
          "minimum": 0.0
        },
        "rerank_model": {
          "description": "Re-ranking model identifier (empty = disabled)",
          "type": "string"
        },
        "retry": {
          "description": "Retry policy for failed NIM calls",
          "allOf": [
            {
              "$ref": "#/definitions/RetryConfig"
            }
          ]
        }
      }
    },
    "RetryConfig": {
      "type": "object",
      "required": [
        "backoff_ms",
        "max_attempts"
      ],
      "properties": {
        "backoff_ms": {
          "description": "Backoff between retries in milliseconds",
          "type": "integer",
          "format": "uint64",
          "minimum": 0.0
        },
        "max_attempts": {
          "description": "Maximum attempts per NIM call",
          "type": "integer",
          "format": "uint32",
          "minimum": 0.0
        }
      }
    },
    "WikiConfig": {
      "type": "object",
      "required": [
        "chunk_overlap_tokens",
        "default_chunk_tokens",
        "min_chunk_tokens",
        "require_frontmatter",
        "require_wikilinks_min"
      ],
      "properties": {
        "chunk_overlap_tokens": {
          "description": "Chunk overlap in tokens (must be < default_chunk_tokens)",
          "type": "integer",
          "format": "uint",
          "minimum": 0.0
        },
        "default_chunk_tokens": {
          "description": "Default chunk size in tokens",
          "type": "integer",
          "format": "uint",
          "minimum": 0.0
        },
        "min_chunk_tokens": {
          "description": "Minimum chunk size in tokens (must be <= default_chunk_tokens)",
          "type": "integer",
          "format": "uint",
          "minimum": 0.0
        },
        "require_frontmatter": {
          "description": "Require YAML frontmatter on every page",
          "type": "boolean"
        },
        "require_wikilinks_min": {
          "description": "Minimum wikilink count per page (0 = no minimum)",
          "type": "integer",
          "format": "uint",
          "minimum": 0.0
        }
      }
    }
  }
}
```
<!-- END SCHEMA -->
