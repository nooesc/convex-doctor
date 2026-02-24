# New Rules Implementation Plan (35 Rules from Issue #1)

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement all 35 new lint rules from GitHub issue #1 across 7 categories, bringing the total from 30 to 65 rules.

**Architecture:** Each rule is a struct implementing the `Rule` trait. Rules read from `FileAnalysis` (populated by the Oxc visitor in `context.rs`) or `ProjectContext` (populated in `engine.rs`). New rules follow existing patterns exactly. Some rules need new fields in `FileAnalysis`/`ProjectContext` and new detection logic in the visitor. Cross-file rules (e.g., comparing filter fields against schema indexes) use `ProjectContext` as the bridge.

**Tech Stack:** Rust, Oxc v0.115 (AST parser/visitor), tempfile (tests)

**Cargo:** Use `/Users/coler/.cargo/bin/cargo` for all commands.

---

## Rule Summary

| # | Rule ID | Severity | Needs Visitor | Needs New Field |
|---|---------|----------|---------------|-----------------|
| 1 | correctness/query-side-effect | Error | No | No |
| 2 | correctness/mutation-in-query | Error | No | No |
| 3 | correctness/cron-uses-public-api | Error | Yes | Yes |
| 4 | correctness/node-query-mutation | Error | No | No |
| 5 | security/missing-table-id | Warning | Yes | Yes |
| 6 | security/missing-http-auth | Error | No | No |
| 7 | security/conditional-function-export | Error | Yes | Yes |
| 8 | perf/missing-index-on-foreign-key | Warning | Yes | Yes |
| 9 | perf/action-from-client | Warning | No | No |
| 10 | perf/collect-then-filter | Warning | Yes | Yes |
| 11 | security/generic-mutation-args | Warning | Yes | Yes |
| 12 | security/overly-broad-patch | Warning | Yes | Yes |
| 13 | security/http-missing-cors | Warning | Yes | Yes |
| 14 | schema/missing-index-for-query | Warning | Yes | Yes |
| 15 | arch/action-without-scheduling | Info | No | No |
| 16 | arch/no-convex-error | Info | Yes | Yes |
| 17 | schema/too-many-indexes | Info | No | No |
| 18 | arch/mixed-function-types | Info | No | No |
| 19 | config/missing-generated-code | Warning | No | ProjectContext |
| 20 | config/outdated-node-version | Warning | No | ProjectContext |
| 21 | config/missing-tsconfig | Info | No | ProjectContext |
| 22 | correctness/scheduler-return-ignored | Info | No | No |
| 23 | correctness/generated-code-modified | Error | No | ProjectContext |
| 24 | correctness/non-deterministic-in-query | Warning | Yes | Yes |
| 25 | correctness/replace-vs-patch | Info | No | No |
| 26 | arch/no-helper-functions | Info | Yes | Yes |
| 27 | arch/deep-function-chain | Warning | No | No |
| 28 | schema/missing-search-index-filter | Info | Yes | Yes |
| 29 | schema/optional-field-no-default-handling | Warning | Yes | Yes |
| 30 | perf/large-document-write | Info | Yes | Yes |
| 31 | perf/no-pagination-for-list | Warning | No | No* |
| 32 | client/mutation-in-render | Error | Yes | Yes |
| 33 | client/unhandled-loading-state | Warning | Yes | Yes |
| 34 | client/action-instead-of-mutation | Info | Yes | Yes |
| 35 | client/missing-convex-provider | Error | Yes | Yes |

*Rule 31 uses existing `collect_calls` correlated with `functions`.

---

### Task 1: Infrastructure — Severity::Info + New Fields

**Files:**
- Modify: `src/diagnostic.rs`
- Modify: `src/scoring.rs`
- Modify: `src/rules/mod.rs`
- Modify: `src/engine.rs`
- Modify: `src/project.rs`
- Modify: `src/reporter.rs` (if severity display needs update)
- Test: `tests/scoring_test.rs`

**Step 1: Add `Severity::Info` variant**

In `src/diagnostic.rs`, add `Info` to the `Severity` enum:

```rust
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
```

**Step 2: Add `Category::ClientSide` variant**

In `src/diagnostic.rs`, add `ClientSide` to the `Category` enum with weight 1.0:

```rust
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
            // ... existing ...
            Category::ClientSide => 1.0,
        }
    }
}

impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // ... existing ...
            Category::ClientSide => write!(f, "Client-Side"),
        }
    }
}
```

**Step 3: Update scoring for Info severity**

In `src/scoring.rs`, Info diagnostics contribute 0 penalty:

```rust
// In compute_score(), the penalty calculation:
let penalty = match d.severity {
    Severity::Error => 3.0,
    Severity::Warning => 1.0,
    Severity::Info => 0.0,
};
```

**Step 4: Add new fields to `FileAnalysis`**

In `src/rules/mod.rs`, add these fields to `FileAnalysis`:

```rust
#[derive(Debug, Default)]
pub struct FileAnalysis {
    // ... existing fields ...

    // New fields for new rules
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

    // Client-side detection
    pub convex_hook_calls: Vec<ConvexHookCall>,
    pub has_convex_provider: bool,
    pub is_component_file: bool,
}
```

Add new structs:

```rust
#[derive(Debug, Clone)]
pub struct HttpRoute {
    pub method: String,
    pub path: String,
    pub line: u32,
}

#[derive(Debug, Clone)]
pub struct SchemaIdField {
    pub field_name: String,
    pub table_ref: String,
    pub line: u32,
    pub col: u32,
}

#[derive(Debug, Clone)]
pub struct FilterField {
    pub field_name: String,
    pub line: u32,
    pub col: u32,
}

#[derive(Debug, Clone)]
pub struct SearchIndexDef {
    pub table: String,
    pub name: String,
    pub has_filter_fields: bool,
    pub line: u32,
}

#[derive(Debug, Clone)]
pub struct ConvexHookCall {
    pub hook_name: String,
    pub line: u32,
    pub col: u32,
    pub in_render_body: bool,
}
```

**Step 5: Add `has_any_validator_in_args` to `ConvexFunction`**

```rust
pub struct ConvexFunction {
    // ... existing fields ...
    pub has_any_validator_in_args: bool,
}
```

Update `FunctionBuilder` accordingly.

**Step 6: Extend `ProjectContext`**

```rust
pub struct ProjectContext {
    // ... existing fields ...
    pub has_generated_dir: bool,
    pub has_tsconfig: bool,
    pub node_version_from_config: Option<String>,
    pub generated_files_modified: bool,
    pub all_index_definitions: Vec<IndexDef>,
    pub all_schema_id_fields: Vec<SchemaIdField>,
}
```

**Step 7: Update engine.rs to populate new ProjectContext fields**

In `engine.rs run()`, after collecting all analyses, populate the new ProjectContext fields:

```rust
let project_ctx = ProjectContext {
    // ... existing fields ...
    has_generated_dir: project.convex_dir.join("_generated").is_dir(),
    has_tsconfig: project.convex_dir.join("tsconfig.json").exists(),
    node_version_from_config: read_node_version_from_convex_json(path),
    generated_files_modified: check_generated_files_modified(path),
    all_index_definitions: analyses.iter()
        .flat_map(|a| a.index_definitions.clone())
        .collect(),
    all_schema_id_fields: analyses.iter()
        .flat_map(|a| a.schema_id_fields.clone())
        .collect(),
};
```

Add helper functions:

```rust
fn read_node_version_from_convex_json(root: &Path) -> Option<String> {
    let path = root.join("convex.json");
    let contents = std::fs::read_to_string(&path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&contents).ok()?;
    json.get("node")
        .and_then(|v| v.get("version"))
        .or_else(|| json.get("nodeVersion"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn check_generated_files_modified(root: &Path) -> bool {
    std::process::Command::new("git")
        .args(["status", "--porcelain", "convex/_generated"])
        .current_dir(root)
        .output()
        .ok()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false)
}
```

**Step 8: Add `client` module**

Create `src/rules/client.rs` (empty for now, will be populated in Task 9):

```rust
// Client-side rules will be added in Task 9
```

Add `pub mod client;` to `src/rules/mod.rs`.

**Step 9: Run tests**

Run: `/Users/coler/.cargo/bin/cargo test 2>&1`
Expected: All existing 82 tests still pass (new fields are Default, no behavioral changes)

**Step 10: Run clippy**

Run: `/Users/coler/.cargo/bin/cargo clippy --all-targets 2>&1`
Expected: No warnings

**Step 11: Commit**

```bash
git add -A && git commit -m "feat: add Info severity, ClientSide category, and new FileAnalysis fields for 35 new rules"
```

---

### Task 2: Visitor Extensions

**Files:**
- Modify: `src/rules/context.rs`
- Test: `tests/analyzer_test.rs` (add tests for new detection patterns)

This task extends the Oxc visitor to populate all new FileAnalysis fields. Each detection pattern is a self-contained addition.

**Step 1: Write failing test — cron API refs**

In `tests/analyzer_test.rs`, add:

```rust
#[test]
fn test_detect_cron_api_refs() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("crons.ts");
    std::fs::write(&path, r#"
import { cronJobs } from "convex/server";
import { api, internal } from "./_generated/api";

const crons = cronJobs();
crons.interval("cleanup", { hours: 1 }, api.tasks.cleanup);
crons.interval("sync", { hours: 1 }, internal.tasks.sync);
export default crons;
"#).unwrap();

    let analysis = analyze_file(&path).unwrap();
    assert_eq!(analysis.cron_api_refs.len(), 1, "Should detect api.* ref in cron");
    assert!(analysis.cron_api_refs[0].detail.contains("api.tasks.cleanup"));
}
```

Run: `/Users/coler/.cargo/bin/cargo test test_detect_cron_api_refs 2>&1`
Expected: FAIL (field exists but empty)

**Step 2: Implement cron API ref detection in visitor**

In `visit_call_expression`, detect cron `.interval()`, `.hourly()`, `.daily()`, `.weekly()`, `.monthly()`, `.cron()` methods where an argument starts with `api.`:

```rust
// Detect cron job definitions using api.* (should use internal.*)
if let Expression::StaticMemberExpression(mem) = &it.callee {
    let method = mem.property.name.as_str();
    if matches!(method, "interval" | "hourly" | "daily" | "weekly" | "monthly" | "cron") {
        for arg in &it.arguments {
            if let Some(expr) = arg.as_expression() {
                if let Some(chain) = Self::resolve_member_chain(expr) {
                    if chain.starts_with("api.") {
                        self.analysis.cron_api_refs.push(CallLocation {
                            line, col,
                            detail: chain,
                        });
                    }
                }
            }
        }
    }
}
```

**Step 3: Run test to verify pass**

Run: `/Users/coler/.cargo/bin/cargo test test_detect_cron_api_refs 2>&1`
Expected: PASS

**Step 4: Write failing test — non-deterministic calls**

```rust
#[test]
fn test_detect_non_deterministic_in_query() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("nd.ts");
    std::fs::write(&path, r#"
import { query } from "convex/server";
export const get = query({
  handler: async (ctx) => {
    const r = Math.random();
    const d = new Date();
    return r;
  },
});
"#).unwrap();

    let analysis = analyze_file(&path).unwrap();
    assert!(analysis.non_deterministic_calls.len() >= 2,
        "Should detect Math.random() and new Date() in query: got {}",
        analysis.non_deterministic_calls.len());
}
```

**Step 5: Implement non-deterministic call detection**

In `visit_call_expression`, detect `Math.random()` in query context (alongside existing Date.now() detection):

```rust
// Detect Math.random() in query functions
if let Expression::StaticMemberExpression(mem) = &it.callee {
    if mem.property.name.as_str() == "random" {
        if let Expression::Identifier(ident) = &mem.object {
            if ident.name.as_str() == "Math"
                && self.current_function_kind.as_ref().is_some_and(|k| k.is_query())
            {
                self.analysis.non_deterministic_calls.push(CallLocation {
                    line, col,
                    detail: "Math.random()".to_string(),
                });
            }
        }
    }
}
```

In `visit_new_expression` (add this visitor method), detect `new Date()` in query context:

```rust
fn visit_new_expression(&mut self, it: &NewExpression<'a>) {
    let (line, col) = self.line_col(it.span.start);
    if let Expression::Identifier(ident) = &it.callee {
        if ident.name.as_str() == "Date"
            && self.current_function_kind.as_ref().is_some_and(|k| k.is_query())
        {
            self.analysis.non_deterministic_calls.push(CallLocation {
                line, col,
                detail: "new Date()".to_string(),
            });
        }
    }
    walk::walk_new_expression(self, it);
}
```

**Step 6: Run test to verify pass**

Run: `/Users/coler/.cargo/bin/cargo test test_detect_non_deterministic 2>&1`

**Step 7: Write failing test — throw generic errors**

```rust
#[test]
fn test_detect_throw_generic_error() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("errors.ts");
    std::fs::write(&path, r#"
import { mutation } from "convex/server";
export const create = mutation({
  args: {},
  handler: async (ctx) => {
    throw new Error("Something went wrong");
  },
});
"#).unwrap();

    let analysis = analyze_file(&path).unwrap();
    assert_eq!(analysis.throw_generic_errors.len(), 1,
        "Should detect throw new Error()");
}
```

**Step 8: Implement throw generic error detection**

Add `visit_throw_statement`:

```rust
fn visit_throw_statement(&mut self, it: &ThrowStatement<'a>) {
    if !self.function_builder_stack.is_empty() {
        if let Expression::NewExpression(new_expr) = &it.argument {
            if let Expression::Identifier(ident) = &new_expr.callee {
                if ident.name.as_str() == "Error" {
                    let (line, col) = self.line_col(it.span.start);
                    self.analysis.throw_generic_errors.push(CallLocation {
                        line, col,
                        detail: "throw new Error(...)".to_string(),
                    });
                }
            }
        }
    }
    walk::walk_throw_statement(self, it);
}
```

**Step 9: Implement remaining visitor patterns**

Add detection for the following (each follows the same pattern as above — detect AST pattern, push to appropriate FileAnalysis field):

**a) `generic_id_validators`** — In the arg property processing (where `builder.has_args_validator = true`), check if any arg value is `v.id()` without a string argument:

```rust
// Inside args processing, after extracting arg_name:
if let Expression::CallExpression(call) = &arg.value {
    if let Expression::StaticMemberExpression(mem) = &call.callee {
        if let Expression::Identifier(v_ident) = &mem.object {
            if v_ident.name.as_str() == "v" && mem.property.name.as_str() == "id" {
                if call.arguments.is_empty() {
                    // v.id() without table name
                    let (line, col) = self.line_col(call.span.start);
                    self.analysis.generic_id_validators.push(CallLocation {
                        line, col,
                        detail: format!("Arg '{}' uses v.id() without table name", arg_name),
                    });
                }
            }
        }
    }
}
```

**b) `has_any_validator_in_args`** — In the same arg processing section, detect `v.any()`:

```rust
if let Expression::CallExpression(call) = &arg.value {
    if let Expression::StaticMemberExpression(mem) = &call.callee {
        if let Expression::Identifier(v_ident) = &mem.object {
            if v_ident.name.as_str() == "v" && mem.property.name.as_str() == "any" {
                builder.has_any_validator_in_args = true;
            }
        }
    }
}
```

**c) `conditional_exports`** — In `visit_export_named_declaration`, check if any declarator init is a ConditionalExpression wrapping function constructors:

```rust
// In visit_export_named_declaration, before walking:
if let Some(Declaration::VariableDeclaration(var_decl)) = &it.declaration {
    for declarator in &var_decl.declarations {
        if let Some(Expression::ConditionalExpression(cond)) = &declarator.init {
            let has_process_env = Self::contains_process_env(&cond.test);
            let has_fn_call = Self::get_function_kind(&cond.consequent).is_some()
                || Self::get_function_kind(&cond.alternate).is_some();
            if has_process_env && has_fn_call {
                let (line, col) = self.line_col(cond.span.start);
                self.analysis.conditional_exports.push(CallLocation {
                    line, col,
                    detail: "Conditional function export based on environment variable".to_string(),
                });
            }
        }
    }
}
```

Add helper:

```rust
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
        Expression::UnaryExpression(unary) => Self::contains_process_env(&unary.argument),
        Expression::LogicalExpression(log) => {
            Self::contains_process_env(&log.left) || Self::contains_process_env(&log.right)
        }
        _ => false,
    }
}
```

**d) `raw_arg_patches`** — In ctx call detection, when chain is "ctx.db.patch" and second argument resolves to "args":

```rust
if chain.starts_with("ctx.db.patch") {
    if let Some(second_arg) = it.arguments.get(1) {
        if let Some(expr) = second_arg.as_expression() {
            if let Some(arg_chain) = Self::resolve_member_chain(expr) {
                if arg_chain == "args" {
                    self.analysis.raw_arg_patches.push(CallLocation {
                        line, col,
                        detail: "ctx.db.patch(id, args) passes raw client args".to_string(),
                    });
                }
            }
        }
    }
}
```

**e) `http_routes`** — Detect httpRouter .route() calls:

```rust
// In visit_call_expression, detect .route() on httpRouter
if let Expression::StaticMemberExpression(mem) = &it.callee {
    if mem.property.name.as_str() == "route" && it.arguments.len() >= 1 {
        if let Some(Argument::ObjectExpression(obj)) = it.arguments.first() {
            let mut method = String::new();
            let mut path_val = String::new();
            for prop in &obj.properties {
                if let ObjectPropertyKind::ObjectProperty(p) = prop {
                    if let Some(name) = p.key.static_name() {
                        match name.as_ref() {
                            "method" => {
                                if let Expression::StringLiteral(s) = &p.value {
                                    method = s.value.as_str().to_string();
                                }
                            }
                            "path" | "pathPrefix" => {
                                if let Expression::StringLiteral(s) = &p.value {
                                    path_val = s.value.as_str().to_string();
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            if !method.is_empty() {
                self.analysis.http_routes.push(HttpRoute { method, path: path_val, line });
            }
        }
    }
}
```

**f) `schema_id_fields`** — In defineTable object processing, detect v.id("table") field definitions. This is in the validator/schema context — when processing object properties inside defineTable arguments, look for `v.id("tableName")` calls:

```rust
// When visiting v.id("tableName") calls in schema context
if let Expression::StaticMemberExpression(mem) = &it.callee {
    if let Expression::Identifier(ident) = &mem.object {
        if ident.name.as_str() == "v" && mem.property.name.as_str() == "id" {
            if let Some(first_arg) = it.arguments.first() {
                if let Some(Expression::StringLiteral(table_name)) = first_arg.as_expression() {
                    // If we're inside a defineTable > v.object context, record the field
                    let (line, col) = self.line_col(it.span.start);
                    self.analysis.schema_id_fields.push(SchemaIdField {
                        field_name: String::new(), // Will be populated by parent context
                        table_ref: table_name.value.as_str().to_string(),
                        line, col,
                    });
                }
            }
        }
    }
}
```

**g) `filter_field_names`** — Detect `.filter(q => q.eq(q.field("name"), ...))` patterns:

```rust
// In visit_call_expression, when we see .filter() on ctx.db chains
if let Expression::StaticMemberExpression(mem) = &it.callee {
    if mem.property.name.as_str() == "filter" {
        // Walk the callback to find q.field("name") patterns
        if let Some(first_arg) = it.arguments.first() {
            if let Some(expr) = first_arg.as_expression() {
                self.extract_filter_field_names(expr);
            }
        }
    }
}
```

Add `extract_filter_field_names` helper that recursively looks for `q.field("fieldName")` and `q.eq(q.field("fieldName"), ...)` patterns.

**h) `search_index_definitions`** — Detect `.searchIndex("name", {...})` calls in schema:

```rust
if let Expression::StaticMemberExpression(mem) = &it.callee {
    if mem.property.name.as_str() == "searchIndex" && it.arguments.len() >= 2 {
        let table = Self::get_index_table_id(&it.callee).unwrap_or_default();
        let name = it.arguments.first().and_then(|a| {
            a.as_expression().and_then(|e| {
                if let Expression::StringLiteral(s) = e { Some(s.value.as_str().to_string()) }
                else { None }
            })
        });
        let has_filter_fields = it.arguments.get(1).map(|a| {
            // Check if the config object has a filterFields property
            if let Some(Expression::ObjectExpression(obj)) = a.as_expression() {
                obj.properties.iter().any(|p| {
                    if let ObjectPropertyKind::ObjectProperty(prop) = p {
                        prop.key.static_name().is_some_and(|n| n.as_ref() == "filterFields")
                    } else { false }
                })
            } else { false }
        }).unwrap_or(false);
        if let Some(name) = name {
            self.analysis.search_index_definitions.push(SearchIndexDef {
                table, name, has_filter_fields, line,
            });
        }
    }
}
```

**i) `optional_schema_fields`** — Detect `v.optional()` calls in schema:

```rust
if let Expression::StaticMemberExpression(mem) = &it.callee {
    if let Expression::Identifier(ident) = &mem.object {
        if ident.name.as_str() == "v" && mem.property.name.as_str() == "optional" {
            let (line, col) = self.line_col(it.span.start);
            self.analysis.optional_schema_fields.push(CallLocation {
                line, col,
                detail: "v.optional(...)".to_string(),
            });
        }
    }
}
```

**j) `large_writes`** — Detect ctx.db.insert/replace with large inline objects (>20 properties):

```rust
if chain.starts_with("ctx.db.insert") || chain.starts_with("ctx.db.replace") {
    if let Some(arg) = it.arguments.last() {
        if let Some(Expression::ObjectExpression(obj)) = arg.as_expression() {
            if obj.properties.len() > 20 {
                self.analysis.large_writes.push(CallLocation {
                    line, col,
                    detail: format!("{} properties in inline object", obj.properties.len()),
                });
            }
        }
    }
}
```

**k) `unexported_function_count`** — Track function declarations not in exports:

Add `visit_function_declaration` to increment a counter for non-exported functions:

```rust
fn visit_function_declaration(&mut self, it: &Function<'a>) {
    if self.current_export_names.is_empty() {
        self.analysis.unexported_function_count += 1;
    }
    walk::walk_function(self, it, /* flags */);
}
```

Also count unexported arrow functions assigned to const declarations.

**l) Client-side hook detection** — Detect useMutation, useQuery, useAction, ConvexProvider:

```rust
// In visit_call_expression:
if let Expression::Identifier(ident) = &it.callee {
    let name = ident.name.as_str();
    if matches!(name, "useMutation" | "useQuery" | "useAction") {
        let (line, col) = self.line_col(it.span.start);
        self.analysis.convex_hook_calls.push(ConvexHookCall {
            hook_name: name.to_string(),
            line, col,
            in_render_body: self.function_builder_stack.is_empty() && self.loop_depth == 0,
        });
    }
}

// In visit_import_declaration, check for ConvexProvider:
if source.contains("convex/react") && specifiers.iter().any(|s| s == "ConvexProvider" || s == "ConvexProviderWithClerk" || s == "ConvexReactClient") {
    self.analysis.has_convex_provider = true;
}
```

**m) Detect collect-then-filter pattern** — Track variable assignments from .collect() and .filter() on same variable:

This needs `visit_variable_declarator` to record which variables hold `.collect()` results, then in subsequent `.filter()` calls check if the object is one of those variables. Simplified approach: track in a `collect_variables: HashSet<String>` on the visitor.

**Step 10: Write comprehensive tests for each new detection pattern**

Write tests for: generic_id_validators, conditional_exports, raw_arg_patches, http_routes, schema_id_fields, filter_field_names, search_index_definitions, large_writes, collect_variable_filters.

**Step 11: Run all tests + clippy**

Run: `/Users/coler/.cargo/bin/cargo test 2>&1 && /Users/coler/.cargo/bin/cargo clippy --all-targets 2>&1`

**Step 12: Commit**

```bash
git add -A && git commit -m "feat: extend visitor with detection patterns for 35 new rules"
```

---

### Task 3: Tier 1 Correctness Rules (4 rules)

**Files:**
- Modify: `src/rules/correctness.rs`
- Create: `tests/tier1_correctness_test.rs`

**Rules:**
1. `correctness/query-side-effect` — Error: ctx.db writes or scheduler calls in queries
2. `correctness/mutation-in-query` — Error: ctx.runMutation from query
3. `correctness/cron-uses-public-api` — Error: cron jobs using api.* instead of internal.*
4. `correctness/node-query-mutation` — Error: query/mutation in "use node" files

**Step 1: Write failing tests for all 4 rules**

```rust
use convex_doctor::rules::context::analyze_file;
use convex_doctor::rules::correctness::*;
use convex_doctor::rules::Rule;
use tempfile::TempDir;

#[test]
fn test_query_side_effect_db_write() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("q.ts");
    std::fs::write(&path, r#"
import { query } from "convex/server";
export const bad = query({
  handler: async (ctx) => {
    await ctx.db.insert("logs", { msg: "hi" });
    return "done";
  },
});
"#).unwrap();
    let analysis = analyze_file(&path).unwrap();
    let diags = QuerySideEffect.check(&analysis);
    assert!(!diags.is_empty(), "Should detect db write in query");
}

#[test]
fn test_query_side_effect_scheduler() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("q2.ts");
    std::fs::write(&path, r#"
import { query } from "convex/server";
import { internal } from "./_generated/api";
export const bad = query({
  handler: async (ctx) => {
    await ctx.scheduler.runAfter(0, internal.tasks.cleanup);
    return [];
  },
});
"#).unwrap();
    let analysis = analyze_file(&path).unwrap();
    let diags = QuerySideEffect.check(&analysis);
    assert!(!diags.is_empty(), "Should detect scheduler in query");
}

#[test]
fn test_mutation_in_query() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("q3.ts");
    std::fs::write(&path, r#"
import { query } from "convex/server";
import { internal } from "./_generated/api";
export const bad = query({
  handler: async (ctx) => {
    await ctx.runMutation(internal.tasks.cleanup);
    return [];
  },
});
"#).unwrap();
    let analysis = analyze_file(&path).unwrap();
    let diags = MutationInQuery.check(&analysis);
    assert!(!diags.is_empty(), "Should detect runMutation in query");
}

#[test]
fn test_cron_uses_public_api() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("crons.ts");
    std::fs::write(&path, r#"
import { cronJobs } from "convex/server";
import { api, internal } from "./_generated/api";
const crons = cronJobs();
crons.interval("cleanup", { hours: 1 }, api.tasks.cleanup);
crons.interval("sync", { hours: 1 }, internal.tasks.sync);
export default crons;
"#).unwrap();
    let analysis = analyze_file(&path).unwrap();
    let diags = CronUsesPublicApi.check(&analysis);
    assert_eq!(diags.len(), 1, "Should detect api.* in cron, not internal.*");
}

#[test]
fn test_node_query_mutation() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("node.ts");
    std::fs::write(&path, r#"
"use node";
import { query, mutation, action } from "convex/server";
export const bad1 = query({ handler: async (ctx) => {} });
export const bad2 = mutation({ args: {}, handler: async (ctx) => {} });
export const ok = action({ args: {}, handler: async (ctx) => {} });
"#).unwrap();
    let analysis = analyze_file(&path).unwrap();
    let diags = NodeQueryMutation.check(&analysis);
    assert_eq!(diags.len(), 2, "Should flag query and mutation in use node file, not action");
}
```

**Step 2: Run tests to verify they fail**

Run: `/Users/coler/.cargo/bin/cargo test --test tier1_correctness_test 2>&1`
Expected: FAIL (structs don't exist yet)

**Step 3: Implement all 4 rules**

In `src/rules/correctness.rs`:

```rust
pub struct QuerySideEffect;
impl Rule for QuerySideEffect {
    fn id(&self) -> &'static str { "correctness/query-side-effect" }
    fn category(&self) -> Category { Category::Correctness }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        const WRITE_PREFIXES: &[&str] = &[
            "ctx.db.insert", "ctx.db.patch", "ctx.db.replace", "ctx.db.delete",
            "ctx.scheduler",
        ];
        analysis.ctx_calls.iter()
            .filter(|c| {
                c.enclosing_function_kind.as_ref().is_some_and(|k| k.is_query())
                    && WRITE_PREFIXES.iter().any(|p| c.chain.starts_with(p))
            })
            .map(|c| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Error,
                category: self.category(),
                message: format!("`{}` in a query function — queries must be read-only", c.chain),
                help: "Queries must be deterministic and side-effect-free. Move writes to a mutation.".to_string(),
                file: analysis.file_path.clone(),
                line: c.line, column: c.col,
            })
            .collect()
    }
}

pub struct MutationInQuery;
impl Rule for MutationInQuery {
    fn id(&self) -> &'static str { "correctness/mutation-in-query" }
    fn category(&self) -> Category { Category::Correctness }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis.ctx_calls.iter()
            .filter(|c| {
                c.chain.starts_with("ctx.runMutation")
                    && c.enclosing_function_kind.as_ref().is_some_and(|k| k.is_query())
            })
            .map(|c| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Error,
                category: self.category(),
                message: "`ctx.runMutation` called from a query function".to_string(),
                help: "Queries cannot call mutations. Move this logic to a mutation or action.".to_string(),
                file: analysis.file_path.clone(),
                line: c.line, column: c.col,
            })
            .collect()
    }
}

pub struct CronUsesPublicApi;
impl Rule for CronUsesPublicApi {
    fn id(&self) -> &'static str { "correctness/cron-uses-public-api" }
    fn category(&self) -> Category { Category::Correctness }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis.cron_api_refs.iter()
            .map(|c| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Error,
                category: self.category(),
                message: format!("Cron job uses public API reference `{}`", c.detail),
                help: "Use `internal.*` instead of `api.*` in cron job definitions. Cron functions should be internal.".to_string(),
                file: analysis.file_path.clone(),
                line: c.line, column: c.col,
            })
            .collect()
    }
}

pub struct NodeQueryMutation;
impl Rule for NodeQueryMutation {
    fn id(&self) -> &'static str { "correctness/node-query-mutation" }
    fn category(&self) -> Category { Category::Correctness }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        if !analysis.has_use_node { return vec![]; }
        analysis.functions.iter()
            .filter(|f| matches!(f.kind,
                FunctionKind::Query | FunctionKind::Mutation
                | FunctionKind::InternalQuery | FunctionKind::InternalMutation))
            .map(|f| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Error,
                category: self.category(),
                message: format!("{} `{}` in a \"use node\" file", f.kind_str(), f.name),
                help: "Only actions can use the Node.js runtime. Queries and mutations must use the Convex runtime.".to_string(),
                file: analysis.file_path.clone(),
                line: f.span_line, column: f.span_col,
            })
            .collect()
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `/Users/coler/.cargo/bin/cargo test --test tier1_correctness_test 2>&1`
Expected: All PASS

**Step 5: Run full test suite + clippy**

**Step 6: Commit**

```bash
git add -A && git commit -m "feat: add correctness rules — query-side-effect, mutation-in-query, cron-uses-public-api, node-query-mutation"
```

---

### Task 4: Tier 1 Security Rules (3 rules)

**Files:**
- Modify: `src/rules/security.rs`
- Create: `tests/tier1_security_test.rs`

**Rules:**
1. `security/missing-table-id` — Warning: v.id() without explicit table name
2. `security/missing-http-auth` — Error: httpAction without auth check
3. `security/conditional-function-export` — Error: conditional export with process.env

**Step 1: Write failing tests**

```rust
#[test]
fn test_missing_table_id() {
    // Fixture: args: { docId: v.id() } — no table name
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("ids.ts");
    std::fs::write(&path, r#"
import { mutation } from "convex/server";
import { v } from "convex/values";
export const remove = mutation({
  args: { docId: v.id() },
  handler: async (ctx, args) => { await ctx.db.delete(args.docId); },
});
"#).unwrap();
    let analysis = analyze_file(&path).unwrap();
    let diags = MissingTableId.check(&analysis);
    assert!(!diags.is_empty(), "Should warn about v.id() without table");
}

#[test]
fn test_missing_table_id_ok() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("ids_ok.ts");
    std::fs::write(&path, r#"
import { mutation } from "convex/server";
import { v } from "convex/values";
export const remove = mutation({
  args: { docId: v.id("documents") },
  handler: async (ctx, args) => { await ctx.db.delete(args.docId); },
});
"#).unwrap();
    let analysis = analyze_file(&path).unwrap();
    let diags = MissingTableId.check(&analysis);
    assert!(diags.is_empty(), "v.id('table') should not be flagged");
}

#[test]
fn test_missing_http_auth() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("http.ts");
    std::fs::write(&path, r#"
import { httpAction } from "convex/server";
export const webhook = httpAction(async (ctx, request) => {
  return new Response("ok");
});
"#).unwrap();
    let analysis = analyze_file(&path).unwrap();
    let diags = MissingHttpAuth.check(&analysis);
    assert!(!diags.is_empty(), "Should detect httpAction without auth check");
}

#[test]
fn test_conditional_function_export() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("cond.ts");
    std::fs::write(&path, r#"
import { mutation, internalMutation } from "convex/server";
export const create = process.env.IS_DEV
  ? mutation({ args: {}, handler: async (ctx) => {} })
  : internalMutation({ args: {}, handler: async (ctx) => {} });
"#).unwrap();
    let analysis = analyze_file(&path).unwrap();
    let diags = ConditionalFunctionExport.check(&analysis);
    assert!(!diags.is_empty(), "Should detect conditional function export");
}
```

**Step 2-4: Implement rules, run tests, verify pass**

Each rule is a simple filter on the corresponding FileAnalysis field.

**Step 5: Commit**

```bash
git add -A && git commit -m "feat: add security rules — missing-table-id, missing-http-auth, conditional-function-export"
```

---

### Task 5: Tier 1 Perf + Tier 2 Easy Rules (5 rules)

**Files:**
- Modify: `src/rules/performance.rs`
- Modify: `src/rules/architecture.rs`
- Modify: `src/rules/security.rs`
- Create: `tests/tier2_easy_rules_test.rs`

**Rules:**
1. `perf/missing-index-on-foreign-key` — Warning: v.id() fields in schema without matching index
2. `perf/action-from-client` — Warning: public action exports
3. `security/generic-mutation-args` — Warning: v.any() in public mutation args
4. `arch/action-without-scheduling` — Info: mutation calling runAction instead of scheduler
5. `arch/no-convex-error` — Info: throw new Error() instead of ConvexError

**Step 1: Write failing tests for all 5**

Each test creates a fixture file, analyzes it, and checks the rule produces expected diagnostics.

`action-from-client`:
```rust
#[test]
fn test_action_from_client() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("a.ts");
    std::fs::write(&path, r#"
import { action, internalAction } from "convex/server";
export const publicAction = action({ args: {}, handler: async (ctx) => {} });
export const internalAct = internalAction({ args: {}, handler: async (ctx) => {} });
"#).unwrap();
    let analysis = analyze_file(&path).unwrap();
    let diags = ActionFromClient.check(&analysis);
    assert_eq!(diags.len(), 1, "Should flag public action, not internal");
}
```

`generic-mutation-args`:
```rust
#[test]
fn test_generic_mutation_args() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("g.ts");
    std::fs::write(&path, r#"
import { mutation } from "convex/server";
import { v } from "convex/values";
export const update = mutation({
  args: { data: v.any() },
  handler: async (ctx, args) => {},
});
"#).unwrap();
    let analysis = analyze_file(&path).unwrap();
    let diags = GenericMutationArgs.check(&analysis);
    assert!(!diags.is_empty(), "Should flag v.any() in public mutation args");
}
```

**Step 2-4: Implement rules, run tests, verify pass**

`missing-index-on-foreign-key` uses `check_project` to cross-reference `all_schema_id_fields` against `all_index_definitions` in ProjectContext.

**Step 5: Commit**

```bash
git add -A && git commit -m "feat: add rules — missing-index-on-foreign-key, action-from-client, generic-mutation-args, action-without-scheduling, no-convex-error"
```

---

### Task 6: Tier 2 Complex Rules (4 rules)

**Files:**
- Modify: `src/rules/performance.rs`
- Modify: `src/rules/security.rs`
- Modify: `src/rules/schema.rs`
- Create: `tests/tier2_complex_rules_test.rs`

**Rules:**
1. `perf/collect-then-filter` — Warning: collect() then .filter() on same variable
2. `security/overly-broad-patch` — Warning: ctx.db.patch(id, args) with raw client args
3. `security/http-missing-cors` — Warning: HTTP routes without OPTIONS handler
4. `schema/missing-index-for-query` — Warning: filter field without matching index

**Step 1: Write failing tests**

`collect-then-filter`:
```rust
#[test]
fn test_collect_then_filter() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("cf.ts");
    std::fs::write(&path, r#"
import { query } from "convex/server";
export const list = query({
  handler: async (ctx) => {
    const all = await ctx.db.query("items").collect();
    const filtered = all.filter(item => item.active);
    return filtered;
  },
});
"#).unwrap();
    let analysis = analyze_file(&path).unwrap();
    let diags = CollectThenFilter.check(&analysis);
    assert!(!diags.is_empty(), "Should detect collect-then-filter pattern");
}
```

`http-missing-cors`:
```rust
#[test]
fn test_http_missing_cors() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("http.ts");
    std::fs::write(&path, r#"
import { httpRouter } from "convex/server";
const http = httpRouter();
http.route({ method: "GET", path: "/api/data", handler: getHandler });
http.route({ method: "POST", path: "/api/data", handler: postHandler });
export default http;
"#).unwrap();
    let analysis = analyze_file(&path).unwrap();
    let diags = HttpMissingCors.check(&analysis);
    assert!(!diags.is_empty(), "Should warn about missing OPTIONS route");
}
```

**Step 2-4: Implement rules, test, verify**

`collect-then-filter` reads from `collect_variable_filters` field.
`overly-broad-patch` reads from `raw_arg_patches` field.
`http-missing-cors` checks `http_routes` for paths that have GET/POST but no OPTIONS.
`missing-index-for-query` uses `check_project` to cross-reference `filter_field_names` against `all_index_definitions`.

**Step 5: Commit**

```bash
git add -A && git commit -m "feat: add rules — collect-then-filter, overly-broad-patch, http-missing-cors, missing-index-for-query"
```

---

### Task 7: Tier 3 Correctness + Config Rules (7 rules)

**Files:**
- Modify: `src/rules/correctness.rs`
- Modify: `src/rules/configuration.rs`
- Create: `tests/tier3_correctness_config_test.rs`

**Rules:**
1. `correctness/scheduler-return-ignored` — Info: scheduler call return not captured
2. `correctness/generated-code-modified` — Error: _generated/ files modified
3. `correctness/non-deterministic-in-query` — Warning: Math.random()/new Date() in queries
4. `correctness/replace-vs-patch` — Info: ctx.db.replace usage
5. `config/missing-generated-code` — Warning: missing _generated/ directory
6. `config/outdated-node-version` — Warning: old Node version in convex.json
7. `config/missing-tsconfig` — Info: missing convex/tsconfig.json

**Step 1: Write tests, implement, verify**

`scheduler-return-ignored`:
```rust
pub struct SchedulerReturnIgnored;
impl Rule for SchedulerReturnIgnored {
    fn id(&self) -> &'static str { "correctness/scheduler-return-ignored" }
    fn category(&self) -> Category { Category::Correctness }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis.ctx_calls.iter()
            .filter(|c| {
                (c.chain.starts_with("ctx.scheduler.runAfter")
                    || c.chain.starts_with("ctx.scheduler.runAt"))
                    && c.assigned_to.is_none()
            })
            .map(|c| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Info,
                category: self.category(),
                message: format!("`{}` return value not captured", c.chain),
                help: "Capture the returned scheduled function ID if you need to cancel or monitor it.".to_string(),
                file: analysis.file_path.clone(),
                line: c.line, column: c.col,
            })
            .collect()
    }
}
```

`non-deterministic-in-query` reads from `non_deterministic_calls`.
`replace-vs-patch` filters `ctx_calls` for `ctx.db.replace`.
`generated-code-modified`, `missing-generated-code`, `outdated-node-version`, `missing-tsconfig` all use `check_project` with ProjectContext fields.

**Step 2: Commit**

```bash
git add -A && git commit -m "feat: add rules — scheduler-return-ignored, generated-code-modified, non-deterministic-in-query, replace-vs-patch, config rules"
```

---

### Task 8: Tier 3 Schema + Arch + Perf Rules (8 rules)

**Files:**
- Modify: `src/rules/schema.rs`
- Modify: `src/rules/architecture.rs`
- Modify: `src/rules/performance.rs`
- Create: `tests/tier3_schema_arch_perf_test.rs`

**Rules:**
1. `schema/too-many-indexes` — Info: 8+ indexes per table
2. `schema/missing-search-index-filter` — Info: search index without filterFields
3. `schema/optional-field-no-default-handling` — Warning: many optional fields (simplified)
4. `arch/mixed-function-types` — Info: public + internal exports in same file
5. `arch/no-helper-functions` — Info: 3+ handlers >15 lines with no helpers
6. `arch/deep-function-chain` — Warning: 3+ alternating runQuery/runMutation in action
7. `perf/large-document-write` — Info: large inline objects in db writes
8. `perf/no-pagination-for-list` — Warning: public query with collect() but no pagination

**Implementation notes:**

`too-many-indexes` groups `index_definitions` by table, warns if count >= 8.

`mixed-function-types` checks if `functions` contains both public and internal kinds.

`no-helper-functions` checks if `exported_function_count >= 3` and handler_line_count > 15 for all, and `unexported_function_count == 0`.

`deep-function-chain` counts alternating runQuery/runMutation calls in actions.

`no-pagination-for-list` correlates public query functions with `collect_calls` — if a file has public queries and collect calls but no `.take()`/`.paginate()` patterns.

`optional-field-no-default-handling` (simplified): warns if a schema file has many `v.optional()` calls as an informational reminder.

**Step 1-4: Write tests, implement, verify, commit**

```bash
git add -A && git commit -m "feat: add rules — too-many-indexes, search-index-filter, optional-fields, mixed-types, no-helpers, deep-chain, large-write, no-pagination"
```

---

### Task 9: Client-Side Rules (4 rules)

**Files:**
- Modify: `src/rules/client.rs`
- Modify: `src/project.rs` (extend file discovery to scan outside convex/)
- Create: `tests/client_rules_test.rs`

**Rules:**
1. `client/mutation-in-render` — Error: useMutation result called in component body
2. `client/unhandled-loading-state` — Warning: useQuery without undefined check
3. `client/action-instead-of-mutation` — Info: useAction when useMutation would suffice
4. `client/missing-convex-provider` — Error: Convex hooks without provider import

**Important:** Client-side rules scan files OUTSIDE `convex/` directory. Extend `ProjectInfo::discover_files()` to also walk `src/`, `app/`, and `pages/` directories for `.tsx`/`.jsx` files. Only run client rules on files outside `convex/`.

**Implementation approach:**
- `mutation-in-render`: Detect `useMutation()` calls where the returned mutate function is called at the top level of a component (not inside useEffect, onClick, etc.). Simplified: warn if file imports `useMutation` from `convex/react`.
- `unhandled-loading-state`: Detect `useQuery()` result used in expressions without `=== undefined` or `?` check nearby. Simplified: warn if `useQuery` is imported and results are accessed without conditional.
- `action-instead-of-mutation`: Detect `useAction()` imports. Info-level suggestion to consider useMutation.
- `missing-convex-provider`: Detect Convex hook imports (`useQuery`, `useMutation`, `useAction`) in files that don't also import or reference `ConvexProvider`.

**Simplified implementations for v1:**
These rules use a heuristic approach (import-based detection) since full component flow analysis is out of scope for a lint tool without type information.

**Step 1-4: Write tests, implement, verify, commit**

```bash
git add -A && git commit -m "feat: add client-side rules — mutation-in-render, unhandled-loading-state, action-instead-of-mutation, missing-convex-provider"
```

---

### Task 10: Registration + Integration

**Files:**
- Modify: `src/rules/mod.rs` (register all 35 new rules in RuleRegistry)
- Modify: `src/engine.rs` (ensure cross-file rules work, client file scanning)
- Create: `tests/new_rules_integration_test.rs`
- Modify: `tests/e2e_test.rs` (add E2E test with new rules)

**Step 1: Register all 35 new rules in RuleRegistry**

In `RuleRegistry::new()`, add all new rules after existing ones:

```rust
// Tier 1 Correctness (4)
Box::new(correctness::QuerySideEffect),
Box::new(correctness::MutationInQuery),
Box::new(correctness::CronUsesPublicApi),
Box::new(correctness::NodeQueryMutation),
// Tier 1 Security (3)
Box::new(security::MissingTableId),
Box::new(security::MissingHttpAuth),
Box::new(security::ConditionalFunctionExport),
// Tier 1 Performance (1)
Box::new(performance::MissingIndexOnForeignKey),
// Tier 2
Box::new(performance::ActionFromClient),
Box::new(performance::CollectThenFilter),
Box::new(security::GenericMutationArgs),
Box::new(security::OverlyBroadPatch),
Box::new(security::HttpMissingCors),
Box::new(schema::MissingIndexForQuery),
Box::new(architecture::ActionWithoutScheduling),
Box::new(architecture::NoConvexError),
// Tier 3
Box::new(schema::TooManyIndexes),
Box::new(architecture::MixedFunctionTypes),
Box::new(configuration::MissingGeneratedCode),
Box::new(configuration::OutdatedNodeVersion),
Box::new(configuration::MissingTsconfig),
Box::new(correctness::SchedulerReturnIgnored),
Box::new(correctness::GeneratedCodeModified),
Box::new(correctness::NonDeterministicInQuery),
Box::new(correctness::ReplaceVsPatch),
Box::new(architecture::NoHelperFunctions),
Box::new(architecture::DeepFunctionChain),
Box::new(schema::MissingSearchIndexFilter),
Box::new(schema::OptionalFieldNoDefaultHandling),
Box::new(performance::LargeDocumentWrite),
Box::new(performance::NoPaginationForList),
// Client-Side (4)
Box::new(client::MutationInRender),
Box::new(client::UnhandledLoadingState),
Box::new(client::ActionInsteadOfMutation),
Box::new(client::MissingConvexProvider),
```

**Step 2: Add cross-file rule execution in engine**

Add a new `check_cross_file` method to `Rule` trait (default returns empty). In `engine.rs`, after per-file rules, run cross-file rules passing all analyses:

```rust
// In Rule trait:
fn check_cross_file(&self, _analysis: &FileAnalysis, _project: &ProjectContext) -> Vec<Diagnostic> {
    vec![]
}
```

Rules like `missing-index-for-query` and `missing-index-on-foreign-key` implement this.

**Step 3: Integration test**

Create a comprehensive test that sets up a sample project with files that trigger multiple new rules and verifies the expected diagnostics count and categories.

**Step 4: Update scoring test**

Add test verifying Info diagnostics don't affect score.

**Step 5: Run full test suite + clippy**

Run: `/Users/coler/.cargo/bin/cargo test 2>&1 && /Users/coler/.cargo/bin/cargo clippy --all-targets 2>&1`
Expected: All tests pass (should be ~120+ tests), no clippy warnings

**Step 6: Commit**

```bash
git add -A && git commit -m "feat: register all 35 new rules, add cross-file analysis, integration tests"
```

---

## Execution Notes

**Dependencies:** Tasks 1 and 2 must complete before Tasks 3-9. Tasks 3-9 are independent and can be parallelized. Task 10 depends on all others.

**Testing pattern:** Each rule test creates a tempfile fixture, runs `analyze_file()`, instantiates the rule, calls `.check()`, and asserts on diagnostics.

**Expected final state:** 65 total rules (30 existing + 35 new), ~120+ tests, all passing, clippy clean.

| Category | Before | After |
|----------|--------|-------|
| Security | 7 | 13 |
| Performance | 7 | 12 |
| Correctness | 7 | 15 |
| Schema | 4 | 8 |
| Architecture | 3 | 8 |
| Configuration | 2 | 5 |
| Client-Side | 0 | 4 |
| **Total** | **30** | **65** |
