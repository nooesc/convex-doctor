use convex_doctor::diagnostic::Category;
use convex_doctor::rules::{ConvexFunction, FunctionKind, ProjectContext, RuleRegistry};

#[test]
fn test_registry_has_all_categories() {
    let registry = RuleRegistry::new();
    let categories: Vec<Category> = registry.rules().iter().map(|r| r.category()).collect();
    assert!(categories.contains(&Category::Security));
    assert!(categories.contains(&Category::Performance));
    assert!(categories.contains(&Category::Correctness));
    assert!(categories.contains(&Category::Architecture));
    assert!(categories.contains(&Category::Schema));
    assert!(categories.contains(&Category::Configuration));
    assert!(categories.contains(&Category::ClientSide));
}

#[test]
fn test_registry_rule_count() {
    let registry = RuleRegistry::new();
    assert_eq!(registry.rules().len(), 65);
}

#[test]
fn test_registry_unique_rule_ids() {
    let registry = RuleRegistry::new();
    let mut ids: Vec<&str> = registry.rules().iter().map(|r| r.id()).collect();
    let original_len = ids.len();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), original_len, "All rule IDs must be unique");
}

#[test]
fn test_project_level_missing_schema() {
    let registry = RuleRegistry::new();
    let ctx = ProjectContext {
        has_schema: false,
        ..Default::default()
    };
    let diagnostics: Vec<_> = registry
        .rules()
        .iter()
        .flat_map(|r| r.check_project(&ctx))
        .collect();
    assert!(
        diagnostics
            .iter()
            .any(|d| d.rule == "schema/missing-schema"),
        "Should flag missing schema"
    );
}

#[test]
fn test_project_level_no_missing_schema_when_present() {
    let registry = RuleRegistry::new();
    let ctx = ProjectContext {
        has_schema: true,
        ..Default::default()
    };
    let diagnostics: Vec<_> = registry
        .rules()
        .iter()
        .flat_map(|r| r.check_project(&ctx))
        .collect();
    assert!(
        !diagnostics
            .iter()
            .any(|d| d.rule == "schema/missing-schema"),
        "Should not flag when schema exists"
    );
}

#[test]
fn test_project_level_missing_auth_config() {
    let registry = RuleRegistry::new();
    let ctx = ProjectContext {
        uses_auth: true,
        has_auth_config: false,
        ..Default::default()
    };
    let diagnostics: Vec<_> = registry
        .rules()
        .iter()
        .flat_map(|r| r.check_project(&ctx))
        .collect();
    assert!(
        diagnostics
            .iter()
            .any(|d| d.rule == "config/missing-auth-config"),
        "Should flag missing auth config when auth is used"
    );
}

#[test]
fn test_project_level_env_not_gitignored() {
    let registry = RuleRegistry::new();
    let ctx = ProjectContext {
        has_env_local: true,
        env_gitignored: false,
        ..Default::default()
    };
    let diagnostics: Vec<_> = registry
        .rules()
        .iter()
        .flat_map(|r| r.check_project(&ctx))
        .collect();
    assert!(
        diagnostics
            .iter()
            .any(|d| d.rule == "security/env-not-gitignored"),
        "Should flag .env.local not in .gitignore"
    );
}

#[test]
fn test_convex_function_is_public() {
    let public_fn = ConvexFunction {
        name: "getMessages".to_string(),
        kind: FunctionKind::Query,
        has_args_validator: true,
        has_any_validator_in_args: false,
        arg_names: vec![],
        has_return_validator: false,
        has_auth_check: false,
        handler_line_count: 10,
        span_line: 5,
        span_col: 1,
    };
    assert!(public_fn.is_public());

    let internal_fn = ConvexFunction {
        name: "sendEmail".to_string(),
        kind: FunctionKind::InternalAction,
        has_args_validator: true,
        has_any_validator_in_args: false,
        arg_names: vec![],
        has_return_validator: false,
        has_auth_check: false,
        handler_line_count: 10,
        span_line: 5,
        span_col: 1,
    };
    assert!(!internal_fn.is_public());
}

#[test]
fn test_function_kind_from_callee() {
    assert_eq!(
        FunctionKind::from_callee("query"),
        Some(FunctionKind::Query)
    );
    assert_eq!(
        FunctionKind::from_callee("mutation"),
        Some(FunctionKind::Mutation)
    );
    assert_eq!(
        FunctionKind::from_callee("internalAction"),
        Some(FunctionKind::InternalAction)
    );
    assert_eq!(FunctionKind::from_callee("unknown"), None);
}
