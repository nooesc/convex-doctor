use convex_doctor::diagnostic::{Category, Diagnostic, Severity};

#[test]
fn test_diagnostic_creation() {
    let d = Diagnostic {
        rule: "security/missing-arg-validators".to_string(),
        severity: Severity::Error,
        category: Category::Security,
        message: "Public mutation without argument validators".to_string(),
        help: "Add `args: { ... }` with validators".to_string(),
        file: "convex/messages.ts".to_string(),
        line: 14,
        column: 1,
    };
    assert_eq!(d.rule, "security/missing-arg-validators");
    assert_eq!(d.severity, Severity::Error);
    assert_eq!(d.category, Category::Security);
}

#[test]
fn test_severity_display() {
    assert_eq!(format!("{}", Severity::Error), "error");
    assert_eq!(format!("{}", Severity::Warning), "warning");
}

#[test]
fn test_category_weight() {
    assert_eq!(Category::Security.weight(), 1.5);
    assert_eq!(Category::Performance.weight(), 1.2);
    assert_eq!(Category::Correctness.weight(), 1.5);
    assert_eq!(Category::Schema.weight(), 1.0);
    assert_eq!(Category::Architecture.weight(), 0.8);
    assert_eq!(Category::Configuration.weight(), 1.0);
}

#[test]
fn test_category_display() {
    assert_eq!(format!("{}", Category::Security), "Security");
    assert_eq!(format!("{}", Category::Performance), "Performance");
}

#[test]
fn test_diagnostic_serialization() {
    let d = Diagnostic {
        rule: "perf/unbounded-collect".to_string(),
        severity: Severity::Error,
        category: Category::Performance,
        message: "Unbounded .collect()".to_string(),
        help: "Use .take(n) or pagination".to_string(),
        file: "convex/messages.ts".to_string(),
        line: 22,
        column: 10,
    };
    let json = serde_json::to_string(&d).unwrap();
    assert!(json.contains("\"rule\":\"perf/unbounded-collect\""));
    assert!(json.contains("\"severity\":\"error\""));
}
