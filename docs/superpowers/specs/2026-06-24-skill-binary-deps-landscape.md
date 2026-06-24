# Skill binary-dependency declaration — landscape research (June 2026)

**Status:** Research synthesis. Durable reference for future skill frontmatter decisions.
**Date:** 2026-06-24
**Author:** Web research informed by `agentskills.io` spec, `vercel-labs/skills`, `anthropics/skills`, OpenClaw docs, Claude Code plugin-marketplaces docs, and Reversec Labs May 2026 security research.
**Question this answers:** When a SKILL.md needs a binary on `PATH` (e.g. `llmwiki-cli`), does any agent host provide a structured, host-enforced mechanism to declare that requirement in frontmatter?

---

## TL;DR

**No host uses a first-class `requires.bins` (or equivalent) frontmatter field.** The agentskills.io spec (June 2026) defines six frontmatter fields and none of them is a structured binary-declaration. The only host that ships a structured mechanism is OpenClaw, via a vendor-namespaced `metadata.openclaw.requires.bins` extension — verified primary source, never upstreamed. Claude Code's "Sandbox dependencies missing" startup warning is about the sandbox *runtime* packages (`bubblewrap`, `socat`, `ripgrep`, `seccomp`), not skill-side binary deps. Claude Code plugin-marketplaces have plugin-to-plugin dependency resolution (`allowCrossMarketplaceDependenciesOn`), which is a different layer entirely.

**Implication for our v0.3.33 hub:** the `compatibility` field (free-text 1–500 chars, spec-compliant) is the correct choice. Our existing text ("Requires the llmwiki-cli binary on PATH and network access to NVIDIA NIM…") is right.

---

## 1. The agentskills.io spec — six fields, no `requires`

Source: <https://agentskills.io/specification> (primary).

| Field | Required | Constraints |
|---|---|---|
| `name` | Yes | 1–64 chars; lowercase letters, digits, hyphens; must not start/end with hyphen; **must match parent directory name** |
| `description` | Yes | 1–1024 chars; should describe what + when |
| `license` | No | Short string or reference to bundled license file |
| `compatibility` | No | 1–500 chars; "intended product, required system packages, network access needs, etc." |
| `metadata` | No | Arbitrary string→string map; clients may use for non-spec properties |
| `allowed-tools` | No | Space-separated pre-approved tools (experimental) |

The `metadata` field is the only documented escape hatch. It does not define any sub-schema — clients use it for "additional properties not defined by the Agent Skills spec". This is where OpenClaw's `requires.bins` lives.

**Independent witnesses:**
- [github.com/anthropics/skills](https://github.com/anthropics/skills) — "The frontmatter requires only two fields: `name` and `description`" (Anthropic's reference implementation).
- [lib.rs/crates/skills-cli](https://lib.rs/crates/skills-cli) — "Same SKILL.md format and YAML frontmatter schema" (vercel-labs/skills Rust rewrite, Jan 2026).
- [agentman.ai/blog](https://agentman.ai/blog/build-your-first-agent-skill-skillmd-anatomy) — third-party tutorial confirming the six-field schema.

**No proposals found (as of 2026-06-22):** No open RFC or working-group discussion on a `requires` namespace in agentskills.io. The six-field schema has been stable since the spec's first public release.

---

## 2. OpenClaw is the lone structured exception

Source: <https://docs.openclaw.ai/tools/skills> (primary) and <https://github.com/openclaw/openclaw/blob/main/docs/tools/skills.md> (source).

OpenClaw uses `metadata.openclaw.requires.bins` as a vendor-namespaced extension:

```yaml
metadata:
  openclaw:
    requires:
      bins: ["llmwiki-cli"]
```

Semantics (from the spec):
- `requires.bins` is checked on the **host** at skill load time.
- If the agent runs in a sandbox, the binary must also exist **inside the container**.
- The check fails-closed: a missing binary gates skill activation.

OpenClaw also uses `metadata.openclaw.requires.config` for gating on config-key presence.

**Independent witnesses:**
- <https://gdplabs.gitbook.io/sdk/gl-connectors/sdk/connectors-skills/references/openclaw-skills> (GL SDK integration guide).
- <https://fcodeai.mintlify.app/tools/skills> (third-party mirror, "Fased").

**OpenClaw is a vercel-labs/skills supported agent** (path: `~/.openclaw/skills/`, project path: `skills/`). It's discoverable by `npx skills add`, but the install path is OpenClaw-specific.

**Dissent on the record:** OpenClaw's extension is not part of agentskills.io. Other agents that respect the spec strictly (Claude Code's plugin loader, Cursor, Codex) will ignore `metadata.openclaw.*` entirely — there is no portable enforcement. A skill author who adds only `metadata.openclaw.requires.bins` will gate loading only on OpenClaw; everywhere else the skill loads unconditionally.

---

## 3. Claude Code — what the "dependencies" UI actually checks

### 3.1 Sandbox dependencies (NOT skill-side)

Source: <https://docs.claude.com/en/docs/claude-code/sandboxing> (primary).

The `/sandbox` command opens a panel with three tabs: **Mode**, **Overrides**, **Config**. When the Linux/WSL2 sandbox can't start because a required package is missing, the panel shows **only** a Dependencies tab:

> "After installing, the Dependencies tab in `/sandbox` shows whether `ripgrep`, `bubblewrap`, `socat`, and the seccomp filter are available on your platform."

- `ripgrep` — bundled with the native Claude Code binary.
- `bubblewrap` — Linux/WSL2 sandbox; `apt-get install bubblewrap` / `dnf install bubblewrap`.
- `socat` — Linux/WSL2 network relay; `apt-get install socat` / `dnf install socat`.
- `seccomp filter` — optional; `npm install -g @anthropic-ai/sandbox-runtime`.

This is the runtime side, not the skill side. Claude Code v2.1.187 removed the startup "setup issues" line under the logo and now exposes the same checks via `/doctor`.

### 3.2 Plugin-marketplaces — plugin-to-plugin, not binary

Source: <https://code.claude.com/docs/en/plugin-marketplaces> (primary).

Claude Code's marketplace schema has `allowCrossMarketplaceDependenciesOn` — an array of other marketplaces a marketplace may pull plugins from. The check runs on `marketplace add`, `install`, `update`, `refresh`, and `auto-update`. This is **plugin-level** dependency resolution (other plugins), not a binary-presence check.

Plugin entries in `marketplace.json` can declare component paths (`skills`, `commands`, `agents`, `hooks`, `mcpServers`, `lspServers`) but **no binary requirement field** at any level — skill, plugin, or marketplace.

The Claude Code Plugin reference (code.claude.com/docs/en/plugins-reference) confirms: skill components are directories with `SKILL.md` (and optional `reference.md`, `scripts/`); no binary-dep frontmatter documented.

---

## 4. Vendor spec-divergence: a real and ongoing risk

The Copilot CLI Issue #894 (Jan 6, 2026, closed Feb 25, 2026) is a useful cautionary tale:

> "When using Copilot CLI skills, validation/loading appears to require a `license:` property in SKILL.md YAML frontmatter. Per the Agent Skills spec, license is optional in the SKILL.md format."

Source: <https://github.com/github/copilot-cli/issues/894>. This was a vendor bug (Copilot CLI enforcing an optional field as required), closed as fixed. But the pattern repeats: **vendors add their own validation on top of the spec.** Adding `metadata.<vendor>.*` to our hub is safe (ignored by spec-compliant agents) but `compatibility` text is the only field guaranteed to be read by all spec-compliant agents.

---

## 5. Security context: skills are instructions to a binary

Reversec Labs published a multi-part research series in May 2026 titled "Skill Issues: Compromising Claude Code with malicious skills & agents":

> "since skill files effectively act as instructions to a binary with command execution and file write controls, is this not equivalent to downloading and running random executables, or poisoning supply-chain packages?"

Source: <https://labs.reversec.com/posts/2026/05/skill-issues-compromising-claude-code-with-malicious-skills-agents-part-1>.

Key takeaways for hub authors:
- Hosts do **not** sandbox skill bodies at load time. Skill text is advisory.
- A `requires.bins` field (where implemented, e.g. OpenClaw) is a *capability gate*, not a *safety mechanism* — it tells the agent "you'll need this binary", not "this binary is safe".
- Minimal hubs (small attack surface) are strictly better than feature-rich hubs. Our 40-line v0.3.33 hub has a smaller blast radius than alternatives that embed troubleshooting workflows.

This confirms our v0.3.33 hub's posture: spec-compliant `compatibility` field, no vendor extensions, no scripts, no references, minimal frontmatter.

---

## 6. Implications for our v0.3.33 hub

### What we already do (correct)

```yaml
compatibility: |
  Requires the llmwiki-cli binary on PATH and network access to NVIDIA NIM
  (https://integrate.api.nvidia.com).
```

- ✅ Spec-compliant (within 500-char limit).
- ✅ Read by every spec-compliant agent at metadata-load time.
- ✅ Communicates the binary requirement + network requirement in plain text.
- ✅ Actionable: the agent sees the requirement and can decide whether to proceed.

### What we explicitly do NOT do (correct, given §1–§5)

- ❌ No `metadata.openclaw.requires.bins: ["llmwiki-cli"]` — would gate only OpenClaw users; ignored by everyone else; no portable enforcement.
- ❌ No `requires.bins` at the spec level — the field doesn't exist; making one up would violate the spec.
- ❌ No `metadata.requires.*` — same reason; speculation.
- ❌ No `allowed-tools` extension — experimental per spec; spec explicitly warns "support for this field may vary between agent implementations". We already have `Bash(llmwiki-cli:*)` and that's the only thing we need.

### When to revisit

Reconsider adding `metadata.openclaw.requires.bins` **only** when:
1. An OpenClaw user explicitly requests the gated-load behavior, **and**
2. The agentskills.io working group has not added a vendor-neutral `requires` field, **and**
3. The cost (one extra metadata block, ~50 bytes) is justified by an actual user, not by spec speculation.

---

## 7. Anti-patterns observed in the wild

From the research:

| Anti-pattern | Example | Why it's bad |
|---|---|---|
| Spec-divergence without warning | Copilot CLI enforcing `license:` as required | Breaks skills written strictly to the spec |
| Hidden tool installation in skill body | "Run `pip install foo`" in skill markdown | Reversec Labs: skill bodies are instructions to a binary; hidden installs are supply-chain attacks |
| `metadata.<vendor>.requires.bins` as sole gate | OpenClaw-only gating when other agents also load the skill | Misleading — implies enforcement that doesn't exist cross-host |
| Bundled scripts in skill | `skills/my-tool/scripts/setup.sh` | 12% of skills on skills.sh had trojanized install scripts (Snyk/ClawHavoc, Jan 2026) |
| Verbose `description` field | >500 chars describing every feature | Burns agent context at startup; metadata loaded for ALL skills |

Our v0.3.33 hub avoids all five.

---

## 8. Open questions for future research

1. **Spec evolution:** Will agentskills.io add a vendor-neutral `requires.bins` (or `requires.*`) field in 2026 H2? Track the agentskills GitHub repo and Discord for RFCs.
2. **Cross-host enforcement:** Will any non-OpenClaw host (Claude Code, Cursor, Codex, Gemini CLI) add a host-side binary-presence check before skill load? Claude Code's `/plugin` Skills section (added v2.1.186) is the most likely vector.
3. **ClawHub trust envelope:** OpenClaw's `clawhub.skill.verify.v1` is a publisher-side mechanism. Could a similar trust envelope become a cross-host standard (e.g. a `metadata.trust.*` block)?
4. **Reversec Labs Part 2/3:** The research series is multi-part; later installments may surface defensive recommendations worth folding into our `compatibility` text.

---

## 9. References (primary sources)

- <https://agentskills.io/specification> — canonical spec
- <https://github.com/anthropics/skills> — Anthropic reference implementation
- <https://github.com/vercel-labs/skills> — npx skills add (72+ agents)
- <https://docs.openclaw.ai/tools/skills> — OpenClaw spec + `requires.bins` extension
- <https://github.com/openclaw/openclaw/blob/main/docs/tools/skills.md> — OpenClaw source
- <https://code.claude.com/docs/en/plugin-marketplaces> — Claude Code plugin-marketplaces
- <https://code.claude.com/docs/en/plugins-reference> — Claude Code plugin components
- <https://docs.claude.com/en/docs/claude-code/sandboxing> — Claude Code sandbox deps
- <https://github.com/github/copilot-cli/issues/894> — Copilot CLI spec-divergence example
- <https://labs.reversec.com/posts/2026/05/skill-issues-compromising-claude-code-with-malicious-skills-agents-part-1> — Reversec Labs May 2026 security research

## 10. Implications for future llmwiki-cli versions

- **v0.3.34+:** Do not add `metadata.openclaw.requires.bins` to the hub unless an OpenClaw user requests it.
- **v0.4.0+:** If agentskills.io adds a vendor-neutral `requires` field, migrate our `compatibility` text to the structured field. Track the spec repo.
- **Any version:** Do not bundle scripts in `skills/SKILL.md`. The hub stays a single file.
- **Any version:** Treat `compatibility` text as agent-facing requirements documentation, not as a security boundary.