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
