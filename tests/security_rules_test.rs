use convex_doctor::rules::context::analyze_file;
use convex_doctor::rules::security::*;
use convex_doctor::rules::Rule;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn test_missing_arg_validators() {
    let analysis = analyze_file(Path::new("tests/fixtures/bad_patterns.ts")).unwrap();
    let rule = MissingArgValidators;
    let diagnostics = rule.check(&analysis);
    assert!(
        diagnostics.iter().any(|d| d.message.contains("listAll")),
        "listAll has no args"
    );
}

#[test]
fn test_no_false_positive_arg_validators() {
    let analysis = analyze_file(Path::new("tests/fixtures/basic_query.ts")).unwrap();
    let rule = MissingArgValidators;
    let diagnostics = rule.check(&analysis);
    assert!(
        !diagnostics
            .iter()
            .any(|d| d.message.contains("getMessages")),
        "getMessages has args"
    );
}

#[test]
fn test_missing_arg_validators_skips_http_action() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("http_action.ts");
    std::fs::write(
        &path,
        r#"
import { httpAction } from "convex/server";

export const webhook = httpAction(async (ctx, request) => {
  return new Response("ok");
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = MissingArgValidators;
    let diagnostics = rule.check(&analysis);
    assert!(
        diagnostics.is_empty(),
        "httpAction should be excluded from argument validator checks, got {:?}",
        diagnostics
    );
}

#[test]
fn test_missing_return_validators() {
    let analysis = analyze_file(Path::new("tests/fixtures/bad_patterns.ts")).unwrap();
    let rule = MissingReturnValidators;
    let diagnostics = rule.check(&analysis);
    assert!(
        !diagnostics.is_empty(),
        "Should flag functions without return validators"
    );
}

#[test]
fn test_missing_auth_check() {
    let analysis = analyze_file(Path::new("tests/fixtures/bad_patterns.ts")).unwrap();
    let rule = MissingAuthCheck;
    let diagnostics = rule.check(&analysis);
    assert!(
        !diagnostics.is_empty(),
        "Public functions without auth check should be flagged"
    );
}

#[test]
fn test_auth_check_not_flagged_when_present() {
    let analysis = analyze_file(Path::new("tests/fixtures/basic_query.ts")).unwrap();
    let rule = MissingAuthCheck;
    let diagnostics = rule.check(&analysis);
    // getMessages has auth check, should NOT be in diagnostics
    assert!(
        !diagnostics
            .iter()
            .any(|d| d.message.contains("getMessages")),
        "getMessages has auth"
    );
}

#[test]
fn test_missing_auth_check_skips_internal_secret_args() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("internal_secret.ts");
    std::fs::write(
        &path,
        r#"
import { mutation } from "convex/server";
import { v } from "convex/values";

export const internalAction = mutation({
  args: {
    internalSecret: v.string(),
    userId: v.string()
  },
  handler: async (ctx, { internalSecret, userId }) => {
    return await ctx.db.get(userId);
  },
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = MissingAuthCheck;
    let diagnostics = rule.check(&analysis);
    assert!(
        diagnostics.is_empty(),
        "internalSecret should suppress missing auth checks, got {:?}",
        diagnostics
    );
}

#[test]
fn test_missing_auth_check_respects_convex_doctor_ignore_comment() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("public_webhook.ts");
    std::fs::write(
        &path,
        r#"
import { httpAction } from "convex/server";

// convex-doctor-ignore
export const healthcheck = httpAction(async (ctx, request) => {
  return new Response("ok");
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = MissingAuthCheck;
    let diagnostics = rule.check(&analysis);
    assert!(
        diagnostics.is_empty(),
        "convex-doctor-ignore comment should suppress missing auth checks, got {:?}",
        diagnostics
    );
}

#[test]
fn test_hardcoded_secrets() {
    let analysis = analyze_file(Path::new("tests/fixtures/secrets_test.ts")).unwrap();
    assert!(
        !analysis.hardcoded_secrets.is_empty(),
        "Should detect hardcoded secret"
    );
}

#[test]
fn test_spoofable_access_control_detected() {
    let analysis = analyze_file(Path::new("tests/fixtures/spoofable_access.ts")).unwrap();
    let rule = SpoofableAccessControl;
    let diagnostics = rule.check(&analysis);
    assert!(
        !diagnostics.is_empty(),
        "Should detect suspicious auth args without ctx.auth checks"
    );
}

#[test]
fn test_internal_secret_suppresses_spoofable_access_control() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("internal_secret_spoof.ts");
    std::fs::write(
        &path,
        r#"
import { query } from "convex/server";
import { v } from "convex/values";

export const getUser = query({
  args: {
    userId: v.string(),
    internalSecret: v.string(),
  },
  handler: async (ctx, { userId }) => {
    return await ctx.db.get(userId);
  },
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = SpoofableAccessControl;
    let diagnostics = rule.check(&analysis);
    assert!(
        diagnostics.is_empty(),
        "internalSecret should suppress spoofable access-control flagging, got {:?}",
        diagnostics
    );
}

#[test]
fn test_missing_auth_skips_scripts_dir() {
    let dir = TempDir::new().unwrap();
    // Simulate a file inside convex/_scripts/
    let scripts_dir = dir.path().join("convex").join("_scripts");
    std::fs::create_dir_all(&scripts_dir).unwrap();
    let path = scripts_dir.join("syncJobs.ts");
    std::fs::write(
        &path,
        r#"
import { query } from "../_generated/server";
import { v } from "convex/values";

export const getJobStatus = query({
  args: { jobId: v.id("sync_jobs") },
  handler: async (ctx, { jobId }) => {
    return await ctx.db.get(jobId);
  },
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = MissingAuthCheck;
    let diagnostics = rule.check(&analysis);
    assert!(
        diagnostics.is_empty(),
        "Public functions in _scripts/ should NOT be flagged for missing auth"
    );
}

#[test]
fn test_missing_auth_still_flags_normal_dir() {
    let dir = TempDir::new().unwrap();
    let conv_dir = dir.path().join("convex");
    std::fs::create_dir_all(&conv_dir).unwrap();
    let path = conv_dir.join("users.ts");
    std::fs::write(
        &path,
        r#"
import { query } from "./_generated/server";
import { v } from "convex/values";

export const getUser = query({
  args: { userId: v.id("users") },
  handler: async (ctx, { userId }) => {
    return await ctx.db.get(userId);
  },
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = MissingAuthCheck;
    let diagnostics = rule.check(&analysis);
    assert!(
        !diagnostics.is_empty(),
        "Public functions in normal convex/ dir should still be flagged for missing auth"
    );
}

#[test]
fn test_internal_api_misuse_skips_internal_secret_functions() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("internal_secret_api.ts");
    std::fs::write(
        &path,
        r#"
import { mutation } from "convex/server";
import { v } from "convex/values";
import { api } from "../_generated/api";

export const callInternal = mutation({
  args: {
    internalSecret: v.string(),
  },
  handler: async (ctx, args) => {
    return await ctx.runMutation(api.users.syncUsers);
  },
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = InternalApiMisuse;
    let diagnostics = rule.check(&analysis);
    assert!(
        diagnostics.is_empty(),
        "internalSecret should suppress internal API misuse, got {:?}",
        diagnostics
    );
}

#[test]
fn test_missing_http_auth_skips_internal_secret_and_ignore_comment() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("http_internal_secret.ts");
    std::fs::write(
        &path,
        r#"
import { httpAction } from "convex/server";
import { v } from "convex/values";

export const webhook = httpAction({
  args: {
    internalSecret: v.string(),
  },
  handler: async (ctx, request) => {
    return new Response("ok");
  },
});

// convex-doctor-ignore
export const webhookPublic = httpAction(async (ctx, request) => {
  return new Response("ok");
});
"#,
    )
    .unwrap();

    let analysis = analyze_file(&path).unwrap();
    let rule = MissingHttpAuth;
    let diagnostics = rule.check(&analysis);
    assert!(
        diagnostics.is_empty(),
        "internalSecret/commented httpAction should skip missing-http-auth, got {:?}",
        diagnostics
    );
}
