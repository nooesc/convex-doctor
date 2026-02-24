use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

fn init_git_repo(dir: &Path) {
    let run = |args: &[&str]| {
        let status = Command::new("git")
            .args(args)
            .current_dir(dir)
            .status()
            .expect("git command should run");
        assert!(status.success(), "git {:?} should succeed", args);
    };

    run(&["init"]);
    run(&["config", "user.email", "tests@example.com"]);
    run(&["config", "user.name", "Tests"]);
    run(&["add", "."]);
    run(&["commit", "-m", "init"]);
}

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

    let result = convex_doctor::engine::run(dir.path(), false, None).unwrap();
    assert!(result.score.value < 100);
    assert!(!result.diagnostics.is_empty());
    assert_eq!(result.files_scanned, 1);
}

#[test]
fn test_engine_no_convex_dir() {
    let dir = TempDir::new().unwrap();
    let result = convex_doctor::engine::run(dir.path(), false, None);
    assert!(result.is_err());
}

#[test]
fn test_get_changed_files_no_git() {
    let dir = TempDir::new().unwrap();
    let files = convex_doctor::engine::get_changed_files(dir.path(), "main");
    assert!(files.is_err());
}

#[test]
fn test_engine_diff_no_changes_scans_no_files() {
    let dir = TempDir::new().unwrap();
    let convex_dir = dir.path().join("convex");
    std::fs::create_dir(&convex_dir).unwrap();
    std::fs::write(
        convex_dir.join("messages.ts"),
        r#"
import { query } from "convex/server";
import { v } from "convex/values";

export const getMessages = query({
  args: {},
  returns: v.null(),
  handler: async (ctx) => {
    return null;
  },
});
"#,
    )
    .unwrap();
    init_git_repo(dir.path());

    let result = convex_doctor::engine::run(dir.path(), false, Some("HEAD")).unwrap();
    assert_eq!(
        result.files_scanned, 0,
        "No changed files should be scanned"
    );
    assert!(
        result.diagnostics.is_empty(),
        "No diagnostics expected when no files changed"
    );
}

#[test]
fn test_engine_env_local_gitignore_wildcard_is_respected() {
    let dir = TempDir::new().unwrap();
    let convex_dir = dir.path().join("convex");
    std::fs::create_dir(&convex_dir).unwrap();
    std::fs::write(
        convex_dir.join("messages.ts"),
        r#"
import { query } from "convex/server";
import { v } from "convex/values";

export const getMessages = query({
  args: {},
  returns: v.null(),
  handler: async (ctx) => {
    return null;
  },
});
"#,
    )
    .unwrap();
    std::fs::write(dir.path().join(".gitignore"), ".env*\n").unwrap();
    std::fs::write(dir.path().join(".env.local"), "SECRET=1\n").unwrap();

    let result = convex_doctor::engine::run(dir.path(), false, None).unwrap();
    assert!(
        !result
            .diagnostics
            .iter()
            .any(|d| d.rule == "security/env-not-gitignored"),
        "Wildcard .gitignore pattern should suppress env-not-gitignored diagnostic"
    );
}

#[test]
fn test_engine_reports_parse_errors_as_diagnostics() {
    let dir = TempDir::new().unwrap();
    let convex_dir = dir.path().join("convex");
    std::fs::create_dir(&convex_dir).unwrap();
    std::fs::write(
        convex_dir.join("broken.ts"),
        r#"
import { query } from "convex/server";

export const bad = query({
  handler: async (ctx) => {
    return ctx.db.query("items").collect(
  },
});
"#,
    )
    .unwrap();

    let result = convex_doctor::engine::run(dir.path(), false, None).unwrap();
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.rule == "correctness/file-parse-error"),
        "Parse failures should surface in diagnostics output"
    );
}

#[test]
fn test_engine_diff_includes_untracked_files() {
    let dir = TempDir::new().unwrap();
    let convex_dir = dir.path().join("convex");
    std::fs::create_dir(&convex_dir).unwrap();
    std::fs::write(
        convex_dir.join("messages.ts"),
        r#"
import { query } from "convex/server";

export const getMessages = query({
  args: {},
  returns: 42,
  handler: async () => {
    return 1;
  },
});
"#,
    )
    .unwrap();
    init_git_repo(dir.path());

    std::fs::write(
        convex_dir.join("new-message.ts"),
        "export const newMessage = 1;",
    )
    .unwrap();

    let result = convex_doctor::engine::run(dir.path(), false, Some("HEAD")).unwrap();
    assert_eq!(result.files_scanned, 1);
}

#[test]
fn test_engine_diff_includes_deleted_files_in_git_change_set() {
    let dir = TempDir::new().unwrap();
    let convex_dir = dir.path().join("convex");
    std::fs::create_dir(&convex_dir).unwrap();
    let tracked = convex_dir.join("messages.ts");
    std::fs::write(
        &tracked,
        r#"
export const deletedMessage = 1;
"#,
    )
    .unwrap();
    init_git_repo(dir.path());
    std::fs::remove_file(tracked).unwrap();

    let result = convex_doctor::engine::run(dir.path(), false, Some("HEAD")).unwrap();
    assert_eq!(result.files_scanned, 0);
}
