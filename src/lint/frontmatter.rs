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

    let Some(fm) = parsed.frontmatter.as_ref() else {
        issues.push(LintIssue {
            severity: "error".into(),
            code: "missing-frontmatter".into(),
            path: path.into(),
            message: "frontmatter block is empty".into(),
        });
        return issues;
    };

    // Required-field checks. Option fields: is_none() means missing.
    // Vec fields: is_empty() means missing (stricter than contains_key —
    // an empty `tags: []` is effectively missing).
    let missing: Vec<&str> = [
        ("schema_version", fm.schema_version.is_none()),
        ("title", fm.title.is_none()),
        ("created", fm.created.is_none()),
        ("updated", fm.updated.is_none()),
        ("type", fm.page_type.is_none()),
        ("tags", fm.tags.is_empty()),
        ("sources", fm.sources.is_empty()),
    ]
    .into_iter()
    .filter(|(_, missing)| *missing)
    .map(|(name, _)| name)
    .collect();
    for field in missing {
        let code = field.replace('_', "-");
        issues.push(LintIssue {
            severity: "error".into(),
            code: format!("missing-{code}"),
            path: path.into(),
            message: format!("frontmatter missing required field `{field}`"),
        });
    }

    if let Some(type_val) = fm.page_type.as_deref() {
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

    if !fm.tags.is_empty() {
        let tag_strs: Vec<String> = fm.tags.clone();

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

    if fm.sources.is_empty() {
        issues.push(LintIssue {
            severity: "warn".into(),
            code: "empty-sources".into(),
            path: path.into(),
            message: "sources list is empty".into(),
        });
    }

    issues
}
