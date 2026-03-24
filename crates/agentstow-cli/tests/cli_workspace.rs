use assert_cmd::Command;
use assert_fs::prelude::*;

#[test]
fn workspace_init_should_create_manifest_and_sample_artifact() {
    let temp = assert_fs::TempDir::new().unwrap();
    let ws = temp.child("ws");

    let mut cmd = Command::cargo_bin("agentstow").unwrap();
    cmd.arg("--cwd")
        .arg(temp.path())
        .arg("--workspace")
        .arg(ws.path())
        .arg("workspace")
        .arg("init");

    cmd.assert().success();

    assert!(ws.child("agentstow.toml").path().exists());
    assert!(ws.child("artifacts/hello.txt.tera").path().exists());
}

#[test]
fn workspace_init_with_git_and_json_should_emit_machine_readable_output() {
    let temp = assert_fs::TempDir::new().unwrap();
    let ws = temp.child("ws");

    let mut cmd = Command::cargo_bin("agentstow").unwrap();
    cmd.arg("--cwd")
        .arg(temp.path())
        .arg("--workspace")
        .arg(ws.path())
        .arg("--json")
        .arg("workspace")
        .arg("init")
        .arg("--git-init");

    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let payload: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(payload["git_inited"], serde_json::json!(true));
    assert!(ws.child(".git").path().exists());
    assert!(ws.child("agentstow.toml").path().exists());
}
