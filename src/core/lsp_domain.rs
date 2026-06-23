//! Shared LSP/MCP domain logic. Stateless per request.

use crate::core::config::Config;
use serde::Serialize;

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
}
