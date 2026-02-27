use convex_doctor::rules::context::analyze_file;
use convex_doctor::rules::security::*;
use convex_doctor::rules::Rule;
use std::io::Write;
use tempfile::NamedTempFile;

/// Helper to write TypeScript content to a temp file and analyze it.
fn analyze_ts(content: &str) -> convex_doctor::rules::FileAnalysis {
    let mut file = NamedTempFile::with_suffix(".ts").unwrap();
    file.write_all(content.as_bytes()).unwrap();
    file.flush().unwrap();
    analyze_file(file.path()).unwrap()
}

// ===========================================================================
// 1. MissingTableId
// ===========================================================================

#[test]
fn test_missing_table_id_detected() {
    let analysis = analyze_ts(
        r#"
import { query } from "convex/server";
import { v } from "convex/values";

export const getDoc = query({
  args: {
    docId: v.id(),
  },
  handler: async (ctx, args) => {
    return null;
  },
});
"#,
    );
    let rule = MissingTableId;
    let diags = rule.check(&analysis);
    assert!(!diags.is_empty(), "Should detect v.id() without table name");
    assert!(
        diags[0].message.contains("v.id()"),
        "Message should mention v.id(), got: {}",
        diags[0].message
    );
    assert!(
        diags[0].severity == convex_doctor::diagnostic::Severity::Warning,
        "Should be a warning"
    );
}

#[test]
fn test_missing_table_id_not_flagged_with_table() {
    let analysis = analyze_ts(
        r#"
import { query } from "convex/server";
import { v } from "convex/values";

export const getDoc = query({
  args: {
    docId: v.id("documents"),
  },
  handler: async (ctx, args) => {
    return null;
  },
});
"#,
    );
    let rule = MissingTableId;
    let diags = rule.check(&analysis);
    assert!(
        diags.is_empty(),
        "Should NOT flag v.id('documents'), got: {:?}",
        diags
    );
}

// ===========================================================================
// 2. MissingHttpAuth
// ===========================================================================

#[test]
fn test_missing_http_auth_detected() {
    let analysis = analyze_ts(
        r#"
import { httpAction } from "convex/server";

export const webhook = httpAction(async (ctx, request) => {
  return new Response("ok");
});
"#,
    );
    let rule = MissingHttpAuth;
    let diags = rule.check(&analysis);
    assert!(
        !diags.is_empty(),
        "Should detect httpAction without auth check"
    );
    assert!(
        diags[0].message.contains("webhook"),
        "Message should mention function name, got: {}",
        diags[0].message
    );
    assert!(
        diags[0].severity == convex_doctor::diagnostic::Severity::Error,
        "Should be an error"
    );
}

#[test]
fn test_missing_http_auth_not_flagged_with_auth() {
    let analysis = analyze_ts(
        r#"
import { httpAction } from "convex/server";

export const webhook = httpAction(async (ctx, request) => {
  const identity = await ctx.auth.getUserIdentity();
  if (!identity) {
    return new Response("Unauthorized", { status: 401 });
  }
  return new Response("ok");
});
"#,
    );
    let rule = MissingHttpAuth;
    let diags = rule.check(&analysis);
    assert!(
        diags.is_empty(),
        "Should NOT flag httpAction with auth check, got: {:?}",
        diags
    );
}

#[test]
fn test_missing_http_auth_not_flagged_with_helper() {
    let analysis = analyze_ts(
        r#"
import { httpAction } from "convex/server";

async function requireAdmin(ctx: unknown, request: Request) {
  const identity = await ctx.auth.getUserIdentity();
  return { ok: Boolean(identity) };
}

export const webhook = httpAction(async (ctx, request) => {
  const admin = await requireAdmin(ctx, request);
  if (!admin.ok) {
    return new Response("Unauthorized", { status: 401 });
  }
  return new Response("ok");
});
"#,
    );
    let rule = MissingHttpAuth;
    let diags = rule.check(&analysis);
    assert!(
        diags.is_empty(),
        "Should NOT flag httpAction using a helper auth check, got: {:?}",
        diags
    );
}

#[test]
fn test_missing_http_auth_not_flagged_for_non_http() {
    let analysis = analyze_ts(
        r#"
import { mutation } from "convex/server";
import { v } from "convex/values";

export const doStuff = mutation({
  args: { name: v.string() },
  handler: async (ctx, args) => {
    return null;
  },
});
"#,
    );
    let rule = MissingHttpAuth;
    let diags = rule.check(&analysis);
    assert!(
        diags.is_empty(),
        "Should NOT flag non-httpAction functions, got: {:?}",
        diags
    );
}

// ===========================================================================
// 3. ConditionalFunctionExport
// ===========================================================================

#[test]
fn test_conditional_export_detected() {
    let analysis = analyze_ts(
        r#"
import { query, mutation } from "convex/server";

export const handler = process.env.IS_PROD ? query({
  handler: async (ctx) => null,
}) : mutation({
  handler: async (ctx) => null,
});
"#,
    );
    let rule = ConditionalFunctionExport;
    let diags = rule.check(&analysis);
    assert!(
        !diags.is_empty(),
        "Should detect conditional export based on process.env"
    );
    assert!(
        diags[0].severity == convex_doctor::diagnostic::Severity::Error,
        "Should be an error"
    );
}

#[test]
fn test_conditional_export_not_flagged_for_normal() {
    let analysis = analyze_ts(
        r#"
import { query } from "convex/server";

export const handler = query({
  handler: async (ctx) => null,
});
"#,
    );
    let rule = ConditionalFunctionExport;
    let diags = rule.check(&analysis);
    assert!(
        diags.is_empty(),
        "Should NOT flag normal exports, got: {:?}",
        diags
    );
}

// ===========================================================================
// 4. GenericMutationArgs
// ===========================================================================

#[test]
fn test_generic_mutation_args_detected() {
    let analysis = analyze_ts(
        r#"
import { mutation } from "convex/server";
import { v } from "convex/values";

export const updateDoc = mutation({
  args: {
    data: v.any(),
  },
  handler: async (ctx, args) => {
    return null;
  },
});
"#,
    );
    let rule = GenericMutationArgs;
    let diags = rule.check(&analysis);
    assert!(
        !diags.is_empty(),
        "Should detect v.any() in public mutation args"
    );
    assert!(
        diags[0].message.contains("v.any()"),
        "Message should mention v.any(), got: {}",
        diags[0].message
    );
    assert!(
        diags[0].severity == convex_doctor::diagnostic::Severity::Warning,
        "Should be a warning"
    );
}

#[test]
fn test_generic_mutation_args_detected_on_query() {
    let analysis = analyze_ts(
        r#"
import { query } from "convex/server";
import { v } from "convex/values";

export const getData = query({
  args: {
    filter: v.any(),
  },
  handler: async (ctx, args) => {
    return null;
  },
});
"#,
    );
    let rule = GenericMutationArgs;
    let diags = rule.check(&analysis);
    assert!(
        !diags.is_empty(),
        "Should detect v.any() in public query args too"
    );
}

#[test]
fn test_generic_mutation_args_not_flagged_with_specific_types() {
    let analysis = analyze_ts(
        r#"
import { mutation } from "convex/server";
import { v } from "convex/values";

export const updateDoc = mutation({
  args: {
    name: v.string(),
    count: v.number(),
  },
  handler: async (ctx, args) => {
    return null;
  },
});
"#,
    );
    let rule = GenericMutationArgs;
    let diags = rule.check(&analysis);
    assert!(
        diags.is_empty(),
        "Should NOT flag specific validators, got: {:?}",
        diags
    );
}

#[test]
fn test_generic_mutation_args_not_flagged_for_internal() {
    let analysis = analyze_ts(
        r#"
import { internalMutation } from "convex/server";
import { v } from "convex/values";

export const updateDoc = internalMutation({
  args: {
    data: v.any(),
  },
  handler: async (ctx, args) => {
    return null;
  },
});
"#,
    );
    let rule = GenericMutationArgs;
    let diags = rule.check(&analysis);
    assert!(
        diags.is_empty(),
        "Should NOT flag internal functions with v.any(), got: {:?}",
        diags
    );
}

// ===========================================================================
// 5. OverlyBroadPatch
// ===========================================================================

#[test]
fn test_overly_broad_patch_detected() {
    let analysis = analyze_ts(
        r#"
import { mutation } from "convex/server";
import { v } from "convex/values";

export const updateDoc = mutation({
  args: {
    id: v.id("docs"),
    data: v.any(),
  },
  handler: async (ctx, args) => {
    await ctx.db.patch(args.id, args);
  },
});
"#,
    );
    let rule = OverlyBroadPatch;
    let diags = rule.check(&analysis);
    assert!(
        !diags.is_empty(),
        "Should detect ctx.db.patch with raw args"
    );
    assert!(
        diags[0].severity == convex_doctor::diagnostic::Severity::Warning,
        "Should be a warning"
    );
}

#[test]
fn test_overly_broad_patch_not_flagged_with_specific_fields() {
    let analysis = analyze_ts(
        r#"
import { mutation } from "convex/server";
import { v } from "convex/values";

export const updateDoc = mutation({
  args: {
    id: v.id("docs"),
    name: v.string(),
  },
  handler: async (ctx, args) => {
    await ctx.db.patch(args.id, { name: args.name });
  },
});
"#,
    );
    let rule = OverlyBroadPatch;
    let diags = rule.check(&analysis);
    assert!(
        diags.is_empty(),
        "Should NOT flag ctx.db.patch with specific object, got: {:?}",
        diags
    );
}

// ===========================================================================
// 6. HttpMissingCors
// ===========================================================================

#[test]
fn test_http_missing_cors_detected() {
    let analysis = analyze_ts(
        r#"
import { httpRouter } from "convex/server";

const http = httpRouter();

http.route({
  method: "GET",
  path: "/api/users",
  handler: async (ctx, req) => new Response("ok"),
});

http.route({
  method: "POST",
  path: "/api/users",
  handler: async (ctx, req) => new Response("ok"),
});

export default http;
"#,
    );
    let rule = HttpMissingCors;
    let diags = rule.check(&analysis);
    assert!(
        !diags.is_empty(),
        "Should detect missing OPTIONS handler for /api/users"
    );
    assert!(
        diags[0].message.contains("/api/users"),
        "Message should mention the path, got: {}",
        diags[0].message
    );
    assert!(
        diags[0].severity == convex_doctor::diagnostic::Severity::Warning,
        "Should be a warning"
    );
}

#[test]
fn test_http_missing_cors_not_flagged_with_options() {
    let analysis = analyze_ts(
        r#"
import { httpRouter } from "convex/server";

const http = httpRouter();

http.route({
  method: "GET",
  path: "/api/users",
  handler: async (ctx, req) => new Response("ok"),
});

http.route({
  method: "OPTIONS",
  path: "/api/users",
  handler: async (ctx, req) => new Response("", { status: 204 }),
});

export default http;
"#,
    );
    let rule = HttpMissingCors;
    let diags = rule.check(&analysis);
    assert!(
        diags.is_empty(),
        "Should NOT flag when OPTIONS handler exists, got: {:?}",
        diags
    );
}

#[test]
fn test_http_missing_cors_multiple_paths() {
    let analysis = analyze_ts(
        r#"
import { httpRouter } from "convex/server";

const http = httpRouter();

http.route({
  method: "GET",
  path: "/api/users",
  handler: async (ctx, req) => new Response("ok"),
});

http.route({
  method: "OPTIONS",
  path: "/api/users",
  handler: async (ctx, req) => new Response("", { status: 204 }),
});

http.route({
  method: "POST",
  path: "/api/webhooks",
  handler: async (ctx, req) => new Response("ok"),
});

export default http;
"#,
    );
    let rule = HttpMissingCors;
    let diags = rule.check(&analysis);
    assert_eq!(
        diags.len(),
        1,
        "Should only flag /api/webhooks (not /api/users), got: {:?}",
        diags
    );
    assert!(
        diags[0].message.contains("/api/webhooks"),
        "Should flag the path without OPTIONS"
    );
}

#[test]
fn test_http_missing_cors_skips_webhook_routes() {
    let analysis = analyze_ts(
        r#"
import { httpRouter } from "convex/server";

const http = httpRouter();

http.route({
  method: "POST",
  path: "/api/webhook/order-created",
  handler: async (ctx, req) => new Response("ok"),
});

http.route({
  method: "GET",
  path: "/api/public",
  handler: async (ctx, req) => new Response("ok"),
});

export default http;
"#,
    );
    let rule = HttpMissingCors;
    let diags = rule.check(&analysis);
    assert!(
        diags.len() == 1,
        "Only /api/public should be flagged, got: {:?}",
        diags
    );
    assert!(
        diags[0].message.contains("/api/public"),
        "Should flag the non-webhook path"
    );
}

#[test]
fn test_http_missing_cors_skips_comment_marked_webhook_routes() {
    let analysis = analyze_ts(
        r#"
import { httpRouter } from "convex/server";

const http = httpRouter();

// Webhook endpoint for async SMS callbacks
http.route({
  method: "POST",
  path: "/simple-texting",
  handler: async (ctx, req) => new Response("ok"),
});

export default http;
"#,
    );
    let rule = HttpMissingCors;
    let diags = rule.check(&analysis);
    assert!(
        diags.is_empty(),
        "Should skip webhook routes without OPTIONS handler when comment indicates webhook context"
    );
}

#[test]
fn test_http_missing_cors_no_routes_no_diagnostic() {
    let analysis = analyze_ts(
        r#"
import { query } from "convex/server";

export const getStuff = query({
  handler: async (ctx) => null,
});
"#,
    );
    let rule = HttpMissingCors;
    let diags = rule.check(&analysis);
    assert!(
        diags.is_empty(),
        "Should produce no diagnostics when there are no HTTP routes"
    );
}
