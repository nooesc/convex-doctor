use std::path::Path;

use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk, Visit};
use oxc_parser::{ParseOptions, Parser};
use oxc_span::{GetSpan, SourceType};

use super::{
    CallLocation, ConvexFunction, CtxCall, DeprecatedCall, FileAnalysis, FunctionKind, ImportInfo,
    IndexDef,
};

/// Analyze a TypeScript/JavaScript file for Convex-specific patterns.
pub fn analyze_file(path: &Path) -> Result<FileAnalysis, String> {
    let source_text = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
    let source_type = SourceType::from_path(path)
        .map_err(|_| format!("Unknown file type: {}", path.display()))?;

    let allocator = Allocator::default();
    let ret = Parser::new(&allocator, &source_text, source_type)
        .with_options(ParseOptions {
            parse_regular_expression: true,
            ..ParseOptions::default()
        })
        .parse();

    if ret.panicked {
        return Err(format!("Parser panicked on {}", path.display()));
    }

    let mut visitor = ConvexVisitor::new(path, &source_text);
    visitor.visit_program(&ret.program);
    Ok(visitor.into_analysis())
}

/// Builder for accumulating function properties during AST traversal.
#[derive(Debug, Default)]
struct FunctionBuilder {
    name: String,
    kind: Option<FunctionKind>,
    has_args_validator: bool,
    has_any_validator_in_args: bool,
    arg_names: Vec<String>,
    has_return_validator: bool,
    has_auth_check: bool,
    handler_line_count: u32,
    span_line: u32,
    span_col: u32,
}

impl FunctionBuilder {
    fn build(self) -> ConvexFunction {
        ConvexFunction {
            name: self.name,
            kind: self.kind.unwrap_or(FunctionKind::Query),
            has_args_validator: self.has_args_validator,
            has_any_validator_in_args: self.has_any_validator_in_args,
            arg_names: self.arg_names,
            has_return_validator: self.has_return_validator,
            has_auth_check: self.has_auth_check,
            handler_line_count: self.handler_line_count,
            span_line: self.span_line,
            span_col: self.span_col,
        }
    }
}

/// Visitor that walks the AST to extract Convex-specific patterns.
struct ConvexVisitor<'a> {
    source_text: &'a str,
    analysis: FileAnalysis,
    loop_depth: u32,
    in_await: bool,
    in_return: bool,
    current_assignment_target: Option<String>,
    current_export_names: Vec<String>,
    next_export_index: usize,
    current_function_kind: Option<FunctionKind>,
    function_builder_stack: Vec<FunctionBuilder>,
    validator_nesting_depth: u32,
    max_validator_nesting_depth: u32,
}

impl<'a> ConvexVisitor<'a> {
    fn new(path: &Path, source_text: &'a str) -> Self {
        Self {
            source_text,
            analysis: FileAnalysis {
                file_path: path.display().to_string(),
                ..Default::default()
            },
            loop_depth: 0,
            in_await: false,
            in_return: false,
            current_assignment_target: None,
            current_export_names: vec![],
            next_export_index: 0,
            current_function_kind: None,
            function_builder_stack: vec![],
            validator_nesting_depth: 0,
            max_validator_nesting_depth: 0,
        }
    }

    fn into_analysis(mut self) -> FileAnalysis {
        self.analysis.schema_nesting_depth = self.max_validator_nesting_depth;
        self.analysis
    }

    fn next_export_name(&mut self) -> Option<String> {
        let name = self
            .current_export_names
            .get(self.next_export_index)
            .cloned();
        if name.is_some() {
            self.next_export_index += 1;
        }
        name
    }

    fn current_builder_mut(&mut self) -> Option<&mut FunctionBuilder> {
        self.function_builder_stack.last_mut()
    }

    /// Compute line and column (1-based) from a byte offset in the source text.
    fn line_col(&self, offset: u32) -> (u32, u32) {
        let offset = offset as usize;
        let slice = &self.source_text[..offset.min(self.source_text.len())];
        let line = slice.matches('\n').count() as u32 + 1;
        let col = match slice.rfind('\n') {
            Some(pos) => (offset - pos) as u32,
            None => offset as u32 + 1,
        };
        (line, col)
    }

    /// Try to resolve a member expression chain into a dotted string like "ctx.db.query".
    fn resolve_member_chain(expr: &Expression<'_>) -> Option<String> {
        match expr {
            Expression::Identifier(ident) => Some(ident.name.as_str().to_string()),
            Expression::StaticMemberExpression(mem) => {
                let obj = Self::resolve_member_chain(&mem.object)?;
                Some(format!("{}.{}", obj, mem.property.name.as_str()))
            }
            Expression::CallExpression(call) => Self::resolve_member_chain(&call.callee),
            _ => None,
        }
    }

    /// Check if a callee expression represents a Convex function constructor (query, mutation, etc.).
    fn get_function_kind(callee: &Expression<'_>) -> Option<FunctionKind> {
        match callee {
            Expression::Identifier(ident) => FunctionKind::from_callee(ident.name.as_str()),
            _ => None,
        }
    }

    /// Find the base defineTable(...) call start offset for a chained .index(...) expression.
    fn find_define_table_call_start(expr: &Expression<'_>) -> Option<u32> {
        match expr {
            Expression::CallExpression(call) => match &call.callee {
                Expression::Identifier(ident) if ident.name.as_str() == "defineTable" => {
                    Some(call.span.start)
                }
                Expression::StaticMemberExpression(mem) => {
                    Self::find_define_table_call_start(&mem.object)
                }
                _ => Self::find_define_table_call_start(&call.callee),
            },
            Expression::StaticMemberExpression(mem) => {
                Self::find_define_table_call_start(&mem.object)
            }
            _ => None,
        }
    }

    /// Resolve a stable table identity for .index(...) calls.
    fn get_index_table_id(callee: &Expression<'_>) -> Option<String> {
        let Expression::StaticMemberExpression(mem) = callee else {
            return None;
        };
        if mem.property.name.as_str() != "index" {
            return None;
        }
        Self::find_define_table_call_start(&mem.object).map(|start| format!("table@{start}"))
    }

    /// Check if an expression chain contains "ctx.auth" at any point.
    fn contains_ctx_auth(expr: &Expression<'_>) -> bool {
        match expr {
            Expression::StaticMemberExpression(mem) => {
                // Check if this is ctx.auth
                if let Expression::Identifier(ident) = &mem.object {
                    if ident.name.as_str() == "ctx" && mem.property.name.as_str() == "auth" {
                        return true;
                    }
                }
                // Check deeper in the chain
                Self::contains_ctx_auth(&mem.object)
            }
            Expression::CallExpression(call) => Self::contains_ctx_auth(&call.callee),
            _ => false,
        }
    }

    /// Check if a call expression is a ctx.* call.
    fn is_ctx_call(callee: &Expression<'_>) -> bool {
        if let Some(chain) = Self::resolve_member_chain(callee) {
            return chain.starts_with("ctx.");
        }
        false
    }
}

impl<'a> Visit<'a> for ConvexVisitor<'a> {
    fn visit_directive(&mut self, it: &Directive<'a>) {
        if it.directive.as_str() == "use node" {
            self.analysis.has_use_node = true;
        }
        walk::walk_directive(self, it);
    }

    fn visit_import_declaration(&mut self, it: &ImportDeclaration<'a>) {
        let source = it.source.value.as_str().to_string();
        let mut specifiers = Vec::new();

        if let Some(specs) = &it.specifiers {
            for spec in specs {
                match spec {
                    ImportDeclarationSpecifier::ImportSpecifier(s) => {
                        specifiers.push(s.local.name.as_str().to_string());
                    }
                    ImportDeclarationSpecifier::ImportDefaultSpecifier(s) => {
                        specifiers.push(s.local.name.as_str().to_string());
                    }
                    ImportDeclarationSpecifier::ImportNamespaceSpecifier(s) => {
                        specifiers.push(s.local.name.as_str().to_string());
                    }
                }
            }
        }

        let (line, _) = self.line_col(it.span.start);
        self.analysis.imports.push(ImportInfo {
            source,
            specifiers,
            line,
        });

        walk::walk_import_declaration(self, it);
    }

    fn visit_export_named_declaration(&mut self, it: &ExportNamedDeclaration<'a>) {
        let mut export_names = vec![];

        // Extract exported names from `export const foo = ..., bar = ...`
        if let Some(Declaration::VariableDeclaration(var_decl)) = &it.declaration {
            for declarator in &var_decl.declarations {
                if let BindingPattern::BindingIdentifier(ident) = &declarator.id {
                    export_names.push(ident.name.as_str().to_string());
                }
            }
        }

        self.current_export_names = export_names;
        self.next_export_index = 0;
        walk::walk_export_named_declaration(self, it);
        self.current_export_names.clear();
        self.next_export_index = 0;
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        let (line, col) = self.line_col(it.span.start);
        let prev_function_kind = self.current_function_kind.clone();
        let mut started_exported_function = false;

        // Check if this is a Convex function definition: query({...}), mutation({...}), etc.
        if let Some(kind) = Self::get_function_kind(&it.callee) {
            if let Some(export_name) = self.next_export_name() {
                let mut builder = FunctionBuilder {
                    name: export_name.clone(),
                    kind: Some(kind.clone()),
                    span_line: line,
                    span_col: col,
                    ..Default::default()
                };

                // Check the first argument for the config object
                if let Some(Argument::ObjectExpression(obj)) = it.arguments.first() {
                    // Inspect config properties before walking
                    for prop in &obj.properties {
                        if let ObjectPropertyKind::ObjectProperty(prop) = prop {
                            if let Some(name) = prop.key.static_name() {
                                match name.as_ref() {
                                    "args" => {
                                        builder.has_args_validator = true;
                                        if let Expression::ObjectExpression(args_obj) = &prop.value
                                        {
                                            for arg_prop in &args_obj.properties {
                                                if let ObjectPropertyKind::ObjectProperty(arg) =
                                                    arg_prop
                                                {
                                                    if let Some(arg_name) = arg.key.static_name() {
                                                        builder
                                                            .arg_names
                                                            .push(arg_name.to_string());
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    "returns" => builder.has_return_validator = true,
                                    "handler" => {
                                        let handler_start_line =
                                            self.line_col(prop.value.span().start).0;
                                        let handler_end_line =
                                            self.line_col(prop.value.span().end).0;
                                        builder.handler_line_count =
                                            handler_end_line.saturating_sub(handler_start_line) + 1;
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                } else if !it.arguments.is_empty() {
                    // Old function syntax: direct function arg instead of config object
                    // e.g., query(async (ctx) => ...) instead of query({ handler: ... })
                    self.analysis
                        .old_syntax_functions
                        .push(super::CallLocation {
                            line,
                            col,
                            detail: format!("{}() using old function syntax", export_name),
                        });
                }

                self.function_builder_stack.push(builder);
                self.current_function_kind = Some(kind);
                started_exported_function = true;
            }
        }

        // Detect .collect() and .filter() calls — only on ctx.db query chains
        if let Expression::StaticMemberExpression(mem) = &it.callee {
            let prop_name = mem.property.name.as_str();
            if prop_name == "collect" {
                let full_chain = Self::resolve_member_chain(&it.callee).unwrap_or_default();
                if full_chain.contains("ctx.db") {
                    self.analysis.collect_calls.push(CallLocation {
                        line,
                        col,
                        detail: "collect()".to_string(),
                    });
                }
            }

            if prop_name == "filter" {
                let full_chain = Self::resolve_member_chain(&it.callee).unwrap_or_default();
                if full_chain.contains("ctx.db") {
                    self.analysis.filter_calls.push(CallLocation {
                        line,
                        col,
                        detail: "filter()".to_string(),
                    });
                }
            }
        }

        // Detect Date.now() calls — only inside query functions
        if let Expression::StaticMemberExpression(mem) = &it.callee {
            if mem.property.name.as_str() == "now" {
                if let Expression::Identifier(ident) = &mem.object {
                    if ident.name.as_str() == "Date"
                        && self
                            .current_function_kind
                            .as_ref()
                            .is_some_and(|k| k.is_query())
                    {
                        self.analysis.date_now_calls.push(CallLocation {
                            line,
                            col,
                            detail: "Date.now()".to_string(),
                        });
                    }
                }
            }
        }

        // Detect deprecated API calls (e.g., v.bigint(), v.bytes(), v.any())
        if let Expression::StaticMemberExpression(mem) = &it.callee {
            if let Expression::Identifier(ident) = &mem.object {
                if ident.name.as_str() == "v" {
                    let prop = mem.property.name.as_str();
                    let deprecated = match prop {
                        "bigint" => Some(("v.bigint()", "Use v.int64() instead")),
                        "bytes" => {
                            Some(("v.bytes()", "Use v.string() with base64 encoding instead"))
                        }
                        "any" => Some((
                            "v.any()",
                            "Use a specific validator type for better type safety",
                        )),
                        _ => None,
                    };
                    if let Some((name, replacement)) = deprecated {
                        self.analysis.deprecated_calls.push(DeprecatedCall {
                            name: name.to_string(),
                            replacement: replacement.to_string(),
                            line,
                            col,
                        });
                    }
                }
            }
        }

        // Detect ctx.* calls and auth checks
        if Self::is_ctx_call(&it.callee) {
            if let Some(chain) = Self::resolve_member_chain(&it.callee) {
                // Track ctx call
                // Extract first_arg_chain from the first argument
                let first_arg_chain = it.arguments.first().and_then(|arg| {
                    arg.as_expression()
                        .and_then(|expr| Self::resolve_member_chain(expr))
                });

                let ctx_call = CtxCall {
                    chain: chain.clone(),
                    line,
                    col,
                    in_loop: self.loop_depth > 0,
                    is_awaited: self.in_await,
                    is_returned: self.in_return,
                    assigned_to: self.current_assignment_target.clone(),
                    enclosing_function_kind: self.current_function_kind.clone(),
                    first_arg_chain,
                };
                self.analysis.ctx_calls.push(ctx_call);

                // If ctx call is inside a loop, record it — only for run*/scheduler calls
                if self.loop_depth > 0 {
                    let loop_relevant_prefixes = [
                        "ctx.runMutation",
                        "ctx.runQuery",
                        "ctx.runAction",
                        "ctx.scheduler",
                    ];
                    if loop_relevant_prefixes.iter().any(|p| chain.starts_with(p)) {
                        self.analysis.loop_ctx_calls.push(CallLocation {
                            line,
                            col,
                            detail: chain,
                        });
                    }
                }
            }
        }

        // Detect v.object()/v.array() nesting depth for schema deep-nesting rule
        let is_validator_nesting = if let Expression::StaticMemberExpression(mem) = &it.callee {
            if let Expression::Identifier(ident) = &mem.object {
                ident.name.as_str() == "v"
                    && matches!(mem.property.name.as_str(), "object" | "array")
            } else {
                false
            }
        } else {
            false
        };

        if is_validator_nesting {
            self.validator_nesting_depth += 1;
            if self.validator_nesting_depth > self.max_validator_nesting_depth {
                self.max_validator_nesting_depth = self.validator_nesting_depth;
            }
        }

        // Detect v.array(v.id(...)) pattern for schema array-relationships rule
        if let Expression::StaticMemberExpression(mem) = &it.callee {
            if let Expression::Identifier(ident) = &mem.object {
                if ident.name.as_str() == "v" && mem.property.name.as_str() == "array" {
                    if let Some(first_arg) = it.arguments.first() {
                        if let Some(Expression::CallExpression(inner_call)) =
                            first_arg.as_expression()
                        {
                            if let Expression::StaticMemberExpression(inner_mem) =
                                &inner_call.callee
                            {
                                if let Expression::Identifier(inner_ident) = &inner_mem.object {
                                    if inner_ident.name.as_str() == "v"
                                        && inner_mem.property.name.as_str() == "id"
                                    {
                                        self.analysis.schema_array_id_fields.push(CallLocation {
                                            line,
                                            col,
                                            detail: "v.array(v.id(...))".to_string(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Detect .index("name", ["field1", "field2"]) calls for schema redundant-index rule
        if let Expression::StaticMemberExpression(mem) = &it.callee {
            if mem.property.name.as_str() == "index" && it.arguments.len() >= 2 {
                let table = Self::get_index_table_id(&it.callee).unwrap_or_default();
                let index_name = it.arguments.first().and_then(|arg| {
                    arg.as_expression().and_then(|e| {
                        if let Expression::StringLiteral(s) = e {
                            Some(s.value.as_str().to_string())
                        } else {
                            None
                        }
                    })
                });
                let fields = it.arguments.get(1).and_then(|arg| {
                    arg.as_expression().and_then(|e| {
                        if let Expression::ArrayExpression(arr) = e {
                            Some(
                                arr.elements
                                    .iter()
                                    .filter_map(|el| {
                                        el.as_expression().and_then(|e| {
                                            if let Expression::StringLiteral(s) = e {
                                                Some(s.value.as_str().to_string())
                                            } else {
                                                None
                                            }
                                        })
                                    })
                                    .collect::<Vec<_>>(),
                            )
                        } else {
                            None
                        }
                    })
                });
                if let (Some(name), Some(fields)) = (index_name, fields) {
                    self.analysis.index_definitions.push(IndexDef {
                        table,
                        name,
                        fields,
                        line,
                    });
                }
            }
        }

        // Detect .first() on ctx.db query chains for correctness/missing-unique rule
        if let Expression::StaticMemberExpression(mem) = &it.callee {
            if mem.property.name.as_str() == "first" {
                let full_chain = Self::resolve_member_chain(&it.callee).unwrap_or_default();
                if full_chain.contains("ctx.db") && full_chain.contains(".withIndex.") {
                    self.analysis.first_calls.push(CallLocation {
                        line,
                        col,
                        detail: full_chain,
                    });
                }
            }
        }

        // Check for ctx.auth in the callee chain (for auth check detection)
        if Self::contains_ctx_auth(&it.callee) {
            if let Some(builder) = self.current_builder_mut() {
                builder.has_auth_check = true;
            }
        }

        walk::walk_call_expression(self, it);

        if started_exported_function {
            if let Some(builder) = self.function_builder_stack.pop() {
                self.analysis.functions.push(builder.build());
                self.analysis.exported_function_count += 1;
            }
            self.current_function_kind = prev_function_kind;
        }

        // Restore validator nesting depth after walking children
        if is_validator_nesting {
            self.validator_nesting_depth -= 1;
        }
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        // Also check for ctx.auth in member expressions that aren't calls
        // e.g., `await ctx.auth.getUserIdentity()` - the `ctx.auth` is inside the call chain
        if let Expression::StaticMemberExpression(mem) = it {
            if let Expression::Identifier(ident) = &mem.object {
                if ident.name.as_str() == "ctx" && mem.property.name.as_str() == "auth" {
                    if let Some(builder) = self.current_builder_mut() {
                        builder.has_auth_check = true;
                    }
                }
            }
        }

        walk::walk_expression(self, it);
    }

    fn visit_await_expression(&mut self, it: &AwaitExpression<'a>) {
        if let Expression::Identifier(ident) = &it.argument {
            let awaited = ident.name.as_str().to_string();
            if !self.analysis.awaited_identifiers.contains(&awaited) {
                self.analysis.awaited_identifiers.push(awaited);
            }
        }

        let prev = self.in_await;
        self.in_await = true;
        walk::walk_await_expression(self, it);
        self.in_await = prev;
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        let prev_assignment = self.current_assignment_target.clone();
        if let BindingPattern::BindingIdentifier(ident) = &it.id {
            self.current_assignment_target = Some(ident.name.as_str().to_string());
        }
        walk::walk_variable_declarator(self, it);
        self.current_assignment_target = prev_assignment;
    }

    fn visit_return_statement(&mut self, it: &ReturnStatement<'a>) {
        let prev_return = self.in_return;
        self.in_return = true;
        walk::walk_return_statement(self, it);
        self.in_return = prev_return;
    }

    fn visit_for_statement(&mut self, it: &ForStatement<'a>) {
        self.loop_depth += 1;
        walk::walk_for_statement(self, it);
        self.loop_depth -= 1;
    }

    fn visit_while_statement(&mut self, it: &WhileStatement<'a>) {
        self.loop_depth += 1;
        walk::walk_while_statement(self, it);
        self.loop_depth -= 1;
    }

    fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
        self.loop_depth += 1;
        walk::walk_for_of_statement(self, it);
        self.loop_depth -= 1;
    }

    fn visit_for_in_statement(&mut self, it: &ForInStatement<'a>) {
        self.loop_depth += 1;
        walk::walk_for_in_statement(self, it);
        self.loop_depth -= 1;
    }

    fn visit_string_literal(&mut self, it: &StringLiteral<'a>) {
        let value = it.value.as_str();
        let secret_prefixes = [
            "sk-", "pk-", "AKIA", "ghp_", "gho_", "sk_live_", "sk_test_", "pk_live_", "pk_test_",
        ];
        for prefix in &secret_prefixes {
            if value.starts_with(prefix) && value.len() > 10 {
                let (line, col) = self.line_col(it.span.start);
                self.analysis.hardcoded_secrets.push(super::CallLocation {
                    line,
                    col,
                    detail: format!("String starting with '{prefix}' looks like a secret"),
                });
                break;
            }
        }
        walk::walk_string_literal(self, it);
    }
}
