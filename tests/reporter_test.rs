use std::time::Duration;

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
    let output = reporter.format(
        &diagnostics,
        &score,
        "my-app",
        false,
        5,
        Duration::from_millis(42),
    );
    let json: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert!(json["score"]["value"].is_number());
    assert!(json["diagnostics"].is_array());
    assert_eq!(json["diagnostics"].as_array().unwrap().len(), 2);
    assert_eq!(json["summary"]["files_scanned"].as_u64().unwrap(), 5);
}

#[test]
fn test_json_summary_counts_errors_warnings_and_infos() {
    let diagnostics = vec![
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
            severity: Severity::Warning,
            category: Category::Performance,
            message: "Unbounded collect".to_string(),
            help: "Use take".to_string(),
            file: "convex/messages.ts".to_string(),
            line: 22,
            column: 10,
        },
        Diagnostic {
            rule: "config/missing-tsconfig".to_string(),
            severity: Severity::Info,
            category: Category::Configuration,
            message: "No tsconfig".to_string(),
            help: "Create tsconfig".to_string(),
            file: "convex/".to_string(),
            line: 0,
            column: 0,
        },
    ];
    let score = compute_score(&diagnostics);
    let reporter = JsonReporter;
    let output = reporter.format(
        &diagnostics,
        &score,
        "my-app",
        false,
        3,
        Duration::from_millis(10),
    );
    let json: serde_json::Value = serde_json::from_str(&output).unwrap();

    assert_eq!(json["summary"]["errors"].as_u64().unwrap(), 1);
    assert_eq!(json["summary"]["warnings"].as_u64().unwrap(), 1);
    assert_eq!(json["summary"]["infos"].as_u64().unwrap(), 1);
}

#[test]
fn test_score_only_output() {
    let diagnostics = sample_diagnostics();
    let score = compute_score(&diagnostics);
    let output = convex_doctor::reporter::score_only(&score);
    let parsed: u32 = output.trim().parse().unwrap();
    assert!(parsed <= 100);
}
