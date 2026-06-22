use super::LintIssue;
use crate::core::markdown::extract_wikilinks;
use std::collections::HashSet;

const MIN_OUTBOUND: usize = 2;
const MAX_LINES: usize = 200;

pub fn check_wikilinks(
    path: &str,
    body: &str,
    all_pages: &[String],
    inbound_count: usize,
    line_count: usize,
) -> Vec<LintIssue> {
    let mut issues = vec![];
    let links = extract_wikilinks(body);

    let page_set: HashSet<String> = all_pages
        .iter()
        .filter_map(|p| std::path::Path::new(p).file_stem().and_then(|s| s.to_str()))
        .map(|s| s.to_lowercase())
        .collect();

    for link in &links {
        let target = link.split('/').next_back().unwrap_or(link).to_lowercase();
        if !page_set.contains(&target) {
            issues.push(LintIssue {
                severity: "error".into(),
                code: "dangling-link".into(),
                path: path.into(),
                message: format!("[[{}]] does not resolve to a wiki page", link),
            });
        }
    }

    if links.is_empty() {
        issues.push(LintIssue {
            severity: "error".into(),
            code: "no-outbound-links".into(),
            path: path.into(),
            message: "page has 0 outbound [[wikilinks]] (minimum 2 required)".into(),
        });
    } else if links.len() < MIN_OUTBOUND {
        issues.push(LintIssue {
            severity: "error".into(),
            code: "below-min-outbound".into(),
            path: path.into(),
            message: format!(
                "page has {} outbound [[wikilinks]] (minimum {} required)",
                links.len(),
                MIN_OUTBOUND
            ),
        });
    }

    if inbound_count == 0 {
        issues.push(LintIssue {
            severity: "warn".into(),
            code: "orphan-page".into(),
            path: path.into(),
            message: "no inbound [[wikilinks]] from any other page (orphan)".into(),
        });
    }

    if line_count > MAX_LINES {
        issues.push(LintIssue {
            severity: "warn".into(),
            code: "page-too-long".into(),
            path: path.into(),
            message: format!(
                "page is {} lines (> {}); consider splitting",
                line_count, MAX_LINES
            ),
        });
    }

    issues
}
