use clap::Parser;
use wiki::cli::Cli;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    wiki::cli::run(cli).await;
}
