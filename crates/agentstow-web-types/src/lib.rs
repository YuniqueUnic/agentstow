use serde::{Deserialize, Serialize};
use ts_rs::TS;

mod bindings;
mod links;
mod workspace;

pub use bindings::export_bindings;
pub use links::*;
pub use workspace::*;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ApiError {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct HealthResponse {
    pub ok: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RenderResponse {
    pub text: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ShellKindResponse {
    Bash,
    Zsh,
    Fish,
    Powershell,
    Cmd,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EnvEmitRequest {
    pub set: Option<String>,
    pub shell: ShellKindResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EnvEmitResponse {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ScriptRunRequest {
    pub stdin: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ScriptRunResponse {
    pub exit_code: i32,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum InstallMethodResponse {
    Symlink,
    Junction,
    Copy,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ArtifactKindResponse {
    File,
    Dir,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ValidateAsResponse {
    None,
    Json,
    Toml,
    Markdown,
    Shell,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum McpTransportKindResponse {
    Stdio,
    Http,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum SecretBindingKindResponse {
    Literal,
    Env,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum EnvUsageOwnerKindResponse {
    EnvEmitSet,
    Script,
    McpServer,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ImpactSubjectKindResponse {
    Artifact,
    Profile,
    ArtifactProfile,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum WatchModeResponse {
    Native,
    Poll,
    Manual,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum WatchTraceLevelResponse {
    Change,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WatchStatusResponse {
    pub mode: WatchModeResponse,
    pub healthy: bool,
    #[ts(type = "number")]
    pub revision: u64,
    #[ts(type = "number | null")]
    pub poll_interval_ms: Option<u64>,
    pub last_event: Option<String>,
    pub last_event_at: Option<String>,
    pub last_error: Option<String>,
    pub watch_roots: Vec<String>,
    pub recent_events: Vec<WatchTraceEventResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WatchTraceEventResponse {
    #[ts(type = "number")]
    pub revision: u64,
    pub level: WatchTraceLevelResponse,
    pub summary: String,
    pub at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WorkspaceCountsResponse {
    pub profile_count: usize,
    pub artifact_count: usize,
    pub target_count: usize,
    pub env_emit_set_count: usize,
    pub script_count: usize,
    pub mcp_server_count: usize,
    pub link_count: usize,
    pub healthy_link_count: usize,
    pub unhealthy_link_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TargetSummaryResponse {
    pub id: String,
    pub artifact_id: String,
    pub profile: Option<String>,
    pub target_path: String,
    pub method: InstallMethodResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ArtifactSummaryResponse {
    pub id: String,
    pub kind: ArtifactKindResponse,
    pub source_path: String,
    pub template: bool,
    pub validate_as: ValidateAsResponse,
    pub target_ids: Vec<String>,
    pub profiles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProfileSummaryResponse {
    pub id: String,
    pub extends: Vec<String>,
    pub variable_keys: Vec<String>,
    pub target_ids: Vec<String>,
    pub artifact_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProfileVarResponse {
    pub key: String,
    pub value_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProfileVarUpdateItemRequest {
    pub key: String,
    pub value_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProfileVarsUpdateRequest {
    pub vars: Vec<ProfileVarUpdateItemRequest>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ProfileVarSyntaxModeResponse {
    Inline,
    VarsObject,
    Mixed,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EnvVarSummaryResponse {
    pub key: String,
    pub binding: String,
    pub binding_kind: SecretBindingKindResponse,
    pub source_env_var: Option<String>,
    pub rendered_placeholder: String,
    pub available: bool,
    pub diagnostic: Option<String>,
    pub referrers: Vec<EnvUsageRefResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EnvUsageRefResponse {
    pub owner_kind: EnvUsageOwnerKindResponse,
    pub owner_id: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EnvEmitSetSummaryResponse {
    pub id: String,
    pub vars: Vec<EnvVarSummaryResponse>,
    pub available_count: usize,
    pub missing_count: usize,
    pub referrers: Vec<EnvUsageRefResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ScriptSummaryResponse {
    pub id: String,
    pub kind: String,
    pub entry: String,
    pub args: Vec<String>,
    pub env_keys: Vec<String>,
    pub env_bindings: Vec<EnvVarSummaryResponse>,
    #[ts(type = "number | null")]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct McpServerSummaryResponse {
    pub id: String,
    pub transport_kind: McpTransportKindResponse,
    pub location: String,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub url: Option<String>,
    pub headers: Vec<McpHeaderResponse>,
    pub env_keys: Vec<String>,
    pub env_bindings: Vec<EnvVarSummaryResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct McpHeaderResponse {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum McpCheckStatusResponse {
    Ok,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct McpCheckResponse {
    pub code: String,
    pub status: McpCheckStatusResponse,
    pub message: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct McpValidateResponse {
    pub server_id: String,
    pub ok: bool,
    pub issues: Vec<ValidationIssueResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct McpRenderResponse {
    pub server_id: String,
    pub transport_kind: McpTransportKindResponse,
    pub launcher_preview: String,
    pub config_json: String,
    pub env_bindings: Vec<EnvVarSummaryResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct McpTestResponse {
    pub server_id: String,
    pub ok: bool,
    pub checks: Vec<McpCheckResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ValidationIssueResponse {
    pub severity: String,
    pub scope: String,
    pub subject_id: String,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WorkspaceSummaryResponse {
    pub workspace_root: String,
    pub counts: WorkspaceCountsResponse,
    pub profiles: Vec<ProfileSummaryResponse>,
    pub artifacts: Vec<ArtifactSummaryResponse>,
    pub targets: Vec<TargetSummaryResponse>,
    pub env_emit_sets: Vec<EnvEmitSetSummaryResponse>,
    pub scripts: Vec<ScriptSummaryResponse>,
    pub mcp_servers: Vec<McpServerSummaryResponse>,
    pub issues: Vec<ValidationIssueResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ArtifactDetailResponse {
    pub artifact: ArtifactSummaryResponse,
    pub targets: Vec<TargetSummaryResponse>,
    pub profiles: Vec<ProfileSummaryResponse>,
    pub issues: Vec<ValidationIssueResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProfileDetailResponse {
    pub profile: ProfileSummaryResponse,
    pub syntax_mode: ProfileVarSyntaxModeResponse,
    pub declared_vars: Vec<ProfileVarResponse>,
    pub merged_vars: Vec<ProfileVarResponse>,
    pub targets: Vec<TargetSummaryResponse>,
    pub artifacts: Vec<ArtifactSummaryResponse>,
    pub issues: Vec<ValidationIssueResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ImpactAnalysisResponse {
    pub subject_kind: ImpactSubjectKindResponse,
    pub subject_id: String,
    pub affected_targets: Vec<TargetSummaryResponse>,
    pub affected_artifacts: Vec<ArtifactSummaryResponse>,
    pub affected_profiles: Vec<ProfileSummaryResponse>,
    pub link_status: Vec<LinkStatusResponseItem>,
    pub issues: Vec<ValidationIssueResponse>,
}

#[cfg(test)]
mod tests;
