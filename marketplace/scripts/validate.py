#!/usr/bin/env python3
"""Validate marketplace plugin format. Stdlib only.

Mirrors taches-principled-light's marketplace-validator.
Exit codes: 0 = pass, 1 = warn, 2 = fail.
"""
import argparse
import json
import re
import sys
from pathlib import Path

HARDCODED_TOOL_NAMES = [
    "Agent", "Bash", "Read", "Edit", "Glob", "Grep",
    "Write", "WebSearch", "WebFetch", "TaskOutput",
]

def fail(msg, path=None):
    print(f"FAIL: {msg}" + (f" [{path}]" if path else ""))
    return 2

def warn(msg, path=None):
    print(f"WARN: {msg}" + (f" [{path}]" if path else ""))
    return 1

def info(msg, path=None):
    print(f"INFO: {msg}" + (f" [{path}]" if path else ""))
    return 0

def validate_skill(path: Path) -> int:
    rc = 0
    text = path.read_text(encoding="utf-8")
    if not text.startswith("---\n"):
        rc |= fail("missing YAML frontmatter", path)
        return rc

    parts = text.split("---\n", 2)
    if len(parts) < 3:
        rc |= fail("unterminated YAML frontmatter", path)
        return rc

    front, body = parts[1], parts[2]
    meta = {}
    for line in front.splitlines():
        if ":" in line and not line.startswith(" "):
            k, _, v = line.partition(":")
            meta[k.strip()] = v.strip()

    for required in ["name", "description"]:
        if required not in meta:
            rc |= fail(f"missing frontmatter field: {required}", path)

    desc = meta.get("description", "")
    if len(desc) > 1024:
        rc |= fail(f"description too long ({len(desc)} chars, max 1024)", path)

    body_lines = body.splitlines()
    if len(body_lines) > 500:
        rc |= warn(f"body has {len(body_lines)} lines (soft cap 500)", path)

    expected_name = path.parent.name.lower()
    if meta.get("name") and meta["name"] != expected_name:
        rc |= fail(f"frontmatter name '{meta['name']}' != dir name '{expected_name}'", path)

    # Warn on hardcoded tool names *only in body prose* — skip the
    # `allowed-tools:` and `skillInstructions:` lines where tool names
    # are legitimate (e.g. `Bash(llmwiki-cli:*)`, `Agent`, `Read`).
    prose = "\n".join(
        line for line in body.splitlines()
        if not line.startswith(("allowed-tools:", "skillInstructions:"))
        and "Bash(" not in line
    )
    for bad_name in HARDCODED_TOOL_NAMES:
        if re.search(rf"\b{bad_name}\b", prose):
            rc |= warn(f"hardcoded tool name '{bad_name}' found in body prose", path)

    # References integrity
    for match in re.finditer(r"references/([\w\-/]+\.md)", body):
        ref = path.parent / match.group(1)
        if not ref.exists():
            rc |= fail(f"broken reference: {match.group(0)}", path)

    return rc

def validate_manifest(path: Path) -> int:
    rc = 0
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as e:
        return fail(f"invalid JSON: {e}", path)

    if "name" not in data:
        rc |= fail("manifest missing 'name'", path)
    if "version" not in data:
        rc |= fail("manifest missing 'version'", path)

    return rc

def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--strict", action="store_true")
    ap.add_argument("--json", action="store_true")
    args = ap.parse_args()

    marketplace = Path(__file__).parent.parent
    rc = 0

    # Validate plugin manifests.
    for plugin_dir in marketplace.glob(".claude-plugin"):
        for f in plugin_dir.glob("*.json"):
            rc |= validate_manifest(f)

    # Validate every SKILL.md.
    for skill_md in marketplace.rglob("SKILL.md"):
        if "skills/" in str(skill_md):
            rc |= validate_skill(skill_md)

    if args.strict and rc == 1:
        rc = 2  # warnings also fail in strict mode

    if args.json:
        print(json.dumps({"exit_code": rc}))

    return rc

if __name__ == "__main__":
    sys.exit(main())
