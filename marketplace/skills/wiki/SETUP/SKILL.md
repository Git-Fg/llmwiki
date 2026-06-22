---
name: setup
description: |
  Verify the `llmwiki-cli` install and bootstrap it on the user's machine.
  Use when the user reports wiki errors, before any other wiki skill runs,
  or when `command -v llmwiki-cli` returns nothing.
  Do NOT use for: routine wiki work (the binary should already be installed).
whenToUse: |
  - "wiki is broken" / "wiki not found" / "llmwiki-cli: command not found"
  - First action in any new session before running ingest/search/query
argument-hint: "[--check]"
allowed-tools: Bash(llmwiki-cli:*), Bash(command -v:*), Bash(curl:*), Bash(cargo install:*)
license: Apache-2.0
---

# Wiki — Setup

## Decision Router

| User says | Action |
|---|---|
| "is wiki installed?" / "check wiki" | Run `--check` mode (steps 1 + 3 only) |
| "install wiki" / "set up wiki" / first-time setup | Run all 3 steps |
| wiki command returns "command not found" | Run all 3 steps |

## Reference Index

| Reference | Purpose |
|---|---|
| `references/schema.json` | Full JSON Schema for `wiki-root.toml` (auto-generated at build time) |

Detailed install options (cargo install, brew, source build) are added in Task 3.2.

## Before anything else

Run this block. If step 1 fails, run step 2, then re-run step 1.

```bash
# 1. Detect
command -v llmwiki-cli && llmwiki-cli --version

# 2. Install (if missing)
curl -LsSf https://github.com/<owner>/llmwiki/raw/main/install.sh | bash

# 3. Verify
llmwiki-cli doctor
```

If `llmwiki-cli doctor` reports an error, **STOP**. Do not proceed to
INGEST/SEARCH/etc. The subsequent commands will fail in opaque ways on an
unverified install.

## --check mode

If invoked with `--check`, run only steps 1 and 3 (skip the install):

```bash
command -v llmwiki-cli && llmwiki-cli --version
llmwiki-cli doctor
```

This is the safe mode for verifying an existing install without writing
anything.

## Edge cases

| Symptom | Action |
|---|---|
| `command -v llmwiki-cli` succeeds but `--version` fails | Corrupted install. Print reinstall steps. |
| `llmwiki-cli doctor` reports missing API key | Print `export NVIDIA_NIM_API_KEY=...` and pause. |
| `llmwiki-cli doctor` reports no NIM connectivity | Suggest `WIKI_NIM_BASE_URL=...` override. |
| `curl \| bash` blocked (corporate proxy, restricted machine) | Recommend only the curl method for now. Detailed install options are added in Task 3.2. |
| `command -v llmwiki-cli` succeeds, `--version` is correct, but doctor fails on NIM | Diagnose the NIM endpoint, not the install. Switch to TROUBLESHOOTING sub-skill. |

## Anti-patterns

❌ Do NOT run `install.sh` for the user. The user runs the curl command themselves. This keeps the security posture clear (the skill is content, not executable).

❌ Do NOT proceed to other wiki skills when `doctor` reports an error.

❌ Do NOT recommend `pip install`, `npm install`, or any package manager other than cargo/brew/install.sh. The CLI is a single Rust binary.

## CONTRAST

NOT for **using** the wiki → use `search/`, `query/`, `ingest/`, etc.
NOT for **fixing errors** that aren't install-related → use `troubleshooting/`.
NOT for **browsing installed wikis** → use `lsp/` or `mcp/` skills.

## When NOT to load

Do NOT load this skill when:
- The user is asking about wiki content (use `search/` or `query/`).
- The binary is installed and `doctor` passes (use the appropriate sub-skill).
- The user is asking about the LSP or MCP integration (use `lsp/` or `mcp/`).