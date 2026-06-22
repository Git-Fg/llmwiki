pub const SKILL_MD: &str = include_str!("skill_md.md");

pub const TOPIC_SETUP: &str = include_str!("topics/setup.md");
pub const TOPIC_INGEST: &str = include_str!("topics/ingest.md");
pub const TOPIC_SEARCH: &str = include_str!("topics/search.md");
pub const TOPIC_QUERY: &str = include_str!("topics/query.md");
pub const TOPIC_LINT: &str = include_str!("topics/lint.md");
pub const TOPIC_MODELS: &str = include_str!("topics/models.md");
pub const TOPIC_SYNC: &str = include_str!("topics/sync.md");
pub const TOPIC_TROUBLESHOOTING: &str = include_str!("topics/troubleshooting.md");

pub const TOPICS: &[(&str, &str)] = &[
    ("setup", TOPIC_SETUP),
    ("ingest", TOPIC_INGEST),
    ("search", TOPIC_SEARCH),
    ("query", TOPIC_QUERY),
    ("lint", TOPIC_LINT),
    ("models", TOPIC_MODELS),
    ("sync", TOPIC_SYNC),
    ("troubleshooting", TOPIC_TROUBLESHOOTING),
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
