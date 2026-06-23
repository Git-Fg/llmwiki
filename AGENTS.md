# AGENTS.md — llmwiki-cli (the tool)

> Behavioral layer for AI agents working on the `llmwiki-cli` Rust CLI codebase at `/Users/felix/Documents/llmwiki/`.
> Structural truth lives in source files; design rationale lives in `docs/superpowers/specs/2026-06-21-karpathy-wiki-design.md` (original concept) and `docs/superpowers/specs/2026-06-22-llmwiki-installable-design.md` (v0.3.0 installable + LSP/MCP).

## Project Overview

`llmwiki-cli` is a single Rust binary CLI for managing a Karpathy-style LLM Wiki (markdown files + JSONL embeddings, no database). It ships with a bundled marketplace skill that auto-installs into the host agent's skill directory.

- Spec (concept): `docs/superpowers/specs/2026-06-21-karpathy-wiki-design.md`
- Spec (v0.3.0 installable): `docs/superpowers/specs/2026-06-22-llmwiki-installable-design.md`
- Plan: `docs/superpowers/plans/2026-06-22-llmwiki-installable.md`

## Build Commands

```bash
cargo build                          # debug build (also runs build.rs to generate SKILL.md stub)
cargo build --release                # release build
cargo test                           # all tests
cargo test --test <name>             # specific test file
cargo install --path .               # install to ~/.cargo/bin/llmwiki-cli
cargo clippy --all-targets           # lint (CI uses -D warnings)
cargo fmt --check                    # formatting check
```

## Module Layout

```text
src/
  main.rs                  # clap entrypoint
  cli/                     # one file per command (init, ingest, embed, search, query, lint, etc.)
  core/                    # domain logic (workspace, config, markdown, embeddings, chunker, nim, models_registry)
  lint/                    # deterministic hygiene checks
  skills/                  # skill content embedded via include_str!
  error.rs                 # WikiError thiserror enum
  lib.rs                   # module exports

build.rs                   # generates marketplace/skills/wiki/SKILL.md hub stub
                          # and marketplace/skills/wiki/SETUP/references/schema.json
```

## Coding Conventions

- Keep changes minimal and consistent with existing code.
- Prefer typed errors and explicit CLI contracts.
- Avoid production `unwrap()`; use `?` or return `WikiError`.
- Use `anyhow::anyhow!` only for simple contextual errors.
- CLI commands live in `src/cli/<name>.rs`.
- Domain logic lives in `src/core/` or `src/lint/`.
- Test CLI commands in `tests/<name>_test.rs`.

## Testing Strategy

- Unit tests: in `core/`, `lint/`, `markdown/`, etc.
- Integration tests: in `tests/`, one file per CLI command.
- E2E test: `tests/e2e_test.rs` runs the full pipeline and is ignored by default.
- Skill smoke test: `tests/skill_smoke.sh`.
- Mock NIM with `wiremock` for any test that hits the network.

## NIM API Conventions (do not change without updating the wiremock tests)

The CLI talks to an OpenAI-compatible endpoint hosted on NVIDIA NIM. Two invariants the code relies on — breaking either of these silently breaks every NIM call:

1. **`base_url` is the host only, with no path or version segment.** The default in `src/core/config.rs` is `https://integrate.api.nvidia.com` (no trailing `/v1`). Every NIM call site builds the full URL as `format!("{}/v1/<endpoint>", base_url.trim_end_matches('/'))`. If you see `/v1/v1/<endpoint>` in a request, `base_url` was set to a value that already includes `/v1` — strip it.

2. **The API key is resolved in this order:** the env var named by `nim.api_key_env` (default `NVIDIA_NIM_API_KEY`) first; then, if that is unset or empty, `NVIDIA_API_KEY` as a fallback. Use `resolve_api_key(&cfg.nim)` from `src/core/config.rs` — never call `std::env::var(&cfg.nim.api_key_env)` directly. `llmwiki-cli doctor` also honors the `WIKI_NIM_BASE_URL` env override; the other commands read it via the same config path.

The `tests/doctor_test.rs::doctor_uses_correct_models_endpoint` and the `tests/e2e_test.rs` wiremock tests lock both invariants — any new NIM call site that bypasses them will pass locally but break in production.

## Adding a New CLI Command

1. Create `src/cli/<name>.rs` with a public `Args` struct and `run(args) -> Result<(), WikiError>`.
2. Add the variant to `Command` in `src/cli/mod.rs`.
3. Wire dispatch in `cli::run()`.
4. Add integration tests in `tests/<name>_test.rs`.
5. Add help text through Clap doc comments and fields.
6. Update skill content if the command is user-facing.

## Importing an Existing Wiki

The canonical workspace layout uses a `wiki/` subdirectory containing markdown pages (`wiki/<page>.md`), a `raw/` directory for ingested sources (`raw/<category>/<source>.<ext>`), a `embeddings.jsonl` index, and a `.wiki/config.yaml`. If the source wiki uses a different layout (e.g. `concepts/`, `entities/`), the manual recipe is:

```bash
llmwiki-cli init /path/to/new-wiki
# Delete the init-template pages you don't want
rm /path/to/new-wiki/wiki/log.md /path/to/new-wiki/wiki/overview.md
cp -r /path/to/old-wiki/concepts/* /path/to/new-wiki/wiki/
llmwiki-cli lint --scope wiki --fix
```

`llmwiki-cli import` is intentionally not provided — automatic frontmatter inference and wikilink rewriting are speculative heuristics and a wrong inference corrupts the wiki. See `CHANGELOG.md` for the full decision.

## Removed

- The `web/` Svelte viewer and `wiki build-viewer` / `wiki serve` commands were removed from the project. The wiki content is consumed directly by the CLI and skill; no static site is generated.

## CLI Commands Reference

### `llmwiki-cli ls` — Granular workspace listing

```
llmwiki-cli ls [--pages] [--raw] [--embed] [--links] [--config] [--json]
```

- **No flags** → shows all sections (pages, raw, embed, links, config).
- **Specific flags** → shows only those sections.
- `--pages` — wiki pages with title, tags, outbound/inbound links, embedded status, chunks, line count.
- `--raw` — raw source files with type, SHA256, ingested date, bytes, frontmatter validity.
- `--embed` — embedded pages with chunk count and embedding dimension.
- `--links` — wikilink pairs (from → to).
- `--config` — resolved config key/value pairs.
- `--json` — machine-readable output (null fields omitted via `skip_serializing_if`).

### `llmwiki-cli tree` — Flat, grep-friendly page listing

```
llmwiki-cli tree [--json]
```

Outputs one line per page: `slug  title [tags] ✓(if embedded)`. Designed for piping to `grep`, `fzf`, etc.
With `--json`: structured array with `slug`, `path`, `title`, `tags`, `embedded`.
