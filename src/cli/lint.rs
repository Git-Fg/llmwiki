use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::core::markdown::{extract_wikilinks, parse_frontmatter};
use crate::core::workspace::discover_workspace;
use crate::error::WikiError;
use crate::lint::frontmatter::check_frontmatter;
use crate::lint::wikilinks::check_wikilinks;
use crate::lint::LintIssue;

pub struct LintArgs {
    pub workspace: Option<PathBuf>,
    pub scope: String,
    pub strict: bool,
    pub fix: bool,
    pub json: bool,
}

pub async fn run(args: LintArgs) -> Result<(), WikiError> {
    let ws = discover_workspace(
        args.workspace.clone(),
        std::env::var("WIKI_WORKSPACE").ok().map(PathBuf::from),
        std::env::current_dir()?,
    )?;

    let mut all_issues: Vec<LintIssue> = vec![];

    if args.scope == "wiki" || args.scope == "all" {
        let wiki_dir = ws.join("wiki");
        if wiki_dir.exists() {
            let mut all_pages: Vec<String> = vec![];
            for entry in walkdir::WalkDir::new(&wiki_dir) {
                let entry = entry.map_err(|e| anyhow::anyhow!(e))?;
                if entry.path().extension().and_then(|s| s.to_str()) == Some("md") {
                    let rel = entry
                        .path()
                        .strip_prefix(&ws)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/");
                    all_pages.push(rel);
                }
            }

            let inbound = compute_inbound_links(&ws, &all_pages);
            let index_content = std::fs::read_to_string(ws.join("index.md")).unwrap_or_default();

            let mut index_paths_seen: HashMap<String, usize> = HashMap::new();
            for line in index_content.lines() {
                if let Some(start) = line.find("](") {
                    let path_start = start + 2;
                    if let Some(end) = line[path_start..].find(')') {
                        let path = &line[path_start..path_start + end];
                        if path.starts_with("wiki/") {
                            *index_paths_seen.entry(path.to_string()).or_insert(0) += 1;
                        }
                    }
                }
            }
            for (path, count) in &index_paths_seen {
                if *count > 1 {
                    all_issues.push(LintIssue {
                        severity: "error".into(),
                        code: "page-in-index-multiple-times".into(),
                        path: "index.md".into(),
                        message: format!("path `{}` appears {} times in index.md", path, count),
                    });
                }
            }

            let mut referenced_raws: HashSet<String> = HashSet::new();

            for page_path in &all_pages {
                let content = std::fs::read_to_string(ws.join(page_path))?;
                let line_count = content.lines().count();
                all_issues.extend(check_frontmatter(page_path, &content));

                let parsed = parse_frontmatter(&content)?;
                let inbound_count = inbound.get(page_path).copied().unwrap_or(0);
                all_issues.extend(check_wikilinks(
                    page_path,
                    &parsed.body,
                    &all_pages,
                    inbound_count,
                    line_count,
                ));

                let links = extract_wikilinks(&parsed.body);
                let page_slug = Path::new(page_path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");
                for link in &links {
                    let link_slug = link.split('/').next_back().unwrap_or(link);
                    if link_slug.eq_ignore_ascii_case(page_slug) {
                        all_issues.push(LintIssue {
                            severity: "warn".into(),
                            code: "link-to-self".into(),
                            path: page_path.into(),
                            message: format!("page links to itself via `[[{}]]`", link),
                        });
                    }
                }

                if let Some(fm) = parsed.frontmatter.as_mapping() {
                    if let Some(sources) = fm.get("sources").and_then(|v| v.as_sequence()) {
                        for src in sources {
                            if let Some(s) = src.as_str() {
                                referenced_raws.insert(s.to_string());
                            }
                        }
                    }
                }

                if let Some(fm) = parsed.frontmatter.as_mapping() {
                    let created = fm.get("created").and_then(|v| v.as_str());
                    let updated = fm.get("updated").and_then(|v| v.as_str());

                    if let Some(c) = created {
                        if chrono::NaiveDate::parse_from_str(c, "%Y-%m-%d").is_err() {
                            all_issues.push(LintIssue {
                                severity: "warn".into(),
                                code: "invalid-date".into(),
                                path: page_path.into(),
                                message: format!("`created` value `{}` is not YYYY-MM-DD", c),
                            });
                        }
                    }
                    if let Some(u) = updated {
                        if chrono::NaiveDate::parse_from_str(u, "%Y-%m-%d").is_err() {
                            all_issues.push(LintIssue {
                                severity: "warn".into(),
                                code: "invalid-date".into(),
                                path: page_path.into(),
                                message: format!("`updated` value `{}` is not YYYY-MM-DD", u),
                            });
                        }
                    }
                    if let (Some(c), Some(u)) = (created, updated) {
                        if let (Ok(c_date), Ok(u_date)) = (
                            chrono::NaiveDate::parse_from_str(c, "%Y-%m-%d"),
                            chrono::NaiveDate::parse_from_str(u, "%Y-%m-%d"),
                        ) {
                            if u_date < c_date {
                                all_issues.push(LintIssue {
                                    severity: "warn".into(),
                                    code: "updated-before-created".into(),
                                    path: page_path.into(),
                                    message: format!(
                                        "`updated` ({}) is before `created` ({})",
                                        u, c
                                    ),
                                });
                            }
                        }
                    }
                }

                let body = &parsed.body;
                let mut refs_seen: Vec<String> = vec![];
                let mut defs_seen: Vec<String> = vec![];

                for line in body.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("[^") {
                        if let Some(rest) = trimmed.strip_prefix("[^") {
                            if let Some(end) = rest.find("]:") {
                                defs_seen.push(rest[..end].to_string());
                            } else if let Some(end) = rest.find(']') {
                                refs_seen.push(rest[..end].to_string());
                            }
                        }
                    } else if let Some(rest) = trimmed.strip_prefix("[^") {
                        if let Some(end) = rest.find(']') {
                            refs_seen.push(rest[..end].to_string());
                        }
                    }
                }

                for label in &refs_seen {
                    if !defs_seen.iter().any(|l| l == label) {
                        all_issues.push(LintIssue {
                            severity: "error".into(),
                            code: "footnote-undefined".into(),
                            path: page_path.into(),
                            message: format!("`[^{}]` used but no definition found", label),
                        });
                    }
                }
                for label in &defs_seen {
                    if !refs_seen.iter().any(|l| l == label) {
                        all_issues.push(LintIssue {
                            severity: "warn".into(),
                            code: "footnote-unused".into(),
                            path: page_path.into(),
                            message: format!("`[^{}]:` defined but never referenced", label),
                        });
                    }
                }
            }

            let raw_dir = ws.join("raw");
            if raw_dir.exists() {
                for entry in walkdir::WalkDir::new(&raw_dir) {
                    let entry = entry.map_err(|e| anyhow::anyhow!(e))?;
                    let ext = entry
                        .path()
                        .extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("");
                    if ext != "md" {
                        continue;
                    }
                    let rel = entry
                        .path()
                        .strip_prefix(&ws)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/");
                    if !referenced_raws.contains(&rel) {
                        all_issues.push(LintIssue {
                            severity: "warn".into(),
                            code: "source-not-cited".into(),
                            path: rel,
                            message: "raw file not cited by any wiki page".into(),
                        });
                    }
                }
            }

            for link_path in index_paths_seen.keys() {
                let target = ws.join(link_path);
                if !target.exists() {
                    all_issues.push(LintIssue {
                        severity: "error".into(),
                        code: "index-points-to-missing".into(),
                        path: "index.md".into(),
                        message: format!("index links to `{}` which does not exist", link_path),
                    });
                }
            }
        }
    }

    if args.scope == "raw" || args.scope == "all" {
        let raw_dir = ws.join("raw");
        if raw_dir.exists() {
            for entry in walkdir::WalkDir::new(&raw_dir) {
                let entry = entry.map_err(|e| anyhow::anyhow!(e))?;
                let ext = entry
                    .path()
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");
                if ext != "md" {
                    continue;
                }
                let rel = entry
                    .path()
                    .strip_prefix(&ws)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                let content = std::fs::read_to_string(entry.path())?;
                let parsed = parse_frontmatter(&content)?;

                if parsed.frontmatter.as_mapping().is_none() {
                    all_issues.push(LintIssue {
                        severity: "warn".into(),
                        code: "raw-no-frontmatter".into(),
                        path: rel.clone(),
                        message: "raw file lacks frontmatter block".into(),
                    });
                }

                if let Some(fm) = parsed.frontmatter.as_mapping() {
                    if let Some(declared_sha) = fm.get("sha256").and_then(|v| v.as_str()) {
                        let mut hasher = Sha256::new();
                        hasher.update(parsed.body.as_bytes());
                        let computed = hex::encode(hasher.finalize());
                        if computed != declared_sha {
                            all_issues.push(LintIssue {
                                severity: "error".into(),
                                code: "raw-drift".into(),
                                path: rel.clone(),
                                message: format!(
                                    "sha256 drift: declared `{}` but body hashes to `{}`",
                                    &declared_sha[..declared_sha.len().min(16)],
                                    &computed[..computed.len().min(16)]
                                ),
                            });
                        }
                    } else {
                        all_issues.push(LintIssue {
                            severity: "warn".into(),
                            code: "raw-no-sha256".into(),
                            path: rel.clone(),
                            message: "raw file missing `sha256` digest for drift detection".into(),
                        });
                    }

                    let has_locator = fm.contains_key("source_url")
                        || fm.contains_key("session_url")
                        || fm.contains_key("source_path");
                    if !has_locator {
                        all_issues.push(LintIssue {
                            severity: "warn".into(),
                            code: "raw-missing-locator".into(),
                            path: rel.clone(),
                            message: "raw file missing source_url, session_url, or source_path"
                                .into(),
                        });
                    }
                }
            }
        }
    }

    let log_path = ws.join("log.md");
    if log_path.exists() {
        let log_content = std::fs::read_to_string(&log_path).unwrap_or_default();
        for (i, line) in log_content.lines().enumerate() {
            if let Some(entry) = line.strip_prefix("## ") {
                if !entry.starts_with('[') || !entry.contains("] ") {
                    all_issues.push(LintIssue {
                        severity: "warn".into(),
                        code: "log-bad-format".into(),
                        path: "log.md".into(),
                        message: format!(
                            "line {}: entry doesn't match `## [date] action | desc` format",
                            i + 1
                        ),
                    });
                }
            }
        }
    }

    if args.json {
        let errors = all_issues.iter().filter(|i| i.severity == "error").count();
        let warnings = all_issues.iter().filter(|i| i.severity == "warn").count();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "errors": errors,
                "warnings": warnings,
                "issues": all_issues,
            }))
            .unwrap()
        );
    } else {
        let errors = all_issues.iter().filter(|i| i.severity == "error").count();
        let warnings = all_issues.iter().filter(|i| i.severity == "warn").count();
        println!("\n{} error(s), {} warning(s)\n", errors, warnings);
        for issue in &all_issues {
            println!(
                "  [{}] [{}] {} — {}",
                issue.severity, issue.code, issue.path, issue.message
            );
        }
    }

    let errors = all_issues.iter().filter(|i| i.severity == "error").count();
    let warnings = all_issues.iter().filter(|i| i.severity == "warn").count();

    if errors > 0 {
        std::process::exit(2);
    }
    if args.strict && warnings > 0 {
        std::process::exit(2);
    }
    Ok(())
}

fn compute_inbound_links(ws: &Path, all_pages: &[String]) -> HashMap<String, usize> {
    let mut inbound: HashMap<String, usize> = HashMap::new();
    for page_path in all_pages {
        let content = std::fs::read_to_string(ws.join(page_path)).unwrap_or_default();
        if let Ok(parsed) = parse_frontmatter(&content) {
            let links = extract_wikilinks(&parsed.body);
            for link in &links {
                let target_slug = link.split('/').next_back().unwrap_or(link).to_lowercase();
                for candidate in all_pages {
                    let candidate_slug = Path::new(candidate)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    if candidate_slug == target_slug && candidate != page_path {
                        *inbound.entry(candidate.clone()).or_insert(0) += 1;
                    }
                }
            }
        }
    }
    inbound
}
