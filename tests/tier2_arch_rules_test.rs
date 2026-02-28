use convex_doctor::diagnostic::Severity;
use convex_doctor::rules::architecture::*;
use convex_doctor::rules::{
    CallLocation, ConvexFunction, CtxCall, FileAnalysis, FunctionKind, Rule,
};

// ── ActionWithoutScheduling ──────────────────────────────────────────

#[test]
fn action_without_scheduling_flags_run_action_in_mutation() {
    let analysis = FileAnalysis {
        file_path: "convex/tasks.ts".to_string(),
        ctx_calls: vec![CtxCall {
            chain: "ctx.runAction".to_string(),
            line: 10,
            col: 5,

            is_awaited: true,
            is_returned: false,
            assigned_to: None,
            enclosing_function_kind: Some(FunctionKind::Mutation),
            enclosing_function_id: None,
            enclosing_function_name: None,
            first_arg_chain: None,
            enclosing_function_has_internal_secret: false,
        }],
        ..Default::default()
    };
    let rule = ActionWithoutScheduling;
    let diagnostics = rule.check(&analysis);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].severity, Severity::Info);
    assert!(diagnostics[0].message.contains("ctx.runAction"));
    assert!(diagnostics[0].help.contains("scheduler.runAfter"));
}

#[test]
fn action_without_scheduling_flags_run_action_in_internal_mutation() {
    let analysis = FileAnalysis {
        file_path: "convex/tasks.ts".to_string(),
        ctx_calls: vec![CtxCall {
            chain: "ctx.runAction".to_string(),
            line: 10,
            col: 5,

            is_awaited: true,
            is_returned: false,
            assigned_to: None,
            enclosing_function_kind: Some(FunctionKind::InternalMutation),
            enclosing_function_id: None,
            enclosing_function_name: None,
            first_arg_chain: None,
            enclosing_function_has_internal_secret: false,
        }],
        ..Default::default()
    };
    let rule = ActionWithoutScheduling;
    let diagnostics = rule.check(&analysis);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn action_without_scheduling_ignores_run_action_in_action() {
    let analysis = FileAnalysis {
        file_path: "convex/tasks.ts".to_string(),
        ctx_calls: vec![CtxCall {
            chain: "ctx.runAction".to_string(),
            line: 10,
            col: 5,

            is_awaited: true,
            is_returned: false,
            assigned_to: None,
            enclosing_function_kind: Some(FunctionKind::Action),
            enclosing_function_id: None,
            enclosing_function_name: None,
            first_arg_chain: None,
            enclosing_function_has_internal_secret: false,
        }],
        ..Default::default()
    };
    let rule = ActionWithoutScheduling;
    let diagnostics = rule.check(&analysis);
    assert!(diagnostics.is_empty());
}

#[test]
fn action_without_scheduling_ignores_run_query_in_mutation() {
    let analysis = FileAnalysis {
        file_path: "convex/tasks.ts".to_string(),
        ctx_calls: vec![CtxCall {
            chain: "ctx.runQuery".to_string(),
            line: 10,
            col: 5,

            is_awaited: true,
            is_returned: false,
            assigned_to: None,
            enclosing_function_kind: Some(FunctionKind::Mutation),
            enclosing_function_id: None,
            enclosing_function_name: None,
            first_arg_chain: None,
            enclosing_function_has_internal_secret: false,
        }],
        ..Default::default()
    };
    let rule = ActionWithoutScheduling;
    let diagnostics = rule.check(&analysis);
    assert!(diagnostics.is_empty());
}

// ── NoConvexError ────────────────────────────────────────────────────

#[test]
fn no_convex_error_flags_throw_generic_errors() {
    let analysis = FileAnalysis {
        file_path: "convex/users.ts".to_string(),
        throw_generic_errors: vec![
            CallLocation {
                line: 5,
                col: 3,
                detail: String::new(),
            },
            CallLocation {
                line: 15,
                col: 3,
                detail: String::new(),
            },
        ],
        ..Default::default()
    };
    let rule = NoConvexError;
    let diagnostics = rule.check(&analysis);
    assert_eq!(diagnostics.len(), 2);
    assert_eq!(diagnostics[0].severity, Severity::Info);
    assert!(diagnostics[0].message.contains("throw new Error"));
    assert!(diagnostics[0].help.contains("ConvexError"));
}

#[test]
fn no_convex_error_clean_when_no_generic_throws() {
    let analysis = FileAnalysis {
        file_path: "convex/users.ts".to_string(),
        throw_generic_errors: vec![],
        ..Default::default()
    };
    let rule = NoConvexError;
    let diagnostics = rule.check(&analysis);
    assert!(diagnostics.is_empty());
}

// ── MixedFunctionTypes ───────────────────────────────────────────────

fn make_function(name: &str, kind: FunctionKind) -> ConvexFunction {
    ConvexFunction {
        name: name.to_string(),
        kind,
        has_args_validator: true,
        has_any_validator_in_args: false,
        arg_names: vec![],
        has_return_validator: false,
        has_auth_check: false,
        has_internal_secret: false,
        is_intentionally_public: false,
        handler_line_count: 10,
        span_line: 1,
        span_col: 1,
    }
}

#[test]
fn mixed_function_types_flags_public_and_internal() {
    let analysis = FileAnalysis {
        file_path: "convex/mixed.ts".to_string(),
        functions: vec![
            make_function("getItems", FunctionKind::Query),
            make_function("internalGet", FunctionKind::InternalQuery),
        ],
        ..Default::default()
    };
    let rule = MixedFunctionTypes;
    let diagnostics = rule.check(&analysis);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].severity, Severity::Info);
    assert!(diagnostics[0].message.contains("public and internal"));
}

#[test]
fn mixed_function_types_clean_when_all_public() {
    let analysis = FileAnalysis {
        file_path: "convex/public.ts".to_string(),
        functions: vec![
            make_function("getItems", FunctionKind::Query),
            make_function("updateItem", FunctionKind::Mutation),
        ],
        ..Default::default()
    };
    let rule = MixedFunctionTypes;
    let diagnostics = rule.check(&analysis);
    assert!(diagnostics.is_empty());
}

#[test]
fn mixed_function_types_clean_when_all_internal() {
    let analysis = FileAnalysis {
        file_path: "convex/internal.ts".to_string(),
        functions: vec![
            make_function("getItems", FunctionKind::InternalQuery),
            make_function("doStuff", FunctionKind::InternalMutation),
        ],
        ..Default::default()
    };
    let rule = MixedFunctionTypes;
    let diagnostics = rule.check(&analysis);
    assert!(diagnostics.is_empty());
}

#[test]
fn mixed_function_types_clean_when_no_functions() {
    let analysis = FileAnalysis {
        file_path: "convex/empty.ts".to_string(),
        functions: vec![],
        ..Default::default()
    };
    let rule = MixedFunctionTypes;
    let diagnostics = rule.check(&analysis);
    assert!(diagnostics.is_empty());
}

#[test]
fn mixed_function_types_emits_only_one_diagnostic() {
    let analysis = FileAnalysis {
        file_path: "convex/mixed.ts".to_string(),
        functions: vec![
            make_function("get1", FunctionKind::Query),
            make_function("get2", FunctionKind::Query),
            make_function("internal1", FunctionKind::InternalQuery),
            make_function("internal2", FunctionKind::InternalMutation),
        ],
        ..Default::default()
    };
    let rule = MixedFunctionTypes;
    let diagnostics = rule.check(&analysis);
    assert_eq!(
        diagnostics.len(),
        1,
        "Should emit exactly one diagnostic per file"
    );
}

// ── NoHelperFunctions ────────────────────────────────────────────────

fn make_large_function(name: &str, kind: FunctionKind, lines: u32) -> ConvexFunction {
    ConvexFunction {
        name: name.to_string(),
        kind,
        has_args_validator: true,
        has_any_validator_in_args: false,
        arg_names: vec![],
        has_return_validator: false,
        has_auth_check: false,
        has_internal_secret: false,
        is_intentionally_public: false,
        handler_line_count: lines,
        span_line: 1,
        span_col: 1,
    }
}

#[test]
fn no_helper_functions_flags_many_large_handlers_no_helpers() {
    let analysis = FileAnalysis {
        file_path: "convex/big.ts".to_string(),
        functions: vec![
            make_large_function("handler1", FunctionKind::Query, 20),
            make_large_function("handler2", FunctionKind::Mutation, 25),
            make_large_function("handler3", FunctionKind::Action, 30),
        ],
        unexported_function_count: 0,
        ..Default::default()
    };
    let rule = NoHelperFunctions;
    let diagnostics = rule.check(&analysis);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].severity, Severity::Info);
    assert!(diagnostics[0].message.contains("3 handlers"));
    assert!(diagnostics[0].help.contains("helper functions"));
}

#[test]
fn no_helper_functions_clean_when_helpers_exist() {
    let analysis = FileAnalysis {
        file_path: "convex/big.ts".to_string(),
        functions: vec![
            make_large_function("handler1", FunctionKind::Query, 20),
            make_large_function("handler2", FunctionKind::Mutation, 25),
            make_large_function("handler3", FunctionKind::Action, 30),
        ],
        unexported_function_count: 2,
        ..Default::default()
    };
    let rule = NoHelperFunctions;
    let diagnostics = rule.check(&analysis);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_helper_functions_clean_when_fewer_than_three_large() {
    let analysis = FileAnalysis {
        file_path: "convex/small.ts".to_string(),
        functions: vec![
            make_large_function("handler1", FunctionKind::Query, 20),
            make_large_function("handler2", FunctionKind::Mutation, 25),
        ],
        unexported_function_count: 0,
        ..Default::default()
    };
    let rule = NoHelperFunctions;
    let diagnostics = rule.check(&analysis);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_helper_functions_ignores_small_handlers() {
    let analysis = FileAnalysis {
        file_path: "convex/small.ts".to_string(),
        functions: vec![
            make_large_function("handler1", FunctionKind::Query, 10),
            make_large_function("handler2", FunctionKind::Mutation, 5),
            make_large_function("handler3", FunctionKind::Action, 15),
        ],
        unexported_function_count: 0,
        ..Default::default()
    };
    let rule = NoHelperFunctions;
    let diagnostics = rule.check(&analysis);
    // handler_line_count must be > 15, not >= 15, so handler3 at exactly 15 doesn't count
    assert!(diagnostics.is_empty());
}

#[test]
fn no_helper_functions_emits_only_one_diagnostic() {
    let analysis = FileAnalysis {
        file_path: "convex/big.ts".to_string(),
        functions: vec![
            make_large_function("h1", FunctionKind::Query, 20),
            make_large_function("h2", FunctionKind::Mutation, 25),
            make_large_function("h3", FunctionKind::Action, 30),
            make_large_function("h4", FunctionKind::Query, 40),
        ],
        unexported_function_count: 0,
        ..Default::default()
    };
    let rule = NoHelperFunctions;
    let diagnostics = rule.check(&analysis);
    assert_eq!(
        diagnostics.len(),
        1,
        "Should emit exactly one diagnostic per file"
    );
    assert!(diagnostics[0].message.contains("4 handlers"));
}

#[test]
fn no_helper_functions_does_not_flag_simple_crud_files() {
    let analysis = FileAnalysis {
        file_path: "convex/crud.ts".to_string(),
        functions: vec![
            make_large_function("listUsers", FunctionKind::Query, 20),
            make_large_function("getUser", FunctionKind::Query, 25),
            make_large_function("createUser", FunctionKind::Mutation, 30),
        ],
        unexported_function_count: 0,
        ..Default::default()
    };
    let rule = NoHelperFunctions;
    let diagnostics = rule.check(&analysis);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_helper_functions_flags_crud_prefixed_helper_like_names() {
    let analysis = FileAnalysis {
        file_path: "convex/mixed_crud_like.ts".to_string(),
        functions: vec![
            make_large_function("getCachedUser", FunctionKind::Query, 20),
            make_large_function("listHelpers", FunctionKind::Query, 25),
            make_large_function("createUser", FunctionKind::Mutation, 30),
        ],
        unexported_function_count: 0,
        ..Default::default()
    };
    let rule = NoHelperFunctions;
    let diagnostics = rule.check(&analysis);
    assert!(
        !diagnostics.is_empty(),
        "CRUD-prefixed helper-like names should not suppress this rule"
    );
}

// ── DeepFunctionChain ────────────────────────────────────────────────

fn make_action_ctx_call(chain: &str, line: u32) -> CtxCall {
    CtxCall {
        chain: chain.to_string(),
        line,
        col: 5,
        is_awaited: true,
        is_returned: false,
        assigned_to: None,
        enclosing_function_kind: Some(FunctionKind::Action),
        enclosing_function_id: Some("anonymous_action".to_string()),
        enclosing_function_name: Some("anonymous_action".to_string()),
        first_arg_chain: None,
        enclosing_function_has_internal_secret: false,
    }
}

#[test]
fn deep_function_chain_flags_four_or_more_run_calls() {
    let analysis = FileAnalysis {
        file_path: "convex/orchestrate.ts".to_string(),
        ctx_calls: vec![
            make_action_ctx_call("ctx.runQuery", 5),
            make_action_ctx_call("ctx.runMutation", 10),
            make_action_ctx_call("ctx.runQuery", 15),
            make_action_ctx_call("ctx.runMutation", 20),
        ],
        ..Default::default()
    };
    let rule = DeepFunctionChain;
    let diagnostics = rule.check(&analysis);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].severity, Severity::Warning);
    assert!(diagnostics[0].message.contains("4 ctx.run*"));
    assert!(diagnostics[0].help.contains("batching"));
}

#[test]
fn deep_function_chain_skips_internal_secret_actions() {
    let analysis = FileAnalysis {
        file_path: "convex/chunked.ts".to_string(),
        ctx_calls: vec![
            CtxCall {
                chain: "ctx.runQuery".to_string(),
                line: 5,
                col: 5,

                is_awaited: true,
                is_returned: false,
                assigned_to: None,
                enclosing_function_kind: Some(FunctionKind::Action),
                enclosing_function_id: None,
                enclosing_function_name: Some("syncUsers".to_string()),
                first_arg_chain: None,
                enclosing_function_has_internal_secret: true,
            },
            CtxCall {
                chain: "ctx.runMutation".to_string(),
                line: 10,
                col: 5,

                is_awaited: true,
                is_returned: false,
                assigned_to: None,
                enclosing_function_kind: Some(FunctionKind::Action),
                enclosing_function_id: Some("syncUsers".to_string()),
                enclosing_function_name: Some("syncUsers".to_string()),
                first_arg_chain: None,
                enclosing_function_has_internal_secret: true,
            },
            CtxCall {
                chain: "ctx.runQuery".to_string(),
                line: 15,
                col: 5,

                is_awaited: true,
                is_returned: false,
                assigned_to: None,
                enclosing_function_kind: Some(FunctionKind::Action),
                enclosing_function_id: Some("syncUsers".to_string()),
                enclosing_function_name: Some("syncUsers".to_string()),
                first_arg_chain: None,
                enclosing_function_has_internal_secret: true,
            },
            CtxCall {
                chain: "ctx.runMutation".to_string(),
                line: 20,
                col: 5,

                is_awaited: true,
                is_returned: false,
                assigned_to: None,
                enclosing_function_kind: Some(FunctionKind::Action),
                enclosing_function_id: Some("syncUsers".to_string()),
                enclosing_function_name: Some("syncUsers".to_string()),
                first_arg_chain: None,
                enclosing_function_has_internal_secret: true,
            },
        ],
        ..Default::default()
    };
    let rule = DeepFunctionChain;
    let diagnostics = rule.check(&analysis);
    assert!(diagnostics.is_empty());
}

#[test]
fn deep_function_chain_clean_with_three_calls() {
    let analysis = FileAnalysis {
        file_path: "convex/orchestrate.ts".to_string(),
        ctx_calls: vec![
            make_action_ctx_call("ctx.runQuery", 5),
            make_action_ctx_call("ctx.runMutation", 10),
            make_action_ctx_call("ctx.runQuery", 15),
        ],
        ..Default::default()
    };
    let rule = DeepFunctionChain;
    let diagnostics = rule.check(&analysis);
    assert!(diagnostics.is_empty());
}

#[test]
fn deep_function_chain_skips_sync_backfill_named_actions() {
    let analysis = FileAnalysis {
        file_path: "convex/chunked.ts".to_string(),
        ctx_calls: vec![
            CtxCall {
                chain: "ctx.runQuery".to_string(),
                line: 5,
                col: 5,

                is_awaited: true,
                is_returned: false,
                assigned_to: None,
                enclosing_function_kind: Some(FunctionKind::Action),
                enclosing_function_id: Some("backfillUsers".to_string()),
                enclosing_function_name: Some("backfillUsers".to_string()),
                first_arg_chain: None,
                enclosing_function_has_internal_secret: false,
            },
            CtxCall {
                chain: "ctx.runMutation".to_string(),
                line: 10,
                col: 5,

                is_awaited: true,
                is_returned: false,
                assigned_to: None,
                enclosing_function_kind: Some(FunctionKind::Action),
                enclosing_function_id: Some("backfillUsers".to_string()),
                enclosing_function_name: Some("backfillUsers".to_string()),
                first_arg_chain: None,
                enclosing_function_has_internal_secret: false,
            },
            CtxCall {
                chain: "ctx.runQuery".to_string(),
                line: 15,
                col: 5,

                is_awaited: true,
                is_returned: false,
                assigned_to: None,
                enclosing_function_kind: Some(FunctionKind::Action),
                enclosing_function_id: Some("backfillUsers".to_string()),
                enclosing_function_name: Some("backfillUsers".to_string()),
                first_arg_chain: None,
                enclosing_function_has_internal_secret: false,
            },
            CtxCall {
                chain: "ctx.runMutation".to_string(),
                line: 20,
                col: 5,

                is_awaited: true,
                is_returned: false,
                assigned_to: None,
                enclosing_function_kind: Some(FunctionKind::Action),
                enclosing_function_id: Some("backfillUsers".to_string()),
                enclosing_function_name: Some("backfillUsers".to_string()),
                first_arg_chain: None,
                enclosing_function_has_internal_secret: false,
            },
        ],
        ..Default::default()
    };
    let rule = DeepFunctionChain;
    let diagnostics = rule.check(&analysis);
    assert!(diagnostics.is_empty());
}

#[test]
fn deep_function_chain_ignores_run_action_calls() {
    let analysis = FileAnalysis {
        file_path: "convex/orchestrate.ts".to_string(),
        ctx_calls: vec![
            make_action_ctx_call("ctx.runQuery", 5),
            make_action_ctx_call("ctx.runMutation", 10),
            make_action_ctx_call("ctx.runAction", 15),
            make_action_ctx_call("ctx.runAction", 20),
        ],
        ..Default::default()
    };
    let rule = DeepFunctionChain;
    let diagnostics = rule.check(&analysis);
    // Only 2 runQuery/runMutation calls, runAction calls don't count
    assert!(diagnostics.is_empty());
}

#[test]
fn deep_function_chain_ignores_calls_in_mutations() {
    let analysis = FileAnalysis {
        file_path: "convex/tasks.ts".to_string(),
        ctx_calls: vec![
            CtxCall {
                chain: "ctx.runQuery".to_string(),
                line: 5,
                col: 5,

                is_awaited: true,
                is_returned: false,
                assigned_to: None,
                enclosing_function_kind: Some(FunctionKind::Mutation),
                enclosing_function_id: None,
                enclosing_function_name: None,
                first_arg_chain: None,
                enclosing_function_has_internal_secret: false,
            },
            CtxCall {
                chain: "ctx.runQuery".to_string(),
                line: 10,
                col: 5,

                is_awaited: true,
                is_returned: false,
                assigned_to: None,
                enclosing_function_kind: Some(FunctionKind::Mutation),
                enclosing_function_id: None,
                enclosing_function_name: None,
                first_arg_chain: None,
                enclosing_function_has_internal_secret: false,
            },
            CtxCall {
                chain: "ctx.runQuery".to_string(),
                line: 15,
                col: 5,

                is_awaited: true,
                is_returned: false,
                assigned_to: None,
                enclosing_function_kind: Some(FunctionKind::Mutation),
                enclosing_function_id: None,
                enclosing_function_name: None,
                first_arg_chain: None,
                enclosing_function_has_internal_secret: false,
            },
            CtxCall {
                chain: "ctx.runQuery".to_string(),
                line: 20,
                col: 5,

                is_awaited: true,
                is_returned: false,
                assigned_to: None,
                enclosing_function_kind: Some(FunctionKind::Mutation),
                enclosing_function_id: None,
                enclosing_function_name: None,
                first_arg_chain: None,
                enclosing_function_has_internal_secret: false,
            },
        ],
        ..Default::default()
    };
    let rule = DeepFunctionChain;
    let diagnostics = rule.check(&analysis);
    assert!(diagnostics.is_empty());
}

#[test]
fn deep_function_chain_counts_internal_actions_too() {
    let analysis = FileAnalysis {
        file_path: "convex/orchestrate.ts".to_string(),
        ctx_calls: vec![
            CtxCall {
                chain: "ctx.runQuery".to_string(),
                line: 5,
                col: 5,

                is_awaited: true,
                is_returned: false,
                assigned_to: None,
                enclosing_function_kind: Some(FunctionKind::InternalAction),
                enclosing_function_id: Some("internalAction".to_string()),
                enclosing_function_name: None,
                first_arg_chain: None,
                enclosing_function_has_internal_secret: false,
            },
            CtxCall {
                chain: "ctx.runMutation".to_string(),
                line: 10,
                col: 5,

                is_awaited: true,
                is_returned: false,
                assigned_to: None,
                enclosing_function_kind: Some(FunctionKind::InternalAction),
                enclosing_function_id: Some("internalAction".to_string()),
                enclosing_function_name: None,
                first_arg_chain: None,
                enclosing_function_has_internal_secret: false,
            },
            CtxCall {
                chain: "ctx.runQuery".to_string(),
                line: 15,
                col: 5,

                is_awaited: true,
                is_returned: false,
                assigned_to: None,
                enclosing_function_kind: Some(FunctionKind::InternalAction),
                enclosing_function_id: Some("internalAction".to_string()),
                enclosing_function_name: None,
                first_arg_chain: None,
                enclosing_function_has_internal_secret: false,
            },
            CtxCall {
                chain: "ctx.runMutation".to_string(),
                line: 20,
                col: 5,

                is_awaited: true,
                is_returned: false,
                assigned_to: None,
                enclosing_function_kind: Some(FunctionKind::InternalAction),
                enclosing_function_id: Some("internalAction".to_string()),
                enclosing_function_name: None,
                first_arg_chain: None,
                enclosing_function_has_internal_secret: false,
            },
        ],
        ..Default::default()
    };
    let rule = DeepFunctionChain;
    let diagnostics = rule.check(&analysis);
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("4 ctx.run*"));
}

#[test]
fn deep_function_chain_emits_only_one_diagnostic() {
    let analysis = FileAnalysis {
        file_path: "convex/orchestrate.ts".to_string(),
        ctx_calls: vec![
            make_action_ctx_call("ctx.runQuery", 5),
            make_action_ctx_call("ctx.runMutation", 10),
            make_action_ctx_call("ctx.runQuery", 15),
            make_action_ctx_call("ctx.runMutation", 20),
            make_action_ctx_call("ctx.runQuery", 25),
        ],
        ..Default::default()
    };
    let rule = DeepFunctionChain;
    let diagnostics = rule.check(&analysis);
    assert_eq!(
        diagnostics.len(),
        1,
        "Should emit exactly one diagnostic per file"
    );
    assert!(diagnostics[0].message.contains("5 ctx.run*"));
}
