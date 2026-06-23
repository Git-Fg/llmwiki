#!/usr/bin/env bash
# Verify the SETUP/SKILL.md content is correct (greppable invariants).

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SKILL="$ROOT/marketplace/skills/wiki/SETUP/SKILL.md"

# Must have the three-step block.
grep -q 'command -v llmwiki-cli' "$SKILL" || { echo "FAIL: missing detect step"; exit 1; }
grep -q 'curl -LsSf' "$SKILL" || { echo "FAIL: missing install step"; exit 1; }
grep -q 'llmwiki-cli doctor' "$SKILL" || { echo "FAIL: missing verify step"; exit 1; }

# Any `| bash` pattern must be in a curl context (not "run install.sh for the user").
# `| bash` outside a `curl ... | bash` line would mean the skill is asking the
# agent to pipe some other command into bash — anti-pattern.
if grep -E '\| bash' "$SKILL" | grep -v 'curl.*\| bash' | grep -q .; then
  echo "FAIL: non-curl '| bash' pattern found (auto-run risk)"
  exit 1
fi

# Must have --check mode documented.
grep -q -- '--check' "$SKILL" || { echo "FAIL: --check mode not documented"; exit 1; }

# Must have anti-patterns section forbidding auto-run.
grep -q '## Anti-patterns' "$SKILL" || { echo "FAIL: missing anti-patterns section"; exit 1; }
grep -q 'Do NOT run .install.sh.' "$SKILL" || { echo "FAIL: anti-pattern about not running install.sh missing"; exit 1; }

# Must cross-reference references/install.md (added in Task 3.2).
grep -q 'references/install.md' "$SKILL" || { echo "FAIL: missing cross-reference to references/install.md"; exit 1; }

echo "✓ SETUP skill passes smoke test"