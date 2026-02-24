pub mod architecture;
pub mod client;
pub mod configuration;
pub mod context;
pub mod correctness;
pub mod performance;
pub mod schema;
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
    pub first_calls: Vec<CallLocation>,
    pub awaited_identifiers: Vec<String>,
    pub cron_api_refs: Vec<CallLocation>,
    pub generic_id_validators: Vec<CallLocation>,
    pub conditional_exports: Vec<CallLocation>,
    pub non_deterministic_calls: Vec<CallLocation>,
    pub throw_generic_errors: Vec<CallLocation>,
    pub raw_arg_patches: Vec<CallLocation>,
    pub http_routes: Vec<HttpRoute>,
    pub schema_id_fields: Vec<SchemaIdField>,
    pub collect_variable_filters: Vec<CallLocation>,
    pub filter_field_names: Vec<FilterField>,
    pub search_index_definitions: Vec<SearchIndexDef>,
    pub large_writes: Vec<CallLocation>,
    pub optional_schema_fields: Vec<CallLocation>,
    pub unexported_function_count: u32,
    pub convex_hook_calls: Vec<ConvexHookCall>,
    pub has_convex_provider: bool,
}

#[derive(Debug, Clone)]
pub struct ConvexFunction {
    pub name: String,
    pub kind: FunctionKind,
    pub has_args_validator: bool,
    pub has_any_validator_in_args: bool,
    pub arg_names: Vec<String>,
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
    pub is_returned: bool,
    pub assigned_to: Option<String>,
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

#[derive(Debug, Clone, Default)]
pub struct HttpRoute {
    pub method: String,
    pub path: String,
    pub line: u32,
}

#[derive(Debug, Clone, Default)]
pub struct SchemaIdField {
    pub field_name: String,
    pub table_ref: String,
    pub line: u32,
    pub col: u32,
}

#[derive(Debug, Clone, Default)]
pub struct FilterField {
    pub field_name: String,
    pub line: u32,
    pub col: u32,
}

#[derive(Debug, Clone, Default)]
pub struct SearchIndexDef {
    pub table: String,
    pub name: String,
    pub has_filter_fields: bool,
    pub line: u32,
}

#[derive(Debug, Clone, Default)]
pub struct ConvexHookCall {
    pub hook_name: String,
    pub line: u32,
    pub col: u32,
    pub in_render_body: bool,
}

#[derive(Debug, Default)]
pub struct ProjectContext {
    pub has_schema: bool,
    pub has_auth_config: bool,
    pub has_convex_json: bool,
    pub has_env_local: bool,
    pub env_gitignored: bool,
    pub uses_auth: bool,
    pub has_generated_dir: bool,
    pub has_tsconfig: bool,
    pub node_version_from_config: Option<String>,
    pub generated_files_modified: bool,
    pub all_index_definitions: Vec<IndexDef>,
    pub all_schema_id_fields: Vec<SchemaIdField>,
}

pub trait Rule: Send + Sync {
    fn id(&self) -> &'static str;
    fn category(&self) -> Category;
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic>;
    /// Project-level check, called once after all files are analyzed.
    /// Default returns empty.
    fn check_project(&self, _ctx: &ProjectContext) -> Vec<Diagnostic> {
        vec![]
    }
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
            // Security (13)
            Box::new(security::MissingArgValidators),
            Box::new(security::MissingReturnValidators),
            Box::new(security::MissingAuthCheck),
            Box::new(security::InternalApiMisuse),
            Box::new(security::HardcodedSecrets),
            Box::new(security::EnvNotGitignored),
            Box::new(security::SpoofableAccessControl),
            Box::new(security::MissingTableId),
            Box::new(security::MissingHttpAuth),
            Box::new(security::ConditionalFunctionExport),
            Box::new(security::GenericMutationArgs),
            Box::new(security::OverlyBroadPatch),
            Box::new(security::HttpMissingCors),
            // Performance (12)
            Box::new(performance::UnboundedCollect),
            Box::new(performance::FilterWithoutIndex),
            Box::new(performance::DateNowInQuery),
            Box::new(performance::LoopRunMutation),
            Box::new(performance::SequentialRunCalls),
            Box::new(performance::UnnecessaryRunAction),
            Box::new(performance::HelperVsRun),
            Box::new(performance::MissingIndexOnForeignKey),
            Box::new(performance::ActionFromClient),
            Box::new(performance::CollectThenFilter),
            Box::new(performance::LargeDocumentWrite),
            Box::new(performance::NoPaginationForList),
            // Correctness (15)
            Box::new(correctness::UnwaitedPromise),
            Box::new(correctness::OldFunctionSyntax),
            Box::new(correctness::DbInAction),
            Box::new(correctness::DeprecatedApi),
            Box::new(correctness::WrongRuntimeImport),
            Box::new(correctness::DirectFunctionRef),
            Box::new(correctness::MissingUnique),
            Box::new(correctness::QuerySideEffect),
            Box::new(correctness::MutationInQuery),
            Box::new(correctness::CronUsesPublicApi),
            Box::new(correctness::NodeQueryMutation),
            Box::new(correctness::SchedulerReturnIgnored),
            Box::new(correctness::NonDeterministicInQuery),
            Box::new(correctness::ReplaceVsPatch),
            Box::new(correctness::GeneratedCodeModified),
            // Schema (8)
            Box::new(schema::MissingSchema),
            Box::new(schema::DeepNesting),
            Box::new(schema::ArrayRelationships),
            Box::new(schema::RedundantIndex),
            Box::new(schema::TooManyIndexes),
            Box::new(schema::MissingSearchIndexFilter),
            Box::new(schema::OptionalFieldNoDefaultHandling),
            Box::new(schema::MissingIndexForQuery),
            // Architecture (8)
            Box::new(architecture::LargeHandler),
            Box::new(architecture::MonolithicFile),
            Box::new(architecture::DuplicatedAuth),
            Box::new(architecture::ActionWithoutScheduling),
            Box::new(architecture::NoConvexError),
            Box::new(architecture::MixedFunctionTypes),
            Box::new(architecture::NoHelperFunctions),
            Box::new(architecture::DeepFunctionChain),
            // Configuration (5)
            Box::new(configuration::MissingConvexJson),
            Box::new(configuration::MissingAuthConfig),
            Box::new(configuration::MissingGeneratedCode),
            Box::new(configuration::OutdatedNodeVersion),
            Box::new(configuration::MissingTsconfig),
            // Client-Side (4)
            Box::new(client::MutationInRender),
            Box::new(client::UnhandledLoadingState),
            Box::new(client::ActionInsteadOfMutation),
            Box::new(client::MissingConvexProvider),
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
