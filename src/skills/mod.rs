//! Skill bundle embedding.
//!
//! The hub at `skills/SKILL.md` is the only agent-facing artifact. It is
//! `include_str!`'d at build time so `llmwiki-cli install-skill` and
//! `npx skills add` both serve the exact same file from the source tree.
//!
//! Sub-skills live in `src/skills/data/` (CLI-internal only) and are
//! embedded via `rust-embed` into the binary. They never end up on disk
//! and cannot drift from the binary.

use rust_embed::RustEmbed;
use std::borrow::Cow;

/// The hub, embedded at build time from `skills/SKILL.md`. Single source
/// of truth for what gets installed by both `llmwiki-cli install-skill`
/// and `npx skills add Git-Fg/llmwiki`.
pub const HUB_SOURCE: &str = include_str!("../../skills/SKILL.md");

/// CLI-internal sub-skill bundle. Embedded from `src/skills/data/`.
/// Served via `skill get`; never installed to disk.
#[derive(RustEmbed)]
#[folder = "src/skills/data/"]
struct SubSkillBundle;

/// Returns the hub SKILL.md content. Used by `wiki skill install --global`
/// to write `~/.agents/skills/wiki/SKILL.md`, and by `wiki skill` (no args)
/// to print the hub on stdout.
pub fn hub() -> Cow<'static, str> {
    Cow::Borrowed(HUB_SOURCE)
}

/// Looks up one sub-skill by topic name. Accepts either the full file stem
/// (`wiki-search`) or just the topic (`search`); the latter is normalized
/// to `wiki-search.md`. Returns the content served from the binary.
pub fn find_skill(name: &str) -> Option<Cow<'static, str>> {
    let stem = normalize_topic(name);
    let path = format!("{stem}.md");
    SubSkillBundle::get(&path).map(cow_to_str)
}

/// Enumerates every CLI-internal sub-skill. Returns `(file_stem, line_count)`
/// sorted alphabetically. Used by `wiki skill list`.
pub fn list_skills() -> Vec<(String, usize)> {
    let mut out: Vec<(String, usize)> = SubSkillBundle::iter()
        .filter_map(|p| {
            let path = p.as_ref();
            // Sub-skills are `wiki-{name}.md` flat files in the bundle.
            if !path.starts_with("wiki-") || !path.ends_with(".md") {
                return None;
            }
            let stem = path.trim_end_matches(".md");
            SubSkillBundle::get(path).map(|f| {
                let lines = cow_to_str(f).lines().count();
                (stem.to_string(), lines)
            })
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// Resolves a topic name to its file stem. Accepts:
///   - `wiki-search` → `wiki-search`
///   - `search` → `wiki-search`
///   - `Search` → `wiki-search`
fn normalize_topic(name: &str) -> String {
    let lower = name.trim().to_ascii_lowercase();
    if lower.starts_with("wiki-") {
        lower
    } else {
        format!("wiki-{lower}")
    }
}

/// Decode an `EmbeddedFile`'s bytes into a `Cow<'static, str>`.
fn cow_to_str(file: rust_embed::EmbeddedFile) -> Cow<'static, str> {
    match file.data {
        Cow::Borrowed(bytes) => {
            Cow::Borrowed(std::str::from_utf8(bytes).expect("sub-skill .md must be valid UTF-8"))
        }
        Cow::Owned(bytes) => {
            Cow::Owned(String::from_utf8(bytes).expect("sub-skill .md must be valid UTF-8"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hub_loads() {
        let content = hub();
        assert!(content.starts_with("---\n"));
        assert!(content.contains("name: wiki"));
    }

    #[test]
    fn find_skill_accepts_full_and_short_names() {
        assert!(find_skill("wiki-search").is_some());
        assert!(find_skill("search").is_some());
        assert!(find_skill("Search").is_some());
        assert!(find_skill("nonexistent").is_none());
    }

    #[test]
    fn list_skills_returns_wiki_prefixed_files() {
        let skills = list_skills();
        // Sub-skill count is the count of `wiki-*.md` files in
        // src/skills/data/ — assert only the invariants, not the literal
        // count, so adding/removing a sub-skill does not break this test.
        assert!(!skills.is_empty(), "no sub-skills found in bundle");
        for (stem, lines) in &skills {
            assert!(stem.starts_with("wiki-"), "unexpected stem {stem}");
            assert!(*lines > 0, "sub-skill {stem} has 0 lines");
        }
    }

    #[test]
    fn normalize_topic_handles_prefix_and_case() {
        assert_eq!(normalize_topic("search"), "wiki-search");
        assert_eq!(normalize_topic("wiki-search"), "wiki-search");
        assert_eq!(normalize_topic("SEARCH"), "wiki-search");
        assert_eq!(normalize_topic("  query  "), "wiki-query");
    }

    #[test]
    fn hub_does_not_contain_sub_skill_bodies_inline() {
        // The hub must redirect sub-skill content to `skill get`, never
        // duplicate it inline. Verify the hub does not mention
        // `nvidia/nv-embed` (a sub-skill fact) which would indicate
        // content drift.
        let content = hub();
        assert!(
            !content.contains("nvidia/nv-embed"),
            "hub leaks sub-skill detail; use `skill get wiki-models` instead"
        );
    }
}
