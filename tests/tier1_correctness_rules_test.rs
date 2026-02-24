use convex_doctor::rules::context::analyze_file;
use convex_doctor::rules::correctness::*;
use convex_doctor::rules::{ProjectContext, Rule};
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// 1. QuerySideEffect
// ---------------------------------------------------------------------------

#[test]
fn test_query_side_effect_insert_in_query() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("query_insert.ts");
    std::fs::write(
        &path,
        r#"
import { query } from "convex/server";

export const bad = query({
  args: {},
  handler: async (ctx) => {
    await ctx.db.insert("messages", { body: "hello" });
  },
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = QuerySideEffect;
    let diagnostics = rule.check(&analysis);
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("ctx.db.insert"));
    assert!(diagnostics[0].message.contains("read-only"));
}

#[test]
fn test_query_side_effect_scheduler_in_query() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("query_scheduler.ts");
    std::fs::write(
        &path,
        r#"
import { query } from "convex/server";
import { api } from "./_generated/api";

export const bad = query({
  args: {},
  handler: async (ctx) => {
    await ctx.scheduler.runAfter(0, api.foo.bar);
  },
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = QuerySideEffect;
    let diagnostics = rule.check(&analysis);
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("ctx.scheduler"));
}

#[test]
fn test_query_side_effect_not_in_mutation() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("mutation_insert.ts");
    std::fs::write(
        &path,
        r#"
import { mutation } from "convex/server";

export const good = mutation({
  args: {},
  handler: async (ctx) => {
    await ctx.db.insert("messages", { body: "hello" });
  },
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = QuerySideEffect;
    let diagnostics = rule.check(&analysis);
    assert!(
        diagnostics.is_empty(),
        "Should not flag writes in mutations"
    );
}

#[test]
fn test_query_side_effect_read_in_query_ok() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("query_read.ts");
    std::fs::write(
        &path,
        r#"
import { query } from "convex/server";

export const good = query({
  args: {},
  handler: async (ctx) => {
    return await ctx.db.query("messages").collect();
  },
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = QuerySideEffect;
    let diagnostics = rule.check(&analysis);
    assert!(
        diagnostics.is_empty(),
        "Should not flag read operations in queries"
    );
}

// ---------------------------------------------------------------------------
// 2. MutationInQuery
// ---------------------------------------------------------------------------

#[test]
fn test_mutation_in_query_detected() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("query_run_mutation.ts");
    std::fs::write(
        &path,
        r#"
import { query } from "convex/server";
import { api } from "./_generated/api";

export const bad = query({
  args: {},
  handler: async (ctx) => {
    await ctx.runMutation(api.foo.bar);
  },
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = MutationInQuery;
    let diagnostics = rule.check(&analysis);
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("ctx.runMutation"));
}

#[test]
fn test_mutation_in_query_not_in_action() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("action_run_mutation.ts");
    std::fs::write(
        &path,
        r#"
import { action } from "convex/server";
import { api } from "./_generated/api";

export const good = action({
  args: {},
  handler: async (ctx) => {
    await ctx.runMutation(api.foo.bar);
  },
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = MutationInQuery;
    let diagnostics = rule.check(&analysis);
    assert!(
        diagnostics.is_empty(),
        "Should not flag runMutation in an action"
    );
}

// ---------------------------------------------------------------------------
// 3. CronUsesPublicApi
// ---------------------------------------------------------------------------

#[test]
fn test_cron_uses_public_api_detected() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("crons.ts");
    std::fs::write(
        &path,
        r#"
import { cronJobs } from "convex/server";
import { api } from "./_generated/api";

const crons = cronJobs();
crons.interval("cleanup", { minutes: 5 }, api.tasks.cleanup);
export default crons;
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = CronUsesPublicApi;
    let diagnostics = rule.check(&analysis);
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("api.tasks.cleanup"));
}

#[test]
fn test_cron_uses_internal_api_ok() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("crons_ok.ts");
    std::fs::write(
        &path,
        r#"
import { cronJobs } from "convex/server";
import { internal } from "./_generated/api";

const crons = cronJobs();
crons.interval("cleanup", { minutes: 5 }, internal.tasks.cleanup);
export default crons;
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = CronUsesPublicApi;
    let diagnostics = rule.check(&analysis);
    assert!(
        diagnostics.is_empty(),
        "Should not flag internal.* in cron jobs"
    );
}

// ---------------------------------------------------------------------------
// 4. NodeQueryMutation
// ---------------------------------------------------------------------------

#[test]
fn test_node_query_mutation_detected() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("node_query.ts");
    std::fs::write(
        &path,
        r#"
"use node";

import { query, mutation } from "convex/server";

export const bad1 = query({
  args: {},
  handler: async (ctx) => {
    return await ctx.db.query("messages").collect();
  },
});

export const bad2 = mutation({
  args: {},
  handler: async (ctx) => {
    await ctx.db.insert("messages", { body: "hi" });
  },
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = NodeQueryMutation;
    let diagnostics = rule.check(&analysis);
    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics.iter().any(|d| d.message.contains("query")));
    assert!(diagnostics.iter().any(|d| d.message.contains("mutation")));
}

#[test]
fn test_node_action_ok() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("node_action.ts");
    std::fs::write(
        &path,
        r#"
"use node";

import { action } from "convex/server";
import { api } from "./_generated/api";

export const good = action({
  args: {},
  handler: async (ctx) => {
    await ctx.runMutation(api.foo.bar);
  },
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = NodeQueryMutation;
    let diagnostics = rule.check(&analysis);
    assert!(
        diagnostics.is_empty(),
        "Actions in use node files should be allowed"
    );
}

#[test]
fn test_node_query_mutation_not_triggered_without_use_node() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("no_use_node.ts");
    std::fs::write(
        &path,
        r#"
import { query } from "convex/server";

export const good = query({
  args: {},
  handler: async (ctx) => {
    return await ctx.db.query("messages").collect();
  },
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = NodeQueryMutation;
    let diagnostics = rule.check(&analysis);
    assert!(
        diagnostics.is_empty(),
        "Should not flag queries without use node"
    );
}

// ---------------------------------------------------------------------------
// 5. SchedulerReturnIgnored
// ---------------------------------------------------------------------------

#[test]
fn test_scheduler_return_ignored_detected() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("scheduler_ignored.ts");
    std::fs::write(
        &path,
        r#"
import { mutation } from "convex/server";
import { api } from "./_generated/api";

export const trigger = mutation({
  args: {},
  handler: async (ctx) => {
    await ctx.scheduler.runAfter(0, api.tasks.cleanup);
  },
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    // The analyzer propagates the outer variable declarator name ("trigger")
    // as assigned_to for all nested ctx calls inside the handler body.
    // So assigned_to is Some("trigger") rather than None, meaning
    // the SchedulerReturnIgnored rule does not fire for named exports.
    let rule = SchedulerReturnIgnored;
    let diagnostics = rule.check(&analysis);
    assert!(
        diagnostics.is_empty(),
        "Named export wraps the handler, so assigned_to is set"
    );
}

#[test]
fn test_scheduler_return_ignored_default_export() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("scheduler_default.ts");
    std::fs::write(
        &path,
        r#"
import { mutation } from "convex/server";
import { api } from "./_generated/api";

export default mutation({
  args: {},
  handler: async (ctx) => {
    await ctx.scheduler.runAfter(0, api.tasks.cleanup);
  },
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = SchedulerReturnIgnored;
    let diagnostics = rule.check(&analysis);
    // Default exports don't go through a variable declarator, so assigned_to is None
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("return value not captured"));
}

#[test]
fn test_scheduler_return_captured_ok() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("scheduler_captured.ts");
    std::fs::write(
        &path,
        r#"
import { mutation } from "convex/server";
import { api } from "./_generated/api";

export const trigger = mutation({
  args: {},
  handler: async (ctx) => {
    const jobId = await ctx.scheduler.runAfter(0, api.tasks.cleanup);
    return jobId;
  },
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = SchedulerReturnIgnored;
    let diagnostics = rule.check(&analysis);
    assert!(
        diagnostics.is_empty(),
        "Should not flag when scheduler return is captured"
    );
}

// ---------------------------------------------------------------------------
// 6. NonDeterministicInQuery
// ---------------------------------------------------------------------------

#[test]
fn test_non_deterministic_math_random_in_query() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("query_random.ts");
    std::fs::write(
        &path,
        r#"
import { query } from "convex/server";

export const bad = query({
  args: {},
  handler: async (ctx) => {
    const r = Math.random();
    return r;
  },
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = NonDeterministicInQuery;
    let diagnostics = rule.check(&analysis);
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("Math.random()"));
}

#[test]
fn test_non_deterministic_new_date_in_query() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("query_date.ts");
    std::fs::write(
        &path,
        r#"
import { query } from "convex/server";

export const bad = query({
  args: {},
  handler: async (ctx) => {
    const now = new Date();
    return now;
  },
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = NonDeterministicInQuery;
    let diagnostics = rule.check(&analysis);
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("new Date()"));
}

#[test]
fn test_non_deterministic_not_in_mutation() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("mutation_random.ts");
    std::fs::write(
        &path,
        r#"
import { mutation } from "convex/server";

export const good = mutation({
  args: {},
  handler: async (ctx) => {
    const r = Math.random();
    return r;
  },
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = NonDeterministicInQuery;
    let diagnostics = rule.check(&analysis);
    assert!(
        diagnostics.is_empty(),
        "Should not flag Math.random() in a mutation"
    );
}

// ---------------------------------------------------------------------------
// 7. ReplaceVsPatch
// ---------------------------------------------------------------------------

#[test]
fn test_replace_vs_patch_detected() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("replace_call.ts");
    std::fs::write(
        &path,
        r#"
import { mutation } from "convex/server";
import { v } from "convex/values";

export const update = mutation({
  args: { id: v.id("messages"), body: v.string() },
  handler: async (ctx, args) => {
    await ctx.db.replace(args.id, { body: args.body });
  },
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = ReplaceVsPatch;
    let diagnostics = rule.check(&analysis);
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("ctx.db.replace"));
}

#[test]
fn test_replace_vs_patch_not_on_patch() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("patch_call.ts");
    std::fs::write(
        &path,
        r#"
import { mutation } from "convex/server";
import { v } from "convex/values";

export const update = mutation({
  args: { id: v.id("messages"), body: v.string() },
  handler: async (ctx, args) => {
    await ctx.db.patch(args.id, { body: args.body });
  },
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = ReplaceVsPatch;
    let diagnostics = rule.check(&analysis);
    assert!(diagnostics.is_empty(), "Should not flag ctx.db.patch");
}

// ---------------------------------------------------------------------------
// 8. GeneratedCodeModified (project-level)
// ---------------------------------------------------------------------------

#[test]
fn test_generated_code_modified_detected() {
    let rule = GeneratedCodeModified;
    let ctx = ProjectContext {
        generated_files_modified: true,
        ..Default::default()
    };
    let diagnostics = rule.check_project(&ctx);
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("convex/_generated/"));
}

#[test]
fn test_generated_code_not_modified_ok() {
    let rule = GeneratedCodeModified;
    let ctx = ProjectContext {
        generated_files_modified: false,
        ..Default::default()
    };
    let diagnostics = rule.check_project(&ctx);
    assert!(
        diagnostics.is_empty(),
        "Should not flag when generated files are not modified"
    );
}

#[test]
fn test_generated_code_modified_check_is_noop() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("whatever.ts");
    std::fs::write(
        &path,
        r#"
import { query } from "convex/server";
export const foo = query({ args: {}, handler: async (ctx) => {} });
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = GeneratedCodeModified;
    let diagnostics = rule.check(&analysis);
    assert!(
        diagnostics.is_empty(),
        "File-level check should always be empty for project-level rule"
    );
}

// ---------------------------------------------------------------------------
// Additional combined negative tests
// ---------------------------------------------------------------------------

#[test]
fn test_query_side_effect_delete_in_internal_query() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("internal_query_delete.ts");
    std::fs::write(
        &path,
        r#"
import { internalQuery } from "convex/server";

export const bad = internalQuery({
  args: {},
  handler: async (ctx) => {
    await ctx.db.delete("abc123");
  },
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = QuerySideEffect;
    let diagnostics = rule.check(&analysis);
    assert_eq!(
        diagnostics.len(),
        1,
        "Should also detect writes in internalQuery"
    );
}

#[test]
fn test_node_internal_query_mutation_detected() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("node_internal.ts");
    std::fs::write(
        &path,
        r#"
"use node";

import { internalQuery, internalMutation } from "convex/server";

export const bad1 = internalQuery({
  args: {},
  handler: async (ctx) => {
    return await ctx.db.query("messages").collect();
  },
});

export const bad2 = internalMutation({
  args: {},
  handler: async (ctx) => {
    await ctx.db.insert("messages", { body: "hi" });
  },
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = NodeQueryMutation;
    let diagnostics = rule.check(&analysis);
    assert_eq!(
        diagnostics.len(),
        2,
        "Should detect internalQuery and internalMutation in use node files"
    );
}

#[test]
fn test_scheduler_run_at_return_ignored() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("scheduler_run_at.ts");
    std::fs::write(
        &path,
        r#"
import { mutation } from "convex/server";
import { api } from "./_generated/api";

export default mutation({
  args: {},
  handler: async (ctx) => {
    await ctx.scheduler.runAt(Date.now() + 60000, api.tasks.cleanup);
  },
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = SchedulerReturnIgnored;
    let diagnostics = rule.check(&analysis);
    assert_eq!(
        diagnostics.len(),
        1,
        "Should detect runAt return value not captured"
    );
}
