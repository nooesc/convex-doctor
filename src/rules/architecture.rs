use crate::diagnostic::{Category, Diagnostic};
use crate::rules::{FileAnalysis, Rule};

pub struct LargeHandler;
impl Rule for LargeHandler {
    fn id(&self) -> &'static str {
        "architecture/large-handler"
    }
    fn category(&self) -> Category {
        Category::Architecture
    }
    fn check(&self, _analysis: &FileAnalysis) -> Vec<Diagnostic> {
        vec![]
    }
}

pub struct MonolithicFile;
impl Rule for MonolithicFile {
    fn id(&self) -> &'static str {
        "architecture/monolithic-file"
    }
    fn category(&self) -> Category {
        Category::Architecture
    }
    fn check(&self, _analysis: &FileAnalysis) -> Vec<Diagnostic> {
        vec![]
    }
}
