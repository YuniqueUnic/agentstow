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
