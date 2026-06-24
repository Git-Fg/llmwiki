# llmwiki Pushy Naming Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rename user-facing surfaces to `llmwiki` and rewrite descriptions to be "pushy" (AI trigger phrases in skill description, brand tagline in Cargo + CLI). Hard cut — no deprecation aliases.

**Architecture:** TDD where logic changes (skill topic routing); direct edits where it's pure content rewrite. The 9 sub-skill files are renamed wholesale (`wiki-X.md` → `llmwiki-X.md`) and their frontmatter + H1 updated; cross-references between sub-skills updated in the same pass. The routing code in `src/skills/mod.rs::normalize_topic` becomes stricter (accepts only `llmwiki-` prefix) with a guard test that legacy `wiki-search` returns "unknown topic".

**Tech Stack:** Rust 2021, clap 4, rust-embed 8, Markdown frontmatter (YAML), cargo test (320 tests as of v0.3.35).

---

## File Structure

**Create** (9 files):
- `src/skills/data/llmwiki-config.md`
- `src/skills/data/llmwiki-ingest.md`
- `src/skills/data/llmwiki-lint.md`
- `src/skills/data/llmwiki-models.md`
- `src/skills/data/llmwiki-query.md`
- `src/skills/data/llmwiki-search.md`
- `src/skills/data/llmwiki-setup.md`
- `src/skills/data/llmwiki-sync.md`
- `src/skills/data/llmwiki-troubleshooting.md`

**Delete** (9 files):
- `src/skills/data/wiki-{config,ingest,lint,models,query,search,setup,sync,troubleshooting}.md`

**Modify** (9 files):
- `skills/SKILL.md` — frontmatter `name:` + `description:`, H1
- `Cargo.toml` — `description` + `keywords`
- `src/cli/mod.rs` — `Cli` struct `about` + `long_about`
- `src/skills/mod.rs` — `normalize_topic`, `list_skills` filter, tests, `LEAK_MARKERS`
- `tests/skill_test.rs` — topic-name assertions
- `tests/install_skill_test.rs` — install path assertions + new guard test
- `README.md` — H1
- `CHANGELOG.md` — v0.3.36 entry
- `Cargo.toml` (separately) — version bump

---

## Task 1: Add failing test for strict topic routing

**Files:**
- Modify: `src/skills/mod.rs` (add test at end of `tests` module)

- [ ] **Step 1: Write the failing test**

Append to `src/skills/mod.rs` `tests` module:

```rust
#[test]
fn normalize_topic_rejects_legacy_wiki_prefix() {
    // Hard cut: legacy `wiki-X` names are NOT supported aliases.
    // They normalize to themselves (unknown topic) so the caller can
    // produce a single "unknown topic" error message.
    assert_eq!(normalize_topic("wiki-search"), "wiki-search");
    assert_eq!(normalize_topic("wiki-config"), "wiki-config");
}

#[test]
fn find_skill_rejects_legacy_wiki_names() {
    // Guard against accidental alias re-introduction.
    assert!(find_skill("wiki-search").is_none());
    assert!(find_skill("wiki-config").is_none());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib normalize_topic_rejects_legacy_wiki_prefix find_skill_rejects_legacy_wiki_names 2>&1 | tail -20`
Expected: First test PASSES (current behavior already returns input unchanged when no `wiki-` prefix normalization happens — but second test FAILS because `wiki-search.md` still exists in the bundle). The first test is a regression guard for later; the second is the actual fail-now signal.

Actually the first test will pass currently (it asserts `normalize_topic("wiki-search") == "wiki-search"`, which is what the current code does because the existing logic only handles the bare-topic case). The real failing test is `find_skill_rejects_legacy_wiki_names`. Keep both — first becomes meaningful after Task 3 deletes the files.

- [ ] **Step 3: Commit**

```bash
git add src/skills/mod.rs
git commit -m "test: add failing guard tests for legacy wiki- topic names"
```

---

## Task 2: Rename sub-skill files (wiki-X.md → llmwiki-X.md)

**Files:**
- Move: `src/skills/data/wiki-{config,ingest,lint,models,query,search,setup,sync,troubleshooting}.md` → `src/skills/data/llmwiki-{config,ingest,lint,models,query,search,setup,sync,troubleshooting}.md`

- [ ] **Step 1: Rename files via git mv**

```bash
cd src/skills/data
for name in config ingest lint models query search setup sync troubleshooting; do
  git mv "wiki-${name}.md" "llmwiki-${name}.md"
done
cd -
```

- [ ] **Step 2: Verify renames**

Run: `ls src/skills/data/`
Expected: 9 files, all named `llmwiki-*.md`, no `wiki-*.md` files remain.

- [ ] **Step 3: Commit**

```bash
git add src/skills/data/
git commit -m "refactor(skills): rename sub-skill files wiki-X.md → llmwiki-X.md"
```

---

## Task 3: Update sub-skill frontmatter `name:` and H1

**Files:**
- Modify: each of the 9 renamed files in `src/skills/data/llmwiki-*.md`

- [ ] **Step 1: For each file, replace `name: wiki-X` with `name: llmwiki-X`**

Run from project root:

```bash
for name in config ingest lint models query search setup sync troubleshooting; do
  sed -i '' "s/^name: wiki-${name}$/name: llmwiki-${name}/" "src/skills/data/llmwiki-${name}.md"
done
```

**WARNING:** Do not use `sed -i` per the project's `~/.agents/AGENTS.md` rule ("Never use `sed` for edits"). Instead, use `Edit` tool calls — one per file — OR delegate to a single `Agent` with `subagent_type: "coder"` and `prompt` containing the exact replacement pattern.

Recommended: use `Edit` tool, one call per file. The pattern is identical in every file:

```yaml
# Before (line 2):
name: wiki-search
# After:
name: llmwiki-search
```

Apply to all 9 files: `llmwiki-config.md`, `llmwiki-ingest.md`, `llmwiki-lint.md`, `llmwiki-models.md`, `llmwiki-query.md`, `llmwiki-search.md`, `llmwiki-setup.md`, `llmwiki-sync.md`, `llmwiki-troubleshooting.md`.

- [ ] **Step 2: For each file, replace `# wiki-X` H1 with `# llmwiki-X`**

Use `Edit` tool on each file. Pattern per file (line ~12):

```markdown
# Before:
# wiki-search
# After:
# llmwiki-search
```

Apply to all 9 files.

- [ ] **Step 3: Verify**

Run: `grep -l "^name: wiki-" src/skills/data/*.md`
Expected: no output.

Run: `grep -l "^name: llmwiki-" src/skills/data/*.md`
Expected: 9 files listed.

Run: `grep -l "^# wiki-" src/skills/data/*.md`
Expected: no output.

- [ ] **Step 4: Commit**

```bash
git add src/skills/data/
git commit -m "refactor(skills): rename frontmatter name + H1 to llmwiki-X"
```

---

## Task 4: Update cross-references between sub-skills

**Files:**
- Modify: each of the 9 renamed files (references to other sub-skills like "see wiki-search")

- [ ] **Step 1: Find all cross-references**

Run: `grep -nE "\bwiki-(config|ingest|lint|models|query|search|setup|sync|troubleshooting)\b" src/skills/data/*.md`
Expected: a list of `(file:line:match)` lines. Most references are in "See also" sections.

- [ ] **Step 2: Replace each occurrence with `llmwiki-X`**

For each file listed, use `Edit` tool with `replace_all=true` (or individual edits if patterns differ). The replacement is `wiki-X` → `llmwiki-X` for each known sub-skill name.

Example for `src/skills/data/llmwiki-sync.md`:

```markdown
# Before (line ~40):
- `wiki-setup` — first-run install / bootstrap
- `wiki-troubleshooting` — when sync breaks (merge conflicts, missing aliases)

# After:
- `llmwiki-setup` — first-run install / bootstrap
- `llmwiki-troubleshooting` — when sync breaks (merge conflicts, missing aliases)
```

- [ ] **Step 3: Verify no stale `wiki-` references remain in sub-skills**

Run: `grep -nE "\bwiki-(config|ingest|lint|models|query|search|setup|sync|troubleshooting)\b" src/skills/data/*.md`
Expected: no output.

- [ ] **Step 4: Commit**

```bash
git add src/skills/data/
git commit -m "refactor(skills): update cross-references wiki-X → llmwiki-X"
```

---

## Task 5: Update `src/skills/mod.rs::normalize_topic` and `list_skills` filter

**Files:**
- Modify: `src/skills/mod.rs`

- [ ] **Step 1: Update `normalize_topic` to only accept `llmwiki-` prefix**

In `src/skills/mod.rs`, replace the `normalize_topic` function (lines ~66-73):

```rust
/// Resolves a topic name to its file stem. Accepts ONLY the canonical
/// `llmwiki-X` prefix; bare topic names get the prefix added. Legacy
/// `wiki-X` names are NOT supported (hard cut at v0.3.36) — they pass
/// through unchanged so the caller surfaces a single "unknown topic"
/// error.
fn normalize_topic(name: &str) -> String {
    let lower = name.trim().to_ascii_lowercase();
    if lower.starts_with("llmwiki-") {
        lower
    } else if lower.starts_with("wiki-") {
        // Legacy alias: pass through unchanged so it looks like any
        // other unknown topic to the caller.
        lower
    } else {
        format!("llmwiki-{lower}")
    }
}
```

- [ ] **Step 2: Update `list_skills` filter to only accept `llmwiki-` prefix**

Replace lines ~47-50:

```rust
// Sub-skills are `llmwiki-{name}.md` flat files in the bundle.
if !path.starts_with("llmwiki-") || !path.ends_with(".md") {
    return None;
}
```

- [ ] **Step 3: Run existing tests — most should pass**

Run: `cargo test --lib skills 2>&1 | tail -30`
Expected: `normalize_topic_handles_prefix_and_case`, `find_skill_accepts_full_and_short_names`, and `list_skills_returns_wiki_prefixed_files` (renamed below) FAIL because they assert old `wiki-` behavior.

- [ ] **Step 4: Update test assertions in `src/skills/mod.rs`**

The `tests` module has these functions to update:

`hub_loads` — change `name: wiki` to `name: llmwiki` (line ~95):

```rust
assert!(content.contains("name: llmwiki"));
```

`find_skill_accepts_full_and_short_names` — replace `wiki-search` with `llmwiki-search`:

```rust
#[test]
fn find_skill_accepts_full_and_short_names() {
    assert!(find_skill("llmwiki-search").is_some());
    assert!(find_skill("search").is_some());
    assert!(find_skill("Search").is_some());
    assert!(find_skill("nonexistent").is_none());
}
```

`list_skills_returns_wiki_prefixed_files` — rename and update assertion:

```rust
#[test]
fn list_skills_returns_llmwiki_prefixed_files() {
    let skills = list_skills();
    assert!(!skills.is_empty(), "no sub-skills found in bundle");
    for (stem, lines) in &skills {
        assert!(stem.starts_with("llmwiki-"), "unexpected stem {stem}");
        assert!(*lines > 0, "sub-skill {stem} has 0 lines");
    }
}
```

`normalize_topic_handles_prefix_and_case` — update to expect `llmwiki-`:

```rust
#[test]
fn normalize_topic_handles_prefix_and_case() {
    assert_eq!(normalize_topic("search"), "llmwiki-search");
    assert_eq!(normalize_topic("llmwiki-search"), "llmwiki-search");
    assert_eq!(normalize_topic("SEARCH"), "llmwiki-search");
    assert_eq!(normalize_topic("  query  "), "llmwiki-query");
}
```

`hub_does_not_contain_sub_skill_bodies_inline` and `leak_markers_are_actually_present_in_sub_skills` — update `LEAK_MARKERS` array (lines ~135-142):

```rust
const LEAK_MARKERS: &[&str] = &[
    "llmwiki-search",        // sub-skill frontmatter + body refs (4 files)
    "llmwiki-config",        // sub-skill frontmatter + body refs (4 files)
    "llmwiki-cli embed",     // llmwiki-embed sub-skill workflow (6 files)
    "llmwiki-cli ingest",    // llmwiki-ingest sub-skill workflow (1 file)
    "## Workflow",           // common sub-skill section header (7 files)
    "Do NOT use for:",       // sub-skill frontmatter contrast line (9 files)
];
```

- [ ] **Step 5: Run tests — all should pass**

Run: `cargo test --lib skills 2>&1 | tail -15`
Expected: `test result: ok. N passed; 0 failed` (the new tests from Task 1 + updated tests from Task 5).

- [ ] **Step 6: Commit**

```bash
git add src/skills/mod.rs
git commit -m "refactor(skills): strict topic routing accepts only llmwiki- prefix"
```

---

## Task 6: Update hub SKILL.md (frontmatter + H1)

**Files:**
- Modify: `skills/SKILL.md`

- [ ] **Step 1: Update frontmatter `name:`**

In `skills/SKILL.md` line 2, change:

```yaml
# Before:
name: wiki
# After:
name: llmwiki
```

- [ ] **Step 2: Update frontmatter `description:`**

Replace the entire `description: |` block (lines 3-9) with:

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

Also update the `metadata:` line at the bottom (line ~17):

```yaml
# Before:
install-skill: npx skills add Git-Fg/llmwiki
# After:
install-skill: npx skills add Git-Fg/llmwiki
```

(`install-skill` URL does not change — repo URL is unchanged.)

- [ ] **Step 3: Update H1 (line ~22)**

```markdown
# Before:
# wiki — Karpathy-style LLM Wiki
# After:
# llmwiki — your LLM's persistent memory
```

- [ ] **Step 4: Update body text — replace `wiki` brand mentions with `llmwiki`**

Within `skills/SKILL.md` body (lines 22+), replace any remaining `# wiki` brand mentions with `# llmwiki`. Specifically check the H2 sections like `## Multi-wiki quick reference` — that one stays (`multi-wiki` is a noun, not a brand). Keep `# llmwiki` as the only brand reference.

- [ ] **Step 5: Verify**

Run: `head -25 skills/SKILL.md`
Expected: frontmatter has `name: llmwiki`, pushy description, H1 says `# llmwiki — your LLM's persistent memory`.

- [ ] **Step 6: Commit**

```bash
git add skills/SKILL.md
git commit -m "feat(skill): rename hub to llmwiki with pushy trigger description"
```

---

## Task 7: Update Cargo.toml (description + keywords)

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Update `description`**

In `Cargo.toml` line 4:

```toml
# Before:
description = "Karpathy-style LLM Wiki CLI — manage multiple wikis, embed pages, search semantically"
# After:
description = "Your LLM's persistent memory in plain markdown. Multiple wikis, semantic search, citations, no database. Single Rust binary, NVIDIA NIM embeddings, agent-skill native."
```

- [ ] **Step 2: Update `keywords`**

In `Cargo.toml` line 9:

```toml
# Before:
keywords = ["llmwiki", "karpathy-wiki", "wiki", "knowledge-base", "rag"]
# After:
keywords = ["llmwiki", "llm-memory", "second-brain", "karpathy-wiki", "wiki", "knowledge-base", "rag", "semantic-search"]
```

- [ ] **Step 3: Verify**

Run: `head -15 Cargo.toml`
Expected: new description + 8 keywords (was 5).

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml
git commit -m "feat(cargo): pushy description + extended keywords for discoverability"
```

---

## Task 8: Update CLI `about` and `long_about`

**Files:**
- Modify: `src/cli/mod.rs` (the `Cli` struct's `#[command(...)]` block, lines ~23-37)

- [ ] **Step 1: Update `about`**

In `src/cli/mod.rs` line 27:

```rust
// Before:
about = "Karpathy-style LLM Wiki",
// After:
about = "Your LLM's persistent memory",
```

- [ ] **Step 2: Update `long_about`**

Replace lines 28-36:

```rust
// Before:
long_about = "Manage a personal LLM Wiki: markdown pages + JSONL embeddings, no database.\n\
              Single binary; no server; works offline against local files.\n\
              NVIDIA NIM provides embeddings + optional LLM for RAG queries.\n\
              \n\
              Quick start:\n  \
                llmwiki-cli doctor                        # verify config + NIM\n  \
                llmwiki-cli skill list                    # discover sub-skills\n  \
                llmwiki-cli <command> --help              # full flag reference\n\n\
              For AI agents: start with `llmwiki-cli skill get <topic>`.",

// After:
long_about = "Your LLM's persistent memory in plain markdown — Karpathy-style wiki with\n\
              semantic search, citations, and no database. Single Rust binary; no server;\n\
              works offline against local files. NVIDIA NIM provides embeddings + optional\n\
              LLM for RAG queries.\n\
              \n\
              Quick start:\n  \
                llmwiki-cli doctor                        # verify config + NIM\n  \
                llmwiki-cli skill list                    # discover sub-skills\n  \
                llmwiki-cli <command> --help              # full flag reference\n\n\
              For AI agents: start with `llmwiki-cli skill get <topic>`.",
```

- [ ] **Step 3: Verify**

Run: `cargo build && llmwiki-cli --help | head -3`
Expected: first line says `Your LLM's persistent memory in plain markdown — Karpathy-style wiki with`.

- [ ] **Step 4: Commit**

```bash
git add src/cli/mod.rs
git commit -m "feat(cli): pushy about + long_about tagline"
```

---

## Task 9: Update README.md H1

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update H1**

In `README.md` line 1:

```markdown
# Before:
# wiki
# After:
# llmwiki
```

- [ ] **Step 2: Audit rest of README for `# wiki` brand mentions**

Run: `grep -n "^# wiki\| wiki —\|`wiki`" README.md`
Expected: only the H1 line. If other matches appear, update them similarly to `# llmwiki`.

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs(readme): rename H1 # wiki → # llmwiki"
```

---

## Task 10: Update integration tests for new paths/topics

**Files:**
- Modify: `tests/skill_test.rs`
- Modify: `tests/install_skill_test.rs`

- [ ] **Step 1: Audit existing topic-name assertions**

Run: `grep -rn "wiki-search\|wiki-config\|wiki-ingest\|wiki-lint\|wiki-models\|wiki-query\|wiki-setup\|wiki-sync\|wiki-troubleshooting" tests/`
Expected: a list of `(file:line)` matches. Some may be intentional references in test descriptions; update each topic-name literal to its `llmwiki-X` equivalent.

- [ ] **Step 2: Update each occurrence**

Use `Edit` tool per file. Replace `wiki-X` with `llmwiki-X` in topic-name literals (skip comments and prose).

- [ ] **Step 3: Add install-path assertion in `tests/install_skill_test.rs`**

Find the test that verifies install path. Add or update:

```rust
#[test]
fn install_skill_writes_to_llmwiki_directory() {
    // v0.3.36+: skill installs to ~/.agents/skills/llmwiki/, not .../wiki/
    // Hard cut — no backward-compat path.
    let home = tempdir().unwrap();
    let skill_dir = home.path().join(".agents/skills/llmwiki");
    // ... (use existing test fixtures, just change the path)
    assert!(skill_dir.join("SKILL.md").exists());
}
```

If a similar test already exists, update its expected path from `wiki` to `llmwiki`.

- [ ] **Step 4: Add guard test for legacy topic name**

In `tests/skill_test.rs`, add:

```rust
#[test]
fn skill_get_rejects_legacy_wiki_topic_name() {
    // v0.3.36+: wiki-search returns "unknown topic" same as any unknown name.
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_llmwiki-cli"))
        .args(["skill", "get", "wiki-search"])
        .output()
        .expect("failed to run llmwiki-cli skill get");
    assert!(!output.status.success(), "legacy topic should error");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unknown") || stderr.contains("not found"),
        "expected 'unknown topic' error, got: {stderr}"
    );
}
```

- [ ] **Step 5: Run integration tests**

Run: `cargo test --test skill_test --test install_skill_test 2>&1 | tail -15`
Expected: all pass.

- [ ] **Step 6: Commit**

```bash
git add tests/skill_test.rs tests/install_skill_test.rs
git commit -m "test: update topic-name assertions + add guard for legacy aliases"
```

---

## Task 11: Add CHANGELOG entry for v0.3.36

**Files:**
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Add v0.3.36 section at the top (above v0.3.35)**

Prepend to `CHANGELOG.md` (after the `# Changelog` line):

```markdown
## [0.3.36] - 2026-06-24 — pushy naming + llmwiki rebrand (hard cut)

**User-facing rebrand (no deprecation aliases):**

- Skill renamed `wiki` → `llmwiki`. Install path becomes
  `~/.agents/skills/llmwiki/SKILL.md`.
- Sub-skill topics renamed: `wiki-search` → `llmwiki-search`,
  `wiki-config` → `llmwiki-config`, etc. (9 topics total).
- **Breaking:** `llmwiki-cli skill get wiki-X` now returns "unknown topic".
  Update aliases / muscle memory in one pass.
- Hub SKILL.md description rewritten as an AI-trigger phrase block
  ("Load this skill whenever the user mentions ANY of: wiki, notes,
  second brain, RAG, citations, ...") for higher agent pickup rate.
- Cargo description + 3 new keywords (`llm-memory`, `second-brain`,
  `semantic-search`).
- CLI `--help` tagline now leads with brand: "Your LLM's persistent memory."
- README H1: `# wiki` → `# llmwiki`.

**Not changed (avoid breaking `cargo install` + every external link):**

- Binary name `llmwiki-cli`.
- Cargo package name `llmwiki-cli`.
- GitHub repo URL `github.com/Git-Fg/llmwiki`.

**Migration:** no automatic migration. Users update sub-skill aliases and
re-run `llmwiki-cli install-skill --global` (it now installs to
`~/.agents/skills/llmwiki/`).
```

- [ ] **Step 2: Commit**

```bash
git add CHANGELOG.md
git commit -m "docs(changelog): v0.3.36 — pushy naming + llmwiki rebrand"
```

---

## Task 12: Bump version to 0.3.36 in Cargo.toml

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Update `version`**

In `Cargo.toml` line 3:

```toml
# Before:
version = "0.3.35"
# After:
version = "0.3.36"
```

- [ ] **Step 2: Verify**

Run: `grep "^version" Cargo.toml`
Expected: `version = "0.3.36"`.

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "chore: bump version to 0.3.36"
```

---

## Task 13: Run full test suite

**Files:** none (verification only)

- [ ] **Step 1: Run all tests**

Run: `cargo test 2>&1 | tail -10`
Expected: `test result: ok. N passed; 0 failed; 0 ignored` across all 42 binaries (or whatever the count is — must be green).

- [ ] **Step 2: If failures, fix and re-run before proceeding**

If any test fails, the issue is most likely:
- A test in `src/skills/mod.rs` not updated in Task 5
- An integration test not updated in Task 10
- A cross-reference in a sub-skill not updated in Task 4

Re-read the failure message, fix, re-run.

- [ ] **Step 3: Run clippy**

Run: `cargo clippy --all-targets -- -D warnings 2>&1 | tail -20`
Expected: no warnings (CI uses `-D warnings`).

---

## Task 14: Reinstall CLI + skill system-wide

**Files:** none (verification only)

- [ ] **Step 1: Install CLI**

Run: `cargo install --path . --locked 2>&1 | tail -5`
Expected: `Installed package llmwiki-cli v0.3.36 (...)`.

- [ ] **Step 2: Install skill**

Run: `llmwiki-cli install-skill --global 2>&1 | tail -5`
Expected: `✓ Installed llmwiki skill to /Users/felix/.agents/skills/llmwiki`.

- [ ] **Step 3: Verify installed paths**

Run: `ls ~/.agents/skills/llmwiki/`
Expected: single `SKILL.md` file.

Run: `llmwiki-cli --version`
Expected: `llmwiki-cli 0.3.36`.

- [ ] **Step 4: Verify pushy tagline visible**

Run: `llmwiki-cli --help | head -3`
Expected: first line leads with `Your LLM's persistent memory in plain markdown`.

Run: `llmwiki-cli skill list`
Expected: 9 entries, all named `llmwiki-X`.

Run: `llmwiki-cli skill get wiki-search 2>&1 | head -3`
Expected: error containing `unknown` or `not found`.

---

## Task 15: Pre-release real-wiki smoke test (mandatory per AGENTS.md)

**Files:** none (verification only)

- [ ] **Step 1: Run the 6-step smoke test on `--wiki minimax`**

```bash
# 1. CLI version
llmwiki-cli --version
# Expected: llmwiki-cli 0.3.36

# 2. Doctor
llmwiki-cli --wiki minimax doctor
# Expected: ✓ Workspace, ✓ Wiki registry, ✓ Active alias, ✓ Config loaded, ✓ NIM reachable. Exit 0.

# 3. JSON shape
llmwiki-cli --wiki minimax doctor --json | jq 'keys'
# Expected: 15-key array

# 4. Config paths
llmwiki-cli --wiki minimax config paths
# Expected: per-computer + per-workspace candidates in priority order

# 5. Effective config
llmwiki-cli --wiki minimax config show-effective
# Expected: ~14 <default>-sourced entries

# 6. Page discovery
llmwiki-cli --wiki minimax ls --pages
llmwiki-cli --wiki minimax tree
# Expected: non-empty listings
```

- [ ] **Step 2: If any step fails, debug before tagging**

Common failure modes after a rebrand:
- `wiki-X` literal still in help text → fix and reinstall
- Skill install writes to wrong path → check `install-skill` source for hardcoded paths
- Cross-references in sub-skills broken → `grep -r "wiki-" src/skills/data/` should return nothing

---

## Task 16: Tag v0.3.36 + push (after user confirmation)

**Files:** none (release action)

- [ ] **Step 1: Show user the diff stat and ask for tagging confirmation**

Run: `git log --oneline v0.3.35..HEAD && echo "---" && git diff --stat v0.3.35..HEAD`

Present to user. **DO NOT tag or push without explicit user instruction** (AGENTS.md: "DO NOT run `git commit`, `git push`, `git reset`, `git rebase` and/or do any other git mutations unless explicitly asked to do so. Ask for confirmation each time when you need to do git mutations").

- [ ] **Step 2: After user confirms, tag and push**

```bash
git tag -a v0.3.36 -m "v0.3.36 — pushy naming + llmwiki rebrand (hard cut)"
git push origin main --tags
```

---

## Self-Review

**1. Spec coverage:**

| Spec section | Task |
|---|---|
| §1 Skill name → llmwiki | Task 6 |
| §2 Skill description (pushy triggers) | Task 6 |
| §3 Skill H1 | Task 6 |
| §4 CLI about/long_about | Task 8 |
| §5 Cargo description | Task 7 |
| §6 Cargo keywords | Task 7 |
| §7 README H1 | Task 9 |
| §8 Sub-skill topic names (hard cut, no aliases) | Tasks 2, 3, 4, 5, 10 |
| §9 after_help (no change required) | n/a |
| Test plan | Tasks 1, 5, 10, 13, 14 |
| Pre-release smoke | Task 15 |
| Tag/push | Task 16 |

All spec requirements covered.

**2. Placeholder scan:** No "TBD", no "implement later", no "fill in details". Every code block is concrete. Every shell command has expected output.

**3. Type consistency:** Function names match across tasks (`normalize_topic`, `find_skill`, `list_skills`, `LEAK_MARKERS`, `Cli`). No drift.

**4. Risks from spec:**
- "345 `wiki-` references in src/" — Task 4 grep audit + Tasks 8-9 manual edits cover the high-risk sites (skill router, sub-skill cross-refs, CLI help). A final `grep -rn '\bwiki-\(config\|ingest\|lint\|models\|query\|search\|setup\|sync\|troubleshooting\)\b' src/ skills/ tests/` after all tasks catches stragglers.

**5. Note on TDD discipline:** Tasks 1 (failing test for legacy rejection) and Task 5 (update existing tests) follow red-green-refactor. Tasks 2-4 and 6-12 are mechanical content rewrites where TDD doesn't apply — the test in Task 10 (guard test) is the post-hoc regression guard.

**6. One concern:** Task 5 Step 1 says the `normalize_topic` change should pass the new `normalize_topic_rejects_legacy_wiki_prefix` test added in Task 1. Re-reading Task 1: the test asserts `normalize_topic("wiki-search") == "wiki-search"`. The new `normalize_topic` implementation in Task 5 Step 1 returns `"wiki-search"` for that input (legacy pass-through branch). So the test passes. ✓