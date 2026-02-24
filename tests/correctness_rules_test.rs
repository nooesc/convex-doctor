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
    assert!(!diagnostics.is_empty(), "Should detect unwaited ctx calls");
}

#[test]
fn test_deprecated_api() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("deprecated.ts");
    std::fs::write(
        &path,
        r#"
import { mutation } from "convex/server";
import { v } from "convex/values";

export const create = mutation({
  args: { count: v.bigint() },
  handler: async (ctx, args) => {},
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = DeprecatedApi;
    let diagnostics = rule.check(&analysis);
    assert!(
        diagnostics.iter().any(|d| d.message.contains("v.bigint()")),
        "Should detect deprecated v.bigint()"
    );
}
