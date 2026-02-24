use convex_doctor::rules::architecture::*;
use convex_doctor::rules::{ConvexFunction, FileAnalysis, FunctionKind, Rule};

#[test]
fn test_large_handler_flagged() {
    let analysis = FileAnalysis {
        file_path: "convex/test.ts".to_string(),
        functions: vec![ConvexFunction {
            name: "bigFunction".to_string(),
            kind: FunctionKind::Mutation,
            has_args_validator: true,
            arg_names: vec![],
            has_return_validator: false,
            has_auth_check: false,
            handler_line_count: 80,
            span_line: 1,
            span_col: 1,
        }],
        ..Default::default()
    };
    let rule = LargeHandler;
    let diagnostics = rule.check(&analysis);
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("80 lines"));
}

#[test]
fn test_small_handler_not_flagged() {
    let analysis = FileAnalysis {
        file_path: "convex/test.ts".to_string(),
        functions: vec![ConvexFunction {
            name: "smallFunction".to_string(),
            kind: FunctionKind::Query,
            has_args_validator: true,
            arg_names: vec![],
            has_return_validator: true,
            has_auth_check: true,
            handler_line_count: 20,
            span_line: 1,
            span_col: 1,
        }],
        ..Default::default()
    };
    let rule = LargeHandler;
    let diagnostics = rule.check(&analysis);
    assert!(diagnostics.is_empty());
}

#[test]
fn test_monolithic_file() {
    let analysis = FileAnalysis {
        file_path: "convex/everything.ts".to_string(),
        exported_function_count: 12,
        ..Default::default()
    };
    let rule = MonolithicFile;
    let diagnostics = rule.check(&analysis);
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("12"));
}

#[test]
fn test_non_monolithic_file() {
    let analysis = FileAnalysis {
        file_path: "convex/small.ts".to_string(),
        exported_function_count: 5,
        ..Default::default()
    };
    let rule = MonolithicFile;
    let diagnostics = rule.check(&analysis);
    assert!(diagnostics.is_empty());
}
