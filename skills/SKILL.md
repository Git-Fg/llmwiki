---
name: wiki
description: |
  Personal Karpathy-style LLM Wiki. Install the CLI
  (`cargo install llmwiki-cli --locked`), then `llmwiki-cli skill list`
  to discover sub-skills and `llmwiki-cli skill get <topic>` to load one
  on demand. Use when the user asks to ingest a source, search the wiki,
  answer a question with citations, lint or maintain the wiki, set up on
  a new device, or pick a different NVIDIA NIM model.
license: Apache-2.0
compatibility: |
  Requires the llmwiki-cli binary on PATH and network access to NVIDIA NIM
  (https://integrate.api.nvidia.com).
metadata:
  author: Git-Fg
  homepage: https://github.com/Git-Fg/llmwiki
  install-cli: cargo install llmwiki-cli --locked
  install-skill: npx skills add Git-Fg/llmwiki
allowed-tools: Bash(llmwiki-cli:*)
---

# wiki — Karpathy-style LLM Wiki

This is the entrypoint. The CLI is the source of truth.

```bash
llmwiki-cli skill list            # enumerate every sub-skill
llmwiki-cli skill get <topic>     # load one (version-matched with your binary)
llmwiki-cli <command> --help      # full flag reference
```

If the binary isn't installed: `cargo install llmwiki-cli --locked`
(or `curl -LsSf https://github.com/Git-Fg/llmwiki/releases/latest/download/install.sh | sh`).

Sub-skills are NOT installed to disk — they live inside the CLI binary,
served on demand. **Always prefer `skill get` over guessing commands.**

When in doubt, run `llmwiki-cli doctor` first — catches missing API keys,
NIM connectivity, broken config, orphans in one pass.

For the full Config schema: run `llmwiki-cli config show-schema`.