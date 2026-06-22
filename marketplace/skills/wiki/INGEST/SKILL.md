---
name: ingest
description: |
  Add a new source file to `raw/`, append a log entry, and compile it
  into a wiki page. Use when the user says "add this to my wiki",
  "ingest a source", "compile a new page", or "add a note".
whenToUse: |
  Do NOT use for searching or querying the wiki.
allowed-tools: Bash(llmwiki-cli:*), Read, Write
---

# Wiki — Ingest

## Workflow

1. **Add the source file** to the `raw/` subdirectory:
   ```bash
   llmwiki-cli ingest path/to/source.md
   ```
   This computes SHA256, writes frontmatter, appends a log entry, and (unless `--no-compile`) compiles the source into a page in `wiki/`.

2. **Verify the page**:
   ```bash
   llmwiki-cli ls --pages
   ```

3. **Add wikilinks** between pages so the graph stays connected.

4. **Embed the new pages** (so semantic search picks them up):
   ```bash
   llmwiki-cli embed
   ```

## Multiple wikis

- `--wiki <alias>` selects which wiki to ingest into
- `llmwiki-cli config add <alias> <path>` registers a new wiki first

## Common flags

- `--no-compile` — add to raw/ but don't compile
- `--source-type <ext>` — hint the parser about the source type

## See also

- `llmwiki-cli search` — semantic search
- `llmwiki-cli query` — RAG question-answering
