//! Shared LSP/MCP domain logic. Stateless per request.

use crate::core::config::validate;
use crate::core::config::Config;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Serialize, Clone)]
pub struct DomainDiagnostic {
    pub line: u32,
    pub character: u32,
    pub end_line: u32,
    pub end_character: u32,
    pub severity: u32, // 1=Error, 2=Warning, 3=Info
    pub message: String,
}

#[derive(Debug)]
pub struct DomainHover {
    pub contents_markdown: String,
}

#[derive(Debug)]
pub struct DomainCompletionItem {
    pub label: String,
    pub kind: u32, // matches lsp_types::CompletionItemKind
    pub detail: Option<String>,
    pub documentation: Option<String>,
}

#[derive(Debug)]
pub struct DomainSymbol {
    pub name: String,
    pub kind: u32, // matches lsp_types::SymbolKind
    pub line: u32,
    pub character: u32,
    pub end_line: u32,
    pub end_character: u32,
    pub children: Vec<DomainSymbol>,
}

pub fn parse_config(text: &str) -> Result<Config, Vec<DomainDiagnostic>> {
    match toml::from_str::<Config>(text) {
        Ok(cfg) => Ok(cfg),
        Err(e) => {
            let span = e.span().unwrap_or(0..0);
            let (line, character) = line_col_from_span(text, span.start);
            let (end_line, end_character) = line_col_from_span(text, span.end);
            Err(vec![DomainDiagnostic {
                line,
                character,
                end_line,
                end_character,
                severity: 1,
                message: format!("parse error: {}", e),
            }])
        }
    }
}

fn line_col_from_span(text: &str, byte_offset: usize) -> (u32, u32) {
    let mut line = 0u32;
    let mut col = 0u32;
    for (i, ch) in text.char_indices() {
        if i >= byte_offset {
            return (line, col);
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    (line, col)
}

pub fn validate_config(cfg: &Config) -> Vec<DomainDiagnostic> {
    validate(cfg)
        .err()
        .unwrap_or_default()
        .into_iter()
        .map(|msg| DomainDiagnostic {
            line: 0,
            character: 0,
            end_line: 0,
            end_character: 0,
            severity: 1,
            message: msg,
        })
        .collect()
}

fn key_doc(key: &str) -> Option<&'static str> {
    Some(match key {
        "nim.embed_model" => {
            "The NIM embedding model to use. Must be one of the whitelisted models."
        }
        "nim.base_url" => "Base URL of the NIM API host. Default: https://integrate.api.nvidia.com",
        "nim.api_key_env" => "Name of the env var holding the API key. Default: NVIDIA_NIM_API_KEY",
        "nim.batch_size" => "Number of chunks to embed per NIM request. Default: 8",
        "nim.request_timeout_secs" => "HTTP timeout for NIM requests. Default: 30",
        "nim.rerank_model" => "The NIM reranking model (used by query). Empty = no rerank.",
        "nim.retry.max_attempts" => "Max retry attempts on transient NIM errors. Default: 3",
        "nim.retry.backoff_ms" => "Backoff between retries in ms. Default: 500",
        "wiki.default_chunk_tokens" => "Target tokens per chunk. Default: 512",
        "wiki.chunk_overlap_tokens" => {
            "Overlap between adjacent chunks. Must be < default_chunk_tokens."
        }
        "wiki.min_chunk_tokens" => "Minimum viable chunk size. Default: 32",
        "wiki.require_frontmatter" => "If true, every page must have YAML frontmatter.",
        "wiki.require_wikilinks_min" => "Minimum outbound wikilinks per page.",
        _ => return None,
    })
}

pub fn hover_for(key: &str) -> Option<DomainHover> {
    key_doc(key).map(|doc| DomainHover {
        contents_markdown: format!("**`{}`**\n\n{}", key, doc),
    })
}

fn all_keys_in_table(parent: &[&str]) -> Vec<String> {
    let prefix = if parent.is_empty() {
        String::new()
    } else {
        format!("{}.", parent.join("."))
    };
    vec![
        "nim.embed_model",
        "nim.base_url",
        "nim.api_key_env",
        "nim.batch_size",
        "nim.request_timeout_secs",
        "nim.rerank_model",
        "nim.retry.max_attempts",
        "nim.retry.backoff_ms",
        "wiki.default_chunk_tokens",
        "wiki.chunk_overlap_tokens",
        "wiki.min_chunk_tokens",
        "wiki.require_frontmatter",
        "wiki.require_wikilinks_min",
    ]
    .into_iter()
    .filter_map(|k| {
        let rest = k.strip_prefix(&prefix).unwrap_or(k);
        if rest.contains('.') {
            None
        } else {
            Some(rest.to_string())
        }
    })
    .collect()
}

fn whitelisted_models() -> Vec<&'static str> {
    vec![
        "nvidia/nv-embed-v1",
        "nvidia/nv-embedqa-e5-v5",
        "nvidia/nv-embedcode-7b-v1",
        "nvidia/llama-nemotron-embed-1b-v2",
        "nvidia/llama-nemotron-embed-vl-1b-v2",
        "nvidia/llama-nemotron-rerank-1b-v2",
        "nvidia/llama-nemotron-rerank-vl-1b-v2",
        "nvidia/nv-rerankqa-mistral-4b-v3",
    ]
}

pub fn completion_for(parent_path: &[&str], _cfg: &Config) -> Vec<DomainCompletionItem> {
    // If completing a value for embed_model, list whitelisted models.
    if parent_path.last() == Some(&"embed_model") {
        return whitelisted_models()
            .into_iter()
            .map(|m| DomainCompletionItem {
                label: m.to_string(),
                kind: 20, // CompletionItemKind::EnumMember
                detail: Some("whitelisted NIM model".into()),
                documentation: None,
            })
            .collect();
    }

    // Otherwise list keys in the current table.
    let keys = if parent_path.is_empty() {
        vec!["nim".to_string(), "wiki".to_string()]
    } else {
        all_keys_in_table(parent_path)
    };

    keys.into_iter()
        .map(|k| {
            let detail = key_doc(&k).map(|d| d.lines().next().unwrap_or("").to_string());
            DomainCompletionItem {
                label: k.clone(),
                kind: 10, // CompletionItemKind::Property
                detail,
                documentation: key_doc(&k).map(String::from),
            }
        })
        .collect()
}

pub fn symbols_for(text: &str) -> Vec<DomainSymbol> {
    let parsed: BTreeMap<String, toml::Value> = toml::from_str(text).unwrap_or_default();
    parsed
        .keys()
        .map(|name| DomainSymbol {
            name: name.clone(),
            kind: 3, // SymbolKind::Namespace
            line: 0,
            character: 0,
            end_line: 0,
            end_character: 0,
            children: vec![],
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_config_accepts_valid() {
        let text = "[w]\npath = \"/tmp\"\n";
        let cfg = parse_config(text).unwrap();
        assert_eq!(cfg.wiki.default_chunk_tokens, 512);
    }

    #[test]
    fn parse_config_reports_diagnostic_on_invalid() {
        let text = "[w\npath = \"/tmp\"\n"; // missing closing bracket
        let err = parse_config(text).unwrap_err();
        assert_eq!(err.len(), 1);
        assert!(err[0].message.contains("expected"));
    }

    #[test]
    fn line_col_from_span_works_at_start() {
        assert_eq!(line_col_from_span("hello", 0), (0, 0));
    }

    #[test]
    fn line_col_from_span_works_after_newline() {
        assert_eq!(line_col_from_span("hello\nworld", 6), (1, 0));
    }

    #[test]
    fn validate_config_emits_diagnostic_for_bad_model() {
        let cfg = Config::default();
        let diags = validate_config(&cfg);
        assert!(diags.is_empty());

        let mut bad = Config::default();
        bad.nim.embed_model = "nvidia/bogus".into();
        let diags = validate_config(&bad);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("unsupported embed_model"));
    }

    #[test]
    fn hover_for_known_key_returns_docstring() {
        let h = hover_for("nim.embed_model").unwrap();
        assert!(
            h.contents_markdown.to_lowercase().contains("embed")
                || h.contents_markdown.to_lowercase().contains("model")
        );
    }

    #[test]
    fn hover_for_unknown_key_returns_none() {
        assert!(hover_for("nim.bogus").is_none());
    }

    #[test]
    fn completion_for_nim_table_lists_nim_keys() {
        let items = completion_for(&["nim"], &Config::default());
        let labels: Vec<_> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"embed_model"));
        assert!(labels.contains(&"base_url"));
    }

    #[test]
    fn completion_for_embed_model_lists_whitelisted_models() {
        let items = completion_for(&["nim", "embed_model"], &Config::default());
        assert!(items.iter().any(|i| i.label == "nvidia/nv-embed-v1"));
    }

    #[test]
    fn symbols_for_returns_table_outline() {
        let text = "[nim]\nembed_model = \"x\"\n\n[wiki]\n";
        let syms = symbols_for(text);
        assert!(syms.iter().any(|s| s.name == "nim"));
        assert!(syms.iter().any(|s| s.name == "wiki"));
    }
}
