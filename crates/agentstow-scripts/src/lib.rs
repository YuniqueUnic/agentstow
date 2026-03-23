use std::process::{Output, Stdio};
use std::time::Duration;

use agentstow_core::{AgentStowError, CwdPolicy, OutputMode, Result, StdinMode};
use agentstow_manifest::{EnvVarDef, ScriptDef};
use command_group::AsyncCommandGroup;
#[cfg(unix)]
use nix::sys::signal::{Signal, killpg};
#[cfg(unix)]
use nix::unistd::Pid;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use tokio::task::{JoinError, JoinHandle};
use tracing::instrument;

#[derive(Debug, Clone)]
pub struct ScriptRunRequest {
    pub workspace_root: std::path::PathBuf,
    pub script: ScriptDef,
    pub stdin_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptRunOutput {
    pub exit_code: i32,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
}

pub struct ScriptRunner;

impl ScriptRunner {
    #[instrument(skip_all, fields(entry=%req.script.entry, cwd_policy=?req.script.cwd_policy))]
    pub async fn run(req: ScriptRunRequest) -> Result<ScriptRunOutput> {
        let mut cmd = tokio::process::Command::new(&req.script.entry);
        cmd.args(&req.script.args);

        match req.script.cwd_policy {
            CwdPolicy::Workspace => cmd.current_dir(&req.workspace_root),
            CwdPolicy::Current => &mut cmd,
        };

        apply_env_bindings(&mut cmd, &req.script.env)?;

        let pipe_stdin = matches!(req.script.stdin_mode, StdinMode::Text | StdinMode::Json)
            && req.stdin_text.is_some();
        cmd.stdin(if pipe_stdin {
            Stdio::piped()
        } else {
            Stdio::null()
        });

        let capture_stdout = matches!(
            req.script.stdout_mode,
            OutputMode::Capture | OutputMode::Json
        );
        let capture_stderr = matches!(
            req.script.stderr_mode,
            OutputMode::Capture | OutputMode::Json
        );

        cmd.stdout(if capture_stdout {
            Stdio::piped()
        } else {
            Stdio::inherit()
        });
        cmd.stderr(if capture_stderr {
            Stdio::piped()
        } else {
            Stdio::inherit()
        });

        let mut child = cmd.group_spawn().map_err(|e| AgentStowError::Script {
            message: format!("spawn 失败：{e}").into(),
        })?;

        if pipe_stdin && let Some(mut stdin) = child.inner().stdin.take() {
            stdin
                .write_all(req.stdin_text.as_deref().unwrap_or_default().as_bytes())
                .await
                .map_err(AgentStowError::from)?;
            drop(stdin);
        }

        let process_group_id = child.id();
        let mut wait_task = tokio::spawn(async move { child.wait_with_output().await });
        let output = match req.script.timeout_ms {
            Some(timeout_ms) => {
                resolve_wait_task_with_timeout(
                    &mut wait_task,
                    process_group_id,
                    timeout_ms,
                    &req.script.entry,
                )
                .await?
            }
            None => resolve_wait_task(&mut wait_task).await?,
        };

        let code = output.status.code().unwrap_or(-1);
        let expected = if req.script.expected_exit_codes.is_empty() {
            vec![0]
        } else {
            req.script.expected_exit_codes.clone()
        };
        if !expected.contains(&code) {
            return Err(AgentStowError::Script {
                message: format!("exit code 不符合预期：code={code}, expected={expected:?}").into(),
            });
        }

        Ok(ScriptRunOutput {
            exit_code: code,
            stdout: capture_stdout.then(|| String::from_utf8_lossy(&output.stdout).to_string()),
            stderr: capture_stderr.then(|| String::from_utf8_lossy(&output.stderr).to_string()),
        })
    }
}

fn apply_env_bindings(cmd: &mut tokio::process::Command, envs: &[EnvVarDef]) -> Result<()> {
    for EnvVarDef { key, binding } in envs {
        let value = binding.resolve()?;
        cmd.env(key, value);
    }
    Ok(())
}

async fn resolve_wait_task(wait_task: &mut JoinHandle<std::io::Result<Output>>) -> Result<Output> {
    resolve_wait_task_result(wait_task.await).map_err(|message| AgentStowError::Script {
        message: message.into(),
    })
}

async fn resolve_wait_task_with_timeout(
    wait_task: &mut JoinHandle<std::io::Result<Output>>,
    process_group_id: Option<u32>,
    timeout_ms: u64,
    entry: &str,
) -> Result<Output> {
    let timeout = Duration::from_millis(timeout_ms);
    match tokio::time::timeout(timeout, &mut *wait_task).await {
        Ok(result) => resolve_wait_task_result(result).map_err(|message| AgentStowError::Script {
            message: message.into(),
        }),
        Err(_) => {
            terminate_process_group(process_group_id);

            let _ = tokio::time::timeout(Duration::from_secs(1), &mut *wait_task).await;
            Err(AgentStowError::Script {
                message: format!("脚本超时（{}ms）: {}", timeout_ms, entry).into(),
            })
        }
    }
}

fn resolve_wait_task_result(
    result: std::result::Result<std::io::Result<Output>, JoinError>,
) -> std::result::Result<Output, String> {
    match result {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(error)) => Err(format!("wait 失败：{error}")),
        Err(error) => Err(join_task_error(error)),
    }
}

fn join_task_error(error: JoinError) -> String {
    if error.is_cancelled() {
        "wait task 被取消".to_string()
    } else if error.is_panic() {
        format!("wait task panic: {error}")
    } else {
        format!("wait task 失败：{error}")
    }
}

#[cfg(unix)]
fn terminate_process_group(process_group_id: Option<u32>) {
    let Some(process_group_id) = process_group_id else {
        return;
    };

    let Ok(raw_pid) = i32::try_from(process_group_id) else {
        return;
    };

    let _ = killpg(Pid::from_raw(raw_pid), Signal::SIGKILL);
}

#[cfg(not(unix))]
fn terminate_process_group(_process_group_id: Option<u32>) {}

#[cfg(test)]
mod tests;
