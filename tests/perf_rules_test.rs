use convex_doctor::rules::context::analyze_file;
use convex_doctor::rules::performance::*;
use convex_doctor::rules::Rule;
use std::path::Path;

#[test]
fn test_unbounded_collect() {
    let analysis = analyze_file(Path::new("tests/fixtures/bad_patterns.ts")).unwrap();
    let rule = UnboundedCollect;
    let diagnostics = rule.check(&analysis);
    assert!(!diagnostics.is_empty(), "Should detect unbounded .collect()");
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
    assert!(!diagnostics.is_empty(), "Should detect Date.now() in query file");
}

#[test]
fn test_loop_run_mutation() {
    let analysis = analyze_file(Path::new("tests/fixtures/bad_patterns.ts")).unwrap();
    let rule = LoopRunMutation;
    let diagnostics = rule.check(&analysis);
    assert!(!diagnostics.is_empty(), "Should detect ctx.runMutation in loop");
}
