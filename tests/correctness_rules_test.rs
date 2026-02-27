use convex_doctor::rules::context::analyze_file;
use convex_doctor::rules::correctness::*;
use convex_doctor::rules::Rule;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn test_unwaited_promise() {
    let analysis = analyze_file(Path::new("tests/fixtures/bad_patterns.ts")).unwrap();
    let rule = UnwaitedPromise;
    let diagnostics = rule.check(&analysis);
    // ctx.scheduler.runAfter without await should be caught
    assert!(!diagnostics.is_empty(), "Should detect unwaited ctx calls");
}

#[test]
fn test_deprecated_api() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("deprecated.ts");
    std::fs::write(
        &path,
        r#"
import { mutation } from "convex/server";
import { v } from "convex/values";

export const create = mutation({
  args: { count: v.bigint() },
  handler: async (ctx, args) => {},
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = DeprecatedApi;
    let diagnostics = rule.check(&analysis);
    assert!(
        diagnostics.iter().any(|d| d.message.contains("v.bigint()")),
        "Should detect deprecated v.bigint()"
    );
}

#[test]
fn test_v_any_not_flagged_as_deprecated() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("any_validator.ts");
    std::fs::write(
        &path,
        r#"
import { mutation } from "convex/server";
import { v } from "convex/values";

export const create = mutation({
  args: { data: v.any() },
  handler: async (ctx, args) => {},
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = DeprecatedApi;
    let diagnostics = rule.check(&analysis);
    assert!(
        diagnostics.is_empty(),
        "v.any() should NOT be flagged as deprecated (it's handled by security/generic-mutation-args)"
    );
}

#[test]
fn test_old_function_syntax() {
    let analysis = analyze_file(Path::new("tests/fixtures/old_syntax.ts")).unwrap();
    assert!(
        !analysis.old_syntax_functions.is_empty(),
        "Should detect old function syntax"
    );
}

#[test]
fn test_old_function_syntax_rule_emits_diagnostics() {
    let analysis = analyze_file(Path::new("tests/fixtures/old_syntax.ts")).unwrap();
    let rule = OldFunctionSyntax;
    let diagnostics = rule.check(&analysis);
    assert!(
        !diagnostics.is_empty(),
        "old-function-syntax rule should emit diagnostics when old syntax is present"
    );
}

#[test]
fn test_unwaited_promise_not_flagged_when_awaited_via_variable() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("deferred_await.ts");
    std::fs::write(
        &path,
        r#"
import { mutation } from "convex/server";
import { v } from "convex/values";

export const create = mutation({
  args: { body: v.string() },
  handler: async (ctx, args) => {
    const pending = ctx.db.insert("messages", { body: args.body });
    return await pending;
  },
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = UnwaitedPromise;
    let diagnostics = rule.check(&analysis);
    assert!(
        diagnostics.is_empty(),
        "Should not flag promise assigned then awaited later"
    );
}

#[test]
fn test_unwaited_promise_not_flagged_when_returned() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("returned_promise.ts");
    std::fs::write(
        &path,
        r#"
import { mutation } from "convex/server";
import { v } from "convex/values";

export const create = mutation({
  args: { body: v.string() },
  handler: async (ctx, args) => {
    return ctx.db.insert("messages", { body: args.body });
  },
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = UnwaitedPromise;
    let diagnostics = rule.check(&analysis);
    assert!(
        diagnostics.is_empty(),
        "Returning a promise from async handler should not be flagged"
    );
}

#[test]
fn test_unwaited_promise_not_flagged_when_awaited_via_promise_all() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("awaited_via_promise_all.ts");
    std::fs::write(
        &path,
        r#"
import { mutation } from "convex/server";
import { v } from "convex/values";

export const create = mutation({
  args: { body: v.string() },
  handler: async (ctx, args) => {
    const pending = ctx.db.insert("messages", { body: args.body });
    await Promise.all([pending]);
  },
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = UnwaitedPromise;
    let diagnostics = rule.check(&analysis);
    assert!(
        diagnostics.is_empty(),
        "Should not flag promise that is awaited through Promise.all"
    );
}

#[test]
fn test_unwaited_promise_not_flagged_when_awaited_through_alias() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("awaited_through_alias.ts");
    std::fs::write(
        &path,
        r#"
import { mutation } from "convex/server";
import { v } from "convex/values";

export const create = mutation({
  args: { body: v.string() },
  handler: async (ctx, args) => {
    const pending = ctx.db.insert("messages", { body: args.body });
    const wrapped = pending;
    return await wrapped;
  },
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = UnwaitedPromise;
    let diagnostics = rule.check(&analysis);
    assert!(
        diagnostics.is_empty(),
        "Should not flag promise that is awaited through identifier aliasing"
    );
}

#[test]
fn test_db_in_action_rule_detects_direct_db_access() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("db_in_action.ts");
    std::fs::write(
        &path,
        r#"
import { action } from "convex/server";

export const bad = action({
  args: {},
  handler: async (ctx) => {
    return await ctx.db.query("messages").collect();
  },
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = DbInAction;
    let diagnostics = rule.check(&analysis);
    assert!(
        !diagnostics.is_empty(),
        "db-in-action rule should detect direct ctx.db usage inside actions"
    );
}
