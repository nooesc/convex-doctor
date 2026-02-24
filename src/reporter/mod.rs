pub mod cli;
pub mod json;

use std::time::Duration;

use crate::diagnostic::Diagnostic;
use crate::scoring::ScoreResult;

pub trait Reporter {
    fn format(
        &self,
        diagnostics: &[Diagnostic],
        score: &ScoreResult,
        project_name: &str,
        verbose: bool,
        files_scanned: usize,
        elapsed: Duration,
    ) -> String;
}

pub fn score_only(score: &ScoreResult) -> String {
    format!("{}\n", score.value)
}
