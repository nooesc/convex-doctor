use convex_doctor::diagnostic::{Category, Diagnostic, Severity};
use convex_doctor::reporter::json::JsonReporter;
use convex_doctor::reporter::Reporter;
use convex_doctor::scoring::compute_score;

fn sample_diagnostics() -> Vec<Diagnostic> {
    vec![
        Diagnostic {
            rule: "security/missing-auth-check".to_string(),
            severity: Severity::Error,
            category: Category::Security,
            message: "Public query does not check auth".to_string(),
            help: "Add auth check".to_string(),
            file: "convex/messages.ts".to_string(),
            line: 5,
            column: 1,
        },
        Diagnostic {
            rule: "perf/unbounded-collect".to_string(),
            severity: Severity::Error,
            category: Category::Performance,
            message: "Unbounded collect".to_string(),
            help: "Use take".to_string(),
            file: "convex/messages.ts".to_string(),
            line: 22,
            column: 10,
        },
    ]
}

#[test]
fn test_json_output_structure() {
    let diagnostics = sample_diagnostics();
    let score = compute_score(&diagnostics);
    let reporter = JsonReporter;
    let output = reporter.format(&diagnostics, &score, "my-app", false);
    let json: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert!(json["score"]["value"].is_number());
    assert!(json["diagnostics"].is_array());
    assert_eq!(json["diagnostics"].as_array().unwrap().len(), 2);
}

#[test]
fn test_score_only_output() {
    let diagnostics = sample_diagnostics();
    let score = compute_score(&diagnostics);
    let output = convex_doctor::reporter::score_only(&score);
    let parsed: u32 = output.trim().parse().unwrap();
    assert!(parsed <= 100);
}
