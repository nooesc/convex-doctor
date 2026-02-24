use std::path::PathBuf;
use std::process;
use std::time::Instant;

use clap::{Parser, ValueEnum};

use convex_doctor::reporter::cli::CliReporter;
use convex_doctor::reporter::json::JsonReporter;
use convex_doctor::reporter::Reporter;

#[derive(Clone, Debug, ValueEnum)]
enum OutputFormat {
    Cli,
    Json,
}

#[derive(Parser)]
#[command(
    name = "convex-doctor",
    version,
    about = "Diagnose your Convex backend"
)]
struct Cli {
    /// Path to the project root (defaults to current directory)
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Output format: cli, json
    #[arg(long, value_enum, default_value_t = OutputFormat::Cli)]
    format: OutputFormat,

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

    let start = Instant::now();
    let result = match convex_doctor::engine::run(&cli.path, cli.verbose, cli.diff.as_deref()) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    };
    let elapsed = start.elapsed();

    if cli.score {
        print!("{}", convex_doctor::reporter::score_only(&result.score));
    } else {
        let output = match cli.format {
            OutputFormat::Json => {
                let reporter = JsonReporter;
                reporter.format(
                    &result.diagnostics,
                    &result.score,
                    &result.project_name,
                    cli.verbose,
                    result.files_scanned,
                    elapsed,
                )
            }
            OutputFormat::Cli => {
                let reporter = CliReporter;
                reporter.format(
                    &result.diagnostics,
                    &result.score,
                    &result.project_name,
                    cli.verbose,
                    result.files_scanned,
                    elapsed,
                )
            }
        };
        print!("{output}");
    }

    if result.fail_below > 0 && result.score.value < result.fail_below {
        process::exit(1);
    }
}
