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
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis
            .old_syntax_functions
            .iter()
            .map(|c| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Warning,
                category: self.category(),
                message: format!("Old function syntax: {}", c.detail),
                help: "Use `query({ args: ..., handler: async (ctx, args) => ... })` instead of `query(async (ctx) => ...)`.".to_string(),
                file: analysis.file_path.clone(),
                line: c.line,
                column: c.col,
            })
            .collect()
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

/// Stub: detect imports from the wrong Convex runtime.
/// TODO: Implement cross-file analysis to detect when a "use node" file imports
/// from a non-"use node" file or vice versa.
pub struct WrongRuntimeImport;
impl Rule for WrongRuntimeImport {
    fn id(&self) -> &'static str {
        "correctness/wrong-runtime-import"
    }
    fn category(&self) -> Category {
        Category::Correctness
    }
    fn check(&self, _analysis: &FileAnalysis) -> Vec<Diagnostic> {
        // Stub: requires cross-file import graph analysis.
        // Will be implemented in a future version.
        vec![]
    }
}

/// Stub: detect when a function reference is passed directly instead of using api.* reference.
/// TODO: Implement detection of patterns like `ctx.runQuery(getMessages)` instead of
/// `ctx.runQuery(api.messages.getMessages)`.
pub struct DirectFunctionRef;
impl Rule for DirectFunctionRef {
    fn id(&self) -> &'static str {
        "correctness/direct-function-ref"
    }
    fn category(&self) -> Category {
        Category::Correctness
    }
    fn check(&self, _analysis: &FileAnalysis) -> Vec<Diagnostic> {
        // Stub: requires type analysis to distinguish function references from api.* references.
        // Will be implemented in a future version.
        vec![]
    }
}

/// Per-file rule: suggest .unique() when .first() is used on an indexed query.
pub struct MissingUnique;
impl Rule for MissingUnique {
    fn id(&self) -> &'static str {
        "correctness/missing-unique"
    }
    fn category(&self) -> Category {
        Category::Correctness
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis
            .first_calls
            .iter()
            .map(|c| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Warning,
                category: self.category(),
                message: format!(
                    "Using `.first()` on an indexed query: {}",
                    c.detail
                ),
                help: "If you expect exactly one result, use `.unique()` instead of `.first()` to get a runtime error when the assumption is violated.".to_string(),
                file: analysis.file_path.clone(),
                line: c.line,
                column: c.col,
            })
            .collect()
    }
}
