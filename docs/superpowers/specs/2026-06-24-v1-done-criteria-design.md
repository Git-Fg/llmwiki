# Design: v1.0 Done Criteria and Refinement Roadmap

> **Purpose**: Define what "finalised" means for `llmwiki-cli`. Establishes a
> 3-release path (v0.3.26 → v0.4.0 → v1.0.0) with explicit acceptance gates,
> so "we shipped v1.0" is a falsifiable claim, not a vibe.
>
> **Scope**: This is a meta-spec about project-shape. It does NOT itself
> introduce any new CLI commands, config keys, or behavior changes — those
> live in per-release design docs (starting with v0.3.26, see "Immediate
> next step" below).

## Background and motivation

The project has shipped 11 releases (v0.3.14 → v0.3.25) in rapid succession
and explicitly self-identifies as "alpha" (v0.3.22 CHANGELOG). The pace has
been good for surfacing latent bugs (e.g. flat-layout discovery in v0.3.25,
parse_frontmatter resilience in v0.3.25, SSoT schema generation in v0.3.25)
but the cadence cannot continue indefinitely — every release is a SemVer
breaking change under the 0.x.y convention, and downstream consumers
(marketplace agents, the LSP/MCP servers, future contributors) need a stable
target.

The v0.3.25 self-critic (`docs/superpowers/specs/2026-06-24-v0.3.25-self-critic.md`)
lists 2 HIGH + 4 MEDIUM + 3 LOW findings. These are necessary fixes but not
a definition of "done". This spec defines "done".

## Definitions

**v1.0 done** = all three dimensions below are green simultaneously:

| Dimension | What it means | Who can falsify it |
|---|---|---|
| **Correctness** | The CLI behaves the way users (and the spec) say it should | Anyone running it against a real wiki |
| **Stability** | The public surface (CLI commands, `--json` shapes, config keys) is documented and won't change without a major version bump | Anyone reading the schemas and CHANGELOG |
| **Real-world validation** | At least one external user has run v0.5.0+ against a non-trivial wiki for ≥ 1 week without filing a P0 bug | The external user |

## The three releases

```
v0.3.25 (shipped)  ──► v0.3.26 ──► v0.4.0 ──► v1.0.0
                    (1 wk)    (3-4 wks) (gate)
                    correct-  stab-    real-world
                    ness      ility    validation
```

Each release has ONE primary dimension it advances. v0.3.26 advances Correctness.
v0.4.0 advances Stability (and folds in CI maturity + dep hygiene as
infrastructure prerequisites). v1.0.0 advances Real-world validation and
**tags the stable release**.

## Dimension 1: Correctness (advanced by v0.3.26)

### Why this is the first dimension

A v1.0 with a wrong default (`pages_dir = "wiki"`) or with a discovery walker
that picks up `.opencode/node_modules/.../README.md` is not "done" regardless
of how stable its API is. Correctness comes first.

### v0.3.26 scope

Picks up every HIGH + MEDIUM + the actionable LOW from the v0.3.25 self-critic:

| ID | Item | Reference |
|---|---|---|
| H1 | Flip `wiki.pages_dir` default `"wiki"` → `""` | self-critic H1 |
| H2 | Add `wiki.exclude_dirs: Vec<String>` config field with sensible defaults | self-critic H2 + research docs |
| M1 | Rename `frontmatter-parse` lint code → `frontmatter-yaml-parse` | self-critic M1 |
| M2 | Update `marketplace/skills/wiki/SKILL.md` to teach agents about both layouts | self-critic M2 |
| M3 | Update `AGENTS.md` filesystem-layout example to show flat | self-critic M3 |
| M4 | `wiki init` prints resolved `pages_dir` + config path | self-critic M4 |
| L1 | Delete the 4 per-workspace `config.toml` files I wrote to user's real wikis | self-critic L1 |
| (new) | Commit the 2 untracked research docs (or absorb into v0.3.26 spec) | this spec |

### `wiki.exclude_dirs` defaults (informed by research)

The qmd + Foam research (`docs/superpowers/specs/2026-06-24-exclude-dirs-research.md`
and `docs/superpowers/specs/2026-06-24-wiki-exclusion-research.md`) surveyed
five tools. The defaults combine qmd's 6 dev-noise patterns + Foam's 18
patterns + wiki-specific patterns surfaced by the user's real wikis:

```toml
[wiki]
exclude_dirs = [
  # Standard dev-project noise (qmd + Foam union)
  "node_modules", ".git", "target", "dist", "build",
  ".next", ".cache", ".turbo", ".venv", "venv", "env",
  "__pycache__", ".idea", ".vscode",
  # Wiki-specific noise (from real-wiki smoke test)
  ".opencode", ".claude", ".mavis", ".harness",
  ".serena", ".principled", ".swe-bench",
]
```

Applied via `walkdir::WalkDir::filter_entry` on `pages_dir`. Glob arrays
relative to the workspace root, matching the qmd + Foam convention. Patterns
evaluated with `**/X/**` semantics (a directory named `X` is skipped at any
depth, plus its contents).

### Acceptance gate (v0.3.26)

- [ ] `cargo test --test flat_layout_test` passes
- [ ] `cargo test --test lint_cli_test` passes (includes renamed
      `frontmatter-yaml-parse` test)
- [ ] `cargo test --test exclude_dirs_test` (new) passes
- [ ] All 5 real wikis in `~/.agents/wiki-root.toml` show realistic page counts
      in `wiki ls --pages` (1411, 127, 14333, 339, etc. — matches v0.3.25
      baseline)
- [ ] `wiki init` in a fresh directory scaffolds the legacy `wiki/` subdir
      AND accepts `--flat` for the new flat default
- [ ] `wiki tree` against `minimax` (1411 pages) completes in < 5 seconds
- [ ] clippy `-D warnings` clean on stable + MSRV 1.88 with `--locked`
- [ ] fmt clean
- [ ] CHANGELOG.md updated with `## [0.3.26]` section

## Dimension 2: Stability (advanced by v0.4.0)

### Why this is the second dimension

Once correctness is locked, the next question is "what promise are we making
about not breaking things?" Downstream consumers (marketplace agents, the
LSP/MCP server, future contributors) need documented stable surfaces.

### v0.4.0 scope

**Stability work:**
- JSON Schema for **every** `--json` output, not just `doctor`. Specifically:
  - `wiki ls --json`
  - `wiki tree --json`
  - `wiki status --json`
  - `wiki config show-effective --json`
  - `wiki embed --json`
  - `wiki search --json`
  - `wiki query --json`
  - `wiki doctor --json` (already exists, regenerated under SSoT)
  - `wiki init --json`, `wiki ingest --json`, `wiki build --json` if they
    accept `--json` (or document that they don't)
- `docs/API_STABILITY.md` — declares the stable surface, the breaking-change
  policy, and the deprecation timeline convention.
- `tests/json_schema_drift_test.rs` — runs every command with `--json`,
  validates against the auto-generated schema. Currently only `doctor` is
  tested.
- Update README + AGENTS.md to link to `API_STABILITY.md`.

**CI maturity (folded in):**
- SHA-pin all `uses:` lines in `.github/workflows/*.yml` (deferred M4 from
  v0.3.23 self-critic).
- Add `cargo deny check` to CI (license + advisory).
- Add macOS runner to `fmt-clippy-test` (currently Linux-only — the user's
  dev box is macOS but the linter pass is Linux).

**Dep hygiene (folded in):**
- Land Dependabot PR #5 (thiserror 1→2, toml 0.8→1.1, notify 6→8).
- Each major bump in its own commit so bisecting is clean.
- Fix Cargo.lock Linux/macOS parity (regenerate on Linux runner, commit).

### Breaking-change policy (the document to write)

Stated in `docs/API_STABILITY.md`:

> A change is **breaking** if it requires an external consumer to modify
> their code, script, or muscle memory. Examples:
>
> - Removing a CLI subcommand or flag.
> - Renaming a CLI subcommand or flag.
> - Changing the shape of a `--json` output.
> - Renaming or removing a config key (in `~/.llmwiki-cli/config.toml`,
>   `<workspace>/.llmwiki-cli/config.toml`, or `wiki-root.toml`).
> - Changing the default value of a config key.
> - Changing the semantics of a config key (e.g. `pages_dir = ""` now
>   meaning something different).
>
> Non-breaking changes (allowed in minor versions):
>
> - Adding a new CLI subcommand or flag.
> - Adding a new field to a `--json` output (consumers ignore unknown fields).
> - Adding a new optional config key with a sensible default.
> - Changing a CLI error message text (consumers don't parse stderr).
> - Fixing a bug that the docs incorrectly described as the "intended"
>   behavior (e.g. the v0.3.25 flat-layout discovery fix).
>
> When a breaking change is unavoidable, it MUST be:
>
> 1. Marked `[BREAKING]` in the CHANGELOG entry.
> 2. Accompanied by a one-line `## Migration` section explaining the
>    one-step change users need to make.
> 3. Released as a **major** version bump (e.g. v0.x.y → v0.(x+1).0).

### Acceptance gate (v0.4.0)

- [ ] Every `--json` command has a JSON Schema file checked into
      `marketplace/skills/wiki/SETUP/references/` (or equivalent)
- [ ] `tests/json_schema_drift_test.rs` passes (every command's `--json`
      output validates against its schema)
- [ ] `docs/API_STABILITY.md` exists with the breaking-change policy
      text quoted above
- [ ] All workflow actions SHA-pinned
- [ ] `cargo deny check` passes (no license violations, no advisories)
- [ ] Dependabot PR #5 closed (either merged or split into per-dep PRs)
- [ ] `cargo check --locked` passes on both Linux and macOS runners
- [ ] 264/264+ tests pass (no regressions, plus new schema tests)
- [ ] clippy `-D warnings` clean on stable + MSRV 1.88 with `--locked`
- [ ] CHANGELOG.md updated

## Dimension 3: Real-world validation (advanced by v1.0.0)

### Why this is the third dimension

The project has been dogfooded by one user (the author) on five wikis. That
catches a lot, but it's not the same as an external user encountering the
install instructions cold. A v1.0 tag is a public claim of "this works for
people who aren't me."

### v1.0.0 scope

**Soak period**: v0.4.0 sits for ≥ 1 week. Any P0 bug filed becomes a
v0.4.1 patch release; the v1.0.0 tag waits until the soak is clean.

**External user trial**: at least one user **other than the author** runs
v0.5.0+ against a non-trivial wiki (≥ 100 pages, ≥ 1 ingest, ≥ 1 embed
batch, ≥ 5 searches) for ≥ 1 week without filing a P0 bug. The trial
can be informal (a colleague, a friend, a GitHub issue participant) — no
contract needed.

**P0 bug definition** (for both the soak period and the external trial):
a bug that prevents the CLI from being installed, init'd, or used for
its primary operations (ingest, embed, search, query, lint, doctor) on
a freshly-set-up system. Anything less severe (cosmetic, edge-case
config, obscure platform quirk) is P1 and does not block v1.0.

**Failure-mode coverage**: expand wiremock fixtures for the failure modes
that haven't been tested yet:

- NIM 401 (bad API key)
- NIM 429 (rate limit) — confirm `wiki embed` retries correctly
- NIM 500 (server error) — confirm `wiki embed` retries correctly
- NIM timeout — confirm error message is actionable
- Network unreachable — confirm `wiki doctor` reports it gracefully

These can land in v0.4.0 if time permits; otherwise they go into v0.5.0
which becomes the soak target.

### Acceptance gate (v1.0.0)

- [ ] v0.4.0 has been soak-tested for ≥ 1 week with no P0 bugs filed
- [ ] At least one external user has completed a ≥ 1-week trial without
      filing a P0 bug
- [ ] All 5 failure-mode wiremock fixtures pass
- [ ] README, AGENTS.md, and marketplace skill SKILL.md all reflect the
      current behavior (no `wiki/` examples in flat-layout contexts)
- [ ] `Cargo.toml` version bumped to `1.0.0`
- [ ] CHANGELOG.md updated with `## [1.0.0] - YYYY-MM-DD` heading
- [ ] GitHub release published with release notes copied from CHANGELOG
- [ ] crates.io publish (if applicable — currently has a publish workflow)
- [ ] README badge updated: `[![Version](https://img.shields.io/crates/v/llmwiki-cli.svg)](...)`
      now points at 1.0.0

## Cross-cutting concerns

### Test discipline (applies to every release)

- Every behavior change has a regression test.
- Every bug fix has a test that fails before the fix.
- Wiremock for any test that hits the network.
- Real-wiki smoke test is mandatory before any tagged release (already
  in AGENTS.md since v0.3.25).

### Branch protection workflow (solo dev)

Per self-critic M5: the solo dev workflow requires temporarily setting
`required_approving_review_count: 0` to merge own PRs. This is a
process-level issue, not a code issue. Options:

- Add the user's own GitHub handle to `.github/CODEOWNERS` so PRs
  automatically request their own review.
- Reconfigure branch protection to allow admin bypass for PRs from
  CODEOWNERS.

This is a low-priority follow-up; do not block v1.0 on it.

### CHANGELOG length (per self-critic L3)

The CHANGELOG is 988 lines. v0.4.0 splits it:

```
CHANGELOG.md              ← index, links to per-version files
CHANGELOG/0.3.x.md         ← v0.3.0 through v0.3.99
CHANGELOG/0.4.x.md         ← (will be created at v0.4.0)
CHANGELOG/1.0.x.md         ← (will be created at v1.0.0)
```

`tests/changelog_check.sh` updated to verify either the top-level heading
(in current CHANGELOG.md) OR the per-version file heading.

## Open questions (deferred — not blocking v0.3.26)

These are questions the v1.0 framework surfaces but does not answer:

1. **What is a "non-trivial wiki" for the external trial?** Spec it during
   v0.4.0 → v1.0.0 transition. Suggested: ≥ 100 markdown pages, ≥ 1 raw
   source ingested, ≥ 1 embed batch run, ≥ 5 searches executed.
2. **Who is the external user?** Decide during v0.4.0 → v1.0.0 transition.
   Could be a colleague, a friend, a GitHub issue participant, or a
   Reddit/HackerNews responder.
3. **What about Windows?** The install.sh / install.ps1 exist but CI is
   Linux-only. Windows validation is out of scope for this spec but
   should be raised as a v1.1 follow-up if/when Windows users appear.
4. **What about the deprecated `wiki` binary name?** The v0.3.0 CHANGELOG
   documents the rename but the old name may still be referenced in
   external scripts. v0.4.0 could add a `wiki` shim that prints a
   deprecation warning, but this is a separate decision.

## Immediate next step

Per the brainstorming flow, this spec transitions to an implementation plan
via the writing-plans skill. The next concrete deliverable is:

1. **v0.3.26 implementation plan** — covers H1+H2+M1-M4+L1 from this spec
   plus the "commit research docs" task. The plan lives in
   `docs/superpowers/plans/2026-06-24-v0.3.26-correctness.md`.

2. After v0.3.26 ships, **v0.4.0 implementation plan** — covers Dimension 2
   (stability + CI + dep hygiene). Plan will be a separate spec
   `docs/superpowers/specs/2026-MM-DD-v0.4-stability-design.md`.

3. v1.0.0 has no implementation plan — it's a release gate, not a feature
   release.

## Acceptance for THIS spec

This spec is accepted when:

- The 3-dimension framework is approved.
- The 3-release roadmap (v0.3.26 → v0.4.0 → v1.0.0) is approved.
- The v0.3.26 scope (the immediate next step) is approved as a unit of work
  that can be planned and shipped independently.
- The deferred questions are acknowledged but not blocking.

Once accepted, the writing-plans skill is invoked to create the v0.3.26
implementation plan.

---

*This spec is the project's definition of "done". It does not introduce
new commands, flags, or config keys — those live in per-release design
specs. It defines the criteria that those releases must collectively meet.*