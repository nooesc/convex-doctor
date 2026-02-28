use crate::diagnostic::{Category, Diagnostic, Severity};
use crate::rules::{FileAnalysis, FunctionKind, ProjectContext, Rule};
use std::collections::HashSet;

pub struct UnboundedCollect;
impl Rule for UnboundedCollect {
    fn id(&self) -> &'static str {
        "perf/unbounded-collect"
    }
    fn category(&self) -> Category {
        Category::Performance
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis
            .collect_calls
            .iter()
            .filter(|c| !c.detail.contains(".take."))
            .map(|c| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Error,
                category: self.category(),
                message: "Unbounded `.collect()` call".to_string(),
                help: "Use `.take(n)` to limit results or implement pagination. All results count toward database bandwidth.".to_string(),
                file: analysis.file_path.clone(),
                line: c.line,
                column: c.col,
            })
            .collect()
    }
}

pub struct FilterWithoutIndex;
impl Rule for FilterWithoutIndex {
    fn id(&self) -> &'static str {
        "perf/filter-without-index"
    }
    fn category(&self) -> Category {
        Category::Performance
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis
            .filter_calls
            .iter()
            .map(|c| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Warning,
                category: self.category(),
                message: "`.filter()` without an index scans the entire table".to_string(),
                help: "Define an index on the filtered field and use `.withIndex()` instead for better performance.".to_string(),
                file: analysis.file_path.clone(),
                line: c.line,
                column: c.col,
            })
            .collect()
    }
}

pub struct DateNowInQuery;
impl Rule for DateNowInQuery {
    fn id(&self) -> &'static str {
        "perf/date-now-in-query"
    }
    fn category(&self) -> Category {
        Category::Performance
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        // Date.now() calls are already filtered to query functions at analysis time
        analysis
            .date_now_calls
            .iter()
            .map(|c| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Error,
                category: self.category(),
                message: "`Date.now()` in a query function breaks caching".to_string(),
                help: "Queries must be deterministic. Pass the timestamp as an argument from the client or use a mutation instead.".to_string(),
                file: analysis.file_path.clone(),
                line: c.line,
                column: c.col,
            })
            .collect()
    }
}

pub struct LoopRunMutation;
impl Rule for LoopRunMutation {
    fn id(&self) -> &'static str {
        "perf/loop-run-mutation"
    }
    fn category(&self) -> Category {
        Category::Performance
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis
            .loop_ctx_calls
            .iter()
            .map(|c| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Error,
                category: self.category(),
                message: format!("ctx call `{}` inside a loop", c.detail),
                help: "Calling ctx.runMutation/ctx.runQuery in a loop causes N+1 round trips. Consider batching operations or restructuring.".to_string(),
                file: analysis.file_path.clone(),
                line: c.line,
                column: c.col,
            })
            .collect()
    }
}

/// Per-file rule: warn when multiple ctx.runQuery/ctx.runMutation calls appear
/// sequentially in an action, suggesting they could be batched.
pub struct SequentialRunCalls;
impl Rule for SequentialRunCalls {
    fn id(&self) -> &'static str {
        "perf/sequential-run-calls"
    }
    fn category(&self) -> Category {
        Category::Performance
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        use std::collections::HashMap;

        let mut by_function: HashMap<String, Vec<&crate::rules::CtxCall>> = HashMap::new();
        for call in analysis.ctx_calls.iter().filter(|c| {
            (c.chain.starts_with("ctx.runQuery") || c.chain.starts_with("ctx.runMutation"))
                && c.enclosing_function_kind.as_ref().is_some_and(|k| k.is_action())
        }) {
            let key = call
                .enclosing_function_id
                .clone()
                .unwrap_or_else(|| format!("__anonymous__@{}:{}", call.line, call.col));
            by_function.entry(key).or_default().push(call);
        }

        by_function
            .into_iter()
            .flat_map(|(function_name, calls)| {
                if calls.len() >= 3 {
                    vec![Diagnostic {
                        rule: self.id().to_string(),
                        severity: Severity::Warning,
                        category: self.category(),
                        message: format!(
                            "Action `{}` has {} sequential ctx.run* calls — consider batching",
                            function_name, calls.len()
                        ),
                        help: "Multiple sequential ctx.runQuery/ctx.runMutation calls each start a separate transaction. Consider combining related reads/writes into a single mutation.".to_string(),
                        file: analysis.file_path.clone(),
                        line: calls[0].line,
                        column: calls[0].col,
                    }]
                } else {
                    Vec::new()
                }
            })
            .collect()
    }
}

/// Per-file rule: warn when ctx.runAction is called from within an action.
pub struct UnnecessaryRunAction;
impl Rule for UnnecessaryRunAction {
    fn id(&self) -> &'static str {
        "perf/unnecessary-run-action"
    }
    fn category(&self) -> Category {
        Category::Performance
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis
            .ctx_calls
            .iter()
            .filter(|c| {
                c.chain.starts_with("ctx.runAction")
                    && c.enclosing_function_kind
                        .as_ref()
                        .is_some_and(|k| k.is_action())
            })
            .map(|c| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Warning,
                category: self.category(),
                message: "`ctx.runAction` called from within an action".to_string(),
                help: "If both actions are in the same runtime, call the helper function directly instead of using ctx.runAction.".to_string(),
                file: analysis.file_path.clone(),
                line: c.line,
                column: c.col,
            })
            .collect()
    }
}

/// Per-file rule: warn when ctx.runQuery/ctx.runMutation is used inside a
/// query or mutation (should use a helper function instead).
pub struct HelperVsRun;
impl Rule for HelperVsRun {
    fn id(&self) -> &'static str {
        "perf/helper-vs-run"
    }
    fn category(&self) -> Category {
        Category::Performance
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis
            .ctx_calls
            .iter()
            .filter(|c| {
                (c.chain.starts_with("ctx.runQuery") || c.chain.starts_with("ctx.runMutation"))
                    && c.enclosing_function_kind
                        .as_ref()
                        .is_some_and(|k| k.is_query() || k.is_mutation())
            })
            .map(|c| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Warning,
                category: self.category(),
                message: format!("`{}` used inside a query/mutation", c.chain),
                help: "Use a helper function instead of ctx.runQuery/ctx.runMutation within queries/mutations. Helper functions share the same transaction.".to_string(),
                file: analysis.file_path.clone(),
                line: c.line,
                column: c.col,
            })
            .collect()
    }
}

/// Project-level rule: warn when a schema field using `v.id("table")` has no
/// matching index that includes that field.
pub struct MissingIndexOnForeignKey;
impl Rule for MissingIndexOnForeignKey {
    fn id(&self) -> &'static str {
        "perf/missing-index-on-foreign-key"
    }
    fn category(&self) -> Category {
        Category::Performance
    }
    fn check(&self, _analysis: &FileAnalysis) -> Vec<Diagnostic> {
        vec![]
    }
    fn check_project(&self, ctx: &ProjectContext) -> Vec<Diagnostic> {
        let mut seen = HashSet::<(String, String, String, u32, u32, String)>::new();
        ctx.all_schema_id_fields
            .iter()
            .filter(|id_field| {
                if id_field.field_name.is_empty() || id_field.table_id.is_empty() || id_field.file.is_empty()
                {
                    return false;
                }

                // Check if any index includes this field on the same table
                !ctx.all_index_definitions.iter().any(|idx| {
                    !idx.table.is_empty()
                        && idx.table == id_field.table_id
                        && idx.fields.contains(&id_field.field_name)
                })
            })
            .filter(|id_field| {
                let key = (
                    id_field.file.clone(),
                    id_field.table_id.clone(),
                    id_field.field_name.clone(),
                    id_field.line,
                    id_field.col,
                    id_field.table_ref.clone(),
                );
                seen.insert(key)
            })
            .map(|id_field| {
                let message = format!(
                    "Foreign key field referencing `{}` has no index",
                    id_field.table_ref
                );
                Diagnostic {
                    rule: self.id().to_string(),
                    severity: Severity::Warning,
                    category: self.category(),
                    message,
                    help: "Fields with `v.id()` references are commonly queried. Add an index to avoid full table scans.".to_string(),
                    file: id_field.file.clone(),
                    line: id_field.line,
                    column: id_field.col,
                }
            })
            .collect()
    }
}

/// Per-file rule: warn when a public action can be called directly from the client.
pub struct ActionFromClient;
impl Rule for ActionFromClient {
    fn id(&self) -> &'static str {
        "perf/action-from-client"
    }
    fn category(&self) -> Category {
        Category::Performance
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis
            .functions
            .iter()
            .filter(|f| f.kind == FunctionKind::Action)
            .map(|f| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Warning,
                category: self.category(),
                message: format!("Public action `{}` can be called directly from client", f.name),
                help: "Calling actions from the browser is an anti-pattern. Use a mutation that schedules the action via `ctx.scheduler.runAfter(0, ...)`.".to_string(),
                file: analysis.file_path.clone(),
                line: f.span_line,
                column: f.span_col,
            })
            .collect()
    }
}

/// Per-file rule: warn when results are collected then filtered in JavaScript
/// instead of using `.withIndex()` or `.filter()` on the query.
pub struct CollectThenFilter;
impl Rule for CollectThenFilter {
    fn id(&self) -> &'static str {
        "perf/collect-then-filter"
    }
    fn category(&self) -> Category {
        Category::Performance
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis
            .collect_variable_filters
            .iter()
            .map(|c| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Warning,
                category: self.category(),
                message: c.detail.clone(),
                help: "Collecting all results then filtering in JavaScript wastes bandwidth and breaks query caching. Use `.withIndex()` or `.filter()` on the query instead.".to_string(),
                file: analysis.file_path.clone(),
                line: c.line,
                column: c.col,
            })
            .collect()
    }
}

/// Per-file rule: info when a large inline document write is detected.
pub struct LargeDocumentWrite;
impl Rule for LargeDocumentWrite {
    fn id(&self) -> &'static str {
        "perf/large-document-write"
    }
    fn category(&self) -> Category {
        Category::Performance
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis
            .large_writes
            .iter()
            .map(|c| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Info,
                category: self.category(),
                message: format!("Large inline document write: {}", c.detail),
                help: "Documents approaching the 1 MiB limit may fail at runtime. Consider breaking large documents into related smaller ones.".to_string(),
                file: analysis.file_path.clone(),
                line: c.line,
                column: c.col,
            })
            .collect()
    }
}

/// Per-file rule: warn when a public query uses `.collect()` without pagination,
/// potentially returning unbounded results to the client.
pub struct NoPaginationForList;
impl Rule for NoPaginationForList {
    fn id(&self) -> &'static str {
        "perf/no-pagination-for-list"
    }
    fn category(&self) -> Category {
        Category::Performance
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        let public_query_collect_calls: Vec<_> = analysis
            .ctx_calls
            .iter()
            .filter(|c| {
                c.enclosing_function_kind == Some(FunctionKind::Query)
                    && c.chain.starts_with("ctx.db.")
                    && c.chain.ends_with(".collect")
                    && !c.chain.contains(".take.")
            })
            .collect();

        if !public_query_collect_calls.is_empty() {
            // Emit one diagnostic per file
            let first_collect = public_query_collect_calls[0];
            vec![Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Warning,
                category: self.category(),
                message: "Public query with `.collect()` may return unbounded results to client"
                    .to_string(),
                help: "Consider using `.paginate()` or `.take(n)` for public queries to limit data sent to clients.".to_string(),
                file: analysis.file_path.clone(),
                line: first_collect.line,
                column: first_collect.col,
            }]
        } else {
            vec![]
        }
    }
}

/// Per-file rule: when `.paginate(...)` is used, ensure the function accepts
/// `paginationOpts` validated with `paginationOptsValidator`.
pub struct MissingPaginationOptsValidator;
impl Rule for MissingPaginationOptsValidator {
    fn id(&self) -> &'static str {
        "perf/missing-pagination-opts-validator"
    }
    fn category(&self) -> Category {
        Category::Performance
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        let functions_with_validator: HashSet<&str> = analysis
            .pagination_validator_functions
            .iter()
            .map(String::as_str)
            .collect();

        let mut seen = HashSet::<&str>::new();
        analysis
            .paginated_functions
            .iter()
            .filter(|loc| !loc.detail.is_empty())
            .filter(|loc| !functions_with_validator.contains(loc.detail.as_str()))
            .filter(|loc| seen.insert(loc.detail.as_str()))
            .map(|loc| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Warning,
                category: self.category(),
                message: format!(
                    "Paginated query `{}` is missing `paginationOptsValidator` in args",
                    loc.detail
                ),
                help: "Add `args: { paginationOpts: paginationOptsValidator, ... }` so clients can pass typed pagination options safely.".to_string(),
                file: analysis.file_path.clone(),
                line: loc.line,
                column: loc.col,
            })
            .collect()
    }
}
