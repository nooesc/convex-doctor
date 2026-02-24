use convex_doctor::rules::context::analyze_file;
use convex_doctor::rules::security::*;
use convex_doctor::rules::Rule;
use std::path::Path;

#[test]
fn test_missing_arg_validators() {
    let analysis = analyze_file(Path::new("tests/fixtures/bad_patterns.ts")).unwrap();
    let rule = MissingArgValidators;
    let diagnostics = rule.check(&analysis);
    assert!(
        diagnostics.iter().any(|d| d.message.contains("listAll")),
        "listAll has no args"
    );
}

#[test]
fn test_no_false_positive_arg_validators() {
    let analysis = analyze_file(Path::new("tests/fixtures/basic_query.ts")).unwrap();
    let rule = MissingArgValidators;
    let diagnostics = rule.check(&analysis);
    assert!(
        !diagnostics
            .iter()
            .any(|d| d.message.contains("getMessages")),
        "getMessages has args"
    );
}

#[test]
fn test_missing_return_validators() {
    let analysis = analyze_file(Path::new("tests/fixtures/bad_patterns.ts")).unwrap();
    let rule = MissingReturnValidators;
    let diagnostics = rule.check(&analysis);
    assert!(
        !diagnostics.is_empty(),
        "Should flag functions without return validators"
    );
}

#[test]
fn test_missing_auth_check() {
    let analysis = analyze_file(Path::new("tests/fixtures/bad_patterns.ts")).unwrap();
    let rule = MissingAuthCheck;
    let diagnostics = rule.check(&analysis);
    assert!(
        !diagnostics.is_empty(),
        "Public functions without auth check should be flagged"
    );
}

#[test]
fn test_auth_check_not_flagged_when_present() {
    let analysis = analyze_file(Path::new("tests/fixtures/basic_query.ts")).unwrap();
    let rule = MissingAuthCheck;
    let diagnostics = rule.check(&analysis);
    // getMessages has auth check, should NOT be in diagnostics
    assert!(
        !diagnostics
            .iter()
            .any(|d| d.message.contains("getMessages")),
        "getMessages has auth"
    );
}

#[test]
fn test_hardcoded_secrets() {
    let analysis = analyze_file(Path::new("tests/fixtures/secrets_test.ts")).unwrap();
    assert!(
        !analysis.hardcoded_secrets.is_empty(),
        "Should detect hardcoded secret"
    );
}

#[test]
fn test_spoofable_access_control_detected() {
    let analysis = analyze_file(Path::new("tests/fixtures/spoofable_access.ts")).unwrap();
    let rule = SpoofableAccessControl;
    let diagnostics = rule.check(&analysis);
    assert!(
        !diagnostics.is_empty(),
        "Should detect suspicious auth args without ctx.auth checks"
    );
}
