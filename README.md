# wiki

A markdown-based personal knowledge base, built on Andrej Karpathy's LLM Wiki pattern. Single Rust binary, NVIDIA NIM embeddings, agent skill integration, tailnet-friendly sync.

## Install

```bash
git clone https://github.com/you/wiki.git ~/code/wiki
cd ~/code/wiki
cargo install --path .
```

This installs `wiki` to `~/.cargo/bin/wiki`. Add to your shell rc if not already on PATH.

## First-run setup

```bash
export NVIDIA_NIM_API_KEY="nvapi-..."        # Get one at https://build.nvidia.com/
wiki install-skill --global                 # install the agent skill
wiki init ~/my-wiki                         # create your wiki
cd ~/my-wiki
wiki doctor                                 # verify config + NIM reachability
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