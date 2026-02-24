use convex_doctor::rules::context::analyze_file;
use std::path::Path;

#[test]
fn test_analyze_basic_query() {
    let analysis = analyze_file(Path::new("tests/fixtures/basic_query.ts")).unwrap();
    assert_eq!(analysis.functions.len(), 2);

    let get_messages = &analysis.functions[0];
    assert_eq!(get_messages.name, "getMessages");
    assert!(get_messages.has_args_validator);
    assert!(get_messages.has_return_validator);
    assert!(get_messages.has_auth_check);

    let send_message = &analysis.functions[1];
    assert_eq!(send_message.name, "sendMessage");
    assert!(!send_message.has_args_validator);
    assert!(!send_message.has_return_validator);
}

#[test]
fn test_analyze_bad_patterns_collect() {
    let analysis = analyze_file(Path::new("tests/fixtures/bad_patterns.ts")).unwrap();
    assert!(
        !analysis.collect_calls.is_empty(),
        "Should detect .collect() calls"
    );
}

#[test]
fn test_analyze_bad_patterns_filter() {
    let analysis = analyze_file(Path::new("tests/fixtures/bad_patterns.ts")).unwrap();
    assert!(
        !analysis.filter_calls.is_empty(),
        "Should detect .filter() calls"
    );
}

#[test]
fn test_analyze_bad_patterns_date_now() {
    let analysis = analyze_file(Path::new("tests/fixtures/bad_patterns.ts")).unwrap();
    assert!(
        !analysis.date_now_calls.is_empty(),
        "Should detect Date.now() calls"
    );
}

#[test]
fn test_analyze_bad_patterns_loop_ctx() {
    let analysis = analyze_file(Path::new("tests/fixtures/bad_patterns.ts")).unwrap();
    assert!(
        !analysis.loop_ctx_calls.is_empty(),
        "Should detect ctx calls in loops"
    );
}

#[test]
fn test_analyze_use_node() {
    let analysis = analyze_file(Path::new("tests/fixtures/use_node.ts")).unwrap();
    assert!(analysis.has_use_node);
}

#[test]
fn test_analyze_missing_args() {
    let analysis = analyze_file(Path::new("tests/fixtures/bad_patterns.ts")).unwrap();
    let list_all = analysis
        .functions
        .iter()
        .find(|f| f.name == "listAll")
        .unwrap();
    assert!(!list_all.has_args_validator);
}

#[test]
fn test_analyze_imports() {
    let analysis = analyze_file(Path::new("tests/fixtures/basic_query.ts")).unwrap();
    assert!(analysis.imports.iter().any(|i| i.source == "convex/server"));
    assert!(analysis.imports.iter().any(|i| i.source == "convex/values"));
}
