# Research: Directory Exclusion Patterns Across Tools

> **Purpose**: Inspiration-only research for a potential future `wiki.exclude_dirs`
> feature (v0.3.26+). This document contains NO implementation plan, NO spec,
> and NO code changes. It is a comparative analysis of how five canonical tools
> handle directory exclusion, with takeaways that may inform — but must not
> dictate — any future design decision.

## Executive Summary

All five surveyed tools (qmd, ripgrep, fd, cargo manifest, cargo workspace) solve
the same problem: "walk a directory tree, skip certain paths." They converge on
**glob-pattern arrays relative to a root**, but diverge on three axes:

1. **Built-in hardcoded excludes** — qmd hardcodes 6 directories; others defer
   to `.gitignore` or leave it fully user-controlled.
2. **Hidden-file handling** — qmd filters post-glob; ripgrep/fd skip by default
   with opt-in flags; cargo uses `.*` glob in user patterns.
3. **Negation** — cargo `include` supports `!`; ripgrep CLI supports `!`; qmd
   and fd `--exclude` do not.

The single most important finding for llmwiki: **qmd is the only tool that
combines hardcoded built-in excludes with user-configurable ignores.** Every
other tool either hardcodes (via `.gitignore` respect) or is purely user-driven.

---

## 1. qmd — The Canonical Karpathy-Wiki Search Tool

Source: `tobi/qmd` — `src/store.ts` (`reindexCollection`), `src/collections.ts`.

### 1.1 Data Model

```typescript
// collections.ts
interface Collection {
  path: string;              // Absolute path to index
  pattern: string;           // Glob pattern (default: '**/*.md')
  ignore?: string[];         // User-specified exclusion globs
  context?: ContextMap;
  update?: string;
  includeByDefault?: boolean;
}
```

Stored in `~/.config/qmd/index.yml`:
```yaml
collections:
  mywiki:
    path: /Users/me/mywiki
    pattern: "**/*.md"
    ignore:
      - "Sessions/**"
      - "drafts/*"
```

### 1.2 Implementation (store.ts `reindexCollection`)

The actual exclusion logic is a **two-tier system**:

```typescript
// Tier 1: hardcoded built-in excludes (NOT configurable, always applied)
const excludeDirs = ["node_modules", ".git", ".cache", "vendor", "dist", "build"];

// Merge built-in + user-specified into a single ignore array
const allIgnore = [
  ...excludeDirs.map(d => `**/${d}/**`),   // → "**/node_modules/**", etc.
  ...(options?.ignorePatterns || []),       // user's Collection.ignore
];

// Tier 2: fast-glob with ignore option
const allFiles = await fastGlob(globPattern, {
  cwd: collectionPath,
  onlyFiles: true,
  followSymbolicLinks: false,
  dot: false,           // skip dotfiles at the glob level
  ignore: allIgnore,
});

// Tier 3: post-glob hidden-file filter (belt-and-suspenders)
const files = allFiles.filter(file => {
  const parts = file.split("/");
  return !parts.some(part => part.startsWith("."));
});
```

### 1.3 Key Characteristics

| Property | Value |
|---|---|
| **Glob library** | `fast-glob` (npm) |
| **Built-in excludes** | `node_modules`, `.git`, `.cache`, `vendor`, `dist`, `build` |
| **Built-in exclude format** | `**/${dir}/**` (wraps bare dir name in `**/.../**`) |
| **User exclude field** | `ignore?: string[]` |
| **User exclude format** | Raw glob patterns (e.g., `"Sessions/**"`) |
| **Pattern evaluation root** | Collection's `path` (via fastGlob `cwd`) |
| **Hidden file handling** | `dot: false` in fastGlob + post-glob filter (any segment starting with `.`) |
| **Symlink following** | Disabled (`followSymbolicLinks: false`) |
| **Negation (`!`)** | Not supported |
| **Anchoring (`/`)** | Not needed — patterns are relative to `cwd` |
| **`.gitignore` respect** | Not automatic; user must add patterns manually |

### 1.4 DB Persistence

The `ignore` array is JSON-serialized and stored in SQLite:
```sql
CREATE TABLE store_collections (
  name TEXT PRIMARY KEY,
  path TEXT NOT NULL,
  pattern TEXT NOT NULL DEFAULT '**/*.md',
  ignore_patterns TEXT,    -- JSON array, nullable
  ...
);
```

### 1.5 Project-Local Config

qmd also supports `.qmd/index.yaml` (or `.yml`) in the project root, found by
walking upward from `cwd`. This keeps config co-located with the collection
data instead of in the global `~/.config/qmd/`.

---

## 2. ripgrep — The Rust Ecosystem Standard

Source: BurntSushi/ripgrep `GUIDE.md`.

### 2.1 Three-Tier Ignore File Hierarchy

ripgrep automatically respects (in precedence order, highest wins):

1. `.rgignore` — ripgrep-specific (highest precedence)
2. `.ignore` — application-agnostic (shared with fd, ag)
3. `.gitignore` — Git's standard ignore file (lowest precedence)

Plus: `$GIT_DIR/info/exclude` and `core.excludesFile` (usually
`$XDG_CONFIG_HOME/git/ignore`).

### 2.2 CLI Manual Filtering

```bash
rg pattern -g '*.rs'           # whitelist: only .rs files
rg pattern -g '!*.toml'        # blacklist: skip .toml files (note: ! prefix)
rg pattern -g '!*.toml' -g '*.toml'  # order matters: later overrides earlier
```

- `--glob` / `-g`: glob pattern. `!` prefix = negation (blacklist).
- `--type rust` / `-t rust`: include files of type.
- `--type-not rust` / `-T rust`: exclude files of type.

### 2.3 Default Exclusions (Automatic)

- Hidden files and directories (`.dotfile`).
- Binary files (any file containing a NUL byte).
- Symbolic links (not followed).
- Everything in `.gitignore` / `.ignore` / `.rgignore`.

Can be toggled: `--no-ignore`, `--hidden` (`-.`), `--unrestricted` (`-u`,
repeated use strips more filtering).

### 2.4 Glob Syntax

Same as `.gitignore`:
- `*` matches any sequence except `/`
- `**` matches any sequence including `/`
- `?` matches any single character
- `[abc]` character class
- Leading `/` anchors to the `.gitignore` file's directory
- Trailing `/` matches directories only
- `!` prefix means re-include (whitelist in file context)

---

## 3. fd — The find(1) Replacement

Source: sharkdp/fd `README.md`.

### 3.1 CLI Exclusion

```bash
fd pattern -E .git               # exclude .git directories
fd pattern -E /mnt/external-drive  # exclude mounted dirs
fd pattern -E '*.bak'            # exclude .bak files
```

`-E` / `--exclude <glob>`: takes a glob pattern. Multiple `-E` flags accumulate.

### 3.2 Ignore File Support

- Respects `.gitignore` by default.
- Supports `.fdignore` files (fd-specific, same syntax as `.gitignore`).
- Supports `.ignore` files (shared with rg, ag).
- Global ignore file: `~/.config/fd/ignore` (or `%APPDATA%\fd\ignore` on Windows).
- `-I` / `--no-ignore`: disable all ignore-file respect.
- `-u` / `--unrestricted`: show hidden + ignored files.

### 3.3 Key Characteristics

| Property | Value |
|---|---|
| **Exclude mechanism** | `-E <glob>` (CLI flag, repeatable) |
| **Negation** | Not on CLI (uses ignore files with `!`) |
| **Anchoring** | Patterns match against full path |
| **Ignore files** | `.gitignore`, `.ignore`, `.fdignore`, global `~/.config/fd/ignore` |

---

## 4. Cargo — Package & Workspace Exclusion

### 4.1 Package Manifest (`[package]`)

Source: `doc.rust-lang.org/cargo/reference/manifest.html`.

```toml
[package]
exclude = ["/ci", "images/", ".*"]
include = ["/src", "COPYRIGHT", "/examples", "!/examples/big_example"]
```

**Semantics:**
- `exclude`: glob patterns of files to skip when packaging.
- `include`: glob patterns of files to explicitly include (overrides exclude).
- Leading `/`: anchors to package root.
- Trailing `/`: matches directories only.
- `.*`: matches any hidden file/dir.
- `!` in `include`: re-include after an exclude (negation).
- Default (no `exclude`): includes everything except sub-packages (dirs with
  their own `Cargo.toml`), `target/`, and git-ignored files.
- `include` and `exclude` cannot be used together (except `include` with `!`
  for re-include semantics).

### 4.2 Workspace (`[workspace]`)

Source: `doc.rust-lang.org/cargo/reference/workspaces.html`.

```toml
[workspace]
members = ["member1", "path/to/member2", "crates/*"]
exclude = ["crates/foo", "path/to/other"]
```

**Semantics:**
- `members`: paths or globs (`*`, `?`) to include as workspace members.
- `exclude`: paths to explicitly exclude from workspace (useful when a glob in
  `members` matches something unwanted).
- Patterns are relative to workspace root.
- Uses the `glob` crate's syntax (standard glob with `*`, `?`, `[...]`).
- No `**` support in workspace members/exclude (unlike `.gitignore`).

---

## 5. Cross-Tool Comparison Matrix

| Feature | qmd | ripgrep | fd | cargo `[package]` | cargo `[workspace]` |
|---|---|---|---|---|---|
| **Config location** | YAML + DB | `.gitignore` etc. | `.gitignore` etc. | `Cargo.toml` | `Cargo.toml` |
| **Field name** | `ignore: string[]` | N/A (files) | N/A (files) | `exclude = [...]` | `exclude = [...]` |
| **Data type** | `string[]` (YAML/JSON) | file lines | file lines | TOML array | TOML array |
| **Built-in excludes** | 6 hardcoded dirs | `.gitignore` rules | `.gitignore` rules | sub-packages, `target/` | none |
| **Hidden files** | Filtered (post-glob) | Skipped by default | Skipped by default | User pattern (`.*`) | N/A |
| **Negation (`!`)** | No | Yes (CLI `!`) | No (CLI), Yes (files) | Yes (`include` with `!`) | No |
| **Anchoring (`/`)** | No (relative to `cwd`) | Yes (leading `/`) | Implicit | Yes (leading `/`) | Implicit |
| **`**` glob** | Yes | Yes | Yes | Yes | No (`*` and `?` only) |
| **Symlink following** | Disabled | Disabled | Configurable | N/A | N/A |
| **User override** | Via `ignore` field | `--hidden`, `--no-ignore` | `-I`, `-u` | N/A | N/A |

---

## 6. Pattern Language Comparison

### Glob Syntax Variants

| Syntax | qmd (fast-glob) | ripgrep (gitignore) | fd | cargo pkg | cargo ws |
|---|---|---|---|---|---|
| `*` (non-`/`) | Yes | Yes | Yes | Yes | Yes |
| `**` (cross-`/`) | Yes | Yes | Yes | Yes | No |
| `?` (single char) | Yes | Yes | Yes | Yes | Yes |
| `[abc]` (charset) | Yes | Yes | Yes | Yes | Yes |
| `{a,b}` (brace) | Yes | No | No | No | No |
| Leading `/` (anchor) | No | Yes | Implicit | Yes | Implicit |
| Trailing `/` (dir-only) | No | Yes | No | Yes | No |
| `!` (negation) | No | Yes (CLI) | No (CLI) | Yes (`include`) | No |

### Key Insight

The **gitignore dialect** (used by ripgrep and fd's ignore files) is the most
expressive: it supports anchoring, directory-only matching, and negation. The
**simple glob dialect** (used by qmd and cargo workspace) is the simplest: just
`*`, `?`, and `**` — no anchoring, no negation, no trailing-slash semantics.

---

## 7. Design Observations (NOT Recommendations)

These are observations about trade-offs, not recommendations for what
`wiki.exclude_dirs` should or should not do.

### 7.1 Two-Tier vs One-Tier

qmd is unique in combining hardcoded built-in excludes with user-configurable
ones. This means:
- **Pro**: Users don't need to manually exclude `node_modules/`, `.git/`, etc.
- **Con**: The hardcoded list is invisible to the user; they can't remove items.
- **Con**: The list may not match the user's project (e.g., a Go project has no
  `node_modules` but has `vendor/` which IS excluded).

ripgrep and fd solve this differently: they respect `.gitignore` by default,
which is already project-aware. A Rust project's `.gitignore` already has
`/target`, a Node project's already has `node_modules/`, etc.

### 7.2 Hidden-File Handling

qmd's post-glob filter (`!parts.some(part => part.startsWith("."))`) is more
aggressive than necessary — it also filters out legitimate directories like
`.github/` or `.vscode/` that may contain markdown files a user wants indexed.

ripgrep and fd skip hidden files by default but provide `--hidden` to opt in.
cargo's approach (`.*` in user patterns) is the most granular — the user
chooses whether to exclude hidden files at all.

### 7.3 Negation Trade-off

Negation (`!`) adds expressive power: "exclude everything in `drafts/` except
`drafts/published.md`." But it also adds complexity:
- Order dependence (later patterns override earlier ones).
- User confusion (`!` means blacklist on ripgrep CLI but whitelist in
  `.gitignore` files).

qmd, fd CLI, and cargo workspace all avoid negation entirely. This keeps the
mental model simple: "these patterns are excluded, period."

### 7.4 Configuration Location

qmd stores exclusion config in YAML (either global `~/.config/qmd/index.yml` or
project-local `.qmd/index.yaml`). This is co-located with the collection
definition itself, not scattered across `.gitignore` files.

ripgrep and fd rely on `.gitignore` + `.ignore` + tool-specific files. This
means exclusion rules are implicitly inherited from the project's existing
git configuration, but the user has no single place to see all active rules.

cargo uses `Cargo.toml` — co-located with the project definition.

### 7.5 qmd's Built-in List Analyzed

qmd's hardcoded list: `["node_modules", ".git", ".cache", "vendor", "dist", "build"]`.

Mapping to real-world wiki noise (from v0.3.25 smoke test):
- `node_modules` → caught `.opencode/` noise? **No** — `.opencode` starts with `.`,
  filtered by hidden-file rule, not by this list.
- `.git` → caught `.harness/` noise? **No** — same reason.
- `.cache` → rarely present in wikis.
- `vendor` → rarely present in wikis.
- `dist` → rarely present in wikis.
- `build` → rarely present in wikis.

**Observation**: qmd's built-in list targets *development project* noise, not
*wiki* noise. The actual wiki noise (`AGENTS.md`, `CLAUDE.md`, `README.md` at
root, `.opencode/`, `.harness/`) is caught by qmd's hidden-file filter and by
the user's explicit `ignore` patterns — not by the built-in directory list.

This suggests that for a wiki-specific tool, the "built-in excludes" concept
may be less useful than a "hidden-file exclusion" concept.

---

## 8. What Each Tool Teaches (Inspiration Only)

| Lesson | Source | Applicability |
|---|---|---|
| Simple glob arrays are sufficient for 95% of use cases | qmd, cargo | High |
| Hidden-file filtering is a separate concern from directory exclusion | qmd, ripgrep, fd | High |
| Negation adds power but also adds confusion; omitting it is a valid choice | qmd, fd, cargo-ws | Medium |
| Co-locating config with the data it controls is cleaner than scattered ignore files | qmd, cargo | High |
| Built-in hardcoded excludes target development noise, not knowledge-base noise | qmd | High (cautionary) |
| The `gitignore` dialect is the most expressive but also the most complex | ripgrep | Low |
| Pattern evaluation root should be explicit (not implicit) | all | High |

---

## 9. Sources

1. **qmd source**: `tobi/qmd` — `src/store.ts` (`reindexCollection`), `src/collections.ts`
   - Fetched from `raw.githubusercontent.com/tobi/qmd/main/src/store.ts` (191KB)
   - Fetched from `raw.githubusercontent.com/tobi/qmd/main/src/collections.ts` (15KB)

2. **ripgrep**: BurntSushi/ripgrep `GUIDE.md`
   - Extracted from `github.com/BurntSushi/ripgrep/blob/master/GUIDE.md`

3. **fd**: sharkdp/fd `README.md`
   - Extracted from `github.com/sharkdp/fd/blob/master/README.md`

4. **Cargo manifest**: `doc.rust-lang.org/cargo/reference/manifest.html`
   - The `exclude` and `include` fields section

5. **Cargo workspace**: `doc.rust-lang.org/cargo/reference/workspaces.html`
   - The `members` and `exclude` fields section

---

*This document is research only. It does not propose, specify, or recommend any
implementation. Any future design decision should be made independently, using
this analysis as one of many inputs.*
