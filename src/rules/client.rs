use crate::diagnostic::{Category, Diagnostic, Severity};
use crate::rules::{FileAnalysis, Rule};

/// Error: `useMutation` called in the render body risks infinite write loops.
pub struct MutationInRender;

impl Rule for MutationInRender {
    fn id(&self) -> &'static str {
        "client/mutation-in-render"
    }
    fn category(&self) -> Category {
        Category::ClientSide
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis
            .convex_hook_calls
            .iter()
            .filter(|h| h.hook_name == "useMutation" && h.in_render_body)
            .map(|h| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Error,
                category: self.category(),
                message: "`useMutation(...)` result is invoked during render".to_string(),
                help: "Invoking mutations during render causes infinite write loops. Call the mutate function inside event handlers or useEffect.".to_string(),
                file: analysis.file_path.clone(),
                line: h.line,
                column: h.col,
            })
            .collect()
    }
}

/// Warning: `useQuery` returns `undefined` on first render; remind to handle loading.
pub struct UnhandledLoadingState;

impl Rule for UnhandledLoadingState {
    fn id(&self) -> &'static str {
        "client/unhandled-loading-state"
    }
    fn category(&self) -> Category {
        Category::ClientSide
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        // Emit at most one diagnostic per file, at the first useQuery location.
        analysis
            .convex_hook_calls
            .iter()
            .find(|h| h.hook_name == "useQuery")
            .map(|h| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Warning,
                category: self.category(),
                message: "`useQuery` result may be undefined while loading".to_string(),
                help: "The first render returns `undefined`. Always check `if (data === undefined) return <Loading />` before using query results.".to_string(),
                file: analysis.file_path.clone(),
                line: h.line,
                column: h.col,
            })
            .into_iter()
            .collect()
    }
}

/// Info: `useAction` used — consider whether `useMutation` would be simpler.
pub struct ActionInsteadOfMutation;

impl Rule for ActionInsteadOfMutation {
    fn id(&self) -> &'static str {
        "client/action-instead-of-mutation"
    }
    fn category(&self) -> Category {
        Category::ClientSide
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        analysis
            .convex_hook_calls
            .iter()
            .filter(|h| h.hook_name == "useAction")
            .map(|h| Diagnostic {
                rule: self.id().to_string(),
                severity: Severity::Info,
                category: self.category(),
                message: "`useAction` used — consider if `useMutation` would suffice".to_string(),
                help: "Actions don't have transactional guarantees. If you're only reading/writing the database, `useMutation` is simpler and more reliable.".to_string(),
                file: analysis.file_path.clone(),
                line: h.line,
                column: h.col,
            })
            .collect()
    }
}

/// Info: Convex hooks require a `ConvexProvider` ancestor in the component tree.
pub struct MissingConvexProvider;

impl Rule for MissingConvexProvider {
    fn id(&self) -> &'static str {
        "client/missing-convex-provider"
    }
    fn category(&self) -> Category {
        Category::ClientSide
    }
    fn check(&self, analysis: &FileAnalysis) -> Vec<Diagnostic> {
        // Only warn if hooks are used but no ConvexProvider import is present.
        if analysis.convex_hook_calls.is_empty() || analysis.has_convex_provider {
            return vec![];
        }

        // Emit a single info-level diagnostic at the first hook call location.
        let first = &analysis.convex_hook_calls[0];
        vec![Diagnostic {
            rule: self.id().to_string(),
            severity: Severity::Info,
            category: self.category(),
            message: "Convex hooks used — ensure ConvexProvider wraps the component tree".to_string(),
            help: "Convex hooks require a ConvexProvider ancestor. Typically set up in your root layout.".to_string(),
            file: analysis.file_path.clone(),
            line: first.line,
            column: first.col,
        }]
    }
}
