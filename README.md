# wiki

A markdown-based personal knowledge base, built on Andrej Karpathy's LLM Wiki pattern. Single Rust binary, NVIDIA NIM embeddings, agent skill integration, tailnet-friendly sync.

## Install

```bash
# 1. Install the CLI
curl -LsSf https://github.com/<owner>/llmwiki/raw/main/install.sh | bash

# 2. Install the agent skill
llmwiki-cli install-skill --global
```

This installs `llmwiki-cli` to `~/.cargo/bin/llmwiki-cli` and the skill to `~/.agents/skills/wiki/`. Add `~/.cargo/bin` to your shell PATH if not already there.

### Alternative: Workspace-local skill

```bash
llmwiki-cli install-skill  # installs to .agents/skills/wiki/ in current workspace
```

Use `--global` for system-wide access, omit for workspace-local.

## First-run setup

```bash
export NVIDIA_NIM_API_KEY="nvapi-..."        # Get one at https://build.nvidia.com/
llmwiki-cli init ~/my-wiki                         # create your wiki
cd ~/my-wiki
llmwiki-cli doctor                                 # verify config + NIM reachability
```

## Quick Help

```bash
llmwiki-cli help              # full command reference
llmwiki-cli skill show        # complete agent skill guide
llmwiki-cli skill list        # list all skill topics
```

## Daily use

```bash
cd ~/my-wiki
git pull                                    # get latest markdown from other devices
llmwiki-cli ingest ~/Downloads/article.pdf         # add a source (auto-compiles to wiki pages)
llmwiki-cli search "what did I write about X?"     # semantic search
llmwiki-cli query "summarize the latest AI notes"  # RAG with citations
llmwiki-cli lint                                   # hygiene check
git add . && git commit -m "ingest: X" && git push
```

## Architecture

- **Wiki content**: Markdown files in `wiki/`, sources in `raw/`, catalog in `index.md`, log in `log.md`. All committed to git.
- **Embeddings**: `embeddings.jsonl` (gitignored, regenerated per device via `llmwiki-cli embed`).
- **CLI**: Single Rust binary. No database.
- **Skill**: stub at `~/.agents/skills/wiki/SKILL.md` (symlinked via `llmwiki-cli install-skill`). Full content served by `llmwiki-cli skill show [topic]`.
- **Sync**: Git between devices. Embeddings regenerated locally.
- **No viewer**: The wiki is consumed directly via the CLI; no static site is generated.

## Commands

```
llmwiki-cli init [path]                scaffold a new wiki
llmwiki-cli ingest <source>           add raw source + log entry
llmwiki-cli build                     list pending raw sources
llmwiki-cli embed                     compute embeddings (or --skip-existing)
llmwiki-cli search <query>            semantic search
llmwiki-cli query <question>          RAG-style query with citations
llmwiki-cli lint                      hygiene checks
llmwiki-cli models                    list supported NIM models
llmwiki-cli doctor                    diagnose config + NIM
llmwiki-cli status                    show wiki stats
llmwiki-cli install-skill             install the bundled skill
llmwiki-cli skill show [topic]        print skill content
```

Run `llmwiki-cli --help` for the full list.

## Embedding models

Default: `nvidia/nv-embed-v1` (4096 dims, non-commercial). Other supported models:

- `nvidia/nv-embedqa-e5-v5` (1024 dims, commercial)
- `nvidia/nv-embedcode-7b-v1` (4096 dims, commercial)
- `nvidia/llama-nemotron-embed-1b-v2` (2048 dims, Matryoshka, commercial)
- `nvidia/llama-nemotron-embed-vl-1b-v2` (multimodal)
- Plus 3 reranker models

Run `llmwiki-cli models` for full specs. Change via `nim.embed_model` in `.wiki/config.yaml`.

## Documentation

- Design spec: `docs/superpowers/specs/2026-06-21-karpathy-wiki-design.md`
- Implementation plan: `docs/superpowers/plans/2026-06-21-karpathy-wiki.md`
- Agent behavioral layer: `AGENTS.md`

## License

Apache 2.0.