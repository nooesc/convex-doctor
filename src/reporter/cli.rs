use std::collections::BTreeMap;
use std::time::Duration;

use owo_colors::OwoColorize;

use crate::diagnostic::{Diagnostic, Severity};
use crate::scoring::ScoreResult;

use super::Reporter;

pub struct CliReporter;

impl CliReporter {
    fn face_art(score: u32) -> [&'static str; 5] {
        match score {
            85..=100 => [
                "╭─────────╮",
                "│  ^   ^  │",
                "│    △    │",
                "│  ╰───╯  │",
                "╰─────────╯",
            ],
            70..=84 => [
                "╭─────────╮",
                "│  ◦   ◦  │",
                "│    △    │",
                "│  ─────  │",
                "╰─────────╯",
            ],
            50..=69 => [
                "╭─────────╮",
                "│  •   •  │",
                "│    △    │",
                "│  ╭───╮  │",
                "╰─────────╯",
            ],
            _ => [
                "╭─────────╮",
                "│  ×   ×  │",
                "│    △    │",
                "│  ╭───╮  │",
                "╰─────────╯",
            ],
        }
    }

    fn progress_bar(score: u32, width: usize) -> String {
        let filled = (score as usize * width) / 100;
        let empty = width - filled;
        let bar = format!("{}{}", "█".repeat(filled), "░".repeat(empty));
        match score {
            85..=100 => bar.green().to_string(),
            70..=84 => bar.yellow().to_string(),
            _ => bar.red().to_string(),
        }
    }

    fn format_duration(d: Duration) -> String {
        let secs = d.as_secs_f64();
        if secs < 1.0 {
            format!("{:.0}ms", secs * 1000.0)
        } else {
            format!("{:.1}s", secs)
        }
    }

    fn severity_icon(severity: &Severity) -> String {
        match severity {
            Severity::Error => "✖".red().bold().to_string(),
            Severity::Warning => "▲".yellow().to_string(),
            Severity::Info => "●".blue().to_string(),
        }
    }
}

impl Reporter for CliReporter {
    fn format(
        &self,
        diagnostics: &[Diagnostic],
        score: &ScoreResult,
        project_name: &str,
        verbose: bool,
        files_scanned: usize,
        elapsed: Duration,
    ) -> String {
        let mut out = String::new();

        // Header
        out.push_str(&format!(
            "\n  {} v{} {} {}\n\n",
            "convex-doctor".bold(),
            env!("CARGO_PKG_VERSION"),
            "─".dimmed(),
            project_name.bold()
        ));

        // Face + Score side by side
        let face = Self::face_art(score.value);
        let score_colored = match score.value {
            85..=100 => format!("{}", score.value).green().bold().to_string(),
            70..=84 => format!("{}", score.value).yellow().bold().to_string(),
            _ => format!("{}", score.value).red().bold().to_string(),
        };
        let label_colored = match score.value {
            85..=100 => score.label.green().to_string(),
            70..=84 => score.label.yellow().to_string(),
            _ => score.label.red().to_string(),
        };
        let bar = Self::progress_bar(score.value, 34);

        for (i, line) in face.iter().enumerate() {
            out.push_str(&format!("     {}", line));
            match i {
                1 => out.push_str(&format!("   {} / 100", score_colored)),
                2 => out.push_str(&format!("   {}", label_colored)),
                3 => out.push_str(&format!("   {}", bar)),
                _ => {}
            }
            out.push('\n');
        }

        // Summary line
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

        let mut parts = Vec::new();
        if errors > 0 {
            parts.push(format!(
                "{} {}",
                errors.to_string().red().bold(),
                if errors == 1 { "error" } else { "errors" }
            ));
        }
        if warnings > 0 {
            parts.push(format!(
                "{} {}",
                warnings.to_string().yellow().bold(),
                if warnings == 1 { "warning" } else { "warnings" }
            ));
        }
        if infos > 0 {
            parts.push(format!(
                "{} {}",
                infos.to_string().blue(),
                if infos == 1 { "info" } else { "infos" }
            ));
        }

        let findings = if parts.is_empty() {
            "No issues found".green().bold().to_string()
        } else {
            parts.join(", ")
        };

        out.push_str(&format!(
            "\n  {} across {} files in {}\n",
            findings,
            files_scanned.to_string().bold(),
            Self::format_duration(elapsed).dimmed()
        ));

        if diagnostics.is_empty() {
            out.push('\n');
            return out;
        }

        // Group diagnostics by category, then by rule
        let mut by_category: BTreeMap<String, BTreeMap<String, Vec<&Diagnostic>>> = BTreeMap::new();
        for d in diagnostics {
            by_category
                .entry(d.category.to_string())
                .or_default()
                .entry(d.rule.clone())
                .or_default()
                .push(d);
        }

        for (category, rules) in &by_category {
            let pad_len = 54usize.saturating_sub(category.len() + 2);
            out.push_str(&format!(
                "\n  {} {} {}\n",
                "──".dimmed(),
                category.bold(),
                "─".repeat(pad_len).dimmed()
            ));

            for (rule, occurrences) in rules {
                let first = occurrences[0];
                let count = occurrences.len();
                let icon = Self::severity_icon(&first.severity);

                let count_str = if count > 1 {
                    format!(" {}", format!("({})", count).dimmed())
                } else {
                    String::new()
                };

                out.push_str(&format!("   {} {}{}\n", icon, first.message, count_str));
                out.push_str(&format!("     {}\n", rule.dimmed()));
                out.push_str(&format!("     {} {}\n", "Help:".cyan(), first.help));

                if verbose {
                    for d in occurrences {
                        out.push_str(&format!(
                            "      {} {}:{}:{}\n",
                            "→".dimmed(),
                            d.file.dimmed(),
                            d.line,
                            d.column
                        ));
                    }
                }
            }
        }

        out.push('\n');
        out
    }
}
