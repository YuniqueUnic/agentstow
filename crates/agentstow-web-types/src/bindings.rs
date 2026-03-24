use ts_rs::Config;
use ts_rs::TS;

use crate::{
    ApiError, ArtifactDetailResponse, ArtifactGitCompareResponse, ArtifactGitHistoryResponse,
    ArtifactGitRollbackRequest, ArtifactGitRollbackResponse, ArtifactKindResponse,
    ArtifactSourceResponse, ArtifactSourceUpdateRequest, ArtifactSummaryResponse, EnvEmitRequest,
    EnvEmitResponse, EnvEmitSetSummaryResponse, EnvUsageOwnerKindResponse, EnvUsageRefResponse,
    EnvVarSummaryResponse, GitCommitSummaryResponse, HealthResponse, ImpactAnalysisResponse,
    ImpactSubjectKindResponse, InstallMethodResponse, LinkApplyRequest, LinkDesiredInstallResponse,
    LinkOperationActionResponse, LinkOperationItemResponse, LinkOperationResponse,
    LinkPlanItemResponse, LinkPlanRequest, LinkRecordResponse, LinkRepairRequest,
    LinkStatusResponseItem, ManifestResponse, ManifestSourceResponse, ManifestSourceUpdateRequest,
    McpCheckResponse, McpCheckStatusResponse, McpHeaderResponse, McpRenderResponse,
    McpServerSummaryResponse, McpTestResponse, McpTransportKindResponse, McpValidateResponse,
    ProfileDetailResponse, ProfileSummaryResponse, ProfileVarResponse,
    ProfileVarSyntaxModeResponse, ProfileVarUpdateItemRequest, ProfileVarsUpdateRequest,
    RenderResponse, ScriptRunRequest, ScriptRunResponse, ScriptSummaryResponse,
    SecretBindingKindResponse, ShellKindResponse, TargetSummaryResponse, ValidateAsResponse,
    ValidationIssueResponse, WatchModeResponse, WatchStatusResponse, WatchTraceEventResponse,
    WatchTraceLevelResponse, WorkspaceCountsResponse, WorkspaceGitSummaryResponse,
    WorkspaceInitRequest, WorkspaceInitResponse, WorkspaceProbeRequest, WorkspaceProbeResponse,
    WorkspaceSelectRequest, WorkspaceSelectResponse, WorkspaceStateResponse,
    WorkspaceSummaryResponse,
};

pub fn export_bindings() -> Result<(), ts_rs::ExportError> {
    let config = Config::from_env();
    ApiError::export_all(&config)?;
    HealthResponse::export_all(&config)?;
    ManifestResponse::export_all(&config)?;
    WorkspaceStateResponse::export_all(&config)?;
    WorkspaceProbeRequest::export_all(&config)?;
    WorkspaceProbeResponse::export_all(&config)?;
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
    ProfileVarUpdateItemRequest::export_all(&config)?;
    ProfileVarsUpdateRequest::export_all(&config)?;
    ProfileVarSyntaxModeResponse::export_all(&config)?;
    EnvUsageRefResponse::export_all(&config)?;
    EnvVarSummaryResponse::export_all(&config)?;
    EnvEmitSetSummaryResponse::export_all(&config)?;
    ScriptSummaryResponse::export_all(&config)?;
    McpServerSummaryResponse::export_all(&config)?;
    McpHeaderResponse::export_all(&config)?;
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
