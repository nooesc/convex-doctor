use crate::diagnostic::{Category, Diagnostic, Severity};
use crate::rules::{FileAnalysis, Rule};

fn is_crud_like_name(name: &str) -> bool {
    let normalized = name
        .to_ascii_lowercase()
        .trim_start_matches('_')
        .to_string();
    if normalized.is_empty() {
        return false;
    }

    const NON_SIMPLE_HINTS: &[&str] = &[
        "cache", "cached", "helper", "util", "service", "sync", "backfill", "batch", "process",
    ];
    if NON_SIMPLE_HINTS
        .iter()
        .any(|token| normalized.contains(token))
    {
        return false;
    }

    const CRUD_PREFIXES: &[&str] = &[
        "get", "list", "create", "update", "delete", "remove", "upsert", "insert", "find", "fetch",
    ];

    CRUD_PREFIXES
        .iter()
        .any(|prefix| normalized == *prefix || normalized.starts_with(prefix))
}

fn is_chunked_processing_action(name: &str) -> bool {
    let normalized = name.to_ascii_lowercase();
    if normalized.is_empty() {
        return false;
    }

    const CHUNK_KEYWORDS: &[&str] = &[
        "sync",
        "backfill",
        "migrate",
        "reconcile",
        "reindex",
        "drain",
    ];

    CHUNK_KEYWORDS
        .iter()
        .any(|keyword| normalized.contains(keyword))
}

pub struct LargeHandler;

impl Rule for LargeHandler {
    fn id(&self) -> &'static str {
        "arch/large-handler"
    }
    fn category(&self) -> Category {
        Category::Architecture
    }

    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis
            .functions
            .iter()
            .filter(|f| f.handler_line_count > 50)
            .map(|f| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Warning,
                category: self.category(),
                message: format!(
                    "Handler `{}` is {} lines long",
                    f.name, f.handler_line_count
                ),
                help: "Extract logic into helper functions. Keep handlers focused on validation, auth, and orchestration.".to_string(),
                file: analysis.file_path.clone(),
                line: f.span_line,
                column: f.span_col,
            })
            .collect()
    }
}

pub struct MonolithicFile;

impl Rule for MonolithicFile {
    fn id(&self) -> &'static str {
        "arch/monolithic-file"
    }
    fn category(&self) -> Category {
        Category::Architecture
    }

    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        if analysis.exported_function_count > 10 {
            vec![Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Warning,
                category: self.category(),
                message: format!(
                    "File has {} exported functions",
                    analysis.exported_function_count
                ),
                help: "Split into smaller files organized by feature.".to_string(),
                file: analysis.file_path.clone(),
                line: 1,
                column: 1,
            }]
        } else {
            vec![]
        }
    }
}

/// Per-file rule: warn when 3+ functions in the same file have inline auth checks,
/// suggesting the auth pattern should be extracted into a shared helper.
pub struct DuplicatedAuth;
impl Rule for DuplicatedAuth {
    fn id(&self) -> &'static str {
        "arch/duplicated-auth"
    }
    fn category(&self) -> Category {
        Category::Architecture
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        let auth_function_count = analysis
            .functions
            .iter()
            .filter(|f| f.has_auth_check)
            .count();
        if auth_function_count >= 3 {
            vec![Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Warning,
                category: self.category(),
                message: format!(
                    "{} functions contain inline auth checks",
                    auth_function_count
                ),
                help: "Extract authentication logic into a shared helper function to avoid copy-pasting the same auth pattern.".to_string(),
                file: analysis.file_path.clone(),
                line: 1,
                column: 1,
            }]
        } else {
            vec![]
        }
    }
}

/// Info rule: `ctx.runAction` called directly from a mutation risks partial commits
/// if the action fails. Suggest using `ctx.scheduler.runAfter(0, ...)` instead.
pub struct ActionWithoutScheduling;
impl Rule for ActionWithoutScheduling {
    fn id(&self) -> &'static str {
        "arch/action-without-scheduling"
    }
    fn category(&self) -> Category {
        Category::Architecture
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis
            .ctx_calls
            .iter()
            .filter(|c| {
                c.chain.starts_with("ctx.runAction")
                    && c.enclosing_function_kind
                        .as_ref()
                        .is_some_and(|k| k.is_mutation())
            })
            .map(|c| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Info,
                category: self.category(),
                message: format!("`ctx.runAction` called directly from mutation `{}`", c.chain),
                help: "If the action fails, mutation writes are still committed. Consider `ctx.scheduler.runAfter(0, ...)` to decouple the action.".to_string(),
                file: analysis.file_path.clone(),
                line: c.line,
                column: c.col,
            })
            .collect()
    }
}

/// Info rule: `throw new Error(...)` in Convex handlers produces redacted "Server Error"
/// messages in production. Suggest using `ConvexError` for structured client errors.
pub struct NoConvexError;
impl Rule for NoConvexError {
    fn id(&self) -> &'static str {
        "arch/no-convex-error"
    }
    fn category(&self) -> Category {
        Category::Architecture
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis
            .throw_generic_errors
            .iter()
            .map(|loc| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Info,
                category: self.category(),
                message: "`throw new Error(...)` in Convex handler".to_string(),
                help: "Generic errors are redacted to 'Server Error' in production. Use `throw new ConvexError(...)` to send structured error data to clients.".to_string(),
                file: analysis.file_path.clone(),
                line: loc.line,
                column: loc.col,
            })
            .collect()
    }
}

/// Info rule: mixing public and internal functions in the same file makes
/// security auditing harder. Suggest splitting into separate files.
pub struct MixedFunctionTypes;
impl Rule for MixedFunctionTypes {
    fn id(&self) -> &'static str {
        "arch/mixed-function-types"
    }
    fn category(&self) -> Category {
        Category::Architecture
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        let has_public = analysis.functions.iter().any(|f| f.is_public());
        let has_internal = analysis.functions.iter().any(|f| !f.is_public());
        if has_public && has_internal {
            vec![Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Info,
                category: self.category(),
                message: "File exports both public and internal functions".to_string(),
                help: "Mixing public and internal functions in the same file makes security auditing harder. Consider splitting into separate files.".to_string(),
                file: analysis.file_path.clone(),
                line: 1,
                column: 1,
            }]
        } else {
            vec![]
        }
    }
}

/// Info rule: files with 3+ large handlers and no helper functions suggest
/// logic should be extracted into shared unexported helpers.
pub struct NoHelperFunctions;
impl Rule for NoHelperFunctions {
    fn id(&self) -> &'static str {
        "arch/no-helper-functions"
    }
    fn category(&self) -> Category {
        Category::Architecture
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        let large_handler_count = analysis
            .functions
            .iter()
            .filter(|f| f.handler_line_count > 15)
            .count();

        let all_handlers_are_crud = !analysis.functions.is_empty()
            && analysis
                .functions
                .iter()
                .all(|f| is_crud_like_name(&f.name));

        if large_handler_count >= 3
            && analysis.unexported_function_count == 0
            && !all_handlers_are_crud
        {
            vec![Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Info,
                category: self.category(),
                message: format!(
                    "{} handlers with >15 lines and no helper functions",
                    large_handler_count
                ),
                help: "Extract shared business logic into unexported helper functions to improve readability and testability.".to_string(),
                file: analysis.file_path.clone(),
                line: 1,
                column: 1,
            }]
        } else {
            vec![]
        }
    }
}

/// Warning rule: 4+ `ctx.run*` calls in a single action function suggests
/// a deep transaction chain that could be batched into fewer mutations.
pub struct DeepFunctionChain;
impl Rule for DeepFunctionChain {
    fn id(&self) -> &'static str {
        "arch/deep-function-chain"
    }
    fn category(&self) -> Category {
        Category::Architecture
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        use std::collections::HashMap;

        let mut by_function: HashMap<String, Vec<&crate::rules::CtxCall>> = HashMap::new();

        for call in analysis.ctx_calls.iter() {
            let is_action = call
                .enclosing_function_kind
                .as_ref()
                .is_some_and(|k| k.is_action());

            let is_chunked_action = call
                .enclosing_function_name
                .as_deref()
                .is_some_and(is_chunked_processing_action);

            if is_action
                && !call.enclosing_function_has_internal_secret
                && !is_chunked_action
                && (call.chain.starts_with("ctx.runQuery") || call.chain.starts_with("ctx.runMutation"))
            {
                let key = call
                    .enclosing_function_id
                    .clone()
                    .unwrap_or_else(|| format!("__anonymous__@{}:{}", call.line, call.col));
                by_function.entry(key).or_default().push(call);
            }
        }

        by_function
            .into_iter()
            .flat_map(|(function_name, calls)| {
                if calls.len() >= 4 {
                    vec![Diagnostic {
                        rule: self.id().to_string(),
                        severity: Severity::Warning,
                        category: self.category(),
                        message: format!(
                            "Action `{}` has {} ctx.run* calls — deep function chain",
                            function_name,
                            calls.len()
                        ),
                        help: "Each `ctx.runQuery`/`ctx.runMutation` is a separate transaction. Consider batching related operations into fewer mutations.".to_string(),
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
