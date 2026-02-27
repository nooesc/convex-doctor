use std::collections::BTreeMap;
use std::path::Path;

use crate::diagnostic::{Category, Diagnostic, Severity};
use crate::rules::{FileAnalysis, FunctionKind, ProjectContext, Rule};

fn path_has_segment(path: &str, segment: &str) -> bool {
    let normalized = path.replace('\\', "/");
    Path::new(&normalized)
        .components()
        .any(|component| component.as_os_str().to_string_lossy() == segment)
}

pub struct MissingArgValidators;
impl Rule for MissingArgValidators {
    fn id(&self) -> &'static str {
        "security/missing-arg-validators"
    }
    fn category(&self) -> Category {
        Category::Security
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis
            .functions
            .iter()
            .filter(|f| {
                f.kind != FunctionKind::HttpAction && !f.has_args_validator
            })
            .map(|f| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Error,
                category: self.category(),
                message: format!(
                    "{} `{}` has no argument validators",
                    f.kind_str(),
                    f.name
                ),
                help: "Add `args: { ... }` with validators for all parameters. Convex guidance requires validators for query/mutation/action and internal variants.".to_string(),
                file: analysis.file_path.clone(),
                line: f.span_line,
                column: f.span_col,
            })
            .collect()
    }
}

pub struct MissingReturnValidators;
impl Rule for MissingReturnValidators {
    fn id(&self) -> &'static str {
        "security/missing-return-validators"
    }
    fn category(&self) -> Category {
        Category::Security
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis
            .functions
            .iter()
            .filter(|f| f.is_public() && !f.has_return_validator)
            .map(|f| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Warning,
                category: self.category(),
                message: format!(
                    "{} `{}` has no return value validator",
                    f.kind_str(),
                    f.name
                ),
                help: "Add `returns: v.object({...})` to validate the return type and prevent accidental data leaks.".to_string(),
                file: analysis.file_path.clone(),
                line: f.span_line,
                column: f.span_col,
            })
            .collect()
    }
}

pub struct MissingAuthCheck;
impl Rule for MissingAuthCheck {
    fn id(&self) -> &'static str {
        "security/missing-auth-check"
    }
    fn category(&self) -> Category {
        Category::Security
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        // Skip conventional admin/migration directories — public functions in
        // _scripts/ or _internal/ are typically only called from admin tooling.
        if path_has_segment(&analysis.file_path, "_scripts")
            || path_has_segment(&analysis.file_path, "_internal")
        {
            return vec![];
        }

        analysis
            .functions
            .iter()
            .filter(|f| {
                f.is_public()
                    && !f.has_auth_check
                    && !f.has_internal_secret
                    && !f.is_intentionally_public
            })
            .map(|f| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Warning,
                category: self.category(),
                message: format!(
                    "Public {} `{}` does not check authentication",
                    f.kind_str(),
                    f.name
                ),
                help: "Consider adding `const identity = await ctx.auth.getUserIdentity()` to verify the caller is authenticated.".to_string(),
                file: analysis.file_path.clone(),
                line: f.span_line,
                column: f.span_col,
            })
            .collect()
    }
}

pub struct InternalApiMisuse;
impl Rule for InternalApiMisuse {
    fn id(&self) -> &'static str {
        "security/internal-api-misuse"
    }
    fn category(&self) -> Category {
        Category::Security
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        let scheduler_or_run_prefixes = [
            "ctx.scheduler",
            "ctx.runMutation",
            "ctx.runQuery",
            "ctx.runAction",
        ];

        analysis
            .ctx_calls
            .iter()
            .filter(|call| {
                let chain_matches = scheduler_or_run_prefixes
                    .iter()
                    .any(|prefix| call.chain.starts_with(prefix));
                let arg_is_public_api = call
                    .first_arg_chain
                    .as_ref()
                    .is_some_and(|arg| arg.starts_with("api."));
                chain_matches && arg_is_public_api && !call.enclosing_function_has_internal_secret
            })
            .map(|call| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Error,
                category: self.category(),
                message: format!(
                    "`{}` is called with public API reference `{}`",
                    call.chain,
                    call.first_arg_chain.as_deref().unwrap_or("unknown")
                ),
                help: "Use `internal.` instead of `api.` for server-to-server calls. Public API references expose endpoints that bypass internal access controls.".to_string(),
                file: analysis.file_path.clone(),
                line: call.line,
                column: call.col,
            })
            .collect()
    }
}

pub struct HardcodedSecrets;
impl Rule for HardcodedSecrets {
    fn id(&self) -> &'static str {
        "security/hardcoded-secrets"
    }
    fn category(&self) -> Category {
        Category::Security
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis
            .hardcoded_secrets
            .iter()
            .map(|secret| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Error,
                category: self.category(),
                message: format!("Hardcoded secret detected: {}", secret.detail),
                help: "Use environment variables via `process.env.SECRET_NAME` instead of hardcoding secrets in source code.".to_string(),
                file: analysis.file_path.clone(),
                line: secret.line,
                column: secret.col,
            })
            .collect()
    }
}

/// Project-level rule: error when .env.local exists but is not in .gitignore.
pub struct EnvNotGitignored;
impl Rule for EnvNotGitignored {
    fn id(&self) -> &'static str {
        "security/env-not-gitignored"
    }
    fn category(&self) -> Category {
        Category::Security
    }
    fn check(&self, _analysis: &FileAnalysis) -> Vec<Diagnostic> {
        vec![]
    }
    fn check_project(&self, ctx: &ProjectContext) -> Vec<Diagnostic> {
        if ctx.has_env_local && !ctx.env_gitignored {
            vec![Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Error,
                category: self.category(),
                message: ".env.local exists but is not in .gitignore".to_string(),
                help: "Add `.env.local` to your .gitignore to prevent committing secrets."
                    .to_string(),
                file: ".env.local".to_string(),
                line: 0,
                column: 0,
            }]
        } else {
            vec![]
        }
    }
}

pub struct SpoofableAccessControl;
impl Rule for SpoofableAccessControl {
    fn id(&self) -> &'static str {
        "security/spoofable-access-control"
    }
    fn category(&self) -> Category {
        Category::Security
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        const SENSITIVE_ARG_NAMES: &[&str] = &[
            "userId",
            "user_id",
            "role",
            "isAdmin",
            "admin",
            "ownerId",
            "organizationId",
            "orgId",
            "teamId",
            "accountId",
            "permission",
            "permissions",
        ];

        analysis
            .functions
            .iter()
            .filter(|f| {
                f.is_public()
                    && !f.has_auth_check
                    && !f.has_internal_secret
                    && !f.is_intentionally_public
            })
            .filter_map(|f| {
                let risky_args: Vec<&str> = f
                    .arg_names
                    .iter()
                    .filter_map(|arg| {
                        let arg_name = arg.as_str();
                        SENSITIVE_ARG_NAMES
                            .iter()
                            .find(|candidate| arg_name.eq_ignore_ascii_case(candidate))
                            .copied()
                    })
                    .collect();

                if risky_args.is_empty() {
                    return None;
                }

                Some(Diagnostic {
                    rule: self.id().to_string(),
                    severity: Severity::Warning,
                    category: self.category(),
                    message: format!(
                        "Public {} `{}` appears to use spoofable access-control args: {}",
                        f.kind_str(),
                        f.name,
                        risky_args.join(", ")
                    ),
                    help: "Avoid authorizing requests using client-provided role/user identifiers. Verify access with `ctx.auth.getUserIdentity()` and server-side ownership checks.".to_string(),
                    file: analysis.file_path.clone(),
                    line: f.span_line,
                    column: f.span_col,
                })
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Tier 1 new security rules
// ---------------------------------------------------------------------------

pub struct MissingTableId;
impl Rule for MissingTableId {
    fn id(&self) -> &'static str {
        "security/missing-table-id"
    }
    fn category(&self) -> Category {
        Category::Security
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis
            .generic_id_validators
            .iter()
            .map(|loc| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Warning,
                category: self.category(),
                message: format!(
                    "Argument validator uses `v.id()` without explicit table: {}",
                    loc.detail
                ),
                help: "Use `v.id(\"tableName\")` to prevent cross-table ID confusion. Matches the ESLint `explicit-table-ids` rule.".to_string(),
                file: analysis.file_path.clone(),
                line: loc.line,
                column: loc.col,
            })
            .collect()
    }
}

pub struct MissingHttpAuth;
impl Rule for MissingHttpAuth {
    fn id(&self) -> &'static str {
        "security/missing-http-auth"
    }
    fn category(&self) -> Category {
        Category::Security
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis
            .functions
            .iter()
            .filter(|f| {
                f.kind == FunctionKind::HttpAction
                    && !f.has_auth_check
                    && !f.has_internal_secret
                    && !f.is_intentionally_public
            })
            .map(|f| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Error,
                category: self.category(),
                message: format!(
                    "httpAction `{}` does not check authentication",
                    f.name
                ),
                help: "HTTP actions are publicly accessible. Add `ctx.auth.getUserIdentity()` or check the Authorization header.".to_string(),
                file: analysis.file_path.clone(),
                line: f.span_line,
                column: f.span_col,
            })
            .collect()
    }
}

pub struct ConditionalFunctionExport;
impl Rule for ConditionalFunctionExport {
    fn id(&self) -> &'static str {
        "security/conditional-function-export"
    }
    fn category(&self) -> Category {
        Category::Security
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis
            .conditional_exports
            .iter()
            .map(|loc| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Error,
                category: self.category(),
                message: "Conditional function export based on environment variable".to_string(),
                help: "Do not condition Convex function exports on environment variables. This can cause inconsistent behavior between deployments.".to_string(),
                file: analysis.file_path.clone(),
                line: loc.line,
                column: loc.col,
            })
            .collect()
    }
}

pub struct GenericMutationArgs;
impl Rule for GenericMutationArgs {
    fn id(&self) -> &'static str {
        "security/generic-mutation-args"
    }
    fn category(&self) -> Category {
        Category::Security
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis
            .functions
            .iter()
            .filter(|f| f.is_public() && f.has_any_validator_in_args)
            .map(|f| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Warning,
                category: self.category(),
                message: format!(
                    "Public {} `{}` uses `v.any()` in argument validators",
                    f.kind_str(),
                    f.name
                ),
                help: "Using `v.any()` defeats the purpose of validation. Use specific validators for type safety and security.".to_string(),
                file: analysis.file_path.clone(),
                line: f.span_line,
                column: f.span_col,
            })
            .collect()
    }
}

pub struct OverlyBroadPatch;
impl Rule for OverlyBroadPatch {
    fn id(&self) -> &'static str {
        "security/overly-broad-patch"
    }
    fn category(&self) -> Category {
        Category::Security
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis
            .raw_arg_patches
            .iter()
            .map(|loc| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Warning,
                category: self.category(),
                message: loc.detail.clone(),
                help: "Passing raw client args to `ctx.db.patch` is a mass-assignment vulnerability. Destructure and pass only the allowed fields.".to_string(),
                file: analysis.file_path.clone(),
                line: loc.line,
                column: loc.col,
            })
            .collect()
    }
}

pub struct HttpMissingCors;
impl Rule for HttpMissingCors {
    fn id(&self) -> &'static str {
        "security/http-missing-cors"
    }
    fn category(&self) -> Category {
        Category::Security
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        // Group routes by path
        let mut routes_by_path: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for route in &analysis.http_routes {
            if route.is_webhook {
                continue;
            }
            routes_by_path
                .entry(route.path.as_str())
                .or_default()
                .push(route.method.as_str());
        }

        let actionable_methods = ["GET", "POST", "PUT", "DELETE", "PATCH"];

        let mut diagnostics = Vec::new();
        for (path, methods) in &routes_by_path {
            let has_options = methods.iter().any(|m| m.eq_ignore_ascii_case("OPTIONS"));
            let has_actionable = methods
                .iter()
                .any(|m| actionable_methods.iter().any(|a| m.eq_ignore_ascii_case(a)));
            if has_actionable && !has_options {
                // Find the line of the first route for this path
                let line = analysis
                    .http_routes
                    .iter()
                    .find(|r| r.path.as_str() == *path)
                    .map(|r| r.line)
                    .unwrap_or(0);
                diagnostics.push(Diagnostic {
                    rule: self.id().to_string(),
                    severity: Severity::Warning,
                    category: self.category(),
                    message: format!(
                        "HTTP route `{}` has no OPTIONS handler for CORS",
                        path
                    ),
                    help: "Add an OPTIONS handler to support CORS preflight requests. See the Convex CORS guide.".to_string(),
                    file: analysis.file_path.clone(),
                    line,
                    column: 0,
                });
            }
        }
        diagnostics
    }
}
