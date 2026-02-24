use std::collections::BTreeMap;

use owo_colors::OwoColorize;

use crate::diagnostic::{Diagnostic, Severity};
use crate::scoring::ScoreResult;

use super::Reporter;

pub struct CliReporter;

impl Reporter for CliReporter {
    fn format(
        &self,
        diagnostics: &[Diagnostic],
        score: &ScoreResult,
        project_name: &str,
        verbose: bool,
    ) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "\n  {} v{}\n\n",
            "convex-doctor".bold(),
            env!("CARGO_PKG_VERSION")
        ));
        out.push_str(&format!("  Project: {}\n", project_name));

        let score_colored = match score.value {
            85..=100 => format!("{}", score.value).green().to_string(),
            70..=84 => format!("{}", score.value).yellow().to_string(),
            _ => format!("{}", score.value).red().to_string(),
        };
        out.push_str(&format!(
            "\n  Score: {} / 100 — {}\n\n",
            score_colored, score.label
        ));

        let errors = diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count();
        let warnings = diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .count();
        let infos = diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Info)
            .count();
        out.push_str(&format!(
            "  {} errors, {} warnings, {} info\n",
            errors.to_string().red(),
            warnings.to_string().yellow(),
            infos.to_string().blue()
        ));

        let mut by_category: BTreeMap<String, Vec<&Diagnostic>> = BTreeMap::new();
        for d in diagnostics {
            by_category
                .entry(d.category.to_string())
                .or_default()
                .push(d);
        }

        for (category, diags) in &by_category {
            out.push_str(&format!(
                "\n  {} {} {}\n",
                "──".dimmed(),
                category,
                "─".repeat(50 - category.len().min(49)).dimmed()
            ));
            for d in diags {
                let severity_str = match d.severity {
                    Severity::Error => "ERROR".red().bold().to_string(),
                    Severity::Warning => " WARN".yellow().to_string(),
                    Severity::Info => " INFO".blue().to_string(),
                };
                out.push_str(&format!("  {}  {}\n", severity_str, d.rule.dimmed()));
                out.push_str(&format!("         {}\n", d.message));
                if verbose {
                    out.push_str(&format!(
                        "         {}:{}:{}\n",
                        d.file.dimmed(),
                        d.line,
                        d.column
                    ));
                }
                out.push_str(&format!("         {}: {}\n", "Help".cyan(), d.help));
            }
        }
        out.push('\n');
        out
    }
}
