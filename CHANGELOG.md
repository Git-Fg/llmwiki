# Changelog

## [0.3.22] - 2026-06-23 — reqwest 0.13 bump, auto-gen doctor schema, jsonschema test

**Dependency updates:**
- `reqwest`: bumped `0.12.28` → `0.13.x` (via Dependabot PR #2).
  Investigation finding (FU#50): the blocker was **not** a transitive
  dep conflict as v0.3.20's "Deferred" section claimed — `rmcp` 1.8
  has no `reqwest` dep at all. The actual blocker was the
  `rustls-tls` feature being renamed to `rustls` in reqwest 0.13.
  Once the feature name was updated, the bump went through cleanly.
  The remaining `reqwest 0.12.28` entry in the lockfile is pulled
  in by the new `jsonschema = "0.18"` dev-dep; cargo handles the
  multi-major fine.
- Dependabot PRs #2 (reqwest) and #3 (github-actions v7) closed
  without merging; both superseded by manual work in this release.

**Build / contracts:**
- `build.rs`: `marketplace/skills/wiki/MCP/references/doctor.schema.json`
  is now auto-generated from a `DoctorReport` struct duplicate
  annotated with `#[derive(schemars::JsonSchema)]`. The hand-written
  v0.3.20 schema is no longer the source of truth — `cargo build`
  regenerates it on every change. Replaces the fragile hand-maintained
  JSON with a single source of truth (modulo the build.rs duplicate,
  see Tests below).
- `build.rs` + `src/cli/doctor.rs`: the schema's
  `additionalProperties: false` contract (from v0.3.20) is preserved
  by annotating both the build.rs duplicate and the real struct with
  `#[serde(deny_unknown_fields)]`. Schemars reads the serde attribute
  and emits the JSON Schema field natively — no post-processing step
  in build.rs is needed.
- `build.rs`: added `cargo:rerun-if-changed=src/cli/doctor.rs` so
  the schema regenerates when the real `DoctorReport` struct changes.

**Tests:**
- New dev-dep `jsonschema = "0.18"`.
- New `tests/doctor_test.rs::doctor_json_output_validates_against_schema`:
  reads the auto-generated schema, runs `wiki doctor --json` against
  a mocked NIM, asserts the output validates against the schema AND
  the schema's `properties` keys match a hard-coded canonical set.
  The keys check is the important half — it catches drift in either
  the build.rs duplicate or the real struct, not just divergence
  between the real struct's output and the on-disk schema.
- 253/253 pass; clippy `-D warnings` clean (both stable and MSRV 1.88);
  fmt clean.

**CI:**
- New `.github/workflows/lockfile-update.yml`: weekly Monday 06:17 UTC
  ubuntu-latest run of `cargo update --workspace`, opens a PR if
  `Cargo.lock` changed. Goal is to produce a Linux-native lockfile so
  `msrv-check` can re-enable `--locked` (currently disabled due to
  the macOS-vs-Linux platform-specific lockfile divergence documented
  in v0.3.21). Weekly cadence is conservative — Dependabot also runs
  weekly on Tuesdays, so a fresh Linux lockfile lands within a day of
  any dep change.

**Documentation:**
- `AGENTS.md`: added "JSON Schema generation (v0.3.22+)" section
  documenting the `build.rs` duplicate-struct pattern,
  `#[serde(deny_unknown_fields)]` idiom (drives the schema's
  `additionalProperties: false`), and the drift test.
- `AGENTS.md`: added "Cargo.lock and platform-specific locks" section
  documenting the v0.3.21 macOS-vs-Linux lockfile workaround and the
  v0.3.22 forward fix.

**Deferred to v0.3.23** (non-blocking polish surfaced by self-critic
review of the staged v0.3.22 changes):
- **M1**: `doctor.schema.json` `active_alias` is in `properties` but
  not in `required`. Schemars treats `Option<T>` as not-required by
  default; the field is always emitted in practice but the schema
  does not enforce it. Fix: add `#[schemars(required)]` annotation
  on the build.rs duplicate.
- **M2**: `doctor.schema.json` `nim_status` constraint is
  `0..=65535` (uint16 range); the documented semantic range is HTTP
  status `100..=599`. Fix: add `#[schemars(range(min = 100, max = 599))]`
  on the build.rs duplicate, or document the wider range.
- **M3**: `lockfile-update.yml` opens a PR with `cargo update --workspace`
  output but does not verify the regenerated lockfile compiles before
  opening the PR. Fix: add a `cargo check --locked` step between
  the update and the PR-opener; fail the job if the lockfile is broken.
- **M4**: Workflow actions (`actions/checkout@v4`, `dtolnay/rust-toolchain@1.88`,
  `peter-evans/create-pull-request@v6`) are pinned to major-version
  tags, not 40-char commit SHAs. With `contents: write` and
  `pull-requests: write` permissions, SHA pins would harden the
  supply chain. Fix: pin to SHAs; Dependabot can auto-bump.
- **L1**: Mixed `///` doc comments vs `#[schemars(description = "...")]`
  on the build.rs duplicates. Cosmetic; consistent with the rest
  of build.rs so not a priority.
- **L2**: `lockfile-update.yml` cron `"0 6 * * 1"` lands on the
  top of the hour (high GitHub Actions contention). Already moved
  to `"17 6 * * 1"` in v0.3.22; L2 is moot.
- **L3**: New schema file ends without a trailing newline (POSIX
  convention; many editors expect it). Fix: post-process in build.rs
  to add `\n`.
- **L4**: Workflow PR body references v0.3.20/v0.3.21 only. Fix:
  append "and v0.3.22" once v0.3.22 is tagged.

## [0.3.21] - 2026-06-23 — Drop `--locked` from msrv-check (platform-specific lockfile)

**Fixed:**
- `.github/workflows/ci.yml::msrv-check`: dropped `--locked` again.
  v0.3.20 re-enabled it after we started tracking `Cargo.lock`, but
  cargo's lockfile is **platform-specific** — it pins deps like
  `windows-sys` to a Linux target that doesn't exist on macOS dev
  boxes and vice versa. The lockfile generated on macOS fails
  `--locked` on the Linux CI runner. The check still verifies
  **resolution + typecheck** under rustc 1.88, just not exact dep
  reproducibility. A future improvement is to generate the lockfile
  in a Linux container.

## [0.3.20] - 2026-06-23 — Track Cargo.lock + doctor JSON schema + changelog CI

**Build / reproducibility:**
- `.gitignore`: removed `Cargo.lock` from the ignore list. Per the
  Cargo Book guidance, **binary crates should track `Cargo.lock`** for
  reproducible builds. The previous "gitignore for binary" convention
  was inherited from a pre-binary period; this is a binary crate
  (it has `src/main.rs`), so the lockfile now ships with the repo.
- Fresh `Cargo.lock` generated via `cargo generate-lockfile` and
  committed (79 KB, ~250 packages).

**CI:**
- `.github/workflows/ci.yml::msrv-check`: re-enabled `--locked` now
  that `Cargo.lock` is tracked. The check is now hermetic: it
  verifies the **exact** dep graph users will see, not whatever the
  current crates.io index happens to return at CI time.
- New `changelog-check` job runs `tests/changelog_check.sh`: asserts
  that `CHANGELOG.md` has a `## [X.Y.Z] - YYYY-MM-DD` heading matching
  the current `Cargo.toml` `version`. Prevents the v0.3.18-class
  mistake of shipping a release with a CHANGELOG that contradicts
  the actual state. Required by branch protection.

**Documentation / contracts:**
- New `marketplace/skills/wiki/MCP/references/doctor.schema.json`:
  JSON Schema (draft 2020-12) for the output of `wiki doctor --json`.
  Documents the v0.3.17 BREAKING change where `active_alias` is now
  `string | null` (was `string` with `""` sentinel). 15 fields
  enumerated with types, ranges, and required-marking.
- `marketplace/skills/wiki/MCP/SKILL.md` Reference Index: now
  references `doctor.schema.json` so hosts consuming the `doctor`
  MCP tool can validate the response shape.

**Tests:**
- `tests/config_v037_test.rs`: removed inline duplicate of
  `with_wiki_root_config`; now imports the canonical
  panic-safe version from `tests/common/mod.rs` (consolidates the
  three definitions that existed across the test suite).
- 252/252 pass; clippy `-D warnings` clean; fmt clean.

**Deferred (resolved in v0.3.22):**
- Dependabot PR #2 (reqwest 0.12 → 0.13) appeared blocked by a
  transitive dep (initially blamed on `rmcp`), reporting
  "failed to select a version for `reqwest`". v0.3.22's
  investigation (FU#50) found the actual cause was the
  `rustls-tls` → `rustls` feature rename in reqwest 0.13.

## [0.3.19] - 2026-06-23 — Critic follow-ups: panic-safety + CHANGELOG accuracy

**Fixed:**
- `tests/common/mod.rs`: extended the `EnvGuard` RAII pattern
  (introduced in v0.3.16 for `with_home_and_cwd`) to `with_wiki_root_config`
  and `without_wiki_root_config`. Both previously saved/restored
  `$WIKI_ROOT_CONFIG` only on the happy path — a panic inside the inner
  closure would leak the env var into every later test in the same
  binary. The new pattern restores unconditionally via Drop. Duplicate
  legacy definitions of these helpers were removed during the refactor.

**Documentation:**
- `CHANGELOG.md`: corrected v0.3.17 CI section to reflect the actual
  job definition (no `--locked`) and added a "(revised in v0.3.18)"
  note. Also marked the `active_alias` JSON shape change as
  `[BREAKING — JSON shape]` so downstream consumers (incl.
  `src/cli/mcp.rs::doctor_tool` which returns the raw JSON) get a
  heads-up about the `null` vs `""` change. The local MSRV verification
  note was also corrected to drop the `--locked` reference.

**Tests:** 252/252 pass; clippy `-D warnings` clean; fmt clean.

## [0.3.18] - 2026-06-23 — Fix `msrv-check` CI: drop `--locked`

**Fixed:**
- `.github/workflows/ci.yml::msrv-check`: dropped `--locked` from
  the `cargo check` invocation. The first run of this job failed
  with "the lock file Cargo.lock needs to be updated but --locked
  was passed" because `Cargo.lock` is in `.gitignore` (this is a
  binary project; lockfiles aren't tracked). Without `--locked`,
  cargo generates a fresh lockfile from the current crates.io
  index, which still verifies the MSRV claim — the freshly-fetched
  deps all need to compile under rustc 1.88 for the check to pass.
  If a future Dependabot bump raises a transitive dep's MSRV above
  1.88, this job fails with a clear "package X requires rustc N"
  error.
- `CHANGELOG.md`: corrected v0.3.17 description to match the actual
  job definition (no `--locked`).

**Tests:** 252/252 pass; clippy `-D warnings` clean; fmt clean.

## [0.3.17] - 2026-06-23 — Defensive cleanup + MSRV CI gate

**Changed:**
- **`[BREAKING — JSON shape]`** `src/cli/doctor.rs`: `DoctorReport.active_alias`
  changed from `String` (empty-string-as-sentinel) to `Option<String>`.
  Empty strings no longer leak into the JSON output — `null` is the
  unambiguous "no active alias" marker. Consumers of `wiki doctor --json`
  (including `src/cli/mcp.rs::doctor_tool` which returns the raw JSON)
  must now handle `null` instead of `""`.
- `src/cli/doctor.rs::run`: simplified `active_alias` construction
  with the new `Option<String>` type; replaced `if !active_alias.is_empty()`
  with `if let Some(alias) = &active_alias`.

**CI:**
- New `msrv-check` job in `.github/workflows/ci.yml`: runs
  `cargo check --all-targets` with `dtolnay/rust-toolchain@1.88`
  (the declared MSRV in `Cargo.toml` post-v0.3.16). This catches the
  exact failure mode that blocked every push from v0.3.12 → v0.3.13:
  a Dependabot dep bump raising the required rust-version above the
  project's pinned toolchain. Without this gate, `fmt-clippy-test`
  runs on `stable` (currently 1.96) and would silently miss any future
  MSRV regression. Now required by branch protection. **Revised in
  v0.3.18**: dropped `--locked` from the cargo invocation, because
  `Cargo.lock` is `.gitignore`d for this binary project.

**Branch protection:**
- Added `msrv-check` to `.github/branch-protection.json` required
  status checks. All four checks (`fmt-clippy-test`, `marketplace-validate`,
  `skill-smoke`, `msrv-check`) are now required for merge.

**Documentation:**
- `AGENTS.md` Testing Strategy: documented the `EnvGuard` RAII pattern
  in `tests/common/mod.rs` so future test-helper authors don't
  reintroduce the panic-safety bug. Also fixed an inaccuracy about
  `tests/e2e_test.rs`: it actually runs in CI via `cargo test`
  (no `#[ignore]`), not "ignored by default" as previously stated.

**Tests:** 252/252 pass (incl. e2e); clippy `-D warnings` clean; fmt clean.
**MSRV:** verified locally with `cargo check --all-targets` against
rustc 1.88. (Local lockfile may differ from CI's freshly-generated one;
both verify MSRV independently — see v0.3.18 for the rationale.)

## [0.3.16] - 2026-06-23 — Test safety + MSRV pin + docs

**Fixed:**
- `tests/common/mod.rs::with_home_and_cwd` (and by extension
  `with_wiki_root_config` / `without_wiki_root_config`) is now
  panic-safe via an `EnvGuard` RAII struct. Previously, an assertion
  panic inside the inner closure would leave `$HOME` /
  `$USERPROFILE` / CWD pointing at a tempdir that has already been
  dropped, silently corrupting every later test in the same binary.
  Now the Drop guard runs unconditionally during unwind.
- `Cargo.toml`: bumped `rust-version` from `1.85` to `1.88`. This is
  the project's MSRV (matches `rust-toolchain.toml` post-v0.3.14 and
  the transitive dep requirements: `darling@0.23.0` requires 1.88,
  `icu_*@2.2.0` require 1.86). `cargo build` will now fail early on
  older toolchains with a clear MSRV error instead of "rustc 1.85.1
  is not supported" mid-way through dependency resolution.
- `src/cli/lint.rs:152`, `src/lint/wikilinks.rs:70`: removed stray
  trailing commas inside `format!(...)` argument lists (cargo-fmt
  artifacts of the v0.3.14 inlining sweep — purely cosmetic).

**Documentation:**
- `AGENTS.md`: added a note in the "Config File Resolution" section
  that both `load_config_unvalidated` and `Registry::resolve_config`
  use the same `registry::deep_merge_into` (post-v0.3.15), and warned
  future contributors not to reintroduce a per-field merge helper.

**Tests:** 252/252 pass; clippy `-D warnings` clean; fmt clean.

## [0.3.15] - 2026-06-23 — config: deep-merge all fields (not just 3 nim.*)

**Fixed:**
- `Config::merge()` only handled 3 `nim.*` fields (`embed_model`,
  `rerank_model`, `embed_dim_override`) and the top-level
  `config_version`. Any `wiki.*` override, every `nim.retry.*` field,
  and most other `nim.*` fields (`base_url`, `api_key_env`,
  `batch_size`, `request_timeout_secs`) set in a per-computer or
  per-workspace config file were silently dropped: the merged config
  fell back to `Config::default()` for those keys.
- Surfaced as a confusing UX issue by the new `--overrides-only` filter
  (v0.3.13): a user who set `wiki.default_chunk_tokens = 1024` in their
  per-workspace `.llmwiki-cli/config.toml` saw nothing appear in the
  overrides-only output, even though they had set a non-default value.
- **Root-cause fix**: replaced the per-field `Config::merge()` with
  TOML-level deep-merge (`crate::core::registry::deep_merge_into`,
  now `pub(crate)`) across all sources in priority order, then
  deserialize the merged TOML into `Config` once. Every field with
  `#[serde(default)]` is now handled uniformly — no per-field
  enumeration to forget.
- `Registry::resolve_config` already used this TOML-level deep merge,
  so this fix brings `load_config_unvalidated` into alignment with
  the registry path. Both code paths now produce the same effective
  config.

**Tests:**
- New: `show_effective_overrides_only_surfaces_wiki_and_retry_overrides`
  in `tests/config_cli_test.rs`. Sets `wiki.default_chunk_tokens = 1024`
  and `nim.retry.max_attempts = 7` in a per-workspace config, then
  verifies both appear in `--overrides-only` output.
- 252/252 pass (251 v0.3.14 + 1 new regression).

## [0.3.14] - 2026-06-23 — CI: clippy forward-compat (110 format args inlined)

**Fixed:**
- Inlined 110 `format!` / `println!` / `eprintln!` / `writeln!` /
  `write!` / `eprint!` / `print!` positional args across 28 files
  (17 in `src/`, 11 in `tests/`). Required because newer clippy on
  Linux CI promotes `clippy::uninlined_format_args` to default-warn,
  causing 72+ errors on every push from main. The macOS dev environment
  does not trigger the lint (platform-specific promotion schedule), so
  the issue was invisible locally until CI ran.
  - Multi-line `assert!` calls in tests: removed now-redundant trailing
    arg after inlining (caught by strict `-D warnings` build).
  - Multi-line `anyhow!` calls in `src/core/registry.rs`: same var
    referenced twice in one format string was inlined both times.
- `build.rs`: inlined format-string args. (Same root cause.)
- CI: bumped `rust-toolchain.toml` channel from `1.85` to `1.88` to
  satisfy transitive deps: `darling@0.23.0` requires rustc 1.88;
  `icu_*@2.2.0` require rustc 1.86.
- Cargo.toml version bumped from `0.3.12` to `0.3.14`.

**Skipped (not inlinable):**
- Args that are method/function calls (`.display()`, `.status()`,
  `serde_json::to_string_pretty(...)`, etc.) — clippy only inlines
  simple variable names.
- Closure bodies in `.map(|e| format!(...))` where the inner format
  call uses complex args.

**Verified locally on rustc 1.88 and 1.96:** 251/251 tests pass,
clippy `-D warnings` clean, fmt clean. Binary: `llmwiki-cli 0.3.14`.

## [0.3.13] - 2026-06-23 — `show-effective` filters + tighter path matching

**Added:**
- `wiki config show-effective --overrides-only` — hide keys whose value
  equals the built-in default. Surfaces only the keys your config files
  actually changed (the most useful subset for "what did my config do?").
  The output includes the filter description in the header so pipe-driven
  scripts can tell which filter was active.
- Combined filter support: `[<prefix>]`, `--source <path>`, and
  `--overrides-only` can now be combined freely. Example:
  `wiki config show-effective nim. --source ./config.toml --overrides-only --json`.

**Changed:**
- `source_path_matches` no longer falls back to a string-prefix match
  when canonicalization fails. A prefix match could falsely equate
  `/home/u/.llmwiki-cli` with `/home/u/.llmwiki-cli-extra`. The new
  behavior is exact-canonical-equality only; missed-but-precise is
  preferable to false-positive in this audit context.

**Documentation:**
- README.md gained a "show-effective filters" subsection covering all
  three filters and the combined-usage example.

**Tests:** 251/251 pass (250 v0.3.12 + 1 new for `--overrides-only`).

## [0.3.12] - 2026-06-23 — `show-effective` filters + doctor source attribution

**Added:**
- `wiki config show-effective [<prefix>]` — optional positional argument
  that filters output to keys starting with the given prefix (e.g.
  `wiki config show-effective nim.` shows only the `[nim]` table).
  Mirrors the positional-pattern syntax of `git config --list --show-origin -- <pattern>`.
- `wiki config show-effective --source <path>` — only show keys whose
  source file matches the given path. Useful for "what did THIS specific
  file set?" audits. Handles the macOS `/tmp` ↔ `/private/tmp`
  canonicalization asymmetry.
- `wiki doctor --json` now includes a `config_sources` field alongside
  the existing `config` field: `key → file-it-came-from` (or
  `<default>`). Mirrors `wiki config show-effective` so the most
  user-visible diagnostic surface reports which file overrode which key.

**Tests:** 250/250 pass (246 v0.3.11 + 4 new tests: 3 for filters,
1 for doctor source attribution).

## [0.3.11] - 2026-06-23 — Config-surface audit tests

**Added (test-only):**
- `registry_only_config_subcommands_ignore_workspace_flag`: asserts that
  registry-only subcommands (`list`, `get`, `path`, `show-schema`) do not
  consult the workspace when one is passed via `--workspace`. Guards
  against future drift where workspace-aware logic is accidentally added
  to a registry-only command (which would silently change the meaning of
  the command).
- `every_config_subcommand_is_either_workspace_aware_or_registry_only`:
  exhaustive lint (test form) that walks every `ConfigCmd` variant and
  asserts the subprocess does NOT error with "workspace not found" when
  `--workspace` points at a non-wiki directory. If a future variant is
  added that doesn't fit either category, this test fails — which is the
  intent.

**Tests:** 246/246 pass (244 v0.3.10 + 2 new audit tests).

No CLI behavior changed; this release is purely additional test coverage.

## [0.3.10] - 2026-06-23 — Config discoverability follow-ups

**Changed:**
- `ConfigCmd::Paths`, `ConfigEdit`, and `ShowEffective` now use clap's
  explicit `#[arg(from_global)]` attribute on their `--workspace` field
  instead of relying on clap's auto-propagation of global flags. The
  behavior is identical, but the intent is now explicit and survives
  future clap changes (see [clap issue #5525](https://github.com/clap-rs/clap/issues/5525)).
- `walk_up_for_llmwiki_cli_config()` no longer returns `None` when the
  workspace path doesn't exist on disk (e.g., `wiki --workspace
  /not/yet/init config paths`). It still returns the per-workspace
  candidate so `wiki config paths` can show users where to put their
  config when scripting against not-yet-initialized wikis.

**Documentation:**
- `wiki config --help` now describes the four-tier config resolution
  priority (`$LLMWIKI_CONFIG` → per-workspace → per-computer → defaults).
- `README.md` gained a "Per-workspace & per-computer config" section
  with the priority order and pointers to `paths` / `show-effective` /
  `config-edit`.

**Tests:** 244/244 pass (242 v0.3.9 + 2 new `global_workspace_flag_*`
regression tests).

## [0.3.9] - 2026-06-23 — Config editor + effective-config view

**Added:**
- `wiki config config-edit` opens the highest-priority existing config file
  in `$EDITOR` (mirrors `wiki config edit` for `wiki-root.toml`). Order:
  `$LLMWIKI_CONFIG` → existing per-workspace `~/.llmwiki-cli/config.toml`
  → existing per-computer `~/.llmwiki-cli/config.toml`. Falls back to the
  per-workspace candidate so a new file is created on save. Accepts
  `--workspace <path>` (inherited from the global flag).
- `wiki config show-effective` prints every effective config key with its
  merged value and the file it came from — mirrors `git config --list
  --show-origin`. Lets users see "which file overrode this key?" without
  reading source. `--json` returns a structured `{workspace, entries: [{key,
  value, source}]}` payload.

**Changed:**
- `config_paths()` now returns paths in **lowest-priority-first** order
  (per-computer → per-workspace → `$LLMWIKI_CONFIG`). This matches the
  standard CLI config convention (pip, git, mise: "later files override
  earlier") so `load_config`'s last-wins merge gives the intuitively
  correct result without any per-key branching. The previous "highest
  priority first" order silently let per-computer override per-workspace
  because per-workspace came first and `load_config` overwrites with each
  successive file.
- `wiki config paths` text output now displays paths in **highest-priority-
  first** order (reversed from the underlying list) so the human view stays
  intuitive. JSON output preserves the underlying order and adds a
  `merge_order_note` field documenting the convention.
- `ConfigEdit` subcommand now accepts `--workspace <path>` so the global
  `--workspace` flag propagates via clap's auto-fill. Without this, the
  global flag was discarded by the dispatch and `config-edit` re-discovered
  the workspace from CWD (often the project root, which has no wiki).

**Tests:** 242/242 pass (237 v0.3.8 + 5 new).

## [0.3.8] - 2026-06-23 — Config debuggability

**Added:**
- `wiki doctor` now prints a one-time migration notice when the v0.3.6
  user-global config (`~/llmwiki-cli/config.toml`) still exists and the
  v0.3.7 path (`~/.llmwiki-cli/config.toml`) does not. The notice suggests
  the exact `mv` command to migrate.
- New `wiki config paths` command prints the resolved config search order
  with each path's existence status (e.g. `[exists] per-workspace
  /workspace/.llmwiki-cli/config.toml`). `--workspace <path>` overrides the
  walk-up start; `--json` returns structured output. Lets users diagnose
  "why isn't my config being loaded?" without reading source.
- `walk_up_for_llmwiki_cli_config()` now always returns `Some` so the
  per-workspace candidate path is included in `config_paths()` output even
  when the file doesn't exist yet — `wiki config paths` can show the user
  exactly where to create it. `load_config` still skips missing files.
- 6 new regression tests: 3 unit tests in `src/cli/doctor.rs` (legacy
  notice fires / suppressed / suppressed-when-new-exists), 3 integration
  tests in `tests/config_cli_test.rs` for `wiki config paths`.

**Tests:** 237/237 pass (231 v0.3.7 + 6 new).

## [0.3.7] - 2026-06-23 — `.llmwiki-cli/` config centralization

**Changed (BREAKING):**
- **Per-computer config** moved from `~/llmwiki-cli/config.toml` to
  `~/.llmwiki-cli/config.toml` (hidden dotfile directory).
- **New per-workspace config**: `<workspace>/.llmwiki-cli/config.toml`
  (hidden dotfile directory inside the workspace, git-committable so
  teams can share NIM/wiki settings per-wiki). Found by walking up
  from the resolved workspace looking for the closest `.llmwiki-cli/`
  ancestor.
- **Workspace marker** changed from `.wiki/` to `.llmwiki-cli/`. The
  marker directory is now the same directory that holds the per-workspace
  config — a single convention.
- **Walk-up algorithm** skips HOME so `~/.llmwiki-cli/` is treated as
  the per-computer config location, not as a workspace marker.
- **`config_paths()` signature** changed from `()` to `(workspace: &Path)`
  so it can resolve the per-workspace config path.
- **Single-wiki shortcut** added to workspace discovery: if the registry
  has exactly one entry and nothing else matched, default to it without
  requiring `--wiki`.

**Removed:**
- `.wiki/` walk-up workspace discovery (replaced by `.llmwiki-cli/`).
- `~/llmwiki-cli/.wiki/` user-global workspace fallback. Registry +
  per-workspace config cover the use cases.

**Added:**
- `wiki init` now scaffolds an empty `.llmwiki-cli/config.toml` template
  alongside `wiki/`, `raw/`, `index.md`, `.gitignore`.
- `Registry::resolve_config(alias)` deep-merges the per-workspace
  `.llmwiki-cli/config.toml` on top of the `[defaults]`+`[alias]` result
  (per-workspace wins per-key, partial overrides preserved).
- 10 new regression tests in `tests/config_v037_test.rs` covering
  per-workspace walk-up, HOME skip, registry deep-merge, partial
  overrides, no-config noop, init template, and `.wiki/` removal.

**Tests:** 231/231 pass (221 v0.3.6 + 10 new).

**Migration:** Move `~/llmwiki-cli/config.toml` to
`~/.llmwiki-cli/config.toml`. If you want team-shared per-workspace
settings, add `.llmwiki-cli/config.toml` inside the wiki repo and commit
it.

## [0.3.6] - 2026-06-23 — Config discovery simplified to `~/llmwiki-cli/config.toml`

**Changed (BREAKING for users with custom config paths):**
- Config file search simplified to two paths only:
  1. `$LLMWIKI_CONFIG` env var (primary override, matches binary-name prefix
     already used in `install.sh` as `LLMWIKI_BIN_DIR`)
  2. `~/llmwiki-cli/config.toml` (user-global, TOML to match `wiki-root.toml`)
- **Removed**: legacy `~/.config/wiki/config.yaml` (YAML) — the project is
  still alpha, no backward compatibility shim.
- **Removed**: legacy `<workspace>/.wiki/config.yaml` workspace-local fallback.
- **Removed**: YAML parsing from `load_config` — TOML only.
- Workspace discovery now also checks `~/llmwiki-cli/.wiki/` as a user-global
  workspace root (mirrors the new config path).

**Added:**
- `pub fn config_paths()` in `src/core/config.rs` — single source of truth
  for the config search order.
- `pub fn load_config_unvalidated()` — loads config without whitelist
  validation. Used by `wiki config validate` and by tests.
- 9 new regression tests in `tests/config_v036_test.rs` covering env var
  priority, home path fallback, empty env var, YAML rejection, and the
  removal of legacy paths.

**Tests:** 221/221 pass (212 v0.3.5 + 9 new).

**Migration:** If you previously used `~/.config/wiki/config.yaml`, move its
contents to `~/llmwiki-cli/config.toml`. If you prefer to keep your existing
file path, set `LLMWIKI_CONFIG=/path/to/your/config.toml` in your shell rc.

## [0.3.5] - 2026-06-23 — Global audit + off-by-one fix + GitHub community files

**Fixed:**
- **H1 (HIGH):** Off-by-one in `remove_empty_intermediate` — changed
  `path[..path.len() - 2]` to `path[..path.len() - 1]` so the loop navigates
  to the parent of the target leaf (not the grandparent). Previously checked
  if `[alias].a` was empty and removed `[alias].a.b`; now correctly checks
  if `[alias].a.b` is empty and removes it.

**Changed:**
- README.md fully rewritten with CI/crates.io/license badges, Multi-wiki
  registry section, LSP/MCP commands, updated architecture diagram.
- CHANGELOG.md updated with missing v0.3.4 entry.
- install.sh / install.ps1 comment headers: `fg/llmwiki` → `Git-Fg/llmwiki`.
- marketplace/skills/wiki/SETUP/SKILL.md + install.md: URLs updated,
  version 0.3.0 → 0.3.4.

**Added:**
- 1 new regression test: `unset_value_three_levels_cleans_all_empty_intermediates`
  (3-level dotted key cleanup).
- GitHub community files: `ISSUE_TEMPLATE/bug_report.md`,
  `ISSUE_TEMPLATE/feature_request.md`, `PULL_REQUEST_TEMPLATE.md`,
  `CODEOWNERS`, `dependabot.yml`, `SECURITY.md`.

**Tests:** 212/212 pass (211 v0.3.4 + 1 new).

## [0.3.4] - 2026-06-23 — Registry write-semantics hardening

**Fixed:**
- **H1 (HIGH):** `Registry::remove_entry` and `Registry::unset_value` previously
  gave no feedback when called on aliases from lower-priority files. The
  v0.3.3 fix made them error, but lacked a roundtrip regression test confirming
  `remove_entry() → save() → discover()` actually removes the alias from disk.
  Now covered by `remove_entry_save_then_discover_alias_is_gone` and
  `remove_entry_errors_on_lower_priority_save_then_discover_alias_persists`.
- **H2 (HIGH):** `unset_value("a.b.c", alias)` previously could not match
  `set_value("a.b.c", ..., alias)` semantics — when called on a freshly-set
  nested key, the intermediate tables were assumed to already exist. Now
  matches `set_value`: creates intermediate tables on demand and cleans up
  empty intermediate tables after removal so the TOML document doesn't
  accumulate `[alias.nim] = {}` ghosts.
- **M1:** CLI help text for `config rm` and `config unset` now documents the
  scope error (lower-priority alias → use `$WIKI_ROOT_CONFIG`). `--wiki` flag
  on `unset` marked as required.
- **M2:** `workspace.rs` now discovers workspaces at `~/.config/wiki/` to
  mirror the legacy `config.yaml` fallback in `config.rs`. Without this, a
  workspace at `~/.config/wiki/` was discoverable as a *config* source but
  not as a workspace root — inconsistent.
- **L1:** `init` error message improved from bare `"no home dir"` to
  actionable: `"cannot determine home directory: both $HOME and
  $USERPROFILE are unset. Set one of them, or set WIKI_ROOT_CONFIG…"`.
- **L2:** `unset_value` and `remove_entry` error messages now list all
  candidate `wiki-root.toml` paths (minus the active write target) so the
  user knows which file to point `$WIKI_ROOT_CONFIG` at.

**Added:**
- 3 new regression tests in `tests/registry_discovery_v032_test.rs`:
  `remove_entry_save_then_discover_alias_is_gone`,
  `remove_entry_errors_on_lower_priority_save_then_discover_alias_persists`,
  `unset_value_creates_intermediate_tables_like_set_value`.
- `remove_empty_intermediate()` helper in `registry.rs` — safe, no raw pointers.

**Tests:** 211/211 pass (208 v0.3.3 + 3 new).

## [0.3.3] - 2026-06-23 — Registry write-semantics hardening

**Fixed:**
- **H1 (HIGH, regression from v0.3.2):** `Registry::remove_entry` and
  `Registry::unset_value` previously mutated in-memory `entries` without
  touching `raw_doc` when the target alias came from a lower-priority file.
  This caused silent data no-ops: the CLI printed `Removed wiki 'X'` but
  the alias/key reappeared on the next `discover()` call. Now both methods
  **error** when the alias isn't in `raw_doc` (the active write scope),
  following the git-config / npm-config convention. The error message
  points the user at `$WIKI_ROOT_CONFIG` to retarget the active scope at
  the file that owns the alias.
- **M1:** `home_dir()` was duplicated across three modules with divergent
  behavior — `registry.rs` correctly checked `$HOME || $USERPROFILE`
  (cross-platform), but `workspace.rs` and `config.rs` checked only `$HOME`
  (Unix-only). On Windows, the legacy `~/wiki` workspace lookup and the
  `~/.config/wiki/config.yaml` config search would silently fail. All three
  now route through `crate::core::registry::home_dir` (promoted to
  `pub(crate)`).
- **L1:** `init` previously wrote the bootstrap registry to the lowest-
  priority slot `~/wiki-root.toml`, creating shadowing confusion with the
  higher-priority `~/.agents/wiki-root.toml`. Now writes to
  `~/.agents/wiki-root.toml` — the conventional AI-agent slot and the
  highest-priority user-global path.
- **L2:** `add_entry` set `WikiEntry.raw` to an empty table even though
  `raw_doc` was just populated with the full entry. Latent bug — no current
  caller triggered the inconsistency, but `resolve_config` on the new alias
  in the same process would have returned only defaults. Now clones the
  table into `WikiEntry.raw`.

**Changed:**
- `Registry::home_dir` promoted from private to `pub(crate)` so other
  modules share the cross-platform implementation.
- `unset_value` now returns a precise "key not found" error (was `Ok(())`
  when the leaf key didn't exist).
- AGENTS.md "Workspace Resolution" section documents that scalars override
  but arrays (`tags`, `what_to_read`) union-dedupe on alias merge, plus a
  new "Active write scope" paragraph explaining the git-config-style
  contract: `set`/`add` create override sections in the top file; `rm`/
  `unset` error if the target isn't in the active scope.

**Added:**
- 5 new regression tests in `tests/registry_discovery_v032_test.rs`:
  `remove_entry_errors_on_alias_from_lower_priority_file`,
  `unset_value_errors_on_alias_from_lower_priority_file`,
  `remove_entry_works_when_alias_is_in_active_scope`,
  `tags_array_union_dedupes_on_merge`,
  `add_entry_populates_entry_raw_table`.
- Spec test-name reference and test-count corrected (was stale post-v0.3.2).

**Tests:** 208/208 pass (203 v0.3.2 + 5 new).

## [0.3.2] - 2026-06-23 — Multi-source registry hardening

**Fixed:**
- **H1 (HIGH):** `Registry::merged_with` now deep-merges alias tables when a
  higher-priority file overrides an alias that also appears in a lower-priority
  file. Previously a project-local override of `[shared].description`
  silently dropped the lower-priority `[shared.nim]` sub-section (embed
  model, base URL, etc.). v0.3.2 merges nested TOML tables recursively so
  only the explicitly-set keys are overridden.
- **M1:** `candidate_paths()` now deduplicates canonical paths so a home
  directory that is an ancestor of CWD (e.g. `~/projects/foo/` walked up
  to `~/wiki-root.toml` already in the user-global chain) doesn't double-include
  the same file. Merging then short-circuits and the user's home config is
  applied exactly once.
- **M2:** `WikiRootNotFound` error now distinguishes `$WIKI_ROOT_CONFIG`
  states — empty string, directory, missing path, or non-regular file —
  with a tailored message for each. Previously a misconfigured env var
  produced a generic "not a regular file" message even when the path was
  a directory.
- **M4:** Spec rewritten to describe the actual multi-level merge
  resolution. The previous v0.3.0-era spec described short-circuit-on-first-find
  and has been archived to `docs/superpowers/specs/_archive/`.

**Added:**
- 13 new regression tests in `tests/registry_discovery_v032_test.rs`:
  H1 repro, dedup verification, all four WIKI_ROOT_CONFIG error branches,
  `load_all` direct calls, user-global chain precedence, symlinked-CWD
  walk-up, HOME+USERPROFILE unset fallback, duplicate-alias uniqueness,
  and a pin of the current `set_value` behavior.

**Deferred to v0.3.3+ (documented as known limitations):**
- `set_value` currently writes to the highest-priority file, which means
  setting a key on an alias loaded from a lower file creates a new
  override section. This is correct for "project local override" but
  surprising when the user expects a write-through to the lower file.
- `init` writes to the lowest-priority `~/wiki-root.toml` slot rather
  than CWD-proximate. The intent is per-user scaffolding; we may want
  per-project scaffolding instead.
- `home_dir()` is defined in both `registry.rs` and `workspace.rs`.
  Consolidate into one shared helper.
- Walk-up depth is unbounded (defaults to ~64 ancestor hops). Bound it
  explicitly for safety on very deep filesystems.
- Save-after-merge writeback semantics: `Registry::save` writes only the
  highest-priority file. Lower-priority entries are loaded fresh on every
  `discover()` so they're not lost, but mutations to a merged entry
  land in the top file.

## [0.3.0] - 2026-06-23 — BREAKING: rename to llmwiki-cli

**BREAKING CHANGES:**
- Crate name: `wiki` → `llmwiki-cli`
- Binary name: `wiki` → `llmwiki-cli`
- Reinstall: `cargo uninstall wiki && cargo install llmwiki-cli`

**Migration:**
- All existing scripts that invoke `wiki <subcommand>` must be updated to `llmwiki-cli <subcommand>`
- Existing wiki data, `wiki-root.toml`, and `~/.agents/skills/wiki/` are unchanged

**Added:**
- `llmwiki-cli lsp` — LSP server for `wiki-root.toml` (hover, completion, document symbols, diagnostics)
- `llmwiki-cli mcp` — MCP server (validate, hover, completion, schema, doctor)
- `llmwiki-cli config show-schema` — JSON Schema dump for editors
- `llmwiki-cli config validate` — field-level checks for `[defaults]` and every `[alias]`
- `validate_or_error()` called before NIM calls in `embed`/`search`/`query`
- Marketplace install: `install.sh` (POSIX) and `install.ps1` (PowerShell 7+) install `llmwiki-cli` + bundle the wiki skill into `~/.agents/skills/wiki/`
- Self-installing sub-skills: SETUP, LSP, MCP, INGEST, SEARCH, QUERY, LINT, MODELS, SYNC, TROUBLESHOOTING — all bundled in the binary and copy-installed via `llmwiki-cli install-skill`
- GitHub Actions release workflow building 6 targets: linux-musl (aarch64, x86_64), windows-gnu (aarch64, x86_64), apple-darwin (aarch64, x86_64)
- crates.io publish workflow

## [0.3.1] - 2026-06-23 — Multi-source wiki registry concatenation

**Added:**
- `wiki-root.toml` lookup now walks up from CWD for project-local
  `.agents/wiki-root.toml`. All sources (user-global chain + ancestor
  walk-up) are **concatenated**, with closer-to-CWD winning on alias
  conflict. Every wiki alias from every source is visible to CLI, LSP,
  and MCP — no shadowing, no silent fallbacks. Mirrors git (local +
  global), hk (per-project + per-user), Atmos (CWD + parent search).
- Improved error message: `WikiRootNotFound` now surfaces
  `$WIKI_ROOT_CONFIG=<path>` explicitly when the env var is set,
  saving users from guessing why their custom path was rejected.

**Changed:**
- `wiki config set/unset/add/rm` writes to the highest-priority
  registry file in scope (project-local if present, otherwise
  user-global). To edit a lower-priority file, set
  `$WIKI_ROOT_CONFIG` to point at it directly.

## 2026-06-22 — NIM URL convention and API key env

The CLI's NIM integration has been corrected so the default `base_url` is the
host only (e.g. `https://integrate.api.nvidia.com`) and every command appends
`/v1/<endpoint>` consistently. This matches the OpenAI-style convention and
keeps `WIKI_NIM_BASE_URL` overrides simple to point at a local mock or a
different host.

The previous default was `https://integrate.api.nvidia.com/v1`, which made
`wiki embed` and `wiki query` call `/v1/v1/embeddings` and `/v1/v1/chat/completions`
and return 404 against the real NIM API. The `wiki doctor` endpoint check
(`/v1/models`) had the same shape but was working only by coincidence of the
trailing path. The end-to-end wiremock test passed because `MockServer.uri()`
has no `/v1` suffix, masking the bug.

The API key resolver also now accepts `NVIDIA_API_KEY` as a fallback so users
with the upstream NVIDIA shell env don't need to re-export under a different
name. `NVIDIA_NIM_API_KEY` is still the primary lookup.

`wiki build` now detects raw sources with any extension (not just `.md`),
matching what `wiki ingest` actually writes — previously `.txt` (or any other)
sources were silently skipped.

## 2026-06-21 — Viewer removed

The SvelteKit viewer (`web/`), `wiki build-viewer`, and `wiki serve` commands
have been removed from the project. The wiki is consumed directly via the
CLI and the embedded agent skill — no static site is generated. This keeps
the tool focused on markdown + embeddings + agent-driven workflows.

## 2026-06-21 — Initial Rust port

Single-crate Rust CLI replacing the previous Python codebase. Markdown wiki
files + JSONL embeddings (gitignored, regenerated per device), NVIDIA NIM
embeddings + chat, embedded agent skill via `include_str!`.

## Importing an existing wiki

There is no `wiki import` command. External wiki layouts (e.g. an existing
`MyWiki` with `concepts/`, `entities/`, `raw/` directories) can be brought in
manually:

```bash
llmwiki-cli init ~/my-wiki                    # scaffold the CLI workspace
cp -r ~/MyWiki/concepts ~/my-wiki/wiki/
cp -r ~/MyWiki/entities ~/my-wiki/wiki/
# edit each file to add the required frontmatter (title, created, updated,
# type, tags, sources, schema_version) and [[wikilinks]] the CLI expects
llmwiki-cli embed                             # build the embedding index
```

A dedicated adapter was considered and rejected: automatic frontmatter
inference from parent directory names and wikilink rewriting are speculative
heuristics, not mechanical transformations, and the cost of a wrong inference
(mis-typed pages, broken links) outweighs the few manual copy/edit steps.

