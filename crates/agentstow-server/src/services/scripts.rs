use agentstow_core::{AgentStowError, Result};
use agentstow_scripts::{ScriptRunRequest, ScriptRunner};
use agentstow_web_types::ScriptRunResponse;

use super::WorkspaceQueryService;

impl WorkspaceQueryService {
    pub(crate) async fn script_run(
        &self,
        script_id: &str,
        stdin: Option<&str>,
    ) -> Result<ScriptRunResponse> {
        let manifest = self.load_manifest()?;
        let script = manifest
            .scripts
            .get(script_id)
            .ok_or_else(|| AgentStowError::Manifest {
                message: format!("script 不存在：{script_id}").into(),
            })?
            .clone();

        let output = ScriptRunner::run(ScriptRunRequest {
            workspace_root: manifest.workspace_root,
            script,
            stdin_text: stdin.map(ToString::to_string),
        })
        .await?;

        Ok(ScriptRunResponse {
            exit_code: output.exit_code,
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }
}
