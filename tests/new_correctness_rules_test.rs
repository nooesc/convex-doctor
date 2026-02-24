use convex_doctor::rules::context::analyze_file;
use convex_doctor::rules::correctness::*;
use convex_doctor::rules::Rule;
use std::path::Path;

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
fn test_stub_rules_return_empty() {
    let analysis = analyze_file(Path::new("tests/fixtures/basic_query.ts")).unwrap();

    let wrong_import = WrongRuntimeImport;
    assert!(
        wrong_import.check(&analysis).is_empty(),
        "Stub rule should return empty"
    );

    let direct_ref = DirectFunctionRef;
    assert!(
        direct_ref.check(&analysis).is_empty(),
        "Stub rule should return empty"
    );
}
