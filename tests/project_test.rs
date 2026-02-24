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
    assert!(files
        .iter()
        .all(|f| !f.to_string_lossy().contains("_generated")));
}
