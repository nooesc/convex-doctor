use std::collections::HashSet;
use std::path::{Path, PathBuf};

use rayon::prelude::*;

use crate::config::Config;
use crate::diagnostic::{Category, Diagnostic, Severity};
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
                let changed: HashSet<String> = changed_files
                    .into_iter()
                    .flat_map(|p| normalize_file_paths(&p, path))
                    .collect();

                files
                    .into_iter()
                    .filter(|f| {
                        normalize_file_paths(f, path)
                            .into_iter()
                            .any(|candidate| changed.contains(&candidate))
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
    let analyzed_results: Vec<_> = files
        .par_iter()
        .map(|file| (file, analyze_file(file)))
        .collect();

    let mut analyses = Vec::new();
    let mut parse_diagnostics = Vec::new();
    for (file, result) in analyzed_results {
        match result {
            Ok(analysis) => analyses.push(analysis),
            Err(e) => {
                eprintln!("Warning: {e}");
                if config.is_rule_enabled("correctness/file-parse-error") {
                    parse_diagnostics.push(Diagnostic {
                        rule: "correctness/file-parse-error".to_string(),
                        severity: Severity::Error,
                        category: Category::Correctness,
                        message: format!("Failed to parse file `{}`", file.display()),
                        help: "Fix syntax or parser-incompatible constructs in this file so all rules can run."
                            .to_string(),
                        file: file.display().to_string(),
                        line: 0,
                        column: 0,
                    });
                }
            }
        }
    }

    // Run per-file rules in parallel
    let mut all_diagnostics: Vec<Diagnostic> = analyses
        .par_iter()
        .flat_map(|analysis| registry.run(analysis, &|rule_id| config.is_rule_enabled(rule_id)))
        .collect();
    all_diagnostics.extend(parse_diagnostics);

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
            has_generated_dir: project.convex_dir.join("_generated").is_dir(),
            has_tsconfig: project.convex_dir.join("tsconfig.json").exists(),
            node_version_from_config: read_node_version_from_convex_json(path),
            generated_files_modified: check_generated_files_modified(path),
            all_index_definitions: analyses
                .iter()
                .flat_map(|a| a.index_definitions.clone())
                .collect(),
            all_schema_id_fields: analyses
                .iter()
                .flat_map(|a| a.schema_id_fields.clone())
                .collect(),
            all_filter_field_names: analyses
                .iter()
                .flat_map(|a| a.filter_field_names.clone())
                .collect(),
        };

        let project_diagnostics: Vec<Diagnostic> = registry
            .rules()
            .iter()
            .filter(|r| config.is_rule_enabled(r.id()))
            .flat_map(|r| r.check_project(&project_ctx))
            .collect();

        all_diagnostics.extend(project_diagnostics);
    }

    config.apply_strictness(&mut all_diagnostics);

    let score = compute_score(&all_diagnostics);

    let project_name = path
        .canonicalize()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .or_else(|| path.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| ".".to_string());

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

fn normalize_file_paths(path: &Path, project_root: &Path) -> HashSet<String> {
    let mut paths = HashSet::new();
    let normalized = |path: &Path| path.to_string_lossy().replace('\\', "/");

    let add_relative = |candidate: &Path, paths: &mut HashSet<String>| {
        if let Ok(relative) = candidate.strip_prefix(project_root) {
            let relative = normalized(relative);
            if !relative.is_empty() {
                paths.insert(relative.clone());
                paths.insert(format!("./{relative}"));
            }
        }
    };

    let canonical = path.canonicalize().ok();

    paths.insert(normalized(path));
    if let Some(ref canonical) = canonical {
        paths.insert(normalized(canonical));
    }

    // Include project-root-relative forms for matching against git output.
    add_relative(path, &mut paths);
    if let Some(canonical) = canonical {
        add_relative(&canonical, &mut paths);
    }

    paths
}

pub fn get_changed_files(root: &Path, base: &str) -> Result<Vec<PathBuf>, String> {
    let mut changed_files: std::collections::HashSet<PathBuf> = get_git_paths(
        root,
        &["diff", "--name-only", "--diff-filter=ACMRTUXB", base],
    )?
    .into_iter()
    .map(|p| root.join(p))
    .collect();

    changed_files.extend(
        get_git_paths(root, &["ls-files", "--others", "--exclude-standard"])?
            .into_iter()
            .map(|p| root.join(p)),
    );

    Ok(changed_files.into_iter().collect())
}

fn get_git_paths(root: &Path, args: &[&str]) -> Result<Vec<String>, String> {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|e| format!("failed to run git {args:?}: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let msg = stderr.trim();
        return Err(if msg.is_empty() {
            format!("git command exited with status {}", output.status)
        } else {
            msg.to_string()
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect())
}

fn read_node_version_from_convex_json(root: &Path) -> Option<String> {
    let path = root.join("convex.json");
    let contents = std::fs::read_to_string(&path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&contents).ok()?;
    json.get("node")
        .and_then(|v| v.get("version"))
        .or_else(|| json.get("nodeVersion"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn check_generated_files_modified(root: &Path) -> bool {
    std::process::Command::new("git")
        .args(["status", "--porcelain", "convex/_generated"])
        .current_dir(root)
        .output()
        .ok()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false)
}
