use convex_doctor::rules::context::analyze_file;
use convex_doctor::rules::performance::*;
use convex_doctor::rules::Rule;
use std::path::Path;

#[test]
fn test_unnecessary_run_action() {
    let analysis = analyze_file(Path::new("tests/fixtures/perf_patterns.ts")).unwrap();
    let rule = UnnecessaryRunAction;
    let diagnostics = rule.check(&analysis);
    assert!(
        !diagnostics.is_empty(),
        "Should detect ctx.runAction called from within an action"
    );
}

#[test]
fn test_sequential_run_calls() {
    let analysis = analyze_file(Path::new("tests/fixtures/perf_patterns.ts")).unwrap();
    let rule = SequentialRunCalls;
    let diagnostics = rule.check(&analysis);
    assert!(
        !diagnostics.is_empty(),
        "Should detect multiple sequential ctx.run* calls in action"
    );
}

#[test]
fn test_helper_vs_run_in_query() {
    let analysis = analyze_file(Path::new("tests/fixtures/perf_patterns.ts")).unwrap();
    let rule = HelperVsRun;
    let diagnostics = rule.check(&analysis);
    assert!(
        !diagnostics.is_empty(),
        "Should detect ctx.runQuery/ctx.runMutation inside query/mutation"
    );
}

#[test]
fn test_helper_vs_run_not_in_action() {
    let analysis = analyze_file(Path::new("tests/fixtures/bad_patterns.ts")).unwrap();
    let rule = HelperVsRun;
    let diagnostics = rule.check(&analysis);
    // bad_patterns.ts uses ctx.runQuery/ctx.runMutation in an action, not in a query/mutation
    assert!(
        diagnostics.is_empty(),
        "Should not flag ctx.run* calls in actions for this rule"
    );
}
