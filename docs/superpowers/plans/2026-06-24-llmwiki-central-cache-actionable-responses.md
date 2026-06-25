# Plan: Central Cache + Actionable Responses (v0.4.0)

**Spec**: `docs/superpowers/specs/2026-06-24-llmwiki-central-cache-actionable-responses-design.md`
**Total effort**: ~13 working days across 5 phases
**Target version**: v0.4.0

Each task ends with: commit + test pass + clippy clean + push to main.

---

## Phase 1: `ActionableResponse<T>` envelope + apply to all commands (3 days)

The single most universally valuable piece. Every command becomes agent-steering without needing the central cache. This can ship alone as v0.3.38 if desired.

### Task 1.1: Define `ActionableResponse<T>` + formatters
**File**: `src/cli/actionable.rs` (NEW)

```rust
pub struct ActionableResponse<T: Serialize> {
    pub status: &'static str,        // "ok" | "warning" | "error"
    pub command: &'static str,       // e.g. "embed"
    pub facts: T,                    // command-specific data
    pub next_steps: Vec<NextStep>,
    pub tips: Vec<&'static str>,
    pub examples: Vec<&'static str>,
}

pub struct NextStep {
    pub command: &'static str,
    pub rationale: &'static str,
    pub priority: u8,
}

impl<T: Serialize> ActionableResponse<T> {
    pub fn ok(command: &'static str, facts: T) -> Self;
    pub fn warning(command: &'static str, facts: T) -> Self;
    pub fn error(command: &'static str, facts: T) -> Self;
    pub fn with_next_step(mut self, command: &'static str, rationale: &'static str, priority: u8) -> Self;
    pub fn with_tip(mut self, tip: &'static str) -> Self;
    pub fn with_example(mut self, cmd: &'static str) -> Self;
    pub fn print(&self, json: bool);  // human OR JSON based on flag
}
```

**Helpers**: `src/cli/actionable.rs::print_ok()`, `print_warning()`, `print_error()` — one-line shortcuts for the common case.

### Task 1.2: Apply envelope to `embed`
**File**: `src/cli/embed.rs`

Replace direct `println!` with `print_ok("embed", facts, steps)`. Facts include: `pages_scanned`, `chunks_embedded`, `chunks_reused`, `cache_hit_rate`, `api_calls`, `duration_ms`.

Steps on success:
- p1: "llmwiki-cli search \"<recent page slug>\"  # verify your changes are findable"
- p3: "llmwiki-cli lint --scope wiki  # confirm wiki contract holds"
- p5: "llmwiki-cli embed --help  # see flags (--force, --dry-run, --no-cache)"

Tips on success:
- "Re-running embed is cheap — unchanged chunks are reused from cache"
- "Per-page SHA256 in embeddings.jsonl means only changed chunks re-embed"

Steps on no-op (nothing to embed):
- p1: "llmwiki-cli ingest <file>  # add new content first"
- p3: "llmwiki-cli status  # see current state"

### Task 1.3: Apply envelope to `search` + `query`
**Files**: `src/cli/search.rs`, `src/cli/query.rs`

Facts on success: `query`, `mode` ("single" | "fleet"), `wikis_searched`, `wikis_skipped`, `top_k`, `duration_ms`.

Steps on success:
- p2: "llmwiki-cli query \"<query>\"  # get a synthesized answer with citations"
- p3: "llmwiki-cli search \"<query>\" --wiki <alias>  # narrow to one wiki"

Tips:
- "Lower --threshold to see more results; raise it for higher precision"
- "Use 'llmwiki-cli tree | grep <slug>' to filter by slug before searching"

Error variants:
- "no embeddings yet" → p1: "llmwiki-cli embed"
- "workspace not found" → p1: "cd into a wiki directory" / p2: "llmwiki-cli use <alias>" / p3: "llmwiki-cli search --wiki <alias>"
- "NIM unreachable" → p1: "check NVIDIA_NIM_API_KEY env" / p2: "WIKI_NIM_BASE_URL=https://staging.api.nvidia.com llmwiki-cli search ..."

### Task 1.4: Apply envelope to `doctor`, `status`, `lint`
**Files**: `src/cli/{doctor,status,lint}.rs`

Each gets:
- facts: command-specific (alias, counts, errors)
- next_steps: 1-3 follow-up commands
- tips: relevant hints

### Task 1.5: Apply envelope to `config`, `use`, `init`, `ingest`, `build`, `completion`
**Files**: `src/cli/{config,use_cmd,init,ingest,build,completion}.rs`

Smaller scope per command; envelope wraps stdout + stderr uniformly.

### Task 1.6: Tests
**File**: `tests/actionable_test.rs` (NEW)

- `envelope_human_mode_omits_json_syntax` — human output is plain text, no JSON braces
- `envelope_json_mode_round_trips` — `--json` output parses as JSON with expected shape
- `envelope_next_steps_sorted_by_priority` — priority 1 appears before 2, etc.
- `envelope_error_response_has_error_status` — `status: "error"` for failures
- Golden file: `tests/fixtures/envelope_golden.json` — snapshot of embed's full response; fail CI if drift without explicit update

### Task 1.7: CHANGELOG + version bump
- v0.3.38 entry (Phase 1 alone) OR v0.4.0 entry (all phases)

**Commit checkpoint after Task 1.7** — Phase 1 is shippable as v0.3.38 if user wants incremental rollout.

---

## Phase 2: Central content-addressable cache (5 days)

### Task 2.1: Define `EmbeddingFingerprint` type
**File**: `src/core/cache.rs` (NEW)

```rust
pub struct EmbeddingFingerprint(String);  // 64-char hex

impl EmbeddingFingerprint {
    pub fn compute(model_id: &str, chunker_version: &str, prompt: &str, content: &[u8]) -> Self;
    pub fn as_path(&self) -> PathBuf;  // ~/.cache/llmwiki/embeddings/<model>/ab/cd1234...5678.bin
    pub fn shard_prefix(&self) -> &str;  // first 2 chars (for dir)
    pub fn shard_suffix(&self) -> &str;  // rest (for filename)
}
```

### Task 2.2: `CentralCache` struct
**File**: `src/core/cache.rs`

```rust
pub struct CentralCache {
    root: PathBuf,  // ~/.cache/llmwiki/
}

impl CentralCache {
    pub fn open() -> Result<Self, WikiError>;  // ensures root exists
    pub fn get(&self, fp: &EmbeddingFingerprint, model_id: &str) -> Result<Option<CachedEmbedding>, WikiError>;
    pub fn put(&self, fp: &EmbeddingFingerprint, model_id: &str, emb: &[f32]) -> Result<(), WikiError>;
    pub fn info(&self) -> Result<CacheStats, WikiError>;
    pub fn gc(&self, opts: GcOptions) -> Result<GcReport, WikiError>;
    pub fn verify(&self) -> Result<VerifyReport, WikiError>;
    pub fn clear(&self, opts: ClearOptions) -> Result<(), WikiError>;
}
```

### Task 2.3: `_INDEX.toml` per model
**File**: `src/core/cache.rs`

On first `put()` for a model:
- Compute `chunker_strategy_hash` from build-time `env!("VERGEN_SHA")` or `include_str!("src/core/chunker.rs")` hash
- Write `_INDEX.toml` with all fingerprint axes used

On every `get()`:
- Validate the model's `_INDEX.toml` axes match the current chunker version
- If mismatch, return `None` (force re-embed) — never silently return wrong vector

### Task 2.4: mmap'd read path
**File**: `src/core/cache.rs`

Use `memmap2` crate (already in many project trees) or `std::fs::read`. Prefer mmap for files > 4KB (one embedding = ~16KB at dim=4096).

```rust
pub struct CachedEmbedding<'a> {
    pub dim: usize,
    pub data: &'a [f32],  // zero-copy slice
    pub embedded_at: SystemTime,
}
```

### Task 2.5: `chunker` versioning
**File**: `src/core/chunker.rs`

Add:
```rust
pub const CHUNKER_VERSION: &str = env!("LLMWIKI_CHUNKER_VERSION");
// In build.rs: compute SHA256 of src/core/chunker.rs
```

### Task 2.6: Refactor `embed` to write sparse index
**File**: `src/cli/embed.rs`

Replace `EmbeddingsFile::write_to` (which serializes vectors inline) with:
```rust
pub fn write_sparse_index(path: &Path, entries: &[SparseEntry]) -> Result<(), WikiError>;
```

Each entry: `{path, sha256, model, chunks: [{fp, start, end}], chunker_version, embedded_at}`.

### Task 2.7: Refactor `embed` to use central cache
**File**: `src/cli/embed.rs`

3-phase flow (SCAN / DIFF / WRITE) per spec §2.4. Each chunk:
1. Compute `EmbeddingFingerprint`
2. `cache.get(fp)` → if hit, reuse vector; if miss, NIM call → `cache.put(fp, vec)`
3. Append `{fp, start, end}` to sparse entry

### Task 2.8: Migration on first run
**File**: `src/cli/embed.rs`

Detect old-format `embeddings.jsonl` (has `chunks[].embedding` field):
- For each chunk: compute fp, write vector to central cache
- Rewrite page entry as sparse index
- Write `_meta` header
- Never delete old file until new is verified

### Task 2.9: `search` / `query` use central cache
**Files**: `src/cli/search.rs`, `src/cli/query.rs`

Replace inline vector reads from `embeddings.jsonl` with `cache.get(fp, model)` → zero-copy slice.

### Task 2.10: Query cache
**Files**: `src/cli/search.rs`, `src/cli/query.rs`

Embed query once → write to `~/.cache/llmwiki/queries/<model>/<fp>.bin`. On repeat queries with the same fingerprint → skip NIM call.

### Task 2.11: Tests
**File**: `tests/cache_test.rs` (NEW)

- `cache_hit_returns_same_vector` — embed twice, second is cache hit (zero NIM calls)
- `cache_corruption_on_model_change` — change `_INDEX.toml` axes → next get returns None
- `cache_mmap_zero_copy` — get returns slice that points to mapped memory
- `cache_gc_removes_orphans` — write 10 entries, gc with max_size of 1 entry, 9 removed
- `cache_verify_detects_corruption` — corrupt a .bin file, verify reports it
- `sparse_index_smaller_than_inline` — old format vs new format, new < 10% of old
- `migration_preserves_vectors` — start with old format, run embed, sparse index has same vectors
- `query_cache_dedupes_repeat_queries` — embed same query twice, second is free

### Task 2.12: CHANGELOG + version bump
v0.4.0 entry covering both Phase 1 + Phase 2.

**Commit checkpoint after Task 2.12** — Phase 2 makes embed/search ~50× cheaper on repeat runs.

---

## Phase 3: `cache` subcommand + GC (2 days)

### Task 3.1: `cache info`
**File**: `src/cli/cache.rs` (NEW)

```rust
pub struct CacheInfo {
    total_size_bytes: u64,
    total_entries: u64,
    hit_rate_24h: f32,           // from telemetry log
    oldest_entry_age_days: u32,
    models: Vec<ModelInfo>,      // per-model stats
    last_validated_at: SystemTime,
    chunker_version: String,
}
```

### Task 3.2: `cache gc`
**File**: `src/cli/cache.rs`

Flags: `--max-age 30d`, `--max-size 5GB`, `--dry-run`. Walks cache, removes entries that:
- Are older than max-age (if set)
- Push total over max-size (if set)
- Are not referenced by any wiki's `embeddings.jsonl` (orphan check)

Returns `ActionableResponse<GcReport>` with freed_bytes, entries_removed, next_step: "llmwiki-cli cache verify".

### Task 3.3: `cache verify`
**File**: `src/cli/cache.rs`

Reads each `.bin` file, validates:
- `_INDEX.toml` exists and matches current chunker version
- File is readable
- Fingerprint re-computation matches filename
- Optional: SHA256 of file matches embedded checksum

Returns `ActionableResponse<VerifyReport>` with errors as actionable items: "this .bin file is for model X, but `_INDEX.toml` says model Y — run `llmwiki-cli cache clear --model X` and re-embed".

### Task 3.4: `cache clear`
**File**: `src/cli/cache.rs`

Flags: `--model <id>` (only clear one model), `--confirm` (required, since destructive).

Returns `ActionableResponse<ClearReport>` with bytes_freed + next_step: "llmwiki-cli embed to repopulate".

### Task 3.5: Tests
**File**: `tests/cache_subcommand_test.rs` (NEW)

- `cache_info_reports_size_and_count`
- `cache_gc_dry_run_does_not_delete`
- `cache_gc_removes_old_entries`
- `cache_verify_detects_axis_mismatch`
- `cache_clear_requires_confirm_flag`

### Task 3.6: Wire `cache` into `cli::Command`
**File**: `src/cli/mod.rs`

Add `Cache(crate::cli::cache::CacheArgs)` variant + dispatch.

---

## Phase 4: Auto-embed on first `search` (1 day)

### Task 4.1: `search` detects empty cache and triggers embed
**File**: `src/cli/search.rs`

If the resolved workspace (or all fleet candidates) has no `embeddings.jsonl` or empty `pages` field → automatically run `embed` first, then search.

User can opt out: `--no-auto-embed` (just error with the actionable hint).

### Task 4.2: Auto-embed progress
**File**: `src/cli/search.rs`

Print progress to stderr: "Auto-embedding N pages... this may take a few minutes." Don't pollute stdout (where results go).

### Task 4.3: Tests
- `search_auto_embeds_empty_wiki` — search before embed → runs embed, returns results
- `search_no_auto_embed_errors` — `--no-auto-embed` on empty wiki → errors with actionable hint

---

## Phase 5: Polish + docs + skill updates (2 days)

### Task 5.1: Update hub SKILL.md
**File**: `skills/SKILL.md`

Document:
- Central cache location + lifecycle
- `cache` subcommand
- Actionable response envelope (mention JSON output is stable contract)

### Task 5.2: Update sub-skills
**Files**: `skills/data/llmwiki-*.md`

- `llmwiki-setup.md`: add cache location to post-install steps
- `llmwiki-config.md`: add cache.gc section
- `llmwiki-sync.md`: document cache portability considerations
- `llmwiki-troubleshooting.md`: add cache corruption troubleshooting

### Task 5.3: AGENTS.md updates
**File**: `AGENTS.md`

Add sections:
- "Central cache architecture" (where it lives, what fingerprint axes are)
- "ActionableResponse contract" (every command's JSON shape)
- "Migration from v0.3.x" (what to expect on first v0.4.0 run)

### Task 5.4: Final verification
- `cargo test --all` (expect ~340-360 tests passing)
- `cargo clippy --all-targets -- -D warnings` clean
- `cargo fmt --check` clean
- `cargo deny check` clean
- Real-wiki smoke test: embed → search → cache info → gc (verify no errors)
- Cross-version migration test: load old-format `embeddings.jsonl`, run embed, verify sparse migration
- Auto-embed test: search before embed on real wiki

### Task 5.5: Tag + push
- Bump Cargo.toml to v0.4.0
- Tag v0.4.0 at HEAD, push
- Update release/v0.3.36 branch (or create release/v0.4.0)
- Open PR for review

---

## What NOT to do (out of scope, deferred)

- **Semantic cache hits** (FAISS HNSW) — defer until hit rate < 80%
- **GPU cache tier** (cudarc) — defer until user request
- **Multi-node cache** — out of scope for single-developer CLI
- **In-flight dedup** — overkill for CLI's single-call-at-a-time pattern
- **Cache encryption at rest** — chunks aren't secrets
- **Telemetry / hit-rate tracking** — would need a separate log; defer to v0.5

---

## Risk mitigations

| Risk | Mitigation |
|---|---|
| Cache corruption | `_INDEX.toml` per-model axis validation; `cache verify` subcommand |
| User clears cache, loses everything | Document in CHANGELOG; `cache clear` requires `--confirm` |
| Sparse index regression at 100K+ chunks | Switch to SQLite (deferred to v0.5) |
| `ActionableResponse` boilerplate per command | One-line `print_ok()` helper, applied at dispatch boundary |
| Windows mmap differences | Use `memmap2` crate (cross-platform) |
| Migration data loss | Write new file → verify → only then delete old |
| `next_steps` content drift | Golden JSON snapshot test in CI |

---

## Commit cadence

After each task:
1. Verify: `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`
2. Commit with conventional message
3. Push to `main`
4. (Phase boundaries) bump version, tag, push tag, update release branch

---

## Final acceptance

- **Cache hit rate > 90%** on a typical second-run `embed`
- **All commands** produce structured `ActionableResponse<T>` output
- **Zero data loss** during v0.3.x → v0.4.0 migration (verified by integration test)
- **No silent corruption** when model/tokenizer/chunker changes (verified by corruption test)
- **Agent success rate**: a coding agent given only `llmwiki-cli --help` can complete init → embed → search without external docs