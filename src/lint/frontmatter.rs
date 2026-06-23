use super::tag_tax;
use super::LintIssue;
use crate::core::markdown::parse_frontmatter;

const VALID_TYPES: &[&str] = &["entity", "concept", "comparison", "query", "summary"];

pub fn check_frontmatter(path: &str, content: &str) -> Vec<LintIssue> {
    let mut issues = vec![];

    if !super::validate_filename(path) {
        issues.push(LintIssue {
            severity: "error".into(),
            code: "bad-filename".into(),
            path: path.into(),
            message: "filename must be lowercase-hyphenated, e.g. `my-page.md`".into(),
        });
    }

    let parsed = match parse_frontmatter(content) {
        Ok(p) => p,
        Err(_) => {
            issues.push(LintIssue {
                severity: "error".into(),
                code: "missing-frontmatter".into(),
                path: path.into(),
                message: "no YAML frontmatter found (must start with `---`)".into(),
            });
            return issues;
        }
    };

    let fm = match parsed.frontmatter.as_mapping() {
        Some(m) => m,
        None => {
            issues.push(LintIssue {
                severity: "error".into(),
                code: "missing-frontmatter".into(),
                path: path.into(),
                message: "frontmatter block is empty".into(),
            });
            return issues;
        }
    };

    for field in &[
        "schema_version",
        "title",
        "created",
        "updated",
        "type",
        "tags",
        "sources",
    ] {
        if !fm.contains_key(*field) {
            let code = field.replace('_', "-");
            issues.push(LintIssue {
                severity: "error".into(),
                code: format!("missing-{code}"),
                path: path.into(),
                message: format!("frontmatter missing required field `{field}`"),
            });
        }
    }

    if let Some(type_val) = fm.get("type").and_then(|v| v.as_str()) {
        if !VALID_TYPES.contains(&type_val) {
            issues.push(LintIssue {
                severity: "error".into(),
                code: "invalid-type".into(),
                path: path.into(),
                message: format!(
                    "type `{}` must be one of: {}",
                    type_val,
                    VALID_TYPES.join(", ")
                ),
            });
        }
    }

    if let Some(tags) = fm.get("tags").and_then(|v| v.as_sequence()) {
        let tag_strs: Vec<String> = tags
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();

        let dup_errors = tag_tax::validate_tag_list(&tag_strs);
        for msg in dup_errors {
            issues.push(LintIssue {
                severity: "error".into(),
                code: "duplicate-tag".into(),
                path: path.into(),
                message: msg,
            });
        }

        for tag in &tag_strs {
            if !tag_tax::valid_tag(tag) {
                issues.push(LintIssue {
                    severity: "error".into(),
                    code: "unknown-tag".into(),
                    path: path.into(),
                    message: format!("tag `{tag}` not in taxonomy"),
                });
            }
        }
    }

    if let Some(sources) = fm.get("sources") {
        if let Some(seq) = sources.as_sequence() {
            if seq.is_empty() {
                issues.push(LintIssue {
                    severity: "warn".into(),
                    code: "empty-sources".into(),
                    path: path.into(),
                    message: "sources list is empty".into(),
                });
            }
        }
    }

    issues
}
