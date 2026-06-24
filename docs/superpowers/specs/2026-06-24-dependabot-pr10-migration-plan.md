# Dependabot PR #10 — Migration plan & risk assessment

**Status:** Research synthesis. Review + decision document.
**Date:** 2026-06-24
**PR:** <https://github.com/Git-Fg/llmwiki/pull/10>
**Triggered:** 2026-06-24 after v0.3.33 merge (847c0ca)

## TL;DR

**3 of the 7 bumped deps are dead weight** — never `use`d anywhere in source. Bump them is wrong; **remove them** is right. The other 4 bumps need verification but are mostly low-risk. **Do NOT auto-merge.**

## Discovery: dead direct dependencies

Cross-referencing `Cargo.toml [dependencies]` against `grep -rn 'use <dep>\|<dep>::' src/ tests/`:

| Dep | In Cargo.toml | Source usage | Verdict |
|---|---|---|---|
| `pulldown-cmark = "0.12"` | ✓ | **None** | Dead — remove |
| `notify = "6"` | ✓ | **None** | Dead — remove |
| `indicatif = "0.17"` | ✓ | **None** | Dead — remove |
| `toml = "0.8"` | ✓ | `src/core/registry.rs`, `src/core/models_registry.rs` | Used |
| `rust-embed = "6"` | ✓ | `src/skills/mod.rs` | Used |
| `thiserror = "1"` | ✓ | `src/error.rs` (and likely others) | Used |
| `jsonschema = "0.18"` | dev-dep | Not directly imported | Dev-only, used via `wiremock` test infra |

The first three probably survived from an earlier iteration of the CLI (likely a feature-flag for live markdown rendering, filesystem watching, progress bars) that was refactored out without removing the Cargo.toml entries. PR #10 attempts to bump them — that's strictly worse than removing them. Removing dead deps:
- Cuts compile time
- Cuts attack surface (smaller dep tree)
- Eliminates future Dependabot noise on unused code

## Per-dep risk assessment

### `pulldown-cmark 0.12.2 → 0.13.4` — REMOVE

- **Source usage:** 0
- **Risk of bumping:** N/A (no callers)
- **Risk of removing:** None — tests confirm no breakage
- **Action:** Delete line 25 from Cargo.toml

### `notify 6.1.1 → 8.2.0` — REMOVE

- **Source usage:** 0
- **Risk of bumping:** N/A
- **Risk of removing:** None
- **Migration cost to bump instead:** 3 majors. notify 7.0 moved event types to `notify-types` crate; notify 8.0 raised MSRV to 1.77. We don't use any of those APIs.
- **Action:** Delete line 26 from Cargo.toml

### `indicatif 0.17.11 → 0.18.4` — REMOVE

- **Source usage:** 0
- **Risk of bumping:** N/A
- **Risk of removing:** None
- **Action:** Delete line 32 from Cargo.toml

### `rust-embed 6.8.1 → 8.11.0` — BUMP (low risk)

- **Source usage:** `src/skills/mod.rs` — `#[derive(RustEmbed)]`, `#[folder = "src/skills/data/"]`, `SubSkillBundle::get(path)`, `SubSkillBundle::iter()`, `EmbeddedFile.data`
- **Migration per docs.rs/changelog:**
  - **8.0.0 (Aug 2023):** store file contents statically + binary search lookup. Performance only.
  - **8.1.0 (Dec 2023):** add `created` to file metadata. Additive.
  - **8.2.0 (Dec 2023):** fix naming collisions in macros.
  - **8.4.0 (May 2024):** re-export `RustEmbed` as `Embed`. Old import still works.
  - **8.5.0 (Jul 2024):** raise min rust-version to 1.70. We're at 1.88. ✓
  - **8.7.0 (Apr 2025):** add deterministic-timestamps flag. Opt-in.
  - **8.9.0 (Oct 2025):** ignore uncanonicalizable paths. Defensive.
- **Verdict:** API surface we use is fully backward-compatible across 6.x → 8.x. Bump should be a Cargo.toml line change + recompile. Verify `src/skills/mod.rs` still compiles.

### `thiserror 1.0.69 → 2.0.18` — BUMP (low risk, ZERO for our code)

- **Source usage:** `src/error.rs` (only 52 lines, derive-heavy)
- **Breaking changes from 2.0.0 release notes:**
  - `{r#type}` syntax no longer accepted; use `{type}`. **We use neither** (no `r#type` fields).
  - Trait bounds not inferred on fields shadowed by named args. **We don't use named-arg shadowing** in any `#[error(...)]` format string.
  - Tuple structs can't mix `{0}` and extra positional args. **We have no tuple-struct variants** with mixed args (only `NimUnreachable(String)` and `UnknownSkillTopic(String)` use single `{0}`).
  - Direct `thiserror` dep required. **We have `thiserror = "1"` in Cargo.toml already.**
- **Verdict:** Zero migration work needed for our code. Bump is a Cargo.toml line change + recompile. Tests should pass.

### `toml 0.8.23 → 1.1.2` — BUMP (UNKNOWN risk)

- **Source usage:** `src/core/registry.rs`, `src/core/models_registry.rs` — `toml::from_str`, `toml::Value`, `content.parse()` (which deserializes to `toml::Value`)
- **Risk:** Without an authoritative changelog URL working, the exact API changes between 0.8 → 1.1 are unknown. Possible areas of change:
  - `toml::Value` enum variants
  - `toml::from_str` error type changes
  - `Display` / `Debug` formatting changes
- **Verdict:** **HIGH unknown risk.** Must compile + run full test suite + verify registry.rs deserialization paths before merging.
- **Mitigation:** If the bump breaks code, consider pinning to latest 0.x (e.g. `toml = "0.8.23"` exact-pin or `"<0.9"` upper-bound) until v0.3.34+ migration work.

### `jsonschema 0.18.3 → 0.46.6` (dev-dep) — BUMP (low risk)

- **Source usage:** Not directly imported. Likely used through `wiremock` test infra or similar.
- **Risk:** Dev-deps don't affect release binaries. Test failures are caught by CI.
- **Verdict:** Bump freely; let CI tell us if anything broke.

## Recommended action plan (in order)

### Step 1: Remove dead deps (5 min)

```diff
 pulldown-cmark = "0.12"
-notify = "6"
 walkdir = "2"
 anyhow = "1"
 thiserror = "1"
 tracing-subscriber = { version = "0.3", features = ["env-filter"] }
-indicatif = "0.17"
 schemars = "1.0"
```

Run `cargo build` after — should compile clean. Run `cargo test` — should pass with same 288 tests.

### Step 2: Bump rust-embed 6 → 8 (10 min)

```diff
-rust-embed = "6"
+rust-embed = "8"
```

Run `cargo build` — `src/skills/mod.rs` should compile unchanged. Run `cargo test` — `hub_does_not_contain_sub_skill_bodies_inline` and friends should pass.

### Step 3: Bump thiserror 1 → 2 (5 min)

```diff
-thiserror = "1"
+thiserror = "2"
```

Run `cargo build` — `src/error.rs` should compile unchanged. Run `cargo test` — all error-display assertions should pass.

### Step 4: Bump toml 0.8 → 1.1 (15-30 min, may need code changes)

```diff
-toml = "0.8"
+toml = "1"
```

Run `cargo build`. If broken, see migration guide or pin to `"<0.9"`. Run `cargo test` — registry tests should pass.

### Step 5: Bump jsonschema dev-dep 0.18 → 0.46 (5 min)

```diff
-jsonschema = "0.18"
+jsonschema = "0.46"
```

Let CI catch any test infra breakage.

### Step 6: Bump remaining minor versions (pulldown-cmark, indicatif) — N/A, removed in Step 1

### Step 7: Verify, commit, push (10 min)

- `cargo test` → 288 (or more)
- `cargo clippy --all-targets -- -D warnings` → clean
- `cargo fmt --check` → clean
- `tests/skill_smoke.sh` → pass
- Commit as v0.3.34 or as a docs-only / refactor commit
- Push and let CI run all 4 checks

## Estimated total effort

| Step | Time |
|---|---|
| Step 1: Remove dead deps | 5 min |
| Step 2: rust-embed bump | 10 min |
| Step 3: thiserror bump | 5 min |
| Step 4: toml bump (may need research) | 15–30 min |
| Step 5: jsonschema dev-dep bump | 5 min |
| Step 6-7: verify + commit + push | 10 min |
| **Total** | **~50 min – 1.5 hours** |

## Risk matrix

| Bump | Risk | Code changes expected | Fallback |
|---|---|---|---|
| Remove `pulldown-cmark` | None | 0 | N/A |
| Remove `notify` | None | 0 | N/A |
| Remove `indicatif` | None | 0 | N/A |
| `rust-embed 6→8` | Low | 0 (verified by changelog) | Pin to 6.x |
| `thiserror 1→2` | Low | 0 (verified by code audit) | Pin to 1.x |
| `toml 0.8→1.1` | **Medium–High** | Unknown | Pin to `<0.9` if blocked |
| `jsonschema 0.18→0.46` | Low | Dev-only | Pin to 0.x if CI fails |

## Should we close PR #10 without merging?

**Recommendation:** Close PR #10 and open a fresh PR with the cleaner diff above (remove dead deps + bump only the used ones). Reasons:
1. **Smaller blast radius.** A PR that removes 3 lines and bumps 4 is easier to review and revert than a PR that bumps 7.
2. **Better narrative.** "Remove dead deps + bump 4 used deps" tells reviewers exactly what's happening.
3. **No functional regressions.** Bumps that don't change runtime behavior shouldn't be mixed with cleanup that does.

## Alternative: merge PR #10 + immediately open a cleanup PR

If you want to consume Dependabot's diff first:
1. Merge PR #10
2. Open PR #12: "remove dead deps" — drops `pulldown-cmark`, `notify`, `indicatif` from Cargo.toml
3. Add this migration plan as the PR description

Either way, the **dead-dep removal** is a separate concern from the version bumps and should not be auto-merged.

## References (primary sources)

- thiserror 2.0 release: <https://github.com/dtolnay/thiserror/releases/tag/2.0.0>
- rust-embed changelog: <https://docs.rs/crate/rust-embed/8.9.0/source/changelog.md>
- notify changelog: <https://github.com/notify-rs/notify/blob/main/notify/CHANGELOG.md>
- Copilot CLI spec-divergence precedent: <https://github.com/github/copilot-cli/issues/894>

## Open questions

1. **Was `notify` ever used?** Need to git-blame the Cargo.toml line to understand when it was added and whether the use case is in a deferred backlog.
2. **Was `pulldown-cmark` ever used?** Same question. A previous milestone might have used it for markdown rendering; if so, is that roadmap item still alive?
3. **Was `indicatif` ever used?** Progress bars for `wiki embed`? `wiki ingest`? If so, we may want to re-add it for a future UX improvement.

These are tangential to PR #10 but worth answering before the dead-dep removal PR, so reviewers know whether to expect re-additions in v0.3.35+.

## Verification commands

```bash
# Confirm dead deps after the removal
grep -rn 'use pulldown_cmark\|pulldown_cmark::' src/ tests/ 2>/dev/null
grep -rn 'use notify\|notify::' src/ tests/ 2>/dev/null
grep -rn 'use indicatif\|indicatif::' src/ tests/ 2>/dev/null

# Should return zero matches in each case

# Full test suite
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
tests/skill_smoke.sh
```