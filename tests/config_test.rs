use convex_doctor::config::Config;
use convex_doctor::diagnostic::{Category, Diagnostic, Severity};
use std::path::Path;
use tempfile::TempDir;

#[test]
fn test_default_config() {
    let config = Config::default();
    assert_eq!(config.ci.fail_below, 0);
    assert!(config.rules.is_empty());
    assert_eq!(config.ignore.files, vec!["convex/_generated/**"]);
    assert_eq!(config.convex.guidance_version, "v0.241.0");
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
    assert!(config.is_file_ignored(Path::new("."), Path::new("convex/_generated/api.d.ts")));
    assert!(!config.is_file_ignored(Path::new("."), Path::new("convex/messages.ts")));
}

#[test]
fn test_is_file_ignored_with_absolute_path() {
    let config = Config::default();
    let root = Path::new("/repo");
    let file = Path::new("/repo/convex/_generated/api.d.ts");
    assert!(config.is_file_ignored(root, file));
}

#[test]
fn test_ignore_filename_wildcard_matches_subdirectories() {
    let mut config = Config::default();
    config.ignore.files = vec!["*.ts".to_string()];
    assert!(config.is_file_ignored(Path::new("."), Path::new("convex/messages.ts")));
    assert!(config.is_file_ignored(Path::new("."), Path::new("convex/helpers/inner.ts")));
}

#[test]
fn test_ignore_directory_pattern_without_wildcard() {
    let mut config = Config::default();
    config.ignore.files = vec!["convex/helpers".to_string()];
    assert!(config.is_file_ignored(Path::new("."), Path::new("convex/helpers/config.ts")));
}

#[test]
fn test_ignore_leading_slash_pattern_matches_root() {
    let mut config = Config::default();
    config.ignore.files = vec!["/convex/messages.ts".to_string()];
    assert!(config.is_file_ignored(Path::new("."), Path::new("convex/messages.ts")));
}

#[test]
fn test_load_convex_strictness_config() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("convex-doctor.toml");
    std::fs::write(
        &config_path,
        r#"
[convex]
guidance_version = "v0.241.0"
strictness = "strict"
"#,
    )
    .unwrap();

    let config = Config::load(dir.path()).unwrap();
    assert_eq!(config.convex.guidance_version, "v0.241.0");
    assert_eq!(format!("{:?}", config.convex.strictness), "Strict");
}

#[test]
fn test_apply_strictness_promotes_info_to_warning() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("convex-doctor.toml");
    std::fs::write(
        &config_path,
        r#"
[convex]
strictness = "strict"
"#,
    )
    .unwrap();

    let config = Config::load(dir.path()).unwrap();
    let mut diagnostics = vec![Diagnostic {
        rule: "arch/no-convex-error".to_string(),
        severity: Severity::Info,
        category: Category::Architecture,
        message: "msg".to_string(),
        help: "help".to_string(),
        file: "convex/messages.ts".to_string(),
        line: 1,
        column: 1,
    }];
    config.apply_strictness(&mut diagnostics);
    assert_eq!(diagnostics[0].severity, Severity::Warning);
}

#[test]
fn test_apply_low_noise_removes_info_and_noisy_warnings() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("convex-doctor.toml");
    std::fs::write(
        &config_path,
        r#"
[convex]
strictness = "low_noise"
"#,
    )
    .unwrap();

    let config = Config::load(dir.path()).unwrap();
    let mut diagnostics = vec![
        Diagnostic {
            rule: "correctness/replace-vs-patch".to_string(),
            severity: Severity::Warning,
            category: Category::Correctness,
            message: "msg".to_string(),
            help: "help".to_string(),
            file: "convex/messages.ts".to_string(),
            line: 1,
            column: 1,
        },
        Diagnostic {
            rule: "security/missing-arg-validators".to_string(),
            severity: Severity::Error,
            category: Category::Security,
            message: "msg".to_string(),
            help: "help".to_string(),
            file: "convex/messages.ts".to_string(),
            line: 1,
            column: 1,
        },
        Diagnostic {
            rule: "arch/no-convex-error".to_string(),
            severity: Severity::Info,
            category: Category::Architecture,
            message: "msg".to_string(),
            help: "help".to_string(),
            file: "convex/messages.ts".to_string(),
            line: 1,
            column: 1,
        },
    ];
    config.apply_strictness(&mut diagnostics);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule, "security/missing-arg-validators");
}
