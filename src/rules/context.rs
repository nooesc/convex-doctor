use std::collections::HashMap;
use std::collections::HashSet;
use std::path::Path;

use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk, Visit};
use oxc_parser::{ParseOptions, Parser};
use oxc_span::{GetSpan, SourceType};

use super::{
    CallLocation, ConvexFunction, ConvexHookCall, CtxCall, DeprecatedCall, FileAnalysis,
    FilterField, FunctionKind, HttpRoute, ImportInfo, IndexDef, SchemaIdField, SearchIndexDef,
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
    has_internal_secret: bool,
    is_intentionally_public: bool,
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
            has_internal_secret: self.has_internal_secret,
            is_intentionally_public: self.is_intentionally_public,
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
    source_lines: Vec<&'a str>,
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
    collect_variables: HashSet<String>,
    current_object_property_name: Option<String>,
    schema_table_id_stack: Vec<String>,
    pending_functions: HashMap<String, ConvexFunction>,
    schema_table_aliases: HashMap<String, String>,
    convex_hook_aliases: HashMap<String, String>,
    identifier_aliases: HashMap<String, String>,
}

impl<'a> ConvexVisitor<'a> {
    fn new(path: &Path, source_text: &'a str) -> Self {
        Self {
            source_text,
            source_lines: source_text.lines().collect(),
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
            collect_variables: HashSet::new(),
            current_object_property_name: None,
            schema_table_id_stack: vec![],
            pending_functions: HashMap::new(),
            schema_table_aliases: HashMap::new(),
            convex_hook_aliases: HashMap::from([
                ("useMutation".to_string(), "useMutation".to_string()),
                ("useQuery".to_string(), "useQuery".to_string()),
                ("useAction".to_string(), "useAction".to_string()),
            ]),
            identifier_aliases: HashMap::new(),
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
            Expression::ChainExpression(chain) => match &chain.expression {
                ChainElement::CallExpression(call) => Self::resolve_member_chain(&call.callee),
                ChainElement::StaticMemberExpression(mem) => {
                    let obj = Self::resolve_member_chain(&mem.object)?;
                    Some(format!("{}.{}", obj, mem.property.name.as_str()))
                }
                ChainElement::ComputedMemberExpression(_) => None,
                ChainElement::PrivateFieldExpression(_) => None,
                _ => None,
            },
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

    /// Resolve a stable table identity for .index(...) / .searchIndex(...) calls.
    fn get_index_table_id(&self, callee: &Expression<'_>) -> Option<String> {
        let Expression::StaticMemberExpression(mem) = callee else {
            return None;
        };
        if !matches!(mem.property.name.as_str(), "index" | "searchIndex") {
            return None;
        }
        self.resolve_table_id_from_expr(&mem.object)
    }

    fn resolve_table_id_from_expr(&self, expr: &Expression<'_>) -> Option<String> {
        match expr {
            Expression::Identifier(ident) => {
                self.schema_table_aliases.get(ident.name.as_str()).cloned()
            }
            Expression::CallExpression(call) => match &call.callee {
                Expression::Identifier(ident) if ident.name.as_str() == "defineTable" => {
                    Some(format!("table@{}", call.span.start))
                }
                Expression::StaticMemberExpression(mem)
                    if matches!(mem.property.name.as_str(), "index" | "searchIndex") =>
                {
                    self.resolve_table_id_from_expr(&mem.object)
                }
                _ => Self::find_define_table_call_start(expr).map(|start| format!("table@{start}")),
            },
            Expression::StaticMemberExpression(mem) => self.resolve_table_id_from_expr(&mem.object),
            _ => Self::find_define_table_call_start(expr).map(|start| format!("table@{start}")),
        }
    }

    fn parse_named_import_entries(raw_import: &str) -> Vec<(String, String)> {
        let Some(open_idx) = raw_import.find('{') else {
            return vec![];
        };
        let Some(close_idx) = raw_import.rfind('}') else {
            return vec![];
        };
        if close_idx <= open_idx {
            return vec![];
        }

        raw_import[open_idx + 1..close_idx]
            .split(',')
            .filter_map(|part| {
                let trimmed = part.trim();
                if trimmed.is_empty() {
                    return None;
                }
                let tokens: Vec<&str> = trimmed.split_whitespace().collect();
                if tokens.is_empty() {
                    return None;
                }
                if tokens.len() == 1 {
                    return Some((tokens[0].to_string(), tokens[0].to_string()));
                }
                if tokens.len() == 3 && tokens[1] == "as" {
                    return Some((tokens[0].to_string(), tokens[2].to_string()));
                }
                None
            })
            .collect()
    }

    fn resolve_convex_hook_name(&self, callee: &Expression<'_>) -> Option<String> {
        match callee {
            Expression::Identifier(ident) => {
                self.convex_hook_aliases.get(ident.name.as_str()).cloned()
            }
            _ => None,
        }
    }

    fn is_pagination_opts_validator_expr(expr: &Expression<'_>) -> bool {
        match expr {
            Expression::Identifier(ident) => ident.name.as_str() == "paginationOptsValidator",
            Expression::StaticMemberExpression(mem) => {
                if mem.property.name.as_str() == "paginationOptsValidator" {
                    return true;
                }
                Self::is_pagination_opts_validator_expr(&mem.object)
            }
            Expression::ChainExpression(chain) => match &chain.expression {
                ChainElement::StaticMemberExpression(mem) => {
                    if mem.property.name.as_str() == "paginationOptsValidator" {
                        return true;
                    }
                    Self::is_pagination_opts_validator_expr(&mem.object)
                }
                ChainElement::CallExpression(call) => {
                    Self::is_pagination_opts_validator_expr(&call.callee)
                }
                _ => false,
            },
            _ => false,
        }
    }

    fn is_immediately_invoked(source_text: &str, call_end: u32) -> bool {
        let mut idx = call_end as usize;
        let bytes = source_text.as_bytes();
        while idx < bytes.len() {
            let ch = bytes[idx] as char;
            if !ch.is_whitespace() {
                return ch == '(';
            }
            idx += 1;
        }
        false
    }

    fn collect_identifiers(expr: &Expression<'_>, out: &mut Vec<String>) {
        match expr {
            Expression::Identifier(ident) => out.push(ident.name.as_str().to_string()),
            Expression::ArrayExpression(arr) => {
                for el in &arr.elements {
                    if let Some(inner) = el.as_expression() {
                        Self::collect_identifiers(inner, out);
                    }
                }
            }
            Expression::CallExpression(call) => {
                Self::collect_identifiers(&call.callee, out);
                for arg in &call.arguments {
                    if let Some(inner) = arg.as_expression() {
                        Self::collect_identifiers(inner, out);
                    }
                }
            }
            Expression::StaticMemberExpression(mem) => {
                Self::collect_identifiers(&mem.object, out);
            }
            Expression::ComputedMemberExpression(mem) => {
                Self::collect_identifiers(&mem.object, out);
                Self::collect_identifiers(&mem.expression, out);
            }
            Expression::ChainExpression(chain) => match &chain.expression {
                ChainElement::CallExpression(call) => {
                    Self::collect_identifiers(&call.callee, out);
                    for arg in &call.arguments {
                        if let Some(inner) = arg.as_expression() {
                            Self::collect_identifiers(inner, out);
                        }
                    }
                }
                ChainElement::StaticMemberExpression(mem) => {
                    Self::collect_identifiers(&mem.object, out);
                }
                ChainElement::ComputedMemberExpression(mem) => {
                    Self::collect_identifiers(&mem.object, out);
                    Self::collect_identifiers(&mem.expression, out);
                }
                _ => {}
            },
            Expression::ParenthesizedExpression(paren) => {
                Self::collect_identifiers(&paren.expression, out);
            }
            Expression::UnaryExpression(unary) => {
                Self::collect_identifiers(&unary.argument, out);
            }
            Expression::BinaryExpression(binary) => {
                Self::collect_identifiers(&binary.left, out);
                Self::collect_identifiers(&binary.right, out);
            }
            Expression::LogicalExpression(logical) => {
                Self::collect_identifiers(&logical.left, out);
                Self::collect_identifiers(&logical.right, out);
            }
            Expression::ConditionalExpression(cond) => {
                Self::collect_identifiers(&cond.test, out);
                Self::collect_identifiers(&cond.consequent, out);
                Self::collect_identifiers(&cond.alternate, out);
            }
            _ => {}
        }
    }

    fn record_awaited_identifier(&mut self, identifier: String) {
        if !self.analysis.awaited_identifiers.contains(&identifier) {
            self.analysis.awaited_identifiers.push(identifier);
        }
    }

    fn resolve_identifier_alias_root(&self, identifier: &str) -> Option<String> {
        let mut current = identifier;
        let mut hops = 0;
        while let Some(next) = self.identifier_aliases.get(current) {
            if next == current {
                break;
            }
            current = next;
            hops += 1;
            if hops > 32 {
                break;
            }
        }
        if current == identifier {
            None
        } else {
            Some(current.to_string())
        }
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
            Expression::ChainExpression(chain) => match &chain.expression {
                ChainElement::CallExpression(call) => Self::contains_ctx_auth(&call.callee),
                ChainElement::StaticMemberExpression(mem) => {
                    if let Expression::Identifier(ident) = &mem.object {
                        if ident.name.as_str() == "ctx" && mem.property.name.as_str() == "auth" {
                            return true;
                        }
                    }
                    Self::contains_ctx_auth(&mem.object)
                }
                _ => false,
            },
            _ => false,
        }
    }

    fn has_doctor_ignore_comment(source_lines: &[&str], line: u32) -> bool {
        if line == 0 {
            return false;
        }

        let target = line.saturating_sub(1) as isize;
        let mut start = target.saturating_sub(2);
        if start < 0 {
            start = 0;
        }

        for idx in start..=target {
            let Some(line_text) = source_lines.get(idx as usize) else {
                continue;
            };
            if line_text
                .to_ascii_lowercase()
                .contains("convex-doctor-ignore")
            {
                return true;
            }
        }

        false
    }

    fn callee_name(expr: &Expression<'_>) -> Option<String> {
        match expr {
            Expression::Identifier(ident) => Some(ident.name.as_str().to_string()),
            Expression::StaticMemberExpression(mem) => Some(mem.property.name.as_str().to_string()),
            Expression::ChainExpression(chain) => match &chain.expression {
                ChainElement::CallExpression(call) => Self::callee_name(&call.callee),
                ChainElement::StaticMemberExpression(mem) => {
                    Some(mem.property.name.as_str().to_string())
                }
                _ => None,
            },
            Expression::CallExpression(call) => Self::callee_name(&call.callee),
            _ => None,
        }
    }

    fn expression_has_identifier(expr: &Expression<'_>, target: &str) -> bool {
        match expr {
            Expression::Identifier(ident) => ident.name.as_str() == target,
            Expression::StaticMemberExpression(mem) => {
                Self::expression_has_identifier(&mem.object, target)
            }
            Expression::CallExpression(call) => {
                Self::expression_has_identifier(&call.callee, target)
                    || call.arguments.iter().any(|arg| {
                        arg.as_expression()
                            .is_some_and(|e| Self::expression_has_identifier(e, target))
                    })
            }
            Expression::ChainExpression(chain) => match &chain.expression {
                ChainElement::CallExpression(call) => {
                    Self::expression_has_identifier(&call.callee, target)
                }
                ChainElement::StaticMemberExpression(mem) => {
                    Self::expression_has_identifier(&mem.object, target)
                }
                _ => false,
            },
            _ => false,
        }
    }

    fn call_uses_auth_helper(call: &CallExpression<'_>) -> bool {
        let Some(callee_name) = Self::callee_name(&call.callee) else {
            return false;
        };

        let name = callee_name.to_ascii_lowercase();
        let known_helpers = [
            "requireadmin",
            "requireauth",
            "requireuser",
            "verifytoken",
            "verifyjwt",
            "ensureauthenticated",
            "assertauthenticated",
            "getauthenticateduser",
            "parseauthheader",
        ];

        if !call.arguments.is_empty()
            && known_helpers.contains(&name.as_str())
            && call.arguments.iter().any(|arg| {
                arg.as_expression().is_some_and(|expr| {
                    Self::expression_has_identifier(expr, "ctx")
                        || Self::expression_has_identifier(expr, "request")
                })
            })
        {
            return true;
        }

        if call.arguments.iter().any(|arg| {
            arg.as_expression().is_some_and(|expr| {
                Self::expression_has_identifier(expr, "ctx")
                    || Self::expression_has_identifier(expr, "request")
            })
        }) && ["require", "ensure", "assert", "verify"]
            .iter()
            .any(|prefix| name.starts_with(prefix))
            && (name.contains("auth")
                || name.contains("token")
                || name.contains("user")
                || name.contains("admin")
                || name.contains("session"))
        {
            return true;
        }

        false
    }

    /// Check if a call expression is a ctx.* call.
    fn is_ctx_call(callee: &Expression<'_>) -> bool {
        if let Some(chain) = Self::resolve_member_chain(callee) {
            return chain.starts_with("ctx.");
        }
        false
    }

    /// Check if an expression tree contains `process.env` anywhere.
    fn contains_process_env(expr: &Expression<'_>) -> bool {
        match expr {
            Expression::StaticMemberExpression(mem) => {
                if let Expression::Identifier(ident) = &mem.object {
                    if ident.name.as_str() == "process" && mem.property.name.as_str() == "env" {
                        return true;
                    }
                }
                Self::contains_process_env(&mem.object)
            }
            Expression::ComputedMemberExpression(mem) => Self::contains_process_env(&mem.object),
            Expression::CallExpression(call) => {
                Self::contains_process_env(&call.callee)
                    || call.arguments.iter().any(|a| {
                        a.as_expression()
                            .is_some_and(|e| Self::contains_process_env(e))
                    })
            }
            Expression::ConditionalExpression(cond) => {
                Self::contains_process_env(&cond.test)
                    || Self::contains_process_env(&cond.consequent)
                    || Self::contains_process_env(&cond.alternate)
            }
            Expression::BinaryExpression(bin) => {
                Self::contains_process_env(&bin.left) || Self::contains_process_env(&bin.right)
            }
            Expression::UnaryExpression(un) => Self::contains_process_env(&un.argument),
            Expression::LogicalExpression(log) => {
                Self::contains_process_env(&log.left) || Self::contains_process_env(&log.right)
            }
            _ => false,
        }
    }

    /// Check if an expression is a Convex function constructor call.
    fn is_convex_function_call(expr: &Expression<'_>) -> bool {
        if let Expression::CallExpression(call) = expr {
            Self::get_function_kind(&call.callee).is_some()
        } else {
            false
        }
    }

    /// Extract field names from a filter callback argument by searching for q.field("name") patterns.
    fn extract_filter_field_names(expr: &Expression<'_>) -> Vec<String> {
        let mut fields = vec![];
        Self::collect_field_calls(expr, &mut fields);
        fields
    }

    fn collect_field_calls(expr: &Expression<'_>, fields: &mut Vec<String>) {
        match expr {
            Expression::CallExpression(call) => {
                // Check if this is q.field("name") or similar param.field("name")
                if let Expression::StaticMemberExpression(mem) = &call.callee {
                    if mem.property.name.as_str() == "field" {
                        if let Expression::Identifier(_) = &mem.object {
                            if let Some(arg) = call.arguments.first() {
                                if let Some(Expression::StringLiteral(s)) = arg.as_expression() {
                                    fields.push(s.value.as_str().to_string());
                                }
                            }
                        }
                    }
                }
                // Recurse into callee and arguments
                Self::collect_field_calls(&call.callee, fields);
                for arg in &call.arguments {
                    if let Some(e) = arg.as_expression() {
                        Self::collect_field_calls(e, fields);
                    }
                }
            }
            Expression::StaticMemberExpression(mem) => {
                Self::collect_field_calls(&mem.object, fields);
            }
            Expression::ArrowFunctionExpression(arrow) => {
                // Walk into the arrow function body
                match &arrow.body.statements.first() {
                    Some(Statement::ExpressionStatement(stmt)) => {
                        Self::collect_field_calls(&stmt.expression, fields);
                    }
                    Some(Statement::ReturnStatement(ret)) => {
                        if let Some(arg) = &ret.argument {
                            Self::collect_field_calls(arg, fields);
                        }
                    }
                    _ => {
                        for stmt in &arrow.body.statements {
                            if let Statement::ExpressionStatement(es) = stmt {
                                Self::collect_field_calls(&es.expression, fields);
                            } else if let Statement::ReturnStatement(ret) = stmt {
                                if let Some(arg) = &ret.argument {
                                    Self::collect_field_calls(arg, fields);
                                }
                            }
                        }
                    }
                }
            }
            Expression::FunctionExpression(func) => {
                if let Some(body) = &func.body {
                    for stmt in &body.statements {
                        if let Statement::ExpressionStatement(es) = stmt {
                            Self::collect_field_calls(&es.expression, fields);
                        } else if let Statement::ReturnStatement(ret) = stmt {
                            if let Some(arg) = &ret.argument {
                                Self::collect_field_calls(arg, fields);
                            }
                        }
                    }
                }
            }
            Expression::BinaryExpression(bin) => {
                Self::collect_field_calls(&bin.left, fields);
                Self::collect_field_calls(&bin.right, fields);
            }
            Expression::LogicalExpression(log) => {
                Self::collect_field_calls(&log.left, fields);
                Self::collect_field_calls(&log.right, fields);
            }
            Expression::UnaryExpression(un) => {
                Self::collect_field_calls(&un.argument, fields);
            }
            Expression::ParenthesizedExpression(paren) => {
                Self::collect_field_calls(&paren.expression, fields);
            }
            Expression::ConditionalExpression(cond) => {
                Self::collect_field_calls(&cond.test, fields);
                Self::collect_field_calls(&cond.consequent, fields);
                Self::collect_field_calls(&cond.alternate, fields);
            }
            _ => {}
        }
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
        let source_str = it.source.value.as_str();
        let start = (it.span.start as usize).min(self.source_text.len());
        let end = (it.span.end as usize).min(self.source_text.len());
        let import_stmt = &self.source_text[start..end];

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

        if source_str.contains("convex/react") {
            for (imported, local) in Self::parse_named_import_entries(import_stmt) {
                match imported.as_str() {
                    "useMutation" | "useQuery" | "useAction" => {
                        self.convex_hook_aliases.insert(local, imported);
                    }
                    "ConvexProvider" | "ConvexReactClient" => {
                        self.analysis.has_convex_provider = true;
                    }
                    _ => {}
                }
            }
        }

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

                // 6. Detect conditional exports: export const x = process.env.X ? query(...) : mutation(...)
                if let Some(Expression::ConditionalExpression(cond)) = &declarator.init {
                    let test_has_process_env = Self::contains_process_env(&cond.test);
                    let has_convex_call = Self::is_convex_function_call(&cond.consequent)
                        || Self::is_convex_function_call(&cond.alternate);
                    if test_has_process_env && has_convex_call {
                        let (line, col) = self.line_col(declarator.span.start);
                        self.analysis.conditional_exports.push(CallLocation {
                            line,
                            col,
                            detail: "Conditional export based on process.env".to_string(),
                        });
                    }
                }
            }
        }

        // Handle export { foo, bar } specifiers — promote pending functions
        for spec in &it.specifiers {
            let local_name = spec.local.name().to_string();
            if let Some(mut func) = self.pending_functions.remove(&local_name) {
                // The exported name might differ: export { foo as bar }
                let exported_name = spec.exported.name().to_string();
                func.name = exported_name;
                self.analysis.functions.push(func);
                self.analysis.exported_function_count += 1;
            }
        }

        self.current_export_names = export_names;
        self.next_export_index = 0;
        walk::walk_export_named_declaration(self, it);
        self.current_export_names.clear();
        self.next_export_index = 0;
    }

    fn visit_export_default_declaration(&mut self, it: &ExportDefaultDeclaration<'a>) {
        // Check if the default export is a Convex function call expression
        if let ExportDefaultDeclarationKind::CallExpression(_) = &it.declaration {
            if Self::is_convex_function_call(it.declaration.to_expression()) {
                self.current_export_names = vec!["default".to_string()];
                self.next_export_index = 0;
            }
        }
        walk::walk_export_default_declaration(self, it);
        self.current_export_names.clear();
        self.next_export_index = 0;
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        let (line, col) = self.line_col(it.span.start);
        let prev_function_kind = self.current_function_kind.clone();
        let mut started_exported_function = false;
        let mut schema_table_id = None;

        if let Expression::Identifier(ident) = &it.callee {
            if ident.name.as_str() == "defineTable"
                && it
                    .arguments
                    .first()
                    .is_some_and(|arg| matches!(arg, Argument::ObjectExpression(_)))
            {
                schema_table_id = Some(format!("table@{}", it.span.start));
            }
        }
        if let Some(table_id) = schema_table_id.clone() {
            self.schema_table_id_stack.push(table_id);
        }

        // Check if this is a Convex function definition: query({...}), mutation({...}), etc.
        let mut is_direct_export = false;
        if let Some(kind) = Self::get_function_kind(&it.callee) {
            let export_name = self.next_export_name();
            is_direct_export = export_name.is_some();
            let tracking_name = export_name.or_else(|| self.current_assignment_target.clone());

            if let Some(name) = tracking_name {
                let mut builder = FunctionBuilder {
                    name: name.clone(),
                    kind: Some(kind.clone()),
                    is_intentionally_public: Self::has_doctor_ignore_comment(
                        &self.source_lines,
                        line,
                    ),
                    span_line: line,
                    span_col: col,
                    ..Default::default()
                };

                // Check the first argument for the config object
                if let Some(Argument::ObjectExpression(obj)) = it.arguments.first() {
                    // Inspect config properties before walking
                    for prop in &obj.properties {
                        if let ObjectPropertyKind::ObjectProperty(prop) = prop {
                            if let Some(prop_name) = prop.key.static_name() {
                                match prop_name.as_ref() {
                                    "args" => {
                                        builder.has_args_validator = true;
                                        if let Expression::ObjectExpression(args_obj) = &prop.value
                                        {
                                            for arg_prop in &args_obj.properties {
                                                if let ObjectPropertyKind::ObjectProperty(arg) =
                                                    arg_prop
                                                {
                                                    let arg_name_str = arg
                                                        .key
                                                        .static_name()
                                                        .map(|n| n.to_string());
                                                    if let Some(ref arg_name) = arg_name_str {
                                                        builder.arg_names.push(arg_name.clone());
                                                        if matches!(
                                                            arg_name.to_ascii_lowercase().as_str(),
                                                            "internalsecret" | "internal_secret"
                                                        ) {
                                                            builder.has_internal_secret = true;
                                                        }
                                                        if arg_name == "paginationOpts"
                                                            && Self::is_pagination_opts_validator_expr(
                                                                &arg.value,
                                                            )
                                                        {
                                                            self.analysis
                                                                .pagination_validator_functions
                                                                .push(name.clone());
                                                        }
                                                    }

                                                    // Check for v.any() in arg values
                                                    if let Expression::CallExpression(val_call) =
                                                        &arg.value
                                                    {
                                                        if let Expression::StaticMemberExpression(
                                                            val_mem,
                                                        ) = &val_call.callee
                                                        {
                                                            if let Expression::Identifier(
                                                                val_ident,
                                                            ) = &val_mem.object
                                                            {
                                                                if val_ident.name.as_str() == "v"
                                                                    && val_mem
                                                                        .property
                                                                        .name
                                                                        .as_str()
                                                                        == "any"
                                                                {
                                                                    builder
                                                                        .has_any_validator_in_args =
                                                                        true;
                                                                }

                                                                // Check for v.id() with zero args (generic id validator)
                                                                if val_ident.name.as_str() == "v"
                                                                    && val_mem
                                                                        .property
                                                                        .name
                                                                        .as_str()
                                                                        == "id"
                                                                    && val_call.arguments.is_empty()
                                                                {
                                                                    let detail = if let Some(
                                                                        ref an,
                                                                    ) = arg_name_str
                                                                    {
                                                                        format!("Arg '{}' uses v.id() without table name", an)
                                                                    } else {
                                                                        "v.id() without table name"
                                                                            .to_string()
                                                                    };
                                                                    self.analysis
                                                                        .generic_id_validators
                                                                        .push(CallLocation {
                                                                            line,
                                                                            col,
                                                                            detail,
                                                                        });
                                                                }
                                                            }
                                                        }
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
                            detail: format!("{}() using old function syntax", name),
                        });
                }

                self.function_builder_stack.push(builder);
                self.collect_variables.clear();
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
                        detail: full_chain,
                    });
                    // Track variables assigned from ctx.db.*.collect() for collect-then-filter detection
                    if let Some(ref target) = self.current_assignment_target {
                        self.collect_variables.insert(target.clone());
                    }
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

                // Detect .filter() on variables that hold collect() results
                if let Expression::Identifier(ident) = &mem.object {
                    let var_name = ident.name.as_str();
                    if self.collect_variables.contains(var_name) {
                        self.analysis.collect_variable_filters.push(CallLocation {
                            line,
                            col,
                            detail: format!("{}.filter() after .collect() — filter in JS instead of using an index", var_name),
                        });
                    }
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

        // Detect deprecated API calls (e.g., v.bigint())
        if let Expression::StaticMemberExpression(mem) = &it.callee {
            if let Expression::Identifier(ident) = &mem.object {
                if ident.name.as_str() == "v" {
                    let prop = mem.property.name.as_str();
                    let deprecated = match prop {
                        "bigint" => Some(("v.bigint()", "Use v.int64() instead")),
                        // v.any() is NOT deprecated — it's flagged separately by
                        // security/generic-mutation-args when used in public arg validators.
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

        // Detect unsupported validator types: v.map() and v.set()
        if let Expression::StaticMemberExpression(mem) = &it.callee {
            if let Expression::Identifier(ident) = &mem.object {
                if ident.name.as_str() == "v" && matches!(mem.property.name.as_str(), "map" | "set")
                {
                    self.analysis
                        .unsupported_validator_calls
                        .push(CallLocation {
                            line,
                            col,
                            detail: format!("v.{}()", mem.property.name.as_str()),
                        });
                }
            }
        }

        // Detect ctx.* calls and auth checks
        if Self::is_ctx_call(&it.callee) {
            if let Some(chain) = Self::resolve_member_chain(&it.callee) {
                // Track ctx call
                // Extract first_arg_chain from the first argument by default.
                // For scheduler.runAfter/runAt, the callable is the second arg.
                let target_arg_index = if chain.starts_with("ctx.scheduler.runAfter")
                    || chain.starts_with("ctx.scheduler.runAt")
                {
                    1
                } else {
                    0
                };
                let first_arg_chain = it.arguments.get(target_arg_index).and_then(|arg| {
                    arg.as_expression()
                        .and_then(|expr| Self::resolve_member_chain(expr))
                });

                let (enclosing_function_name, enclosing_function_has_internal_secret) =
                    match self.current_builder_mut() {
                        Some(builder) => (Some(builder.name.clone()), builder.has_internal_secret),
                        None => (None, false),
                    };

                let ctx_call = CtxCall {
                    chain: chain.clone(),
                    line,
                    col,
                    is_awaited: self.in_await,
                    is_returned: self.in_return,
                    assigned_to: self.current_assignment_target.clone(),
                    enclosing_function_kind: self.current_function_kind.clone(),
                    enclosing_function_name,
                    enclosing_function_has_internal_secret,
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
                let table = self.get_index_table_id(&it.callee).unwrap_or_default();
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

        // Detect .delete() on query chains in query handlers
        if let Expression::StaticMemberExpression(mem) = &it.callee {
            if mem.property.name.as_str() == "delete"
                && self
                    .current_function_kind
                    .as_ref()
                    .is_some_and(|k| k.is_query())
            {
                let full_chain = Self::resolve_member_chain(&it.callee).unwrap_or_default();
                if full_chain.contains("ctx.db")
                    && (full_chain.contains(".query.")
                        || full_chain.contains(".withIndex.")
                        || full_chain.contains(".withSearchIndex."))
                {
                    self.analysis.query_delete_calls.push(CallLocation {
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
        if Self::call_uses_auth_helper(it) {
            if let Some(builder) = self.current_builder_mut() {
                builder.has_auth_check = true;
            }
        }

        // --- NEW DETECTION PATTERNS ---

        // 1. Detect cron scheduling method calls with api.* arguments
        if let Expression::StaticMemberExpression(mem) = &it.callee {
            let prop_name = mem.property.name.as_str();
            if matches!(
                prop_name,
                "interval" | "hourly" | "daily" | "weekly" | "monthly" | "cron"
            ) {
                // Only flag if the receiver looks like a cron scheduling object
                let receiver_chain = Self::resolve_member_chain(&mem.object).unwrap_or_default();
                if receiver_chain == "crons"
                    || receiver_chain.contains("cron")
                    || receiver_chain.contains("Cron")
                {
                    if matches!(prop_name, "hourly" | "daily" | "weekly") {
                        self.analysis.cron_helper_calls.push(CallLocation {
                            line,
                            col,
                            detail: format!("crons.{prop_name}(...)"),
                        });
                    }

                    // For cron methods, the callable reference is typically the 3rd arg.
                    if let Some(function_ref_arg) = it.arguments.get(2) {
                        if let Some(expr) = function_ref_arg.as_expression() {
                            if let Some(chain) = Self::resolve_member_chain(expr) {
                                if !chain.starts_with("internal.") && !chain.starts_with("api.") {
                                    self.analysis.cron_non_reference_calls.push(CallLocation {
                                        line,
                                        col,
                                        detail: chain,
                                    });
                                }
                            }
                        }
                    }

                    // Check if any argument resolves to an api.* chain
                    for arg in &it.arguments {
                        if let Some(expr) = arg.as_expression() {
                            if let Some(chain) = Self::resolve_member_chain(expr) {
                                if chain.starts_with("api.") {
                                    self.analysis.cron_api_refs.push(CallLocation {
                                        line,
                                        col,
                                        detail: chain,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        // 2a. Detect Math.random() in query function context
        if let Expression::StaticMemberExpression(mem) = &it.callee {
            if mem.property.name.as_str() == "random" {
                if let Expression::Identifier(ident) = &mem.object {
                    if ident.name.as_str() == "Math"
                        && self
                            .current_function_kind
                            .as_ref()
                            .is_some_and(|k| k.is_query())
                    {
                        self.analysis.non_deterministic_calls.push(CallLocation {
                            line,
                            col,
                            detail: "Math.random()".to_string(),
                        });
                    }
                }
            }
        }

        // 7. Detect ctx.db.patch(id, args) — raw arg patching
        if Self::is_ctx_call(&it.callee) {
            if let Some(chain) = Self::resolve_member_chain(&it.callee) {
                if chain.starts_with("ctx.db.patch") {
                    if let Some(second_arg) = it.arguments.get(1) {
                        if let Some(expr) = second_arg.as_expression() {
                            if let Some(arg_chain) = Self::resolve_member_chain(expr) {
                                if arg_chain == "args" {
                                    self.analysis.raw_arg_patches.push(CallLocation {
                                        line,
                                        col,
                                        detail: "ctx.db.patch(id, args) passes raw args"
                                            .to_string(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        // 8. Detect HTTP .route() calls with { method, path } config
        if let Expression::StaticMemberExpression(mem) = &it.callee {
            if mem.property.name.as_str() == "route" {
                if let Some(first_arg) = it.arguments.first() {
                    if let Some(Expression::ObjectExpression(obj)) = first_arg.as_expression() {
                        let mut method = None;
                        let mut path = None;
                        for prop in &obj.properties {
                            if let ObjectPropertyKind::ObjectProperty(p) = prop {
                                if let Some(key_name) = p.key.static_name() {
                                    match key_name.as_ref() {
                                        "method" => {
                                            if let Expression::StringLiteral(s) = &p.value {
                                                method = Some(s.value.as_str().to_string());
                                            }
                                        }
                                        "path" | "pathPrefix" => {
                                            if let Expression::StringLiteral(s) = &p.value {
                                                path = Some(s.value.as_str().to_string());
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                        if let (Some(m), Some(p)) = (method, path) {
                            let path_contains_webhook = p
                                .to_ascii_lowercase()
                                .split('/')
                                .filter(|segment| !segment.is_empty())
                                .any(|segment| segment == "webhook");
                            let mut comment_marks_webhook = false;
                            if line > 0 {
                                let route_line_index = usize::try_from(line.saturating_sub(1)).ok();
                                if let Some(route_idx) = route_line_index {
                                    let start = route_idx.saturating_sub(3);
                                    for idx in start..=route_idx {
                                        if let Some(raw_line) = self.source_lines.get(idx) {
                                            let text = raw_line.to_ascii_lowercase();
                                            if text.contains("webhook") {
                                                comment_marks_webhook = true;
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                            self.analysis.http_routes.push(HttpRoute {
                                method: m,
                                path: p,
                                is_webhook: path_contains_webhook || comment_marks_webhook,
                                line,
                            });
                        }
                    }
                }
            }
        }

        // 9. Detect v.id("tableName") for schema_id_fields tracking
        if let Expression::StaticMemberExpression(mem) = &it.callee {
            if let Expression::Identifier(ident) = &mem.object {
                if ident.name.as_str() == "v" && mem.property.name.as_str() == "id" {
                    if let Some(first_arg) = it.arguments.first() {
                        if let Some(Expression::StringLiteral(s)) = first_arg.as_expression() {
                            let table_ref = s.value.as_str().to_string();
                            if self.schema_table_id_stack.last().is_some() {
                                self.analysis.schema_id_fields.push(SchemaIdField {
                                    field_name: self
                                        .current_object_property_name
                                        .clone()
                                        .unwrap_or_default(),
                                    table_ref,
                                    table_id: self
                                        .schema_table_id_stack
                                        .last()
                                        .cloned()
                                        .unwrap_or_default(),
                                    file: self.analysis.file_path.clone(),
                                    line,
                                    col,
                                });
                            }
                        }
                    }
                }
            }
        }

        // 10. Detect .filter() on ctx.db chains and extract q.field("name") patterns
        if let Expression::StaticMemberExpression(mem) = &it.callee {
            if mem.property.name.as_str() == "filter" {
                let full_chain = Self::resolve_member_chain(&it.callee).unwrap_or_default();
                if full_chain.contains("ctx.db") {
                    // Try to extract field names from the filter callback
                    if let Some(first_arg) = it.arguments.first() {
                        if let Some(expr) = first_arg.as_expression() {
                            let field_names = Self::extract_filter_field_names(expr);
                            for field_name in field_names {
                                self.analysis.filter_field_names.push(FilterField {
                                    field_name,
                                    line,
                                    col,
                                });
                            }
                        }
                    }
                }
            }
        }

        // 11. Detect .searchIndex("name", { searchField, filterFields }) calls
        if let Expression::StaticMemberExpression(mem) = &it.callee {
            if mem.property.name.as_str() == "searchIndex" && it.arguments.len() >= 2 {
                let table = self.get_index_table_id(&it.callee).unwrap_or_default();
                let index_name = it.arguments.first().and_then(|arg| {
                    arg.as_expression().and_then(|e| {
                        if let Expression::StringLiteral(s) = e {
                            Some(s.value.as_str().to_string())
                        } else {
                            None
                        }
                    })
                });
                let has_filter_fields = it.arguments.get(1).is_some_and(|arg| {
                    arg.as_expression().is_some_and(|e| {
                        if let Expression::ObjectExpression(obj) = e {
                            obj.properties.iter().any(|p| {
                                if let ObjectPropertyKind::ObjectProperty(prop) = p {
                                    prop.key
                                        .static_name()
                                        .is_some_and(|n| n.as_ref() == "filterFields")
                                } else {
                                    false
                                }
                            })
                        } else {
                            false
                        }
                    })
                });
                if let Some(name) = index_name {
                    self.analysis.search_index_definitions.push(SearchIndexDef {
                        table,
                        name,
                        has_filter_fields,
                        line,
                    });
                }
            }
        }

        // 12. Detect v.optional() calls
        if let Expression::StaticMemberExpression(mem) = &it.callee {
            if let Expression::Identifier(ident) = &mem.object {
                if ident.name.as_str() == "v" && mem.property.name.as_str() == "optional" {
                    self.analysis.optional_schema_fields.push(CallLocation {
                        line,
                        col,
                        detail: "v.optional()".to_string(),
                    });
                }
            }
        }

        // 13. Detect large writes: ctx.db.insert/ctx.db.replace with >20 properties
        if Self::is_ctx_call(&it.callee) {
            if let Some(chain) = Self::resolve_member_chain(&it.callee) {
                if chain.starts_with("ctx.db.insert") || chain.starts_with("ctx.db.replace") {
                    // Check the appropriate arg — for insert it's the 2nd arg (index 1),
                    // for replace it's also the 2nd arg
                    let data_arg = it.arguments.get(1);
                    if let Some(arg) = data_arg {
                        if let Some(Expression::ObjectExpression(obj)) = arg.as_expression() {
                            let prop_count = obj.properties.len();
                            if prop_count > 20 {
                                self.analysis.large_writes.push(CallLocation {
                                    line,
                                    col,
                                    detail: format!("{} with {} properties", chain, prop_count),
                                });
                            }
                        }
                    }
                }

                if chain.starts_with("ctx.storage.getMetadata") {
                    self.analysis.storage_metadata_calls.push(CallLocation {
                        line,
                        col,
                        detail: chain,
                    });
                }
            }
        }

        // 14. Track functions that use `.paginate(...)` for pagination validator checks
        if let Expression::StaticMemberExpression(mem) = &it.callee {
            if mem.property.name.as_str() == "paginate"
                && self
                    .current_function_kind
                    .as_ref()
                    .is_some_and(|k| k.is_query())
            {
                let function_name = self
                    .current_builder_mut()
                    .map(|builder| builder.name.clone());
                if let Some(name) = function_name {
                    self.analysis.paginated_functions.push(CallLocation {
                        line,
                        col,
                        detail: name,
                    });
                }
            }
        }

        // 15. Detect Convex hook calls: useMutation, useQuery, useAction
        if let Some(hook_name) = self.resolve_convex_hook_name(&it.callee) {
            let in_render_body = hook_name == "useMutation"
                && Self::is_immediately_invoked(self.source_text, it.span.end);
            self.analysis.convex_hook_calls.push(ConvexHookCall {
                hook_name,
                line,
                col,
                in_render_body,
            });
        }

        walk::walk_call_expression(self, it);
        if schema_table_id.is_some() {
            self.schema_table_id_stack.pop();
        }

        if started_exported_function {
            if let Some(builder) = self.function_builder_stack.pop() {
                let func = builder.build();
                if is_direct_export {
                    // Direct export: export const foo = query({...}) or export default query({...})
                    self.analysis.functions.push(func);
                    self.analysis.exported_function_count += 1;
                } else {
                    // Deferred: const foo = query({...}) — might be exported later via export { foo }
                    self.pending_functions.insert(func.name.clone(), func);
                }
            }
            self.current_function_kind = prev_function_kind;
        }

        // Restore validator nesting depth after walking children
        if is_validator_nesting {
            self.validator_nesting_depth -= 1;
        }
    }

    fn visit_object_property(&mut self, it: &ObjectProperty<'a>) {
        let prev = self.current_object_property_name.take();
        if let Some(name) = it.key.static_name() {
            self.current_object_property_name = Some(name.to_string());
        }
        walk::walk_object_property(self, it);
        self.current_object_property_name = prev;
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
        let mut identifiers = Vec::new();
        Self::collect_identifiers(&it.argument, &mut identifiers);
        for identifier in identifiers {
            self.record_awaited_identifier(identifier.clone());
            if let Some(root) = self.resolve_identifier_alias_root(&identifier) {
                self.record_awaited_identifier(root);
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
            let name = ident.name.as_str().to_string();
            self.current_assignment_target = Some(name.clone());

            if let Some(init) = &it.init {
                if let Some(start) = Self::find_define_table_call_start(init) {
                    self.schema_table_aliases
                        .insert(name.clone(), format!("table@{start}"));
                }

                if let Expression::Identifier(source_ident) = init {
                    self.identifier_aliases
                        .insert(name.clone(), source_ident.name.as_str().to_string());
                } else {
                    self.identifier_aliases.remove(&name);
                }
            } else {
                self.identifier_aliases.remove(&name);
            }
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

    // 2b. Detect `new Date()` in query function context
    fn visit_new_expression(&mut self, it: &NewExpression<'a>) {
        if let Expression::Identifier(ident) = &it.callee {
            if ident.name.as_str() == "Date"
                && self
                    .current_function_kind
                    .as_ref()
                    .is_some_and(|k| k.is_query())
            {
                let (line, col) = self.line_col(it.span.start);
                self.analysis.non_deterministic_calls.push(CallLocation {
                    line,
                    col,
                    detail: "new Date()".to_string(),
                });
            }
        }
        walk::walk_new_expression(self, it);
    }

    // 3. Detect `throw new Error(...)` in Convex function handlers
    fn visit_throw_statement(&mut self, it: &ThrowStatement<'a>) {
        if !self.function_builder_stack.is_empty() {
            if let Expression::NewExpression(new_expr) = &it.argument {
                if let Expression::Identifier(ident) = &new_expr.callee {
                    if ident.name.as_str() == "Error" {
                        let (line, col) = self.line_col(it.span.start);
                        self.analysis.throw_generic_errors.push(CallLocation {
                            line,
                            col,
                            detail: "throw new Error() — use ConvexError for client-visible errors"
                                .to_string(),
                        });
                    }
                }
            }
        }
        walk::walk_throw_statement(self, it);
    }

    // 14. Track unexported function declarations and variable declarations
    //     that appear outside of export statements for unexported_function_count
    fn visit_statement(&mut self, it: &Statement<'a>) {
        match it {
            // Standalone function declaration (not inside export)
            Statement::FunctionDeclaration(_) => {
                if self.current_export_names.is_empty() && self.function_builder_stack.is_empty() {
                    self.analysis.unexported_function_count += 1;
                }
            }
            // Standalone variable declaration with arrow/function expression init
            Statement::VariableDeclaration(var_decl) => {
                if self.current_export_names.is_empty() && self.function_builder_stack.is_empty() {
                    for declarator in &var_decl.declarations {
                        if let Some(init) = &declarator.init {
                            if matches!(
                                init,
                                Expression::ArrowFunctionExpression(_)
                                    | Expression::FunctionExpression(_)
                            ) {
                                self.analysis.unexported_function_count += 1;
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        walk::walk_statement(self, it);
    }
}
