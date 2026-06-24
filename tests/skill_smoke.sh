#!/usr/bin/env bash
# v0.3.36 skill smoke test — validates the new llmwiki entrypoint-only layout.
# (Hub at skills/SKILL.md + sub-skills at src/skills/data/llmwiki-*.md;
# no flat wiki-*.md siblings in skills/.)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

fail() { echo "FAIL: $*" >&2; exit 1; }

# 1. Source-of-truth files exist
test -f skills/SKILL.md || fail "skills/SKILL.md (hub) missing"
for sub in setup config ingest search query lint models sync troubleshooting; do
    test -f "src/skills/data/llmwiki-$sub.md" || fail "src/skills/data/llmwiki-$sub.md missing"
done

# 2. Hub SKILL.md mentions key commands
for cmd in skill doctor; do
    grep -q "llmwiki-cli $cmd" skills/SKILL.md \
        || fail "SKILL.md missing 'llmwiki-cli $cmd'"
done

# 3. `llmwiki-cli skill` (no args) returns hub content (from include_str!)
content=$(cargo run --quiet -- skill)
echo "$content" | grep -q "name: llmwiki" || fail "skill (hub) missing frontmatter"

# 4. `llmwiki-cli skill list` enumerates sub-skills (from src/skills/data/)
cargo run --quiet -- skill list | grep -q "llmwiki-setup"   || fail "llmwiki-setup missing from skill list"
cargo run --quiet -- skill list | grep -q "llmwiki-troubleshooting" || fail "llmwiki-troubleshooting missing from skill list"

# 5. `llmwiki-cli skill get <topic>` returns the sub-skill content
#    (canonical discovery primitive, matches agent-browser's `skills get`).
cargo run --quiet -- skill get llmwiki-search | grep -q "name: llmwiki-search" \
    || fail "skill get llmwiki-search missing frontmatter"
cargo run --quiet -- skill get search | grep -q "name: llmwiki-search" \
    || fail "skill get search (short form) missing frontmatter"

# 6. `llmwiki-cli install-skill --workspace <tmp>` installs ONLY the hub.
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
cargo run --quiet -- install-skill --workspace "$TMP" >/dev/null
test -f "$TMP/.agents/skills/llmwiki/SKILL.md" \
    || fail "hub SKILL.md not installed to $TMP/.agents/skills/llmwiki/"
# Sub-skills are NOT installed (v0.3.29 simplification, v0.3.33 layout).
# Flat llmwiki-*.md files must NOT appear at the install target — they live
# in src/skills/data/ and are CLI-internal only.
for sub in setup config ingest search query lint models sync troubleshooting; do
    test ! -f "$TMP/.agents/skills/llmwiki/llmwiki-$sub.md" \
        || fail "stale llmwiki-$sub.md installed to disk; should be CLI-internal"
done

# 7. v0.3.36 hard-cut guard: legacy `wiki-X` topic names return error
cargo run --quiet -- skill get wiki-search 2>&1 | grep -qE "unknown|not found" \
    || fail "skill get wiki-search (legacy) should error; it should NOT be a back-compat alias"

echo "✓ skill smoke test passed"