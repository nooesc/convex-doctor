use crate::diagnostic::{Category, Diagnostic, Severity};
use crate::rules::{FileAnalysis, Rule};

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
