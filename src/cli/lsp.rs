use crate::cli::LspArgs;
use crate::core::lsp_domain::{
    self, DomainCompletionItem, DomainDiagnostic, DomainHover, DomainSymbol,
};
use crate::error::WikiError;
use tower_lsp_server::jsonrpc::Result;
use tower_lsp_server::ls_types::*;
use tower_lsp_server::{Client, LanguageServer, LspService, Server};

#[derive(Clone, Debug)]
struct Backend {
    client: Client,
}

impl Backend {
    const fn new(client: Client) -> Self {
        Self { client }
    }
}

fn to_lsp_diag(d: &DomainDiagnostic) -> Diagnostic {
    Diagnostic {
        range: Range {
            start: Position::new(d.line, d.character),
            end: Position::new(d.end_line, d.end_character),
        },
        severity: Some(match d.severity {
            1 => DiagnosticSeverity::ERROR,
            2 => DiagnosticSeverity::WARNING,
            _ => DiagnosticSeverity::INFORMATION,
        }),
        source: Some("llmwiki-cli".into()),
        message: d.message.clone(),
        ..Default::default()
    }
}

fn to_lsp_hover(h: DomainHover) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: h.contents_markdown,
        }),
        range: None,
    }
}

fn to_lsp_completion_item(i: DomainCompletionItem) -> CompletionItem {
    CompletionItem {
        label: i.label,
        kind: Some(match i.kind {
            20 => CompletionItemKind::ENUM_MEMBER,
            _ => CompletionItemKind::PROPERTY,
        }),
        detail: i.detail,
        documentation: i.documentation.map(|d| {
            Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: d,
            })
        }),
        ..Default::default()
    }
}

fn to_lsp_symbol(s: &DomainSymbol) -> DocumentSymbol {
    DocumentSymbol {
        name: s.name.clone(),
        kind: SymbolKind::NAMESPACE,
        range: Range {
            start: Position::new(s.line, s.character),
            end: Position::new(s.end_line, s.end_character),
        },
        selection_range: Range {
            start: Position::new(s.line, s.character),
            end: Position::new(s.line, s.character + s.name.len() as u32),
        },
        children: Some(s.children.iter().map(to_lsp_symbol).collect()),
        detail: None,
        tags: None,
        #[allow(deprecated)]
        deprecated: None,
    }
}

impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec!["=".into(), ".".into(), "[".into(), "\"".into()]),
                    ..Default::default()
                }),
                document_symbol_provider: Some(OneOf::Left(true)),
                ..ServerCapabilities::default()
            },
            server_info: Some(ServerInfo {
                name: "llmwiki-cli-lsp".into(),
                version: Some(env!("CARGO_PKG_VERSION").into()),
            }),
            offset_encoding: None,
        })
    }

    async fn initialized(&self, _: InitializedParams) {}

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.validate_and_publish(&params.text_document.uri, &params.text_document.text)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().last() {
            self.validate_and_publish(&params.text_document.uri, &change.text)
                .await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let _ = self
            .client
            .publish_diagnostics(params.text_document.uri, vec![], None)
            .await;
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let text = read_text(&params.text_document_position_params.text_document.uri).await?;
        let key = lsp_domain::key_at_position(
            &text,
            params.text_document_position_params.position.line,
            params.text_document_position_params.position.character,
        );
        Ok(key
            .and_then(|k| lsp_domain::hover_for(&k))
            .map(to_lsp_hover))
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let text = read_text(&params.text_document_position.text_document.uri).await?;
        let parent = lsp_domain::parent_path_at_position(
            &text,
            params.text_document_position.position.line,
            params.text_document_position.position.character,
        );
        let cfg = lsp_domain::parse_config(&text).unwrap_or_default();
        let parent_refs: Vec<&str> = parent.iter().map(String::as_str).collect();
        Ok(Some(CompletionResponse::Array(
            lsp_domain::completion_for(&parent_refs, &cfg)
                .into_iter()
                .map(to_lsp_completion_item)
                .collect(),
        )))
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let text = read_text(&params.text_document.uri).await?;
        Ok(Some(DocumentSymbolResponse::Nested(
            lsp_domain::symbols_for(&text)
                .iter()
                .map(to_lsp_symbol)
                .collect(),
        )))
    }
}

impl Backend {
    async fn validate_and_publish(&self, uri: &Uri, text: &str) {
        let mut all_diags = Vec::new();
        match lsp_domain::parse_config(text) {
            Ok(cfg) => all_diags.extend(lsp_domain::validate_config(&cfg)),
            Err(parse_diags) => all_diags.extend(parse_diags),
        }
        let lsp_diags: Vec<_> = all_diags.iter().map(to_lsp_diag).collect();
        let _ = self
            .client
            .publish_diagnostics(uri.clone(), lsp_diags, None)
            .await;
    }
}

async fn read_text(uri: &Uri) -> Result<String> {
    let scheme_str = uri.scheme().as_str();
    if scheme_str != "file" {
        return Err(tower_lsp_server::jsonrpc::Error::invalid_params(
            "only file:// URIs supported",
        ));
    }
    let path = uri
        .to_file_path()
        .ok_or_else(|| tower_lsp_server::jsonrpc::Error::invalid_params("invalid file path"))?;
    tokio::fs::read_to_string(&path).await.map_err(|e| {
        tower_lsp_server::jsonrpc::Error::invalid_params(format!("read failed: {}", e))
    })
}

pub async fn run(_args: LspArgs) -> std::result::Result<(), WikiError> {
    let (service, socket) = LspService::new(Backend::new);
    let _ = Server::new(tokio::io::stdin(), tokio::io::stdout(), socket)
        .serve(service)
        .await;
    Ok(())
}
