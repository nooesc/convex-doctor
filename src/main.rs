use clap::Parser;
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(name = "convex-doctor", version, about = "Diagnose your Convex backend")]
struct Cli {
    /// Path to the project root (defaults to current directory)
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Output format
    #[arg(long, default_value = "cli")]
    format: String,

    /// Only output the score (0-100)
    #[arg(long)]
    score: bool,

    /// Only analyze files changed vs this base branch
    #[arg(long)]
    diff: Option<String>,

    /// Show verbose output with all affected locations
    #[arg(long, short)]
    verbose: bool,
}

fn main() {
    let cli = Cli::parse();

    match convex_doctor::engine::run(&cli.path, cli.verbose) {
        Ok(_result) => {
            process::exit(0);
        }
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    }
}
