# Ingest

Drop a source file into `raw/articles/` (or another `raw/` subdir), then:

```bash
wiki ingest raw/articles/filename.md
```

This:
1. Computes `sha256` of the source
2. Adds raw frontmatter with provenance (source_url, ingested, sha256)
3. Triggers a compile pass: LLM reads the source, creates/updates wiki pages in `wiki/`, links them from related pages, updates `index.md`, and appends a structured entry to `log.md`.

For URLs: `wiki ingest https://example.com/article` — downloads via curl and proceeds as above.

Use `--no-compile` if you only want to stage the source without compiling.

Best practice: ingest one source at a time, read the resulting wiki pages, confirm they look right, then commit (`git add . && git commit -m "ingest: filename"`).