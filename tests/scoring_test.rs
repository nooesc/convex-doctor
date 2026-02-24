use convex_doctor::diagnostic::{Category, Diagnostic, Severity};
use convex_doctor::scoring::compute_score;

fn make_diagnostic(rule: &str, severity: Severity, category: Category) -> Diagnostic {
    Diagnostic {
        rule: rule.to_string(),
        severity,
        category,
        message: "test".to_string(),
        help: "test".to_string(),
        file: "convex/test.ts".to_string(),
        line: 1,
        column: 1,
    }
}

#[test]
fn test_perfect_score() {
    let result = compute_score(&[]);
    assert_eq!(result.value, 100);
    assert_eq!(result.label, "Healthy");
}

#[test]
fn test_single_error_deduction() {
    let diagnostics = vec![make_diagnostic(
        "perf/unbounded-collect",
        Severity::Error,
        Category::Performance,
    )];
    let result = compute_score(&diagnostics);
    // error = -3, performance weight = 1.2, deduction = 3.6, score = 96
    assert_eq!(result.value, 96);
    assert_eq!(result.label, "Healthy");
}

#[test]
fn test_single_warning_deduction() {
    let diagnostics = vec![make_diagnostic(
        "arch/large-handler",
        Severity::Warning,
        Category::Architecture,
    )];
    let result = compute_score(&diagnostics);
    // warning = -1, architecture weight = 0.8, deduction = 0.8, score = 99
    assert_eq!(result.value, 99);
}

#[test]
fn test_security_error_weighted_higher() {
    let diagnostics = vec![make_diagnostic(
        "security/missing-auth-check",
        Severity::Error,
        Category::Security,
    )];
    let result = compute_score(&diagnostics);
    // error = -3, security weight = 1.5, deduction = 4.5, score = 95.5 (rounds to 96)
    assert_eq!(result.value, 96);
}

#[test]
fn test_per_rule_cap_errors() {
    let diagnostics: Vec<_> = (0..6)
        .map(|_| {
            make_diagnostic(
                "perf/unbounded-collect",
                Severity::Error,
                Category::Performance,
            )
        })
        .collect();
    let result = compute_score(&diagnostics);
    // 6 * 3 * 1.2 = 21.6, capped at 15 * 1.2 = 18, score = 82
    assert_eq!(result.value, 82);
}

#[test]
fn test_per_rule_cap_warnings() {
    let diagnostics: Vec<_> = (0..6)
        .map(|_| {
            make_diagnostic(
                "arch/large-handler",
                Severity::Warning,
                Category::Architecture,
            )
        })
        .collect();
    let result = compute_score(&diagnostics);
    // 6 * 1 * 0.8 = 4.8, capped at 5 * 0.8 = 4.0, score = 96
    assert_eq!(result.value, 96);
}

#[test]
fn test_score_floor_at_zero() {
    let diagnostics: Vec<_> = (0..50)
        .map(|i| {
            make_diagnostic(
                &format!("security/rule-{i}"),
                Severity::Error,
                Category::Security,
            )
        })
        .collect();
    let result = compute_score(&diagnostics);
    assert_eq!(result.value, 0);
    assert_eq!(result.label, "Critical");
}

#[test]
fn test_multiple_categories() {
    let diagnostics = vec![
        make_diagnostic(
            "security/missing-auth-check",
            Severity::Error,
            Category::Security,
        ),
        make_diagnostic(
            "perf/unbounded-collect",
            Severity::Error,
            Category::Performance,
        ),
        make_diagnostic(
            "arch/large-handler",
            Severity::Warning,
            Category::Architecture,
        ),
    ];
    let result = compute_score(&diagnostics);
    // security: -3 * 1.5 = -4.5
    // perf: -3 * 1.2 = -3.6
    // arch: -1 * 0.8 = -0.8
    // total deduction = 8.9, score = 91
    assert_eq!(result.value, 91);
}
