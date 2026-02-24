use crate::diagnostic::{Category, Diagnostic, Severity};
use crate::rules::{FileAnalysis, Rule};

pub struct UnboundedCollect;
impl Rule for UnboundedCollect {
    fn id(&self) -> &'static str {
        "performance/unbounded-collect"
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
        "performance/filter-without-index"
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
        "performance/date-now-in-query"
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
        "performance/loop-run-mutation"
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
