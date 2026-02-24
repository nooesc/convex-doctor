use std::collections::{HashMap, HashSet};

use crate::diagnostic::{Category, Diagnostic, Severity};
use crate::rules::{FileAnalysis, ProjectContext, Rule};

/// Project-level rule: warn when no schema.ts file exists in convex/ directory.
pub struct MissingSchema;
impl Rule for MissingSchema {
    fn id(&self) -> &'static str {
        "schema/missing-schema"
    }
    fn category(&self) -> Category {
        Category::Schema
    }
    fn check(&self, _analysis: &FileAnalysis) -> Vec<Diagnostic> {
        vec![]
    }
    fn check_project(&self, ctx: &ProjectContext) -> Vec<Diagnostic> {
        if !ctx.has_schema {
            vec![Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Warning,
                category: self.category(),
                message: "No schema.ts file found in convex/ directory".to_string(),
                help: "Create convex/schema.ts to define your database schema with type safety."
                    .to_string(),
                file: "convex/".to_string(),
                line: 0,
                column: 0,
            }]
        } else {
            vec![]
        }
    }
}

/// Per-file rule: warn when schema validators are nested more than 3 levels deep.
pub struct DeepNesting;
impl Rule for DeepNesting {
    fn id(&self) -> &'static str {
        "schema/deep-nesting"
    }
    fn category(&self) -> Category {
        Category::Schema
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        if analysis.schema_nesting_depth > 3 {
            vec![Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Warning,
                category: self.category(),
                message: format!(
                    "Schema validators nested {} levels deep",
                    analysis.schema_nesting_depth
                ),
                help: "Consider flattening deeply nested validators by splitting into separate tables or using v.any() for complex data.".to_string(),
                file: analysis.file_path.clone(),
                line: 1,
                column: 1,
            }]
        } else {
            vec![]
        }
    }
}

/// Per-file rule: warn when v.array(v.id(...)) is used for relationships.
pub struct ArrayRelationships;
impl Rule for ArrayRelationships {
    fn id(&self) -> &'static str {
        "schema/array-relationships"
    }
    fn category(&self) -> Category {
        Category::Schema
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis
            .schema_array_id_fields
            .iter()
            .map(|c| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Warning,
                category: self.category(),
                message: format!("Array of document references: {}", c.detail),
                help: "Arrays of v.id() for relationships can grow unbounded. Consider using a separate join table instead.".to_string(),
                file: analysis.file_path.clone(),
                line: c.line,
                column: c.col,
            })
            .collect()
    }
}

/// Per-file rule: warn when one index is a prefix of another index on the same table.
pub struct RedundantIndex;
impl Rule for RedundantIndex {
    fn id(&self) -> &'static str {
        "schema/redundant-index"
    }
    fn category(&self) -> Category {
        Category::Schema
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];
        for (i, idx) in analysis.index_definitions.iter().enumerate() {
            for other in analysis.index_definitions.iter().skip(i + 1) {
                if !idx.table.is_empty() && idx.table == other.table {
                    // Check if idx.fields is a strict prefix of other.fields
                    if idx.fields.len() < other.fields.len()
                        && idx
                            .fields
                            .iter()
                            .zip(other.fields.iter())
                            .all(|(a, b)| a == b)
                    {
                        diagnostics.push(Diagnostic {
                            rule: self.id().to_string(),
                            severity: Severity::Warning,
                            category: self.category(),
                            message: format!(
                                "Index '{}' is redundant — it's a prefix of index '{}'",
                                idx.name, other.name
                            ),
                            help: "A compound index can serve queries on its prefix fields. Remove the shorter index to reduce storage overhead.".to_string(),
                            file: analysis.file_path.clone(),
                            line: idx.line,
                            column: 1,
                        });
                    }
                    // Check reverse: other.fields is a strict prefix of idx.fields
                    if other.fields.len() < idx.fields.len()
                        && other
                            .fields
                            .iter()
                            .zip(idx.fields.iter())
                            .all(|(a, b)| a == b)
                    {
                        diagnostics.push(Diagnostic {
                            rule: self.id().to_string(),
                            severity: Severity::Warning,
                            category: self.category(),
                            message: format!(
                                "Index '{}' is redundant — it's a prefix of index '{}'",
                                other.name, idx.name
                            ),
                            help: "A compound index can serve queries on its prefix fields. Remove the shorter index to reduce storage overhead.".to_string(),
                            file: analysis.file_path.clone(),
                            line: other.line,
                            column: 1,
                        });
                    }
                }
            }
        }
        diagnostics
    }
}

/// Per-file rule: info when a single table has >= 8 indexes.
pub struct TooManyIndexes;
impl Rule for TooManyIndexes {
    fn id(&self) -> &'static str {
        "schema/too-many-indexes"
    }
    fn category(&self) -> Category {
        Category::Schema
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        let mut by_table: HashMap<&str, Vec<&crate::rules::IndexDef>> = HashMap::new();
        for idx in &analysis.index_definitions {
            if !idx.table.is_empty() {
                by_table.entry(idx.table.as_str()).or_default().push(idx);
            }
        }
        let mut diagnostics = vec![];
        for (table, indexes) in &by_table {
            if indexes.len() >= 8 {
                diagnostics.push(Diagnostic {
                    rule: self.id().to_string(),
                    severity: Severity::Info,
                    category: self.category(),
                    message: format!(
                        "Table '{}' has {} indexes (limit is 32)",
                        table,
                        indexes.len()
                    ),
                    help: "Each index adds storage overhead and slows writes. Consider consolidating or removing unused indexes.".to_string(),
                    file: analysis.file_path.clone(),
                    line: indexes[0].line,
                    column: 1,
                });
            }
        }
        diagnostics
    }
}

/// Per-file rule: info when a search index has no filterFields.
pub struct MissingSearchIndexFilter;
impl Rule for MissingSearchIndexFilter {
    fn id(&self) -> &'static str {
        "schema/missing-search-index-filter"
    }
    fn category(&self) -> Category {
        Category::Schema
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis
            .search_index_definitions
            .iter()
            .filter(|s| !s.has_filter_fields)
            .map(|s| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Info,
                category: self.category(),
                message: format!("Search index `{}` has no filterFields", s.name),
                help: "Adding filterFields to search indexes improves query performance by narrowing results before full-text search.".to_string(),
                file: analysis.file_path.clone(),
                line: s.line,
                column: 1,
            })
            .collect()
    }
}

/// Per-file rule: warning when a schema file has >= 5 optional fields without
/// explicit undefined handling.
pub struct OptionalFieldNoDefaultHandling;
impl Rule for OptionalFieldNoDefaultHandling {
    fn id(&self) -> &'static str {
        "schema/optional-field-no-default-handling"
    }
    fn category(&self) -> Category {
        Category::Schema
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        if analysis.file_path.contains("schema") && analysis.optional_schema_fields.len() >= 5 {
            vec![Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Warning,
                category: self.category(),
                message: format!(
                    "{} optional fields in schema — ensure undefined is handled",
                    analysis.optional_schema_fields.len()
                ),
                help: "Optional fields return `undefined` when not set. Ensure all access sites handle the missing case.".to_string(),
                file: analysis.file_path.clone(),
                line: 1,
                column: 1,
            }]
        } else {
            vec![]
        }
    }
}

/// Project-level rule: warning when query filter fields have no matching index.
///
/// Cross-references `filter_field_names` from each file against
/// `ctx.all_index_definitions`.  For v1 this is a simplified check that looks
/// at the first field of each index definition.
// TODO: In a future version, access per-file FileAnalysis from check_project
// to provide file-level diagnostics with precise locations.  The current Rule
// trait only passes ProjectContext to check_project, so we aggregate here.
pub struct MissingIndexForQuery;
impl Rule for MissingIndexForQuery {
    fn id(&self) -> &'static str {
        "schema/missing-index-for-query"
    }
    fn category(&self) -> Category {
        Category::Schema
    }
    fn check(&self, _analysis: &FileAnalysis) -> Vec<Diagnostic> {
        vec![]
    }
    fn check_project(&self, ctx: &ProjectContext) -> Vec<Diagnostic> {
        if !ctx.has_schema {
            return vec![];
        }

        // If no indexes at all but schema exists, that's worth a warning
        if ctx.all_index_definitions.is_empty() && !ctx.all_filter_field_names.is_empty() {
            return vec![Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Warning,
                category: self.category(),
                message: "Schema exists but no database indexes are defined".to_string(),
                help: "Define indexes on fields you query frequently to avoid full table scans."
                    .to_string(),
                file: "convex/schema.ts".to_string(),
                line: 1,
                column: 1,
            }];
        }

        // Collect all indexed first-fields
        let indexed_fields: HashSet<&str> = ctx
            .all_index_definitions
            .iter()
            .filter_map(|idx| idx.fields.first().map(|f| f.as_str()))
            .collect();

        // Warn for filter fields not covered by any index
        ctx.all_filter_field_names
            .iter()
            .filter(|ff| !indexed_fields.contains(ff.field_name.as_str()))
            .map(|ff| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Warning,
                category: self.category(),
                message: format!(
                    "Query filters on field `{}` but no index starts with that field",
                    ff.field_name
                ),
                help: "Add an index starting with this field to avoid full table scans."
                    .to_string(),
                file: "convex/schema.ts".to_string(),
                line: ff.line,
                column: ff.col,
            })
            .collect()
    }
}
