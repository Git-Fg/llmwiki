/// Typed representation of a wiki page's YAML frontmatter.
///
/// The 21 known fields are the keys with clear semantic purpose
/// that appear in at least one of the 4 real flat-layout wikis
/// (`mevin`, `minimax-code-wiki`, `MyWiki`, `PharmaWiki`). 15 of
/// 21 meet an original ≥50-occurrence target; 6 fall below but are
/// kept because each captures a distinct semantic category. See
/// `docs/superpowers/specs/2026-06-24-frontmatter-field-audit.md`.
///
/// `extra` captures all other keys (per-wiki taxonomy extensions like
/// `avatar`, `timezone`, `license`, `requiresBeta`, `session`,
/// `schedule`, `retired`, `triggers`, etc.) so they are preserved
/// without forcing a closed schema.
///
/// Type imports are intentionally written as full paths on the derive
/// attributes (see `src/core/config_types.rs` for rationale) so this
/// file does not need any `use` statements. This keeps the file
/// self-contained in any include! scope without introducing duplicate
/// `JsonSchema` name collisions when build.rs includes
/// `config_types.rs`, `doctor_report.rs`, and this file together.
#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct Frontmatter {
    pub title: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(rename = "type")]
    pub page_type: Option<String>,
    #[serde(default)]
    pub sources: Vec<String>,
    pub confidence: Option<f64>,
    pub created: Option<String>,
    pub updated: Option<String>,
    pub schema_version: Option<i64>,
    pub status: Option<String>,
    pub kind: Option<String>,
    pub domain: Option<String>,
    pub maturity: Option<String>,
    pub reviewed: Option<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub related: Vec<String>,
    pub source_type: Option<String>,
    pub sha256: Option<String>,
    pub ingested: Option<String>,
    pub name: Option<String>,
    #[serde(default)]
    pub descriptions: Vec<String>,
    #[serde(flatten)]
    pub extra: std::collections::BTreeMap<String, serde_json::Value>,
}
