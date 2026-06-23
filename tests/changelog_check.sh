#!/usr/bin/env bash
# Asserts that CHANGELOG.md has a heading matching the current Cargo.toml
# version, with a YYYY-MM-DD date. Prevents the v0.3.18-class mistake of
# shipping a release with an inconsistent CHANGELOG.
#
# Exit codes:
#   0 = match
#   1 = version mismatch
#   2 = date missing or malformed
#   3 = heading not found

set -euo pipefail

version=$(grep '^version = ' Cargo.toml | head -1 | sed -E 's/^version = "([^"]+)".*/\1/')
if [ -z "$version" ]; then
    echo "ERROR: could not extract version from Cargo.toml" >&2
    exit 3
fi

heading_pattern="^## \[${version}\] - ([0-9]{4}-[0-9]{2}-[0-9]{2})"

# Extract the matching line (and a few surrounding lines for context)
match=$(grep -E "^## \[${version}\]" CHANGELOG.md || true)
if [ -z "$match" ]; then
    echo "ERROR: no CHANGELOG heading for version ${version}" >&2
    echo "  expected: '## [${version}] - YYYY-MM-DD'" >&2
    echo "  found top 3 headings:" >&2
    grep -E "^## \[" CHANGELOG.md | head -3 >&2
    exit 1
fi

# Check the date format
if ! echo "$match" | grep -qE "$heading_pattern"; then
    echo "ERROR: CHANGELOG heading for ${version} missing YYYY-MM-DD date" >&2
    echo "  found: '${match}'" >&2
    echo "  expected: '## [${version}] - YYYY-MM-DD'" >&2
    exit 2
fi

date_str=$(echo "$match" | sed -E "s/$heading_pattern/\1/")
echo "OK: CHANGELOG has '## [${version}] - ${date_str}'"
