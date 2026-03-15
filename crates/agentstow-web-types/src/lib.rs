use serde::{Deserialize, Serialize};
use ts_rs::{Config, TS};

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
pub struct ManifestResponse {
    pub workspace_root: String,
    pub profiles: Vec<String>,
    pub artifacts: Vec<String>,
    pub targets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WorkspaceStateResponse {
    pub workspace_root: Option<String>,
    pub manifest_present: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WorkspaceGitSummaryResponse {
    pub repo_root: String,
    pub branch: Option<String>,
    pub head: String,
    pub head_short: String,
    pub dirty: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct GitCommitSummaryResponse {
    pub revision: String,
    pub short_revision: String,
    pub summary: String,
    pub author_name: String,
    pub authored_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ArtifactGitHistoryResponse {
    pub artifact_id: String,
    pub source_path: String,
    pub repo_relative_path: String,
    pub branch: Option<String>,
    pub head: String,
    pub head_short: String,
    pub dirty: bool,
    pub commits: Vec<GitCommitSummaryResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ArtifactGitCompareResponse {
    pub artifact_id: String,
    pub source_path: String,
    pub repo_relative_path: String,
    pub base_revision: String,
    pub head_revision: String,
    pub base_label: String,
    pub head_label: String,
    pub base_content: String,
    pub head_content: String,
    pub changed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ArtifactGitRollbackRequest {
    pub revision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ArtifactGitRollbackResponse {
    pub artifact_id: String,
    pub commit: GitCommitSummaryResponse,
    pub source: ArtifactSourceResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WorkspaceSelectRequest {
    pub workspace_root: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WorkspaceSelectResponse {
    pub workspace_root: String,
    pub manifest_present: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WorkspaceInitRequest {
    pub workspace_root: String,
    pub git_init: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WorkspaceInitResponse {
    pub workspace_root: String,
    pub manifest_path: String,
    pub created: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ManifestSourceResponse {
    pub source_path: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ArtifactSourceResponse {
    pub artifact_id: String,
    pub kind: ArtifactKindResponse,
    pub source_path: String,
    pub template: bool,
    pub validate_as: ValidateAsResponse,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ArtifactSourceUpdateRequest {
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ManifestSourceUpdateRequest {
    pub content: String,
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
    pub env_set_id: String,
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
    EnvSet,
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
pub struct LinkRecordResponse {
    pub artifact_id: String,
    pub profile: String,
    pub target_path: String,
    pub method: InstallMethodResponse,
    pub rendered_path: Option<String>,
    pub blake3: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LinkStatusResponseItem {
    pub artifact_id: String,
    pub profile: String,
    pub target_path: String,
    pub method: InstallMethodResponse,
    pub ok: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LinkPlanRequest {
    /// 可选：为空则对所有 targets 生效
    pub targets: Vec<String>,
    /// target 未声明 profile 时可用（等价于 CLI 的全局 --profile）
    pub default_profile: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LinkApplyRequest {
    /// 可选：为空则对所有 targets 生效
    pub targets: Vec<String>,
    /// target 未声明 profile 时可用（等价于 CLI 的全局 --profile）
    pub default_profile: Option<String>,
    pub force: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LinkRepairRequest {
    /// 可选：为空则只修复不健康项（坏链扫描结果）
    pub targets: Vec<String>,
    /// target 未声明 profile 时可用（等价于 CLI 的全局 --profile）
    pub default_profile: Option<String>,
    pub force: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export)]
pub enum LinkDesiredInstallResponse {
    Symlink { source_path: String },
    Junction { source_path: String },
    Copy { blake3: String, bytes_len: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LinkPlanItemResponse {
    pub target: String,
    pub artifact_id: String,
    pub profile: String,
    pub artifact_kind: ArtifactKindResponse,
    pub method: InstallMethodResponse,
    pub target_path: String,
    pub desired: LinkDesiredInstallResponse,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum LinkOperationActionResponse {
    Planned,
    Applied,
    Repaired,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LinkOperationItemResponse {
    pub action: LinkOperationActionResponse,
    pub item: LinkPlanItemResponse,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LinkOperationResponse {
    pub items: Vec<LinkOperationItemResponse>,
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
    pub env_set_count: usize,
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
pub struct EnvSetSummaryResponse {
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
    pub env_sets: Vec<EnvSetSummaryResponse>,
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

pub fn export_bindings() -> Result<(), ts_rs::ExportError> {
    let config = Config::from_env();
    ApiError::export_all(&config)?;
    HealthResponse::export_all(&config)?;
    ManifestResponse::export_all(&config)?;
    WorkspaceStateResponse::export_all(&config)?;
    WorkspaceGitSummaryResponse::export_all(&config)?;
    GitCommitSummaryResponse::export_all(&config)?;
    ArtifactGitHistoryResponse::export_all(&config)?;
    ArtifactGitCompareResponse::export_all(&config)?;
    ArtifactGitRollbackRequest::export_all(&config)?;
    ArtifactGitRollbackResponse::export_all(&config)?;
    WorkspaceSelectRequest::export_all(&config)?;
    WorkspaceSelectResponse::export_all(&config)?;
    WorkspaceInitRequest::export_all(&config)?;
    WorkspaceInitResponse::export_all(&config)?;
    ManifestSourceResponse::export_all(&config)?;
    ArtifactSourceResponse::export_all(&config)?;
    ArtifactSourceUpdateRequest::export_all(&config)?;
    ManifestSourceUpdateRequest::export_all(&config)?;
    RenderResponse::export_all(&config)?;
    ShellKindResponse::export_all(&config)?;
    EnvEmitRequest::export_all(&config)?;
    EnvEmitResponse::export_all(&config)?;
    ScriptRunRequest::export_all(&config)?;
    ScriptRunResponse::export_all(&config)?;
    InstallMethodResponse::export_all(&config)?;
    ArtifactKindResponse::export_all(&config)?;
    ValidateAsResponse::export_all(&config)?;
    McpTransportKindResponse::export_all(&config)?;
    SecretBindingKindResponse::export_all(&config)?;
    EnvUsageOwnerKindResponse::export_all(&config)?;
    ImpactSubjectKindResponse::export_all(&config)?;
    WatchModeResponse::export_all(&config)?;
    WatchTraceLevelResponse::export_all(&config)?;
    LinkRecordResponse::export_all(&config)?;
    LinkStatusResponseItem::export_all(&config)?;
    LinkPlanRequest::export_all(&config)?;
    LinkApplyRequest::export_all(&config)?;
    LinkRepairRequest::export_all(&config)?;
    LinkDesiredInstallResponse::export_all(&config)?;
    LinkPlanItemResponse::export_all(&config)?;
    LinkOperationActionResponse::export_all(&config)?;
    LinkOperationItemResponse::export_all(&config)?;
    LinkOperationResponse::export_all(&config)?;
    WatchTraceEventResponse::export_all(&config)?;
    WatchStatusResponse::export_all(&config)?;
    WorkspaceCountsResponse::export_all(&config)?;
    TargetSummaryResponse::export_all(&config)?;
    ArtifactSummaryResponse::export_all(&config)?;
    ProfileSummaryResponse::export_all(&config)?;
    ProfileVarResponse::export_all(&config)?;
    EnvUsageRefResponse::export_all(&config)?;
    EnvVarSummaryResponse::export_all(&config)?;
    EnvSetSummaryResponse::export_all(&config)?;
    ScriptSummaryResponse::export_all(&config)?;
    McpServerSummaryResponse::export_all(&config)?;
    McpCheckStatusResponse::export_all(&config)?;
    McpCheckResponse::export_all(&config)?;
    McpValidateResponse::export_all(&config)?;
    McpRenderResponse::export_all(&config)?;
    McpTestResponse::export_all(&config)?;
    ValidationIssueResponse::export_all(&config)?;
    WorkspaceSummaryResponse::export_all(&config)?;
    ArtifactDetailResponse::export_all(&config)?;
    ProfileDetailResponse::export_all(&config)?;
    ImpactAnalysisResponse::export_all(&config)?;
    Ok(())
}

#[cfg(test)]
mod tests;
