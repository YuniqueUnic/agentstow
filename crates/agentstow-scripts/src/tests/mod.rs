use pretty_assertions::assert_eq;

use super::*;

#[tokio::test]
async fn run_capture_stdout_should_work() {
    let req = ScriptRunRequest {
        workspace_root: std::env::current_dir().unwrap(),
        script: agentstow_manifest::ScriptDef {
            kind: "shell".to_string(),
            entry: "bash".to_string(),
            args: vec!["-lc".to_string(), "echo hello".to_string()],
            cwd_policy: agentstow_core::CwdPolicy::Current,
            env: vec![],
            stdin_mode: agentstow_core::StdinMode::None,
            stdout_mode: agentstow_core::OutputMode::Capture,
            stderr_mode: agentstow_core::OutputMode::Capture,
            timeout_ms: Some(5_000),
            expected_exit_codes: vec![0],
        },
        stdin_text: None,
    };

    let out = ScriptRunner::run(req).await.unwrap();
    assert_eq!(out.exit_code, 0);
    assert_eq!(out.stdout.unwrap(), "hello\n");
}

#[cfg(not(windows))]
#[tokio::test]
async fn run_text_mode_without_stdin_should_close_pipe_and_finish() {
    let req = ScriptRunRequest {
        workspace_root: std::env::current_dir().unwrap(),
        script: agentstow_manifest::ScriptDef {
            kind: "shell".to_string(),
            entry: "bash".to_string(),
            args: vec!["-lc".to_string(), "cat >/dev/null && echo done".to_string()],
            cwd_policy: agentstow_core::CwdPolicy::Current,
            env: vec![],
            stdin_mode: agentstow_core::StdinMode::Text,
            stdout_mode: agentstow_core::OutputMode::Capture,
            stderr_mode: agentstow_core::OutputMode::Capture,
            timeout_ms: Some(1_000),
            expected_exit_codes: vec![0],
        },
        stdin_text: None,
    };

    let out = ScriptRunner::run(req).await.unwrap();
    assert_eq!(out.exit_code, 0);
    assert_eq!(out.stdout.unwrap(), "done\n");
}

#[cfg(not(windows))]
#[tokio::test]
async fn run_should_fail_when_timeout_is_reached() {
    let req = ScriptRunRequest {
        workspace_root: std::env::current_dir().unwrap(),
        script: agentstow_manifest::ScriptDef {
            kind: "shell".to_string(),
            entry: "bash".to_string(),
            args: vec!["-lc".to_string(), "sleep 1".to_string()],
            cwd_policy: agentstow_core::CwdPolicy::Current,
            env: vec![],
            stdin_mode: agentstow_core::StdinMode::None,
            stdout_mode: agentstow_core::OutputMode::Capture,
            stderr_mode: agentstow_core::OutputMode::Capture,
            timeout_ms: Some(50),
            expected_exit_codes: vec![0],
        },
        stdin_text: None,
    };

    let err = ScriptRunner::run(req).await.unwrap_err();
    assert_eq!(
        err.exit_code(),
        agentstow_core::ExitCode::ExternalCommandFailed
    );
    assert!(err.to_string().contains("脚本超时"));
}

#[tokio::test]
async fn run_text_stdin_should_close_pipe_even_when_input_missing() {
    let req = ScriptRunRequest {
        workspace_root: std::env::current_dir().unwrap(),
        script: agentstow_manifest::ScriptDef {
            kind: "shell".to_string(),
            entry: "bash".to_string(),
            args: vec![
                "-lc".to_string(),
                "python3 -c 'import sys; data=sys.stdin.read(); print(len(data))'".to_string(),
            ],
            cwd_policy: agentstow_core::CwdPolicy::Current,
            env: vec![],
            stdin_mode: agentstow_core::StdinMode::Text,
            stdout_mode: agentstow_core::OutputMode::Capture,
            stderr_mode: agentstow_core::OutputMode::Capture,
            timeout_ms: Some(1_000),
            expected_exit_codes: vec![0],
        },
        stdin_text: None,
    };

    let out = ScriptRunner::run(req).await.unwrap();
    assert_eq!(out.exit_code, 0);
    assert_eq!(out.stdout.unwrap(), "0\n");
}

#[tokio::test]
async fn run_should_error_when_exit_code_is_unexpected() {
    let req = ScriptRunRequest {
        workspace_root: std::env::current_dir().unwrap(),
        script: agentstow_manifest::ScriptDef {
            kind: "shell".to_string(),
            entry: "bash".to_string(),
            args: vec!["-lc".to_string(), "exit 7".to_string()],
            cwd_policy: agentstow_core::CwdPolicy::Current,
            env: vec![],
            stdin_mode: agentstow_core::StdinMode::None,
            stdout_mode: agentstow_core::OutputMode::Capture,
            stderr_mode: agentstow_core::OutputMode::Capture,
            timeout_ms: Some(1_000),
            expected_exit_codes: vec![0],
        },
        stdin_text: None,
    };

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
    let workspace_root = std::env::temp_dir().join("agentstow-script-cwd");
    std::fs::create_dir_all(&workspace_root).unwrap();

    let req = ScriptRunRequest {
        workspace_root: workspace_root.clone(),
        script: agentstow_manifest::ScriptDef {
            kind: "shell".to_string(),
            entry: "bash".to_string(),
            args: vec!["-lc".to_string(), "pwd".to_string()],
            cwd_policy: agentstow_core::CwdPolicy::Workspace,
            env: vec![],
            stdin_mode: agentstow_core::StdinMode::None,
            stdout_mode: agentstow_core::OutputMode::Capture,
            stderr_mode: agentstow_core::OutputMode::Capture,
            timeout_ms: Some(1_000),
            expected_exit_codes: vec![0],
        },
        stdin_text: None,
    };

    let out = ScriptRunner::run(req).await.unwrap();
    assert_eq!(out.exit_code, 0);
    let actual = std::fs::canonicalize(out.stdout.unwrap().trim()).unwrap();
    let expected = std::fs::canonicalize(&workspace_root).unwrap();
    assert_eq!(actual, expected);
    std::fs::remove_dir_all(workspace_root).unwrap();
}

#[cfg(not(windows))]
#[tokio::test]
async fn run_should_pass_resolved_env_bindings_to_child_process() {
    let req = ScriptRunRequest {
        workspace_root: std::env::current_dir().unwrap(),
        script: agentstow_manifest::ScriptDef {
            kind: "shell".to_string(),
            entry: "bash".to_string(),
            args: vec!["-lc".to_string(), "printf %s \"$TOKEN\"".to_string()],
            cwd_policy: agentstow_core::CwdPolicy::Current,
            env: vec![agentstow_manifest::EnvVarDef {
                key: "TOKEN".to_string(),
                binding: agentstow_core::SecretBinding::Literal {
                    value: "from-test".to_string(),
                },
            }],
            stdin_mode: agentstow_core::StdinMode::None,
            stdout_mode: agentstow_core::OutputMode::Capture,
            stderr_mode: agentstow_core::OutputMode::Capture,
            timeout_ms: Some(1_000),
            expected_exit_codes: vec![0],
        },
        stdin_text: None,
    };

    let out = ScriptRunner::run(req).await.unwrap();
    assert_eq!(out.exit_code, 0);
    assert_eq!(out.stdout.unwrap(), "from-test");
}
