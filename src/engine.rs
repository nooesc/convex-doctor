use std::collections::HashSet;
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
    pub fail_below: u32,
}

pub fn run(path: &Path, _verbose: bool, diff_base: Option<&str>) -> Result<EngineResult, String> {
    let project = ProjectInfo::detect(path)?;
    let config = Config::load(path)?;
    let registry = RuleRegistry::new();
    let files = project.discover_files(&config);

    // If a diff base is provided, filter to only changed files
    let files = if let Some(base) = diff_base {
        let changed: HashSet<_> = get_changed_files(path, base)
            .into_iter()
            .map(|p| p.canonicalize().unwrap_or(p))
            .collect();
        if changed.is_empty() {
            files // If git diff fails or no changes, scan all
        } else {
            files
                .into_iter()
                .filter(|f| {
                    let canon = f.canonicalize().unwrap_or_else(|_| f.clone());
                    changed.contains(&canon)
                })
                .collect()
        }
    } else {
        files
    };

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
        fail_below: config.ci.fail_below,
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
