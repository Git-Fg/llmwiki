# Changelog

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

