use clap::Parser;
use std::path::PathBuf;
use std::process;

use convex_doctor::reporter::cli::CliReporter;
use convex_doctor::reporter::json::JsonReporter;
use convex_doctor::reporter::Reporter;

#[derive(Parser)]
#[command(name = "convex-doctor", version, about = "Diagnose your Convex backend")]
struct Cli {
    /// Path to the project root (defaults to current directory)
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Output format: cli, json
    #[arg(long, default_value = "cli")]
    format: String,

    /// Only output the score (0-100)
    #[arg(long)]
    score: bool,

    /// Only analyze files changed vs this base branch
    #[arg(long)]
    diff: Option<String>,

    /// Show verbose output with file paths and line numbers
    #[arg(long, short)]
    verbose: bool,
}

fn main() {
    let cli = Cli::parse();

    let result = match convex_doctor::engine::run(&cli.path, cli.verbose) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    };

    if cli.score {
        print!("{}", convex_doctor::reporter::score_only(&result.score));
    } else {
        let output = match cli.format.as_str() {
            "json" => {
                let reporter = JsonReporter;
                reporter.format(
                    &result.diagnostics,
                    &result.score,
                    &result.project_name,
                    cli.verbose,
                )
            }
            _ => {
                let reporter = CliReporter;
                reporter.format(
                    &result.diagnostics,
                    &result.score,
                    &result.project_name,
                    cli.verbose,
                )
            }
        };
        print!("{output}");
    }

    let config = convex_doctor::config::Config::load(&cli.path).unwrap_or_default();
    if config.ci.fail_below > 0 && result.score.value < config.ci.fail_below {
        process::exit(1);
    }
}
