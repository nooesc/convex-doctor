pub mod architecture;
pub mod context;
pub mod correctness;
pub mod performance;
pub mod security;

use crate::diagnostic::{Category, Diagnostic};

#[derive(Debug, Default)]
pub struct FileAnalysis {
    pub file_path: String,
    pub has_use_node: bool,
    pub functions: Vec<ConvexFunction>,
    pub imports: Vec<ImportInfo>,
    pub ctx_calls: Vec<CtxCall>,
    pub collect_calls: Vec<CallLocation>,
    pub filter_calls: Vec<CallLocation>,
    pub date_now_calls: Vec<CallLocation>,
    pub loop_ctx_calls: Vec<CallLocation>,
    pub deprecated_calls: Vec<DeprecatedCall>,
    pub hardcoded_secrets: Vec<CallLocation>,
    pub old_syntax_functions: Vec<CallLocation>,
    pub exported_function_count: u32,
    pub schema_nesting_depth: u32,
    pub schema_array_id_fields: Vec<CallLocation>,
    pub index_definitions: Vec<IndexDef>,
}

#[derive(Debug, Clone)]
pub struct ConvexFunction {
    pub name: String,
    pub kind: FunctionKind,
    pub has_args_validator: bool,
    pub has_return_validator: bool,
    pub has_auth_check: bool,
    pub handler_line_count: u32,
    pub span_line: u32,
    pub span_col: u32,
}

impl ConvexFunction {
    pub fn is_public(&self) -> bool {
        matches!(
            self.kind,
            FunctionKind::Query
                | FunctionKind::Mutation
                | FunctionKind::Action
                | FunctionKind::HttpAction
        )
    }

    pub fn kind_str(&self) -> &'static str {
        match self.kind {
            FunctionKind::Query => "query",
            FunctionKind::Mutation => "mutation",
            FunctionKind::Action => "action",
            FunctionKind::HttpAction => "httpAction",
            FunctionKind::InternalQuery => "internalQuery",
            FunctionKind::InternalMutation => "internalMutation",
            FunctionKind::InternalAction => "internalAction",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FunctionKind {
    Query,
    Mutation,
    Action,
    HttpAction,
    InternalQuery,
    InternalMutation,
    InternalAction,
}

impl FunctionKind {
    pub fn from_callee(s: &str) -> Option<Self> {
        match s {
            "query" => Some(FunctionKind::Query),
            "mutation" => Some(FunctionKind::Mutation),
            "action" => Some(FunctionKind::Action),
            "httpAction" => Some(FunctionKind::HttpAction),
            "internalQuery" => Some(FunctionKind::InternalQuery),
            "internalMutation" => Some(FunctionKind::InternalMutation),
            "internalAction" => Some(FunctionKind::InternalAction),
            _ => None,
        }
    }

    pub fn is_action(&self) -> bool {
        matches!(self, FunctionKind::Action | FunctionKind::InternalAction)
    }

    pub fn is_query(&self) -> bool {
        matches!(self, FunctionKind::Query | FunctionKind::InternalQuery)
    }

    pub fn is_mutation(&self) -> bool {
        matches!(
            self,
            FunctionKind::Mutation | FunctionKind::InternalMutation
        )
    }
}

#[derive(Debug, Clone)]
pub struct ImportInfo {
    pub source: String,
    pub specifiers: Vec<String>,
    pub line: u32,
}

#[derive(Debug, Clone)]
pub struct CtxCall {
    pub chain: String,
    pub line: u32,
    pub col: u32,
    pub in_loop: bool,
    pub is_awaited: bool,
    pub enclosing_function_kind: Option<FunctionKind>,
    pub first_arg_chain: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CallLocation {
    pub line: u32,
    pub col: u32,
    pub detail: String,
}

#[derive(Debug, Clone)]
pub struct DeprecatedCall {
    pub name: String,
    pub replacement: String,
    pub line: u32,
    pub col: u32,
}

#[derive(Debug, Clone)]
pub struct IndexDef {
    pub table: String,
    pub name: String,
    pub fields: Vec<String>,
    pub line: u32,
}

pub trait Rule: Send + Sync {
    fn id(&self) -> &'static str;
    fn category(&self) -> Category;
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic>;
}

pub struct RuleRegistry {
    rules: Vec<Box<dyn Rule>>,
}

impl Default for RuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl RuleRegistry {
    pub fn new() -> Self {
        let rules: Vec<Box<dyn Rule>> = vec![
            Box::new(security::MissingArgValidators),
            Box::new(security::MissingReturnValidators),
            Box::new(security::MissingAuthCheck),
            Box::new(security::InternalApiMisuse),
            Box::new(security::HardcodedSecrets),
            Box::new(performance::UnboundedCollect),
            Box::new(performance::FilterWithoutIndex),
            Box::new(performance::DateNowInQuery),
            Box::new(performance::LoopRunMutation),
            Box::new(correctness::UnwaitedPromise),
            Box::new(correctness::OldFunctionSyntax),
            Box::new(correctness::DbInAction),
            Box::new(correctness::DeprecatedApi),
            Box::new(architecture::LargeHandler),
            Box::new(architecture::MonolithicFile),
        ];

        RuleRegistry { rules }
    }

    pub fn rules(&self) -> &[Box<dyn Rule>] {
        &self.rules
    }

    pub fn run(&self, analysis: &FileAnalysis, enabled: &dyn Fn(&str) -> bool) -> Vec<Diagnostic> {
        self.rules
            .iter()
            .filter(|r| enabled(r.id()))
            .flat_map(|r| r.check(analysis))
            .collect()
    }
}
