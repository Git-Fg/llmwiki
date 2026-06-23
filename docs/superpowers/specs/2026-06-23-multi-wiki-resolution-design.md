# Multi-Wiki Resolution Finalisation — Design Spec

**Date:** 2026-06-23
**Status:** Finalization design (post v0.3.0 tag)
**Spec refines:** `docs/superpowers/specs/2026-06-22-wiki-root-toml-design.md`

## Audit Findings

The registry (`src/core/registry.rs`) and workspace discovery (`src/core/workspace.rs`) implement a 7-step priority chain, but **one key convention is missing**: project-local registry files in the CWD or its ancestor directories. The user's request — `~/.agents/wiki-root.toml` and/or `.agents/wiki-root.toml` resolved from CWD — points to this gap.

### Current resolution order

`Registry::candidate_paths()` (registry.rs:131-142):
```
1. $WIKI_ROOT_CONFIG
2. ~/.agents/wiki-root.toml
3. ~/.claude/wiki-root.toml
4. ~/wiki-root.toml
```

`discover_workspace()` (workspace.rs:15-67):
```
1. --workspace flag
2. --wiki <alias> flag (looks up path in registry)
3. $WIKI_WORKSPACE env
4. $WIKI_ACTIVE env (looks up path in registry)
5. registry CWD prefix match
6. walk up from CWD for .wiki/ (legacy)
7. ~/wiki if it has .wiki/
```

### Prior art (web search)

- **Atmos** (closest analog): walks up from CWD for `atmos.yaml`. Precedence: CLI flag > env > profile > current dir > parent search > git repo root > home > system.
- **hk**: per-project `hk.local.pkl` overrides per-user `~/.config/hk/config.pkl`.
- **git**: `--local` (`.git/config`) overrides `--global` (`~/.gitconfig`) overrides `--system` (`/etc/gitconfig`).

All three tools follow the same principle: **project-local config > user-global > system-wide**.

### Gap

`llmwiki-cli` only supports user-global registry locations. A project that wants a scoped wiki-root.toml (e.g., a monorepo with `frontend-wiki` and `backend-wiki` aliased for team use) must either:
- Pollute `~/.agents/wiki-root.toml` (not team-shareable)
- Use `$WIKI_ROOT_CONFIG` per invocation (not ergonomic)
- Use `wiki init` in the project (loses multi-wiki aliasing)

## Goal

Make `llmwiki-cli` honor project-local `.agents/wiki-root.toml` resolved from CWD, with **project-local taking precedence over user-global**. Match the convention used by Atmos, hk, and git.

## Non-Goals

- No deep-merge of project-local + user-global registries (pick one, project wins). A user with both files is choosing per-project scoping.
- No new TOML format. Same `[alias]` table.
- No new env vars. Existing `$WIKI_ROOT_CONFIG` already covers "use exactly this file".
- No changes to `wiki-root.toml` schema or semantics.

## Resolution Order (Final)

```
Registry::candidate_paths() (highest priority first):
1. $WIKI_ROOT_CONFIG                          [env var, exact path override]
2. <cwd>/.agents/wiki-root.toml               [NEW: project-local, walk-up]
3. <cwd>/../.agents/wiki-root.toml            [NEW: ancestor walk-up]
4. ... (continue walking up to /)
5. ~/.agents/wiki-root.toml                   [user-global, agents host]
6. ~/.claude/wiki-root.toml                   [user-global, claude host]
7. ~/wiki-root.toml                           [user-global, no-host fallback]
```

If `WIKI_ROOT_CONFIG` is set, ONLY that path is consulted (no fallback). Otherwise, project-local wins if found, then user-global.

`discover_workspace()` priority stays as documented. The change is internal to how the registry is loaded.

## Edge Cases (Resolved)

| Case | Behavior |
| --- | --- |
| `$WIKI_ROOT_CONFIG` set | Only that file consulted. No walk-up, no fallback. |
| Project has `.agents/wiki-root.toml` | That file used (walk-up). User-global ignored. |
| Project has `.claude/wiki-root.toml` but no `.agents/` | NOT consulted (only `.agents/` walk-up; mirrors user-global convention). |
| Both `~/.agents/wiki-root.toml` and `<cwd>/.agents/wiki-root.toml` exist | Project-local wins (matches git/hk/atmos). |
| Project-local file is malformed TOML | Hard error at parse time; do NOT silently fall back to user-global. The user wrote the file; they must fix it. |
| Project-local file has zero `[alias]` entries | Treated as a valid (empty) registry. Subsequent alias lookups return `AliasNotFound`. |
| CWD is `/` (filesystem root) | Walk-up terminates; falls through to user-global. |
| CWD symlinked | `canonicalize()` first, then walk-up. Resolves the symlink before walking. |
| Multiple `.agents/wiki-root.toml` in ancestors (e.g., nested git worktrees) | Closest-to-CWD wins. Same as git's `.git/config` resolution. |
| `--workspace /nonexistent` flag | Still canonicalizes and returns; downstream commands fail with their own error. (Unchanged behavior — flag is a hard override.) |
| `--wiki foo` with project-local registry that has no `foo` | `AliasNotFound` error listing only the project-local entries (not user-global entries, which are shadowed). |

## Implementation

**File: `src/core/registry.rs`**

Replace the `candidate_paths()` body with walk-up logic. Walk from `current_dir()` up to `/`, checking `<ancestor>/.agents/wiki-root.toml` at each level. After walk-up, fall back to user-global.

```rust
pub fn candidate_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // 1. Hard override via env var — no fallback.
    if let Ok(p) = std::env::var("WIKI_ROOT_CONFIG") {
        paths.push(PathBuf::from(p));
        return paths;
    }

    // 2. Walk up from CWD looking for project-local .agents/wiki-root.toml.
    if let Ok(cwd) = std::env::current_dir() {
        if let Ok(canonical_cwd) = cwd.canonicalize() {
            let mut current = Some(canonical_cwd);
            while let Some(dir) = current {
                let candidate = dir.join(".agents").join("wiki-root.toml");
                if candidate.is_file() {
                    paths.push(candidate);
                    return paths; // project-local wins; no fallback
                }
                current = dir.parent().map(Path::to_path_buf);
            }
        }
    }

    // 3. User-global fallback chain.
    if let Some(home) = home_dir() {
        paths.push(home.join(".agents").join("wiki-root.toml"));
        paths.push(home.join(".claude").join("wiki-root.toml"));
        paths.push(home.join("wiki-root.toml"));
    }
    paths
}
```

**Key behaviors locked in:**
- `WIKI_ROOT_CONFIG` short-circuits everything (no fallback).
- First project-local match short-circuits walk-up (closest-to-CWD wins, no multi-level merge).
- Walk-up failure falls through to user-global chain.
- Home fallback chain preserved (`.agents` → `.claude` → bare).

**File: `src/core/registry.rs` — error reporting**

When `WIKI_ROOT_CONFIG` points to a missing file, the current code returns `WikiRootNotFound { searched: [that one path] }`. Improve the error message to explicitly say "WIKI_ROOT_CONFIG points to a missing file". Small fix, prevents support confusion.

**File: `src/cli/init.rs`**

When `wiki init` runs in a directory, write an empty `.agents/wiki-root.toml` if one doesn't exist (to make project-local scoping explicit). Optional — leave to a follow-up if scope creep risk. **Decision: skip for v0.3.0.** Project-local files are opt-in; users create them manually.

## Tests

Add to `tests/registry_test.rs` (or a new `tests/registry_discovery_test.rs`):

1. `candidate_paths_with_wiki_root_config_skips_walkup` — `$WIKI_ROOT_CONFIG=/tmp/foo` returns only `/tmp/foo`, even if CWD has `.agents/wiki-root.toml`.
2. `candidate_paths_walks_up_from_cwd` — CWD is `~/project/sub/`, project root has `.agents/wiki-root.toml`, walk-up finds it before user-global.
3. `candidate_paths_project_wins_over_user_global` — both exist; project-local returned first.
4. `candidate_paths_falls_back_to_user_global` — no project-local anywhere; user-global chain returned.
5. `candidate_paths_walkup_stops_at_filesystem_root` — CWD is `/`, no project-local; user-global returned.
6. `candidate_paths_handles_nonexistent_wiki_root_config` — `$WIKI_ROOT_CONFIG=/nope` set; `Registry::discover()` returns `WikiRootNotFound` (not silent success).

Add to `tests/workspace_test.rs`:

7. `discover_workspace_uses_project_local_registry` — CWD is in a project with `.agents/wiki-root.toml`; workspace discovery finds a wiki via the project-local registry without consulting `~/.agents/wiki-root.toml`.

## Docs Updates

**`AGENTS.md`** — add a "Workspace Resolution" section:

```markdown
## Workspace Resolution

The CLI locates the active wiki (or the registry of wikis) in this order:

1. `--workspace <path>` flag (hard override)
2. `--wiki <alias>` flag (looks up alias in the registry)
3. `$WIKI_WORKSPACE` env var
4. `$WIKI_ACTIVE` env var (looks up alias in the registry)
5. Registry CWD prefix match against registered wiki paths
6. Walk up from CWD looking for `.wiki/` (legacy v0.1 convention)
7. `~/wiki` if it has `.wiki/`

Registry file lookup (used by `--wiki`, `$WIKI_ACTIVE`, and the CWD prefix match):

1. `$WIKI_ROOT_CONFIG` — hard override, no fallback
2. Walk up from CWD looking for `.agents/wiki-root.toml` (project-local wins)
3. `~/.agents/wiki-root.toml` (user-global, agents host)
4. `~/.claude/wiki-root.toml` (user-global, claude host)
5. `~/wiki-root.toml` (user-global, no-host fallback)

Project-local > user-global. Mirrors git, hk, and Atmos.
```

**`CHANGELOG.md`** — append to v0.3.0 section:

```markdown
- `wiki-root.toml` lookup now walks up from CWD for project-local
  `.agents/wiki-root.toml` before falling back to user-global.
  Project-local > user-global (matches git, hk, Atmos conventions).
```

**`marketplace/skills/wiki/SETUP/references/install.md`** — link to AGENTS.md for the full precedence table.

## Risks

| Risk | Mitigation |
| --- | --- |
| User has `~/.agents/wiki-root.toml` with one alias and creates `.agents/wiki-root.toml` in CWD; expects both to apply | Documented: project-local SHADOWS user-global, doesn't merge. Single alias set wins. |
| Walk-up finds `.agents/wiki-root.toml` from a parent project, user confused why their commands operate on parent project | `current_dir().canonicalize()` + closest-wins means it's always the closest project. Error message includes the resolved path so users can debug. |
| Symlink loops in walk-up | `canonicalize()` resolves symlinks; `parent()` on root returns `None`, loop exits. |
| Performance: 100-deep CWD walk-up | Bounded by filesystem depth; in practice ≤20. Negligible. |
| Test pollution from existing user-global file | All new tests use `tempfile::tempdir()` and set `HOME` to a temp dir via `env::set_var` (Rust test isolation). |

## Acceptance Criteria

- `Registry::candidate_paths()` returns project-local path first when present.
- `Registry::candidate_paths()` returns only `$WIKI_ROOT_CONFIG` when set.
- New tests cover all 7 cases in the Edge Cases table.
- All existing 175 tests still pass.
- `cargo clippy --all-targets -- -D warnings` clean.
- `cargo fmt --check` clean.
- AGENTS.md has the new resolution section.
- CHANGELOG.md has the v0.3.0 addendum.
