#!/usr/bin/env bash
# v0.3.29 skill smoke test — validates the new minimal-stub layout
# (no marketplace, no agents/skills/ mirror, no per-subdir install).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

fail() { echo "FAIL: $*" >&2; exit 1; }

# 1. Source-of-truth files exist
test -f skills/SKILL.md || fail "skills/SKILL.md missing"
for sub in setup config ingest search query lint models sync troubleshooting; do
    test -f "skills/wiki-$sub.md" || fail "skills/wiki-$sub.md missing"
done

# 2. Hub SKILL.md mentions key commands
for cmd in ingest doctor skill; do
    grep -q "llmwiki-cli $cmd" skills/SKILL.md \
        || fail "SKILL.md missing 'llmwiki-cli $cmd'"
done

# 3. `llmwiki-cli skill` (no args) returns hub content
content=$(cargo run --quiet -- skill)
echo "$content" | grep -q "name: wiki" || fail "skill (hub) missing frontmatter"

# 4. `llmwiki-cli skill list` enumerates inline sub-skills (rust-embed)
cargo run --quiet -- skill list | grep -q "wiki-setup"   || fail "wiki-setup missing from skill list"
cargo run --quiet -- skill list | grep -q "wiki-troubleshooting" || fail "wiki-troubleshooting missing from skill list"

# 5. `llmwiki-cli skill get <topic>` returns the sub-skill content
#    (canonical discovery primitive, matches agent-browser's `skills get`).
cargo run --quiet -- skill get wiki-search | grep -q "name: wiki-search" \
    || fail "skill get wiki-search missing frontmatter"
cargo run --quiet -- skill get search | grep -q "name: wiki-search" \
    || fail "skill get search (short form) missing frontmatter"

# 6. `llmwiki-cli install-skill --workspace <tmp>` installs only the hub.
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
cargo run --quiet -- install-skill --workspace "$TMP" >/dev/null
test -f "$TMP/.agents/skills/wiki/SKILL.md" \
    || fail "hub SKILL.md not installed to $TMP/.agents/skills/wiki/"
# Sub-skills are NOT installed (v0.3.29 simplification).
test ! -d "$TMP/.agents/skills/wiki/SETUP" \
    || fail "stale SETUP subdir exists — should not be installed in v0.3.29"

echo "✓ skill smoke test passed"