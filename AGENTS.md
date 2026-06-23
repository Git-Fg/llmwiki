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

build.rs                   # generates agents/skills/wiki/SKILL.md hub stub,
                          # marketplace/skills/wiki/SETUP/references/schema.json
                          # (from Config), and
                          # marketplace/skills/wiki/MCP/references/doctor.schema.json
                          # (from a build.rs-local DoctorReport duplicate annotated
                          # with #[derive(JsonSchema)] + #[serde(deny_unknown_fields)]).
                          # See "JSON Schema generation" below.
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
- E2E test: `tests/e2e_test.rs` exercises the full pipeline (init → embed → search → status → lint) against a wiremock-stubbed NIM endpoint. Runs in CI via `cargo test` (no `#[ignore]`).
- Skill smoke test: `tests/skill_smoke.sh`.
- Mock NIM with `wiremock` for any test that hits the network.

**Test isolation with env vars / CWD**: tests that mutate `$HOME`, `$USERPROFILE`, `$WIKI_ROOT_CONFIG`, or CWD MUST go through the helpers in `tests/common/mod.rs`: `with_lock` (serializes across all test binaries), `with_home_and_cwd`, `with_wiki_root_config`, `without_wiki_root_config`, `isolated_tempdir`. The `with_home_and_cwd` helper uses an `EnvGuard` RAII struct (post-v0.3.16) so the captured env state is restored on Drop, including during unwinding from a panic. **Do not write a fresh env-modifying helper** without the same Drop-guard pattern — a panic in the inner closure would otherwise leak `$HOME`/`CWD` into every later test in the same binary, producing flaky NotFound failures that are very hard to reproduce.

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

## Workspace Resolution

The CLI locates the active wiki (or the registry of wikis) in this order:

1. `--workspace <path>` flag (hard override)
2. `--wiki <alias>` flag (looks up alias in the registry)
3. `$WIKI_WORKSPACE` env var
4. `$WIKI_ACTIVE` env var (looks up alias in the registry)
5. Registry CWD prefix match against registered wiki paths
6. Walk up from CWD looking for `.llmwiki-cli/` directory (skip HOME so `~/.llmwiki-cli/` is treated as user-global config, not a workspace marker)
7. Single-wiki shortcut (registry has exactly one entry) — defaults to it without requiring `--wiki`

Registry file lookup (used by `--wiki`, `$WIKI_ACTIVE`, and the CWD prefix match) — **all sources are concatenated, with later (higher-priority) entries winning on alias conflict**:

1. `$WIKI_ROOT_CONFIG` — exact path, no merging, no fallback
2. User-global chain (lowest priority, loaded first):
   - `~/wiki-root.toml`
   - `~/.claude/wiki-root.toml`
   - `~/.agents/wiki-root.toml`
3. Project-local chain (ancestor walk-up from CWD, loaded furthest-to-closest so closest wins on conflict):
   - `<closest-ancestor>/.agents/wiki-root.toml`
   - ... up to `<farthest-ancestor>/.agents/wiki-root.toml`

Duplicate canonical paths (e.g. home that is also an ancestor of CWD) are deduplicated before merging. On alias conflict, **the alias table is deep-merged recursively** — top-level scalar keys (`path`, `description`, `qmd_slug`, etc.) follow scalar-override semantics (higher wins per key), arrays (`tags`, `what_to_read`) follow union-dedupe semantics (lower file's items are preserved, higher file's items are appended if not already present), and nested TOML tables like `[alias.nim]` are merged key-by-key so a project-local override of `description` does NOT drop lower-priority `[alias.nim]` sub-sections like `embed_model` or `base_url`. The `[defaults]` table follows the same deep-merge rule.

**Every wiki alias from every source is visible** to all commands (CLI, LSP, MCP). This lets teams register shared knowledge in `~/.agents/wiki-root.toml` while individual projects add their own scoped wikis via project-local `.agents/wiki-root.toml` — no duplication, no precedence guessing. Convention mirrors git (local + global), hk (per-project + per-user), and Atmos (CWD + parent search + git root).

When no registry is found, `WikiRootNotFound` error distinguishes `$WIKI_ROOT_CONFIG` states (empty string / directory / missing / non-regular file) and lists every searched path so users can fix the config without guessing.

**Active write scope**: `Registry::save()` writes only to the highest-priority file (`root_path`). Mutations via `config set/add/rm/unset` that target an alias loaded from a lower-priority file follow the git-config convention: `set` and `add` create a fresh override section in the active (highest-priority) file (correct — the override is what the user wants); `rm` and `unset` **error** instead of silently no-op'ing, because the alias/key isn't in `raw_doc` and a delete would have no effect. The error message points the user at `WIKI_ROOT_CONFIG` to retarget the active scope at the file that owns the alias.

## Config File Resolution

Config files are searched in this order (highest priority first; later files deep-merge on top):

1. `$LLMWIKI_CONFIG` env var — exact file path, no merging
2. `<workspace>/.llmwiki-cli/config.toml` — per-workspace, found by walking up from the resolved workspace looking for the closest `.llmwiki-cli/` ancestor (HOME is skipped so `~/.llmwiki-cli/` is not mistaken for a workspace). Git-committable so a team can share settings per-wiki.
3. `~/.llmwiki-cli/config.toml` — per-computer, hidden dotfile directory
4. Built-in `Config::default()` — applied when no files exist

When the registry has a matching `[alias]` entry for the workspace, `Registry::resolve_config()` deep-merges `[defaults]` + `[alias]` first, then deep-merges `<workspace>/.llmwiki-cli/config.toml` on top. Per-workspace config wins per-key over registry entries; per-computer config is folded into `Config::default()` upstream.

**Both code paths use the same TOML-level deep merge** (`registry::deep_merge_into`, post-v0.3.15). The non-registry path (`load_config_unvalidated`) was refactored in v0.3.15 from a brittle per-field `Config::merge()` (which silently dropped every `wiki.*` and most `nim.*` override) to the same `deep_merge_into` the registry path uses. Every field with `#[serde(default)]` is now handled uniformly — no per-field enumeration to forget. Do not reintroduce a per-field merge helper; if a new config field is added, just annotate it with `#[serde(default)]` and the deep merge picks it up automatically.

TOML only — matches `wiki-root.toml` format. YAML and legacy `~/.config/wiki/config.yaml` paths were removed in v0.3.6; `.wiki/` walk-up and the `~/llmwiki-cli/.wiki/` workspace fallback were removed in v0.3.7. The project is still alpha, so no backward compatibility shims. To customize:

```bash
export LLMWIKI_CONFIG=/etc/llmwiki-cli.toml   # full override
# or edit ~/.llmwiki-cli/config.toml directly
# or commit a per-workspace override to <workspace>/.llmwiki-cli/config.toml
```

## JSON Schema generation (v0.3.22+)

Two JSON Schema documents ship inside the skill bundle, both **auto-generated by `build.rs`** from a Rust struct (no hand-edited JSON):

1. `marketplace/skills/wiki/SETUP/references/schema.json` — for the `Config` type. The real struct lives in `src/core/config.rs`; `build.rs` has its own annotated duplicate with `#[derive(schemars::JsonSchema)]` + `#[allow(dead_code)]`. `cargo build` regenerates the JSON on every change.
2. `marketplace/skills/wiki/MCP/references/doctor.schema.json` — for the `DoctorReport` type. Same pattern: the real struct lives in `src/cli/doctor.rs`; `build.rs` has its own annotated duplicate.

**Schema root contract — `additionalProperties: false`**: Only the `DoctorReport` duplicate carries `#[serde(deny_unknown_fields)]` (v0.3.22). Schemars reads the serde attribute and emits `"additionalProperties": false` at the schema root natively. The same attribute is added to `src/cli/doctor.rs::DoctorReport` for documentation of intent — harmless at runtime because the real struct only derives `Serialize`, never `Deserialize`. The `Config` schema intentionally does NOT set `additionalProperties: false` — extra keys are tolerated in the config layer because users may add their own keys under `[wiki]` or `[nim]` sections that the typed `Config` struct doesn't surface, and the deep-merge semantics in `src/core/registry.rs::deep_merge_into` need to see those keys.

**Drift protection (DoctorReport side)**: `tests/doctor_test.rs::doctor_json_output_validates_against_schema` asserts three things every CI run: (1) the on-disk schema validates against a real `wiki doctor --json` run against a mocked NIM, (2) the schema's `properties` key set matches a hard-coded canonical 15-key set, and (3) the `nim_status` field's `"minimum": 100, "maximum": 599` HTTP-range constraint is present. The keys check is the load-bearing half — `jsonschema::is_valid` alone cannot catch drift between `src/cli/doctor.rs::DoctorReport` and the `build.rs` duplicate, because the real struct's output would still satisfy a schema that silently lost a field.

**Drift protection (Config side)**: `tests/doctor_test.rs::config_schema_has_canonical_keys` asserts the on-disk `Config` schema's `properties` key set matches a hard-coded canonical set covering `nim`, `wiki`, and `config_version`. There is no `jsonschema::is_valid` end-to-end check on the Config schema (the runtime `Config` is deserialized from TOML, not the schema) — drift protection is keys-only.

**Maintainer invariant**: When you change either `src/core/config.rs::Config` / `src/cli/doctor.rs::DoctorReport` or the corresponding build.rs duplicate, you MUST change both. The drift tests catch forgetting. Build also re-runs because `build.rs` declares `cargo:rerun-if-changed=src/cli/doctor.rs` and `cargo:rerun-if-changed=src/core/config.rs`.

## Cargo.lock (v0.3.20+, platform-specific question resolved in v0.3.23)

`Cargo.lock` is **tracked** in git (un-ignored in v0.3.20). For binary crates this gives reproducible builds.

**v0.3.21 concern** was that cargo's lockfile might be **platform-specific** — it pins deps like `windows-sys` to a Linux target that does not exist on macOS dev boxes (and vice versa), which would cause the macOS-committed lockfile to fail `cargo check --locked` on the Linux CI runner. As a precaution, v0.3.21 dropped `--locked` from `msrv-check`.

**v0.3.23 resolution**: the `.github/workflows/lockfile-update.yml` workflow was triggered manually (run #28054735840) and produced a lockfile byte-identical to the macOS-committed one (`git rev-list --right-only --count main...lockfile-update` returned 0). For the current dep graph, the platform-specific lockfile concern does not apply, and v0.3.23 re-enabled `--locked` on `msrv-check`. The lockfile-update workflow is kept as "always-fresh" automation — if a future Dependabot PR introduces a dep with genuine platform-specific entries, the workflow's new `cargo check --all-targets --locked` verification step (v0.3.23) will fail and refuse to open a PR, surfacing the issue before CI breaks.
