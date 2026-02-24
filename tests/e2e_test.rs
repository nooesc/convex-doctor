use std::path::Path;

#[test]
fn test_e2e_sample_project() {
    let result =
        convex_doctor::engine::run(Path::new("tests/fixtures/sample_project"), false, None).unwrap();

    // Score should be reasonable but not perfect (missing return validators, etc.)
    assert!(result.score.value > 0);
    assert!(result.score.value <= 100);

    // Should have found some diagnostics
    println!("Score: {} ({})", result.score.value, result.score.label);
    println!("Diagnostics: {}", result.diagnostics.len());
    for d in &result.diagnostics {
        println!("  [{}] {} — {}", d.severity, d.rule, d.message);
    }
}

#[test]
fn test_e2e_json_output() {
    use convex_doctor::reporter::json::JsonReporter;
    use convex_doctor::reporter::Reporter;

    let result =
        convex_doctor::engine::run(Path::new("tests/fixtures/sample_project"), false, None).unwrap();

    let reporter = JsonReporter;
    let json_str = reporter.format(&result.diagnostics, &result.score, "sample_project", false);
    let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    assert!(json["score"]["value"].as_u64().unwrap() <= 100);
    assert!(json["diagnostics"].as_array().is_some());
}
