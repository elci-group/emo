use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn inspect_is_a_single_machine_readable_document() {
    let output = Command::new(env!("CARGO_BIN_EXE_emo"))
        .arg("inspect")
        .arg("--root")
        .arg(env!("CARGO_MANIFEST_DIR"))
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["mode"], "suggest");
}

#[test]
fn suggest_echoes_message_and_emits_a_noop_decision() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_emo"))
        .arg("suggest")
        .arg("--root")
        .arg(env!("CARGO_MANIFEST_DIR"))
        .arg("--format")
        .arg("json")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(
        br#"{"schema":"emo.request.v1","request_id":"r1","session_id":"s1","message":"Do not deploy.\n8 GiB"}"#,
    ).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["status"], "noop");
    assert_eq!(value["output"], "Do not deploy.\n8 GiB");
    assert_eq!(value["original_digest"], value["output_digest"]);
    assert_eq!(value["usage"]["status"], "not_invoked");
}

#[test]
fn commands_require_an_explicit_root_and_json_format() {
    let output = Command::new(env!("CARGO_BIN_EXE_emo"))
        .arg("inspect")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
}

#[test]
fn policy_validation_is_scoped_to_the_project_root() {
    let output = Command::new(env!("CARGO_BIN_EXE_emo"))
        .args(["policy", "validate", "policies/noop.v1.json", "--root"])
        .arg(env!("CARGO_MANIFEST_DIR"))
        .args(["--format", "json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["valid"], true);
}

#[test]
fn policy_validation_rejects_paths_outside_the_project_root() {
    let output = Command::new(env!("CARGO_BIN_EXE_emo"))
        .args(["policy", "validate", "../Cargo.toml", "--root"])
        .arg(env!("CARGO_MANIFEST_DIR"))
        .args(["--format", "json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
}
