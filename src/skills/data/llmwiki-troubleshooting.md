---
name: llmwiki-troubleshooting
description: |
  Diagnose wiki errors. Routes to `llmwiki-cli doctor`,
  `llmwiki-cli config validate`, and the per-command `--help`. Use
  when `doctor` reports a failure, `embed`/`search`/`query` returns
  empty, or any command exits non-zero unexpectedly.
  Do NOT use for: routine wiki work; first-run install (wiki-setup).
allowed-tools: Bash(llmwiki-cli:*)
---

# llmwiki-troubleshooting

Diagnose what's broken when a wiki command fails or returns wrong results.

## Commands

```bash
llmwiki-cli doctor                       # full diagnostic (config + NIM + workspace)
llmwiki-cli doctor --json                # machine-readable diagnostic
llmwiki-cli config validate              # catch typos / bad model names
llmwiki-cli config show-effective        # see what's actually loaded
```

## Common symptoms

| Symptom | First thing to try |
|---|---|
| `command not found` | `wiki-setup` |
| `doctor` reports no API key | `export NVIDIA_NIM_API_KEY=...` then retry |
| `doctor` reports no NIM connectivity | check `WIKI_NIM_BASE_URL` override |
| `search` / `query` returns empty after a content change | `llmwiki-cli embed` |
| `embed` fails with model error | `wiki-models` to switch to a whitelisted model |
| `lint` flags unknown key | `wiki-config` to fix the typo |

## Sibling skills

- `wiki-setup` — install / bootstrap issues
- `wiki-config` — config typos / wrong values
- `wiki-models` — bad embedding / reranking model

## Full reference

```bash
llmwiki-cli doctor --help
```