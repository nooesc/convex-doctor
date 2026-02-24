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
