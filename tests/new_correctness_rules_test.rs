use convex_doctor::rules::context::analyze_file;
use convex_doctor::rules::correctness::*;
use convex_doctor::rules::Rule;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn test_missing_unique_detected() {
    let analysis = analyze_file(Path::new("tests/fixtures/first_call_patterns.ts")).unwrap();
    let rule = MissingUnique;
    let diagnostics = rule.check(&analysis);
    assert!(
        !diagnostics.is_empty(),
        "Should detect .first() on indexed query chain"
    );
}

#[test]
fn test_missing_unique_not_flagged_without_first() {
    let analysis = analyze_file(Path::new("tests/fixtures/basic_query.ts")).unwrap();
    let rule = MissingUnique;
    let diagnostics = rule.check(&analysis);
    assert!(
        diagnostics.is_empty(),
        "Should not flag when .first() is not used"
    );
}

#[test]
fn test_wrong_runtime_import_detected_without_use_node() {
    let analysis = analyze_file(Path::new("tests/fixtures/runtime_import_issues.ts")).unwrap();
    let wrong_import = WrongRuntimeImport;
    assert!(
        !wrong_import.check(&analysis).is_empty(),
        "Should detect Node-only runtime imports without use node"
    );
}

#[test]
fn test_wrong_runtime_import_detected_with_use_node_browser_import() {
    let analysis = analyze_file(Path::new(
        "tests/fixtures/runtime_use_node_browser_import.ts",
    ))
    .unwrap();
    let wrong_import = WrongRuntimeImport;
    assert!(
        !wrong_import.check(&analysis).is_empty(),
        "Should detect browser runtime imports in use node file"
    );
}

#[test]
fn test_direct_function_ref_detected() {
    let analysis = analyze_file(Path::new("tests/fixtures/direct_function_ref.ts")).unwrap();
    let direct_ref = DirectFunctionRef;
    assert!(
        !direct_ref.check(&analysis).is_empty(),
        "Should detect direct function refs passed to ctx.runQuery"
    );
}

#[test]
fn test_missing_unique_not_flagged_for_non_indexed_first() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("non_indexed_first.ts");
    std::fs::write(
        &path,
        r#"
import { query } from "convex/server";
import { v } from "convex/values";

export const getByEmail = query({
  args: { email: v.string() },
  handler: async (ctx, args) => {
    return await ctx.db.query("users").filter((q) => q.eq(q.field("email"), args.email)).first();
  },
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = MissingUnique;
    let diagnostics = rule.check(&analysis);
    assert!(
        diagnostics.is_empty(),
        "Should only flag .first() when chain includes withIndex"
    );
}
