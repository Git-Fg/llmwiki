# Security Policy

## Supported Versions

| Version | Supported          |
|---------|--------------------|
| 0.3.x   | :white_check_mark: |
| < 0.3   | :x:                |

## Reporting a Vulnerability

**Please do not report security vulnerabilities through public GitHub issues.**

Open a private security advisory instead:

1. Go to <https://github.com/Git-Fg/llmwiki/security/advisories/new>
2. Fill in the title and description
3. Submit

You should receive an initial response within 72 hours. If you don't, follow up
via a public issue tagged `@Git-Fg` asking for triage.

## Scope

In scope:
- Code execution via crafted `wiki-root.toml`, `raw/`, or `.wiki/config.yaml`
- NIM API key leakage (config, logs, error messages, telemetry)
- Path traversal in `init`, `ingest`, or `wiki config path`
- Unsafe `unwrap()`/`expect()` reachable from public CLI commands
- Memory unsafety in `unsafe` blocks (currently none in the codebase)

Out of scope:
- Vulnerabilities in upstream dependencies — report upstream
- Social engineering
- Physical access attacks

## Disclosure Policy

- We follow coordinated disclosure: 90 days from report to public disclosure,
  with extensions for complex issues.
- Credit is given in the CHANGELOG and GitHub Security Advisory unless you
  prefer anonymity.
