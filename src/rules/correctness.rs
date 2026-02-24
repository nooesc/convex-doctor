use crate::diagnostic::{Category, Diagnostic, Severity};
use crate::rules::{FileAnalysis, ProjectContext, Rule};

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

/// Detect ctx.db write operations and ctx.scheduler calls inside query functions.
pub struct QuerySideEffect;
impl Rule for QuerySideEffect {
    fn id(&self) -> &'static str {
        "correctness/query-side-effect"
    }
    fn category(&self) -> Category {
        Category::Correctness
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        const WRITE_PREFIXES: &[&str] = &[
            "ctx.db.insert",
            "ctx.db.patch",
            "ctx.db.replace",
            "ctx.db.delete",
            "ctx.scheduler",
        ];
        analysis
            .ctx_calls
            .iter()
            .filter(|c| {
                c.enclosing_function_kind
                    .as_ref()
                    .is_some_and(|k| k.is_query())
                    && WRITE_PREFIXES.iter().any(|p| c.chain.starts_with(p))
            })
            .map(|c| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Error,
                category: self.category(),
                message: format!(
                    "`{}` in a query function — queries must be read-only",
                    c.chain
                ),
                help:
                    "Queries must be deterministic and side-effect-free. Move writes to a mutation."
                        .to_string(),
                file: analysis.file_path.clone(),
                line: c.line,
                column: c.col,
            })
            .collect()
    }
}

/// Detect ctx.runMutation called from query functions.
pub struct MutationInQuery;
impl Rule for MutationInQuery {
    fn id(&self) -> &'static str {
        "correctness/mutation-in-query"
    }
    fn category(&self) -> Category {
        Category::Correctness
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis
            .ctx_calls
            .iter()
            .filter(|c| {
                c.chain.starts_with("ctx.runMutation")
                    && c.enclosing_function_kind
                        .as_ref()
                        .is_some_and(|k| k.is_query())
            })
            .map(|c| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Error,
                category: self.category(),
                message: format!("`{}` called from a query function", c.chain),
                help: "Queries cannot call mutations. Move mutation calls to a mutation or action."
                    .to_string(),
                file: analysis.file_path.clone(),
                line: c.line,
                column: c.col,
            })
            .collect()
    }
}

/// Detect cron jobs that use public API references instead of internal ones.
pub struct CronUsesPublicApi;
impl Rule for CronUsesPublicApi {
    fn id(&self) -> &'static str {
        "correctness/cron-uses-public-api"
    }
    fn category(&self) -> Category {
        Category::Correctness
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis
            .cron_api_refs
            .iter()
            .map(|c| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Error,
                category: self.category(),
                message: format!("Cron job uses public API reference `{}`", c.detail),
                help: "Use `internal.*` instead of `api.*` in cron job definitions.".to_string(),
                file: analysis.file_path.clone(),
                line: c.line,
                column: c.col,
            })
            .collect()
    }
}

/// Detect queries/mutations defined in "use node" files (only actions allowed).
pub struct NodeQueryMutation;
impl Rule for NodeQueryMutation {
    fn id(&self) -> &'static str {
        "correctness/node-query-mutation"
    }
    fn category(&self) -> Category {
        Category::Correctness
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        if !analysis.has_use_node {
            return vec![];
        }
        analysis
            .functions
            .iter()
            .filter(|f| {
                matches!(
                    f.kind,
                    crate::rules::FunctionKind::Query
                        | crate::rules::FunctionKind::Mutation
                        | crate::rules::FunctionKind::InternalQuery
                        | crate::rules::FunctionKind::InternalMutation
                )
            })
            .map(|f| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Error,
                category: self.category(),
                message: format!("{} `{}` in a \"use node\" file", f.kind_str(), f.name),
                help: "Only actions can use the Node.js runtime.".to_string(),
                file: analysis.file_path.clone(),
                line: f.span_line,
                column: f.span_col,
            })
            .collect()
    }
}

/// Suggest capturing scheduler return values for future cancellation/monitoring.
pub struct SchedulerReturnIgnored;
impl Rule for SchedulerReturnIgnored {
    fn id(&self) -> &'static str {
        "correctness/scheduler-return-ignored"
    }
    fn category(&self) -> Category {
        Category::Correctness
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis
            .ctx_calls
            .iter()
            .filter(|c| {
                (c.chain.starts_with("ctx.scheduler.runAfter")
                    || c.chain.starts_with("ctx.scheduler.runAt"))
                    && c.assigned_to.is_none()
            })
            .map(|c| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Info,
                category: self.category(),
                message: format!("`{}` return value not captured", c.chain),
                help: "Capture the returned scheduled function ID if you need to cancel or monitor it.".to_string(),
                file: analysis.file_path.clone(),
                line: c.line,
                column: c.col,
            })
            .collect()
    }
}

/// Detect non-deterministic calls (Math.random(), new Date()) in query functions.
pub struct NonDeterministicInQuery;
impl Rule for NonDeterministicInQuery {
    fn id(&self) -> &'static str {
        "correctness/non-deterministic-in-query"
    }
    fn category(&self) -> Category {
        Category::Correctness
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis
            .non_deterministic_calls
            .iter()
            .map(|c| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Warning,
                category: self.category(),
                message: format!("`{}` in a query function breaks determinism", c.detail),
                help: "Queries must be deterministic. Pass values as arguments or use a mutation."
                    .to_string(),
                file: analysis.file_path.clone(),
                line: c.line,
                column: c.col,
            })
            .collect()
    }
}

/// Suggest using ctx.db.patch instead of ctx.db.replace for partial updates.
pub struct ReplaceVsPatch;
impl Rule for ReplaceVsPatch {
    fn id(&self) -> &'static str {
        "correctness/replace-vs-patch"
    }
    fn category(&self) -> Category {
        Category::Correctness
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis
            .ctx_calls
            .iter()
            .filter(|c| c.chain.starts_with("ctx.db.replace"))
            .map(|c| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Info,
                category: self.category(),
                message: "`ctx.db.replace` overwrites the entire document".to_string(),
                help: "Did you mean `ctx.db.patch`? `replace` removes all fields not in the new object. Use `patch` for partial updates.".to_string(),
                file: analysis.file_path.clone(),
                line: c.line,
                column: c.col,
            })
            .collect()
    }
}

/// Project-level rule: detect modifications to convex/_generated/ files.
pub struct GeneratedCodeModified;
impl Rule for GeneratedCodeModified {
    fn id(&self) -> &'static str {
        "correctness/generated-code-modified"
    }
    fn category(&self) -> Category {
        Category::Correctness
    }
    fn check(&self, _analysis: &FileAnalysis) -> Vec<Diagnostic> {
        vec![]
    }
    fn check_project(&self, ctx: &ProjectContext) -> Vec<Diagnostic> {
        if ctx.generated_files_modified {
            vec![Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Error,
                category: self.category(),
                message: "Modified files detected in convex/_generated/".to_string(),
                help: "Files in _generated/ are auto-generated and will be overwritten. Revert manual changes.".to_string(),
                file: "convex/_generated/".to_string(),
                line: 0,
                column: 0,
            }]
        } else {
            vec![]
        }
    }
}
