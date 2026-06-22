pub mod build;
pub mod doctor;
pub mod embed;
pub mod ingest;
pub mod init;
pub mod install_skill;
pub mod lint;
pub mod ls;
pub mod models;
pub mod query;
pub mod search;
pub mod skill;
pub mod status;

use clap::{Parser, Subcommand};

use crate::cli::skill::SkillArgs;

#[derive(Parser)]
#[command(name = "wiki", version, about = "Karpathy-style LLM Wiki")]
pub struct Cli {
    #[arg(long, global = true)]
    pub workspace: Option<std::path::PathBuf>,
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
    Init { path: std::path::PathBuf },
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
        target: Option<std::path::PathBuf>,
    },
    /// Print version
    Version,
}

pub async fn run(cli: Cli) {
    let result: Result<(), crate::error::WikiError> = match cli.command {
        Some(Command::Build { since, dry_run }) => {
            crate::cli::build::run(crate::cli::build::BuildArgs {
                workspace: cli.workspace,
                since,
                dry_run,
            })
        }
        Some(Command::Init { path }) => crate::cli::init::run(path),
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
                json,
            })
            .await
        }
        Some(Command::Status { json }) => crate::cli::status::run(crate::cli::status::StatusArgs {
            workspace: cli.workspace,
            json,
        }),
        Some(Command::Skill(args)) => crate::cli::skill::run(args),
        Some(Command::InstallSkill { global, target }) => {
            crate::cli::install_skill::run(crate::cli::install_skill::InstallSkillArgs {
                global,
                workspace: cli.workspace,
                target,
            })
        }
        Some(Command::Ingest {
            source,
            no_compile,
            source_type,
        }) => crate::cli::ingest::run(crate::cli::ingest::IngestArgs {
            workspace: cli.workspace,
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
            pages,
            raw,
            embed,
            links,
            config,
            json,
        }),
        Some(Command::Version) | None => {
            println!("wiki {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
    };
    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
