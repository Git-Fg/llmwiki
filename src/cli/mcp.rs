//! `llmwiki-cli mcp` — run the Model Context Protocol server over stdio.
//!
//! Exposes the same domain logic as `llmwiki-cli lsp` (validate / hover /
//! completion / schema / doctor) over MCP JSON-RPC for MCP-aware editors and
//! agents. Backed by `lsp_domain`; the LSP and MCP servers share the same
//! stateless validation and code-intel core.

use crate::cli::McpArgs;
use crate::core::config::Config;
use crate::core::lsp_domain;
use crate::error::WikiError;
use rmcp::handler::server::wrapper::Json;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::schemars::JsonSchema;
use rmcp::{tool, tool_handler, tool_router, transport::stdio, ServerHandler, ServiceExt};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Result of validating a wiki-root.toml string.")]
pub struct ValidationReport {
    pub valid: bool,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Input: a wiki-root.toml string + cursor line and character.")]
pub struct HoverParams {
    #[schemars(description = "The full file content (TOML).")]
    pub config_text: String,
    #[schemars(description = "Zero-indexed line of the cursor.")]
    pub line: u32,
    #[schemars(description = "Zero-indexed character column of the cursor.")]
    pub character: u32,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Hover result: docstring for the key at the cursor, if any.")]
pub struct HoverResult {
    pub contents: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Input: a wiki-root.toml string + cursor line and character.")]
pub struct CompletionParams {
    pub config_text: String,
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Completion result: items available at the cursor.")]
pub struct CompletionResult {
    pub items: Vec<CompletionItemOut>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Single completion suggestion.")]
pub struct CompletionItemOut {
    pub label: String,
    /// "property" for keys, "enum" for enum-like value completions.
    pub kind: String,
    pub detail: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Input: optional workspace alias for the doctor report.")]
pub struct DoctorParams {
    #[schemars(description = "Optional workspace alias (matches `llmwiki-cli --wiki <alias>`).")]
    pub workspace: Option<String>,
}

#[derive(Clone)]
pub struct WikiMcp;

#[tool_router]
impl WikiMcp {
    /// Validate a wiki-root.toml string. Returns whether it parses and passes
    /// all field-level checks.
    #[tool(description = "Validate a wiki-root.toml string and return any errors.")]
    async fn validate(
        &self,
        Parameters(ValidationInput { config_text }): Parameters<ValidationInput>,
    ) -> Result<Json<ValidationReport>, rmcp::ErrorData> {
        let mut errors = Vec::new();
        match lsp_domain::parse_config(&config_text) {
            Ok(cfg) => {
                for d in lsp_domain::validate_config(&cfg) {
                    errors.push(d.message);
                }
            }
            Err(diags) => {
                for d in diags {
                    errors.push(d.message);
                }
            }
        }
        Ok(Json(ValidationReport {
            valid: errors.is_empty(),
            errors,
        }))
    }

    /// Get hover information for the key at a given cursor position.
    #[tool(description = "Get the hover docstring for the config key at a given cursor position.")]
    async fn hover(
        &self,
        Parameters(p): Parameters<HoverParams>,
    ) -> Result<Json<HoverResult>, rmcp::ErrorData> {
        let key = lsp_domain::key_at_position(&p.config_text, p.line, p.character);
        Ok(Json(HoverResult {
            contents: key
                .and_then(|k| lsp_domain::hover_for(&k))
                .map(|h| h.contents_markdown),
        }))
    }

    /// Get completion items for a position in the config.
    #[tool(description = "List completion items available at a given cursor position.")]
    async fn completion(
        &self,
        Parameters(p): Parameters<CompletionParams>,
    ) -> Result<Json<CompletionResult>, rmcp::ErrorData> {
        let parent = lsp_domain::parent_path_at_position(&p.config_text, p.line, p.character);
        let cfg: Config = lsp_domain::parse_config(&p.config_text).unwrap_or_default();
        let parent_refs: Vec<&str> = parent.iter().map(String::as_str).collect();
        let items = lsp_domain::completion_for(&parent_refs, &cfg)
            .into_iter()
            .map(|i| CompletionItemOut {
                label: i.label,
                kind: match i.kind {
                    20 => "enum".into(),
                    _ => "property".into(),
                },
                detail: i.detail,
            })
            .collect();
        Ok(Json(CompletionResult { items }))
    }

    /// Return the full JSON Schema for the Config type as a JSON-encoded text string.
    #[tool(description = "Return the JSON Schema for the wiki-root.toml Config type.")]
    async fn schema(&self) -> Result<String, rmcp::ErrorData> {
        // `Config` derives `schemars::JsonSchema` in `src/core/config.rs`. Both
        // crates see the same schemars version (1.0), so the derive macro and
        // the `schema_for!` call resolve to the same trait.
        let schema = schemars::schema_for!(Config);
        serde_json::to_string_pretty(&schema)
            .map_err(|e| rmcp::ErrorData::internal_error(format!("schema serialize: {e}"), None))
    }

    /// Run llmwiki-cli doctor. Returns the JSON report as a JSON-encoded text string.
    /// Times out after 30 seconds to keep the MCP request handler responsive.
    #[tool(description = "Run llmwiki-cli doctor and return the JSON diagnostic report.")]
    async fn doctor(
        &self,
        Parameters(p): Parameters<DoctorParams>,
    ) -> Result<String, rmcp::ErrorData> {
        // Resolve the running binary at call time. The `CARGO_BIN_EXE_*` env
        // var is only set while compiling the `[[bin]]` target, not the lib.
        let exe = std::env::current_exe().map_err(|e| {
            rmcp::ErrorData::internal_error(format!("locate current exe: {e}"), None)
        })?;
        let mut cmd = tokio::process::Command::new(exe);
        cmd.arg("doctor").arg("--json");
        if let Some(alias) = p.workspace.as_deref() {
            // `workspace` is documented as a wiki alias, so pass it via `--wiki`.
            cmd.arg("--wiki").arg(alias);
        }
        let output = tokio::time::timeout(std::time::Duration::from_secs(30), cmd.output())
            .await
            .map_err(|_| {
                rmcp::ErrorData::internal_error("doctor timed out after 30s".to_string(), None)
            })?
            .map_err(|e| rmcp::ErrorData::internal_error(format!("spawn failed: {e}"), None))?;
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Input: the wiki-root.toml string to validate.")]
pub struct ValidationInput {
    #[schemars(description = "The full wiki-root.toml string to validate.")]
    pub config_text: String,
}

// `#[tool_handler]` fills in `call_tool`, `list_tools`, `get_info`, etc.
// `version` defaults to `CARGO_PKG_VERSION`, so we only override `name` and
// `instructions`.
#[tool_handler(
    name = "llmwiki-cli",
    instructions = "LLM Wiki tools: validate, hover, completion, schema, doctor."
)]
impl ServerHandler for WikiMcp {}

pub async fn run(_args: McpArgs) -> Result<(), WikiError> {
    let service = WikiMcp
        .serve(stdio())
        .await
        .map_err(|e| WikiError::Other(anyhow::anyhow!("mcp serve: {e}")))?;
    service
        .waiting()
        .await
        .map_err(|e| WikiError::Other(anyhow::anyhow!("mcp waiting: {e}")))?;
    Ok(())
}
