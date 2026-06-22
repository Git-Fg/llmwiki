use clap::Parser;
use llmwiki_cli::cli::Cli;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    llmwiki_cli::cli::run(cli).await;
}
