use clap::Parser;

fn main() {
    let cli = mem_agent::cli::Cli::parse();
    if let Err(e) = mem_agent::cli::run(cli) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
