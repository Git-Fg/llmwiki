# AGENTS.md — wiki (the tool)

> Behavioral layer for AI agents working on the `wiki` Rust CLI codebase at `/Users/felix/Documents/llmwiki/`.
> Structural truth lives in source files; design rationale lives in `docs/superpowers/specs/2026-06-21-karpathy-wiki-design.md`.

## Project Overview

`wiki` is a single Rust binary CLI for managing a Karpathy-style LLM Wiki (markdown files + JSONL embeddings, no database). It ships with a bundled skill.

- Spec: `docs/superpowers/specs/2026-06-21-karpathy-wiki-design.md`
- Plan: `docs/superpowers/plans/2026-06-21-karpathy-wiki.md`

## Build Commands

```bash
cargo build                          # debug build (also runs build.rs to generate SKILL.md stub)
cargo build --release                # release build
cargo test                           # all tests
cargo test --test <name>             # specific test file
cargo install --path .               # install to ~/.cargo/bin/wiki
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

build.rs                   # generates agents/skills/wiki/SKILL.md stub
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

## Adding a New CLI Command

1. Create `src/cli/<name>.rs` with a public `Args` struct and `run(args) -> Result<(), WikiError>`.
2. Add the variant to `Command` in `src/cli/mod.rs`.
3. Wire dispatch in `cli::run()`.
4. Add integration tests in `tests/<name>_test.rs`.
5. Add help text through Clap doc comments and fields.
6. Update skill content if the command is user-facing.

## Removed

- The `web/` Svelte viewer and `wiki build-viewer` / `wiki serve` commands were removed from the project. The wiki content is consumed directly by the CLI and skill; no static site is generated.
