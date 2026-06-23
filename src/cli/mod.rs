pub mod build;
pub mod config;
pub mod doctor;
pub mod embed;
pub mod ingest;
pub mod init;
pub mod install_skill;
pub mod lint;
pub mod ls;
pub mod lsp;
pub mod mcp;
pub mod models;
pub mod query;
pub mod search;
pub mod skill;
pub mod status;
pub mod tree;

use clap::{Parser, Subcommand};

use crate::cli::skill::SkillArgs;

#[derive(Parser)]
#[command(name = "llmwiki-cli", version, about = "Karpathy-style LLM Wiki")]
pub struct Cli {
    #[arg(long, global = true)]
    pub workspace: Option<std::path::PathBuf>,
    /// Select wiki by alias from wiki-root.toml
    #[arg(long, global = true)]
    pub wiki: Option<String>,
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Compile pending raw sources into wiki pages
    Build {
        #[arg(long)]
        since: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
    /// Scaffold a new wiki at <path>
    Init {
        path: std::path::PathBuf,
        /// Wiki alias for wiki-root.toml registration
        #[arg(long)]
        alias: Option<String>,
        /// Tags for this wiki (repeatable)
        #[arg(long = "tag", value_name = "TAG")]
        tags: Vec<String>,
    },
    /// List whitelisted NVIDIA NIM Models
    Models {
        #[arg(long)]
        embed: bool,
        #[arg(long)]
        rerank: bool,
        #[arg(long)]
        commercial: bool,
        #[arg(long)]
        json: bool,
    },
    /// Embed wiki markdown pages into embeddings.jsonl
    Embed {
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        dims: Option<usize>,
        #[arg(long)]
        skip_existing: bool,
        #[arg(long)]
        batch_size: Option<usize>,
    },
    /// Search embedded wiki pages by vector similarity
    Search {
        query: String,
        #[arg(long, default_value_t = 5)]
        top_k: usize,
        #[arg(long, default_value_t = 0.0)]
        threshold: f32,
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Ask a RAG question over the wiki
    Query {
        question: String,
        #[arg(long, default_value_t = 5)]
        top_k: usize,
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        llm_model: Option<String>,
        #[arg(long)]
        no_citations: bool,
        #[arg(long)]
        json: bool,
    },
    /// Run quality checks over wiki pages, raw sources, and log
    Lint {
        #[arg(long, default_value = "wiki")]
        scope: String,
        #[arg(long)]
        strict: bool,
        #[arg(long)]
        json: bool,
    },
    /// List wiki pages, raw sources, embeddings, links, or config
    Ls {
        #[arg(long)]
        pages: bool,
        #[arg(long)]
        raw: bool,
        #[arg(long)]
        embed: bool,
        #[arg(long)]
        links: bool,
        #[arg(long)]
        config: bool,
        #[arg(long)]
        json: bool,
    },
    /// Diagnose workspace, config, and NIM connectivity
    Doctor {
        #[arg(long)]
        json: bool,
    },
    /// Flat, grep-friendly listing of wiki pages (slug, path, title, tags, embedded)
    Tree {
        #[arg(long)]
        json: bool,
    },
    /// Report pages, embeddings, and raw-source coverage
    Status {
        #[arg(long)]
        json: bool,
    },
    /// Add a source file to raw/ and append a log entry
    Ingest {
        source: std::path::PathBuf,
        #[arg(long)]
        no_compile: bool,
        #[arg(long)]
        source_type: Option<String>,
    },
    /// Show or list the embedded wiki agent skill
    Skill(SkillArgs),
    /// Install the wiki skill as a global or workspace-local agent skill
    InstallSkill {
        #[arg(long)]
        global: bool,
        #[arg(long)]
        workspace: Option<std::path::PathBuf>,
    },
    /// Print version
    Version,
    /// Manage wiki-root.toml configuration
    Config {
        #[command(subcommand)]
        cmd: ConfigCmd,
    },
    /// Run the LSP server (stdio)
    Lsp(LspArgs),
    /// Run the MCP server (stdio)
    Mcp(McpArgs),
}

#[derive(clap::Args, Debug)]
pub struct LspArgs {
    /// Transport (currently only stdio)
    #[arg(long, default_value = "stdio")]
    pub transport: String,
}

#[derive(clap::Args, Debug)]
pub struct McpArgs {}

#[derive(clap::Subcommand, Debug)]
pub enum ConfigCmd {
    /// Print the resolved wiki-root.toml file path
    Path,
    /// List all wikis or show merged config for a specific wiki
    List {
        /// Show config for this alias
        #[arg(long)]
        wiki: Option<String>,
        /// JSON output
        #[arg(long)]
        json: bool,
    },
    /// Get a config value by dotted key
    Get {
        /// e.g. nim.embed_model
        key: String,
        /// Wiki alias (defaults to [defaults])
        #[arg(long)]
        wiki: Option<String>,
    },
    /// Set a config value by dotted key
    Set {
        /// e.g. nim.embed_model
        key: String,
        /// e.g. nvidia/nv-embed-v1
        value: String,
        /// Wiki alias (defaults to [defaults])
        #[arg(long)]
        wiki: Option<String>,
    },
    /// Remove a config override (revert to default). Errors if the wiki alias
    /// is from a lower-priority wiki-root.toml (use WIKI_ROOT_CONFIG to target
    /// the file that owns the alias, or edit it directly).
    Unset {
        /// e.g. nim.embed_model
        key: String,
        /// Wiki alias (required for unset)
        #[arg(long)]
        wiki: Option<String>,
    },
    /// Register a new wiki
    Add {
        /// Wiki alias
        alias: String,
        /// Path to wiki directory
        path: std::path::PathBuf,
        /// Tags (repeatable)
        #[arg(long = "tag", value_name = "TAG")]
        tags: Vec<String>,
        /// Description
        #[arg(long)]
        description: Option<String>,
    },
    /// Remove a wiki from the registry. Errors if the alias is from a
    /// lower-priority wiki-root.toml (use WIKI_ROOT_CONFIG to target the file
    /// that owns the alias, or edit it directly).
    Rm {
        /// Wiki alias to remove
        alias: String,
    },
    /// Open wiki-root.toml in $EDITOR
    Edit,
    /// Validate wiki-root.toml: parse [defaults] + every [alias], run field-level checks
    Validate,
    /// Print the JSON Schema for the resolved Config type (for editor / LSP use)
    ShowSchema,
    /// Print the resolved config search order with each path's existence status.
    /// Useful for debugging why a particular config.toml is or isn't being loaded.
    /// Pass --workspace <path> to override the walk-up start; otherwise the
    /// resolved workspace is used (registry → env → walk-up → single-wiki).
    Paths {
        /// Override the workspace used as the walk-up start
        #[arg(long)]
        workspace: Option<std::path::PathBuf>,
        /// JSON output
        #[arg(long)]
        json: bool,
    },
    /// Open the highest-priority existing config file in $EDITOR (per-workspace
    /// first, then per-computer, then $LLMWIKI_CONFIG). If no file exists yet,
    /// opens the per-workspace candidate so you can create one.
    ConfigEdit {
        /// Override the workspace used as the walk-up start. Inherited from
        /// the global `--workspace` flag, which clap auto-propagates here so
        /// `wiki --workspace <ws> config config-edit` works without a
        /// subcommand-level flag.
        #[arg(long)]
        workspace: Option<std::path::PathBuf>,
    },
    /// Print every effective config key, its merged value, and the file it
    /// came from. Mirrors `git config --list --show-origin` so users can see
    /// which file overrides which key.
    ShowEffective {
        /// Override the workspace used as the walk-up start
        #[arg(long)]
        workspace: Option<std::path::PathBuf>,
        /// JSON output
        #[arg(long)]
        json: bool,
    },
}

pub async fn run(cli: Cli) {
    let result: Result<(), crate::error::WikiError> = match cli.command {
        Some(Command::Build { since, dry_run }) => {
            crate::cli::build::run(crate::cli::build::BuildArgs {
                workspace: cli.workspace,
                wiki: cli.wiki,
                since,
                dry_run,
            })
        }
        Some(Command::Init { path, alias, tags }) => {
            crate::cli::init::run(crate::cli::init::InitArgs { path, alias, tags })
        }
        Some(Command::Models {
            embed,
            rerank,
            commercial,
            json,
        }) => crate::cli::models::run(embed, rerank, commercial, json),
        Some(Command::Embed {
            model,
            dims,
            skip_existing,
            batch_size,
        }) => {
            crate::cli::embed::run(crate::cli::embed::EmbedArgs {
                workspace: cli.workspace,
                wiki: cli.wiki.clone(),
                model,
                dims,
                skip_existing,
                batch_size,
            })
            .await
        }
        Some(Command::Search {
            query,
            top_k,
            threshold,
            model,
            json,
        }) => {
            crate::cli::search::run(crate::cli::search::SearchArgs {
                workspace: cli.workspace,
                wiki: cli.wiki.clone(),
                query,
                top_k,
                threshold,
                model,
                json,
            })
            .await
        }
        Some(Command::Query {
            question,
            top_k,
            model,
            llm_model,
            no_citations,
            json,
        }) => {
            crate::cli::query::run(crate::cli::query::QueryArgs {
                workspace: cli.workspace,
                wiki: cli.wiki.clone(),
                question,
                top_k,
                model,
                llm_model,
                no_citations,
                json,
            })
            .await
        }
        Some(Command::Doctor { json }) => {
            crate::cli::doctor::run(crate::cli::doctor::DoctorArgs {
                workspace: cli.workspace,
                wiki: cli.wiki,
                json,
            })
            .await
        }
        Some(Command::Status { json }) => crate::cli::status::run(crate::cli::status::StatusArgs {
            workspace: cli.workspace,
            wiki: cli.wiki.clone(),
            json,
        }),
        Some(Command::Skill(args)) => crate::cli::skill::run(args),
        Some(Command::InstallSkill { global, workspace }) => {
            crate::cli::install_skill::run(crate::cli::install_skill::InstallSkillArgs {
                global,
                workspace,
            })
        }
        Some(Command::Ingest {
            source,
            no_compile,
            source_type,
        }) => crate::cli::ingest::run(crate::cli::ingest::IngestArgs {
            workspace: cli.workspace,
            wiki: cli.wiki.clone(),
            source,
            no_compile,
            source_type,
        }),
        Some(Command::Lint {
            scope,
            strict,
            json,
        }) => {
            crate::cli::lint::run(crate::cli::lint::LintArgs {
                workspace: cli.workspace,
                wiki: cli.wiki.clone(),
                scope,
                strict,
                json,
            })
            .await
        }
        Some(Command::Ls {
            pages,
            raw,
            embed,
            links,
            config,
            json,
        }) => crate::cli::ls::run(crate::cli::ls::LsArgs {
            workspace: cli.workspace,
            wiki: cli.wiki.clone(),
            pages,
            raw,
            embed,
            links,
            config,
            json,
        }),
        Some(Command::Tree { json }) => crate::cli::tree::run(crate::cli::tree::TreeArgs {
            workspace: cli.workspace,
            wiki: cli.wiki.clone(),
            json,
        }),
        Some(Command::Config { cmd }) => crate::cli::config::run(cmd).await,
        Some(Command::Lsp(args)) => crate::cli::lsp::run(args).await,
        Some(Command::Mcp(args)) => crate::cli::mcp::run(args).await,
        Some(Command::Version) | None => {
            println!("llmwiki-cli {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
    };
    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
