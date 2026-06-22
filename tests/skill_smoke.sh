#!/usr/bin/env bash
set -euo pipefail

# 1. Skill files exist
test -f agents/skills/wiki/SKILL.md || { echo "FAIL: SKILL.md missing"; exit 1; }

# 2. SKILL.md mentions key commands
for cmd in init ingest embed search query lint models doctor status skill; do
    grep -q "llmwiki-cli $cmd" agents/skills/wiki/SKILL.md || { echo "FAIL: SKILL.md missing llmwiki-cli $cmd"; exit 1; }
done

# 3. `llmwiki-cli skill show` returns content
content=$(cargo run --quiet -- skill show)
echo "$content" | grep -q "Wiki Skill" || { echo "FAIL: skill show empty"; exit 1; }

# 4. `llmwiki-cli skill list` returns topics
cargo run --quiet -- skill list | grep -q "setup"
cargo run --quiet -- skill list | grep -q "troubleshooting"

# 5. `llmwiki-cli skill show search` returns search topic
cargo run --quiet -- skill show search | grep -q "Search"

echo "OK: skill smoke test passed"