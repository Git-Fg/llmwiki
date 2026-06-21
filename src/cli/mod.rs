pub mod init;

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
    /// Print version
    Version,
}

pub fn run(cli: Cli) {
    let result: Result<(), crate::error::WikiError> = match cli.command {
        Some(Command::Init { path }) => crate::cli::init::run(path),
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
