use convex_doctor::rules::context::analyze_file;
use std::io::Write;
use tempfile::NamedTempFile;

/// Helper to write TypeScript content to a temp file and analyze it.
fn analyze_ts(content: &str) -> convex_doctor::rules::FileAnalysis {
    let mut file = NamedTempFile::with_suffix(".ts").unwrap();
    file.write_all(content.as_bytes()).unwrap();
    file.flush().unwrap();
    analyze_file(file.path()).unwrap()
}

// --------------------------------------------------------------------------
// 1. Cron API refs
// --------------------------------------------------------------------------

#[test]
fn test_cron_api_refs_detected() {
    let analysis = analyze_ts(
        r#"
import { cronJobs } from "convex/server";
import { api } from "./_generated/api";

const crons = cronJobs();
crons.interval("cleanup", { hours: 1 }, api.tasks.cleanup);
crons.daily("report", { hourUTC: 8, minuteUTC: 0 }, api.reports.generate);
export default crons;
"#,
    );
    assert_eq!(
        analysis.cron_api_refs.len(),
        2,
        "Should detect 2 cron api refs, found: {:?}",
        analysis.cron_api_refs
    );
    assert!(analysis.cron_api_refs[0].detail.starts_with("api."));
}

// --------------------------------------------------------------------------
// 2. Non-deterministic calls (Math.random() and new Date()) in queries
// --------------------------------------------------------------------------

#[test]
fn test_math_random_in_query_detected() {
    let analysis = analyze_ts(
        r#"
import { query } from "convex/server";

export const getRandom = query({
  handler: async (ctx) => {
    const r = Math.random();
    return r;
  },
});
"#,
    );
    assert!(
        analysis
            .non_deterministic_calls
            .iter()
            .any(|c| c.detail.contains("Math.random()")),
        "Should detect Math.random() in query, found: {:?}",
        analysis.non_deterministic_calls
    );
}

#[test]
fn test_new_date_in_query_detected() {
    let analysis = analyze_ts(
        r#"
import { query } from "convex/server";

export const getTime = query({
  handler: async (ctx) => {
    const now = new Date();
    return now.toISOString();
  },
});
"#,
    );
    assert!(
        analysis
            .non_deterministic_calls
            .iter()
            .any(|c| c.detail.contains("new Date()")),
        "Should detect new Date() in query, found: {:?}",
        analysis.non_deterministic_calls
    );
}

#[test]
fn test_math_random_in_mutation_not_detected() {
    let analysis = analyze_ts(
        r#"
import { mutation } from "convex/server";

export const doStuff = mutation({
  handler: async (ctx) => {
    const r = Math.random();
    return r;
  },
});
"#,
    );
    assert!(
        analysis.non_deterministic_calls.is_empty(),
        "Should NOT detect Math.random() in mutation"
    );
}

// --------------------------------------------------------------------------
// 3. Throw generic errors
// --------------------------------------------------------------------------

#[test]
fn test_throw_generic_error_in_handler_detected() {
    let analysis = analyze_ts(
        r#"
import { mutation } from "convex/server";

export const doStuff = mutation({
  handler: async (ctx) => {
    throw new Error("Something went wrong");
  },
});
"#,
    );
    assert!(
        !analysis.throw_generic_errors.is_empty(),
        "Should detect throw new Error() in handler, found: {:?}",
        analysis.throw_generic_errors
    );
}

#[test]
fn test_throw_generic_error_outside_handler_not_detected() {
    let analysis = analyze_ts(
        r#"
function helper() {
  throw new Error("fail");
}
"#,
    );
    assert!(
        analysis.throw_generic_errors.is_empty(),
        "Should NOT detect throw new Error() outside of Convex handler"
    );
}

// --------------------------------------------------------------------------
// 4. has_any_validator_in_args
// --------------------------------------------------------------------------

#[test]
fn test_has_any_validator_in_args_detected() {
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
    let func = analysis.functions.iter().find(|f| f.name == "getData").unwrap();
    assert!(
        func.has_any_validator_in_args,
        "Should detect v.any() in args"
    );
}

#[test]
fn test_no_any_validator_when_specific_type() {
    let analysis = analyze_ts(
        r#"
import { query } from "convex/server";
import { v } from "convex/values";

export const getData = query({
  args: {
    name: v.string(),
  },
  handler: async (ctx, args) => {
    return null;
  },
});
"#,
    );
    let func = analysis.functions.iter().find(|f| f.name == "getData").unwrap();
    assert!(
        !func.has_any_validator_in_args,
        "Should NOT flag v.string() as v.any()"
    );
}

// --------------------------------------------------------------------------
// 5. Generic ID validators (v.id() without table name)
// --------------------------------------------------------------------------

#[test]
fn test_generic_id_validator_detected() {
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
    assert!(
        !analysis.generic_id_validators.is_empty(),
        "Should detect v.id() without table name, found: {:?}",
        analysis.generic_id_validators
    );
    assert!(analysis.generic_id_validators[0].detail.contains("docId"));
}

#[test]
fn test_specific_id_validator_not_flagged() {
    let analysis = analyze_ts(
        r#"
import { query } from "convex/server";
import { v } from "convex/values";

export const getDoc = query({
  args: {
    docId: v.id("users"),
  },
  handler: async (ctx, args) => {
    return null;
  },
});
"#,
    );
    assert!(
        analysis.generic_id_validators.is_empty(),
        "Should NOT flag v.id('users') as generic"
    );
}

// --------------------------------------------------------------------------
// 6. Conditional exports with process.env
// --------------------------------------------------------------------------

#[test]
fn test_conditional_export_with_process_env_detected() {
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
    assert!(
        !analysis.conditional_exports.is_empty(),
        "Should detect conditional export based on process.env, found: {:?}",
        analysis.conditional_exports
    );
}

#[test]
fn test_non_conditional_export_not_flagged() {
    let analysis = analyze_ts(
        r#"
import { query } from "convex/server";

export const handler = query({
  handler: async (ctx) => null,
});
"#,
    );
    assert!(
        analysis.conditional_exports.is_empty(),
        "Should NOT flag normal exports"
    );
}

// --------------------------------------------------------------------------
// 7. Raw arg patches (ctx.db.patch(id, args))
// --------------------------------------------------------------------------

#[test]
fn test_raw_arg_patch_detected() {
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
    assert!(
        !analysis.raw_arg_patches.is_empty(),
        "Should detect ctx.db.patch(id, args) with raw args, found: {:?}",
        analysis.raw_arg_patches
    );
}

#[test]
fn test_specific_patch_not_flagged() {
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
    assert!(
        analysis.raw_arg_patches.is_empty(),
        "Should NOT flag ctx.db.patch with specific object"
    );
}

// --------------------------------------------------------------------------
// 8. HTTP routes
// --------------------------------------------------------------------------

#[test]
fn test_http_route_detected() {
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
  pathPrefix: "/api/webhook",
  handler: async (ctx, req) => new Response("ok"),
});

export default http;
"#,
    );
    assert_eq!(
        analysis.http_routes.len(),
        2,
        "Should detect 2 HTTP routes, found: {:?}",
        analysis.http_routes
    );
    assert_eq!(analysis.http_routes[0].method, "GET");
    assert_eq!(analysis.http_routes[0].path, "/api/users");
    assert_eq!(analysis.http_routes[1].method, "POST");
    assert_eq!(analysis.http_routes[1].path, "/api/webhook");
}

// --------------------------------------------------------------------------
// 9. Schema ID fields (v.id("tableName"))
// --------------------------------------------------------------------------

#[test]
fn test_schema_id_fields_detected() {
    let analysis = analyze_ts(
        r#"
import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  posts: defineTable({
    authorId: v.id("users"),
    categoryId: v.id("categories"),
  }),
});
"#,
    );
    assert_eq!(
        analysis.schema_id_fields.len(),
        2,
        "Should detect 2 schema ID fields, found: {:?}",
        analysis.schema_id_fields
    );
    assert!(analysis.schema_id_fields.iter().any(|f| f.table_ref == "users"));
    assert!(analysis.schema_id_fields.iter().any(|f| f.table_ref == "categories"));
}

// --------------------------------------------------------------------------
// 10. Filter field names
// --------------------------------------------------------------------------

#[test]
fn test_filter_field_names_extracted() {
    let analysis = analyze_ts(
        r#"
import { query } from "convex/server";

export const getActive = query({
  handler: async (ctx) => {
    return await ctx.db
      .query("items")
      .filter((q) => q.eq(q.field("status"), "active"))
      .collect();
  },
});
"#,
    );
    assert!(
        analysis.filter_field_names.iter().any(|f| f.field_name == "status"),
        "Should extract 'status' from filter q.field('status'), found: {:?}",
        analysis.filter_field_names
    );
}

// --------------------------------------------------------------------------
// 11. Search index definitions
// --------------------------------------------------------------------------

#[test]
fn test_search_index_definitions_detected() {
    let analysis = analyze_ts(
        r#"
import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  posts: defineTable({
    title: v.string(),
    body: v.string(),
    authorId: v.id("users"),
  })
    .searchIndex("search_body", {
      searchField: "body",
      filterFields: ["authorId"],
    }),
});
"#,
    );
    assert_eq!(
        analysis.search_index_definitions.len(),
        1,
        "Should detect 1 search index, found: {:?}",
        analysis.search_index_definitions
    );
    assert_eq!(analysis.search_index_definitions[0].name, "search_body");
    assert!(analysis.search_index_definitions[0].has_filter_fields);
}

// --------------------------------------------------------------------------
// 12. Optional schema fields (v.optional())
// --------------------------------------------------------------------------

#[test]
fn test_optional_schema_fields_detected() {
    let analysis = analyze_ts(
        r#"
import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  users: defineTable({
    name: v.string(),
    email: v.optional(v.string()),
    bio: v.optional(v.string()),
  }),
});
"#,
    );
    assert_eq!(
        analysis.optional_schema_fields.len(),
        2,
        "Should detect 2 optional fields, found: {:?}",
        analysis.optional_schema_fields
    );
}

// --------------------------------------------------------------------------
// 13. Large writes (>20 properties)
// --------------------------------------------------------------------------

#[test]
fn test_large_write_detected() {
    // Generate a mutation with an insert call having 25 properties
    let props: Vec<String> = (0..25)
        .map(|i| format!("    field{}: \"value{}\"", i, i))
        .collect();
    let props_str = props.join(",\n");
    let source = format!(
        r#"
import {{ mutation }} from "convex/server";

export const create = mutation({{
  handler: async (ctx) => {{
    await ctx.db.insert("items", {{
{}
    }});
  }},
}});
"#,
        props_str
    );
    let analysis = analyze_ts(&source);
    assert!(
        !analysis.large_writes.is_empty(),
        "Should detect large write with 25 properties, found: {:?}",
        analysis.large_writes
    );
}

#[test]
fn test_small_write_not_flagged() {
    let analysis = analyze_ts(
        r#"
import { mutation } from "convex/server";

export const create = mutation({
  handler: async (ctx) => {
    await ctx.db.insert("items", {
      name: "test",
      value: 42,
    });
  },
});
"#,
    );
    assert!(
        analysis.large_writes.is_empty(),
        "Should NOT flag small write with 2 properties"
    );
}

// --------------------------------------------------------------------------
// 14. Unexported function count
// --------------------------------------------------------------------------

#[test]
fn test_unexported_function_count() {
    let analysis = analyze_ts(
        r#"
import { query } from "convex/server";

function helper() {
  return 42;
}

const anotherHelper = () => {
  return "hello";
};

export const getData = query({
  handler: async (ctx) => {
    return helper();
  },
});
"#,
    );
    assert_eq!(
        analysis.unexported_function_count, 2,
        "Should count 2 unexported functions (helper + anotherHelper)"
    );
}

// --------------------------------------------------------------------------
// 15. Convex hook calls and has_convex_provider
// --------------------------------------------------------------------------

#[test]
fn test_convex_hook_calls_detected() {
    let analysis = analyze_ts(
        r#"
import { useQuery, useMutation } from "convex/react";
import { api } from "../convex/_generated/api";

function MyComponent() {
  const data = useQuery(api.tasks.list);
  const create = useMutation(api.tasks.create);
  return null;
}
"#,
    );
    assert_eq!(
        analysis.convex_hook_calls.len(),
        2,
        "Should detect 2 hook calls, found: {:?}",
        analysis.convex_hook_calls
    );
    assert!(analysis.convex_hook_calls.iter().any(|h| h.hook_name == "useQuery"));
    assert!(analysis.convex_hook_calls.iter().any(|h| h.hook_name == "useMutation"));
}

#[test]
fn test_has_convex_provider_detected() {
    let analysis = analyze_ts(
        r#"
import { ConvexProvider } from "convex/react";

function App() {
  return null;
}
"#,
    );
    assert!(
        analysis.has_convex_provider,
        "Should detect ConvexProvider import"
    );
}

#[test]
fn test_no_convex_provider_when_absent() {
    let analysis = analyze_ts(
        r#"
import { useQuery } from "convex/react";

function App() {
  return null;
}
"#,
    );
    assert!(
        !analysis.has_convex_provider,
        "Should NOT flag has_convex_provider when ConvexProvider not imported"
    );
}

// --------------------------------------------------------------------------
// 16. Collect-then-filter detection
// --------------------------------------------------------------------------

#[test]
fn test_collect_then_filter_visitor_detection() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("ctf.ts");
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
    assert!(!analysis.collect_variable_filters.is_empty(),
        "Should detect collect-then-filter pattern");
}
