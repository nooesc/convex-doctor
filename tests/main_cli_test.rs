use std::process::Command;

#[test]
fn test_invalid_format_is_rejected() {
    let output = Command::new(env!("CARGO_BIN_EXE_convex-doctor"))
        .args(["--format", "jsn"])
        .output()
        .expect("binary should run");

    assert!(
        !output.status.success(),
        "Invalid --format values should cause a non-zero exit"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("invalid value"),
        "stderr should explain invalid format value. stderr: {stderr}"
    );
}
