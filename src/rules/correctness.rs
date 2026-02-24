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
                    && !c.is_returned
                    && !c
                        .assigned_to
                        .as_ref()
                        .is_some_and(|name| analysis.awaited_identifiers.iter().any(|a| a == name))
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
pub struct WrongRuntimeImport;
impl Rule for WrongRuntimeImport {
    fn id(&self) -> &'static str {
        "correctness/wrong-runtime-import"
    }
    fn category(&self) -> Category {
        Category::Correctness
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        const NODE_BUILTINS: &[&str] = &["fs", "path", "crypto", "child_process", "os", "stream"];

        analysis
            .imports
            .iter()
            .filter_map(|import| {
                let source = import.source.as_str();

                if !analysis.has_use_node
                    && (source == "convex/node"
                        || source.starts_with("node:")
                        || NODE_BUILTINS.contains(&source))
                {
                    Some(Diagnostic {
                        rule: self.id().to_string(),
                        severity: Severity::Warning,
                        category: self.category(),
                        message: format!("Import `{source}` requires Node runtime"),
                        help: "Add `\"use node\";` at the top of this file or replace Node-only imports with Convex runtime-compatible APIs.".to_string(),
                        file: analysis.file_path.clone(),
                        line: import.line,
                        column: 1,
                    })
                } else if analysis.has_use_node
                    && (source == "convex/browser" || source == "convex/react")
                {
                    Some(Diagnostic {
                        rule: self.id().to_string(),
                        severity: Severity::Warning,
                        category: self.category(),
                        message: format!("Node runtime file imports browser runtime package `{source}`"),
                        help: "Avoid browser/runtime client imports in server files. Use server-side Convex modules instead.".to_string(),
                        file: analysis.file_path.clone(),
                        line: import.line,
                        column: 1,
                    })
                } else {
                    None
                }
            })
            .collect()
    }
}

pub struct DirectFunctionRef;
impl Rule for DirectFunctionRef {
    fn id(&self) -> &'static str {
        "correctness/direct-function-ref"
    }
    fn category(&self) -> Category {
        Category::Correctness
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis
            .ctx_calls
            .iter()
            .filter_map(|call| {
                if !(call.chain.starts_with("ctx.runQuery")
                    || call.chain.starts_with("ctx.runMutation")
                    || call.chain.starts_with("ctx.runAction"))
                {
                    return None;
                }

                let arg = call.first_arg_chain.as_deref()?;
                if arg.starts_with("api.") || arg.starts_with("internal.") {
                    return None;
                }

                Some(Diagnostic {
                    rule: self.id().to_string(),
                    severity: Severity::Warning,
                    category: self.category(),
                    message: format!("`{}` called with direct function reference `{arg}`", call.chain),
                    help: "Use generated API references like `api.module.fn` or `internal.module.fn` instead of passing direct function identifiers.".to_string(),
                    file: analysis.file_path.clone(),
                    line: call.line,
                    column: call.col,
                })
            })
            .collect()
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
