use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::core::config::resolve_config;
use crate::core::markdown::{extract_wikilinks, parse_frontmatter};
use crate::core::workspace::{discover_workspace, pages_dir, rel_path};
use crate::error::WikiError;
use crate::lint::frontmatter::check_frontmatter;
use crate::lint::wikilinks::check_wikilinks;
use crate::lint::LintIssue;

pub struct LintArgs {
    pub workspace: Option<PathBuf>,
    pub wiki: Option<String>,
    pub scope: String,
    pub strict: bool,
    pub json: bool,
}

pub async fn run(args: LintArgs) -> Result<(), WikiError> {
    let ws = discover_workspace(
        args.workspace.clone(),
        args.wiki.as_deref(),
        std::env::var("WIKI_WORKSPACE").ok().map(PathBuf::from),
        std::env::var("WIKI_ACTIVE").ok().as_deref(),
        std::env::current_dir()?,
    )?;
    let cfg = resolve_config(&ws)?;

    let mut all_issues: Vec<LintIssue> = vec![];

    if args.scope == "wiki" || args.scope == "all" {
        let wiki_dir = pages_dir(&ws, &cfg.wiki.pages_dir);
        let pages_dir_prefix = if cfg.wiki.pages_dir.is_empty() {
            String::new()
        } else {
            format!("{}/", cfg.wiki.pages_dir)
        };
        if wiki_dir.exists() {
            let mut all_pages: Vec<String> = vec![];
            for entry in crate::core::workspace::walk_pages(&wiki_dir, &cfg.wiki.exclude_dirs) {
                let entry = entry.map_err(|e| anyhow::anyhow!(e))?;
                if entry.path().extension().and_then(|s| s.to_str()) != Some("md") {
                    continue;
                }
                if !crate::core::workspace::is_wiki_page_entry(&ws, entry.path()) {
                    continue;
                }
                let rel = rel_path(&ws, entry.path())
                    .unwrap_or_else(|| entry.path().display().to_string());
                all_pages.push(rel);
            }

            let inbound = compute_inbound_links(&ws, &all_pages);
            let index_content = std::fs::read_to_string(ws.join("index.md")).unwrap_or_default();

            let mut index_paths_seen: HashMap<String, usize> = HashMap::new();
            for line in index_content.lines() {
                if let Some(start) = line.find("](") {
                    let path_start = start + 2;
                    // find returns a char boundary; path_start and the
                    // inner find result are also char boundaries.
                    #[expect(
                        clippy::string_slice,
                        reason = "both indices come from str::find which guarantees char boundaries"
                    )]
                    if let Some(end) = line[path_start..].find(')') {
                        #[expect(
                            clippy::string_slice,
                            reason = "both indices come from str::find which guarantees char boundaries"
                        )]
                        let path = &line[path_start..path_start + end];
                        if pages_dir_prefix.is_empty() || path.starts_with(&pages_dir_prefix) {
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
                        message: format!("path `{path}` appears {count} times in index.md"),
                    });
                }
            }

            let mut referenced_raws: HashSet<String> = HashSet::new();

            for page_path in &all_pages {
                let content = std::fs::read_to_string(ws.join(page_path))?;
                let line_count = content.lines().count();
                all_issues.extend(check_frontmatter(page_path, &content));

                // Report unparseable frontmatter as a lint error rather than
                // failing the whole `llmwiki-cli lint` run. This makes the command
                // resilient to individual bad pages in large wikis.
                let parsed = match parse_frontmatter(&content) {
                    Ok(p) => p,
                    Err(e) => {
                        all_issues.push(LintIssue {
                            severity: "error".into(),
                            code: "frontmatter-yaml-parse".into(),
                            path: page_path.clone(),
                            message: format!("could not parse YAML frontmatter: {e}"),
                        });
                        continue;
                    }
                };
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
                            message: format!("page links to itself via `[[{link}]]`"),
                        });
                    }
                }

                if let Some(fm) = parsed.frontmatter.as_ref() {
                    for src in &fm.sources {
                        referenced_raws.insert(src.clone());
                    }
                }

                if let Some(fm) = parsed.frontmatter.as_ref() {
                    let created = fm.created.as_deref();
                    let updated = fm.updated.as_deref();

                    if let Some(c) = created {
                        if chrono::NaiveDate::parse_from_str(c, "%Y-%m-%d").is_err() {
                            all_issues.push(LintIssue {
                                severity: "warn".into(),
                                code: "invalid-date".into(),
                                path: page_path.into(),
                                message: format!("`created` value `{c}` is not YYYY-MM-DD"),
                            });
                        }
                    }
                    if let Some(u) = updated {
                        if chrono::NaiveDate::parse_from_str(u, "%Y-%m-%d").is_err() {
                            all_issues.push(LintIssue {
                                severity: "warn".into(),
                                code: "invalid-date".into(),
                                path: page_path.into(),
                                message: format!("`updated` value `{u}` is not YYYY-MM-DD"),
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
                                    message: format!("`updated` ({u}) is before `created` ({c})"),
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
                            #[expect(clippy::string_slice, reason = "find returns a char boundary")]
                            if let Some(end) = rest.find("]:") {
                                defs_seen.push(rest[..end].to_string());
                            } else {
                                #[expect(
                                    clippy::string_slice,
                                    reason = "find returns a char boundary"
                                )]
                                if let Some(end) = rest.find(']') {
                                    refs_seen.push(rest[..end].to_string());
                                }
                            }
                        }
                    } else if let Some(rest) = trimmed.strip_prefix("[^") {
                        #[expect(clippy::string_slice, reason = "find returns a char boundary")]
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
                            message: format!("`[^{label}]` used but no definition found"),
                        });
                    }
                }
                for label in &defs_seen {
                    if !refs_seen.iter().any(|l| l == label) {
                        all_issues.push(LintIssue {
                            severity: "warn".into(),
                            code: "footnote-unused".into(),
                            path: page_path.into(),
                            message: format!("`[^{label}]:` defined but never referenced"),
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
                    let rel = rel_path(&ws, entry.path())
                        .unwrap_or_else(|| entry.path().display().to_string());
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
                        message: format!("index links to `{link_path}` which does not exist"),
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
                let rel = rel_path(&ws, entry.path())
                    .unwrap_or_else(|| entry.path().display().to_string());
                let content = std::fs::read_to_string(entry.path())?;
                // Same: report unparseable frontmatter as a lint issue
                // instead of failing the whole lint run.
                let parsed = match parse_frontmatter(&content) {
                    Ok(p) => p,
                    Err(e) => {
                        all_issues.push(LintIssue {
                            severity: "error".into(),
                            code: "frontmatter-yaml-parse".into(),
                            path: rel.clone(),
                            message: format!("could not parse YAML frontmatter: {e}"),
                        });
                        continue;
                    }
                };

                if parsed.frontmatter.is_none() {
                    all_issues.push(LintIssue {
                        severity: "warn".into(),
                        code: "raw-no-frontmatter".into(),
                        path: rel.clone(),
                        message: "raw file lacks frontmatter block".into(),
                    });
                }

                if let Some(fm) = parsed.frontmatter.as_ref() {
                    if let Some(declared_sha) = fm.sha256.as_deref() {
                        let mut hasher = Sha256::new();
                        hasher.update(parsed.body.as_bytes());
                        let computed = hex::encode(hasher.finalize());
                        if computed != declared_sha {
                            // SHA256 hex strings are 64 ASCII chars; min(16)
                            // always lands on a char boundary.
                            #[expect(
                                clippy::string_slice,
                                reason = "SHA256 hex output is ASCII; min(16) is at a char boundary"
                            )]
                            let declared_short = &declared_sha[..declared_sha.len().min(16)];
                            #[expect(
                                clippy::string_slice,
                                reason = "SHA256 hex output is ASCII; min(16) is at a char boundary"
                            )]
                            let computed_short = &computed[..computed.len().min(16)];
                            all_issues.push(LintIssue {
                                severity: "error".into(),
                                code: "raw-drift".into(),
                                path: rel.clone(),
                                message: format!(
                                    "sha256 drift: declared `{declared_short}` but body hashes to `{computed_short}`",
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

                    let has_locator = fm.extra.contains_key("source_url")
                        || fm.extra.contains_key("session_url")
                        || fm.extra.contains_key("source_path");
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
        println!("\n{errors} error(s), {warnings} warning(s)\n");
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
