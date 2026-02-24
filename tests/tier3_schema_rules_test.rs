use convex_doctor::diagnostic::Severity;
use convex_doctor::rules::context::analyze_file;
use convex_doctor::rules::schema::*;
use convex_doctor::rules::{ProjectContext, Rule};
use std::path::Path;
use tempfile::TempDir;

// ── TooManyIndexes ──────────────────────────────────────────────────────────

#[test]
fn test_too_many_indexes_triggered() {
    let analysis =
        analyze_file(Path::new("tests/fixtures/schema_many_indexes.ts")).unwrap();
    let rule = TooManyIndexes;
    let diagnostics = rule.check(&analysis);
    assert!(
        !diagnostics.is_empty(),
        "Should flag table with {} indexes (threshold is 8). Index defs: {:?}",
        analysis.index_definitions.len(),
        analysis
            .index_definitions
            .iter()
            .map(|i| format!("{}.{}", i.table, i.name))
            .collect::<Vec<_>>()
    );
    assert_eq!(diagnostics[0].severity, Severity::Info);
    assert!(diagnostics[0].message.contains("indexes"));
}

#[test]
fn test_too_many_indexes_not_triggered_below_threshold() {
    // schema_patterns.ts has only 2 indexes on 'posts'
    let analysis =
        analyze_file(Path::new("tests/fixtures/schema_patterns.ts")).unwrap();
    let rule = TooManyIndexes;
    let diagnostics = rule.check(&analysis);
    assert!(
        diagnostics.is_empty(),
        "Should not flag table with only {} indexes",
        analysis.index_definitions.len()
    );
}

#[test]
fn test_too_many_indexes_exactly_8() {
    let dir = TempDir::new().unwrap();
    let schema_path = dir.path().join("schema.ts");
    std::fs::write(
        &schema_path,
        r#"
import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  items: defineTable({
    a: v.string(),
    b: v.string(),
    c: v.string(),
    d: v.string(),
    e: v.string(),
    f: v.string(),
    g: v.string(),
    h: v.string(),
  })
    .index("idx1", ["a"])
    .index("idx2", ["b"])
    .index("idx3", ["c"])
    .index("idx4", ["d"])
    .index("idx5", ["e"])
    .index("idx6", ["f"])
    .index("idx7", ["g"])
    .index("idx8", ["h"]),
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&schema_path).unwrap();
    let rule = TooManyIndexes;
    let diagnostics = rule.check(&analysis);
    assert_eq!(
        diagnostics.len(),
        1,
        "Exactly 8 indexes should trigger the rule"
    );
}

// ── MissingSearchIndexFilter ────────────────────────────────────────────────

#[test]
fn test_missing_search_index_filter_detected() {
    let analysis =
        analyze_file(Path::new("tests/fixtures/schema_search_no_filter.ts")).unwrap();
    let rule = MissingSearchIndexFilter;
    let diagnostics = rule.check(&analysis);

    // search_body has no filterFields, search_title does
    assert_eq!(
        diagnostics.len(),
        1,
        "Should flag 1 search index without filterFields, found {}. Search defs: {:?}",
        diagnostics.len(),
        analysis
            .search_index_definitions
            .iter()
            .map(|s| format!(
                "{}(has_filter={})",
                s.name, s.has_filter_fields
            ))
            .collect::<Vec<_>>()
    );
    assert_eq!(diagnostics[0].severity, Severity::Info);
    assert!(diagnostics[0].message.contains("search_body"));
}

#[test]
fn test_missing_search_index_filter_all_have_filters() {
    let dir = TempDir::new().unwrap();
    let schema_path = dir.path().join("schema.ts");
    std::fs::write(
        &schema_path,
        r#"
import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  articles: defineTable({
    title: v.string(),
    body: v.string(),
    category: v.string(),
  })
    .searchIndex("search_body", {
      searchField: "body",
      filterFields: ["category"],
    })
    .searchIndex("search_title", {
      searchField: "title",
      filterFields: ["category"],
    }),
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&schema_path).unwrap();
    let rule = MissingSearchIndexFilter;
    let diagnostics = rule.check(&analysis);
    assert!(
        diagnostics.is_empty(),
        "No diagnostics when all search indexes have filterFields"
    );
}

// ── OptionalFieldNoDefaultHandling ──────────────────────────────────────────

#[test]
fn test_optional_fields_warning_triggered() {
    let analysis =
        analyze_file(Path::new("tests/fixtures/schema_many_optional.ts")).unwrap();
    let rule = OptionalFieldNoDefaultHandling;
    let diagnostics = rule.check(&analysis);
    assert_eq!(
        diagnostics.len(),
        1,
        "Should emit exactly one diagnostic for {} optional fields",
        analysis.optional_schema_fields.len()
    );
    assert_eq!(diagnostics[0].severity, Severity::Warning);
    assert!(diagnostics[0].message.contains("optional fields"));
    assert!(diagnostics[0].message.contains(&analysis.optional_schema_fields.len().to_string()));
}

#[test]
fn test_optional_fields_not_triggered_below_threshold() {
    let dir = TempDir::new().unwrap();
    let schema_path = dir.path().join("schema.ts");
    std::fs::write(
        &schema_path,
        r#"
import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  users: defineTable({
    name: v.string(),
    email: v.optional(v.string()),
  }),
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&schema_path).unwrap();
    let rule = OptionalFieldNoDefaultHandling;
    let diagnostics = rule.check(&analysis);
    assert!(
        diagnostics.is_empty(),
        "Should not flag with only {} optional fields (threshold is 5)",
        analysis.optional_schema_fields.len()
    );
}

#[test]
fn test_optional_fields_not_triggered_for_non_schema_file() {
    // basic_query.ts doesn't contain "schema" in filename
    let analysis =
        analyze_file(Path::new("tests/fixtures/basic_query.ts")).unwrap();
    let rule = OptionalFieldNoDefaultHandling;
    let diagnostics = rule.check(&analysis);
    assert!(
        diagnostics.is_empty(),
        "Should not flag non-schema files"
    );
}

// ── MissingIndexForQuery (project-level) ────────────────────────────────────

#[test]
fn test_missing_index_for_query_no_indexes() {
    let rule = MissingIndexForQuery;
    let ctx = ProjectContext {
        has_schema: true,
        all_index_definitions: vec![],
        ..Default::default()
    };
    let diagnostics = rule.check_project(&ctx);
    assert_eq!(
        diagnostics.len(),
        1,
        "Should warn when schema exists but no indexes defined"
    );
    assert_eq!(diagnostics[0].severity, Severity::Warning);
    assert!(diagnostics[0].message.contains("no database indexes"));
}

#[test]
fn test_missing_index_for_query_has_indexes() {
    let rule = MissingIndexForQuery;
    let ctx = ProjectContext {
        has_schema: true,
        all_index_definitions: vec![convex_doctor::rules::IndexDef {
            table: "users".to_string(),
            name: "by_email".to_string(),
            fields: vec!["email".to_string()],
            line: 10,
        }],
        ..Default::default()
    };
    let diagnostics = rule.check_project(&ctx);
    assert!(
        diagnostics.is_empty(),
        "Should not warn when indexes exist"
    );
}

#[test]
fn test_missing_index_for_query_no_schema() {
    let rule = MissingIndexForQuery;
    let ctx = ProjectContext {
        has_schema: false,
        all_index_definitions: vec![],
        ..Default::default()
    };
    let diagnostics = rule.check_project(&ctx);
    assert!(
        diagnostics.is_empty(),
        "Should not warn when there is no schema at all"
    );
}
