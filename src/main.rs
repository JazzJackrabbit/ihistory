mod app;
mod history;
mod search;
mod ui;

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "ih", bin_name = "ih")]
#[command(version, about = "A minimal, fast, fuzzy shell history search tool")]
pub struct Args {
    /// Initial search query
    #[arg()]
    pub query: Option<String>,

    /// Custom history file path
    #[arg(short, long)]
    pub file: Option<PathBuf>,

    /// Max entries to load (0 = unlimited)
    #[arg(short = 'n', long, default_value = "50000")]
    pub limit: usize,
}

fn main() {
    let args = Args::parse();

    match app::run(args) {
        Ok(exit_code) => std::process::exit(exit_code),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
