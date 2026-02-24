use std::time::Duration;

use serde::Serialize;

use crate::diagnostic::{Diagnostic, Severity};
use crate::scoring::ScoreResult;

use super::Reporter;

pub struct JsonReporter;

#[derive(Serialize)]
struct JsonOutput<'a> {
    version: &'static str,
    score: ScoreJson,
    summary: SummaryJson,
    diagnostics: &'a [Diagnostic],
}

#[derive(Serialize)]
struct ScoreJson {
    value: u32,
    label: String,
}

#[derive(Serialize)]
struct SummaryJson {
    errors: usize,
    warnings: usize,
    infos: usize,
    files_scanned: usize,
}

impl Reporter for JsonReporter {
    fn format(
        &self,
        diagnostics: &[Diagnostic],
        score: &ScoreResult,
        _project_name: &str,
        _verbose: bool,
        files_scanned: usize,
        _elapsed: Duration,
    ) -> String {
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
        let output = JsonOutput {
            version: env!("CARGO_PKG_VERSION"),
            score: ScoreJson {
                value: score.value,
                label: score.label.to_string(),
            },
            summary: SummaryJson {
                errors,
                warnings,
                infos,
                files_scanned,
            },
            diagnostics,
        };
        serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string())
    }
}
