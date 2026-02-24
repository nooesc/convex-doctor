# Convex Doctor Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a Rust CLI that scans Convex backend codebases for 30 anti-patterns across 6 categories, outputting a 0-100 health score with actionable diagnostics.

**Architecture:** Single Rust binary using Oxc parser for TypeScript AST analysis. Visitor pattern walks each file's AST running rules. Diagnostics collected, scored, and formatted into CLI/JSON/score-only output. Config file (`convex-doctor.toml`) for customization.

**Tech Stack:** Rust, oxc_parser/oxc_ast/oxc_ast_visit (v0.115), clap, rayon, ignore, owo-colors, serde/serde_json, toml

**Design doc:** `docs/plans/2026-02-23-convex-doctor-design.md`

---

## Task 1: Project Scaffold

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/lib.rs`
- Create: `.gitignore`

**Step 1: Create Cargo.toml**

```toml
[package]
name = "convex-doctor"
version = "0.1.0"
edition = "2021"
description = "Diagnose your Convex backend for anti-patterns, security issues, and performance problems"
license = "MIT"
repository = "https://github.com/coler/convex-doctor"

[dependencies]
oxc_allocator = "0.115"
oxc_parser = "0.115"
oxc_ast = "0.115"
oxc_ast_visit = "0.115"
oxc_span = "0.115"
oxc_diagnostics = "0.115"
clap = { version = "4", features = ["derive"] }
rayon = "1.10"
ignore = "0.4"
owo-colors = "4"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
regex = "1"
miette = { version = "7", features = ["fancy"] }
thiserror = "2"

[dev-dependencies]
tempfile = "3"
insta = { version = "1", features = ["yaml"] }

[profile.release]
lto = true
strip = true
codegen-units = 1
```

**Step 2: Create .gitignore**

```
/target
*.swp
*.swo
.DS_Store
```

**Step 3: Create src/lib.rs with module declarations**

```rust
pub mod config;
pub mod diagnostic;
pub mod engine;
pub mod project;
pub mod reporter;
pub mod rules;
pub mod scoring;
```

**Step 4: Create src/main.rs with minimal CLI**

```rust
use clap::Parser;
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(name = "convex-doctor", version, about = "Diagnose your Convex backend")]
struct Cli {
    /// Path to the project root (defaults to current directory)
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Output format
    #[arg(long, default_value = "cli")]
    format: String,

    /// Only output the score (0-100)
    #[arg(long)]
    score: bool,

    /// Only analyze files changed vs this base branch
    #[arg(long)]
    diff: Option<String>,

    /// Show verbose output with all affected locations
    #[arg(long, short)]
    verbose: bool,
}

fn main() {
    let cli = Cli::parse();

    match convex_doctor::engine::run(&cli.path, cli.verbose) {
        Ok(result) => {
            // TODO: format output based on cli.format / cli.score
            process::exit(0);
        }
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    }
}
```

**Step 5: Create stub modules so it compiles**

Create each of these files with just `// TODO`:
- `src/config.rs`
- `src/diagnostic.rs`
- `src/engine.rs` — needs a stub `pub fn run(path: &Path, verbose: bool) -> Result<(), String> { Ok(()) }`
- `src/project.rs`
- `src/reporter.rs` (as `src/reporter/mod.rs`)
- `src/rules.rs` (as `src/rules/mod.rs`)
- `src/scoring.rs`

**Step 6: Verify it compiles**

Run: `cargo build`
Expected: Compiles with no errors (warnings OK)

**Step 7: Commit**

```bash
git add -A && git commit -m "feat: project scaffold with dependencies and module structure"
```

---

## Task 2: Core Types — Diagnostic, Severity, Category

**Files:**
- Create: `src/diagnostic.rs`
- Create: `tests/diagnostic_test.rs`

**Step 1: Write tests for Diagnostic**

Create `tests/diagnostic_test.rs`:

```rust
use convex_doctor::diagnostic::{Category, Diagnostic, Severity};

#[test]
fn test_diagnostic_creation() {
    let d = Diagnostic {
        rule: "security/missing-arg-validators".to_string(),
        severity: Severity::Error,
        category: Category::Security,
        message: "Public mutation without argument validators".to_string(),
        help: "Add `args: { ... }` with validators".to_string(),
        file: "convex/messages.ts".to_string(),
        line: 14,
        column: 1,
    };
    assert_eq!(d.rule, "security/missing-arg-validators");
    assert_eq!(d.severity, Severity::Error);
    assert_eq!(d.category, Category::Security);
}

#[test]
fn test_severity_display() {
    assert_eq!(format!("{}", Severity::Error), "error");
    assert_eq!(format!("{}", Severity::Warning), "warning");
}

#[test]
fn test_category_weight() {
    assert_eq!(Category::Security.weight(), 1.5);
    assert_eq!(Category::Performance.weight(), 1.2);
    assert_eq!(Category::Correctness.weight(), 1.5);
    assert_eq!(Category::Schema.weight(), 1.0);
    assert_eq!(Category::Architecture.weight(), 0.8);
    assert_eq!(Category::Configuration.weight(), 1.0);
}

#[test]
fn test_category_display() {
    assert_eq!(format!("{}", Category::Security), "Security");
    assert_eq!(format!("{}", Category::Performance), "Performance");
}

#[test]
fn test_diagnostic_serialization() {
    let d = Diagnostic {
        rule: "perf/unbounded-collect".to_string(),
        severity: Severity::Error,
        category: Category::Performance,
        message: "Unbounded .collect()".to_string(),
        help: "Use .take(n) or pagination".to_string(),
        file: "convex/messages.ts".to_string(),
        line: 22,
        column: 10,
    };
    let json = serde_json::to_string(&d).unwrap();
    assert!(json.contains("\"rule\":\"perf/unbounded-collect\""));
    assert!(json.contains("\"severity\":\"error\""));
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --test diagnostic_test`
Expected: FAIL — module doesn't exist yet

**Step 3: Implement src/diagnostic.rs**

```rust
use serde::Serialize;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    pub rule: String,
    pub severity: Severity,
    pub category: Category,
    pub message: String,
    pub help: String,
    pub file: String,
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Error => write!(f, "error"),
            Severity::Warning => write!(f, "warning"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum Category {
    Security,
    Performance,
    Correctness,
    Schema,
    Architecture,
    Configuration,
}

impl Category {
    pub fn weight(&self) -> f64 {
        match self {
            Category::Security => 1.5,
            Category::Performance => 1.2,
            Category::Correctness => 1.5,
            Category::Schema => 1.0,
            Category::Architecture => 0.8,
            Category::Configuration => 1.0,
        }
    }
}

impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Category::Security => write!(f, "Security"),
            Category::Performance => write!(f, "Performance"),
            Category::Correctness => write!(f, "Correctness"),
            Category::Schema => write!(f, "Schema"),
            Category::Architecture => write!(f, "Architecture"),
            Category::Configuration => write!(f, "Configuration"),
        }
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --test diagnostic_test`
Expected: All 5 tests pass

**Step 5: Commit**

```bash
git add src/diagnostic.rs tests/diagnostic_test.rs && git commit -m "feat: core Diagnostic, Severity, and Category types"
```

---

## Task 3: Scoring System

**Files:**
- Create: `src/scoring.rs`
- Create: `tests/scoring_test.rs`

**Step 1: Write tests for scoring**

Create `tests/scoring_test.rs`:

```rust
use convex_doctor::diagnostic::{Category, Diagnostic, Severity};
use convex_doctor::scoring::{compute_score, ScoreResult};

fn make_diagnostic(rule: &str, severity: Severity, category: Category) -> Diagnostic {
    Diagnostic {
        rule: rule.to_string(),
        severity,
        category,
        message: "test".to_string(),
        help: "test".to_string(),
        file: "convex/test.ts".to_string(),
        line: 1,
        column: 1,
    }
}

#[test]
fn test_perfect_score() {
    let result = compute_score(&[]);
    assert_eq!(result.value, 100);
    assert_eq!(result.label, "Healthy");
}

#[test]
fn test_single_error_deduction() {
    let diagnostics = vec![
        make_diagnostic("perf/unbounded-collect", Severity::Error, Category::Performance),
    ];
    let result = compute_score(&diagnostics);
    // error = -3, performance weight = 1.2, deduction = 3.6, score = 96
    assert_eq!(result.value, 96);
    assert_eq!(result.label, "Healthy");
}

#[test]
fn test_single_warning_deduction() {
    let diagnostics = vec![
        make_diagnostic("arch/large-handler", Severity::Warning, Category::Architecture),
    ];
    let result = compute_score(&diagnostics);
    // warning = -1, architecture weight = 0.8, deduction = 0.8, score = 99
    assert_eq!(result.value, 99);
}

#[test]
fn test_security_error_weighted_higher() {
    let diagnostics = vec![
        make_diagnostic("security/missing-auth-check", Severity::Error, Category::Security),
    ];
    let result = compute_score(&diagnostics);
    // error = -3, security weight = 1.5, deduction = 4.5, score = 95
    assert_eq!(result.value, 95);
}

#[test]
fn test_per_rule_cap_errors() {
    // 6 errors of same rule. error = -3 each = -18, but capped at -15 per rule
    let diagnostics: Vec<_> = (0..6)
        .map(|_| make_diagnostic("perf/unbounded-collect", Severity::Error, Category::Performance))
        .collect();
    let result = compute_score(&diagnostics);
    // capped at -15, weight 1.2, deduction = 18, score = 82
    assert_eq!(result.value, 82);
}

#[test]
fn test_per_rule_cap_warnings() {
    // 6 warnings of same rule. warning = -1 each = -6, but capped at -5 per rule
    let diagnostics: Vec<_> = (0..6)
        .map(|_| make_diagnostic("arch/large-handler", Severity::Warning, Category::Architecture))
        .collect();
    let result = compute_score(&diagnostics);
    // capped at -5, weight 0.8, deduction = 4, score = 96
    assert_eq!(result.value, 96);
}

#[test]
fn test_score_floor_at_zero() {
    // Many errors to push below 0
    let diagnostics: Vec<_> = (0..50)
        .map(|i| {
            make_diagnostic(
                &format!("security/rule-{i}"),
                Severity::Error,
                Category::Security,
            )
        })
        .collect();
    let result = compute_score(&diagnostics);
    assert_eq!(result.value, 0);
    assert_eq!(result.label, "Critical");
}

#[test]
fn test_score_labels() {
    // 85-100 Healthy, 70-84 Needs attention, 50-69 Unhealthy, 0-49 Critical
    assert_eq!(compute_score(&[]).label, "Healthy");

    // Create enough to get score into each bracket
    let diagnostics_75: Vec<_> = (0..5)
        .map(|i| make_diagnostic(&format!("security/rule-{i}"), Severity::Error, Category::Security))
        .collect();
    let r = compute_score(&diagnostics_75);
    // 5 different rules * -3 * 1.5 = -22.5, score = 77
    assert_eq!(r.label, "Needs attention");
}

#[test]
fn test_multiple_categories() {
    let diagnostics = vec![
        make_diagnostic("security/missing-auth-check", Severity::Error, Category::Security),
        make_diagnostic("perf/unbounded-collect", Severity::Error, Category::Performance),
        make_diagnostic("arch/large-handler", Severity::Warning, Category::Architecture),
    ];
    let result = compute_score(&diagnostics);
    // security: -3 * 1.5 = -4.5
    // perf: -3 * 1.2 = -3.6
    // arch: -1 * 0.8 = -0.8
    // total deduction = 8.9, score = 91
    assert_eq!(result.value, 91);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --test scoring_test`
Expected: FAIL

**Step 3: Implement src/scoring.rs**

```rust
use std::collections::HashMap;

use crate::diagnostic::{Diagnostic, Severity};

#[derive(Debug, Clone)]
pub struct ScoreResult {
    pub value: u32,
    pub label: &'static str,
}

pub fn compute_score(diagnostics: &[Diagnostic]) -> ScoreResult {
    // Group diagnostics by rule, compute raw deductions per rule
    let mut rule_deductions: HashMap<&str, (f64, f64)> = HashMap::new(); // (raw_sum, cap)

    for d in diagnostics {
        let (raw_per_instance, cap) = match d.severity {
            Severity::Error => (3.0, 15.0),
            Severity::Warning => (1.0, 5.0),
        };
        let weight = d.category.weight();
        let entry = rule_deductions
            .entry(&d.rule)
            .or_insert((0.0, cap * weight));
        entry.0 += raw_per_instance * weight;
    }

    let total_deduction: f64 = rule_deductions
        .values()
        .map(|(raw, cap)| raw.min(*cap))
        .sum();

    let score_f64 = (100.0 - total_deduction).max(0.0).min(100.0);
    let value = score_f64.round() as u32;

    let label = match value {
        85..=100 => "Healthy",
        70..=84 => "Needs attention",
        50..=69 => "Unhealthy",
        _ => "Critical",
    };

    ScoreResult { value, label }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --test scoring_test`
Expected: All 9 tests pass

**Step 5: Commit**

```bash
git add src/scoring.rs tests/scoring_test.rs && git commit -m "feat: scoring system with category weights and per-rule caps"
```

---

## Task 4: Configuration

**Files:**
- Create: `src/config.rs`
- Create: `tests/config_test.rs`

**Step 1: Write tests for config parsing**

Create `tests/config_test.rs`:

```rust
use convex_doctor::config::Config;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn test_default_config() {
    let config = Config::default();
    assert_eq!(config.ci.fail_below, 0);
    assert!(config.rules.is_empty());
    assert_eq!(config.ignore.files, vec!["convex/_generated/**"]);
}

#[test]
fn test_load_config_from_toml() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("convex-doctor.toml");
    std::fs::write(
        &config_path,
        r#"
[rules]
"perf/unbounded-collect" = "off"

[ignore]
files = ["convex/_generated/**", "convex/test/**"]

[ci]
fail_below = 70
"#,
    )
    .unwrap();

    let config = Config::load(dir.path()).unwrap();
    assert_eq!(config.rules.get("perf/unbounded-collect").unwrap(), "off");
    assert_eq!(config.ignore.files.len(), 2);
    assert_eq!(config.ci.fail_below, 70);
}

#[test]
fn test_missing_config_uses_defaults() {
    let dir = TempDir::new().unwrap();
    let config = Config::load(dir.path()).unwrap();
    assert_eq!(config.ci.fail_below, 0);
}

#[test]
fn test_is_rule_enabled() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("convex-doctor.toml");
    std::fs::write(
        &config_path,
        r#"
[rules]
"perf/unbounded-collect" = "off"
"#,
    )
    .unwrap();

    let config = Config::load(dir.path()).unwrap();
    assert!(!config.is_rule_enabled("perf/unbounded-collect"));
    assert!(config.is_rule_enabled("security/missing-auth-check"));
}

#[test]
fn test_is_file_ignored() {
    let config = Config::default();
    assert!(config.is_file_ignored(Path::new("convex/_generated/api.d.ts")));
    assert!(!config.is_file_ignored(Path::new("convex/messages.ts")));
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --test config_test`
Expected: FAIL

**Step 3: Implement src/config.rs**

```rust
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    pub rules: HashMap<String, String>,
    pub ignore: IgnoreConfig,
    pub ci: CiConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct IgnoreConfig {
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct CiConfig {
    pub fail_below: u32,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            rules: HashMap::new(),
            ignore: IgnoreConfig::default(),
            ci: CiConfig::default(),
        }
    }
}

impl Default for IgnoreConfig {
    fn default() -> Self {
        IgnoreConfig {
            files: vec!["convex/_generated/**".to_string()],
        }
    }
}

impl Default for CiConfig {
    fn default() -> Self {
        CiConfig { fail_below: 0 }
    }
}

impl Config {
    pub fn load(project_root: &Path) -> Result<Self, String> {
        let config_path = project_root.join("convex-doctor.toml");
        if !config_path.exists() {
            return Ok(Config::default());
        }
        let contents = std::fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read config: {e}"))?;
        let config: Config =
            toml::from_str(&contents).map_err(|e| format!("Failed to parse config: {e}"))?;
        Ok(config)
    }

    pub fn is_rule_enabled(&self, rule_id: &str) -> bool {
        match self.rules.get(rule_id) {
            Some(v) if v == "off" => false,
            _ => true,
        }
    }

    pub fn is_file_ignored(&self, file_path: &Path) -> bool {
        let path_str = file_path.to_string_lossy();
        for pattern in &self.ignore.files {
            if let Ok(glob) = glob::Pattern::new(pattern) {
                if glob.matches(&path_str) {
                    return true;
                }
            }
        }
        false
    }
}
```

**Note:** Add `glob = "0.3"` to `[dependencies]` in `Cargo.toml`.

**Step 4: Run tests to verify they pass**

Run: `cargo test --test config_test`
Expected: All 5 tests pass

**Step 5: Commit**

```bash
git add src/config.rs tests/config_test.rs Cargo.toml && git commit -m "feat: config file loading with rule toggling and file ignoring"
```

---

## Task 5: Project Detection

**Files:**
- Create: `src/project.rs`
- Create: `tests/project_test.rs`

**Step 1: Write tests**

Create `tests/project_test.rs`:

```rust
use convex_doctor::project::ProjectInfo;
use tempfile::TempDir;

#[test]
fn test_detect_convex_project() {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir(dir.path().join("convex")).unwrap();
    std::fs::write(dir.path().join("convex/schema.ts"), "// schema").unwrap();
    std::fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies": {"convex": "^1.17.0"}}"#,
    )
    .unwrap();

    let info = ProjectInfo::detect(dir.path()).unwrap();
    assert!(info.convex_dir.exists());
    assert!(info.has_schema);
    assert_eq!(info.convex_version, Some("^1.17.0".to_string()));
}

#[test]
fn test_detect_no_convex_dir() {
    let dir = TempDir::new().unwrap();
    let result = ProjectInfo::detect(dir.path());
    assert!(result.is_err());
}

#[test]
fn test_detect_framework_nextjs() {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir(dir.path().join("convex")).unwrap();
    std::fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies": {"next": "14.0.0", "convex": "1.17.0"}}"#,
    )
    .unwrap();

    let info = ProjectInfo::detect(dir.path()).unwrap();
    assert_eq!(info.framework, Some("nextjs".to_string()));
}

#[test]
fn test_discover_convex_files() {
    let dir = TempDir::new().unwrap();
    let convex_dir = dir.path().join("convex");
    std::fs::create_dir(&convex_dir).unwrap();
    std::fs::write(convex_dir.join("messages.ts"), "// messages").unwrap();
    std::fs::write(convex_dir.join("users.ts"), "// users").unwrap();
    std::fs::create_dir(convex_dir.join("_generated")).unwrap();
    std::fs::write(convex_dir.join("_generated/api.d.ts"), "// generated").unwrap();

    let info = ProjectInfo::detect(dir.path()).unwrap();
    let files = info.discover_files(&convex_doctor::config::Config::default());
    assert_eq!(files.len(), 2);
    assert!(files.iter().all(|f| !f.to_string_lossy().contains("_generated")));
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --test project_test`
Expected: FAIL

**Step 3: Implement src/project.rs**

```rust
use std::path::{Path, PathBuf};

use crate::config::Config;

#[derive(Debug)]
pub struct ProjectInfo {
    pub root: PathBuf,
    pub convex_dir: PathBuf,
    pub has_schema: bool,
    pub has_auth_config: bool,
    pub has_convex_json: bool,
    pub convex_version: Option<String>,
    pub framework: Option<String>,
}

impl ProjectInfo {
    pub fn detect(root: &Path) -> Result<Self, String> {
        let convex_dir = root.join("convex");
        if !convex_dir.is_dir() {
            return Err(format!(
                "No convex/ directory found in {}",
                root.display()
            ));
        }

        let has_schema = convex_dir.join("schema.ts").exists()
            || convex_dir.join("schema.js").exists();
        let has_auth_config = convex_dir.join("auth.config.ts").exists()
            || convex_dir.join("auth.config.js").exists();
        let has_convex_json = root.join("convex.json").exists();

        let (convex_version, framework) = Self::parse_package_json(root);

        Ok(ProjectInfo {
            root: root.to_path_buf(),
            convex_dir,
            has_schema,
            has_auth_config,
            has_convex_json,
            convex_version,
            framework,
        })
    }

    fn parse_package_json(root: &Path) -> (Option<String>, Option<String>) {
        let pkg_path = root.join("package.json");
        let contents = match std::fs::read_to_string(&pkg_path) {
            Ok(c) => c,
            Err(_) => return (None, None),
        };
        let json: serde_json::Value = match serde_json::from_str(&contents) {
            Ok(v) => v,
            Err(_) => return (None, None),
        };

        let deps = json.get("dependencies").and_then(|d| d.as_object());
        let dev_deps = json.get("devDependencies").and_then(|d| d.as_object());

        let convex_version = deps
            .and_then(|d| d.get("convex"))
            .or_else(|| dev_deps.and_then(|d| d.get("convex")))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let framework = if Self::has_dep(deps, dev_deps, "next") {
            Some("nextjs".to_string())
        } else if Self::has_dep(deps, dev_deps, "vite") {
            Some("vite".to_string())
        } else if Self::has_dep(deps, dev_deps, "remix") || Self::has_dep(deps, dev_deps, "@remix-run/node") {
            Some("remix".to_string())
        } else {
            None
        };

        (convex_version, framework)
    }

    fn has_dep(
        deps: Option<&serde_json::Map<String, serde_json::Value>>,
        dev_deps: Option<&serde_json::Map<String, serde_json::Value>>,
        name: &str,
    ) -> bool {
        deps.is_some_and(|d| d.contains_key(name))
            || dev_deps.is_some_and(|d| d.contains_key(name))
    }

    pub fn discover_files(&self, config: &Config) -> Vec<PathBuf> {
        let mut files = Vec::new();
        Self::walk_dir(&self.convex_dir, config, &mut files);
        files.sort();
        files
    }

    fn walk_dir(dir: &Path, config: &Config, files: &mut Vec<PathBuf>) {
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Skip _generated by default
                if path.file_name().is_some_and(|n| n == "_generated") {
                    continue;
                }
                Self::walk_dir(&path, config, files);
            } else if let Some(ext) = path.extension() {
                if matches!(ext.to_str(), Some("ts" | "tsx" | "js" | "jsx")) {
                    if !config.is_file_ignored(&path) {
                        files.push(path);
                    }
                }
            }
        }
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --test project_test`
Expected: All 4 tests pass

**Step 5: Commit**

```bash
git add src/project.rs tests/project_test.rs && git commit -m "feat: project detection — convex dir, schema, framework, file discovery"
```

---

## Task 6: Rule Trait and Registry

**Files:**
- Create: `src/rules/mod.rs`
- Create: `src/rules/context.rs`
- Create: `tests/rules_test.rs`

**Step 1: Write tests**

Create `tests/rules_test.rs`:

```rust
use convex_doctor::diagnostic::{Category, Severity};
use convex_doctor::rules::{ConvexFunction, FileAnalysis, FunctionKind, RuleRegistry};

#[test]
fn test_registry_has_all_categories() {
    let registry = RuleRegistry::new();
    let categories: Vec<Category> = registry.rules().iter().map(|r| r.category()).collect();
    assert!(categories.contains(&Category::Security));
    assert!(categories.contains(&Category::Performance));
    assert!(categories.contains(&Category::Correctness));
}

#[test]
fn test_registry_rule_count() {
    let registry = RuleRegistry::new();
    // Should have at least 10 rules in initial implementation
    assert!(registry.rules().len() >= 10);
}

#[test]
fn test_convex_function_is_public() {
    let public_fn = ConvexFunction {
        name: "getMessages".to_string(),
        kind: FunctionKind::Query,
        has_args_validator: true,
        has_return_validator: false,
        has_auth_check: false,
        handler_line_count: 10,
        span_line: 5,
        span_col: 1,
    };
    assert!(public_fn.is_public());

    let internal_fn = ConvexFunction {
        name: "sendEmail".to_string(),
        kind: FunctionKind::InternalAction,
        has_args_validator: true,
        has_return_validator: false,
        has_auth_check: false,
        handler_line_count: 10,
        span_line: 5,
        span_col: 1,
    };
    assert!(!internal_fn.is_public());
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --test rules_test`
Expected: FAIL

**Step 3: Implement src/rules/mod.rs**

```rust
pub mod context;

// Rule category modules — will be added in subsequent tasks
pub mod security;
pub mod performance;
pub mod correctness;
pub mod schema;
pub mod architecture;
pub mod configuration;

use crate::diagnostic::{Category, Diagnostic};

/// Information extracted from a single file about Convex functions and patterns
#[derive(Debug, Default)]
pub struct FileAnalysis {
    pub file_path: String,
    pub has_use_node: bool,
    pub functions: Vec<ConvexFunction>,
    pub imports: Vec<ImportInfo>,
    pub ctx_calls: Vec<CtxCall>,
    pub collect_calls: Vec<CallLocation>,
    pub filter_calls: Vec<CallLocation>,
    pub date_now_calls: Vec<CallLocation>,
    pub loop_ctx_calls: Vec<CallLocation>,
    pub deprecated_calls: Vec<DeprecatedCall>,
    pub hardcoded_secrets: Vec<CallLocation>,
    pub exported_function_count: u32,
    pub schema_nesting_depth: u32,
    pub schema_array_id_fields: Vec<CallLocation>,
    pub index_definitions: Vec<IndexDef>,
}

#[derive(Debug, Clone)]
pub struct ConvexFunction {
    pub name: String,
    pub kind: FunctionKind,
    pub has_args_validator: bool,
    pub has_return_validator: bool,
    pub has_auth_check: bool,
    pub handler_line_count: u32,
    pub span_line: u32,
    pub span_col: u32,
}

impl ConvexFunction {
    pub fn is_public(&self) -> bool {
        matches!(
            self.kind,
            FunctionKind::Query | FunctionKind::Mutation | FunctionKind::Action | FunctionKind::HttpAction
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FunctionKind {
    Query,
    Mutation,
    Action,
    HttpAction,
    InternalQuery,
    InternalMutation,
    InternalAction,
}

impl FunctionKind {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "query" => Some(FunctionKind::Query),
            "mutation" => Some(FunctionKind::Mutation),
            "action" => Some(FunctionKind::Action),
            "httpAction" => Some(FunctionKind::HttpAction),
            "internalQuery" => Some(FunctionKind::InternalQuery),
            "internalMutation" => Some(FunctionKind::InternalMutation),
            "internalAction" => Some(FunctionKind::InternalAction),
            _ => None,
        }
    }

    pub fn is_action(&self) -> bool {
        matches!(self, FunctionKind::Action | FunctionKind::InternalAction)
    }

    pub fn is_query(&self) -> bool {
        matches!(self, FunctionKind::Query | FunctionKind::InternalQuery)
    }

    pub fn is_mutation(&self) -> bool {
        matches!(self, FunctionKind::Mutation | FunctionKind::InternalMutation)
    }
}

#[derive(Debug, Clone)]
pub struct ImportInfo {
    pub source: String,
    pub specifiers: Vec<String>,
    pub line: u32,
}

#[derive(Debug, Clone)]
pub struct CtxCall {
    pub chain: String, // e.g. "ctx.db.get", "ctx.runMutation"
    pub line: u32,
    pub col: u32,
    pub in_loop: bool,
    pub is_awaited: bool,
    /// Which function kind this call is inside
    pub enclosing_function_kind: Option<FunctionKind>,
    /// First arg if it's an identifier (e.g. api.foo.bar or internal.foo.bar)
    pub first_arg_chain: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CallLocation {
    pub line: u32,
    pub col: u32,
    pub detail: String,
}

#[derive(Debug, Clone)]
pub struct DeprecatedCall {
    pub name: String,
    pub replacement: String,
    pub line: u32,
    pub col: u32,
}

#[derive(Debug, Clone)]
pub struct IndexDef {
    pub table: String,
    pub name: String,
    pub fields: Vec<String>,
    pub line: u32,
}

/// Trait that all rules implement
pub trait Rule: Send + Sync {
    fn id(&self) -> &'static str;
    fn category(&self) -> Category;
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic>;
}

/// Registry of all rules
pub struct RuleRegistry {
    rules: Vec<Box<dyn Rule>>,
}

impl RuleRegistry {
    pub fn new() -> Self {
        let mut rules: Vec<Box<dyn Rule>> = Vec::new();

        // Security rules
        rules.push(Box::new(security::MissingArgValidators));
        rules.push(Box::new(security::MissingReturnValidators));
        rules.push(Box::new(security::MissingAuthCheck));
        rules.push(Box::new(security::InternalApiMisuse));
        rules.push(Box::new(security::HardcodedSecrets));

        // Performance rules
        rules.push(Box::new(performance::UnboundedCollect));
        rules.push(Box::new(performance::FilterWithoutIndex));
        rules.push(Box::new(performance::DateNowInQuery));
        rules.push(Box::new(performance::LoopRunMutation));

        // Correctness rules
        rules.push(Box::new(correctness::UnwaitedPromise));
        rules.push(Box::new(correctness::OldFunctionSyntax));
        rules.push(Box::new(correctness::DbInAction));
        rules.push(Box::new(correctness::DeprecatedApi));

        // Architecture rules
        rules.push(Box::new(architecture::LargeHandler));
        rules.push(Box::new(architecture::MonolithicFile));

        rules
    }

    pub fn rules(&self) -> &[Box<dyn Rule>] {
        &self.rules
    }

    pub fn run(&self, analysis: &FileAnalysis, enabled: &dyn Fn(&str) -> bool) -> Vec<Diagnostic> {
        self.rules
            .iter()
            .filter(|r| enabled(r.id()))
            .flat_map(|r| r.check(analysis))
            .collect()
    }
}
```

**Step 4: Create stub rule modules**

Create each of these files with empty rule structs (implementations in Tasks 7-10):
- `src/rules/security.rs` — stubs for `MissingArgValidators`, `MissingReturnValidators`, `MissingAuthCheck`, `InternalApiMisuse`, `HardcodedSecrets`
- `src/rules/performance.rs` — stubs for `UnboundedCollect`, `FilterWithoutIndex`, `DateNowInQuery`, `LoopRunMutation`
- `src/rules/correctness.rs` — stubs for `UnwaitedPromise`, `OldFunctionSyntax`, `DbInAction`, `DeprecatedApi`
- `src/rules/schema.rs` — empty (added later)
- `src/rules/architecture.rs` — stubs for `LargeHandler`, `MonolithicFile`
- `src/rules/configuration.rs` — empty (added later)
- `src/rules/context.rs` — empty (added later)

Each stub rule implements `Rule` with `check()` returning `vec![]`.

Example stub for `src/rules/security.rs`:

```rust
use crate::diagnostic::{Category, Diagnostic, Severity};
use crate::rules::{FileAnalysis, Rule};

pub struct MissingArgValidators;

impl Rule for MissingArgValidators {
    fn id(&self) -> &'static str {
        "security/missing-arg-validators"
    }

    fn category(&self) -> Category {
        Category::Security
    }

    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        vec![] // TODO: implement
    }
}

// ... same pattern for other stubs
```

**Step 5: Run tests to verify they pass**

Run: `cargo test --test rules_test`
Expected: All 3 tests pass

**Step 6: Commit**

```bash
git add src/rules/ tests/rules_test.rs && git commit -m "feat: Rule trait, registry, and FileAnalysis types"
```

---

## Task 7: AST Analyzer (Oxc Visitor)

This is the core of the project — the visitor that walks the AST and populates `FileAnalysis`.

**Files:**
- Create: `src/rules/context.rs`
- Create: `tests/analyzer_test.rs`
- Create: `tests/fixtures/` (test Convex files)

**Step 1: Create test fixture files**

Create `tests/fixtures/basic_query.ts`:
```typescript
import { query, mutation } from "convex/server";
import { v } from "convex/values";

export const getMessages = query({
  args: { channelId: v.id("channels") },
  returns: v.array(v.object({ body: v.string(), author: v.string() })),
  handler: async (ctx, args) => {
    const identity = await ctx.auth.getUserIdentity();
    return await ctx.db
      .query("messages")
      .withIndex("by_channel", (q) => q.eq("channelId", args.channelId))
      .collect();
  },
});

export const sendMessage = mutation({
  handler: async (ctx, args) => {
    await ctx.db.insert("messages", { body: args.body });
  },
});
```

Create `tests/fixtures/bad_patterns.ts`:
```typescript
import { query, mutation, action } from "convex/server";
import { v } from "convex/values";
import { api } from "./_generated/api";

export const listAll = query({
  handler: async (ctx) => {
    return await ctx.db.query("items").collect();
  },
});

export const filterItems = query({
  args: {},
  handler: async (ctx) => {
    const items = await ctx.db.query("items").filter((q) => q.eq(q.field("status"), "active")).collect();
    const now = Date.now();
    return items;
  },
});

export const processAll = action({
  args: {},
  handler: async (ctx) => {
    const items = await ctx.runQuery(api.items.listAll);
    for (const item of items) {
      await ctx.runMutation(api.items.update, { id: item._id });
    }
    ctx.scheduler.runAfter(0, api.items.cleanup);
  },
});
```

Create `tests/fixtures/use_node.ts`:
```typescript
"use node";

import { action } from "convex/server";

export const sendEmail = action({
  args: {},
  handler: async (ctx) => {
    // Node.js action
    await ctx.runMutation(api.emails.markSent);
  },
});
```

**Step 2: Write tests for the analyzer**

Create `tests/analyzer_test.rs`:

```rust
use convex_doctor::rules::context::analyze_file;
use std::path::Path;

#[test]
fn test_analyze_basic_query() {
    let analysis = analyze_file(Path::new("tests/fixtures/basic_query.ts")).unwrap();
    assert_eq!(analysis.functions.len(), 2);

    let get_messages = &analysis.functions[0];
    assert_eq!(get_messages.name, "getMessages");
    assert!(get_messages.has_args_validator);
    assert!(get_messages.has_return_validator);
    assert!(get_messages.has_auth_check);

    let send_message = &analysis.functions[1];
    assert_eq!(send_message.name, "sendMessage");
    assert!(!send_message.has_args_validator);
    assert!(!send_message.has_return_validator);
    assert!(!send_message.has_auth_check);
}

#[test]
fn test_analyze_bad_patterns() {
    let analysis = analyze_file(Path::new("tests/fixtures/bad_patterns.ts")).unwrap();

    // Should detect unbounded .collect()
    assert!(!analysis.collect_calls.is_empty());

    // Should detect .filter() calls
    assert!(!analysis.filter_calls.is_empty());

    // Should detect Date.now()
    assert!(!analysis.date_now_calls.is_empty());

    // Should detect loop ctx calls (ctx.runMutation in for loop)
    assert!(!analysis.loop_ctx_calls.is_empty());

    // Should detect ctx.scheduler without await
    let unwaited: Vec<_> = analysis.ctx_calls.iter().filter(|c| !c.is_awaited).collect();
    assert!(!unwaited.is_empty());
}

#[test]
fn test_analyze_use_node() {
    let analysis = analyze_file(Path::new("tests/fixtures/use_node.ts")).unwrap();
    assert!(analysis.has_use_node);
}

#[test]
fn test_analyze_missing_args() {
    let analysis = analyze_file(Path::new("tests/fixtures/bad_patterns.ts")).unwrap();
    let list_all = analysis.functions.iter().find(|f| f.name == "listAll").unwrap();
    assert!(!list_all.has_args_validator);
}
```

**Step 3: Run tests to verify they fail**

Run: `cargo test --test analyzer_test`
Expected: FAIL

**Step 4: Implement src/rules/context.rs**

This is the largest single file. It implements the Oxc `Visit` trait to walk the AST and populate `FileAnalysis`.

```rust
use std::path::Path;

use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk, Visit};
use oxc_parser::{ParseOptions, Parser};
use oxc_span::SourceType;

use super::{
    CallLocation, ConvexFunction, CtxCall, DeprecatedCall, FileAnalysis, FunctionKind, ImportInfo,
};

pub fn analyze_file(path: &Path) -> Result<FileAnalysis, String> {
    let source_text =
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
    let source_type =
        SourceType::from_path(path).map_err(|_| format!("Unknown file type: {}", path.display()))?;

    let allocator = Allocator::default();
    let ret = Parser::new(&allocator, &source_text, source_type)
        .with_options(ParseOptions {
            parse_regular_expression: true,
            ..ParseOptions::default()
        })
        .parse();

    if ret.panicked {
        return Err(format!("Parser panicked on {}", path.display()));
    }

    let mut visitor = ConvexVisitor::new(path, &source_text);
    visitor.visit_program(&ret.program);

    Ok(visitor.into_analysis())
}

struct ConvexVisitor<'a> {
    source_text: &'a str,
    analysis: FileAnalysis,
    /// Stack tracking current context: function kind we're inside
    current_function_kind: Option<FunctionKind>,
    /// Track if we're currently inside a Convex function's handler
    in_handler: bool,
    /// Track loop depth for detecting ctx calls in loops
    loop_depth: u32,
    /// Track the current exported variable name
    current_export_name: Option<String>,
    /// Track if current call expression is awaited
    in_await: bool,
    /// Track Convex function being built
    building_function: Option<ConvexFunctionBuilder>,
}

struct ConvexFunctionBuilder {
    name: String,
    kind: FunctionKind,
    has_args_validator: bool,
    has_return_validator: bool,
    has_auth_check: bool,
    handler_line_count: u32,
    span_line: u32,
    span_col: u32,
}

impl<'a> ConvexVisitor<'a> {
    fn new(path: &Path, source_text: &'a str) -> Self {
        ConvexVisitor {
            source_text,
            analysis: FileAnalysis {
                file_path: path.to_string_lossy().to_string(),
                ..FileAnalysis::default()
            },
            current_function_kind: None,
            in_handler: false,
            loop_depth: 0,
            current_export_name: None,
            in_await: false,
            building_function: None,
        }
    }

    fn into_analysis(self) -> FileAnalysis {
        self.analysis
    }

    fn line_col(&self, span: oxc_span::Span) -> (u32, u32) {
        let offset = span.start as usize;
        let line = self.source_text[..offset].matches('\n').count() as u32 + 1;
        let last_newline = self.source_text[..offset].rfind('\n').map_or(0, |p| p + 1);
        let col = (offset - last_newline) as u32 + 1;
        (line, col)
    }

    fn line_count_from_span(&self, span: oxc_span::Span) -> u32 {
        let text = &self.source_text[span.start as usize..span.end as usize];
        text.matches('\n').count() as u32 + 1
    }

    fn resolve_member_chain(&self, expr: &Expression<'a>) -> Option<String> {
        match expr {
            Expression::Identifier(id) => Some(id.name.to_string()),
            Expression::StaticMemberExpression(member) => {
                let obj = self.resolve_member_chain(&member.object)?;
                Some(format!("{}.{}", obj, member.property.name))
            }
            _ => None,
        }
    }

    fn is_convex_fn_call(&self, call: &CallExpression<'a>) -> Option<FunctionKind> {
        if let Expression::Identifier(id) = &call.callee {
            return FunctionKind::from_str(id.name.as_str());
        }
        None
    }

    fn check_function_config_object(&mut self, obj: &ObjectExpression<'a>, kind: FunctionKind, span: oxc_span::Span) {
        let (line, col) = self.line_col(span);
        let mut builder = ConvexFunctionBuilder {
            name: self.current_export_name.clone().unwrap_or_else(|| "anonymous".to_string()),
            kind: kind.clone(),
            has_args_validator: false,
            has_return_validator: false,
            has_auth_check: false,
            handler_line_count: 0,
            span_line: line,
            span_col: col,
        };

        for prop in obj.properties.iter() {
            if let ObjectPropertyKind::ObjectProperty(prop) = prop {
                if let Some(key_name) = prop.key.static_name() {
                    match key_name {
                        "args" => builder.has_args_validator = true,
                        "returns" => builder.has_return_validator = true,
                        "handler" => {
                            let handler_span = match &prop.value {
                                Expression::ArrowFunctionExpression(f) => Some(f.span),
                                Expression::FunctionExpression(f) => Some(f.span),
                                _ => None,
                            };
                            if let Some(s) = handler_span {
                                builder.handler_line_count = self.line_count_from_span(s);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        self.building_function = Some(builder);
        self.current_function_kind = Some(kind);
        self.in_handler = true;
    }
}

impl<'a> Visit<'a> for ConvexVisitor<'a> {
    fn visit_directive(&mut self, directive: &Directive<'a>) {
        if directive.directive.as_str() == "use node" {
            self.analysis.has_use_node = true;
        }
        walk::walk_directive(self, directive);
    }

    fn visit_import_declaration(&mut self, decl: &ImportDeclaration<'a>) {
        let source = decl.source.value.to_string();
        let mut specifiers = Vec::new();
        if let Some(specs) = &decl.specifiers {
            for spec in specs.iter() {
                match spec {
                    ImportDeclarationSpecifier::ImportSpecifier(s) => {
                        specifiers.push(s.local.name.to_string());
                    }
                    ImportDeclarationSpecifier::ImportDefaultSpecifier(s) => {
                        specifiers.push(s.local.name.to_string());
                    }
                    ImportDeclarationSpecifier::ImportNamespaceSpecifier(s) => {
                        specifiers.push(format!("* as {}", s.local.name));
                    }
                }
            }
        }
        let (line, _) = self.line_col(decl.span);
        self.analysis.imports.push(ImportInfo {
            source,
            specifiers,
            line,
        });
        walk::walk_import_declaration(self, decl);
    }

    fn visit_export_named_declaration(&mut self, decl: &ExportNamedDeclaration<'a>) {
        self.analysis.exported_function_count += 1;

        if let Some(Declaration::VariableDeclaration(var_decl)) = &decl.declaration {
            for declarator in var_decl.declarations.iter() {
                if let BindingPatternKind::BindingIdentifier(id) = &declarator.id.kind {
                    self.current_export_name = Some(id.name.to_string());
                }
            }
        }

        walk::walk_export_named_declaration(self, decl);

        // After walking, finalize any function being built
        if let Some(builder) = self.building_function.take() {
            self.analysis.functions.push(ConvexFunction {
                name: builder.name,
                kind: builder.kind,
                has_args_validator: builder.has_args_validator,
                has_return_validator: builder.has_return_validator,
                has_auth_check: builder.has_auth_check,
                handler_line_count: builder.handler_line_count,
                span_line: builder.span_line,
                span_col: builder.span_col,
            });
        }
        self.current_export_name = None;
        self.current_function_kind = None;
        self.in_handler = false;
    }

    fn visit_export_default_declaration(&mut self, decl: &ExportDefaultDeclaration<'a>) {
        self.analysis.exported_function_count += 1;
        self.current_export_name = Some("default".to_string());
        walk::walk_export_default_declaration(self, decl);

        if let Some(builder) = self.building_function.take() {
            self.analysis.functions.push(ConvexFunction {
                name: builder.name,
                kind: builder.kind,
                has_args_validator: builder.has_args_validator,
                has_return_validator: builder.has_return_validator,
                has_auth_check: builder.has_auth_check,
                handler_line_count: builder.handler_line_count,
                span_line: builder.span_line,
                span_col: builder.span_col,
            });
        }
        self.current_export_name = None;
        self.current_function_kind = None;
        self.in_handler = false;
    }

    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        // Check if this is a Convex function definition: query({...}), mutation({...}), etc.
        if let Some(kind) = self.is_convex_fn_call(call) {
            // Check if arg is an object literal (new syntax) or function (old syntax)
            if let Some(Argument::ObjectExpression(obj)) = call.arguments.first() {
                self.check_function_config_object(obj, kind, call.span);
            }
            // Old syntax: query(async (ctx) => ...) — no config object
            // Detected by having a function/arrow as the first arg
        }

        // Check for ctx.* calls
        if let Some(chain) = self.resolve_member_chain(&call.callee) {
            if chain.starts_with("ctx.") {
                let (line, col) = self.line_col(call.span);
                self.analysis.ctx_calls.push(CtxCall {
                    chain: chain.clone(),
                    line,
                    col,
                    in_loop: self.loop_depth > 0,
                    is_awaited: self.in_await,
                    enclosing_function_kind: self.current_function_kind.clone(),
                    first_arg_chain: call
                        .arguments
                        .first()
                        .and_then(|a| match a {
                            Argument::Identifier(id) => Some(id.name.to_string()),
                            Argument::StaticMemberExpression(m) => {
                                self.resolve_member_chain(&Expression::StaticMemberExpression(
                                    // Can't easily reconstruct — use callee approach
                                    todo!()
                                ))
                            }
                            _ => None,
                        })
                        .or_else(|| {
                            call.arguments.first().and_then(|a| {
                                if let Argument::Identifier(id) = a {
                                    Some(id.name.to_string())
                                } else {
                                    None
                                }
                            })
                        }),
                });

                // Track auth check
                if chain == "ctx.auth.getUserIdentity" || chain == "ctx.auth" {
                    if let Some(ref mut builder) = self.building_function {
                        builder.has_auth_check = true;
                    }
                }

                // Track loop calls
                if self.loop_depth > 0
                    && (chain.starts_with("ctx.runMutation")
                        || chain.starts_with("ctx.runQuery")
                        || chain.starts_with("ctx.runAction"))
                {
                    self.analysis.loop_ctx_calls.push(CallLocation {
                        line,
                        col,
                        detail: chain.clone(),
                    });
                }

                // Track db calls in actions
                if chain.starts_with("ctx.db.") {
                    if let Some(ref kind) = self.current_function_kind {
                        if kind.is_action() {
                            // db-in-action detected
                        }
                    }
                }

                // Track scheduler/db calls that should be awaited
                if !self.in_await
                    && (chain.starts_with("ctx.scheduler")
                        || chain.starts_with("ctx.db.patch")
                        || chain.starts_with("ctx.db.insert")
                        || chain.starts_with("ctx.db.replace")
                        || chain.starts_with("ctx.db.delete"))
                {
                    // Will be caught by unwaited-promise rule
                }
            }

            // Check for .collect() calls
            if chain.ends_with(".collect") {
                let (line, col) = self.line_col(call.span);
                self.analysis.collect_calls.push(CallLocation {
                    line,
                    col,
                    detail: chain.clone(),
                });
            }

            // Check for .filter() calls
            if chain.ends_with(".filter") {
                let (line, col) = self.line_col(call.span);
                self.analysis.filter_calls.push(CallLocation {
                    line,
                    col,
                    detail: chain.clone(),
                });
            }

            // Check for Date.now()
            if chain == "Date.now" {
                let (line, col) = self.line_col(call.span);
                self.analysis.date_now_calls.push(CallLocation {
                    line,
                    col,
                    detail: "Date.now()".to_string(),
                });
            }

            // Check for deprecated v.bigint()
            if chain == "v.bigint" {
                let (line, col) = self.line_col(call.span);
                self.analysis.deprecated_calls.push(DeprecatedCall {
                    name: "v.bigint()".to_string(),
                    replacement: "v.int64()".to_string(),
                    line,
                    col,
                });
            }
        }

        // Check for secret patterns in string arguments
        for arg in call.arguments.iter() {
            if let Argument::StringLiteral(s) = arg {
                self.check_secret_pattern(s.value.as_str(), s.span);
            }
        }

        walk::walk_call_expression(self, call);
    }

    fn visit_await_expression(&mut self, expr: &AwaitExpression<'a>) {
        let prev = self.in_await;
        self.in_await = true;
        walk::walk_await_expression(self, expr);
        self.in_await = prev;
    }

    fn visit_for_statement(&mut self, stmt: &ForStatement<'a>) {
        self.loop_depth += 1;
        walk::walk_for_statement(self, stmt);
        self.loop_depth -= 1;
    }

    fn visit_while_statement(&mut self, stmt: &WhileStatement<'a>) {
        self.loop_depth += 1;
        walk::walk_while_statement(self, stmt);
        self.loop_depth -= 1;
    }

    fn visit_for_of_statement(&mut self, stmt: &ForOfStatement<'a>) {
        self.loop_depth += 1;
        walk::walk_for_of_statement(self, stmt);
        self.loop_depth -= 1;
    }

    fn visit_for_in_statement(&mut self, stmt: &ForInStatement<'a>) {
        self.loop_depth += 1;
        walk::walk_for_in_statement(self, stmt);
        self.loop_depth -= 1;
    }

    fn visit_string_literal(&mut self, lit: &StringLiteral<'a>) {
        self.check_secret_pattern(lit.value.as_str(), lit.span);
        walk::walk_string_literal(self, lit);
    }
}

impl<'a> ConvexVisitor<'a> {
    fn check_secret_pattern(&mut self, value: &str, span: oxc_span::Span) {
        // Simple heuristic: check for patterns that look like API keys/tokens
        let secret_patterns = [
            "sk_live_", "sk_test_", "pk_live_", "pk_test_",
            "ghp_", "gho_", "github_pat_",
            "xoxb-", "xoxp-",
            "AIza",
        ];
        for pattern in &secret_patterns {
            if value.starts_with(pattern) {
                let (line, col) = self.line_col(span);
                self.analysis.hardcoded_secrets.push(CallLocation {
                    line,
                    col,
                    detail: format!("Possible hardcoded secret starting with '{pattern}'"),
                });
                return;
            }
        }
    }
}
```

**Important note:** The `first_arg_chain` field in `CtxCall` has a `todo!()` that will need to be simplified. Replace the complex match with a simpler approach that just extracts the first argument if it's a simple identifier or member expression chain. The implementer should simplify this to avoid the `todo!()` — just match on `Argument` variants directly using a helper.

**Step 5: Run tests**

Run: `cargo test --test analyzer_test`
Expected: Tests pass (may need iteration to get the visitor exactly right)

**Step 6: Commit**

```bash
git add src/rules/context.rs tests/analyzer_test.rs tests/fixtures/ && git commit -m "feat: AST analyzer using Oxc visitor for Convex patterns"
```

---

## Task 8: Security Rules Implementation

**Files:**
- Modify: `src/rules/security.rs`
- Create: `tests/security_rules_test.rs`

**Step 1: Write tests**

Create `tests/security_rules_test.rs`:

```rust
use convex_doctor::diagnostic::{Category, Severity};
use convex_doctor::rules::context::analyze_file;
use convex_doctor::rules::security::*;
use convex_doctor::rules::Rule;
use std::path::Path;

#[test]
fn test_missing_arg_validators() {
    let analysis = analyze_file(Path::new("tests/fixtures/bad_patterns.ts")).unwrap();
    let rule = MissingArgValidators;
    let diagnostics = rule.check(&analysis);
    // listAll has no args validator
    assert!(diagnostics.iter().any(|d| d.message.contains("listAll")));
}

#[test]
fn test_no_false_positive_arg_validators() {
    let analysis = analyze_file(Path::new("tests/fixtures/basic_query.ts")).unwrap();
    let rule = MissingArgValidators;
    let diagnostics = rule.check(&analysis);
    // getMessages has args validator, should not be flagged
    assert!(!diagnostics.iter().any(|d| d.message.contains("getMessages")));
    // sendMessage has no args — should be flagged
    assert!(diagnostics.iter().any(|d| d.message.contains("sendMessage")));
}

#[test]
fn test_missing_auth_check() {
    let analysis = analyze_file(Path::new("tests/fixtures/bad_patterns.ts")).unwrap();
    let rule = MissingAuthCheck;
    let diagnostics = rule.check(&analysis);
    // Public functions without auth check should be flagged
    assert!(!diagnostics.is_empty());
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --test security_rules_test`
Expected: FAIL (rules return empty vecs)

**Step 3: Implement src/rules/security.rs**

```rust
use crate::diagnostic::{Category, Diagnostic, Severity};
use crate::rules::{FileAnalysis, Rule};

pub struct MissingArgValidators;

impl Rule for MissingArgValidators {
    fn id(&self) -> &'static str { "security/missing-arg-validators" }
    fn category(&self) -> Category { Category::Security }

    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis.functions.iter()
            .filter(|f| f.is_public() && !f.has_args_validator)
            .map(|f| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Error,
                category: self.category(),
                message: format!("Public {} `{}` has no argument validators", f.kind_str(), f.name),
                help: "Add `args: { ... }` with validators for all parameters. Public functions can be called by anyone.".to_string(),
                file: analysis.file_path.clone(),
                line: f.span_line,
                column: f.span_col,
            })
            .collect()
    }
}

pub struct MissingReturnValidators;

impl Rule for MissingReturnValidators {
    fn id(&self) -> &'static str { "security/missing-return-validators" }
    fn category(&self) -> Category { Category::Security }

    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis.functions.iter()
            .filter(|f| !f.has_return_validator)
            .map(|f| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Warning,
                category: self.category(),
                message: format!("Function `{}` has no return value validator", f.name),
                help: "Add `returns: v.null()` or the appropriate validator for type safety.".to_string(),
                file: analysis.file_path.clone(),
                line: f.span_line,
                column: f.span_col,
            })
            .collect()
    }
}

pub struct MissingAuthCheck;

impl Rule for MissingAuthCheck {
    fn id(&self) -> &'static str { "security/missing-auth-check" }
    fn category(&self) -> Category { Category::Security }

    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis.functions.iter()
            .filter(|f| f.is_public() && !f.has_auth_check)
            .map(|f| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Error,
                category: self.category(),
                message: format!("Public {} `{}` does not check authentication", f.kind_str(), f.name),
                help: "Call `await ctx.auth.getUserIdentity()` and handle unauthenticated users.".to_string(),
                file: analysis.file_path.clone(),
                line: f.span_line,
                column: f.span_col,
            })
            .collect()
    }
}

pub struct InternalApiMisuse;

impl Rule for InternalApiMisuse {
    fn id(&self) -> &'static str { "security/internal-api-misuse" }
    fn category(&self) -> Category { Category::Security }

    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis.ctx_calls.iter()
            .filter(|c| {
                (c.chain.starts_with("ctx.scheduler") || c.chain.starts_with("ctx.runMutation")
                    || c.chain.starts_with("ctx.runQuery") || c.chain.starts_with("ctx.runAction"))
                    && c.first_arg_chain.as_deref().is_some_and(|a| a.starts_with("api."))
            })
            .map(|c| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Error,
                category: self.category(),
                message: format!("Using `api.*` in `{}` — use `internal.*` instead", c.chain),
                help: "Replace `api.` with `internal.` for server-side calls. Using `api.*` exposes the endpoint to external attackers.".to_string(),
                file: analysis.file_path.clone(),
                line: c.line,
                column: c.col,
            })
            .collect()
    }
}

pub struct HardcodedSecrets;

impl Rule for HardcodedSecrets {
    fn id(&self) -> &'static str { "security/hardcoded-secrets" }
    fn category(&self) -> Category { Category::Security }

    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis.hardcoded_secrets.iter()
            .map(|s| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Error,
                category: self.category(),
                message: s.detail.clone(),
                help: "Use Convex environment variables instead of hardcoding secrets.".to_string(),
                file: analysis.file_path.clone(),
                line: s.line,
                column: s.col,
            })
            .collect()
    }
}
```

**Note:** Add a `kind_str()` method to `ConvexFunction`:

```rust
impl ConvexFunction {
    pub fn kind_str(&self) -> &'static str {
        match self.kind {
            FunctionKind::Query => "query",
            FunctionKind::Mutation => "mutation",
            FunctionKind::Action => "action",
            FunctionKind::HttpAction => "httpAction",
            FunctionKind::InternalQuery => "internalQuery",
            FunctionKind::InternalMutation => "internalMutation",
            FunctionKind::InternalAction => "internalAction",
        }
    }
}
```

**Step 4: Run tests**

Run: `cargo test --test security_rules_test`
Expected: All 3 tests pass

**Step 5: Commit**

```bash
git add src/rules/security.rs tests/security_rules_test.rs && git commit -m "feat: security rules — missing validators, auth checks, API misuse, secrets"
```

---

## Task 9: Performance & Correctness Rules

**Files:**
- Modify: `src/rules/performance.rs`
- Modify: `src/rules/correctness.rs`
- Create: `tests/perf_rules_test.rs`
- Create: `tests/correctness_rules_test.rs`

**Step 1: Write performance rule tests**

Create `tests/perf_rules_test.rs`:

```rust
use convex_doctor::rules::context::analyze_file;
use convex_doctor::rules::performance::*;
use convex_doctor::rules::Rule;
use std::path::Path;

#[test]
fn test_unbounded_collect() {
    let analysis = analyze_file(Path::new("tests/fixtures/bad_patterns.ts")).unwrap();
    let rule = UnboundedCollect;
    let diagnostics = rule.check(&analysis);
    assert!(!diagnostics.is_empty());
}

#[test]
fn test_date_now_in_query() {
    let analysis = analyze_file(Path::new("tests/fixtures/bad_patterns.ts")).unwrap();
    let rule = DateNowInQuery;
    let diagnostics = rule.check(&analysis);
    assert!(!diagnostics.is_empty());
}

#[test]
fn test_loop_run_mutation() {
    let analysis = analyze_file(Path::new("tests/fixtures/bad_patterns.ts")).unwrap();
    let rule = LoopRunMutation;
    let diagnostics = rule.check(&analysis);
    assert!(!diagnostics.is_empty());
}
```

**Step 2: Write correctness rule tests**

Create `tests/correctness_rules_test.rs`:

```rust
use convex_doctor::rules::context::analyze_file;
use convex_doctor::rules::correctness::*;
use convex_doctor::rules::Rule;
use std::path::Path;

#[test]
fn test_unwaited_promise() {
    let analysis = analyze_file(Path::new("tests/fixtures/bad_patterns.ts")).unwrap();
    let rule = UnwaitedPromise;
    let diagnostics = rule.check(&analysis);
    // ctx.scheduler.runAfter without await should be caught
    assert!(!diagnostics.is_empty());
}

#[test]
fn test_deprecated_api() {
    // Create a fixture with deprecated API usage
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("deprecated.ts");
    std::fs::write(&path, r#"
import { mutation } from "convex/server";
import { v } from "convex/values";

export const create = mutation({
  args: { count: v.bigint() },
  handler: async (ctx, args) => {},
});
"#).unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = DeprecatedApi;
    let diagnostics = rule.check(&analysis);
    assert!(diagnostics.iter().any(|d| d.message.contains("v.bigint()")));
}
```

**Step 3: Run tests to verify they fail**

Run: `cargo test --test perf_rules_test --test correctness_rules_test`
Expected: FAIL

**Step 4: Implement src/rules/performance.rs**

```rust
use crate::diagnostic::{Category, Diagnostic, Severity};
use crate::rules::{FileAnalysis, Rule};

pub struct UnboundedCollect;

impl Rule for UnboundedCollect {
    fn id(&self) -> &'static str { "perf/unbounded-collect" }
    fn category(&self) -> Category { Category::Performance }

    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis.collect_calls.iter()
            .map(|c| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Error,
                category: self.category(),
                message: format!("Unbounded `.collect()` call"),
                help: "Use `.take(n)` to limit results or implement pagination with `paginationOptsValidator`. All results count toward database bandwidth.".to_string(),
                file: analysis.file_path.clone(),
                line: c.line,
                column: c.col,
            })
            .collect()
    }
}

pub struct FilterWithoutIndex;

impl Rule for FilterWithoutIndex {
    fn id(&self) -> &'static str { "perf/filter-without-index" }
    fn category(&self) -> Category { Category::Performance }

    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis.filter_calls.iter()
            .map(|c| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Warning,
                category: self.category(),
                message: "Using `.filter()` — consider `.withIndex()` for better performance".to_string(),
                help: "`.filter()` performs a full table scan. Add an index on the filtered field in schema.ts and use `.withIndex()`.".to_string(),
                file: analysis.file_path.clone(),
                line: c.line,
                column: c.col,
            })
            .collect()
    }
}

pub struct DateNowInQuery;

impl Rule for DateNowInQuery {
    fn id(&self) -> &'static str { "perf/date-now-in-query" }
    fn category(&self) -> Category { Category::Performance }

    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        // Only flag Date.now() inside query functions
        let in_query = analysis.functions.iter().any(|f| f.kind.is_query());
        if !in_query {
            return vec![];
        }

        analysis.date_now_calls.iter()
            .map(|c| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Error,
                category: self.category(),
                message: "`Date.now()` in query function breaks caching".to_string(),
                help: "Queries with `Date.now()` never cache properly and invalidate too frequently. Pass the timestamp as an argument from the client instead.".to_string(),
                file: analysis.file_path.clone(),
                line: c.line,
                column: c.col,
            })
            .collect()
    }
}

pub struct LoopRunMutation;

impl Rule for LoopRunMutation {
    fn id(&self) -> &'static str { "perf/loop-run-mutation" }
    fn category(&self) -> Category { Category::Performance }

    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis.loop_ctx_calls.iter()
            .map(|c| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Error,
                category: self.category(),
                message: format!("`{}` called inside a loop", c.detail),
                help: "Each call creates a separate transaction. Batch operations into a single mutation instead.".to_string(),
                file: analysis.file_path.clone(),
                line: c.line,
                column: c.col,
            })
            .collect()
    }
}
```

**Step 5: Implement src/rules/correctness.rs**

```rust
use crate::diagnostic::{Category, Diagnostic, Severity};
use crate::rules::{FileAnalysis, Rule};

pub struct UnwaitedPromise;

impl Rule for UnwaitedPromise {
    fn id(&self) -> &'static str { "correctness/unwaited-promise" }
    fn category(&self) -> Category { Category::Correctness }

    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        let awaitable_prefixes = [
            "ctx.scheduler", "ctx.db.patch", "ctx.db.insert",
            "ctx.db.replace", "ctx.db.delete", "ctx.runMutation",
            "ctx.runQuery", "ctx.runAction",
        ];

        analysis.ctx_calls.iter()
            .filter(|c| {
                !c.is_awaited && awaitable_prefixes.iter().any(|p| c.chain.starts_with(p))
            })
            .map(|c| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Error,
                category: self.category(),
                message: format!("`{}` called without `await`", c.chain),
                help: "Missing `await` causes the operation to run fire-and-forget. Errors will be silently swallowed.".to_string(),
                file: analysis.file_path.clone(),
                line: c.line,
                column: c.col,
            })
            .collect()
    }
}

pub struct OldFunctionSyntax;

impl Rule for OldFunctionSyntax {
    fn id(&self) -> &'static str { "correctness/old-function-syntax" }
    fn category(&self) -> Category { Category::Correctness }

    fn check(&self, _analysis: &FileAnalysis) -> Vec<Diagnostic> {
        // This requires detecting query(async (ctx) => ...) vs query({ handler: ... })
        // The analyzer flags this by checking if a Convex function call's first arg
        // is a function rather than an object. This will be tracked in FileAnalysis
        // as functions without has_args_validator AND without being in the new object form.
        // For now, this needs the analyzer to explicitly track old-syntax functions.
        vec![] // TODO: needs analyzer enhancement to track syntax form
    }
}

pub struct DbInAction;

impl Rule for DbInAction {
    fn id(&self) -> &'static str { "correctness/db-in-action" }
    fn category(&self) -> Category { Category::Correctness }

    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis.ctx_calls.iter()
            .filter(|c| {
                c.chain.starts_with("ctx.db.")
                    && c.enclosing_function_kind.as_ref().is_some_and(|k| k.is_action())
            })
            .map(|c| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Error,
                category: self.category(),
                message: format!("`{}` used in an action — actions cannot access the database directly", c.chain),
                help: "Use `ctx.runQuery()` or `ctx.runMutation()` to access the database from an action.".to_string(),
                file: analysis.file_path.clone(),
                line: c.line,
                column: c.col,
            })
            .collect()
    }
}

pub struct DeprecatedApi;

impl Rule for DeprecatedApi {
    fn id(&self) -> &'static str { "correctness/deprecated-api" }
    fn category(&self) -> Category { Category::Correctness }

    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis.deprecated_calls.iter()
            .map(|d| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Warning,
                category: self.category(),
                message: format!("`{}` is deprecated", d.name),
                help: format!("Use `{}` instead.", d.replacement),
                file: analysis.file_path.clone(),
                line: d.line,
                column: d.col,
            })
            .collect()
    }
}
```

**Step 6: Run tests**

Run: `cargo test --test perf_rules_test --test correctness_rules_test`
Expected: All tests pass

**Step 7: Commit**

```bash
git add src/rules/performance.rs src/rules/correctness.rs tests/perf_rules_test.rs tests/correctness_rules_test.rs && git commit -m "feat: performance and correctness rules — collect, filter, Date.now, loops, unwaited promises"
```

---

## Task 10: Architecture Rules

**Files:**
- Modify: `src/rules/architecture.rs`
- Create: `tests/arch_rules_test.rs`

**Step 1: Write tests**

Create `tests/arch_rules_test.rs`:

```rust
use convex_doctor::rules::architecture::*;
use convex_doctor::rules::{ConvexFunction, FileAnalysis, FunctionKind, Rule};

#[test]
fn test_large_handler() {
    let analysis = FileAnalysis {
        file_path: "convex/test.ts".to_string(),
        functions: vec![ConvexFunction {
            name: "bigFunction".to_string(),
            kind: FunctionKind::Mutation,
            has_args_validator: true,
            has_return_validator: false,
            has_auth_check: false,
            handler_line_count: 80,
            span_line: 1,
            span_col: 1,
        }],
        ..Default::default()
    };
    let rule = LargeHandler;
    let diagnostics = rule.check(&analysis);
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("80 lines"));
}

#[test]
fn test_monolithic_file() {
    let analysis = FileAnalysis {
        file_path: "convex/everything.ts".to_string(),
        exported_function_count: 12,
        ..Default::default()
    };
    let rule = MonolithicFile;
    let diagnostics = rule.check(&analysis);
    assert_eq!(diagnostics.len(), 1);
}
```

**Step 2: Implement src/rules/architecture.rs**

```rust
use crate::diagnostic::{Category, Diagnostic, Severity};
use crate::rules::{FileAnalysis, Rule};

pub struct LargeHandler;

impl Rule for LargeHandler {
    fn id(&self) -> &'static str { "arch/large-handler" }
    fn category(&self) -> Category { Category::Architecture }

    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis.functions.iter()
            .filter(|f| f.handler_line_count > 50)
            .map(|f| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Warning,
                category: self.category(),
                message: format!("Handler `{}` is {} lines long", f.name, f.handler_line_count),
                help: "Extract logic into helper functions. Keep handlers focused on validation, auth, and orchestration.".to_string(),
                file: analysis.file_path.clone(),
                line: f.span_line,
                column: f.span_col,
            })
            .collect()
    }
}

pub struct MonolithicFile;

impl Rule for MonolithicFile {
    fn id(&self) -> &'static str { "arch/monolithic-file" }
    fn category(&self) -> Category { Category::Architecture }

    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        if analysis.exported_function_count > 10 {
            vec![Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Warning,
                category: self.category(),
                message: format!("File has {} exported functions", analysis.exported_function_count),
                help: "Split into smaller files organized by feature (e.g., convex/users.ts, convex/messages.ts).".to_string(),
                file: analysis.file_path.clone(),
                line: 1,
                column: 1,
            }]
        } else {
            vec![]
        }
    }
}
```

**Step 3: Run tests**

Run: `cargo test --test arch_rules_test`
Expected: All 2 tests pass

**Step 4: Commit**

```bash
git add src/rules/architecture.rs tests/arch_rules_test.rs && git commit -m "feat: architecture rules — large handlers, monolithic files"
```

---

## Task 11: Reporters (CLI, JSON, Score-Only)

**Files:**
- Create: `src/reporter/mod.rs`
- Create: `src/reporter/cli.rs`
- Create: `src/reporter/json.rs`
- Create: `tests/reporter_test.rs`

**Step 1: Write tests**

Create `tests/reporter_test.rs`:

```rust
use convex_doctor::diagnostic::{Category, Diagnostic, Severity};
use convex_doctor::reporter::json::JsonReporter;
use convex_doctor::reporter::Reporter;
use convex_doctor::scoring::{compute_score, ScoreResult};

fn sample_diagnostics() -> Vec<Diagnostic> {
    vec![
        Diagnostic {
            rule: "security/missing-auth-check".to_string(),
            severity: Severity::Error,
            category: Category::Security,
            message: "Public query `getMessages` does not check authentication".to_string(),
            help: "Call `await ctx.auth.getUserIdentity()`".to_string(),
            file: "convex/messages.ts".to_string(),
            line: 5,
            column: 1,
        },
        Diagnostic {
            rule: "perf/unbounded-collect".to_string(),
            severity: Severity::Error,
            category: Category::Performance,
            message: "Unbounded `.collect()` call".to_string(),
            help: "Use `.take(n)` to limit results".to_string(),
            file: "convex/messages.ts".to_string(),
            line: 22,
            column: 10,
        },
    ]
}

#[test]
fn test_json_output_structure() {
    let diagnostics = sample_diagnostics();
    let score = compute_score(&diagnostics);
    let reporter = JsonReporter;
    let output = reporter.format(&diagnostics, &score, "my-app", false);
    let json: serde_json::Value = serde_json::from_str(&output).unwrap();

    assert!(json["score"]["value"].is_number());
    assert!(json["diagnostics"].is_array());
    assert_eq!(json["diagnostics"].as_array().unwrap().len(), 2);
}

#[test]
fn test_score_only_output() {
    let diagnostics = sample_diagnostics();
    let score = compute_score(&diagnostics);
    let output = convex_doctor::reporter::score_only(&score);
    let parsed: u32 = output.trim().parse().unwrap();
    assert!(parsed <= 100);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --test reporter_test`
Expected: FAIL

**Step 3: Implement src/reporter/mod.rs**

```rust
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
```

**Step 4: Implement src/reporter/json.rs**

```rust
use serde::Serialize;

use crate::diagnostic::Diagnostic;
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
}

impl Reporter for JsonReporter {
    fn format(
        &self,
        diagnostics: &[Diagnostic],
        score: &ScoreResult,
        _project_name: &str,
        _verbose: bool,
    ) -> String {
        let errors = diagnostics.iter().filter(|d| d.severity == crate::diagnostic::Severity::Error).count();
        let warnings = diagnostics.len() - errors;

        let output = JsonOutput {
            version: env!("CARGO_PKG_VERSION"),
            score: ScoreJson {
                value: score.value,
                label: score.label.to_string(),
            },
            summary: SummaryJson { errors, warnings },
            diagnostics,
        };

        serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string())
    }
}
```

**Step 5: Implement src/reporter/cli.rs**

```rust
use std::collections::BTreeMap;

use owo_colors::OwoColorize;

use crate::diagnostic::{Category, Diagnostic, Severity};
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

        // Header
        out.push_str(&format!("\n  {} v{}\n\n", "convex-doctor".bold(), env!("CARGO_PKG_VERSION")));
        out.push_str(&format!("  Project: {}\n", project_name));

        // Score
        let score_colored = match score.value {
            85..=100 => format!("{}", score.value).green().to_string(),
            70..=84 => format!("{}", score.value).yellow().to_string(),
            50..=69 => format!("{}", score.value).red().to_string(),
            _ => format!("{}", score.value).red().bold().to_string(),
        };
        out.push_str(&format!("\n  Score: {} / 100 — {}\n\n", score_colored, score.label));

        // Summary counts
        let errors = diagnostics.iter().filter(|d| d.severity == Severity::Error).count();
        let warnings = diagnostics.len() - errors;
        out.push_str(&format!("  {} errors, {} warnings\n", errors.to_string().red(), warnings.to_string().yellow()));

        // Group by category
        let mut by_category: BTreeMap<String, Vec<&Diagnostic>> = BTreeMap::new();
        for d in diagnostics {
            by_category.entry(d.category.to_string()).or_default().push(d);
        }

        for (category, diags) in &by_category {
            out.push_str(&format!("\n  {} {} {}\n", "──".dimmed(), category, "─".repeat(50 - category.len()).dimmed()));
            for d in diags {
                let severity_str = match d.severity {
                    Severity::Error => "ERROR".red().bold().to_string(),
                    Severity::Warning => " WARN".yellow().to_string(),
                };
                out.push_str(&format!("  {}  {}\n", severity_str, d.rule.dimmed()));
                out.push_str(&format!("         {}\n", d.message));
                if verbose {
                    out.push_str(&format!("         {}:{}:{}\n", d.file.dimmed(), d.line, d.column));
                }
                out.push_str(&format!("         {}: {}\n", "Help".cyan(), d.help));
            }
        }

        out.push('\n');
        out
    }
}
```

**Step 6: Run tests**

Run: `cargo test --test reporter_test`
Expected: All 2 tests pass

**Step 7: Commit**

```bash
git add src/reporter/ tests/reporter_test.rs && git commit -m "feat: CLI and JSON reporters with score-only mode"
```

---

## Task 12: Engine — Orchestrate Everything

**Files:**
- Modify: `src/engine.rs`
- Modify: `src/main.rs`
- Create: `tests/engine_test.rs`

**Step 1: Write integration test**

Create `tests/engine_test.rs`:

```rust
use tempfile::TempDir;

#[test]
fn test_engine_on_sample_project() {
    let dir = TempDir::new().unwrap();
    let convex_dir = dir.path().join("convex");
    std::fs::create_dir(&convex_dir).unwrap();
    std::fs::write(
        convex_dir.join("messages.ts"),
        r#"
import { query } from "convex/server";

export const getMessages = query({
  handler: async (ctx) => {
    return await ctx.db.query("messages").collect();
  },
});
"#,
    )
    .unwrap();

    let result = convex_doctor::engine::run(dir.path(), false).unwrap();
    assert!(result.score.value < 100); // Should have issues
    assert!(!result.diagnostics.is_empty());
}

#[test]
fn test_engine_no_convex_dir() {
    let dir = TempDir::new().unwrap();
    let result = convex_doctor::engine::run(dir.path(), false);
    assert!(result.is_err());
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --test engine_test`
Expected: FAIL

**Step 3: Implement src/engine.rs**

```rust
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

pub fn run(path: &std::path::Path, _verbose: bool) -> Result<EngineResult, String> {
    let project = ProjectInfo::detect(path)?;
    let config = Config::load(path)?;
    let registry = RuleRegistry::new();

    let files = project.discover_files(&config);
    let files_scanned = files.len();

    // Analyze files in parallel
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
```

**Step 4: Update src/main.rs for full CLI**

```rust
use clap::Parser;
use std::path::PathBuf;
use std::process;

use convex_doctor::reporter::cli::CliReporter;
use convex_doctor::reporter::json::JsonReporter;
use convex_doctor::reporter::Reporter;

#[derive(Parser)]
#[command(name = "convex-doctor", version, about = "Diagnose your Convex backend")]
struct Cli {
    /// Path to the project root (defaults to current directory)
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Output format: cli, json
    #[arg(long, default_value = "cli")]
    format: String,

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

    let result = match convex_doctor::engine::run(&cli.path, cli.verbose) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    };

    if cli.score {
        print!("{}", convex_doctor::reporter::score_only(&result.score));
    } else {
        let output = match cli.format.as_str() {
            "json" => {
                let reporter = JsonReporter;
                reporter.format(&result.diagnostics, &result.score, &result.project_name, cli.verbose)
            }
            _ => {
                let reporter = CliReporter;
                reporter.format(&result.diagnostics, &result.score, &result.project_name, cli.verbose)
            }
        };
        print!("{output}");
    }

    // Exit with non-zero if score is below CI threshold
    let config = convex_doctor::config::Config::load(&cli.path).unwrap_or_default();
    if config.ci.fail_below > 0 && result.score.value < config.ci.fail_below {
        process::exit(1);
    }
}
```

**Step 5: Run tests**

Run: `cargo test --test engine_test`
Expected: All 2 tests pass

**Step 6: Run full test suite**

Run: `cargo test`
Expected: All tests pass

**Step 7: Test the binary manually**

Run: `cargo run -- tests/fixtures/` (or a directory with a convex/ subdirectory)

**Step 8: Commit**

```bash
git add src/engine.rs src/main.rs tests/engine_test.rs && git commit -m "feat: engine orchestration and full CLI with all output formats"
```

---

## Task 13: Diff Mode

**Files:**
- Modify: `src/engine.rs`
- Modify: `src/main.rs`
- Create: `tests/diff_test.rs`

**Step 1: Write test**

Create `tests/diff_test.rs`:

```rust
use convex_doctor::engine::get_changed_files;
use std::path::Path;

#[test]
fn test_get_changed_files_returns_empty_on_no_git() {
    let dir = tempfile::TempDir::new().unwrap();
    let files = get_changed_files(dir.path(), "main");
    assert!(files.is_empty());
}
```

**Step 2: Add diff support to engine.rs**

Add this function to `src/engine.rs`:

```rust
pub fn get_changed_files(root: &std::path::Path, base: &str) -> Vec<PathBuf> {
    let output = std::process::Command::new("git")
        .args(["diff", "--name-only", base])
        .current_dir(root)
        .output();

    match output {
        Ok(o) if o.status.success() => {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(|l| root.join(l))
                .filter(|p| p.exists())
                .collect()
        }
        _ => vec![],
    }
}
```

Update the `run` function to accept an optional `diff_base` parameter and filter files accordingly.

**Step 3: Run tests**

Run: `cargo test --test diff_test`
Expected: Pass

**Step 4: Commit**

```bash
git add src/engine.rs tests/diff_test.rs && git commit -m "feat: diff mode — analyze only changed files vs base branch"
```

---

## Task 14: End-to-End Integration Test

**Files:**
- Create: `tests/e2e_test.rs`
- Create: `tests/fixtures/sample_project/` (full sample Convex project)

**Step 1: Create a realistic sample project**

Create `tests/fixtures/sample_project/convex/schema.ts`:
```typescript
import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  messages: defineTable({
    body: v.string(),
    author: v.id("users"),
    channelId: v.id("channels"),
  }).index("by_channel", ["channelId"]),
  users: defineTable({
    name: v.string(),
    email: v.string(),
  }),
  channels: defineTable({
    name: v.string(),
  }),
});
```

Create `tests/fixtures/sample_project/convex/messages.ts`:
```typescript
import { query, mutation } from "convex/server";
import { v } from "convex/values";

export const list = query({
  args: { channelId: v.id("channels") },
  handler: async (ctx, args) => {
    return await ctx.db
      .query("messages")
      .withIndex("by_channel", (q) => q.eq("channelId", args.channelId))
      .collect();
  },
});

export const send = mutation({
  args: { body: v.string(), channelId: v.id("channels") },
  handler: async (ctx, args) => {
    const identity = await ctx.auth.getUserIdentity();
    if (!identity) throw new Error("Not authenticated");
    await ctx.db.insert("messages", {
      body: args.body,
      author: identity.subject,
      channelId: args.channelId,
    });
  },
});
```

**Step 2: Write e2e test**

Create `tests/e2e_test.rs`:

```rust
use std::path::Path;

#[test]
fn test_e2e_sample_project() {
    let result = convex_doctor::engine::run(
        Path::new("tests/fixtures/sample_project"),
        false,
    )
    .unwrap();

    // Score should be reasonable but not perfect (missing return validators, etc.)
    assert!(result.score.value > 0);
    assert!(result.score.value <= 100);

    // Should have found some diagnostics
    println!("Score: {} ({})", result.score.value, result.score.label);
    println!("Diagnostics: {}", result.diagnostics.len());
    for d in &result.diagnostics {
        println!("  [{}] {} — {}", d.severity, d.rule, d.message);
    }
}

#[test]
fn test_e2e_json_output() {
    use convex_doctor::reporter::json::JsonReporter;
    use convex_doctor::reporter::Reporter;

    let result = convex_doctor::engine::run(
        Path::new("tests/fixtures/sample_project"),
        false,
    )
    .unwrap();

    let reporter = JsonReporter;
    let json_str = reporter.format(&result.diagnostics, &result.score, "sample_project", false);
    let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    assert!(json["score"]["value"].as_u64().unwrap() <= 100);
    assert!(json["diagnostics"].as_array().is_some());
}
```

**Step 3: Run e2e tests**

Run: `cargo test --test e2e_test`
Expected: All 2 tests pass

**Step 4: Run full test suite**

Run: `cargo test`
Expected: All tests pass

**Step 5: Commit**

```bash
git add tests/e2e_test.rs tests/fixtures/sample_project/ && git commit -m "test: end-to-end integration tests with sample Convex project"
```

---

## Task 15: GitHub Actions CI + Release Workflow

**Files:**
- Create: `.github/workflows/ci.yml`
- Create: `.github/workflows/release.yml`

**Step 1: Create CI workflow**

Create `.github/workflows/ci.yml`:

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo test --all-targets
      - run: cargo clippy -- -D warnings
      - run: cargo fmt --check

  build:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
          - os: macos-latest
            target: aarch64-apple-darwin
          - os: windows-latest
            target: x86_64-pc-windows-msvc
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - uses: Swatinem/rust-cache@v2
      - run: cargo build --release --target ${{ matrix.target }}
```

**Step 2: Create release workflow**

Create `.github/workflows/release.yml`:

```yaml
name: Release

on:
  push:
    tags: ["v*"]

permissions:
  contents: write

jobs:
  build:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            artifact: convex-doctor-x86_64-linux
          - os: ubuntu-latest
            target: aarch64-unknown-linux-gnu
            artifact: convex-doctor-aarch64-linux
          - os: macos-latest
            target: x86_64-apple-darwin
            artifact: convex-doctor-x86_64-darwin
          - os: macos-latest
            target: aarch64-apple-darwin
            artifact: convex-doctor-aarch64-darwin
          - os: windows-latest
            target: x86_64-pc-windows-msvc
            artifact: convex-doctor-x86_64-windows.exe
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - uses: Swatinem/rust-cache@v2
      - name: Install cross-compilation tools
        if: matrix.target == 'aarch64-unknown-linux-gnu'
        run: |
          sudo apt-get update
          sudo apt-get install -y gcc-aarch64-linux-gnu
      - run: cargo build --release --target ${{ matrix.target }}
      - name: Rename binary
        shell: bash
        run: |
          if [[ "${{ matrix.os }}" == "windows-latest" ]]; then
            cp target/${{ matrix.target }}/release/convex-doctor.exe ${{ matrix.artifact }}
          else
            cp target/${{ matrix.target }}/release/convex-doctor ${{ matrix.artifact }}
          fi
      - uses: softprops/action-gh-release@v2
        with:
          files: ${{ matrix.artifact }}
```

**Step 3: Commit**

```bash
git add .github/ && git commit -m "ci: GitHub Actions for CI testing and cross-platform release builds"
```

---

## Task 16: README and Final Polish

**Files:**
- Create: `README.md`

**Step 1: Create README**

Write a README with:
- Project description and badge
- Quick start (`curl -fsSL ... | sh` or `cargo install convex-doctor`)
- Usage examples (basic, verbose, json, score-only, diff)
- Rule categories with counts
- Config file example
- CI integration example
- Contributing section

**Step 2: Run `cargo clippy` and fix all warnings**

Run: `cargo clippy -- -D warnings`

**Step 3: Run `cargo fmt`**

Run: `cargo fmt`

**Step 4: Run full test suite one last time**

Run: `cargo test`
Expected: All tests pass

**Step 5: Commit**

```bash
git add README.md && git commit -m "docs: README with usage, rules, and CI integration examples"
```

---

## Summary

| Task | Description | Est. Files | Key Deliverable |
|------|-------------|-----------|-----------------|
| 1 | Project scaffold | 8 | Compiling Rust project |
| 2 | Core types | 2 | Diagnostic, Severity, Category |
| 3 | Scoring system | 2 | Score computation with weights |
| 4 | Configuration | 2 | TOML config loading |
| 5 | Project detection | 2 | Find convex dir, framework, files |
| 6 | Rule trait + registry | 8+ | Rule trait, registry, FileAnalysis |
| 7 | AST analyzer | 4 | Oxc visitor populating FileAnalysis |
| 8 | Security rules | 2 | 5 security rules |
| 9 | Perf + correctness rules | 4 | 4 perf + 4 correctness rules |
| 10 | Architecture rules | 2 | 2 architecture rules |
| 11 | Reporters | 4 | CLI, JSON, score-only output |
| 12 | Engine | 3 | Parallel orchestration, full CLI |
| 13 | Diff mode | 2 | Git diff file filtering |
| 14 | E2E tests | 3 | Integration tests with sample project |
| 15 | CI/CD | 2 | GitHub Actions workflows |
| 16 | README + polish | 1 | Documentation, clippy clean |
