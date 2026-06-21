use clap::Parser;

#[derive(Parser)]
#[command(name = "wiki", version = "0.1.0", about = "Karpathy-style LLM Wiki")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Print version
    Version,
}

fn main() {
    let cli = Cli::parse();
    if matches!(cli.command, Some(Command::Version)) {
        println!("wiki {}", env!("CARGO_PKG_VERSION"));
    }
}
