use std::time::Duration;

use assert_fs::prelude::*;
use pretty_assertions::assert_eq;

use super::*;

fn script_req(args: &[&str]) -> ScriptRunRequest {
    ScriptRunRequest {
        workspace_root: std::env::current_dir().unwrap(),
        script: agentstow_manifest::ScriptDef {
            kind: "shell".to_string(),
            entry: "bash".to_string(),
            args: args.iter().map(|value| value.to_string()).collect(),
            cwd_policy: agentstow_core::CwdPolicy::Current,
            env: vec![],
            stdin_mode: agentstow_core::StdinMode::None,
            stdout_mode: agentstow_core::OutputMode::Capture,
            stderr_mode: agentstow_core::OutputMode::Capture,
            timeout_ms: Some(1_000),
            expected_exit_codes: vec![0],
        },
        stdin_text: None,
    }
}

#[tokio::test]
async fn run_capture_stdout_should_work() {
    let req = script_req(&["-lc", "echo hello"]);
    let out = ScriptRunner::run(req).await.unwrap();
    assert_eq!(out.exit_code, 0);
    assert_eq!(out.stdout.unwrap(), "hello\n");
}

#[cfg(not(windows))]
#[tokio::test]
async fn run_text_mode_without_stdin_should_use_null_stdin_and_finish() {
    let mut req = script_req(&["-lc", "cat >/dev/null && echo done"]);
    req.script.stdin_mode = agentstow_core::StdinMode::Text;

    let out = ScriptRunner::run(req).await.unwrap();
    assert_eq!(out.exit_code, 0);
    assert_eq!(out.stdout.unwrap(), "done\n");
}

#[cfg(not(windows))]
#[tokio::test]
async fn run_should_fail_when_timeout_is_reached() {
    let mut req = script_req(&["-lc", "sleep 1"]);
    req.script.timeout_ms = Some(50);

    let err = ScriptRunner::run(req).await.unwrap_err();
    assert_eq!(
        err.exit_code(),
        agentstow_core::ExitCode::ExternalCommandFailed
    );
    assert!(err.to_string().contains("脚本超时"));
}

#[tokio::test]
async fn run_text_mode_without_input_should_observe_eof() {
    let mut req = script_req(&[
        "-lc",
        "python3 -c 'import sys; data=sys.stdin.read(); print(len(data))'",
    ]);
    req.script.stdin_mode = agentstow_core::StdinMode::Text;

    let out = ScriptRunner::run(req).await.unwrap();
    assert_eq!(out.exit_code, 0);
    assert_eq!(out.stdout.unwrap(), "0\n");
}

#[tokio::test]
async fn run_text_mode_with_input_should_write_payload() {
    let mut req = script_req(&[
        "-lc",
        "python3 -c 'import sys; print(sys.stdin.read(), end=\"\")'",
    ]);
    req.script.stdin_mode = agentstow_core::StdinMode::Text;
    req.stdin_text = Some("hello-from-stdin".to_string());

    let out = ScriptRunner::run(req).await.unwrap();
    assert_eq!(out.exit_code, 0);
    assert_eq!(out.stdout.unwrap(), "hello-from-stdin");
}

#[cfg(not(windows))]
#[tokio::test]
async fn run_should_kill_background_process_group_when_timeout_is_reached() {
    let temp = assert_fs::TempDir::new().unwrap();
    let pid_file = temp.child("bg.pid");

    let mut req = script_req(&[
        "-lc",
        &format!(
            "sleep 5 & bg=$!; printf %s \"$bg\" > {}; wait",
            shell_quote(pid_file.path())
        ),
    ]);
    req.workspace_root = temp.path().to_path_buf();
    req.script.timeout_ms = Some(250);

    let err = ScriptRunner::run(req).await.unwrap_err();
    assert!(err.to_string().contains("脚本超时"));

    wait_for_file(pid_file.path()).await;
    let pid = std::fs::read_to_string(pid_file.path()).unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let status = std::process::Command::new("bash")
        .args(["-lc", &format!("kill -0 {} 2>/dev/null", pid.trim())])
        .status()
        .unwrap();
    assert!(!status.success(), "background child should be gone");
}

#[tokio::test]
async fn run_should_error_when_exit_code_is_unexpected() {
    let req = script_req(&["-lc", "exit 7"]);

    let err = ScriptRunner::run(req).await.unwrap_err();
    assert_eq!(
        err.exit_code(),
        agentstow_core::ExitCode::ExternalCommandFailed
    );
    assert!(err.to_string().contains("code=7"));
}

#[cfg(not(windows))]
#[tokio::test]
async fn run_should_use_workspace_cwd_when_requested() {
    let temp = assert_fs::TempDir::new().unwrap();
    let workspace = temp.child("workspace");
    workspace.create_dir_all().unwrap();

    let mut req = script_req(&["-lc", "pwd"]);
    req.workspace_root = workspace.path().to_path_buf();
    req.script.cwd_policy = agentstow_core::CwdPolicy::Workspace;

    let out = ScriptRunner::run(req).await.unwrap();
    assert_eq!(out.exit_code, 0);

    let actual = std::fs::canonicalize(out.stdout.unwrap().trim()).unwrap();
    let expected = std::fs::canonicalize(workspace.path()).unwrap();
    assert_eq!(actual, expected);
}

#[cfg(not(windows))]
#[tokio::test]
async fn run_should_pass_resolved_env_bindings_to_child_process() {
    let mut req = script_req(&["-lc", "printf %s \"$TOKEN\""]);
    req.script.env = vec![agentstow_manifest::EnvVarDef {
        key: "TOKEN".to_string(),
        binding: agentstow_core::SecretBinding::Literal {
            value: "from-test".to_string(),
        },
    }];

    let out = ScriptRunner::run(req).await.unwrap();
    assert_eq!(out.exit_code, 0);
    assert_eq!(out.stdout.unwrap(), "from-test");
}

#[cfg(not(windows))]
async fn wait_for_file(path: &std::path::Path) {
    for _ in 0..100 {
        if path.exists() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("pid file was not created: {}", path.display());
}

#[cfg(not(windows))]
fn shell_quote(path: &std::path::Path) -> String {
    let text = path.display().to_string();
    format!("'{}'", text.replace('\'', "'\"'\"'"))
}
