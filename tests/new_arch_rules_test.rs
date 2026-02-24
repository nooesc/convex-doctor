use convex_doctor::rules::context::analyze_file;
use convex_doctor::rules::architecture::*;
use convex_doctor::rules::Rule;
use std::path::Path;

#[test]
fn test_duplicated_auth_detected() {
    let analysis = analyze_file(Path::new("tests/fixtures/perf_patterns.ts")).unwrap();
    let rule = DuplicatedAuth;
    let diagnostics = rule.check(&analysis);
    // perf_patterns.ts has 3 functions with auth checks
    assert!(
        !diagnostics.is_empty(),
        "Should detect 3+ functions with inline auth checks. Found {} functions with auth: {:?}",
        analysis.functions.iter().filter(|f| f.has_auth_check).count(),
        analysis.functions.iter().map(|f| format!("{}(auth={})", f.name, f.has_auth_check)).collect::<Vec<_>>()
    );
}

#[test]
fn test_duplicated_auth_not_flagged_for_few() {
    let analysis = analyze_file(Path::new("tests/fixtures/basic_query.ts")).unwrap();
    let rule = DuplicatedAuth;
    let diagnostics = rule.check(&analysis);
    // basic_query.ts has only 1 function with auth check
    assert!(
        diagnostics.is_empty(),
        "Should not flag when fewer than 3 functions have auth checks"
    );
}
