use convex_doctor::diagnostic::Severity;
use convex_doctor::rules::configuration::{MissingGeneratedCode, MissingTsconfig, OutdatedNodeVersion};
use convex_doctor::rules::{ProjectContext, Rule};

// --- MissingGeneratedCode ---

#[test]
fn test_missing_generated_code() {
    let ctx = ProjectContext {
        has_generated_dir: false,
        ..Default::default()
    };
    let diags = MissingGeneratedCode.check_project(&ctx);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "config/missing-generated-code");
    assert_eq!(diags[0].severity, Severity::Warning);
    assert_eq!(diags[0].message, "Missing convex/_generated/ directory");
    assert_eq!(diags[0].file, "convex/");
}

#[test]
fn test_missing_generated_code_ok() {
    let ctx = ProjectContext {
        has_generated_dir: true,
        ..Default::default()
    };
    let diags = MissingGeneratedCode.check_project(&ctx);
    assert!(diags.is_empty());
}

// --- OutdatedNodeVersion ---

#[test]
fn test_outdated_node_version_16() {
    let ctx = ProjectContext {
        node_version_from_config: Some("16".to_string()),
        ..Default::default()
    };
    let diags = OutdatedNodeVersion.check_project(&ctx);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "config/outdated-node-version");
    assert_eq!(diags[0].severity, Severity::Warning);
    assert!(diags[0].message.contains("Node 16"));
    assert_eq!(diags[0].file, "convex.json");
}

#[test]
fn test_outdated_node_version_18() {
    let ctx = ProjectContext {
        node_version_from_config: Some("18".to_string()),
        ..Default::default()
    };
    let diags = OutdatedNodeVersion.check_project(&ctx);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("Node 18"));
}

#[test]
fn test_outdated_node_version_20_ok() {
    let ctx = ProjectContext {
        node_version_from_config: Some("20".to_string()),
        ..Default::default()
    };
    let diags = OutdatedNodeVersion.check_project(&ctx);
    assert!(diags.is_empty());
}

#[test]
fn test_outdated_node_version_none_ok() {
    let ctx = ProjectContext {
        node_version_from_config: None,
        ..Default::default()
    };
    let diags = OutdatedNodeVersion.check_project(&ctx);
    assert!(diags.is_empty());
}

// --- MissingTsconfig ---

#[test]
fn test_missing_tsconfig_with_schema() {
    let ctx = ProjectContext {
        has_schema: true,
        has_tsconfig: false,
        ..Default::default()
    };
    let diags = MissingTsconfig.check_project(&ctx);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "config/missing-tsconfig");
    assert_eq!(diags[0].severity, Severity::Info);
    assert_eq!(diags[0].message, "No tsconfig.json found in convex/ directory");
    assert_eq!(diags[0].file, "convex/");
}

#[test]
fn test_missing_tsconfig_no_schema_ok() {
    let ctx = ProjectContext {
        has_schema: false,
        has_tsconfig: false,
        ..Default::default()
    };
    let diags = MissingTsconfig.check_project(&ctx);
    assert!(diags.is_empty());
}

#[test]
fn test_missing_tsconfig_has_tsconfig_ok() {
    let ctx = ProjectContext {
        has_schema: true,
        has_tsconfig: true,
        ..Default::default()
    };
    let diags = MissingTsconfig.check_project(&ctx);
    assert!(diags.is_empty());
}
