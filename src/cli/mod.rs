pub mod embed;
pub mod init;
pub mod models;

use clap::{Parser, Subcommand};

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
    /// Print version
    Version,
}

pub async fn run(cli: Cli) {
    let result: Result<(), crate::error::WikiError> = match cli.command {
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
