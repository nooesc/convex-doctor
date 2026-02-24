use std::path::{Path, PathBuf};

use rayon::prelude::*;

use crate::config::Config;
use crate::diagnostic::Diagnostic;
use crate::project::ProjectInfo;
use crate::rules::context::analyze_file;
use crate::rules::RuleRegistry;
use crate::scoring::{compute_score, ScoreResult};

pub struct EngineResult {
    pub diagnostics: Vec<Diagnostic>,
    pub score: ScoreResult,
    pub project_name: String,
    pub files_scanned: usize,
}

pub fn run(path: &Path, _verbose: bool) -> Result<EngineResult, String> {
    let project = ProjectInfo::detect(path)?;
    let config = Config::load(path)?;
    let registry = RuleRegistry::new();
    let files = project.discover_files(&config);
    let files_scanned = files.len();

    let all_diagnostics: Vec<Diagnostic> = files
        .par_iter()
        .flat_map(|file| {
            let analysis = match analyze_file(file) {
                Ok(a) => a,
                Err(e) => {
                    eprintln!("Warning: {e}");
                    return vec![];
                }
            };
            registry.run(&analysis, &|rule_id| config.is_rule_enabled(rule_id))
        })
        .collect();

    let score = compute_score(&all_diagnostics);

    let project_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    Ok(EngineResult {
        diagnostics: all_diagnostics,
        score,
        project_name,
        files_scanned,
    })
}

pub fn get_changed_files(root: &Path, base: &str) -> Vec<PathBuf> {
    let output = std::process::Command::new("git")
        .args(["diff", "--name-only", base])
        .current_dir(root)
        .output();
    match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .lines()
            .map(|l| root.join(l))
            .filter(|p| p.exists())
            .collect(),
        _ => vec![],
    }
}
