# Lint

Run `wiki lint` periodically and before committing broad changes.

```bash
wiki lint                     # all checks
wiki lint --scope wiki        # only wiki pages
wiki lint --strict            # warnings become errors
wiki lint --json              # machine-readable
```

Checks:
- **Frontmatter**: missing `title`/`created`/`updated`/`tags`/`sources`, bad filename casing
- **Wikilinks**: dangling `[[links]]`, dead-end pages (0 outbound), orphans (0 inbound)
- **Footnotes**: used-but-not-defined, defined-but-not-used, duplicate IDs
- **Index.md**: page not in index, index pointing to missing page
- **Raw**: missing frontmatter, sha256 drift
- **Log**: malformed `## [date] action | desc` entries

Exit codes:
- 0 = no issues
- 2 = errors found (blocking)

Run lint after every ingest, before commits, and on every new device setup.