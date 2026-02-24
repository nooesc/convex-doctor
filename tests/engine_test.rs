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
    assert!(result.score.value < 100);
    assert!(!result.diagnostics.is_empty());
    assert_eq!(result.files_scanned, 1);
}

#[test]
fn test_engine_no_convex_dir() {
    let dir = TempDir::new().unwrap();
    let result = convex_doctor::engine::run(dir.path(), false);
    assert!(result.is_err());
}

#[test]
fn test_get_changed_files_no_git() {
    let dir = TempDir::new().unwrap();
    let files = convex_doctor::engine::get_changed_files(dir.path(), "main");
    assert!(files.is_empty());
}
