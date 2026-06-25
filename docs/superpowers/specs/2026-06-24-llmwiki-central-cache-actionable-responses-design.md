# Spec: Central Embedding Cache + Actionable CLI Responses (v0.4.0)

**Status**: Design approved. Awaiting plan execution.
**Owner**: Felix
**Target**: v0.4.0 (mid-2026)
**Supersedes**: v0.3.x single-wiki `embeddings.jsonl` model

---

## 1. The user-visible problem

Today (v0.3.37), `llmwiki-cli` works per-wiki: every workspace has its own `embeddings.jsonl`, every `embed` re-computes vectors from scratch (even chunks that already exist in another wiki), and CLI responses are static text — humans skim them, but AI agents have to parse natural-language errors to guess the next step.

The user's vision (verbatim):

> Users should just have to tell to use `llmwiki` and it works per default on embedding — reusing cache if existing, incremental if new diff appeared, central to avoid having to regenerate it per workspace within `~/.llmwiki` — all the various wiki concatenated from both computer-wide and project-side TOML file containing wiki's descriptions/paths/id, while allowing to search per wiki, with always actionable response and actionable tips/tricks on responses with optimal guidances that strongly steer/push the AI agent using the CLI in correct direction on messages response + errors.

This decomposes into 4 orthogonal problems that v0.4.0 solves together:

| # | Problem | v0.3.37 state | v0.4.0 target |
|---|---|---|---|
| 1 | Embedding cache reuse across wikis | Every wiki embeds independently | Central content-addressable cache shared by all wikis |
| 2 | Incremental on change | Whole-wiki re-embed | Per-chunk diff: only new/changed chunks re-embed |
| 3 | Central, not per-workspace | `embeddings.jsonl` is per-workspace | `~/.cache/llmwiki/embeddings/` is global; `embeddings.jsonl` is a sparse **index** |
| 4 | Agent-steering CLI responses | Plain text errors; next step is implicit | Structured `ActionableResponse<T>` with `next_steps`, `tips`, `examples` |

---

## 2. Design decisions

### 2.1 Storage layout

XDG-compliant (matches ruff, biome, cargo):

```
$XDG_CACHE_HOME/llmwiki/             # default: ~/.cache/llmwiki/
├── embeddings/
│   └── <model_id>/                  # e.g. nvidia/nv-embed-v1
│       ├── ab/
│       │   └── cd1234...5678.bin    # 2-char shard for FS limits
│       ├── cd/
│       │   └── ...
│       └── _INDEX.toml              # model_id, chunker_version, fingerprint_axes
├── queries/
│   └── <model_id>/
│       └── <fingerprint>.bin        # same scheme; query-side cache
└── config.toml                      # max_size, gc_policy, last_invalidation_time

<workspace>/
├── embeddings.jsonl                 # sparse index (paths → fingerprints)
│   # {"path": "comparisons/rust-enums.md",
│   #  "model": "nvidia/nv-embed-v1",
│   #  "sha256": "page-hash",
│   #  "chunked": true,
│   #  "chunks": [
│   #    {"fp": "ab/cd1234...5678", "start": 0,   "end": 500},
│   #    {"fp": "ef/gh5678...9012", "start": 500, "end": 1000}
│   #  ],
│   #  "embedded_at": "2026-06-24T10:30:00Z",
│   #  "chunker_version": "v0.4.0"}
└── .llmwiki-cli/config.toml
```

**Storage backend**: bincode-encoded (Vec<f32> + metadata). Not SQLite (overkill for key-value lookup; no queries needed beyond `get(fp)`).

**Why bincode + 2-char sharding**:
- 2-char sharding caps per-directory entries at 256²=65,536 — well under ext4's 65K subdir limit
- bincode is ~2× smaller than JSON, ~3× faster to parse
- mmap-friendly: large embedding files can be loaded with `mmap()` instead of `read()`

### 2.2 `EmbeddingFingerprint` — composite hash for correctness

Adapted from embcache (Bharath, May 2026). Plain `hash(text)` cache silently corrupts on any pipeline change. We compute:

```
EmbeddingFingerprint = SHA256(
    model_id                    # "nvidia/nv-embed-v1"
  + "|" + tokenizer_version     # locked to model version
  + "|" + chunker_strategy_hash # SHA256 of src/core/chunker.rs (build-time)
  + "|" + chunker_window_spec   # "tokens=512,overlap=64"
  + "|" + prompt_template_hash  # "query" vs "passage" (asymmetric models differ)
  + "|" + content_sha256        # SHA256 of the chunk text bytes
)
```

**Cache hit = fingerprint matches exactly. No partial matching.** The cost is cold re-embed on any pipeline change — accepted, because a warm cache returning wrong vectors is worse than a cold cache returning correct ones (embcache principle).

**Per-model `_INDEX.toml`** records the fingerprint axes used, so a future reader can validate that a cache entry is still valid:

```toml
[meta]
model_id = "nvidia/nv-embed-v1"
chunker_version = "v0.4.0"
chunker_window_spec = "tokens=512,overlap=64"
fingerprint_axes = ["model_id", "tokenizer_version", "chunker_strategy_hash", "chunker_window_spec", "prompt_template_hash", "content_sha256"]
created_at = "2026-06-24T10:00:00Z"
last_validated_at = "2026-06-24T10:00:00Z"
```

### 2.3 Per-wiki `embeddings.jsonl` becomes a sparse index

The on-disk format shrinks from ~kilobytes-of-vectors-per-page to ~100-bytes-per-chunk:

**Before (v0.3.37)**:
```json
{
  "path": "comparisons/rust-enums.md",
  "sha256": "abc...",
  "model": "nvidia/nv-embed-v1",
  "dim": 4096,
  "chunked": true,
  "chunks": [
    {"start": 0, "end": 500, "tokens": 120, "embedding": [0.0123, -0.0456, ...]},
    ...
  ]
}
```

**After (v0.4.0)**:
```json
{
  "path": "comparisons/rust-enums.md",
  "sha256": "abc...",
  "model": "nvidia/nv-embed-v1",
  "chunked": true,
  "chunks": [
    {"fp": "ab/cd1234...5678", "start": 0, "end": 500},
    ...
  ],
  "embedded_at": "2026-06-24T10:30:00Z",
  "chunker_version": "v0.4.0"
}
```

**Reduction**: ~95% smaller for typical pages. A 1MB page with 50 chunks drops from ~2MB to ~10KB.

**Backwards compat**: v0.4.0's `embed` detects old-format `embeddings.jsonl` and migrates on first run (writes vectors to central cache, rewrites as sparse index).

### 2.4 Incremental embedding

`embed` becomes a 3-phase pipeline:

```
Phase 1: SCAN
  walk pages, compute page SHA256
  compare against sparse index

Phase 2: DIFF
  - new pages: all chunks → fingerprint, cache miss → embed
  - changed pages (page SHA256 differs): all chunks → fingerprint,
    cache hit keeps the existing vector (since content SHA256 IS the
    chunk-level hash, chunks whose text didn't change are cache hits),
    cache miss → embed
  - unchanged pages: skip entirely

Phase 3: WRITE
  - new vectors → central cache (bincode file)
  - rewritten sparse index → embeddings.jsonl
  - GC: optionally clean orphan cache entries not referenced by any wiki
```

**Key insight**: content SHA256 IS the chunk-level hash. A page whose text didn't change has identical chunks, identical fingerprints, identical cache keys → all cache hits → zero API calls. Only chunks whose text actually changed incur cost.

### 2.5 Actionable CLI responses — every command

Every command's output goes through a new envelope:

```rust
#[derive(Serialize)]
pub struct ActionableResponse<T: Serialize> {
    pub status: &'static str,           // "ok" | "warning" | "error"
    pub command: &'static str,          // which command produced this
    pub facts: T,                        // command-specific data
    pub next_steps: Vec<NextStep>,       // 1-3 ranked actions
    pub tips: Vec<&'static str>,         // 1-line hints
    pub examples: Vec<&'static str>,     // copy-paste-able commands
}

pub struct NextStep {
    pub command: &'static str,           // the literal command
    pub rationale: &'static str,         // WHY this is the next thing
    pub priority: u8,                    // 1=critical, 5=optional
}
```

**Human mode** flattens to:
```
✓ <command> succeeded.
  <one-line summary>
  Next: <command>  (<rationale>)
  Tip:  <hint>
```

**JSON mode** keeps the full structure (what agents consume).

**Where the envelope applies**: `embed`, `search`, `query`, `doctor`, `status`, `lint`, `config`, `use`, `init`, `ingest`, `build`, `completion`. Error responses use the same envelope with `status: "error"` and a `facts.error` field.

### 2.6 New `cache` subcommand

```
llmwiki-cli cache info              # total size, hit rate, oldest entry, axis values
llmwiki-cli cache gc [--max-age 30d] [--max-size 5GB] [--dry-run]
llmwiki-cli cache verify            # check all central cache entries against _INDEX.toml
llmwiki-cli cache clear [--model nvidia/nv-embed-v1]
```

Each subcommand returns an `ActionableResponse<CacheStats>` with:
- facts: total_size, hit_rate, oldest_entry_age, entry_count, model_versions
- next_steps: GC if size > limit; verify if last_validated_at > 7d; clear if obsolete model versions present
- tips: "Use --no-shared-cache to disable cross-wiki cache reuse if disk is constrained"

### 2.7 `search` and `query` flow with central cache

```
search(query):
  - Resolve workspace OR fall back to fleet (v0.3.37 behavior)
  - Try query cache: fingerprint(query + model) → if hit, reuse embedding (zero NIM call)
  - On miss, embed query via NIM, write to query cache
  - For each candidate wiki:
      - Read sparse embeddings.jsonl
      - For each chunk fingerprint, mmap central cache → read f32×dim
      - Cosine similarity
  - Merge, sort, truncate to top_k
  - ActionableResponse<SearchResults>
```

**Performance**: mmap'd cache reads are ~3× faster than parse-JSON + copy. Plus, query cache eliminates ~200ms of NIM latency on repeat queries.

---

## 3. Key files / modules

| Path | What |
|---|---|
| `src/core/cache.rs` (NEW) | `CentralCache` struct, `EmbeddingFingerprint` type, bincode I/O, mmap reader, GC |
| `src/core/embeddings.rs` | Refactor to write sparse index instead of inlining vectors |
| `src/core/chunker.rs` | Add `version()` returning build-time hash |
| `src/cli/cache.rs` (NEW) | `cache info`, `cache gc`, `cache verify`, `cache clear` |
| `src/cli/actionable.rs` (NEW) | `ActionableResponse<T>` envelope + human/JSON formatters |
| `src/cli/embed.rs` | Refactor to 3-phase scan/diff/write with cache |
| `src/cli/search.rs` | Query cache + mmap'd vector reads |
| `src/cli/query.rs` | Same as search |
| `src/cli/{doctor,status,lint,config,use,init,ingest,build,completion}.rs` | Wrap output in `ActionableResponse<T>` |
| `src/cli/mod.rs` | Wire `Cache` subcommand |
| `tests/cache_test.rs` (NEW) | Central cache hit/miss/corruption-detection tests |
| `tests/actionable_test.rs` (NEW) | Envelope shape + human/JSON parity |
| `CHANGELOG.md` | v0.4.0 entry |
| `Cargo.toml` | Version bump; new deps (bincode, memmap2 if not already in tree) |
| `~/.agents/AGENTS.md` | Add "central cache" + "actionable response" sections |
| `skills/SKILL.md` | Update hub with cache workflow |
| `skills/data/llmwiki-*.md` | Update each sub-skill with new behaviors |

---

## 4. Migration strategy (backwards compat)

**v0.4.0 keeps reading v0.3.x `embeddings.jsonl`**:
1. On `embed`, detect old-format entries (has `chunks[].embedding` field)
2. Migrate one page at a time:
   - Write each chunk's vector to central cache
   - Compute its fingerprint
   - Rewrite the entry as a sparse index record
3. Once all pages migrated, rewrite `embeddings.jsonl` is fully sparse
4. If migration fails mid-way, fall back to old behavior for that wiki

**No data loss**: every vector from the old format is preserved in the central cache, keyed by the same fingerprint the new format would have computed.

**Versioning**: `embeddings.jsonl` gains a `_meta` record at the top:
```json
{"_meta": {"version": "v0.4.0-sparse", "migrated_at": "2026-06-24T..."}}
```

v0.3.x binaries refuse to read v0.4.0 sparse files (unrecognized field). v0.4.0 refuses to write old-format files. Clean break.

---

## 5. Out of scope (deferred to v0.5+)

- **Semantic cache hits** (embcache's FAISS HNSW for near-duplicate queries): valuable but adds a 100MB native dep (FAISS). Defer until users report cache hit rate < 80% even with composite fingerprints.
- **GPU cache tier** (shared LRU CUDA slab): requires `cudarc` or similar; defer.
- **Multi-node cache sharing**: out of scope for a single-developer CLI.
- **In-flight deduplication** (single in-progress request collapses concurrent identical requests): valuable for server-side but overkill for a CLI that makes one embedding call at a time.
- **Cache encryption at rest**: out of scope; chunks are derived from user content but aren't secrets.

---

## 6. Risks & mitigations

| Risk | Likelihood | Mitigation |
|---|---|---|
| Cache corruption on disk → wrong vectors silently returned | Medium | SHA256 fingerprint catches mismatches; `cache verify` subcommand; `embed` writes `_INDEX.toml` axis metadata per model |
| User clears `~/.cache/llmwiki/` and loses all embeddings → must re-embed everything | High | Document in migration guide; `cache clear` requires `--confirm`; v0.4.0 keeps old `embeddings.jsonl` as backup during migration |
| Sparse index grows with chunks faster than old format | Low | Sparse is ~95% smaller; only the index overhead matters at very high chunk counts (100K+) — at that scale switch to SQLite (deferred to v0.5) |
| `ActionableResponse` envelope adds ~50 lines per command | High | Encapsulate in `cli::actionable::print()` helper; one-line usage: `print_actionable("embed", "ok", facts, &steps)` |
| `mmap` reads on Windows differ from Unix | Medium | Use `memmap2` crate (cross-platform); fall back to `read()` on error |
| Backwards compat migration corrupts data | Medium | Write new sparse file first, verify with a check pass, then remove old file. Never delete old until new is validated. |
| `next_steps` content drifts from reality as the codebase changes | Medium | Snapshot the actionable test against golden JSON; fail CI if output drifts without an explicit update |

---

## 7. Success metrics

- **Cache hit rate > 90%** on a typical second-run `embed` (typical user's wiki has 80%+ chunks unchanged between runs)
- **All commands** produce structured output consumable by an LLM without prior knowledge of CLI semantics
- **Single point of truth** for embeddings: deleting the central cache forces re-embed of every wiki (proof: `cache clear` + `embed` re-populates from NIM)
- **No silent corruption**: `cache verify` returns "ok" within 10s for a 10GB cache
- **Agent success rate**: a coding agent (Claude Code / kimi-code) given only the `llmwiki-cli --help` output can complete a 3-step workflow (init → embed → search) without any external docs

---

## 8. References

- **embcache / Bharath** (May 2026): https://bh3r1th.medium.com/the-vector-embedding-cache-bug-that-costs-nothing-and-corrupts-everything-157be6c575e8 — the composite-fingerprint pattern that prevents silent cache corruption on pipeline changes
- **TypeGraph incremental re-indexing**: https://typegraph.ai/blog/incremental-re-indexing-rag-change-detection — content-hash-based re-index
- **DecryptCode AI ingestion**: https://decryptcode.com/blog/ai-document-ingestion-pipeline/ — document-level change detection
- **the-main-thread (Oct 2025)**: https://www.the-main-thread.com/p/ai-friendly-java-api-design — 9 principles for AI-Ready APIs (explicit discoverability, rich actionable error context, performance transparency, progressive disclosure)
- **Anthropic Skills progressive disclosure**: https://www.anthropic.com/engineering/equipping-agents-for-the-real-world-with-agent-skills — metadata → SKILL.md → referenced files (3-layer load)
- **LLVM CAS**: https://llvm.org/docs/ContentAddressableStorage.html — content-addressable storage with DAG dedup
- **Ruff XDG cache**: https://github.com/astral-sh/ruff — `~/.cache/ruff/` per-tool XDG-compliant cache layout