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
