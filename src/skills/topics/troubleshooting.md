# Troubleshooting

**`workspace not found`** — You're not in a wiki folder. Run `wiki --workspace /path/to/wiki <cmd>` or `cd ~/my-wiki` first.

**`NIM API key not set`** — `export NVIDIA_NIM_API_KEY="nvapi-..."` in your shell rc, or pass it inline.

**`NIM unreachable`** — Check `wiki doctor`. Common causes: bad API key (401), rate limit (429), network issue. The CLI retries with backoff on 5xx.

**`no embeddings yet`** — Run `wiki embed` first.

**`No matches for query`** — Try lowering `--threshold` (default 0.3), broadening the query, or running `wiki embed --skip-existing` to refresh stale vectors.

**`lint: missing-frontmatter`** — Add YAML frontmatter with required fields.

**`file changed during embed`** — Re-run `wiki embed`. The JSONL is atomic on write, so partial state is impossible.

**`git push rejected`** — Likely a non-fast-forward. Run `git pull --rebase` then push.

**Embeddings slow on first run** — First embed re-computes everything from scratch. Subsequent runs use `--skip-existing` and only embed changed files (by sha256).

**Out of disk** — embeddings.jsonl is gitignored; clean it up with `rm embeddings.jsonl && wiki embed`. The wiki content is unaffected.

**Wrong model in embeddings** — Edit `nim.embed_model` in `.wiki/config.yaml` and re-run `wiki embed`. Old entries with previous models are kept (different model entries coexist in the JSONL).