use clap::Parser;

mod cli;
mod core;
mod db;
mod embed;
mod mcp;
mod persist;
mod search;
mod simulation;
mod text;
mod vector;

fn main() {
    let cli = cli::Cli::parse();
    if let Err(e) = cli::run(cli) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
