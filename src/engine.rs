use std::collections::HashSet;
use std::path::{Path, PathBuf};

use rayon::prelude::*;

use crate::config::Config;
use crate::diagnostic::Diagnostic;
use crate::project::ProjectInfo;
use crate::rules::context::analyze_file;
use crate::rules::{ProjectContext, RuleRegistry};
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
        match get_changed_files(path, base) {
            Ok(changed_files) => {
                let changed: HashSet<_> = changed_files
                    .into_iter()
                    .map(|p| p.canonicalize().unwrap_or(p))
                    .collect();

                files
                    .into_iter()
                    .filter(|f| {
                        let canon = f.canonicalize().unwrap_or_else(|_| f.clone());
                        changed.contains(&canon)
                    })
                    .collect()
            }
            Err(e) => {
                eprintln!(
                    "Warning: failed to compute changed files for --diff {base}: {e}. Scanning all files."
                );
                files
            }
        }
    } else {
        files
    };

    let files_scanned = files.len();

    // Analyze all files in parallel
    let analyses: Vec<_> = files
        .par_iter()
        .filter_map(|file| match analyze_file(file) {
            Ok(a) => Some(a),
            Err(e) => {
                eprintln!("Warning: {e}");
                None
            }
        })
        .collect();

    // Run per-file rules in parallel
    let mut all_diagnostics: Vec<Diagnostic> = analyses
        .par_iter()
        .flat_map(|analysis| registry.run(analysis, &|rule_id| config.is_rule_enabled(rule_id)))
        .collect();

    // Project-level checks are intentionally skipped in diff mode because
    // they are global and not attributable to changed files.
    if diff_base.is_none() {
        let uses_auth = analyses
            .iter()
            .any(|a| a.functions.iter().any(|f| f.has_auth_check));
        let project_ctx = ProjectContext {
            has_schema: project.has_schema,
            has_auth_config: project.has_auth_config,
            has_convex_json: project.has_convex_json,
            has_env_local: path.join(".env.local").exists(),
            env_gitignored: check_gitignore_contains(path, ".env.local"),
            uses_auth,
        };

        let project_diagnostics: Vec<Diagnostic> = registry
            .rules()
            .iter()
            .filter(|r| config.is_rule_enabled(r.id()))
            .flat_map(|r| r.check_project(&project_ctx))
            .collect();

        all_diagnostics.extend(project_diagnostics);
    }

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

fn check_gitignore_contains(root: &Path, pattern: &str) -> bool {
    let basename = Path::new(pattern)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(pattern);
    let gitignore_path = root.join(".gitignore");
    if let Ok(contents) = std::fs::read_to_string(&gitignore_path) {
        contents.lines().any(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('!') {
                return false;
            }

            let candidate = trimmed.trim_start_matches('/');
            if candidate == pattern || candidate == basename {
                return true;
            }

            glob::Pattern::new(candidate)
                .map(|glob| glob.matches(pattern) || glob.matches(basename))
                .unwrap_or(false)
        })
    } else {
        false
    }
}

pub fn get_changed_files(root: &Path, base: &str) -> Result<Vec<PathBuf>, String> {
    let output = std::process::Command::new("git")
        .args(["diff", "--name-only", base])
        .current_dir(root)
        .output()
        .map_err(|e| format!("failed to run git diff: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let msg = stderr.trim();
        return Err(if msg.is_empty() {
            format!("git diff exited with status {}", output.status)
        } else {
            msg.to_string()
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|l| root.join(l))
        .filter(|p| p.exists())
        .collect())
}
