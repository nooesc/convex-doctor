use convex_doctor::diagnostic::Severity;
use convex_doctor::rules::context::analyze_file;
use convex_doctor::rules::performance::*;
use convex_doctor::rules::{
    CallLocation, ConvexFunction, FileAnalysis, FunctionKind, IndexDef, ProjectContext, Rule,
    SchemaIdField,
};
use std::path::Path;

// ---------------------------------------------------------------------------
// 1. MissingIndexOnForeignKey (project-level)
// ---------------------------------------------------------------------------

#[test]
fn test_missing_index_on_foreign_key_fires() {
    let rule = MissingIndexOnForeignKey;
    let ctx = ProjectContext {
        all_schema_id_fields: vec![SchemaIdField {
            field_name: "userId".to_string(),
            table_ref: "users".to_string(),
            table_id: "table@users".to_string(),
            file: "tests/fixtures/perf_patterns.ts".to_string(),
            line: 5,
            col: 10,
        }],
        all_index_definitions: vec![], // no indexes at all
        ..Default::default()
    };
    let diagnostics = rule.check_project(&ctx);
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0]
        .message
        .contains("Foreign key field referencing `users` has no index"));
    assert_eq!(diagnostics[0].severity, Severity::Warning);
    assert!(diagnostics[0].help.contains("v.id()"));
}

#[test]
fn test_missing_index_on_foreign_key_not_fired_when_index_exists() {
    let rule = MissingIndexOnForeignKey;
    let ctx = ProjectContext {
        all_schema_id_fields: vec![SchemaIdField {
            field_name: "userId".to_string(),
            table_ref: "users".to_string(),
            table_id: "table@users".to_string(),
            file: "tests/fixtures/perf_patterns.ts".to_string(),
            line: 5,
            col: 10,
        }],
        all_index_definitions: vec![IndexDef {
            table: "table@users".to_string(),
            name: "by_user".to_string(),
            fields: vec!["userId".to_string()],
            line: 10,
        }],
        ..Default::default()
    };
    let diagnostics = rule.check_project(&ctx);
        assert!(
            diagnostics.is_empty(),
        "Should not flag when an index includes the field"
        );
}

#[test]
fn test_missing_index_on_foreign_key_matches_table_specific() {
    let rule = MissingIndexOnForeignKey;
    let ctx = ProjectContext {
        all_schema_id_fields: vec![SchemaIdField {
            field_name: "userId".to_string(),
            table_ref: "users".to_string(),
            table_id: "table@users".to_string(),
            file: "tests/fixtures/perf_patterns.ts".to_string(),
            line: 5,
            col: 10,
        }],
        all_index_definitions: vec![IndexDef {
            table: "table@orders".to_string(),
            name: "by_user".to_string(),
            fields: vec!["userId".to_string()],
            line: 10,
        }],
        ..Default::default()
    };
    let diagnostics = rule.check_project(&ctx);
    assert_eq!(diagnostics.len(), 1);
    assert!(
        diagnostics[0].message.contains("users"),
        "Should flag when matching field is on a different table"
    );
}

#[test]
fn test_missing_index_on_foreign_key_empty_field_name() {
    let rule = MissingIndexOnForeignKey;
    let ctx = ProjectContext {
        all_schema_id_fields: vec![SchemaIdField {
            field_name: "".to_string(), // empty field name
            table_ref: "users".to_string(),
            table_id: "".to_string(),
            file: "tests/fixtures/perf_patterns.ts".to_string(),
            line: 5,
            col: 10,
        }],
        all_index_definitions: vec![],
        ..Default::default()
    };
    let diagnostics = rule.check_project(&ctx);
    assert!(diagnostics.is_empty(), "Empty field names should be skipped");
}

#[test]
fn test_missing_index_on_foreign_key_per_file_returns_empty() {
    let rule = MissingIndexOnForeignKey;
    let analysis = FileAnalysis::default();
    let diagnostics = rule.check(&analysis);
    assert!(
        diagnostics.is_empty(),
        "Per-file check should always return empty for project-level rule"
    );
}

// ---------------------------------------------------------------------------
// 2. ActionFromClient
// ---------------------------------------------------------------------------

#[test]
fn test_action_from_client_fires_on_fixture() {
    let analysis = analyze_file(Path::new("tests/fixtures/bad_patterns.ts")).unwrap();
    let rule = ActionFromClient;
    let diagnostics = rule.check(&analysis);
    assert!(
        !diagnostics.is_empty(),
        "Should detect public action in bad_patterns.ts"
    );
    assert!(diagnostics[0].message.contains("Public action"));
    assert!(diagnostics[0]
        .message
        .contains("can be called directly from client"));
    assert_eq!(diagnostics[0].severity, Severity::Warning);
}

#[test]
fn test_action_from_client_ignores_internal_action() {
    let rule = ActionFromClient;
    let analysis = FileAnalysis {
        functions: vec![ConvexFunction {
            name: "sendEmail".to_string(),
            kind: FunctionKind::InternalAction,
            has_args_validator: true,
            has_any_validator_in_args: false,
            arg_names: vec![],
            has_return_validator: false,
            has_auth_check: false,
            has_internal_secret: false,
            is_intentionally_public: false,
            handler_line_count: 10,
            span_line: 1,
            span_col: 1,
        }],
        ..Default::default()
    };
    let diagnostics = rule.check(&analysis);
    assert!(
        diagnostics.is_empty(),
        "Should not flag internalAction functions"
    );
}

#[test]
fn test_action_from_client_ignores_queries_and_mutations() {
    let rule = ActionFromClient;
    let analysis = FileAnalysis {
        functions: vec![
            ConvexFunction {
                name: "getItems".to_string(),
                kind: FunctionKind::Query,
                has_args_validator: true,
                has_any_validator_in_args: false,
                arg_names: vec![],
                has_return_validator: false,
                has_auth_check: false,
                has_internal_secret: false,
                is_intentionally_public: false,
                handler_line_count: 5,
                span_line: 1,
                span_col: 1,
            },
            ConvexFunction {
                name: "updateItem".to_string(),
                kind: FunctionKind::Mutation,
                has_args_validator: true,
                has_any_validator_in_args: false,
                arg_names: vec![],
                has_return_validator: false,
                has_auth_check: false,
                has_internal_secret: false,
                is_intentionally_public: false,
                handler_line_count: 5,
                span_line: 10,
                span_col: 1,
            },
        ],
        ..Default::default()
    };
    let diagnostics = rule.check(&analysis);
    assert!(
        diagnostics.is_empty(),
        "Should not flag queries or mutations"
    );
}

// ---------------------------------------------------------------------------
// 3. CollectThenFilter
// ---------------------------------------------------------------------------

#[test]
fn test_collect_then_filter_fires() {
    let rule = CollectThenFilter;
    let analysis = FileAnalysis {
        collect_variable_filters: vec![CallLocation {
            line: 10,
            col: 5,
            detail: "Variable `items` from .collect() is later filtered with .filter()".to_string(),
        }],
        ..Default::default()
    };
    let diagnostics = rule.check(&analysis);
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("items"));
    assert_eq!(diagnostics[0].severity, Severity::Warning);
    assert!(diagnostics[0].help.contains("Collecting all results"));
}

#[test]
fn test_collect_then_filter_empty_when_no_pattern() {
    let rule = CollectThenFilter;
    let analysis = FileAnalysis::default();
    let diagnostics = rule.check(&analysis);
    assert!(
        diagnostics.is_empty(),
        "Should not flag when no collect-then-filter pattern exists"
    );
}

#[test]
fn test_collect_then_filter_multiple() {
    let rule = CollectThenFilter;
    let analysis = FileAnalysis {
        collect_variable_filters: vec![
            CallLocation {
                line: 10,
                col: 5,
                detail: "first pattern".to_string(),
            },
            CallLocation {
                line: 20,
                col: 5,
                detail: "second pattern".to_string(),
            },
        ],
        ..Default::default()
    };
    let diagnostics = rule.check(&analysis);
    assert_eq!(diagnostics.len(), 2);
}

// ---------------------------------------------------------------------------
// 4. LargeDocumentWrite
// ---------------------------------------------------------------------------

#[test]
fn test_large_document_write_fires() {
    let rule = LargeDocumentWrite;
    let analysis = FileAnalysis {
        large_writes: vec![CallLocation {
            line: 15,
            col: 3,
            detail: "ctx.db.insert with 25 properties".to_string(),
        }],
        ..Default::default()
    };
    let diagnostics = rule.check(&analysis);
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0]
        .message
        .contains("Large inline document write"));
    assert!(diagnostics[0].message.contains("25 properties"));
    assert_eq!(diagnostics[0].severity, Severity::Info);
    assert!(diagnostics[0].help.contains("1 MiB"));
}

#[test]
fn test_large_document_write_empty() {
    let rule = LargeDocumentWrite;
    let analysis = FileAnalysis::default();
    let diagnostics = rule.check(&analysis);
    assert!(
        diagnostics.is_empty(),
        "Should not flag when no large writes exist"
    );
}

#[test]
fn test_large_document_write_multiple() {
    let rule = LargeDocumentWrite;
    let analysis = FileAnalysis {
        large_writes: vec![
            CallLocation {
                line: 15,
                col: 3,
                detail: "ctx.db.insert with 25 properties".to_string(),
            },
            CallLocation {
                line: 30,
                col: 3,
                detail: "ctx.db.replace with 30 properties".to_string(),
            },
        ],
        ..Default::default()
    };
    let diagnostics = rule.check(&analysis);
    assert_eq!(diagnostics.len(), 2);
}

// ---------------------------------------------------------------------------
// 5. NoPaginationForList
// ---------------------------------------------------------------------------

#[test]
fn test_no_pagination_for_list_fires_on_fixture() {
    let analysis = analyze_file(Path::new("tests/fixtures/bad_patterns.ts")).unwrap();
    let rule = NoPaginationForList;
    let diagnostics = rule.check(&analysis);
    assert!(
        !diagnostics.is_empty(),
        "Should detect public query with .collect() in bad_patterns.ts"
    );
    assert!(diagnostics[0]
        .message
        .contains("Public query with `.collect()`"));
    assert_eq!(diagnostics[0].severity, Severity::Warning);
    // Only one diagnostic per file
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn test_no_pagination_for_list_no_public_query() {
    let rule = NoPaginationForList;
    let analysis = FileAnalysis {
        functions: vec![ConvexFunction {
            name: "getItems".to_string(),
            kind: FunctionKind::InternalQuery,
            has_args_validator: true,
            has_any_validator_in_args: false,
            arg_names: vec![],
            has_return_validator: false,
            has_auth_check: false,
            has_internal_secret: false,
            is_intentionally_public: false,
            handler_line_count: 5,
            span_line: 1,
            span_col: 1,
        }],
        collect_calls: vec![CallLocation {
            line: 5,
            col: 10,
            detail: ".collect()".to_string(),
        }],
        ..Default::default()
    };
    let diagnostics = rule.check(&analysis);
    assert!(
        diagnostics.is_empty(),
        "Should not flag when there are no public queries"
    );
}

#[test]
fn test_no_pagination_for_list_no_collect() {
    let rule = NoPaginationForList;
    let analysis = FileAnalysis {
        functions: vec![ConvexFunction {
            name: "getItems".to_string(),
            kind: FunctionKind::Query,
            has_args_validator: true,
            has_any_validator_in_args: false,
            arg_names: vec![],
            has_return_validator: false,
            has_auth_check: false,
            has_internal_secret: false,
            is_intentionally_public: false,
            handler_line_count: 5,
            span_line: 1,
            span_col: 1,
        }],
        collect_calls: vec![], // no collect calls
        ..Default::default()
    };
    let diagnostics = rule.check(&analysis);
    assert!(
        diagnostics.is_empty(),
        "Should not flag when there are no .collect() calls"
    );
}

#[test]
fn test_no_pagination_emits_one_diagnostic_per_file() {
    let rule = NoPaginationForList;
    let analysis = FileAnalysis {
        functions: vec![
            ConvexFunction {
                name: "getItems".to_string(),
                kind: FunctionKind::Query,
                has_args_validator: true,
                has_any_validator_in_args: false,
                arg_names: vec![],
                has_return_validator: false,
                has_auth_check: false,
                has_internal_secret: false,
                is_intentionally_public: false,
                handler_line_count: 5,
                span_line: 1,
                span_col: 1,
            },
            ConvexFunction {
                name: "getOtherItems".to_string(),
                kind: FunctionKind::Query,
                has_args_validator: true,
                has_any_validator_in_args: false,
                arg_names: vec![],
                has_return_validator: false,
                has_auth_check: false,
                has_internal_secret: false,
                is_intentionally_public: false,
                handler_line_count: 5,
                span_line: 10,
                span_col: 1,
            },
        ],
        collect_calls: vec![
            CallLocation {
                line: 5,
                col: 10,
                detail: ".collect()".to_string(),
            },
            CallLocation {
                line: 15,
                col: 10,
                detail: ".collect()".to_string(),
            },
        ],
        ..Default::default()
    };
    let diagnostics = rule.check(&analysis);
    assert_eq!(
        diagnostics.len(),
        1,
        "Should emit exactly one diagnostic per file"
    );
}

// ---------------------------------------------------------------------------
// Rule ID and category checks
// ---------------------------------------------------------------------------

#[test]
fn test_rule_ids_are_correct() {
    assert_eq!(
        MissingIndexOnForeignKey.id(),
        "perf/missing-index-on-foreign-key"
    );
    assert_eq!(ActionFromClient.id(), "perf/action-from-client");
    assert_eq!(CollectThenFilter.id(), "perf/collect-then-filter");
    assert_eq!(LargeDocumentWrite.id(), "perf/large-document-write");
    assert_eq!(NoPaginationForList.id(), "perf/no-pagination-for-list");
}

#[test]
fn test_all_rules_are_performance_category() {
    use convex_doctor::diagnostic::Category;
    assert_eq!(MissingIndexOnForeignKey.category(), Category::Performance);
    assert_eq!(ActionFromClient.category(), Category::Performance);
    assert_eq!(CollectThenFilter.category(), Category::Performance);
    assert_eq!(LargeDocumentWrite.category(), Category::Performance);
    assert_eq!(NoPaginationForList.category(), Category::Performance);
}

// ---------------------------------------------------------------------------
// End-to-end: CollectThenFilter through the visitor
// ---------------------------------------------------------------------------

#[test]
fn test_collect_then_filter_end_to_end() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("collect_filter.ts");
    std::fs::write(
        &path,
        r#"
import { query } from "convex/server";

export const bad = query({
  args: {},
  handler: async (ctx) => {
    const items = await ctx.db.query("tasks").collect();
    return items.filter(i => i.done);
  },
});
"#,
    )
    .unwrap();
    let analysis = analyze_file(&path).unwrap();
    let rule = CollectThenFilter;
    let diagnostics = rule.check(&analysis);
    assert!(
        !diagnostics.is_empty(),
        "Should detect collect-then-filter pattern end-to-end"
    );
}

// ---------------------------------------------------------------------------
// Boundary: LargeDocumentWrite (20 props = no fire, 21 = fire)
// ---------------------------------------------------------------------------

#[test]
fn test_large_document_write_boundary() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("boundary.ts");
    // Generate exactly 20 properties -- should NOT fire
    let props_20: String = (1..=20)
        .map(|i| format!("f{}: \"v\"", i))
        .collect::<Vec<_>>()
        .join(", ");
    let props_21: String = (1..=21)
        .map(|i| format!("f{}: \"v\"", i))
        .collect::<Vec<_>>()
        .join(", ");
    std::fs::write(
        &path,
        format!(
            r#"
import {{ mutation }} from "convex/server";

export const ok = mutation({{
  args: {{}},
  handler: async (ctx) => {{
    await ctx.db.insert("t", {{ {} }});
  }},
}});

export const bad = mutation({{
  args: {{}},
  handler: async (ctx) => {{
    await ctx.db.insert("t", {{ {} }});
  }},
}});
"#,
            props_20, props_21
        ),
    )
    .unwrap();
    let analysis = analyze_file(&path).unwrap();
    let rule = LargeDocumentWrite;
    let diagnostics = rule.check(&analysis);
    assert_eq!(
        diagnostics.len(),
        1,
        "Should fire only for >20 properties, not >=20"
    );
}
