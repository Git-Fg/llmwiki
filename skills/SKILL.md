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

**Always start with `llmwiki-cli --help`** — it shows the active wiki
alias, workspace, and which resolution step matched, so you know exactly
which wiki you're operating on. The same info is also printed when you
run `llmwiki-cli` with no subcommand at all (try it: `llmwiki-cli`).

**Install shell completion once per host** (zero-config TAB-completion
for every command, subcommand, and flag):

```bash
# bash
llmwiki-cli completion bash > ~/.local/share/bash-completion/completions/llmwiki-cli
# zsh
llmwiki-cli completion zsh > "${fpath[1]}/_llmwiki-cli"
# fish
llmwiki-cli completion fish > ~/.config/fish/completions/llmwiki-cli.fish
```

```bash
llmwiki-cli --help                # active wiki + full command reference
llmwiki-cli doctor                # diagnose install + config + NIM
llmwiki-cli config current        # alias + workspace + resolution source
llmwiki-cli skill list            # enumerate every sub-skill
llmwiki-cli skill get <topic>     # load one (version-matched with your binary)
llmwiki-cli <command> --help      # full flag reference
```

If the binary isn't installed: `cargo install llmwiki-cli --locked`
(or `curl -LsSf https://github.com/Git-Fg/llmwiki/releases/latest/download/install.sh | sh`).

Sub-skills are NOT installed to disk — they live inside the CLI binary,
served on demand. **Always prefer `skill get` over guessing commands.**

Multi-wiki quick reference:

- `llmwiki-cli config list` — every registered alias
- `llmwiki-cli status --all` — one-line health summary per wiki
- `llmwiki-cli use <alias>` — pin this workspace to a specific wiki
  (writes `<workspace>/.llmwiki-cli/state/active-wiki`, gitignored by
  `wiki init`)

When in doubt, run `llmwiki-cli doctor` first — catches missing API keys,
NIM connectivity, broken config, orphans in one pass. If you don't know
which wiki the CLI is operating on, run `llmwiki-cli config current`
(pinned to the active alias + resolution source + registry file path).

For the full Config schema: run `llmwiki-cli config show-schema`.