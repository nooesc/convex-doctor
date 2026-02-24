use crate::diagnostic::{Category, Diagnostic};
use crate::rules::{FileAnalysis, Rule};

pub struct UnboundedCollect;
impl Rule for UnboundedCollect {
    fn id(&self) -> &'static str {
        "performance/unbounded-collect"
    }
    fn category(&self) -> Category {
        Category::Performance
    }
    fn check(&self, _analysis: &FileAnalysis) -> Vec<Diagnostic> {
        vec![]
    }
}

pub struct FilterWithoutIndex;
impl Rule for FilterWithoutIndex {
    fn id(&self) -> &'static str {
        "performance/filter-without-index"
    }
    fn category(&self) -> Category {
        Category::Performance
    }
    fn check(&self, _analysis: &FileAnalysis) -> Vec<Diagnostic> {
        vec![]
    }
}

pub struct DateNowInQuery;
impl Rule for DateNowInQuery {
    fn id(&self) -> &'static str {
        "performance/date-now-in-query"
    }
    fn category(&self) -> Category {
        Category::Performance
    }
    fn check(&self, _analysis: &FileAnalysis) -> Vec<Diagnostic> {
        vec![]
    }
}

pub struct LoopRunMutation;
impl Rule for LoopRunMutation {
    fn id(&self) -> &'static str {
        "performance/loop-run-mutation"
    }
    fn category(&self) -> Category {
        Category::Performance
    }
    fn check(&self, _analysis: &FileAnalysis) -> Vec<Diagnostic> {
        vec![]
    }
}
