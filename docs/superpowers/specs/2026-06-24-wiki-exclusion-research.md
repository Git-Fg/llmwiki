# Research: Directory Exclusion in Production Flat-File Wiki/KB Tools

> **Purpose**: Inspiration-only research for a potential future `wiki.exclude_dirs`
> feature (v0.3.26+) in `llmwiki-cli`. This document contains NO implementation
> plan, NO spec, and NO code changes. It is a comparative analysis of how four
> canonical flat-file wiki tools (Obsidian, Logseq, Foam, Dendron) handle
> directory exclusion, with takeaways that may inform — but must not dictate —
> any future design decision.
>
> **Companion document**: `2026-06-24-exclude-dirs-research.md` covers qmd,
> ripgrep, fd, and cargo. This document covers wiki-specific tools that qmd and
> those general-purpose tools do not represent.

## Executive Summary

The four surveyed tools split into three distinct exclusion paradigms:

1. **Glob arrays** (Foam): the closest analogue to qmd's `ignore?: string[]`.
   Foam ships with 16 hardcoded sensible defaults aimed at developer wikis.
2. **Regex arrays** (Obsidian): a more expressive but harder-to-author model.
   Obsidian exposes this via `app.vault.setConfig("userIgnoreFilters", ...)`.
3. **Hierarchical dot-paths** (Dendron): a wiki-native primitive that is not
   a glob at all but matches Dendron's hierarchy concept.
4. **Literal-path arrays** (Logseq): the simplest model — only exact paths
   work, and there's a known bug where globs are silently ignored.

**The single most important finding for llmwiki**: there is no consensus on
glob syntax across wiki tools. Foam uses `**/folder/**/*`, Dendron uses
hierarchical dot-paths, Obsidian uses regex patterns, and Logseq uses literal
strings. If `wiki.exclude_dirs` follows Foam's convention (glob arrays relative
to workspace root), it would align with the closest analog (qmd) and the most
configurable production wiki tool (Foam).

---

## 1. Obsidian

### 1.1 Configuration Mechanism

Obsidian exposes exclusion via its **internal vault config API**:

```typescript
// Source: obsidian-hide-folders plugin (JonasDoesThings)
this.app.vault.getConfig("userIgnoreFilters")   // string[] | undefined
this.app.vault.setConfig("userIgnoreFilters", ignoreList);
```

The `userIgnoreFilters` setting is exposed in the **Settings → Files & Links →
"Excluded files"** textbox (one pattern per line). Obsidian's documentation
path (`help.obsidian.md/Plugins/User+defined+filters`) currently 404s, but the
underlying API is documented by community plugin code.

### 1.2 Pattern Syntax

**Regex patterns**, not globs. Stored as `string[]`. Example from the
`obsidian-hide-folders` plugin's auto-generation logic:

```typescript
createIgnoreListRegExpForFolderName(rawFolderName: string) {
  // For a folder named "attachments", generates:
  // /attachments/
  //
  // For rawFolderName = "startsWith::notes", generates:
  // /(^notes)|(\/notes)/
  //
  // For rawFolderName = "endsWith::_attachments", generates:
  // /(attachments$)|(attachments\/)/
}
```

So users type literal folder names in the UI (one per line), and Obsidian
internally wraps them as **regex patterns** of the form `/<name>/` (or
`/^<name>/|/\<name>/` for startswith, or `/<name>$/|/\<name>/` for endswith).

### 1.3 Default Exclusions

Obsidian's `.obsidian/` directory itself is excluded from indexing (since it's
the vault's metadata store). Beyond that, **Obsidian has no hardcoded global
exclusion list** — users opt in via the "Excluded files" setting.

### 1.4 Behavior

- Excluded files are hidden from: search, graph view, unlinked mentions.
- They are "less noticeable" in quick switcher and link suggestions (still
  present but deprioritized).
- The setting is per-vault, stored in `.obsidian/app.json` (or the modern
  equivalent `.obsidian/workspace.json`).

### 1.5 Key Characteristics

| Property | Value |
|---|---|
| **Field name** | `userIgnoreFilters` (internal), "Excluded files" (UI) |
| **Data type** | `string[]` (array of regex patterns) |
| **Pattern language** | JavaScript regex wrapped in `/.../` |
| **Built-in excludes** | `.obsidian/` (the metadata dir) |
| **Negation** | Yes (regex supports `!` via regex syntax) |
| **Config location** | `.obsidian/app.json` (per-vault) |
| **API surface** | `app.vault.getConfig/setConfig` |

---

## 2. Logseq

### 2.1 Configuration Mechanism

Logseq uses Clojure-style EDN configuration in `logseq/config.edn`:

```clojure
:hidden ["/archived" "/test.md"]
```

The `:hidden` key takes an array of strings.

### 2.2 Pattern Syntax

**Literal paths only** — not globs. Confirmed by forum thread:
> "I have tried `:hidden ["*/readme.md" "*/README.md"]` but this does not seem
> to work." — community user, 2023-02-01

Forum maintainer reply:
> "In `config.edn` (under `...` > `General`), find the following line:
> `:hidden []`. Within the square brackets, add any folders or files you want
> to hide in Logseq, wrapped in double quotes and separated by a space."

Example: `:hidden ["/archived" "/test.md"]`

The leading `/` indicates a root-relative path (similar to qmd's `cwd`-relative
approach).

### 2.3 Known Issues

- **Bug #8822**: `:hidden` is lost on app restart for some users (re-indexing
  re-loads everything).
- **Glob limitation**: glob patterns (`*`, `**`, `?`) are silently ignored.
  Only literal paths work.

### 2.4 Built-in Excludes

Logseq has no hardcoded global exclusion list. Users opt in via `:hidden`.

### 2.5 Key Characteristics

| Property | Value |
|---|---|
| **Field name** | `:hidden` (Clojure keyword) |
| **Data type** | Vector of strings |
| **Pattern language** | Literal paths only (no globs) |
| **Built-in excludes** | None |
| **Negation** | No |
| **Config location** | `logseq/config.edn` (per-graph) |
| **Known bugs** | Lost on restart (#8822); globs silently ignored |

---

## 3. Foam (VS Code Extension)

### 3.1 Configuration Mechanism

Foam uses VS Code's `contributes.configuration` schema in
`packages/foam-vscode/package.json`. Settings are stored in the workspace's
`.vscode/settings.json`:

```json
{
  "foam.files.exclude": [
    "**/.vscode/**/*",
    "**/_layouts/**/*",
    "**/node_modules/**/*"
  ],
  "foam.files.include": ["**/*.md", "**/*.markdown"]
}
```

### 3.2 Pattern Syntax

**Glob arrays**, using `picomatch` (also `micromatch` in deps). Patterns are
**workspace-relative** (rooted at the VS Code workspace folder).

Foam's docs explicitly recommend the pattern `<folderName>/**/*` to ignore a
folder's full contents (note the leading `**`).

### 3.3 Default Excludes (Hardcoded)

Foam ships with **16 sensible defaults** (the longest built-in list of any
surveyed tool):

```json
"foam.files.exclude": [
  "**/.vscode/**/*",
  "**/_layouts/**/*",
  "**/_site/**/*",
  "**/node_modules/**/*",
  "**/.venv/**/*",
  "**/venv/**/*",
  "**/env/**/*",
  "**/.env/**/*",
  "**/target/**/*",
  "**/dist/**/*",
  "**/build/**/*",
  "**/.next/**/*",
  "**/.nuxt/**/*",
  "**/.terraform/**/*",
  "**/.gradle/**/*",
  "**/.idea/**/*",
  "**/.cache/**/*",
  "**/__pycache__/**/*"
]
```

Categorized:
- **VS Code / editor**: `.vscode/`, `.idea/`
- **JS/TS build outputs**: `node_modules/`, `dist/`, `build/`, `.next/`,
  `.nuxt/`, `_site/`
- **Python venvs/caches**: `.venv/`, `venv/`, `env/`, `.env/`, `.cache/`,
  `__pycache__/`
- **JVM build outputs**: `.gradle/`, `target/`
- **IaC**: `.terraform/`
- **Static site generators**: `_layouts/` (Jekyll)

### 3.4 Multi-Source Merge

Foam combines three sources for exclusion:
1. `foam.files.exclude` (Foam's own setting, defaults shown above)
2. `foam.files.ignore` (deprecated, older API)
3. VS Code's built-in `files.exclude` (workspace-level)

This is documented in the setting description: "This setting is combined with
'foam.files.ignore' (deprecated) and 'files.exclude'."

### 3.5 Include vs Exclude Semantics

Foam uses a three-rule system documented in the setting:
1. Files must match at least one `foam.files.include` pattern.
2. Files must NOT match any `foam.files.exclude` pattern.
3. Built-in defaults are applied.

The default `include` is `["**/*"]` (everything). Users can narrow this to
e.g. `["notes/**"]` to limit Foam to a specific directory.

### 3.6 Per-Feature Excludes

Foam also has feature-specific excludes:
- `foam.orphans.exclude` — exclude paths from the orphans report
- `foam.placeholders.exclude` — exclude paths from the placeholders report

Both are empty by default and use the same `**/<folder>/**/*` syntax.

### 3.7 Key Characteristics

| Property | Value |
|---|---|
| **Field name (current)** | `foam.files.exclude` |
| **Field name (deprecated)** | `foam.files.ignore` |
| **Companion field** | `foam.files.include` |
| **Data type** | `string[]` (array of globs) |
| **Pattern language** | `picomatch` / `micromatch` globs |
| **Built-in excludes** | 16 hardcoded defaults |
| **Negation** | No (use `include` to whitelist) |
| **Config location** | `.vscode/settings.json` |
| **Glob library** | `picomatch` |

---

## 4. Dendron

### 4.1 Configuration Mechanism

Dendron uses YAML in `{workspaceRoot}/dendron.yml`:

```yaml
randomNote:
  include:
    - alpha
    - beta.foo
  exclude:
    - alpha.bar
```

### 4.2 Pattern Syntax

**Hierarchical dot-paths**, NOT globs. This is a wiki-native primitive.

- `"alpha"` matches all notes under the `alpha` hierarchy (e.g., `alpha.foo`,
  `alpha.bar`, `alpha.foo.baz`).
- `"beta.foo"` matches only notes directly under `beta.foo` (the exact
  hierarchy level, plus its descendants).
- `"alpha.bar"` excludes notes under `alpha.bar` from the `alpha` include.

The `*` suffix acts as a **prefix wildcard**:
- `"beta.foo*"` matches `beta.foo.bar`, `beta.foo.baz`, `beta.foo.qux`, etc.

### 4.3 Precedence Rule

> "If `include` is not specified, then the `include` matching pattern will
> match all notes. `exclude` takes precedence over `include`, so if the
> patterns are identical, no notes will match."

This is Dendron's explicit guarantee: exclude always wins over include.

### 4.4 Built-in Excludes

Dendron has hardcoded filename **restrictions** (not exclusions):
- Empty hierarchies (`foo..md`, `.foo.md`)
- Mixed-case names within the same hierarchy
- Disallowed characters: `(`, `)`, `'`, `,`

These are not directory exclusions per se — they prevent creating notes with
problematic filenames. Dendron has no built-in directory exclusion list.

### 4.5 Multi-Vault Scoping

Dendron's exclusion system operates per-vault in a multi-vault workspace,
allowing fine-grained control over which hierarchies are included/excluded in
each vault's random-note pool.

### 4.6 Key Characteristics

| Property | Value |
|---|---|
| **Field name** | `<feature>.include` / `<feature>.exclude` |
| **Data type** | `string[]` |
| **Pattern language** | Hierarchical dot-paths (wiki-native) |
| **Built-in excludes** | None (filename restrictions only) |
| **Negation** | No (`exclude` precedence is sufficient) |
| **Config location** | `dendron.yml` |
| **Default include** | All notes |

---

## 5. Cross-Tool Comparison Matrix

| Feature | Obsidian | Logseq | Foam | Dendron |
|---|---|---|---|---|
| **Config location** | `.obsidian/app.json` | `logseq/config.edn` | `.vscode/settings.json` | `dendron.yml` |
| **Field name** | `userIgnoreFilters` | `:hidden` | `foam.files.exclude` | `<feature>.exclude` |
| **Data type** | `string[]` (regex) | `string[]` (literal) | `string[]` (globs) | `string[]` (dot-paths) |
| **Pattern language** | Regex `/.../` | Literal paths | `picomatch` globs | Hierarchical dots |
| **Built-in excludes** | `.obsidian/` | None | 16 patterns | None (filename rules) |
| **Negation (`!`)** | Yes (regex) | No | No | No (exclude precedence) |
| **Anchoring** | Regex-anchored | Leading `/` | `**/.../*` prefix | Dot-hierarchy |
| **Hidden files default** | Skipped in UI | N/A | N/A | Restriction-based |
| **Companion include** | No | No | `foam.files.include` | `<feature>.include` |
| **Per-feature scoping** | No | No | Yes (orphans, placeholders) | Yes (per command) |
| **Self-merge with IDE** | No | No | Yes (VS Code's `files.exclude`) | N/A |

---

## 6. Built-In Default Lists Comparison

| Tool | Defaults | Targets |
|---|---|---|
| **qmd** | 6 patterns: `node_modules`, `.git`, `.cache`, `vendor`, `dist`, `build` | Dev project noise |
| **Foam** | 18 patterns: `.vscode`, `_layouts`, `_site`, `node_modules`, `.venv`, `venv`, `env`, `.env`, `target`, `dist`, `build`, `.next`, `.nuxt`, `.terraform`, `.gradle`, `.idea`, `.cache`, `__pycache__` | Dev project noise (deeper) |
| **Obsidian** | 1 pattern: `.obsidian/` | Vault metadata dir only |
| **Logseq** | 0 patterns | None |
| **Dendron** | 0 patterns (only filename restrictions) | None |

**Observation**: The two tools that ship hardcoded defaults (qmd, Foam) both
target **developer project noise** — build outputs, dependency directories, IDE
metadata. Neither targets wiki-specific noise (e.g., root-level `AGENTS.md`,
`README.md`, `.opencode/`, `.harness/`).

---

## 7. Pattern Language Comparison

### Syntax Variants

| Tool | `*` | `**` | `?` | `[abc]` | Anchoring | Negation | Comments |
|---|---|---|---|---|---|---|---|
| **Obsidian** | Yes (regex) | Yes (regex) | Yes (regex) | Yes (regex) | Regex | Yes (`!`) | Full regex power |
| **Logseq** | No | No | No | No | Leading `/` | No | Literal only |
| **Foam** | Yes | Yes (`**/`) | Yes | Yes | `**/.../*` | No | `picomatch` globs |
| **Dendron** | Suffix only (`foo*`) | No | No | No | Implicit (dot hierarchy) | No | Domain-specific |

### Key Insight

Foam and qmd are the only tools that converge on **the same glob dialect**
(`picomatch` / `fast-glob` are essentially compatible for `*`, `**`, `?`).
Obsidian's regex approach is more expressive but harder to author correctly
(witness the auto-generation logic in `obsidian-hide-folders`). Logseq's
literal-only approach is the simplest but most limited. Dendron's dot-path
approach is the most wiki-native but is Dendron-specific.

---

## 8. Design Observations (NOT Recommendations)

These are observations about trade-offs, not recommendations for what
`wiki.exclude_dirs` should or should not do.

### 8.1 Glob Arrays Are the Consensus

Foam (2020-present, 17k stars) and qmd (2024-present, 27k stars) — the two
most actively maintained Karpathy/flat-file-flavored wikis with exclusion
features — both use **string arrays of glob patterns** as their core model.
This is strong evidence that the model is right for this domain.

### 8.2 Logseq's Bug Is Cautionary

Logseq's `:hidden` setting has two documented issues:
1. Glob patterns are silently ignored (only literals work).
2. The setting is sometimes lost on app restart.

This demonstrates that **getting exclusion semantics wrong has real user
impact**. A wiki tool's exclusion model needs to either:
- Match what users expect from their prior tools (globs), OR
- Be obviously simpler (literal paths with clear UI guidance).

### 8.3 Obsidian's Regex Is Powerful But Hidden

Obsidian's `userIgnoreFilters` accepts full regex, but the UI exposes it as a
plain text box where users type folder names. The auto-wrap-to-regex logic is
done internally. This means:
- Most users don't know they're authoring regex.
- Power users can use full regex via the internal config.
- Plugin authors can use exact regex patterns.

This is a good example of **progressive disclosure**: simple UI, powerful
backend. Could be a model for `wiki.exclude_dirs` (glob in YAML by default,
regex via advanced override).

### 8.4 Foam's Default List Is the Most Comprehensive

Foam's 16 defaults are the most thorough of any surveyed tool. They target:
- **Build outputs** (multiple languages): `.next/`, `.nuxt/`, `target/`,
  `dist/`, `build/`, `.gradle/`
- **Dependency directories**: `node_modules/`, `.venv/`, `venv/`
- **IDE metadata**: `.vscode/`, `.idea/`
- **Caches**: `.cache/`, `__pycache__/`
- **Static site generator**: `_layouts/`, `_site/`

This list is **dev-project noise**, not wiki noise. But it's more thorough
than qmd's 6 patterns. For a wiki tool that wants sensible defaults, Foam's
list is a good starting point to consider.

### 8.5 Dendron's Hierarchy Concept Is Wiki-Native

Dendron's dot-path approach (`alpha.bar`) is the only model that thinks in
**hierarchy** rather than **path**. This maps directly to how notes are
organized in a wiki (hierarchies like `daily/2024/01/15` or `projects/llmwiki`).
For a wiki tool, this could be more intuitive than raw globs.

But it requires the tool to enforce a hierarchy convention on filenames
(Dendron uses `alpha.bar.md` → note `alpha.bar`). llmwiki does not enforce
any hierarchy convention — filenames are user-chosen.

### 8.6 Per-Feature Scoping vs Global Setting

Foam and Dendron both support **per-feature** exclusion (`foam.orphans.exclude`,
`foam.placeholders.exclude`, Dendron's per-command `include`/`exclude`).
Obsidian and Logseq use a **single global** setting.

The per-feature model is more expressive (you might want to exclude `assets/`
from random-note but include it in search). The global model is simpler.
For llmwiki, the choice depends on whether the CLI commands (lint, embed,
search, ls) need different exclusion semantics — which they probably don't,
so a single global `wiki.exclude_dirs` would suffice.

### 8.7 Default-Include Behavior

Foam and Dendron both support an `include` whitelist. qmd does not (always
includes all that match `pattern`, then excludes via `ignore`). Obsidian and
Logseq do not.

For llmwiki, the `include` semantics are already handled by `wiki.pages_dir`
(v0.3.25). So a separate `include` would be redundant.

---

## 9. What Each Tool Teaches (Inspiration Only)

| Lesson | Source | Applicability |
|---|---|---|
| Glob string arrays work well for this domain | Foam, qmd | **High** |
| Hardcoded defaults should target dev-project noise (the most common) | Foam | **High** |
| Hidden files need separate handling from directory exclusion | Obsidian, Logseq | High |
| Progressive disclosure (simple UI, regex backend) avoids user confusion | Obsidian | Medium |
| Pure-literal paths are too limiting for power users | Logseq (bug) | Medium |
| Wiki-native hierarchy concepts can replace globs IF the tool enforces them | Dendron | Low (llmwiki is not hierarchical) |
| Per-feature exclusion is nice-to-have but rarely essential | Foam, Dendron | Low |
| Regex arrays are powerful but require careful UI | Obsidian | Medium |
| Sensible defaults should be `**[name]/**/*` form (Foam's prefix style) | Foam | High |

---

## 10. Sources

1. **Obsidian API**: `JonasDoesThings/obsidian-hide-folders/blob/master/main.ts`
   - Confirms `app.vault.getConfig/setConfig("userIgnoreFilters", ignoreList)`
   - Shows the regex auto-generation logic
   - Fetched from `raw.githubusercontent.com/JonasDoesThings/obsidian-hide-folders/master/main.ts`

2. **Logseq community forum**: `discuss.logseq.com/t/how-do-i-exclude-a-folder-from-logseq-indexing/12777`
   - Confirms `:hidden` syntax in `config.edn`
   - Reports globs are ignored, only literals work
   - Fetched via trafilatura

3. **Logseq bug**: `github.com/logseq/logseq/issues/8822` (referenced in forum)
   - Confirms `:hidden` is lost on app restart for some users

4. **Foam package.json**: `github.com/foambubble/foam/blob/main/packages/foam-vscode/package.json`
   - Full `foam.files.exclude` default list (18 patterns)
   - `foam.files.include` description
   - `foam.files.ignore` deprecated alias
   - Per-feature excludes (`foam.orphans.exclude`, `foam.placeholders.exclude`)
   - Fetched from `raw.githubusercontent.com/foambubble/foam/main/packages/foam-vscode/package.json` (39.7KB)

5. **Foam issue #130**: `github.com/foambubble/foam/issues/130`
   - Confirms `files.exclude` integration with VS Code
   - Shows the recommended pattern: `**/node_modules`

6. **Foam issue #300**: `github.com/foambubble/foam/issues/300`
   - Shows the original feature request that led to `foam.files.exclude`
   - Shows the settings.json schema for `foam.edit.linkReferenceDefinitions.ignore`

7. **Dendron Commands docs**: `wiki.dendron.so/notes/eea2b078-1acc-4071-a14e-18299fc28f47/`
   - Shows `randomNote: { include, exclude }` syntax
   - Confirms hierarchical dot-path pattern
   - Confirms `*` suffix for prefix matching
   - Confirms exclude-takes-precedence rule

---

*This document is research only. It does not propose, specify, or recommend any
implementation. Any future design decision should be made independently, using
this analysis as one of many inputs.*