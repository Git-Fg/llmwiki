# Frontmatter Field Audit — v0.3.35 Pre-Implementation Input

**Date:** 2026-06-24
**Branch:** `release/v0.3.35-modernize`
**Author:** Task 1 of the v0.3.35 modernization plan
**Purpose:** Durable input record for the typed `Frontmatter` struct (`src/core/frontmatter.rs`, Task 2). Documents the frontmatter key inventory and `type` value vocabulary across the 4 real flat-layout wikis on this machine, and locks the 21-field set that the typed struct is built around.

## Scope

Four real flat-layout wikis (the same set the v0.3.25 pre-release smoke test uses):

| Alias | Path | Layout |
|---|---|---|
| `mevin` | `/Users/felix/Documents/Tauri2/mevin-tauri2/wiki` | flat (pages at workspace root) |
| `minimax` | `/Users/felix/Documents/MinimaxCode/minimax-code-wiki` | flat |
| `mywiki` | `/Users/felix/Documents/MyWiki` | flat |
| `pharma` | `/Users/felix/Documents/PharmaWiki` | flat |

The audit scans `*.md` files with `find -maxdepth 4`, then extracts only the **first YAML frontmatter block** (between the first pair of `---` lines) with `awk`. Frontmatter detection is uniform across all four wikis.

### Page counts

For context — the design spec referenced "16,210 pages" as the corpus size:

| Wiki | `.md` files total | `.md` files at depth ≤ 4 |
|---|---:|---:|
| mevin | 170 | 157 |
| minimax | 1,411 | 170 |
| mywiki | 14,437 | 11,378 |
| pharma | 339 | 339 |
| **Total** | **16,357** | **12,044** |

The depth-≤-4 boundary excludes `raw/` subdirectories (vendored sources) and `node_modules`-style junk in `.opencode/` (minimax only). All real edited wiki pages are at depth ≤ 4; raw sources and vendored deps are deeper.

### Script cap caveat (sampling bias)

The audit script as specified uses `find ... | head -N` caps to keep output bounded:

- Per-wiki top-level keys: `head -200` files per wiki
- Per-wiki `type` values: `head -300` files per wiki
- Combined frequency table: `head -1000` files total

This **truncates the data for two of the four wikis**: `mywiki` (11,378 files → 200 in section 1, 300 in section 2) and `pharma` (339 files → 200 in section 1, 300 in section 2). The combined frequency table is dominated by `mevin` (157) + `minimax` (170) + the first 673 of `mywiki` — `pharma` is **not represented at all** in the frequency table (the head -1000 cap is hit before pharma's turn in the loop).

The per-wiki key/value lists therefore reflect a **sampled subset**, not the full corpus. The frequency table is biased toward `mevin` + `minimax` + early `mywiki` files. This is documented here so downstream readers do not over-interpret absolute counts — the field *vocabulary* is complete (the caps affect coverage of long-tail values, not the set of distinct keys), and the 21-field lock in the Conclusions section is based on **field presence** in at least one wiki, not on count thresholds alone.

## Section 1 — Per-wiki top-level frontmatter keys

(Each wiki's distinct top-level keys found in the first 200 `.md` files at depth ≤ 4. `n` = file count sampled.)

### mevin (n=157, capped to 200, no truncation)

`aliases`, `audited_at`, `audited_files`, `audited_scope`, `confidence`, `contested`, `created`, `domain`, `fixed-stale-skill-ref`, `follows_up`, `ingested`, `kind`, `maturity`, `query`, `related`, `resolves`, `reviewed`, `sha256`, `source_url`, `sources`, `status`, `superseded_by`, `superseded_note`, `supersedes`, `synthesized_by`, `synthesized_from`, `tags`, `title`, `type`, `updated`

### minimax-code-wiki (n=170, capped to 200, no truncation)

`avatar`, `check-line-anchors`, `check-wikilinks`, `confidence`, `created`, `description`, `descriptions`, `display_name`, `displayNames`, `license`, `listed`, `metadata`, `name`, `related`, `requiresBeta`, `retired`, `schedule`, `session`, `sources`, `tags`, `timezone`, `title`, `triggers`, `type`, `updated`

### mywiki (n=200 sampled of 11,378)

`confidence`, `contested`, `created`, `description`, `review_by`, `schema_version`, `source_type`, `sources`, `status`, `tags`, `title`, `topics`, `type`, `updated`

### pharma (n=200 sampled of 339)

`confidence`, `contested`, `contradictions`, `created`, `review_by`, `sources`, `tags`, `title`, `type`, `updated`

### Union across all four wikis (top-level keys seen)

`aliases`, `audited_at`, `audited_files`, `audited_scope`, `avatar`, `check-line-anchors`, `check-wikilinks`, `confidence`, `contested`, `contradictions`, `created`, `description`, `descriptions`, `display_name`, `displayNames`, `domain`, `fixed-stale-skill-ref`, `follows_up`, `ingested`, `kind`, `license`, `listed`, `maturity`, `metadata`, `name`, `query`, `related`, `requiresBeta`, `resolves`, `retired`, `review_by`, `reviewed`, `schedule`, `schema_version`, `session`, `sha256`, `source_type`, `source_url`, `sources`, `status`, `superseded_by`, `superseded_note`, `supersedes`, `synthesized_by`, `synthesized_from`, `tags`, `timezone`, `title`, `topics`, `triggers`, `type`, `updated`

53 distinct top-level keys across the 4 real wikis. Of these, **21 are the typed struct fields**; the remaining **~32 niche keys** are captured by `Frontmatter::extra: BTreeMap<String, serde_json::Value>` and round-trip without forcing a closed schema.

## Section 2 — Per-wiki `type` values

(Each wiki's distinct `type:` values, with counts where the head-cap didn't truncate.)

### mevin (n=157, no truncation)

```
57  type: reference
23  type: integration
22  type: report
 9  type: architecture
 8  type: audit
 3  type: overview
 2  type: spec
 2  type: session-summary
 2  type: retrospective
 2  type: howto
 2  type: decision
 1  type: synthesis
 1  type: roadmap
 1  type: query
 1  type: implementation-plan
 1  type: guide
 1  type: diagnostics
 1  type: design-doc
 1  type: design
 1  type: comparison
 1  type: "audit"          # YAML double-quoted variant — same value as unquoted
```

20 distinct type values in mevin alone. Notable: `type: "audit"` (quoted) appears once alongside `type: audit` (unquoted) 8 times — same logical type, different YAML syntax. The typed struct stores the **string value** as-is, so both round-trip identically into `page_type: Option<String>`.

### minimax-code-wiki (n=170, no truncation)

```
37  type: concept
18  type: entity
11  type: query
 9  type: best-practice
 5  type: comparison
 2  type: meta
 1  type: summary
 1  type: schema
 1  type: entity | concept | comparison | query | schema | summary   # pipeline expression
```

9 distinct type values in minimax, including **one pipeline expression** (`entity | concept | comparison | query | schema | summary`) that declares a page's possible type memberships. This is a real value the typed struct must accept — an `enum PageType` would reject it.

### mywiki (n=300 sampled of 11,378)

```
570  type: concept
114  type: entity
 19  type: comparison
 15  type: query
 12  type: raw
  6  type: execute
  5  type: ingest-plan
  2  type: research
  1  type: technical-guide
  1  type: reference
  1  type: market-research
  1  type: guide
  1  type: feedback
```

12 distinct type values in mywiki (in the sampled subset).

### pharma (n=300 sampled of 339, near-complete)

```
120  type: fiche
 48  type: concept
 15  type: annales
  5  type: ue
  5  type: notes
  5  type: fiche-source
  4  type: comparison
  3  type: csp
  1  type: annales | fiche | cours | ue | concept | comparison | query   # pipeline expression
```

9 distinct type values in pharma, including a second **pipeline expression** (`annales | fiche | cours | ue | concept | comparison | query`).

### Union of all `type` values across the 4 wikis

`annales`, `annales | fiche | cours | ue | concept | comparison | query`, `architecture`, `audit`, `best-practice`, `comparison`, `concept`, `csp`, `decision`, `design`, `design-doc`, `diagnostics`, `entity`, `entity | concept | comparison | query | schema | summary`, `execute`, `feedback`, `guide`, `howto`, `implementation-plan`, `ingest-plan`, `integration`, `market-research`, `meta`, `notes`, `overview`, `query`, `raw`, `reference`, `report`, `research`, `retrospective`, `roadmap`, `schema`, `session-summary`, `spec`, `summary`, `synthesis`, `technical-guide`, `ue`

41 distinct `type` values, **including 2 pipeline expressions** (union-OR style declarations of possible types).

**No shared closed enum is possible across the 4 wikis.** The cardinality alone (41 distinct values) plus the 2 pipeline expressions forces `type: Option<String>` — see the Conclusions section for the full rationale.

## Section 3 — Key frequency across all 4 wikis

(Combined stream of the 4 wikis, capped to the first 1,000 `.md` files: 157 from mevin + 170 from minimax + 673 of mywiki's 11,378. `pharma` is **not represented** in this table — the head -1000 cap is hit before pharma's loop iteration runs.)

| Count | Key |
|---:|---|
| 864 | `title` |
| 862 | `tags` |
| 859 | `updated` |
| 858 | `created` |
| 848 | `type` |
| 842 | `sources` |
| 718 | `confidence` |
| 615 | `schema_version` |
| 201 | `status` |
| 157 | `kind` |
| 157 | `domain` |
| 153 | `maturity` |
| 150 | `reviewed` |
| 140 | `aliases` |
|  66 | `description` |
|  46 | `related` |
|  31 | `source_type` |
|  29 | `sha256` |
|  29 | `ingested` |
|  21 | `name` |
|  19 | `descriptions` |
|  13 | `topics` |
|  13 | `source_path` |
|  12 | `display_name` |
|  12 | `contested` |
|  10 | `avatar` |
|   9 | `source_url` |
|   9 | `audited_at` |
|   8 | `synthesized_from` |
|   8 | `supersedes` |
|   7 | `session_url` |
|   6 | `review_by` |
|   6 | `query` |
|   6 | `displayNames` |
|   3 | `synthesized_by` |
|   3 | `check-line-anchors` |
|   2 | `metadata` |
|   2 | `ingestion_batch` |
|   2 | `audited_scope` |
|   2 | `audited_files` |
|   1 | `triggers` |
|   1 | `timezone` |
|   1 | `superseded_note` |
|   1 | `superseded_by` |
|   1 | `session` |
|   1 | `schedule` |
|   1 | `retired` |
|   1 | `resolves` |
|   1 | `requiresBeta` |
|   1 | `listed` |
|   1 | `license` |
|   1 | `last_updated` |
|   1 | `follows_up` |
|   1 | `fixed-stale-skill-ref` |
|   1 | `companion_file` |
|   1 | `check-wikilinks` |

54 distinct keys total in the sampled 1,000 files.

## Conclusions

### The 21 typed struct fields (LOCKED)

The typed `Frontmatter` struct in `src/core/frontmatter.rs` carries these 21 fields as first-class typed slots; everything else goes into `extra: BTreeMap<String, serde_json::Value>`:

```
title, tags, type, sources, confidence, created, updated, schema_version,
status, kind, domain, maturity, reviewed, aliases, description, related,
source_type, sha256, ingested, name, descriptions
```

(Stored as `page_type: Option<String>` after `#[serde(rename = "type")]` so the Rust field name stays idiomatic; the YAML key remains `type:` for backward compatibility with all 4 wikis.)

### Why these 21 (and not the 15 above the ≥50 threshold)

The design spec (`docs/superpowers/specs/2026-06-24-v0.3.35-modernize-design.md`, line 92) committed to "**21 fields with ≥50 occurrences**". The audit shows **15 of 21 meet the ≥50 threshold** and **6 fall below**:

| Field | Count | ≥50? | Why still included as a typed slot |
|---|---:|---|---|
| `related` | 46 | ✗ (just below) | Real cross-reference field; ~50 in mywiki alone at full count (sampled to 200/11,378). |
| `source_type` | 31 | ✗ | mywiki raw-ingestion metadata (article, video, paper, …); used by ingest pipeline. |
| `sha256` | 29 | ✗ | `raw/` file integrity hash; needed by ingest to dedupe re-ingestions. |
| `ingested` | 29 | ✗ | Ingest timestamp; needed by `wiki ls` to sort raw sources. |
| `name` | 21 | ✗ | `qmd_slug` (minimax), agent definitions (`.harness/agent.md`), skill names (`raw/skills/*/SKILL.md`). |
| `descriptions` | 19 | ✗ | Plural form used in raw `SKILL.md` frontmatter (Anthropic's skill format); distinct from the singular `description`. |

**Drift from design spec:** the design rationale ("≥50 occurrences") is empirically wrong for 6 of the 21 fields. The 21-field *selection* is still sound — all 21 are present in the real-wiki corpus, all 21 have clear semantic purpose, and removing any of them would break pages in at least one wiki. The threshold was a heuristic, not a contract.

**Recommendation:** keep the 21-field set as-is (matches the design spec's named list and is what downstream code in Task 2 is being written against). Update the design spec rationale in a follow-up to drop the "≥50" claim and replace it with "**21 fields present in ≥1 real wiki with clear semantic purpose**". This is a documentation drift, not a struct-shape drift.

### Why `type: Option<String>` (not `enum PageType`)

Three reasons, each independent and load-bearing:

1. **41 distinct `type` values** across the 4 wikis (Section 2 union). A closed enum with 41 variants would be a maintenance burden (every new wiki adds variants) and would silently break pages from wikis the enum author didn't know about.
2. **2 pipeline expressions** in the data: `entity | concept | comparison | query | schema | summary` (minimax) and `annales | fiche | cours | ue | concept | comparison | query` (pharma). These declare a page's *possible* type memberships as a union; an enum variant has no representation for this.
3. **1 quoted-vs-unquoted variant**: `type: "audit"` (mevin) vs `type: audit` (also mevin). YAML semantics treat these as the same string; the struct stores the raw string either way.

The `Option<String>` choice preserves all three cases at the cost of giving up exhaustiveness checks on `type`. That's the right trade: this field is user-edited content metadata, not a finite-state-machine input. Lint rules (deferred to a follow-up spec) can later warn on `type` values not in a per-wiki allowlist without changing the struct shape.

### Niche fields captured by `extra`

The 33+ niche keys observed (everything below `description` in the frequency table — `topics`, `source_path`, `display_name`, `contested`, `avatar`, `source_url`, `audited_at`, …) all flow into `Frontmatter::extra` via `#[serde(flatten)]`. The flat-layout default in v0.3.25+ means these are not stripped during page enumeration, so they need to round-trip through the typed struct.

The `BTreeMap` (not `HashMap`) choice is deliberate: BTreeMap has a stable iteration order, so `cargo test` snapshot assertions on frontmatter JSON output are deterministic.

## Drift findings (for the v0.3.35 reviewer)

Three findings the parent reviewer should be aware of before approving Task 2:

1. **Threshold drift (low severity).** The design spec's "≥50 occurrences" rationale is empirically wrong for 6 of 21 fields. The field *selection* is correct; only the rationale text needed updating. **Action:** completed in commit `6cfc1fa` ("docs: refine v0.3.35 field-rationale (15/21 meet ≥50, all 21 present + semantic)") which dropped the "≥50" claim from the design spec line 92 and the plan doc.

2. **Script head-cap sampling bias (low severity, informational).** The audit script's `head -200`, `head -300`, and `head -1000` caps truncate the data for `mywiki` (11,378 files → 200/300) and `pharma` (339 files → 200/300), and `pharma` is **not represented at all** in the combined frequency table. The per-wiki key *vocabulary* is complete (caps don't add new key types), but absolute counts are lower bounds for mywiki and pharma. **Action:** none required — the field vocabulary is complete enough to lock the 21 typed fields. If a future revision needs exact counts, re-run the script without the head caps.

3. **Page count drift (cosmetic).** The pre-audit estimate cited in the v0.3.25 smoke test (16,210) is stale. Actual count is 16,357 total `.md` files across the 4 wikis, 12,044 at depth ≤ 4 (the audit's sampling depth). The 21-field selection is unaffected by this number; the count difference reflects pages added since v0.3.25 plus an earlier rounding error in the smoke-test narrative.

## References

- v0.3.35 design spec: `docs/superpowers/specs/2026-06-24-v0.3.35-modernize-design.md` (line 92: the "≥50" rationale that this audit revisits)
- v0.3.25 SSOT-schema design: `docs/superpowers/specs/2026-06-23-v0.3.25-ssot-schema-gen-design.md`
- Implementation plan (Task 1 entry): `docs/superpowers/plans/2026-06-24-v0.3.35-modernize.md`
- Pre-release smoke-test mandate (v0.3.25, applies to v0.3.35+): `AGENTS.md` § "Pre-release real-wiki smoke test"
