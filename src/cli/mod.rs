use clap::Parser;

#[derive(Parser)]
#[command(name = "wiki", version, about = "Karpathy-style LLM Wiki")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(clap::Subcommand)]
pub enum Command {
    Version,
}

pub fn run(_cli: Cli) {
    println!("wiki {}", env!("CARGO_PKG_VERSION"));
}
