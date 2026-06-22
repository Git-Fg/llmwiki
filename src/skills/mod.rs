pub const SKILL_MD: &str = include_str!("WIKI.md");

pub const SETUP: &str = include_str!("SETUP/SKILL.md");
pub const INGEST: &str = include_str!("INGEST/SKILL.md");
pub const SEARCH: &str = include_str!("SEARCH/SKILL.md");
pub const QUERY: &str = include_str!("QUERY/SKILL.md");
pub const LINT: &str = include_str!("LINT/SKILL.md");
pub const MODELS: &str = include_str!("MODELS/SKILL.md");
pub const SYNC: &str = include_str!("SYNC/SKILL.md");
pub const TROUBLESHOOTING: &str = include_str!("TROUBLESHOOTING/SKILL.md");

pub const TOPICS: &[(&str, &str)] = &[
    ("setup", SETUP),
    ("ingest", INGEST),
    ("search", SEARCH),
    ("query", QUERY),
    ("lint", LINT),
    ("models", MODELS),
    ("sync", SYNC),
    ("troubleshooting", TROUBLESHOOTING),
];

pub fn find_topic(name: &str) -> Option<&'static str> {
    TOPICS
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| *v)
}

pub fn list_topics() -> Vec<(&'static str, usize)> {
    TOPICS
        .iter()
        .map(|(k, v)| (*k, v.lines().count()))
        .collect()
}
