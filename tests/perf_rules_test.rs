use convex_doctor::rules::context::analyze_file;
use convex_doctor::rules::performance::*;
use convex_doctor::rules::Rule;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn test_unbounded_collect() {
    let analysis = analyze_file(Path::new("tests/fixtures/bad_patterns.ts")).unwrap();
    let rule = UnboundedCollect;
    let diagnostics = rule.check(&analysis);
    assert!(
        !diagnostics.is_empty(),
        "Should detect unbounded .collect()"
    );
}

#[test]
fn test_filter_without_index() {
    let analysis = analyze_file(Path::new("tests/fixtures/bad_patterns.ts")).unwrap();
    let rule = FilterWithoutIndex;
    let diagnostics = rule.check(&analysis);
    assert!(!diagnostics.is_empty(), "Should detect .filter() calls");
}

#[test]
fn test_date_now_in_query() {
    let analysis = analyze_file(Path::new("tests/fixtures/bad_patterns.ts")).unwrap();
    let rule = DateNowInQuery;
    let diagnostics = rule.check(&analysis);
    assert!(
        !diagnostics.is_empty(),
        "Should detect Date.now() in query file"
    );
}

#[test]
fn test_loop_run_mutation() {
    let analysis = analyze_file(Path::new("tests/fixtures/bad_patterns.ts")).unwrap();
    let rule = LoopRunMutation;
    let diagnostics = rule.check(&analysis);
    assert!(
        !diagnostics.is_empty(),
        "Should detect ctx.runMutation in loop"
    );
}

#[test]
fn test_unbounded_collect_not_flagged_when_take_is_used() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("bounded_collect.ts");
    std::fs::write(
        &path,
        r#"
import { query } from "convex/server";

export const list = query({
  args: {},
  handler: async (ctx) => {
    return await ctx.db.query("items").take(20).collect();
  },
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = UnboundedCollect;
    let diagnostics = rule.check(&analysis);
    assert!(
        diagnostics.is_empty(),
        "collect() with take(n) should not be flagged as unbounded"
    );
}
