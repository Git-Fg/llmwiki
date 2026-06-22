# wiki

A markdown-based personal knowledge base, built on Andrej Karpathy's LLM Wiki pattern. Single Rust binary, NVIDIA NIM embeddings, agent skill integration, tailnet-friendly sync.

## Install

```bash
# 1. Install the CLI
git clone https://github.com/you/wiki.git ~/code/wiki
cd ~/code/wiki
cargo install --path .

# 2. Install the agent skill
wiki install-skill --global
```

This installs `wiki` to `~/.cargo/bin/wiki` and the skill to `~/.agents/skills/wiki/`. Add `~/.cargo/bin` to your shell PATH if not already there.

### Alternative: Workspace-local skill

```bash
wiki install-skill  # installs to .agents/skills/wiki/ in current workspace
```

Use `--global` for system-wide access, omit for workspace-local.

## First-run setup

```bash
export NVIDIA_NIM_API_KEY="nvapi-..."        # Get one at https://build.nvidia.com/
wiki init ~/my-wiki                         # create your wiki
cd ~/my-wiki
wiki doctor                                 # verify config + NIM reachability
```

## Quick Help

```bash
wiki help              # full command reference
wiki skill show        # complete agent skill guide
wiki skill list        # list all skill topics
```

## Daily use

```bash
cd ~/my-wiki
git pull                                    # get latest markdown from other devices
wiki ingest ~/Downloads/article.pdf         # add a source (auto-compiles to wiki pages)
wiki search "what did I write about X?"     # semantic search
wiki query "summarize the latest AI notes"  # RAG with citations
wiki lint                                   # hygiene check
git add . && git commit -m "ingest: X" && git push
```

## Architecture

- **Wiki content**: Markdown files in `wiki/`, sources in `raw/`, catalog in `index.md`, log in `log.md`. All committed to git.
- **Embeddings**: `embeddings.jsonl` (gitignored, regenerated per device via `wiki embed`).
- **CLI**: Single Rust binary. No database.
- **Skill**: stub at `~/.agents/skills/wiki/SKILL.md` (symlinked via `wiki install-skill`). Full content served by `wiki skill show [topic]`.
- **Sync**: Git between devices. Embeddings regenerated locally.
- **No viewer**: The wiki is consumed directly via the CLI; no static site is generated.

## Commands

```
wiki init [path]                scaffold a new wiki
wiki ingest <source>           add raw source + log entry
wiki build                     list pending raw sources
wiki embed                     compute embeddings (or --skip-existing)
wiki search <query>            semantic search
wiki query <question>          RAG-style query with citations
wiki lint                      hygiene checks
wiki models                    list supported NIM models
wiki doctor                    diagnose config + NIM
wiki status                    show wiki stats
wiki install-skill             install the bundled skill
wiki skill show [topic]        print skill content
```

Run `wiki --help` for the full list.

## Embedding models

Default: `nvidia/nv-embed-v1` (4096 dims, non-commercial). Other supported models:

- `nvidia/nv-embedqa-e5-v5` (1024 dims, commercial)
- `nvidia/nv-embedcode-7b-v1` (4096 dims, commercial)
- `nvidia/llama-nemotron-embed-1b-v2` (2048 dims, Matryoshka, commercial)
- `nvidia/llama-nemotron-embed-vl-1b-v2` (multimodal)
- Plus 3 reranker models

Run `wiki models` for full specs. Change via `nim.embed_model` in `.wiki/config.yaml`.

## Documentation

- Design spec: `docs/superpowers/specs/2026-06-21-karpathy-wiki-design.md`
- Implementation plan: `docs/superpowers/plans/2026-06-21-karpathy-wiki.md`
- Agent behavioral layer: `AGENTS.md`

## License

Apache 2.0.