use crate::diagnostic::{Category, Diagnostic, Severity};
use crate::rules::{FileAnalysis, Rule};

/// Patterns that should be awaited when used with ctx.
const AWAITABLE_CTX_PREFIXES: &[&str] = &[
    "ctx.scheduler",
    "ctx.db.patch",
    "ctx.db.insert",
    "ctx.db.replace",
    "ctx.db.delete",
    "ctx.runMutation",
    "ctx.runQuery",
    "ctx.runAction",
];

pub struct UnwaitedPromise;
impl Rule for UnwaitedPromise {
    fn id(&self) -> &'static str {
        "correctness/unwaited-promise"
    }
    fn category(&self) -> Category {
        Category::Correctness
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis
            .ctx_calls
            .iter()
            .filter(|c| {
                !c.is_awaited
                    && AWAITABLE_CTX_PREFIXES
                        .iter()
                        .any(|prefix| c.chain.starts_with(prefix))
            })
            .map(|c| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Error,
                category: self.category(),
                message: format!("`{}` is not awaited", c.chain),
                help: "This call returns a Promise that must be awaited. Without `await`, the operation may not complete before the function returns.".to_string(),
                file: analysis.file_path.clone(),
                line: c.line,
                column: c.col,
            })
            .collect()
    }
}

pub struct OldFunctionSyntax;
impl Rule for OldFunctionSyntax {
    fn id(&self) -> &'static str {
        "correctness/old-function-syntax"
    }
    fn category(&self) -> Category {
        Category::Correctness
    }
    fn check(&self, _analysis: &FileAnalysis) -> Vec<Diagnostic> {
        // Needs analyzer enhancement to detect old syntax (e.g., `export default query(...)`)
        vec![]
    }
}

pub struct DbInAction;
impl Rule for DbInAction {
    fn id(&self) -> &'static str {
        "correctness/db-in-action"
    }
    fn category(&self) -> Category {
        Category::Correctness
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis
            .ctx_calls
            .iter()
            .filter(|c| {
                c.chain.starts_with("ctx.db.")
                    && c.enclosing_function_kind
                        .as_ref()
                        .is_some_and(|k| k.is_action())
            })
            .map(|c| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Error,
                category: self.category(),
                message: format!("`{}` used in an action", c.chain),
                help: "Actions cannot directly access the database. Use `ctx.runQuery` or `ctx.runMutation` to read/write data from an action.".to_string(),
                file: analysis.file_path.clone(),
                line: c.line,
                column: c.col,
            })
            .collect()
    }
}

pub struct DeprecatedApi;
impl Rule for DeprecatedApi {
    fn id(&self) -> &'static str {
        "correctness/deprecated-api"
    }
    fn category(&self) -> Category {
        Category::Correctness
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis
            .deprecated_calls
            .iter()
            .map(|c| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Warning,
                category: self.category(),
                message: format!("`{}` is deprecated", c.name),
                help: c.replacement.clone(),
                file: analysis.file_path.clone(),
                line: c.line,
                column: c.col,
            })
            .collect()
    }
}
