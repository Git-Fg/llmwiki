# llmwiki — Pushy Naming & Description Redesign

**Date:** 2026-06-24
**Status:** Approved (Approach A)
**Scope:** User-facing brand surfaces only. No binary, crate, or repo rename.

## Motivation

The current skill name is `wiki` — a single generic word that collides with
Anthropic's official `wiki` skill and dozens of other tools. Agents and humans
can't tell our wiki apart from any other. The Cargo description is descriptive
but doesn't load triggers for AI skill routing. The CLI help first line is
factual but inert — it doesn't sell the value.

This spec renames the user-facing surfaces to **`llmwiki`** (matching the repo
URL and Cargo crate keyword) and rewrites descriptions to be **"pushy"** in
two complementary ways:

1. **AI trigger phrases** for the skill description — front-load the list of
   situations that should cause an agent to load the skill.
2. **Brand voice** for the Cargo description and CLI tagline — confident,
   short, memorable.

## Brand tagline (used in 3 places)

> **Your LLM's persistent memory.**

Short. Confident. Repeats across Cargo description, CLI `--help` first line,
and skill H1 subtitle. Easy to remember, easy to say.

## Surface-by-surface changes

### 1. Skill `name:` field

- **Before:** `wiki`
- **After:** `llmwiki`
- **File:** `skills/SKILL.md`
- **Impact:** Skill directory becomes `~/.agents/skills/llmwiki/`. Anthropic's
  official `wiki` skill is no longer shadowed.

### 2. Skill `description:` (the trigger block)

- **Before:** "Personal Karpathy-style LLM Wiki. Install the CLI..."
- **After:** (full text below)
- **File:** `skills/SKILL.md` frontmatter

```yaml
description: |
  Your LLM's persistent memory in plain markdown — Karpathy-style wiki with
  semantic search, citations, and no database. Load this skill whenever the
  user mentions ANY of: wiki, notes, knowledge base, second brain, RAG,
  citations, ingest, search-by-meaning, "find my notes on X", "I keep
  forgetting", "set up on a new device", "pick a different model",
  "lint the wiki", "sync across machines", or wants to embed markdown for
  semantic search.

  Install the CLI once: cargo install llmwiki-cli --locked (or
  curl -LsSf https://github.com/Git-Fg/llmwiki/releases/latest/download/install.sh | sh).
  Then ALWAYS start with `llmwiki-cli --help` — it shows the active wiki
  and full command surface. Use `llmwiki-cli skill list` to enumerate
  sub-skills and `llmwiki-cli skill get <topic>` to load one on demand.
  Sub-skills are version-matched with the binary, served on demand —
  never write commands from memory.
```

### 3. Skill H1 (the title)

- **Before:** `# wiki — Karpathy-style LLM Wiki`
- **After:** `# llmwiki — your LLM's persistent memory`
- **File:** `skills/SKILL.md`

### 4. CLI help `about` and `long_about`

- **Before (`about`):** `"Karpathy-style LLM Wiki"`
- **After (`about`):** `"Your LLM's persistent memory"`

- **Before (`long_about`):** "Manage a personal LLM Wiki: markdown pages + JSONL embeddings, no database. Single binary; no server; works offline against local files. NVIDIA NIM provides embeddings + optional LLM for RAG queries."
- **After (`long_about`):**

```
Your LLM's persistent memory in plain markdown — Karpathy-style wiki with
semantic search, citations, and no database. Single Rust binary; no server;
works offline against local files. NVIDIA NIM provides embeddings + optional
LLM for RAG queries.

Quick start:
  llmwiki-cli doctor                        # verify config + NIM
  llmwiki-cli skill list                    # discover sub-skills
  llmwiki-cli <command> --help              # full flag reference

For AI agents: start with `llmwiki-cli skill get <topic>`.
```

- **File:** `src/cli/mod.rs` (the `Cli` struct's `#[command(...)]` attributes)

### 5. Cargo `package.description`

- **Before:** `"Karpathy-style LLM Wiki CLI — manage multiple wikis, embed pages, search semantically"`
- **After:** `"Your LLM's persistent memory in plain markdown. Multiple wikis, semantic search, citations, no database. Single Rust binary, NVIDIA NIM embeddings, agent-skill native."`
- **File:** `Cargo.toml`

### 6. Cargo `keywords`

- **Before:** `["llmwiki", "karpathy-wiki", "wiki", "knowledge-base", "rag"]`
- **After:** `["llmwiki", "llm-memory", "second-brain", "karpathy-wiki", "wiki", "knowledge-base", "rag", "semantic-search"]`
- **File:** `Cargo.toml`

### 7. README H1

- **Before:** `# wiki`
- **After:** `# llmwiki`
- **File:** `README.md`

### 8. Sub-skill topic names (9 files)

- **Before:** `wiki-config`, `wiki-ingest`, `wiki-lint`, `wiki-models`, `wiki-query`, `wiki-search`, `wiki-setup`, `wiki-sync`, `wiki-troubleshooting`
- **After (canonical):** `llmwiki-config`, `llmwiki-ingest`, `llmwiki-lint`, `llmwiki-models`, `llmwiki-query`, `llmwiki-search`, `llmwiki-setup`, `llmwiki-sync`, `llmwiki-troubleshooting`
- **Old names:** accepted as deprecated aliases. `skill get wiki-search` prints
  a one-line deprecation notice and forwards to `llmwiki-search`.
- **Files:**
  - `src/skills/data/wiki-{config,ingest,lint,models,query,search,setup,sync,troubleshooting}.md` → rename to `llmwiki-*.md`
  - Each file: frontmatter `name:` field + H1 line updated
  - Cross-references between sub-skills updated to new names
  - `src/skills/mod.rs::normalize_topic()` accepts both prefixes (`wiki-` and `llmwiki-`); canonicalizes to `llmwiki-`
  - `src/skills/mod.rs::list_skills()` filter accepts either prefix
  - `src/skills/mod.rs::tests`:
    - `LEAK_MARKERS` array updated (markers must match new frontmatter names)
    - `hub_loads()` updates the `name: llmwiki` assertion
    - `find_skill_accepts_full_and_short_names()` adds `llmwiki-search` case
    - `normalize_topic_handles_prefix_and_case()` adds `llmwiki-search` → `llmwiki-search` case and `wiki-search` → `llmwiki-search` (deprecation)
    - `list_skills_returns_wiki_prefixed_files()` renamed to `list_skills_returns_canonical_prefixed_files` and updated to expect `llmwiki-`

### 9. `after_help` in `src/cli/mod.rs`

- **Before:** references "sub-skills" without naming them
- **After:** same content; no functional change required (sub-skill mentions are
  via `skill list` which auto-discovers)

## What does NOT change

| Surface | Reason |
|---|---|
| Binary name `llmwiki-cli` | Already contains `llmwiki`; rename would break `cargo install`, install scripts, every README |
| Cargo `package.name = "llmwiki-cli"` | Same — rename breaks `cargo install llmwiki-cli` |
| GitHub repo URL `github.com/Git-Fg/llmwiki` | Rename breaks every link in the wild |
| CLI subcommand names (`doctor`, `config`, `search`, etc.) | User-facing verb-noun commands — renaming breaks muscle memory |
| Resolution chain order | Behavior, not branding |
| Sub-skill body content | Workflows don't need rephrasing |

## Risks and mitigations

| Risk | Mitigation |
|---|---|
| Sub-skill topic rename breaks muscle memory | Old names accepted as deprecated aliases with a one-line notice |
| `LEAK_MARKERS` array drift if frontmatter names change but tests don't | Update markers in same PR; add a second test (`leak_markers_are_actually_present_in_sub_skills`) already exists as a meta-guard |
| README H1 change shifts doc anchors | None expected (single H1) |
| Brand tagline "persistent memory" too generic / not specific enough | One-line revision possible in next release; tagline lives in 3 places, not deep in code |
| New `llmwiki` skill name collides with another existing skill on the user's machine | Unlikely (Anthropic's is just `wiki`); if it happens, user can choose directory on install |
| Internal `wiki-` references missed (345 occurrences in src/) | Use `grep -rn "wiki-" src/` after edits to find stragglers; the embedded bundle paths in `list_skills` are the high-risk sites |

## Test plan

- **Unit tests** (`cargo test`):
  - `src/skills/mod.rs` tests updated for new prefix (5 test fns + LEAK_MARKERS)
  - Add 2 new tests:
    - `normalize_topic_accepts_legacy_wiki_prefix` — `wiki-search` → `llmwiki-search`
    - `find_skill_legacy_alias_still_works` — `wiki-search` returns the renamed content
- **Integration tests**:
  - `tests/skill_test.rs` — update any topic-name assertions to new canonical names; add a test that the legacy alias still resolves
  - `tests/install_skill_test.rs` — verify installed path is `~/.agents/skills/llmwiki/` not `~/.agents/skills/wiki/`
- **Manual smoke**:
  - `llmwiki-cli --help` shows new tagline
  - `llmwiki-cli skill list` shows new topic names
  - `llmwiki-cli skill get wiki-search` (legacy) prints deprecation notice and serves content
  - `llmwiki-cli skill get llmwiki-search` (canonical) works without notice
  - `llmwiki-cli install-skill --global` writes to `~/.agents/skills/llmwiki/`
- **Pre-release smoke** (mandatory per AGENTS.md): run the 6-step real-wiki smoke test on `--wiki minimax` before tagging v0.3.36.

## Open items (deferred)

- Whether the Cargo `categories` should change (currently `["command-line-utilities", "development-tools"]`); no clear improvement, defer.
- Whether the GitHub repo description (the one-liner on the repo page) should change; orthogonal to this spec, defer.
- Whether the `llmwiki-cli doctor` first line should change (currently factual "One-shot health check"); defer.

## Implementation order (for the plan)

1. **Sub-skill rename** (the largest refactor) — `src/skills/data/*.md` files, `src/skills/mod.rs`, tests
2. **Hub SKILL.md** — `skills/SKILL.md` frontmatter + H1
3. **Cargo description + keywords** — `Cargo.toml`
4. **CLI help `about`/`long_about`** — `src/cli/mod.rs`
5. **README H1** — `README.md`
6. **CHANGELOG entry** — `CHANGELOG.md` v0.3.36 section
7. **Run full test suite + pre-release smoke**
8. **Bump version to v0.3.36 in `Cargo.toml`**
9. **Tag v0.3.36 + push (after user confirmation per AGENTS.md)**