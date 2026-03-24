use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{ArtifactKindResponse, ValidateAsResponse};

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
    pub workspace: Option<WorkspaceProbeResponse>,
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
pub struct WorkspaceProbeRequest {
    pub workspace_root: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WorkspaceProbeResponse {
    pub requested_workspace_root: String,
    pub resolved_workspace_root: String,
    pub exists: bool,
    pub is_directory: bool,
    pub manifest_present: bool,
    pub manifest_path: String,
    pub git_present: bool,
    pub selectable: bool,
    pub initializable: bool,
    pub reason: Option<String>,
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
    pub workspace: WorkspaceProbeResponse,
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
    pub workspace: WorkspaceProbeResponse,
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
