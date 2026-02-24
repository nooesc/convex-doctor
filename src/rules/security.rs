use crate::diagnostic::{Category, Diagnostic, Severity};
use crate::rules::{FileAnalysis, Rule};

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
            .filter(|f| f.is_public() && !f.has_args_validator)
            .map(|f| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Error,
                category: self.category(),
                message: format!(
                    "Public {} `{}` has no argument validators",
                    f.kind_str(),
                    f.name
                ),
                help: "Add `args: { ... }` with validators for all parameters. Public functions can be called by anyone.".to_string(),
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
            .filter(|f| !f.has_return_validator)
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
        analysis
            .functions
            .iter()
            .filter(|f| f.is_public() && !f.has_auth_check)
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
                chain_matches && arg_is_public_api
            })
            .map(|call| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Warning,
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
