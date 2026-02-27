use convex_doctor::rules::context::analyze_file;
use convex_doctor::rules::correctness::{
    CronDirectFunctionReference, CronHelperMethodUsage, DeprecatedApi, QueryDeleteUnsupported,
    StorageGetMetadataDeprecated, UnsupportedValidatorType,
};
use convex_doctor::rules::performance::MissingPaginationOptsValidator;
use convex_doctor::rules::security::MissingArgValidators;
use convex_doctor::rules::Rule;
use tempfile::TempDir;

#[test]
fn test_v_bytes_not_flagged_as_deprecated() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("bytes_ok.ts");
    std::fs::write(
        &path,
        r#"
import { mutation } from "convex/server";
import { v } from "convex/values";

export const save = mutation({
  args: { payload: v.bytes() },
  handler: async () => null,
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = DeprecatedApi;
    let diagnostics = rule.check(&analysis);
    assert!(
        diagnostics.is_empty(),
        "v.bytes() should not be flagged as deprecated"
    );
}

#[test]
fn test_missing_arg_validators_internal_functions() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("internal_missing_args.ts");
    std::fs::write(
        &path,
        r#"
import { internalQuery } from "./_generated/server";

export const hidden = internalQuery({
  handler: async () => null,
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = MissingArgValidators;
    let diagnostics = rule.check(&analysis);
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("internalQuery"));
}

#[test]
fn test_unsupported_validator_types_detected() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("unsupported_validators.ts");
    std::fs::write(
        &path,
        r#"
import { mutation } from "convex/server";
import { v } from "convex/values";

export const save = mutation({
  args: {
    byId: v.map(v.string(), v.string()),
    tags: v.set(v.string()),
  },
  handler: async () => null,
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = UnsupportedValidatorType;
    let diagnostics = rule.check(&analysis);
    assert_eq!(diagnostics.len(), 2);
}

#[test]
fn test_query_delete_unsupported_detected() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("query_delete.ts");
    std::fs::write(
        &path,
        r#"
import { query } from "convex/server";

export const purge = query({
  args: {},
  handler: async (ctx) => {
    await ctx.db.query("tasks").delete();
    return null;
  },
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = QueryDeleteUnsupported;
    let diagnostics = rule.check(&analysis);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn test_cron_helper_methods_detected() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("cron_helper.ts");
    std::fs::write(
        &path,
        r#"
import { cronJobs } from "convex/server";
import { internal } from "./_generated/api";

const crons = cronJobs();
crons.hourly("cleanup", { minuteUTC: 0 }, internal.jobs.cleanup, {});
export default crons;
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = CronHelperMethodUsage;
    let diagnostics = rule.check(&analysis);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn test_cron_direct_function_reference_detected() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("cron_direct_ref.ts");
    std::fs::write(
        &path,
        r#"
import { cronJobs } from "convex/server";
import { internalAction } from "./_generated/server";

export const run = internalAction({
  args: {},
  handler: async () => null,
});

const crons = cronJobs();
crons.interval("run", { hours: 1 }, run, {});
export default crons;
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = CronDirectFunctionReference;
    let diagnostics = rule.check(&analysis);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn test_storage_get_metadata_detected() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("storage_metadata.ts");
    std::fs::write(
        &path,
        r#"
import { query } from "convex/server";
import { v } from "convex/values";

export const meta = query({
  args: { fileId: v.id("_storage") },
  handler: async (ctx, args) => {
    return await ctx.storage.getMetadata(args.fileId);
  },
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = StorageGetMetadataDeprecated;
    let diagnostics = rule.check(&analysis);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn test_missing_pagination_opts_validator_detected() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("missing_pagination_validator.ts");
    std::fs::write(
        &path,
        r#"
import { query } from "convex/server";

export const list = query({
  args: {},
  handler: async (ctx, args) => {
    return await ctx.db.query("messages").paginate(args.paginationOpts);
  },
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = MissingPaginationOptsValidator;
    let diagnostics = rule.check(&analysis);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn test_missing_pagination_opts_validator_not_flagged_when_present() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("pagination_validator_ok.ts");
    std::fs::write(
        &path,
        r#"
import { query, paginationOptsValidator } from "convex/server";

export const list = query({
  args: { paginationOpts: paginationOptsValidator },
  handler: async (ctx, args) => {
    return await ctx.db.query("messages").paginate(args.paginationOpts);
  },
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = MissingPaginationOptsValidator;
    let diagnostics = rule.check(&analysis);
    assert!(diagnostics.is_empty());
}
