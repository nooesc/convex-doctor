use crate::diagnostic::{Category, Diagnostic, Severity};
use crate::rules::{FileAnalysis, ProjectContext, Rule};

/// Project-level rule: error when convex.json is missing from project root.
pub struct MissingConvexJson;
impl Rule for MissingConvexJson {
    fn id(&self) -> &'static str {
        "config/missing-convex-json"
    }
    fn category(&self) -> Category {
        Category::Configuration
    }
    fn check(&self, _analysis: &FileAnalysis) -> Vec<Diagnostic> {
        vec![]
    }
    fn check_project(&self, ctx: &ProjectContext) -> Vec<Diagnostic> {
        if !ctx.has_convex_json {
            vec![Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Warning,
                category: self.category(),
                message: "No convex.json found in project root".to_string(),
                help: "Create convex.json to configure your Convex deployment settings.".to_string(),
                file: ".".to_string(),
                line: 0,
                column: 0,
            }]
        } else {
            vec![]
        }
    }
}

/// Project-level rule: error when functions use auth but no auth.config.ts exists.
pub struct MissingAuthConfig;
impl Rule for MissingAuthConfig {
    fn id(&self) -> &'static str {
        "config/missing-auth-config"
    }
    fn category(&self) -> Category {
        Category::Configuration
    }
    fn check(&self, _analysis: &FileAnalysis) -> Vec<Diagnostic> {
        vec![]
    }
    fn check_project(&self, ctx: &ProjectContext) -> Vec<Diagnostic> {
        if ctx.uses_auth && !ctx.has_auth_config {
            vec![Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Error,
                category: self.category(),
                message: "Functions use ctx.auth but no auth.config.ts found".to_string(),
                help: "Create convex/auth.config.ts to configure authentication providers."
                    .to_string(),
                file: "convex/".to_string(),
                line: 0,
                column: 0,
            }]
        } else {
            vec![]
        }
    }
}
