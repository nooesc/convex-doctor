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
                help: "Create convex.json to configure your Convex deployment settings."
                    .to_string(),
                file: ".".to_string(),
                line: 0,
                column: 0,
            }]
        } else {
            vec![]
        }
    }
}

/// Project-level rule: warn when convex/_generated/ directory is missing.
pub struct MissingGeneratedCode;
impl Rule for MissingGeneratedCode {
    fn id(&self) -> &'static str {
        "config/missing-generated-code"
    }
    fn category(&self) -> Category {
        Category::Configuration
    }
    fn check(&self, _analysis: &FileAnalysis) -> Vec<Diagnostic> {
        vec![]
    }
    fn check_project(&self, ctx: &ProjectContext) -> Vec<Diagnostic> {
        if !ctx.has_generated_dir {
            vec![Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Warning,
                category: self.category(),
                message: "Missing convex/_generated/ directory".to_string(),
                help: "Run `npx convex dev` to generate type-safe API references. Consider checking in generated code per Convex recommendations.".to_string(),
                file: "convex/".to_string(),
                line: 0,
                column: 0,
            }]
        } else {
            vec![]
        }
    }
}

/// Project-level rule: warn when convex.json specifies an outdated Node version.
pub struct OutdatedNodeVersion;
impl Rule for OutdatedNodeVersion {
    fn id(&self) -> &'static str {
        "config/outdated-node-version"
    }
    fn category(&self) -> Category {
        Category::Configuration
    }
    fn check(&self, _analysis: &FileAnalysis) -> Vec<Diagnostic> {
        vec![]
    }
    fn check_project(&self, ctx: &ProjectContext) -> Vec<Diagnostic> {
        if let Some(ref version_str) = ctx.node_version_from_config {
            if let Ok(version) = version_str.parse::<u32>() {
                if version <= 18 {
                    return vec![Diagnostic {
                        rule: self.id().to_string(),
                        severity: Severity::Warning,
                        category: self.category(),
                        message: format!(
                            "convex.json specifies Node {} which is no longer supported",
                            version
                        ),
                        help: "Update to Node 20 or later in convex.json for continued support."
                            .to_string(),
                        file: "convex.json".to_string(),
                        line: 0,
                        column: 0,
                    }];
                }
            }
        }
        vec![]
    }
}

/// Project-level rule: info when tsconfig.json is missing but schema exists.
pub struct MissingTsconfig;
impl Rule for MissingTsconfig {
    fn id(&self) -> &'static str {
        "config/missing-tsconfig"
    }
    fn category(&self) -> Category {
        Category::Configuration
    }
    fn check(&self, _analysis: &FileAnalysis) -> Vec<Diagnostic> {
        vec![]
    }
    fn check_project(&self, ctx: &ProjectContext) -> Vec<Diagnostic> {
        if ctx.has_schema && !ctx.has_tsconfig {
            vec![Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Info,
                category: self.category(),
                message: "No tsconfig.json found in convex/ directory".to_string(),
                help: "Create convex/tsconfig.json for proper TypeScript type-checking during `npx convex dev`.".to_string(),
                file: "convex/".to_string(),
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
