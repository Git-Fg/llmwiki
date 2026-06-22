use crate::error::WikiError;
use regex::Regex;
use serde_yaml::Value;
use std::sync::LazyLock;

#[derive(Debug, Clone)]
pub struct ParsedPage {
    pub frontmatter: Value,
    pub body: String,
}

pub fn parse_frontmatter(content: &str) -> Result<ParsedPage, WikiError> {
    let trimmed = content.trim_start_matches('\u{feff}'); // strip BOM
    if !trimmed.starts_with("---") {
        return Ok(ParsedPage {
            frontmatter: Value::Null,
            body: content.to_string(),
        });
    }
    // Find closing ---
    let after_first = &trimmed[3..];
    let after_first = after_first.strip_prefix('\n').unwrap_or(after_first);
    let end = after_first
        .find("\n---")
        .ok_or_else(|| WikiError::Other(anyhow::anyhow!("unclosed frontmatter")))?;
    let yaml_text = &after_first[..end];
    let body_start = end + 4; // skip \n---
    let body = after_first[body_start..]
        .strip_prefix('\n')
        .unwrap_or(&after_first[body_start..])
        .to_string();

    let frontmatter: Value = if yaml_text.trim().is_empty() {
        Value::Null
    } else {
        serde_yaml::from_str(yaml_text)?
    };
    Ok(ParsedPage { frontmatter, body })
}

static WIKILINK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[\[([^\[\]|]+)(?:\|[^\[\]]+)?\]\]").unwrap());

pub fn extract_wikilinks(body: &str) -> Vec<String> {
    WIKILINK_RE
        .captures_iter(body)
        .map(|c| c.get(1).unwrap().as_str().trim().to_string())
        .collect()
}

#[derive(Debug, Clone, PartialEq)]
pub struct FootnoteDef {
    pub id: String,
    pub body: String,
}

static FOOTNOTE_DEF_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^\[\^([^\]]+)\]:\s+(.+)$").unwrap());

static FOOTNOTE_REF_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[\^([^\]]+)\]").unwrap());

pub fn extract_footnotes(body: &str) -> Vec<FootnoteDef> {
    FOOTNOTE_DEF_RE
        .captures_iter(body)
        .map(|c| FootnoteDef {
            id: c.get(1).unwrap().as_str().to_string(),
            body: c.get(2).unwrap().as_str().trim().to_string(),
        })
        .collect()
}

pub fn extract_footnote_refs(body: &str) -> Vec<String> {
    let mut refs = Vec::new();
    for cap in FOOTNOTE_REF_RE.captures_iter(body) {
        if let Some(m) = cap.get(0) {
            let end_idx = m.end();
            let is_def = body[end_idx..].starts_with(':');
            if !is_def {
                if let Some(id) = cap.get(1) {
                    refs.push(id.as_str().to_string());
                }
            }
        }
    }
    refs
}
