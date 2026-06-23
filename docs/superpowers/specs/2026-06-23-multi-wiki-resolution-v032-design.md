# Multi-Wiki Resolution v0.3.2 — Design Spec

**Date:** 2026-06-23
**Status:** Finalization design (post v0.3.1 tag)
**Replaces:** `_archive/2026-06-23-multi-wiki-resolution-v030-design.md` (v0.3.0-era
short-circuit design, retained as historical record of the design iteration).

## Background

v0.3.0/v0.3.1 introduced multi-source `wiki-root.toml` lookup: every file in
scope is loaded and **merged**, with closer-to-CWD winning on alias conflict
and `[defaults]` deep-merged per key. The v0.3.2 release hardens this design
in response to a self-critic review (`agent-44`).

## Self-Critic Findings (v0.3.1 → v0.3.2 delta)

| ID  | Severity | Finding                                                                                  | Fixed in v0.3.2 |
| --- | -------- | ---------------------------------------------------------------------------------------- | --------------- |
| H1  | HIGH     | `merged_with` did whole-entry replacement, dropping lower file's `[alias.nim]` sub-keys.  | ✅              |
| M1  | MEDIUM   | Walk-up duplicated paths when HOME is an ancestor of CWD.                                 | ✅              |
| M2  | MEDIUM   | `WIKI_ROOT_CONFIG=""` or pointing at a directory gave misleading error.                  | ✅              |
| M3  | MEDIUM   | `set_value` silently creates override section for an alias loaded from a lower file.     | Deferred (documented as v0.3.3 work) |
| M4  | MEDIUM   | Old spec described short-circuit behavior; this release archive + rewrite.                | ✅              |
| L1  | LOW      | `init` writes to lowest-priority user-global slot.                                       | Deferred |
| L2  | LOW      | Duplicate `home_dir()` definitions in registry.rs vs workspace.rs.                       | Deferred |
| L3  | LOW      | Walk-up depth unbounded.                                                                  | Deferred |
| L4  | LOW      | `WikiRootNotFound` doesn't differentiate empty vs missing.                                | ✅ (subsumed by M2) |

## Implementation (v0.3.2)

### H1 fix: deep-merge alias tables

`Registry::merged_with()` now deep-merges the alias's raw TOML table via the
new `merge_alias_tables()` helper, then re-derives `WikiEntry.path`,
`tags`, `description`, `what_to_read`, `qmd_slug` from the merged table.

Behavior change (this is the bug fix):
- Before: `[shared] path=/B description=p` in the higher file completely
  replaced `[shared] path=/A description=g\n[shared.nim] embed_model=GLOBAL`
  from the lower file. The merged alias had only `path=/B description=p` —
  the `[shared.nim]` block was silently lost.
- After: The higher file's `path`/`description` override the lower file's,
  but the lower file's `[shared.nim]` block survives. The merged alias
  table is `{path=/B, description=p, nim={embed_model=GLOBAL}}`.

Test coverage: `alias_subkeys_preserved_when_only_top_level_overridden`
in `tests/registry_discovery_test.rs` reproduces the H1 scenario and
asserts the merged alias has both the higher-priority `description` AND
the lower-priority `[shared.nim]` block.

### M1 fix: dedupe candidate paths

`Registry::candidate_paths()` now calls `dedupe_paths()` (new helper) to
remove duplicates by canonical path. Prevents `~/.agents/wiki-root.toml`
from being added twice when HOME is an ancestor of CWD.

Test coverage: `candidate_paths_dedupes_when_cwd_walks_into_home_agents` in
`tests/registry_discovery_v032_test.rs`.

### M2 fix: WIKI_ROOT_CONFIG error differentiation

The registry code now inspects `$WIKI_ROOT_CONFIG` and produces a
human-readable suffix describing the failure mode:

| `$WIKI_ROOT_CONFIG` value       | Error suffix                                                                |
| ------------------------------ | --------------------------------------------------------------------------- |
| unset                          | (no suffix)                                                                 |
| `""` (empty)                   | ` (WIKI_ROOT_CONFIG is set to an empty string; unset it or point it at a real file)` |
| `/path/to/file` (missing)      | ` (WIKI_ROOT_CONFIG=/path/to/file did not exist)`                            |
| `/path/to/dir` (existing dir)  | ` (WIKI_ROOT_CONFIG=/path/to/dir exists but is a directory, not a file)`    |
| `/path/to/dev` (special file)  | ` (WIKI_ROOT_CONFIG=/path/to/dev is not a regular file)`                     |

The `from_env` field on `WikiError::WikiRootNotFound` now stores the
pre-formatted suffix string instead of the raw env var value.

Test coverage: `wiki_root_config_empty_string_distinguished_from_missing`
and `wiki_root_config_directory_distinguished_from_missing` in
`tests/error_test.rs`.

## Resolution Order (v0.3.2)

`Registry::candidate_paths()` returns paths in **lowest-to-highest priority** order:

1. `$WIKI_ROOT_CONFIG` (hard override, single path; no merging, no fallback)
2. User-global chain (lowest priority, loaded first):
   - `~/wiki-root.toml`
   - `~/.claude/wiki-root.toml`
   - `~/.agents/wiki-root.toml`
3. Project-local chain (ancestor walk-up from CWD, closest-to-CWD first,
   then reversed so closest wins on conflict):
   - `<closest-ancestor>/.agents/wiki-root.toml`
   - ... up to `<farthest-ancestor>/.agents/wiki-root.toml`
4. Dedupe by canonical path.

Then `Registry::load_all(paths)`:
- For each path that exists as a file: `load_from(p)` and `merged_with(r)`
- `merged_with`: deep-merges alias tables (H1 fix), deep-merges `[defaults]`,
  adopts the highest-priority file's `root_path` + `raw_doc`.
- If no file existed: `WikiRootNotFound` with `from_env` suffix.

## Deferred to v0.3.3+

- **M3**: `set_value` silently creates override sections. Two options:
  (a) refuse by default, require explicit `--create-override`; (b) emit a
  warning. Leaning toward (a) for safety.
- **L1**: `init` should pick the conventional `~/.agents/wiki-root.toml` slot
  when no registry exists anywhere.
- **L2**: Consolidate `home_dir()` into a single `crate::core::paths` module.
- **L3**: Bound walk-up depth (default 64) to avoid pathological `/proc`-style
  filesystems.
- **Save-after-merge semantics**: lower-priority entries that came from
  non-`raw_doc` sources are not preserved on `save()`. Future work could
  either refuse to save (with a clear error pointing at the missing file)
  or rewrite each source file independently. Documented as a known
  limitation; mitigated for v0.3.1 users because `wiki config set` is the
  primary mutator and it explicitly targets the highest-priority file.

## Acceptance Criteria

- 203+ tests pass (188 v0.3.1 baseline + 13 v0.3.2 tests + 5 v0.3.3 regression tests).
- All 6 CI gates green: build, test, clippy, fmt, validator, e2e.
- `cargo clippy --all-targets -- -D warnings` clean.
- `cargo fmt --check` clean.
- Marketplace validator passes.
- Shellcheck on `install.sh` clean.
- All 4 GitHub Actions workflows parse as valid YAML.
- Release binary reports `llmwiki-cli 0.3.2`.
