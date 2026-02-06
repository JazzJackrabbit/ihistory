mod app;
mod history;
mod search;
mod ui;

use clap::Parser;
use std::path::PathBuf;

const ZSH_SCRIPT: &str = include_str!("../shell/ihistory.zsh");
const BASH_SCRIPT: &str = include_str!("../shell/ihistory.bash");

#[derive(Parser, Debug)]
#[command(name = "ih", bin_name = "ih")]
#[command(version, about = "A minimal, fast, fuzzy shell history search tool")]
pub struct Args {
    #[arg(long, num_args = 0..=1, default_missing_value = "auto")]
    pub init: Option<String>,

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

    if let Some(ref shell) = args.init {
        match shell.as_str() {
            "zsh" => {
                print!("{}", ZSH_SCRIPT);
            }
            "bash" => {
                print!("{}", BASH_SCRIPT);
            }
            "auto" => {
                setup_shell();
            }
            other => {
                eprintln!("Unknown shell: {}. Supported: zsh, bash", other);
                std::process::exit(1);
            }
        }
        return;
    }

    match app::run(args) {
        Ok(exit_code) => std::process::exit(exit_code),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn setup_shell() {
    // 1. Detect shell from $SHELL
    let shell_env = match std::env::var("SHELL") {
        Ok(s) => s,
        Err(_) => {
            eprintln!("Could not detect shell from $SHELL environment variable.");
            eprintln!("Run manually: ihistory --init zsh  (or bash)");
            std::process::exit(1);
        }
    };

    let (shell_name, profile_path) = if shell_env.contains("zsh") {
        ("zsh", dirs::home_dir().unwrap().join(".zshrc"))
    } else if shell_env.contains("bash") {
        ("bash", dirs::home_dir().unwrap().join(".bashrc"))
    } else {
        eprintln!("Unsupported shell: {}", shell_env);
        eprintln!("Supported shells: zsh, bash");
        eprintln!("Run manually: ihistory --init zsh  (or bash)");
        std::process::exit(1);
    };

    // 2. Check if already configured
    let profile_str = profile_path.display().to_string();
    if let Ok(contents) = std::fs::read_to_string(&profile_path) {
        if contents.contains("ihistory --init") {
            eprintln!("Already configured in {}", profile_str);
            eprintln!("ihistory shell integration is active.");
            return;
        }
    }

    // 3. Append eval line
    let eval_line = format!("\neval \"$(ihistory --init {})\"\n", shell_name);
    if let Err(e) = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(&profile_path)
        .and_then(|mut f| std::io::Write::write_all(&mut f, eval_line.as_bytes()))
    {
        eprintln!("Failed to write to {}: {}", profile_str, e);
        std::process::exit(1);
    }

    // 4. Print success
    eprintln!("Added to {}:", profile_str);
    eprintln!("  eval \"$(ihistory --init {})\"", shell_name);
    eprintln!();
    eprintln!("Restart your shell or run:");
    eprintln!("  source {}", profile_str);
}
