use serde::Serialize;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    pub rule: String,
    pub severity: Severity,
    pub category: Category,
    pub message: String,
    pub help: String,
    pub file: String,
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Error => write!(f, "error"),
            Severity::Warning => write!(f, "warning"),
            Severity::Info => write!(f, "info"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum Category {
    Security,
    Performance,
    Correctness,
    Schema,
    Architecture,
    Configuration,
    ClientSide,
}

impl Category {
    pub fn weight(&self) -> f64 {
        match self {
            Category::Security => 1.5,
            Category::Performance => 1.2,
            Category::Correctness => 1.5,
            Category::Schema => 1.0,
            Category::Architecture => 0.8,
            Category::Configuration => 1.0,
            Category::ClientSide => 1.0,
        }
    }
}

impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Category::Security => write!(f, "Security"),
            Category::Performance => write!(f, "Performance"),
            Category::Correctness => write!(f, "Correctness"),
            Category::Schema => write!(f, "Schema"),
            Category::Architecture => write!(f, "Architecture"),
            Category::Configuration => write!(f, "Configuration"),
            Category::ClientSide => write!(f, "Client-Side"),
        }
    }
}
