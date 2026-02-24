pub mod cli;
pub mod json;

use crate::diagnostic::Diagnostic;
use crate::scoring::ScoreResult;

pub trait Reporter {
    fn format(
        &self,
        diagnostics: &[Diagnostic],
        score: &ScoreResult,
        project_name: &str,
        verbose: bool,
    ) -> String;
}

pub fn score_only(score: &ScoreResult) -> String {
    format!("{}\n", score.value)
}
