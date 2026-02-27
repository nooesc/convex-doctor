use convex_doctor::rules::client::*;
use convex_doctor::rules::context::analyze_file;
use convex_doctor::rules::Rule;
use std::io::Write;
use tempfile::NamedTempFile;

/// Helper: write TSX content to a temp file and analyze it.
fn analyze_tsx(content: &str) -> convex_doctor::rules::FileAnalysis {
    let mut file = NamedTempFile::with_suffix(".tsx").unwrap();
    file.write_all(content.as_bytes()).unwrap();
    file.flush().unwrap();
    analyze_file(file.path()).unwrap()
}

/// Helper: write TS content to a temp file and analyze it.
fn analyze_ts(content: &str) -> convex_doctor::rules::FileAnalysis {
    let mut file = NamedTempFile::with_suffix(".ts").unwrap();
    file.write_all(content.as_bytes()).unwrap();
    file.flush().unwrap();
    analyze_file(file.path()).unwrap()
}

// =========================================================================
// MutationInRender
// =========================================================================

#[test]
fn mutation_in_render_not_flagged_for_normal_hook_usage() {
    // Normal hook usage should not fire.
    let analysis = analyze_tsx(
        r#"
import { useMutation } from "convex/react";
import { api } from "../convex/_generated/api";

export default function BadComponent() {
  const result = useMutation(api.tasks.create);
  return <div />;
}
"#,
    );
    let diags = MutationInRender.check(&analysis);
    assert!(
        diags.is_empty(),
        "Normal hook usage should not fire, got: {:?}",
        diags
    );
}

#[test]
fn mutation_inside_convex_handler_not_flagged() {
    // When useMutation is called inside a Convex function builder (e.g. mutation handler),
    // in_render_body is false because the visitor tracks function_builder_stack.
    // Plain arrow functions in client code don't affect in_render_body (v1 heuristic).
    let analysis = analyze_ts(
        r#"
import { mutation } from "convex/server";

export const myMutation = mutation({
  handler: async (ctx) => {
    const create = useMutation(api.tasks.create);
  },
});
"#,
    );
    let diags = MutationInRender.check(&analysis);
    assert!(
        diags.is_empty(),
        "Should NOT flag useMutation inside a Convex handler, got: {:?}",
        diags
    );
}

#[test]
fn no_mutation_no_diagnostic() {
    let analysis = analyze_tsx(
        r#"
import { useQuery } from "convex/react";

export default function ReadOnly() {
  const data = useQuery(api.tasks.list);
  return <div />;
}
"#,
    );
    let diags = MutationInRender.check(&analysis);
    assert!(diags.is_empty(), "No useMutation means no diagnostic");
}

#[test]
fn mutation_in_render_detected_for_immediately_invoked_hook() {
    let analysis = analyze_tsx(
        r#"
import { useMutation } from "convex/react";
import { api } from "../convex/_generated/api";

export default function BadComponent() {
  useMutation(api.tasks.create)({ name: "x" });
  return <div />;
}
"#,
    );
    let diags = MutationInRender.check(&analysis);
    assert!(
        !diags.is_empty(),
        "Immediate invocation of useMutation result should be flagged"
    );
}

// =========================================================================
// UnhandledLoadingState
// =========================================================================

#[test]
fn unhandled_loading_state_detected() {
    let analysis = analyze_tsx(
        r#"
import { useQuery } from "convex/react";
import { api } from "../convex/_generated/api";

export default function MyComponent() {
  const data = useQuery(api.tasks.list);
  return <div>{data.map(t => <p>{t.name}</p>)}</div>;
}
"#,
    );
    let diags = UnhandledLoadingState.check(&analysis);
    assert!(
        !diags.is_empty(),
        "Should warn about unhandled loading state"
    );
    assert!(diags[0].message.contains("undefined"));
}

#[test]
fn unhandled_loading_state_emits_only_one() {
    let analysis = analyze_tsx(
        r#"
import { useQuery } from "convex/react";
import { api } from "../convex/_generated/api";

export default function Multi() {
  const tasks = useQuery(api.tasks.list);
  const users = useQuery(api.users.list);
  const projects = useQuery(api.projects.list);
  return <div />;
}
"#,
    );
    let diags = UnhandledLoadingState.check(&analysis);
    assert_eq!(
        diags.len(),
        1,
        "Should emit exactly one diagnostic per file, got {}",
        diags.len()
    );
}

#[test]
fn unhandled_loading_state_detected_for_aliased_use_query() {
    let analysis = analyze_tsx(
        r#"
import { useQuery as useConvexQuery } from "convex/react";
import { api } from "../convex/_generated/api";

export default function AliasQuery() {
  const data = useConvexQuery(api.tasks.list);
  return <div>{data?.length}</div>;
}
"#,
    );
    let diags = UnhandledLoadingState.check(&analysis);
    assert!(
        !diags.is_empty(),
        "Aliased useQuery import should still be detected"
    );
}

#[test]
fn no_use_query_no_loading_warning() {
    let analysis = analyze_tsx(
        r#"
import { useMutation } from "convex/react";

export default function NoQuery() {
  return <div />;
}
"#,
    );
    let diags = UnhandledLoadingState.check(&analysis);
    assert!(
        diags.is_empty(),
        "No useQuery means no loading-state warning"
    );
}

// =========================================================================
// ActionInsteadOfMutation
// =========================================================================

#[test]
fn action_instead_of_mutation_detected() {
    let analysis = analyze_tsx(
        r#"
import { useAction } from "convex/react";
import { api } from "../convex/_generated/api";

export default function ActionUser() {
  const doThing = useAction(api.tasks.doThing);
  return <button onClick={() => doThing()} />;
}
"#,
    );
    let diags = ActionInsteadOfMutation.check(&analysis);
    assert!(
        !diags.is_empty(),
        "Should suggest considering useMutation instead of useAction"
    );
    assert!(diags[0].message.contains("useAction"));
}

#[test]
fn multiple_actions_multiple_diagnostics() {
    let analysis = analyze_tsx(
        r#"
import { useAction } from "convex/react";
import { api } from "../convex/_generated/api";

export default function MultiAction() {
  const a = useAction(api.tasks.a);
  const b = useAction(api.tasks.b);
  return <div />;
}
"#,
    );
    let diags = ActionInsteadOfMutation.check(&analysis);
    assert_eq!(
        diags.len(),
        2,
        "Each useAction should produce its own diagnostic, got {}",
        diags.len()
    );
}

#[test]
fn action_instead_of_mutation_detected_for_aliased_use_action() {
    let analysis = analyze_tsx(
        r#"
import { useAction as useConvexAction } from "convex/react";
import { api } from "../convex/_generated/api";

export default function ActionAlias() {
  const run = useConvexAction(api.tasks.doThing);
  return <button onClick={() => run()} />;
}
"#,
    );
    let diags = ActionInsteadOfMutation.check(&analysis);
    assert!(
        !diags.is_empty(),
        "Aliased useAction import should still be detected"
    );
}

#[test]
fn no_action_no_diagnostic() {
    let analysis = analyze_tsx(
        r#"
import { useMutation } from "convex/react";

export default function Clean() {
  const create = useMutation(api.tasks.create);
  return <div />;
}
"#,
    );
    let diags = ActionInsteadOfMutation.check(&analysis);
    assert!(diags.is_empty(), "No useAction means no diagnostic");
}

// =========================================================================
// MissingConvexProvider
// =========================================================================

#[test]
fn missing_provider_when_hooks_used() {
    let analysis = analyze_tsx(
        r#"
import { useQuery, useMutation } from "convex/react";
import { api } from "../convex/_generated/api";

export default function Component() {
  const data = useQuery(api.tasks.list);
  const create = useMutation(api.tasks.create);
  return <div />;
}
"#,
    );
    let diags = MissingConvexProvider.check(&analysis);
    assert!(
        !diags.is_empty(),
        "Should remind about ConvexProvider when hooks used without it"
    );
    assert!(diags[0].message.contains("ConvexProvider"));
}

#[test]
fn no_warning_when_provider_imported() {
    let analysis = analyze_ts(
        r#"
import { ConvexProvider } from "convex/react";
import { useQuery } from "convex/react";

function App() {
  const data = useQuery(api.tasks.list);
  return null;
}
"#,
    );
    let diags = MissingConvexProvider.check(&analysis);
    assert!(
        diags.is_empty(),
        "Should NOT warn when ConvexProvider is imported, got: {:?}",
        diags
    );
}

#[test]
fn no_warning_when_provider_is_aliased() {
    let analysis = analyze_ts(
        r#"
import { ConvexProvider as Provider } from "convex/react";
import { useQuery } from "convex/react";

function App() {
  const data = useQuery(api.tasks.list);
  return null;
}
"#,
    );
    let diags = MissingConvexProvider.check(&analysis);
    assert!(
        diags.is_empty(),
        "Aliased ConvexProvider import should count as provider presence"
    );
}

#[test]
fn no_warning_when_no_hooks() {
    let analysis = analyze_tsx(
        r#"
export default function PlainComponent() {
  return <div>Hello</div>;
}
"#,
    );
    let diags = MissingConvexProvider.check(&analysis);
    assert!(
        diags.is_empty(),
        "Should NOT warn when no Convex hooks are used"
    );
}

#[test]
fn missing_provider_emits_single_diagnostic() {
    let analysis = analyze_tsx(
        r#"
import { useQuery, useMutation, useAction } from "convex/react";
import { api } from "../convex/_generated/api";

export default function Multi() {
  const a = useQuery(api.tasks.list);
  const b = useMutation(api.tasks.create);
  const c = useAction(api.tasks.doThing);
  return <div />;
}
"#,
    );
    let diags = MissingConvexProvider.check(&analysis);
    assert_eq!(
        diags.len(),
        1,
        "Should emit exactly one provider reminder per file, got {}",
        diags.len()
    );
}

// =========================================================================
// Severity checks
// =========================================================================

#[test]
fn mutation_in_render_severity_is_error_when_triggered() {
    // Verify the rule severity via a synthetic FileAnalysis with in_render_body = true.
    let mut analysis = convex_doctor::rules::FileAnalysis {
        file_path: "test.tsx".to_string(),
        ..Default::default()
    };
    analysis
        .convex_hook_calls
        .push(convex_doctor::rules::ConvexHookCall {
            hook_name: "useMutation".to_string(),
            line: 1,
            col: 1,
            in_render_body: true,
        });
    let diags = MutationInRender.check(&analysis);
    assert!(!diags.is_empty());
    assert_eq!(
        diags[0].severity,
        convex_doctor::diagnostic::Severity::Error
    );
}

#[test]
fn unhandled_loading_is_warning_severity() {
    let analysis = analyze_tsx(
        r#"
import { useQuery } from "convex/react";
export default function C() {
  const d = useQuery(api.tasks.list);
  return <div />;
}
"#,
    );
    let diags = UnhandledLoadingState.check(&analysis);
    assert!(!diags.is_empty());
    assert_eq!(
        diags[0].severity,
        convex_doctor::diagnostic::Severity::Warning
    );
}

#[test]
fn action_instead_of_mutation_is_info_severity() {
    let analysis = analyze_tsx(
        r#"
import { useAction } from "convex/react";
export default function C() {
  const a = useAction(api.tasks.doThing);
  return <div />;
}
"#,
    );
    let diags = ActionInsteadOfMutation.check(&analysis);
    assert!(!diags.is_empty());
    assert_eq!(diags[0].severity, convex_doctor::diagnostic::Severity::Info);
}

#[test]
fn missing_provider_is_info_severity() {
    let analysis = analyze_tsx(
        r#"
import { useQuery } from "convex/react";
export default function C() {
  const d = useQuery(api.tasks.list);
  return <div />;
}
"#,
    );
    let diags = MissingConvexProvider.check(&analysis);
    assert!(!diags.is_empty());
    assert_eq!(diags[0].severity, convex_doctor::diagnostic::Severity::Info);
}

// =========================================================================
// Category checks
// =========================================================================

#[test]
fn all_rules_have_client_side_category() {
    assert_eq!(
        MutationInRender.category(),
        convex_doctor::diagnostic::Category::ClientSide
    );
    assert_eq!(
        UnhandledLoadingState.category(),
        convex_doctor::diagnostic::Category::ClientSide
    );
    assert_eq!(
        ActionInsteadOfMutation.category(),
        convex_doctor::diagnostic::Category::ClientSide
    );
    assert_eq!(
        MissingConvexProvider.category(),
        convex_doctor::diagnostic::Category::ClientSide
    );
}

// =========================================================================
// Rule ID format
// =========================================================================

#[test]
fn rule_ids_follow_naming_convention() {
    assert_eq!(MutationInRender.id(), "client/mutation-in-render");
    assert_eq!(UnhandledLoadingState.id(), "client/unhandled-loading-state");
    assert_eq!(
        ActionInsteadOfMutation.id(),
        "client/action-instead-of-mutation"
    );
    assert_eq!(MissingConvexProvider.id(), "client/missing-convex-provider");
}
