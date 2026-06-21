use clap::Parser;
use wiki::cli::Cli;

fn main() {
    let cli = Cli::parse();
    wiki::cli::run(cli);
}
