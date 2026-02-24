use convex_doctor::rules::context::analyze_file;
use convex_doctor::rules::schema::*;
use convex_doctor::rules::Rule;
use std::path::Path;

#[test]
fn test_deep_nesting_detected() {
    let analysis = analyze_file(Path::new("tests/fixtures/schema_patterns.ts")).unwrap();
    let rule = DeepNesting;
    let diagnostics = rule.check(&analysis);
    assert!(
        !diagnostics.is_empty(),
        "Should detect deeply nested validators (depth {})",
        analysis.schema_nesting_depth
    );
}

#[test]
fn test_deep_nesting_not_flagged_for_shallow() {
    let analysis = analyze_file(Path::new("tests/fixtures/basic_query.ts")).unwrap();
    let rule = DeepNesting;
    let diagnostics = rule.check(&analysis);
    // basic_query.ts has v.array(v.object(...)) which is depth 2, below threshold
    assert!(
        diagnostics.is_empty(),
        "Should not flag nesting depth {} (threshold is 3)",
        analysis.schema_nesting_depth
    );
}

#[test]
fn test_array_relationships_detected() {
    let analysis = analyze_file(Path::new("tests/fixtures/schema_patterns.ts")).unwrap();
    let rule = ArrayRelationships;
    let diagnostics = rule.check(&analysis);
    assert!(
        !diagnostics.is_empty(),
        "Should detect v.array(v.id(...)) pattern"
    );
}

#[test]
fn test_redundant_index_detected() {
    let analysis = analyze_file(Path::new("tests/fixtures/schema_patterns.ts")).unwrap();
    let rule = RedundantIndex;
    let diagnostics = rule.check(&analysis);
    assert!(
        !diagnostics.is_empty(),
        "Should detect that by_author is a prefix of by_author_title. Found {} index definitions: {:?}",
        analysis.index_definitions.len(),
        analysis.index_definitions.iter().map(|i| format!("{}({:?})", i.name, i.fields)).collect::<Vec<_>>()
    );
}
