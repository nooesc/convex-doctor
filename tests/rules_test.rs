use convex_doctor::diagnostic::Category;
use convex_doctor::rules::{ConvexFunction, FunctionKind, RuleRegistry};

#[test]
fn test_registry_has_all_categories() {
    let registry = RuleRegistry::new();
    let categories: Vec<Category> = registry.rules().iter().map(|r| r.category()).collect();
    assert!(categories.contains(&Category::Security));
    assert!(categories.contains(&Category::Performance));
    assert!(categories.contains(&Category::Correctness));
    assert!(categories.contains(&Category::Architecture));
}

#[test]
fn test_registry_rule_count() {
    let registry = RuleRegistry::new();
    assert!(registry.rules().len() >= 10);
}

#[test]
fn test_convex_function_is_public() {
    let public_fn = ConvexFunction {
        name: "getMessages".to_string(),
        kind: FunctionKind::Query,
        has_args_validator: true,
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
