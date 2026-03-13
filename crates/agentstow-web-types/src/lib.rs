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
pub struct RenderResponse {
    pub text: String,
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
pub struct WatchStatusResponse {
    pub mode: WatchModeResponse,
    pub revision: u64,
    pub poll_interval_ms: Option<u64>,
    pub last_event: Option<String>,
    pub last_event_at: Option<String>,
    pub last_error: Option<String>,
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
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EnvSetSummaryResponse {
    pub id: String,
    pub vars: Vec<EnvVarSummaryResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ScriptSummaryResponse {
    pub id: String,
    pub kind: String,
    pub entry: String,
    pub args: Vec<String>,
    pub env_keys: Vec<String>,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct McpServerSummaryResponse {
    pub id: String,
    pub transport_kind: McpTransportKindResponse,
    pub location: String,
    pub env_keys: Vec<String>,
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
    RenderResponse::export_all(&config)?;
    InstallMethodResponse::export_all(&config)?;
    ArtifactKindResponse::export_all(&config)?;
    ValidateAsResponse::export_all(&config)?;
    McpTransportKindResponse::export_all(&config)?;
    ImpactSubjectKindResponse::export_all(&config)?;
    WatchModeResponse::export_all(&config)?;
    LinkRecordResponse::export_all(&config)?;
    LinkStatusResponseItem::export_all(&config)?;
    WatchStatusResponse::export_all(&config)?;
    WorkspaceCountsResponse::export_all(&config)?;
    TargetSummaryResponse::export_all(&config)?;
    ArtifactSummaryResponse::export_all(&config)?;
    ProfileSummaryResponse::export_all(&config)?;
    ProfileVarResponse::export_all(&config)?;
    EnvVarSummaryResponse::export_all(&config)?;
    EnvSetSummaryResponse::export_all(&config)?;
    ScriptSummaryResponse::export_all(&config)?;
    McpServerSummaryResponse::export_all(&config)?;
    ValidationIssueResponse::export_all(&config)?;
    WorkspaceSummaryResponse::export_all(&config)?;
    ArtifactDetailResponse::export_all(&config)?;
    ProfileDetailResponse::export_all(&config)?;
    ImpactAnalysisResponse::export_all(&config)?;
    Ok(())
}

#[cfg(test)]
mod tests;
