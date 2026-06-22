pub mod frontmatter;
pub mod tag_tax;
pub mod wikilinks;

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct LintIssue {
    pub severity: String,
    pub code: String,
    pub path: String,
    pub message: String,
}

pub fn validate_filename(path: &str) -> bool {
    let filename = std::path::Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    !filename.is_empty()
        && filename.ends_with(".md")
        && filename
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '.')
        && !filename.starts_with('-')
        && !filename.contains("--")
}
