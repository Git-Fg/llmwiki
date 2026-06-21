pub mod init;
pub mod models;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "wiki", version, about = "Karpathy-style LLM Wiki")]
pub struct Cli {
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
    /// Print version
    Version,
}

pub fn run(cli: Cli) {
    let result: Result<(), crate::error::WikiError> = match cli.command {
        Some(Command::Init { path }) => crate::cli::init::run(path),
        Some(Command::Models {
            embed,
            rerank,
            commercial,
            json,
        }) => crate::cli::models::run(embed, rerank, commercial, json),
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
