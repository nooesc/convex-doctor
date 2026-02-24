use crate::diagnostic::{Category, Diagnostic, Severity};
use crate::rules::{FileAnalysis, Rule};

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
        let run_calls: Vec<_> = analysis
            .ctx_calls
            .iter()
            .filter(|c| {
                (c.chain.starts_with("ctx.runQuery") || c.chain.starts_with("ctx.runMutation"))
                    && c.enclosing_function_kind
                        .as_ref()
                        .is_some_and(|k| k.is_action())
            })
            .collect();

        if run_calls.len() >= 3 {
            vec![Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Warning,
                category: self.category(),
                message: format!(
                    "{} sequential ctx.run* calls in action — consider batching",
                    run_calls.len()
                ),
                help: "Multiple sequential ctx.runQuery/ctx.runMutation calls each start a separate transaction. Consider combining related reads/writes into a single mutation.".to_string(),
                file: analysis.file_path.clone(),
                line: run_calls[0].line,
                column: run_calls[0].col,
            }]
        } else {
            vec![]
        }
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
