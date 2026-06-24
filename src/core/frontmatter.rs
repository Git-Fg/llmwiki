/// Custom deserializer: accepts either a single string or a YAML/JSON
/// array of strings, normalizes to `Vec<String>`. Required for the
/// `sources` and `tags` fields, which mywiki writes as bare scalars
/// (`sources: raw/articles/foo.md`) while other wikis use list form
/// (`sources: [raw/articles/foo.md, ...]`). Without this, the typed
/// struct would reject the scalar form even though the old
/// `serde_yaml::Value` tree silently coerced both shapes.
fn deserialize_string_or_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Visitor};
    struct V;
    impl<'de> Visitor<'de> for V {
        type Value = Vec<String>;

        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("a string or array of strings")
        }

        fn visit_str<E: de::Error>(self, s: &str) -> Result<Self::Value, E> {
            Ok(vec![s.to_string()])
        }

        fn visit_string<E: de::Error>(self, s: String) -> Result<Self::Value, E> {
            Ok(vec![s])
        }

        fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut out = Vec::new();
            while let Some(s) = seq.next_element::<String>()? {
                out.push(s);
            }
            Ok(out)
        }
    }

    deserializer.deserialize_any(V)
}

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
    #[serde(default, deserialize_with = "deserialize_string_or_vec")]
    pub tags: Vec<String>,
    #[serde(rename = "type")]
    pub page_type: Option<String>,
    #[serde(default, deserialize_with = "deserialize_string_or_vec")]
    pub sources: Vec<String>,
    /// Confidence in the page's claims. Polymorphic across wikis:
    ///   - Mevin: string ("high", "medium", "low", or pipeline expressions
    ///     like "high | medium | low")
    ///   - MiniMax code-wiki: numeric (0.8, 0.85, 0.9, 0.95) or string
    ///   - MyWiki, PharmaWiki: rarely used
    ///
    /// Preserved as `String` to handle both numeric and string forms;
    /// consumers can parse to f64 if needed.
    pub confidence: Option<String>,
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
    /// Per-locale descriptions for SKILL.md-style pages. Used by
    /// minimax-code-wiki in the form `descriptions: { zh-Hans: "...", en: "..." }`.
    /// None of the other 3 wikis use this field.
    #[serde(default)]
    pub descriptions: std::collections::BTreeMap<String, String>,
    #[serde(flatten)]
    pub extra: std::collections::BTreeMap<String, serde_json::Value>,
}
