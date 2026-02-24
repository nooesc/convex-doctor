use crate::diagnostic::{Category, Diagnostic};
use crate::rules::{FileAnalysis, Rule};

pub struct MissingArgValidators;
impl Rule for MissingArgValidators {
    fn id(&self) -> &'static str {
        "security/missing-arg-validators"
    }
    fn category(&self) -> Category {
        Category::Security
    }
    fn check(&self, _analysis: &FileAnalysis) -> Vec<Diagnostic> {
        vec![]
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
    fn check(&self, _analysis: &FileAnalysis) -> Vec<Diagnostic> {
        vec![]
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
    fn check(&self, _analysis: &FileAnalysis) -> Vec<Diagnostic> {
        vec![]
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
    fn check(&self, _analysis: &FileAnalysis) -> Vec<Diagnostic> {
        vec![]
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
    fn check(&self, _analysis: &FileAnalysis) -> Vec<Diagnostic> {
        vec![]
    }
}
