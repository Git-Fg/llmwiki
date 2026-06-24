//! Skill bundle embedding.
//!
//! v0.3.29: skills live at the repo root under `skills/` (no more
//! `marketplace/`). The CLI binary embeds:
//!   - `skills/SKILL.md` — the hub, written to disk by `wiki skill install`
//!   - `skills/wiki-{name}.md` — inline sub-skills, served on demand via
//!     `wiki skill get <topic>`
//!
//! Why rust-embed instead of nine `include_str!` constants:
//!   1. Adding a new sub-skill is a pure file-system change — no Rust edit.
//!   2. Debug builds read .md from the file system, so edits are picked up
//!      without a rebuild. Release builds embed the bytes.
//!
//! CLI surface (the agent-browser convention):
//!   `wiki skill list`           — enumerate every inline sub-skill
//!   `wiki skill get <topic>`    — print one sub-skill
//!   `wiki <command> --help`     — full flag reference

use rust_embed::RustEmbed;
use std::borrow::Cow;

/// Embedded skill bundle — `skills/` at the repo root.
#[derive(RustEmbed)]
#[folder = "skills/"]
struct SkillBundle;

/// Filename of the hub SKILL.md inside the bundle.
pub const HUB_FILE: &str = "SKILL.md";

/// Returns the hub SKILL.md content. Used by `wiki skill install --global`
/// to write `~/.agents/skills/wiki/SKILL.md`, and by `wiki skill` (no args)
/// to print the hub on stdout.
pub fn hub() -> Cow<'static, str> {
    SkillBundle::get(HUB_FILE)
        .map(cow_to_str)
        .unwrap_or(Cow::Borrowed(""))
}

/// Looks up one inline sub-skill by topic name. Accepts either the full
/// file stem (`wiki-search`) or just the topic (`search`); the latter is
/// normalized to `wiki-search.md`.
pub fn find_skill(name: &str) -> Option<Cow<'static, str>> {
    let stem = normalize_topic(name);
    let path = format!("{stem}.md");
    SkillBundle::get(&path).map(cow_to_str)
}

/// Enumerates every inline sub-skill (excludes the hub). Returns
/// `(file_stem, line_count)` sorted alphabetically. Used by `wiki skill list`.
pub fn list_skills() -> Vec<(String, usize)> {
    let mut out: Vec<(String, usize)> = SkillBundle::iter()
        .filter_map(|p| {
            let path = p.as_ref();
            // We only care about flat sub-skill files: `wiki-{name}.md`.
            // The hub is `SKILL.md` (no `wiki-` prefix); skip it.
            if !path.starts_with("wiki-") || !path.ends_with(".md") {
                return None;
            }
            let stem = path.trim_end_matches(".md");
            SkillBundle::get(path).map(|f| {
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
            Cow::Borrowed(std::str::from_utf8(bytes).expect("SKILL.md must be valid UTF-8"))
        }
        Cow::Owned(bytes) => {
            Cow::Owned(String::from_utf8(bytes).expect("SKILL.md must be valid UTF-8"))
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
    fn list_skills_returns_nine_excluding_hub() {
        let skills = list_skills();
        assert_eq!(skills.len(), 9, "expected 9 sub-skills, got {:?}", skills);
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
}
