use crate::core::frontmatter::Frontmatter;
use crate::error::WikiError;
use regex::Regex;
use std::sync::LazyLock;

#[derive(Debug, Clone)]
pub struct ParsedPage {
    /// `None` when the markdown has no `---` frontmatter block at all.
    /// `Some(Frontmatter::default())` when the block is empty (`---\n---`).
    /// `Some(Frontmatter { .. })` when the block parsed successfully.
    pub frontmatter: Option<Frontmatter>,
    pub body: String,
}

pub fn parse_frontmatter(content: &str) -> Result<ParsedPage, WikiError> {
    let trimmed = content.trim_start_matches('\u{feff}'); // strip BOM
    if !trimmed.starts_with("---") {
        return Ok(ParsedPage {
            frontmatter: None,
            body: content.to_string(),
        });
    }
    // Find closing ---
    #[expect(
        clippy::string_slice,
        reason = "starts_with(\"---\") already confirmed the first 3 bytes are ASCII '---', so [3..] is at a char boundary"
    )]
    let after_first = &trimmed[3..];
    let after_first = after_first.strip_prefix('\n').unwrap_or(after_first);
    // Special case: empty frontmatter block `---\n---<body>` or `---\n---`.
    // After stripping the opening `\n`, the remaining text starts with the
    // closing `---` (followed by `\n` or end-of-input). The general
    // `\n---` scan below would not find a match.
    if let Some(rest) = after_first.strip_prefix("---") {
        let body = rest.strip_prefix('\n').unwrap_or(rest).to_string();
        return Ok(ParsedPage {
            frontmatter: Some(Frontmatter::default()),
            body,
        });
    }
    let end = after_first
        .find("\n---")
        .ok_or_else(|| WikiError::Other(anyhow::anyhow!("unclosed frontmatter")))?;
    #[expect(
        clippy::string_slice,
        reason = "find returns char-boundary positions; `end` is the offset of the matched '\\n' which is ASCII"
    )]
    let yaml_text = &after_first[..end];
    let body_start = end + 4; // skip \n---
    #[expect(
        clippy::string_slice,
        reason = "end+4 advances past '\\n---' (3 ASCII bytes + the matched '\\n'); always a char boundary"
    )]
    let body = after_first[body_start..]
        .strip_prefix('\n')
        .unwrap_or(&after_first[body_start..])
        .to_string();

    let frontmatter: Frontmatter = if yaml_text.trim().is_empty() {
        Frontmatter::default()
    } else {
        serde_saphyr::from_str(yaml_text)?
    };
    Ok(ParsedPage {
        frontmatter: Some(frontmatter),
        body,
    })
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
            // m.end() from regex is guaranteed to be at a char boundary.
            #[expect(
                clippy::string_slice,
                reason = "regex m.end() always returns a char boundary"
            )]
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
