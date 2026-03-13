use std::process::Stdio;
use std::time::Duration;

use agentstow_core::{AgentStowError, CwdPolicy, OutputMode, Result, StdinMode};
use agentstow_manifest::{EnvVarDef, ScriptDef};
use command_group::AsyncCommandGroup;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
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

        match req.script.stdin_mode {
            StdinMode::None => {
                cmd.stdin(Stdio::null());
            }
            StdinMode::Text | StdinMode::Json => {
                cmd.stdin(Stdio::piped());
            }
        }

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

        if matches!(req.script.stdin_mode, StdinMode::Text | StdinMode::Json)
            && let Some(mut stdin) = child.inner().stdin.take()
        {
            if let Some(stdin_text) = &req.stdin_text {
                stdin
                    .write_all(stdin_text.as_bytes())
                    .await
                    .map_err(AgentStowError::from)?;
            }
            drop(stdin);
        }

        let mut stdout_buf = Vec::new();
        let mut stderr_buf = Vec::new();

        let mut stdout = child.inner().stdout.take();
        let mut stderr = child.inner().stderr.take();

        let read_stdout = async {
            if let Some(mut out) = stdout.take() {
                tokio::io::AsyncReadExt::read_to_end(&mut out, &mut stdout_buf)
                    .await
                    .map_err(AgentStowError::from)?;
            }
            Ok::<(), AgentStowError>(())
        };
        let read_stderr = async {
            if let Some(mut out) = stderr.take() {
                tokio::io::AsyncReadExt::read_to_end(&mut out, &mut stderr_buf)
                    .await
                    .map_err(AgentStowError::from)?;
            }
            Ok::<(), AgentStowError>(())
        };

        let wait_fut = async {
            let status = child.wait().await.map_err(|e| AgentStowError::Script {
                message: format!("wait 失败：{e}").into(),
            })?;
            Ok::<std::process::ExitStatus, AgentStowError>(status)
        };

        let status = if let Some(ms) = req.script.timeout_ms {
            match tokio::time::timeout(Duration::from_millis(ms), async {
                let (status, _, _) = tokio::try_join!(wait_fut, read_stdout, read_stderr)?;
                Ok::<_, AgentStowError>(status)
            })
            .await
            {
                Ok(Ok(status)) => status,
                Ok(Err(e)) => return Err(e),
                Err(_) => {
                    // timeout
                    let _ = child.kill().await;
                    return Err(AgentStowError::Script {
                        message: format!("脚本超时（{}ms）: {}", ms, req.script.entry).into(),
                    });
                }
            }
        } else {
            let (status, _, _) = tokio::try_join!(wait_fut, read_stdout, read_stderr)?;
            status
        };

        let code = status.code().unwrap_or(-1);
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

        let stdout_s = if capture_stdout {
            Some(String::from_utf8_lossy(&stdout_buf).to_string())
        } else {
            None
        };
        let stderr_s = if capture_stderr {
            Some(String::from_utf8_lossy(&stderr_buf).to_string())
        } else {
            None
        };

        Ok(ScriptRunOutput {
            exit_code: code,
            stdout: stdout_s,
            stderr: stderr_s,
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

#[cfg(test)]
mod tests;
